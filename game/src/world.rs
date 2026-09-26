//! Die Spielwelt, wie sie Server und Clients gleichermaßen aufbauen.

use std::collections::{BTreeMap, HashMap};

use engine::prelude::*;

use crate::animals::{self, Animal};
use crate::island::{self, ResourceKind, ResourceSpec};
use crate::characters::{Action, Puppet, CHOP_STRIKE, MINE_STRIKE};
use crate::protocol::{CharacterClass, Inventory, NetId, ObjectKind, PlayerId, PlayerInput, Tool, HOST_PLAYER};

pub const WALK_SPEED: f32 = 5.0;
pub const SPRINT_SPEED: f32 = 9.0;
/// Takte zwischen zwei Zaubern desselben Spielers (0,7 s).
pub const CAST_COOLDOWN_TICKS: u64 = 42;
/// So weit fliegt ein Zauber (Meter).
pub const CAST_RANGE: f32 = 45.0;
/// Tempo des Zaubergeschosses (m/s).
pub const BOLT_SPEED: f32 = 34.0;
/// So lange holt der Magier aus, bevor das Geschoss losfliegt (Sekunden, passt zur Animation).
pub const CAST_DELAY: f32 = 0.22;
/// Takte zwischen zwei Axthieben auf einen Baum (so lang wie die Animation „Hacken“).
pub const HARVEST_COOLDOWN_TICKS: u64 = 36;
/// Takte zwischen zwei Schlägen mit der Spitzhacke (so lang wie die Animation „Abbauen“).
pub const MINE_COOLDOWN_TICKS: u64 = 48;
/// Wie nah man einem Rohstoff sein muss (Meter vom Rand).
pub const HARVEST_REACH: f32 = 2.2;
/// Nach dieser Zeit wachsen Bäume nach und Felsen tauchen wieder auf (2 Minuten).
pub const RESPAWN_TICKS: u64 = 60 * 120;

pub struct Avatar {
    pub entity: EntityId,
    pub character: CharacterId,
    /// Blickrichtung (Yaw in Radiant), folgt der Laufrichtung.
    pub facing: f32,
    /// Werkzeug in der Hand (aus der Auswahlleiste).
    pub tool: Tool,
    pub last_cast_tick: u64,
    pub last_harvest_tick: u64,
    pub name: String,
    pub class: CharacterClass,
    /// Admin: fliegt frei durch Wände
    pub noclip: bool,
}

pub struct NetObject {
    pub entity: EntityId,
    pub body: RigidBodyHandle,
    pub kind: ObjectKind,
}

/// Ein Baum oder Fels, der abgebaut werden kann.
pub struct Resource {
    pub spec: ResourceSpec,
    pub health: u8,
    entity: Option<EntityId>,
    body: Option<RigidBodyHandle>,
    /// Abgebaut: in welchem Takt der Rohstoff nachwächst.
    pub regrows_at: Option<u64>,
    /// Restzeit des Wackelns nach einem Treffer (Sekunden).
    shake: f32,
    /// Optik: Lebensstand, den das Modell gerade zeigt (Vorkommen schrumpfen mit jedem Schlag).
    shown_health: u8,
    /// Optik: Schlag mit der Spitzhacke, der erst noch auftrifft (Restzeit in Sekunden).
    strike: Option<f32>,
    /// Optik: Stauchen nach einem Treffer (1 → 0), Wachsen nach dem Nachwachsen (0 → 1),
    /// aktuelle Größe (weich nachgeführt).
    pop: f32,
    grow: f32,
    size: f32,
}

impl Resource {
    pub fn is_present(&self) -> bool {
        self.regrows_at.is_none()
    }

    /// Ungefährer Radius am Boden (für die Reichweite).
    fn radius(&self) -> f32 {
        match self.spec.collider {
            Shape::Capsule { radius, .. } => radius,
            Shape::Box { size } => size.x.max(size.z) * 0.5,
            Shape::Sphere { radius } => radius,
        }
    }
}

/// Etwas Hörbares ist passiert (nur mit Fenster gesammelt, das Spiel spielt es ab).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SoundEvent {
    Hit { kind: ResourceKind, at: Vec3, finished: bool },
    Cast { player: PlayerId },
    /// Ein Zauber schlägt ein; `animal` = in ein Tier (sonst Boden, Baum oder Luft).
    Impact { at: Vec3, animal: bool, killed: bool },
    /// Holz knistert im Lagerfeuer.
    Crackle { at: Vec3 },
    /// Donner nach einem Blitz (überall zu hören).
    Thunder { volume: f32 },
    /// Hammerschlag auf einer Baustelle bzw. Gebäude fertig (`done`)
    Built { at: Vec3, done: bool },
}

/// Ein fliegendes Zaubergeschoss (nur Optik; ob es trifft, entscheidet der Server).
struct Bolt {
    entity: EntityId,
    origin: Vec3,
    target: Vec3,
    /// Sekunden seit dem Zaubern (erst nach `CAST_DELAY` fliegt es los).
    age: f32,
    hit: bool,
}

/// Eine Zeile im Chat: von einem Spieler oder ein Hinweis (`from` = None, z. B. „… ist beigetreten“).
#[derive(Clone, Debug)]
pub struct ChatLine {
    pub from: Option<PlayerId>,
    pub name: String,
    pub text: String,
}

impl ChatLine {
    pub fn notice(text: String) -> ChatLine {
        ChatLine { from: None, name: String::new(), text }
    }
}

