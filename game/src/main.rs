// Im Release-Build kein schwarzes Konsolenfenster neben dem Spiel öffnen.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use engine::prelude::*;

/// Testszene zum Ausprobieren der Engine.
#[derive(Default)]
struct Sandbox {
    fly: FlyController,
    spinner: Option<EntityId>,
    orbs: Vec<EntityId>,
}

impl Game for Sandbox {
    fn init(&mut self, ctx: &mut Context) {
        ctx.camera.position = vec3(0.0, 5.0, 14.0);
        ctx.camera.look_at(vec3(0.0, 1.0, 0.0));

        let (cube, plane, sphere) = (ctx.assets.cube(), ctx.assets.plane(), ctx.assets.sphere());

        ctx.scene.spawn(
            Entity::new("Boden", plane)
                .with_transform(Transform::default().with_scale(vec3(2000.0, 1.0, 2000.0)))
                .with_color(vec4(0.25, 0.5, 0.2, 1.0)),
        );

        // Ring aus Säulen in Regenbogenfarben
        let count = 12;
        for i in 0..count {
            let angle = i as f32 / count as f32 * std::f32::consts::TAU;
            let height = 1.0 + (i % 4) as f32 * 0.75;
            let position = vec3(angle.cos() * 8.0, height / 2.0, angle.sin() * 8.0);
            ctx.scene.spawn(
                Entity::new(format!("Säule {i}"), cube)
                    .with_transform(
                        Transform::from_position(position)
                            .with_scale(vec3(1.2, height, 1.2))
                            .with_rotation(Quat::from_rotation_y(-angle)),
                    )
                    .with_color(hue(i as f32 / count as f32).extend(1.0)),
            );
        }

        self.spinner = Some(ctx.scene.spawn(
            Entity::new("Drehwürfel", cube)
                .with_transform(Transform::from_position(vec3(0.0, 2.0, 0.0)).with_scale(Vec3::splat(1.5)))
                .with_color(vec4(0.9, 0.9, 0.95, 1.0)),
        ));

        for i in 0..3 {
            let id = ctx.scene.spawn(
                Entity::new(format!("Kugel {i}"), sphere).with_color(vec4(1.0, 0.55, 0.1, 1.0)),
            );
            self.orbs.push(id);
        }
    }

    fn update(&mut self, ctx: &mut Context) {
        if ctx.input.key_pressed(KeyCode::Escape) {
            ctx.exit();
        }
        self.fly.update(ctx);

        let t = ctx.time.elapsed;
        if let Some(id) = self.spinner {
            ctx.scene.get_mut(id).transform.rotation = Quat::from_rotation_y(t) * Quat::from_rotation_x(t * 0.6);
        }
        for (i, &id) in self.orbs.iter().enumerate() {
            let angle = t * 0.8 + i as f32 * std::f32::consts::TAU / 3.0;
            let position = vec3(angle.cos() * 4.0, 1.0 + (t * 2.0 + i as f32).sin() * 0.5 + 0.5, angle.sin() * 4.0);
            ctx.scene.get_mut(id).transform.position = position;
        }
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
    run(EngineConfig { title: "Engine JN – Sandbox".into(), ..Default::default() }, Sandbox::default());
}
