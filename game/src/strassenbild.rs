//! Die Heerstraßen im Bild (nur mit Fenster): Kopfsteinpflaster im Läuferverband mit dunklen
//! Fugen und ausgebrochenen Steinen an den Rändern, Randsteine, Meilensteine alle 50 m und
//! Laternen alle 40 m. Im düsteren Land um die Festung wird alles dunkler, die Laternen glühen violett.
//!
//! Alles aus einfachen Vierecken in wenigen großen Meshes (je Straße ein Stück alle ~35 m), damit
//! es kaum Draw-Calls kostet; die Höhe folgt dem Gelände unter jedem Stein.

use engine::prelude::*;

use crate::island::duester;

/// Halbe Breite des Pflasters, Kantenlänge eines Steins, Abstand der Randsteine zur Mitte
const HALB: f32 = 2.35;
const STEIN: f32 = 0.52;
const RAND: f32 = HALB + 0.3;
/// So hoch liegt das Pflaster über dem Gelände (gegen Flimmern)
const UEBER: f32 = 0.06;
/// Reihen je Mesh-Stück
const STUECK: usize = 64;

/// Einfacher Zufall aus Ganzzahlen (auf allen Rechnern gleich, unabhängig von der Reihenfolge).
fn zufall(a: u32, b: u32, c: u32) -> f32 {
    let mut h = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77) ^ c.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h & 0xFFFF) as f32 / 65535.0
}

/// Farbe im düsteren Land abdunkeln und ins Violette ziehen.
fn duester_machen(farbe: Vec3, p: Vec2) -> Vec3 {
    let d = duester(p);
    farbe.lerp(farbe * vec3(0.45, 0.4, 0.55), d)
}

/// Ein flaches Viereck (a, b, c, d gegen den Uhrzeigersinn von oben gesehen).
fn viereck(mesh: &mut MeshData, a: Vec3, b: Vec3, c: Vec3, d: Vec3, farbe: Vec3) {
    mesh.push_triangle(a, c, b, farbe);
    mesh.push_triangle(a, d, c, farbe);
}

/// Ein Quader (ohne Boden) mit Mitte unten, Richtung `vorne` (x, z) und Größe (breit, hoch, lang).
fn kiste(mesh: &mut MeshData, unten: Vec3, vorne: Vec2, groesse: Vec3, farbe: Vec3) {
    let f = vec3(vorne.x, 0.0, vorne.y) * groesse.z * 0.5;
    let r = vec3(-vorne.y, 0.0, vorne.x) * groesse.x * 0.5;
    let h = Vec3::Y * groesse.y;
    let ecke = |sf: f32, sr: f32, oben: bool| unten + f * sf + r * sr + if oben { h } else { Vec3::ZERO };
    let (a, b, c, d) = (ecke(-1.0, -1.0, true), ecke(1.0, -1.0, true), ecke(1.0, 1.0, true), ecke(-1.0, 1.0, true));
    mesh.push_triangle(a, c, b, farbe);
    mesh.push_triangle(a, d, c, farbe);
    let dunkler = farbe * 0.8;
    for (p, q) in [((-1.0, -1.0), (1.0, -1.0)), ((1.0, -1.0), (1.0, 1.0)), ((1.0, 1.0), (-1.0, 1.0)), ((-1.0, 1.0), (-1.0, -1.0))] {
        let (u0, u1) = (ecke(p.0, p.1, false), ecke(q.0, q.1, false));
        let (o0, o1) = (ecke(p.0, p.1, true), ecke(q.0, q.1, true));
        mesh.push_triangle(u0, o1, u1, dunkler);
        mesh.push_triangle(u0, o0, o1, dunkler);
    }
}

/// Spitze (vierseitig) mit Mitte unten.
fn spitze(mesh: &mut MeshData, unten: Vec3, breite: f32, hoehe: f32, farbe: Vec3) {
    let top = unten + Vec3::Y * hoehe;
    let e = [vec3(-1.0, 0.0, -1.0), vec3(1.0, 0.0, -1.0), vec3(1.0, 0.0, 1.0), vec3(-1.0, 0.0, 1.0)].map(|v| unten + v * breite * 0.5);
    for i in 0..4 {
        mesh.push_triangle(e[(i + 1) % 4], e[i], top, farbe * if i % 2 == 0 { 1.0 } else { 0.85 });
    }
}

