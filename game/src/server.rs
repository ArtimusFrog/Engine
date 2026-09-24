//! Die Seite, die das Sagen hat: rechnet die echte Physik und verteilt den Zustand.
//! Läuft beim Host, auf dem dedizierten Server und im Einzelspieler (dann ohne Netzwerk).

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;
use std::time::Duration;

use engine::prelude::*;

use crate::characters::Action;
use crate::protocol::*;
use crate::save::{player_key, WorldSave};
use crate::world::{World, FIRST_RUNTIME_ID, HARVEST_COOLDOWN_TICKS, THROW_COOLDOWN_TICKS, THROW_SPEED};

/// Alle wie viele Takte ein Snapshot rausgeht (2 = 30 pro Sekunde).
const SNAPSHOT_INTERVAL: u64 = 2;
/// Alle wie viele Takte auch ruhende Objekte mitgeschickt werden.
const FULL_SNAPSHOT_INTERVAL: u64 = 60;
/// Mehr gepufferte Eingaben pro Spieler erhöhen nur die Verzögerung.
const MAX_QUEUED_INPUTS: usize = 12;
/// Wie viele verspätete Eingaben ein Client auf einmal nachholen darf.
const MAX_INPUT_CREDIT: u32 = 8;
/// Geworfene Bälle; die ältesten verschwinden.
const MAX_THROWN: usize = 30;
/// Alle wie viele Takte der Spielstand gespeichert wird (30 Sekunden).
const SAVE_INTERVAL: u64 = 60 * 30;

struct RemoteClient {
    inputs: VecDeque<PlayerInput>,
    last_received: u32,
    last_processed: u32,
    /// Wie viele Eingaben der Client gerade verarbeiten lassen darf (+1 pro Takt).
    credit: u32,
}

pub struct Authority {
    net: Option<NetServer>,
    clients: HashMap<ClientId, RemoteClient>,
    next_object_id: NetId,
    thrown: VecDeque<NetId>,
    send_full_snapshot: bool,
    /// Wohin der Spielstand geschrieben wird (`None` = gar nicht, z. B. in Tests).
    save_path: Option<PathBuf>,
    /// Inventare aller bekannten Spieler nach Namen, auch wenn sie gerade nicht da sind.
    inventories: BTreeMap<String, Inventory>,
    /// Aktueller Server-Takt (für das Speichern beim Beenden).
    tick: u64,
}

impl Authority {
    pub fn new(net: Option<NetServer>, save_path: Option<PathBuf>) -> Self {
        Authority {
            net,
            clients: HashMap::new(),
            next_object_id: FIRST_RUNTIME_ID,
            thrown: VecDeque::new(),
            send_full_snapshot: true,
            save_path,
            inventories: BTreeMap::new(),
            tick: 0,
        }
    }

    pub fn port(&self) -> Option<u16> {
        self.net.as_ref().map(NetServer::port)
    }

    pub fn player_count(&self) -> usize {
        self.clients.len()
    }

