//! Die Client-Seite im Multiplayer.
//!
//! - Die eigene Figur wird sofort lokal bewegt (Vorhersage). Meldet der Server eine
//!   andere Position, wird auf seinen Stand zurückgesetzt und alle seitdem gemachten
//!   Eingaben werden erneut abgespielt.
//! - Mitspieler und Objekte werden leicht verzögert zwischen zwei Server-Ständen
//!   interpoliert, damit sie trotz Netzwerk-Schwankungen flüssig laufen.

use std::collections::VecDeque;
use std::time::Duration;

use engine::prelude::*;

use crate::characters::Action;
use crate::protocol::*;
use std::collections::HashMap;

use crate::world::World;

/// Verzögerung der Darstellung von Mitspielern und Objekten in Takten (8 ≈ 133 ms).
const INTERPOLATION_DELAY: f64 = 8.0;
/// Abweichung zum Server, ab der die eigene Figur korrigiert wird (Meter).
const CORRECTION_THRESHOLD: f32 = 0.02;
/// Wie viele der letzten Eingaben jedes Paket zusätzlich enthält.
const REDUNDANT_INPUTS: usize = 4;

struct PendingInput {
    input: PlayerInput,
    /// Position der Figur, nachdem diese Eingabe lokal angewendet wurde.
    predicted: Vec3,
}

pub struct Replica {
    net: NetClient,
    local_id: Option<PlayerId>,
    next_seq: u32,
    pending: VecDeque<PendingInput>,
    snapshots: VecDeque<Snapshot>,
    /// Geschätzter aktueller Server-Takt.
    server_tick: Option<f64>,
    last_reconciled: u32,
    pub corrections: u32,
    name: String,
    class: CharacterClass,
    /// Rohstoffe, deren Treffer schon vorab gezeigt wurden (Takt der Vorschau).
    previewed: HashMap<u32, u64>,
}

impl Replica {
    pub fn connect(address: &str, hello: &Hello) -> std::io::Result<Self> {
        Ok(Replica {
            net: NetClient::connect(address, PROTOCOL_ID, &encode(hello))?,
            name: hello.name.clone(),
            class: hello.class,
            previewed: HashMap::new(),
            local_id: None,
            next_seq: 1,
            pending: VecDeque::new(),
            snapshots: VecDeque::new(),
            server_tick: None,
            last_reconciled: 0,
            corrections: 0,
        })
    }

    pub fn local_id(&self) -> Option<PlayerId> {
        self.local_id
    }

    pub fn is_connected(&self) -> bool {
        self.net.is_connected()
    }

    pub fn ping_ms(&self) -> f64 {
        self.net.ping() * 1000.0
    }

    /// Merkt sich, dass ein Treffer schon vorab gezeigt wurde.
    pub fn note_preview(&mut self, id: u32, tick: u64) {
        self.previewed.insert(id, tick);
    }

    /// Ein Takt. Liefert einen Fehlertext, wenn die Verbindung weg ist.
    pub fn tick(&mut self, ctx: &mut Context, world: &mut World, input: PlayerInput) -> Result<(), String> {
        self.net.receive(Duration::from_secs_f32(Physics::FIXED_DT));
        if let Some(reason) = self.net.disconnected() {
            return Err(reason);
        }
        if !self.net.is_connected() {
            self.net.flush();
            return Ok(());
        }

        while let Some(bytes) = self.net.message(Channel::Reliable) {
            if let Some(message) = decode::<ServerMessage>(&bytes) {
                self.handle(ctx, world, message);
            }
        }
        while let Some(bytes) = self.net.message(Channel::Unreliable) {
            if let Some(message) = decode::<ServerMessage>(&bytes) {
                self.handle(ctx, world, message);
            }
        }

        world.day.advance(Physics::FIXED_DT);
        if let Some(tick) = &mut self.server_tick {
            *tick += 1.0;
        }
        self.predict(ctx, world, input);
        self.interpolate(ctx, world);
        self.net.flush();
        Ok(())
    }

