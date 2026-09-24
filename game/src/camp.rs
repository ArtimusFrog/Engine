//! Das Lager am Startpunkt: Blockhütte, Lagerfeuer, Laterne, Holzstapel, Bänke, Fass.
//!
//! Alle Modelle entstehen im Code aus Grundformen – im Stil der KayKit-Figuren: kräftige
//! Formen, warme Farben, facettiert.

use engine::noise::{perlin, Rng};
use engine::prelude::*;

// ---------- Farben (linear) ----------

const LOG: Vec3 = vec3(0.34, 0.19, 0.085);
const LOG_END: Vec3 = vec3(0.62, 0.45, 0.26);
const PLANK: Vec3 = vec3(0.42, 0.25, 0.11);
const DARK_WOOD: Vec3 = vec3(0.16, 0.09, 0.045);
const ROOF: Vec3 = vec3(0.42, 0.12, 0.06);
const STONE: Vec3 = vec3(0.3, 0.29, 0.28);
const IRON: Vec3 = vec3(0.08, 0.08, 0.09);

// ---------- Maße der Hütte ----------

const WIDTH: f32 = 5.0;
const DEPTH: f32 = 4.0;
const LOG_RADIUS: f32 = 0.17;
const COURSE: f32 = 0.32;
const COURSES: usize = 8;
const FLOOR: f32 = 0.35;
const WALL_TOP: f32 = FLOOR + COURSES as f32 * COURSE + 0.18;
const RIDGE: f32 = WALL_TOP + 1.55;

fn vary(color: Vec3, rng: &mut Rng, amount: f32) -> Vec3 {
    color * (1.0 + rng.range(-amount, amount))
}

/// Quader mit Größe, Mittelpunkt und Drehung.
fn block(mesh: &mut MeshData, size: Vec3, center: Vec3, rotation: Quat, color: Vec3) {
    mesh.append(&MeshData::cube().recolor(|_, _| color), Mat4::from_scale_rotation_translation(size, rotation, center));
}

/// Liegender Stamm entlang `axis` (X oder Z) mit hellen Schnittflächen an den Enden.
fn log(mesh: &mut MeshData, center: Vec3, length: f32, radius: f32, along_x: bool, color: Vec3) {
    let rotation = if along_x { Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2) } else { Quat::from_rotation_x(std::f32::consts::FRAC_PI_2) };
    let axis = rotation * Vec3::Y;
    let start = center - axis * length / 2.0;
    mesh.append(&MeshData::cylinder(radius, radius, length, 8, color), Mat4::from_rotation_translation(rotation, start));
    // Schnittflächen: dünne, helle Scheiben knapp vor den Enden
    for end in [start - axis * 0.005, center + axis * (length / 2.0 - 0.015)] {
        mesh.append(&MeshData::cylinder(radius * 0.86, radius * 0.86, 0.02, 8, LOG_END), Mat4::from_rotation_translation(rotation, end));
    }
}

