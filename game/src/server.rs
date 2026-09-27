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
use crate::faehigkeiten::Faehigkeit;
use crate::wildnis::Treffer;
use crate::world::{World, HARVEST_COOLDOWN_TICKS, HEILEN_ANTEIL, HEILEN_NACH, MINE_COOLDOWN_TICKS, RUNEN_REICHWEITE};

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

/// Was ein Geschoss getroffen hat.
#[derive(Clone, Copy, Debug)]
enum Getroffen {
    Tier(u16),
    /// Einheit der Festung
    Feind(u16),
    /// Bewohner eines Lagers der Wildnis
    Wild(u16),
}

/// Wo eine Fähigkeit trifft.
#[derive(Clone, Copy, Debug)]
enum Bereich {
    /// Genau dieses Ziel (Geschoss, Wurfhammer)
    Ziel(Getroffen),
    /// Alles im Umkreis von `punkt`
    Kreis(f32),
    /// Alles im Ring um `punkt` (innen, außen) – Eiswelle, äußerer Ring des Erdbebens
    Ring(f32, f32),
    /// Alles vor der Figur (Weite, cos des halben Winkels)
    Kegel(f32, f32),
    /// Alles auf dem Strahl von `from` in `richtung` (Länge, halbe Breite) – Arkanlanze
    Linie(f32, f32),
}

/// Eine Fähigkeit wirkt in Takt `due`: Geschoss am Ziel, Hammer niedergesaust, Eiswelle angekommen.
struct PendingHit {
    due: u64,
    art: Faehigkeit,
    by: PlayerId,
    /// Wo die Figur stand bzw. das Geschoss losflog, und wohin sie zielte
    from: Vec3,
    richtung: Vec3,
    /// Einschlag (Geschoss) bzw. Mitte der Wirkung (um sich)
    punkt: Vec3,
    bereich: Bereich,
    /// Schaden und Nachwirkungen (mit der Waffe verrechnet)
    schaden: f32,
    wirkung: crate::faehigkeiten::Wirkung,
    /// Druckwelle um `punkt` für alle, die sonst nicht getroffen wurden (Radius, Anteil am Schaden)
    neben: Option<(f32, f32)>,
    /// Ein Treffer lädt eine arkane Ladung
    laedt: bool,
    /// Hinterlässt einen Flammenteppich
    flammen: bool,
    /// Waffe: Faktor auf die Nachwirkung (für den Flammenteppich)
    faktor: f32,
}

/// Flammenteppich eines Feuerballs.
struct Flammen {
    mitte: Vec3,
    bis: u64,
    by: PlayerId,
    faktor: f32,
}

pub struct Authority {
    net: Option<NetServer>,
    clients: HashMap<ClientId, RemoteClient>,
    pending_hits: Vec<PendingHit>,
    flammen: Vec<Flammen>,
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
    /// Siedlungsplätze mit eingesetztem Runenstein: wem sie gehören (je Straße)
    runen: Vec<Option<String>>,
    /// Zufall für Beute (Runenfragmente, Waffen)
    rng: Rng,
    /// Beute am Boden: nächste ID und wann sie verschwindet (Takt)
    beute_naechste: u32,
    beute_bis: HashMap<u32, u64>,
    /// Serverbrowser: beantwortet Statusanfragen (Name, Spielport, höchste Spielerzahl)
    status: Option<(crate::status::StatusAntwort, String, u16, u16)>,
}

impl Authority {
    pub fn new(net: Option<NetServer>, save_path: Option<PathBuf>) -> Self {
        Authority {
            net,
            clients: HashMap::new(),
            pending_hits: Vec::new(),
            flammen: Vec::new(),
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
            runen: Vec::new(),
            rng: Rng::new(0xB0_07E),
            beute_naechste: 1,
            beute_bis: HashMap::new(),
            status: None,
        }
    }