pub struct World {
    pub players: HashMap<PlayerId, Avatar>,
    /// Neue Chatzeilen seit dem letzten Bild (die Oberfläche holt sie ab).
    pub chat_events: Vec<ChatLine>,
    /// Gezeichnete Übersichtskarte (nur mit Fenster), siehe `island::map_image`.
    pub map: Option<Image>,
    /// Besondere Orte: Lagerfeuer und Wegweiser.
    pub places: crate::orte::Places,
    /// Zeit bis zum nächsten Knistern eines Lagerfeuers.
    crackle_timer: f32,
    /// Geworfene Bälle, die übers Netzwerk abgeglichen werden.
    pub objects: BTreeMap<NetId, NetObject>,
    pub resources: BTreeMap<u32, Resource>,
    by_entity: HashMap<EntityId, u32>,
    pub inventories: HashMap<PlayerId, Inventory>,
    /// Sichtbare, animierte Figuren (nur mit Fenster).
    puppets: HashMap<PlayerId, Puppet>,
    /// Startpunkt für neue Spieler.
    pub spawn: Vec3,
    pub terrain: Terrain,
    /// Tiere; der Index ist ihre Netzwerk-ID.
    pub animals: Vec<Animal>,
    /// Tageszeit mit Sonne, Mond und Himmelsfarben.
    pub day: DayCycle,
    /// Zufall für Effekte wie Glühwürmchen (muss nicht auf allen Rechnern gleich sein).
    effects_rng: Rng,
    firefly_timer: f32,
    capsule: MeshId,
    /// Geräusche seit dem letzten Bild (siehe `SoundEvent`).
    pub sound_events: Vec<SoundEvent>,
    /// Fliegende Zaubergeschosse (nur mit Fenster).
    bolts: Vec<Bolt>,
    /// Vögel, Möwen, Schmetterlinge, Fische (nur mit Fenster).
    wildlife: Option<crate::leben::Wildlife>,
    /// Zwei Ritter, die vor dem Schlossportal patrouillieren (nur mit Fenster).
    wachen: Option<crate::wachen::Wachen>,
    /// Wolken, Regen, Gewitter, Regenbogen, Polarlicht (aus Tag und Uhrzeit).
    pub weather: crate::wetter::Weather,
    /// Magische Kristallvorkommen (Mitte am Boden): leuchten und funkeln.
    crystals: Vec<Vec3>,
    /// Von Spielern errichtete Gebäude (Holzfäller, Steinbruch, Erzmine)
    pub buildings: Vec<crate::bauten::Building>,
    /// Baustellen, fertige Gebäude und die Bauvorschau (nur mit Fenster)
    bau: crate::bauten::BauVisuals,
    /// Truppen der Schattenfestung: Simulation (nur beim Server) …
    pub heer: crate::heer::Heer,
    /// … und was alle davon sehen (beim Client aus den Schnappschüssen)
    pub feinde: Vec<crate::heer::EnemyState>,
    /// Angriffe der Truppen, die noch gezeigt werden sollen
    pub strikes: Vec<(crate::heer::EnemyKind, Vec3, Vec3)>,
    heer_ansicht: crate::heer::HeerAnsicht,
    /// Erzwungenes Wetter (Index in `protocol::WETTER`, 0 = automatisch)
    pub weather_choice: u8,
    /// Gebäude: unsichtbares Objekt (zum Anvisieren) und die festen Körper
    building_bodies: HashMap<u32, (EntityId, Vec<RigidBodyHandle>)>,
    /// Schüsse der Türme, die noch gezeigt werden sollen
    pub tower_shots: Vec<crate::tuerme::Schuss>,
    /// Schutzsteine an den Straßenenden (Mitte am Boden) und die zuletzt gezeigten Leben
    pub schutzsteine: Vec<Vec3>,
    shown_leben: u32,
    /// Stand der Verteidigung (beim Server selbst gerechnet, beim Client aus dem Schnappschuss)
    pub td: crate::td::TdStand,
    /// Kills und Schaden je Turm, Schaden und Kills je Spieler (zuletzt gemeldet)
    pub td_stats: HashMap<u32, (u32, u32)>,
    pub td_beitrag: Vec<(String, u32, u32)>,
    /// Auswertungen überstandener Wellen (die Oberfläche zeigt die neueste)
    pub berichte: Vec<crate::td::WellenBericht>,
    /// Ereignisse der Verteidigung, die noch gezeigt werden sollen
    pub ereignisse: Vec<crate::td::Ereignis>,
    /// Brand- und Giftfelder am Boden, Runen- und Frostfelder (Mitte, Radius, Restzeit, Art)
    felder: Vec<(Vec3, f32, f32, u8)>,
}

impl World {
    /// Baut die Insel. Auf allen Rechnern identisch, damit die IDs passen.
    pub fn new(ctx: &mut Context) -> Self {
        let settings = CharacterSettings::default();
        let capsule = ctx.assets.named_mesh("spielfigur", || MeshData::capsule(settings.radius, settings.height, 24, 8));
        let island = island::build(ctx);
        // Marschrouten der Truppen: vom Hof durch das Tor über die Rampe und die Straße
        let achsen = [Vec2::Y, Vec2::X, Vec2::NEG_Y, Vec2::NEG_X];
        let routes = island
            .strassen
            .iter()
            .zip(achsen)
            .map(|(strasse, achse)| crate::heer::Route::new(achse, island::festung_hoehe(), strasse, |p| island.terrain.height_at(p.x, p.y)))
            .collect();
        let animals = animals::populate(&island.terrain, island.spawn, island::SEED, island::moisture);
        let mut world = World {
            players: HashMap::new(),
            objects: BTreeMap::new(),
            resources: BTreeMap::new(),
            by_entity: HashMap::new(),
            inventories: HashMap::new(),
            puppets: HashMap::new(),
            crystals: island.crystals,
            chat_events: Vec::new(),
            map: island.map,
            places: island.places,
            crackle_timer: 0.0,
            spawn: island.spawn + Vec3::Y * 1.2,
            terrain: island.terrain,
            animals,
            day: DayCycle::default(),
            effects_rng: Rng::new(7),
            firefly_timer: 0.0,
            capsule,
            sound_events: Vec::new(),
            bolts: Vec::new(),
            wildlife: None,
            wachen: None,
            weather: Default::default(),
            buildings: Vec::new(),
            bau: Default::default(),
            heer: crate::heer::Heer::new(routes),
            feinde: Vec::new(),
            strikes: Vec::new(),
            heer_ansicht: Default::default(),
            weather_choice: 0,
            building_bodies: HashMap::new(),
            tower_shots: Vec::new(),
            schutzsteine: Vec::new(),
            shown_leben: crate::heer::MAX_LEBEN,
            td: Default::default(),
            td_stats: HashMap::new(),
            td_beitrag: Vec::new(),
            berichte: Vec::new(),
            ereignisse: Vec::new(),
            felder: Vec::new(),
        };
        // Schutzsteine am Ende jeder Heerstraße (etwas hinter dem Ende, quer zur Straße)
        for (ende, richtung) in world.heer.enden() {
            let p = vec2(ende.x, ende.z) + richtung * 5.0;
            let boden = vec3(p.x, world.terrain.height_at(p.x, p.y), p.y);
            world.schutzsteine.push(boden);
            if !ctx.is_headless() {
                if let Some(&(mesh, glow)) = crate::asset_files::load_variants(ctx, "bauten", "schutzstein", Vec3::ONE, 0.0).first() {
                    let transform = Transform::from_position(boden).with_rotation(Quat::from_rotation_y(richtung.x.atan2(richtung.y)));
                    ctx.scene.spawn(Entity::new("Schutzstein", mesh).with_transform(transform));
                    if let Some(glow) = glow {
                        ctx.scene.spawn(Entity::new("Schutzstein (leuchtet)", glow).with_transform(transform).with_material(Material::Emissive { glow: 2.2 }));
                    }
                }
                world.places.lights.push((boden + Vec3::Y * 6.0, vec3(0.6, 1.4, 2.4), 12.0));
                world.places.labels.push(("Schutzstein", p));
            }
        }
        // Die Heerstraßen gepflastert, mit Randsteinen, Meilensteinen und Laternen
        if !ctx.is_headless() {
            let strassen: Vec<Vec<Vec2>> = world.heer.strassen().collect();
            let terrain = &world.terrain;
            crate::strassenbild::bauen(ctx, &|p| terrain.height_at(p.x, p.y), &strassen, &mut world.places.lights);
        }
        for (id, spec) in island.resources {
            let health = spec.max_health;
            world.resources.insert(
                id,
                Resource {
                    spec,
                    health,
                    entity: None,
                    body: None,
                    regrows_at: None,
                    shake: 0.0,
                    shown_health: health,
                    strike: None,
                    pop: 0.0,
                    grow: 1.0,
                    size: 1.0,
                },
            );
            world.place_resource(ctx, id);
        }
        world
    }

    // ---------- Gebäude ----------

    /// Ein Gebäude aufstellen (neu gebaut, aus dem Spielstand oder vom Server gemeldet).
    pub fn place_building(&mut self, ctx: &mut Context, building: crate::bauten::Building) {
        if self.buildings.iter().any(|b| b.id == building.id) {
            return;
        }
        let mut marker = Entity::new("Gebäude", ctx.assets.cube());
        marker.visible = false;
        let marker = ctx.scene.spawn(marker);
        let bodies = crate::bauten::add_colliders(ctx, &building, marker);
        self.building_bodies.insert(building.id, (marker, bodies));
        if !ctx.is_headless() {
            let by_entity = &self.by_entity;
            self.bau.add_site(ctx, &building, &|id| !by_entity.contains_key(&id));
        }
        self.buildings.push(building);
    }