/// Blockhütte. Ursprung in der Mitte am Boden, Tür zeigt nach +Z.
pub fn cabin() -> MeshData {
    let mut mesh = MeshData::default();
    let mut rng = Rng::new(0xCAB1);
    let (hw, hd) = (WIDTH / 2.0, DEPTH / 2.0);

    // Steinfundament mit einzelnen Steinen am Rand
    block(&mut mesh, vec3(WIDTH + 0.5, 1.0, DEPTH + 0.5), Vec3::Y * (FLOOR - 0.5), Quat::IDENTITY, STONE * 0.9);
    let mut x = -hw - 0.2;
    while x < hw + 0.2 {
        for z in [-hd - 0.25, hd + 0.25] {
            let stone = MeshData::icosphere(0, vary(STONE, &mut rng, 0.15)).flat_shaded();
            let size = vec3(rng.range(0.4, 0.55), 0.3, 0.3);
            mesh.append(&stone, Mat4::from_scale_rotation_translation(size, Quat::from_rotation_y(rng.range(-0.3, 0.3)), vec3(x, FLOOR - 0.05, z)));
        }
        x += 0.45;
    }

    // Wände aus Rundholz: Vorder- und Rückwand, Seitenwände um eine halbe Lage versetzt,
    // damit sich die Stämme an den Ecken verschränken.
    let door = 0.62;
    let window = (FLOOR + 0.95, FLOOR + 1.85, 0.55);
    for course in 0..COURSES {
        let y = FLOOR + course as f32 * COURSE + LOG_RADIUS;
        // Vorderwand mit Türöffnung
        if y < FLOOR + 2.05 {
            for (from, to) in [(-hw - 0.25, -door), (door, hw + 0.25)] {
                log(&mut mesh, vec3((from + to) / 2.0, y, hd), to - from, LOG_RADIUS, true, vary(LOG, &mut rng, 0.12));
            }
        } else {
            log(&mut mesh, vec3(0.0, y, hd), WIDTH + 0.5, LOG_RADIUS, true, vary(LOG, &mut rng, 0.12));
        }
        log(&mut mesh, vec3(0.0, y, -hd), WIDTH + 0.5, LOG_RADIUS, true, vary(LOG, &mut rng, 0.12));

        // Seitenwände mit je einem Fenster
        let ys = y + COURSE / 2.0;
        for side in [-1.0, 1.0] {
            if ys > window.0 && ys < window.1 {
                for (from, to) in [(-hd - 0.25, -window.2), (window.2, hd + 0.25)] {
                    log(&mut mesh, vec3(side * hw, ys, (from + to) / 2.0), to - from, LOG_RADIUS, false, vary(LOG, &mut rng, 0.12));
                }
            } else {
                log(&mut mesh, vec3(side * hw, ys, 0.0), DEPTH + 0.5, LOG_RADIUS, false, vary(LOG, &mut rng, 0.12));
            }
        }
    }

    // Innenraum dunkel, damit man durch Tür und Fenster nicht ins Leere schaut
    block(&mut mesh, vec3(WIDTH - 0.3, WALL_TOP - FLOOR, DEPTH - 0.3), Vec3::Y * (FLOOR + (WALL_TOP - FLOOR) / 2.0), Quat::IDENTITY, DARK_WOOD * 0.4);

    // Giebel aus senkrechten Brettern (vorne und hinten)
    for (z, front) in [(hd, true), (-hd, false)] {
        let (a, b, c) = (vec3(-hw, WALL_TOP, z), vec3(hw, WALL_TOP, z), vec3(0.0, RIDGE, z));
        if front {
            mesh.push_triangle(a, b, c, PLANK);
        } else {
            mesh.push_triangle(b, a, c, PLANK * 0.9);
        }
        let mut bx = -hw + 0.35;
        while bx < hw - 0.3 {
            let top = WALL_TOP + (RIDGE - WALL_TOP) * (1.0 - bx.abs() / hw) - 0.1;
            let height = top - WALL_TOP;
            if height > 0.1 {
                let out = if front { 0.03 } else { -0.03 };
                block(&mut mesh, vec3(0.07, height, 0.05), vec3(bx, WALL_TOP + height / 2.0, z + out), Quat::IDENTITY, DARK_WOOD * 1.6);
            }
            bx += 0.5;
        }
    }

    // Dach: gestaffelte Schindelreihen auf beiden Seiten, Firstbalken oben
    let run = hd + 0.65;
    let rise = RIDGE - WALL_TOP + 0.2;
    let slope = (rise / run).atan();
    let length = (run * run + rise * rise).sqrt();
    let rows = 8;
    for side in [-1.0f32, 1.0] {
        let tilt = Quat::from_rotation_x(side * slope);
        for row in 0..rows {
            let t = (row as f32 + 0.5) / rows as f32;
            let along = length * (1.0 - t);
            let base = vec3(0.0, RIDGE + 0.12 - (along * slope.sin()), side * along * slope.cos());
            let pieces = 5;
            let offset = if row % 2 == 0 { 0.0 } else { 0.5 };
            for p in 0..pieces {
                let piece_width = (WIDTH + 1.1) / pieces as f32;
                let px = -(WIDTH + 1.1) / 2.0 + (p as f32 + 0.5 + offset) * piece_width;
                let clamped = px.clamp(-(WIDTH + 1.1) / 2.0 + piece_width / 2.0, (WIDTH + 1.1) / 2.0 - piece_width / 2.0);
                let color = vary(if (row + p) % 3 == 0 { ROOF * 0.85 } else { ROOF }, &mut rng, 0.1);
                block(&mut mesh, vec3(piece_width - 0.04, 0.09, length / rows as f32 + 0.12), vec3(clamped, base.y, base.z), tilt, color);
            }
        }
    }
    log(&mut mesh, vec3(0.0, RIDGE + 0.2, 0.0), WIDTH + 1.2, 0.14, true, vary(LOG, &mut rng, 0.1));

    // Tür: senkrechte Bretter, Querleisten, Beschläge, Knauf, Türsturz
    let door_z = hd + 0.08;
    for i in 0..3 {
        let x = -0.38 + i as f32 * 0.38;
        block(&mut mesh, vec3(0.36, 1.72, 0.07), vec3(x, FLOOR + 0.86, door_z), Quat::IDENTITY, vary(PLANK * 0.85, &mut rng, 0.1));
    }
    for y in [FLOOR + 0.35, FLOOR + 1.4] {
        block(&mut mesh, vec3(1.1, 0.12, 0.05), vec3(0.0, y, door_z + 0.05), Quat::IDENTITY, DARK_WOOD * 1.5);
        block(&mut mesh, vec3(0.28, 0.06, 0.03), vec3(-0.44, y, door_z + 0.09), Quat::IDENTITY, IRON);
    }
    block(&mut mesh, vec3(0.1, 1.2, 0.05), vec3(0.0, FLOOR + 0.87, door_z + 0.05), Quat::from_rotation_z(0.72), DARK_WOOD * 1.5);
    mesh.append(&MeshData::icosphere(0, vec3(0.75, 0.55, 0.2)).flat_shaded(), Mat4::from_scale_rotation_translation(Vec3::splat(0.09), Quat::IDENTITY, vec3(0.38, FLOOR + 0.9, door_z + 0.1)));
    block(&mut mesh, vec3(1.5, 0.2, 0.3), vec3(0.0, FLOOR + 1.85, hd + 0.02), Quat::IDENTITY, DARK_WOOD * 1.3);
    // Trittstein vor der Tür
    block(&mut mesh, vec3(1.4, 0.2, 0.7), vec3(0.0, FLOOR - 0.12, hd + 0.75), Quat::from_rotation_y(0.05), STONE * 1.05);

    // Fensterrahmen mit Sprossen und aufgeklappten Läden
    for side in [-1.0f32, 1.0] {
        let x = side * (hw + 0.06);
        let (y0, y1, half) = window;
        let cy = (y0 + y1) / 2.0;
        let frame = DARK_WOOD * 1.4;
        block(&mut mesh, vec3(0.1, 0.1, half * 2.0 + 0.2), vec3(x, y0, 0.0), Quat::IDENTITY, frame);
        block(&mut mesh, vec3(0.1, 0.12, half * 2.0 + 0.3), vec3(x, y1, 0.0), Quat::IDENTITY, frame);
        block(&mut mesh, vec3(0.1, y1 - y0, 0.08), vec3(x, cy, 0.0), Quat::IDENTITY, frame);
        block(&mut mesh, vec3(0.1, 0.07, half * 2.0), vec3(x, cy, 0.0), Quat::IDENTITY, frame);
        for s in [-1.0f32, 1.0] {
            let hinge = vec3(x + side * 0.02, cy, s * (half + 0.05));
            let shutter = Quat::from_rotation_y(side * s * 0.9);
            let offset = shutter * vec3(0.0, 0.0, s * 0.26);
            block(&mut mesh, vec3(0.05, y1 - y0 - 0.05, 0.5), hinge + offset + vec3(side * 0.12, 0.0, 0.0), shutter, vary(ROOF * 1.2, &mut rng, 0.08));
        }
        // Blumenkasten
        block(&mut mesh, vec3(0.25, 0.18, half * 2.0 + 0.1), vec3(x + side * 0.12, y0 - 0.12, 0.0), Quat::IDENTITY, PLANK);
        for i in 0..5 {
            let flower = [vec3(0.9, 0.2, 0.25), vec3(0.95, 0.8, 0.2), vec3(0.6, 0.3, 0.9)][i % 3];
            let blob = MeshData::icosphere(0, flower).flat_shaded();
            mesh.append(&blob, Mat4::from_scale_rotation_translation(Vec3::splat(0.12), Quat::IDENTITY, vec3(x + side * 0.12, y0 + 0.02, -half + 0.1 + i as f32 * 0.22)));
        }
    }

    // Schornstein aus Steinen an der Rückseite
    let chimney = vec3(1.4, 0.0, -hd - 0.45);
    let mut y = FLOOR - 0.2;
    let mut course = 0;
    while y < RIDGE + 0.8 {
        let offset = if course % 2 == 0 { 0.0 } else { 0.06 };
        for (dx, dz) in [(-0.2, -0.2), (0.2, -0.2), (-0.2, 0.2), (0.2, 0.2)] {
            block(
                &mut mesh,
                vec3(0.42, 0.26, 0.42),
                chimney + vec3(dx + offset, y + 0.13, dz),
                Quat::from_rotation_y(rng.range(-0.08, 0.08)),
                vary(STONE, &mut rng, 0.18),
            );
        }
        y += 0.27;
        course += 1;
    }
    block(&mut mesh, vec3(1.0, 0.12, 1.0), chimney + Vec3::Y * (y + 0.05), Quat::IDENTITY, STONE * 0.7);
    mesh.flat_shaded()
}

