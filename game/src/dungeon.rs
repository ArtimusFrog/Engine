//! Dungeons: drei unterirdische Anlagen mit mehreren Ebenen – die Spinnengrotte (3 Ebenen), die
//! Gruft der Vergessenen (4) und die Schmiede der Tiefe (5). Ihre Eingänge stehen auf der Insel;
//! wer hindurchgeht (E), landet im ersten Raum der ersten Ebene. Jede Ebene ist eine Folge von
//! Räumen, verbunden durch Gänge, mit Fackeln und Einrichtung passend zum Thema; im letzten Raum
//! führt eine Treppe tiefer, am Anfang eine zurück. Ganz unten wartet der Endgegner, dahinter der
//! Weg zurück ans Tageslicht.
//!
//! Die Ebenen liegen weit draußen über dem Meer (x ab `DUNGEON_X`), ringsum geschlossen – so gelten
//! Physik, Netzwerk und Gegner wie überall, nur das Licht wird drinnen dunkel (`licht`).
//! Gegner stellen die Lager der Wildnis (`wildnis.rs`) – je Raum ein Lager, das seinen Raum nicht
//! verlässt. Aufbau und Kollision entstehen auf allen Rechnern gleich (fester Zufall).

use engine::noise::Rng;
use engine::prelude::*;

/// Ab hier (Welt-x) liegen die Dungeons.
pub const DUNGEON_X: f32 = 6000.0;
/// Bodenhöhe aller Ebenen.
pub const BODEN_Y: f32 = 60.0;
const WAND_H: f32 = 5.6;
const WAND_D: f32 = 0.7;
const GANG_B: f32 = 4.4;
/// Rastermaß der Räume
const ZELLE: f32 = 36.0;
/// So nah muss man an einem Durchgang stehen (Meter).
pub const DURCHGANG_WEITE: f32 = 3.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Thema {
    Grotte,
    Gruft,
    Schmiede,
}

