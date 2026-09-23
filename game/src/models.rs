//! Prozedurale Low-Poly-Modelle für die Insel: Bäume, Felsen, Pflanzen, Kristalle.
//!
//! Alle Modelle stehen mit dem Fuß im Ursprung, +Y ist oben. Farben in linearem RGB.

use engine::noise::{perlin, Rng};
use engine::prelude::*;

pub const BARK: Vec3 = vec3(0.19, 0.11, 0.055);
pub const BARK_DARK: Vec3 = vec3(0.10, 0.06, 0.035);

fn jitter_color(color: Vec3, rng: &mut Rng, amount: f32) -> Vec3 {
    color * (1.0 + rng.range(-amount, amount))
}

/// Klumpige Kugel: Ikosphäre mit Rausch-Verformung, facettiert.
fn blob(subdivisions: u32, roughness: f32, seed: u32, color: impl Fn(Vec3) -> Vec3) -> MeshData {
    MeshData::icosphere(subdivisions, Vec3::ONE)
        .displace(|p| {
            let n = perlin(vec2(p.x * 3.1 + p.y * 1.7, p.z * 3.1 - p.y * 2.3), seed);
            p * (1.0 + n * roughness)
        })
        .recolor(|p, _| color(p))
        .flat_shaded()
}

/// Laubbaum mit runder, klumpiger Krone. Höhe ungefähr 5–7 m.
pub fn oak(seed: u32) -> MeshData {
    let mut rng = Rng::new(seed as u64);
    let mut mesh = MeshData::default();
    let trunk_height = rng.range(2.2, 3.0);
    mesh.append(&MeshData::cylinder(0.35, 0.24, trunk_height, 7, BARK), Mat4::IDENTITY);
    // Zwei Äste
    for side in [-1.0f32, 1.0] {
        let branch = MeshData::cylinder(0.13, 0.08, 1.3, 5, BARK);
        let tilt = Quat::from_rotation_z(side * 0.8) * Quat::from_rotation_y(rng.range(0.0, 3.0));
        mesh.append(&branch, Mat4::from_rotation_translation(tilt, Vec3::Y * trunk_height * 0.75));
    }
    let leaf = vec3(0.10, 0.30, 0.05);
    let crowns = [(Vec3::Y * (trunk_height + 1.2), 3.2), (vec3(1.0, trunk_height + 0.6, 0.3), 2.3), (vec3(-0.9, trunk_height + 0.8, -0.4), 2.4), (vec3(0.1, trunk_height + 2.2, -0.2), 2.2)];
    for (i, (offset, size)) in crowns.into_iter().enumerate() {
        let tone = jitter_color(leaf, &mut rng, 0.18);
        let crown = blob(1, 0.18, seed + i as u32, |p| tone * (0.8 + 0.35 * (p.y + 0.5)));
        mesh.append(&crown, Mat4::from_scale_rotation_translation(Vec3::splat(size), Quat::from_rotation_y(rng.range(0.0, 6.0)), offset));
    }
    mesh
}

/// Nadelbaum aus gestapelten Kegeln; mit `snow` weiße Spitzen.
pub fn pine(seed: u32, snow: bool) -> MeshData {
    let mut rng = Rng::new(seed as u64 ^ 0x51);
    let mut mesh = MeshData::default();
    let height = rng.range(6.0, 8.5);
    mesh.append(&MeshData::cylinder(0.3, 0.18, height * 0.35, 6, BARK_DARK), Mat4::IDENTITY);
    let needles = jitter_color(vec3(0.03, 0.16, 0.07), &mut rng, 0.15);
    let layers = 4;
    for i in 0..layers {
        let t = i as f32 / layers as f32;
        let radius = 2.2 * (1.0 - t * 0.7);
        let base = height * (0.22 + t * 0.62);
        let cone = MeshData::cylinder(radius, 0.0, height * 0.34, 7, needles).recolor(|p, c| {
            if snow && p.y > height * 0.34 * 0.45 { vec3(0.85, 0.88, 0.92) } else { c * (0.85 + p.y * 0.12) }
        });
        mesh.append(&cone, Mat4::from_rotation_translation(Quat::from_rotation_y(rng.range(0.0, 6.0)), Vec3::Y * base));
    }
    mesh
}