    fn handle(&mut self, ctx: &mut Context, world: &mut World, message: ServerMessage) {
        match message {
            ServerMessage::Welcome { player_id, tick } => {
                log::info!("Mit dem Server verbunden, meine Spieler-ID: {player_id}");
                self.local_id = Some(player_id);
                self.server_tick = Some(tick as f64);
                let spawn = world.spawn;
                world.spawn_player(ctx, player_id, &self.name, self.class, spawn);
            }
            ServerMessage::PlayerJoined { player_id, name, class } => {
                let spawn = world.spawn;
                world.spawn_player(ctx, player_id, &name, class, spawn);
            }
            ServerMessage::PlayerLeft { player_id } => world.remove_player(ctx, player_id),
            ServerMessage::Spawn { id, kind, position, velocity, by } => {
                if let Some(thrower) = by.filter(|&p| Some(p) != self.local_id) {
                    world.play_action(thrower, Action::Throw);
                }
                if !world.objects.contains_key(&id) {
                    world.spawn_object(ctx, id, kind, position, velocity);
                }
            }
            ServerMessage::Despawn { id } => world.remove_object(ctx, id),
            ServerMessage::Snapshot(snapshot) => self.receive_snapshot(ctx, world, snapshot),
            ServerMessage::ResourceHit { id, health, by } => {
                if Some(by) != self.local_id {
                    world.play_action(by, Action::Chop);
                }
                // Eigene Schläge wurden schon vorab gezeigt – nicht doppelt.
                let shown = self.previewed.remove(&id).is_some_and(|tick| ctx.time.tick < tick + 60);
                world.resource_hit(ctx, id, health, !shown);
            }
            ServerMessage::ResourceGone { id, by } => {
                if Some(by) != self.local_id {
                    world.play_action(by, Action::Chop);
                }
                self.previewed.remove(&id);
                world.resource_hit(ctx, id, 0, true);
            }
            ServerMessage::ResourceBack { id } => world.resource_back(ctx, id),
            ServerMessage::ResourceStates { gone, damaged } => {
                for id in gone {
                    world.resource_hit(ctx, id, 0, false);
                }
                for (id, health) in damaged {
                    world.resource_hit(ctx, id, health, false);
                }
            }
            ServerMessage::Inventory(inventory) => {
                if let Some(local) = self.local_id {
                    world.inventories.insert(local, inventory);
                }
            }
        }
    }

    fn receive_snapshot(&mut self, ctx: &mut Context, world: &mut World, snapshot: Snapshot) {
        world.day.sync(snapshot.hour, snapshot.day);
        let tick = snapshot.tick as f64;
        match &mut self.server_tick {
            Some(estimate) if (*estimate - tick).abs() < 30.0 => *estimate += (tick - *estimate) * 0.05,
            estimate => *estimate = Some(tick),
        }

        if snapshot.tick > self.last_reconciled {
            self.last_reconciled = snapshot.tick;
            self.reconcile(ctx, world, &snapshot);
        }

        // Nach Takt sortiert einfügen (UDP kann die Reihenfolge vertauschen).
        let index = self.snapshots.iter().position(|s| s.tick > snapshot.tick).unwrap_or(self.snapshots.len());
        if self.snapshots.get(index.wrapping_sub(1)).is_none_or(|s| s.tick != snapshot.tick) {
            self.snapshots.insert(index, snapshot);
        }
        while self.snapshots.len() > 32 {
            self.snapshots.pop_front();
        }
    }

