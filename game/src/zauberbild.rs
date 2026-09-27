//! Wie die Fähigkeiten aussehen (nur mit Fenster, reine Optik – ob etwas trifft, entscheidet der
//! Server): Geschosse aus hellem Kern, flackernder Hülle und Schweif; Einschläge mit Lichtblitz,
//! Druckwelle, Funken, Rauch und Spuren am Boden; Eis- und Felsdornen, die in Wellen aus dem Boden
//! brechen; der geworfene Hammer; arkane Ladungen um den Magier; Kamerawackeln und Treffer-Stopp.
//!
//! Leuchtendes nutzt `Material::Glow` (additiv, blendet weich aus), Feuer und Magie leuchtende
//! Partikel (`burst_glow`). Konzept: `docs/konzept_faehigkeiten.md`.

use std::collections::{HashMap, HashSet};
use std::f32::consts::{PI, TAU};

use engine::mesh::Vertex;
use engine::prelude::*;

use crate::faehigkeiten::{self, Faehigkeit};
use crate::protocol::{CharacterClass, PlayerId};
use crate::world::SoundEvent;

/// Klänge der Fähigkeiten (`sounds.rs` erzeugt sie).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Klang {
    Arkan,
    Lanze,
    ArkanTreffer,
    Feuerwurf,
    Explosion,
    Frost,
    Eisbruch,
    Schwung,
    Hammer,
    Wurf,
    Fangen,
    Beben,
    Pfeilschuss,
    Pfeiltreffer,
    /// Feuerkreis am Himmel, Geisterhammer: tiefes Dröhnen
    Himmel,
}

/// Was die Effekte von einer Spielfigur wissen müssen (jedes Bild neu, aus `World`).
#[derive(Clone, Copy, Debug)]
pub struct Figur {
    /// Mitte der Kapsel (Füße 0,9 m darunter)
    pub mitte: Vec3,
    pub facing: f32,
    /// Spitze der Waffe (Stab oben, Hammerkopf) und beide Hände, in der Welt
    pub spitze: Vec3,
    pub hand_l: Vec3,
    pub hand_r: Vec3,
    pub ladung: u8,
    pub klasse: CharacterClass,
    /// Knoten der Waffe im Figurenmodell (für den fliegenden Hammer)
    pub waffe: &'static str,
}

impl Figur {
    fn vorne(&self) -> Vec3 {
        vec3(self.facing.sin(), 0.0, -self.facing.cos())
    }
}

/// Einmal angelegte Formen.
#[derive(Clone, Copy)]
struct Formen {
    kugel: MeshId,
    wuerfel: MeshId,
    /// Flacher Ring (weich: innen und außen dunkel)
    ring: MeshId,
    /// Zylinderwand, unten hell, oben dunkel (Druckwelle)
    wand: MeshId,
    /// Runde Scheibe, Mitte hell, Rand dunkel (Glut, Leuchtfleck)
    scheibe: MeshId,
    /// Unregelmäßiger Fleck (Brandfleck, Raureif) – fest, nicht leuchtend
    fleck: MeshId,
    /// Felsdorn und Eiskristall (Höhe 1, Fuß im Ursprung)
    dorn: MeshId,
    kristall: MeshId,
    /// Zylinder entlang +Y (Strahl der Arkanlanze)
    strahl: MeshId,
    /// Pfeil entlang +Z (Mitte im Ursprung) und ein kantiger Meteor
    pfeil: MeshId,
    meteor: MeshId,
    /// Wurfdolch entlang +Z (Mitte im Ursprung) und ein Kranz aus sechs Klingen (Radius 1, in XZ)
    dolch: MeshId,
    klingen: MeshId,
}

fn formen(ctx: &mut Context) -> Formen {
    let a = &mut ctx.assets;
    Formen {
        kugel: a.sphere(),
        wuerfel: a.cube(),
        ring: a.named_mesh("zauber_ring", ring_mesh),
        wand: a.named_mesh("zauber_wand", wand_mesh),
        scheibe: a.named_mesh("zauber_scheibe", scheibe_mesh),
        fleck: a.named_mesh("zauber_fleck", fleck_mesh),
        dorn: a.named_mesh("zauber_dorn", dorn_mesh),
        kristall: a.named_mesh("zauber_kristall", kristall_mesh),
        strahl: a.named_mesh("zauber_strahl", strahl_mesh),
        pfeil: a.named_mesh("zauber_pfeil", pfeil_mesh),
        meteor: a.named_mesh("zauber_meteor", meteor_mesh),
        dolch: a.named_mesh("zauber_dolch", dolch_mesh),
        klingen: a.named_mesh("zauber_klingen", klingen_mesh),
    }
}

/// Eiskristall (für gefrorene Gegner in `heer.rs`).
pub fn kristall(ctx: &mut Context) -> MeshId {
    ctx.assets.named_mesh("zauber_kristall", kristall_mesh)
}

fn ring_mesh() -> MeshData {
    let mut m = MeshData { double_sided: true, ..Default::default() };
    let n = 64;
    for i in 0..=n {
        let w = TAU * i as f32 / n as f32;
        let d = vec3(w.cos(), 0.0, w.sin());
        for (r, hell) in [(0.7, 0.0), (0.9, 1.0), (1.0, 0.0)] {
            m.vertices.push(Vertex::new(d * r, Vec3::Y, Vec3::splat(hell)));
        }
    }
    for i in 0..n {
        let (a, b) = (i * 3, (i + 1) * 3);
        for k in 0..2 {
            m.indices.extend_from_slice(&[a + k, b + k, a + k + 1, a + k + 1, b + k, b + k + 1]);
        }
    }
    m
}

fn wand_mesh() -> MeshData {
    let mut m = MeshData { double_sided: true, ..Default::default() };
    let n = 48;
    for i in 0..=n {
        let w = TAU * i as f32 / n as f32;
        let d = vec3(w.cos(), 0.0, w.sin());
        for (y, hell) in [(0.0, 0.25), (0.15, 1.0), (1.0, 0.0)] {
            m.vertices.push(Vertex::new(d + Vec3::Y * y, d, Vec3::splat(hell)));
        }
    }
    for i in 0..n {
        let (a, b) = (i * 3, (i + 1) * 3);
        for k in 0..2 {
            m.indices.extend_from_slice(&[a + k, b + k, a + k + 1, a + k + 1, b + k, b + k + 1]);
        }
    }
    m
}

fn scheibe_mesh() -> MeshData {
    let mut m = MeshData { double_sided: true, ..Default::default() };
    let n = 40;
    m.vertices.push(Vertex::new(Vec3::ZERO, Vec3::Y, Vec3::ONE));
    for i in 0..=n {
        let w = TAU * i as f32 / n as f32;
        let d = vec3(w.cos(), 0.0, w.sin());
        m.vertices.push(Vertex::new(d * 0.55, Vec3::Y, Vec3::splat(0.7)));
        m.vertices.push(Vertex::new(d, Vec3::Y, Vec3::ZERO));
    }
    for i in 0..n {
        let (a, b) = (1 + i * 2, 1 + (i + 1) * 2);
        m.indices.extend_from_slice(&[0, b, a, a, b, a + 1, a + 1, b, b + 1]);
    }
    m
}

fn fleck_mesh() -> MeshData {
    let mut m = MeshData::default();
    let n = 28;
    m.vertices.push(Vertex::new(Vec3::ZERO, Vec3::Y, Vec3::splat(0.85)));
    for i in 0..=n {
        let w = TAU * (i % n) as f32 / n as f32;
        let r = 0.85 + 0.1 * (w * 3.0).sin() + 0.06 * (w * 7.0 + 1.3).sin();
        m.vertices.push(Vertex::new(vec3(w.cos() * r, 0.0, w.sin() * r), Vec3::Y, Vec3::ONE));
    }
    for i in 0..n {
        m.indices.extend_from_slice(&[0, 2 + i, 1 + i]);
    }
    m
}

fn dorn_mesh() -> MeshData {
    // Fünfeckiger, leicht verdrehter Felsdorn: unten dunkler, oben heller
    let mut m = MeshData::default();
    let n = 5;
    let punkt = |i: u32, y: f32, r: f32, dreh: f32| {
        let w = TAU * i as f32 / n as f32 + dreh;
        let wackel = 1.0 + 0.18 * ((i * 7 + (y * 10.0) as u32) % 3) as f32 / 2.0;
        vec3(w.cos() * r * wackel, y, w.sin() * r * wackel)
    };
    for i in 0..n {
        let (a0, a1) = (punkt(i, 0.0, 0.5, 0.0), punkt(i + 1, 0.0, 0.5, 0.0));
        let (b0, b1) = (punkt(i, 0.45, 0.33, 0.3), punkt(i + 1, 0.45, 0.33, 0.3));
        let spitze = vec3(0.04, 1.0, -0.03);
        m.push_triangle(a0, b1, a1, Vec3::splat(0.72));
        m.push_triangle(a0, b0, b1, Vec3::splat(0.8));
        m.push_triangle(b0, spitze, b1, Vec3::splat(1.0));
    }
    m
}

fn kristall_mesh() -> MeshData {
    // Sechseckiger Eiskristall: schmaler Fuß, breiteste Stelle unten, lange Spitze
    let mut m = MeshData::default();
    let n = 6;
    let punkt = |i: u32, y: f32, r: f32| {
        let w = TAU * i as f32 / n as f32;
        vec3(w.cos() * r, y, w.sin() * r)
    };
    for i in 0..n {
        let (a0, a1) = (punkt(i, 0.0, 0.32), punkt(i + 1, 0.0, 0.32));
        let (b0, b1) = (punkt(i, 0.28, 0.5), punkt(i + 1, 0.28, 0.5));
        let spitze = Vec3::Y;
        let hell = if i % 2 == 0 { 1.0 } else { 0.86 };
        m.push_triangle(a0, b1, a1, Vec3::splat(0.75 * hell));
        m.push_triangle(a0, b0, b1, Vec3::splat(0.75 * hell));
        m.push_triangle(b0, spitze, b1, Vec3::splat(hell));
    }
    m
}

fn strahl_mesh() -> MeshData {
    let mut m = MeshData { double_sided: true, ..Default::default() };
    let n = 16;
    for i in 0..=n {
        let w = TAU * i as f32 / n as f32;
        let d = vec3(w.cos(), 0.0, w.sin());
        for y in [0.0, 0.03, 0.97, 1.0] {
            let hell = if y == 0.0 || y == 1.0 { 0.0 } else { 1.0 };
            m.vertices.push(Vertex::new(d + Vec3::Y * y, d, Vec3::splat(hell)));
        }
    }
    for i in 0..n {
        let (a, b) = (i * 4, (i + 1) * 4);
        for k in 0..3 {
            m.indices.extend_from_slice(&[a + k, b + k, a + k + 1, a + k + 1, b + k, b + k + 1]);
        }
    }
    m
}

/// Pfeil (80 cm) entlang +Z: Holzschaft, Stahlspitze vorne, rot-weiße Federn hinten.
fn pfeil_mesh() -> MeshData {
    let mut m = MeshData::default();
    let holz = vec3(0.55, 0.37, 0.2);
    let stahl = vec3(0.55, 0.58, 0.63);
    let n = 5;
    let ring = |z: f32, r: f32, k: u32| {
        let w = TAU * k as f32 / n as f32;
        vec3(w.cos() * r, w.sin() * r, z)
    };
    for k in 0..n {
        let (a0, a1) = (ring(-0.4, 0.011, k), ring(-0.4, 0.011, k + 1));
        let (b0, b1) = (ring(0.34, 0.011, k), ring(0.34, 0.011, k + 1));
        m.push_triangle(a0, b1, a1, holz);
        m.push_triangle(a0, b0, b1, holz * 0.9);
        // Spitze: breite Klinge, die spitz zuläuft
        let (c0, c1) = (ring(0.33, 0.026, k), ring(0.33, 0.026, k + 1));
        m.push_triangle(c0, vec3(0.0, 0.0, 0.42), c1, stahl);
        m.push_triangle(c1, vec3(0.0, 0.0, 0.33), c0, stahl * 0.7);
    }
    // Drei Federn hinten
    for k in 0..3 {
        let w = TAU * k as f32 / 3.0;
        let d = vec3(w.cos(), w.sin(), 0.0);
        let farbe = if k == 0 { vec3(0.95, 0.92, 0.85) } else { vec3(0.75, 0.12, 0.1) };
        let (a, b, c) = (d * 0.01 + Vec3::Z * -0.38, d * 0.01 + Vec3::Z * -0.22, d * 0.045 + Vec3::Z * -0.36);
        m.push_triangle(a, b, c, farbe);
        m.push_triangle(a, c, b, farbe * 0.85);
    }
    m
}

/// Wurfdolch: rautenförmige Klinge nach +Z, kleine Parierstange, umwickelter Griff.
fn dolch_mesh() -> MeshData {
    let mut m = MeshData::default();
    let stahl = vec3(0.72, 0.76, 0.82);
    let gift = vec3(0.35, 0.75, 0.3);
    let leder = vec3(0.3, 0.2, 0.13);
    // Klinge: Querschnitt Raute (Breite X, Dicke Y), vorne spitz
    let klinge = [(0.0f32, 0.03f32), (0.08, 0.034), (0.2, 0.02), (0.28, 0.0)];
    for w in klinge.windows(2) {
        let ((z0, b0), (z1, b1)) = (w[0], w[1]);
        let d0 = b0 * 0.25;
        let d1 = b1 * 0.25;
        let ecken0 = [vec3(b0, 0.0, z0), vec3(0.0, d0, z0), vec3(-b0, 0.0, z0), vec3(0.0, -d0, z0)];
        let ecken1 = [vec3(b1, 0.0, z1), vec3(0.0, d1, z1), vec3(-b1, 0.0, z1), vec3(0.0, -d1, z1)];
        for k in 0..4 {
            let n = (k + 1) % 4;
            let farbe = if k % 2 == 0 { stahl } else { stahl * 0.8 };
            // Das Gift glänzt grün auf der Schneide
            let farbe = if z0 > 0.1 { farbe.lerp(gift, 0.35) } else { farbe };
            m.push_triangle(ecken0[k], ecken1[k], ecken1[n], farbe);
            m.push_triangle(ecken0[k], ecken1[n], ecken0[n], farbe * 0.92);
        }
    }
    // Parierstange und Griff als flache Kästchen
    let kasten = |m: &mut MeshData, mitte: Vec3, halb: Vec3, farbe: Vec3| {
        let p = |x: f32, y: f32, z: f32| mitte + vec3(x * halb.x, y * halb.y, z * halb.z);
        let seiten = [
            [p(-1., -1., 1.), p(1., -1., 1.), p(1., 1., 1.), p(-1., 1., 1.)],
            [p(1., -1., -1.), p(-1., -1., -1.), p(-1., 1., -1.), p(1., 1., -1.)],
            [p(-1., 1., -1.), p(-1., 1., 1.), p(1., 1., 1.), p(1., 1., -1.)],
            [p(-1., -1., 1.), p(-1., -1., -1.), p(1., -1., -1.), p(1., -1., 1.)],
            [p(1., -1., 1.), p(1., -1., -1.), p(1., 1., -1.), p(1., 1., 1.)],
            [p(-1., -1., -1.), p(-1., -1., 1.), p(-1., 1., 1.), p(-1., 1., -1.)],
        ];
        for (i, s) in seiten.iter().enumerate() {
            let f = farbe * (0.8 + 0.04 * i as f32);
            m.push_triangle(s[0], s[1], s[2], f);
            m.push_triangle(s[0], s[2], s[3], f);
        }
    };
    kasten(&mut m, vec3(0.0, 0.0, -0.005), vec3(0.055, 0.012, 0.008), vec3(0.62, 0.52, 0.3));
    kasten(&mut m, vec3(0.0, 0.0, -0.06), vec3(0.013, 0.013, 0.05), leder);
    kasten(&mut m, vec3(0.0, 0.0, -0.115), vec3(0.02, 0.018, 0.012), vec3(0.62, 0.52, 0.3));
    m
}

/// Kranz aus sechs sichelförmigen Klingen (flach in XZ, Radius etwa 1), innen dunkel, zur Spitze hell.
fn klingen_mesh() -> MeshData {
    let mut m = MeshData::default();
    let n = 6;
    let schritte = 8;
    for k in 0..n {
        let basis = TAU * k as f32 / n as f32;
        let punkt = |t: f32, seite: f32| {
            // Von innen (r 0,35) nach außen (r 1,0), dabei nach vorne gebogen
            let w = basis + t * 0.9;
            let r = 0.35 + 0.65 * t;
            let breite = 0.14 * (t * PI).sin().powf(0.7) * (1.0 - 0.4 * t);
            let quer = vec3(-w.sin(), 0.0, w.cos());
            vec3(w.cos() * r, 0.0, w.sin() * r) + quer * breite * seite
        };
        for s in 0..schritte {
            let (t0, t1) = (s as f32 / schritte as f32, (s + 1) as f32 / schritte as f32);
            let hell0 = Vec3::splat(0.25 + 0.75 * t0);
            let hell1 = Vec3::splat(0.25 + 0.75 * t1);
            let (a, b, c, d) = (punkt(t0, 1.0), punkt(t1, 1.0), punkt(t1, -1.0), punkt(t0, -1.0));
            let farbe = (hell0 + hell1) * 0.5;
            // beidseitig sichtbar
            m.push_triangle(a, b, c, farbe);
            m.push_triangle(a, c, d, farbe);
            m.push_triangle(a, c, b, farbe);
            m.push_triangle(a, d, c, farbe);
        }
    }
    m
}

/// Kantiger Meteor (Durchmesser etwa 1), die Farbe kommt vom Objekt.
fn meteor_mesh() -> MeshData {
    let mut m = MeshData::icosphere(1, Vec3::ONE);
    for (i, v) in m.vertices.iter_mut().enumerate() {
        let p = Vec3::from(v.position);
        let beule = 0.85 + 0.3 * (((i * 7919) % 97) as f32 / 97.0);
        v.position = (p * beule).into();
        let dunkel = 0.55 + 0.45 * (((i * 104729) % 89) as f32 / 89.0);
        v.color = [dunkel, dunkel * 0.8, dunkel * 0.7];
    }
    m
}

