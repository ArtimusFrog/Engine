//! Die Spielwelt, wie sie Server und Clients gleichermaßen aufbauen.

use std::collections::{BTreeMap, HashMap};

use engine::prelude::*;

use crate::protocol::{NetId, ObjectKind, PlayerId, PlayerInput, HOST_PLAYER};

pub const SPAWN_POINT: Vec3 = vec3(0.0, 1.5, 8.0);
pub const WALK_SPEED: f32 = 5.0;
pub const SPRINT_SPEED: f32 = 9.0;
pub const THROW_SPEED: f32 = 18.0;
/// Takte zwischen zwei Würfen desselben Spielers.
pub const THROW_COOLDOWN_TICKS: u64 = 15;
/// Objekte aus dem Level bekommen feste IDs ab 0, zur Laufzeit erzeugte ab hier.
pub const FIRST_RUNTIME_ID: NetId = 10_000;

pub struct Avatar {
    pub entity: EntityId,
    pub character: CharacterId,
    /// Blickrichtung (Yaw in Radiant), folgt der Laufrichtung.
    pub facing: f32,
    pub last_throw_tick: u64,
    pub name: String,
}

pub struct NetObject {
    pub entity: EntityId,
    pub body: RigidBodyHandle,
    pub kind: Option<ObjectKind>,
}

pub struct World {
    pub players: HashMap<PlayerId, Avatar>,
    /// Alle beweglichen Objekte, die übers Netzwerk abgeglichen werden.
    pub objects: BTreeMap<NetId, NetObject>,
    capsule: MeshId,
}

