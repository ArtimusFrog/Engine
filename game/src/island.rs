//! Die Insel: Landschaft, Biome, Wasser und Bewuchs.
//!
//! Alles entsteht aus einem Startwert. Server und Clients bauen die Insel unabhängig
//! voneinander und erhalten exakt dieselbe Welt – übertragen werden nur Änderungen.

use engine::noise::{fbm, hash01, ridged, Rng};
use engine::prelude::*;

use crate::asset_files;
use crate::models;

pub const SEED: u32 = 20_260_924;
/// Kennung der Insel für Spielstände: bei jeder Änderung an Gestalt oder Verteilung der
/// Rohstoffe hochzählen, sonst passen die Rohstoff-IDs gespeicherter Spielstände nicht mehr.
pub const WORLD_ID: u32 = SEED + 8;
/// Radius des Festlands in Metern (die Küste franst um diesen Wert aus).
pub const ISLAND_RADIUS: f32 = 760.0;
/// Maßstab gegenüber der ersten, kleineren Insel (330 m): Bach und See wachsen mit.
const SCALE: f32 = ISLAND_RADIUS / 330.0;
const TERRAIN_SIZE: f32 = 2040.0;
/// 3 m je Zelle
const TERRAIN_CELLS: usize = 680;
/// Bergsee im Westen: Mitte, Radius der Wasserfläche und Höhe des Wasserspiegels.
const LAKE_CENTER: Vec2 = vec2(-0.42 * ISLAND_RADIUS, 0.12 * ISLAND_RADIUS);
const LAKE_RADIUS: f32 = 38.0 * SCALE;
const LAKE_LEVEL: f32 = 6.0;
/// Um den Startpunkt bleibt eine Lichtung frei.
const SPAWN_CLEARING: f32 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ResourceKind {
    Wood,
    /// Steinvorkommen (nur mit der Spitzhacke)
    Stone,
    /// Erzvorkommen (nur mit der Spitzhacke)
    Ore,
}

impl ResourceKind {
    /// Braucht man zum Abbauen die Spitzhacke?
    pub fn needs_pickaxe(self) -> bool {
        matches!(self, ResourceKind::Stone | ResourceKind::Ore)
    }

    /// Werkzeug, mit dem man es abbaut: Axt für Bäume, Spitzhacke für Vorkommen.
    pub fn tool(self) -> crate::protocol::Tool {
        if self.needs_pickaxe() { crate::protocol::Tool::Pickaxe } else { crate::protocol::Tool::Axe }
    }
}

/// Ein abbaubarer Rohstoff, so wie er beim Aufbau der Insel entsteht.
#[derive(Clone, Debug)]
pub struct ResourceSpec {
    pub kind: ResourceKind,
    pub name: &'static str,
    pub mesh: MeshId,
    pub transform: Transform,
    pub color: Vec4,
    pub material: Material,
    /// Zusätzliches leuchtendes Teil (Früchte des Zauberbaums).
    pub glow_part: Option<MeshId>,
    pub collider: Shape,
    /// Mittelpunkt des Kollisionskörpers relativ zum Fuß.
    pub collider_offset: Vec3,
    pub max_health: u8,
}

pub struct Island {
    pub terrain: Terrain,
    /// Rohstoffe mit ihrer ID (= Nummer der Rasterzelle, siehe `build`).
    pub resources: Vec<(u32, ResourceSpec)>,
    pub spawn: Vec3,
    /// Magische Kristallvorkommen (Mitte am Boden) – für Licht und Funken in der Nähe.
    pub crystals: Vec<Vec3>,
    /// Gezeichnete Übersichtskarte (nur mit Fenster).
    pub map: Option<Image>,
    /// Besondere Orte (Lager, Sehenswürdigkeiten): Feuer und Wegweiser.
    pub places: crate::orte::Places,
}

/// Die Übersichtskarte zeigt ±`MAP_EXTENT` Meter um die Inselmitte (Norden = -z oben).
pub const MAP_EXTENT: f32 = ISLAND_RADIUS * 1.08;
const MAP_SIZE: usize = 2048;

/// Orte, die auf der Karte beschriftet werden.
pub fn landmarks() -> [(&'static str, Vec2); 4] {
    [
        ("Bergsee", LAKE_CENTER),
        (FESTUNG_NAME, Vec2::ZERO),
        (BURG_NAME, BURG_ORT),
        ("Nebelgebirge", vec2(0.05 * ISLAND_RADIUS, -0.62 * ISLAND_RADIUS)),
    ]
}

/// Zeichnet die Übersichtskarte im Pergament-Stil: Gelände mit Schattierung und Höhenlinien,
/// Wasser nach Tiefe, Küstenlinie, Trampelpfade und Bäume als Punkte.
fn map_image(terrain: &Terrain, paths: &Paths, trees: &[Vec2]) -> Image {
    let size = MAP_SIZE;
    let texel = MAP_EXTENT * 2.0 / size as f32;
    let mut rgba = vec![0u8; size * size * 4];
    let light = vec3(-0.5, 0.8, -0.35).normalize();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(16);
    let rows_per_thread = size.div_ceil(threads);
    std::thread::scope(|scope| {
        for (chunk_index, chunk) in rgba.chunks_mut(rows_per_thread * size * 4).enumerate() {
            scope.spawn(move || {
                for (i, pixel) in chunk.chunks_exact_mut(4).enumerate() {
                    let (x, z) = (i % size, chunk_index * rows_per_thread + i / size);
                    let p = Vec2::splat(-MAP_EXTENT) + (vec2(x as f32, z as f32) + 0.5) * texel;
                    let h = terrain.height_at(p.x, p.y);
                    let n = terrain.normal_at(p.x, p.y);
                    let linear = if in_lake(p) && h < LAKE_LEVEL {
                        vec3(0.08, 0.3, 0.42)
                    } else if stream_distance(p) < STREAM_BED + 0.5 {
                        vec3(0.1, 0.36, 0.48)
                    } else if h < 0.0 {
                        vec3(0.2, 0.5, 0.58).lerp(vec3(0.05, 0.19, 0.32), smoothstep(0.0, 9.0, -h))
                    } else {
                        // Gelände: Farbe wie im Spiel, von Nordwesten beleuchtet, mit Höhenlinien
                        let shade = (n.dot(light) / light.y).clamp(0.55, 1.3);
                        let contour = if h > 1.5 && (h / 4.0).fract() < 0.07 { 0.78 } else { 1.0 };
                        let mut c = ground_color(vec3(p.x, h, p.y), n) * shade * contour;
                        let path = paths.at(p) * smoothstep(2.3, 3.0, h);
                        c = c.lerp(vec3(0.3, 0.18, 0.08), smoothstep(0.3, 0.6, path));
                        burg_karte(p).or_else(|| festung_karte(p)).map_or(c, |bau| bau * shade)
                    };
                    let mut srgb = linear.to_array().map(linear_to_srgb);
                    // Küstenlinie
                    if h.abs() < 0.35 {
                        srgb = [0.3, 0.24, 0.17];
                    }
                    for (k, channel) in srgb.into_iter().enumerate() {
                        pixel[k] = (channel * 255.0).round() as u8;
                    }
                    pixel[3] = 255;
                }
            });
        }
    });
    // Bäume als kleine dunkelgrüne Punkte mit hellem Glanz oben links
    let mut dot = |p: Vec2, radius: f32, color: [u8; 3]| {
        let center = (p + Vec2::splat(MAP_EXTENT)) / texel;
        let r = radius.ceil() as i32 + 1;
        for dz in -r..=r {
            for dx in -r..=r {
                let (x, z) = (center.x as i32 + dx, center.y as i32 + dz);
                if x < 0 || z < 0 || x >= size as i32 || z >= size as i32 {
                    continue;
                }
                let d = vec2(x as f32 + 0.5, z as f32 + 0.5).distance(center);
                let cover = (radius + 0.5 - d).clamp(0.0, 1.0);
                let i = (z as usize * size + x as usize) * 4;
                for k in 0..3 {
                    rgba[i + k] = (rgba[i + k] as f32 + (color[k] as f32 - rgba[i + k] as f32) * cover) as u8;
                }
            }
        }
    };
    for &tree in trees {
        dot(tree, 1.7, [28, 64, 26]);
        dot(tree - Vec2::splat(0.45 * texel), 0.7, [70, 120, 50]);
    }
    // Pergament: alles leicht gelblich, zum Rand hin dunkler
    for z in 0..size {
        for x in 0..size {
            let i = (z * size + x) * 4;
            let r = vec2(x as f32 / size as f32 - 0.5, z as f32 / size as f32 - 0.5).length() * 2.0;
            let edge = smoothstep(0.78, 1.15, r);
            for (k, (paper, sepia)) in [(232.0, 140.0), (214.0, 107.0), (174.0, 71.0)].into_iter().enumerate() {
                let c = rgba[i + k] as f32;
                let c = c + (paper - c) * 0.2;
                rgba[i + k] = (c + (sepia - c) * edge * 0.55) as u8;
            }
        }
    }
    Image { width: size as u32, height: size as u32, rgba }
}

/// Wie weit Wind Laub bewegt.
const LEAVES: Material = Material::Leaves { sway: 0.035 };
const GRASS: Material = Material::Leaves { sway: 0.25 };
const FLOWERS: Material = Material::Foliage { sway: 0.25 };

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Der Silberbach: von der Quelle im Nebelgebirge durch ein Hochtal, über eine Felsstufe
/// (Wasserfall zwischen Punkt 2 und 3) und durch die Ebene in den Bergsee.
const STREAM: [Vec2; 7] = [
    vec2(-120.0 * SCALE, -145.0 * SCALE),
    vec2(-128.0 * SCALE, -118.0 * SCALE),
    vec2(-134.0 * SCALE, -92.0 * SCALE),
    vec2(-138.0 * SCALE, -70.0 * SCALE),
    vec2(-140.0 * SCALE, -45.0 * SCALE),
    vec2(-141.0 * SCALE, -18.0 * SCALE),
    vec2(-139.0 * SCALE, 8.0 * SCALE),
];
/// Abschnitt mit dem Wasserfall (von STREAM[i] nach STREAM[i+1]) und wo darin die Kante liegt.
const WATERFALL_SEGMENT: usize = 2;
const WATERFALL_EDGE: f32 = 0.55;
/// Halbe Breite des flachen Bachbetts und bis wohin die Ufer reichen.
const STREAM_BED: f32 = 1.8;
const STREAM_BANK: f32 = 6.5;
/// Wasser über dem Bachbett.
const STREAM_DEPTH: f32 = 0.55;

/// Abstand zwischen zwei Messpunkten des Bachbetts (Meter).
const STREAM_STEP: f32 = 1.5;

/// Länge bis zum Anfang jedes Abschnitts von STREAM (und die Gesamtlänge am Ende).
fn stream_lengths() -> [f32; 7] {
    let mut lengths = [0.0; 7];
    for i in 1..STREAM.len() {
        lengths[i] = lengths[i - 1] + STREAM[i].distance(STREAM[i - 1]);
    }
    lengths
}

/// Punkt auf der Mittellinie nach `s` Metern.
fn stream_point(s: f32) -> Vec2 {
    let lengths = stream_lengths();
    let i = (1..STREAM.len()).find(|&i| lengths[i] >= s).unwrap_or(STREAM.len() - 1) - 1;
    let u = ((s - lengths[i]) / (lengths[i + 1] - lengths[i])).clamp(0.0, 1.0);
    STREAM[i] + (STREAM[i + 1] - STREAM[i]) * u
}

/// Höhe des Bachbetts alle `STREAM_STEP` Meter: folgt dem tiefsten Gelände (natürliche Höhe
/// minus 1,2 m), fällt aber stetig; an der Wasserfallkante 5 m tiefer; nie unter den See.
fn stream_beds() -> &'static Vec<f32> {
    static BEDS: std::sync::OnceLock<Vec<f32>> = std::sync::OnceLock::new();
    BEDS.get_or_init(|| {
        let lengths = stream_lengths();
        let total = lengths[STREAM.len() - 1];
        let edge = lengths[WATERFALL_SEGMENT] + (lengths[WATERFALL_SEGMENT + 1] - lengths[WATERFALL_SEGMENT]) * WATERFALL_EDGE;
        let count = (total / STREAM_STEP).ceil() as usize + 1;
        let lowest = LAKE_LEVEL - 0.6;
        let mut beds: Vec<f32> = Vec::with_capacity(count);
        for k in 0..count {
            let s = (k as f32 * STREAM_STEP).min(total);
            let mut bed = height_raw(stream_point(s)) - 1.2;
            if let Some(&previous) = beds.last() {
                let falls_over_edge = s >= edge && s - STREAM_STEP < edge;
                bed = bed.min(previous - if falls_over_edge { 5.0 } else { 0.05 });
            }
            beds.push(bed.max(lowest));
        }
        // Zum See hin sanft auslaufen
        if let Some(last) = beds.last_mut() {
            *last = lowest;
        }
        beds
    })
}

