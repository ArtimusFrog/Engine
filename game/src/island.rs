//! Die Insel: Landschaft, Biome, Wasser und Bewuchs.
//!
//! Alles entsteht aus einem Startwert. Server und Clients bauen die Insel unabhängig
//! voneinander und erhalten exakt dieselbe Welt – übertragen werden nur Änderungen.

use engine::mesh::Vertex;
use engine::noise::{fbm, hash01, ridged, Rng};
use engine::prelude::*;

use crate::asset_files;
use crate::models;

pub const SEED: u32 = 20_260_924;
/// Kennung der Insel für Spielstände: bei jeder Änderung an Gestalt oder Verteilung der
/// Rohstoffe hochzählen, sonst passen die Rohstoff-IDs gespeicherter Spielstände nicht mehr.
pub const WORLD_ID: u32 = SEED + 10;
/// Radius des Festlands in Metern (die Küste franst um diesen Wert aus).
pub const ISLAND_RADIUS: f32 = 760.0;
const TERRAIN_SIZE: f32 = 2040.0;
/// 3 m je Zelle
const TERRAIN_CELLS: usize = 680;
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
    /// Die vier Heerstraßen von den Rampen der Festung (Süd, Ost, Nord, West)
    pub strassen: Vec<Vec<Vec2>>,
    /// Siedlungsplätze am Ende jeder Heerstraße: Mitte und Höhe des geebneten Bodens
    pub siedlungen: Vec<(Vec2, f32)>,
}

/// Siedlungsplatz: so weit hinter dem Straßenende liegt die Mitte, bis hierhin ist der Boden eben,
/// bis hierhin läuft er weich ins Gelände aus, und in diesem Umkreis wächst nichts.
pub const SIEDLUNG_ABSTAND: f32 = 28.0;
pub const SIEDLUNG_EBEN: f32 = 50.0;
pub const SIEDLUNG_RAND: f32 = 90.0;
pub const SIEDLUNG_FREI: f32 = 52.0;

/// Die Siedlungsplätze am Ende der Heerstraßen: Mitte etwas hinter dem Ende (in Straßenrichtung),
/// Höhe = mittlere Geländehöhe dort (mindestens knapp über dem Strand).
fn siedlungsplaetze(terrain: &Terrain, strassen: &[Vec<Vec2>]) -> Vec<(Vec2, f32)> {
    strassen
        .iter()
        .map(|strasse| {
            let n = strasse.len();
            let ende = strasse[n - 1];
            let richtung = (ende - strasse[n.saturating_sub(4)]).normalize_or(ende.normalize_or(Vec2::Y));
            let mitte = ende + richtung * SIEDLUNG_ABSTAND;
            let (mut summe, mut anzahl) = (0.0, 0.0);
            for iz in -5..=5 {
                for ix in -5..=5 {
                    let p = mitte + vec2(ix as f32, iz as f32) * 5.0;
                    if p.distance(mitte) <= 25.0 {
                        summe += terrain.height_at(p.x, p.y);
                        anzahl += 1.0;
                    }
                }
            }
            (mitte, (summe / anzahl).max(2.8))
        })
        .collect()
}

/// Gelände mit den Siedlungsplätzen: innen eben, nach außen weich ins natürliche Gelände.
fn siedlungsgrund(p: Vec2, h: f32, plaetze: &[(Vec2, f32)]) -> f32 {
    let mut h = h;
    for &(mitte, eben) in plaetze {
        let d = p.distance(mitte);
        if d < SIEDLUNG_RAND {
            h = eben + (h - eben) * smoothstep(SIEDLUNG_EBEN, SIEDLUNG_RAND, d);
        }
    }
    h
}

/// Die Übersichtskarte zeigt ±`MAP_EXTENT` Meter um die Inselmitte (Norden = -z oben).
pub const MAP_EXTENT: f32 = ISLAND_RADIUS * 1.08;
const MAP_SIZE: usize = 2048;