impl World {
    /// Baut das Level. Auf allen Rechnern in derselben Reihenfolge, damit die IDs passen.
    pub fn new(ctx: &mut Context) -> Self {
        let settings = CharacterSettings::default();
        let capsule = ctx.assets.add_mesh(MeshData::capsule(settings.radius, settings.height, 24, 8));
        let mut world = World { players: HashMap::new(), objects: BTreeMap::new(), capsule };
        world.build_level(ctx);
        world
    }

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
        self.players.insert(id, Avatar { entity, character, facing: 0.0, last_throw_tick: 0, name: name.to_string() });
        log::info!("{name} ({id}) ist da");
    }

    pub fn remove_player(&mut self, ctx: &mut Context, id: PlayerId) {
        if let Some(avatar) = self.players.remove(&id) {
            ctx.physics.remove_character(avatar.character);
            ctx.scene.despawn(avatar.entity);
            log::info!("{} ({id}) ist weg", avatar.name);
        }
    }

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
                self.objects.insert(id, NetObject { entity, body, kind: Some(kind) });
            }
        }
    }

    pub fn remove_object(&mut self, ctx: &mut Context, id: NetId) {
        if let Some(object) = self.objects.remove(&id) {
            ctx.physics.remove_body(object.body);
            ctx.scene.despawn(object.entity);
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

    /// Dreht die Figuren weich in ihre Blickrichtung. Einmal pro Bild.
    pub fn update_visuals(&self, ctx: &mut Context) {
        let blend = (ctx.time.delta * 12.0).min(1.0);
        for avatar in self.players.values() {
            if let Some(entity) = ctx.scene.try_get_mut(avatar.entity) {
                let target = Quat::from_rotation_y(-avatar.facing);
                entity.transform.rotation = entity.transform.rotation.slerp(target, blend);
            }
        }
    }

    #[cfg(test)]
    pub fn player_position(&self, ctx: &Context, id: PlayerId) -> Option<Vec3> {
        self.players.get(&id).map(|a| ctx.physics.character_position(a.character))
    }

    fn build_level(&mut self, ctx: &mut Context) {
        let cube = ctx.assets.cube();
        let stone = vec4(0.45, 0.45, 0.48, 1.0);

        let solid = |ctx: &mut Context, name: &str, transform: Transform, color: Vec4| {
            let id = ctx.scene.spawn(Entity::new(name, cube).with_transform(transform).with_color(color));
            ctx.physics.add_body(id, &transform, BodyDesc::fixed(Shape::Box { size: transform.scale }));
        };

        solid(
            ctx,
            "Boden",
            Transform::from_position(vec3(0.0, -0.5, 0.0)).with_scale(vec3(400.0, 1.0, 400.0)),
            vec4(0.25, 0.5, 0.2, 1.0),
        );

        // Säulenring als Begrenzung
        let count = 16;
        for i in 0..count {
            let angle = i as f32 / count as f32 * std::f32::consts::TAU;
            let height = 2.0 + (i % 4) as f32;
            solid(
                ctx,
                "Säule",
                Transform::from_position(vec3(angle.cos() * 22.0, height / 2.0, angle.sin() * 22.0))
                    .with_scale(vec3(1.5, height, 1.5))
                    .with_rotation(Quat::from_rotation_y(-angle)),
                hue(i as f32 / count as f32).extend(1.0),
            );
        }

        // Treppe nach links mit Plattform
        for step in 0..6 {
            let height = 0.3 * (step + 1) as f32;
            solid(
                ctx,
                "Stufe",
                Transform::from_position(vec3(-6.0 - step as f32 * 0.8, height / 2.0, -2.0)).with_scale(vec3(0.8, height, 3.0)),
                stone,
            );
        }
        solid(ctx, "Plattform", Transform::from_position(vec3(-12.8, 0.9, -2.0)).with_scale(vec3(4.0, 1.8, 6.0)), stone);

        // Rampe nach rechts hinten mit Plattform
        let tilt = 17f32.to_radians();
        solid(
            ctx,
            "Rampe",
            Transform::from_position(vec3(7.0, 4.0 * tilt.sin(), -4.0))
                .with_scale(vec3(3.0, 0.3, 8.0))
                .with_rotation(Quat::from_rotation_x(tilt)),
            vec4(0.6, 0.5, 0.35, 1.0),
        );
        let top = 8.0 * tilt.sin();
        solid(ctx, "Rampenplattform", Transform::from_position(vec3(7.0, top / 2.0, -9.3)).with_scale(vec3(3.0, top, 3.0)), stone);

        let mut next_id: NetId = 0;
        let mut dynamic = |ctx: &mut Context, world: &mut World, entity: Entity, desc: BodyDesc| {
            let transform = entity.transform;
            let entity = ctx.scene.spawn(entity);
            let body = ctx.physics.add_body(entity, &transform, desc);
            world.objects.insert(next_id, NetObject { entity, body, kind: None });
            next_id += 1;
        };

        // Kistenpyramide
        let crate_color = vec4(0.5, 0.3, 0.12, 1.0);
        for layer in 0..4 {
            for i in 0..(4 - layer) {
                let x = (i as f32 - (3 - layer) as f32 / 2.0) * 1.05;
                dynamic(
                    ctx,
                    self,
                    Entity::new("Kiste", cube)
                        .with_transform(Transform::from_position(vec3(x, 0.5 + layer as f32, -3.0)))
                        .with_color(crate_color),
                    BodyDesc::dynamic(Shape::Box { size: Vec3::ONE }).with_density(0.5),
                );
            }
        }

        // Ein paar Bälle zum Wegkicken
        for i in 0..5 {
            dynamic(
                ctx,
                self,
                Entity::new("Ball", ctx.assets.sphere())
                    .with_transform(Transform::from_position(vec3(3.0 + i as f32 * 0.9, 3.0 + i as f32, 3.0)).with_scale(Vec3::splat(0.8)))
                    .with_color(hue(i as f32 / 5.0 + 0.1).extend(1.0)),
                BodyDesc::dynamic(Shape::Sphere { radius: 0.4 }).with_restitution(0.6).with_density(0.3),
            );
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