/// Leuchtende Fensterscheiben der Hütte (eigenes Objekt mit Leucht-Material).
pub fn cabin_windows() -> MeshData {
    let mut mesh = MeshData::default();
    let (y0, y1, half) = (FLOOR + 0.95, FLOOR + 1.85, 0.55);
    for side in [-1.0f32, 1.0] {
        block(&mut mesh, vec3(0.04, y1 - y0 - 0.05, half * 2.0), vec3(side * (WIDTH / 2.0 + 0.02), (y0 + y1) / 2.0, 0.0), Quat::IDENTITY, vec3(1.0, 0.72, 0.35));
    }
    mesh
}

/// Kollisionskörper der Hütte: (Größe, Mittelpunkt relativ zum Ursprung).
pub fn cabin_collider() -> (Vec3, Vec3) {
    (vec3(WIDTH + 0.4, WALL_TOP, DEPTH + 0.4), Vec3::Y * WALL_TOP / 2.0)
}

/// Lagerfeuer ohne Flammen: Steinring, Holzscheite, Glut.
pub fn campfire() -> MeshData {
    let mut mesh = MeshData::default();
    let mut rng = Rng::new(0xF1AE);
    for i in 0..11 {
        let a = i as f32 / 11.0 * std::f32::consts::TAU;
        let stone = MeshData::icosphere(0, vary(STONE, &mut rng, 0.2)).flat_shaded();
        let size = vec3(rng.range(0.32, 0.42), rng.range(0.22, 0.3), rng.range(0.3, 0.38));
        mesh.append(&stone, Mat4::from_scale_rotation_translation(size, Quat::from_rotation_y(-a), vec3(a.cos() * 0.72, 0.08, a.sin() * 0.72)));
    }
    let ash = MeshData::icosphere(1, vec3(0.05, 0.04, 0.04)).flat_shaded();
    mesh.append(&ash, Mat4::from_scale_rotation_translation(vec3(1.1, 0.12, 1.1), Quat::IDENTITY, Vec3::Y * 0.02));
    // Scheite als Zelt aufgestellt
    for i in 0..5 {
        let a = i as f32 / 5.0 * std::f32::consts::TAU + 0.3;
        let tilt = Quat::from_rotation_y(-a) * Quat::from_rotation_z(0.62);
        let foot = vec3(a.cos() * 0.42, 0.0, a.sin() * 0.42);
        let piece = MeshData::cylinder(0.075, 0.06, 0.85, 6, vary(LOG * 0.8, &mut rng, 0.15)).recolor(|p, c| if p.y > 0.55 { vec3(0.06, 0.04, 0.03) } else { c });
        mesh.append(&piece, Mat4::from_rotation_translation(tilt, foot));
    }
    mesh.flat_shaded()
}