impl Thema {
    /// Stein der Wände, Boden, Akzent (Leuchten), Fackelfeuer
    fn farben(self) -> (Vec3, Vec3, Vec3, Vec3) {
        match self {
            Thema::Grotte => (vec3(0.36, 0.34, 0.3), vec3(0.3, 0.28, 0.24), vec3(0.4, 0.95, 0.75), vec3(1.0, 0.62, 0.28)),
            Thema::Gruft => (vec3(0.5, 0.5, 0.52), vec3(0.34, 0.34, 0.36), vec3(0.55, 0.75, 1.0), vec3(0.7, 0.85, 1.0)),
            Thema::Schmiede => (vec3(0.26, 0.23, 0.22), vec3(0.2, 0.18, 0.17), vec3(1.0, 0.45, 0.1), vec3(1.0, 0.55, 0.2)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RaumArt {
    Start,
    Kampf,
    /// Letzter Raum der Ebene (stärkere Gegner) bzw. ganz unten der Endgegner
    Ende,
    /// Seitenraum mit Gegnern
    Neben,
}

#[derive(Clone, Copy, Debug)]
pub struct Raum {
    pub min: Vec2,
    pub max: Vec2,
    pub art: RaumArt,
}

impl Raum {
    pub fn mitte(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }
}

pub struct Ebene {
    pub raeume: Vec<Raum>,
    /// Gänge als Rechtecke (min, max)
    pub gaenge: Vec<(Vec2, Vec2)>,
    /// Treppe hinauf (im Startraum) und hinab (im letzten Raum, fehlt ganz unten)
    pub hoch: Vec3,
    pub runter: Option<Vec3>,
    /// Ganz unten: der Weg zurück ans Tageslicht (hinter dem Endgegner)
    pub ausgang: Option<Vec3>,
    /// Fackeln: Ort der Flamme und Richtung von der Wand weg
    pub fackeln: Vec<(Vec3, Vec3)>,
}

impl Ebene {
    /// Ankunft beim Herabsteigen bzw. Heraufkommen (neben der Treppe, zum Raum hin)
    pub fn ankunft_oben(&self) -> Vec3 {
        let m = self.raeume[0].mitte();
        self.hoch.lerp(vec3(m.x, BODEN_Y, m.y), 0.35) + Vec3::Y * 1.2
    }

    pub fn ankunft_unten(&self) -> Option<Vec3> {
        let r = self.raeume.iter().find(|r| r.art == RaumArt::Ende)?;
        let m = r.mitte();
        Some(self.runter?.lerp(vec3(m.x, BODEN_Y, m.y), 0.35) + Vec3::Y * 1.2)
    }

    /// Liegt (x, z) auf dem Boden dieser Ebene?
    pub fn enthaelt(&self, p: Vec2) -> bool {
        self.raeume.iter().any(|r| p.cmpge(r.min).all() && p.cmple(r.max).all()) || self.gaenge.iter().any(|(a, b)| p.cmpge(*a).all() && p.cmple(*b).all())
    }
}

pub struct Dungeon {
    pub name: &'static str,
    pub thema: Thema,
    /// Auf der Insel: Mitte des Eingangs am Boden, Blickrichtung (Winkel um Y, 0 = +Z) und wo man
    /// herauskommt
    pub eingang: Vec3,
    pub eingang_yaw: f32,
    pub ebenen: Vec<Ebene>,
    /// Lagerart (Index in `wildnis::ARTEN`) je Ebene für Kampf-, Neben- und End-Räume
    arten: &'static [(usize, usize, usize)],
}

impl Dungeon {
    pub fn draussen(&self) -> Vec3 {
        self.eingang + Quat::from_rotation_y(self.eingang_yaw) * vec3(0.0, 1.2, 7.5)
    }
}

/// Ein Durchgang (E): wo man steht, wohin es geht, was dran steht.
#[derive(Clone, Debug)]
pub struct Durchgang {
    pub ort: Vec3,
    pub ziel: Vec3,
    pub text: String,
}

/// (Name, Thema, Lagerarten je Ebene: Kampf, Nebenraum, Ende)
const PLAN: [(&str, Thema, &[(usize, usize, usize)]); 3] = [
    ("Spinnengrotte", Thema::Grotte, &[(5, 5, 6), (6, 5, 6), (5, 6, 7)]),
    ("Gruft der Vergessenen", Thema::Gruft, &[(8, 8, 9), (9, 8, 10), (10, 9, 10), (9, 10, 11)]),
    ("Schmiede der Tiefe", Thema::Schmiede, &[(12, 12, 13), (13, 12, 14), (14, 13, 15), (12, 14, 14), (14, 13, 16)]),
];

/// Ursprung einer Ebene (Welt).
fn ursprung(d: usize, e: usize) -> Vec2 {
    vec2(DUNGEON_X + d as f32 * 1400.0, -700.0 + e as f32 * 280.0)
}

/// Baut die drei Dungeons (Grundriss). `eingaenge`: Orte der Eingänge auf der Insel mit Blickrichtung.
pub fn planen(eingaenge: &[(Vec3, f32)]) -> Vec<Dungeon> {
    PLAN.iter()
        .zip(eingaenge)
        .enumerate()
        .map(|(d, (&(name, thema, arten), &(eingang, eingang_yaw)))| {
            let mut rng = Rng::new(0xD0_0000 + d as u64 * 77);
            let ebenen = (0..arten.len()).map(|e| ebene(&mut rng, ursprung(d, e), e + 1 == arten.len(), thema)).collect();
            Dungeon { name, thema, eingang, eingang_yaw, ebenen, arten }
        })
        .collect()
}

fn ebene(rng: &mut Rng, o: Vec2, letzte: bool, thema: Thema) -> Ebene {
    // Zufälliger Weg durch ein 5 × 5-Raster, dazu ein, zwei Seitenräume
    let n = 5 + (rng.next_u32() % 3) as i32;
    let mut weg = vec![(2i32, 0i32)];
    let schritte = [(0, 1), (1, 0), (-1, 0), (0, -1)];
    while (weg.len() as i32) < n {
        let (x, y) = *weg.last().unwrap();
        let frei: Vec<(i32, i32)> =
            schritte.iter().map(|(dx, dy)| (x + dx, y + dy)).filter(|&(a, b)| (0..5).contains(&a) && (0..5).contains(&b) && !weg.contains(&(a, b))).collect();
        if frei.is_empty() {
            break;
        }
        // Lieber vorwärts als zurück
        let vor: Vec<(i32, i32)> = frei.iter().copied().filter(|&(_, b)| b >= y).collect();
        let wahl = if !vor.is_empty() && rng.chance(0.75) { &vor } else { &frei };
        weg.push(wahl[(rng.next_u32() as usize) % wahl.len()]);
    }
    let mut zellen: Vec<((i32, i32), RaumArt)> = weg.iter().enumerate().map(|(i, &z)| (z, if i == 0 { RaumArt::Start } else if i + 1 == weg.len() { RaumArt::Ende } else { RaumArt::Kampf })).collect();
    let mut verbindungen: Vec<(usize, usize)> = (1..zellen.len()).map(|i| (i - 1, i)).collect();
    for _ in 0..2 {
        let von = 1 + (rng.next_u32() as usize) % (weg.len() - 2).max(1);
        let (x, y) = zellen[von].0;
        let frei: Vec<(i32, i32)> = schritte
            .iter()
            .map(|(dx, dy)| (x + dx, y + dy))
            .filter(|&(a, b)| (0..5).contains(&a) && (0..5).contains(&b) && !zellen.iter().any(|z| z.0 == (a, b)))
            .collect();
        if let Some(&z) = frei.first() {
            zellen.push((z, RaumArt::Neben));
            verbindungen.push((von, zellen.len() - 1));
        }
    }
    let raeume: Vec<Raum> = zellen
        .iter()
        .map(|&((x, y), art)| {
            let mitte = o + vec2(x as f32, y as f32) * ZELLE;
            let (w, h) = match art {
                RaumArt::Start => (14.0, 14.0),
                RaumArt::Ende => (26.0, 26.0),
                _ => (rng.range(16.0, 24.0), rng.range(16.0, 24.0)),
            };
            Raum { min: mitte - vec2(w, h) * 0.5, max: mitte + vec2(w, h) * 0.5, art }
        })
        .collect();
    let gaenge: Vec<(Vec2, Vec2)> = verbindungen
        .iter()
        .map(|&(a, b)| {
            let (ra, rb) = (raeume[a], raeume[b]);
            let (ma, mb) = (ra.mitte(), rb.mitte());
            if (ma.x - mb.x).abs() > 1.0 {
                // waagerecht (entlang x)
                let (l, r) = if ma.x < mb.x { (ra, rb) } else { (rb, ra) };
                (vec2(l.max.x - 0.2, ma.y - GANG_B / 2.0), vec2(r.min.x + 0.2, ma.y + GANG_B / 2.0))
            } else {
                let (u, v) = if ma.y < mb.y { (ra, rb) } else { (rb, ra) };
                (vec2(ma.x - GANG_B / 2.0, u.max.y - 0.2), vec2(ma.x + GANG_B / 2.0, v.min.y + 0.2))
            }
        })
        .collect();
    // Treppen: hinauf an der Rückwand des Startraums, hinab bzw. hinaus in der Mitte des letzten
    let start = raeume[0];
    let hoch = vec3(start.mitte().x, BODEN_Y, start.min.y + 2.2);
    let ende = raeume.iter().find(|r| r.art == RaumArt::Ende).copied().unwrap_or(start);
    let unten = vec3(ende.mitte().x, BODEN_Y, ende.max.y - 3.5);
    let (runter, ausgang) = if letzte { (None, Some(unten)) } else { (Some(unten), None) };
    // Fackeln an den Wänden der Räume (etwa alle 8 m), nicht in Türöffnungen
    let mut fackeln = Vec::new();
    for r in &raeume {
        let kanten = [
            (vec2(r.min.x, r.min.y), vec2(r.max.x, r.min.y), vec2(0.0, 1.0)),
            (vec2(r.min.x, r.max.y), vec2(r.max.x, r.max.y), vec2(0.0, -1.0)),
            (vec2(r.min.x, r.min.y), vec2(r.min.x, r.max.y), vec2(1.0, 0.0)),
            (vec2(r.max.x, r.min.y), vec2(r.max.x, r.max.y), vec2(-1.0, 0.0)),
        ];
        for (a, b, innen) in kanten {
            let laenge = a.distance(b);
            let anzahl = ((laenge / 8.0) as usize).max(1);
            for k in 0..anzahl {
                let p = a.lerp(b, (k as f32 + 0.5) / anzahl as f32);
                let in_tuer = gaenge.iter().any(|(g0, g1)| p.cmpge(*g0 - Vec2::splat(1.5)).all() && p.cmple(*g1 + Vec2::splat(1.5)).all());
                if !in_tuer {
                    let q = p + innen * (WAND_D * 0.5 + 0.25);
                    fackeln.push((vec3(q.x, BODEN_Y + 3.0, q.y), vec3(innen.x, 0.0, innen.y)));
                }
            }
        }
    }
    let _ = thema;
    Ebene { raeume, gaenge, hoch, runter, ausgang, fackeln }
}

/// Alle Durchgänge (Eingänge, Treppen, Ausgänge) in fester Reihenfolge.
pub fn durchgaenge(dungeons: &[Dungeon]) -> Vec<Durchgang> {
    let mut liste = Vec::new();
    for d in dungeons {
        let n = d.ebenen.len();
        liste.push(Durchgang { ort: d.eingang, ziel: d.ebenen[0].ankunft_oben(), text: format!("{} betreten ({n} Ebenen)", d.name) });
        for (e, ebene) in d.ebenen.iter().enumerate() {
            let hinauf = if e == 0 { d.draussen() } else { d.ebenen[e - 1].ankunft_unten().unwrap_or(d.draussen()) };
            let text = if e == 0 { "Zurück ans Tageslicht".to_string() } else { format!("Hinauf zu Ebene {e}") };
            liste.push(Durchgang { ort: ebene.hoch, ziel: hinauf, text });
            if let Some(runter) = ebene.runter {
                liste.push(Durchgang { ort: runter, ziel: d.ebenen[e + 1].ankunft_oben(), text: format!("Hinab zu Ebene {}", e + 2) });
            }
            if let Some(ausgang) = ebene.ausgang {
                liste.push(Durchgang { ort: ausgang, ziel: d.draussen(), text: "Der Weg zurück ans Tageslicht".to_string() });
            }
        }
    }
    liste
}

/// Bodenhöhe in einem Dungeon (sonst `None`).
pub fn boden(dungeons: &[Dungeon], p: Vec2) -> Option<f32> {
    if p.x < DUNGEON_X - 200.0 {
        return None;
    }
    dungeons.iter().flat_map(|d| &d.ebenen).any(|e| e.enthaelt(p)).then_some(BODEN_Y)
}

/// In welchem Dungeon (und welcher Ebene) liegt dieser Punkt?
pub fn wo(dungeons: &[Dungeon], p: Vec3) -> Option<(usize, usize)> {
    if p.x < DUNGEON_X - 200.0 {
        return None;
    }
    for (d, dungeon) in dungeons.iter().enumerate() {
        for (e, ebene) in dungeon.ebenen.iter().enumerate() {
            let o = ursprung(d, e);
            if (vec2(p.x, p.z) - o - vec2(2.0 * ZELLE, 2.0 * ZELLE)).abs().max_element() < 3.0 * ZELLE && (p.y - BODEN_Y).abs() < 20.0 {
                let _ = ebene;
                return Some((d, e));
            }
        }
    }
    None
}

/// Lager für die Wildnis: je Raum mit Gegnern (Mitte, Lagerart, Bereich).
pub fn lager(dungeons: &[Dungeon]) -> Vec<(Vec2, usize, (Vec2, Vec2))> {
    let mut liste = Vec::new();
    for d in dungeons {
        for (e, ebene) in d.ebenen.iter().enumerate() {
            let (kampf, neben, ende) = d.arten[e];
            for r in &ebene.raeume {
                let art = match r.art {
                    RaumArt::Start => continue,
                    RaumArt::Kampf => kampf,
                    RaumArt::Neben => neben,
                    RaumArt::Ende => ende,
                };
                liste.push((r.mitte(), art, (r.min + Vec2::splat(1.2), r.max - Vec2::splat(1.2))));
            }
        }
    }
    liste
}

// ---------------------------------------------------------------------------
// Geometrie: Wände aus einzelnen Steinen, gefliester Boden, Decke, Kollision
// ---------------------------------------------------------------------------

/// Quader mit Normalen nach außen.
fn quader(m: &mut MeshData, min: Vec3, max: Vec3, farbe: Vec3) {
    let p = |x: f32, y: f32, z: f32| vec3(if x > 0.0 { max.x } else { min.x }, if y > 0.0 { max.y } else { min.y }, if z > 0.0 { max.z } else { min.z });
    let seiten = [
        ([p(0., 0., 1.), p(1., 0., 1.), p(1., 1., 1.), p(0., 1., 1.)], 0.95),
        ([p(1., 0., 0.), p(0., 0., 0.), p(0., 1., 0.), p(1., 1., 0.)], 0.85),
        ([p(0., 1., 0.), p(0., 1., 1.), p(1., 1., 1.), p(1., 1., 0.)], 1.1),
        ([p(0., 0., 1.), p(0., 0., 0.), p(1., 0., 0.), p(1., 0., 1.)], 0.6),
        ([p(1., 0., 1.), p(1., 0., 0.), p(1., 1., 0.), p(1., 1., 1.)], 0.9),
        ([p(0., 0., 0.), p(0., 0., 1.), p(0., 1., 1.), p(0., 1., 0.)], 0.8),
    ];
    for (s, hell) in seiten {
        let f = farbe * hell;
        m.push_triangle(s[0], s[1], s[2], f);
        m.push_triangle(s[0], s[2], s[3], f);
    }
}

/// Kollisionsquader (Eckpunkte und Dreiecke für ein Dreiecksnetz).
fn kollision_quader(v: &mut Vec<Vec3>, t: &mut Vec<[u32; 3]>, min: Vec3, max: Vec3) {
    let b = v.len() as u32;
    for i in 0..8 {
        v.push(vec3(if i & 1 != 0 { max.x } else { min.x }, if i & 2 != 0 { max.y } else { min.y }, if i & 4 != 0 { max.z } else { min.z }));
    }
    for [a, c, d, e] in [[0, 1, 3, 2], [4, 6, 7, 5], [0, 4, 5, 1], [2, 3, 7, 6], [0, 2, 6, 4], [1, 5, 7, 3]] {
        t.push([b + a, b + c, b + d]);
        t.push([b + a, b + d, b + e]);
    }
}

/// Eine gemauerte Wand entlang (x oder z) von `a` nach `b`, Türöffnungen ausgespart.
fn wand(m: &mut MeshData, rng: &mut Rng, a: Vec2, b: Vec2, oeffnungen: &[(f32, f32)], stein: Vec3, kol: &mut (Vec<Vec3>, Vec<[u32; 3]>)) {
    let laengs_x = (b.x - a.x).abs() > (b.y - a.y).abs();
    let (s0, s1) = if laengs_x { (a.x.min(b.x), a.x.max(b.x)) } else { (a.y.min(b.y), a.y.max(b.y)) };
    let quer = if laengs_x { a.y } else { a.x };
    // Stücke ohne Öffnungen
    let mut stuecke = vec![(s0, s1)];
    for &(mitte, breite) in oeffnungen {
        let (o0, o1) = (mitte - breite / 2.0, mitte + breite / 2.0);
        stuecke = stuecke.into_iter().flat_map(|(u, v)| if o1 <= u || o0 >= v { vec![(u, v)] } else { vec![(u, o0.max(u)), (o1.min(v), v)] }).filter(|(u, v)| v - u > 0.05).collect();
    }
    let reihe_h = 0.56;
    for (u, v) in stuecke {
        let (min, max) = if laengs_x {
            (vec3(u, BODEN_Y, quer - WAND_D / 2.0), vec3(v, BODEN_Y + WAND_H, quer + WAND_D / 2.0))
        } else {
            (vec3(quer - WAND_D / 2.0, BODEN_Y, u), vec3(quer + WAND_D / 2.0, BODEN_Y + WAND_H, v))
        };
        kollision_quader(&mut kol.0, &mut kol.1, min, max);
        // Einzelne Steine in versetzten Reihen, leicht unterschiedlich in Farbe und Tiefe
        let reihen = (WAND_H / reihe_h).ceil() as usize;
        for r in 0..reihen {
            let y0 = BODEN_Y + r as f32 * reihe_h;
            let y1 = (y0 + reihe_h - 0.04).min(BODEN_Y + WAND_H);
            let mut s = u + if r % 2 == 0 { 0.0 } else { -0.55 };
            while s < v {
                let laenge = rng.range(0.9, 1.4);
                let (t0, t1) = (s.max(u), (s + laenge - 0.04).min(v));
                s += laenge;
                if t1 - t0 < 0.1 {
                    continue;
                }
                let vor = rng.range(0.0, 0.06);
                let farbe = stein * rng.range(0.78, 1.12);
                let (min, max) = if laengs_x {
                    (vec3(t0, y0, quer - WAND_D / 2.0 - vor), vec3(t1, y1, quer + WAND_D / 2.0 + vor))
                } else {
                    (vec3(quer - WAND_D / 2.0 - vor, y0, t0), vec3(quer + WAND_D / 2.0 + vor, y1, t1))
                };
                quader(m, min, max, farbe);
            }
        }
    }
}

/// Boden (Fliesen) und Decke eines Rechtecks.
fn boden_decke(m: &mut MeshData, rng: &mut Rng, min: Vec2, max: Vec2, boden: Vec3, decke: Vec3, kol: &mut (Vec<Vec3>, Vec<[u32; 3]>)) {
    kollision_quader(&mut kol.0, &mut kol.1, vec3(min.x, BODEN_Y - 1.0, min.y), vec3(max.x, BODEN_Y, max.y));
    kollision_quader(&mut kol.0, &mut kol.1, vec3(min.x, BODEN_Y + WAND_H, min.y), vec3(max.x, BODEN_Y + WAND_H + 1.0, max.y));
    let fliese = 2.0;
    let (nx, nz) = (((max.x - min.x) / fliese).ceil() as usize, ((max.y - min.y) / fliese).ceil() as usize);
    for i in 0..nx {
        for k in 0..nz {
            let x0 = min.x + i as f32 * fliese;
            let z0 = min.y + k as f32 * fliese;
            let (x1, z1) = ((x0 + fliese).min(max.x), (z0 + fliese).min(max.y));
            let f = boden * rng.range(0.82, 1.12) * if (i + k) % 2 == 0 { 1.0 } else { 0.93 };
            let y = BODEN_Y + rng.range(0.0, 0.03);
            // Fugen: jede Fliese etwas kleiner
            let (a, b, c, d) = (vec3(x0 + 0.04, y, z0 + 0.04), vec3(x1 - 0.04, y, z0 + 0.04), vec3(x1 - 0.04, y, z1 - 0.04), vec3(x0 + 0.04, y, z1 - 0.04));
            m.push_triangle(a, d, c, f);
            m.push_triangle(a, c, b, f);
        }
    }
    // Fugengrund und Decke
    let (a, b, c, d) = (vec3(min.x, BODEN_Y - 0.01, min.y), vec3(max.x, BODEN_Y - 0.01, min.y), vec3(max.x, BODEN_Y - 0.01, max.y), vec3(min.x, BODEN_Y - 0.01, max.y));
    m.push_triangle(a, d, c, boden * 0.35);
    m.push_triangle(a, c, b, boden * 0.35);
    let h = BODEN_Y + WAND_H;
    let (a, b, c, d) = (vec3(min.x, h, min.y), vec3(max.x, h, min.y), vec3(max.x, h, max.y), vec3(min.x, h, max.y));
    m.push_triangle(a, b, c, decke);
    m.push_triangle(a, c, d, decke);
}

/// Die ganze Ebene als ein Mesh (Wände, Böden, Decken, Einrichtung) und ihr Kollisionsnetz.
pub fn ebene_mesh(dungeon: &Dungeon, e: usize) -> (MeshData, (Vec<Vec3>, Vec<[u32; 3]>)) {
    let ebene = &dungeon.ebenen[e];
    let (stein, boden, akzent, _) = dungeon.thema.farben();
    let mut rng = Rng::new(0xE0_0000 + e as u64 * 31 + dungeon.name.len() as u64);
    let mut m = MeshData::default();
    let mut kol = (Vec::new(), Vec::new());
    for r in &ebene.raeume {
        boden_decke(&mut m, &mut rng, r.min, r.max, boden, stein * 0.35, &mut kol);
        let kanten = [(vec2(r.min.x, r.min.y), vec2(r.max.x, r.min.y)), (vec2(r.min.x, r.max.y), vec2(r.max.x, r.max.y)), (vec2(r.min.x, r.min.y), vec2(r.min.x, r.max.y)),
                      (vec2(r.max.x, r.min.y), vec2(r.max.x, r.max.y))];
        for (a, b) in kanten {
            let laengs_x = (b.x - a.x).abs() > 0.1;
            // Öffnungen: wo ein Gang diese Kante kreuzt
            let oeffnungen: Vec<(f32, f32)> = ebene
                .gaenge
                .iter()
                .filter(|(g0, g1)| if laengs_x { g0.y - 0.5 <= a.y && g1.y + 0.5 >= a.y } else { g0.x - 0.5 <= a.x && g1.x + 0.5 >= a.x })
                .filter(|(g0, g1)| if laengs_x { g1.x > a.x.min(b.x) && g0.x < a.x.max(b.x) } else { g1.y > a.y.min(b.y) && g0.y < a.y.max(b.y) })
                .map(|(g0, g1)| if laengs_x { ((g0.x + g1.x) / 2.0, g1.x - g0.x) } else { ((g0.y + g1.y) / 2.0, g1.y - g0.y) })
                .collect();
            wand(&mut m, &mut rng, a, b, &oeffnungen, stein, &mut kol);
        }
        einrichtung(&mut m, &mut rng, dungeon.thema, r, stein, akzent, &mut kol);
    }
    for &(g0, g1) in &ebene.gaenge {
        boden_decke(&mut m, &mut rng, g0, g1, boden * 0.9, stein * 0.3, &mut kol);
        let laengs_x = g1.x - g0.x > g1.y - g0.y;
        if laengs_x {
            wand(&mut m, &mut rng, vec2(g0.x, g0.y), vec2(g1.x, g0.y), &[], stein * 0.95, &mut kol);
            wand(&mut m, &mut rng, vec2(g0.x, g1.y), vec2(g1.x, g1.y), &[], stein * 0.95, &mut kol);
        } else {
            wand(&mut m, &mut rng, vec2(g0.x, g0.y), vec2(g0.x, g1.y), &[], stein * 0.95, &mut kol);
            wand(&mut m, &mut rng, vec2(g1.x, g0.y), vec2(g1.x, g1.y), &[], stein * 0.95, &mut kol);
        }
    }
    // Treppen
    treppe(&mut m, ebene.hoch, true, stein, akzent);
    if let Some(p) = ebene.runter.or(ebene.ausgang) {
        treppe(&mut m, p, false, stein, akzent);
    }
    // Fackelhalter
    for &(p, n) in &ebene.fackeln {
        let wand_p = p - n * 0.25;
        quader(&mut m, wand_p - vec3(0.08, 0.5, 0.08), wand_p + vec3(0.08, -0.1, 0.08), vec3(0.2, 0.18, 0.16));
        quader(&mut m, p - vec3(0.06, 0.35, 0.06), p + vec3(0.06, 0.0, 0.06), vec3(0.4, 0.26, 0.14));
    }
    (m, kol)
}

/// Treppe: hinauf (Stufen zur Wand) bzw. hinab (dunkler Schacht mit Stufen), mit Säulen und Glut.
fn treppe(m: &mut MeshData, p: Vec3, hinauf: bool, stein: Vec3, akzent: Vec3) {
    let r = if hinauf { -1.0 } else { 1.0 };
    for k in 0..6 {
        let k = k as f32;
        // hinauf steigen die Stufen zum Tor, hinab führen zwei flache Stufen zum Schlund
        let (y0, y1) = if hinauf { (BODEN_Y, BODEN_Y + 0.3 * (k + 1.0)) } else { (BODEN_Y, BODEN_Y + 0.12 * (6.0 - k)) };
        let z0 = p.z + r * (k * 0.45 - 0.2);
        let (za, zb) = (z0.min(z0 + r * 0.45), z0.max(z0 + r * 0.45));
        quader(m, vec3(p.x - 1.6, y0 - if hinauf { 0.0 } else { 0.05 }, za), vec3(p.x + 1.6, y1, zb), stein * (0.85 + 0.04 * k));
    }
    // Ein Torbogen über der Treppe: hinauf hell, hinab ein dunkler Schlund
    let tor = if hinauf { akzent * 0.9 } else { vec3(0.01, 0.01, 0.012) };
    quader(m, vec3(p.x - 1.75, BODEN_Y, p.z + r * 2.9 - 0.05), vec3(p.x + 1.75, BODEN_Y + 3.2, p.z + r * 2.9 + 0.05), tor);
    quader(m, vec3(p.x - 2.35, BODEN_Y + 3.2, p.z + r * 2.9 - 0.3), vec3(p.x + 2.35, BODEN_Y + 3.6, p.z + r * 2.9 + 0.3), stein * 1.2);
    for s in [-1.0f32, 1.0] {
        quader(m, vec3(p.x + s * 2.0 - 0.3, BODEN_Y, p.z + r * 2.9 - 0.3), vec3(p.x + s * 2.0 + 0.3, BODEN_Y + 3.2, p.z + r * 2.9 + 0.3), stein * 1.1);
    }
    for s in [-1.0f32, 1.0] {
        quader(m, vec3(p.x + s * 2.0 - 0.3, BODEN_Y, p.z - 0.4), vec3(p.x + s * 2.0 + 0.3, BODEN_Y + 3.2, p.z + 0.2), stein * 1.15);
        quader(m, vec3(p.x + s * 2.0 - 0.35, BODEN_Y + 3.2, p.z - 0.45), vec3(p.x + s * 2.0 + 0.35, BODEN_Y + 3.45, p.z + 0.25), akzent * 0.5);
    }
}

/// Einrichtung passend zum Thema, am Rand der Räume (die Mitte bleibt frei für den Kampf).
fn einrichtung(m: &mut MeshData, rng: &mut Rng, thema: Thema, r: &Raum, stein: Vec3, akzent: Vec3, kol: &mut (Vec<Vec3>, Vec<[u32; 3]>)) {
    let rand = |rng: &mut Rng| {
        // Ein Punkt 1,5–3 m von einer Wand
        let seite = rng.next_u32() % 4;
        let t = rng.range(0.15, 0.85);
        let tief = rng.range(1.5, 3.0);
        match seite {
            0 => vec2(r.min.x + (r.max.x - r.min.x) * t, r.min.y + tief),
            1 => vec2(r.min.x + (r.max.x - r.min.x) * t, r.max.y - tief),
            2 => vec2(r.min.x + tief, r.min.y + (r.max.y - r.min.y) * t),
            _ => vec2(r.max.x - tief, r.min.y + (r.max.y - r.min.y) * t),
        }
    };
    // Säulen in großen Räumen
    if (r.max - r.min).min_element() > 19.0 {
        for (fx, fz) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
            let p = r.min + (r.max - r.min) * vec2(fx, fz);
            let (min, max) = (vec3(p.x - 0.55, BODEN_Y, p.y - 0.55), vec3(p.x + 0.55, BODEN_Y + WAND_H, p.y + 0.55));
            kollision_quader(&mut kol.0, &mut kol.1, min, max);
            for k in 0..8 {
                let y0 = BODEN_Y + k as f32 * WAND_H / 8.0;
                let schmal = if k == 0 || k == 7 { 0.0 } else { 0.08 };
                quader(m, vec3(min.x + schmal, y0, min.z + schmal), vec3(max.x - schmal, y0 + WAND_H / 8.0 - 0.03, max.z - schmal), stein * rng.range(0.9, 1.15));
            }
        }
    }
    let anzahl = 6 + (rng.next_u32() % 5) as usize;
    for _ in 0..anzahl {
        let p = rand(rng);
        match thema {
            Thema::Grotte => {
                // Tropfsteine, Spinneneier, Leuchtpilze
                match rng.next_u32() % 3 {
                    0 => {
                        let h = rng.range(0.8, 2.2);
                        let mesh = MeshData::cylinder(rng.range(0.25, 0.5), 0.02, h, 7, stein * 0.9);
                        m.append(&mesh, Mat4::from_translation(vec3(p.x, BODEN_Y + h / 2.0, p.y)));
                        let oben = MeshData::cylinder(0.02, rng.range(0.2, 0.4), h * 0.6, 7, stein * 0.8);
                        m.append(&oben, Mat4::from_translation(vec3(p.x + 0.4, BODEN_Y + WAND_H - h * 0.3, p.y)));
                    }
                    1 => {
                        for k in 0..3 {
                            let e = MeshData::icosphere(1, vec3(0.9, 0.88, 0.8));
                            let q = p + vec2(k as f32 * 0.35 - 0.35, (k % 2) as f32 * 0.3);
                            m.append(&e, Mat4::from_scale_rotation_translation(vec3(0.22, 0.3, 0.22), Quat::IDENTITY, vec3(q.x, BODEN_Y + 0.25, q.y)));
                        }
                    }
                    _ => {
                        for k in 0..4 {
                            let q = p + vec2(rng.range(-0.6, 0.6), rng.range(-0.6, 0.6));
                            let h = rng.range(0.2, 0.5);
                            let stiel = MeshData::cylinder(0.04, 0.05, h, 6, vec3(0.8, 0.8, 0.7));
                            m.append(&stiel, Mat4::from_translation(vec3(q.x, BODEN_Y + h / 2.0, q.y)));
                            let hut = MeshData::cylinder(0.18 - 0.02 * k as f32, 0.02, 0.12, 8, akzent);
                            m.append(&hut, Mat4::from_translation(vec3(q.x, BODEN_Y + h + 0.05, q.y)));
                        }
                    }
                }
            }
            Thema::Gruft => {
                // Sarkophage, Knochenhaufen, Urnen
                match rng.next_u32() % 3 {
                    0 => {
                        let quer = rng.chance(0.5);
                        let (hx, hz) = if quer { (1.1, 0.5) } else { (0.5, 1.1) };
                        let (min, max) = (vec3(p.x - hx, BODEN_Y, p.y - hz), vec3(p.x + hx, BODEN_Y + 0.8, p.y + hz));
                        kollision_quader(&mut kol.0, &mut kol.1, min, max);
                        quader(m, min, max, stein * 1.1);
                        quader(m, min + vec3(0.1, 0.8, 0.1), max + vec3(-0.1, 0.12, -0.1), stein * 1.25);
                    }
                    1 => {
                        for _ in 0..7 {
                            let q = p + vec2(rng.range(-0.5, 0.5), rng.range(-0.5, 0.5));
                            let lang = rng.range(0.3, 0.6);
                            let knochen = MeshData::cylinder(0.035, 0.035, lang, 5, vec3(0.9, 0.87, 0.78));
                            let dreh = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2) * Quat::from_rotation_x(rng.range(0.0, 3.0));
                            m.append(&knochen, Mat4::from_rotation_translation(dreh, vec3(q.x, BODEN_Y + 0.05, q.y)));
                        }
                        let schaedel = MeshData::icosphere(1, vec3(0.92, 0.9, 0.82));
                        m.append(&schaedel, Mat4::from_scale_rotation_translation(Vec3::splat(0.14), Quat::IDENTITY, vec3(p.x, BODEN_Y + 0.14, p.y)));
                    }
                    _ => {
                        let urne = MeshData::cylinder(0.25, 0.18, 0.7, 10, vec3(0.45, 0.35, 0.28));
                        m.append(&urne, Mat4::from_translation(vec3(p.x, BODEN_Y + 0.35, p.y)));
                    }
                }
            }
            Thema::Schmiede => {
                // Ambosse, Erzkisten, Lavarinnen
                match rng.next_u32() % 3 {
                    0 => {
                        let (min, max) = (vec3(p.x - 0.5, BODEN_Y, p.y - 0.3), vec3(p.x + 0.5, BODEN_Y + 0.9, p.y + 0.3));
                        kollision_quader(&mut kol.0, &mut kol.1, min, max);
                        quader(m, vec3(p.x - 0.3, BODEN_Y, p.y - 0.25), vec3(p.x + 0.3, BODEN_Y + 0.6, p.y + 0.25), vec3(0.2, 0.2, 0.22));
                        quader(m, vec3(p.x - 0.6, BODEN_Y + 0.6, p.y - 0.28), vec3(p.x + 0.6, BODEN_Y + 0.9, p.y + 0.28), vec3(0.28, 0.28, 0.3));
                    }
                    1 => {
                        let (min, max) = (vec3(p.x - 0.6, BODEN_Y, p.y - 0.6), vec3(p.x + 0.6, BODEN_Y + 1.0, p.y + 0.6));
                        kollision_quader(&mut kol.0, &mut kol.1, min, max);
                        quader(m, min, max, vec3(0.16, 0.1, 0.055));
                        quader(m, vec3(min.x - 0.02, BODEN_Y + 0.42, min.z - 0.02), vec3(max.x + 0.02, BODEN_Y + 0.52, max.z + 0.02), vec3(0.08, 0.08, 0.09));
                        for k in 0..4 {
                            let erz = MeshData::icosphere(0, vec3(0.62, 0.3, 0.16));
                            m.append(&erz, Mat4::from_scale_rotation_translation(Vec3::splat(0.2), Quat::IDENTITY, vec3(p.x - 0.3 + 0.2 * k as f32, BODEN_Y + 1.05, p.y)));
                        }
                    }
                    _ => {
                        // Lavarinne (leuchtende Farbe, das Glühen kommt von den Lichtern)
                        quader(m, vec3(p.x - 1.5, BODEN_Y + 0.01, p.y - 0.3), vec3(p.x + 1.5, BODEN_Y + 0.05, p.y + 0.3), akzent * 1.6);
                    }
                }
            }
        }
    }
}

type Kol = (Vec<Vec3>, Vec<[u32; 3]>);

/// Kleiner Zufall aus einer Position (für unregelmäßige Felsen).
fn krumm(p: Vec3, saat: f32) -> f32 {
    ((p.x * 12.9898 + p.y * 78.233 + p.z * 37.719 + saat).sin() * 43758.547).fract().abs()
}

/// Quader mit Kollision.
fn block(m: &mut MeshData, kol: &mut Kol, min: Vec3, max: Vec3, farbe: Vec3) {
    quader(m, min, max, farbe);
    kollision_quader(&mut kol.0, &mut kol.1, min, max);
}

/// Die Dreiecke eines Meshes als Kollision.
fn kol_mesh(kol: &mut Kol, mesh: &MeshData, tr: Mat4) {
    let base = kol.0.len() as u32;
    kol.0.extend(mesh.vertices.iter().map(|v| tr.transform_point3(Vec3::from(v.position))));
    kol.1.extend(mesh.indices.chunks_exact(3).map(|t| [t[0] + base, t[1] + base, t[2] + base]));
}

/// Ein facettierter Felsbrocken: unten `farbe`, oben `oben` (Moos, Asche); `adern` > 0 lässt
/// einzelne Ecken glühen (Lava).
#[allow(clippy::too_many_arguments)]
fn fels(m: &mut MeshData, kol: &mut Kol, mitte: Vec3, groesse: Vec3, drehung: f32, farbe: Vec3, oben: Vec3, adern: f32, saat: f32) {
    let roh = MeshData::icosphere(1, farbe)
        .recolor(|p, c| {
            if p.y > 0.2 {
                oben * (0.9 + 0.2 * krumm(p, saat + 1.0))
            } else {
                c * (0.85 + 0.3 * krumm(p, saat + 3.0))
            }
        })
        .displace(|p| p * (0.82 + 0.36 * krumm(p, saat)));
    let tr = Mat4::from_scale_rotation_translation(groesse, Quat::from_rotation_y(drehung), mitte);
    let mut flach = roh.flat_shaded();
    // Glühende Adern: einzelne Flächen ganz in Lavafarbe
    for (k, dreieck) in flach.vertices.chunks_mut(3).enumerate() {
        if krumm(Vec3::from(dreieck[0].position) + Vec3::splat(k as f32 * 0.37), saat + 7.0) < adern {
            for v in dreieck {
                v.color = [1.3, 0.32, 0.05];
            }
        }
    }
    m.append(&flach, tr);
    kol_mesh(kol, &roh, tr);
}

/// Dünner, flacher Faden von a nach b in der Ebene senkrecht zu `normale` (beidseitig).
fn faden(m: &mut MeshData, a: Vec3, b: Vec3, normale: Vec3, dicke: f32, farbe: Vec3) {
    let quer = (b - a).cross(normale).normalize_or_zero() * dicke * 0.5;
    let (p1, p2, p3, p4) = (a - quer, b - quer, b + quer, a + quer);
    m.push_triangle(p1, p2, p3, farbe);
    m.push_triangle(p1, p3, p4, farbe);
    m.push_triangle(p1, p3, p2, farbe);
    m.push_triangle(p1, p4, p3, farbe);
}

/// Spinnennetz (in der Ebene z = const, zeigt nach +Z): Speichen zwischen den Winkeln, Ringe.
fn netz(m: &mut MeshData, mitte: Vec3, radius: f32, von: f32, bis: f32) {
    let weiss = vec3(0.92, 0.92, 0.9);
    let speichen = 7;
    let richtung = |k: usize| {
        let w = (von + (bis - von) * k as f32 / (speichen - 1) as f32).to_radians();
        vec3(w.cos(), w.sin(), 0.0)
    };
    for k in 0..speichen {
        faden(m, mitte, mitte + richtung(k) * radius, Vec3::Z, 0.035, weiss);
    }
    for ring in 1..5 {
        let r = radius * ring as f32 / 4.6;
        for k in 0..speichen - 1 {
            let durchhang = 0.9 + 0.08 * (ring as f32 * 1.7 + k as f32).sin();
            faden(m, mitte + richtung(k) * r * durchhang, mitte + richtung(k + 1) * r, Vec3::Z, 0.025, weiss);
        }
    }
}

/// Beidseitiges Dreieck (Dächer, Giebel).
fn dreieck(m: &mut MeshData, a: Vec3, b: Vec3, c: Vec3, farbe: Vec3) {
    m.push_triangle(a, b, c, farbe);
    m.push_triangle(a, c, b, farbe);
}

/// Schacht hinter der Öffnung: wird nach hinten immer dunkler (als ginge es tief hinab) und endet
/// in Schwärze.
fn schacht(m: &mut MeshData, kol: &mut Kol, z_vorn: f32, z_hinten: f32, boden: f32, hoehe: f32, stein: Vec3) {
    let n = 6;
    for k in 0..n {
        let (t0, t1) = (k as f32 / n as f32, (k + 1) as f32 / n as f32);
        let (z0, z1) = (z_vorn + (z_hinten - z_vorn) * t0, z_vorn + (z_hinten - z_vorn) * t1);
        let dunkel = stein * 0.55 * (1.0 - t0).powi(2);
        // Stufen hinab (nur angedeutet) …
        quader(m, vec3(-1.9, boden - 0.4, z1), vec3(1.9, boden + 0.04 - 0.05 * k as f32, z0), dunkel * 0.8);
        quader(m, vec3(-1.9, boden + hoehe, z1), vec3(1.9, boden + hoehe + 0.35, z0), dunkel * 0.6);
        for s in [-1.0f32, 1.0] {
            quader(m, vec3(s * 2.0 - 0.15, boden - 0.4, z1), vec3(s * 2.0 + 0.15, boden + hoehe + 0.35, z0), dunkel);
        }
    }
    quader(m, vec3(-1.9, boden - 0.4, z_hinten - 0.2), vec3(1.9, boden + hoehe + 0.35, z_hinten), vec3(0.004, 0.004, 0.005));
    for s in [-1.0f32, 1.0] {
        kollision_quader(&mut kol.0, &mut kol.1, vec3(s * 2.0 - 0.15, boden - 0.4, z_hinten), vec3(s * 2.0 + 0.15, boden + hoehe + 0.35, z_vorn));
    }
    kollision_quader(&mut kol.0, &mut kol.1, vec3(-2.2, boden - 0.4, z_hinten - 0.3), vec3(2.2, boden + hoehe + 0.35, z_hinten));
}

/// Etwas, das am Eingang leuchtet (lokal): Flammen, Eier, Lava, Geisterfeuer.
pub struct Leuchte {
    pub ort: Vec3,
    pub groesse: Vec3,
    pub farbe: Vec3,
    pub staerke: f32,
    /// Radius des Lichts (0 = leuchtet nur selbst)
    pub licht: f32,
    pub flackern: bool,
}

fn leuchte(ort: Vec3, groesse: Vec3, farbe: Vec3, staerke: f32, licht: f32, flackern: bool) -> Leuchte {
    Leuchte { ort, groesse, farbe, staerke, licht, flackern }
}

/// Was am Eingang leuchtet (lokal, Öffnung nach +Z) und wo Rauch aufsteigt.
pub fn eingang_leuchten(thema: Thema) -> (Vec<Leuchte>, Vec<Vec3>) {
    let (_, _, akzent, feuer) = thema.farben();
    match thema {
        Thema::Grotte => {
            let mut l = vec![
                leuchte(vec3(-2.9, 2.75, 1.9), vec3(0.28, 0.5, 0.28), feuer, 2.4, 10.0, true),
                leuchte(vec3(2.9, 2.75, 1.9), vec3(0.28, 0.5, 0.28), feuer, 2.4, 10.0, true),
            ];
            // Ein Gelege leuchtender Spinneneier links und rechts
            for (k, &(x, z)) in [(-4.3, 2.4), (-4.9, 2.9), (-3.8, 3.0), (-4.6, 3.6), (4.6, 2.8), (5.1, 3.3)].iter().enumerate() {
                let s = 0.36 + 0.06 * (k % 3) as f32;
                l.push(leuchte(vec3(x, s * 0.5, z), vec3(s, s * 1.3, s), akzent, 1.1, if k == 0 { 5.0 } else { 0.0 }, false));
            }
            (l, Vec::new())
        }
        Thema::Gruft => (
            vec![
                leuchte(vec3(-5.4, 2.1, 2.2), vec3(0.4, 0.75, 0.4), feuer, 2.6, 11.0, true),
                leuchte(vec3(5.4, 2.1, 2.2), vec3(0.4, 0.75, 0.4), feuer, 2.6, 11.0, true),
                leuchte(vec3(-0.2, 7.05, 2.45), Vec3::splat(0.13), akzent, 3.0, 0.0, false),
                leuchte(vec3(0.2, 7.05, 2.45), Vec3::splat(0.13), akzent, 3.0, 0.0, false),
            ],
            Vec::new(),
        ),
        Thema::Schmiede => (
            vec![
                leuchte(vec3(-3.7, 0.1, 3.0), vec3(2.3, 0.35, 3.1), vec3(1.0, 0.36, 0.06), 2.2, 9.0, false),
                leuchte(vec3(3.7, 0.1, 3.0), vec3(2.3, 0.35, 3.1), vec3(1.0, 0.36, 0.06), 2.2, 9.0, false),
                leuchte(vec3(-5.8, 1.2, 0.8), vec3(0.5, 0.75, 0.5), feuer, 2.4, 0.0, true),
                leuchte(vec3(5.8, 1.2, 0.8), vec3(0.5, 0.75, 0.5), feuer, 2.4, 0.0, true),
                leuchte(vec3(0.0, 5.15, 1.45), Vec3::splat(0.4), akzent, 3.0, 6.0, false),
                leuchte(vec3(3.0, 13.1, -7.5), vec3(1.4, 0.3, 1.4), feuer, 2.0, 0.0, true),
            ],
            vec![vec3(3.0, 13.4, -7.5)],
        ),
    }
}

/// Eingang auf der Insel (lokal, Öffnung nach +Z): die Spinnengrotte ein bemooster Felshügel voller
/// Netze, die Gruft ein Mausoleum mit Säulenhalle, die Schmiede ein Berg aus Basalt mit Fallgitter,
/// Lava und rauchendem Schlot.
pub fn eingang_mesh(dungeon: &Dungeon) -> (MeshData, Kol) {
    // Draußen in der Sonne wirken die Farben viel heller als drinnen: dunkler ansetzen
    let stein = match dungeon.thema {
        Thema::Grotte => vec3(0.17, 0.16, 0.14),
        Thema::Gruft => vec3(0.3, 0.3, 0.31),
        Thema::Schmiede => vec3(0.034, 0.03, 0.03),
    };
    let mut m = MeshData::default();
    let mut kol: Kol = (Vec::new(), Vec::new());
    let mut rng = Rng::new(dungeon.name.len() as u64 * 991);
    let knochen = vec3(0.9, 0.87, 0.78);
    match dungeon.thema {
        Thema::Grotte => {
            let moos = vec3(0.12, 0.22, 0.06);
            schacht(&mut m, &mut kol, 0.8, -4.6, 0.0, 4.2, stein);
            let felsen = [
                (vec3(0.0, 1.0, -8.0), vec3(15.0, 13.0, 11.0), 0.3),
                (vec3(-6.6, 0.4, -4.4), vec3(8.0, 8.5, 8.0), 1.1),
                (vec3(6.8, 0.2, -4.0), vec3(8.5, 7.5, 7.5), 2.0),
                (vec3(-4.1, 1.2, -0.2), vec3(3.4, 7.0, 3.8), 0.7),
                (vec3(4.1, 1.0, -0.1), vec3(3.4, 6.6, 3.6), 2.4),
                (vec3(0.0, 5.9, -0.9), vec3(8.2, 2.8, 4.2), 0.0),
                (vec3(-9.6, -0.3, 0.6), vec3(3.0, 2.4, 3.0), 0.5),
                (vec3(8.9, -0.4, 1.4), vec3(2.4, 2.0, 2.2), 1.3),
                (vec3(-6.2, -0.3, 3.6), vec3(1.6, 1.2, 1.5), 0.2),
            ];
            for (k, &(mitte, groesse, dreh)) in felsen.iter().enumerate() {
                fels(&mut m, &mut kol, mitte, groesse, dreh, stein * rng.range(0.85, 1.1), moos, 0.0, k as f32 * 3.1);
            }
            // Zähne aus Tropfstein über der Öffnung
            for (x, l) in [(-1.4, 0.8), (-0.75, 1.3), (-0.1, 0.9), (0.55, 1.4), (1.25, 0.7)] {
                let zahn = MeshData::cylinder(0.24, 0.0, l, 5, stein * 0.9).flat_shaded();
                m.append(&zahn, Mat4::from_rotation_translation(Quat::from_rotation_x(std::f32::consts::PI), vec3(x, 4.55, 0.9)));
            }
            // Netze in den oberen Ecken der Öffnung und quer im Schacht
            netz(&mut m, vec3(-1.95, 4.25, 1.0), 1.7, -90.0, 0.0);
            netz(&mut m, vec3(1.95, 4.25, 1.0), 1.5, 180.0, 270.0);
            netz(&mut m, vec3(0.3, 2.4, -2.2), 1.9, 0.0, 330.0);
            // Fackeln auf Pfählen
            for s in [-1.0f32, 1.0] {
                let pfahl = MeshData::cylinder(0.1, 0.08, 2.5, 6, vec3(0.35, 0.22, 0.12));
                m.append(&pfahl, Mat4::from_translation(vec3(s * 2.9, -0.3, 1.9)));
                quader(&mut m, vec3(s * 2.9 - 0.18, 2.2, 1.72), vec3(s * 2.9 + 0.18, 2.45, 2.08), vec3(0.2, 0.15, 0.1));
            }
            // Knochen früherer Abenteurer
            for &(x, z, w) in &[(-1.4, 3.2, 0.4), (1.1, 4.0, 1.9), (2.2, 2.6, 2.7), (-2.6, 4.6, 1.1)] {
                let stab = MeshData::cylinder(0.07, 0.07, 0.8, 5, knochen);
                m.append(&stab, Mat4::from_rotation_translation(Quat::from_rotation_y(w) * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2), vec3(x + 0.4, 0.07, z)));
            }
            let schaedel = MeshData::icosphere(1, knochen).flat_shaded();
            m.append(&schaedel, Mat4::from_scale_rotation_translation(vec3(0.34, 0.3, 0.36), Quat::IDENTITY, vec3(-0.4, 0.14, 3.7)));
        }
        Thema::Gruft => {
            let hell = vec3(0.44, 0.43, 0.42);
            let dach = vec3(0.09, 0.1, 0.14);
            // Sockel mit Stufen
            block(&mut m, &mut kol, vec3(-6.2, -2.0, -9.0), vec3(6.2, 0.35, 2.6), stein * 0.95);
            block(&mut m, &mut kol, vec3(-4.2, -2.0, 2.6), vec3(4.2, 0.23, 3.2), stein * 0.9);
            block(&mut m, &mut kol, vec3(-4.2, -2.0, 3.2), vec3(4.2, 0.11, 3.8), stein * 0.85);
            // Das Grabhaus mit dem Tor
            block(&mut m, &mut kol, vec3(-4.6, 0.35, -8.0), vec3(-1.9, 6.2, -1.2), hell);
            block(&mut m, &mut kol, vec3(1.9, 0.35, -8.0), vec3(4.6, 6.2, -1.2), hell);
            block(&mut m, &mut kol, vec3(-1.9, 4.6, -8.0), vec3(1.9, 6.2, -1.2), hell);
            schacht(&mut m, &mut kol, -1.2, -6.0, 0.35, 4.2, hell);
            // Türrahmen und Eckpfeiler
            quader(&mut m, vec3(-2.3, 0.35, -1.25), vec3(-1.9, 4.9, -1.0), hell * 0.8);
            quader(&mut m, vec3(1.9, 0.35, -1.25), vec3(2.3, 4.9, -1.0), hell * 0.8);
            quader(&mut m, vec3(-2.3, 4.55, -1.25), vec3(2.3, 4.95, -1.0), hell * 0.8);
            for s in [-1.0f32, 1.0] {
                quader(&mut m, vec3(s * 4.6 - 0.35, 0.35, -8.15), vec3(s * 4.6 + 0.35, 6.2, -1.05), hell * 0.88);
            }
            // Säulenhalle: vier Säulen, Gebälk, Giebel, Satteldach
            for x in [-4.3f32, -2.6, 2.6, 4.3] {
                block(&mut m, &mut kol, vec3(x - 0.5, 0.35, 1.0), vec3(x + 0.5, 0.65, 2.0), hell * 0.9);
                let saeule = MeshData::cylinder(0.38, 0.32, 4.65, 10, hell * 1.05);
                m.append(&saeule, Mat4::from_translation(vec3(x, 0.65, 1.5)));
                kollision_quader(&mut kol.0, &mut kol.1, vec3(x - 0.36, 0.65, 1.14), vec3(x + 0.36, 5.3, 1.86));
                quader(&mut m, vec3(x - 0.5, 5.3, 1.0), vec3(x + 0.5, 5.6, 2.0), hell * 0.9);
            }
            quader(&mut m, vec3(-5.1, 5.6, -1.2), vec3(5.1, 6.3, 2.2), hell * 0.95);
            let (vorn, hinten) = (2.2f32, -8.3f32);
            dreieck(&mut m, vec3(-5.1, 6.3, vorn), vec3(5.1, 6.3, vorn), vec3(0.0, 8.3, vorn), hell * 0.92);
            dreieck(&mut m, vec3(-4.7, 6.2, hinten), vec3(4.7, 6.2, hinten), vec3(0.0, 8.3, hinten), hell * 0.85);
            for s in [-1.0f32, 1.0] {
                let (a, b) = (vec3(s * 5.5, 6.15, vorn + 0.25), vec3(0.0, 8.55, vorn + 0.25));
                let (c, d) = (vec3(0.0, 8.55, hinten - 0.25), vec3(s * 5.5, 6.15, hinten - 0.25));
                dreieck(&mut m, a, b, c, dach);
                dreieck(&mut m, a, c, d, dach);
            }
            quader(&mut m, vec3(-0.18, 8.3, hinten - 0.25), vec3(0.18, 8.75, vorn + 0.25), dach * 0.8);
            // Totenschädel im Giebel
            let schaedel = MeshData::icosphere(2, knochen);
            m.append(&schaedel, Mat4::from_scale_rotation_translation(vec3(0.75, 0.8, 0.6), Quat::IDENTITY, vec3(0.0, 7.1, 2.2)));
            quader(&mut m, vec3(-0.22, 6.55, 2.15), vec3(0.22, 6.85, 2.45), knochen * 0.95);
            // Sockel für die Geisterfeuer, Grabsteine davor
            for s in [-1.0f32, 1.0] {
                block(&mut m, &mut kol, vec3(s * 5.4 - 0.45, 0.35, 1.75), vec3(s * 5.4 + 0.45, 1.5, 2.65), hell * 0.8);
                let schale = MeshData::cylinder(0.3, 0.55, 0.35, 10, vec3(0.16, 0.16, 0.18));
                m.append(&schale, Mat4::from_translation(vec3(s * 5.4, 1.5, 2.2)));
            }
            for k in 0..8 {
                let s = if k % 2 == 0 { -1.0 } else { 1.0 };
                let (x, z) = (s * rng.range(7.5, 11.0), rng.range(-2.0, 9.0));
                let (b, h) = (rng.range(0.7, 1.0), rng.range(0.9, 1.4));
                let farbe = stein * rng.range(0.75, 1.05);
                block(&mut m, &mut kol, vec3(x - b * 0.5, -1.0, z - 0.14), vec3(x + b * 0.5, h, z + 0.14), farbe);
                let kappe = MeshData::cylinder(b * 0.5, b * 0.5, 0.28, 10, farbe);
                m.append(&kappe, Mat4::from_rotation_translation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2), vec3(x, h, z - 0.14)));
            }
        }
        Thema::Schmiede => {
            let asche = vec3(0.075, 0.066, 0.062);
            let eisen = vec3(0.04, 0.04, 0.045);
            schacht(&mut m, &mut kol, 0.6, -4.6, 0.0, 4.2, stein * 1.4);
            let felsen = [
                (vec3(0.0, 1.5, -8.5), vec3(17.0, 15.0, 12.0), 0.2),
                (vec3(-7.6, 0.5, -4.5), vec3(9.0, 9.0, 8.5), 1.3),
                (vec3(7.6, 0.4, -4.3), vec3(9.0, 8.4, 8.5), 2.2),
                (vec3(-4.3, 1.0, -0.5), vec3(3.6, 7.2, 3.8), 0.4),
                (vec3(4.3, 1.0, -0.4), vec3(3.6, 7.0, 3.8), 2.6),
                (vec3(0.0, 6.5, -1.3), vec3(9.0, 3.0, 4.2), 0.0),
                (vec3(-10.4, -0.4, 1.2), vec3(3.2, 2.6, 3.0), 0.8),
                (vec3(10.0, -0.4, 0.6), vec3(2.8, 2.4, 2.6), 1.9),
            ];
            for (k, &(mitte, groesse, dreh)) in felsen.iter().enumerate() {
                fels(&mut m, &mut kol, mitte, groesse, dreh, stein * rng.range(0.85, 1.1), asche, 0.03, k as f32 * 5.3);
            }
            // Das Tor: eiserne Pfeiler mit Nieten, schwerer Sturz, Dornen, Fallgitter
            for s in [-1.0f32, 1.0] {
                block(&mut m, &mut kol, vec3(s * 2.3 - 0.45, -1.0, 0.4), vec3(s * 2.3 + 0.45, 4.7, 1.4), eisen);
                for k in 0..5 {
                    let y = 0.5 + k as f32 * 0.9;
                    quader(&mut m, vec3(s * 2.3 - 0.09, y, 1.4), vec3(s * 2.3 + 0.09, y + 0.18, 1.5), vec3(0.5, 0.45, 0.4));
                }
            }
            block(&mut m, &mut kol, vec3(-3.3, 4.7, 0.3), vec3(3.3, 5.7, 1.5), eisen * 1.2);
            for x in [-2.8f32, -1.3, 1.3, 2.8] {
                let dorn = MeshData::cylinder(0.22, 0.0, 1.1, 6, eisen).flat_shaded();
                m.append(&dorn, Mat4::from_translation(vec3(x, 5.7, 0.9)));
            }
            for k in 0..9 {
                let x = -1.6 + k as f32 * 0.4;
                quader(&mut m, vec3(x - 0.05, 3.1, 0.85), vec3(x + 0.05, 4.7, 0.95), eisen * 0.8);
                let spitze = MeshData::cylinder(0.07, 0.0, 0.25, 4, eisen * 0.8);
                m.append(&spitze, Mat4::from_rotation_translation(Quat::from_rotation_x(std::f32::consts::PI), vec3(x, 3.1, 0.9)));
            }
            quader(&mut m, vec3(-1.75, 3.6, 0.83), vec3(1.75, 3.72, 0.97), eisen * 0.8);
            // Einfassung der Lavabecken und Glutbecken an den Seiten
            for s in [-1.0f32, 1.0] {
                for k in 0..10 {
                    let w = k as f32 / 10.0 * std::f32::consts::TAU;
                    let p = vec3(s * 3.7 + w.cos() * 1.15, -0.2, 3.0 + w.sin() * 1.55);
                    fels(&mut m, &mut kol, p, Vec3::splat(rng.range(0.5, 0.75)), w, stein, asche, 0.0, k as f32 + s * 11.0);
                }
                let becken = MeshData::cylinder(0.45, 0.65, 0.9, 10, vec3(0.2, 0.2, 0.22));
                m.append(&becken, Mat4::from_translation(vec3(s * 5.8, 0.0, 0.8)));
            }
            // Schlot oben auf dem Berg
            let schlot = MeshData::cylinder(1.25, 0.95, 5.4, 10, vec3(0.16, 0.14, 0.13));
            m.append(&schlot, Mat4::from_translation(vec3(3.0, 7.8, -7.5)));
            let rand = MeshData::cylinder(1.15, 1.15, 0.4, 10, eisen);
            m.append(&rand, Mat4::from_translation(vec3(3.0, 12.9, -7.5)));
        }
    }
    (m, kol)
}

