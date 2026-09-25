//! Kleines Leben als Kulisse (nur Optik, jeder Rechner für sich): Vogelschwärme über dem Wald,
//! Möwen über den Stränden, Schmetterlinge über den Wiesen, Fische, die aus dem Bergsee springen.
//!
//! Modelle aus `game/assets/kleintiere/` (Blender: `art/modelle/kleintiere/`): Körper und die
//! beiden Flügel sind eigene Knoten, das Spiel schlägt die Flügel selbst.

use engine::noise::Rng;
use engine::prelude::*;

use crate::asset_files;

/// Ein fliegendes Tier: Körper mit zwei Flügeln als Kinder (Gelenk im Ursprung des Körpers).
struct Flyer {
    body: EntityId,
    wings: [EntityId; 2],
    /// Versatz im Schwarm, eigene Flügelschlag-Phase und -Tempo
    offset: Vec3,
    phase: f32,
    speed: f32,
}

/// Ein Schwarm, der auf einer großen Schleife um seinen Mittelpunkt zieht.
struct Flock {
    center: Vec2,
    radius: f32,
    height: f32,
    angle: f32,
    /// Winkelgeschwindigkeit (rad/s), negativ = andersherum
    turn: f32,
    members: Vec<Flyer>,
    /// Möwen gleiten viel und schlagen selten
    glide: bool,
}

struct Butterfly {
    root: EntityId,
    wings: [EntityId; 2],
    position: Vec3,
    velocity: Vec3,
    phase: f32,
}

struct Jump {
    entity: EntityId,
    from: Vec3,
    to: Vec3,
    time: f32,
}

pub struct Wildlife {
    flocks: Vec<Flock>,
    butterflies: Vec<Butterfly>,
    fish: Option<EntityId>,
    jump: Option<Jump>,
    next_jump: f32,
    lake: (Vec2, f32, f32),
    rng: Rng,
}

/// Lädt ein Kleintier-Modell und liefert die Meshes von Körper und Flügeln.
fn load_parts(ctx: &mut Context, name: &str, body_node: &str) -> Option<(MeshId, Option<(MeshId, MeshId)>)> {
    let path = asset_files::variants("kleintiere", name).into_iter().next()?;
    let model = Model::from_file(&path).ok()?;
    // Flügel sind flache Flächen: von beiden Seiten zeichnen
    let mut add = |node: &str| model.extract_part(node, None).map(|mesh| ctx.assets.add_mesh(MeshData { double_sided: true, ..mesh }));
    let body = add(body_node)?;
    let wings = add("FluegelL").zip(add("FluegelR"));
    Some((body, wings))
}

/// Blender-Modelle schauen nach +X; unsere Welt misst die Richtung wie bei Spielern (0 = −Z).
fn heading(dir: Vec3) -> Quat {
    Quat::from_rotation_y((-dir.z).atan2(dir.x))
}