/// Palme mit gebogenem Stamm, für den Strand.
pub fn palm(seed: u32) -> MeshData {
    let mut rng = Rng::new(seed as u64 ^ 0x9A1);
    let mut mesh = MeshData::default();
    let segments = 6;
    let lean = rng.range(0.25, 0.45);
    let mut top = Vec3::ZERO;
    for i in 0..segments {
        let t = i as f32 / segments as f32;
        let radius = 0.28 - t * 0.12;
        let piece = MeshData::cylinder(radius, radius * 0.9, 1.0, 6, if i % 2 == 0 { vec3(0.32, 0.22, 0.11) } else { vec3(0.26, 0.17, 0.08) });
        let rotation = Quat::from_rotation_z(-lean * t * t * 1.6);
        mesh.append(&piece, Mat4::from_rotation_translation(rotation, top));
        top += rotation * Vec3::Y * 0.98;
    }
    let frond_color = vec3(0.12, 0.36, 0.06);
    for i in 0..7 {
        let angle = i as f32 / 7.0 * std::f32::consts::TAU + rng.range(-0.2, 0.2);
        // Blatt: flacher, langer Keil, der nach außen hängt
        let mut frond = MeshData::default();
        let (w, l) = (0.45, 2.8);
        let tip = vec3(l, -0.9, 0.0);
        let mid = vec3(l * 0.5, 0.15, 0.0);
        frond.push_triangle(Vec3::ZERO, vec3(l * 0.5, 0.1, w), mid, frond_color);
        frond.push_triangle(mid, vec3(l * 0.5, 0.1, w), tip, frond_color * 0.9);
        frond.push_triangle(Vec3::ZERO, mid, vec3(l * 0.5, 0.1, -w), frond_color);
        frond.push_triangle(mid, tip, vec3(l * 0.5, 0.1, -w), frond_color * 0.9);
        // Rückseite, damit das Blatt von unten nicht verschwindet
        let back = frond.clone();
        for tri in back.indices.chunks_exact(3) {
            let [a, b, c] = [tri[0], tri[1], tri[2]].map(|i| Vec3::from(back.vertices[i as usize].position));
            frond.push_triangle(a, c, b, frond_color * 0.7);
        }
        mesh.append(&frond, Mat4::from_rotation_translation(Quat::from_rotation_y(angle), top));
    }
    // Kokosnüsse
    for i in 0..3 {
        let a = i as f32 * 2.1;
        let nut = MeshData::icosphere(0, vec3(0.16, 0.10, 0.04)).flat_shaded();
        mesh.append(&nut, Mat4::from_scale_rotation_translation(Vec3::splat(0.3), Quat::IDENTITY, top + vec3(a.cos() * 0.25, -0.25, a.sin() * 0.25)));
    }
    mesh
}

/// Zauberbaum: verdrehter dunkler Stamm, leuchtend violett-türkise Krone.
pub fn magic_tree(seed: u32) -> MeshData {
    let mut rng = Rng::new(seed as u64 ^ 0x3A61C);
    let mut mesh = MeshData::default();
    let mut top = Vec3::ZERO;
    for i in 0..5 {
        let r = 0.38 - i as f32 * 0.05;
        let twist = Quat::from_rotation_y(i as f32 * 0.7) * Quat::from_rotation_z(rng.range(-0.25, 0.25));
        mesh.append(&MeshData::cylinder(r, r * 0.85, 0.9, 6, vec3(0.09, 0.05, 0.1)), Mat4::from_rotation_translation(twist, top));
        top += twist * Vec3::Y * 0.85;
    }
    let palette = [vec3(0.35, 0.08, 0.55), vec3(0.08, 0.35, 0.45), vec3(0.55, 0.12, 0.45)];
    for i in 0..5 {
        let angle = i as f32 * 1.3;
        let offset = top + vec3(angle.cos() * 1.1, rng.range(-0.3, 1.2), angle.sin() * 1.1) * if i == 0 { 0.0 } else { 1.0 };
        let tone = palette[(seed as usize + i) % palette.len()];
        let crown = blob(1, 0.22, seed * 7 + i as u32, |p| tone * (0.8 + 0.4 * (p.y + 0.5)));
        mesh.append(&crown, Mat4::from_scale_rotation_translation(Vec3::splat(rng.range(1.8, 2.6)), Quat::IDENTITY, offset));
    }
    mesh
}

/// Leuchtende Früchte für den Zauberbaum (eigenes Objekt mit Leucht-Material).
pub fn glow_fruits(seed: u32, tree: &MeshData) -> MeshData {
    let mut rng = Rng::new(seed as u64 ^ 0xF00D);
    let top = tree.vertices.iter().map(|v| v.position[1]).fold(0.0f32, f32::max);
    let mut mesh = MeshData::default();
    for _ in 0..9 {
        let p = vec3(rng.range(-2.0, 2.0), rng.range(top * 0.55, top * 0.9), rng.range(-2.0, 2.0));
        let orb = MeshData::icosphere(0, vec3(0.5, 0.95, 1.0)).flat_shaded();
        mesh.append(&orb, Mat4::from_scale_rotation_translation(Vec3::splat(rng.range(0.18, 0.3)), Quat::IDENTITY, p));
    }
    mesh
}