    /// Ein Takt. `local_input` ist die Eingabe des Hosts (fehlt beim dedizierten Server).
    pub fn tick(&mut self, ctx: &mut Context, world: &mut World, local_input: Option<PlayerInput>) {
        self.tick = ctx.time.tick;
        if self.net.is_some() {
            if ctx.time.tick % SNAPSHOT_INTERVAL == 0 {
                self.send_snapshot(ctx, world);
            }
            self.receive(ctx, world);
        }

        let ids: Vec<ClientId> = self.clients.keys().copied().collect();
        for id in ids {
            // Jede Eingabe wird genau einmal abgespielt – dann rechnet der Server exakt
            // dasselbe wie die Vorhersage des Clients. Kommen Pakete gebündelt an, holt er
            // mehrere pro Takt nach; das Guthaben verhindert, dass ein manipulierter Client
            // durch zusätzliche Eingaben schneller läuft.
            let client = self.clients.get_mut(&id).expect("Client fehlt");
            client.credit = (client.credit + 1).min(MAX_INPUT_CREDIT);
            let mut batch = Vec::new();
            while client.credit > 0 {
                let Some(input) = client.inputs.pop_front() else { break };
                client.credit -= 1;
                client.last_processed = input.seq;
                batch.push(input);
            }
            for input in batch {
                self.play_input(ctx, world, id, &input);
            }
        }
        if let Some(input) = local_input {
            self.play_input(ctx, world, HOST_PLAYER, &input);
        }

        world.day.advance(Physics::FIXED_DT);
        world.think_animals(ctx);
        if ctx.time.tick % SAVE_INTERVAL == SAVE_INTERVAL - 1 {
            self.save(world);
        }

        // Nachwachsen: abgebaute Rohstoffe kommen nach einer Weile zurück.
        let regrown: Vec<u32> = world
            .resources
            .iter()
            .filter(|(_, r)| r.regrows_at.is_some_and(|t| ctx.time.tick >= t))
            .map(|(&id, _)| id)
            .collect();
        for id in regrown {
            world.resource_back(ctx, id);
            self.broadcast(ServerMessage::ResourceBack { id });
        }

        // Wer vom Rand fällt, fängt am Startpunkt neu an; wer durch den Boden rutscht,
        // wird wieder auf die Oberfläche gesetzt.
        for avatar in world.players.values() {
            let position = ctx.physics.character_position(avatar.character);
            let ground = world.terrain.height_at(position.x, position.z);
            if position.y < -30.0 {
                ctx.physics.teleport_character(avatar.character, world.spawn);
            } else if position.y < ground - 2.0 {
                ctx.physics.teleport_character(avatar.character, vec3(position.x, ground + 1.2, position.z));
            }
        }

        if let Some(net) = &mut self.net {
            net.flush();
        }
    }

    fn play_input(&mut self, ctx: &mut Context, world: &mut World, player: PlayerId, input: &PlayerInput) {
        world.apply_input(ctx, player, input);
        if let Some(id) = input.harvest {
            self.harvest(ctx, world, player, id);
        }
        if let Some(aim) = input.throw {
            self.throw_ball(ctx, world, player, aim);
        }
    }

    /// Ein Schlag auf einen Rohstoff. Der Server prüft Reichweite und Tempo selbst,
    /// damit niemand aus der Ferne oder zu schnell abbauen kann.
    fn harvest(&mut self, ctx: &mut Context, world: &mut World, player: PlayerId, id: u32) {
        let Some(avatar) = world.players.get(&player) else { return };
        if ctx.time.tick < avatar.last_harvest_tick + HARVEST_COOLDOWN_TICKS || !world.in_reach(ctx, player, id, 1.0) {
            return;
        }
        if let Some(avatar) = world.players.get_mut(&player) {
            avatar.last_harvest_tick = ctx.time.tick;
        }
        let Some(resource) = world.resources.get(&id) else { return };
        let kind = resource.spec.kind;
        let health = resource.health.saturating_sub(1);

        world.resource_hit(ctx, id, health, true);
        world.play_action(player, Action::Chop);
        let by = player;
        self.broadcast(if health == 0 { ServerMessage::ResourceGone { id, by } } else { ServerMessage::ResourceHit { id, health, by } });

        // Jeder Schlag bringt etwas, der letzte einen Bonus.
        let inventory = world.inventories.entry(player).or_default();
        inventory.add(kind, if health == 0 { 4 } else { 1 });
        let inventory = *inventory;
        if player != HOST_PLAYER {
            if let Some(net) = &mut self.net {
                net.send(player, Channel::Reliable, encode(&ServerMessage::Inventory(inventory)));
            }
        }
    }

