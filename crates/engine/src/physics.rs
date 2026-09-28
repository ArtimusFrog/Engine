//! Physik auf Basis von Rapier.
//!
//! Die Simulation läuft in festen Takten ([`Physics::FIXED_DT`]), unabhängig von der
//! Bildrate. Das hält sie stabil und macht sie für Multiplayer vorhersagbar. Für die
//! Darstellung werden die Positionen zwischen den letzten beiden Takten interpoliert.

use glam::{Quat, Vec3};
use rapier3d::control::{CharacterAutostep, CharacterLength, KinematicCharacterController};
use rapier3d::prelude::*;

use crate::scene::{EntityId, Scene, Transform};

pub use rapier3d::prelude::RigidBodyHandle;

/// Kollisionsform. Die Maße entsprechen den eingebauten Meshes bei gleicher Skalierung:
/// ein Würfel mit Skalierung `size` passt genau zu `Shape::Box { size }`.
#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Box { size: Vec3 },
    Sphere { radius: f32 },
    /// Aufrechte Kapsel, `height` ist die Gesamthöhe inklusive Halbkugeln.
    Capsule { radius: f32, height: f32 },
}

impl Shape {
    fn collider(self) -> ColliderBuilder {
        match self {
            Shape::Box { size } => ColliderBuilder::cuboid(size.x / 2.0, size.y / 2.0, size.z / 2.0),
            Shape::Sphere { radius } => ColliderBuilder::ball(radius),
            Shape::Capsule { radius, height } => ColliderBuilder::capsule_y((height / 2.0 - radius).max(0.0), radius),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyKind {
    /// Bewegt sich nie (Boden, Wände).
    Static,
    /// Wird von der Physik bewegt (Kisten, Bälle).
    Dynamic,
}

#[derive(Clone, Copy, Debug)]
pub struct BodyDesc {
    pub kind: BodyKind,
    pub shape: Shape,
    pub friction: f32,
    /// Sprungkraft: 0 = gar nicht, 1 = verlustfrei.
    pub restitution: f32,
    /// Dichte in kg/m³ ÷ 1000 (1 = Wasser).
    pub density: f32,
    pub velocity: Vec3,
}

impl BodyDesc {
    pub fn fixed(shape: Shape) -> Self {
        BodyDesc { kind: BodyKind::Static, shape, friction: 0.7, restitution: 0.0, density: 1.0, velocity: Vec3::ZERO }
    }

    pub fn dynamic(shape: Shape) -> Self {
        BodyDesc { kind: BodyKind::Dynamic, ..Self::fixed(shape) }
    }

    pub fn with_restitution(mut self, restitution: f32) -> Self {
        self.restitution = restitution;
        self
    }

    pub fn with_density(mut self, density: f32) -> Self {
        self.density = density;
        self
    }

    pub fn with_velocity(mut self, velocity: Vec3) -> Self {
        self.velocity = velocity;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CharacterId(usize);

/// Einstellungen einer Spielfigur, ähnlich der CharacterMovementComponent in Unreal.
#[derive(Clone, Copy, Debug)]
pub struct CharacterSettings {
    pub radius: f32,
    pub height: f32,
    pub jump_speed: f32,
    /// Maximale Stufenhöhe, die automatisch erklommen wird.
    pub step_height: f32,
    /// Steilster begehbarer Hang in Grad.
    pub max_slope_degrees: f32,
    /// Masse beim Wegschieben von Objekten.
    pub mass: f32,
    /// Steuerbarkeit in der Luft (0 = keine, 1 = wie am Boden).
    pub air_control: f32,
}

impl Default for CharacterSettings {
    fn default() -> Self {
        CharacterSettings {
            radius: 0.4,
            height: 1.8,
            jump_speed: 7.0,
            step_height: 0.45,
            max_slope_degrees: 50.0,
            mass: 80.0,
            air_control: 0.35,
        }
    }
}

/// Aktueller Bewegungszustand einer Spielfigur.
#[derive(Clone, Copy, Debug, Default)]
pub struct CharacterState {
    pub velocity: Vec3,
    pub grounded: bool,
}

struct Character {
    body: RigidBodyHandle,
    collider: ColliderHandle,
    settings: CharacterSettings,
    controller: KinematicCharacterController,
    state: CharacterState,
}

/// Verbindung zwischen Physikkörper und Objekt in der Szene.
struct Link {
    body: RigidBodyHandle,
    entity: EntityId,
    sync_rotation: bool,
    previous: (Vec3, Quat),
}

pub struct Physics {
    world: PhysicsWorld,
    links: Vec<Link>,
    characters: Vec<Option<Character>>,
    replica: bool,
}

impl Default for Physics {
    fn default() -> Self {
        Self::new()
    }
}

impl Physics {
    /// Länge eines Physik-Takts in Sekunden (60 Hz).
    pub const FIXED_DT: f32 = 1.0 / 60.0;

    pub fn new() -> Self {
        let mut world = PhysicsWorld::new();
        world.gravity = Vec3::new(0.0, -9.81, 0.0);
        world.integration_parameters.dt = Self::FIXED_DT;
        Physics { world, links: Vec::new(), characters: Vec::new(), replica: false }
    }

    /// Replika-Modus für Multiplayer-Clients: Dynamische Körper werden nicht simuliert,
    /// sondern per [`set_body_pose`](Self::set_body_pose) vom Server gesteuert.
    /// Muss vor dem Erzeugen der Körper gesetzt werden.
    pub fn set_replica(&mut self, replica: bool) {
        self.replica = replica;
    }

    pub fn is_replica(&self) -> bool {
        self.replica
    }

    pub fn gravity(&self) -> Vec3 {
        self.world.gravity
    }

    pub fn set_gravity(&mut self, gravity: Vec3) {
        self.world.gravity = gravity;
    }

    /// Gibt einem Objekt der Szene einen Physikkörper. Dynamische Körper bewegen das
    /// Objekt danach automatisch.
    pub fn add_body(&mut self, entity: EntityId, transform: &Transform, desc: BodyDesc) -> RigidBodyHandle {
        let builder = match desc.kind {
            BodyKind::Static => RigidBodyBuilder::fixed(),
            BodyKind::Dynamic if self.replica => RigidBodyBuilder::kinematic_position_based(),
            BodyKind::Dynamic => RigidBodyBuilder::dynamic().linvel(desc.velocity).ccd_enabled(true),
        };
        let body = builder.pose(Pose::from_parts(transform.position, transform.rotation)).build();
        let collider = desc
            .shape
            .collider()
            .friction(desc.friction)
            .restitution(desc.restitution)
            .density(desc.density)
            .user_data(entity_user_data(entity));
        let (handle, _) = self.world.insert(body, collider);
        if desc.kind == BodyKind::Dynamic {
            self.links.push(Link { body: handle, entity, sync_rotation: true, previous: (transform.position, transform.rotation) });
        }
        handle
    }

    pub fn remove_body(&mut self, body: RigidBodyHandle) {
        self.world.remove_body(body);
        self.links.retain(|l| l.body != body);
    }

    /// Position und Drehung eines Körpers.
    pub fn body_pose(&self, body: RigidBodyHandle) -> Option<(Vec3, Quat)> {
        self.world.bodies.get(body).map(|b| (b.translation(), *b.rotation()))
    }

    /// Ruht der Körper (bewegt sich nicht mehr)?
    pub fn is_sleeping(&self, body: RigidBodyHandle) -> bool {
        self.world.bodies.get(body).is_none_or(|b| b.is_sleeping())
    }

    /// Setzt das Ziel eines Replika-Körpers für den nächsten Takt (nur im Replika-Modus sinnvoll).
    pub fn set_body_pose(&mut self, body: RigidBodyHandle, position: Vec3, rotation: Quat) {
        if let Some(body) = self.world.bodies.get_mut(body) {
            body.set_next_kinematic_position(Pose::from_parts(position, rotation));
        }
    }

    /// Festes Dreiecksnetz, z. B. eine Landschaft (siehe `Terrain::collision_mesh`).
    pub fn add_static_mesh(&mut self, entity: Option<EntityId>, vertices: Vec<Vec3>, triangles: Vec<[u32; 3]>) {
        let collider = ColliderBuilder::trimesh(vertices, triangles)
            .expect("Ungültiges Dreiecksnetz für die Kollision")
            .friction(0.8)
            .user_data(entity.map_or(0, entity_user_data));
        self.world.insert_collider(collider, None);
    }

    /// Unsichtbares, festes Hindernis ohne Objekt in der Szene (z. B. Weltgrenzen).
    pub fn add_static_collider(&mut self, transform: &Transform, shape: Shape) {
        let collider = shape.collider().position(Pose::from_parts(transform.position, transform.rotation));
        self.world.insert_collider(collider, None);
    }

    pub fn apply_impulse(&mut self, body: RigidBodyHandle, impulse: Vec3) {
        if let Some(body) = self.world.bodies.get_mut(body) {
            body.apply_impulse(impulse, true);
        }
    }

    /// Erstellt eine Spielfigur. `position` ist der Mittelpunkt der Kapsel.
    pub fn add_character(&mut self, entity: EntityId, position: Vec3, settings: CharacterSettings) -> CharacterId {
        let body = self.world.insert_body(RigidBodyBuilder::kinematic_position_based().translation(position));
        let collider = self.world.insert_collider(
            Shape::Capsule { radius: settings.radius, height: settings.height }
                .collider()
                .user_data(entity_user_data(entity)),
            Some(body),
        );
        let controller = KinematicCharacterController {
            autostep: Some(CharacterAutostep {
                max_height: CharacterLength::Absolute(settings.step_height),
                min_width: CharacterLength::Absolute(settings.radius * 0.5),
                include_dynamic_bodies: false,
            }),
            max_slope_climb_angle: settings.max_slope_degrees.to_radians(),
            min_slope_slide_angle: settings.max_slope_degrees.to_radians(),
            snap_to_ground: Some(CharacterLength::Absolute(0.3)),
            offset: CharacterLength::Absolute(0.02),
            ..Default::default()
        };
        self.links.push(Link { body, entity, sync_rotation: false, previous: (position, Quat::IDENTITY) });
        self.characters.push(Some(Character { body, collider, settings, controller, state: CharacterState::default() }));
        CharacterId(self.characters.len() - 1)
    }

    pub fn remove_character(&mut self, id: CharacterId) {
        if let Some(character) = self.characters.get_mut(id.0).and_then(Option::take) {
            self.remove_body(character.body);
        }
    }

    fn char(&self, id: CharacterId) -> &Character {
        self.characters[id.0].as_ref().expect("Spielfigur wurde entfernt")
    }

    pub fn character(&self, id: CharacterId) -> CharacterState {
        self.char(id).state
    }

    /// Mittelpunkt der Kapsel.
    pub fn character_position(&self, id: CharacterId) -> Vec3 {
        self.world.bodies[self.char(id).body].translation()
    }

    /// Steuert eine Spielfigur einen Takt weit. Aus `Game::fixed_update` aufrufen.
    ///
    /// `move_velocity` ist die gewünschte horizontale Geschwindigkeit in m/s. Die Figur
    /// bewegt sich sofort; dadurch lassen sich Eingaben für die Client-Vorhersage auch
    /// mehrfach hintereinander nachspielen.
    pub fn drive_character(&mut self, id: CharacterId, move_velocity: Vec3, jump: bool) {
        let dt = Self::FIXED_DT;
        let gravity = self.world.gravity;
        let character = self.characters[id.0].as_mut().expect("Spielfigur wurde entfernt");
        let settings = character.settings;
        let state = &mut character.state;

        let horizontal = Vec3::new(move_velocity.x, 0.0, move_velocity.z);
        if state.grounded {
            state.velocity.x = horizontal.x;
            state.velocity.z = horizontal.z;
            // Leicht nach unten drücken, damit die Figur auf Hängen am Boden bleibt.
            state.velocity.y = if jump { settings.jump_speed } else { gravity.y * dt };
        } else {
            let blend = (settings.air_control * 10.0 * dt).min(1.0);
            state.velocity.x += (horizontal.x - state.velocity.x) * blend;
            state.velocity.z += (horizontal.z - state.velocity.z) * blend;
            state.velocity.y += gravity.y * dt;
        }

        let body = &self.world.bodies[character.body];
        let position = *body.position();
        let shape = self.world.colliders[character.collider].shared_shape().clone();
        let filter = QueryFilter::default().exclude_rigid_body(character.body).exclude_sensors();

        let mut collisions = Vec::new();
        let movement = {
            let queries = self.world.query_pipeline_with_filter(filter);
            character.controller.move_shape(dt, &queries, &*shape, &position, state.velocity * dt, |c| collisions.push(c))
        };

        // Figur schiebt Kisten und Bälle weg (im Replika-Modus erledigt das der Server).
        if !self.replica {
            let world = &mut self.world;
            let mut queries = world.broad_phase.as_query_pipeline_mut(
                world.narrow_phase.query_dispatcher(),
                &mut world.bodies,
                &mut world.colliders,
                filter,
            );
            character.controller.solve_character_collision_impulses(dt, &mut queries, &*shape, settings.mass, &collisions);
        }

        let hit_ceiling = state.velocity.y > 0.0 && movement.translation.y < state.velocity.y * dt * 0.5;
        state.grounded = movement.grounded && state.velocity.y <= 0.0;
        if hit_ceiling {
            state.velocity.y = 0.0;
        }

        let target = position.translation + movement.translation;
        let body = &mut self.world.bodies[character.body];
        body.set_translation(target, false);
        body.set_next_kinematic_translation(target);
    }

    /// Setzt Position und Bewegungszustand einer Spielfigur direkt (Server-Korrektur).
    pub fn set_character_state(&mut self, id: CharacterId, position: Vec3, state: CharacterState) {
        let character = self.characters[id.0].as_mut().expect("Spielfigur wurde entfernt");
        character.state = state;
        let body = &mut self.world.bodies[character.body];
        body.set_translation(position, false);
        body.set_next_kinematic_translation(position);
    }

    /// Stößt eine Spielfigur weg (Rückstoß): sie verliert den Boden und fliegt mit `velocity`.
    pub fn stossen(&mut self, id: CharacterId, velocity: Vec3) {
        let character = self.characters[id.0].as_mut().expect("Spielfigur wurde entfernt");
        character.state.velocity = velocity;
        character.state.grounded = false;
    }

    /// Setzt eine Spielfigur sofort an eine neue Position, ohne Übergang (z. B. Respawn).
    pub fn teleport_character(&mut self, id: CharacterId, position: Vec3) {
        self.set_character_state(id, position, CharacterState::default());
        let body = self.char(id).body;
        if let Some(link) = self.links.iter_mut().find(|l| l.body == body) {
            link.previous.0 = position;
        }
    }

    /// Bewegt eine Figur, die nicht selbst gesteuert wird (Mitspieler auf dem Client),
    /// im nächsten Takt an `position`.
    pub fn move_character_to(&mut self, id: CharacterId, position: Vec3) {
        let body = self.char(id).body;
        self.world.bodies[body].set_next_kinematic_translation(position);
    }

    /// Erster Treffer eines Strahls: (Objekt, falls vorhanden; Entfernung).
    pub fn raycast(&self, origin: Vec3, direction: Vec3, max_distance: f32, ignore: Option<CharacterId>) -> Option<(Option<EntityId>, f32)> {
        let mut filter = QueryFilter::default().exclude_sensors();
        if let Some(id) = ignore {
            filter = filter.exclude_rigid_body(self.char(id).body);
        }
        let ray = Ray::new(origin, direction.normalize_or_zero());
        let (collider, distance) = self.world.cast_ray(&ray, max_distance, true, filter)?;
        let entity = entity_from_user_data(self.world.colliders[collider].user_data);
        Some((entity, distance))
    }

    /// Merkt sich die aktuellen Positionen als Ausgangspunkt für die Interpolation.
    /// Zu Beginn jedes Takts, vor `Game::fixed_update`.
    pub(crate) fn begin_tick(&mut self) {
        for link in &mut self.links {
            let body = &self.world.bodies[link.body];
            link.previous = (body.translation(), *body.rotation());
        }
    }

    /// Ein Physik-Takt.
    pub(crate) fn step(&mut self) {
        self.world.step();
    }

    /// Überträgt die Physik-Positionen in die Szene. `alpha` (0..1) gibt an, wie weit die
    /// Zeit zwischen dem vorletzten und dem letzten Takt fortgeschritten ist.
    pub(crate) fn sync_to_scene(&self, scene: &mut Scene, alpha: f32) {
        for link in &self.links {
            let body = &self.world.bodies[link.body];
            let Some(entity) = scene.try_get_mut(link.entity) else { continue };
            let transform = &mut entity.transform;
            transform.position = link.previous.0.lerp(body.translation(), alpha);
            if link.sync_rotation {
                transform.rotation = link.previous.1.slerp(*body.rotation(), alpha);
            }
        }
    }
}

fn entity_user_data(entity: EntityId) -> u128 {
    entity.index() as u128 + 1
}

fn entity_from_user_data(data: u128) -> Option<EntityId> {
    (data > 0).then(|| EntityId::from_index(data as usize - 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::Assets;
    use crate::scene::Entity;

    struct TestWorld {
        scene: Scene,
        physics: Physics,
        assets: Assets,
    }

    impl TestWorld {
        fn new() -> Self {
            let mut world = TestWorld { scene: Scene::default(), physics: Physics::new(), assets: Assets::new() };
            world.solid(Transform::from_position(Vec3::new(0.0, -0.5, 0.0)).with_scale(Vec3::new(100.0, 1.0, 100.0)));
            world
        }

        fn solid(&mut self, transform: Transform) {
            let id = self.scene.spawn(Entity::new("fest", self.assets.cube()).with_transform(transform));
            self.physics.add_body(id, &transform, BodyDesc::fixed(Shape::Box { size: transform.scale }));
        }

        fn character(&mut self, position: Vec3) -> (EntityId, CharacterId) {
            let entity = self.scene.spawn(Entity::new("figur", self.assets.cube()));
            (entity, self.physics.add_character(entity, position, CharacterSettings::default()))
        }

        fn run(&mut self, ticks: u32, mut each: impl FnMut(&mut Physics)) {
            for _ in 0..ticks {
                self.physics.begin_tick();
                each(&mut self.physics);
                self.physics.step();
            }
            self.physics.sync_to_scene(&mut self.scene, 1.0);
        }
    }

    #[test]
    fn kiste_faellt_und_bleibt_auf_dem_boden_liegen() {
        let mut w = TestWorld::new();
        let transform = Transform::from_position(Vec3::new(0.0, 5.0, 0.0));
        let id = w.scene.spawn(Entity::new("kiste", w.assets.cube()).with_transform(transform));
        w.physics.add_body(id, &transform, BodyDesc::dynamic(Shape::Box { size: Vec3::ONE }));

        w.run(180, |_| {});

        let y = w.scene.get(id).transform.position.y;
        assert!((y - 0.5).abs() < 0.05, "Kiste liegt bei y = {y}, erwartet 0.5");
    }

    #[test]
    fn figur_landet_auf_dem_boden() {
        let mut w = TestWorld::new();
        let (entity, ch) = w.character(Vec3::new(0.0, 3.0, 0.0));

        w.run(120, |p| p.drive_character(ch, Vec3::ZERO, false));

        let y = w.scene.get(entity).transform.position.y;
        assert!(w.physics.character(ch).grounded, "Figur sollte am Boden stehen");
        assert!((y - 0.9).abs() < 0.1, "Kapselmitte bei y = {y}, erwartet ~0.9");
    }

    #[test]
    fn figur_laeuft_vorwaerts() {
        let mut w = TestWorld::new();
        let (_, ch) = w.character(Vec3::new(0.0, 0.95, 0.0));

        w.run(60, |p| p.drive_character(ch, Vec3::new(0.0, 0.0, -5.0), false));

        let z = w.physics.character_position(ch).z;
        assert!((z + 5.0).abs() < 0.5, "Nach 1 s bei 5 m/s: z = {z}, erwartet ~-5");
    }

    #[test]
    fn figur_steigt_treppenstufe_hoch() {
        let mut w = TestWorld::new();
        w.solid(Transform::from_position(Vec3::new(0.0, 0.15, -3.0)).with_scale(Vec3::new(4.0, 0.3, 2.0)));
        let (_, ch) = w.character(Vec3::new(0.0, 0.95, 0.0));

        w.run(60, |p| p.drive_character(ch, Vec3::new(0.0, 0.0, -4.0), false));

        let pos = w.physics.character_position(ch);
        assert!(pos.z < -2.5, "Figur ist an der Stufe hängen geblieben: {pos}");
        assert!(pos.y > 1.1, "Figur sollte auf der Stufe stehen: {pos}");
    }

    #[test]
    fn figur_springt_und_landet_wieder() {
        let mut w = TestWorld::new();
        let (_, ch) = w.character(Vec3::new(0.0, 0.95, 0.0));
        w.run(10, |p| p.drive_character(ch, Vec3::ZERO, false));
        let ground = w.physics.character_position(ch).y;

        w.run(1, |p| p.drive_character(ch, Vec3::ZERO, true));
        let mut peak = ground;
        for _ in 0..90 {
            w.run(1, |p| p.drive_character(ch, Vec3::ZERO, false));
            peak = peak.max(w.physics.character_position(ch).y);
        }

        // v²/2g = 49/19.62 ≈ 2.5 m
        assert!(peak - ground > 2.0, "Sprung nur {} m hoch", peak - ground);
        assert!(w.physics.character(ch).grounded, "Figur sollte wieder gelandet sein");
    }

    #[test]
    fn nachgespielte_eingaben_ergeben_dieselbe_position() {
        // Grundlage der Client-Vorhersage: Zustand zurücksetzen und Eingaben erneut
        // abspielen muss exakt dasselbe Ergebnis liefern.
        let mut w = TestWorld::new();
        let (_, ch) = w.character(Vec3::new(0.0, 0.95, 0.0));
        w.run(20, |p| p.drive_character(ch, Vec3::ZERO, false));
        let start = (w.physics.character_position(ch), w.physics.character(ch));

        let inputs: Vec<(Vec3, bool)> = (0..90).map(|i| (Vec3::new((i as f32 * 0.1).sin() * 5.0, 0.0, -4.0), i == 10)).collect();
        for &(v, jump) in &inputs {
            w.physics.drive_character(ch, v, jump);
        }
        let first = w.physics.character_position(ch);

        w.physics.set_character_state(ch, start.0, start.1);
        for &(v, jump) in &inputs {
            w.physics.drive_character(ch, v, jump);
        }
        let second = w.physics.character_position(ch);
        assert!(first.distance(second) < 1e-4, "{first} != {second}");
    }

    #[test]
    fn replika_koerper_folgen_nur_dem_server() {
        let mut w = TestWorld::new();
        w.physics.set_replica(true);
        let transform = Transform::from_position(Vec3::new(0.0, 5.0, 0.0));
        let id = w.scene.spawn(Entity::new("kiste", w.assets.cube()).with_transform(transform));
        let body = w.physics.add_body(id, &transform, BodyDesc::dynamic(Shape::Box { size: Vec3::ONE }));

        w.run(60, |_| {});
        assert_eq!(w.physics.body_pose(body).unwrap().0.y, 5.0, "Replika darf nicht von allein fallen");

        w.run(1, |p| p.set_body_pose(body, Vec3::new(3.0, 1.0, 0.0), Quat::IDENTITY));
        assert_eq!(w.scene.get(id).transform.position, Vec3::new(3.0, 1.0, 0.0));
    }

    #[test]
    fn objekte_loeschen() {
        let mut w = TestWorld::new();
        let transform = Transform::from_position(Vec3::new(0.0, 5.0, 0.0));
        let id = w.scene.spawn(Entity::new("kiste", w.assets.cube()).with_transform(transform));
        let child = w.scene.spawn(Entity::new("deckel", w.assets.cube()).with_parent(id));
        let body = w.physics.add_body(id, &transform, BodyDesc::dynamic(Shape::Box { size: Vec3::ONE }));
        let (figur, ch) = w.character(Vec3::new(3.0, 1.0, 0.0));

        w.physics.remove_body(body);
        w.scene.despawn(id);
        w.physics.remove_character(ch);
        w.scene.despawn(figur);
        w.run(10, |_| {});

        assert!(!w.scene.contains(id) && !w.scene.contains(child) && !w.scene.contains(figur));
        assert_eq!(w.scene.len(), 1, "nur der Boden bleibt");
    }

    #[test]
    fn strahl_trifft_objekt() {
        let mut w = TestWorld::new();
        w.solid(Transform::from_position(Vec3::new(0.0, 1.0, -10.0)));
        w.run(1, |_| {});

        let (entity, distance) = w.physics.raycast(Vec3::new(0.0, 1.0, 0.0), Vec3::NEG_Z, 50.0, None).unwrap();
        assert!(entity.is_some());
        assert!((distance - 9.5).abs() < 0.01, "Entfernung {distance}, erwartet 9.5");
    }
}