    /// Ein Gebäude abreißen (Hindernisse, Darstellung, Eintrag).
    pub fn remove_building(&mut self, ctx: &mut Context, id: u32) {
        if let Some((marker, bodies)) = self.building_bodies.remove(&id) {
            for body in bodies {
                ctx.physics.remove_body(body);
            }
            ctx.scene.despawn(marker);
        }
        if !ctx.is_headless() {
            self.bau.remove_site(ctx, id);
        }
        self.buildings.retain(|b| b.id != id);
    }

    /// Ein Gebäude durch seinen neuen Stand ersetzen (Turm aufgewertet: neues Modell, Gerüst).
    pub fn replace_building(&mut self, ctx: &mut Context, building: crate::bauten::Building) {
        self.remove_building(ctx, building.id);
        self.place_building(ctx, building);
    }

    /// Welches Gebäude gehört zu diesem (unsichtbaren) Objekt? Zum Anvisieren.
    pub fn building_at(&self, entity: EntityId) -> Option<u32> {
        self.building_bodies.iter().find(|(_, (marker, _))| *marker == entity).map(|(&id, _)| id)
    }

    /// Baufortschritt, ein Takt (auf Server und Clients gleich schnell).
    pub fn advance_buildings(&mut self, dt: f32) {
        for building in &mut self.buildings {
            if building.progress < 1.0 {
                building.progress = (building.progress + dt / building.kind.build_seconds(building.level)).min(1.0);
            }
        }
    }

    /// Wetter erzwingen (Index in `protocol::WETTER`, 0 = wieder automatisch).
    pub fn set_weather(&mut self, choice: u8) {
        if choice == self.weather_choice {
            return;
        }
        self.weather_choice = choice;
        match crate::protocol::WETTER.get(choice as usize) {
            Some(&name) if choice > 0 => self.weather.force_named(name),
            _ => self.weather.force = None,
        }
    }

    /// Gegner unter dem Fadenkreuz: Art, Lebenspunkte, Mitte, Zustände.
    pub fn aimed_enemy(&self, from: Vec3, direction: Vec3, max: f32) -> Option<(crate::heer::EnemyKind, u8, Vec3, u16)> {
        self.heer_ansicht.aimed(from, direction, max)
    }

    /// Schadenszahlen über den Einheiten und die Bosse, die zu sehen sind (für die Oberfläche).
    pub fn schadenszahlen(&self) -> &[crate::heer::Schadenszahl] {
        &self.heer_ansicht.zahlen
    }

    pub fn sichtbare_bosse(&self) -> Vec<(&'static str, u8)> {
        self.heer_ansicht.bosse()
    }

    /// Neuen Stand der Verteidigung übernehmen (Statistik nur, wenn sie mitgekommen ist).
    pub fn td_uebernehmen(&mut self, mut td: crate::td::TdStand) {
        if !td.turm_stats.is_empty() {
            self.td_stats = td.turm_stats.iter().map(|&(id, k, s)| (id, (k, s))).collect();
        }
        if !td.beitrag.is_empty() {
            self.td_beitrag = std::mem::take(&mut td.beitrag);
            self.td_beitrag.sort_by(|a, b| b.1.cmp(&a.1));
        }
        td.turm_stats.clear();
        self.td = td;
    }

    /// Vorschau beim Platzieren (Art, Ort, Drehung, passt?) – `None` blendet sie aus.
    pub fn set_build_preview(&mut self, ctx: &mut Context, preview: Option<(crate::bauten::BuildingKind, Vec3, f32, bool)>) {
        if !ctx.is_headless() {
            self.bau.set_ghost(ctx, preview);
        }
    }

    // ---------- Tiere ----------

    /// Ein Takt Tier-Verhalten (nur auf dem Server): grasen, umherstreifen, fliehen.
    pub fn think_animals(&mut self, ctx: &Context) {
        let players: Vec<Vec3> = self.players.values().map(|a| ctx.physics.character_position(a.character)).collect();
        for animal in &mut self.animals {
            animal.think(Physics::FIXED_DT, &players, &self.terrain);
        }
    }

    // ---------- Spieler ----------

    pub fn spawn_player(&mut self, ctx: &mut Context, id: PlayerId, name: &str, class: CharacterClass, position: Vec3) {
        if self.players.contains_key(&id) {
            return;
        }
        // Das Objekt der Kapsel trägt Position und Blickrichtung; sichtbar ist die animierte Figur.
        let mut root = Entity::new(format!("Spieler {id}"), self.capsule)
            .with_transform(Transform::from_position(position))
            .with_color(player_color(id).extend(1.0));
        root.visible = ctx.is_headless();
        let entity = ctx.scene.spawn(root);
        if !ctx.is_headless() {
            self.puppets.insert(id, Puppet::new(ctx, class, entity));
        }
        let character = ctx.physics.add_character(entity, position, CharacterSettings::default());
        self.players.insert(
            id,
            Avatar {
                entity,
                character,
                facing: 0.0,
                tool: Tool::default(),
                last_cast_tick: 0,
                last_harvest_tick: 0,
                name: name.to_string(),
                class,
                noclip: false,
            },
        );
        self.inventories.entry(id).or_default();
        log::info!("{name} ({id}) ist da");
    }

    pub fn remove_player(&mut self, ctx: &mut Context, id: PlayerId) {
        if let Some(avatar) = self.players.remove(&id) {
            ctx.physics.remove_character(avatar.character);
            ctx.scene.despawn(avatar.entity);
            self.inventories.remove(&id);
            self.puppets.remove(&id);
            log::info!("{} ({id}) ist weg", avatar.name);
        }
    }

    /// Bewegt eine Spielfigur einen Takt weit. Läuft auf dem Server für alle Spieler und
    /// auf dem Client zusätzlich für die eigene Figur (Vorhersage).
    pub fn apply_input(&mut self, ctx: &mut Context, id: PlayerId, input: &PlayerInput) {
        // Beim Abbauen zum Rohstoff drehen (wie beim Zaubern zum Ziel).
        let harvest_target = input.harvest.and_then(|r| self.resources.get(&r)).map(|r| r.spec.transform.position);
        let Some(avatar) = self.players.get_mut(&id) else { return };
        avatar.tool = input.tool;
        avatar.noclip = input.noclip;
        let wish = vec3(input.wish.x, 0.0, input.wish.y).clamp_length_max(1.0);
        let speed = if input.sprint { SPRINT_SPEED } else { WALK_SPEED };
        if input.noclip {
            // Frei fliegen, durch alles hindurch (Admin)
            let fly = (wish * 3.0 + Vec3::Y * input.rise.clamp(-1.0, 1.0) * 1.6) * speed;
            let position = ctx.physics.character_position(avatar.character) + fly * Physics::FIXED_DT;
            ctx.physics.teleport_character(avatar.character, position);
        } else {
            ctx.physics.drive_character(avatar.character, wish * speed, input.jump);
        }
        if wish.length_squared() > 0.01 {
            avatar.facing = wish.x.atan2(-wish.z);
        }
        // Beim Zaubern und Abbauen zum Ziel drehen.
        if let Some(target) = input.cast.or(harvest_target) {
            let from = ctx.physics.character_position(avatar.character);
            let to = target - from;
            if vec2(to.x, to.z).length_squared() > 0.01 {
                avatar.facing = to.x.atan2(-to.z);
            }
        }
    }

