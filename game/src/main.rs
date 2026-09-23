// Im Release-Build kein schwarzes Konsolenfenster neben dem Spiel öffnen.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use engine::prelude::*;

const WALK_SPEED: f32 = 5.0;
const SPRINT_SPEED: f32 = 9.0;
const THROW_SPEED: f32 = 18.0;
const SPAWN_POINT: Vec3 = vec3(0.0, 1.5, 8.0);

struct Player {
    entity: EntityId,
    character: CharacterId,
    /// Blickrichtung der Figur (Yaw in Radiant), folgt der Laufrichtung.
    facing: f32,
}

/// Spielplatz zum Testen von Physik und Spielfigur.
#[derive(Default)]
struct Playground {
    player: Option<Player>,
    orbit: OrbitController,
    fly: FlyController,
    free_camera: bool,
    // Eingaben, die zwischen zwei Physik-Takten fallen, bis zum nächsten Takt merken.
    jump_requested: bool,
    throw_requested: bool,
    /// Nur zum Testen: Figur läuft von allein (Kommandozeile `--autopilot`).
    autopilot: bool,
}

impl Game for Playground {
    fn init(&mut self, ctx: &mut Context) {
        self.autopilot = std::env::args().any(|a| a == "--autopilot");
        ctx.camera.pitch = -0.35;

        build_level(ctx);

        let settings = CharacterSettings::default();
        let body_mesh = ctx.assets.add_mesh(MeshData::capsule(settings.radius, settings.height, 24, 8));
        let entity = ctx.scene.spawn(
            Entity::new("Spieler", body_mesh)
                .with_transform(Transform::from_position(SPAWN_POINT))
                .with_color(vec4(0.1, 0.3, 0.9, 1.0)),
        );
        // Visier zeigt, wohin die Figur schaut.
        ctx.scene.spawn(
            Entity::new("Visier", ctx.assets.cube())
                .with_parent(entity)
                .with_transform(Transform::from_position(vec3(0.0, 0.45, -0.33)).with_scale(vec3(0.55, 0.18, 0.2)))
                .with_color(vec4(0.02, 0.02, 0.03, 1.0)),
        );
        let character = ctx.physics.add_character(entity, SPAWN_POINT, settings);
        self.player = Some(Player { entity, character, facing: 0.0 });
    }

    fn fixed_update(&mut self, ctx: &mut Context) {
        let Some(player) = &mut self.player else { return };

        let (forward, right) = flat_axes(ctx.camera.yaw);
        let mut wish = Vec3::ZERO;
        if !self.free_camera {
            for (key, dir) in [(KeyCode::KeyW, forward), (KeyCode::KeyS, -forward), (KeyCode::KeyD, right), (KeyCode::KeyA, -right)] {
                if ctx.input.key(key) {
                    wish += dir;
                }
            }
        }
        if self.autopilot {
            wish = forward;
            self.jump_requested |= ctx.time.tick % 120 == 60;
        }
        let wish = wish.normalize_or_zero();
        let speed = if ctx.input.key(KeyCode::ShiftLeft) { SPRINT_SPEED } else { WALK_SPEED };

        ctx.physics.drive_character(player.character, wish * speed, self.jump_requested);
        self.jump_requested = false;

        if wish != Vec3::ZERO {
            player.facing = wish.x.atan2(-wish.z);
        }

        if self.throw_requested {
            self.throw_requested = false;
            let aim = ctx.camera.forward();
            let origin = ctx.physics.character_position(player.character) + Vec3::Y * 0.5 + aim * 0.9;
            let ball = ctx.scene.spawn(
                Entity::new("Ball", ctx.assets.sphere())
                    .with_transform(Transform::from_position(origin).with_scale(Vec3::splat(0.4)))
                    .with_color(vec4(1.0, 0.45, 0.05, 1.0)),
            );
            let transform = ctx.scene.get(ball).transform;
            ctx.physics.add_body(
                ball,
                &transform,
                BodyDesc::dynamic(Shape::Sphere { radius: 0.2 })
                    .with_density(3.0)
                    .with_restitution(0.4)
                    .with_velocity(aim * THROW_SPEED + Vec3::Y * 2.0),
            );
        }
    }