/// Mittellinie in gleichmäßigen Abständen: Ort (x, z) und Richtung.
fn abtasten(linie: &[Vec2], schritt: f32) -> Vec<(Vec2, Vec2)> {
    let mut punkte = Vec::new();
    let mut rest = 0.0;
    for paar in linie.windows(2) {
        let (a, b) = (paar[0], paar[1]);
        let laenge = a.distance(b);
        if laenge < 1e-4 {
            continue;
        }
        let dir = (b - a) / laenge;
        let mut t = rest;
        while t < laenge {
            punkte.push((a + dir * t, dir));
            t += schritt;
        }
        rest = t - laenge;
    }
    // Richtungen glätten, damit die Reihen in Kurven nicht springen
    let roh: Vec<Vec2> = punkte.iter().map(|p| p.1).collect();
    for (i, p) in punkte.iter_mut().enumerate() {
        let lo = i.saturating_sub(3);
        let hi = (i + 4).min(roh.len());
        p.1 = roh[lo..hi].iter().copied().sum::<Vec2>().normalize_or(p.1);
    }
    punkte
}

/// Alle Heerstraßen bauen. `hoehe`: Gelände; `lichter`: Laternenlichter kommen hier dazu.
pub fn bauen(ctx: &mut Context, hoehe: &dyn Fn(Vec2) -> f32, strassen: &[Vec<Vec2>], lichter: &mut Vec<(Vec3, Vec3, f32)>) {
    let mut dreiecke = 0;
    for (nr, linie) in strassen.iter().enumerate() {
        let reihen = abtasten(linie, STEIN);
        let n = nr as u32;
        for (stueck, abschnitt) in reihen.chunks(STUECK).enumerate() {
            let mut pflaster = MeshData::default();
            let mut rand = MeshData::default();
            let mut glas = MeshData::default();
            for (k, &(mitte, dir)) in abschnitt.iter().enumerate() {
                let i = (stueck * STUECK + k) as u32;
                let quer = vec2(-dir.y, dir.x);
                let auf = |p: Vec2, extra: f32| vec3(p.x, hoehe(p) + UEBER + extra, p.y);
                // Steine im Läuferverband, zum Rand hin ausgebrochen
                let versatz = if i % 2 == 0 { 0.0 } else { STEIN * 0.5 };
                let spalten = (HALB / STEIN).ceil() as i32 + 1;
                for j in -spalten..=spalten {
                    let seitlich = j as f32 * STEIN + versatz;
                    if seitlich.abs() > HALB - STEIN * 0.35 {
                        continue;
                    }
                    let aussen = (seitlich.abs() / HALB).powi(3);
                    if zufall(n, i, j as u32 + 100) < 0.04 + aussen * 0.45 {
                        continue;
                    }
                    // Farbe: grau, warmer Sandstein, kühler Basalt, dunkle Steine, am Rand bemoost
                    let r = zufall(n, i, j as u32 + 300);
                    let ton = 0.15 + 0.11 * zufall(n, i, j as u32 + 400);
                    let farbe = if r > 0.85 {
                        vec3(ton * 1.15, ton * 1.0, ton * 0.78)
                    } else if r < 0.15 {
                        vec3(ton * 0.8, ton * 0.84, ton * 0.92)
                    } else if r < 0.25 {
                        vec3(ton * 0.6, ton * 0.58, ton * 0.56)
                    } else {
                        vec3(ton, ton * 0.97, ton * 0.9)
                    };
                    let farbe = if zufall(n, i, j as u32 + 700) < aussen * 0.6 { farbe.lerp(vec3(0.1, 0.16, 0.06), 0.6) } else { farbe };
                    let farbe = duester_machen(farbe, mitte);
                    // Unregelmäßige Steine: jede Ecke etwas verschoben, schmale Fugen
                    let luecke = 0.028;
                    let (l, s) = (STEIN * 0.5 - luecke, STEIN * 0.5 - luecke);
                    let c = mitte + quer * seitlich;
                    let buckel = 0.02 + 0.03 * zufall(n, i, j as u32 + 500);
                    let wackel = |k: u32| (zufall(n, i, j as u32 * 8 + k + 900) - 0.5) * 0.07;
                    viereck(
                        &mut pflaster,
                        auf(c - dir * (l + wackel(0)) - quer * (s + wackel(1)), buckel),
                        auf(c + dir * (l + wackel(2)) - quer * (s + wackel(3)), buckel),
                        auf(c + dir * (l + wackel(4)) + quer * (s + wackel(5)), buckel),
                        auf(c - dir * (l + wackel(6)) + quer * (s + wackel(7)), buckel),
                        farbe,
                    );
                }
                // Randsteine links und rechts (jeder zweite Reihe einer, hier und da fehlt einer)
                if i % 2 == 0 {
                    for seite in [-1.0f32, 1.0] {
                        if zufall(n, i, if seite > 0.0 { 7 } else { 8 }) < 0.08 {
                            continue;
                        }
                        let p = mitte + quer * RAND * seite;
                        let ton = 0.26 + 0.06 * zufall(n, i, 9);
                        let farbe = duester_machen(vec3(ton, ton * 0.97, ton * 0.92), p);
                        kiste(&mut rand, vec3(p.x, hoehe(p) - 0.1, p.y), dir, vec3(0.3, 0.32, STEIN * 1.85), farbe);
                    }
                }
                // Meilenstein alle 50 m (rechts), Laterne alle 40 m (abwechselnd links und rechts)
                let meter = i as f32 * STEIN;
                if (meter % 50.0) < STEIN && i > 0 {
                    let p = mitte + quer * (RAND + 0.7);
                    let unten = vec3(p.x, hoehe(p) - 0.2, p.y);
                    let farbe = duester_machen(vec3(0.42, 0.4, 0.36), p);
                    kiste(&mut rand, unten, dir, vec3(0.38, 0.95, 0.3), farbe);
                    spitze(&mut rand, unten + Vec3::Y * 0.95, 0.38, 0.25, farbe);
                    kiste(&mut rand, unten + Vec3::Y * 0.55, dir, vec3(0.4, 0.14, 0.32), duester_machen(vec3(0.45, 0.12, 0.1), p));
                }
                if (meter % 40.0) < STEIN && i > 20 {
                    let seite = if (meter / 40.0) as i32 % 2 == 0 { 1.0 } else { -1.0 };
                    let p = mitte + quer * (RAND + 0.9) * seite;
                    let unten = vec3(p.x, hoehe(p) - 0.2, p.y);
                    let holz = duester_machen(vec3(0.2, 0.12, 0.06), p);
                    kiste(&mut rand, unten, dir, vec3(0.16, 3.0, 0.16), holz);
                    // Ausleger zur Straße und Laterne daran
                    let arm = unten + Vec3::Y * 2.75 - vec3(quer.x, 0.0, quer.y) * seite * 0.35;
                    kiste(&mut rand, arm - Vec3::Y * 0.05, quer, vec3(0.1, 0.1, 0.8), holz);
                    let lampe = unten + Vec3::Y * 2.2 - vec3(quer.x, 0.0, quer.y) * seite * 0.65;
                    kiste(&mut rand, lampe + Vec3::Y * 0.5, dir, vec3(0.36, 0.06, 0.36), vec3(0.08, 0.08, 0.09));
                    spitze(&mut rand, lampe + Vec3::Y * 0.56, 0.4, 0.2, vec3(0.08, 0.08, 0.09));
                    let d = duester(p);
                    let licht = vec3(1.0, 0.72, 0.35).lerp(vec3(0.75, 0.4, 1.0), d);
                    kiste(&mut glas, lampe + Vec3::Y * 0.12, dir, vec3(0.28, 0.38, 0.28), licht);
                    lichter.push((lampe + Vec3::Y * 0.3, licht * 1.6, 9.0));
                }
            }
            dreiecke += (pflaster.indices.len() + rand.indices.len() + glas.indices.len()) / 3;
            for (name, mesh, leuchten) in [("Straßenpflaster", pflaster, false), ("Randsteine", rand, false), ("Laternenglas", glas, true)] {
                if mesh.indices.is_empty() {
                    continue;
                }
                let id = ctx.assets.add_mesh(mesh);
                let mut entity = Entity::new(name, id);
                entity.casts_shadow = name == "Randsteine";
                if leuchten {
                    entity.material = Material::Emissive { glow: 2.4 };
                }
                ctx.scene.spawn(entity);
            }
        }
    }
    log::info!("Heerstraßen gepflastert: {dreiecke} Dreiecke");
}