    /// Lässt die Figur eines Spielers eine Aktion ausführen (nur Optik).
    pub fn play_action(&mut self, player: PlayerId, action: Action) {
        if let Some(puppet) = self.puppets.get_mut(&player) {
            puppet.act(action);
            if action == Action::Cast {
                self.sound_events.push(SoundEvent::Cast { player });
            }
        }
    }

    // ---------- Zauber ----------

    /// Wo der Zauber losfliegt: an der Spitze des Stabs, rechts vor dem Magier.
    pub fn cast_origin(&self, ctx: &Context, player: PlayerId, target: Vec3) -> Option<Vec3> {
        let avatar = self.players.get(&player)?;
        let center = ctx.physics.character_position(avatar.character);
        let to = target - center;
        let facing = if vec2(to.x, to.z).length_squared() > 0.01 { to.x.atan2(-to.z) } else { avatar.facing };
        let forward = vec3(facing.sin(), 0.0, -facing.cos());
        let right = vec3(facing.cos(), 0.0, facing.sin());
        Some(center + Vec3::Y * 0.75 + forward * 0.65 + right * 0.3)
    }

    /// Verfolgt einen Strahl bis zum ersten Treffer: Tier, Boden, Baum oder Fels.
    /// Liefert den Punkt und – falls es ein Tier war – dessen ID.
    pub fn spell_target(&self, ctx: &Context, from: Vec3, direction: Vec3, max_distance: f32, ignore: Option<PlayerId>) -> (Vec3, Option<u16>) {
        let direction = direction.normalize_or(Vec3::NEG_Z);
        let ignore = ignore.and_then(|p| self.players.get(&p)).map(|a| a.character);
        let mut nearest = ctx.physics.raycast(from, direction, max_distance, ignore).map_or(max_distance, |(_, d)| d);
        let mut animal = None;
        for (id, candidate) in self.animals.iter().enumerate().filter(|(_, a)| a.is_alive()) {
            let (center, radius) = candidate.hit_sphere();
            let along = (center - from).dot(direction);
            if along <= 0.0 || along - radius > nearest {
                continue;
            }
            let miss = (from + direction * along).distance_squared(center);
            if miss < radius * radius {
                let entry = (along - (radius * radius - miss).sqrt()).max(0.0);
                if entry < nearest {
                    nearest = entry;
                    animal = Some(id as u16);
                }
            }
        }
        (from + direction * nearest, animal)
    }

    /// Ein Spieler zaubert (nur Optik): Animation, Klang und das fliegende Geschoss.
    pub fn cast_spell(&mut self, ctx: &mut Context, player: PlayerId, origin: Vec3, target: Vec3, hit: bool, animate: bool) {
        if animate {
            self.play_action(player, Action::Cast);
        } else {
            self.sound_events.push(SoundEvent::Cast { player });
        }
        if ctx.is_headless() {
            return;
        }
        let mut entity = Entity::new("Zauber", ctx.assets.sphere())
            .with_transform(Transform::from_position(origin).with_scale(Vec3::splat(0.0)))
            .with_color(vec4(0.14, 0.2, 1.0, 1.0))
            .with_material(Material::Emissive { glow: 1.6 });
        entity.visible = false;
        let entity = ctx.scene.spawn(entity);
        self.bolts.push(Bolt { entity, origin, target, age: 0.0, hit });
    }

    /// Ein Tier wurde getroffen: Lebensstand übernehmen und – falls gewünscht – Effekte zeigen.
    pub fn animal_hit(&mut self, ctx: &mut Context, id: u16, health: u8, effects: bool) {
        let Some(animal) = self.animals.get_mut(id as usize) else { return };
        animal.set_health(health, effects);
        if !effects || ctx.is_headless() {
            return;
        }
        let (center, radius) = animal.hit_sphere();
        let killed = health == 0;
        ctx.particles.burst(Burst {
            position: center,
            count: if killed { 40 } else { 18 },
            color: vec3(0.6, 0.45, 1.0),
            color_variation: 0.35,
            speed: if killed { 4.0 } else { 3.0 },
            direction: Vec3::Y * 0.5,
            size: 0.1 + radius * 0.05,
            life: 0.9,
            gravity: 0.5,
            glow: 4.0,
            grow: 0.0,
            round: true,
        });
        self.sound_events.push(SoundEvent::Impact { at: center, animal: true, killed });
    }

    /// Zaubergeschosse bewegen, leuchten lassen und am Ziel verpuffen lassen.
    fn update_bolts(&mut self, ctx: &mut Context) {
        let dt = ctx.time.delta;
        let mut finished = Vec::new();
        for (index, bolt) in self.bolts.iter_mut().enumerate() {
            bolt.age += dt;
            let length = bolt.origin.distance(bolt.target).max(0.01);
            let flight = bolt.age - CAST_DELAY;
            if flight < 0.0 {
                // Ausholen: Funken sammeln sich an der Stabspitze.
                if self.effects_rng.chance(0.6) {
                    ctx.particles.burst(Burst {
                        position: bolt.origin,
                        count: 2,
                        color: vec3(0.5, 0.6, 1.0),
                        color_variation: 0.3,
                        speed: 0.8,
                        direction: Vec3::ZERO,
                        size: 0.06,
                        life: 0.3,
                        gravity: -0.5,
                        glow: 5.0,
                        grow: 0.0,
                        round: true,
                    });
                }
                ctx.lights.push(PointLight { position: bolt.origin, color: vec3(0.6, 0.7, 2.0) * (bolt.age / CAST_DELAY), radius: 4.0 });
                continue;
            }
            let progress = (flight * BOLT_SPEED / length).min(1.0);
            let position = bolt.origin.lerp(bolt.target, progress);
            let pulse = 1.0 + (bolt.age * 40.0).sin() * 0.15;
            if let Some(entity) = ctx.scene.try_get_mut(bolt.entity) {
                entity.visible = true;
                entity.transform.position = position;
                entity.transform.scale = Vec3::splat(0.42 * pulse);
            }
            ctx.lights.push(PointLight { position, color: vec3(0.7, 0.8, 3.0), radius: 7.0 });
            // Leuchtspur
            ctx.particles.burst(Burst {
                position,
                count: 5,
                color: vec3(0.45, 0.4, 1.0),
                color_variation: 0.4,
                speed: 0.6,
                direction: Vec3::ZERO,
                size: 0.14,
                life: 0.45,
                gravity: -0.2,
                glow: 4.0,
                grow: 0.0,
                round: true,
            });
            if progress >= 1.0 {
                finished.push(index);
                if !bolt.hit {
                    // Verpufft am Boden, an einem Baum oder in der Luft.
                    ctx.particles.burst(Burst {
                        position,
                        count: 16,
                        color: vec3(0.55, 0.6, 1.0),
                        color_variation: 0.3,
                        speed: 2.5,
                        direction: Vec3::Y * 0.3,
                        size: 0.08,
                        life: 0.6,
                        gravity: 1.0,
                        glow: 4.0,
                        grow: 0.0,
                        round: true,
                    });
                    self.sound_events.push(SoundEvent::Impact { at: position, animal: false, killed: false });
                }
            }
        }
        for index in finished.into_iter().rev() {
            let bolt = self.bolts.swap_remove(index);
            ctx.scene.despawn(bolt.entity);
        }
    }

    pub fn player_position(&self, ctx: &Context, id: PlayerId) -> Option<Vec3> {
        self.players.get(&id).map(|a| ctx.physics.character_position(a.character))
    }

    // ---------- Geworfene Objekte ----------