/// Wo liegt `p` zum Bach? Abstand zur Mittellinie, Abschnitt und Lage darin (0..1).
fn stream_nearest(p: Vec2) -> (f32, usize, f32) {
    let mut best = (f32::MAX, 0, 0.0);
    for i in 0..STREAM.len() - 1 {
        let (a, b) = (STREAM[i], STREAM[i + 1]);
        let ab = b - a;
        let u = ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
        let d = p.distance(a + ab * u);
        if d < best.0 {
            best = (d, i, u);
        }
    }
    best
}

/// Höhe des Bachbetts im Abschnitt `i` an der Stelle `u`.
fn stream_bed(i: usize, u: f32) -> f32 {
    let lengths = stream_lengths();
    let s = lengths[i] + (lengths[i + 1] - lengths[i]) * u;
    let beds = stream_beds();
    let x = s / STREAM_STEP;
    let k = (x.floor() as usize).min(beds.len() - 2);
    let f = (x - k as f32).clamp(0.0, 1.0);
    beds[k] + (beds[k + 1] - beds[k]) * f
}

/// Der Bach im Spiel: Wasseroberfläche und Wasserfall-Vorhang (nur mit Fenster), eine Brücke
/// dort, wo ein Pfad ihn kreuzt, Namen für die Karte. Liefert Sperrzonen für Bäume.
fn build_stream(ctx: &mut Context, terrain: &Terrain, paths: &Paths, places: &mut crate::orte::Places) -> Vec<(Vec2, f32)> {
    let lengths = stream_lengths();
    let total = lengths[STREAM.len() - 1];
    let edge = lengths[WATERFALL_SEGMENT] + (lengths[WATERFALL_SEGMENT + 1] - lengths[WATERFALL_SEGMENT]) * WATERFALL_EDGE;
    let (top, foot) = waterfall();
    places.waterfall = Some((top, foot));
    places.labels.push(("Wasserfall", vec2(foot.x, foot.z) + (vec2(foot.x, foot.z) - vec2(top.x, top.z)).normalize_or(Vec2::Y) * 4.0));
    places.labels.push(("Silberbach", stream_point(total * 0.78) + vec2(9.0, 0.0)));
    let water_at = |s: f32| {
        let p = stream_point(s);
        let (_, i, u) = stream_nearest(p);
        vec3(p.x, stream_bed(i, u) + STREAM_DEPTH, p.y)
    };
    let across = |s: f32| {
        let ahead = stream_point((s + 1.0).min(total)) - stream_point((s - 1.0).max(0.0));
        let side = ahead.normalize_or(Vec2::X).perp();
        vec3(side.x, 0.0, side.y)
    };

    if !ctx.is_headless() {
        // Wasserband: oberhalb und unterhalb des Wasserfalls
        let half = STREAM_BED + 0.7;
        let mut water = MeshData { double_sided: true, ..Default::default() };
        for (from, to) in [(0.0, edge - STREAM_STEP), (edge + STREAM_STEP, total)] {
            let mut s = from;
            while s < to {
                let next = (s + 1.0).min(to);
                let (a, b) = (water_at(s), water_at(next));
                let (sa, sb) = (across(s) * half, across(next) * half);
                water.push_triangle(a - sa, b - sb, a + sa, Vec3::ONE);
                water.push_triangle(a + sa, b - sb, b + sb, Vec3::ONE);
                s = next;
            }
        }
        let mesh = ctx.assets.add_mesh(water);
        ctx.scene.spawn(Entity::new("Silberbach", mesh).with_material(Material::Water));

        // Wasserfall: einzelne Wasserstränge (schmale Bänder mit Lücken), oben bläulich und klar,
        // nach unten weiß aufgeschäumt; leicht nach vorne gewölbt, von beiden Seiten sichtbar.
        let mut curtain = MeshData { double_sided: true, ..Default::default() };
        let side = across(edge);
        let forward = (foot - top).with_y(0.0).normalize_or(Vec3::X);
        let width = STREAM_BED;
        let strands = 11;
        let rows = 12;
        let mut rng = Rng::new(SEED as u64 ^ 0x5A55_E4F4);
        for k in 0..strands {
            let center = (k as f32 + 0.5) / strands as f32 * 2.0 - 1.0 + rng.range(-0.04, 0.04);
            let half = rng.range(0.28, 0.42) / strands as f32 * 2.0;
            let reach = rng.range(0.25, 0.55);
            let start_row = if rng.chance(0.25) { 1 } else { 0 };
            let point = |u: f32, r: usize| {
                let v = r as f32 / rows as f32;
                let bulge = (v * std::f32::consts::PI).sin() * 0.3 + v * reach;
                top + side * (u * width) + forward * (bulge + 0.05) + Vec3::Y * ((foot.y - top.y) * v)
            };
            for r in start_row..rows {
                let v = r as f32 / rows as f32;
                let color = vec3(0.45, 0.68, 0.85).lerp(vec3(0.95, 0.98, 1.0), smoothstep(0.1, 0.8, v)) * rng.range(0.92, 1.05);
                let (a, b) = (point(center - half, r), point(center + half, r));
                let (c, d) = (point(center - half * 1.1, r + 1), point(center + half * 1.1, r + 1));
                curtain.push_triangle(a, b, c, color);
                curtain.push_triangle(b, d, c, color);
            }
        }
        let mesh = ctx.assets.add_mesh(curtain);
        ctx.scene.spawn(Entity::new("Wasserfall", mesh).with_material(Material::Emissive { glow: 0.15 }));
    }

    // Brücke: wo ein Pfad den Bach kreuzt, sonst in der Ebene vor dem See
    let crossing = (10..(total as usize - 10))
        .step_by(2)
        .map(|s| s as f32)
        .filter(|&s| (s - edge).abs() > 12.0)
        .find(|&s| paths.at(stream_point(s)) > 0.5)
        .unwrap_or(total * 0.72);
    let center = stream_point(crossing);
    let along = across(crossing);
    let along = vec2(along.x, along.z);
    let end_height = |sign: f32| {
        let q = center + along * 4.5 * sign;
        terrain.height_at(q.x, q.y)
    };
    let base = vec3(center.x, end_height(1.0).min(end_height(-1.0)) - 0.1, center.y);
    let rotation = Quat::from_rotation_y((-along.y).atan2(along.x));
    if let Some((mesh, _)) = asset_files::load_variants(ctx, "gebaeude", "bruecke", Vec3::ONE, 0.0).first().copied() {
        let entity = ctx.scene.spawn(Entity::new("Brücke", mesh).with_transform(Transform::from_position(base).with_rotation(rotation)));
        // Begehbarer Bogen aus sechs geneigten Kästen
        const LENGTH: f32 = 9.0;
        const ARCH: f32 = 0.9;
        for k in 0..6 {
            let x = -LENGTH / 2.0 + LENGTH * (k as f32 + 0.5) / 6.0;
            let y = ARCH * (1.0 - (x / (LENGTH / 2.0)).powi(2)) + 0.12;
            let slope = (-2.0 * ARCH * x / (LENGTH / 2.0).powi(2)).atan();
            let transform = Transform::from_position(base + rotation * vec3(x, y, 0.0)).with_rotation(rotation * Quat::from_rotation_z(slope));
            ctx.physics.add_body(entity, &transform, BodyDesc::fixed(Shape::Box { size: vec3(LENGTH / 6.0 + 0.1, 0.16, 1.8) }));
        }
        places.labels.push(("Brücke", center));
    }
    vec![(center, 6.0), (vec2(foot.x, foot.z), 6.0)]
}

/// Schilf rund um den Bergsee und am unteren Bach, Seerosen auf dem See (nur mit Fenster,
/// reine Deko ohne Kollision – deshalb dürfen sie vom Zufall des Rechners abhängen).
fn plant_shores(ctx: &mut Context, terrain: &Terrain, lib: &Library, places: &crate::orte::Places) {
    if ctx.is_headless() || lib.reeds.is_empty() {
        return;
    }
    let mut rng = Rng::new(SEED as u64 ^ 0x5C41_1F);
    let near_place = |p: Vec2| places.labels.iter().any(|&(name, at)| matches!(name, "Seeschrein" | "Brücke") && p.distance(at) < 7.0);
    // Ufer des Bergsees: dort, wo das Gelände knapp über dem Wasser liegt
    for k in 0..220 {
        let w = k as f32 / 220.0 * std::f32::consts::TAU + rng.range(-0.01, 0.01);
        let dir = Vec2::from_angle(w);
        let Some(p) = (0..60)
            .map(|i| LAKE_CENTER + dir * (LAKE_RADIUS - 14.0 + i as f32 * 0.5))
            .find(|q| terrain.height_at(q.x, q.y) > LAKE_LEVEL - 0.25)
        else {
            continue;
        };
        if near_place(p) || !rng.chance(0.55) {
            continue;
        }
        let spot = p + dir * rng.range(-1.2, 0.8) + dir.perp() * rng.range(-0.6, 0.6);
        let base = vec3(spot.x, terrain.height_at(spot.x, spot.y).max(LAKE_LEVEL - 0.3), spot.y);
        let reed = lib.reeds[rng.next_u32() as usize % lib.reeds.len()];
        decor(ctx, reed, base, Quat::from_rotation_y(rng.range(0.0, std::f32::consts::TAU)), rng.range(0.8, 1.25), Vec4::ONE, GRASS);
    }
    // Seerosen: Flecken auf dem Wasser nahe am Ufer
    for _ in 0..16 {
        if lib.lilies.is_empty() {
            break;
        }
        let dir = Vec2::from_angle(rng.range(0.0, std::f32::consts::TAU));
        let at = LAKE_CENTER + dir * rng.range(LAKE_RADIUS * 0.45, LAKE_RADIUS - 6.0);
        if terrain.height_at(at.x, at.y) > LAKE_LEVEL - 0.4 || near_place(at) {
            continue;
        }
        let lily = lib.lilies[rng.next_u32() as usize % lib.lilies.len()];
        let transform = Transform::from_position(vec3(at.x, LAKE_LEVEL + 0.02, at.y))
            .with_rotation(Quat::from_rotation_y(rng.range(0.0, std::f32::consts::TAU)))
            .with_scale(Vec3::splat(rng.range(1.0, 1.6)));
        ctx.scene.spawn(Entity::new("Seerosen", lily.0).with_transform(transform).with_material(Material::Foliage { sway: 0.0 }));
    }
    // Schilf am unteren Bach (unterhalb des Wasserfalls)
    let lengths = stream_lengths();
    let total = lengths[STREAM.len() - 1];
    let mut s = lengths[WATERFALL_SEGMENT + 1];
    while s < total - 4.0 {
        s += rng.range(3.0, 7.0);
        let p = stream_point(s);
        let ahead = (stream_point((s + 1.0).min(total)) - stream_point(s - 1.0)).normalize_or(Vec2::X);
        let side = ahead.perp() * if rng.chance(0.5) { 1.0 } else { -1.0 };
        let spot = p + side * rng.range(STREAM_BED + 0.2, STREAM_BED + 1.4);
        if near_place(spot) {
            continue;
        }
        let base = vec3(spot.x, terrain.height_at(spot.x, spot.y), spot.y);
        let reed = lib.reeds[rng.next_u32() as usize % lib.reeds.len()];
        decor(ctx, reed, base, Quat::from_rotation_y(rng.range(0.0, std::f32::consts::TAU)), rng.range(0.7, 1.1), Vec4::ONE, GRASS);
    }
}