impl Wildlife {
    /// `forests`: Mittelpunkte großer Wälder, `beaches`: Punkte an Stränden, `lake`: Mitte, Radius, Spiegel.
    pub fn new(ctx: &mut Context, forests: &[Vec2], beaches: &[Vec2], lake: (Vec2, f32, f32)) -> Wildlife {
        let mut rng = Rng::new(0xB1_4D5);
        let mut flocks = Vec::new();
        let spawn_flock = |ctx: &mut Context, rng: &mut Rng, parts: (MeshId, Option<(MeshId, MeshId)>), center: Vec2, size: usize, radius: f32, height: f32, glide: bool, scale: f32| {
            let (body_mesh, wings) = parts;
            let Some((left, right)) = wings else { return None };
            let members = (0..size)
                .map(|i| {
                    let body = ctx.scene.spawn(Entity::new("Vogel", body_mesh).with_transform(Transform::default().with_scale(Vec3::splat(scale))));
                    let left_wing = ctx.scene.spawn(Entity::new("Flügel", left).with_parent(body));
                    let right_wing = ctx.scene.spawn(Entity::new("Flügel", right).with_parent(body));
                    let wings = [left_wing, right_wing];
                    let spread = if glide { 6.0 } else { 3.5 };
                    Flyer {
                        body,
                        wings,
                        offset: vec3(rng.range(-spread, spread), rng.range(-1.5, 1.5) + i as f32 * 0.1, rng.range(-spread, spread)),
                        phase: rng.range(0.0, std::f32::consts::TAU),
                        speed: rng.range(0.9, 1.15),
                    }
                })
                .collect();
            let turn = rng.range(0.035, 0.06) * if rng.chance(0.5) { 1.0 } else { -1.0 };
            Some(Flock { center, radius, height, angle: rng.range(0.0, std::f32::consts::TAU), turn, members, glide })
        };
        if let Some(bird) = load_parts(ctx, "vogel", "Vogel") {
            for &forest in forests.iter().take(4) {
                let size = 6 + (rng.next_u32() % 5) as usize;
                let (radius, height) = (rng.range(30.0, 55.0), rng.range(22.0, 32.0));
                if let Some(flock) = spawn_flock(ctx, &mut rng, bird, forest, size, radius, height, false, 2.2) {
                    flocks.push(flock);
                }
            }
        }
        if let Some(gull) = load_parts(ctx, "moewe", "Moewe") {
            for &beach in beaches.iter().take(3) {
                let (radius, height) = (rng.range(18.0, 30.0), rng.range(9.0, 15.0));
                if let Some(flock) = spawn_flock(ctx, &mut rng, gull, beach, 3, radius, height, true, 2.0) {
                    flocks.push(flock);
                }
            }
        }
        let fish = load_parts(ctx, "fisch", "Fisch").map(|(mesh, _)| {
            let mut entity = Entity::new("Fisch", mesh).with_transform(Transform::default().with_scale(Vec3::splat(1.6)));
            entity.visible = false;
            ctx.scene.spawn(entity)
        });

        // Schmetterlinge: flache, bunte Flügelpaare (im Programm gebaut)
        let mut butterflies = Vec::new();
        for i in 0..14 {
            let color = [vec3(1.0, 0.72, 0.15), vec3(0.95, 0.95, 0.9), vec3(0.35, 0.55, 1.0), vec3(1.0, 0.45, 0.2), vec3(0.8, 0.4, 0.95)][i % 5];
            let mut wing_mesh = |side: f32| {
                let mut mesh = MeshData { double_sided: true, ..Default::default() };
                let (a, b, c, d) = (Vec3::ZERO, vec3(0.05, 0.0, side * 0.09), vec3(-0.02, 0.0, side * 0.11), vec3(-0.07, 0.0, side * 0.06));
                mesh.push_triangle(a, b, c, color);
                mesh.push_triangle(a, c, d, color * 0.8);
                ctx.assets.add_mesh(mesh)
            };
            let (left, right) = (wing_mesh(-1.0), wing_mesh(1.0));
            let mut root = Entity::new("Schmetterling", ctx.assets.cube()).with_transform(Transform::default().with_scale(Vec3::splat(1.0)));
            root.visible = false;
            let root = ctx.scene.spawn(root);
            let left_wing = ctx.scene.spawn(Entity::new("Flügel", left).with_parent(root).with_material(Material::Foliage { sway: 0.0 }));
            let right_wing = ctx.scene.spawn(Entity::new("Flügel", right).with_parent(root).with_material(Material::Foliage { sway: 0.0 }));
            let wings = [left_wing, right_wing];
            butterflies.push(Butterfly { root, wings, position: Vec3::splat(f32::MAX), velocity: Vec3::ZERO, phase: rng.range(0.0, 6.0) });
        }
        log::info!("Kleines Leben: {} Schwärme ({} Tiere), {} Schmetterlinge, Fisch: {}", flocks.len(), flocks.iter().map(|f: &Flock| f.members.len()).sum::<usize>(), butterflies.len(), fish.is_some());
        Wildlife { flocks, butterflies, fish, jump: None, next_jump: 4.0, lake, rng }
    }