    pub fn spawn_object(&mut self, ctx: &mut Context, id: NetId, kind: ObjectKind, position: Vec3, velocity: Vec3) {
        match kind {
            ObjectKind::Ball => {
                let transform = Transform::from_position(position).with_scale(Vec3::splat(0.4));
                let entity = ctx.scene.spawn(
                    Entity::new("Ball", ctx.assets.sphere()).with_transform(transform).with_color(vec4(1.0, 0.45, 0.05, 1.0)),
                );
                let body = ctx.physics.add_body(
                    entity,
                    &transform,
                    BodyDesc::dynamic(Shape::Sphere { radius: 0.2 }).with_density(3.0).with_restitution(0.4).with_velocity(velocity),
                );
                self.objects.insert(id, NetObject { entity, body, kind });
            }
        }
    }

    pub fn remove_object(&mut self, ctx: &mut Context, id: NetId) {
        if let Some(object) = self.objects.remove(&id) {
            ctx.physics.remove_body(object.body);
            ctx.scene.despawn(object.entity);
        }
    }

    // ---------- Rohstoffe ----------

    fn place_resource(&mut self, ctx: &mut Context, id: u32) {
        let Some(resource) = self.resources.get_mut(&id) else { return };
        let spec = &resource.spec;
        let entity = ctx.scene.spawn(
            Entity::new(spec.name, spec.mesh).with_transform(spec.transform).with_color(spec.color).with_material(spec.material),
        );
        if let Some(glow) = spec.glow_part {
            ctx.scene.spawn(Entity::new("Leuchtfrüchte", glow).with_parent(entity).with_material(Material::Emissive { glow: 2.2 }));
        }
        let collider = Transform::from_position(spec.transform.position + spec.collider_offset).with_rotation(spec.transform.rotation);
        resource.body = Some(ctx.physics.add_body(entity, &collider, BodyDesc::fixed(spec.collider)));
        resource.entity = Some(entity);
        self.by_entity.insert(entity, id);
    }

    fn remove_resource_visual(&mut self, ctx: &mut Context, id: u32) {
        let Some(resource) = self.resources.get_mut(&id) else { return };
        if let Some(body) = resource.body.take() {
            ctx.physics.remove_body(body);
        }
        resource.strike = None;
        if let Some(entity) = resource.entity.take() {
            ctx.scene.despawn(entity);
            self.by_entity.remove(&entity);
        }
    }

    /// Welcher Rohstoff gehört zu diesem Objekt der Szene?
    pub fn resource_at(&self, entity: EntityId) -> Option<u32> {
        self.by_entity.get(&entity).copied()
    }

    /// Ist der Spieler nah genug, um den Rohstoff zu bearbeiten? `slack` gibt dem Server
    /// etwas Spielraum, weil Client und Server die Figur leicht versetzt sehen.
    pub fn in_reach(&self, ctx: &Context, player: PlayerId, id: u32, slack: f32) -> bool {
        let (Some(position), Some(resource)) = (self.player_position(ctx, player), self.resources.get(&id)) else { return false };
        let base = resource.spec.transform.position;
        let horizontal = vec2(position.x - base.x, position.z - base.z).length();
        resource.is_present() && horizontal <= resource.radius() + HARVEST_REACH + slack && (position.y - base.y).abs() < 4.0
    }

    /// Ein Treffer ist angekommen: Zustand übernehmen und – falls gewünscht – Effekte zeigen.
    /// Bei Vorkommen kommen die Effekte erst, wenn die Spitzhacke in der Animation auftrifft.
    pub fn resource_hit(&mut self, ctx: &mut Context, id: u32, health: u8, effects: bool) {
        let Some(resource) = self.resources.get_mut(&id) else { return };
        if !resource.is_present() {
            return;
        }
        resource.health = health;
        if health == 0 {
            resource.regrows_at = Some(ctx.time.tick + RESPAWN_TICKS);
        }
        let delayed = !ctx.is_headless();
        if delayed && (effects || resource.strike.is_some()) {
            // Das Modell zeigt den Stand erst beim Auftreffen; weg ist es dann auch erst dort.
            if effects && resource.strike.is_none() {
                resource.strike = Some(strike_delay(resource.spec.kind));
            }
            if health == 0 {
                if let Some(body) = resource.body.take() {
                    ctx.physics.remove_body(body);
                }
            }
            return;
        }
        if !delayed || !effects {
            resource.shown_health = health;
        }
        if effects {
            resource.shake = 0.35;
            hit_particles(ctx, &resource.spec, health == 0);
            if !ctx.is_headless() {
                let (kind, at) = (resource.spec.kind, resource.spec.transform.position);
                self.sound_events.push(SoundEvent::Hit { kind, at, finished: health == 0 });
            }
        }
        if health == 0 {
            self.remove_resource_visual(ctx, id);
        }
    }

    /// Die Spitzhacke trifft (Optik): Wackeln, Stauchen, Splitter, Klang – und ist das Vorkommen
    /// erschöpft, zerfällt es.
    fn strike(&mut self, ctx: &mut Context, id: u32) {
        let Some(resource) = self.resources.get_mut(&id) else { return };
        resource.strike = None;
        resource.shown_health = resource.health;
        resource.shake = 0.3;
        resource.pop = 1.0;
        let finished = !resource.is_present();
        hit_particles(ctx, &resource.spec, finished);
        let (kind, at) = (resource.spec.kind, resource.spec.transform.position);
        self.sound_events.push(SoundEvent::Hit { kind, at, finished });
        if finished {
            self.remove_resource_visual(ctx, id);
        }
    }

    /// Nur Optik: Wackeln und Splitter sofort zeigen, bevor der Server antwortet.
    pub fn preview_hit(&mut self, ctx: &mut Context, id: u32) {
        if let Some(resource) = self.resources.get_mut(&id).filter(|r| r.is_present()) {
            if resource.strike.is_none() && !ctx.is_headless() {
                resource.strike = Some(strike_delay(resource.spec.kind));
            }
        }
    }

    /// Rohstoff ist nachgewachsen.
    pub fn resource_back(&mut self, ctx: &mut Context, id: u32) {
        let Some(resource) = self.resources.get_mut(&id) else { return };
        if resource.is_present() {
            return;
        }
        resource.regrows_at = None;
        resource.health = resource.spec.max_health;
        resource.shown_health = resource.health;
        // Ein Rest vom Zerfallen könnte noch zu sehen sein.
        if resource.entity.is_some() {
            self.remove_resource_visual(ctx, id);
        }
        if let Some(resource) = self.resources.get_mut(&id) {
            // Wächst sichtbar aus dem Boden (ohne Fenster sofort fertig).
            resource.grow = if ctx.is_headless() { 1.0 } else { 0.0 };
            resource.size = 1.0;
        }
        self.place_resource(ctx, id);
    }

    /// Kristallvorkommen in der Nähe: blaues Licht (nachts kräftiger) und aufsteigende Funken.
    fn crystal_glow(&mut self, ctx: &mut Context) {
        let camera = ctx.camera.position;
        let night = ctx.env.sky.stars;
        let dt = ctx.time.delta;
        for &crystal in &self.crystals {
            let distance = crystal.distance(camera);
            if distance > 60.0 {
                continue;
            }
            let pulse = 0.85 + 0.15 * (ctx.time.elapsed * 1.3 + crystal.x).sin();
            ctx.lights.push(PointLight { position: crystal + Vec3::Y * 1.0, color: vec3(0.25, 0.55, 1.6) * (0.35 + night * 1.3) * pulse, radius: 8.0 });
            if distance < 30.0 && self.effects_rng.chance(dt * 3.0) {
                let rng = &mut self.effects_rng;
                let offset = vec3(rng.range(-0.6, 0.6), rng.range(0.3, 1.4), rng.range(-0.6, 0.6));
                ctx.particles.burst(Burst {
                    position: crystal + offset,
                    count: 1,
                    color: vec3(0.45, 0.8, 1.0),
                    color_variation: 0.15,
                    speed: 0.25,
                    direction: Vec3::Y * 0.5,
                    size: 0.05,
                    life: 2.2,
                    gravity: -0.35,
                    glow: 5.0,
                    grow: 0.0,
                    round: true,
                });
            }
        }
    }

