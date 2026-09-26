//! Die Seite, die das Sagen hat: rechnet die echte Physik und verteilt den Zustand.
//! Läuft beim Host, auf dem dedizierten Server und im Einzelspieler (dann ohne Netzwerk).

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;
use std::time::Duration;

use engine::prelude::*;

use crate::bauten::{self, Building, BuildingKind, PRODUCTION_SECONDS};
use crate::characters::Action;
use crate::protocol::*;
use crate::save::{player_key, WorldSave};
use crate::world::{World, BOLT_SPEED, CAST_COOLDOWN_TICKS, CAST_DELAY, CAST_RANGE, HARVEST_COOLDOWN_TICKS, MINE_COOLDOWN_TICKS};

/// Alle wie viele Takte ein Snapshot rausgeht (2 = 30 pro Sekunde).
const SNAPSHOT_INTERVAL: u64 = 2;
/// Alle wie viele Takte auch ruhende Objekte mitgeschickt werden.
const FULL_SNAPSHOT_INTERVAL: u64 = 60;
/// Mehr gepufferte Eingaben pro Spieler erhöhen nur die Verzögerung.
const MAX_QUEUED_INPUTS: usize = 12;
/// Wie viele verspätete Eingaben ein Client auf einmal nachholen darf.
const MAX_INPUT_CREDIT: u32 = 8;

/// Alle wie viele Takte der Spielstand gespeichert wird (30 Sekunden).
const SAVE_INTERVAL: u64 = 60 * 30;

struct RemoteClient {
    inputs: VecDeque<PlayerInput>,
    last_received: u32,
    last_processed: u32,
    /// Wie viele Eingaben der Client gerade verarbeiten lassen darf (+1 pro Takt).
    credit: u32,
}

/// Ein Zauber ist unterwegs und trifft in Takt `due` das Tier `animal`.
struct PendingHit {
    due: u64,
    /// Tier oder (bei `enemy`) Einheit der Festung
    animal: u16,
    enemy: bool,
    by: PlayerId,
    from: Vec3,
}

pub struct Authority {
    net: Option<NetServer>,
    clients: HashMap<ClientId, RemoteClient>,
    pending_hits: Vec<PendingHit>,
    send_full_snapshot: bool,
    /// Wohin der Spielstand geschrieben wird (`None` = gar nicht, z. B. in Tests).
    save_path: Option<PathBuf>,
    /// Inventare aller bekannten Spieler nach Namen, auch wenn sie gerade nicht da sind.
    inventories: BTreeMap<String, Inventory>,
    /// Aktueller Server-Takt (für das Speichern beim Beenden).
    tick: u64,
    /// Wann jeder Spieler zuletzt geschrieben hat (Takte), gegen Überfluten des Chats.
    chat_times: HashMap<PlayerId, Vec<u64>>,
    /// Angriffe der Truppen seit dem letzten Schnappschuss
    strikes: Vec<(crate::heer::EnemyKind, Vec3, Vec3)>,
}

