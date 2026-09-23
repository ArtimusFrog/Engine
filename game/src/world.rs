//! Die Spielwelt, wie sie Server und Clients gleichermaßen aufbauen.

use std::collections::{BTreeMap, HashMap};

use engine::prelude::*;

use crate::island::{self, ResourceKind, ResourceSpec};
use crate::protocol::{Inventory, NetId, ObjectKind, PlayerId, PlayerInput, HOST_PLAYER};

pub const WALK_SPEED: f32 = 5.0;
pub const SPRINT_SPEED: f32 = 9.0;
pub const THROW_SPEED: f32 = 18.0;
/// Takte zwischen zwei Würfen desselben Spielers.
pub const THROW_COOLDOWN_TICKS: u64 = 15;
/// Takte zwischen zwei Schlägen auf einen Rohstoff.
pub const HARVEST_COOLDOWN_TICKS: u64 = 22;
/// Wie nah man einem Rohstoff sein muss (Meter vom Rand).
pub const HARVEST_REACH: f32 = 2.2;
/// Nach dieser Zeit wachsen Bäume nach und Felsen tauchen wieder auf (2 Minuten).
pub const RESPAWN_TICKS: u64 = 60 * 120;
/// Zur Laufzeit erzeugte Objekte (geworfene Bälle) bekommen IDs ab hier.
pub const FIRST_RUNTIME_ID: NetId = 10_000;

pub struct Avatar {
    pub entity: EntityId,
    pub character: CharacterId,
    /// Blickrichtung (Yaw in Radiant), folgt der Laufrichtung.
    pub facing: f32,
    pub last_throw_tick: u64,
    pub last_harvest_tick: u64,
    pub name: String,
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
    /// Seit welchem Takt der Rohstoff abgebaut ist.
    pub gone_since: Option<u64>,
    /// Restzeit des Wackelns nach einem Treffer (Sekunden).
    shake: f32,
}

impl Resource {
    pub fn is_present(&self) -> bool {
        self.gone_since.is_none()
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

pub struct World {
    pub players: HashMap<PlayerId, Avatar>,
    /// Geworfene Bälle, die übers Netzwerk abgeglichen werden.
    pub objects: BTreeMap<NetId, NetObject>,
    pub resources: BTreeMap<u32, Resource>,
    by_entity: HashMap<EntityId, u32>,
    pub inventories: HashMap<PlayerId, Inventory>,
    /// Startpunkt für neue Spieler.
    pub spawn: Vec3,
    pub terrain: Terrain,
    capsule: MeshId,
}

impl World {
    /// Baut die Insel. Auf allen Rechnern identisch, damit die IDs passen.
    pub fn new(ctx: &mut Context) -> Self {
        let settings = CharacterSettings::default();
        let capsule = ctx.assets.named_mesh("spielfigur", || MeshData::capsule(settings.radius, settings.height, 24, 8));
        let island = island::build(ctx);
        let mut world = World {
            players: HashMap::new(),
            objects: BTreeMap::new(),
            resources: BTreeMap::new(),
            by_entity: HashMap::new(),
            inventories: HashMap::new(),
            spawn: island.spawn + Vec3::Y * 1.2,
            terrain: island.terrain,
            capsule,
        };
        for (id, spec) in island.resources {
            let health = spec.max_health;
            world.resources.insert(id, Resource { spec, health, entity: None, body: None, gone_since: None, shake: 0.0 });
            world.place_resource(ctx, id);
        }
        world
    }

    // ---------- Spieler ----------

    pub fn spawn_player(&mut self, ctx: &mut Context, id: PlayerId, name: &str, position: Vec3) {
        if self.players.contains_key(&id) {
            return;
        }
        let entity = ctx.scene.spawn(
            Entity::new(format!("Spieler {id}"), self.capsule)
                .with_transform(Transform::from_position(position))
                .with_color(player_color(id).extend(1.0)),
        );
        // Visier zeigt, wohin die Figur schaut.
        ctx.scene.spawn(
            Entity::new("Visier", ctx.assets.cube())
                .with_parent(entity)
                .with_transform(Transform::from_position(vec3(0.0, 0.45, -0.33)).with_scale(vec3(0.55, 0.18, 0.2)))
                .with_color(vec4(0.02, 0.02, 0.03, 1.0)),
        );
        let character = ctx.physics.add_character(entity, position, CharacterSettings::default());
        self.players.insert(
            id,
            Avatar { entity, character, facing: 0.0, last_throw_tick: 0, last_harvest_tick: 0, name: name.to_string() },
        );
        self.inventories.entry(id).or_default();
        log::info!("{name} ({id}) ist da");
    }

    pub fn remove_player(&mut self, ctx: &mut Context, id: PlayerId) {
        if let Some(avatar) = self.players.remove(&id) {
            ctx.physics.remove_character(avatar.character);
            ctx.scene.despawn(avatar.entity);
            self.inventories.remove(&id);
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
        }
        if health == 0 {
            resource.gone_since = Some(ctx.time.tick);
            self.remove_resource_visual(ctx, id);
        }
    }

    /// Nur Optik: Wackeln und Splitter sofort zeigen, bevor der Server antwortet.
    pub fn preview_hit(&mut self, ctx: &mut Context, id: u32) {
        if let Some(resource) = self.resources.get_mut(&id).filter(|r| r.is_present()) {
            resource.shake = 0.35;
            hit_particles(ctx, &resource.spec, false);
        }
    }

    /// Rohstoff ist nachgewachsen.
    pub fn resource_back(&mut self, ctx: &mut Context, id: u32) {
        let Some(resource) = self.resources.get_mut(&id) else { return };
        if resource.is_present() {
            return;
        }
        resource.gone_since = None;
        resource.health = resource.spec.max_health;
        self.place_resource(ctx, id);
    }

    /// Dreht Figuren weich in ihre Blickrichtung und lässt getroffene Rohstoffe wackeln.
    pub fn update_visuals(&mut self, ctx: &mut Context) {
        let dt = ctx.time.delta;
        let blend = (dt * 12.0).min(1.0);
        for avatar in self.players.values() {
            if let Some(entity) = ctx.scene.try_get_mut(avatar.entity) {
                let target = Quat::from_rotation_y(-avatar.facing);
                entity.transform.rotation = entity.transform.rotation.slerp(target, blend);
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