    /// Brandung an flachen Stränden in der Nähe: Wellen laufen als Schaumlinie auf den Sand,
    /// alle paar Sekunden eine neue, jede Stelle zu ihrer eigenen Zeit.
    fn surf(&mut self, ctx: &mut Context) {
        let camera = ctx.camera.position;
        let t = ctx.time.elapsed;
        let dt = ctx.time.delta;
        for (i, &(at, inland)) in self.places.surf.iter().enumerate() {
            if at.distance(camera) > 55.0 {
                continue;
            }
            let period = 5.5 + (i % 5) as f32 * 0.4;
            let phase = ((t + i as f32 * 1.37) / period).fract();
            let previous = ((t - dt + i as f32 * 1.37) / period).fract();
            // Kurz bevor die Welle ankommt: eine Reihe Schaumtropfen längs der Wasserlinie
            if previous < 0.8 && phase >= 0.8 {
                let along = vec3(-inland.y, 0.0, inland.x);
                let forward = vec3(inland.x, 0.0, inland.y);
                for k in -3..=3 {
                    ctx.particles.burst(Burst {
                        position: at + along * (k as f32 * 0.9) + forward * -1.2 + Vec3::Y * 0.12,
                        count: 2,
                        color: vec3(0.92, 0.96, 1.0),
                        color_variation: 0.04,
                        speed: 0.9,
                        direction: forward * 1.4 + Vec3::Y * 0.4,
                        size: 0.2,
                        life: 1.5,
                        gravity: 0.8,
                        glow: 0.2,
                        grow: 1.2,
                        round: true,
                    });
                }
            }
        }
    }

    /// Lampen der Sehenswürdigkeiten (nachts kräftiger), schaukelnde Boote, Windmühlen.
    fn place_lights(&mut self, ctx: &mut Context) {
        let night = ctx.env.sky.stars;
        let camera = ctx.camera.position;
        for &(at, color, radius) in &self.places.lights {
            if at.distance(camera) < 90.0 {
                ctx.lights.push(PointLight { position: at, color: color * (0.25 + night * 1.2), radius });
            }
        }
        let t = ctx.time.elapsed;
        // Windmühlenflügel drehen sich gemächlich im Wind
        for &(entity, hub, rotation) in &self.places.windmills {
            if hub.distance(camera) < 500.0 {
                if let Some(sails) = ctx.scene.try_get_mut(entity) {
                    sails.transform.rotation = rotation * Quat::from_rotation_x(-t * 0.55);
                }
            }
        }
        for &(entity, base, rotation) in &self.places.boats {
            if let Some(boat) = ctx.scene.try_get_mut(entity) {
                boat.transform.position = base + Vec3::Y * ((t * 1.3).sin() * 0.04);
                boat.transform.rotation = rotation * Quat::from_rotation_x((t * 1.1).sin() * 0.04) * Quat::from_rotation_z((t * 0.8).sin() * 0.02);
            }
        }
    }

    /// Lagerfeuer in der Nähe: Flammen, Funken, Rauch, flackerndes warmes Licht und Knistern.
    fn campfires(&mut self, ctx: &mut Context) {
        let camera = ctx.camera.position;
        let night = ctx.env.sky.stars;
        let t = ctx.time.elapsed;
        let dt = ctx.time.delta;
        self.crackle_timer -= dt;
        for &fire in &self.places.fires {
            let distance = fire.distance(camera);
            if distance > 80.0 {
                continue;
            }
            // Flackern aus mehreren Sinuswellen
            let flicker = 0.82 + 0.1 * (t * 11.0).sin() + 0.06 * (t * 23.0 + 1.3).sin() + 0.04 * (t * 37.0).sin();
            ctx.lights.push(PointLight { position: fire + Vec3::Y * 0.7, color: vec3(3.0, 1.35, 0.45) * (0.5 + night * 1.1) * flicker, radius: 11.0 });
            if distance > 45.0 {
                continue;
            }
            let rng = &mut self.effects_rng;
            // Flammen: viele kurze, leuchtende Teilchen, die aufsteigen und kleiner werden
            for _ in 0..2 {
                let offset = vec3(rng.range(-0.22, 0.22), rng.range(0.08, 0.25), rng.range(-0.22, 0.22));
                ctx.particles.burst(Burst {
                    position: fire + offset,
                    count: 1,
                    color: vec3(1.0, 0.28, 0.03).lerp(vec3(1.0, 0.55, 0.1), rng.range(0.0, 1.0)),
                    color_variation: 0.06,
                    speed: 0.3,
                    direction: Vec3::Y * 1.2,
                    size: rng.range(0.11, 0.2),
                    life: rng.range(0.4, 0.65),
                    gravity: -2.0,
                    glow: 1.6,
                    grow: 0.0,
                    round: true,
                });
            }
            // Funken
            if rng.chance(dt * 6.0) {
                ctx.particles.burst(Burst {
                    position: fire + Vec3::Y * 0.3,
                    count: 1,
                    color: vec3(1.0, 0.7, 0.3),
                    color_variation: 0.1,
                    speed: 1.2,
                    direction: Vec3::Y * 1.5,
                    size: 0.035,
                    life: 1.4,
                    gravity: -0.8,
                    glow: 7.0,
                    grow: 0.0,
                    round: false,
                });
            }
            // Rauch: weich, grau, wächst und zieht langsam nach oben
            if rng.chance(dt * 2.5) {
                ctx.particles.burst(Burst {
                    position: fire + Vec3::Y * 1.1,
                    count: 1,
                    color: vec3(0.14, 0.135, 0.13),
                    color_variation: 0.04,
                    speed: 0.2,
                    direction: vec3(0.15, 1.0, 0.05),
                    size: 0.16,
                    life: 3.0,
                    gravity: -0.4,
                    glow: 0.0,
                    grow: 2.0,
                    round: true,
                });
            }
            if distance < 25.0 && self.crackle_timer <= 0.0 {
                self.crackle_timer = rng.range(0.25, 0.9);
                self.sound_events.push(SoundEvent::Crackle { at: fire + Vec3::Y * 0.3 });
            }
        }
    }

    /// Nur für Screenshots: Position des nächsten Vogelschwarms.
    pub fn nearest_flock(&self, ctx: &Context, from: Vec3) -> Option<Vec3> {
        self.wildlife.as_ref().and_then(|w| w.nearest_flock(ctx, from))
    }

    pub fn visible_butterflies(&self, ctx: &Context) -> usize {
        self.wildlife.as_ref().map_or(0, |w| w.visible_butterflies(ctx))
    }

    /// Mitte der Kristallvorkommen (am Boden).
    pub fn crystals(&self) -> &[Vec3] {
        &self.crystals
    }