/// Flammen (eigenes, flackerndes Objekt mit Leucht-Material).
pub fn flames() -> MeshData {
    let mut mesh = MeshData::default();
    for (i, (radius, height, color)) in
        [(0.3, 0.75, vec3(1.0, 0.28, 0.04)), (0.22, 1.0, vec3(1.0, 0.5, 0.08)), (0.13, 0.7, vec3(1.0, 0.85, 0.3))].into_iter().enumerate()
    {
        for j in 0..3 {
            let a = j as f32 * 2.1 + i as f32;
            let offset = vec3(a.cos(), 0.0, a.sin()) * if i == 2 { 0.0 } else { 0.12 };
            let cone = MeshData::cylinder(radius, 0.0, height * (0.8 + j as f32 * 0.15), 5, color);
            mesh.append(&cone, Mat4::from_rotation_translation(Quat::from_rotation_y(a), offset + Vec3::Y * 0.1));
        }
    }
    mesh
}

/// Laterne an einem Holzpfosten. Liefert das Gestell; die Scheiben kommen extra.
pub fn lantern_post() -> MeshData {
    let mut mesh = MeshData::cylinder(0.08, 0.07, 2.4, 6, DARK_WOOD * 1.4);
    block(&mut mesh, vec3(0.65, 0.1, 0.1), vec3(0.28, 2.3, 0.0), Quat::IDENTITY, DARK_WOOD * 1.4);
    block(&mut mesh, vec3(0.08, 0.35, 0.08), vec3(0.1, 2.12, 0.0), Quat::from_rotation_z(-0.8), DARK_WOOD * 1.4);
    block(&mut mesh, vec3(0.02, 0.2, 0.02), vec3(0.55, 2.15, 0.0), Quat::IDENTITY, IRON);
    // Gehäuse: Dach, Boden, vier Streben
    let center = vec3(0.55, 1.85, 0.0);
    mesh.append(&MeshData::cylinder(0.2, 0.0, 0.18, 4, IRON), Mat4::from_rotation_translation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_4), center + Vec3::Y * 0.17));
    block(&mut mesh, vec3(0.26, 0.04, 0.26), center - Vec3::Y * 0.17, Quat::IDENTITY, IRON);
    for (dx, dz) in [(-0.11, -0.11), (0.11, -0.11), (-0.11, 0.11), (0.11, 0.11)] {
        block(&mut mesh, vec3(0.03, 0.34, 0.03), center + vec3(dx, 0.0, dz), Quat::IDENTITY, IRON);
    }
    mesh.flat_shaded()
}