impl Authority {
    pub fn new(net: Option<NetServer>, save_path: Option<PathBuf>) -> Self {
        Authority {
            net,
            clients: HashMap::new(),
            pending_hits: Vec::new(),
            send_full_snapshot: true,
            save_path,
            inventories: BTreeMap::new(),
            tick: 0,
            chat_times: HashMap::new(),
            strikes: Vec::new(),
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
        world.advance_buildings(Physics::FIXED_DT);
        self.produce(world);
        world.think_animals(ctx);
        // Truppen der Festung: marschieren, kämpfen gegen Spieler in der Nähe (nicht gegen Flieger)
        let players: Vec<Vec3> = world.players.values().filter(|a| !a.noclip).map(|a| ctx.physics.character_position(a.character)).collect();
        let strikes = world.heer.tick(Physics::FIXED_DT, &players);
        world.feinde = world.heer.states();
        for strike in strikes {
            let entry = (strike.kind, strike.from, strike.target);
            self.strikes.push(entry);
            world.strikes.push(entry);
        }
        self.land_hits(ctx, world);
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
        for avatar in world.players.values().filter(|a| !a.noclip) {
            let position = ctx.physics.character_position(avatar.character);
            let ground = world.terrain.height_at(position.x, position.z);
            if position.y < -30.0 {
                ctx.physics.teleport_character(avatar.character, world.spawn);
            } else if position.y < ground - 2.0 {
                ctx.physics.teleport_character(avatar.character, vec3(position.x, ground + 1.2, position.z));
            }
        }

        if self.net.is_none() {
            self.strikes.clear();
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
        if let Some(target) = input.cast {
            self.cast(ctx, world, player, target);
        }
    }

    /// Ein Schlag auf einen Rohstoff. Der Server prüft Reichweite, Tempo und Werkzeug selbst,
    /// damit niemand aus der Ferne, zu schnell oder ohne Spitzhacke abbauen kann.
    fn harvest(&mut self, ctx: &mut Context, world: &mut World, player: PlayerId, id: u32) {
        let Some(avatar) = world.players.get(&player) else { return };
        let Some(kind) = world.resources.get(&id).map(|r| r.spec.kind) else { return };
        let mining = kind.needs_pickaxe();
        let cooldown = if mining { MINE_COOLDOWN_TICKS } else { HARVEST_COOLDOWN_TICKS };
        if avatar.tool != kind.tool() || ctx.time.tick < avatar.last_harvest_tick + cooldown || !world.in_reach(ctx, player, id, 1.0) {
            return;
        }
        if let Some(avatar) = world.players.get_mut(&player) {
            avatar.last_harvest_tick = ctx.time.tick;
        }
        let Some(resource) = world.resources.get(&id) else { return };
        let kind = resource.spec.kind;
        let health = resource.health.saturating_sub(1);

        world.resource_hit(ctx, id, health, true);
        world.play_action(player, if mining { Action::Mine } else { Action::Chop });
        let by = player;
        self.broadcast(if health == 0 { ServerMessage::ResourceGone { id, by } } else { ServerMessage::ResourceHit { id, health, by } });

        // Jeder Schlag bringt etwas, der letzte einen Bonus.
        let inventory = world.inventories.entry(player).or_default();
        inventory.add(kind, if health == 0 { 4 } else { 1 });
        let inventory = *inventory;
        self.send_inventory(player, inventory);
    }

    /// Ein Zauber Richtung `target`. Der Server rechnet selbst nach, was getroffen wird;
    /// der Client liefert nur die Richtung (und höchstens `CAST_RANGE` weit).
    fn cast(&mut self, ctx: &mut Context, world: &mut World, player: PlayerId, target: Vec3) {
        let Some(avatar) = world.players.get_mut(&player) else { return };
        if avatar.tool != Tool::Staff || ctx.time.tick < avatar.last_cast_tick + CAST_COOLDOWN_TICKS || !target.is_finite() {
            return;
        }
        avatar.last_cast_tick = ctx.time.tick;
        let Some(origin) = world.cast_origin(ctx, player, target) else { return };
        let direction = (target - origin).normalize_or(Vec3::NEG_Z);
        let range = origin.distance(target).min(CAST_RANGE) + 0.5;
        let (mut point, animal) = world.spell_target(ctx, origin, direction, range, Some(player));
        // Einheiten der Festung haben keine Kollision: eigener Strahltest
        let mut target = animal.map(|a| (a, false));
        if let Some((enemy, distance)) = world.heer.ray_hit(origin, direction, origin.distance(point)) {
            point = origin + direction * distance;
            target = Some((enemy, true));
        }

        world.cast_spell(ctx, player, origin, point, target.is_some(), true);
        self.broadcast(ServerMessage::SpellCast { by: player, origin, target: point, hit: target.is_some() });
        if let Some((animal, enemy)) = target {
            let flight = CAST_DELAY + origin.distance(point) / BOLT_SPEED;
            let due = ctx.time.tick + (flight / Physics::FIXED_DT).round() as u64;
            self.pending_hits.push(PendingHit { due, animal, enemy, by: player, from: origin });
        }
    }

    /// Zauber, die jetzt ankommen: Schaden, bei erlegten Tieren Beute für den Zaubernden.
    fn land_hits(&mut self, ctx: &mut Context, world: &mut World) {
        let tick = ctx.time.tick;
        let (landed, waiting): (Vec<_>, Vec<_>) = self.pending_hits.drain(..).partition(|h| h.due <= tick);
        self.pending_hits = waiting;
        for hit in landed {
            if hit.enemy {
                world.heer.hit(hit.animal);
                continue;
            }
            let Some(animal) = world.animals.get_mut(hit.animal as usize) else { continue };
            if !animal.is_alive() {
                continue;
            }
            let kind = animal.kind;
            let health = animal.hit(hit.from);
            world.animal_hit(ctx, hit.animal, health, true);
            self.broadcast(ServerMessage::AnimalHit { id: hit.animal, health, by: hit.by });
            if health == 0 {
                let inventory = world.inventories.entry(hit.by).or_default();
                for &(item, amount) in kind.loot() {
                    inventory.add_item(item, amount);
                }
                let inventory = *inventory;
                self.send_inventory(hit.by, inventory);
            }
        }
    }

    // ---------- Gebäude ----------

    /// Ein Spieler möchte ein Gebäude errichten: Platz und Kosten prüfen, dann bauen lassen.
    pub fn build(&mut self, ctx: &mut Context, world: &mut World, player: PlayerId, kind: BuildingKind, at: Vec2, yaw: f32) -> Result<(), String> {
        let Some(avatar) = world.players.get(&player) else { return Err("Unbekannter Spieler".into()) };
        let (owner, name) = (player_key(&avatar.name), avatar.name.clone());
        let builder = ctx.physics.character_position(avatar.character);
        if !at.is_finite() {
            return Err("Ungültiger Bauplatz".into());
        }
        let ground = bauten::check_site(world, kind, at, Some(builder)).map_err(str::to_string)?;
        let inventory = world.inventories.entry(player).or_default();
        if !bauten::affordable(inventory, kind) {
            return Err(format!("Nicht genug Rohstoffe für {}", kind.with_article()));
        }
        for &(item, amount) in kind.cost() {
            inventory.remove_item(item, amount);
        }
        let inventory = *inventory;
        let id = world.buildings.iter().map(|b| b.id + 1).max().unwrap_or(1);
        let yaw = if yaw.is_finite() { yaw.rem_euclid(std::f32::consts::TAU) } else { 0.0 };
        let building = Building { id, kind, position: vec3(at.x, ground, at.y), yaw, progress: 0.0, owner, produce_in: PRODUCTION_SECONDS };
        log::info!("{name} baut {} bei ({:.0}, {:.0})", kind.with_article(), at.x, at.y);
        world.place_building(ctx, building.clone());
        world.chat_events.push(crate::world::ChatLine::notice(format!("{name} baut {}", kind.with_article())));
        self.broadcast(ServerMessage::BuildingPlaced(building));
        self.send_inventory(player, inventory);
        self.save(world);
        Ok(())
    }

    /// Befehl aus dem Admin-Panel (Wetter, Truppen der Festung).
    pub fn admin(&mut self, world: &mut World, command: AdminCommand) {
        log::info!("Admin: {command:?}");
        match command {
            AdminCommand::Weather(choice) => world.set_weather(choice.min(WETTER.len() as u8 - 1)),
            AdminCommand::Waves(on) => world.heer.set_enabled(on),
            AdminCommand::WaveNow => world.heer.spawn_wave(),
            AdminCommand::ClearEnemies => world.heer.clear(),
        }
    }

    /// Fertige Gebäude liefern ihrem Erbauer regelmäßig Rohstoffe – auch wenn er gerade nicht da ist.
    fn produce(&mut self, world: &mut World) {
        let mut deliveries = Vec::new();
        for building in world.buildings.iter_mut().filter(|b| b.finished()) {
            building.produce_in -= Physics::FIXED_DT;
            if building.produce_in <= 0.0 {
                building.produce_in += PRODUCTION_SECONDS;
                deliveries.push((building.owner.clone(), building.kind.produces()));
            }
        }
        for (owner, item) in deliveries {
            let online = world.players.iter().find(|(_, a)| player_key(&a.name) == owner).map(|(&id, _)| id);
            match online {
                Some(id) => {
                    let inventory = world.inventories.entry(id).or_default();
                    inventory.add_item(item, 1);
                    let inventory = *inventory;
                    self.send_inventory(id, inventory);
                }
                None => self.inventories.entry(owner).or_default().add_item(item, 1),
            }
        }
    }

    fn send_inventory(&mut self, player: PlayerId, inventory: Inventory) {
        if player != HOST_PLAYER {
            if let Some(net) = &mut self.net {
                net.send(player, Channel::Reliable, encode(&ServerMessage::Inventory(inventory)));
            }
        }
    }

    /// Eine Chatnachricht: prüfen, an alle verteilen und selbst anzeigen. Höchstens fünf
    /// Nachrichten in fünf Sekunden je Spieler.
    pub fn chat(&mut self, ctx: &Context, world: &mut World, from: PlayerId, text: &str) {
        let Some(text) = clean_chat(text) else { return };
        let Some(name) = world.players.get(&from).map(|a| a.name.clone()) else { return };
        let now = ctx.time.tick;
        let times = self.chat_times.entry(from).or_default();
        times.retain(|&t| now.saturating_sub(t) < 60 * 5);
        if times.len() >= 5 {
            return;
        }
        times.push(now);
        log::info!("Chat {name}: {text}");
        world.chat_events.push(crate::world::ChatLine { from: Some(from), name: name.clone(), text: text.clone() });
        self.broadcast(ServerMessage::Chat { from, name, text });
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
                    world.chat_events.push(crate::world::ChatLine::notice(format!("{name} ist beigetreten")));
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
                    intro.push(ServerMessage::Buildings(world.buildings.clone()));
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
                    if let Some(avatar) = world.players.get(&id) {
                        world.chat_events.push(crate::world::ChatLine::notice(format!("{} hat das Spiel verlassen", avatar.name)));
                    }
                    self.chat_times.remove(&id);
                    world.remove_player(ctx, id);
                    left = true;
                    net.broadcast(Channel::Reliable, encode(&ServerMessage::PlayerLeft { player_id: id }));
                }
            }
        }