/// Welt-Transformation des Eingangs.
pub fn eingang_transform(dungeon: &Dungeon) -> Mat4 {
    Mat4::from_rotation_translation(Quat::from_rotation_y(dungeon.eingang_yaw), dungeon.eingang)
}

/// Kollision aller Ebenen und Eingänge (auf allen Rechnern).
pub fn kollision(ctx: &mut Context, dungeons: &[Dungeon]) {
    for d in dungeons {
        for e in 0..d.ebenen.len() {
            let (_, (v, t)) = ebene_mesh(d, e);
            ctx.physics.add_static_mesh(None, v, t);
        }
        let (_, (v, t)) = eingang_mesh(d);
        let tr = eingang_transform(d);
        ctx.physics.add_static_mesh(None, v.into_iter().map(|p| tr.transform_point3(p)).collect(), t);
    }
}

/// Eingänge auf der Insel suchen: ebener Boden an Land in drei Himmelsrichtungen, weit weg von
/// Straßen, Siedlungen, Lagern und dem Start. Die Öffnung zeigt zur Inselmitte hin.
pub fn eingaenge_suchen(terrain: &Terrain, meiden: &[(Vec2, f32)], strassen: &[Vec<Vec2>]) -> Vec<(Vec3, f32)> {
    let mut rng = Rng::new(0xD_E1_6A);
    let mut liste: Vec<(Vec3, f32)> = Vec::new();
    let radius = crate::island::ISLAND_RADIUS;
    for sektor in 0..3 {
        let mut bester: Option<(Vec3, f32, f32)> = None;
        for _ in 0..1500 {
            let w = (sektor as f32 + rng.range(0.1, 0.9)) / 3.0 * std::f32::consts::TAU;
            let r = rng.range(0.35, 0.72) * radius;
            let p = vec2(w.cos(), w.sin()) * r;
            let h = terrain.height_at(p.x, p.y);
            if !(4.0..28.0).contains(&h) {
                continue;
            }
            // eben genug für den Eingang
            let steil = (0..8).map(|k| {
                let q = p + Vec2::from_angle(k as f32 * 0.785) * 5.0;
                (terrain.height_at(q.x, q.y) - h).abs()
            });
            let steil = steil.fold(0.0f32, f32::max);
            if steil > 1.4 {
                continue;
            }
            if meiden.iter().any(|&(m, abstand)| m.distance(p) < abstand) {
                continue;
            }
            if strassen.iter().flatten().any(|s| s.distance(p) < 40.0) {
                continue;
            }
            if liste.iter().any(|(q, _)| vec2(q.x, q.z).distance(p) < 250.0) {
                continue;
            }
            let guete = steil + (r / radius - 0.55).abs() * 2.0;
            if bester.is_none_or(|b| guete < b.2) {
                let zur_mitte = -p.normalize_or(Vec2::X);
                bester = Some((vec3(p.x, h, p.y), zur_mitte.x.atan2(zur_mitte.y), guete));
            }
        }
        if let Some((p, yaw, _)) = bester {
            liste.push((p, yaw));
        }
    }
    liste
}