pub fn lantern_glass() -> MeshData {
    let mut mesh = MeshData::default();
    block(&mut mesh, vec3(0.2, 0.3, 0.2), vec3(0.55, 1.85, 0.0), Quat::IDENTITY, vec3(1.0, 0.75, 0.35));
    mesh
}

/// Bank aus einem halbierten Stamm.
pub fn bench() -> MeshData {
    let mut mesh = MeshData::default();
    log(&mut mesh, Vec3::Y * 0.24, 1.9, 0.24, true, LOG * 1.05);
    block(&mut mesh, vec3(1.8, 0.04, 0.3), Vec3::Y * 0.47, Quat::IDENTITY, LOG_END * 0.9);
    mesh.flat_shaded()
}

/// Sauber aufgeschichteter Holzstapel.
pub fn woodpile() -> MeshData {
    let mut mesh = MeshData::default();
    let mut rng = Rng::new(0x5701);
    for (row, count) in [4, 3, 2].into_iter().enumerate() {
        for i in 0..count {
            let x = (i as f32 - (count - 1) as f32 / 2.0) * 0.3;
            log(&mut mesh, vec3(x, 0.14 + row as f32 * 0.25, 0.0), 1.2, 0.14, false, vary(LOG, &mut rng, 0.15));
        }
    }
    block(&mut mesh, vec3(0.06, 0.9, 0.06), vec3(-0.68, 0.45, 0.45), Quat::IDENTITY, DARK_WOOD);
    block(&mut mesh, vec3(0.06, 0.9, 0.06), vec3(0.68, 0.45, 0.45), Quat::IDENTITY, DARK_WOOD);
    mesh.flat_shaded()
}