    /// Vergleicht die eigene vorhergesagte Position mit dem Server und korrigiert bei Bedarf.
    fn reconcile(&mut self, ctx: &mut Context, world: &mut World, snapshot: &Snapshot) {
        let Some(local) = self.local_id else { return };
        let Some(state) = snapshot.players.iter().find(|p| p.id == local) else { return };
        let Some(avatar) = world.players.get(&local) else { return };
        let character = avatar.character;

        let predicted = self.pending.iter().find(|p| p.input.seq == state.last_input).map(|p| p.predicted);
        while self.pending.front().is_some_and(|p| p.input.seq <= state.last_input) {
            self.pending.pop_front();
        }
        let error = predicted.map_or(f32::INFINITY, |p| p.distance(state.position));
        if error <= CORRECTION_THRESHOLD {
            return;
        }

        if predicted.is_some() {
            self.corrections += 1;
            log::debug!("Korrektur um {error:.3} m, {} Eingaben nachgespielt", self.pending.len());
        }
        ctx.physics.set_character_state(
            character,
            state.position,
            CharacterState { velocity: state.velocity, grounded: state.grounded },
        );
        for pending in &mut self.pending {
            world.apply_input(ctx, local, &pending.input);
            pending.predicted = ctx.physics.character_position(character);
        }
    }

    fn predict(&mut self, ctx: &mut Context, world: &mut World, mut input: PlayerInput) {
        let Some(local) = self.local_id else { return };
        let Some(avatar) = world.players.get(&local) else { return };
        let character = avatar.character;

        input.seq = self.next_seq;
        self.next_seq += 1;
        world.apply_input(ctx, local, &input);
        self.pending.push_back(PendingInput { input, predicted: ctx.physics.character_position(character) });
        // Ohne Antwort vom Server nicht endlos sammeln.
        while self.pending.len() > 120 {
            self.pending.pop_front();
        }

        let recent: Vec<PlayerInput> =
            self.pending.iter().rev().take(REDUNDANT_INPUTS).rev().map(|p| p.input).collect();
        self.net.send(Channel::Unreliable, encode(&ClientMessage::Inputs(recent)));
    }

    /// Setzt Mitspieler und Objekte auf den Stand von vor `INTERPOLATION_DELAY` Takten.
    fn interpolate(&mut self, ctx: &mut Context, world: &mut World) {
        let Some(now) = self.server_tick else { return };
        let render_tick = now - INTERPOLATION_DELAY;

        let Some(from) = self.snapshots.iter().rev().find(|s| s.tick as f64 <= render_tick).or(self.snapshots.front()) else {
            return;
        };
        let to = self.snapshots.iter().find(|s| s.tick as f64 > render_tick).unwrap_or(from);
        let t = if to.tick > from.tick {
            ((render_tick - from.tick as f64) / (to.tick - from.tick) as f64).clamp(0.0, 1.0) as f32
        } else {
            1.0
        };

        for target in &to.players {
            if Some(target.id) == self.local_id {
                continue;
            }
            let Some(avatar) = world.players.get_mut(&target.id) else { continue };
            let (position, facing) = match from.players.iter().find(|p| p.id == target.id) {
                Some(start) => (start.position.lerp(target.position, t), lerp_angle(start.facing, target.facing, t)),
                None => (target.position, target.facing),
            };
            ctx.physics.move_character_to(avatar.character, position);
            avatar.facing = facing;
        }

        // Objekte, die nur im älteren Snapshot vorkommen, sind inzwischen zur Ruhe gekommen.
        for start in &from.objects {
            if to.objects.iter().all(|o| o.id != start.id) {
                if let Some(object) = world.objects.get(&start.id) {
                    ctx.physics.set_body_pose(object.body, start.position, start.rotation);
                }
            }
        }
        for target in &to.objects {
            let Some(object) = world.objects.get(&target.id) else { continue };
            let (position, rotation) = match from.objects.iter().find(|o| o.id == target.id) {
                Some(start) => (start.position.lerp(target.position, t), start.rotation.slerp(target.rotation, t)),
                None => (target.position, target.rotation),
            };
            ctx.physics.set_body_pose(object.body, position, rotation);
        }
    }
}

fn lerp_angle(from: f32, to: f32, t: f32) -> f32 {
    let diff = (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    from + diff * t
}