    /// Ab jetzt im Serverbrowser sichtbar (Statusanfragen auf `port + 1`).
    pub fn status_starten(&mut self, name: String, port: u16, max: u16) {
        self.status = crate::status::StatusAntwort::neu(port).map(|a| (a, name, port, max));
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

        if let Some((antwort, name, port, max)) = &self.status {
            antwort.beantworten(|| crate::status::Status {
                name: name.clone(),
                spieler: world.players.len() as u16,
                max: *max,
                protokoll: PROTOCOL_ID,
                port: *port,
                welle: world.heer.welle,
            });
        }
        world.day.advance(Physics::FIXED_DT);
        world.advance_buildings(Physics::FIXED_DT);
        self.produce(world);
        world.think_animals(ctx);
        self.heilen(ctx, world);
        let tick = ctx.time.tick;
        let alt: Vec<u32> = self.beute_bis.iter().filter(|&(_, &bis)| tick >= bis).map(|(&id, _)| id).collect();
        for id in alt {
            self.beute_bis.remove(&id);
            world.beute.remove(&id);
            self.broadcast(ServerMessage::BeuteWeg(id));
        }
        for (id, avatar) in world.players.iter_mut() {
            avatar.waffe = world.inventories.get(id).map_or(0, |i| i.waffe);
        }
        self.wildnis_takt(ctx, world);
        self.verteidigen(ctx, world);
        for strike in std::mem::take(&mut self.neue_strikes) {
            let entry = (strike.kind, strike.from, strike.target);
            self.strikes.push(entry);
            world.strikes.push(entry);
            // Truppen der Festung, die einen Spieler angreifen, treffen ihn auch
            if strike.ziel == Ziel::Spieler {
                let naechster = world
                    .players
                    .iter()
                    .filter(|(_, a)| !a.noclip && a.leben > 0.0)
                    .map(|(&id, a)| (id, ctx.physics.character_position(a.character).distance(strike.target)))
                    .filter(|&(_, d)| d < 4.0)
                    .min_by(|a, b| a.1.total_cmp(&b.1));
                if let Some((id, _)) = naechster {
                    self.spieler_schaden(ctx, world, id, strike.schaden * 0.5, strike.kind.label());
                }
            }
        }
        self.land_hits(ctx, world);
        self.flammen_takt(ctx, world);
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

    /// Die Fähigkeit in der Hand Richtung `target`. Der Server rechnet selbst nach, was getroffen
    /// wird; der Client liefert nur die Richtung. Abklingzeit, Reichweite und Wirkung stehen in
    /// `faehigkeiten.rs`, Kombo und arkane Ladungen zählt der Server je Spieler.
    fn cast(&mut self, ctx: &mut Context, world: &mut World, player: PlayerId, target: Vec3) {
        use crate::faehigkeiten::*;
        let waffe = world.inventories.get(&player).map_or(0, |i| i.waffe);
        let tick = ctx.time.tick;
        let Some(avatar) = world.players.get_mut(&player) else { return };
        let Tool::Faehigkeit(platz) = avatar.tool else { return };
        let art = Faehigkeit::von(avatar.class, platz);
        let fach = (platz as usize).min(2);
        if tick < avatar.abklingen[fach] || avatar.leben <= 0.0 || !target.is_finite() {
            return;
        }
        // Stufe: Kombo des Hammerschlags, Arkanlanze mit voller Ladung
        let stufe = match art {
            Faehigkeit::Hammerschlag => kombo_stufe(avatar.kombo, tick),
            Faehigkeit::Arkangeschoss if avatar.ladung >= LADUNG_MAX => 1,
            _ => 0,
        };
        if art == Faehigkeit::Hammerschlag {
            avatar.kombo = Some((tick, stufe));
        }
        if art == Faehigkeit::Arkangeschoss && stufe == 1 {
            avatar.ladung = 0;
        }
        let (f_schaden, f_wirkung, f_abklingen) = crate::waffen::faktoren(waffe, avatar.class, art);
        avatar.abklingen[fach] = tick + (art.abklingen(stufe) as f32 * f_abklingen).round() as u64;
        let center = ctx.physics.character_position(avatar.character);
        let facing = avatar.facing;
        let character = avatar.character;
        let takte = |sekunden: f32| (sekunden / Physics::FIXED_DT).round() as u64;
        let ausholen = tick + takte(art.ausholen(stufe));
        let schaden = art.schaden(stufe) * f_schaden;
        let wirkung = |w: Wirkung| Wirkung { bremse: (w.bremse * f_wirkung).min(0.85), stun: w.stun * f_wirkung, brand: w.brand * f_wirkung, dauer: w.dauer, frost: w.frost * f_wirkung };
        let schlag = |due: u64, from: Vec3, richtung: Vec3, punkt: Vec3, bereich: Bereich, schaden: f32, w: Wirkung| PendingHit {
            due,
            art,
            by: player,
            from,
            richtung,
            punkt,
            bereich,
            schaden,
            wirkung: w,
            neben: None,
            laedt: false,
            flammen: false,
            faktor: f_wirkung,
        };
        let mut hits = Vec::new();
        let mut kette = Vec::new();
        let flach = |v: Vec3| {
            let v = v.with_y(0.0);
            if v.length_squared() > 0.01 { v.normalize() } else { vec3(facing.sin(), 0.0, -facing.cos()) }
        };
        let fuesse = center - Vec3::Y * 0.9;
        let (origin, point, hit) = match (art, stufe) {
            (Faehigkeit::Arkangeschoss, 1) => {
                // Arkanlanze: ein Strahl bis zum Boden (oder 45 m), durchbohrt alles darauf
                let Some(origin) = world.cast_origin(ctx, player, target) else { return };
                let richtung = (target - origin).normalize_or(Vec3::NEG_Z);
                let laenge = ctx.physics.raycast(origin, richtung, LANZE_WEITE, Some(character)).map_or(LANZE_WEITE, |(_, d)| d);
                let mut h = schlag(ausholen + 2, origin, richtung, origin, Bereich::Linie(laenge, LANZE_BREITE), schaden, wirkung(art.wirkung(stufe)));
                h.laedt = false;
                hits.push(h);
                (origin, origin + richtung * laenge, true)
            }
            (Faehigkeit::Arkangeschoss | Faehigkeit::Feuerball | Faehigkeit::Wurfhammer, _) => {
                let Form::Geschoss { tempo, flaeche } = art.form() else { return };
                let Some(origin) = world.cast_origin(ctx, player, target) else { return };
                let richtung = (target - origin).normalize_or(Vec3::NEG_Z);
                let range = origin.distance(target).min(art.reichweite()) + 0.5;
                let (mut point, animal) = world.spell_target(ctx, origin, richtung, range, Some(player));
                let mut ziel = animal.map(Getroffen::Tier);
                // Einheiten der Festung und der Lager haben keine Kollision: eigene Strahltests
                if let Some((enemy, distance)) = world.heer.ray_hit(origin, richtung, origin.distance(point)) {
                    point = origin + richtung * distance;
                    ziel = Some(Getroffen::Feind(enemy));
                }
                if let Some((wild, distance)) = world.wildnis.ray_hit(origin, richtung, origin.distance(point)) {
                    point = origin + richtung * distance;
                    ziel = Some(Getroffen::Wild(wild));
                }
                let due = ausholen + takte(origin.distance(point) / tempo);
                if flaeche > 0.0 {
                    let mut h = schlag(due, origin, richtung, point, Bereich::Kreis(flaeche), schaden, wirkung(art.wirkung(stufe)));
                    h.flammen = art == Faehigkeit::Feuerball;
                    hits.push(h);
                } else if let Some(ziel) = ziel {
                    let mut h = schlag(due, origin, richtung, point, Bereich::Ziel(ziel), schaden, wirkung(art.wirkung(stufe)));
                    h.laedt = art == Faehigkeit::Arkangeschoss;
                    hits.push(h);
                    if art == Faehigkeit::Wurfhammer {
                        // Der Hammer prallt zum nächsten Gegner ab (nicht zweimal derselbe)
                        let (mut feinde, mut wilde) = (Vec::new(), Vec::new());
                        match ziel {
                            Getroffen::Feind(id) => feinde.push(id),
                            Getroffen::Wild(id) => wilde.push(id),
                            Getroffen::Tier(_) => {}
                        }
                        let (mut von, mut zeit) = (point, due);
                        for i in 1..=ABPRALLE {
                            let feind = world.heer.nearest_except(von, ABPRALL_WEITE, &feinde, crate::heer::Filter::ALLE).map(|(id, p)| (Getroffen::Feind(id), p));
                            let wild = world.wildnis.naechster(von, ABPRALL_WEITE, &wilde).map(|(id, p)| (Getroffen::Wild(id), p));
                            let Some((naechstes, ort)) = [feind, wild].into_iter().flatten().min_by(|a, b| a.1.distance(von).total_cmp(&b.1.distance(von))) else { break };
                            match naechstes {
                                Getroffen::Feind(id) => feinde.push(id),
                                Getroffen::Wild(id) => wilde.push(id),
                                Getroffen::Tier(_) => {}
                            }
                            zeit += takte(von.distance(ort) / tempo);
                            let w = Wirkung { stun: ABPRALL_STUN[i] * f_wirkung, ..Default::default() };
                            hits.push(schlag(zeit, von, (ort - von).normalize_or(richtung), ort, Bereich::Ziel(naechstes), schaden * ABPRALL_SCHADEN[i], w));
                            kette.push(ort);
                            von = ort;
                        }
                    }
                }
                (origin, point, ziel.is_some())
            }
            (Faehigkeit::Hammerschlag, _) => {
                let richtung = flach(target - center);
                let (weite, winkel) = match stufe {
                    2 => (3.8, 42.0f32),
                    1 => (3.4, 75.0),
                    _ => (3.4, 65.0),
                };
                let einschlag = fuesse + richtung * 2.2;
                let mut h = schlag(ausholen, center, richtung, einschlag, Bereich::Kegel(weite, winkel.to_radians().cos()), schaden, wirkung(art.wirkung(stufe)));
                if stufe == 2 {
                    h.neben = Some(SCHMETTERN_WELLE);
                }
                hits.push(h);
                (center, einschlag, false)
            }
            (Faehigkeit::Frostnova, _) => {
                // Die Eiswelle läuft nach außen: wer weiter weg steht, wird später getroffen
                let Form::UmSich { radius } = art.form() else { return };
                let ringe = 3;
                for r in 0..ringe {
                    let (innen, aussen) = (radius * r as f32 / ringe as f32, radius * (r + 1) as f32 / ringe as f32);
                    let due = ausholen + takte((innen + aussen) * 0.5 / EISWELLE_TEMPO);
                    let bereich = if r == 0 { Bereich::Kreis(aussen) } else { Bereich::Ring(innen, aussen) };
                    hits.push(schlag(due, center, Vec3::NEG_Z, fuesse, bereich, schaden, wirkung(art.wirkung(stufe))));
                }
                (center, fuesse, false)
            }
            (Faehigkeit::Erdbeben, _) => {
                let Form::UmSich { radius } = art.form() else { return };
                hits.push(schlag(ausholen, center, Vec3::NEG_Z, fuesse, Bereich::Kreis(BEBEN_INNEN), schaden, wirkung(art.wirkung(stufe))));
                let aussen = Wirkung { stun: 1.0, ..Default::default() };
                hits.push(schlag(ausholen, center, Vec3::NEG_Z, fuesse, Bereich::Ring(BEBEN_INNEN, radius), schaden * BEBEN_AUSSEN_ANTEIL, wirkung(aussen)));
                for (nach, r, s) in NACHBEBEN {
                    let w = Wirkung { bremse: 0.4, ..Default::default() };
                    hits.push(schlag(ausholen + takte(nach), center, Vec3::NEG_Z, fuesse, Bereich::Kreis(r), s * f_schaden, wirkung(w)));
                }
                (center, fuesse, false)
            }
        };
        world.cast_spell(ctx, player, origin, point, hit, true, art, stufe, &kette);
        self.broadcast(ServerMessage::SpellCast { by: player, origin, target: point, hit, art, stufe, kette });
        self.pending_hits.extend(hits);
    }

    /// Fähigkeiten, die jetzt wirken: Schaden an Tieren, Truppen der Festung und Lagerbewohnern,
    /// bei erlegten Tieren Beute für den Spieler.
    fn land_hits(&mut self, ctx: &mut Context, world: &mut World) {
        let tick = ctx.time.tick;
        let (landed, waiting): (Vec<_>, Vec<_>) = self.pending_hits.drain(..).partition(|h| h.due <= tick);
        self.pending_hits = waiting;
        for hit in landed {
            let art = hit.art;
            let boden = hit.art == Faehigkeit::Erdbeben;
            let filter = if boden { crate::heer::Filter::BODEN } else { crate::heer::Filter::ALLE };
            // Was getroffen wird
            let lebende_tiere = || world.animals.iter().enumerate().filter(|(_, a)| a.is_alive());
            let im_kreis = |mitte: Vec3, innen: f32, aussen: f32, p: Vec3, r: f32| {
                let d = p.distance(mitte);
                d <= aussen + r && d > innen + r * 0.5
            };
            let (tiere, feinde, wilde): (Vec<u16>, Vec<u16>, Vec<u16>) = match hit.bereich {
                Bereich::Ziel(Getroffen::Tier(id)) => (vec![id], vec![], vec![]),
                Bereich::Ziel(Getroffen::Feind(id)) => (vec![], vec![id], vec![]),
                Bereich::Ziel(Getroffen::Wild(id)) => (vec![], vec![], vec![id]),
                Bereich::Kreis(radius) => (
                    lebende_tiere().filter(|(_, a)| a.hit_sphere().0.distance(hit.punkt) <= radius + a.hit_sphere().1).map(|(i, _)| i as u16).collect(),
                    world.heer.within(hit.punkt, radius, filter),
                    world.wildnis.within(hit.punkt, radius),
                ),
                Bereich::Ring(innen, aussen) => {
                    let innen_feinde = world.heer.within(hit.punkt, innen, filter);
                    let innen_wilde = world.wildnis.within(hit.punkt, innen);
                    (
                        lebende_tiere().filter(|(_, a)| im_kreis(hit.punkt, innen, aussen, a.hit_sphere().0, a.hit_sphere().1)).map(|(i, _)| i as u16).collect(),
                        world.heer.within(hit.punkt, aussen, filter).into_iter().filter(|id| !innen_feinde.contains(id)).collect(),
                        world.wildnis.within(hit.punkt, aussen).into_iter().filter(|id| !innen_wilde.contains(id)).collect(),
                    )
                }
                Bereich::Kegel(weite, cos) => (
                    lebende_tiere()
                        .filter(|(_, a)| {
                            let d = (a.hit_sphere().0 - hit.from).with_y(0.0);
                            d.length() <= weite + a.hit_sphere().1 && d.normalize_or_zero().dot(hit.richtung) >= cos
                        })
                        .map(|(i, _)| i as u16)
                        .collect(),
                    world.heer.im_kegel(hit.from, hit.richtung, weite, cos, crate::heer::Filter::NAHKAMPF).into_iter().map(|(id, _)| id).collect(),
                    world.wildnis.im_kegel(hit.from, hit.richtung, weite, cos),
                ),
                Bereich::Linie(laenge, breite) => (
                    lebende_tiere()
                        .filter(|(_, a)| {
                            let to = a.hit_sphere().0 - hit.from;
                            let along = to.dot(hit.richtung);
                            along > 0.0 && along <= laenge && (to - hit.richtung * along).length() <= breite + a.hit_sphere().1
                        })
                        .map(|(i, _)| i as u16)
                        .collect(),
                    world.heer.auf_linie(hit.from, hit.richtung, laenge, breite, crate::heer::Filter::ALLE).into_iter().map(|(id, _)| id).collect(),
                    world.wildnis.auf_linie(hit.from, hit.richtung, laenge, breite),
                ),
            };
            // Schmetterschlag: Druckwelle um den Einschlag trifft auch, wer nicht im Kegel stand
            let (neben_feinde, neben_wilde): (Vec<u16>, Vec<u16>) = match hit.neben {
                Some((radius, _)) => (
                    world.heer.within(hit.punkt, radius, crate::heer::Filter::BODEN).into_iter().filter(|id| !feinde.contains(id)).collect(),
                    world.wildnis.within(hit.punkt, radius).into_iter().filter(|id| !wilde.contains(id)).collect(),
                ),
                None => (Vec::new(), Vec::new()),
            };
            let getroffen = !(tiere.is_empty() && feinde.is_empty() && wilde.is_empty());
            let name = world.players.get(&hit.by).map(|a| player_key(&a.name)).unwrap_or_default();
            let w = hit.wirkung;
            let anteil = hit.neben.map_or(0.0, |(_, a)| a);
            for (id, schaden, voll) in feinde.into_iter().map(|id| (id, hit.schaden, true)).chain(neben_feinde.into_iter().map(|id| (id, hit.schaden * anteil, false))) {
                let treffer = if voll {
                    crate::heer::Hit { schaden, art: art.art(), bremse: w.bremse, brand: w.brand, dauer: w.dauer, stun: w.stun, frost: w.frost, ..Default::default() }
                } else {
                    crate::heer::Hit { schaden, art: art.art(), ..Default::default() }
                };
                world.heer.damage(id, treffer, Quelle { name: &name, turm: None });
            }
            for (id, schaden, voll) in wilde.into_iter().map(|id| (id, hit.schaden, true)).chain(neben_wilde.into_iter().map(|id| (id, hit.schaden * anteil, false))) {
                let treffer = if voll {
                    Treffer { schaden, art: art.art(), bremse: w.bremse, stun: w.stun, brand: w.brand, dauer: w.dauer, frost: w.frost }
                } else {
                    Treffer { schaden, art: art.art(), ..Default::default() }
                };
                world.wildnis.damage(id, treffer, &name, Some(hit.by));
            }
            for id in tiere {
                let Some(animal) = world.animals.get_mut(id as usize) else { continue };
                if !animal.is_alive() {
                    continue;
                }
                let kind = animal.kind;
                let health = animal.hit(hit.from);
                world.animal_hit(ctx, id, health, true);
                self.broadcast(ServerMessage::AnimalHit { id, health, by: hit.by });
                if health == 0 {
                    let ort = world.animals[id as usize].hit_sphere().0;
                    for &(item, amount) in kind.loot() {
                        self.beute_ablegen(world, ort, crate::beute::Fund::Gegenstand(item, amount));
                    }
                }
            }
            // Arkangeschoss: ein Treffer lädt eine arkane Ladung
            if hit.laedt && getroffen {
                if let Some(avatar) = world.players.get_mut(&hit.by) {
                    avatar.ladung = (avatar.ladung + 1).min(crate::faehigkeiten::LADUNG_MAX);
                    avatar.ladung_tick = tick;
                }
            }
            if hit.flammen {
                let (_, dauer, _) = crate::faehigkeiten::FLAMMEN;
                self.flammen.push(Flammen { mitte: hit.punkt, bis: tick + (dauer / Physics::FIXED_DT) as u64, by: hit.by, faktor: hit.faktor });
            }
        }
    }

    /// Flammenteppiche der Feuerbälle: alle halbe Sekunde brennt, wer darin steht, weiter.
    fn flammen_takt(&mut self, ctx: &Context, world: &mut World) {
        let tick = ctx.time.tick;
        self.flammen.retain(|f| f.bis > tick);
        if tick % 30 != 0 {
            return;
        }
        let (radius, _, brand) = crate::faehigkeiten::FLAMMEN;
        for f in &self.flammen {
            let name = world.players.get(&f.by).map(|a| player_key(&a.name)).unwrap_or_default();
            for id in world.heer.within(f.mitte, radius, crate::heer::Filter::BODEN) {
                let hit = crate::heer::Hit { schaden: 0.0, art: crate::tuerme::DamageKind::Fire, brand: brand * f.faktor, dauer: 1.5, ..Default::default() };
                world.heer.damage(id, hit, Quelle { name: &name, turm: None });
            }
            for id in world.wildnis.within(f.mitte, radius) {
                let treffer = Treffer { schaden: 0.0, art: crate::tuerme::DamageKind::Fire, brand: brand * f.faktor, dauer: 1.5, ..Default::default() };
                world.wildnis.damage(id, treffer, &name, Some(f.by));
            }
        }
    }

    // ---------- Leben der Spieler, Wildnis, Runen ----------

    /// Ein Spieler nimmt Schaden. Fällt er, steht er mit vollen Leben an seinem Startpunkt wieder
    /// auf (vor der eigenen Dorfhalle, sonst im Startlager).
    fn spieler_schaden(&mut self, ctx: &mut Context, world: &mut World, player: PlayerId, schaden: f32, von: &str) {
        let Some(avatar) = world.players.get_mut(&player) else { return };
        if avatar.noclip || avatar.leben <= 0.0 || schaden <= 0.0 {
            return;
        }
        avatar.leben -= schaden;
        avatar.getroffen = ctx.time.tick;
        let wert = schaden.round().max(1.0) as u16;
        world.spieler_getroffen(player, wert);
        self.broadcast(ServerMessage::SpielerGetroffen { player, schaden: wert });
        let Some(avatar) = world.players.get_mut(&player) else { return };
        if avatar.leben > 0.0 {
            return;
        }
        avatar.leben = avatar.max_leben();
        let (character, name) = (avatar.character, avatar.name.clone());
        let start = world.startpunkt(&name);
        ctx.physics.teleport_character(character, start);
        log::info!("{name} wurde von {von} besiegt");
        world.spieler_gefallen(player, von);
        self.broadcast(ServerMessage::SpielerGefallen { player, von: von.to_string() });
    }

    /// Wer eine Weile nicht getroffen wurde, heilt sich langsam.
    fn heilen(&mut self, ctx: &Context, world: &mut World) {
        for avatar in world.players.values_mut() {
            // Arkane Ladungen verfallen ohne neuen Treffer
            if avatar.ladung > 0 && ctx.time.tick > avatar.ladung_tick + crate::faehigkeiten::LADUNG_HAELT {
                avatar.ladung = 0;
            }
            if avatar.leben < avatar.max_leben() && ctx.time.tick > avatar.getroffen + HEILEN_NACH {
                avatar.leben = (avatar.leben + avatar.max_leben() * HEILEN_ANTEIL * Physics::FIXED_DT).min(avatar.max_leben());
            }
        }
    }

    /// Ein Takt der Wildnis: Lagerbewohner bewegen sich und greifen an; Besiegte bringen Gold und
    /// mit etwas Glück ein Runenfragment.
    fn wildnis_takt(&mut self, ctx: &mut Context, world: &mut World) {
        let spieler: Vec<(PlayerId, Vec3)> = world
            .players
            .iter()
            .filter(|(_, a)| !a.noclip && a.leben > 0.0)
            .map(|(&id, a)| (id, ctx.physics.character_position(a.character)))
            .collect();
        let terrain = &world.terrain;
        let angriffe = world.wildnis.tick(Physics::FIXED_DT, &spieler, &|p| terrain.height_at(p.x, p.y));
        for a in angriffe {
            let entry = (a.kind, a.von, a.ziel);
            self.strikes.push(entry);
            world.strikes.push(entry);
            self.spieler_schaden(ctx, world, a.spieler, a.schaden, a.kind.label());
        }
        world.wildnis.besetzt.clear();
        for g in std::mem::take(&mut world.wildnis.gefallen) {
            // Alles fällt zu Boden und will mit E aufgehoben werden
            use crate::beute::Fund;
            self.beute_ablegen(world, g.ort, Fund::Gegenstand(Item::Gold, crate::wildnis::gold(g.gefahr, g.anfuehrer)));
            if self.rng.chance(crate::wildnis::fragment_chance(g.gefahr, g.anfuehrer)) {
                self.beute_ablegen(world, g.ort, Fund::Gegenstand(Item::Runenfragment, 1));
                let ereignis = Ereignis::Heilung(g.ort + Vec3::Y);
                world.ereignisse.push(ereignis);
                self.ereignisse.push(ereignis);
            }
            if self.rng.chance(crate::waffen::waffen_chance(g.gefahr, g.anfuehrer)) {
                // Eine Waffe für die Klasse dessen, der den letzten Treffer hatte
                let class = world.players.values().find(|a| player_key(&a.name) == g.von).map_or(CharacterClass::Mage, |a| a.class);
                let seltenheit = crate::waffen::seltenheit_fuer(g.gefahr, self.rng.range(0.0, 1.0));
                let passend: Vec<u8> = crate::waffen::WAFFEN.iter().filter(|w| w.klasse == class && w.seltenheit == seltenheit).map(|w| w.id).collect();
                if !passend.is_empty() {
                    let id = passend[(self.rng.next_u32() % passend.len() as u32) as usize];
                    self.beute_ablegen(world, g.ort, Fund::Waffe(id));
                    if seltenheit >= crate::waffen::Seltenheit::Episch {
                        let name = crate::waffen::waffe(id).map_or("", |w| w.name);
                        let text = format!("{} ({}) hinterlässt eine {} Waffe: {name}!", g.kind.label(), g.lager, if seltenheit == crate::waffen::Seltenheit::Legendaer { "legendäre" } else { "epische" });
                        world.chat_events.push(crate::world::ChatLine::notice(text.clone()));
                        self.broadcast(ServerMessage::Notice(text));
                    }
                }
            }
        }
    }

    /// Legt Beute neben `ort` auf den Boden (etwas verstreut) und meldet sie allen.
    fn beute_ablegen(&mut self, world: &mut World, ort: Vec3, fund: crate::beute::Fund) {
        let winkel = self.rng.range(0.0, std::f32::consts::TAU);
        let weite = self.rng.range(0.4, 1.4);
        let p = vec2(ort.x + winkel.cos() * weite, ort.z + winkel.sin() * weite);
        let boden = world.terrain.height_at(p.x, p.y);
        let id = self.beute_naechste;
        self.beute_naechste += 1;
        let beute = crate::beute::Bodenbeute { id, ort: vec3(p.x, boden, p.y), fund };
        world.beute.insert(id, beute);
        self.beute_bis.insert(id, self.tick + (crate::beute::LIEGT_SEKUNDEN / Physics::FIXED_DT) as u64);
        self.broadcast(ServerMessage::Beute(vec![beute]));
    }

    /// Ein Spieler hebt Beute auf (E): Reichweite prüfen, ins Inventar, für alle verschwinden lassen.
    /// Eine Waffe, die man schon hat, wird zu Gold.
    pub fn aufheben(&mut self, ctx: &mut Context, world: &mut World, player: PlayerId, id: u32) -> Result<(), String> {
        let Some(avatar) = world.players.get(&player) else { return Err("Unbekannter Spieler".into()) };
        let (name, class) = (avatar.name.clone(), avatar.class);
        let ort = ctx.physics.character_position(avatar.character);
        let Some(beute) = world.beute.get(&id).copied() else { return Err("Die Beute ist schon weg".into()) };
        if vec2(beute.ort.x, beute.ort.z).distance(vec2(ort.x, ort.z)) > crate::beute::AUFHEBEN_WEITE + 0.8 || (beute.ort.y - ort.y).abs() > 3.5 {
            return Err("Zu weit weg".into());
        }
        world.beute.remove(&id);
        self.beute_bis.remove(&id);
        self.broadcast(ServerMessage::BeuteWeg(id));
        let inventory = world.inventories.entry(player).or_default();
        let mut meldung = None;
        match beute.fund {
            crate::beute::Fund::Gegenstand(item, n) => inventory.add_item(item, n),
            crate::beute::Fund::Waffe(waffe) => {
                let Some(w) = crate::waffen::waffe(waffe) else { return Ok(()) };
                let bit = 1u16 << w.id;
                if inventory.waffen & bit != 0 {
                    inventory.gold += w.seltenheit.gold_fuer_doppelte();
                    meldung = Some((format!("{} hast du schon – eingeschmolzen für {} Gold.", w.name, w.seltenheit.gold_fuer_doppelte()), false));
                } else {
                    inventory.waffen |= bit;
                    // Die erste Waffe der eigenen Klasse gleich in die Hand nehmen
                    if w.klasse == class && crate::waffen::ausgeruestet(inventory.waffe, class).is_none() {
                        inventory.waffe = w.id;
                    }
                    let wertvoll = w.seltenheit >= crate::waffen::Seltenheit::Episch;
                    meldung = Some((format!("{name} findet: {} ({})!", w.name, w.seltenheit.label()), wertvoll));
                }
            }
        }
        let inventory = *inventory;
        self.send_inventory(player, inventory);
        if let Some((text, an_alle)) = meldung {
            world.chat_events.push(crate::world::ChatLine::notice(text.clone()));
            if an_alle {
                self.broadcast(ServerMessage::Notice(text));
            } else if let Some(net) = &mut self.net {
                if player != HOST_PLAYER {
                    net.send(player, Channel::Reliable, encode(&ServerMessage::Notice(text)));
                }
            }
        }
        Ok(())
    }

    /// Eine erbeutete Waffe (oder mit 0 die Startwaffe) in die Hand nehmen.
    pub fn ausruesten(&mut self, world: &mut World, player: PlayerId, waffe: u8) -> Result<(), String> {
        let class = world.players.get(&player).map(|a| a.class).ok_or("Unbekannter Spieler")?;
        let inventory = world.inventories.entry(player).or_default();
        if waffe != 0 {
            let w = crate::waffen::waffe(waffe).ok_or("Diese Waffe gibt es nicht")?;
            if inventory.waffen & (1u16 << w.id) == 0 {
                return Err("Diese Waffe hast du nicht".into());
            }
            if w.klasse != class {
                return Err(format!("{} kann nur {} führen", w.name, if w.klasse == CharacterClass::Zwerg { "ein Zwerg" } else { "ein Magier" }));
            }
        }
        inventory.waffe = waffe;
        let inventory = *inventory;
        self.send_inventory(player, inventory);
        Ok(())
    }

    /// Runen: am Runenbrunnen vier Fragmente zu einem Runenstein vereinen, einen Runenstein in
    /// den Schutzstein eines freien Siedlungsplatzes setzen (dann gehört er dem Spieler).
    pub fn runen(&mut self, ctx: &mut Context, world: &mut World, player: PlayerId, befehl: RunenBefehl) -> Result<(), String> {
        let Some(avatar) = world.players.get(&player) else { return Err("Unbekannter Spieler".into()) };
        let (key, name) = (player_key(&avatar.name), avatar.name.clone());
        let ort = ctx.physics.character_position(avatar.character);
        let flach = |a: Vec3, b: Vec3| vec2(a.x, a.z).distance(vec2(b.x, b.z));
        match befehl {
            RunenBefehl::Schmieden => {
                if flach(ort, world.runenbrunnen) > RUNEN_REICHWEITE + 4.0 {
                    return Err("Zu weit vom Runenbrunnen entfernt".into());
                }
                let inventory = world.inventories.entry(player).or_default();
                if !inventory.remove_item(Item::Runenfragment, FRAGMENTE_JE_STEIN) {
                    return Err(format!("Du brauchst {FRAGMENTE_JE_STEIN} Runenfragmente (Beute aus den Lagern der Wildnis)"));
                }
                inventory.add_item(Item::Runenstein, 1);
                let inventory = *inventory;
                self.send_inventory(player, inventory);
                let text = format!("{name} vereint am Runenbrunnen vier Fragmente zu einem Runenstein!");
                world.chat_events.push(crate::world::ChatLine::notice(text.clone()));
                self.broadcast(ServerMessage::Notice(text));
                let ereignis = Ereignis::Puls(world.runenbrunnen + Vec3::Y * 1.5, 8.0);
                world.ereignisse.push(ereignis);
                self.ereignisse.push(ereignis);
            }
            RunenBefehl::Einsetzen(platz) => {
                let platz = platz as usize;
                let Some(&stein) = world.schutzsteine.get(platz) else { return Err("Diesen Siedlungsplatz gibt es nicht".into()) };
                if flach(ort, stein) > RUNEN_REICHWEITE + 4.0 {
                    return Err("Zu weit vom Schutzstein entfernt".into());
                }
                self.runen.resize(world.schutzsteine.len(), None);
                if let Some(besitzer) = &self.runen[platz] {
                    return Err(if *besitzer == key { "Dieser Siedlungsplatz gehört dir schon".into() } else { format!("Dieser Siedlungsplatz gehört schon {besitzer}") });
                }
                if self.runen.iter().any(|r| r.as_deref() == Some(key.as_str())) {
                    return Err("Du hast schon einen Siedlungsplatz".into());
                }
                let inventory = world.inventories.entry(player).or_default();
                if !inventory.remove_item(Item::Runenstein, 1) {
                    return Err("Du brauchst einen Runenstein (vier Runenfragmente am Runenbrunnen der Burg)".into());
                }
                let inventory = *inventory;
                self.send_inventory(player, inventory);
                self.runen[platz] = Some(key);
                let strasse = world.heer.strassen_namen().get(platz).copied().unwrap_or("?");
                let text = format!("{name} setzt einen Runenstein in den Schutzstein der Straße {strasse} – der Siedlungsplatz gehört jetzt {name}.");
                world.chat_events.push(crate::world::ChatLine::notice(text.clone()));
                self.broadcast(ServerMessage::Notice(text));
                let ereignis = Ereignis::Puls(stein + Vec3::Y * 2.0, 10.0);
                world.ereignisse.push(ereignis);
                self.ereignisse.push(ereignis);
                let stand = self.td_stand(world, false);
                world.td_uebernehmen(stand);
                self.save(world);
            }
        }
        Ok(())
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
            AdminCommand::Runenfragmente => {
                let inventory = world.inventories.entry(player).or_default();
                inventory.runenfragmente += FRAGMENTE_JE_STEIN;
                let inventory = *inventory;
                self.send_inventory(player, inventory);
            }
            AdminCommand::LagerNeu => {
                let terrain = &world.terrain;
                world.wildnis.alle_neu(&|p| terrain.height_at(p.x, p.y));
                world.wildnis.besetzt.clear();
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
        world.feinde.extend(world.wildnis.states());
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
            runen: (0..world.schutzsteine.len()).map(|i| self.runen.get(i).cloned().flatten()).collect(),
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
                    intro.push(ServerMessage::Beute(world.beute.values().copied().collect()));
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
        let mut runen: Vec<(ClientId, RunenBefehl)> = Vec::new();
        let mut aufheben: Vec<(ClientId, u32)> = Vec::new();
        let mut ausruesten: Vec<(ClientId, u8)> = Vec::new();
        for (&id, client) in &mut self.clients {
            while let Some(bytes) = net.message(id, Channel::Reliable) {
                match decode(&bytes) {
                    Some(ClientMessage::Chat(text)) => chats.push((id, text)),
                    Some(ClientMessage::Build { kind, at, yaw }) => builds.push((id, kind, at, yaw)),
                    Some(ClientMessage::Admin(command)) => admins.push((id, command)),
                    Some(ClientMessage::Td(befehl)) => changes.push((id, befehl)),
                    Some(ClientMessage::Runen(befehl)) => runen.push((id, befehl)),
                    Some(ClientMessage::Aufheben(beute)) => aufheben.push((id, beute)),
                    Some(ClientMessage::Ausruesten(waffe)) => ausruesten.push((id, waffe)),
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
        for (id, beute) in aufheben {
            let result = self.aufheben(ctx, world, id, beute);
            if let (Err(reason), Some(net)) = (result, &mut self.net) {
                net.send(id, Channel::Reliable, encode(&ServerMessage::BuildRefused(reason)));
            }
        }
        for (id, waffe) in ausruesten {
            let result = self.ausruesten(world, id, waffe);
            if let (Err(reason), Some(net)) = (result, &mut self.net) {
                net.send(id, Channel::Reliable, encode(&ServerMessage::BuildRefused(reason)));
            }
        }
        for (id, befehl) in runen {
            let result = self.runen(ctx, world, id, befehl);
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
        self.runen = save.runen;
        // Ältere Stände: wer dort schon eine Dorfhalle hat, dem gehört der Siedlungsplatz
        self.runen.resize(world.schutzsteine.len(), None);
        for platz in 0..self.runen.len() {
            if self.runen[platz].is_none() {
                self.runen[platz] = world.dorfhalle_auf_platz(platz).map(|h| h.owner.clone());
            }
        }
        let stand = self.td_stand(world, false);
        world.td_uebernehmen(stand);
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
        save.runen = self.runen.clone();
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
                    leben: avatar.leben.max(0.0).ceil() as u16,
                    waffe: avatar.waffe,
                    ladung: avatar.ladung,
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