    fn throw_ball(&mut self, ctx: &mut Context, world: &mut World, player: PlayerId, aim: Vec3) {
        let Some(avatar) = world.players.get_mut(&player) else { return };
        if ctx.time.tick < avatar.last_throw_tick + THROW_COOLDOWN_TICKS {
            return;
        }
        avatar.last_throw_tick = ctx.time.tick;
        let aim = aim.normalize_or(Vec3::NEG_Z);
        let position = ctx.physics.character_position(avatar.character) + Vec3::Y * 0.5 + aim * 0.9;
        let velocity = aim * THROW_SPEED + Vec3::Y * 2.0;

        let id = self.next_object_id;
        self.next_object_id += 1;
        world.spawn_object(ctx, id, ObjectKind::Ball, position, velocity);
        world.play_action(player, Action::Throw);
        self.broadcast(ServerMessage::Spawn { id, kind: ObjectKind::Ball, position, velocity, by: Some(player) });

        self.thrown.push_back(id);
        if self.thrown.len() > MAX_THROWN {
            let old = self.thrown.pop_front().expect("Liste ist nicht leer");
            world.remove_object(ctx, old);
            self.broadcast(ServerMessage::Despawn { id: old });
        }
    }

    fn receive(&mut self, ctx: &mut Context, world: &mut World) {
        let Some(net) = &mut self.net else { return };
        let dt = Duration::from_secs_f32(Physics::FIXED_DT);
        // Erst nach dem Empfangen erledigen (solange `net` ausgeliehen ist, geht es nicht).
        let mut deferred_welcome = Vec::new();
        let mut left = false;

        for event in net.receive(dt) {
            match event {
                ServerEvent::Connected(id) => {
                    let spawn = world.spawn + vec3((id % 5) as f32 - 2.0, 0.0, 0.0);
                    let hello = Hello::parse(&net.hello(id));
                    let (name, class) = (hello.name, hello.class);
                    world.spawn_player(ctx, id, &name, class, spawn);
                    let returning = self.inventories.contains_key(&player_key(&name));
                    // Neuer Spieler: begrüßen und über alles informieren, was schon da ist.
                    let mut intro = vec![ServerMessage::Welcome { player_id: id, tick: ctx.time.tick as u32 }];
                    intro.extend(
                        world.players.iter().filter(|&(&p, _)| p != id).map(|(&p, a)| ServerMessage::PlayerJoined { player_id: p, name: a.name.clone(), class: a.class }),
                    );
                    for (&object_id, object) in &world.objects {
                        if let Some((position, _)) = ctx.physics.body_pose(object.body) {
                            intro.push(ServerMessage::Spawn { id: object_id, kind: object.kind, position, velocity: Vec3::ZERO, by: None });
                        }
                    }
                    let gone = world.resources.iter().filter(|(_, r)| !r.is_present()).map(|(&id, _)| id).collect();
                    let damaged = world
                        .resources
                        .iter()
                        .filter(|(_, r)| r.is_present() && r.health < r.spec.max_health)
                        .map(|(&id, r)| (id, r.health))
                        .collect();
                    intro.push(ServerMessage::ResourceStates { gone, damaged });
                    for message in intro {
                        net.send(id, Channel::Reliable, encode(&message));
                    }
                    for &other in self.clients.keys() {
                        net.send(other, Channel::Reliable, encode(&ServerMessage::PlayerJoined { player_id: id, name: name.clone(), class }));
                    }
                    self.clients.insert(
                        id,
                        RemoteClient { inputs: VecDeque::new(), last_received: 0, last_processed: 0, credit: 0 },
                    );
                    if returning {
                        deferred_welcome.push(id);
                    }
                    self.send_full_snapshot = true;
                }
                ServerEvent::Disconnected(id, reason) => {
                    log::info!("Verbindung zu {id} getrennt: {reason}");
                    self.clients.remove(&id);
                    if let (Some(avatar), Some(inventory)) = (world.players.get(&id), world.inventories.get(&id)) {
                        self.inventories.insert(player_key(&avatar.name), *inventory);
                    }
                    world.remove_player(ctx, id);
                    left = true;
                    net.broadcast(Channel::Reliable, encode(&ServerMessage::PlayerLeft { player_id: id }));
                }
            }
        }

        for (&id, client) in &mut self.clients {
            while let Some(bytes) = net.message(id, Channel::Unreliable) {
                let Some(ClientMessage::Inputs(inputs)) = decode(&bytes) else { continue };
                for input in inputs {
                    if input.seq > client.last_received {
                        client.last_received = input.seq;
                        client.inputs.push_back(input);
                    }
                }
            }
            while client.inputs.len() > MAX_QUEUED_INPUTS {
                client.inputs.pop_front();
            }
        }

        for id in deferred_welcome {
            self.welcome_back(world, id);
        }
        if left {
            self.save(world);
        }
    }