/// Stellen am Meer, an denen Wellen an den Strand rollen: Punkt auf der Wasserlinie und
/// Richtung aufs Land (x, z).
fn find_surf(terrain: &Terrain) -> Vec<(Vec3, Vec2)> {
    let mut surf = Vec::new();
    for k in 0..560 {
        let dir = Vec2::from_angle(k as f32 / 560.0 * std::f32::consts::TAU);
        // Von außen nach innen zur ersten Stelle, an der das Land aus dem Wasser steigt
        let Some(r) = (0..(ISLAND_RADIUS * 0.9) as i32).map(|i| ISLAND_RADIUS * 1.35 - i as f32 * 1.0).find(|&r| {
            let p = dir * r;
            terrain.height_at(p.x, p.y) > -0.05
        }) else {
            continue;
        };
        let p = dir * (r + 0.5);
        // Nur flache Strände (kein Steilufer)
        let inland = dir * (r - 8.0);
        if terrain.height_at(inland.x, inland.y) > 2.2 {
            continue;
        }
        surf.push((vec3(p.x, 0.0, p.y), -dir));
    }
    surf
}

/// Wasserfall: obere Kante und Fuß (Punkte auf der Wasseroberfläche).
pub fn waterfall() -> (Vec3, Vec3) {
    let lengths = stream_lengths();
    let edge = lengths[WATERFALL_SEGMENT] + (lengths[WATERFALL_SEGMENT + 1] - lengths[WATERFALL_SEGMENT]) * WATERFALL_EDGE;
    let at = |s: f32| {
        let p = stream_point(s);
        let (_, i, u) = stream_nearest(p);
        vec3(p.x, stream_bed(i, u) + STREAM_DEPTH, p.y)
    };
    (at(edge - STREAM_STEP), at(edge + STREAM_STEP))
}

/// Abstand zum Bach (für Pflanzen, Bodenfarbe, Karte).
pub fn stream_distance(p: Vec2) -> f32 {
    stream_nearest(p).0
}

// ---------------------------------------------------------------------------
// Burg Grünfels auf dem Tafelberg
// ---------------------------------------------------------------------------

/// Ursprung des Burgmodells (Mitte der Burg) und Höhe des Plateaus, auf dem sie steht.
pub const BURG_ORT: Vec2 = vec2(0.45 * ISLAND_RADIUS + 25.0, 0.15 * ISLAND_RADIUS);
pub const BURG_HOEHE: f32 = 23.5;
pub const BURG_NAME: &str = "Burg Grünfels";

/// Modellkoordinaten der Burganlage (Engine-Achsen x, z; das Tor liegt bei +z) → Welt.
/// Das Tor zeigt nach Westen, zum Startlager hin.
pub fn burg_welt(l: Vec2) -> Vec2 {
    BURG_ORT + vec2(-l.y, l.x)
}

fn burg_lokal(p: Vec2) -> Vec2 {
    let d = p - BURG_ORT;
    vec2(d.y, -d.x)
}

/// Drehung des Modells: lokales +Z (Tor) zeigt nach Westen (-X).
pub fn burg_drehung() -> Quat {
    Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)
}

/// Abstand zum Rand des Plateaus (negativ = oben drauf): abgerundetes Rechteck um Ringmauer,
/// Torhaus mit Vorplatz und Marktplatz.
pub fn burg_rand(p: Vec2) -> f32 {
    let l = burg_lokal(p) - vec2(0.0, 25.0);
    let q = l.abs() - vec2(50.0, 48.0);
    q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0) - 20.0
}

/// Die Auffahrt in Modellkoordinaten: ebener Absatz vor dem Tor, dann in einem Bogen am Hang
/// entlang hinab bis in die Ebene.
const BURG_WEG: [Vec2; 6] = [vec2(0.0, 86.0), vec2(0.0, 104.0), vec2(12.0, 117.0), vec2(36.0, 127.0), vec2(70.0, 133.0), vec2(118.0, 134.0)];
/// Halbe Breite der Fahrbahn (daneben Böschungen)
const BURG_WEG_BREITE: f32 = 4.5;

/// Höhe der Auffahrt an jedem Punkt von BURG_WEG: oben auf dem Plateau, dann gleichmäßig
/// fallend bis zum Gelände am Fuß.
fn burg_weg_hoehen() -> &'static [f32; 6] {
    static HOEHEN: std::sync::OnceLock<[f32; 6]> = std::sync::OnceLock::new();
    HOEHEN.get_or_init(|| {
        let mut laenge = [0.0f32; 6];
        for i in 1..6 {
            laenge[i] = laenge[i - 1] + BURG_WEG[i].distance(BURG_WEG[i - 1]);
        }
        let fuss = height_raw(burg_welt(BURG_WEG[5])).max(1.5) + 0.2;
        let mut hoehen = [BURG_HOEHE; 6];
        for i in 2..6 {
            let t = (laenge[i] - laenge[1]) / (laenge[5] - laenge[1]);
            hoehen[i] = BURG_HOEHE + (fuss - BURG_HOEHE) * t;
        }
        hoehen
    })
}

/// Abstand zur Mittellinie der Auffahrt und die Höhe der Fahrbahn an der nächsten Stelle.
pub fn burg_weg(p: Vec2) -> (f32, f32) {
    let l = burg_lokal(p);
    let hoehen = burg_weg_hoehen();
    let mut best = (f32::MAX, 0.0);
    for i in 0..BURG_WEG.len() - 1 {
        let (a, b) = (BURG_WEG[i], BURG_WEG[i + 1]);
        let ab = b - a;
        let t = ((l - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0);
        let d = l.distance(a + ab * t);
        if d < best.0 {
            best = (d, hoehen[i] + (hoehen[i + 1] - hoehen[i]) * t);
        }
    }
    best
}

/// Unterer Endpunkt der Auffahrt (Ziel des Trampelpfads vom Lager).
pub fn burg_weg_fuss() -> Vec2 {
    burg_welt(BURG_WEG[BURG_WEG.len() - 1])
}

/// Auf dem Plateau und der Auffahrt wächst nichts und steht nichts anderes.
pub fn burg_frei(p: Vec2) -> bool {
    burg_rand(p) < 12.0 || burg_weg(p).0 < BURG_WEG_BREITE + 7.0
}

/// Burgberg: oben eben, zu den Seiten steile, zerklüftete Hänge; dazu die Auffahrt mit Böschungen.
fn burgberg(p: Vec2, h: f32) -> f32 {
    let d = burg_rand(p);
    if d > 170.0 {
        return h;
    }
    let mut out = h;
    if d < 45.0 {
        let rau = fbm(p * 0.07, 2, SEED + 41) * 2.0 * smoothstep(2.0, 12.0, d);
        let hang = BURG_HOEHE - d.max(0.0) * 0.85 + rau;
        let ziel = h.max(hang);
        out = ziel + (BURG_HOEHE - ziel) * (1.0 - smoothstep(0.0, 8.0, d));
    }
    let (abstand, weg) = burg_weg(p);
    if abstand < BURG_WEG_BREITE + 9.0 {
        let w = 1.0 - smoothstep(BURG_WEG_BREITE, BURG_WEG_BREITE + 9.0, abstand);
        out += (weg - out) * w;
    }
    out
}

/// Karte: Mauerring mit Türmen, Burg mit grünem Dach, Markthäuser mit roten Dächern, Pflaster.
fn burg_karte(p: Vec2) -> Option<Vec3> {
    if burg_rand(p) > 0.0 {
        return None;
    }
    let l = burg_lokal(p);
    let turm = [(-60.0, -32.0), (60.0, -32.0), (-60.0, 75.0), (60.0, 75.0), (-7.6, 76.2), (7.6, 76.2)]
        .iter()
        .any(|&(x, z)| l.distance(vec2(x, z)) < 5.5);
    let mauer = ((l.x.abs() - 60.0).abs() < 1.6 && (-33.6..76.6).contains(&l.y))
        || (((l.y + 32.0).abs() < 1.6 || (l.y - 75.0).abs() < 1.6) && l.x.abs() < 61.6);
    let burg = (l.x.abs() < 24.5 && l.y.abs() < 12.0) || (l.x.abs() < 4.4 && (0.0..15.5).contains(&l.y));
    let tuerme = l.distance(vec2(-28.0, 9.5)) < 4.5 || l.distance(vec2(28.3, 6.0)) < 5.0;
    let haeuser = (-57.0..-49.0).contains(&l.x) && (27.0..62.0).contains(&l.y);
    Some(if turm || tuerme {
        vec3(0.2, 0.42, 0.26)
    } else if mauer {
        vec3(0.36, 0.3, 0.24)
    } else if burg {
        vec3(0.26, 0.52, 0.3)
    } else if haeuser {
        vec3(0.55, 0.22, 0.14)
    } else {
        vec3(0.5, 0.46, 0.4)
    })
}

// ---------------------------------------------------------------------------
// Schattenfestung in der Inselmitte
// ---------------------------------------------------------------------------

pub const FESTUNG_NAME: &str = "Schattenfestung";

/// Grundriss der Festung (Mitte = Inselmitte, das Tor zeigt nach Süden, +z): der Felssockel
/// (Radius 44 m) und die Rampe davor bis z ≈ 72. Abstand zum Rand, negativ = drinnen.
pub fn festung_rand(p: Vec2) -> f32 {
    let kreis = p.length() - 44.0;
    let q = vec2(p.x.abs() - 5.5, (p.y - 36.0).abs() - 36.0);
    let rampe = q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0);
    kreis.min(rampe)
}

/// Höhe, auf der die Festung steht: Mittel des natürlichen Geländes unter dem Grundriss.
pub fn festung_hoehe() -> f32 {
    static HOEHE: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *HOEHE.get_or_init(|| {
        let mut summe = 0.0;
        let mut anzahl = 0.0;
        for iz in -8..=8 {
            for ix in -8..=8 {
                let p = vec2(ix as f32, iz as f32) * 5.0;
                if festung_rand(p) < 0.0 {
                    summe += height_raw(p);
                    anzahl += 1.0;
                }
            }
        }
        (summe / anzahl).max(3.0)
    })
}

/// Unter der Festung und der Rampe ist der Boden eben, nach außen läuft er weich aus.
fn festungsgrund(p: Vec2, h: f32) -> f32 {
    let d = festung_rand(p);
    if d > 30.0 {
        return h;
    }
    let eben = festung_hoehe();
    eben + (h - eben) * smoothstep(0.0, 25.0, d)
}

/// Auf dem Festungsgrund wächst nichts und steht nichts anderes.
pub fn festung_frei(p: Vec2) -> bool {
    festung_rand(p) < 8.0
}

/// Karte: dunkler Fels, Ringmauer, violetter Bergfried.
fn festung_karte(p: Vec2) -> Option<Vec3> {
    let r = p.length();
    if r > 40.0 {
        return None;
    }
    Some(if r < 9.0 {
        vec3(0.34, 0.14, 0.48)
    } else if (r - 28.0).abs() < 1.8 {
        vec3(0.1, 0.08, 0.12)
    } else {
        vec3(0.22, 0.2, 0.25)
    })
}

/// Höhe der Landschaft an (x, z) – mit eingegrabenem Bach, dem Burgberg und dem Festungsgrund.
pub fn height(p: Vec2) -> f32 {
    burgberg(p, festungsgrund(p, height_mit_bach(p)))
}

fn height_mit_bach(p: Vec2) -> f32 {
    let raw = height_raw(p);
    let (d, i, u) = stream_nearest(p);
    if d > STREAM_BANK + 8.0 {
        return raw;
    }
    let bed = stream_bed(i, u);
    // Wo das Gelände neben dem Bach tiefer liegt als das Wasser: sanfter Uferwall statt Damm
    let bank = bed + STREAM_DEPTH + 0.5;
    let raised = raw.max(raw + (bank - raw) * (1.0 - smoothstep(STREAM_BANK, STREAM_BANK + 8.0, d)));
    let carve = 1.0 - smoothstep(STREAM_BED, STREAM_BANK, d);
    raised + (bed - raised) * carve
}