/// Bauchiges Fass mit Eisenringen.
pub fn barrel() -> MeshData {
    let mut mesh = MeshData::default();
    let profile = [0.3, 0.35, 0.37, 0.37, 0.35, 0.3];
    let height = 0.95 / (profile.len() - 1) as f32;
    for i in 0..profile.len() - 1 {
        let piece = MeshData::cylinder(profile[i], profile[i + 1], height, 10, PLANK * 0.95);
        mesh.append(&piece, Mat4::from_translation(Vec3::Y * i as f32 * height));
    }
    for y in [0.18, 0.75] {
        mesh.append(&MeshData::cylinder(0.375, 0.375, 0.06, 10, IRON), Mat4::from_translation(Vec3::Y * y));
    }
    mesh.append(&MeshData::cylinder(0.28, 0.28, 0.03, 10, LOG_END * 0.8), Mat4::from_translation(Vec3::Y * 0.94));
    mesh.flat_shaded()
}

/// Wo die Teile des Lagers stehen. Hängt nur vom Startpunkt ab und ist deshalb auf
/// allen Rechnern gleich.
pub struct Layout {
    pub fire: Vec3,
    pub cabin: Vec3,
    pub cabin_yaw: f32,
    pub lantern: Vec3,
    pub woodpile: Vec3,
    pub barrel: Vec3,
    pub benches: [(Vec3, f32); 2],
}

impl Layout {
    pub fn new(spawn: Vec3, terrain: &Terrain) -> Layout {
        let ground = |p: Vec3| vec3(p.x, terrain.height_at(p.x, p.z), p.z);
        let fire = ground(spawn + vec3(0.0, 0.0, -6.0));
        let cabin_flat = spawn + vec3(-5.5, 0.0, -13.0);
        // Hütte tiefster Ecke folgend setzen, damit das Fundament nirgends schwebt.
        let corners = [(-2.8, -2.3), (2.8, -2.3), (-2.8, 2.3), (2.8, 2.3)];
        let cabin_yaw = (fire.x - cabin_flat.x).atan2(fire.z - cabin_flat.z);
        let rotation = Quat::from_rotation_y(cabin_yaw);
        let low = corners
            .iter()
            .map(|&(x, z)| {
                let p = cabin_flat + rotation * vec3(x, 0.0, z);
                terrain.height_at(p.x, p.z)
            })
            .fold(f32::MAX, f32::min);
        let cabin = vec3(cabin_flat.x, low, cabin_flat.z);
        let forward = rotation * Vec3::Z;
        let right = rotation * Vec3::X;
        let to_fire = (fire - spawn).normalize_or(Vec3::NEG_Z);
        let side = vec3(-to_fire.z, 0.0, to_fire.x);
        Layout {
            fire,
            cabin,
            cabin_yaw,
            lantern: ground(cabin + forward * 3.2 - right * 1.6),
            woodpile: ground(cabin + right * 3.4 + forward * 0.5),
            barrel: ground(cabin + forward * 2.6 + right * 1.9),
            benches: [(ground(fire + side * 2.3), side.x.atan2(side.z)), (ground(fire - side * 2.3), side.x.atan2(side.z))],
        }
    }

    /// Soll an dieser Stelle nichts wachsen (Hütte, Feuerstelle, Wege)?
    pub fn blocks(&self, p: Vec2) -> bool {
        let near = |c: Vec3, r: f32| vec2(c.x - p.x, c.z - p.y).length() < r;
        near(self.cabin, 4.6) || near(self.fire, 3.2) || near(self.woodpile, 1.4)
    }
}