/// Wie ein Teil sich über seine Lebenszeit verändert.
#[derive(Clone, Copy, Debug)]
enum Art {
    /// Leuchtende Druckwelle (Zylinderwand): Radius wächst, Höhe und Helligkeit fallen
    Welle { von: f32, bis: f32, hoehe: f32 },
    /// Leuchtring (am Boden oder quer im Raum)
    Ring { von: f32, bis: f32 },
    /// Kugel, die schnell auf `groesse` wächst und verglüht (Farbe wandert zu `ende`)
    Blitz { groesse: Vec3, ende: Vec3 },
    /// Wächst aus dem Boden, bleibt, versinkt (Größe: Dicke, Länge, Dicke)
    Dorn { groesse: Vec3, wachsen: f32 },
    /// Fleck am Boden: bleibt, versinkt am Ende entlang `normal`
    Fleck { normal: Vec3 },
    /// Leuchten, das verglüht (Glut, Risse, Hiebsichel)
    Gluehen,
    /// Brocken: fliegt, dreht sich, fällt, bleibt liegen, versinkt
    Brocken { geschw: Vec3, drehen: Vec3, boden: f32, groesse: f32 },
    /// Strahl der Arkanlanze (Länge, Dicke)
    Strahl { laenge: f32, dicke: f32 },
    /// Leuchtet und blinkt immer schneller (Zünder des Explosivpfeils)
    Blinken,
    /// Steht, dann im letzten Drittel so tief in den Boden sinken
    Versinken { tiefe: f32 },
    /// Leuchtender Kreis, der sich dreht, weich ein- und ausblendet
    Drehen { tempo: f32, groesse: Vec3 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PfeilArt {
    Normal,
    /// Kritischer Treffer: goldener Schweif
    Krit,
    Explosiv,
    /// Giftdolch des Schurken (aus der linken Hand geworfen)
    Dolch,
    /// Schwerer Bolzen der Ballista: schlägt mit einer Druckwelle ein
    Bolzen,
}

/// Ein fliegender Pfeil (Pfeilschuss, Salve, Explosivpfeil).
struct Pfeil {
    spieler: PlayerId,
    art: PfeilArt,
    von: Option<Vec3>,
    pos: Vec3,
    nach: Vec3,
    hit: bool,
    alter: f32,
    ausholen: f32,
    tempo: f32,
    holz: EntityId,
    schweif: EntityId,
    glut: Option<EntityId>,
    groesse: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FallArt {
    Meteor,
    Hammer,
    Pfeil,
}

/// Etwas stürzt vom Himmel (Teile: Objekt, Versatz entlang der Fallrichtung, Größe).
struct Faller {
    teile: Vec<(EntityId, f32, Vec3)>,
    von: Vec3,
    nach: Vec3,
    alter: f32,
    dauer: f32,
    drehung: Quat,
    art: FallArt,
}

struct Teil {
    entity: EntityId,
    /// Sekunden seit dem Erscheinen (negativ: erscheint erst noch)
    alter: f32,
    dauer: f32,
    ort: Vec3,
    drehung: Quat,
    farbe: Vec3,
    art: Art,
}

struct Licht {
    ort: Vec3,
    farbe: Vec3,
    radius: f32,
    alter: f32,
    dauer: f32,
}

/// Ein fliegendes Geschoss (Arkangeschoss, Feuerball); die Arkanlanze ist ein Strahl.
struct Geschoss {
    spieler: PlayerId,
    art: Faehigkeit,
    nach: Vec3,
    hit: bool,
    alter: f32,
    ausholen: f32,
    tempo: f32,
    /// Wo es losflog (erst beim Loslassen an der Stabspitze festgelegt)
    von: Option<Vec3>,
    pos: Vec3,
    kern: EntityId,
    huelle: EntityId,
    schweif: EntityId,
    /// Größe (Helden 1), Höhe des Flugbogens (Anteil der Weite), von einem Turm geschossen
    groesse: f32,
    bogen: f32,
    turm: bool,
}

/// Ein Felsbrocken des Katapults (bei Brandgeschossen glühend).
struct Wurf {
    entity: EntityId,
    von: Vec3,
    nach: Vec3,
    alter: f32,
    dauer: f32,
    hoehe: f32,
    feuer: bool,
    groesse: f32,
    drehung: Vec3,
}

/// Der geworfene Hammer: fliegt über alle Treffer und zurück in die Hand.
struct Hammerflug {
    spieler: PlayerId,
    /// Ziel und Abpraller; danach geht es zurück
    punkte: Vec<Vec3>,
    hit: bool,
    naechster: usize,
    pos: Vec3,
    alter: f32,
    ausholen: f32,
    losgeflogen: bool,
    hammer: Option<EntityId>,
    aura: EntityId,
    schweif: EntityId,
    drehung: f32,
}

/// Etwas, das nach dem Ausholen bzw. später passiert.
#[derive(Clone, Copy, Debug)]
enum Spaeter {
    Frostnova { mitte: Vec3 },
    Beben { spieler: PlayerId, mitte: Vec3, haupt: bool, radius: f32 },
    Hieb { spieler: PlayerId, stufe: u8, richtung: Vec3 },
    Lanze { spieler: PlayerId, nach: Vec3 },
    Klang { klang: Klang, at: Vec3, laut: f32 },
    Zuendung { ort: Vec3 },
    Himmelskreis { mitte: Vec3 },
    Meteor { ort: Vec3 },
    Ahnenkreis { mitte: Vec3 },
    Ahnenfall { spieler: PlayerId, mitte: Vec3 },
    Ahnenschlag { spieler: PlayerId, mitte: Vec3 },
    Regenstart { spieler: PlayerId, mitte: Vec3 },
    Regenwelle { mitte: Vec3 },
    Klinge { spieler: PlayerId, stufe: u8, richtung: Vec3 },
    Rauchbombe { mitte: Vec3 },
    Rauchnach { mitte: Vec3 },
    Klingenkreis { mitte: Vec3 },
    Klingenwelle { mitte: Vec3 },
    Klingenschluss { spieler: PlayerId, mitte: Vec3 },
}

/// Die Figur holt aus: Magie sammelt sich an Stab und Händen.
struct Sammeln {
    spieler: PlayerId,
    art: Faehigkeit,
    stufe: u8,
    alter: f32,
    dauer: f32,
    kugel: Option<EntityId>,
}

pub struct Zauberbild {
    rng: Rng,
    formen: Option<Formen>,
    teile: Vec<Teil>,
    lichter: Vec<Licht>,
    geschosse: Vec<Geschoss>,
    haemmer: Vec<Hammerflug>,
    pfeile: Vec<Pfeil>,
    wuerfe: Vec<Wurf>,
    faller: Vec<Faller>,
    termine: Vec<(f32, Spaeter)>,
    sammeln: Vec<Sammeln>,
    /// Flammenteppiche (Mitte, Restzeit)
    flammen: Vec<(Vec3, f32)>,
    /// Kreisende Lichter der arkanen Ladungen je Magier (Kern, Schein)
    ladungen: HashMap<PlayerId, Vec<(EntityId, EntityId)>>,
    /// Kamerawackeln (0–1), fällt von selbst ab
    pub wackeln: f32,
    /// Treffer-Stopp: diese Figuren halten ihre Animation kurz an (Sekunden)
    pub stopp: Vec<(PlayerId, f32)>,
    /// Spieler, deren Hammer gerade fliegt (die Hand ist leer)
    pub hammer_weg: HashSet<PlayerId>,
    /// Einmalige Animationen, die Figuren spielen sollen (Clip, Tempo) – z. B. Fangen
    pub animationen: Vec<(PlayerId, &'static str, f32)>,
    kamera: Vec3,
}

impl Default for Zauberbild {
    fn default() -> Self {
        Zauberbild {
            rng: Rng::new(0x5A0B),
            formen: None,
            teile: Vec::new(),
            lichter: Vec::new(),
            geschosse: Vec::new(),
            haemmer: Vec::new(),
            pfeile: Vec::new(),
            wuerfe: Vec::new(),
            faller: Vec::new(),
            termine: Vec::new(),
            sammeln: Vec::new(),
            flammen: Vec::new(),
            ladungen: HashMap::new(),
            wackeln: 0.0,
            stopp: Vec::new(),
            hammer_weg: HashSet::new(),
            animationen: Vec::new(),
            kamera: Vec3::ZERO,
        }
    }
}

/// Weiche Kurven für Bewegungen: schnell los, sanft aus.
fn aus(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// Mit leichtem Überschwingen (Dornen schießen heraus und federn zurück).
fn federnd(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0) - 1.0;
    1.0 + 2.2 * t * t * t + 1.2 * t * t
}

/// Drehung, die +Y auf `richtung` legt.
fn nach_y(richtung: Vec3) -> Quat {
    Quat::from_rotation_arc(Vec3::Y, richtung.normalize_or(Vec3::Y))
}

/// Bodenhöhe: das Gelände, außer es weicht stark von `bezug` ab (Burg, Brücke) – dann `bezug`.
fn boden(terrain: &Terrain, p: Vec3, bezug: f32) -> f32 {
    let h = terrain.height_at(p.x, p.z);
    if (h - bezug).abs() > 1.2 {
        bezug
    } else {
        h
    }
}

impl Zauberbild {
    fn formen(&mut self, ctx: &mut Context) -> Formen {
        *self.formen.get_or_insert_with(|| formen(ctx))
    }

    /// Ein leuchtendes (additives) Teil.
    #[allow(clippy::too_many_arguments)]
    fn leuchten(&mut self, ctx: &mut Context, mesh: MeshId, ort: Vec3, drehung: Quat, groesse: Vec3, farbe: Vec3, staerke: f32, weich: f32) -> EntityId {
        let mut e = Entity::new("Zauber", mesh)
            .with_transform(Transform::from_position(ort).with_rotation(drehung).with_scale(groesse))
            .with_color(farbe.extend(1.0))
            .with_material(Material::Glow { strength: staerke, soft: weich });
        e.casts_shadow = false;
        ctx.scene.spawn(e)
    }

    /// Ein festes Teil (Fels, Eis, Fleck).
    fn fest(&mut self, ctx: &mut Context, mesh: MeshId, ort: Vec3, drehung: Quat, groesse: Vec3, farbe: Vec3, material: Material, schatten: bool) -> EntityId {
        let mut e = Entity::new("Zauber", mesh)
            .with_transform(Transform::from_position(ort).with_rotation(drehung).with_scale(groesse))
            .with_color(farbe.extend(1.0))
            .with_material(material);
        e.casts_shadow = schatten;
        ctx.scene.spawn(e)
    }

    fn teil(&mut self, entity: EntityId, verzoegerung: f32, dauer: f32, ort: Vec3, drehung: Quat, farbe: Vec3, art: Art) {
        self.teile.push(Teil { entity, alter: -verzoegerung, dauer, ort, drehung, farbe, art });
    }

    fn licht(&mut self, ort: Vec3, farbe: Vec3, radius: f32, dauer: f32) {
        self.lichter.push(Licht { ort, farbe, radius, alter: 0.0, dauer });
    }

    /// Kamerawackeln je nach Entfernung.
    fn erschuettern(&mut self, ort: Vec3, staerke: f32, weite: f32) {
        let nah = (1.0 - ort.distance(self.kamera) / weite).max(0.0);
        self.wackeln = (self.wackeln + staerke * nah * nah).min(1.0);
    }

    // ---------- Bausteine ----------

    /// Leuchtende Druckwelle (Wand) und Ring am Boden.
    #[allow(clippy::too_many_arguments)]
    fn druckwelle(&mut self, ctx: &mut Context, mitte: Vec3, bis: f32, dauer: f32, hoehe: f32, farbe: Vec3, staerke: f32, verzoegerung: f32) {
        let f = self.formen(ctx);
        let wand = self.leuchten(ctx, f.wand, mitte, Quat::IDENTITY, Vec3::ZERO, farbe, staerke, 0.0);
        self.teil(wand, verzoegerung, dauer, mitte, Quat::IDENTITY, farbe, Art::Welle { von: 0.4, bis, hoehe });
        let ring = self.leuchten(ctx, f.ring, mitte + Vec3::Y * 0.06, Quat::IDENTITY, Vec3::ZERO, farbe, staerke * 1.3, 0.0);
        self.teil(ring, verzoegerung, dauer * 1.15, mitte + Vec3::Y * 0.06, Quat::IDENTITY, farbe, Art::Ring { von: 0.3, bis: bis * 1.04 });
    }

    /// Kurzer Lichtblitz (Kugel).
    fn blitz(&mut self, ctx: &mut Context, ort: Vec3, groesse: f32, farbe: Vec3, ende: Vec3, dauer: f32, staerke: f32) {
        let f = self.formen(ctx);
        let e = self.leuchten(ctx, f.kugel, ort, Quat::IDENTITY, Vec3::ZERO, farbe, staerke, 1.0);
        self.teil(e, 0.0, dauer, ort, Quat::IDENTITY, farbe, Art::Blitz { groesse: Vec3::splat(groesse), ende });
    }

    /// Fleck am Boden (liegt dem Gelände an).
    #[allow(clippy::too_many_arguments)]
    fn fleck(&mut self, ctx: &mut Context, terrain: &Terrain, ort: Vec3, radius: f32, farbe: Vec3, dauer: f32, material: Material, bezug: f32) {
        let f = self.formen(ctx);
        let y = boden(terrain, ort, bezug);
        let normal = if (y - terrain.height_at(ort.x, ort.z)).abs() < 0.01 { terrain.normal_at(ort.x, ort.z) } else { Vec3::Y };
        let drehung = Quat::from_rotation_arc(Vec3::Y, normal) * Quat::from_rotation_y(self.rng.range(0.0, TAU));
        let p = vec3(ort.x, y, ort.z) + normal * 0.035;
        let e = self.fest(ctx, f.fleck, p, drehung, vec3(radius, 1.0, radius), farbe, material, false);
        self.teil(e, 0.0, dauer, p, drehung, farbe, Art::Fleck { normal });
    }

    /// Dorn (Fels oder Eis), der schräg nach außen aus dem Boden schießt.
    #[allow(clippy::too_many_arguments)]
    fn dorn(&mut self, ctx: &mut Context, fuss: Vec3, aussen: Vec3, neigung: f32, laenge: f32, dicke: f32, eis: bool, verzoegerung: f32, dauer: f32) {
        let f = self.formen(ctx);
        let achse = (Vec3::Y * neigung.cos() + aussen.with_y(0.0).normalize_or(Vec3::X) * neigung.sin()).normalize();
        let drehung = nach_y(achse) * Quat::from_rotation_y(self.rng.range(0.0, TAU));
        let (mesh, farbe, material) = if eis {
            let t = self.rng.range(0.0, 1.0);
            (f.kristall, vec3(0.32, 0.6, 0.9).lerp(vec3(0.55, 0.78, 0.95), t), Material::Emissive { glow: 0.12 })
        } else {
            let t = self.rng.range(0.8, 1.15);
            (f.dorn, vec3(0.3, 0.25, 0.21) * t, Material::Standard)
        };
        let e = self.fest(ctx, mesh, fuss, drehung, Vec3::ZERO, farbe, material, true);
        let groesse = vec3(dicke, laenge, dicke);
        self.teil(e, verzoegerung, dauer, fuss, drehung, farbe, Art::Dorn { groesse, wachsen: 0.09 });
    }

    /// Ein Felsbrocken fliegt davon.
    fn brocken(&mut self, ctx: &mut Context, ort: Vec3, geschw: Vec3, groesse: f32, boden_y: f32, farbe: Vec3) {
        let f = self.formen(ctx);
        let drehung = Quat::from_euler(glam::EulerRot::XYZ, self.rng.range(0.0, TAU), self.rng.range(0.0, TAU), 0.0);
        let form = vec3(self.rng.range(0.8, 1.3), self.rng.range(0.6, 1.0), self.rng.range(0.8, 1.2)) * groesse;
        let e = self.fest(ctx, f.wuerfel, ort, drehung, form, farbe * 0.75, Material::Standard, true);
        let drehen = vec3(self.rng.range(-9.0, 9.0), self.rng.range(-9.0, 9.0), self.rng.range(-9.0, 9.0));
        let dauer = self.rng.range(2.4, 3.2);
        self.teil(e, 0.0, dauer, ort, drehung, farbe, Art::Brocken { geschw, drehen, boden: boden_y, groesse });
    }

    fn funken(&mut self, ctx: &mut Context, ort: Vec3, anzahl: u32, farbe: Vec3, tempo: f32, groesse: f32, leben: f32, schwere: f32, staerke: f32) {
        ctx.particles.burst_glow(Burst {
            position: ort,
            count: anzahl,
            color: farbe,
            color_variation: 0.25,
            speed: tempo,
            direction: Vec3::Y * 0.4,
            size: groesse,
            life: leben,
            gravity: schwere,
            glow: staerke,
            grow: 0.0,
            round: false,
        });
    }

    fn glut(&mut self, ctx: &mut Context, ort: Vec3, anzahl: u32, farbe: Vec3, tempo: f32, groesse: f32, leben: f32, schwere: f32, wachsen: f32, richtung: Vec3) {
        ctx.particles.burst_glow(Burst {
            position: ort,
            count: anzahl,
            color: farbe,
            color_variation: 0.3,
            speed: tempo,
            direction: richtung,
            size: groesse,
            life: leben,
            gravity: schwere,
            glow: 1.6,
            grow: wachsen,
            round: true,
        });
    }

    /// Erdkrümel (kleine, dunkle Würfel, die herunterfallen).
    fn kruemel(ctx: &mut Context, ort: Vec3, anzahl: u32, tempo: f32) {
        ctx.particles.burst(Burst {
            position: ort,
            count: anzahl,
            color: vec3(0.24, 0.19, 0.14),
            color_variation: 0.25,
            speed: tempo,
            direction: Vec3::Y,
            size: 0.07,
            life: 0.9,
            gravity: 14.0,
            glow: 0.0,
            grow: 0.0,
            round: false,
        });
    }

    fn rauch(ctx: &mut Context, ort: Vec3, anzahl: u32, farbe: Vec3, tempo: f32, groesse: f32, leben: f32, richtung: Vec3) {
        ctx.particles.burst(Burst {
            position: ort,
            count: anzahl,
            color: farbe,
            color_variation: 0.12,
            speed: tempo,
            direction: richtung,
            size: groesse,
            life: leben,
            gravity: -0.35,
            glow: 0.0,
            grow: 1.4,
            round: true,
        });
    }

    // ---------- Start ----------

    /// Eine Fähigkeit wird eingesetzt (vom Server gemeldet): Ausholen, Geschoss, Einschlag …
    #[allow(clippy::too_many_arguments)]
    pub fn start(&mut self, ctx: &mut Context, figuren: &HashMap<PlayerId, Figur>, spieler: PlayerId, art: Faehigkeit, stufe: u8, origin: Vec3, target: Vec3, hit: bool, kette: &[Vec3], sounds: &mut Vec<SoundEvent>) {
        let f = self.formen(ctx);
        let ausholen = art.ausholen(stufe);
        let figur = figuren.get(&spieler).copied();
        let klang = |klang: Klang, at: Vec3, laut: f32| SoundEvent::Zauber { klang, at, laut };
        match (art, stufe) {
            (Faehigkeit::Arkangeschoss, 1) => {
                self.sammeln.push(Sammeln { spieler, art, stufe, alter: 0.0, dauer: ausholen, kugel: None });
                self.termine.push((ausholen, Spaeter::Lanze { spieler, nach: target }));
                sounds.push(SoundEvent::Cast { player: spieler });
            }
            (Faehigkeit::Arkangeschoss | Faehigkeit::Feuerball, _) => {
                let tempo = match art.form() {
                    faehigkeiten::Form::Geschoss { tempo, .. } => tempo,
                    _ => 30.0,
                };
                let feuer = art == Faehigkeit::Feuerball;
                let (kern_farbe, huelle_farbe) = if feuer { (vec3(1.0, 0.85, 0.5), vec3(1.0, 0.42, 0.1)) } else { (vec3(0.85, 0.8, 1.0), vec3(0.5, 0.32, 1.0)) };
                let kern = self.leuchten(ctx, f.kugel, origin, Quat::IDENTITY, Vec3::ZERO, kern_farbe, 2.0, 1.0);
                let huelle = self.leuchten(ctx, f.kugel, origin, Quat::IDENTITY, Vec3::ZERO, huelle_farbe, 1.4, 1.0);
                let schweif = self.leuchten(ctx, f.kugel, origin, Quat::IDENTITY, Vec3::ZERO, huelle_farbe, 1.1, 1.0);
                let bogen = if feuer { 0.03 } else { 0.0 };
                self.geschosse.push(Geschoss { spieler, art, nach: target, hit, alter: 0.0, ausholen, tempo, von: None, pos: origin, kern, huelle, schweif, groesse: 1.0, bogen, turm: false });
                self.sammeln.push(Sammeln { spieler, art, stufe, alter: 0.0, dauer: ausholen, kugel: None });
                if feuer {
                    sounds.push(klang(Klang::Feuerwurf, origin, 0.8));
                }
            }
            (Faehigkeit::Frostnova, _) => {
                self.sammeln.push(Sammeln { spieler, art, stufe, alter: 0.0, dauer: ausholen, kugel: None });
                self.termine.push((ausholen, Spaeter::Frostnova { mitte: target }));
                sounds.push(SoundEvent::Cast { player: spieler });
            }
            (Faehigkeit::Hammerschlag, _) => {
                let richtung = (target - origin).with_y(0.0).normalize_or(figur.map_or(Vec3::NEG_Z, |f| f.vorne()));
                self.termine.push((ausholen, Spaeter::Hieb { spieler, stufe, richtung }));
                sounds.push(klang(Klang::Schwung, origin, if stufe == 2 { 0.9 } else { 0.6 }));
            }
            (Faehigkeit::Wurfhammer, _) => {
                let mut punkte = vec![target];
                punkte.extend_from_slice(kette);
                let aura = self.leuchten(ctx, f.kugel, origin, Quat::IDENTITY, Vec3::ZERO, vec3(1.0, 0.75, 0.35), 0.9, 1.0);
                let schweif = self.leuchten(ctx, f.kugel, origin, Quat::IDENTITY, Vec3::ZERO, vec3(0.55, 0.75, 1.0), 0.9, 1.0);
                self.haemmer.push(Hammerflug { spieler, punkte, hit, naechster: 0, pos: origin, alter: 0.0, ausholen, losgeflogen: false, hammer: None, aura, schweif, drehung: 0.0 });
                self.sammeln.push(Sammeln { spieler, art, stufe, alter: 0.0, dauer: ausholen, kugel: None });
            }
            (Faehigkeit::Pfeilschuss | Faehigkeit::Explosivpfeil | Faehigkeit::Salve | Faehigkeit::Wurfdolche, _) => {
                let tempo = match art.form() {
                    faehigkeiten::Form::Geschoss { tempo, .. } => tempo,
                    _ => 50.0,
                };
                let pfeil_art = match (art, stufe) {
                    (Faehigkeit::Explosivpfeil, _) => PfeilArt::Explosiv,
                    (Faehigkeit::Pfeilschuss, 1) => PfeilArt::Krit,
                    (Faehigkeit::Wurfdolche, _) => PfeilArt::Dolch,
                    _ => PfeilArt::Normal,
                };
                let faecher = matches!(art, Faehigkeit::Salve | Faehigkeit::Wurfdolche);
                for (k, ziel) in std::iter::once(target).chain(if faecher { kette.iter().copied() } else { [].iter().copied() }).enumerate() {
                    // Die Dolche verlassen die Hand kurz nacheinander
                    let versatz = if pfeil_art == PfeilArt::Dolch { k as f32 * 0.035 } else { 0.0 };
                    self.pfeil_los(ctx, spieler, pfeil_art, ziel, hit, ausholen + versatz, tempo);
                }
                self.sammeln.push(Sammeln { spieler, art, stufe, alter: 0.0, dauer: ausholen, kugel: None });
                let (klang, laut) = match art {
                    Faehigkeit::Wurfdolche => (Klang::Wurf, 0.7),
                    Faehigkeit::Pfeilschuss => (Klang::Pfeilschuss, 0.6),
                    _ => (Klang::Pfeilschuss, 0.9),
                };
                self.termine.push((ausholen, Spaeter::Klang { klang, at: origin, laut }));
            }
            (Faehigkeit::Klingenhieb, _) => {
                let richtung = (target - origin).with_y(0.0).normalize_or(figur.map_or(Vec3::NEG_Z, |f| f.vorne()));
                self.termine.push((ausholen, Spaeter::Klinge { spieler, stufe, richtung }));
                sounds.push(klang(Klang::Schwung, origin, if stufe == 2 { 0.75 } else { 0.5 }));
            }
            (Faehigkeit::Rauchbombe, _) => {
                self.sammeln.push(Sammeln { spieler, art, stufe, alter: 0.0, dauer: ausholen, kugel: None });
                self.termine.push((ausholen, Spaeter::Rauchbombe { mitte: target }));
                for k in 1..=5 {
                    self.termine.push((ausholen + k as f32 * 0.35, Spaeter::Rauchnach { mitte: target }));
                }
            }
            (Faehigkeit::Schattenklingen, _) => {
                self.sammeln.push(Sammeln { spieler, art, stufe, alter: 0.0, dauer: ausholen, kugel: None });
                self.termine.push((ausholen, Spaeter::Klingenkreis { mitte: target }));
                for i in 0..faehigkeiten::KLINGEN_WELLEN {
                    self.termine.push((faehigkeiten::klingen_zeit(i) - 0.08, Spaeter::Klingenwelle { mitte: target }));
                }
                self.termine.push((faehigkeiten::klingen_schluss() - 0.05, Spaeter::Klingenschluss { spieler, mitte: target }));
                sounds.push(SoundEvent::Cast { player: spieler });
            }
            (Faehigkeit::Pfeilregen, _) => {
                self.sammeln.push(Sammeln { spieler, art, stufe, alter: 0.0, dauer: ausholen, kugel: None });
                self.termine.push((ausholen, Spaeter::Regenstart { spieler, mitte: target }));
                for i in 0..faehigkeiten::REGEN_WELLEN {
                    self.termine.push((faehigkeiten::regen_zeit(i) - 0.45, Spaeter::Regenwelle { mitte: target }));
                }
            }
            (Faehigkeit::Meteorsturm, _) => {
                self.sammeln.push(Sammeln { spieler, art, stufe, alter: 0.0, dauer: ausholen, kugel: None });
                self.termine.push((ausholen * 0.6, Spaeter::Himmelskreis { mitte: target }));
                for (i, &ort) in kette.iter().enumerate() {
                    self.termine.push((faehigkeiten::meteor_zeit(i) - faehigkeiten::METEOR_FALL, Spaeter::Meteor { ort }));
                }
                sounds.push(SoundEvent::Cast { player: spieler });
            }
            (Faehigkeit::Ahnenhammer, _) => {
                self.sammeln.push(Sammeln { spieler, art, stufe, alter: 0.0, dauer: ausholen, kugel: None });
                let fall = faehigkeiten::AHNEN_FALL;
                self.termine.push(((ausholen - fall - 0.35).max(0.0), Spaeter::Ahnenkreis { mitte: target }));
                self.termine.push((ausholen - fall, Spaeter::Ahnenfall { spieler, mitte: target }));
                self.termine.push((ausholen, Spaeter::Ahnenschlag { spieler, mitte: target }));
            }
            (Faehigkeit::Erdbeben, _) => {
                self.sammeln.push(Sammeln { spieler, art, stufe, alter: 0.0, dauer: ausholen, kugel: None });
                self.termine.push((ausholen, Spaeter::Beben { spieler, mitte: target, haupt: true, radius: 7.0 }));
                for (nach, radius, _) in faehigkeiten::NACHBEBEN {
                    self.termine.push((ausholen + nach, Spaeter::Beben { spieler, mitte: target, haupt: false, radius }));
                }
            }
        }
    }

    // ---------- Jedes Bild ----------

    pub fn update(&mut self, ctx: &mut Context, terrain: &Terrain, figuren: &HashMap<PlayerId, Figur>, sounds: &mut Vec<SoundEvent>) {
        let dt = ctx.time.delta;
        self.kamera = ctx.camera.position;
        self.wackeln = (self.wackeln - dt * 1.7).max(0.0);
        for (_, rest) in &mut self.stopp {
            *rest -= dt;
        }
        self.stopp.retain(|(_, rest)| *rest > 0.0);

        self.update_sammeln(ctx, figuren);
        self.update_geschosse(ctx, terrain, figuren, sounds);
        self.update_haemmer(ctx, terrain, figuren, sounds);
        self.update_pfeile(ctx, terrain, figuren, sounds);
        self.update_wuerfe(ctx, terrain, sounds);
        self.update_faller(ctx, terrain, sounds);
        // Was jetzt dran ist
        let mut i = 0;
        while i < self.termine.len() {
            self.termine[i].0 -= dt;
            if self.termine[i].0 <= 0.0 {
                let (_, was) = self.termine.swap_remove(i);
                self.ausloesen(ctx, terrain, figuren, was, sounds);
            } else {
                i += 1;
            }
        }
        self.update_flammen(ctx, terrain);
        self.update_ladungen(ctx, figuren);
        self.update_teile(ctx, terrain);
        for licht in &mut self.lichter {
            licht.alter += dt;
            let t = (licht.alter / licht.dauer).clamp(0.0, 1.0);
            ctx.lights.push(PointLight { position: licht.ort, color: licht.farbe * (1.0 - t) * (1.0 - t), radius: licht.radius });
        }
        self.lichter.retain(|l| l.alter < l.dauer);
    }

    fn update_sammeln(&mut self, ctx: &mut Context, figuren: &HashMap<PlayerId, Figur>) {
        let dt = ctx.time.delta;
        let f = self.formen(ctx);
        let mut sammeln = std::mem::take(&mut self.sammeln);
        for s in &mut sammeln {
            s.alter += dt;
            let Some(figur) = figuren.get(&s.spieler).copied() else { continue };
            let t = (s.alter / s.dauer).clamp(0.0, 1.0);
            let fuesse = figur.mitte - Vec3::Y * 0.9;
            match s.art {
                Faehigkeit::Arkangeschoss | Faehigkeit::Feuerball => {
                    let feuer = s.art == Faehigkeit::Feuerball;
                    let lanze = s.stufe == 1;
                    // Feuer formt sich zwischen den Händen, Magie an der Stabspitze
                    let ort = if feuer { (figur.hand_l + figur.hand_r) * 0.5 + figur.vorne() * 0.15 } else { figur.spitze };
                    let farbe = if feuer { vec3(1.0, 0.5, 0.12) } else { vec3(0.55, 0.4, 1.0) };
                    let groesse = if feuer { 0.55 } else if lanze { 0.42 } else { 0.28 };
                    let kugel = *s.kugel.get_or_insert_with(|| self.leuchten(ctx, f.kugel, ort, Quat::IDENTITY, Vec3::ZERO, farbe, 1.6, 1.0));
                    let flackern = 1.0 + (s.alter * 37.0).sin() * 0.08;
                    if let Some(e) = ctx.scene.try_get_mut(kugel) {
                        e.transform.position = ort;
                        e.transform.scale = Vec3::splat(groesse * aus(t) * flackern);
                    }
                    ctx.lights.push(PointLight { position: ort, color: farbe * 3.0 * t, radius: 5.0 });
                    // Funken ziehen zur Mitte hin
                    let n = if lanze { 3 } else { 2 };
                    for _ in 0..n {
                        let d = vec3(self.rng.range(-1.0, 1.0), self.rng.range(-1.0, 1.0), self.rng.range(-1.0, 1.0)).normalize_or(Vec3::Y);
                        let weite = if lanze { 1.1 } else { 0.6 };
                        ctx.particles.burst_glow(Burst {
                            position: ort + d * weite,
                            count: 1,
                            color: farbe,
                            color_variation: 0.3,
                            speed: weite * 4.0,
                            direction: -d * 3.0,
                            size: if feuer { 0.12 } else { 0.07 },
                            life: 0.22,
                            gravity: if feuer { -3.0 } else { 0.0 },
                            glow: 2.5,
                            grow: 0.0,
                            round: true,
                        });
                    }
                }
                Faehigkeit::Frostnova => {
                    // Frost wirbelt spiralförmig um den Magier nach oben und sammelt sich am Stab
                    for k in 0..3 {
                        let w = s.alter * 9.0 + k as f32 * TAU / 3.0;
                        let r = 1.6 * (1.0 - t) + 0.3;
                        let p = fuesse + vec3(w.cos() * r, 0.2 + t * 1.6, w.sin() * r);
                        self.funken(ctx, p, 1, vec3(0.7, 0.92, 1.0), 0.4, 0.07, 0.35, 0.0, 3.0);
                    }
                    let kugel = *s.kugel.get_or_insert_with(|| self.leuchten(ctx, f.kugel, figur.spitze, Quat::IDENTITY, Vec3::ZERO, vec3(0.6, 0.88, 1.0), 2.5, 1.0));
                    if let Some(e) = ctx.scene.try_get_mut(kugel) {
                        e.transform.position = figur.spitze;
                        e.transform.scale = Vec3::splat(0.45 * aus(t));
                    }
                    ctx.lights.push(PointLight { position: figur.spitze, color: vec3(1.2, 2.2, 3.5) * t, radius: 6.0 });
                }
                Faehigkeit::Erdbeben => {
                    // Der Hammerkopf glüht auf; beim Absprung staubt es
                    let kugel = *s.kugel.get_or_insert_with(|| self.leuchten(ctx, f.kugel, figur.spitze, Quat::IDENTITY, Vec3::ZERO, vec3(1.0, 0.55, 0.2), 1.6, 1.0));
                    if let Some(e) = ctx.scene.try_get_mut(kugel) {
                        e.transform.position = figur.spitze;
                        e.transform.scale = Vec3::splat(0.5 * t * t);
                    }
                    if (s.alter - dt..s.alter).contains(&(s.dauer * 0.35)) {
                        Self::rauch(ctx, fuesse + Vec3::Y * 0.1, 8, vec3(0.55, 0.5, 0.44), 1.4, 0.2, 0.8, Vec3::Y * 0.2);
                    }
                }
                Faehigkeit::Wurfhammer => {
                    if self.rng.chance(0.5) {
                        self.funken(ctx, figur.spitze, 1, vec3(1.0, 0.85, 0.45), 0.8, 0.05, 0.3, 0.0, 3.0);
                    }
                }
                Faehigkeit::Hammerschlag | Faehigkeit::Klingenhieb | Faehigkeit::Wurfdolche => {}
                Faehigkeit::Rauchbombe => {
                    // Die Lunte zischt in der linken Hand
                    self.funken(ctx, figur.hand_l, 2, vec3(1.0, 0.6, 0.25), 1.2, 0.03, 0.25, 1.0, 3.0);
                    ctx.lights.push(PointLight { position: figur.hand_l, color: vec3(2.0, 1.1, 0.4) * t, radius: 2.5 });
                }
                Faehigkeit::Schattenklingen => {
                    // Schattenkraft sammelt sich an der Klinge, violette Funken ziehen zur Spitze
                    let farbe = vec3(0.6, 0.35, 1.0);
                    let kugel = *s.kugel.get_or_insert_with(|| self.leuchten(ctx, f.kugel, figur.spitze, Quat::IDENTITY, Vec3::ZERO, farbe, 1.8, 1.0));
                    if let Some(e) = ctx.scene.try_get_mut(kugel) {
                        e.transform.position = figur.spitze;
                        e.transform.scale = Vec3::splat(0.12 + 0.3 * t * (1.0 + (s.alter * 31.0).sin() * 0.08));
                    }
                    let aussen = figur.spitze + vec3(self.rng.range(-1.0, 1.0), self.rng.range(-0.5, 1.0), self.rng.range(-1.0, 1.0)) * 0.9;
                    self.glut(ctx, aussen, 1, farbe, 0.4, 0.06, 0.35, 0.0, 0.0, (figur.spitze - aussen) * 2.5);
                    ctx.lights.push(PointLight { position: figur.spitze, color: vec3(1.8, 1.0, 3.0) * t, radius: 4.0 });
                    let _ = fuesse;
                }
                Faehigkeit::Pfeilschuss | Faehigkeit::Salve | Faehigkeit::Explosivpfeil | Faehigkeit::Pfeilregen => {
                    // Aufgelegter Pfeil: Nocke an der Zughand, die Spitze ragt über den Bogen hinaus
                    let nocke = figur.hand_r;
                    let richtung = (figur.spitze - nocke).normalize_or(figur.vorne());
                    let pfeil = *s.kugel.get_or_insert_with(|| self.fest(ctx, f.pfeil, nocke, Quat::IDENTITY, Vec3::ONE, Vec3::ONE, Material::Standard, false));
                    if let Some(e) = ctx.scene.try_get_mut(pfeil) {
                        e.transform.position = nocke + richtung * 0.4;
                        e.transform.rotation = Quat::from_rotation_arc(Vec3::Z, richtung);
                    }
                    let spitze = nocke + richtung * 0.82;
                    match s.art {
                        Faehigkeit::Explosivpfeil => {
                            self.glut(ctx, spitze, 1, vec3(1.0, 0.4, 0.1), 0.4, 0.1, 0.25, -1.0, 0.6, Vec3::ZERO);
                            ctx.lights.push(PointLight { position: spitze, color: vec3(2.5, 1.0, 0.3) * t, radius: 3.0 });
                        }
                        Faehigkeit::Pfeilregen => {
                            self.funken(ctx, spitze, 2, vec3(1.0, 0.85, 0.45), 0.8, 0.04, 0.3, -1.0, 3.5);
                            ctx.lights.push(PointLight { position: spitze, color: vec3(2.5, 2.0, 1.0) * t, radius: 4.0 });
                        }
                        _ => {}
                    }
                }
                Faehigkeit::Meteorsturm => {
                    // Glut sammelt sich über dem Stab, Funken steigen um den Magier auf
                    let kugel = *s.kugel.get_or_insert_with(|| self.leuchten(ctx, f.kugel, figur.spitze, Quat::IDENTITY, Vec3::ZERO, vec3(1.0, 0.4, 0.08), 1.8, 1.0));
                    if let Some(e) = ctx.scene.try_get_mut(kugel) {
                        e.transform.position = figur.spitze + Vec3::Y * 0.2;
                        e.transform.scale = Vec3::splat(0.7 * aus(t) * (1.0 + 0.1 * (s.alter * 31.0).sin()));
                    }
                    for k in 0..2 {
                        let w = s.alter * 6.0 + k as f32 * PI;
                        let p = fuesse + vec3(w.cos() * 1.3, 0.1, w.sin() * 1.3);
                        self.glut(ctx, p, 1, vec3(1.0, 0.4, 0.08), 1.5, 0.14, 0.6, -5.0, 0.4, Vec3::Y);
                    }
                    ctx.lights.push(PointLight { position: figur.spitze, color: vec3(4.0, 1.6, 0.4) * t, radius: 7.0 });
                }
                Faehigkeit::Ahnenhammer => {
                    // Der Hammer glüht golden, Lichter der Ahnen steigen im Kreis auf
                    let kugel = *s.kugel.get_or_insert_with(|| self.leuchten(ctx, f.kugel, figur.spitze, Quat::IDENTITY, Vec3::ZERO, vec3(1.0, 0.8, 0.4), 1.8, 1.0));
                    if let Some(e) = ctx.scene.try_get_mut(kugel) {
                        e.transform.position = figur.spitze;
                        e.transform.scale = Vec3::splat(0.8 * aus(t));
                    }
                    for k in 0..3 {
                        let w = s.alter * 4.0 + k as f32 * TAU / 3.0;
                        let p = fuesse + vec3(w.cos() * 1.1, 0.2 + t * 1.5, w.sin() * 1.1);
                        self.funken(ctx, p, 1, vec3(1.0, 0.82, 0.45), 0.5, 0.06, 0.5, -1.5, 3.0);
                    }
                    ctx.lights.push(PointLight { position: figur.spitze, color: vec3(4.0, 3.2, 1.6) * t, radius: 8.0 });
                }
            }
        }
        for s in &sammeln {
            if s.alter >= s.dauer {
                if let Some(k) = s.kugel {
                    ctx.scene.despawn(k);
                }
            }
        }
        sammeln.retain(|s| s.alter < s.dauer);
        self.sammeln.append(&mut sammeln);
    }

    fn update_geschosse(&mut self, ctx: &mut Context, terrain: &Terrain, figuren: &HashMap<PlayerId, Figur>, sounds: &mut Vec<SoundEvent>) {
        let dt = ctx.time.delta;
        let mut geschosse = std::mem::take(&mut self.geschosse);
        let mut fertig = Vec::new();
        for (index, g) in geschosse.iter_mut().enumerate() {
            g.alter += dt;
            let feuer = g.art == Faehigkeit::Feuerball;
            if g.alter < g.ausholen {
                continue;
            }
            // Losfliegen an der Stab- bzw. Handspitze
            let von = *g.von.get_or_insert_with(|| {
                let start = figuren.get(&g.spieler).map_or(g.pos, |f| if feuer { (f.hand_l + f.hand_r) * 0.5 + f.vorne() * 0.2 } else { f.spitze });
                start
            });
            if g.alter - dt < g.ausholen {
                let farbe = g.art.farbe();
                self.blitz(ctx, von, if feuer { 1.0 } else { 0.6 }, farbe, farbe * 0.3, 0.18, 1.8);
                self.funken(ctx, von, 12, farbe, 3.5, 0.05, 0.3, 0.0, 3.0);
                if !feuer && !g.turm {
                    sounds.push(SoundEvent::Zauber { klang: Klang::Arkan, at: von, laut: 0.55 });
                }
            }
            let laenge = von.distance(g.nach).max(0.01);
            let flug = g.alter - g.ausholen;
            let fortschritt = (flug * g.tempo / laenge).min(1.0);
            let richtung = (g.nach - von).normalize_or(Vec3::NEG_Z);
            // Der Feuerball zieht einen leichten Bogen
            let bogen = (fortschritt * PI).sin() * laenge * g.bogen;
            let pos = von.lerp(g.nach, fortschritt) + Vec3::Y * bogen;
            let vorher = g.pos;
            g.pos = pos;
            let flackern = 1.0 + (g.alter * 43.0).sin() * 0.1 + (g.alter * 71.0).sin() * 0.06;
            let (kern, huelle, schweif) = if feuer { (0.38, 0.85, vec3(0.62, 0.62, 2.4)) } else { (0.2, 0.46, vec3(0.3, 0.3, 1.8)) };
            let (kern, huelle, schweif) = (kern * g.groesse, huelle * g.groesse, schweif * g.groesse);
            let drehung = Quat::from_rotation_arc(Vec3::Z, richtung);
            for (e, s, versatz) in [(g.kern, Vec3::splat(kern), 0.0), (g.huelle, Vec3::splat(huelle * flackern), 0.0), (g.schweif, schweif, -schweif.z * 0.45)] {
                if let Some(entity) = ctx.scene.try_get_mut(e) {
                    entity.transform.position = pos + richtung * versatz;
                    entity.transform.scale = s;
                    entity.transform.rotation = drehung;
                }
            }
            let farbe = g.art.farbe();
            ctx.lights.push(PointLight { position: pos, color: farbe * if feuer { 4.0 * flackern } else { 2.6 }, radius: if feuer { 10.0 } else { 6.0 } });
            // Schweif: Partikel entlang der zurückgelegten Strecke
            let schritte = ((pos.distance(vorher) / 0.35).ceil() as u32).clamp(1, 6);
            for k in 0..schritte {
                let p = vorher.lerp(pos, k as f32 / schritte as f32);
                if feuer {
                    self.glut(ctx, p, 2, vec3(1.0, 0.45, 0.1), 0.8, 0.26, 0.35, -2.0, 1.2, Vec3::ZERO);
                    if self.rng.chance(0.5) {
                        Self::rauch(ctx, p, 1, vec3(0.22, 0.2, 0.19), 0.4, 0.28, 1.1, Vec3::Y);
                    }
                    if self.rng.chance(0.35) {
                        self.funken(ctx, p, 1, vec3(1.0, 0.75, 0.3), 2.0, 0.05, 0.6, 5.0, 3.0);
                    }
                } else {
                    let schwer = if g.art == Faehigkeit::Wurfdolche { 3.0 } else { 0.0 };
                    self.glut(ctx, p, 1, farbe, 0.3, 0.13 * g.groesse.max(0.6), 0.3, schwer, 0.0, Vec3::ZERO);
                    if self.rng.chance(0.4) {
                        self.funken(ctx, p, 1, farbe.lerp(Vec3::ONE, 0.5), 1.2, 0.035, 0.35, 0.0, 4.0);
                    }
                }
            }
            if fortschritt >= 1.0 {
                fertig.push(index);
                for e in [g.kern, g.huelle, g.schweif] {
                    ctx.scene.despawn(e);
                }
                match g.art {
                    Faehigkeit::Feuerball => self.explosion(ctx, terrain, pos, g.groesse, !g.turm, sounds),
                    Faehigkeit::Frostnova => self.eistreffer(ctx, terrain, pos, g.groesse, sounds),
                    Faehigkeit::Wurfdolche => self.gifttreffer(ctx, terrain, pos, g.groesse, sounds),
                    _ => self.arkan_treffer(ctx, pos, richtung, g.hit, sounds),
                }
            }
        }
        for index in fertig.into_iter().rev() {
            geschosse.swap_remove(index);
        }
        geschosse.append(&mut self.geschosse);
        self.geschosse = geschosse;
    }

    fn arkan_treffer(&mut self, ctx: &mut Context, ort: Vec3, richtung: Vec3, hit: bool, sounds: &mut Vec<SoundEvent>) {
        let f = self.formen(ctx);
        let farbe = vec3(0.55, 0.4, 1.0);
        self.blitz(ctx, ort, if hit { 1.2 } else { 0.8 }, vec3(0.75, 0.65, 1.0), farbe * 0.4, 0.22, 1.8);
        // Runenring quer zur Flugbahn
        let drehung = Quat::from_rotation_arc(Vec3::Y, -richtung);
        let ring = self.leuchten(ctx, f.ring, ort, drehung, Vec3::ZERO, farbe, 2.5, 0.0);
        self.teil(ring, 0.0, 0.3, ort, drehung, farbe, Art::Ring { von: 0.15, bis: if hit { 1.3 } else { 0.8 } });
        self.funken(ctx, ort, if hit { 26 } else { 14 }, farbe, 5.0, 0.05, 0.45, 2.0, 3.5);
        self.glut(ctx, ort, 6, farbe, 1.2, 0.2, 0.35, 0.0, 1.0, Vec3::ZERO);
        self.licht(ort, farbe * 4.0, 6.0, 0.25);
        sounds.push(SoundEvent::Zauber { klang: Klang::ArkanTreffer, at: ort, laut: if hit { 0.7 } else { 0.4 } });
    }

    /// Feuerball schlägt ein: Feuerkugel, Druckwelle, Funken, Rauch, Brandfleck, Flammenteppich.
    fn explosion(&mut self, ctx: &mut Context, terrain: &Terrain, ort: Vec3, groesse: f32, teppich: bool, sounds: &mut Vec<SoundEvent>) {
        let boden_y = boden(terrain, ort, ort.y - 0.5).min(ort.y);
        let unten = vec3(ort.x, boden_y, ort.z);
        let am_boden = ort.y - boden_y < 2.5;
        self.blitz(ctx, ort, 3.4 * groesse, vec3(1.0, 0.92, 0.7), vec3(0.9, 0.2, 0.02), 0.45, 3.0);
        let f = self.formen(ctx);
        let kugel = self.leuchten(ctx, f.kugel, ort, Quat::IDENTITY, Vec3::ZERO, vec3(1.0, 0.45, 0.08), 1.6, 1.0);
        self.teil(kugel, 0.05, 0.7, ort, Quat::IDENTITY, vec3(1.0, 0.45, 0.08), Art::Blitz { groesse: Vec3::splat(5.0 * groesse), ende: vec3(0.4, 0.05, 0.0) });
        if am_boden && !teppich {
            self.druckwelle(ctx, unten, 5.8 * groesse, 0.45, 1.5, vec3(1.0, 0.5, 0.15), 2.0, 0.0);
            self.fleck(ctx, terrain, unten, 2.4 * groesse, vec3(0.05, 0.04, 0.035), 8.0, Material::Standard, boden_y);
        }
        if am_boden && teppich {
            self.druckwelle(ctx, unten, 5.8, 0.45, 1.5, vec3(1.0, 0.5, 0.15), 2.0, 0.0);
            let (radius, dauer, _) = faehigkeiten::FLAMMEN;
            self.fleck(ctx, terrain, unten, radius * 0.85, vec3(0.05, 0.04, 0.035), 10.0, Material::Standard, boden_y);
            let glut = self.leuchten(ctx, f.scheibe, unten + Vec3::Y * 0.08, Quat::IDENTITY, vec3(radius, 1.0, radius), vec3(1.0, 0.35, 0.05), 1.6, 0.0);
            self.teil(glut, 0.0, dauer + 1.0, unten + Vec3::Y * 0.08, Quat::IDENTITY, vec3(1.0, 0.35, 0.05), Art::Gluehen);
            self.flammen.push((unten, dauer));
            for _ in 0..8 {
                let d = vec3(self.rng.range(-1.0, 1.0), self.rng.range(0.6, 1.4), self.rng.range(-1.0, 1.0));
                let g = self.rng.range(0.12, 0.26);
                let v = d * self.rng.range(4.0, 8.0);
                self.brocken(ctx, unten + Vec3::Y * 0.3, v, g, boden_y, vec3(0.2, 0.17, 0.15));
            }
        }
        self.glut(ctx, ort, 70, vec3(1.0, 0.48, 0.12), 6.0, 0.38, 0.6, -1.5, 1.6, Vec3::Y * 0.3);
        self.glut(ctx, ort, 30, vec3(1.0, 0.8, 0.4), 3.0, 0.3, 0.35, 0.0, 1.0, Vec3::ZERO);
        self.funken(ctx, ort, 45, vec3(1.0, 0.72, 0.3), 11.0, 0.07, 0.9, 9.0, 4.0);
        Self::rauch(ctx, ort + Vec3::Y * 0.4, if teppich { 14 } else { 6 }, vec3(0.2, 0.19, 0.18), 1.6, 0.42 * groesse, 2.2, Vec3::Y * 0.8);
        self.licht(ort + Vec3::Y * 0.8, vec3(9.0, 4.5, 1.5) * groesse.min(1.0), 16.0 * groesse.min(1.0), 0.7);
        self.erschuettern(ort, 0.5 * groesse.min(1.0).powi(2), 28.0);
        sounds.push(SoundEvent::Zauber { klang: Klang::Explosion, at: ort, laut: groesse.min(1.0) });
    }

    fn update_flammen(&mut self, ctx: &mut Context, terrain: &Terrain) {
        let dt = ctx.time.delta;
        let (radius, _, _) = faehigkeiten::FLAMMEN;
        let flammen = std::mem::take(&mut self.flammen);
        let mut bleiben = Vec::new();
        for (mitte, rest) in flammen {
            let rest = rest - dt;
            if rest <= 0.0 {
                continue;
            }
            bleiben.push((mitte, rest));
            if mitte.distance(self.kamera) > 120.0 {
                continue;
            }
            let staerke = (rest / 1.0).min(1.0);
            let menge = (60.0 * dt * staerke).floor() as u32 + self.rng.chance((60.0 * dt * staerke).fract()) as u32;
            for _ in 0..menge {
                let w = self.rng.range(0.0, TAU);
                let r = self.rng.range(0.0, 1.0).sqrt() * radius;
                let p = vec3(mitte.x + w.cos() * r, 0.0, mitte.z + w.sin() * r);
                let p = vec3(p.x, boden(terrain, p, mitte.y) + 0.1, p.z);
                let hoch = self.rng.range(0.6, 1.2);
                self.glut(ctx, p, 1, vec3(1.0, 0.42, 0.08), 0.5, 0.22 * hoch, 0.55 * hoch, -3.5, 1.0, Vec3::Y);
                if self.rng.chance(0.15) {
                    Self::rauch(ctx, p + Vec3::Y * 0.6, 1, vec3(0.18, 0.17, 0.16), 0.5, 0.3, 1.4, Vec3::Y);
                }
            }
            let flackern = 0.8 + 0.2 * (rest * 23.0).sin();
            ctx.lights.push(PointLight { position: mitte + Vec3::Y * 0.8, color: vec3(3.0, 1.3, 0.35) * staerke * flackern, radius: 7.0 });
        }
        self.flammen = bleiben;
    }

    fn update_haemmer(&mut self, ctx: &mut Context, terrain: &Terrain, figuren: &HashMap<PlayerId, Figur>, sounds: &mut Vec<SoundEvent>) {
        let dt = ctx.time.delta;
        let mut haemmer = std::mem::take(&mut self.haemmer);
        let mut fertig = Vec::new();
        for (index, h) in haemmer.iter_mut().enumerate() {
            h.alter += dt;
            let figur = figuren.get(&h.spieler).copied();
            if h.alter < h.ausholen {
                continue;
            }
            if !h.losgeflogen {
                h.losgeflogen = true;
                h.pos = figur.map_or(h.pos, |f| f.spitze);
                self.hammer_weg.insert(h.spieler);
                // Das echte Modell der Waffe fliegt
                if let Some(f) = figur {
                    if let Some(mesh) = crate::characters::waffen_flugmesh(ctx, f.klasse, f.waffe) {
                        let e = self.fest(ctx, mesh, h.pos, Quat::IDENTITY, Vec3::ONE, Vec3::ONE, Material::Standard, true);
                        h.hammer = Some(e);
                    }
                }
                sounds.push(SoundEvent::Zauber { klang: Klang::Wurf, at: h.pos, laut: 0.8 });
            }
            // Ziel: nächster Treffer oder zurück zur Hand
            let zurueck = h.naechster >= h.punkte.len();
            let ziel = if zurueck { figur.map_or(h.pos, |f| f.hand_r) } else { h.punkte[h.naechster] };
            let tempo = if zurueck { faehigkeiten::RUECKFLUG_TEMPO } else { match Faehigkeit::Wurfhammer.form() { faehigkeiten::Form::Geschoss { tempo, .. } => tempo, _ => 28.0 } };
            let zu = ziel - h.pos;
            let schritt = tempo * dt;
            let vorher = h.pos;
            let angekommen = zu.length() <= schritt.max(0.3);
            h.pos = if angekommen { ziel } else { h.pos + zu.normalize() * schritt };
            let richtung = zu.normalize_or(Vec3::NEG_Z);
            h.drehung += dt * 22.0;
            let quer = richtung.cross(Vec3::Y).normalize_or(Vec3::X);
            let drehung = Quat::from_axis_angle(quer, h.drehung) * Quat::from_rotation_arc(Vec3::Y, richtung);
            if let Some(e) = h.hammer.and_then(|e| ctx.scene.try_get_mut(e)) {
                e.transform.position = h.pos;
                e.transform.rotation = drehung;
            }
            for (e, s, versatz) in [(h.aura, Vec3::splat(0.9), 0.0), (h.schweif, vec3(0.35, 0.35, 1.9), -0.8)] {
                if let Some(entity) = ctx.scene.try_get_mut(e) {
                    entity.transform.position = h.pos + richtung * versatz;
                    entity.transform.scale = s;
                    entity.transform.rotation = Quat::from_rotation_arc(Vec3::Z, richtung);
                }
            }
            ctx.lights.push(PointLight { position: h.pos, color: vec3(2.4, 1.9, 1.0), radius: 5.5 });
            let schritte = ((h.pos.distance(vorher) / 0.4).ceil() as u32).clamp(1, 5);
            for k in 0..schritte {
                let p = vorher.lerp(h.pos, k as f32 / schritte as f32);
                self.funken(ctx, p, 1, vec3(1.0, 0.85, 0.45), 1.0, 0.05, 0.35, 0.0, 3.5);
                if self.rng.chance(0.5) {
                    self.funken(ctx, p, 1, vec3(0.55, 0.8, 1.0), 2.0, 0.035, 0.25, 0.0, 4.0);
                }
            }
            if angekommen {
                if zurueck {
                    fertig.push(index);
                    self.blitz(ctx, h.pos, 0.7, vec3(1.0, 0.9, 0.6), vec3(0.4, 0.3, 0.1), 0.18, 2.0);
                    self.funken(ctx, h.pos, 12, vec3(1.0, 0.85, 0.45), 2.5, 0.04, 0.3, 2.0, 3.0);
                    self.animationen.push((h.spieler, "Fangen", 1.0));
                    sounds.push(SoundEvent::Zauber { klang: Klang::Fangen, at: h.pos, laut: 0.6 });
                } else {
                    // Aufprall: am Gegner (bzw. am Boden, wenn nichts getroffen wurde)
                    let getroffen = h.hit || h.naechster > 0;
                    self.blitz(ctx, h.pos, if getroffen { 1.4 } else { 0.8 }, vec3(1.0, 0.92, 0.7), vec3(0.6, 0.35, 0.1), 0.22, 2.5);
                    self.funken(ctx, h.pos, 24, vec3(1.0, 0.82, 0.4), 7.0, 0.06, 0.5, 8.0, 4.0);
                    self.funken(ctx, h.pos, 10, vec3(0.55, 0.8, 1.0), 5.0, 0.04, 0.35, 0.0, 4.0);
                    let f = self.formen(ctx);
                    let drehung = Quat::from_rotation_arc(Vec3::Y, -richtung);
                    let ring = self.leuchten(ctx, f.ring, h.pos, drehung, Vec3::ZERO, vec3(1.0, 0.8, 0.4), 2.0, 0.0);
                    self.teil(ring, 0.0, 0.28, h.pos, drehung, vec3(1.0, 0.8, 0.4), Art::Ring { von: 0.2, bis: 1.5 });
                    if !getroffen {
                        let b = boden(terrain, h.pos, h.pos.y);
                        Self::rauch(ctx, vec3(h.pos.x, b + 0.1, h.pos.z), 8, vec3(0.5, 0.46, 0.4), 1.5, 0.3, 1.0, Vec3::Y * 0.3);
                    }
                    self.licht(h.pos, vec3(4.0, 3.0, 1.5), 7.0, 0.3);
                    self.erschuettern(h.pos, 0.2, 20.0);
                    sounds.push(SoundEvent::Zauber { klang: Klang::Hammer, at: h.pos, laut: if getroffen { 0.8 } else { 0.5 } });
                    h.naechster += 1;
                }
            }
            // Sicherheitsnetz: nie ewig unterwegs
            if h.alter > 8.0 && !fertig.contains(&index) {
                fertig.push(index);
            }
        }
        fertig.sort_unstable();
        fertig.dedup();
        for index in fertig.into_iter().rev() {
            let h = haemmer.swap_remove(index);
            for e in [h.hammer, Some(h.aura), Some(h.schweif)].into_iter().flatten() {
                ctx.scene.despawn(e);
            }
            self.hammer_weg.remove(&h.spieler);
        }
        haemmer.append(&mut self.haemmer);
        self.haemmer = haemmer;
    }

    fn ausloesen(&mut self, ctx: &mut Context, terrain: &Terrain, figuren: &HashMap<PlayerId, Figur>, was: Spaeter, sounds: &mut Vec<SoundEvent>) {
        match was {
            Spaeter::Frostnova { mitte } => self.frostnova(ctx, terrain, mitte, sounds),
            Spaeter::Beben { spieler, mitte, haupt, radius } => {
                let kopf = figuren.get(&spieler).map(|f| f.spitze).filter(|k| k.distance(mitte) < 3.0).unwrap_or(mitte);
                self.beben(ctx, terrain, spieler, mitte, kopf, haupt, radius, sounds)
            }
            Spaeter::Hieb { spieler, stufe, richtung } => {
                if let Some(figur) = figuren.get(&spieler).copied() {
                    self.hieb(ctx, terrain, spieler, figur, stufe, richtung, sounds);
                }
            }
            Spaeter::Lanze { spieler, nach } => {
                let von = figuren.get(&spieler).map_or(nach, |f| f.spitze);
                self.lanze(ctx, von, nach, sounds);
            }
            Spaeter::Klang { klang, at, laut } => sounds.push(SoundEvent::Zauber { klang, at, laut }),
            Spaeter::Zuendung { ort } => self.explosion(ctx, terrain, ort, 0.75, false, sounds),
            Spaeter::Himmelskreis { mitte } => self.himmelskreis(ctx, terrain, mitte, sounds),
            Spaeter::Meteor { ort } => self.meteor(ctx, ort),
            Spaeter::Ahnenkreis { mitte } => self.ahnenkreis(ctx, terrain, mitte),
            Spaeter::Ahnenfall { spieler, mitte } => self.ahnenfall(ctx, terrain, figuren, spieler, mitte, sounds),
            Spaeter::Ahnenschlag { spieler, mitte } => self.ahnenschlag(ctx, terrain, spieler, mitte, sounds),
            Spaeter::Regenstart { spieler, mitte } => self.regenstart(ctx, terrain, figuren, spieler, mitte, sounds),
            Spaeter::Regenwelle { mitte } => self.regenwelle(ctx, terrain, mitte, sounds),
            Spaeter::Klinge { spieler, stufe, richtung } => {
                if let Some(figur) = figuren.get(&spieler).copied() {
                    self.klingenhieb(ctx, spieler, figur, stufe, richtung, sounds);
                }
            }
            Spaeter::Rauchbombe { mitte } => self.rauchbombe(ctx, terrain, mitte, sounds),
            Spaeter::Rauchnach { mitte } => {
                // Der Rauch quillt noch eine Weile nach
                let radius = match Faehigkeit::Rauchbombe.form() {
                    faehigkeiten::Form::UmSich { radius } => radius,
                    _ => 5.0,
                };
                for _ in 0..3 {
                    let w = self.rng.range(0.0, TAU);
                    let r = self.rng.range(0.2, 1.0) * radius * 0.7;
                    let p = mitte + vec3(w.cos() * r, 0.3, w.sin() * r);
                    Self::rauch(ctx, p, 3, vec3(0.6, 0.56, 0.66), 0.7, 0.26, 1.3, Vec3::Y * 0.3);
                }
            }
            Spaeter::Klingenkreis { mitte } => self.klingenkreis(ctx, terrain, mitte, sounds),
            Spaeter::Klingenwelle { mitte } => self.klingenwelle(ctx, terrain, mitte, sounds),
            Spaeter::Klingenschluss { spieler, mitte } => self.klingenschluss(ctx, terrain, spieler, mitte, sounds),
        }
    }


    // ---------- Pfeile, Meteore, Ahnenhammer, Pfeilregen ----------

    /// Ein Pfeil fliegt los (nach dem Ausholen) von der Bogenhand zum Ziel.
    #[allow(clippy::too_many_arguments)]
    fn pfeil_los(&mut self, ctx: &mut Context, spieler: PlayerId, art: PfeilArt, nach: Vec3, hit: bool, ausholen: f32, tempo: f32) {
        let f = self.formen(ctx);
        let mesh = if art == PfeilArt::Dolch { f.dolch } else { f.pfeil };
        let holz = self.fest(ctx, mesh, nach, Quat::IDENTITY, Vec3::ZERO, Vec3::ONE, Material::Standard, false);
        let (farbe, staerke) = match art {
            PfeilArt::Krit => (vec3(1.0, 0.8, 0.35), 1.4),
            PfeilArt::Explosiv => (vec3(1.0, 0.45, 0.12), 1.2),
            PfeilArt::Dolch => (vec3(0.45, 0.95, 0.35), 1.0),
            PfeilArt::Bolzen => (vec3(1.0, 0.85, 0.6), 0.8),
            PfeilArt::Normal => (vec3(0.75, 0.8, 0.7), 0.45),
        };
        let schweif = self.leuchten(ctx, f.kugel, nach, Quat::IDENTITY, Vec3::ZERO, farbe, staerke, 1.0);
        let glut = (art == PfeilArt::Explosiv).then(|| self.leuchten(ctx, f.kugel, nach, Quat::IDENTITY, Vec3::ZERO, vec3(1.0, 0.4, 0.1), 2.2, 1.0));
        self.pfeile.push(Pfeil { spieler, art, von: None, pos: nach, nach, hit, alter: 0.0, ausholen, tempo, holz, schweif, glut, groesse: 1.0 });
    }

    fn update_pfeile(&mut self, ctx: &mut Context, terrain: &Terrain, figuren: &HashMap<PlayerId, Figur>, sounds: &mut Vec<SoundEvent>) {
        let dt = ctx.time.delta;
        let mut pfeile = std::mem::take(&mut self.pfeile);
        let mut fertig = Vec::new();
        for (index, p) in pfeile.iter_mut().enumerate() {
            p.alter += dt;
            if p.alter < p.ausholen {
                continue;
            }
            let dolch = p.art == PfeilArt::Dolch;
            let von = *p.von.get_or_insert_with(|| figuren.get(&p.spieler).map_or(p.pos, |f| if dolch { f.hand_l } else { f.spitze }));
            let laenge = von.distance(p.nach).max(0.01);
            let t = ((p.alter - p.ausholen) * p.tempo / laenge).min(1.0);
            // Flacher Bogen bei weiten Schüssen
            let bogen = (t * PI).sin() * laenge * 0.025;
            let pos = von.lerp(p.nach, t) + Vec3::Y * bogen;
            let richtung = (pos - p.pos).normalize_or((p.nach - von).normalize_or(Vec3::NEG_Z));
            let vorher = p.pos;
            p.pos = pos;
            let drehung = Quat::from_rotation_arc(Vec3::Z, richtung);
            if let Some(e) = ctx.scene.try_get_mut(p.holz) {
                e.transform.position = pos;
                e.transform.rotation = drehung;
                e.transform.scale = Vec3::splat(p.groesse);
            }
            let lang = match p.art {
                PfeilArt::Normal => 1.4,
                PfeilArt::Dolch => 1.1,
                _ => 2.0,
            };
            let dicke = if dolch { 0.05 } else { 0.07 } * p.groesse;
            let lang = lang * p.groesse.sqrt();
            if let Some(e) = ctx.scene.try_get_mut(p.schweif) {
                e.transform.position = pos - richtung * lang * 0.5;
                e.transform.rotation = drehung;
                e.transform.scale = vec3(dicke, dicke, lang);
            }
            if dolch {
                // Der Dolch dreht sich um seine Längsachse, damit die Klinge im Licht blitzt
                if let Some(e) = ctx.scene.try_get_mut(p.holz) {
                    e.transform.rotation = drehung * Quat::from_rotation_z(p.alter * 18.0);
                    e.transform.scale = Vec3::splat(1.3);
                }
            }
            if let Some(e) = p.glut.and_then(|g| ctx.scene.try_get_mut(g)) {
                e.transform.position = pos + richtung * 0.4;
                e.transform.scale = Vec3::splat(0.16);
            }
            match p.art {
                PfeilArt::Krit => {
                    self.funken(ctx, pos, 1, vec3(1.0, 0.85, 0.4), 0.6, 0.04, 0.3, 0.0, 3.5);
                    ctx.lights.push(PointLight { position: pos, color: vec3(2.0, 1.6, 0.8), radius: 4.0 });
                }
                PfeilArt::Explosiv => {
                    self.glut(ctx, pos + richtung * 0.4, 1, vec3(1.0, 0.45, 0.1), 0.4, 0.12, 0.3, -1.0, 0.8, Vec3::ZERO);
                    ctx.lights.push(PointLight { position: pos, color: vec3(3.0, 1.3, 0.4), radius: 5.0 });
                }
                PfeilArt::Normal => {
                    if self.rng.chance(0.3) {
                        self.funken(ctx, vorher, 1, vec3(0.85, 0.9, 0.8), 0.2, 0.025, 0.2, 0.0, 1.5);
                    }
                }
                PfeilArt::Bolzen => {
                    self.funken(ctx, vorher, 1, vec3(1.0, 0.85, 0.6), 0.4, 0.04, 0.3, 0.0, 2.5);
                }
                PfeilArt::Dolch => {
                    // Gifttropfen hinter dem Dolch
                    if self.rng.chance(0.6) {
                        self.glut(ctx, vorher, 1, vec3(0.4, 0.95, 0.3), 0.3, 0.05, 0.35, 3.0, 0.2, Vec3::ZERO);
                    }
                    ctx.lights.push(PointLight { position: pos, color: vec3(0.6, 1.6, 0.5), radius: 2.5 });
                }
            }
            if t < 1.0 {
                continue;
            }
            fertig.push(index);
            ctx.scene.despawn(p.schweif);
            let spitze = pos + richtung * 0.4;
            match p.art {
                PfeilArt::Explosiv => {
                    // Bleibt stecken, blinkt immer schneller und explodiert
                    let tief = pos - richtung * 0.12;
                    self.teil(p.holz, 0.0, faehigkeiten::ZUENDER, tief, drehung, Vec3::ONE, Art::Fleck { normal: Vec3::ZERO });
                    if let Some(g) = p.glut {
                        self.teil(g, 0.0, faehigkeiten::ZUENDER, spitze - richtung * 0.12, Quat::IDENTITY, vec3(1.0, 0.4, 0.1), Art::Blinken);
                    }
                    self.funken(ctx, spitze, 10, vec3(1.0, 0.6, 0.25), 3.0, 0.04, 0.35, 5.0, 3.0);
                    self.termine.push((faehigkeiten::ZUENDER, Spaeter::Zuendung { ort: spitze }));
                    sounds.push(SoundEvent::Zauber { klang: Klang::Pfeiltreffer, at: spitze, laut: 0.6 });
                }
                _ => {
                    if let Some(g) = p.glut {
                        ctx.scene.despawn(g);
                    }
                    if p.hit && dolch {
                        // Giftspritzer: grüne Tropfen und eine kleine Wolke
                        ctx.scene.despawn(p.holz);
                        self.funken(ctx, spitze, 14, vec3(0.45, 1.0, 0.35), 3.0, 0.04, 0.4, 5.0, 2.5);
                        Self::rauch(ctx, spitze, 4, vec3(0.35, 0.6, 0.3), 0.6, 0.25, 0.9, Vec3::Y * 0.3);
                        self.blitz(ctx, spitze, 0.5, vec3(0.5, 1.0, 0.4), vec3(0.1, 0.3, 0.05), 0.18, 1.2);
                        sounds.push(SoundEvent::Zauber { klang: Klang::Pfeiltreffer, at: spitze, laut: 0.6 });
                    } else if p.hit && p.art == PfeilArt::Bolzen {
                        // Der Bolzen schlägt durch: Druckwelle, Splitter, Staub
                        ctx.scene.despawn(p.holz);
                        let b = boden(terrain, spitze, spitze.y);
                        let unten = vec3(spitze.x, b.min(spitze.y), spitze.z);
                        self.druckwelle(ctx, unten, 2.2 * p.groesse.sqrt(), 0.3, 0.7, vec3(1.0, 0.8, 0.55), 1.4, 0.0);
                        self.blitz(ctx, spitze, 0.9, vec3(1.0, 0.9, 0.7), vec3(0.4, 0.25, 0.1), 0.2, 1.6);
                        self.funken(ctx, spitze, 24, vec3(1.0, 0.8, 0.5), 6.0, 0.05, 0.45, 6.0, 3.0);
                        Self::kruemel(ctx, unten, 14, 4.5);
                        Self::rauch(ctx, unten + Vec3::Y * 0.2, 5, vec3(0.62, 0.56, 0.48), 1.4, 0.22, 1.0, Vec3::Y * 0.4);
                        self.erschuettern(spitze, 0.12, 16.0);
                        sounds.push(SoundEvent::Zauber { klang: Klang::Hammer, at: spitze, laut: 0.55 });
                    } else if p.hit {
                        // Im Gegner: Splitter und kurzer Blitz, der Pfeil verschwindet
                        ctx.scene.despawn(p.holz);
                        let farbe = if p.art == PfeilArt::Krit { vec3(1.0, 0.8, 0.35) } else { vec3(0.95, 0.9, 0.8) };
                        self.funken(ctx, spitze, if p.art == PfeilArt::Krit { 22 } else { 8 }, farbe, 4.0, 0.035, 0.35, 4.0, 3.0);
                        if p.art == PfeilArt::Krit {
                            self.blitz(ctx, spitze, 1.0, vec3(1.0, 0.85, 0.45), vec3(0.5, 0.3, 0.05), 0.2, 1.8);
                            let f = self.formen(ctx);
                            let quer = Quat::from_rotation_arc(Vec3::Y, -richtung);
                            let ring = self.leuchten(ctx, f.ring, spitze, quer, Vec3::ZERO, vec3(1.0, 0.8, 0.35), 2.0, 0.0);
                            self.teil(ring, 0.0, 0.25, spitze, quer, vec3(1.0, 0.8, 0.35), Art::Ring { von: 0.15, bis: 1.0 });
                        }
                        sounds.push(SoundEvent::Zauber { klang: Klang::Pfeiltreffer, at: spitze, laut: if p.art == PfeilArt::Krit { 0.9 } else { 0.6 } });
                    } else {
                        // Steckt im Boden oder Baum und zittert nach
                        let tief = pos - richtung * 0.15;
                        self.teil(p.holz, 0.0, 4.0, tief, drehung, Vec3::ONE, Art::Fleck { normal: richtung });
                        let b = boden(terrain, spitze, spitze.y);
                        if (spitze.y - b).abs() < 0.4 {
                            Self::kruemel(ctx, spitze, 3, 2.0);
                        }
                        sounds.push(SoundEvent::Zauber { klang: Klang::Pfeiltreffer, at: spitze, laut: 0.35 });
                    }
                }
            }
        }
        for index in fertig.into_iter().rev() {
            pfeile.swap_remove(index);
        }
        pfeile.append(&mut self.pfeile);
        self.pfeile = pfeile;
    }

    /// Etwas stürzt vom Himmel: Meteor, Geisterhammer, Pfeile des Pfeilregens.
    fn update_faller(&mut self, ctx: &mut Context, terrain: &Terrain, sounds: &mut Vec<SoundEvent>) {
        let dt = ctx.time.delta;
        let mut faller = std::mem::take(&mut self.faller);
        let mut fertig = Vec::new();
        for (index, f) in faller.iter_mut().enumerate() {
            f.alter += dt;
            let t = (f.alter / f.dauer).clamp(0.0, 1.0);
            // Beschleunigt fallen
            let pos = f.von.lerp(f.nach, t * t);
            let richtung = (f.nach - f.von).normalize_or(Vec3::NEG_Y);
            for &(e, versatz, groesse) in &f.teile {
                if let Some(entity) = ctx.scene.try_get_mut(e) {
                    entity.transform.position = pos + richtung * versatz;
                    entity.transform.rotation = f.drehung;
                    entity.transform.scale = groesse;
                }
            }
            match f.art {
                FallArt::Meteor => {
                    let drehen = Quat::from_scaled_axis(vec3(3.0, 1.0, 2.0) * dt);
                    f.drehung = drehen * f.drehung;
                    // Die Streifen zeigen in Flugrichtung, der Stein dreht sich
                    for &(e, versatz, _) in &f.teile[1..] {
                        if let Some(entity) = ctx.scene.try_get_mut(e) {
                            entity.transform.rotation = Quat::from_rotation_arc(Vec3::Z, richtung);
                            entity.transform.position = pos + richtung * versatz;
                        }
                    }
                    self.glut(ctx, pos, 3, vec3(1.0, 0.45, 0.1), 1.0, 0.45, 0.45, -1.0, 1.4, Vec3::ZERO);
                    if self.rng.chance(0.6) {
                        Self::rauch(ctx, pos - richtung * 0.8, 1, vec3(0.16, 0.14, 0.13), 0.5, 0.45, 1.4, Vec3::Y);
                    }
                    ctx.lights.push(PointLight { position: pos, color: vec3(5.0, 2.2, 0.6), radius: 12.0 });
                }
                FallArt::Hammer => {
                    self.funken(ctx, pos + Vec3::Y * 1.5, 2, vec3(1.0, 0.85, 0.45), 1.0, 0.08, 0.5, 0.0, 3.0);
                    ctx.lights.push(PointLight { position: pos, color: vec3(5.0, 4.0, 2.0), radius: 14.0 });
                }
                FallArt::Pfeil => {}
            }
            if t < 1.0 {
                continue;
            }
            fertig.push(index);
            match f.art {
                FallArt::Meteor => {
                    for &(e, _, _) in &f.teile {
                        ctx.scene.despawn(e);
                    }
                    self.explosion(ctx, terrain, f.nach + Vec3::Y * 0.4, 0.8, false, sounds);
                    // Glühender Krater mit Gesteinsbrocken
                    let b = boden(terrain, f.nach, f.nach.y);
                    self.risse(ctx, terrain, vec3(f.nach.x, b, f.nach.z), 5, 2.0, b);
                }
                FallArt::Hammer => {
                    // Der Hammer steckt im Boden, glüht nach und versinkt; der Schein verblasst
                    for (k, &(e, versatz, _)) in f.teile.iter().enumerate() {
                        let ort = f.nach + richtung * versatz;
                        let farbe = ctx.scene.try_get(e).map_or(Vec3::ONE, |x| x.color.truncate());
                        let art = if k == 0 { Art::Versinken { tiefe: 4.5 } } else { Art::Gluehen };
                        self.teil(e, 0.0, if k == 0 { 2.2 } else { 1.0 }, ort, f.drehung, farbe, art);
                    }
                }
                FallArt::Pfeil => {
                    for &(schweif, _, _) in &f.teile[1..] {
                        ctx.scene.despawn(schweif);
                    }
                    let (e, _, _) = f.teile[0];
                    self.teil(e, 0.0, 1.8, f.nach, f.drehung, Vec3::ONE, Art::Fleck { normal: richtung });
                    if self.rng.chance(0.5) {
                        Self::kruemel(ctx, f.nach, 2, 1.5);
                    }
                    if self.rng.chance(0.25) {
                        Self::rauch(ctx, f.nach, 1, vec3(0.4, 0.35, 0.29), 0.6, 0.14, 0.6, Vec3::Y);
                    }
                }
            }
        }
        for index in fertig.into_iter().rev() {
            faller.swap_remove(index);
        }
        faller.append(&mut self.faller);
        self.faller = faller;
    }

    /// Meteorsturm: ein Feuerkreis öffnet sich hoch über dem Ziel, am Boden glüht der Zielkreis.
    fn himmelskreis(&mut self, ctx: &mut Context, terrain: &Terrain, mitte: Vec3, sounds: &mut Vec<SoundEvent>) {
        let f = self.formen(ctx);
        let dauer = faehigkeiten::METEOR_DAUER + faehigkeiten::METEOR_FALL + 0.6;
        let oben = mitte + Vec3::Y * 22.0;
        let rot = vec3(1.0, 0.3, 0.06);
        for (r, staerke) in [(7.5, 1.6), (5.5, 1.1), (3.0, 0.8)] {
            let e = self.leuchten(ctx, f.ring, oben, Quat::IDENTITY, vec3(r, 1.0, r), rot, staerke, 0.0);
            self.teil(e, 0.0, dauer, oben, Quat::IDENTITY, rot, Art::Drehen { tempo: 0.8 / r * if r > 5.0 { 1.0 } else { -1.0 }, groesse: vec3(r, 1.0, r) });
        }
        let schein = self.leuchten(ctx, f.scheibe, oben, Quat::IDENTITY, vec3(8.5, 1.0, 8.5), vec3(0.8, 0.18, 0.04), 0.9, 0.0);
        self.teil(schein, 0.0, dauer, oben, Quat::IDENTITY, vec3(0.8, 0.18, 0.04), Art::Drehen { tempo: 0.2, groesse: vec3(8.5, 1.0, 8.5) });
        let b = boden(terrain, mitte, mitte.y);
        let unten = vec3(mitte.x, b + 0.08, mitte.z);
        let radius = match Faehigkeit::Meteorsturm.form() {
            faehigkeiten::Form::Flaeche { radius, .. } => radius,
            _ => 6.0,
        };
        let ring = self.leuchten(ctx, f.ring, unten, Quat::IDENTITY, vec3(radius, 1.0, radius), vec3(1.0, 0.35, 0.08), 1.2, 0.0);
        self.teil(ring, 0.0, dauer, unten, Quat::IDENTITY, vec3(1.0, 0.35, 0.08), Art::Drehen { tempo: -0.3, groesse: vec3(radius, 1.0, radius) });
        self.licht(oben, vec3(8.0, 2.5, 0.6), 30.0, dauer);
        self.funken(ctx, oben, 40, rot, 6.0, 0.12, 1.2, -1.0, 3.0);
        sounds.push(SoundEvent::Zauber { klang: Klang::Himmel, at: mitte, laut: 1.0 });
    }

    /// Ein Meteor stürzt schräg vom Feuerkreis auf `ort`.
    fn meteor(&mut self, ctx: &mut Context, ort: Vec3) {
        let f = self.formen(ctx);
        let schraeg = vec3(self.rng.range(-1.0, 1.0), 0.0, self.rng.range(-1.0, 1.0)).normalize_or(Vec3::X) * 7.0;
        let von = ort + Vec3::Y * 22.0 + schraeg;
        let groesse = self.rng.range(0.55, 0.8);
        let stein = self.fest(ctx, f.meteor, von, Quat::IDENTITY, Vec3::splat(groesse), vec3(1.0, 0.55, 0.25), Material::Emissive { glow: 0.9 }, false);
        let huelle = self.leuchten(ctx, f.kugel, von, Quat::IDENTITY, Vec3::ZERO, vec3(1.0, 0.45, 0.1), 1.3, 1.0);
        let schweif = self.leuchten(ctx, f.kugel, von, Quat::IDENTITY, Vec3::ZERO, vec3(1.0, 0.35, 0.06), 1.0, 1.0);
        let teile = vec![(stein, 0.0, Vec3::splat(groesse)), (huelle, 0.0, Vec3::splat(groesse * 2.4)), (schweif, -3.0, vec3(groesse * 1.8, groesse * 1.8, 7.0))];
        self.faller.push(Faller { teile, von, nach: ort, alter: 0.0, dauer: faehigkeiten::METEOR_FALL, drehung: Quat::IDENTITY, art: FallArt::Meteor });
    }

    /// Ahnenhammer: Runenkreis am Ziel und eine Lichtsäule vom Himmel.
    fn ahnenkreis(&mut self, ctx: &mut Context, terrain: &Terrain, mitte: Vec3) {
        let f = self.formen(ctx);
        let b = boden(terrain, mitte, mitte.y);
        let unten = vec3(mitte.x, b + 0.08, mitte.z);
        let gold = vec3(1.0, 0.78, 0.35);
        let dauer = 0.35 + faehigkeiten::AHNEN_FALL + 0.5;
        for (r, tempo) in [(6.0, 0.6), (4.2, -0.9), (2.2, 1.4)] {
            let e = self.leuchten(ctx, f.ring, unten, Quat::IDENTITY, vec3(r, 1.0, r), gold, 1.5, 0.0);
            self.teil(e, 0.0, dauer, unten, Quat::IDENTITY, gold, Art::Drehen { tempo, groesse: vec3(r, 1.0, r) });
        }
        // Runen im Kreis
        for k in 0..8 {
            let w = TAU * k as f32 / 8.0;
            let p = unten + vec3(w.cos(), 0.0, w.sin()) * 5.1;
            let e = self.leuchten(ctx, f.kugel, p, Quat::IDENTITY, vec3(0.35, 0.08, 0.35), gold, 2.0, 1.0);
            self.teil(e, k as f32 * 0.03, dauer, p, Quat::IDENTITY, gold, Art::Gluehen);
        }
        // Lichtsäule
        let saeule = self.leuchten(ctx, f.strahl, unten, Quat::IDENTITY, vec3(1.2, 30.0, 1.2), gold * 0.7, 0.8, 1.0);
        self.teil(saeule, 0.0, dauer, unten, Quat::IDENTITY, gold * 0.7, Art::Strahl { laenge: 30.0, dicke: 1.4 });
        self.licht(unten + Vec3::Y * 2.0, vec3(5.0, 4.0, 2.0), 14.0, dauer);
    }

    /// Der Geisterhammer der Ahnen stürzt vom Himmel.
    fn ahnenfall(&mut self, ctx: &mut Context, terrain: &Terrain, figuren: &HashMap<PlayerId, Figur>, spieler: PlayerId, mitte: Vec3, sounds: &mut Vec<SoundEvent>) {
        let figur = figuren.get(&spieler).copied();
        let b = boden(terrain, mitte, mitte.y);
        let groesse = 7.0;
        let mesh = figur.and_then(|f| crate::characters::waffen_flugmesh(ctx, f.klasse, f.waffe));
        let mut teile = Vec::new();
        let gold = vec3(1.0, 0.78, 0.35);
        // Das Mesh liegt mit dem Stiel entlang Y, der Kopf unten; im Boden steckt der Kopf zur Hälfte
        let halbe = 0.4 * groesse;
        let nach = vec3(mitte.x, b + halbe, mitte.z);
        let von = nach + Vec3::Y * 18.0;
        let drehung = Quat::from_rotation_y(figur.map_or(0.0, |f| -f.facing));
        if let Some(mesh) = mesh {
            // Fester, golden leuchtender Hammer – die Form bleibt erkennbar – und ein Schein darum
            let e = self.fest(ctx, mesh, von, drehung, Vec3::splat(groesse), vec3(1.0, 0.8, 0.4), Material::Emissive { glow: 0.9 }, true);
            teile.push((e, 0.0, Vec3::splat(groesse)));
            let aura = self.leuchten(ctx, mesh, von, drehung, Vec3::splat(groesse * 1.08), gold * 0.5, 0.8, 0.6);
            teile.push((aura, 0.0, Vec3::splat(groesse * 1.08)));
        }
        let f = self.formen(ctx);
        let schein = self.leuchten(ctx, f.kugel, von, drehung, Vec3::ZERO, gold * 0.5, 0.6, 1.0);
        teile.push((schein, 0.0, vec3(3.0, 7.0, 3.0)));
        self.faller.push(Faller { teile, von, nach, alter: 0.0, dauer: faehigkeiten::AHNEN_FALL, drehung, art: FallArt::Hammer });
        sounds.push(SoundEvent::Zauber { klang: Klang::Himmel, at: mitte, laut: 0.8 });
    }

    /// Einschlag des Geisterhammers: goldene Druckwellen, Felsdornen, Risse, Brocken.
    fn ahnenschlag(&mut self, ctx: &mut Context, terrain: &Terrain, spieler: PlayerId, mitte: Vec3, sounds: &mut Vec<SoundEvent>) {
        let b = boden(terrain, mitte, mitte.y);
        let unten = vec3(mitte.x, b, mitte.z);
        let gold = vec3(1.0, 0.78, 0.35);
        self.blitz(ctx, unten + Vec3::Y * 1.0, 4.5, vec3(1.0, 0.9, 0.6), vec3(0.6, 0.35, 0.05), 0.35, 1.8);
        self.druckwelle(ctx, unten, 9.0, 0.45, 2.6, gold, 1.5, 0.0);
        self.druckwelle(ctx, unten, 6.0, 0.35, 1.4, vec3(1.0, 0.95, 0.8), 1.0, 0.1);
        for (r, anzahl) in [(2.0, 8), (3.8, 12), (5.6, 16)] {
            let versatz = self.rng.range(0.0, TAU);
            for k in 0..anzahl {
                let w = versatz + TAU * (k as f32 + self.rng.range(-0.3, 0.3)) / anzahl as f32;
                let aussen = vec3(w.cos(), 0.0, w.sin());
                let p = unten + aussen * (r + self.rng.range(-0.3, 0.3));
                let p = vec3(p.x, boden(terrain, p, unten.y) - 0.1, p.z);
                let (laenge, dicke, neigung) = (self.rng.range(1.2, 2.2) * (1.0 - r / 9.0), self.rng.range(0.7, 1.0), self.rng.range(0.3, 0.6));
                self.dorn(ctx, p, aussen, neigung, laenge, dicke * laenge.sqrt(), false, r / 22.0, 1.8);
                Self::kruemel(ctx, p + Vec3::Y * 0.2, 3, 3.5);
            }
        }
        self.risse(ctx, terrain, unten, 10, 6.5, unten.y);
        for _ in 0..20 {
            let d = vec3(self.rng.range(-1.0, 1.0), self.rng.range(1.2, 2.2), self.rng.range(-1.0, 1.0));
            let (g, v) = (self.rng.range(0.15, 0.35), self.rng.range(4.0, 8.0));
            self.brocken(ctx, unten + Vec3::Y * 0.4, d * v, g, unten.y, vec3(0.42, 0.36, 0.3));
        }
        self.funken(ctx, unten + Vec3::Y * 0.5, 60, gold, 9.0, 0.08, 0.9, 8.0, 3.5);
        Self::kruemel(ctx, unten + Vec3::Y * 0.3, 40, 7.0);
        self.licht(unten + Vec3::Y * 2.0, vec3(9.0, 7.0, 3.5), 20.0, 0.8);
        self.erschuettern(unten, 1.0, 40.0);
        self.stopp.push((spieler, 0.1));
        sounds.push(SoundEvent::Zauber { klang: Klang::Beben, at: unten, laut: 1.0 });
        sounds.push(SoundEvent::Zauber { klang: Klang::Hammer, at: unten, laut: 1.0 });
        sounds.push(SoundEvent::Thunder { volume: 0.5 });
    }

    /// Pfeilregen: der Schuss in den Himmel und der Zielkreis am Boden.
    fn regenstart(&mut self, ctx: &mut Context, terrain: &Terrain, figuren: &HashMap<PlayerId, Figur>, spieler: PlayerId, mitte: Vec3, sounds: &mut Vec<SoundEvent>) {
        let f = self.formen(ctx);
        if let Some(figur) = figuren.get(&spieler) {
            // Ein leuchtender Pfeil steigt steil auf und zerplatzt hoch oben
            let hoch = figur.spitze.lerp(mitte, 0.4) + Vec3::Y * 20.0;
            self.pfeil_los(ctx, spieler, PfeilArt::Krit, hoch, true, 0.0, 70.0);
            self.blitz(ctx, figur.spitze, 0.8, vec3(1.0, 0.85, 0.5), vec3(0.4, 0.3, 0.1), 0.2, 1.5);
        }
        let b = boden(terrain, mitte, mitte.y);
        let unten = vec3(mitte.x, b + 0.08, mitte.z);
        let radius = match Faehigkeit::Pfeilregen.form() {
            faehigkeiten::Form::Flaeche { radius, .. } => radius,
            _ => 6.0,
        };
        let dauer = faehigkeiten::REGEN_FLUG + faehigkeiten::REGEN_WELLEN as f32 * faehigkeiten::REGEN_ABSTAND + 0.4;
        let gold = vec3(1.0, 0.82, 0.4);
        let ring = self.leuchten(ctx, f.ring, unten, Quat::IDENTITY, vec3(radius, 1.0, radius), gold, 1.2, 0.0);
        self.teil(ring, 0.0, dauer, unten, Quat::IDENTITY, gold, Art::Drehen { tempo: 0.4, groesse: vec3(radius, 1.0, radius) });
        let innen = self.leuchten(ctx, f.scheibe, unten, Quat::IDENTITY, vec3(radius, 1.0, radius), gold * 0.35, 0.6, 0.0);
        self.teil(innen, 0.0, dauer, unten, Quat::IDENTITY, gold * 0.35, Art::Drehen { tempo: 0.0, groesse: vec3(radius, 1.0, radius) });
        sounds.push(SoundEvent::Zauber { klang: Klang::Pfeilschuss, at: unten, laut: 0.9 });
    }

    /// Eine Welle des Pfeilregens: Pfeile fallen schräg vom Himmel in den Zielkreis.
    fn regenwelle(&mut self, ctx: &mut Context, terrain: &Terrain, mitte: Vec3, sounds: &mut Vec<SoundEvent>) {
        let f = self.formen(ctx);
        let radius = match Faehigkeit::Pfeilregen.form() {
            faehigkeiten::Form::Flaeche { radius, .. } => radius,
            _ => 6.0,
        };
        let schraeg = vec3(0.35, 0.0, 0.2);
        for _ in 0..9 {
            let w = self.rng.range(0.0, TAU);
            let r = self.rng.range(0.0, 1.0).sqrt() * radius;
            let p = mitte + vec3(w.cos() * r, 0.0, w.sin() * r);
            let nach = vec3(p.x, boden(terrain, p, mitte.y) + 0.05, p.z);
            let von = nach + (Vec3::Y + schraeg) * 14.0;
            let richtung = (nach - von).normalize();
            let drehung = Quat::from_rotation_arc(Vec3::Z, richtung);
            // Größer als ein echter Pfeil und mit hellem Schweif, damit man den Regen sieht
            let e = self.fest(ctx, f.pfeil, von, drehung, Vec3::splat(1.8), Vec3::ONE, Material::Standard, false);
            let schweif = self.leuchten(ctx, f.kugel, von, drehung, Vec3::ZERO, vec3(1.0, 0.9, 0.6), 0.7, 1.0);
            let dauer = 0.45 * self.rng.range(0.9, 1.1);
            let teile = vec![(e, 0.0, Vec3::splat(1.8)), (schweif, -1.4, vec3(0.08, 0.08, 2.4))];
            self.faller.push(Faller { teile, von, nach: nach - richtung * 0.45, alter: 0.0, dauer, drehung, art: FallArt::Pfeil });
        }
        sounds.push(SoundEvent::Zauber { klang: Klang::Pfeiltreffer, at: mitte, laut: 0.5 });
    }

    /// Arkanlanze: ein gleißender Strahl, Mündungsringe, Funken entlang der Bahn.
    fn lanze(&mut self, ctx: &mut Context, von: Vec3, nach: Vec3, sounds: &mut Vec<SoundEvent>) {
        let f = self.formen(ctx);
        let richtung = (nach - von).normalize_or(Vec3::NEG_Z);
        let laenge = von.distance(nach).max(0.5);
        let drehung = nach_y(richtung);
        let violett = vec3(0.55, 0.38, 1.0);
        for (dicke, farbe, staerke, weich, dauer) in [(0.13, vec3(0.9, 0.85, 1.0), 2.6, 0.0, 0.32), (0.5, violett, 1.4, 1.0, 0.55), (1.0, violett * 0.6, 0.7, 1.0, 0.4)] {
            let e = self.leuchten(ctx, f.strahl, von, drehung, Vec3::ZERO, farbe, staerke, weich);
            self.teil(e, 0.0, dauer, von, drehung, farbe, Art::Strahl { laenge, dicke });
        }
        let quer = Quat::from_rotation_arc(Vec3::Y, richtung);
        for (k, bis) in [(0.0, 0.7), (0.05, 1.05), (0.1, 1.4)] {
            let ring = self.leuchten(ctx, f.ring, von + richtung * (0.3 + k * 8.0), quer, Vec3::ZERO, violett, 2.5, 0.0);
            self.teil(ring, k, 0.3, von + richtung * (0.3 + k * 8.0), quer, violett, Art::Ring { von: 0.2, bis });
        }
        let schritte = (laenge / 1.2) as u32;
        for k in 0..schritte {
            let p = von + richtung * (k as f32 * 1.2);
            self.funken(ctx, p, 2, vec3(0.75, 0.65, 1.0), 2.5, 0.05, 0.45, 0.0, 4.0);
            if k % 3 == 0 {
                self.glut(ctx, p, 1, violett, 0.6, 0.3, 0.45, 0.0, 1.2, Vec3::ZERO);
            }
        }
        self.blitz(ctx, von, 0.9, vec3(0.75, 0.65, 1.0), violett * 0.3, 0.25, 1.8);
        self.blitz(ctx, nach, 1.5, vec3(0.75, 0.65, 1.0), violett * 0.3, 0.35, 1.8);
        self.funken(ctx, nach, 30, violett, 6.0, 0.06, 0.5, 2.0, 4.0);
        self.licht(von, violett * 6.0, 9.0, 0.35);
        self.licht(nach, violett * 5.0, 8.0, 0.4);
        self.erschuettern(von, 0.3, 18.0);
        sounds.push(SoundEvent::Zauber { klang: Klang::Lanze, at: von, laut: 1.0 });
    }

    /// Frostnova: Eiswelle mit Druckwelle, Eisdornen in Ringen, Raureif, Frostnebel.
    fn frostnova(&mut self, ctx: &mut Context, terrain: &Terrain, mitte: Vec3, sounds: &mut Vec<SoundEvent>) {
        let radius = match Faehigkeit::Frostnova.form() {
            faehigkeiten::Form::UmSich { radius } => radius,
            _ => 7.5,
        };
        let tempo = faehigkeiten::EISWELLE_TEMPO;
        let eis = vec3(0.6, 0.88, 1.0);
        self.blitz(ctx, mitte + Vec3::Y * 0.6, 2.0, vec3(0.6, 0.8, 1.0), eis * 0.2, 0.3, 1.6);
        self.druckwelle(ctx, mitte, radius, radius / tempo, 1.3, vec3(0.4, 0.7, 1.0), 1.3, 0.0);
        // Eisdornen brechen in Ringen aus dem Boden, wenn die Welle sie erreicht
        for (r, anzahl) in [(1.4, 5), (2.7, 9), (4.0, 12), (5.3, 15), (6.5, 18), (7.3, 20)] {
            let r = r * radius / 7.5;
            let versatz = self.rng.range(0.0, TAU);
            for k in 0..anzahl {
                let w = versatz + TAU * (k as f32 + self.rng.range(-0.3, 0.3)) / anzahl as f32;
                let aussen = vec3(w.cos(), 0.0, w.sin());
                let p = mitte + aussen * (r + self.rng.range(-0.3, 0.3));
                let p = vec3(p.x, boden(terrain, p, mitte.y) - 0.05, p.z);
                let laenge = self.rng.range(0.55, 1.25) * (1.0 - r / radius * 0.35);
                let neigung = self.rng.range(0.25, 0.6);
                let dicke = self.rng.range(0.22, 0.36) * laenge.sqrt();
                let spaeter = r / tempo;
                self.dorn(ctx, p, aussen, neigung, laenge, dicke, true, spaeter, 1.9);
            }
        }
        // Raureif auf dem Boden
        for _ in 0..16 {
            let w = self.rng.range(0.0, TAU);
            let r = self.rng.range(0.0, 1.0).sqrt() * radius * 0.95;
            let p = mitte + vec3(w.cos() * r, 0.0, w.sin() * r);
            let g = self.rng.range(0.7, 1.4);
            self.fleck(ctx, terrain, p, g, vec3(0.6, 0.74, 0.86), 6.0, Material::Standard, mitte.y);
        }
        // Frostnebel und Schneegestöber am Rand der Welle
        for k in 0..24 {
            let w = TAU * k as f32 / 24.0;
            let aussen = vec3(w.cos(), 0.0, w.sin());
            let p = mitte + aussen * radius * 0.8 + Vec3::Y * 0.3;
            // Frostnebel: kalt leuchtender Dunst
            ctx.particles.burst_glow(Burst {
                position: p,
                count: 1,
                color: vec3(0.45, 0.62, 0.8),
                color_variation: 0.1,
                speed: 1.6,
                direction: aussen * 0.6 + Vec3::Y * 0.2,
                size: 0.4,
                life: 1.3,
                gravity: -0.2,
                glow: 0.5,
                grow: 1.5,
                round: true,
            });
            self.funken(ctx, mitte + aussen * 0.8 + Vec3::Y * 0.4, 2, vec3(0.85, 0.95, 1.0), 9.0, 0.05, 0.8, 1.0, 3.0);
        }
        self.licht(mitte + Vec3::Y, vec3(2.0, 3.5, 5.5), radius * 1.8, 0.8);
        self.erschuettern(mitte, 0.3, 22.0);
        sounds.push(SoundEvent::Zauber { klang: Klang::Frost, at: mitte, laut: 1.0 });
    }

    /// Hammerschlag: leuchtende Sichel der Schwungbahn; der Schmetterschlag bricht den Boden auf.
    #[allow(clippy::too_many_arguments)]
/// Klingenhieb des Schurken: eine eisblaue Runensichel (Stufe 0 von rechts, 1 von links) bzw. beim
    /// Stich ein gleißender Stoß nach vorne mit Ring an der Spitze.
    fn klingenhieb(&mut self, ctx: &mut Context, spieler: PlayerId, figur: Figur, stufe: u8, richtung: Vec3, sounds: &mut Vec<SoundEvent>) {
        let f = self.formen(ctx);
        let quer = richtung.cross(Vec3::Y).normalize_or(Vec3::X);
        let blau = vec3(0.55, 0.85, 1.0);
        let brust = figur.mitte + Vec3::Y * 0.1;
        if stufe == 2 {
            let von = brust + richtung * 0.4;
            let nach = brust + richtung * 3.1;
            let drehung = Quat::from_rotation_arc(Vec3::Z, richtung);
            for k in 0..9 {
                let t = k as f32 / 8.0;
                let p = von.lerp(nach, t);
                let dicke = 0.05 + 0.12 * (1.0 - t);
                let e = self.leuchten(ctx, f.kugel, p, drehung, vec3(dicke, dicke, 0.5), blau.lerp(Vec3::ONE, t * 0.5), 2.2, 1.0);
                self.teil(e, t * 0.04, 0.22, p, drehung, blau, Art::Gluehen);
            }
            let quer_ring = Quat::from_rotation_arc(Vec3::Y, richtung);
            let ring = self.leuchten(ctx, f.ring, nach, quer_ring, Vec3::ZERO, blau, 2.4, 0.0);
            self.teil(ring, 0.04, 0.28, nach, quer_ring, blau, Art::Ring { von: 0.2, bis: 1.3 });
            self.blitz(ctx, nach, 0.9, vec3(0.8, 0.95, 1.0), vec3(0.1, 0.3, 0.6), 0.2, 2.0);
            self.funken(ctx, nach, 18, blau, 5.0, 0.035, 0.35, 3.0, 3.5);
            self.licht(nach, vec3(1.5, 2.5, 3.5), 5.0, 0.3);
            self.erschuettern(nach, 0.18, 12.0);
            self.stopp.push((spieler, 0.06));
            sounds.push(SoundEvent::Zauber { klang: Klang::Pfeiltreffer, at: nach, laut: 0.8 });
            return;
        }
        let seite = if stufe == 1 { -1.0 } else { 1.0 };
        for k in 0..13 {
            let t = k as f32 / 12.0;
            let w = (t - 0.5) * 2.0 * 1.2 * seite;
            // leicht schräg: von oben rechts nach unten links bzw. umgekehrt
            let p = brust + Vec3::Y * (0.25 * seite * (0.5 - t)) + (richtung * w.cos() + quer * w.sin()) * 1.5;
            let tangente = (quer * w.cos() - richtung * w.sin()).normalize() * -seite;
            let drehung = Quat::from_rotation_arc(Vec3::Z, tangente);
            let breite = (t * PI).sin();
            let aussen = self.leuchten(ctx, f.kugel, p, drehung, vec3(0.05 + 0.13 * breite, 0.02 + 0.05 * breite, 0.45), blau, 1.9, 1.0);
            self.teil(aussen, t * 0.035, 0.18, p, drehung, blau, Art::Gluehen);
            let kern = self.leuchten(ctx, f.kugel, p, drehung, vec3(0.02 + 0.04 * breite, 0.01 + 0.02 * breite, 0.4), Vec3::ONE, 2.2, 1.0);
            self.teil(kern, t * 0.035, 0.12, p, drehung, Vec3::ONE, Art::Gluehen);
        }
        let ende = brust + (richtung * (1.2f32).cos() - quer * (1.2f32).sin() * seite) * 1.5;
        self.funken(ctx, ende, 8, blau, 2.5, 0.03, 0.25, 2.0, 3.0);
        self.erschuettern(figur.mitte, 0.05, 10.0);
    }

    /// Rauchbombe: ein Knall, eine dichte violettgraue Wolke, Funken und ein dunkler Ring.
    fn rauchbombe(&mut self, ctx: &mut Context, terrain: &Terrain, mitte: Vec3, sounds: &mut Vec<SoundEvent>) {
        let radius = match Faehigkeit::Rauchbombe.form() {
            faehigkeiten::Form::UmSich { radius } => radius,
            _ => 5.0,
        };
        let b = boden(terrain, mitte, mitte.y);
        let unten = vec3(mitte.x, b + 0.1, mitte.z);
        self.blitz(ctx, unten + Vec3::Y * 0.3, 1.2, vec3(1.0, 0.85, 0.6), vec3(0.3, 0.2, 0.35), 0.18, 1.8);
        self.druckwelle(ctx, unten, radius, 0.4, 1.2, vec3(0.55, 0.45, 0.7), 1.0, 0.0);
        for k in 0..14 {
            let w = TAU * k as f32 / 14.0 + self.rng.range(-0.2, 0.2);
            let r = self.rng.range(0.3, 1.0) * radius * 0.8;
            let p = unten + vec3(w.cos() * r, 0.2, w.sin() * r);
            Self::rauch(ctx, p, 4, vec3(0.6, 0.56, 0.66), 1.4, 0.3, 1.5, (p - unten).normalize_or(Vec3::Y) * 0.6 + Vec3::Y * 0.4);
        }
        Self::rauch(ctx, unten + Vec3::Y * 0.4, 20, vec3(0.62, 0.58, 0.7), 2.8, 0.32, 1.6, Vec3::Y * 0.6);
        // Violetter Dunst: weich leuchtende Wolken über dem Rauch
        let f = self.formen(ctx);
        for k in 0..7 {
            let w = TAU * k as f32 / 7.0;
            let p = unten + vec3(w.cos() * radius * 0.45, 0.7, w.sin() * radius * 0.45);
            let groesse = Vec3::splat(radius * 0.45);
            let wolke = self.leuchten(ctx, f.kugel, p, Quat::IDENTITY, groesse, vec3(0.35, 0.25, 0.5), 0.5, 1.0);
            self.teil(wolke, 0.0, 2.2, p, Quat::IDENTITY, vec3(0.35, 0.25, 0.5), Art::Blitz { groesse, ende: vec3(0.1, 0.05, 0.2) });
        }
        self.funken(ctx, unten + Vec3::Y * 0.3, 26, vec3(1.0, 0.75, 0.4), 6.0, 0.04, 0.45, 6.0, 3.0);
        // Sterne über den Betäubten: weiße Blitze im Rauch
        for _ in 0..6 {
            let w = self.rng.range(0.0, TAU);
            let r = self.rng.range(0.5, radius * 0.8);
            let p = unten + vec3(w.cos() * r, self.rng.range(1.2, 1.9), w.sin() * r);
            self.funken(ctx, p, 3, vec3(1.0, 1.0, 0.8), 0.8, 0.05, 0.6, -0.5, 3.0);
        }
        self.licht(unten + Vec3::Y * 0.8, vec3(3.0, 2.2, 1.4), 7.0, 0.3);
        self.erschuettern(unten, 0.3, 16.0);
        sounds.push(SoundEvent::Zauber { klang: Klang::Explosion, at: unten, laut: 0.55 });
    }

    /// Schattenklingen, Beginn: ein violetter Bannkreis und zwei gegenläufig kreisende Klingenkränze.
    fn klingenkreis(&mut self, ctx: &mut Context, terrain: &Terrain, mitte: Vec3, sounds: &mut Vec<SoundEvent>) {
        let f = self.formen(ctx);
        let radius = match Faehigkeit::Schattenklingen.form() {
            faehigkeiten::Form::Flaeche { radius, .. } => radius,
            _ => 5.5,
        };
        let b = boden(terrain, mitte, mitte.y);
        let unten = vec3(mitte.x, b + 0.08, mitte.z);
        let violett = vec3(0.6, 0.35, 1.0);
        let dauer = faehigkeiten::klingen_schluss() - Faehigkeit::Schattenklingen.ausholen(0) + 0.2;
        let ring = self.leuchten(ctx, f.ring, unten, Quat::IDENTITY, vec3(radius, 1.0, radius), violett, 1.5, 0.0);
        self.teil(ring, 0.0, dauer, unten, Quat::IDENTITY, violett, Art::Drehen { tempo: 0.6, groesse: vec3(radius, 1.0, radius) });
        let innen = self.leuchten(ctx, f.scheibe, unten, Quat::IDENTITY, vec3(radius, 1.0, radius), violett * 0.3, 0.7, 0.0);
        self.teil(innen, 0.0, dauer, unten, Quat::IDENTITY, violett * 0.3, Art::Drehen { tempo: 0.0, groesse: vec3(radius, 1.0, radius) });
        for (hoehe, tempo, groesse, neigung) in [(0.7, 7.0, 0.85, 0.12), (1.5, -5.5, 0.6, -0.18)] {
            let p = unten + Vec3::Y * hoehe;
            let drehung = Quat::from_rotation_x(neigung);
            let kranz = self.leuchten(ctx, f.klingen, p, drehung, Vec3::splat(radius * groesse), violett.lerp(Vec3::ONE, 0.35), 3.2, 0.2);
            self.teil(kranz, 0.0, dauer, p, drehung, violett.lerp(Vec3::ONE, 0.25), Art::Drehen { tempo, groesse: Vec3::splat(radius * groesse) });
        }
        self.licht(unten + Vec3::Y, vec3(2.0, 1.1, 3.5), radius * 2.0, dauer);
        sounds.push(SoundEvent::Zauber { klang: Klang::Himmel, at: unten, laut: 0.7 });
    }

    /// Eine Klingenwelle: zwei Schnitte quer durch den Kreis, Funken, ein Aufblitzen am Boden.
    fn klingenwelle(&mut self, ctx: &mut Context, terrain: &Terrain, mitte: Vec3, sounds: &mut Vec<SoundEvent>) {
        let f = self.formen(ctx);
        let radius = match Faehigkeit::Schattenklingen.form() {
            faehigkeiten::Form::Flaeche { radius, .. } => radius,
            _ => 5.5,
        };
        let b = boden(terrain, mitte, mitte.y);
        let violett = vec3(0.6, 0.35, 1.0);
        for _ in 0..2 {
            let w = self.rng.range(0.0, TAU);
            let hoehe = b + self.rng.range(0.5, 1.6);
            let richtung = vec3(w.cos(), self.rng.range(-0.25, 0.25), w.sin()).normalize();
            let versatz = vec3(-w.sin(), 0.0, w.cos()) * self.rng.range(-0.4, 0.4) * radius;
            let von = vec3(mitte.x, hoehe, mitte.z) + versatz - richtung * radius * 0.85;
            let drehung = Quat::from_rotation_arc(Vec3::Z, richtung);
            for k in 0..10 {
                let t = k as f32 / 9.0;
                let p = von + richtung * radius * 1.7 * t;
                let breite = (t * PI).sin();
                let e = self.leuchten(ctx, f.kugel, p, drehung, vec3(0.04 + 0.1 * breite, 0.02 + 0.04 * breite, 0.6), violett.lerp(Vec3::ONE, 0.4 * breite), 2.0, 1.0);
                self.teil(e, t * 0.05, 0.2, p, drehung, violett, Art::Gluehen);
            }
            let ende = von + richtung * radius * 1.7;
            self.funken(ctx, ende, 6, violett, 2.0, 0.035, 0.3, 1.0, 3.0);
        }
        self.glut(ctx, vec3(mitte.x, b + 0.3, mitte.z), 8, violett, 2.5, 0.08, 0.5, -1.0, 0.3, Vec3::Y * 0.5);
        sounds.push(SoundEvent::Zauber { klang: Klang::Schwung, at: mitte, laut: 0.45 });
    }

    /// Schlussschlag der Schattenklingen: die Kränze zerspringen in einer violetten Explosion.
    fn klingenschluss(&mut self, ctx: &mut Context, terrain: &Terrain, spieler: PlayerId, mitte: Vec3, sounds: &mut Vec<SoundEvent>) {
        let radius = match Faehigkeit::Schattenklingen.form() {
            faehigkeiten::Form::Flaeche { radius, .. } => radius,
            _ => 5.5,
        };
        let b = boden(terrain, mitte, mitte.y);
        let unten = vec3(mitte.x, b + 0.1, mitte.z);
        let violett = vec3(0.6, 0.35, 1.0);
        self.druckwelle(ctx, unten, radius + 1.0, 0.45, 1.8, violett, 2.0, 0.0);
        self.blitz(ctx, unten + Vec3::Y * 0.8, 2.6, vec3(0.9, 0.8, 1.0), vec3(0.3, 0.1, 0.6), 0.3, 2.4);
        self.funken(ctx, unten + Vec3::Y * 0.8, 50, violett.lerp(Vec3::ONE, 0.3), 9.0, 0.05, 0.6, 5.0, 3.5);
        Self::rauch(ctx, unten + Vec3::Y * 0.3, 12, vec3(0.26, 0.2, 0.34), 2.5, 0.26, 1.2, Vec3::Y * 0.5);
        self.licht(unten + Vec3::Y, vec3(4.0, 2.5, 6.0), radius * 2.5, 0.45);
        self.erschuettern(unten, 0.6, 26.0);
        self.stopp.push((spieler, 0.08));
        sounds.push(SoundEvent::Zauber { klang: Klang::Explosion, at: unten, laut: 0.9 });
    }

    fn hieb(&mut self, ctx: &mut Context, terrain: &Terrain, spieler: PlayerId, figur: Figur, stufe: u8, richtung: Vec3, sounds: &mut Vec<SoundEvent>) {
        let f = self.formen(ctx);
        let quer = richtung.cross(Vec3::Y).normalize_or(Vec3::X);
        let gold = vec3(1.0, 0.8, 0.45);
        let brust = figur.mitte + Vec3::Y * 0.05;
        // Punkte der Sichel: waagerecht (rechts→links, links→rechts) bzw. senkrecht von oben
        let punkte: Vec<(Vec3, Vec3)> = (0..11)
            .map(|k| {
                let t = k as f32 / 10.0;
                match stufe {
                    2 => {
                        let w = PI * 0.95 * (1.0 - t) - 0.1;
                        let p = brust + Vec3::Y * 0.2 + (richtung * w.sin() + Vec3::Y * w.cos()) * 1.75;
                        (p, (richtung * w.cos() - Vec3::Y * w.sin()).normalize())
                    }
                    s => {
                        let seite = if s == 1 { -1.0 } else { 1.0 };
                        let w = (t - 0.5) * 2.0 * 1.15 * seite;
                        let p = brust - Vec3::Y * (0.1 + 0.15 * s as f32) + (richtung * w.cos() + quer * w.sin()) * 1.65;
                        (p, (quer * w.cos() - richtung * w.sin()).normalize() * -seite)
                    }
                }
            })
            .collect();
        for (k, (p, tangente)) in punkte.iter().enumerate() {
            let t = k as f32 / 10.0;
            let breite = (t * PI).sin();
            let drehung = Quat::from_rotation_arc(Vec3::Z, *tangente);
            let groesse = vec3(0.12 + 0.2 * breite, 0.12 + 0.2 * breite, 0.55);
            let e = self.leuchten(ctx, f.kugel, *p, drehung, groesse, gold, 1.8, 1.0);
            self.teil(e, t * 0.05, 0.2, *p, drehung, gold, Art::Gluehen);
        }
        if let Some((ende, _)) = punkte.last() {
            self.funken(ctx, *ende, 8, gold, 3.0, 0.04, 0.3, 4.0, 3.0);
        }
        if stufe == 2 {
            // Schmetterschlag: Einschlag vor dem Zwerg
            let fuesse = figur.mitte - Vec3::Y * 0.9;
            let p = if figur.spitze.distance(fuesse) < 3.0 { figur.spitze } else { fuesse + richtung * 1.6 };
            let p = vec3(p.x, boden(terrain, p, fuesse.y), p.z);
            self.druckwelle(ctx, p, faehigkeiten::SCHMETTERN_WELLE.0 + 0.6, 0.3, 0.9, vec3(1.0, 0.7, 0.35), 1.8, 0.0);
            self.blitz(ctx, p + Vec3::Y * 0.2, 1.3, vec3(1.0, 0.9, 0.65), vec3(0.5, 0.25, 0.05), 0.22, 2.2);
            self.risse(ctx, terrain, p, 5, 1.8, fuesse.y);
            Self::rauch(ctx, p + Vec3::Y * 0.15, 8, vec3(0.55, 0.5, 0.44), 2.0, 0.22, 1.0, Vec3::Y * 0.4);
            for _ in 0..6 {
                let d = vec3(self.rng.range(-1.0, 1.0), self.rng.range(0.8, 1.6), self.rng.range(-1.0, 1.0));
                let g = self.rng.range(0.1, 0.2);
                let v = d * self.rng.range(3.5, 6.5);
                self.brocken(ctx, p + Vec3::Y * 0.2, v, g, p.y, vec3(0.42, 0.36, 0.3));
            }
            self.funken(ctx, p, 20, gold, 6.0, 0.05, 0.5, 8.0, 3.5);
            self.licht(p + Vec3::Y * 0.5, vec3(4.0, 3.0, 1.5), 7.0, 0.35);
            self.erschuettern(p, 0.55, 22.0);
            self.stopp.push((spieler, 0.08));
            sounds.push(SoundEvent::Zauber { klang: Klang::Hammer, at: p, laut: 1.0 });
            sounds.push(SoundEvent::Zauber { klang: Klang::Beben, at: p, laut: 0.4 });
        } else {
            self.erschuettern(figur.mitte, 0.08, 12.0);
        }
    }

    /// Risse im Boden, die kurz glühen und dann dunkel liegen bleiben.
    fn risse(&mut self, ctx: &mut Context, terrain: &Terrain, mitte: Vec3, anzahl: u32, laenge: f32, bezug: f32) {
        let f = self.formen(ctx);
        let versatz = self.rng.range(0.0, TAU);
        for k in 0..anzahl {
            let mut w = versatz + TAU * (k as f32 + self.rng.range(-0.25, 0.25)) / anzahl as f32;
            let mut p = mitte;
            let stuecke = 4;
            let l = laenge * self.rng.range(0.55, 1.0);
            for s in 0..stuecke {
                w += self.rng.range(-0.5, 0.5);
                let d = vec3(w.cos(), 0.0, w.sin());
                let stueck = l / stuecke as f32;
                let q = p + d * stueck;
                let m = (p + q) * 0.5;
                let y = boden(terrain, m, bezug);
                let breite = 0.11 * (1.0 - s as f32 / stuecke as f32 * 0.75) * (laenge / 3.0).clamp(0.6, 1.3);
                let drehung = Quat::from_rotation_y(-w);
                let ort = vec3(m.x, y + 0.03, m.z);
                let dunkel = self.fest(ctx, f.wuerfel, ort, drehung, vec3(stueck, 0.03, breite), vec3(0.06, 0.05, 0.04), Material::Standard, false);
                self.teil(dunkel, 0.0, 3.5, ort, drehung, vec3(0.06, 0.05, 0.04), Art::Fleck { normal: Vec3::Y });
                let glut = self.leuchten(ctx, f.wuerfel, ort + Vec3::Y * 0.02, drehung, vec3(stueck, 0.05, breite * 0.8), vec3(1.0, 0.38, 0.08), 2.4, 0.0);
                self.teil(glut, s as f32 * 0.03, 0.9, ort + Vec3::Y * 0.02, drehung, vec3(1.0, 0.38, 0.08), Art::Gluehen);
                p = q;
            }
        }
    }

    /// Erdbeben (Haupt- und Nachbeben): Druckwelle, Felsdornen in Wellen, Risse, Staub, Brocken.
    #[allow(clippy::too_many_arguments)]
    fn beben(&mut self, ctx: &mut Context, terrain: &Terrain, spieler: PlayerId, mitte: Vec3, kopf: Vec3, haupt: bool, radius: f32, sounds: &mut Vec<SoundEvent>) {
        let erde = vec3(1.0, 0.6, 0.28);
        let staub = vec3(0.4, 0.35, 0.29);
        if haupt {
            self.blitz(ctx, mitte + Vec3::Y * 0.3, 1.6, vec3(1.0, 0.8, 0.5), vec3(0.5, 0.2, 0.05), 0.22, 1.5);
            self.druckwelle(ctx, mitte, radius, 0.38, 1.7, erde, 1.3, 0.0);
            self.druckwelle(ctx, mitte, radius * 0.6, 0.3, 1.0, vec3(0.9, 0.7, 0.45), 0.8, 0.08);
            for (r, anzahl) in [(1.5, 6), (3.1, 9), (4.7, 12), (6.2, 15)] {
                let versatz = self.rng.range(0.0, TAU);
                for k in 0..anzahl {
                    let w = versatz + TAU * (k as f32 + self.rng.range(-0.3, 0.3)) / anzahl as f32;
                    let aussen = vec3(w.cos(), 0.0, w.sin());
                    let p = mitte + aussen * (r + self.rng.range(-0.35, 0.35));
                    let p = vec3(p.x, boden(terrain, p, mitte.y) - 0.1, p.z);
                    let laenge = self.rng.range(1.0, 1.9) * (1.0 - r / radius * 0.35);
                    let dicke = self.rng.range(0.6, 0.9) * laenge.sqrt();
                    let neigung = self.rng.range(0.3, 0.65);
                    self.dorn(ctx, p, aussen, neigung, laenge, dicke, false, r / 20.0, 1.5);
                    Self::kruemel(ctx, p + Vec3::Y * 0.2, 4, 3.5);
                    if k % 3 == 0 {
                        Self::rauch(ctx, p + Vec3::Y * 0.2, 1, staub, 1.0, 0.18, 0.8, aussen * 0.4 + Vec3::Y * 0.5);
                    }
                }
            }
            let kopf = vec3(kopf.x, boden(terrain, kopf, mitte.y), kopf.z);
            self.risse(ctx, terrain, kopf, 8, radius * 0.6, mitte.y);
            self.blitz(ctx, kopf + Vec3::Y * 0.1, 1.0, vec3(1.0, 0.8, 0.5), vec3(0.6, 0.25, 0.05), 0.2, 2.5);
            for _ in 0..16 {
                let d = vec3(self.rng.range(-1.0, 1.0), self.rng.range(1.0, 2.0), self.rng.range(-1.0, 1.0));
                let g = self.rng.range(0.12, 0.3);
                let (v, hell) = (d * self.rng.range(3.0, 6.0), self.rng.range(0.8, 1.1));
                self.brocken(ctx, mitte + Vec3::Y * 0.3, v, g, mitte.y, vec3(0.42, 0.36, 0.3) * hell);
            }
            for k in 0..16 {
                let w = TAU * k as f32 / 16.0;
                let aussen = vec3(w.cos(), 0.0, w.sin());
                Self::rauch(ctx, mitte + aussen * 1.0 + Vec3::Y * 0.2, 1, staub, 3.0, 0.2, 0.8, aussen + Vec3::Y * 0.15);
            }
            Self::kruemel(ctx, mitte + Vec3::Y * 0.3, 30, 6.0);
            self.funken(ctx, mitte, 30, erde, 7.0, 0.06, 0.6, 9.0, 3.0);
            self.licht(mitte + Vec3::Y, vec3(6.0, 3.5, 1.5), 12.0, 0.5);
            self.erschuettern(mitte, 0.9, 34.0);
            self.stopp.push((spieler, 0.1));
            sounds.push(SoundEvent::Zauber { klang: Klang::Beben, at: mitte, laut: 1.0 });
            sounds.push(SoundEvent::Thunder { volume: 0.3 });
        } else {
            // Nachbeben: flacher, staubiger, ein paar kleine Dornen
            self.druckwelle(ctx, mitte, radius, 0.32, 0.8, vec3(0.8, 0.55, 0.3), 1.2, 0.0);
            for _ in 0..8 {
                let w = self.rng.range(0.0, TAU);
                let r = self.rng.range(1.2, radius);
                let aussen = vec3(w.cos(), 0.0, w.sin());
                let p = mitte + aussen * r;
                let p = vec3(p.x, boden(terrain, p, mitte.y) - 0.1, p.z);
                let (neigung, laenge, dicke) = (self.rng.range(0.3, 0.6), self.rng.range(0.35, 0.7), self.rng.range(0.3, 0.42));
                self.dorn(ctx, p, aussen, neigung, laenge * 1.4, dicke * 1.5, false, r / 20.0, 0.9);
            }
            for k in 0..10 {
                let w = TAU * k as f32 / 10.0;
                let aussen = vec3(w.cos(), 0.0, w.sin());
                Self::rauch(ctx, mitte + aussen * radius * 0.6 + Vec3::Y * 0.2, 1, staub, 1.6, 0.18, 0.7, aussen * 0.5 + Vec3::Y * 0.2);
                Self::kruemel(ctx, mitte + aussen * radius * 0.6 + Vec3::Y * 0.1, 3, 3.0);
            }
            self.erschuettern(mitte, 0.35, 28.0);
            sounds.push(SoundEvent::Zauber { klang: Klang::Beben, at: mitte, laut: 0.55 });
        }
    }

    /// Arkane Ladungen: kleine Lichter kreisen um jeden Magier, der welche hat.
    fn update_ladungen(&mut self, ctx: &mut Context, figuren: &HashMap<PlayerId, Figur>) {
        let f = self.formen(ctx);
        let zeit = ctx.time.elapsed;
        let violett = vec3(0.6, 0.45, 1.0);
        // Nicht mehr gebrauchte entfernen
        let weg: Vec<PlayerId> = self.ladungen.keys().filter(|id| figuren.get(id).is_none_or(|fig| fig.ladung == 0)).copied().collect();
        for id in weg {
            for (a, b) in self.ladungen.remove(&id).unwrap_or_default() {
                if let Some(p) = ctx.scene.try_get(a).map(|e| e.transform.position) {
                    self.funken(ctx, p, 5, violett, 1.5, 0.04, 0.25, 0.0, 3.0);
                }
                ctx.scene.despawn(a);
                ctx.scene.despawn(b);
            }
        }
        for (&id, figur) in figuren.iter().filter(|(_, f)| f.ladung > 0) {
            let n = figur.ladung.min(faehigkeiten::LADUNG_MAX) as usize;
            let mut liste = self.ladungen.remove(&id).unwrap_or_default();
            while liste.len() < n {
                let kern = self.leuchten(ctx, f.kugel, figur.mitte, Quat::IDENTITY, Vec3::splat(0.13), vec3(0.8, 0.7, 1.0), 1.6, 1.0);
                let schein = self.leuchten(ctx, f.kugel, figur.mitte, Quat::IDENTITY, Vec3::splat(0.36), violett, 0.7, 1.0);
                self.funken(ctx, figur.mitte, 8, violett, 2.0, 0.04, 0.3, 0.0, 3.5);
                liste.push((kern, schein));
            }
            while liste.len() > n {
                if let Some((a, b)) = liste.pop() {
                    ctx.scene.despawn(a);
                    ctx.scene.despawn(b);
                }
            }
            let voll = n >= faehigkeiten::LADUNG_MAX as usize;
            let tempo = if voll { 4.5 } else { 2.6 };
            for (i, &(kern, schein)) in liste.iter().enumerate() {
                let w = zeit * tempo + i as f32 * TAU / n as f32;
                let p = figur.mitte + Vec3::Y * (0.25 + 0.08 * (zeit * 3.0 + i as f32).sin()) + vec3(w.cos(), 0.0, w.sin()) * 0.62;
                let puls = if voll { 1.0 + 0.25 * (zeit * 10.0).sin() } else { 1.0 };
                if let Some(e) = ctx.scene.try_get_mut(kern) {
                    e.transform.position = p;
                    e.transform.scale = Vec3::splat(0.13 * puls);
                }
                if let Some(e) = ctx.scene.try_get_mut(schein) {
                    e.transform.position = p;
                    e.transform.scale = Vec3::splat(0.36 * puls);
                }
                if self.rng.chance(if voll { 0.5 } else { 0.2 }) {
                    self.funken(ctx, p, 1, violett, 0.3, 0.035, 0.35, 0.0, 3.5);
                }
            }
            if voll {
                ctx.lights.push(PointLight { position: figur.mitte + Vec3::Y * 0.3, color: violett * 1.6, radius: 4.0 });
            }
            self.ladungen.insert(id, liste);
        }
    }

    fn update_teile(&mut self, ctx: &mut Context, terrain: &Terrain) {
        let dt = ctx.time.delta;
        let _ = terrain;
        let mut i = 0;
        while i < self.teile.len() {
            let teil = &mut self.teile[i];
            teil.alter += dt;
            let Some(entity) = ctx.scene.try_get_mut(teil.entity) else {
                self.teile.swap_remove(i);
                continue;
            };
            if teil.alter < 0.0 {
                entity.visible = false;
                i += 1;
                continue;
            }
            entity.visible = true;
            let t = (teil.alter / teil.dauer).clamp(0.0, 1.0);
            let blass = |k: f32| teil.farbe * (1.0 - t).powf(k);
            match teil.art {
                Art::Welle { von, bis, hoehe } => {
                    let r = von + (bis - von) * aus(t);
                    entity.transform.position = teil.ort;
                    entity.transform.scale = vec3(r, hoehe * (1.0 - t).powf(1.3) + 0.05, r);
                    entity.color = blass(1.2).extend(1.0);
                }
                Art::Ring { von, bis } => {
                    let r = von + (bis - von) * aus(t);
                    entity.transform.scale = vec3(r, 1.0, r);
                    entity.transform.rotation = teil.drehung;
                    entity.color = blass(1.5).extend(1.0);
                }
                Art::Blitz { groesse, ende } => {
                    let wachsen = (t / 0.18).min(1.0).sqrt();
                    entity.transform.scale = groesse * wachsen * (1.0 + 0.15 * t);
                    entity.color = (teil.farbe.lerp(ende, t.sqrt()) * (1.0 - t).powf(1.4)).extend(1.0);
                }
                Art::Dorn { groesse, wachsen } => {
                    // Herausschießen, stehen, im letzten Viertel wieder versinken
                    let raus = federnd(teil.alter / wachsen);
                    let rein = ((t - 0.75) / 0.25).clamp(0.0, 1.0);
                    let achse = teil.drehung * Vec3::Y;
                    entity.transform.position = teil.ort - achse * groesse.y * rein * rein;
                    entity.transform.rotation = teil.drehung;
                    entity.transform.scale = vec3(groesse.x, groesse.y * raus, groesse.z);
                }
                Art::Fleck { normal } => {
                    let sinken = ((t - 0.8) / 0.2).clamp(0.0, 1.0);
                    entity.transform.position = teil.ort - normal * 0.09 * sinken;
                }
                Art::Gluehen => {
                    entity.color = blass(1.6).extend(1.0);
                }
                Art::Brocken { ref mut geschw, drehen, boden, groesse } => {
                    let pos = entity.transform.position;
                    if geschw.length_squared() > 0.0 {
                        geschw.y -= 18.0 * dt;
                        let neu = pos + *geschw * dt;
                        if neu.y <= boden + groesse * 0.3 {
                            entity.transform.position = vec3(neu.x, boden + groesse * 0.3, neu.z);
                            *geschw = Vec3::ZERO;
                        } else {
                            entity.transform.position = neu;
                            entity.transform.rotation = Quat::from_scaled_axis(drehen * dt) * entity.transform.rotation;
                        }
                    } else if t > 0.7 {
                        entity.transform.position.y -= groesse * 1.2 * dt / (teil.dauer * 0.3);
                    }
                }
                Art::Versinken { tiefe } => {
                    let sinken = ((t - 0.6) / 0.4).clamp(0.0, 1.0);
                    entity.transform.position = teil.ort - Vec3::Y * tiefe * sinken * sinken;
                    entity.transform.rotation = teil.drehung;
                }
                Art::Blinken => {
                    let takt = 6.0 + 30.0 * t * t;
                    let an = 0.35 + 0.65 * (teil.alter * takt).sin().abs();
                    entity.color = (teil.farbe * an).extend(1.0);
                    entity.transform.scale = Vec3::splat(0.14 + 0.1 * an);
                }
                Art::Drehen { tempo, groesse } => {
                    let ein = (teil.alter / 0.2).min(1.0);
                    let aus_ = ((1.0 - t) / 0.2).min(1.0);
                    entity.transform.rotation = teil.drehung * Quat::from_rotation_y(teil.alter * tempo);
                    entity.transform.scale = groesse * (0.85 + 0.15 * ein);
                    entity.color = (teil.farbe * ein * aus_ * (0.85 + 0.15 * (teil.alter * 5.0).sin())).extend(1.0);
                }
                Art::Strahl { laenge, dicke } => {
                    let auf = (teil.alter / 0.05).min(1.0);
                    let d = dicke * auf * (1.0 - t).powf(1.2) * (1.0 + 0.12 * (teil.alter * 60.0).sin());
                    entity.transform.position = teil.ort;
                    entity.transform.rotation = teil.drehung;
                    entity.transform.scale = vec3(d, laenge, d);
                    entity.color = blass(0.8).extend(1.0);
                }
            }
            if teil.alter >= teil.dauer {
                let e = teil.entity;
                ctx.scene.despawn(e);
                self.teile.swap_remove(i);
                continue;
            }
            i += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Türme: Schüsse und Einschläge im Stil der Heldenfähigkeiten
// ---------------------------------------------------------------------------

impl Zauberbild {
    /// Ein Turm schießt (nur Optik; ob er trifft, entscheidet der Server). `stufe` 1–3 macht die
    /// Effekte größer. Liefert `false` für Türme ohne Geschoss (Banner, Kaserne …).
    #[allow(clippy::too_many_arguments)]
    pub fn turmschuss(&mut self, ctx: &mut Context, terrain: &Terrain, art: crate::tuerme::TowerKind, zweig: u8, stufe: u8, von: Vec3, nach: Vec3, sounds: &mut Vec<SoundEvent>) -> bool {
        use crate::tuerme::TowerKind as T;
        let g = 0.75 + 0.2 * stufe.clamp(1, 3) as f32;
        match art {
            T::Arrow => {
                self.muendung(ctx, von, vec3(1.0, 0.85, 0.6), 0.4);
                let pfeil = if zweig == 2 { PfeilArt::Krit } else { PfeilArt::Normal };
                self.turmpfeil(ctx, von, nach, pfeil, 1.2 + 0.1 * g, 62.0);
                sounds.push(SoundEvent::Zauber { klang: Klang::Pfeilschuss, at: von, laut: 0.35 });
            }
            T::Scout => {
                self.turmpfeil(ctx, von, nach, PfeilArt::Krit, 1.2, 70.0);
                sounds.push(SoundEvent::Zauber { klang: Klang::Pfeilschuss, at: von, laut: 0.3 });
            }
            T::Ballista => {
                self.muendung(ctx, von, vec3(1.0, 0.85, 0.6), 0.8);
                self.turmpfeil(ctx, von, nach, PfeilArt::Bolzen, 2.4 * g.sqrt(), 50.0);
                sounds.push(SoundEvent::Zauber { klang: Klang::Wurf, at: von, laut: 0.6 });
            }
            T::Catapult => self.katapult(ctx, von, nach, zweig == 1, g, sounds),
            T::Fire if zweig == 2 => self.drachenatem(ctx, terrain, von, nach, g, sounds),
            T::Fire => {
                self.turmgeschoss(ctx, Faehigkeit::Feuerball, von, nach, 0.6 * g, 26.0, 0.1, (vec3(1.0, 0.85, 0.5), vec3(1.0, 0.42, 0.1)));
                sounds.push(SoundEvent::Zauber { klang: Klang::Feuerwurf, at: von, laut: 0.55 });
            }
            T::Frost => {
                self.turmgeschoss(ctx, Faehigkeit::Frostnova, von, nach, 0.55 * g, 38.0, 0.03, (vec3(0.9, 0.97, 1.0), vec3(0.45, 0.75, 1.0)));
                sounds.push(SoundEvent::Zauber { klang: Klang::Frost, at: von, laut: 0.3 });
            }
            T::Arcane => {
                self.turmgeschoss(ctx, Faehigkeit::Arkangeschoss, von, nach, 0.75 * g, 32.0, 0.02, (vec3(0.85, 0.8, 1.0), vec3(0.5, 0.32, 1.0)));
                sounds.push(SoundEvent::Zauber { klang: Klang::Arkan, at: von, laut: 0.4 });
            }
            T::Poison => {
                self.turmgeschoss(ctx, Faehigkeit::Wurfdolche, von, nach, 0.6 * g, 20.0, 0.22, (vec3(0.8, 1.0, 0.6), vec3(0.35, 0.9, 0.25)));
                sounds.push(SoundEvent::Zauber { klang: Klang::Wurf, at: von, laut: 0.35 });
            }
            T::Lightning => self.blitzschlag(ctx, terrain, von, nach, g, sounds),
            T::Sun => self.sonnenstrahl(ctx, terrain, von, nach, g, sounds),
            T::Storm => self.sturmboe(ctx, von, nach, g, sounds),
            _ => return false,
        }
        true
    }

    /// Mündungsblitz am Turmkopf.
    fn muendung(&mut self, ctx: &mut Context, ort: Vec3, farbe: Vec3, staerke: f32) {
        self.blitz(ctx, ort, 0.5 + 0.5 * staerke, farbe, farbe * 0.3, 0.15, 1.5);
        self.funken(ctx, ort, (6.0 + 10.0 * staerke) as u32, farbe, 3.0, 0.04, 0.25, 1.0, 3.0);
        Self::rauch(ctx, ort, (2.0 + 4.0 * staerke) as u32, vec3(0.6, 0.57, 0.52), 1.2, 0.14 + 0.1 * staerke, 0.8, Vec3::Y * 0.3);
    }

    fn turmpfeil(&mut self, ctx: &mut Context, von: Vec3, nach: Vec3, art: PfeilArt, groesse: f32, tempo: f32) {
        self.pfeil_los(ctx, 0, art, nach, true, 0.0, tempo);
        if let Some(p) = self.pfeile.last_mut() {
            p.von = Some(von);
            p.pos = von;
            p.groesse = groesse;
        }
    }

    /// Leuchtendes Geschoss (Feuer-, Eis-, Arkan- und Giftturm) mit Kern, Hülle und Schweif.
    #[allow(clippy::too_many_arguments)]
    fn turmgeschoss(&mut self, ctx: &mut Context, art: Faehigkeit, von: Vec3, nach: Vec3, groesse: f32, tempo: f32, bogen: f32, (kern_farbe, huelle_farbe): (Vec3, Vec3)) {
        let f = self.formen(ctx);
        let kern = self.leuchten(ctx, f.kugel, von, Quat::IDENTITY, Vec3::ZERO, kern_farbe, 2.0, 1.0);
        let huelle = self.leuchten(ctx, f.kugel, von, Quat::IDENTITY, Vec3::ZERO, huelle_farbe, 1.4, 1.0);
        let schweif = self.leuchten(ctx, f.kugel, von, Quat::IDENTITY, Vec3::ZERO, huelle_farbe, 1.1, 1.0);
        self.geschosse.push(Geschoss {
            spieler: 0,
            art,
            nach,
            hit: true,
            alter: 0.0,
            ausholen: 0.0,
            tempo,
            von: Some(von),
            pos: von,
            kern,
            huelle,
            schweif,
            groesse,
            bogen,
            turm: true,
        });
    }

    /// Eisgeschoss schlägt ein: Eisdornen im Kreis, Kältewelle, Splitter, Frostnebel.
    fn eistreffer(&mut self, ctx: &mut Context, terrain: &Terrain, ort: Vec3, groesse: f32, sounds: &mut Vec<SoundEvent>) {
        let eis = vec3(0.6, 0.88, 1.0);
        let b = boden(terrain, ort, ort.y - 0.5).min(ort.y);
        let unten = vec3(ort.x, b, ort.z);
        self.blitz(ctx, ort, 1.6 * groesse, vec3(0.85, 0.95, 1.0), eis * 0.2, 0.25, 1.8);
        self.druckwelle(ctx, unten, 3.2 * groesse, 0.3, 0.8, vec3(0.4, 0.7, 1.0), 1.3, 0.0);
        let anzahl = 7;
        let versatz = self.rng.range(0.0, TAU);
        for k in 0..anzahl {
            let w = versatz + TAU * k as f32 / anzahl as f32;
            let aussen = vec3(w.cos(), 0.0, w.sin());
            let p = unten + aussen * self.rng.range(0.4, 1.1) * groesse;
            let p = vec3(p.x, boden(terrain, p, b) - 0.05, p.z);
            let laenge = self.rng.range(0.5, 1.0) * groesse;
            let neigung = self.rng.range(0.25, 0.6);
            self.dorn(ctx, p, aussen, neigung, laenge, 0.2 * laenge.sqrt(), true, 0.03 * k as f32, 1.3);
        }
        self.fleck(ctx, terrain, unten, 1.4 * groesse, vec3(0.6, 0.74, 0.86), 4.0, Material::Standard, b);
        self.funken(ctx, ort, 26, vec3(0.85, 0.95, 1.0), 6.0, 0.05, 0.5, 6.0, 3.0);
        ctx.particles.burst_glow(Burst {
            position: unten + Vec3::Y * 0.3,
            count: 8,
            color: vec3(0.45, 0.62, 0.8),
            color_variation: 0.1,
            speed: 1.8,
            direction: Vec3::Y * 0.2,
            size: 0.35 * groesse,
            life: 1.0,
            gravity: -0.2,
            glow: 0.5,
            grow: 1.5,
            round: true,
        });
        self.licht(ort, vec3(2.0, 3.5, 5.5), 7.0 * groesse, 0.4);
        self.erschuettern(ort, 0.08, 14.0);
        sounds.push(SoundEvent::Zauber { klang: Klang::Eisbruch, at: ort, laut: 0.55 });
    }

    /// Giftkugel platzt: grüne Spritzer, eine Giftlache, aufsteigender Giftdunst.
    fn gifttreffer(&mut self, ctx: &mut Context, terrain: &Terrain, ort: Vec3, groesse: f32, sounds: &mut Vec<SoundEvent>) {
        let f = self.formen(ctx);
        let gift = vec3(0.45, 0.95, 0.35);
        let b = boden(terrain, ort, ort.y - 0.5).min(ort.y);
        let unten = vec3(ort.x, b + 0.06, ort.z);
        self.blitz(ctx, ort, 1.4 * groesse, vec3(0.7, 1.0, 0.5), vec3(0.1, 0.3, 0.05), 0.22, 1.5);
        self.funken(ctx, ort, 30, gift, 5.0, 0.06, 0.6, 8.0, 2.5);
        self.fleck(ctx, terrain, unten, 1.8 * groesse, vec3(0.18, 0.32, 0.1), 5.0, Material::Standard, b);
        let lache = self.leuchten(ctx, f.scheibe, unten + Vec3::Y * 0.04, Quat::IDENTITY, vec3(1.6 * groesse, 1.0, 1.6 * groesse), gift * 0.6, 0.9, 0.0);
        self.teil(lache, 0.0, 3.0, unten + Vec3::Y * 0.04, Quat::IDENTITY, gift * 0.6, Art::Gluehen);
        for _ in 0..6 {
            let w = self.rng.range(0.0, TAU);
            let r = self.rng.range(0.0, 1.4) * groesse;
            let p = unten + vec3(w.cos() * r, 0.2, w.sin() * r);
            ctx.particles.burst_glow(Burst {
                position: p,
                count: 2,
                color: vec3(0.3, 0.7, 0.2),
                color_variation: 0.15,
                speed: 0.6,
                direction: Vec3::Y,
                size: 0.3 * groesse,
                life: 1.8,
                gravity: -0.6,
                glow: 0.6,
                grow: 1.6,
                round: true,
            });
        }
        self.licht(ort, vec3(1.5, 3.0, 1.0), 6.0 * groesse, 0.5);
        sounds.push(SoundEvent::Zauber { klang: Klang::ArkanTreffer, at: ort, laut: 0.4 });
    }

    /// Katapult: ein Felsbrocken (bei Brandgeschossen glühend) fliegt in hohem Bogen.
    fn katapult(&mut self, ctx: &mut Context, von: Vec3, nach: Vec3, feuer: bool, groesse: f32, sounds: &mut Vec<SoundEvent>) {
        let f = self.formen(ctx);
        let (farbe, material) = if feuer { (vec3(1.0, 0.5, 0.2), Material::Emissive { glow: 1.5 }) } else { (vec3(0.52, 0.49, 0.45), Material::Standard) };
        let g = 0.75 * groesse;
        let entity = self.fest(ctx, f.meteor, von, Quat::IDENTITY, Vec3::splat(g), farbe, material, true);
        let weite = von.distance(nach);
        let drehung = vec3(self.rng.range(-6.0, 6.0), self.rng.range(-6.0, 6.0), self.rng.range(-6.0, 6.0));
        self.wuerfe.push(Wurf { entity, von, nach, alter: 0.0, dauer: (weite / 20.0).max(0.4), hoehe: weite * 0.3, feuer, groesse, drehung });
        self.muendung(ctx, von, vec3(0.9, 0.8, 0.6), 0.7);
        sounds.push(SoundEvent::Zauber { klang: Klang::Wurf, at: von, laut: 0.7 });
    }

    fn update_wuerfe(&mut self, ctx: &mut Context, terrain: &Terrain, sounds: &mut Vec<SoundEvent>) {
        let dt = ctx.time.delta;
        let mut wuerfe = std::mem::take(&mut self.wuerfe);
        for w in &mut wuerfe {
            w.alter += dt;
            let t = (w.alter / w.dauer).min(1.0);
            let pos = w.von.lerp(w.nach, t) + Vec3::Y * (4.0 * w.hoehe * t * (1.0 - t));
            if let Some(e) = ctx.scene.try_get_mut(w.entity) {
                e.transform.position = pos;
                e.transform.rotation = Quat::from_rotation_x(w.drehung.x * w.alter) * Quat::from_rotation_y(w.drehung.y * w.alter) * Quat::from_rotation_z(w.drehung.z * w.alter);
            }
            if w.feuer {
                self.glut(ctx, pos, 2, vec3(1.0, 0.45, 0.1), 0.6, 0.3 * w.groesse, 0.4, -1.5, 1.2, Vec3::ZERO);
                ctx.lights.push(PointLight { position: pos, color: vec3(4.0, 2.0, 0.6), radius: 8.0 });
            } else if self.rng.chance(0.5) {
                Self::rauch(ctx, pos, 1, vec3(0.6, 0.56, 0.5), 0.3, 0.16, 0.7, Vec3::ZERO);
            }
            if t >= 1.0 {
                ctx.scene.despawn(w.entity);
                if w.feuer {
                    self.explosion(ctx, terrain, pos, 0.75 * w.groesse, false, sounds);
                } else {
                    self.steinschlag(ctx, terrain, pos, w.groesse, sounds);
                }
            }
        }
        wuerfe.retain(|w| w.alter < w.dauer);
        wuerfe.append(&mut self.wuerfe);
        self.wuerfe = wuerfe;
    }

    /// Ein Felsbrocken schlägt ein: der Boden bricht, Brocken fliegen, eine Staubwelle rollt.
    fn steinschlag(&mut self, ctx: &mut Context, terrain: &Terrain, ort: Vec3, groesse: f32, sounds: &mut Vec<SoundEvent>) {
        let b = boden(terrain, ort, ort.y - 0.5).min(ort.y);
        let unten = vec3(ort.x, b, ort.z);
        self.druckwelle(ctx, unten, 4.0 * groesse, 0.4, 1.0, vec3(0.9, 0.75, 0.55), 1.2, 0.0);
        self.blitz(ctx, unten + Vec3::Y * 0.3, 1.2 * groesse, vec3(1.0, 0.92, 0.75), vec3(0.4, 0.3, 0.2), 0.2, 1.4);
        self.risse(ctx, terrain, unten, 5, 1.6 * groesse, b);
        for _ in 0..9 {
            let d = vec3(self.rng.range(-1.0, 1.0), self.rng.range(0.7, 1.5), self.rng.range(-1.0, 1.0));
            let gr = self.rng.range(0.12, 0.26) * groesse;
            let v = d * self.rng.range(3.5, 7.0);
            self.brocken(ctx, unten + Vec3::Y * 0.3, v, gr, b, vec3(0.5, 0.47, 0.43));
        }
        Self::rauch(ctx, unten + Vec3::Y * 0.2, 14, vec3(0.66, 0.6, 0.52), 2.6, 0.3 * groesse, 1.6, Vec3::Y * 0.5);
        Self::kruemel(ctx, unten, 18, 5.0);
        self.erschuettern(unten, 0.3, 22.0);
        sounds.push(SoundEvent::Zauber { klang: Klang::Hammer, at: unten, laut: 0.8 });
        sounds.push(SoundEvent::Zauber { klang: Klang::Beben, at: unten, laut: 0.35 });
    }

    /// Drachenatem (Feuerturm, zweiter Zweig): ein Flammenkegel aus wachsenden Feuerkugeln.
    fn drachenatem(&mut self, ctx: &mut Context, terrain: &Terrain, von: Vec3, nach: Vec3, groesse: f32, sounds: &mut Vec<SoundEvent>) {
        let f = self.formen(ctx);
        let richtung = (nach - von).normalize_or(Vec3::Z);
        let weite = von.distance(nach);
        for k in 0..14 {
            let t = (k as f32 + 1.0) / 14.0;
            let quer = vec3(self.rng.range(-1.0, 1.0), self.rng.range(-0.5, 0.8), self.rng.range(-1.0, 1.0)) * t * 0.6 * groesse;
            let p = von + richtung * weite * t + quer;
            let g = Vec3::splat((0.5 + 1.6 * t) * groesse);
            let farbe = vec3(1.0, 0.7, 0.3).lerp(vec3(1.0, 0.35, 0.05), t);
            let e = self.leuchten(ctx, f.kugel, p, Quat::IDENTITY, Vec3::ZERO, farbe, 1.5, 1.0);
            self.teil(e, t * 0.18, 0.45, p, Quat::IDENTITY, farbe, Art::Blitz { groesse: g, ende: vec3(0.3, 0.05, 0.0) });
            self.glut(ctx, p, 3, vec3(1.0, 0.5, 0.12), 2.0, 0.3 * groesse, 0.5, -1.5, 1.4, richtung * 2.0);
        }
        self.muendung(ctx, von, vec3(1.0, 0.6, 0.2), 1.0);
        self.explosion(ctx, terrain, nach, 0.55 * groesse, false, sounds);
        sounds.push(SoundEvent::Zauber { klang: Klang::Feuerwurf, at: von, laut: 0.8 });
    }

    /// Ein Ast des Blitzes: gezackte, leuchtende Stücke mit weißem Kern.
    fn blitzast(&mut self, ctx: &mut Context, a: Vec3, b: Vec3, zacken: f32, dicke: f32) {
        let f = self.formen(ctx);
        let blau = vec3(0.55, 0.8, 1.0);
        let schritte = ((a.distance(b) / 0.9).ceil() as usize).clamp(3, 40);
        let mut vorher = a;
        for i in 1..=schritte {
            let t = i as f32 / schritte as f32;
            let mut p = a.lerp(b, t);
            if i < schritte {
                p += vec3(self.rng.range(-1.0, 1.0), self.rng.range(-1.0, 1.0), self.rng.range(-1.0, 1.0)) * zacken;
            }
            let mitte = (vorher + p) * 0.5;
            let laenge = vorher.distance(p);
            let drehung = Quat::from_rotation_arc(Vec3::Z, (p - vorher).normalize_or(Vec3::Z));
            let aussen = self.leuchten(ctx, f.kugel, mitte, drehung, vec3(dicke * 4.0, dicke * 4.0, laenge * 1.2), blau, 1.8, 1.0);
            self.teil(aussen, 0.0, 0.22, mitte, drehung, blau, Art::Gluehen);
            let kern = self.leuchten(ctx, f.kugel, mitte, drehung, vec3(dicke * 1.4, dicke * 1.4, laenge * 1.15), Vec3::ONE, 2.8, 1.0);
            self.teil(kern, 0.0, 0.16, mitte, drehung, Vec3::ONE, Art::Gluehen);
            vorher = p;
        }
    }

    /// Blitzturm: gezackter, verästelter Blitz mit gleißendem Kern und Einschlag.
    fn blitzschlag(&mut self, ctx: &mut Context, terrain: &Terrain, von: Vec3, nach: Vec3, groesse: f32, sounds: &mut Vec<SoundEvent>) {
        let blau = vec3(0.55, 0.8, 1.0);
        self.blitzast(ctx, von, nach, 0.3 * groesse, 0.07 * groesse);
        for _ in 0..3 {
            let t = self.rng.range(0.2, 0.8);
            let start = von.lerp(nach, t);
            let ende = start + vec3(self.rng.range(-1.5, 1.5), self.rng.range(-1.5, 0.5), self.rng.range(-1.5, 1.5)) * groesse;
            self.blitzast(ctx, start, ende, 0.25, 0.025 * groesse);
        }
        self.blitz(ctx, nach, 1.5 * groesse, vec3(0.9, 0.95, 1.0), blau * 0.3, 0.2, 2.4);
        self.funken(ctx, nach, 28, vec3(0.8, 0.92, 1.0), 7.0, 0.05, 0.4, 4.0, 4.0);
        let b = boden(terrain, nach, nach.y - 0.5).min(nach.y);
        self.fleck(ctx, terrain, vec3(nach.x, b, nach.z), 0.9 * groesse, vec3(0.08, 0.08, 0.1), 4.0, Material::Standard, b);
        self.licht(nach, vec3(4.0, 6.0, 9.0), 12.0 * groesse, 0.2);
        self.licht(von, vec3(3.0, 4.0, 6.0), 8.0, 0.15);
        self.erschuettern(nach, 0.1, 16.0);
        sounds.push(SoundEvent::Zauber { klang: Klang::Lanze, at: nach, laut: 0.45 });
    }

    /// Sonnenturm: ein gleißender goldener Strahl mit Brennfleck.
    fn sonnenstrahl(&mut self, ctx: &mut Context, terrain: &Terrain, von: Vec3, nach: Vec3, groesse: f32, sounds: &mut Vec<SoundEvent>) {
        let f = self.formen(ctx);
        let richtung = (nach - von).normalize_or(Vec3::NEG_Z);
        let laenge = von.distance(nach).max(0.5);
        let drehung = nach_y(richtung);
        let gold = vec3(1.0, 0.82, 0.35);
        for (dicke, farbe, staerke, weich, dauer) in [(0.1, vec3(1.0, 0.97, 0.85), 2.6, 0.0, 0.3), (0.35, gold, 1.4, 1.0, 0.45)] {
            let e = self.leuchten(ctx, f.strahl, von, drehung, Vec3::ZERO, farbe, staerke, weich);
            self.teil(e, 0.0, dauer, von, drehung, farbe, Art::Strahl { laenge, dicke: dicke * groesse });
        }
        let quer = Quat::from_rotation_arc(Vec3::Y, richtung);
        let ring = self.leuchten(ctx, f.ring, von + richtung * 0.4, quer, Vec3::ZERO, gold, 2.2, 0.0);
        self.teil(ring, 0.0, 0.25, von + richtung * 0.4, quer, gold, Art::Ring { von: 0.2, bis: 0.9 * groesse });
        self.blitz(ctx, nach, 1.3 * groesse, vec3(1.0, 0.95, 0.75), gold * 0.3, 0.3, 2.2);
        let b = boden(terrain, nach, nach.y - 0.5).min(nach.y);
        let unten = vec3(nach.x, b + 0.06, nach.z);
        let brand = self.leuchten(ctx, f.scheibe, unten, Quat::IDENTITY, vec3(0.9 * groesse, 1.0, 0.9 * groesse), gold, 1.4, 0.0);
        self.teil(brand, 0.0, 0.9, unten, Quat::IDENTITY, gold, Art::Gluehen);
        self.funken(ctx, nach, 22, gold, 5.0, 0.05, 0.45, 3.0, 4.0);
        self.licht(nach, vec3(7.0, 5.5, 2.5), 10.0 * groesse, 0.3);
        sounds.push(SoundEvent::Zauber { klang: Klang::Lanze, at: von, laut: 0.4 });
    }

    /// Sturmturm: eine Windböe aus Ringen, die zum Ziel rast, und eine Druckwelle.
    fn sturmboe(&mut self, ctx: &mut Context, von: Vec3, nach: Vec3, groesse: f32, sounds: &mut Vec<SoundEvent>) {
        let f = self.formen(ctx);
        let richtung = (nach - von).normalize_or(Vec3::NEG_Z);
        let weite = von.distance(nach);
        let weiss = vec3(0.85, 0.95, 1.0);
        let quer = Quat::from_rotation_arc(Vec3::Y, richtung);
        for k in 0..6 {
            let t = (k as f32 + 1.0) / 6.0;
            let p = von + richtung * weite * t;
            let ring = self.leuchten(ctx, f.ring, p, quer, Vec3::ZERO, weiss, 1.4, 0.0);
            self.teil(ring, t * 0.25, 0.3, p, quer, weiss, Art::Ring { von: 0.3, bis: (0.8 + 0.8 * t) * groesse });
            self.funken(ctx, p, 3, weiss, 3.0, 0.04, 0.3, 0.0, 2.5);
        }
        let b = nach - Vec3::Y * 0.5;
        self.druckwelle(ctx, b, 3.0 * groesse, 0.35, 1.1, weiss, 1.1, 0.25);
        Self::rauch(ctx, b + Vec3::Y * 0.3, 8, vec3(0.8, 0.82, 0.86), 3.5, 0.2, 0.8, Vec3::ZERO);
        sounds.push(SoundEvent::Zauber { klang: Klang::Schwung, at: von, laut: 0.6 });
    }
}