    fn update(&mut self, ctx: &mut Context) {
        // Maus einfangen: Klick ins Fenster. Freigeben: Esc.
        if ctx.input.key_pressed(KeyCode::Escape) {
            ctx.cursor_locked = false;
        } else if !ctx.cursor_locked && ctx.input.mouse_pressed(MouseButton::Left) && !self.free_camera {
            ctx.cursor_locked = true;
        } else if ctx.cursor_locked && ctx.input.mouse_pressed(MouseButton::Left) {
            self.throw_requested = true;
        }
        self.jump_requested |= ctx.input.key_pressed(KeyCode::Space);

        if ctx.input.key_pressed(KeyCode::F1) {
            self.free_camera = !self.free_camera;
            ctx.cursor_locked = false;
        }

        let Some(player) = &self.player else { return };

        // Figur weich in Laufrichtung drehen.
        let transform = &mut ctx.scene.get_mut(player.entity).transform;
        let target = Quat::from_rotation_y(-player.facing);
        transform.rotation = transform.rotation.slerp(target, (ctx.time.delta * 12.0).min(1.0));
        let position = transform.position;

        if position.y < -30.0 {
            ctx.physics.teleport_character(player.character, SPAWN_POINT);
        }

        if self.free_camera {
            self.fly.update(ctx);
        } else {
            self.orbit.update(ctx, position + Vec3::Y * 0.6, Some(player.character));
        }
    }
}

/// Horizontale Vorwärts- und Rechtsrichtung zum Kamera-Yaw.
fn flat_axes(yaw: f32) -> (Vec3, Vec3) {
    (vec3(yaw.sin(), 0.0, -yaw.cos()), vec3(yaw.cos(), 0.0, yaw.sin()))
}

fn build_level(ctx: &mut Context) {
    let cube = ctx.assets.cube();
    let stone = vec4(0.45, 0.45, 0.48, 1.0);

    let solid =|ctx: &mut Context, name: &str, transform: Transform, color: Vec4| {
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

    // Kistenpyramide
    let crate_color = vec4(0.5, 0.3, 0.12, 1.0);
    for layer in 0..4 {
        for i in 0..(4 - layer) {
            let x = (i as f32 - (3 - layer) as f32 / 2.0) * 1.05;
            let transform = Transform::from_position(vec3(x, 0.5 + layer as f32 * 1.0, -3.0)).with_scale(Vec3::ONE);
            let id = ctx.scene.spawn(Entity::new("Kiste", cube).with_transform(transform).with_color(crate_color));
            ctx.physics.add_body(id, &transform, BodyDesc::dynamic(Shape::Box { size: Vec3::ONE }).with_density(0.5));
        }
    }

    // Ein paar Bälle zum Wegkicken
    for i in 0..5 {
        let transform = Transform::from_position(vec3(3.0 + i as f32 * 0.9, 3.0 + i as f32, 3.0)).with_scale(Vec3::splat(0.8));
        let id = ctx.scene.spawn(
            Entity::new("Ball", ctx.assets.sphere())
                .with_transform(transform)
                .with_color(hue(i as f32 / 5.0 + 0.1).extend(1.0)),
        );
        ctx.physics.add_body(id, &transform, BodyDesc::dynamic(Shape::Sphere { radius: 0.4 }).with_restitution(0.6).with_density(0.3));
    }
}

/// Farbton (0..1) in lineares RGB mit voller Sättigung.
fn hue(h: f32) -> Vec3 {
    let k = |n: f32| {
        let k = (n + h * 6.0) % 6.0;
        1.0 - k.min(4.0 - k).clamp(0.0, 1.0)
    };
    vec3(k(5.0), k(3.0), k(1.0)).powf(2.2) * 0.9
}

fn main() {
    run(EngineConfig { title: "Engine JN – Spielplatz".into(), ..Default::default() }, Playground::default());
}