/// Felsbrocken, oben mit Moos.
pub fn rock(seed: u32) -> MeshData {
    let mut rng = Rng::new(seed as u64 ^ 0x60C4);
    let stone = jitter_color(vec3(0.24, 0.23, 0.22), &mut rng, 0.12);
    let moss = vec3(0.12, 0.22, 0.06);
    let mesh = MeshData::icosphere(1, Vec3::ONE)
        .displace(|p| {
            let n = perlin(vec2(p.x * 2.3 + p.z * 1.1, p.y * 2.3) + Vec2::splat(seed as f32 * 0.37), seed);
            let flat_bottom = if p.y < -0.25 { -0.25 + (p.y + 0.25) * 0.2 } else { p.y };
            vec3(p.x, flat_bottom, p.z) * (1.0 + n * 0.45)
        })
        .recolor(|p, _| if p.y > 0.28 { moss } else { stone });
    mesh.flat_shaded()
}

/// Kleiner Busch.
pub fn bush(seed: u32) -> MeshData {
    let mut rng = Rng::new(seed as u64 ^ 0xB05);
    let mut mesh = MeshData::default();
    let tone = jitter_color(vec3(0.08, 0.25, 0.05), &mut rng, 0.2);
    for i in 0..3 {
        let angle = i as f32 * 2.1 + rng.range(0.0, 1.0);
        let offset = vec3(angle.cos() * 0.4, 0.45, angle.sin() * 0.4);
        let part = blob(1, 0.2, seed + i, |p| tone * (0.75 + 0.4 * (p.y + 0.5)));
        mesh.append(&part, Mat4::from_scale_rotation_translation(Vec3::splat(rng.range(0.8, 1.2)), Quat::IDENTITY, offset));
    }
    mesh
}

/// Grasbüschel aus schmalen, spitzen Halmen.
pub fn grass(seed: u32, tint: Vec3) -> MeshData {
    let mut rng = Rng::new(seed as u64 ^ 0x6A55);
    let mut mesh = MeshData::default();
    for _ in 0..7 {
        let height = rng.range(0.35, 0.75);
        let blade = MeshData::cylinder(0.05, 0.0, height, 3, tint).recolor(|p, c| c * (0.55 + p.y / height * 0.7));
        let tilt = Quat::from_rotation_y(rng.range(0.0, 6.3)) * Quat::from_rotation_x(rng.range(-0.35, 0.35));
        mesh.append(&blade, Mat4::from_rotation_translation(tilt, vec3(rng.range(-0.25, 0.25), 0.0, rng.range(-0.25, 0.25))));
    }
    mesh
}

/// Blume mit Stiel und farbiger Blüte.
pub fn flower(petal: Vec3) -> MeshData {
    let mut mesh = MeshData::cylinder(0.03, 0.02, 0.4, 4, vec3(0.08, 0.3, 0.05));
    let bloom = MeshData::icosphere(0, petal).flat_shaded();
    mesh.append(&bloom, Mat4::from_scale_rotation_translation(vec3(0.22, 0.12, 0.22), Quat::IDENTITY, Vec3::Y * 0.42));
    let center = MeshData::icosphere(0, vec3(0.9, 0.6, 0.05)).flat_shaded();
    mesh.append(&center, Mat4::from_scale_rotation_translation(Vec3::splat(0.08), Quat::IDENTITY, Vec3::Y * 0.48));
    mesh
}

/// Pilz. Mit `cap` die Farbe des Huts.
pub fn mushroom(cap: Vec3, size: f32) -> MeshData {
    let mut mesh = MeshData::cylinder(0.08 * size, 0.06 * size, 0.3 * size, 6, vec3(0.8, 0.78, 0.7));
    let hat = MeshData::icosphere(1, cap).displace(|p| vec3(p.x, p.y.max(-0.05) * 0.8, p.z)).flat_shaded();
    mesh.append(&hat, Mat4::from_scale_rotation_translation(vec3(0.5, 0.35, 0.5) * size, Quat::IDENTITY, Vec3::Y * 0.3 * size));
    mesh
}

/// Kristall-Gruppe (spitze Prismen).
pub fn crystals(seed: u32) -> MeshData {
    let mut rng = Rng::new(seed as u64 ^ 0xC4157);
    let mut mesh = MeshData::default();
    for i in 0..4 {
        let height = rng.range(0.8, 1.9) * if i == 0 { 1.3 } else { 1.0 };
        let mut shard = MeshData::cylinder(0.18, 0.14, height, 5, Vec3::ONE);
        shard.append(&MeshData::cylinder(0.14, 0.0, 0.35, 5, Vec3::ONE), Mat4::from_translation(Vec3::Y * height));
        let tilt = Quat::from_rotation_y(rng.range(0.0, 6.3)) * Quat::from_rotation_x(if i == 0 { 0.0 } else { rng.range(0.3, 0.7) });
        mesh.append(&shard, Mat4::from_rotation_translation(tilt, vec3(rng.range(-0.2, 0.2), 0.0, rng.range(-0.2, 0.2))));
    }
    mesh
}