/// Die lebendigen Teile des Lagers (Flammen, Licht, Rauch), die jedes Bild aktualisiert werden.
pub struct Camp {
    layout: Layout,
    flames: EntityId,
    windows: EntityId,
    lantern_glass: EntityId,
    rng: Rng,
    ember_timer: f32,
    smoke_timer: f32,
}

impl Camp {
    /// Setzt das Lager in die Welt (Grafik und Kollision).
    pub fn build(ctx: &mut Context, layout: Layout) -> Camp {
        let place = |ctx: &mut Context, name: &str, mesh: MeshId, position: Vec3, yaw: f32, material: Material| {
            ctx.scene.spawn(
                Entity::new(name, mesh)
                    .with_transform(Transform::from_position(position).with_rotation(Quat::from_rotation_y(yaw)))
                    .with_material(material),
            )
        };
        let cabin_mesh = ctx.assets.named_mesh("huette", cabin);
        let windows_mesh = ctx.assets.named_mesh("huette_fenster", cabin_windows);
        let fire_mesh = ctx.assets.named_mesh("lagerfeuer", campfire);
        let flames_mesh = ctx.assets.named_mesh("flammen", flames);
        let lantern_mesh = ctx.assets.named_mesh("laterne", lantern_post);
        let glass_mesh = ctx.assets.named_mesh("laterne_glas", lantern_glass);
        let bench_mesh = ctx.assets.named_mesh("bank", bench);
        let pile_mesh = ctx.assets.named_mesh("holzstapel", woodpile);
        let barrel_mesh = ctx.assets.named_mesh("fass", barrel);

        let cabin = place(ctx, "Hütte", cabin_mesh, layout.cabin, layout.cabin_yaw, Material::Standard);
        let windows = place(ctx, "Fenster", windows_mesh, layout.cabin, layout.cabin_yaw, Material::Emissive { glow: 0.5 });
        let (size, offset) = cabin_collider();
        let rotation = Quat::from_rotation_y(layout.cabin_yaw);
        let collider = Transform::from_position(layout.cabin + rotation * offset).with_rotation(rotation);
        ctx.physics.add_body(cabin, &collider, BodyDesc::fixed(Shape::Box { size }));

        place(ctx, "Lagerfeuer", fire_mesh, layout.fire, 0.0, Material::Standard);
        let flames = place(ctx, "Flammen", flames_mesh, layout.fire, 0.0, Material::Emissive { glow: 2.5 });
        let lantern_yaw = layout.cabin_yaw + std::f32::consts::FRAC_PI_2;
        let post = place(ctx, "Laterne", lantern_mesh, layout.lantern, lantern_yaw, Material::Standard);
        ctx.physics.add_body(post, &Transform::from_position(layout.lantern + Vec3::Y * 1.2), BodyDesc::fixed(Shape::Box { size: vec3(0.2, 2.4, 0.2) }));
        let lantern_glass = place(ctx, "Laternenglas", glass_mesh, layout.lantern, lantern_yaw, Material::Emissive { glow: 0.5 });
        for (position, yaw) in layout.benches {
            let bench = place(ctx, "Bank", bench_mesh, position, yaw, Material::Standard);
            let rotation = Quat::from_rotation_y(yaw);
            ctx.physics.add_body(bench, &Transform::from_position(position + Vec3::Y * 0.25).with_rotation(rotation), BodyDesc::fixed(Shape::Box { size: vec3(1.9, 0.5, 0.45) }));
        }
        let pile = place(ctx, "Holzstapel", pile_mesh, layout.woodpile, layout.cabin_yaw, Material::Standard);
        ctx.physics.add_body(
            pile,
            &Transform::from_position(layout.woodpile + Vec3::Y * 0.4).with_rotation(rotation),
            BodyDesc::fixed(Shape::Box { size: vec3(1.3, 0.8, 1.2) }),
        );
        let barrel = place(ctx, "Fass", barrel_mesh, layout.barrel, 0.4, Material::Standard);
        ctx.physics.add_body(barrel, &Transform::from_position(layout.barrel + Vec3::Y * 0.48), BodyDesc::fixed(Shape::Capsule { radius: 0.37, height: 0.96 }));

        Camp { layout, flames, windows, lantern_glass, rng: Rng::new(0xF1E), ember_timer: 0.0, smoke_timer: 0.0 }
    }