// ---------------------------------------------------------------------------
// Ansicht (nur mit Fenster): Meshes der Ebenen, Fackeln, dunkles Licht drinnen
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct DungeonAnsicht {
    ebenen: Vec<Vec<EntityId>>,
    flammen: Vec<Vec<(EntityId, Vec3)>>,
    /// Was an den Eingängen leuchtet (Objekt, Ort in der Welt, Leuchte) und wo Rauch aufsteigt
    aussen: Vec<(EntityId, Vec3, Leuchte)>,
    rauch: Vec<Vec3>,
    rauch_uhr: f32,
    zeit: f32,
    fertig: bool,
}

impl DungeonAnsicht {
    fn aufbauen(&mut self, ctx: &mut Context, dungeons: &[Dungeon]) {
        self.fertig = true;
        let glut = ctx.assets.sphere();
        for d in dungeons {
            let (_, _, _, feuer) = d.thema.farben();
            for (e, ebene) in d.ebenen.iter().enumerate() {
                let (mesh, _) = ebene_mesh(d, e);
                let id = ctx.assets.add_mesh(mesh);
                let mut entity = Entity::new(format!("{} Ebene {}", d.name, e + 1), id);
                entity.visible = false;
                self.ebenen.push(vec![ctx.scene.spawn(entity)]);
                let flammen = ebene
                    .fackeln
                    .iter()
                    .map(|&(p, _)| {
                        let mut e = Entity::new("Fackel", glut)
                            .with_transform(Transform::from_position(p).with_scale(vec3(0.22, 0.4, 0.22)))
                            .with_color(feuer.extend(1.0))
                            .with_material(Material::Glow { strength: 2.4, soft: 1.0 });
                        e.casts_shadow = false;
                        e.visible = false;
                        (ctx.scene.spawn(e), p)
                    })
                    .collect();
                self.flammen.push(flammen);
            }
            let (mesh, _) = eingang_mesh(d);
            let id = ctx.assets.add_mesh(mesh);
            let tr = eingang_transform(d);
            let (_, rot, pos) = tr.to_scale_rotation_translation();
            ctx.scene.spawn(Entity::new(d.name, id).with_transform(Transform::from_position(pos).with_rotation(rot)));
            let _ = feuer;
            // Was am Eingang leuchtet, und der Rauch aus dem Schlot
            let (leuchten, rauch) = eingang_leuchten(d.thema);
            for l in leuchten {
                let p = tr.transform_point3(l.ort);
                let mut e = Entity::new("Leuchte", glut)
                    .with_transform(Transform::from_position(p).with_rotation(rot).with_scale(l.groesse))
                    .with_color(l.farbe.extend(1.0))
                    .with_material(Material::Glow { strength: l.staerke, soft: 1.0 });
                e.casts_shadow = false;
                self.aussen.push((ctx.scene.spawn(e), p, l));
            }
            self.rauch.extend(rauch.into_iter().map(|p| tr.transform_point3(p)));
        }
    }