/// Orte, die auf der Karte beschriftet werden.
pub fn landmarks() -> [(&'static str, Vec2); 3] {
    [
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
                    let linear = if h < 0.0 {
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

/// Wo die vier Rampen der Festung am Boden ankommen (Süd, Ost, Nord, West) – hier beginnen die
/// Wege über die Insel und die Marschrouten der Festungstruppen.
pub const FESTUNG_RAMPEN: [Vec2; 4] = [vec2(0.0, 76.0), vec2(76.0, 0.0), vec2(0.0, -76.0), vec2(-76.0, 0.0)];

/// Grundriss der Festung (Mitte = Inselmitte, vier Tore nach Süden, Osten, Norden und Westen):
/// der Felssockel (Radius 44 m) und die vier Rampen bis 72 m hinaus. Abstand zum Rand,
/// negativ = drinnen.
pub fn festung_rand(p: Vec2) -> f32 {
    let kreis = p.length() - 44.0;
    // Rampe entlang der Achse: `quer` = Abstand zur Mittellinie, `laengs` = Weg nach außen
    let rampe = |quer: f32, laengs: f32| {
        let q = vec2(quer.abs() - 5.5, (laengs - 36.0).abs() - 36.0);
        q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0)
    };
    let rampen = rampe(p.x, p.y).min(rampe(p.y, p.x)).min(rampe(p.x, -p.y)).min(rampe(p.y, -p.x));
    kreis.min(rampen)
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

/// Höhe der Landschaft an (x, z) – mit dem Burgberg und dem Festungsgrund.
pub fn height(p: Vec2) -> f32 {
    burgberg(p, festungsgrund(p, height_raw(p)))
}

/// Natürliche Höhe der Landschaft.
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
    // Im Inland nie unter den Meeresspiegel: kein Wasser in Senken (nur das Meer rund um die Insel)
    let inland = (hills + swells.max(-2.0) + peaks + plateau - valley).max(1.5);

    // Flacher Strand: nahe der Küstenlinie wird die Höhe zusammengedrückt.
    let shaped = -9.0 + (inland + 9.0) * land.sqrt() * land.sqrt().sqrt();
    if shaped > 0.0 && shaped < 3.0 { shaped * (0.55 + shaped * 0.15) } else { shaped }
}

pub fn moisture(p: Vec2) -> f32 {
    fbm(p * 0.008 + vec2(40.0, -13.0), 3, SEED + 5) * 0.5 + 0.5
}

/// Wie düster das Land hier ist: 1 rund um die Schattenfestung (bis 170 m), bis 240 m auslaufend.
/// Boden wird aschig, Bäume und Gras dunkel, das Licht fahl (siehe `World::update_visuals`).
pub fn duester(p: Vec2) -> f32 {
    let rand = fbm(p * 0.02 + vec2(5.0, -9.0), 2, SEED + 90) * 18.0;
    1.0 - smoothstep(170.0, 240.0, p.length() + rand)
}

/// Farbe, mit der Bäume, Gras und Steine im düsteren Land eingefärbt werden.
pub fn duester_farbe(p: Vec2) -> Vec4 {
    Vec4::ONE.lerp(vec4(0.15, 0.12, 0.19, 1.0), duester(p))
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
    /// Die vier Straßen von den Rampen der Festung (gleich lang, siehe `festung_strasse`)
    pub strassen: Vec<Vec<Vec2>>,
}

impl Paths {
    fn origin() -> Vec2 {
        Vec2::splat(-TERRAIN_SIZE / 2.0)
    }

    /// Wege vom Startpunkt zur Burg, zur Festung, zum Strand und ins Gebirge.
    fn build(terrain: &Terrain, spawn: Vec3) -> Paths {
        let res = (TERRAIN_SIZE / PATH_CELL) as usize;
        let mut paths = Paths { mask: vec![0.0; res * res], res, strassen: Vec::new() };
        let start = vec2(spawn.x, spawn.z);
        let r = ISLAND_RADIUS;
        let mut targets = Vec::new();
        // Fuß der Auffahrt zur Burg auf dem Tafelberg
        targets.push(burg_weg_fuss() + (start - burg_weg_fuss()).normalize_or(Vec2::X) * 3.0);
        // Fuß der Südrampe der Schattenfestung in der Inselmitte
        targets.push(FESTUNG_RAMPEN[0]);
        // Strand: vom Startpunkt nach außen, bis der Sand beginnt
        let outward = start.normalize_or(Vec2::Y);
        if let Some(beach) = find_along(terrain, start, outward, |h| h < 2.6) {
            targets.push(beach);
        }
        // Gebirge: Richtung Norden, bis das Gelände steil ansteigt
        if let Some(foot) = find_along(terrain, start, (vec2(0.0, -0.45 * r) - start).normalize(), |h| h > 17.0) {
            targets.push(foot);
        }
        let trails: Vec<Vec<Vec2>> = targets.iter().enumerate().map(|(i, &to)| trail(terrain, start, to, i as u32)).collect();
        // Die vier Heerstraßen der Festung: breit und festgetreten, alle gleich lang
        for index in 0..4 {
            let strasse = festung_strasse(terrain, index, &paths.strassen);
            for pair in strasse.windows(2) {
                paths.stamp(pair[0], pair[1], 2.2);
            }
            paths.strassen.push(strasse);
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
/// Länge jeder Heerstraße ab dem Fuß ihrer Rampe (alle gleich lang – fair für alle Richtungen).
pub const STRASSEN_LAENGE: f32 = 380.0;

/// Eine Heerstraße von der Rampe `index` (Süd, Ost, Nord, West) nach außen: in sanften Schwüngen,
/// um Wasser, steile Hänge, die Burg und die anderen Straßen herum, genau `STRASSEN_LAENGE` lang.
/// Ohne Winkelfunktionen gerechnet, damit sie auf allen Rechnern bitgenau gleich ausfällt.
fn festung_strasse(terrain: &Terrain, index: usize, frueher: &[Vec<Vec2>]) -> Vec<Vec2> {
    // Drehungen um ±10° und ±20° (cos, sin) – feste Zahlen statt sin/cos
    const DREHUNGEN: [(f32, f32); 5] = [(0.939_692_6, -0.342_020_1), (0.984_807_8, -0.173_648_2), (1.0, 0.0), (0.984_807_8, 0.173_648_2), (0.939_692_6, 0.342_020_1)];
    let start = FESTUNG_RAMPEN[index];
    let aussen = start.normalize_or(Vec2::Y);
    let hoehe = |p: Vec2| terrain.height_at(p.x, p.y);
    let mut punkte = vec![start];
    let (mut p, mut richtung, mut laenge) = (start, aussen, 0.0);
    while laenge < STRASSEN_LAENGE - 1e-3 {
        let schritt = (STRASSEN_LAENGE - laenge).min(4.0);
        // Gewünschte Richtung: nach außen, mit weiten Schwüngen aus Rauschen
        let schwung = fbm(p * 0.004, 2, SEED + 70 + index as u32) * 1.6;
        let wunsch = (aussen + aussen.perp() * schwung).normalize_or(aussen);
        let mut beste = (f32::MAX, richtung);
        for (c, s) in DREHUNGEN {
            let d = vec2(richtung.x * c - richtung.y * s, richtung.x * s + richtung.y * c);
            let q = p + d * 10.0;
            let mut kosten = (hoehe(q) - hoehe(p)).abs() * 1.5 + (1.0 - d.dot(wunsch)) * 4.0;
            if hoehe(q) < 1.4 {
                kosten += 60.0; // nicht ins Wasser
            }
            if laenge > 10.0 && festung_rand(q) < 3.0 {
                kosten += 40.0; // nicht zurück an die Festung
            }
            if q.distance(BURG_ORT) < 130.0 {
                kosten += 30.0; // um den Tafelberg der Burg herum
            }
            if frueher.iter().any(|alt| alt.iter().step_by(2).any(|a| a.distance(q) < 45.0)) {
                kosten += 20.0; // Abstand zu den anderen Straßen
            }
            if kosten < beste.0 {
                beste = (kosten, d);
            }
        }
        richtung = beste.1;
        p += richtung * schritt;
        laenge += schritt;
        punkte.push(p);
    }
    punkte
}

fn find_along(terrain: &Terrain, from: Vec2, direction: Vec2, wanted: impl Fn(f32) -> bool) -> Option<Vec2> {
    (1..100).map(|i| from + direction * (i as f32 * 4.0)).find(|p| wanted(terrain.height_at(p.x, p.y))).map(|p| p - direction * 4.0)
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
            if h < 1.8 {
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
    color = sand.lerp(color, smoothstep(1.4, 2.6, c.y));
    color = wet_sand.lerp(color, smoothstep(-0.8, 0.6, c.y));
    color = color.lerp(rock, smoothstep(0.42, 0.58, slope));
    color = color.lerp(snow, smoothstep(29.0, 34.0, c.y) * (1.0 - smoothstep(0.55, 0.75, slope)));
    // Rund um die Schattenfestung: aschiger, toter Boden mit glimmenden violetten Adern
    let d = duester(p);
    if d > 0.0 {
        let flecken = fbm(p * 0.05 + vec2(2.0, 7.0), 3, SEED + 91) * 0.5 + 0.5;
        let asche = vec3(0.05, 0.043, 0.055).lerp(vec3(0.085, 0.07, 0.075), flecken);
        let ader = (1.0 - (fbm(p * 0.09 + vec2(-4.0, 1.0), 2, SEED + 92) * 6.0).abs()).max(0.0);
        let boden = asche.lerp(vec3(0.16, 0.035, 0.24), smoothstep(0.82, 0.97, ader) * 0.8);
        color = color.lerp(boden, d * 0.92);
    }
    color
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


/// Wiese (für Schmetterlinge): grün, flach, nicht feucht, nicht im Gebirge.
pub fn is_meadow(terrain: &Terrain, p: Vec2) -> bool {
    let h = terrain.height_at(p.x, p.y);
    (3.0..14.0).contains(&h) && moisture(p) < 0.52 && terrain.normal_at(p.x, p.y).y > 0.9
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
/// Siedlungsplätze (Mitte): dort und drumherum stehen keine anderen Orte (einmal gesetzt, immer gleich).
static SIEDLUNGEN: std::sync::OnceLock<Vec<Vec2>> = std::sync::OnceLock::new();

fn best_spot(mut score: impl FnMut(Vec2) -> Option<f32>) -> Option<Vec2> {
    let r = ISLAND_RADIUS * 1.05;
    let steps = (r * 2.0 / 6.0) as i32;
    let mut best: Option<(f32, Vec2)> = None;
    for iz in 0..=steps {
        for ix in 0..=steps {
            let p = vec2(-r + ix as f32 * 6.0, -r + iz as f32 * 6.0);
            // Rund um die Burg und ihre Auffahrt ist kein Platz für andere Orte
            if burg_rand(p) < 30.0 || burg_weg(p).0 < 15.0 || festung_rand(p) < 40.0 || SIEDLUNGEN.get().is_some_and(|l| l.iter().any(|m| p.distance(*m) < SIEDLUNG_RAND + 25.0)) {
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
            && magic(p) < 0.58
            && unevenness(terrain, p, 4.0) < 0.9)
            .then(|| height - p.distance(vec2(0.0, 0.1 * ISLAND_RADIUS)) * 0.03)
    })
    .map(|p| (p, (camp - p).normalize_or(Vec2::X)));
    // Steinkreis: ebene Lichtung tief im Zauberwald
    let circle = best_spot(|p| {
        let height = h(p);
        ((3.0..16.0).contains(&height) && magic(p) > 0.62 && p.distance(camp) > 60.0 && unevenness(terrain, p, 6.0) < 1.2)
            .then(|| magic(p) - unevenness(terrain, p, 6.0) * 0.1)
    });
    // Kein Bergsee mehr: der Schrein am Seeufer entfällt
    let lake_shore = None;
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
    // Mühlenhof: große, ebene Wiese ein Stück vom Lager entfernt, fern von Burg und Festung
    let farm = best_spot(|p| {
        let height = h(p);
        let distance = p.distance(start);
        let flat = unevenness(terrain, p, 14.0);
        ((2.5..14.0).contains(&height)
            && (110.0..280.0).contains(&distance)
            && p.distance(camp) > 100.0
            && p.distance(BURG_ORT) > 170.0
            && p.length() > 190.0
            && tower.is_none_or(|(t, _)| t.distance(p) > 70.0)
            && magic(p) < 0.5
            && flat < 1.1
            // offene Wiese ringsum (dort stehen kaum Bäume)
            && DIRECTIONS.iter().all(|&d| is_meadow(terrain, p + d * 16.0)))
            .then(|| -(distance - 160.0).abs() * 0.01 - flat)
    })
    .map(|p| (p, (camp - p).normalize_or(Vec2::X)));
    // Tempelruine: ebene Anhöhe mit Weitblick, fern von Burg, Festung und den anderen Orten
    let others: Vec<Vec2> = [tower.map(|t| t.0), farm.map(|f| f.0), circle].into_iter().flatten().collect();
    let ruin = best_spot(|p| {
        let height = h(p);
        let flat = unevenness(terrain, p, 7.0);
        ((13.0..30.0).contains(&height)
            && flat < 1.2
            && p.length() > 170.0
            && p.distance(BURG_ORT) > 150.0
            && p.distance(camp) > 120.0
            && others.iter().all(|o| o.distance(p) > 90.0))
            .then(|| height * 0.1 - flat * 2.0)
    })
    .map(|p| (p, (camp - p).normalize_or(Vec2::X)));
    crate::orte::SightSpots { tower, circle, lake_shore, wreck, lighthouse, cave, farm, ruin }
}


/// Sucht einen flachen Platz auf einer Wiese im Süden der Insel.
fn find_spawn(terrain: &Terrain) -> Vec3 {
    for distance in (20..(ISLAND_RADIUS * 0.8) as i32).rev().step_by(4) {
        for step in 0..24 {
            let angle = std::f32::consts::FRAC_PI_2 + (step as f32 - 12.0) * 0.08;
            let p = vec2(angle.cos(), angle.sin()) * distance as f32;
            let h = terrain.height_at(p.x, p.y);
            if (3.0..9.0).contains(&h) && terrain.normal_at(p.x, p.y).y > 0.93 && magic(p) < 0.55 {
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

/// Wasserfläche als Ring von `innen` bis `aussen` (Meter): innen fein, nach außen gröber.
fn meer_ring(innen: f32, aussen: f32) -> MeshData {
    let ecken = 160;
    let ringe = 60;
    let mut mesh = MeshData::default();
    for i in 0..=ringe {
        // Abstand wächst nach außen quadratisch: am Ufer dicht (Wellen), draußen weit
        let t = i as f32 / ringe as f32;
        let r = innen + (aussen - innen) * t * t;
        for k in 0..ecken {
            let w = std::f32::consts::TAU * k as f32 / ecken as f32;
            mesh.vertices.push(Vertex::new(vec3(w.cos() * r, 0.0, w.sin() * r), Vec3::Y, Vec3::ONE));
        }
    }
    for i in 0..ringe as u32 {
        for k in 0..ecken as u32 {
            let a = i * ecken as u32 + k;
            let b = i * ecken as u32 + (k + 1) % ecken as u32;
            let (c, d) = (a + ecken as u32, b + ecken as u32);
            mesh.indices.extend([a, c, b, b, c, d]);
        }
    }
    mesh
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

/// Kantenlänge eines Landschaftsstücks in Zellen (40 × 3 m = 120 m).
const CHUNK_CELLS: usize = 40;

/// Die Landschaft in Stücken: Die Grafikkarte zeichnet nur, was im Blickfeld (bzw. im
/// Schattenbereich) liegt, und ferne Stücke gröber. Stücke, die ganz unter dem Meer liegen,
/// fallen weg – das (undurchsichtige) Wasser deckt sie ohnehin zu.
fn spawn_ground_chunks(ctx: &mut Context, terrain: &Terrain, texture: TextureId) {
    let tint = |c: Vec3, _: Vec3| Vec3::splat(facet_jitter(c));
    let cells = terrain.cells();
    for z0 in (0..cells).step_by(CHUNK_CELLS) {
        for x0 in (0..cells).step_by(CHUNK_CELLS) {
            if terrain.max_height_in(x0, z0, CHUNK_CELLS) < -2.0 {
                continue;
            }
            let full = ctx.assets.add_mesh(terrain.chunk_mesh(x0, z0, CHUNK_CELLS, 1, texture, tint));
            let half = ctx.assets.add_mesh(terrain.chunk_mesh(x0, z0, CHUNK_CELLS, 2, texture, tint));
            let quarter = ctx.assets.add_mesh(terrain.chunk_mesh(x0, z0, CHUNK_CELLS, 4, texture, tint));
            ctx.assets.set_lods(full, vec![Lod { distance: 210.0, mesh: Some(half) }, Lod { distance: 460.0, mesh: Some(quarter) }]);
            ctx.scene.spawn(Entity::new("Boden", full).with_material(Material::Ground));
        }
    }
}

/// Baut die Insel in die Szene: Landschaft, Wasser und Deko. Die abbaubaren Rohstoffe
/// werden nur beschrieben – die Welt erzeugt sie, damit sie verschwinden und
/// nachwachsen können.
pub fn build(ctx: &mut Context) -> Island {
    // Licht und Himmel kommen vom Tag-Nacht-Zyklus (`World::day`).
    ctx.env.shadow_range = 45.0;

    // Erst das natürliche Gelände: daraus Startpunkt, Wege und Heerstraßen. Dann werden an den
    // Straßenenden die Siedlungsplätze geebnet (das endgültige Gelände).
    let natur = Terrain::generate(Vec2::ZERO, TERRAIN_SIZE, TERRAIN_CELLS, height);
    let spawn = find_spawn(&natur);
    let mut paths = Paths::build(&natur, spawn);
    let siedlungen = siedlungsplaetze(&natur, &paths.strassen);
    let _ = SIEDLUNGEN.set(siedlungen.iter().map(|s| s.0).collect());
    drop(natur);
    let terrain = Terrain::generate(Vec2::ZERO, TERRAIN_SIZE, TERRAIN_CELLS, |p| siedlungsgrund(p, height(p), &siedlungen));
    let spawn = vec3(spawn.x, terrain.height_at(spawn.x, spawn.z), spawn.z);
    // Dorfweg vom Straßenende bis in die Mitte des Siedlungsplatzes
    for (strasse, &(mitte, _)) in paths.strassen.clone().iter().zip(&siedlungen) {
        paths.stamp(*strasse.last().unwrap_or(&mitte), mitte, 1.6);
    }
    // Mit Fenster: fein aufgelöste Bodentextur; der Server braucht nur die Form.
    let ground = if ctx.is_headless() {
        let terrain_mesh = ctx.assets.named_mesh(&format!("insel{SEED}"), || terrain.mesh(|c, n| ground_color(c, n) * facet_jitter(c)));
        ctx.scene.spawn(Entity::new("Insel", terrain_mesh).with_material(Material::Ground))
    } else {
        let started = std::time::Instant::now();
        let texture = ctx.assets.named_texture(&format!("boden{WORLD_ID}"), || ground_texture(&terrain, &paths));
        log::info!("Bodentextur in {:.0} ms", started.elapsed().as_secs_f32() * 1000.0);
        spawn_ground_chunks(ctx, &terrain, texture);
        // Die Kollision gehört zu einem unsichtbaren Objekt „Insel“; gezeichnet werden die Stücke.
        let mut ground = Entity::new("Insel", ctx.assets.cube());
        ground.visible = false;
        ctx.scene.spawn(ground)
    };
    let (vertices, triangles) = terrain.collision_mesh();
    ctx.physics.add_static_mesh(Some(ground), vertices, triangles);

    // Das Meer als Ring um die Insel: im Inneren liegt immer Land, dort muss kein Wasser gezeichnet werden
    let water = ctx.assets.named_mesh("meer_ring", || meer_ring(ISLAND_RADIUS * 0.28, 2100.0));
    ctx.scene.spawn(Entity::new("Meer", water).with_material(Material::Water));

    let lib = Library::load(ctx);
    let mut resources = Vec::new();
    let mut crystals = Vec::new();
    let mut places = crate::orte::Places::default();
    crate::orte::build_camp(ctx, &terrain, spawn, &mut places, &landmarks());
    crate::orte::build_castle(ctx, &mut places);
    crate::orte::build_festung(ctx, &mut places);
    let camp = crate::orte::camp_center(spawn);
    let spots = find_sights(&terrain, spawn, camp);
    let blocked = crate::orte::build_sights(ctx, &terrain, &spots, &mut places);
    let is_blocked = |p: Vec2| {
        blocked.iter().any(|&(c, r)| p.distance(c) < r) || burg_frei(p) || festung_frei(p) || siedlungen.iter().any(|&(m, _)| p.distance(m) < SIEDLUNG_FREI)
    };


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
            if h < 0.9 {
                continue;
            }
            let normal = terrain.normal_at(p.x, p.y);
            let slope = 1.0 - normal.y;
            let base = vec3(p.x, h, p.y);
            // Auf Wegen und um den Startpunkt wächst nichts Großes.
            let clearing = base.distance(spawn) < SPAWN_CLEARING
                || p.distance(camp) < crate::orte::CAMP_RADIUS
                || paths.at(p) > 0.15
                || is_blocked(p);
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
                    // Jede zweite Laterne: Rastplatz mit Bank am Wegrand, Blick auf den Weg,
                    // manchmal mit Fass oder Kiste daneben
                    if places.lanterns.len() % 2 == 0 {
                        let toward = DIRECTIONS.iter().copied().max_by(|a, b| paths.at(p + *a * 1.6).total_cmp(&paths.at(p + *b * 1.6))).unwrap_or(Vec2::X);
                        let seat = p - toward * 1.3 + toward.perp() * 1.6;
                        let turn = Quat::from_rotation_y((-toward.y).atan2(toward.x));
                        let seat_base = vec3(seat.x, terrain.height_at(seat.x, seat.y) - 0.03, seat.y);
                        if let Some(&(bench, _)) = asset_files::load_variants(ctx, "gebaeude", "bank", Vec3::ONE, 0.0).first() {
                            ctx.scene.spawn(Entity::new("Rastbank", bench).with_transform(Transform::from_position(seat_base).with_rotation(turn * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2))));
                        }
                        let extra = if hash01(ix, iz, SEED + 64) < 0.5 { "fass" } else { "kiste" };
                        let spot = seat + toward.perp() * 1.4;
                        if let Some(&(mesh, _)) = asset_files::load_variants(ctx, "gebaeude", extra, Vec3::ONE, 0.0).first() {
                            let at = vec3(spot.x, terrain.height_at(spot.x, spot.y) - 0.03, spot.y);
                            ctx.scene.spawn(Entity::new("Rastplatz", mesh).with_transform(Transform::from_position(at).with_rotation(yaw)));
                        }
                    }
                }
            }

            // Um Vorkommen herum kein hohes Gras, sonst verschwinden sie darin.
            let node_here = found.as_ref().is_some_and(|s| s.kind.needs_pickaxe()) || crystals.last() == Some(&base);
            if let Some(mut spec) = found {
                spec.color *= duester_farbe(vec2(spec.transform.position.x, spec.transform.position.z));
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
    let strassen = paths.strassen.clone();
    Island { terrain, resources, spawn, crystals, map, places, strassen, siedlungen }
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
    let color = color * duester_farbe(vec2(base.x, base.z));
    let mut entity = Entity::new("Deko", mesh).with_transform(transform).with_color(color).with_material(material);
    // Gras, Farne und Blumen werfen keinen Schatten (winzig, aber sehr viele Blattkarten)
    entity.casts_shadow = !(material == GRASS || material == FLOWERS);
    ctx.scene.spawn(entity);
    // Leuchtende Teile aus Blender-Modellen (Material mit Emission) glühen nachts.
    if let Some(glow) = glow {
        ctx.scene.spawn(Entity::new("Deko-Leuchten", glow).with_transform(transform).with_material(Material::Emissive { glow: 1.2 }));
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