    /// Flackern, Licht, Funken und Rauch. `night` = 0 am Tag, 1 in der Nacht.
    pub fn update(&mut self, ctx: &mut Context, night: f32) {
        let t = ctx.time.elapsed;
        let flicker = 0.85 + 0.15 * perlin(vec2(t * 6.0, 0.5), 3) + 0.08 * (t * 23.0).sin();
        if let Some(flames) = ctx.scene.try_get_mut(self.flames) {
            let sway = perlin(vec2(t * 3.0, 1.7), 5) * 0.12;
            flames.transform.scale = vec3(1.0 + sway * 0.5, flicker * (1.0 + perlin(vec2(t * 8.0, 4.0), 9) * 0.2), 1.0 - sway * 0.5);
            flames.transform.rotation = Quat::from_rotation_y(t * 0.8) * Quat::from_rotation_z(sway * 0.4);
        }
        let glow = 0.4 + night * 2.6;
        for id in [self.windows, self.lantern_glass] {
            if let Some(entity) = ctx.scene.try_get_mut(id) {
                entity.material = Material::Emissive { glow };
            }
        }

        let rotation = Quat::from_rotation_y(self.layout.cabin_yaw);
        let fire = self.layout.fire + Vec3::Y * 0.8;
        ctx.lights.push(PointLight { position: fire, color: vec3(3.2, 1.35, 0.4) * flicker * (0.5 + 0.5 * night), radius: 10.0 });
        if night > 0.05 {
            ctx.lights.push(PointLight {
                position: self.layout.lantern + Quat::from_rotation_y(self.layout.cabin_yaw + std::f32::consts::FRAC_PI_2) * vec3(0.55, 1.85, 0.0),
                color: vec3(2.0, 1.3, 0.55) * night,
                radius: 6.5,
            });
            for side in [-1.0f32, 1.0] {
                ctx.lights.push(PointLight {
                    position: self.layout.cabin + rotation * vec3(side * 3.2, 1.6, 0.0),
                    color: vec3(1.4, 0.8, 0.3) * night,
                    radius: 4.0,
                });
            }
        }

        // Funken und Rauch
        let dt = ctx.time.delta;
        self.ember_timer -= dt;
        while self.ember_timer < 0.0 {
            self.ember_timer += 0.07;
            ctx.particles.burst(Burst {
                position: self.layout.fire + vec3(self.rng.range(-0.25, 0.25), 0.4, self.rng.range(-0.25, 0.25)),
                count: 1,
                color: vec3(1.0, 0.45, 0.1),
                color_variation: 0.2,
                speed: 1.4,
                direction: Vec3::Y,
                size: 0.045,
                life: 1.3,
                gravity: -0.6,
                glow: 6.0,
                grow: 0.0,
                round: false,
            });
        }
        self.smoke_timer -= dt;
        while self.smoke_timer < 0.0 {
            self.smoke_timer += 0.12;
            let chimney = self.layout.cabin + rotation * vec3(1.45, RIDGE + 1.1, -DEPTH / 2.0 - 0.45);
            for (position, size) in [(chimney, 0.2), (self.layout.fire + Vec3::Y * 1.3, 0.13)] {
                ctx.particles.burst(Burst {
                    position,
                    count: 1,
                    color: vec3(0.62, 0.6, 0.58) * (0.35 + 0.65 * (1.0 - night)),
                    color_variation: 0.1,
                    speed: 0.5,
                    direction: vec3(0.25, 1.0, 0.08),
                    size,
                    life: 4.5,
                    gravity: -0.35,
                    glow: 0.0,
                    grow: 2.2,
                    round: true,
                });
            }
        }
    }
}