    /// Einmal pro Bild: nur die Ebene zeigen, in der die Kamera ist; Fackeln flackern und
    /// leuchten; drinnen wird es dunkel. Liefert, ob die Kamera in einem Dungeon ist.
    pub fn update(&mut self, ctx: &mut Context, dungeons: &[Dungeon]) -> bool {
        if !self.fertig {
            self.aufbauen(ctx, dungeons);
        }
        self.zeit += ctx.time.delta;
        let kamera = ctx.camera.position;
        let hier = wo(dungeons, kamera);
        let mut index = 0;
        for (d, dungeon) in dungeons.iter().enumerate() {
            let (_, _, akzent, feuer) = dungeon.thema.farben();
            for e in 0..dungeon.ebenen.len() {
                let aktiv = hier == Some((d, e));
                for &id in &self.ebenen[index] {
                    if let Some(entity) = ctx.scene.try_get_mut(id) {
                        entity.visible = aktiv;
                    }
                }
                for (k, &(id, p)) in self.flammen[index].iter().enumerate() {
                    let flackern = 1.0 + 0.12 * (self.zeit * 11.0 + k as f32 * 1.7).sin() + 0.08 * (self.zeit * 23.0 + k as f32).sin();
                    if let Some(entity) = ctx.scene.try_get_mut(id) {
                        entity.visible = aktiv;
                        entity.transform.scale = vec3(0.22, 0.4, 0.22) * flackern;
                    }
                    if aktiv && p.distance(kamera) < 40.0 {
                        ctx.lights.push(PointLight { position: p + Vec3::Y * 0.2, color: feuer * 2.2 * flackern, radius: 10.0 });
                    }
                }
                if aktiv {
                    // Treppen leuchten in der Farbe des Themas
                    let ebene = &dungeon.ebenen[e];
                    for p in std::iter::once(ebene.hoch).chain(ebene.runter).chain(ebene.ausgang) {
                        ctx.lights.push(PointLight { position: p + vec3(0.0, 2.5, 0.0), color: akzent * 2.5, radius: 9.0 });
                    }
                }
                index += 1;
            }
        }
        // Die Eingänge: Flammen flackern, die nächsten Leuchten werfen Licht, der Schlot raucht
        if hier.is_none() {
            let mut nah: Vec<(f32, Vec3, Vec3, f32)> = Vec::new();
            for (k, (id, p, l)) in self.aussen.iter().enumerate() {
                let flackern = if l.flackern { 1.0 + 0.12 * (self.zeit * 10.0 + k as f32 * 2.1).sin() + 0.07 * (self.zeit * 21.0 + k as f32).sin() } else { 1.0 };
                if let Some(entity) = ctx.scene.try_get_mut(*id) {
                    entity.transform.scale = l.groesse * flackern;
                }
                let abstand = p.distance(kamera);
                if l.licht > 0.0 && abstand < 70.0 {
                    nah.push((abstand, *p + Vec3::Y * 0.4, l.farbe * 3.0 * flackern, l.licht));
                }
            }
            nah.sort_by(|a, b| a.0.total_cmp(&b.0));
            for &(_, position, color, radius) in nah.iter().take(4) {
                ctx.lights.push(PointLight { position, color, radius });
            }
            self.rauch_uhr -= ctx.time.delta;
            if self.rauch_uhr <= 0.0 {
                self.rauch_uhr = 0.2;
                for &p in &self.rauch {
                    if p.distance(kamera) < 260.0 {
                        ctx.particles.burst(Burst {
                            position: p,
                            count: 1,
                            color: vec3(0.13, 0.12, 0.12),
                            color_variation: 0.04,
                            speed: 1.2,
                            direction: vec3(0.25, 1.0, 0.1),
                            size: 0.3,
                            life: 5.0,
                            gravity: -0.35,
                            glow: 0.0,
                            grow: 3.0,
                            round: true,
                        });
                        ctx.particles.burst_glow(Burst {
                            position: p,
                            count: 2,
                            color: vec3(1.0, 0.45, 0.1),
                            color_variation: 0.15,
                            speed: 2.5,
                            direction: Vec3::Y,
                            size: 0.08,
                            life: 1.4,
                            gravity: -0.5,
                            glow: 3.0,
                            grow: 0.0,
                            round: false,
                        });
                    }
                }
            }
        }
        if let Some((d, _)) = hier {
            // Drinnen: keine Sonne, dunkles Umgebungslicht, dichter dunkler Dunst
            let (stein, _, akzent, _) = dungeons[d].thema.farben();
            let env = &mut ctx.env;
            env.sun_color *= 0.0;
            env.sky_ambient = stein * 0.07 + akzent * 0.03;
            env.ground_ambient = stein * 0.03;
            env.sky_color = stein * 0.06;
            env.zenith_color = stein * 0.04;
            env.fog_density = 0.035;
            env.sky.stars = 0.0;
            env.sky.sun_visible = 0.0;
            env.sky.moon_visible = 0.0;
            env.sky.rain = 0.0;
            env.sky.clouds = 0.0;
            env.sky.aurora = 0.0;
            env.sky.rainbow = 0.0;
            env.sky.mist = 0.0;
        }
        hier.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dungeons() -> Vec<Dungeon> {
        planen(&[(vec3(100.0, 5.0, 0.0), 0.0), (vec3(-100.0, 5.0, 0.0), 1.0), (vec3(0.0, 5.0, 100.0), 2.0)])
    }

    #[test]
    fn drei_dungeons_mit_mehreren_ebenen_und_verbundenen_raeumen() {
        let dungeons = test_dungeons();
        assert_eq!(dungeons.iter().map(|d| d.ebenen.len()).collect::<Vec<_>>(), vec![3, 4, 5]);
        for d in &dungeons {
            for (e, ebene) in d.ebenen.iter().enumerate() {
                assert!(ebene.raeume.len() >= 5, "{} Ebene {e}: nur {} Räume", d.name, ebene.raeume.len());
                // Jeder Gang verbindet zwei Räume: seine Mitte liegt frei, beide Enden in Räumen
                for &(a, b) in &ebene.gaenge {
                    let mitte = (a + b) * 0.5;
                    assert!(ebene.enthaelt(mitte));
                    assert!(ebene.raeume.iter().filter(|r| r.min.cmple(b + Vec2::splat(0.5)).all() && r.max.cmpge(a - Vec2::splat(0.5)).all()).count() >= 2);
                }
                assert_eq!(wo(&dungeons, vec3(ebene.hoch.x, BODEN_Y + 1.0, ebene.hoch.z)), Some((dungeons.iter().position(|x| x.name == d.name).unwrap(), e)));
                assert!(ebene.enthaelt(vec2(ebene.ankunft_oben().x, ebene.ankunft_oben().z)));
            }
            assert!(d.ebenen.last().unwrap().ausgang.is_some() && d.ebenen.last().unwrap().runter.is_none());
        }
        // Jede Ebene hat Hinauf, alle bis auf die letzte Hinab, die letzte einen Ausgang
        let n: usize = dungeons.iter().map(|d| 1 + d.ebenen.len() * 2).sum();
        assert_eq!(durchgaenge(&dungeons).len(), n);
    }

    #[test]
    fn jeder_raum_ausser_dem_start_hat_gegner() {
        let dungeons = test_dungeons();
        let lager = lager(&dungeons);
        let raeume: usize = dungeons.iter().flat_map(|d| &d.ebenen).map(|e| e.raeume.len() - 1).sum();
        assert_eq!(lager.len(), raeume);
        assert!(lager.iter().all(|(_, art, _)| crate::wildnis::ARTEN[*art].dungeon));
    }
}