    /// Glühwürmchen in der Nacht rund um die Kamera, über Wiesen und im Wald.
    fn fireflies(&mut self, ctx: &mut Context) {
        let night = ctx.env.sky.stars;
        if night < 0.3 {
            return;
        }
        self.firefly_timer -= ctx.time.delta;
        while self.firefly_timer < 0.0 {
            self.firefly_timer += 0.12 / night;
            let rng = &mut self.effects_rng;
            let offset = vec2(rng.range(-28.0, 28.0), rng.range(-28.0, 28.0));
            let (x, z) = (ctx.camera.position.x + offset.x, ctx.camera.position.z + offset.y);
            let ground = self.terrain.height_at(x, z);
            if !(2.0..20.0).contains(&ground) {
                continue;
            }
            let color = if rng.chance(0.2) { vec3(0.35, 0.9, 1.0) } else { vec3(0.85, 1.0, 0.3) };
            ctx.particles.burst(Burst {
                position: vec3(x, ground + rng.range(0.4, 2.2), z),
                count: 1,
                color,
                color_variation: 0.1,
                speed: 0.5,
                direction: Vec3::ZERO,
                size: 0.09,
                life: 5.0,
                gravity: -0.03,
                glow: 5.0,
                grow: 0.0,
                round: false,
            });
        }
    }

    /// Tageszeit auf Licht und Himmel anwenden, Figuren drehen und animieren,
    /// getroffene Rohstoffe wackeln lassen.
    pub fn update_visuals(&mut self, ctx: &mut Context) {
        self.day.apply(&mut ctx.env);
        self.weather.apply(ctx, self.day.day, self.day.hour);
        // Im düsteren Land um die Schattenfestung: fahles Licht, violetter Dunst
        let d = island::duester(vec2(ctx.camera.position.x, ctx.camera.position.z));
        if d > 0.0 {
            let env = &mut ctx.env;
            let dunst = vec3(0.16, 0.12, 0.2);
            env.sun_color *= 1.0 - 0.45 * d;
            env.sky_ambient = env.sky_ambient.lerp(env.sky_ambient * vec3(0.7, 0.6, 0.85), d);
            env.ground_ambient *= 1.0 - 0.35 * d;
            env.sky_color = env.sky_color.lerp(dunst, 0.55 * d);
            env.zenith_color = env.zenith_color.lerp(vec3(0.1, 0.07, 0.14), 0.45 * d);
            env.fog_density += 0.0045 * d;
        }
        if let Some(volume) = self.weather.take_thunder(ctx.time.delta) {
            self.sound_events.push(SoundEvent::Thunder { volume });
        }
        use crate::messung::messen;
        messen("glühwürmchen", || self.fireflies(ctx));
        messen("kristalle", || self.crystal_glow(ctx));
        messen("feuer", || self.campfires(ctx));
        messen("lichter", || self.place_lights(ctx));
        if self.wildlife.is_none() && !ctx.is_headless() {
            let (forests, beaches) = crate::island::wildlife_spots(&self.terrain);
            self.wildlife = Some(crate::leben::Wildlife::new(ctx, &forests, &beaches));
        }
        if let Some(wildlife) = &mut self.wildlife {
            let terrain = &self.terrain;
            messen("kleinleben", || wildlife.update(ctx, &|x, z| terrain.height_at(x, z), &|p| crate::island::is_meadow(terrain, p)));
        }
        messen("brandung", || self.surf(ctx));
        if self.wachen.is_none() && !ctx.is_headless() {
            self.wachen = crate::wachen::Wachen::new(ctx);
        }
        if let Some(wachen) = &mut self.wachen {
            messen("wachen", || wachen.update(ctx, &self.day));
        }
        for kind in self.bau.update(ctx, &self.buildings, &mut self.sound_events) {
            let text = match kind.produces() {
                Some(item) => format!("{} ist fertig gebaut und liefert jetzt {}.", kind.label(), item.label()),
                None => format!("{} ist bereit und verteidigt die Straße.", kind.label()),
            };
            self.chat_events.push(ChatLine::notice(text));
        }
        // Durchbruch: die Schutzsteine blitzen auf
        if self.td.leben < self.shown_leben && self.td.welle > 0 {
            for &stein in &self.schutzsteine {
                ctx.particles.burst(Burst {
                    position: stein + Vec3::Y * 5.8,
                    count: 50,
                    color: vec3(1.0, 0.35, 0.3),
                    color_variation: 0.2,
                    speed: 5.0,
                    direction: Vec3::Y,
                    size: 0.14,
                    life: 1.2,
                    gravity: 2.0,
                    glow: 4.0,
                    grow: 0.0,
                    round: true,
                });
            }
        }
        self.shown_leben = self.td.leben;
        if !ctx.is_headless() {
            for ereignis in std::mem::take(&mut self.ereignisse) {
                crate::td_ansicht::ereignis(ctx, &mut self.sound_events, &mut self.felder, ereignis);
            }
            crate::td_ansicht::felder(ctx, &mut self.felder, &mut self.effects_rng);
        } else {
            self.ereignisse.clear();
        }
        for shot in std::mem::take(&mut self.tower_shots) {
            if let Some(building) = self.buildings.iter().find(|b| b.id == shot.turm) {
                self.bau.shot(ctx, building, shot.ziel, &mut self.sound_events);
            }
        }
        if !ctx.is_headless() {
            self.heer_ansicht.update(ctx, &self.feinde, &self.td.soldaten, &mut self.sound_events);
            for (kind, from, target) in std::mem::take(&mut self.strikes) {
                strike_visual(ctx, &mut self.sound_events, kind, from, target);
            }
        } else {
            self.strikes.clear();
        }
        self.update_bolts(ctx);
        messen("tiere", || {
            for animal in &mut self.animals {
                animal.update_visual(ctx);
            }
        });
        let _figuren = std::time::Instant::now();
        let dt = ctx.time.delta;
        let blend = (dt * 12.0).min(1.0);
        for (id, avatar) in &self.players {
            let Some(entity) = ctx.scene.try_get_mut(avatar.entity) else { continue };
            let target = Quat::from_rotation_y(-avatar.facing);
            entity.transform.rotation = entity.transform.rotation.slerp(target, blend);
            let position = entity.transform.position;
            if let Some(puppet) = self.puppets.get_mut(id) {
                // Boden unter den Füßen aus der Physik (Burg, Festung, Brücken …), sonst das Gelände –
                // sonst hielte die Figur Böden von Bauwerken für Luft und schwebte statt zu gehen
                let ground = ctx
                    .physics
                    .raycast(position, Vec3::NEG_Y, 4.0, Some(avatar.character))
                    .map(|(_, d)| position.y - d)
                    .unwrap_or_else(|| self.terrain.height_at(position.x, position.z));
                puppet.update(ctx, position, ground);
            }
        }
        for (id, avatar) in &self.players {
            if let Some(puppet) = self.puppets.get_mut(id) {
                puppet.set_tool(avatar.tool);
            }
        }
        crate::messung::eintragen("figuren", _figuren);
        let _rohstoffe = std::time::Instant::now();

        // Schläge mit der Spitzhacke, die jetzt auftreffen
        let mut struck = Vec::new();
        for (&id, resource) in self.resources.iter_mut() {
            if let Some(left) = &mut resource.strike {
                *left -= dt;
                if *left <= 0.0 {
                    struck.push(id);
                }
            }
        }
        for id in struck {
            self.strike(ctx, id);
        }

        // Wackeln, Stauchen, Schrumpfen (Vorkommen werden mit jedem Schlag kleiner) und Nachwachsen
        for resource in self.resources.values_mut() {
            let target = if resource.spec.kind.needs_pickaxe() {
                0.55 + 0.45 * resource.shown_health as f32 / resource.spec.max_health.max(1) as f32
            } else {
                1.0
            };
            if resource.shake <= 0.0 && resource.pop <= 0.0 && resource.grow >= 1.0 && (resource.size - target).abs() < 0.001 {
                continue;
            }
            resource.shake = (resource.shake - dt).max(0.0);
            resource.pop = (resource.pop - dt * 4.0).max(0.0);
            resource.grow = (resource.grow + dt / 0.9).min(1.0);
            resource.size += (target - resource.size) * (dt * 10.0).min(1.0);
            if (resource.size - target).abs() < 0.001 {
                resource.size = target;
            }
            let Some(entity) = resource.entity.and_then(|e| ctx.scene.try_get_mut(e)) else { continue };
            let wobble = (ctx.time.elapsed * 45.0).sin() * resource.shake * 0.12;
            entity.transform.rotation = resource.spec.transform.rotation * Quat::from_rotation_x(wobble) * Quat::from_rotation_z(wobble * 0.6);
            // Aus dem Boden wachsen: schnell, mit leichtem Überschwingen
            let g = resource.grow;
            let grown = 1.0 - (1.0 - g).powi(3) + (g * std::f32::consts::PI).sin() * 0.08;
            let squash = (resource.pop * std::f32::consts::PI).sin() * 0.12;
            let scale = resource.spec.transform.scale * resource.size * grown;
            entity.transform.scale = scale * vec3(1.0 + squash * 0.5, 1.0 - squash, 1.0 + squash * 0.5);
        }
    }
}