        let mut chats = Vec::new();
        let mut builds = Vec::new();
        let mut admins = Vec::new();
        for (&id, client) in &mut self.clients {
            while let Some(bytes) = net.message(id, Channel::Reliable) {
                match decode(&bytes) {
                    Some(ClientMessage::Chat(text)) => chats.push((id, text)),
                    Some(ClientMessage::Build { kind, at, yaw }) => builds.push((id, kind, at, yaw)),
                    Some(ClientMessage::Admin(command)) => admins.push(command),
                    _ => {}
                }
            }
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
        for (id, text) in chats {
            self.chat(ctx, world, id, &text);
        }
        for command in admins {
            self.admin(world, command);
        }
        for (id, kind, at, yaw) in builds {
            if let Err(reason) = self.build(ctx, world, id, kind, at, yaw) {
                if let Some(net) = &mut self.net {
                    net.send(id, Channel::Reliable, encode(&ServerMessage::BuildRefused(reason)));
                }
            }
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
        for building in save.buildings.iter().cloned() {
            world.place_building(ctx, building);
        }
        log::info!(
            "Spielstand geladen: Tag {}, {} Spieler bekannt, {} Rohstoffe abgebaut, {} Gebäude",
            save.day,
            save.inventories.len(),
            save.gone.len(),
            save.buildings.len()
        );
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
                    tool: avatar.tool,
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

        let snapshot = ServerMessage::Snapshot(Snapshot {
            tick: ctx.time.tick as u32,
            players,
            objects,
            hour: world.day.hour,
            day: world.day.day,
            animals,
            enemies: world.feinde.clone(),
            strikes: std::mem::take(&mut self.strikes),
            weather: world.weather_choice,
            waves: world.heer.enabled,
        });
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