    /// Nur für Screenshots: wo der nächste Schwarm gerade fliegt, und wie viele Schmetterlinge zu sehen sind.
    pub fn nearest_flock(&self, ctx: &Context, from: Vec3) -> Option<Vec3> {
        self.flocks
            .iter()
            .filter_map(|f| f.members.first())
            .filter_map(|bird| ctx.scene.try_get(bird.body).map(|e| e.transform.position))
            .min_by(|a, b| a.distance(from).total_cmp(&b.distance(from)))
    }

    pub fn visible_butterflies(&self, ctx: &Context) -> usize {
        self.butterflies.iter().filter(|b| ctx.scene.try_get(b.wings[0]).is_some_and(|w| w.visible)).count()
    }

    /// Einmal pro Bild. `ground(x, z)` = Bodenhöhe, `meadow(p)` = liegt der Punkt auf einer Wiese?
    pub fn update(&mut self, ctx: &mut Context, ground: &dyn Fn(f32, f32) -> f32, meadow: &dyn Fn(Vec2) -> bool) {
        let dt = ctx.time.delta;
        let t = ctx.time.elapsed;
        let night = ctx.env.sky.stars;
        let day = night < 0.4;

        // ---------- Schwärme ----------
        for flock in &mut self.flocks {
            flock.angle += flock.turn * dt;
            let (s, c) = flock.angle.sin_cos();
            // Schleife mit leichtem Achter-Anteil, damit es nicht wie ein Karussell wirkt
            let path = flock.center + vec2(c, s * 0.7 + (flock.angle * 2.0).sin() * 0.25) * flock.radius;
            let tangent = vec3(-s, 0.0, c * 0.7 + (flock.angle * 2.0).cos() * 0.5).normalize() * flock.turn.signum();
            for bird in &mut flock.members {
                let wobble = vec3((t * 0.7 + bird.phase).sin(), (t * 0.9 + bird.phase).sin() * 0.4, (t * 0.6 + bird.phase).cos()) * 1.2;
                let base = vec3(path.x, 0.0, path.y) + bird.offset + wobble;
                let height = ground(base.x, base.z).max(0.0) + flock.height + bird.offset.y;
                let visible = day || !flock.glide;
                // Flügelschlag: Möwen gleiten und schlagen ab und zu ein paar Mal
                let flapping = !flock.glide || ((t * 0.35 + bird.phase).sin() > 0.55);
                let beat = if flapping { (t * 11.0 * bird.speed + bird.phase).sin() * 0.75 } else { 0.12 };
                let bank = flock.turn.signum() * 0.25;
                if let Some(body) = ctx.scene.try_get_mut(bird.body) {
                    body.visible = visible;
                    body.transform.position = vec3(base.x, height, base.z);
                    body.transform.rotation = heading(tangent) * Quat::from_rotation_x(bank);
                }
                for (i, &wing) in bird.wings.iter().enumerate() {
                    if let Some(w) = ctx.scene.try_get_mut(wing) {
                        let side = if i == 0 { 1.0 } else { -1.0 };
                        w.transform.rotation = Quat::from_rotation_x(beat * side);
                        w.visible = visible;
                    }
                }
            }
        }

        // ---------- Schmetterlinge (tagsüber auf Wiesen rund um die Kamera) ----------
        let camera = ctx.camera.position;
        for fly in &mut self.butterflies {
            let far = fly.position.distance(camera) > 30.0;
            if far && day {
                // Neu irgendwo auf einer Wiese in der Nähe
                for _ in 0..6 {
                    let p = vec2(camera.x, camera.z) + Vec2::from_angle(self.rng.range(0.0, std::f32::consts::TAU)) * self.rng.range(6.0, 22.0);
                    if meadow(p) {
                        fly.position = vec3(p.x, ground(p.x, p.y) + self.rng.range(0.4, 1.2), p.y);
                        break;
                    }
                }
            }
            let visible = day && fly.position.distance(camera) < 30.0;
            // Noch keinen Platz auf einer Wiese gefunden: unsichtbar lassen
            if !visible {
                for &wing in &fly.wings {
                    if let Some(w) = ctx.scene.try_get_mut(wing) {
                        w.visible = false;
                    }
                }
                continue;
            }
            // Taumelnder Flug: Richtung ändert sich ständig, bleibt nahe über dem Boden
            let wander = vec3((t * 1.3 + fly.phase).sin(), 0.0, (t * 1.1 + fly.phase * 1.7).cos());
            fly.velocity = fly.velocity.lerp(wander * 1.1, (dt * 1.5).min(1.0));
            fly.position += fly.velocity * dt;
            let floor = ground(fly.position.x, fly.position.z);
            let bob = (t * 3.0 + fly.phase).sin() * 0.25;
            fly.position.y = fly.position.y.clamp(floor + 0.3, floor + 1.6) + (floor + 0.8 + bob - fly.position.y) * (dt * 1.5).min(1.0);
            if let Some(root) = ctx.scene.try_get_mut(fly.root) {
                root.transform.position = fly.position;
                root.transform.rotation = heading(fly.velocity.normalize_or(Vec3::X));
            }
            let beat = (t * 16.0 + fly.phase).sin() * 1.1;
            for (i, &wing) in fly.wings.iter().enumerate() {
                if let Some(w) = ctx.scene.try_get_mut(wing) {
                    let side = if i == 0 { -1.0 } else { 1.0 };
                    w.transform.rotation = Quat::from_rotation_x(beat * side);
                    w.visible = visible;
                }
            }
        }

        // ---------- Fische springen aus dem Bergsee ----------
        let (lake_center, lake_radius, level) = self.lake;
        let near_lake = vec2(camera.x, camera.z).distance(lake_center) < lake_radius + 45.0;
        self.next_jump -= dt;
        if let (Some(fish), None, true, true) = (self.fish, &self.jump, near_lake, self.next_jump <= 0.0) {
            self.next_jump = self.rng.range(3.0, 8.0);
            let at = lake_center + Vec2::from_angle(self.rng.range(0.0, std::f32::consts::TAU)) * self.rng.range(4.0, lake_radius - 8.0);
            let dir = Vec2::from_angle(self.rng.range(0.0, std::f32::consts::TAU)) * self.rng.range(1.2, 2.2);
            let from = vec3(at.x, level, at.y);
            self.jump = Some(Jump { entity: fish, from, to: from + vec3(dir.x, 0.0, dir.y), time: 0.0 });
            splash(ctx, from);
        }
        if let Some(jump) = &mut self.jump {
            jump.time += dt;
            let u = (jump.time / 0.8).min(1.0);
            let position = jump.from.lerp(jump.to, u) + Vec3::Y * (4.0 * u * (1.0 - u) * 0.9);
            let slope = 0.9 * 4.0 * (1.0 - 2.0 * u) / (jump.to - jump.from).length().max(0.1);
            if let Some(fish) = ctx.scene.try_get_mut(jump.entity) {
                fish.visible = u < 1.0;
                fish.transform.position = position;
                fish.transform.rotation = heading(jump.to - jump.from) * Quat::from_rotation_z(slope.atan());
            }
            if u >= 1.0 {
                splash(ctx, jump.to);
                self.jump = None;
            }
        }
    }
}

/// Kleiner Wasserspritzer mit Ring auf der Oberfläche.
fn splash(ctx: &mut Context, at: Vec3) {
    ctx.particles.burst(Burst {
        position: at + Vec3::Y * 0.05,
        count: 10,
        color: vec3(0.85, 0.93, 1.0),
        color_variation: 0.05,
        speed: 1.6,
        direction: Vec3::Y * 1.4,
        size: 0.07,
        life: 0.7,
        gravity: 7.0,
        glow: 0.2,
        grow: 0.0,
        round: true,
    });
}