/// Zeit vom Beginn der Animation bis zum Auftreffen des Werkzeugs.
fn strike_delay(kind: ResourceKind) -> f32 {
    if kind.needs_pickaxe() { MINE_STRIKE } else { CHOP_STRIKE }
}

/// Splitter, Blätter und Brocken beim Abbauen.
fn hit_particles(ctx: &mut Context, spec: &ResourceSpec, finished: bool) {
    let base = spec.transform.position;
    let scale = spec.transform.scale.y;
    let many = if finished { 3 } else { 1 };
    match spec.kind {
        ResourceKind::Wood => {
            ctx.particles.burst(Burst {
                position: base + Vec3::Y * 1.2 * scale,
                count: 10 * many,
                color: vec3(0.35, 0.22, 0.1),
                speed: 5.0,
                direction: Vec3::Y * 0.6,
                size: 0.12,
                ..Default::default()
            });
            let magic = spec.name == "Zauberbaum";
            ctx.particles.burst(Burst {
                position: base + Vec3::Y * 4.0 * scale,
                count: 8 * many,
                color: if magic { vec3(0.55, 0.2, 0.9) } else { vec3(0.15, 0.38, 0.08) },
                color_variation: 0.3,
                speed: 2.5,
                direction: Vec3::ZERO,
                size: 0.16,
                life: 2.2,
                gravity: 1.5,
                glow: if magic { 1.5 } else { 0.0 },
                grow: 0.0,
                round: false,
            });
        }
        ResourceKind::Stone | ResourceKind::Ore => {
            let ore = spec.kind == ResourceKind::Ore;
            let at = base + Vec3::Y * 0.6 * scale;
            // Splitter
            ctx.particles.burst(Burst {
                position: at,
                count: 12 * many,
                color: if ore { vec3(0.09, 0.085, 0.1) } else { vec3(0.36, 0.34, 0.31) },
                color_variation: 0.25,
                speed: if finished { 7.0 } else { 5.0 },
                direction: Vec3::Y * 0.8,
                size: if finished { 0.2 } else { 0.12 },
                life: 1.1,
                ..Default::default()
            });
            // Staubwolke
            ctx.particles.burst(Burst {
                position: at,
                count: 5 * many,
                color: if ore { vec3(0.3, 0.26, 0.24) } else { vec3(0.55, 0.52, 0.48) },
                color_variation: 0.1,
                speed: 1.2,
                direction: Vec3::Y * 0.3,
                size: 0.3,
                life: 1.4,
                gravity: -0.3,
                glow: 0.0,
                grow: 2.0,
                round: true,
            });
            if ore {
                // Rostrote Erzbrocken und Funken vom Eisen der Hacke
                ctx.particles.burst(Burst {
                    position: at,
                    count: 6 * many,
                    color: vec3(0.55, 0.16, 0.04),
                    color_variation: 0.2,
                    speed: 5.0,
                    direction: Vec3::Y * 0.8,
                    size: 0.12,
                    life: 1.1,
                    ..Default::default()
                });
                ctx.particles.burst(Burst {
                    position: at + Vec3::Y * 0.1,
                    count: 10 * many,
                    color: vec3(1.0, 0.65, 0.2),
                    color_variation: 0.15,
                    speed: 6.5,
                    direction: Vec3::Y * 0.5,
                    size: 0.04,
                    life: 0.45,
                    gravity: 9.0,
                    glow: 6.0,
                    grow: 0.0,
                    round: false,
                });
            }
        }
    }
}

/// Jeder Spieler bekommt eine eigene, gut unterscheidbare Farbe.
pub fn player_color(id: PlayerId) -> Vec3 {
    if id == HOST_PLAYER {
        return vec3(0.1, 0.3, 0.9);
    }
    let h = (id.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40) as f32 / (1u64 << 24) as f32;
    hue(h)
}

/// Farbton (0..1) in lineares RGB mit voller Sättigung.
pub fn hue(h: f32) -> Vec3 {
    let k = |n: f32| {
        let k = (n + h.fract() * 6.0) % 6.0;
        1.0 - k.min(4.0 - k).clamp(0.0, 1.0)
    };
    vec3(k(5.0), k(3.0), k(1.0)).powf(2.2) * 0.9
}

/// Ein Angriff der Festungstruppen (nur Optik): Pfeil, Zauber oder Hieb.
fn strike_visual(ctx: &mut Context, sounds: &mut Vec<SoundEvent>, kind: crate::heer::EnemyKind, from: Vec3, target: Vec3) {
    use crate::heer::EnemyKind;
    let target = target + Vec3::Y * 0.2;
    let (color, glow, streak) = match kind {
        EnemyKind::Archer => (vec3(0.45, 0.32, 0.2), 0.0, true),
        EnemyKind::Warlock => (vec3(0.8, 0.35, 1.0), 4.0, true),
        _ => (vec3(0.9, 0.85, 0.8), 1.5, false),
    };
    if streak {
        // Flugbahn als kurze Spur aus Funken
        let steps = (from.distance(target) / 0.8).clamp(4.0, 30.0) as usize;
        for i in 0..steps {
            let t = i as f32 / steps as f32;
            ctx.particles.burst(Burst {
                position: from.lerp(target, t) + Vec3::Y * (t * (1.0 - t) * from.distance(target) * 0.08),
                count: 1,
                color,
                color_variation: 0.1,
                speed: 0.1,
                direction: Vec3::ZERO,
                size: if glow > 0.0 { 0.12 } else { 0.06 },
                life: 0.25 + t * 0.2,
                gravity: 0.0,
                glow,
                grow: 0.0,
                round: glow > 0.0,
            });
        }
    }
    ctx.particles.burst(Burst {
        position: target,
        count: 10,
        color,
        color_variation: 0.2,
        speed: 2.5,
        direction: Vec3::Y * 0.5,
        size: 0.07,
        life: 0.4,
        gravity: 6.0,
        glow,
        grow: 0.0,
        round: false,
    });
    sounds.push(SoundEvent::Impact { at: target, animal: false, killed: false });
}