    // ---------- Spielstand ----------

    /// Lädt den gespeicherten Stand in die frisch gebaute Welt (Tageszeit, Rohstoffe, Inventare).
    pub fn restore(&mut self, ctx: &mut Context, world: &mut World) {
        let Some(save) = self.save_path.as_deref().and_then(WorldSave::load) else { return };
        world.day.hour = save.hour;
        world.day.day = save.day;
        for &(id, health) in &save.damaged {
            world.resource_hit(ctx, id, health, false);
        }
        for &(id, remaining) in &save.gone {
            world.resource_hit(ctx, id, 0, false);
            if let Some(resource) = world.resources.get_mut(&id) {
                resource.regrows_at = Some(ctx.time.tick + remaining);
            }
        }
        log::info!("Spielstand geladen: Tag {}, {} Spieler bekannt, {} Rohstoffe abgebaut", save.day, save.inventories.len(), save.gone.len());
        self.inventories = save.inventories;
    }

    /// Gibt einem (wieder)kommenden Spieler sein altes Inventar zurück.
    pub fn welcome_back(&mut self, world: &mut World, player: PlayerId) {
        let Some(avatar) = world.players.get(&player) else { return };
        let Some(&inventory) = self.inventories.get(&player_key(&avatar.name)) else { return };
        world.inventories.insert(player, inventory);
        if player != HOST_PLAYER {
            if let Some(net) = &mut self.net {
                net.send(player, Channel::Reliable, encode(&ServerMessage::Inventory(inventory)));
            }
        }
    }

    /// Schreibt den Spielstand (falls ein Speicherort festgelegt ist).
    pub fn save(&mut self, world: &World) {
        let Some(path) = &self.save_path else { return };
        let save = WorldSave::capture(world, self.tick, &self.inventories);
        self.inventories = save.inventories.clone();
        match save.store(path) {
            Ok(()) => log::debug!("Spielstand gespeichert: {}", path.display()),
            Err(e) => log::error!("Spielstand {} lässt sich nicht speichern: {e}", path.display()),
        }
    }

    fn send_snapshot(&mut self, ctx: &Context, world: &mut World) {
        let full = self.send_full_snapshot || ctx.time.tick % FULL_SNAPSHOT_INTERVAL == 0;
        self.send_full_snapshot = false;

        let players = world
            .players
            .iter()
            .map(|(&id, avatar)| {
                let state = ctx.physics.character(avatar.character);
                PlayerState {
                    id,
                    position: ctx.physics.character_position(avatar.character),
                    velocity: state.velocity,
                    grounded: state.grounded,
                    facing: avatar.facing,
                    last_input: self.clients.get(&id).map_or(0, |c| c.last_processed),
                }
            })
            .collect();
        let objects = world
            .objects
            .iter()
            .filter(|(_, o)| full || !ctx.physics.is_sleeping(o.body))
            .filter_map(|(&id, o)| {
                let (position, rotation) = ctx.physics.body_pose(o.body)?;
                Some(ObjectState { id, position, rotation })
            })
            .collect();

        let animals = world
            .animals
            .iter_mut()
            .enumerate()
            .filter(|(_, a)| full || a.moved)
            .map(|(id, a)| {
                a.moved = false;
                a.state(id as u16)
            })
            .collect();

        let snapshot =
            ServerMessage::Snapshot(Snapshot { tick: ctx.time.tick as u32, players, objects, hour: world.day.hour, day: world.day.day, animals });
        if let Some(net) = &mut self.net {
            net.broadcast(Channel::Unreliable, encode(&snapshot));
        }
    }

    fn broadcast(&mut self, message: ServerMessage) {
        if let Some(net) = &mut self.net {
            net.broadcast(Channel::Reliable, encode(&message));
        }
    }
}