/// Höhe der Landschaft ohne Bach.
fn height_raw(p: Vec2) -> f32 {
    let r = ISLAND_RADIUS;
    let distance = p.length() / r;
    // Küste mit Buchten und Halbinseln
    let coast = fbm(p * 0.004, 4, SEED) * 0.38;
    let land = 1.0 - smoothstep(0.58, 1.02, distance + coast);

    // Hügel überall, dazu großflächige Wellen im Gelände
    let hills = fbm(p * 0.009, 4, SEED + 1) * 6.0 + 5.0;
    let swells = fbm(p * 0.0035, 2, SEED + 9) * 7.0;
    // Gebirge im Norden (negatives z), zur Küste hin auslaufend.
    let north = smoothstep(-0.05 * r, -0.45 * r, p.y + fbm(p * 0.006, 2, SEED + 2) * 60.0);
    // r·√r statt powf: Wurzeln rechnen auf jedem System bitgenau gleich.
    let ridge = ridged(p * 0.009, 5, SEED + 3);
    let peaks = ridge * ridge.sqrt() * 58.0 * north * (1.0 - smoothstep(0.72, 0.95, distance));
    // Tafelberg im Osten: steile Ränder, oben in Stufen abgesetzt
    let east = vec2(0.45 * r, 0.15 * r);
    let edge = (p - east).length() / (0.22 * r) + fbm(p * 0.012, 2, SEED + 11) * 0.25;
    let steps = smoothstep(1.0, 0.72, edge) * 3.0;
    let plateau = (steps.floor() + smoothstep(0.7, 1.0, steps.fract())) / 3.0 * 16.0;
    // Gewundene Täler, wo das Rauschen die Null kreuzt (im Gebirge flacher)
    let valley = (1.0 - smoothstep(0.0, 0.07, fbm(p * 0.005, 3, SEED + 15).abs())) * 6.0 * (1.0 - north * 0.7);
    let mut inland = hills + swells.max(-2.0) + peaks + plateau - valley;

    // Bergsee: Rand etwas erhöht, darin eine Mulde unter den Wasserspiegel
    let to_lake = (p - LAKE_CENTER).length();
    // Der See liegt auf einer Hochebene: das Land ringsum steigt sanft zu seinem Rand an
    inland = inland.max((LAKE_LEVEL + 1.4) * smoothstep(LAKE_RADIUS + 95.0, LAKE_RADIUS + 10.0, to_lake));
    let bowl = smoothstep(LAKE_RADIUS + 4.0, LAKE_RADIUS - 14.0, to_lake);
    inland += (LAKE_LEVEL - 2.5 - inland) * bowl;

    // Flacher Strand: nahe der Küstenlinie wird die Höhe zusammengedrückt.
    let shaped = -9.0 + (inland + 9.0) * land.sqrt() * land.sqrt().sqrt();
    if shaped > 0.0 && shaped < 3.0 { shaped * (0.55 + shaped * 0.15) } else { shaped }
}

pub fn moisture(p: Vec2) -> f32 {
    fbm(p * 0.008 + vec2(40.0, -13.0), 3, SEED + 5) * 0.5 + 0.5
}

fn magic(p: Vec2) -> f32 {
    fbm(p * 0.009 + vec2(-71.0, 22.0), 3, SEED + 7) * 0.5 + 0.5
}

/// Wo Birken in Hainen zusammenstehen (> 0.64).
fn birch_grove(p: Vec2) -> f32 {
    fbm(p * 0.02 + vec2(13.0, 57.0), 2, SEED + 13) * 0.5 + 0.5
}

/// Auflösung der Bodentextur (über die ganze Landschaft, ≈ 0,46 m je Pixel).
const GROUND_TEXTURE: usize = 3072;
/// Rasterweite der Wege-Maske in Metern.
const PATH_CELL: f32 = 1.0;

/// Trampelpfade als Maske über die ganze Landschaft: 0 = kein Weg, 1 = Wegmitte.
pub struct Paths {
    mask: Vec<f32>,
    res: usize,
}

impl Paths {
    fn origin() -> Vec2 {
        Vec2::splat(-TERRAIN_SIZE / 2.0)
    }

    /// Wege vom Startpunkt zum Bergsee, zum Strand, zum Tafelberg und ins Gebirge.
    fn build(terrain: &Terrain, spawn: Vec3) -> Paths {
        let res = (TERRAIN_SIZE / PATH_CELL) as usize;
        let mut paths = Paths { mask: vec![0.0; res * res], res };
        let start = vec2(spawn.x, spawn.z);
        let r = ISLAND_RADIUS;
        let mut targets = Vec::new();
        // Seeufer auf der Seite zum Startpunkt
        targets.push(LAKE_CENTER + (start - LAKE_CENTER).normalize() * (LAKE_RADIUS + 9.0));
        // Fuß der Auffahrt zur Burg auf dem Tafelberg
        targets.push(burg_weg_fuss() + (start - burg_weg_fuss()).normalize_or(Vec2::X) * 3.0);
        // Fuß der Rampe zur Schattenfestung in der Inselmitte
        targets.push(vec2(0.0, 76.0));
        // Strand: vom Startpunkt nach außen, bis der Sand beginnt
        let outward = start.normalize_or(Vec2::Y);
        if let Some(beach) = find_along(terrain, start, outward, |h| h < 2.6) {
            targets.push(beach);
        }
        // Gebirge: Richtung Norden, bis das Gelände steil ansteigt
        if let Some(foot) = find_along(terrain, start, (vec2(0.0, -0.45 * r) - start).normalize(), |h| h > 17.0) {
            targets.push(foot);
        }
        let mut trails: Vec<Vec<Vec2>> = targets.iter().enumerate().map(|(i, &to)| trail(terrain, start, to, i as u32)).collect();
        // Querweg vom Bergsee zum Gebirge
        if trails.len() >= 4 {
            let (from, to) = (*trails[0].last().expect("Weg hat Punkte"), *trails[3].last().expect("Weg hat Punkte"));
            trails.push(trail(terrain, from, to, 9));
        }
        // Die Auffahrt selbst: breiter, festgefahrener Weg
        let weg: Vec<Vec2> = BURG_WEG.iter().map(|&l| burg_welt(l)).collect();
        for pair in weg.windows(2) {
            paths.stamp(pair[0], pair[1], 3.4);
        }
        for (index, trail) in trails.iter().enumerate() {
            for (k, pair) in trail.windows(2).enumerate() {
                // Breite schwankt leicht; zum Ende hin (am Ziel) läuft der Weg aus.
                let along = k as f32 / trail.len().max(2) as f32;
                let wobble = fbm(pair[0] * 0.05, 2, SEED + 30 + index as u32) * 0.35;
                let width = (1.15 + wobble) * (1.0 - smoothstep(0.85, 1.0, along) * 0.6);
                paths.stamp(pair[0], pair[1], width);
            }
        }
        paths
    }

    /// Trägt ein Wegstück ein (`width` = halbe Breite des festgetretenen Teils).
    fn stamp(&mut self, a: Vec2, b: Vec2, width: f32) {
        let reach = width + 1.5;
        let (low, high) = (a.min(b) - Vec2::splat(reach), a.max(b) + Vec2::splat(reach));
        let cell = |v: f32| ((v - Self::origin().x) / PATH_CELL).floor().clamp(0.0, (self.res - 1) as f32) as usize;
        let segment = b - a;
        let length_sq = segment.length_squared().max(1e-6);
        for z in cell(low.y)..=cell(high.y) {
            for x in cell(low.x)..=cell(high.x) {
                let p = Self::origin() + (vec2(x as f32, z as f32) + 0.5) * PATH_CELL;
                let t = ((p - a).dot(segment) / length_sq).clamp(0.0, 1.0);
                let distance = p.distance(a + segment * t);
                let value = 1.0 - smoothstep(width * 0.55, reach, distance);
                let slot = &mut self.mask[z * self.res + x];
                *slot = slot.max(value);
            }
        }
    }

    /// Wie sehr liegt der Punkt auf einem Weg (0..1, weich interpoliert)?
    pub fn at(&self, p: Vec2) -> f32 {
        let local = (p - Self::origin()) / PATH_CELL - Vec2::splat(0.5);
        let base = local.floor();
        let f = local - base;
        let get = |dx: i32, dz: i32| {
            let (x, z) = (base.x as i32 + dx, base.y as i32 + dz);
            if x < 0 || z < 0 || x >= self.res as i32 || z >= self.res as i32 { 0.0 } else { self.mask[z as usize * self.res + x as usize] }
        };
        let top = get(0, 0) + (get(1, 0) - get(0, 0)) * f.x;
        let bottom = get(0, 1) + (get(1, 1) - get(0, 1)) * f.x;
        top + (bottom - top) * f.y
    }
}

/// Erster Punkt entlang einer Richtung, an dem die Höhe `wanted` erfüllt (höchstens 400 m weit).
fn find_along(terrain: &Terrain, from: Vec2, direction: Vec2, wanted: impl Fn(f32) -> bool) -> Option<Vec2> {
    (1..100).map(|i| from + direction * (i as f32 * 4.0)).find(|p| wanted(terrain.height_at(p.x, p.y)) && !in_lake(*p)).map(|p| p - direction * 4.0)
}

/// Ein gewundener Pfad von `from` nach `to`: in 3-m-Schritten, meidet Wasser und steile
/// Anstiege und schlängelt sich leicht. Nur Grundrechenarten und Wurzeln (keine
/// Winkelfunktionen), damit jeder Rechner exakt denselben Weg findet.
fn trail(terrain: &Terrain, from: Vec2, to: Vec2, seed: u32) -> Vec<Vec2> {
    const STEP: f32 = 3.0;
    // Drehung um 0,12 rad (cos, sin) als feste Zahlen
    const TURN: (f32, f32) = (0.992_808_6, 0.119_712_21);
    let rotate = |v: Vec2, times: i32| {
        let mut v = v;
        for _ in 0..times.abs() {
            let s = if times > 0 { TURN.1 } else { -TURN.1 };
            v = vec2(v.x * TURN.0 - v.y * s, v.x * s + v.y * TURN.0);
        }
        v.normalize()
    };
    let mut points = vec![from];
    let mut here = from;
    let mut heading = (to - from).normalize_or(Vec2::Y);
    for _ in 0..600 {
        let remaining = here.distance(to);
        if remaining < STEP * 1.5 {
            points.push(to);
            break;
        }
        let goal = (to - here) / remaining;
        // Schlängeln: seitlich vom direkten Weg abweichen, kurz vor dem Ziel nicht mehr.
        let wander = fbm(here * 0.012 + vec2(seed as f32 * 17.3, 4.0), 2, SEED + 21) * 1.2 * smoothstep(10.0, 50.0, remaining);
        let preferred = (goal + goal.perp() * wander).normalize();
        let here_height = terrain.height_at(here.x, here.y);
        let mut best: Option<(f32, Vec2)> = None;
        for k in -5..=5 {
            let direction = rotate(heading, k);
            let next = here + direction * STEP;
            let h = terrain.height_at(next.x, next.y);
            if h < 1.8 || in_lake(next) {
                continue;
            }
            let rise = (h - here_height).abs() / STEP;
            let cost = (1.0 - direction.dot(preferred)) * 4.0 + rise * rise * 12.0 + (k as f32).abs() * 0.02;
            if best.is_none_or(|(c, _)| cost < c) {
                best = Some((cost, direction));
            }
        }
        let Some((_, direction)) = best else { break };
        heading = direction;
        here += direction * STEP;
        points.push(here);
    }
    points
}

