//! Die Spielwelt, wie sie Server und Clients gleichermaßen aufbauen.

use std::collections::{BTreeMap, HashMap};

use engine::prelude::*;

use crate::animals::{self, Animal};
use crate::island::{self, ResourceKind, ResourceSpec};
use crate::characters::{Action, Puppet};
use crate::protocol::{CharacterClass, Inventory, NetId, ObjectKind, PlayerId, PlayerInput, HOST_PLAYER};

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
/// Takte zwischen zwei Schlägen auf einen Rohstoff.
pub const HARVEST_COOLDOWN_TICKS: u64 = 22;
/// Wie nah man einem Rohstoff sein muss (Meter vom Rand).
pub const HARVEST_REACH: f32 = 2.2;
/// Nach dieser Zeit wachsen Bäume nach und Felsen tauchen wieder auf (2 Minuten).
pub const RESPAWN_TICKS: u64 = 60 * 120;

pub struct Avatar {
    pub entity: EntityId,
    pub character: CharacterId,
    /// Blickrichtung (Yaw in Radiant), folgt der Laufrichtung.
    pub facing: f32,
    pub last_cast_tick: u64,
    pub last_harvest_tick: u64,
    pub name: String,
    pub class: CharacterClass,
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
    Step { at: Vec3, sand: bool, running: bool },
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

pub struct World {
    pub players: HashMap<PlayerId, Avatar>,
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
    /// Zurückgelegte Strecke seit dem letzten Schritt je Spieler.
    stride: HashMap<PlayerId, (Vec3, f32)>,
    /// Fliegende Zaubergeschosse (nur mit Fenster).
    bolts: Vec<Bolt>,
}

impl World {
    /// Baut die Insel. Auf allen Rechnern identisch, damit die IDs passen.
    pub fn new(ctx: &mut Context) -> Self {
        let settings = CharacterSettings::default();
        let capsule = ctx.assets.named_mesh("spielfigur", || MeshData::capsule(settings.radius, settings.height, 24, 8));
        let island = island::build(ctx);
        let animals = animals::populate(&island.terrain, island.spawn, island::SEED, island::moisture);
        let mut world = World {
            players: HashMap::new(),
            objects: BTreeMap::new(),
            resources: BTreeMap::new(),
            by_entity: HashMap::new(),
            inventories: HashMap::new(),
            puppets: HashMap::new(),
            spawn: island.spawn + Vec3::Y * 1.2,
            terrain: island.terrain,
            animals,
            day: DayCycle::default(),
            effects_rng: Rng::new(7),
            firefly_timer: 0.0,
            capsule,
            sound_events: Vec::new(),
            stride: HashMap::new(),
            bolts: Vec::new(),
        };
        for (id, spec) in island.resources {
            let health = spec.max_health;
            world.resources.insert(id, Resource { spec, health, entity: None, body: None, regrows_at: None, shake: 0.0 });
            world.place_resource(ctx, id);
        }
        world
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
            Avatar { entity, character, facing: 0.0, last_cast_tick: 0, last_harvest_tick: 0, name: name.to_string(), class },
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
        let Some(avatar) = self.players.get_mut(&id) else { return };
        let wish = vec3(input.wish.x, 0.0, input.wish.y).clamp_length_max(1.0);
        let speed = if input.sprint { SPRINT_SPEED } else { WALK_SPEED };
        ctx.physics.drive_character(avatar.character, wish * speed, input.jump);
        if wish.length_squared() > 0.01 {
            avatar.facing = wish.x.atan2(-wish.z);
        }
        // Beim Zaubern zum Ziel drehen.
        if let Some(target) = input.cast {
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
    pub fn resource_hit(&mut self, ctx: &mut Context, id: u32, health: u8, effects: bool) {
        let Some(resource) = self.resources.get_mut(&id) else { return };
        if !resource.is_present() {
            return;
        }
        resource.health = health;
        if effects {
            resource.shake = 0.35;
            hit_particles(ctx, &resource.spec, health == 0);
            if !ctx.is_headless() {
                let (kind, at) = (resource.spec.kind, resource.spec.transform.position);
                self.sound_events.push(SoundEvent::Hit { kind, at, finished: health == 0 });
            }
        }
        if health == 0 {
            resource.regrows_at = Some(ctx.time.tick + RESPAWN_TICKS);
            self.remove_resource_visual(ctx, id);
        }
    }

    /// Nur Optik: Wackeln und Splitter sofort zeigen, bevor der Server antwortet.
    pub fn preview_hit(&mut self, ctx: &mut Context, id: u32) {
        if let Some(resource) = self.resources.get_mut(&id).filter(|r| r.is_present()) {
            resource.shake = 0.35;
            hit_particles(ctx, &resource.spec, false);
            if !ctx.is_headless() {
                self.sound_events.push(SoundEvent::Hit { kind: resource.spec.kind, at: resource.spec.transform.position, finished: false });
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
        self.place_resource(ctx, id);
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
        self.fireflies(ctx);
        self.update_bolts(ctx);
        for animal in &mut self.animals {
            animal.update_visual(ctx);
        }
        let dt = ctx.time.delta;
        let blend = (dt * 12.0).min(1.0);
        for (id, avatar) in &self.players {
            let Some(entity) = ctx.scene.try_get_mut(avatar.entity) else { continue };
            let target = Quat::from_rotation_y(-avatar.facing);
            entity.transform.rotation = entity.transform.rotation.slerp(target, blend);
            let position = entity.transform.position;
            if let Some(puppet) = self.puppets.get_mut(id) {
                let ground = self.terrain.height_at(position.x, position.z);
                puppet.update(ctx, position, ground);
                // Schritte: je nach Tempo alle gut ein bis zwei Meter, nur mit Bodenkontakt.
                let (last, walked) = self.stride.entry(*id).or_insert((position, 0.0));
                let moved = vec2(position.x - last.x, position.z - last.z).length();
                *last = position;
                let on_ground = position.y - 0.9 - ground < 0.3;
                if on_ground && moved < 1.0 {
                    *walked += moved;
                }
                let running = moved / dt.max(1e-4) > 6.5;
                if *walked > if running { 1.7 } else { 1.05 } {
                    *walked = 0.0;
                    self.sound_events.push(SoundEvent::Step { at: vec3(position.x, ground, position.z), sand: ground < 2.4, running });
                }
            }
        }
        for resource in self.resources.values_mut().filter(|r| r.shake > 0.0) {
            resource.shake = (resource.shake - dt).max(0.0);
            let Some(entity) = resource.entity.and_then(|e| ctx.scene.try_get_mut(e)) else { continue };
            let wobble = (ctx.time.elapsed * 45.0).sin() * resource.shake * 0.12;
            entity.transform.rotation = resource.spec.transform.rotation * Quat::from_rotation_x(wobble) * Quat::from_rotation_z(wobble * 0.6);
        }
    }
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
        ResourceKind::Stone => {
            ctx.particles.burst(Burst {
                position: base + Vec3::Y * spec.transform.scale.y * 0.5,
                count: 14 * many,
                color: vec3(0.3, 0.29, 0.28),
                color_variation: 0.25,
                speed: 6.0,
                direction: Vec3::Y * 0.8,
                size: 0.15,
                life: 1.0,
                ..Default::default()
            });
        }
    }
}

/// Jeder Spieler bekommt eine eigene, gut unterscheidbare Farbe.
fn player_color(id: PlayerId) -> Vec3 {
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
