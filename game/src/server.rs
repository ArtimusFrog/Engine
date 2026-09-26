//! Die Seite, die das Sagen hat: rechnet die echte Physik und verteilt den Zustand.
//! Läuft beim Host, auf dem dedizierten Server und im Einzelspieler (dann ohne Netzwerk).

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::path::PathBuf;
use std::time::Duration;

use engine::prelude::*;

use crate::bauten::{self, Building, BuildingKind, PRODUCTION_SECONDS};
use crate::characters::Action;
use crate::protocol::*;
use crate::heer::{Blocker, Quelle, Ziel};
use crate::save::{player_key, WorldSave};
use crate::td::{Ereignis, TdBefehl, TdStand};
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
    /// Türme im Kampf und ihre Schüsse seit dem letzten Schnappschuss
    verteidigung: crate::tuerme::Verteidigung,
    shots: Vec<crate::tuerme::Schuss>,
    /// Angriffe der Truppen aus dem letzten Takt der Verteidigung
    neue_strikes: Vec<crate::heer::Strike>,
    /// Ereignisse der Verteidigung seit dem letzten Schnappschuss
    ereignisse: Vec<Ereignis>,
    /// Wer welche Heerstraße verteidigt (Name je Straße)
    strassen_spieler: Vec<Option<String>>,
    /// Wer sein Startgold schon bekommen hat (nach Namen)
    startgold: BTreeSet<String>,
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
            verteidigung: Default::default(),
            shots: Vec::new(),
            neue_strikes: Vec::new(),
            ereignisse: Vec::new(),
            strassen_spieler: Vec::new(),
            startgold: BTreeSet::new(),
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
        self.verteidigen(ctx, world);
        for strike in std::mem::take(&mut self.neue_strikes) {
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
                ctx.physics.teleport_character(avatar.character, world.startpunkt(&avatar.name));
            } else if position.y < ground - 2.0 {
                ctx.physics.teleport_character(avatar.character, vec3(position.x, ground + 1.2, position.z));
            }
        }

        if self.net.is_none() {
            self.strikes.clear();
            self.shots.clear();
            self.ereignisse.clear();
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
                let name = world.players.get(&hit.by).map(|a| player_key(&a.name)).unwrap_or_default();
                let zauber = crate::heer::Hit { schaden: 20.0, art: crate::tuerme::DamageKind::Arcane, ..Default::default() };
                world.heer.damage(hit.animal, zauber, Quelle { name: &name, turm: None });
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
        // Fallen liegen immer mitten auf der Straße, quer zu ihr
        let (at, yaw) = match kind.falle().and_then(|_| bauten::falle_platz(world, at)) {
            Some(platz) => platz,
            None => (at, yaw),
        };
        let ground = bauten::check_site(world, kind, at, Some(builder)).map_err(str::to_string)?;
        bauten::siedlung_pruefen(world, kind, at, &owner).map_err(str::to_string)?;
        let inventory = world.inventories.entry(player).or_default();
        if !bauten::affordable(inventory, kind) {
            return Err(format!("Nicht genug Gold oder Rohstoffe für {}", kind.with_article()));
        }
        for (item, amount) in kind.cost() {
            inventory.remove_item(item, amount);
        }
        let inventory = *inventory;
        let id = world.buildings.iter().map(|b| b.id + 1).max().unwrap_or(1);
        let yaw = if yaw.is_finite() { yaw.rem_euclid(std::f32::consts::TAU) } else { 0.0 };
        let building = Building { id, kind, position: vec3(at.x, ground, at.y), yaw, progress: 0.0, owner, produce_in: PRODUCTION_SECONDS, level: 1, zweig: 0, ziel: Default::default() };
        log::info!("{name} baut {} bei ({:.0}, {:.0})", kind.with_article(), at.x, at.y);
        world.place_building(ctx, building.clone());
        let text = match world.siedlungsplatz_bei(at, 26.0).filter(|_| kind == BuildingKind::Dorfhalle) {
            Some(platz) => format!("{name} gründet eine Siedlung am Ende der Straße {}", world.heer.strassen_namen()[platz]),
            None => format!("{name} baut {}", kind.with_article()),
        };
        world.chat_events.push(crate::world::ChatLine::notice(text.clone()));
        if kind == BuildingKind::Dorfhalle {
            self.broadcast(ServerMessage::Notice(text));
        }
        self.broadcast(ServerMessage::BuildingPlaced(building));
        self.send_inventory(player, inventory);
        self.save(world);
        Ok(())
    }

    /// Befehl aus dem Admin-Panel (Wetter, Truppen der Festung, Tower Defense).
    pub fn admin(&mut self, world: &mut World, player: PlayerId, command: AdminCommand) {
        log::info!("Admin: {command:?}");
        match command {
            AdminCommand::Weather(choice) => world.set_weather(choice.min(WETTER.len() as u8 - 1)),
            AdminCommand::Waves(on) => world.heer.set_enabled(on),
            AdminCommand::WaveNow => world.heer.spawn_wave(),
            AdminCommand::ClearEnemies => world.heer.clear(),
            AdminCommand::ResetWaves => world.heer.reset(),
            AdminCommand::Schwierigkeit(s) => world.heer.set_schwierigkeit(s),
            AdminCommand::Endlos(an) => {
                world.heer.endlos = an;
                world.heer.meldungen.push(if an { "Endlosmodus: nach Welle 30 geht es weiter.".into() } else { "Ziel: Welle 30 überstehen.".into() });
            }
            AdminCommand::Gold(n) => {
                let inventory = world.inventories.entry(player).or_default();
                inventory.gold += n.min(100_000);
                let inventory = *inventory;
                self.send_inventory(player, inventory);
            }
            AdminCommand::SpringeZuWelle(w) => {
                world.heer.clear();
                world.heer.springe_zu_welle(w.min(200));
            }
        }
    }

    /// Ein Takt der Verteidigung: Truppen marschieren und kämpfen, Soldaten und Türme wehren sich,
    /// Beute, Kopfgeld, Auswertungen und der Stand für alle.
    fn verteidigen(&mut self, ctx: &mut Context, world: &mut World) {
        let dt = Physics::FIXED_DT;
        world.heer.spieler = world.players.len().max(1);
        // Was die Truppen aufhält: Spieler (nicht im Flug), Soldaten und Barrikaden
        let mut blocker: Vec<Blocker> = world
            .players
            .values()
            .filter(|a| !a.noclip)
            .map(|a| Blocker { ort: ctx.physics.character_position(a.character), ziel: Ziel::Spieler })
            .collect();
        blocker.extend(self.verteidigung.blocker(&world.buildings));
        let terrain = &world.terrain;
        let boden = |p: Vec2| terrain.height_at(p.x, p.y);
        let strikes = world.heer.tick(dt, &blocker, &boden);
        let shots = self.verteidigung.tick(dt, &world.buildings, &mut world.heer, &strikes, &boden);
        self.neue_strikes = strikes;
        world.tower_shots.extend(shots.iter().copied());
        self.shots.extend(shots);
        // Gefallene Straßen: die Siedlung an ihrem Ende wird zerstört (Dorfhalle und die Gebäude
        // ihres Besitzers im Bauradius)
        for lane in std::mem::take(&mut world.heer.gefallene_lanes) {
            let Some(halle) = world.dorfhalle_auf_platz(lane).cloned() else { continue };
            let radius = bauten::bauradius(halle.level);
            let weg: Vec<(u32, Vec3)> = world
                .buildings
                .iter()
                .filter(|b| b.id == halle.id || (b.owner == halle.owner && b.kind.produces().is_some() && b.position.distance(halle.position) <= radius))
                .map(|b| (b.id, b.position))
                .collect();
            for (id, ort) in &weg {
                world.remove_building(ctx, *id);
                self.broadcast(ServerMessage::BuildingRemoved(*id));
                let ereignis = crate::td::Ereignis::Explosion(*ort + Vec3::Y * 2.0, 5.0);
                world.ereignisse.push(ereignis);
                self.ereignisse.push(ereignis);
            }
            world.heer.meldungen.push(format!("Die Siedlung von {} ist zerstört ({} Gebäude).", halle.owner, weg.len()));
            self.save(world);
        }
        // Zerschlagene Barrikaden
        for id in std::mem::take(&mut self.verteidigung.zerstoert) {
            world.remove_building(ctx, id);
            self.broadcast(ServerMessage::BuildingRemoved(id));
            world.heer.meldungen.push("Eine Barrikade wurde zerschlagen!".into());
        }
        for text in std::mem::take(&mut world.heer.meldungen) {
            world.chat_events.push(crate::world::ChatLine::notice(text.clone()));
            self.broadcast(ServerMessage::Notice(text));
        }
        // Beute und Kopfgeld für besiegte Einheiten (Schatzkammern in der Nähe: mehr davon)
        let schwierigkeit = world.heer.schwierigkeit;
        let kammern: Vec<(Vec3, f32, f32)> = world
            .buildings
            .iter()
            .filter(|b| b.finished() && b.tower() == Some(crate::tuerme::TowerKind::Treasury))
            .map(|b| {
                let w = b.kind_werte();
                (b.position, w.reichweite, w.beute)
            })
            .collect();
        for g in std::mem::take(&mut world.heer.gefallen) {
            let bonus = kammern.iter().filter(|(ort, weite, _)| ort.distance(g.ort) <= *weite).map(|k| k.2).fold(0.0f32, f32::max);
            let mut beute: Vec<(Item, u32)> = crate::tuerme::beute(g.kind).iter().map(|&(item, n)| (item, (n as f32 * (1.0 + bonus)).round() as u32)).collect();
            let gold = (crate::td::kopfgeld(g.kind, g.boss, g.welle, schwierigkeit) as f32 * (1.0 + bonus + g.bonus)).round() as u32;
            beute.push((Item::Gold, gold));
            self.give(world, &g.von, &beute);
        }
        for (owner, gold) in std::mem::take(&mut self.verteidigung.gold) {
            self.give(world, &owner, &[(Item::Gold, gold)]);
        }
        // Auswertungen überstandener Wellen: Wellenbonus für alle
        for (mut bericht, bester) in std::mem::take(&mut world.heer.berichte) {
            bericht.bester_turm = bester.and_then(|(id, kills)| world.buildings.iter().find(|b| b.id == id).map(|b| (b.kind.label().to_string(), b.owner.clone(), kills)));
            bericht.gold = crate::td::wellenbonus(bericht.welle, schwierigkeit);
            let namen: Vec<String> = world.players.values().map(|a| player_key(&a.name)).collect();
            for name in namen {
                self.give(world, &name, &[(Item::Gold, bericht.gold)]);
            }
            // Jede Dorfhalle bringt ihrem Besitzer zusätzlich Gold
            let hallen: Vec<(String, u8)> = world.buildings.iter().filter(|b| b.kind == BuildingKind::Dorfhalle && b.finished()).map(|b| (b.owner.clone(), b.level)).collect();
            for (owner, level) in hallen {
                self.give(world, &owner, &[(Item::Gold, bauten::dorfhalle_gold(level))]);
            }
            let mut text = format!("Welle {} überstanden: {} besiegt, {} durchgebrochen · +{} Gold für alle", bericht.welle, bericht.besiegt, bericht.durchgebrochen, bericht.gold);
            if let Some((name, schaden)) = &bericht.bester_spieler {
                text += &format!(" · Bester: {name} ({schaden} Schaden)");
            }
            world.chat_events.push(crate::world::ChatLine::notice(text.clone()));
            self.broadcast(ServerMessage::Notice(text));
            world.berichte.push(bericht.clone());
            self.broadcast(ServerMessage::Bericht(bericht));
        }
        // Sieg: Belohnung für alle
        if std::mem::take(&mut world.heer.sieg_neu) {
            let namen: Vec<String> = world.players.values().map(|a| player_key(&a.name)).collect();
            for name in namen {
                self.give(world, &name, &[(Item::Gold, 500), (Item::Ore, 20)]);
            }
            let text = "Belohnung für alle Verteidiger: 500 Gold und 20 Eisenerz!".to_string();
            world.chat_events.push(crate::world::ChatLine::notice(text.clone()));
            self.broadcast(ServerMessage::Notice(text));
        }
        let mut ereignisse = std::mem::take(&mut world.heer.ereignisse);
        ereignisse.append(&mut self.verteidigung.ereignisse);
        world.ereignisse.extend(ereignisse.iter().copied());
        self.ereignisse.extend(ereignisse);
        world.feinde = world.heer.states();
        let stand = self.td_stand(world, ctx.time.tick % 60 == 0);
        world.td_uebernehmen(stand);
    }

    /// Stand der Verteidigung für alle (mit Statistik nur etwa einmal pro Sekunde).
    fn td_stand(&self, world: &World, mit_stats: bool) -> TdStand {
        let heer = &world.heer;
        let (vorschau, vorschau_boss) = heer.vorschau();
        TdStand {
            aktiv: heer.enabled,
            welle: heer.welle,
            leben: heer.leben.clone(),
            max_leben: heer.schwierigkeit.leben(),
            naechste: heer.naechste_in(),
            vorschau,
            vorschau_boss,
            schwierigkeit: heer.schwierigkeit,
            endlos: heer.endlos,
            sieg: heer.sieg,
            // Wer am Ende einer Straße siedelt, verteidigt sie; sonst gilt, wer sie gewählt hat
            strassen: heer
                .strassen_namen()
                .iter()
                .enumerate()
                .map(|(i, n)| (n.to_string(), world.dorfhalle_auf_platz(i).map(|h| h.owner.clone()).or_else(|| self.strassen_spieler.get(i).cloned().flatten())))
                .collect(),
            soldaten: self.verteidigung.soldaten(),
            barrikaden: self.verteidigung.barrikaden(),
            turm_stats: if mit_stats { heer.turm_stats.iter().map(|(&id, &(s, k))| (id, k, s.round() as u32)).collect() } else { Vec::new() },
            beitrag: if mit_stats { heer.beitrag.iter().map(|(n, &(s, k))| (n.clone(), s.round() as u32, k)).collect() } else { Vec::new() },
        }
    }

    /// Befehle der Spieler zur Verteidigung: aufwerten, abreißen, zielen, Welle rufen, Straße wählen.
    pub fn td(&mut self, ctx: &mut Context, world: &mut World, player: PlayerId, befehl: TdBefehl) -> Result<(), String> {
        let name = world.players.get(&player).map(|a| a.name.clone()).unwrap_or_default();
        match befehl {
            TdBefehl::Aufwerten(id, zweig) => self.upgrade(ctx, world, player, id, zweig),
            TdBefehl::Abreissen(id) => self.demolish(ctx, world, player, id),
            TdBefehl::Zielen(id, modus) => {
                let Some(building) = world.buildings.iter_mut().find(|b| b.id == id) else { return Err("Das Gebäude gibt es nicht mehr".into()) };
                if building.tower().is_none() {
                    return Err("Nur Türme zielen".into());
                }
                building.ziel = modus;
                let neu = building.clone();
                self.broadcast(ServerMessage::BuildingChanged(neu));
                Ok(())
            }
            TdBefehl::WelleRufen => {
                let Some(gespart) = world.heer.rufen() else { return Err("Gerade lässt sich keine Welle rufen".into()) };
                let gold = (gespart * 0.8).round() as u32;
                let namen: Vec<String> = world.players.values().map(|a| player_key(&a.name)).collect();
                for key in namen {
                    self.give(world, &key, &[(Item::Gold, gold)]);
                }
                let text = format!("{name} ruft Welle {} früher – +{gold} Gold für alle", world.heer.welle);
                world.chat_events.push(crate::world::ChatLine::notice(text.clone()));
                self.broadcast(ServerMessage::Notice(text));
                Ok(())
            }
            TdBefehl::Strasse(index) => {
                let key = player_key(&name);
                let namen = world.heer.strassen_namen();
                self.strassen_spieler.resize(namen.len(), None);
                for eintrag in &mut self.strassen_spieler {
                    if eintrag.as_deref() == Some(key.as_str()) {
                        *eintrag = None;
                    }
                }
                if world.dorfhalle_auf_platz(index as usize).is_some_and(|h| h.owner != key) {
                    return Err("An dieser Straße siedelt schon jemand anderes".into());
                }
                if let Some(eintrag) = self.strassen_spieler.get_mut(index as usize) {
                    *eintrag = Some(key);
                    let text = format!("{name} verteidigt jetzt die Straße {}", namen[index as usize]);
                    world.chat_events.push(crate::world::ChatLine::notice(text.clone()));
                    self.broadcast(ServerMessage::Notice(text));
                }
                Ok(())
            }
        }
    }

    /// Fertige Gebäude liefern ihrem Erbauer regelmäßig Rohstoffe – auch wenn er gerade nicht da ist.
    fn produce(&mut self, world: &mut World) {
        let mut deliveries = Vec::new();
        // Gebäude im Radius der eigenen Dorfhalle arbeiten 10 % schneller
        let hallen: Vec<(String, Vec3, f32)> = world
            .buildings
            .iter()
            .filter(|b| b.kind == BuildingKind::Dorfhalle && b.finished())
            .map(|b| (b.owner.clone(), b.position, bauten::bauradius(b.level)))
            .collect();
        for building in world.buildings.iter_mut().filter(|b| b.finished()) {
            let Some(item) = building.kind.produces() else { continue };
            let im_radius = hallen.iter().any(|(owner, ort, r)| *owner == building.owner && ort.distance(building.position) <= *r);
            building.produce_in -= Physics::FIXED_DT * if im_radius { 1.1 } else { 1.0 };
            if building.produce_in <= 0.0 {
                building.produce_in += PRODUCTION_SECONDS;
                deliveries.push((building.owner.clone(), item));
            }
        }
        for (owner, item) in deliveries {
            self.give(world, &owner, &[(item, 1)]);
        }
    }

    /// Gegenstände an einen Spieler (nach Namen) – auch wenn er gerade nicht da ist.
    fn give(&mut self, world: &mut World, owner: &str, items: &[(Item, u32)]) {
        if owner.is_empty() || items.is_empty() {
            return;
        }
        let online = world.players.iter().find(|(_, a)| player_key(&a.name) == owner).map(|(&id, _)| id);
        match online {
            Some(id) => {
                let inventory = world.inventories.entry(id).or_default();
                for &(item, n) in items {
                    inventory.add_item(item, n);
                }
                let inventory = *inventory;
                self.send_inventory(id, inventory);
            }
            None => {
                let inventory = self.inventories.entry(owner.to_string()).or_default();
                for &(item, n) in items {
                    inventory.add_item(item, n);
                }
            }
        }
    }

    /// Einen Turm um eine Stufe aufwerten (jeder darf, der die Rohstoffe hat). Auf Stufe 3 wird
    /// dabei die Richtung gewählt (`zweig` 1 = A, 2 = B).
    pub fn upgrade(&mut self, ctx: &mut Context, world: &mut World, player: PlayerId, id: u32, zweig: u8) -> Result<(), String> {
        let Some(building) = world.buildings.iter().find(|b| b.id == id).cloned() else { return Err("Das Gebäude gibt es nicht mehr".into()) };
        if building.kind == BuildingKind::Dorfhalle && world.players.get(&player).map(|a| player_key(&a.name)).as_deref() != Some(building.owner.as_str()) {
            return Err("Nur der Besitzer baut seine Dorfhalle aus".into());
        }
        if building.tower().is_none() && building.kind != BuildingKind::Dorfhalle {
            return Err("Nur Türme und die Dorfhalle lassen sich aufwerten".into());
        }
        if !building.finished() {
            return Err("Erst fertig bauen".into());
        }
        if building.level >= crate::tuerme::MAX_STUFE {
            return Err("Schon auf der höchsten Stufe".into());
        }
        if building.tower().is_some() && building.level + 1 == crate::tuerme::MAX_STUFE && !(1..=2).contains(&zweig) {
            return Err("Für Stufe 3 eine Richtung wählen".into());
        }
        let cost = building.kind.upgrade_cost(building.level + 1);
        let inventory = world.inventories.entry(player).or_default();
        if !bauten::can_pay(inventory, &cost) {
            return Err("Nicht genug Gold oder Rohstoffe zum Aufwerten".into());
        }
        for &(item, n) in &cost {
            inventory.remove_item(item, n);
        }
        let inventory = *inventory;
        let level = building.level + 1;
        let zweig = if level == crate::tuerme::MAX_STUFE && building.tower().is_some() { zweig } else { building.zweig };
        let neu = Building { level, progress: 0.0, zweig, ..building };
        if neu.kind == BuildingKind::Dorfhalle {
            let name = world.players.get(&player).map(|a| a.name.clone()).unwrap_or_default();
            let text = format!("{name} baut die Dorfhalle zum {} aus", neu.kind.stufen_name(level));
            world.chat_events.push(crate::world::ChatLine::notice(text.clone()));
            self.broadcast(ServerMessage::Notice(text));
        }
        world.replace_building(ctx, neu.clone());
        self.broadcast(ServerMessage::BuildingChanged(neu));
        self.send_inventory(player, inventory);
        self.save(world);
        Ok(())
    }

    /// Ein eigenes Gebäude abreißen: die Hälfte der Kosten kommt zurück.
    pub fn demolish(&mut self, ctx: &mut Context, world: &mut World, player: PlayerId, id: u32) -> Result<(), String> {
        let Some(building) = world.buildings.iter().find(|b| b.id == id).cloned() else { return Err("Das Gebäude gibt es nicht mehr".into()) };
        let name = world.players.get(&player).map(|a| player_key(&a.name)).unwrap_or_default();
        if building.owner != name {
            return Err("Nur wer es gebaut hat, darf es abreißen".into());
        }
        let zurueck: Vec<(Item, u32)> = building.paid().into_iter().map(|(item, n)| (item, n / 2)).filter(|&(_, n)| n > 0).collect();
        world.remove_building(ctx, id);
        self.broadcast(ServerMessage::BuildingRemoved(id));
        self.give(world, &name, &zurueck);
        self.save(world);
        Ok(())
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
                    let spawn = world.startpunkt(&name) + (spawn - world.spawn);
                    world.spawn_player(ctx, id, &name, class, spawn);
                    world.chat_events.push(crate::world::ChatLine::notice(format!("{name} ist beigetreten")));
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
                    deferred_welcome.push(id);
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
        let mut changes: Vec<(ClientId, TdBefehl)> = Vec::new();
        for (&id, client) in &mut self.clients {
            while let Some(bytes) = net.message(id, Channel::Reliable) {
                match decode(&bytes) {
                    Some(ClientMessage::Chat(text)) => chats.push((id, text)),
                    Some(ClientMessage::Build { kind, at, yaw }) => builds.push((id, kind, at, yaw)),
                    Some(ClientMessage::Admin(command)) => admins.push((id, command)),
                    Some(ClientMessage::Td(befehl)) => changes.push((id, befehl)),
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
        for (id, command) in admins {
            self.admin(world, id, command);
        }
        for (id, befehl) in changes {
            let result = self.td(ctx, world, id, befehl);
            if let (Err(reason), Some(net)) = (result, &mut self.net) {
                net.send(id, Channel::Reliable, encode(&ServerMessage::BuildRefused(reason)));
            }
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
        self.startgold = save.startgold;
    }

    /// Gibt einem (wieder)kommenden Spieler sein altes Inventar zurück; wer zum ersten Mal da ist
    /// (oder zum ersten Mal, seit es Gold gibt), bekommt Startgold.
    pub fn welcome_back(&mut self, world: &mut World, player: PlayerId) {
        let Some(avatar) = world.players.get(&player) else { return };
        let key = player_key(&avatar.name);
        if let Some(&inventory) = self.inventories.get(&key) {
            world.inventories.insert(player, inventory);
        }
        if self.startgold.insert(key) {
            world.inventories.entry(player).or_default().gold += crate::td::STARTGOLD;
        }
        let inventory = world.inventories.get(&player).copied().unwrap_or_default();
        if player != HOST_PLAYER {
            if let Some(net) = &mut self.net {
                net.send(player, Channel::Reliable, encode(&ServerMessage::Inventory(inventory)));
            }
        }
    }

    /// Schreibt den Spielstand (falls ein Speicherort festgelegt ist).
    pub fn save(&mut self, world: &World) {
        let Some(path) = &self.save_path else { return };
        let mut save = WorldSave::capture(world, self.tick, &self.inventories);
        save.startgold = self.startgold.clone();
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
            shots: std::mem::take(&mut self.shots),
            // Statistik nur etwa einmal pro Sekunde mitschicken
            td: if ctx.time.tick % 60 == 0 { self.td_stand(world, true) } else { world.td.clone() },
            ereignisse: std::mem::take(&mut self.ereignisse),
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