/// Farbe des Bodens (linear) an der Stelle `c` mit Normale `n`. Wege malt erst der Shader
/// (siehe `ground_texture`), damit ihre Ränder auch aus der Nähe scharf sind.
fn ground_color(c: Vec3, n: Vec3) -> Vec3 {
    let p = vec2(c.x, c.z);
    let slope = 1.0 - n.y;
    let meadow = vec3(0.085, 0.215, 0.04);
    let forest = vec3(0.045, 0.145, 0.03);
    let enchanted = vec3(0.03, 0.2, 0.2);
    let alpine = vec3(0.22, 0.30, 0.11);
    let sand = vec3(0.56, 0.43, 0.22);
    let wet_sand = vec3(0.30, 0.25, 0.15);
    let rock = vec3(0.21, 0.20, 0.19);
    let snow = vec3(0.86, 0.89, 0.94);

    let mut color = meadow.lerp(forest, smoothstep(0.45, 0.6, moisture(p)));
    // Wiesen nicht einfarbig: großflächig hellere (trockenere) und sattere, dunklere Flecken.
    let patches = fbm(p * 0.03 + vec2(9.0, 4.0), 2, SEED + 11) * 0.5 + 0.5;
    color = color.lerp(color * vec3(1.35, 1.12, 0.8), smoothstep(0.55, 0.8, patches) * 0.6);
    color = color.lerp(color * 0.72, smoothstep(0.45, 0.2, patches) * 0.5);
    color = color.lerp(enchanted, smoothstep(0.58, 0.66, magic(p)));
    // Waldboden: Laub und Erde in Flecken, wo es feucht ist
    let litter = fbm(p * 0.07 + vec2(-3.0, 8.0), 3, SEED + 17) * 0.5 + 0.5;
    let forest_floor = smoothstep(0.5, 0.62, moisture(p)) * smoothstep(0.5, 0.72, litter);
    color = color.lerp(vec3(0.11, 0.075, 0.035), forest_floor * 0.55);
    color = color.lerp(alpine, smoothstep(15.0, 24.0, c.y));
    // Trockenes, gelbliches Gras als Saum zum Strand
    color = color.lerp(vec3(0.26, 0.25, 0.07), smoothstep(3.6, 2.5, c.y) * 0.7);
    // Bachbett: nasse Kiesel, daneben feuchte, dunklere Erde
    let river = stream_distance(p);
    if river < STREAM_BANK + 2.0 {
        color = color.lerp(color * 0.72, (1.0 - smoothstep(STREAM_BED + 1.0, STREAM_BANK + 2.0, river)) * 0.6);
        color = color.lerp(vec3(0.15, 0.14, 0.12), 1.0 - smoothstep(STREAM_BED - 0.2, STREAM_BED + 1.2, river));
    }
    color = sand.lerp(color, smoothstep(1.4, 2.6, c.y));
    color = wet_sand.lerp(color, smoothstep(-0.8, 0.6, c.y));
    // Seeufer: nasser, dunkler Grund rund um den Bergsee
    let ufer = smoothstep(LAKE_RADIUS + 6.0, LAKE_RADIUS - 2.0, (p - LAKE_CENTER).length()) * (1.0 - smoothstep(LAKE_LEVEL + 0.3, LAKE_LEVEL + 1.2, c.y));
    color = color.lerp(wet_sand * 0.85, ufer);
    color = color.lerp(rock, smoothstep(0.42, 0.58, slope));
    color.lerp(snow, smoothstep(29.0, 34.0, c.y) * (1.0 - smoothstep(0.55, 0.75, slope)))
}

/// Jedes Dreieck leicht anders hell – das macht den facettierten Look lebendig.
fn facet_jitter(c: Vec3) -> f32 {
    1.0 + (hash01((c.x * 5.0).floor() as i32, (c.z * 5.0).floor() as i32, SEED) - 0.5) * 0.14
}

/// Bodentextur über die ganze Landschaft (sRGB), auf allen Kernen parallel berechnet.
/// Im Alphakanal steckt der Weg-Anteil: 255 = kein Weg, 128 = Wegmitte (nie darunter, sonst
/// würde der Shader den Boden als durchsichtig verwerfen). Daraus malt `Material::Ground` die Wege.
fn ground_texture(terrain: &Terrain, paths: &Paths) -> Image {
    let size = GROUND_TEXTURE;
    let texel = TERRAIN_SIZE / size as f32;
    let origin = Vec2::splat(-TERRAIN_SIZE / 2.0);
    let mut rgba = vec![0u8; size * size * 4];
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(16);
    let rows_per_thread = size.div_ceil(threads);
    std::thread::scope(|scope| {
        for (chunk_index, chunk) in rgba.chunks_mut(rows_per_thread * size * 4).enumerate() {
            scope.spawn(move || {
                for (i, pixel) in chunk.chunks_exact_mut(4).enumerate() {
                    let (x, z) = (i % size, chunk_index * rows_per_thread + i / size);
                    let p = origin + (vec2(x as f32, z as f32) + 0.5) * texel;
                    let c = vec3(p.x, terrain.height_at(p.x, p.y), p.y);
                    let color = ground_color(c, terrain.normal_at(p.x, p.y));
                    for (k, channel) in color.to_array().into_iter().enumerate() {
                        pixel[k] = (linear_to_srgb(channel) * 255.0).round() as u8;
                    }
                    // Wege nur im Grünen: nicht auf Sand und nicht hoch im Fels
                    let path = paths.at(p) * smoothstep(2.3, 3.0, c.y) * (1.0 - smoothstep(15.0, 20.0, c.y));
                    pixel[3] = 255 - (path.clamp(0.0, 1.0) * 127.0).round() as u8;
                }
            });
        }
    });
    Image { width: size as u32, height: size as u32, rgba }
}

fn linear_to_srgb(v: f32) -> f32 {
    let v = v.clamp(0.0, 1.0);
    if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
}

/// Im Zauberwald? (für Musik und Stimmung)
pub fn is_enchanted(p: Vec2) -> bool {
    magic(p) > 0.62 && height(p) < 18.0
}

/// Bergsee: Mitte, Radius, Wasserspiegel.
pub fn lake() -> (Vec2, f32, f32) {
    (LAKE_CENTER, LAKE_RADIUS, LAKE_LEVEL)
}

/// Wiese (für Schmetterlinge): grün, flach, nicht feucht, nicht im Gebirge.
pub fn is_meadow(terrain: &Terrain, p: Vec2) -> bool {
    let h = terrain.height_at(p.x, p.y);
    (3.0..14.0).contains(&h) && moisture(p) < 0.52 && terrain.normal_at(p.x, p.y).y > 0.9 && !in_lake(p)
}

/// Wo Vogelschwärme kreisen (große Wälder) und Möwen fliegen (Strände).
pub fn wildlife_spots(terrain: &Terrain) -> (Vec<Vec2>, Vec<Vec2>) {
    let mut forests = Vec::new();
    let mut beaches = Vec::new();
    let r = ISLAND_RADIUS;
    for iz in -6..=6 {
        for ix in -6..=6 {
            let p = vec2(ix as f32, iz as f32) * r / 6.0;
            let h = terrain.height_at(p.x, p.y);
            if (3.0..18.0).contains(&h) && moisture(p) > 0.56 && forests.iter().all(|&f: &Vec2| f.distance(p) > 110.0) {
                forests.push(p);
            }
        }
    }
    for k in 0..18 {
        let dir = Vec2::from_angle(k as f32 / 18.0 * std::f32::consts::TAU);
        if let Some(rr) = (0..(r * 0.9) as i32).map(|i| r * 1.3 - i as f32).find(|&rr| terrain.height_at((dir * rr).x, (dir * rr).y) > 0.2) {
            let p = dir * rr;
            if terrain.height_at(p.x, p.y) < 2.5 && beaches.iter().all(|&b: &Vec2| b.distance(p) > 150.0) {
                beaches.push(p);
            }
        }
    }
    (forests, beaches)
}

/// Bester Punkt eines 6-m-Rasters über die Insel (höchste Wertung, `None` = ungeeignet).
fn best_spot(mut score: impl FnMut(Vec2) -> Option<f32>) -> Option<Vec2> {
    let r = ISLAND_RADIUS * 1.05;
    let steps = (r * 2.0 / 6.0) as i32;
    let mut best: Option<(f32, Vec2)> = None;
    for iz in 0..=steps {
        for ix in 0..=steps {
            let p = vec2(-r + ix as f32 * 6.0, -r + iz as f32 * 6.0);
            // Rund um die Burg und ihre Auffahrt ist kein Platz für andere Orte
            if burg_rand(p) < 30.0 || burg_weg(p).0 < 15.0 || festung_rand(p) < 40.0 {
                continue;
            }
            if let Some(s) = score(p) {
                if best.is_none_or(|(b, _)| s > b) {
                    best = Some((s, p));
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Acht Himmelsrichtungen als feste Zahlen (keine Winkelfunktionen: jeder Rechner muss exakt
/// dieselben Plätze finden, sonst stünden Bäume und Kollisionen woanders).
const DIRECTIONS: [Vec2; 8] = [
    vec2(1.0, 0.0),
    vec2(0.707_106_77, 0.707_106_77),
    vec2(0.0, 1.0),
    vec2(-0.707_106_77, 0.707_106_77),
    vec2(-1.0, 0.0),
    vec2(-0.707_106_77, -0.707_106_77),
    vec2(0.0, -1.0),
    vec2(0.707_106_77, -0.707_106_77),
];

/// Dreht um einen festen Winkel, gegeben als (cos, sin).
fn turn(v: Vec2, (c, s): (f32, f32)) -> Vec2 {
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// Ist das Gelände rund um `p` (Radius `r`) eben genug? Höchster minus tiefster Punkt.
fn unevenness(terrain: &Terrain, p: Vec2, r: f32) -> f32 {
    let heights: Vec<f32> = DIRECTIONS
        .iter()
        .map(|&d| p + d * r)
        .chain([p])
        .map(|q| terrain.height_at(q.x, q.y))
        .collect();
    heights.iter().cloned().fold(f32::MIN, f32::max) - heights.iter().cloned().fold(f32::MAX, f32::min)
}

/// Sucht passende Plätze für die Sehenswürdigkeiten (auf allen Rechnern gleich).
fn find_sights(terrain: &Terrain, spawn: Vec3, camp: Vec2) -> crate::orte::SightSpots {
    let start = vec2(spawn.x, spawn.z);
    let h = |p: Vec2| terrain.height_at(p.x, p.y);
    let near_sea = |p: Vec2, d: f32| DIRECTIONS.iter().any(|&dir| h(p + dir * d) < -0.5);
    // Wachturm: auf einer ebenen Anhöhe fern vom Lager (der Tafelberg trägt jetzt die Burg),
    // Tor Richtung Startlager
    let tower = best_spot(|p| {
        let height = h(p);
        ((9.0..20.0).contains(&height)
            && p.distance(start) > 140.0
            && p.distance(camp) > 140.0
            && !in_lake(p)
            && magic(p) < 0.58
            && unevenness(terrain, p, 4.0) < 0.9)
            .then(|| height - p.distance(vec2(0.0, 0.1 * ISLAND_RADIUS)) * 0.03)
    })
    .map(|p| (p, (camp - p).normalize_or(Vec2::X)));
    // Steinkreis: ebene Lichtung tief im Zauberwald
    let circle = best_spot(|p| {
        let height = h(p);
        ((3.0..16.0).contains(&height) && !in_lake(p) && magic(p) > 0.62 && p.distance(camp) > 60.0 && unevenness(terrain, p, 6.0) < 1.2)
            .then(|| magic(p) - unevenness(terrain, p, 6.0) * 0.1)
    });
    // Bergsee: Uferpunkt etwas neben dem Pfad vom Lager
    let toward_start = (start - LAKE_CENTER).normalize_or(Vec2::Y);
    // 0,45 rad neben dem Pfad
    let shore_dir = turn(toward_start, (0.900_447_1, 0.434_965_5)).normalize();
    let lake_shore = (0..80)
        .map(|i| LAKE_CENTER + shore_dir * (LAKE_RADIUS - 12.0 + i as f32 * 0.5))
        .find(|&p| h(p) > LAKE_LEVEL + 0.1)
        .map(|shore| (shore, -shore_dir, LAKE_LEVEL));
    // Schiffswrack: flacher Sandstrand, ein Stück seitlich vom Startplatz (0,6 rad weiter)
    let wreck_dir = turn(start.normalize_or(Vec2::Y), (0.825_335_6, 0.564_642_5)).normalize();
    let wreck = best_spot(|p| {
        let height = h(p);
        ((0.3..1.3).contains(&height) && unevenness(terrain, p, 5.0) < 0.9 && p.distance(start) > 50.0 && near_sea(p, 14.0))
            .then(|| p.normalize_or(Vec2::X).dot(wreck_dir))
    })
    .map(|p| (p, p.normalize_or(Vec2::X).perp()));
    // Leuchtturm: Anhöhe direkt an der Küste im Nordosten
    let lighthouse = best_spot(|p| {
        let height = h(p);
        ((3.5..10.0).contains(&height) && unevenness(terrain, p, 2.5) < 1.2 && near_sea(p, 16.0) && p.distance(camp) > 80.0)
            .then(|| height * 0.2 + p.normalize_or(Vec2::X).dot(vec2(0.825_335_6, -0.564_642_5)) * 3.0)
    })
    .map(|p| (p, p.normalize_or(Vec2::X)));
    // Höhle: steiler Hang im Gebirge, Öffnung talwärts
    let mountains = vec2(0.0, -0.45 * ISLAND_RADIUS);
    let cave = best_spot(|p| {
        let height = h(p);
        let normal = terrain.normal_at(p.x, p.y);
        ((14.0..30.0).contains(&height) && (0.5..0.8).contains(&normal.y)).then(|| -p.distance(mountains))
    })
    .map(|p| {
        let normal = terrain.normal_at(p.x, p.y);
        let out = vec2(normal.x, normal.z).normalize_or(Vec2::Y);
        (p + out * 1.5, out)
    });
    crate::orte::SightSpots { tower, circle, lake_shore, wreck, lighthouse, cave }
}

/// Liegt der Punkt im Bergsee (unter dem Wasserspiegel)?
pub fn in_lake(p: Vec2) -> bool {
    (p - LAKE_CENTER).length() < LAKE_RADIUS + 4.0 && height(p) < LAKE_LEVEL + 0.3
}

/// Sucht einen flachen Platz auf einer Wiese im Süden der Insel.
fn find_spawn(terrain: &Terrain) -> Vec3 {
    for distance in (20..(ISLAND_RADIUS * 0.8) as i32).rev().step_by(4) {
        for step in 0..24 {
            let angle = std::f32::consts::FRAC_PI_2 + (step as f32 - 12.0) * 0.08;
            let p = vec2(angle.cos(), angle.sin()) * distance as f32;
            let h = terrain.height_at(p.x, p.y);
            if (3.0..9.0).contains(&h) && terrain.normal_at(p.x, p.y).y > 0.93 && magic(p) < 0.55 && (p - LAKE_CENTER).length() > LAKE_RADIUS + 30.0 {
                return vec3(p.x, h, p.y);
            }
        }
    }
    vec3(0.0, height(Vec2::ZERO), 0.0)
}

/// Ein Modell für die Insel: Mesh und optional ein leuchtendes Zusatzteil.
type Variant = (MeshId, Option<MeshId>);

struct Library {
    oaks: Vec<Variant>,
    birches: Vec<Variant>,
    /// Tannen wachsen nur im Gebirge.
    pines: Vec<Variant>,
    palms: Vec<Variant>,
    magic_trees: Vec<Variant>,
    /// Abbaubare Vorkommen (Spitzhacke)
    stone_nodes: Vec<Variant>,
    ore_nodes: Vec<Variant>,
    /// Magische Kristallvorkommen (leuchten, noch nicht abbaubar)
    crystal_nodes: Vec<Variant>,
    /// Strandgut
    driftwood: Vec<Variant>,
    shells: Vec<Variant>,
    /// Am und im Wasser
    reeds: Vec<Variant>,
    /// Waldboden und Felsen
    ferns: Vec<Variant>,
    logs: Vec<Variant>,
    stumps: Vec<Variant>,
    ivy: Vec<Variant>,
    lantern: Vec<Variant>,
    lilies: Vec<Variant>,
    bushes: Vec<Variant>,
    grass: Vec<Variant>,
    teal_grass: Vec<Variant>,
    flowers: Vec<Variant>,
    magic_flowers: Vec<Variant>,
    red_mushroom: Vec<Variant>,
    glow_mushroom: Vec<Variant>,
    crystals: Vec<Variant>,
}

/// Maßstab, in dem die Insel Felsen aufstellt (Mittelwert der Zufallsgrößen in `build`).
/// Fels-Dateien werden dadurch geteilt, damit sie im Schnitt so groß sind wie in Blender.
const ROCK_SCALE: Vec3 = vec3(2.0, 1.4, 2.0);

impl Library {
    fn load(ctx: &mut Context) -> Self {
        // Liegen Dateien in game/assets/natur/<name>[_n].gltf, ersetzen sie das eingebaute Modell.
        let slot = |ctx: &mut Context, name: &str, count: u32, build: &dyn Fn(u32) -> MeshData| -> Vec<Variant> {
            let files = asset_files::load_variants(ctx, "natur", name, Vec3::ONE, 0.0);
            if !files.is_empty() {
                return files;
            }
            (0..count).map(|i| (ctx.assets.named_mesh(&format!("{name}{i}"), || build(i + 1)), None)).collect()
        };
        let colored = |ctx: &mut Context, name: &str, colors: &[Vec3], build: &dyn Fn(Vec3) -> MeshData| -> Vec<Variant> {
            slot(ctx, name, colors.len() as u32, &|i| build(colors[i as usize - 1]))
        };

        // Stein- und Erzvorkommen aus Blender; fehlen sie, eingebaute Felsen als Ersatz.
        let mut stone_nodes = asset_files::load_variants(ctx, "natur", "steinvorkommen", Vec3::ONE, 0.0);
        if stone_nodes.is_empty() {
            stone_nodes = (0..5)
                .map(|i| (ctx.assets.named_mesh(&format!("fels{i}"), || models::rock((i + 1) * 7).displace(|p| p * ROCK_SCALE * 0.6)), None))
                .collect();
        }
        let mut ore_nodes = asset_files::load_variants(ctx, "natur", "erzvorkommen", Vec3::ONE, 0.0);
        if ore_nodes.is_empty() {
            ore_nodes = stone_nodes.clone();
        }
        let mut magic_trees = asset_files::load_variants(ctx, "natur", "zauberbaum", Vec3::ONE, 0.0);
        if magic_trees.is_empty() {
            magic_trees = (0..2)
                .map(|i| {
                    let tree = models::magic_tree(i * 11 + 3);
                    let fruits = models::glow_fruits(i * 5 + 1, &tree);
                    let a = &mut ctx.assets;
                    (a.named_mesh(&format!("zauberbaum{i}"), || tree), Some(a.named_mesh(&format!("zauberfrucht{i}"), || fruits)))
                })
                .collect();
        }

        let library = Library {
            oaks: slot(ctx, "eiche", 3, &|s| models::oak(s * 17)),
            birches: slot(ctx, "birke", 3, &|s| models::oak(s * 23)),
            pines: slot(ctx, "tanne", 3, &|s| models::pine(s * 29, false)),
            palms: slot(ctx, "palme", 2, &|s| models::palm(s * 13)),
            magic_trees,
            stone_nodes,
            ore_nodes,
            crystal_nodes: asset_files::load_variants(ctx, "natur", "kristallvorkommen", Vec3::ONE, 0.0),
            driftwood: asset_files::load_variants(ctx, "natur", "treibholz", Vec3::ONE, 0.0),
            shells: asset_files::load_variants(ctx, "natur", "muscheln", Vec3::ONE, 0.0),
            reeds: asset_files::load_variants(ctx, "natur", "schilf", Vec3::ONE, 0.0),
            ferns: asset_files::load_variants(ctx, "natur", "farn", Vec3::ONE, 0.0),
            logs: asset_files::load_variants(ctx, "natur", "baumstamm", Vec3::ONE, 0.0),
            stumps: asset_files::load_variants(ctx, "natur", "baumstumpf", Vec3::ONE, 0.0),
            ivy: asset_files::load_variants(ctx, "natur", "efeu", Vec3::ONE, 0.0),
            lantern: asset_files::load_variants(ctx, "gebaeude", "laterne", Vec3::ONE, 0.0),
            lilies: asset_files::load_variants(ctx, "natur", "seerosen", Vec3::ONE, 0.0),
            bushes: slot(ctx, "busch", 3, &|s| models::bush(s * 3)),
            grass: slot(ctx, "gras", 3, &|s| models::grass(s * 5, vec3(0.16, 0.4, 0.06))),
            teal_grass: slot(ctx, "zaubergras", 1, &|_| models::grass(99, vec3(0.05, 0.35, 0.3))),
            flowers: colored(ctx, "blume", &[vec3(0.9, 0.75, 0.1), vec3(0.95, 0.95, 0.9), vec3(0.8, 0.12, 0.1), vec3(0.3, 0.35, 0.95)], &models::flower),
            magic_flowers: colored(ctx, "zauberblume", &[vec3(0.7, 0.2, 0.95), vec3(0.2, 0.8, 0.95)], &models::flower),
            red_mushroom: slot(ctx, "fliegenpilz", 1, &|_| models::mushroom(vec3(0.7, 0.06, 0.04), 1.0)),
            glow_mushroom: slot(ctx, "leuchtpilz", 1, &|_| models::mushroom(vec3(0.15, 0.85, 0.95), 1.3)),
            crystals: slot(ctx, "kristall", 2, &|s| models::crystals(s * 41)),
        };
        // In der Ferne einfachere Modelle, Kleinkram verschwindet ganz (spart viel Grafikleistung).
        // Neue Bäume/Felsen aus Blender bekommen das automatisch mit.
        let trees = [&library.oaks, &library.birches, &library.pines, &library.palms, &library.magic_trees];
        for variants in trees {
            add_lods(ctx, variants, &[Level(45.0, Some(0.35)), Level(110.0, Some(0.9)), Level(260.0, Some(2.0)), Level(650.0, None)]);
        }
        add_lods(ctx, &library.stone_nodes, &[Level(60.0, Some(0.3)), Level(350.0, None)]);
        add_lods(ctx, &library.ore_nodes, &[Level(60.0, Some(0.3)), Level(350.0, None)]);
        add_lods(ctx, &library.crystal_nodes, &[Level(120.0, Some(0.4)), Level(420.0, None)]);
        // Kleinkram verschwindet je nach Größe – auf der großen Insel wären sonst Zehntausende im Bild
        add_lods(ctx, &library.logs, &[Level(50.0, Some(0.25)), Level(170.0, None)]);
        add_lods(ctx, &library.stumps, &[Level(40.0, Some(0.2)), Level(130.0, None)]);
        for (variants, weit) in [(&library.driftwood, 110.0), (&library.shells, 60.0), (&library.reeds, 100.0), (&library.ferns, 80.0),
                                 (&library.ivy, 90.0), (&library.lilies, 100.0), (&library.crystals, 160.0), (&library.lantern, 220.0)] {
            add_lods(ctx, variants, &[Level(weit, None)]);
        }
        add_lods(ctx, &library.bushes, &[Level(40.0, Some(0.25)), Level(150.0, None)]);
        for variants in [&library.grass, &library.teal_grass, &library.flowers] {
            add_lods(ctx, variants, &[Level(85.0, None)]);
        }
        add_lods(ctx, &library.magic_flowers, &[Level(110.0, None)]);
        add_lods(ctx, &library.red_mushroom, &[Level(70.0, None)]);
        add_lods(ctx, &library.glow_mushroom, &[Level(110.0, None)]);
        library
    }
}

/// Detailstufe für die Insel: ab `distance` Metern vereinfacht (Zellgröße in Metern
/// des Modells) oder, bei `None`, gar nicht mehr gezeichnet.
struct Level(f32, Option<f32>);

/// Hinterlegt für alle Varianten eines Modells die Detailstufen. Leuchtende Zusatzteile
/// verschwinden mit der letzten Stufe.
fn add_lods(ctx: &mut Context, variants: &[Variant], levels: &[Level]) {
    for &(mesh, glow) in variants {
        // Die Insel wird öfter neu gebaut (Menü, Runde) – Stufen nur einmal anlegen.
        if ctx.assets.has_lods(mesh) {
            continue;
        }
        let lods = levels
            .iter()
            .map(|&Level(distance, cell)| Lod {
                distance,
                mesh: cell.map(|cell| {
                    let coarse = ctx.assets.mesh(mesh).simplified(cell);
                    ctx.assets.add_mesh(coarse)
                }),
            })
            .collect();
        ctx.assets.set_lods(mesh, lods);
        if let (Some(glow), Some(last)) = (glow, levels.last()) {
            ctx.assets.set_lods(glow, vec![Lod { distance: last.0.max(90.0), mesh: None }]);
        }
    }
}

/// Baut die Insel in die Szene: Landschaft, Wasser und Deko. Die abbaubaren Rohstoffe
/// werden nur beschrieben – die Welt erzeugt sie, damit sie verschwinden und
/// nachwachsen können.
pub fn build(ctx: &mut Context) -> Island {
    // Licht und Himmel kommen vom Tag-Nacht-Zyklus (`World::day`).
    ctx.env.shadow_range = 45.0;

    let terrain = Terrain::generate(Vec2::ZERO, TERRAIN_SIZE, TERRAIN_CELLS, height);
    let spawn = find_spawn(&terrain);
    let paths = Paths::build(&terrain, spawn);
    // Mit Fenster: fein aufgelöste Bodentextur; der Server braucht nur die Form.
    let terrain_mesh = if ctx.is_headless() {
        ctx.assets.named_mesh(&format!("insel{SEED}"), || terrain.mesh(|c, n| ground_color(c, n) * facet_jitter(c)))
    } else {
        let started = std::time::Instant::now();
        let texture = ctx.assets.named_texture(&format!("boden{WORLD_ID}"), || ground_texture(&terrain, &paths));
        log::info!("Bodentextur in {:.0} ms", started.elapsed().as_secs_f32() * 1000.0);
        ctx.assets.named_mesh(&format!("insel{WORLD_ID}"), || terrain.mesh_textured(texture, |c, _| Vec3::splat(facet_jitter(c))))
    };
    let ground = ctx.scene.spawn(Entity::new("Insel", terrain_mesh).with_material(Material::Ground));
    let (vertices, triangles) = terrain.collision_mesh();
    ctx.physics.add_static_mesh(Some(ground), vertices, triangles);

    let water = ctx.assets.named_mesh("wasser", || MeshData::grid(128));
    ctx.scene.spawn(
        Entity::new("Meer", water)
            .with_transform(Transform::default().with_scale(vec3(4200.0, 1.0, 4200.0)))
            .with_material(Material::Water),
    );
    // Bergsee im Westen: eigene Wasserfläche auf Höhe des Seespiegels
    let lake = ctx.assets.named_mesh("see", || MeshData::grid(48));
    ctx.scene.spawn(
        Entity::new("See", lake)
            .with_transform(Transform::from_position(vec3(LAKE_CENTER.x, LAKE_LEVEL, LAKE_CENTER.y)).with_scale(vec3(LAKE_RADIUS * 2.6, 1.0, LAKE_RADIUS * 2.6)))
            .with_material(Material::Water),
    );

    let lib = Library::load(ctx);
    let mut resources = Vec::new();
    let mut crystals = Vec::new();
    let mut places = crate::orte::Places::default();
    crate::orte::build_camp(ctx, &terrain, spawn, &mut places, &landmarks());
    crate::orte::build_castle(ctx, &mut places);
    crate::orte::build_festung(ctx, &mut places);
    let camp = crate::orte::camp_center(spawn);
    let spots = find_sights(&terrain, spawn, camp);
    let mut blocked = crate::orte::build_sights(ctx, &terrain, &spots, &mut places);
    blocked.extend(build_stream(ctx, &terrain, &paths, &mut places));
    let is_blocked = |p: Vec2| blocked.iter().any(|&(c, r)| p.distance(c) < r) || burg_frei(p) || festung_frei(p);


    let spacing = 3.2;
    let cells = ((ISLAND_RADIUS * 2.3) / spacing) as i32;
    let half = cells as f32 * spacing / 2.0;
    for iz in 0..cells {
        for ix in 0..cells {

            // Die Zellnummer ist die ID des Rohstoffs: stabil, auch wenn ein anderer Rechner
            // an einer einzelnen Stelle minimal anders rechnet.
            let id = (iz * cells + ix) as u32;
            let mut rng = Rng::new(((SEED as u64) << 32) | id as u64);
            let (x, z) = (-half + ix as f32 * spacing, -half + iz as f32 * spacing);
            let p = vec2(x + rng.range(-1.4, 1.4), z + rng.range(-1.4, 1.4));

            let h = terrain.height_at(p.x, p.y);
            if h < 0.9 || in_lake(p) {
                continue;
            }
            let normal = terrain.normal_at(p.x, p.y);
            let slope = 1.0 - normal.y;
            let base = vec3(p.x, h, p.y);
            // Auf Wegen und um den Startpunkt wächst nichts Großes.
            let clearing = base.distance(spawn) < SPAWN_CLEARING
                || p.distance(camp) < crate::orte::CAMP_RADIUS
                || paths.at(p) > 0.15
                || is_blocked(p)
                || stream_distance(p) < STREAM_BANK;
            let roll = rng.next_f32();
            let yaw = Quat::from_rotation_y(rng.range(0.0, std::f32::consts::TAU));
            let size = rng.range(0.8, 1.25);
            // Eichen: meist Standardgröße, etwa jede fünfte deutlich größer (und mit mehr Holz)
            let oak = |rng: &mut Rng| if rng.chance(0.22) { (size * rng.range(1.35, 1.75), 8) } else { (size, 5) };
            // Tannen (nur im Gebirge): von jung und schmal bis alt und mächtig
            let pine_size = |rng: &mut Rng| size * rng.range(0.8, 1.35);
            let pick = |list: &[Variant], rng: &mut Rng| list[(rng.next_u32() as usize) % list.len()];
            // Für Modelle ohne eigenen Zufallswurf: Auswahl über die Zellnummer.
            let by_id = |list: &[Variant]| list[id as usize % list.len()];

            let wet = moisture(p);
            let enchanted = magic(p) > 0.62 && h < 18.0;

            // Bäume und Felsen (abbaubar)
            let tree = |name: &'static str, (mesh, glow_part): Variant, size: f32, health: u8| ResourceSpec {
                kind: ResourceKind::Wood,
                name,
                mesh,
                transform: Transform::from_position(base - Vec3::Y * 0.15).with_rotation(yaw).with_scale(Vec3::splat(size)),
                color: Vec4::ONE,
                material: LEAVES,
                glow_part,
                collider: Shape::Capsule { radius: 0.4 * size, height: 4.0 * size },
                collider_offset: Vec3::Y * 2.0 * size,
                max_health: health,
            };
            // Vorkommen: im Gebirge und an steilen Hängen oft Erz, im Flachland meist Stein, am Strand nie Erz.
            let ore_chance = if h > 17.0 || slope > 0.55 { 0.45 } else if h < 2.4 { 0.0 } else { 0.18 };
            let node = |rng: &mut Rng| {
                let (list, name, kind, health) = if rng.chance(ore_chance) {
                    (&lib.ore_nodes, "Erzvorkommen", ResourceKind::Ore, 6)
                } else {
                    (&lib.stone_nodes, "Steinvorkommen", ResourceKind::Stone, 5)
                };
                let (mesh, glow_part) = pick(list, rng);
                let scale = rng.range(1.15, 1.5);
                ResourceSpec {
                    kind,
                    name,
                    mesh,
                    transform: Transform::from_position(base - Vec3::Y * 0.05).with_rotation(yaw).with_scale(Vec3::splat(scale)),
                    color: Vec4::ONE,
                    material: Material::Standard,
                    glow_part,
                    collider: Shape::Box { size: vec3(1.3, 0.9, 1.3) * scale },
                    collider_offset: Vec3::Y * 0.45 * scale,
                    max_health: health,
                }
            };

            let mut found: Option<ResourceSpec> = None;
            if !clearing {
                if slope > 0.55 {
                    if roll < 0.08 {
                        found = Some(node(&mut rng));
                    }
                } else if h < 2.4 {
                    if roll < 0.035 {
                        found = Some(tree("Palme", pick(&lib.palms, &mut rng), size, 4));
                    } else if roll < 0.05 {
                        found = Some(node(&mut rng));
                    } else if roll < 0.075 && !lib.driftwood.is_empty() {
                        decor(ctx, pick(&lib.driftwood, &mut rng), base, yaw, size, Vec4::ONE, Material::Standard);
                    } else if roll < 0.1 && !lib.shells.is_empty() {
                        decor(ctx, pick(&lib.shells, &mut rng), base, yaw, 1.0, Vec4::ONE, Material::Standard);
                    }
                } else if h > 28.0 {
                    if roll < 0.06 {
                        found = Some(tree("Tanne", pick(&lib.pines, &mut rng), pine_size(&mut rng), 5));
                    } else if roll < 0.10 {
                        found = Some(node(&mut rng));
                    } else if roll < 0.112 && !lib.crystal_nodes.is_empty() {
                        crystals.push(crystal_node(ctx, pick(&lib.crystal_nodes, &mut rng), base, yaw, rng.range(1.0, 1.4)));
                    }
                } else if h > 17.0 {
                    if roll < 0.17 {
                        found = Some(tree("Tanne", pick(&lib.pines, &mut rng), pine_size(&mut rng), 5));
                    } else if roll < 0.24 {
                        found = Some(node(&mut rng));
                    }
                } else if enchanted {
                    if roll < 0.13 {
                        found = Some(tree("Zauberbaum", pick(&lib.magic_trees, &mut rng), size, 6));
                    } else if roll < 0.16 {
                        decor(ctx, pick(&lib.crystals, &mut rng), base, yaw, size, vec4(0.55, 0.25, 1.0, 1.0), Material::Emissive { glow: 1.6 });
                    } else if roll < 0.28 {
                        decor(ctx, by_id(&lib.glow_mushroom), base, yaw, size, Vec4::ONE, Material::Emissive { glow: 0.9 });
                    } else if roll < 0.42 {
                        decor(ctx, pick(&lib.magic_flowers, &mut rng), base, yaw, size, Vec4::ONE, Material::Emissive { glow: 0.5 });
                    } else if roll < 0.445 && !lib.crystal_nodes.is_empty() {
                        crystals.push(crystal_node(ctx, pick(&lib.crystal_nodes, &mut rng), base, yaw, rng.range(0.9, 1.3)));
                    }
                } else if birch_grove(p) > 0.64 && h < 14.0 {
                    // Birkenhain: helle Stämme dicht beieinander, dazwischen Büsche und Blumen
                    if roll < 0.17 {
                        found = Some(tree("Birke", pick(&lib.birches, &mut rng), size, 4));
                    } else if roll < 0.21 {
                        decor(ctx, pick(&lib.bushes, &mut rng), base, yaw, size, Vec4::ONE, LEAVES);
                    } else if roll < 0.36 {
                        decor(ctx, pick(&lib.flowers, &mut rng), base, yaw, size, Vec4::ONE, FLOWERS);
                    }
                } else if wet > 0.52 {
                    if roll < 0.2 {
                        let (oak_size, health) = oak(&mut rng);
                        found = Some(tree("Eiche", pick(&lib.oaks, &mut rng), oak_size, health));
                    } else if roll < 0.25 {
                        found = Some(tree("Birke", pick(&lib.birches, &mut rng), size, 4));
                    } else if roll < 0.29 {
                        found = Some(tree("Birke", pick(&lib.birches, &mut rng), size, 4));
                    } else if roll < 0.40 {
                        decor(ctx, pick(&lib.bushes, &mut rng), base, yaw, size, Vec4::ONE, LEAVES);
                    } else if roll < 0.45 {
                        decor(ctx, by_id(&lib.red_mushroom), base, yaw, size, Vec4::ONE, Material::Standard);
                    } else if roll < 0.47 {
                        found = Some(node(&mut rng));
                    } else if roll < 0.56 && !lib.ferns.is_empty() {
                        decor(ctx, pick(&lib.ferns, &mut rng), base, yaw, size * 1.2, Vec4::ONE, GRASS);
                    } else if roll < 0.575 && !lib.stumps.is_empty() {
                        decor(ctx, pick(&lib.stumps, &mut rng), base, yaw, size, Vec4::ONE, Material::Standard);
                    } else if roll < 0.585 && !lib.logs.is_empty() && slope < 0.2 {
                        decor(ctx, pick(&lib.logs, &mut rng), base, yaw, size, Vec4::ONE, Material::Standard);
                    } else if roll < 0.59 {
                        // Hexenring: Pilze im Kreis
                        for k in 0..8 {
                            let w = k as f32 / 8.0 * std::f32::consts::TAU + rng.range(-0.2, 0.2);
                            let q = p + Vec2::from_angle(w) * rng.range(1.1, 1.4);
                            let spot = vec3(q.x, terrain.height_at(q.x, q.y), q.y);
                            decor(ctx, by_id(&lib.red_mushroom), spot, yaw, rng.range(0.7, 1.1), Vec4::ONE, Material::Standard);
                        }
                    }
                } else if roll < 0.02 {
                    let (oak_size, health) = oak(&mut rng);
                    found = Some(tree("Eiche", pick(&lib.oaks, &mut rng), oak_size, health));
                } else if roll < 0.035 {
                    found = Some(tree("Birke", pick(&lib.birches, &mut rng), size, 4));
                } else if roll < 0.07 {
                    decor(ctx, pick(&lib.bushes, &mut rng), base, yaw, size, Vec4::ONE, LEAVES);
                } else if roll < 0.09 {
                    found = Some(node(&mut rng));
                } else if roll < 0.28 {
                    decor(ctx, pick(&lib.flowers, &mut rng), base, yaw, size, Vec4::ONE, FLOWERS);
                }
            }

            // Efeu an steilen Felswänden (mittlere Höhen), zum Hang hinaus gedreht
            if found.is_none() && (0.5..0.8).contains(&slope) && (4.0..24.0).contains(&h) && !lib.ivy.is_empty() && hash01(ix, iz, SEED + 61) < 0.18 {
                let out = vec2(normal.x, normal.z).normalize_or(Vec2::Y);
                let turn = Quat::from_rotation_y(out.x.atan2(out.y));
                let spot = base - vec3(out.x, 0.0, out.y) * 0.35 - Vec3::Y * 0.3;
                decor(ctx, lib.ivy[0], spot, turn, rng.range(0.8, 1.3), Vec4::ONE, LEAVES);
            }
            // An den Pfaden: Wildblumen am Rand, ab und zu eine Laterne
            let path_here = paths.at(p);
            if (0.08..0.4).contains(&path_here) && p.distance(camp) > crate::orte::CAMP_RADIUS {
                if hash01(ix, iz, SEED + 62) < 0.55 {
                    for _ in 0..2 {
                        let q = p + vec2(rng.range(-1.2, 1.2), rng.range(-1.2, 1.2));
                        if paths.at(q) < 0.3 {
                            let spot = vec3(q.x, terrain.height_at(q.x, q.y), q.y);
                            decor(ctx, pick(&lib.flowers, &mut rng), spot, yaw, rng.range(0.8, 1.2), Vec4::ONE, FLOWERS);
                        }
                    }
                }
                if !lib.lantern.is_empty() && hash01(ix, iz, SEED + 63) < 0.06 && places.lanterns.iter().all(|l: &Vec3| vec2(l.x, l.z).distance(p) > 30.0) {
                    decor(ctx, lib.lantern[0], base, yaw, 1.0, Vec4::ONE, Material::Standard);
                    places.lanterns.push(base);
                    places.lights.push((base + vec3(0.0, 1.66, 0.0) + yaw * vec3(0.42, 0.0, 0.0), vec3(2.2, 1.35, 0.55), 8.0));
                }
            }

            // Um Vorkommen herum kein hohes Gras, sonst verschwinden sie darin.
            let node_here = found.as_ref().is_some_and(|s| s.kind.needs_pickaxe()) || crystals.last() == Some(&base);
            if let Some(spec) = found {
                resources.push((id, spec));
            }

            // Hohes Gras fast überall, wo es grün ist - mehrere Büschel je Zelle, damit Wiesen satt wirken
            if h > 2.2 && h < 26.0 && slope < 0.45 {
                let tufts = if wet > 0.52 { 2 } else { 3 };
                for _ in 0..tufts {
                    if !rng.chance(0.6) {
                        continue;
                    }
                    let q = vec2(x + rng.range(-1.6, 1.6), z + rng.range(-1.6, 1.6));
                    if paths.at(q) > 0.3
                        || (node_here && q.distance(p) < 1.8)
                        || q.distance(camp) < crate::orte::CAMP_RADIUS - 1.0
                        || is_blocked(q)
                        || stream_distance(q) < STREAM_BED + 1.5
                    {
                        continue;
                    }
                    let spot = vec3(q.x, terrain.height_at(q.x, q.y), q.y);
                    let turn = Quat::from_rotation_y(rng.range(0.0, std::f32::consts::TAU));
                    let mesh = if enchanted { by_id(&lib.teal_grass) } else { pick(&lib.grass, &mut rng) };
                    decor(ctx, mesh, spot, turn, rng.range(0.8, 1.35), Vec4::ONE, GRASS);
                }
            }
        }

    }

    log::info!("Insel gebaut: {} Rohstoffe, {} Objekte insgesamt", resources.len(), ctx.scene.len());
    plant_shores(ctx, &terrain, &lib, &places);
    places.surf = find_surf(&terrain);
    log::info!("{} Kristallvorkommen, {} Brandungsstellen", crystals.len(), places.surf.len());
    let map = (!ctx.is_headless()).then(|| {
        let started = std::time::Instant::now();
        let trees: Vec<Vec2> = resources
            .iter()
            .filter(|(_, r)| r.kind == ResourceKind::Wood)
            .map(|(_, r)| vec2(r.transform.position.x, r.transform.position.z))
            .collect();
        let image = map_image(&terrain, &paths, &trees);
        log::info!("Übersichtskarte in {:.0} ms", started.elapsed().as_secs_f32() * 1000.0);
        image
    });
    Island { terrain, resources, spawn, crystals, map, places }
}

/// Ein magisches Kristallvorkommen: leuchtet, ist fest (man läuft nicht hindurch), lässt sich
/// aber noch nicht abbauen. Liefert die Mitte am Boden.
fn crystal_node(ctx: &mut Context, (mesh, glow): Variant, base: Vec3, rotation: Quat, scale: f32) -> Vec3 {
    let transform = Transform::from_position(base - Vec3::Y * 0.05).with_rotation(rotation).with_scale(Vec3::splat(scale));
    let entity = ctx.scene.spawn(Entity::new("Kristallvorkommen", mesh).with_transform(transform));
    if let Some(glow) = glow {
        ctx.scene.spawn(Entity::new("Kristalle", glow).with_transform(transform).with_material(Material::Emissive { glow: 1.5 }));
    }
    let collider = Transform::from_position(base + Vec3::Y * 0.6 * scale).with_rotation(rotation);
    ctx.physics.add_body(entity, &collider, BodyDesc::fixed(Shape::Box { size: vec3(1.1, 1.2, 1.1) * scale }));
    base
}

fn decor(ctx: &mut Context, (mesh, glow): Variant, base: Vec3, rotation: Quat, size: f32, color: Vec4, material: Material) {
    let transform = Transform::from_position(base - Vec3::Y * 0.05).with_rotation(rotation).with_scale(Vec3::splat(size));
    ctx.scene.spawn(Entity::new("Deko", mesh).with_transform(transform).with_color(color).with_material(material));
    // Leuchtende Teile aus Blender-Modellen (Material mit Emission) glühen nachts.
    if let Some(glow) = glow {
        ctx.scene.spawn(Entity::new("Deko-Leuchten", glow).with_transform(transform).with_material(Material::Emissive { glow: 1.2 }));
    }
}

#[cfg(test)]
mod bach_test {
    use super::*;
    #[test]
    fn bach_faellt_stetig_und_hat_einen_wasserfall() {
        let beds = stream_beds();
        for pair in beds.windows(2) {
            assert!(pair[1] <= pair[0], "Bach fließt bergauf: {beds:?}");
        }
        let (top, foot) = waterfall();
        assert!(top.y - foot.y >= 4.5, "kein Wasserfall: {top} {foot}");
        assert!((beds[beds.len() - 1] - (LAKE_LEVEL - 0.6)).abs() < 0.01, "endet nicht im See");
        // Das Wasser liegt überall unter den Ufern (kein Damm)
        for i in 0..STREAM.len() - 1 {
            for k in 0..10 {
                let u = k as f32 / 10.0;
                let p = STREAM[i] + (STREAM[i + 1] - STREAM[i]) * u;
                let side = (STREAM[i + 1] - STREAM[i]).normalize().perp() * (STREAM_BANK + 1.0);
                let water = stream_bed(i, u) + STREAM_DEPTH;
                let bank = height(p + side).min(height(p - side));
                assert!(bank >= water - 0.05, "Abschnitt {i} bei {u:.1}: Wasser {water:.1} über dem Ufer {bank:.1}");
            }
        }
    }
}


#[cfg(test)]
mod burg_test {
    use super::*;

    #[test]
    fn burg_steht_auf_dem_plateau_und_die_mauer_ist_fest() {
        let mut ctx = Context::headless();
        let world = crate::world::World::new(&mut ctx);
        // Ein Physiktakt, damit neue Kollisionskörper für Strahlen sichtbar werden
        struct Leer;
        impl Game for Leer {
            fn init(&mut self, _: &mut Context) {}
            fn update(&mut self, _: &mut Context) {}
        }
        ctx.fixed_tick(&mut Leer);
        // Mitte des Vorhofs (Brunnenplatz) liegt genau auf Plateauhöhe
        let hof = burg_welt(vec2(0.0, 48.0));
        assert!((world.terrain.height_at(hof.x, hof.y) - BURG_HOEHE).abs() < 0.3);
        // Von außen waagerecht gegen die Ringmauer neben dem Tor (Außenseite bei 76,3 m)
        let vor = burg_welt(vec2(-20.0, 85.0));
        let innen = burg_welt(vec2(-20.0, 70.0));
        let from = vec3(vor.x, BURG_HOEHE + 3.0, vor.y);
        let direction = (vec3(innen.x, BURG_HOEHE + 3.0, innen.y) - from).normalize();
        let hit = ctx.physics.raycast(from, direction, 30.0, None);
        assert!(hit.is_some_and(|(_, d)| (d - 8.7).abs() < 1.0), "Mauer nicht getroffen: {hit:?}");
        // Durch das offene Tor kommt man hinein (das Fallgitter hängt oben)
        let tor = burg_welt(vec2(0.0, 85.0));
        let from = vec3(tor.x, BURG_HOEHE + 1.0, tor.y);
        let hit = ctx.physics.raycast(from, direction, 20.0, None);
        assert!(hit.is_none_or(|(_, d)| d > 14.0), "Tor versperrt: {hit:?}");
        // Durch das Schlossportal über den Teppich bis zum Thron (Blender y = -20 → Saal)
        let vor_portal = burg_welt(vec2(0.0, 20.0));
        let saal = burg_welt(vec2(0.0, 0.0));
        let from = vec3(vor_portal.x, BURG_HOEHE + 2.5, vor_portal.y);
        let direction = (vec3(saal.x, BURG_HOEHE + 2.5, saal.y) - from).normalize();
        let hit = ctx.physics.raycast(from, direction, 40.0, None);
        assert!(hit.is_none_or(|(_, d)| d > 22.0), "Schlossportal versperrt: {hit:?}");
    }
}
