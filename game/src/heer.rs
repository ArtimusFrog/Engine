//! Die Truppen der Schattenfestung: Gruppen gegnerischer Einheiten entstehen im Festungshof,
//! marschieren durch die vier Tore, die Rampen hinab und die Straßen entlang. Wer das Ende der
//! Straße erreicht, bricht durch und kostet die Insel Leben.
//!
//! Die Einheiten haben Eigenschaften, auf die man mit passenden Türmen antworten muss: Harpyien
//! fliegen, Schattenmeuchler sind getarnt, Dunkelmagier heilen, Dunkle Ritter schirmen ihre
//! Nachbarn ab, Steingolems zerfallen in Felslinge. Jede fünfte Welle führt ein Boss mit eigener
//! Fähigkeit an. Soldaten der Kaserne und Barrikaden halten Gruppen auf wie Spieler.
//!
//! Der Server rechnet alles (`Heer`), die Clients bekommen mit jedem Schnappschuss die sichtbaren
//! Einheiten (`EnemyState`) und zeigen sie an (`HeerAnsicht`, auch die Soldaten der Kasernen).
//! Modelle: `game/assets/gegner/` (Blender: art/lib/gegner.py).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use engine::prelude::*;
use serde::{Deserialize, Serialize};

use crate::asset_files;
use crate::td::{Ereignis, Schwierigkeit, SoldatState, WellenBericht, Zielmodus, ZIEL_WELLE};
use crate::tuerme::DamageKind;
use crate::world::SoundEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EnemyKind {
    Knight,
    Archer,
    Pikeman,
    Skeleton,
    Warlock,
    Golem,
    Wolf,
    Ghost,
    /// Fliegt über die Straße (nur manche Türme treffen sie)
    Harpy,
    /// Getarnt: Türme sehen ihn erst, wenn er aufgedeckt ist
    Assassin,
    /// Bruchstück eines Steingolems
    Felsling,
}

impl EnemyKind {
    pub const ALL: [EnemyKind; 11] = [
        EnemyKind::Knight,
        EnemyKind::Archer,
        EnemyKind::Pikeman,
        EnemyKind::Skeleton,
        EnemyKind::Warlock,
        EnemyKind::Golem,
        EnemyKind::Wolf,
        EnemyKind::Ghost,
        EnemyKind::Harpy,
        EnemyKind::Assassin,
        EnemyKind::Felsling,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EnemyKind::Knight => "Dunkler Ritter",
            EnemyKind::Archer => "Bogenschütze",
            EnemyKind::Pikeman => "Pikenier",
            EnemyKind::Skeleton => "Skelettkrieger",
            EnemyKind::Warlock => "Dunkelmagier",
            EnemyKind::Golem => "Steingolem",
            EnemyKind::Wolf => "Schattenwolf",
            EnemyKind::Ghost => "Gespenst",
            EnemyKind::Harpy => "Harpyie",
            EnemyKind::Assassin => "Schattenmeuchler",
            EnemyKind::Felsling => "Felsling",
        }
    }

    /// Kurzname für die Wellenvorschau
    pub fn kurz(self) -> &'static str {
        match self {
            EnemyKind::Knight => "Ritter",
            EnemyKind::Archer => "Bogen",
            EnemyKind::Pikeman => "Pike",
            EnemyKind::Skeleton => "Skelett",
            EnemyKind::Warlock => "Magier",
            EnemyKind::Golem => "Golem",
            EnemyKind::Wolf => "Wolf",
            EnemyKind::Ghost => "Geist",
            EnemyKind::Harpy => "Harpyie",
            EnemyKind::Assassin => "Meuchler",
            EnemyKind::Felsling => "Felsling",
        }
    }

    /// Was an der Einheit besonders ist (für Vorschau und Tooltip).
    pub fn eigenschaft(self) -> Option<&'static str> {
        match self {
            EnemyKind::Harpy => Some("fliegt – nur Pfeil, Balliste, Frost, Blitz, Sonne, Arkan, Sturm und Späher treffen sie"),
            EnemyKind::Assassin => Some("getarnt – erst sichtbar im Umkreis eines Späherturms oder Sonnenturms (Läuterung)"),
            EnemyKind::Warlock => Some("heilt Einheiten in der Nähe – Gift und Läuterung verhindern das"),
            EnemyKind::Knight => Some("Schild schützt Nachbarn vor Pfeilen und Bolzen – Felsbrocken und Läuterung brechen ihn"),
            EnemyKind::Golem => Some("zerfällt beim Tod in drei Felslinge"),
            EnemyKind::Ghost => Some("Rüstung gegen Waffen, anfällig für Heiliges – Runen treffen es nicht"),
            EnemyKind::Skeleton => Some("untot: immun gegen Gift, doppelter Schaden durch Heiliges"),
            _ => None,
        }
    }

    fn file_name(self) -> &'static str {
        match self {
            EnemyKind::Knight => "dunkler_ritter",
            EnemyKind::Archer => "bogenschuetze",
            EnemyKind::Pikeman => "pikenier",
            EnemyKind::Skeleton => "skelettkrieger",
            EnemyKind::Warlock => "dunkelmagier",
            EnemyKind::Golem | EnemyKind::Felsling => "steingolem",
            EnemyKind::Wolf => "schattenwolf",
            EnemyKind::Ghost => "gespenst",
            EnemyKind::Harpy => "harpyie",
            EnemyKind::Assassin => "schattenmeuchler",
        }
    }

    /// Darstellungsgröße (Felslinge sind kleine Golems)
    fn groesse(self) -> f32 {
        if self == EnemyKind::Felsling { 0.55 } else { 1.0 }
    }

    pub fn max_health(self) -> f32 {
        match self {
            EnemyKind::Knight => 160.0,
            EnemyKind::Archer => 70.0,
            EnemyKind::Pikeman => 120.0,
            EnemyKind::Skeleton => 80.0,
            EnemyKind::Warlock => 90.0,
            EnemyKind::Golem => 420.0,
            EnemyKind::Wolf => 60.0,
            EnemyKind::Ghost => 100.0,
            EnemyKind::Harpy => 55.0,
            EnemyKind::Assassin => 75.0,
            EnemyKind::Felsling => 110.0,
        }
    }

    /// Rüstung gegen physischen Schaden (Anteil, der abgehalten wird).
    pub fn armor(self) -> f32 {
        match self {
            EnemyKind::Knight => 0.4,
            EnemyKind::Archer | EnemyKind::Skeleton | EnemyKind::Harpy => 0.1,
            EnemyKind::Pikeman => 0.25,
            EnemyKind::Golem => 0.6,
            EnemyKind::Felsling => 0.5,
            EnemyKind::Ghost => 0.75,
            EnemyKind::Assassin => 0.15,
            EnemyKind::Warlock | EnemyKind::Wolf => 0.0,
        }
    }

    pub fn undead(self) -> bool {
        matches!(self, EnemyKind::Skeleton | EnemyKind::Ghost)
    }

    pub fn fliegt(self) -> bool {
        self == EnemyKind::Harpy
    }

    pub fn getarnt(self) -> bool {
        self == EnemyKind::Assassin
    }

    fn schwer(self) -> bool {
        matches!(self, EnemyKind::Golem | EnemyKind::Felsling)
    }

    /// Wie stark eine Schadensart wirkt (1 = voll). `geschwaecht`: Rüstung durch Gift gemindert.
    pub fn factor(self, art: DamageKind, geschwaecht: f32) -> f32 {
        let ruestung = self.armor() * (1.0 - geschwaecht);
        match art {
            DamageKind::Physical => 1.0 - ruestung,
            DamageKind::Pierce => 1.0 - ruestung * 0.5,
            DamageKind::Arcane | DamageKind::Lightning if self == EnemyKind::Warlock => 0.6,
            DamageKind::Arcane | DamageKind::Lightning | DamageKind::Frost => 1.0,
            DamageKind::Holy if self.undead() => 2.0,
            DamageKind::Holy => 1.0,
            DamageKind::Poison if self == EnemyKind::Skeleton => 0.0,
            DamageKind::Poison => 1.0,
            DamageKind::Fire if self.schwer() => 0.5,
            DamageKind::Fire => 1.0,
        }
    }

    /// Marschtempo (m/s); eine Gruppe läuft so schnell wie ihr langsamstes Mitglied.
    fn speed(self) -> f32 {
        match self {
            EnemyKind::Golem => 2.0,
            EnemyKind::Felsling => 2.4,
            EnemyKind::Warlock => 2.4,
            EnemyKind::Wolf => 3.6,
            EnemyKind::Ghost => 2.8,
            EnemyKind::Harpy => 3.4,
            EnemyKind::Assassin => 3.2,
            _ => 2.6,
        }
    }

    /// Angriffsreichweite (m) und Pause zwischen zwei Angriffen (s).
    fn attack(self) -> (f32, f32) {
        match self {
            EnemyKind::Archer => (22.0, 2.6),
            EnemyKind::Warlock => (17.0, 3.2),
            EnemyKind::Golem => (3.4, 3.0),
            EnemyKind::Wolf => (2.6, 1.6),
            EnemyKind::Pikeman => (3.2, 2.0),
            EnemyKind::Harpy => (2.6, 1.8),
            EnemyKind::Assassin => (2.2, 1.2),
            EnemyKind::Felsling => (2.6, 2.2),
            _ => (2.4, 1.8),
        }
    }

    /// Schaden eines Angriffs auf Soldaten und Barrikaden (Welle 1).
    fn schlag(self) -> f32 {
        match self {
            EnemyKind::Knight => 14.0,
            EnemyKind::Archer => 8.0,
            EnemyKind::Pikeman => 12.0,
            EnemyKind::Skeleton => 9.0,
            EnemyKind::Warlock => 12.0,
            EnemyKind::Golem => 30.0,
            EnemyKind::Wolf => 8.0,
            EnemyKind::Ghost => 12.0,
            EnemyKind::Harpy => 7.0,
            EnemyKind::Assassin => 16.0,
            EnemyKind::Felsling => 12.0,
        }
    }

    /// Mittelpunkt und Radius der Trefferkugel über den Füßen.
    pub fn hit_sphere(self) -> (f32, f32) {
        match self {
            EnemyKind::Golem => (1.5, 1.2),
            EnemyKind::Felsling => (0.85, 0.7),
            EnemyKind::Wolf => (0.7, 0.8),
            EnemyKind::Harpy => (1.0, 0.75),
            _ => (1.0, 0.6),
        }
    }

    /// So viele Leben kostet ein Durchbruch.
    fn durchbruch(self, boss: bool) -> u32 {
        if boss {
            5
        } else if self == EnemyKind::Golem {
            3
        } else {
            1
        }
    }
}

/// Was eine Einheit gerade tut (für die Animation).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnemyAction {
    Idle,
    Walk,
    Attack,
    /// Besiegt: kippt um und verschwindet
    Dying,
}

/// Zustände einer Einheit (Bits in `EnemyState::flags`).
pub mod zustand {
    pub const FLIEGT: u16 = 1;
    pub const GETARNT: u16 = 2;
    pub const SCHILD: u16 = 4;
    pub const MARKIERT: u16 = 8;
    pub const BETAEUBT: u16 = 16;
    pub const BRENNT: u16 = 32;
    pub const VERGIFTET: u16 = 64;
    pub const VERLANGSAMT: u16 = 128;
    pub const UNVERWUNDBAR: u16 = 256;
    pub const WUT: u16 = 512;
    pub const GETEERT: u16 = 1024;
}

/// Was alle Rechner von einer Einheit wissen müssen.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnemyState {
    pub id: u16,
    pub kind: EnemyKind,
    pub position: Vec3,
    pub facing: f32,
    pub action: EnemyAction,
    /// Lebenspunkte in Prozent (0 = besiegt)
    pub health: u8,
    /// Anführer einer Bosswelle (größer dargestellt)
    pub boss: bool,
    /// Lebenspunkte (für Schadenszahlen)
    pub lp: u32,
    /// Zustände (`zustand::…`)
    pub flags: u16,
}

/// Wirkungen eines Treffers (Bits in `Hit::effekte`).
pub mod wirkung {
    /// Gift springt beim Tod als neue Wolke über (Giftturm: Seuche)
    pub const SEUCHE: u8 = 1;
    /// Brand springt beim Tod auf Nachbarn über (Feuerturm: Flächenbrand)
    pub const AUSBREITEN: u8 = 2;
    /// Bricht den Schild der Ritter
    pub const SCHILDBRUCH: u8 = 4;
    /// Holt Flieger für einige Sekunden auf den Boden
    pub const ERDEN: u8 = 8;
    /// Vergiftet (verhindert Heilung, macht Feuer explosiv)
    pub const GIFT: u8 = 16;
}

/// Ein Treffer: Schaden, Art und Nachwirkungen.
#[derive(Clone, Copy, Debug, Default)]
pub struct Hit {
    pub schaden: f32,
    pub art: DamageKind,
    /// Verlangsamung (Anteil) für 2,5 s
    pub bremse: f32,
    /// Brand: Schaden pro Sekunde für `dauer` Sekunden
    pub brand: f32,
    pub dauer: f32,
    /// Betäubung (Sekunden)
    pub stun: f32,
    /// `wirkung::…`
    pub effekte: u8,
}

impl Default for DamageKind {
    fn default() -> Self {
        DamageKind::Physical
    }
}

/// Wer getroffen hat: Spieler (klein geschrieben) und ggf. der Turm.
#[derive(Clone, Copy, Debug)]
pub struct Quelle<'a> {
    pub name: &'a str,
    pub turm: Option<u32>,
}

/// Welche Einheiten ein Turm treffen kann.
#[derive(Clone, Copy, Debug)]
pub struct Filter {
    pub flieger: bool,
    pub boden: bool,
    pub geister: bool,
    /// Auch Getarnte, die nicht aufgedeckt sind (Soldaten, Spieler)
    pub getarnte: bool,
}

impl Filter {
    pub const ALLE: Filter = Filter { flieger: true, boden: true, geister: true, getarnte: false };
    pub const BODEN: Filter = Filter { flieger: false, boden: true, geister: true, getarnte: false };
    pub const BODEN_OHNE_GEISTER: Filter = Filter { flieger: false, boden: true, geister: false, getarnte: false };
    pub const NAHKAMPF: Filter = Filter { flieger: false, boden: true, geister: true, getarnte: true };
}

/// Eine besiegte Einheit (für Beute, Gold und Wirkungen beim Tod).
#[derive(Clone, Debug)]
pub struct Gefallen {
    pub von: String,
    pub turm: Option<u32>,
    pub kind: EnemyKind,
    pub boss: bool,
    pub welle: u32,
    pub ort: Vec3,
    /// Späher (Kopfgeldjäger): zusätzliches Kopfgeld (Anteil)
    pub bonus: f32,
    /// Seuche: war mit Seuchengift vergiftet
    pub seuche: bool,
    /// Flächenbrand: brannte beim Tod
    pub ausbreiten: bool,
}

/// Was eine Gruppe aufhält und angegriffen werden kann.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ziel {
    Spieler,
    Soldat(u16),
    Barrikade(u32),
}

#[derive(Clone, Copy, Debug)]
pub struct Blocker {
    pub ort: Vec3,
    pub ziel: Ziel,
}

// ---------- Marschrouten ----------

/// Weg einer Gruppe: Festungshof → Tor → Rampe → Straße bis zum Ende.
#[derive(Clone, Debug)]
pub struct Route {
    points: Vec<Vec3>,
    /// Zurückgelegte Strecke bis zu jedem Punkt
    along: Vec<f32>,
    /// Richtung des Tores von der Festungsmitte aus (x, z)
    achse: Vec2,
}

/// Ab diesem Punkt der Route beginnt die Straße (davor Hof, Tor und Rampe).
const STRASSE_AB: usize = 4;

impl Route {
    /// `axis`: Richtung des Tores (x, z) von der Festungsmitte aus; `ground`: Höhe der Festung
    /// (Felsplateau = ground + 11); `road`: die Straße ab dem Fuß der Rampe.
    pub fn new(axis: Vec2, ground: f32, road: &[Vec2], height: impl Fn(Vec2) -> f32) -> Route {
        let hof = ground + 11.0;
        let at = |d: f32, y: f32| {
            let p = axis * d;
            vec3(p.x, y, p.y)
        };
        let mut points = vec![at(18.0, hof), at(25.9, hof), at(30.0, hof), at(33.9, hof), at(69.9, ground)];
        points.extend(road.iter().map(|&p| vec3(p.x, height(p), p.y)));
        let mut along = Vec::with_capacity(points.len());
        let mut total = 0.0;
        for (i, p) in points.iter().enumerate() {
            // Waagerechte Strecke: so dauert der Marsch auf jeder Straße gleich lang, egal wie bergig
            if i > 0 {
                total += vec2(p.x - points[i - 1].x, p.z - points[i - 1].z).length();
            }
            along.push(total);
        }
        Route { points, along, achse: axis }
    }

    pub fn length(&self) -> f32 {
        *self.along.last().unwrap_or(&0.0)
    }

    /// Ort und Richtung (x, z) nach `distance` Metern.
    pub fn sample(&self, distance: f32) -> (Vec3, Vec2) {
        let d = distance.clamp(0.0, self.length());
        let i = self.along.partition_point(|&a| a <= d).clamp(1, self.points.len() - 1);
        let (a, b) = (self.points[i - 1], self.points[i]);
        let span = (self.along[i] - self.along[i - 1]).max(1e-4);
        let t = (d - self.along[i - 1]) / span;
        let dir = vec2(b.x - a.x, b.z - a.z).normalize_or(Vec2::Y);
        (a.lerp(b, t), dir)
    }

    /// Liegt `distance` schon auf der Straße (nicht mehr auf Hof, Tor oder Rampe)?
    fn auf_strasse(&self, distance: f32) -> bool {
        distance >= self.along[STRASSE_AB]
    }

    /// Himmelsrichtung der Straße (für Anzeige und Meldungen)
    pub fn name(&self) -> &'static str {
        if self.achse.y.abs() >= self.achse.x.abs() {
            if self.achse.y > 0.0 { "Süd" } else { "Nord" }
        } else if self.achse.x > 0.0 {
            "Ost"
        } else {
            "West"
        }
    }
}

// ---------- Die Truppen (nur auf dem Server) ----------

/// Eine Gruppenvorlage (vorne → hinten) und ab welcher Welle sie vorkommt.
struct Vorlage {
    einheiten: &'static [EnemyKind],
    ab: u32,
}

impl Vorlage {
    fn fliegend(&self) -> bool {
        self.einheiten.iter().all(|k| k.fliegt())
    }
}

use EnemyKind::*;
const GRUPPEN: [Vorlage; 11] = [
    Vorlage { einheiten: &[Knight, Knight, Pikeman, Pikeman, Archer, Archer], ab: 1 },
    Vorlage { einheiten: &[Skeleton, Skeleton, Skeleton, Skeleton, Ghost], ab: 1 },
    Vorlage { einheiten: &[Wolf, Wolf, Wolf, Wolf], ab: 1 },
    Vorlage { einheiten: &[Knight, Knight, Warlock, Warlock], ab: 3 },
    Vorlage { einheiten: &[Golem, Pikeman, Pikeman, Archer], ab: 4 },
    Vorlage { einheiten: &[Harpy, Harpy, Harpy, Harpy, Harpy], ab: 4 },
    Vorlage { einheiten: &[Ghost, Ghost, Skeleton, Warlock, Skeleton], ab: 5 },
    Vorlage { einheiten: &[Assassin, Assassin, Assassin, Wolf, Wolf], ab: 6 },
    Vorlage { einheiten: &[Knight, Knight, Assassin, Assassin, Warlock], ab: 8 },
    Vorlage { einheiten: &[Golem, Golem, Warlock, Knight], ab: 11 },
    Vorlage { einheiten: &[Harpy, Harpy, Harpy, Harpy, Harpy, Harpy, Harpy], ab: 12 },
];
/// Abstand zwischen zwei Wellen (s).
pub const WAVE_SECONDS: f32 = 45.0;
/// Leben der Insel auf „Normal“ (siehe `Schwierigkeit::leben`).
pub const MAX_LEBEN: u32 = 20;
/// Anführer der Bosswellen (jede fünfte Welle, reihum)
const BOSSE: [EnemyKind; 4] = [Golem, Knight, Warlock, Ghost];
/// Obergrenze gleichzeitiger Einheiten (darüber fällt eine Welle aus)
const MAX_ENEMIES: usize = 140;
/// Ab dieser Entfernung zu einem Spieler bleibt eine Gruppe stehen und kämpft.
const ENGAGE: f32 = 16.0;
/// So nah muss die Spitze an Soldaten oder einer Barrikade sein, damit die Gruppe stehen bleibt.
const SPERRE: f32 = 4.5;
/// Flughöhe der Harpyien über dem Boden
const FLUGHOEHE: f32 = 5.0;

struct Member {
    id: u16,
    kind: EnemyKind,
    /// Anführer einer Bosswelle
    boss: bool,
    welle: u32,
    /// Zähigkeit der Welle (Faktor auf die Grundlebenspunkte) und Schlagkraft
    zaeh: f32,
    staerke: f32,
    max_health: f32,
    health: f32,
    /// Platz in der Formation (Reihe hinter der Spitze, seitlich)
    row: f32,
    side: f32,
    cooldown: f32,
    /// Wie weit die Einheit auf ihrer Route ist (für die Zielwahl der Türme)
    progress: f32,
    /// Verlangsamung (Anteil, Restzeit), Brand (Schaden/s, Restzeit), Rüstung geschwächt (Anteil, Restzeit)
    slow: (f32, f32),
    burn: (f32, f32),
    burn_by: Option<(String, Option<u32>)>,
    burn_ausbreiten: bool,
    weak: (f32, f32),
    /// Restzeiten: vergiftet, geteert, betäubt, aufgedeckt, Heilsperre, Schild gebrochen, am Boden
    gift: f32,
    seuche: bool,
    geteert: f32,
    stun: f32,
    aufgedeckt: f32,
    heilsperre: f32,
    schild_bruch: f32,
    geerdet: f32,
    unverwundbar: f32,
    /// Markiert (Aufschlag auf jeden Schaden, Restzeit) und Kopfgeldbonus
    mark: (f32, f32),
    mark_bonus: f32,
    /// Steht im Schutz eines Ritterschilds (jeden Takt neu berechnet)
    geschuetzt: bool,
    /// Ritter-Boss in Wut
    wut: bool,
    /// Bossfähigkeit bzw. Heilung: Zeit bis zum nächsten Mal
    faehigkeit: f32,
    heilen: f32,
    /// Aktuelle Flughöhe (Harpyien)
    hoehe: f32,
    /// Seit wann besiegt (dann nach kurzer Zeit weg)
    dying: Option<f32>,
    /// Weicht zum Kämpfen von seinem Platz ab
    offset: Vec2,
    attacking: f32,
    position: Vec3,
    facing: f32,
}

impl Member {
    fn fliegt_jetzt(&self) -> bool {
        self.kind.fliegt() && self.geerdet <= 0.0
    }

    fn passt(&self, filter: Filter) -> bool {
        self.dying.is_none()
            && (if self.fliegt_jetzt() { filter.flieger } else { filter.boden })
            && (filter.geister || self.kind != Ghost)
            && (filter.getarnte || !self.kind.getarnt() || self.aufgedeckt > 0.0)
    }

    fn center(&self) -> Vec3 {
        self.position + Vec3::Y * self.kind.hit_sphere().0 * self.kind.groesse() * if self.boss { 1.6 } else { 1.0 }
    }

    fn flags(&self) -> u16 {
        use zustand::*;
        let mut f = 0;
        let mut setze = |an: bool, bit: u16| {
            if an {
                f |= bit;
            }
        };
        setze(self.fliegt_jetzt(), FLIEGT);
        setze(self.kind.getarnt() && self.aufgedeckt <= 0.0, GETARNT);
        setze(self.geschuetzt, SCHILD);
        setze(self.mark.1 > 0.0, MARKIERT);
        setze(self.stun > 0.0, BETAEUBT);
        setze(self.burn.1 > 0.0, BRENNT);
        setze(self.gift > 0.0, VERGIFTET);
        setze(self.slow.1 > 0.0, VERLANGSAMT);
        setze(self.unverwundbar > 0.0, UNVERWUNDBAR);
        setze(self.wut, WUT);
        setze(self.geteert > 0.0, GETEERT);
        f
    }
}

struct Group {
    route: usize,
    distance: f32,
    speed: f32,
    halted: bool,
    /// Nur Flieger: bleiben nie stehen
    fliegend: bool,
    members: Vec<Member>,
}

/// Ein Angriff einer Einheit (für Geschosse, Klang und den Schaden an Soldaten und Barrikaden).
#[derive(Clone, Copy, Debug)]
pub struct Strike {
    pub kind: EnemyKind,
    pub from: Vec3,
    pub target: Vec3,
    pub ziel: Ziel,
    pub schaden: f32,
}

/// Zählt eine Welle mit, bis ihre letzte Einheit besiegt oder durchgebrochen ist.
#[derive(Default)]
struct Zaehler {
    besiegt: u32,
    durch: u32,
    schaden: BTreeMap<String, f32>,
    kills_turm: BTreeMap<u32, u32>,
}

pub struct Heer {
    routes: Vec<Route>,
    groups: Vec<Group>,
    next_id: u16,
    timer: f32,
    /// Wellen aus der Festung an (Admin-Panel)
    pub enabled: bool,
    rng: Rng,
    /// Besiegte Einheiten seit dem letzten Abholen
    pub gefallen: Vec<Gefallen>,
    /// Nummer der zuletzt losgeschickten Welle (0 = noch keine) und die Leben jeder Straße
    /// (jede Lane für sich – es gibt keinen gemeinsamen Pool)
    pub welle: u32,
    pub leben: Vec<u32>,
    /// Straßen, die in diesem Takt gefallen sind (der Server zerstört dort die Siedlung)
    pub gefallene_lanes: Vec<usize>,
    /// Meldungen für alle Spieler (neue Welle, Durchbruch, Niederlage)
    pub meldungen: Vec<String>,
    pub schwierigkeit: Schwierigkeit,
    /// Nach Welle 30 weiter (sonst ist dann Schluss: Sieg)
    pub endlos: bool,
    pub sieg: bool,
    /// Gerade gesiegt (der Server verteilt die Belohnung)
    pub sieg_neu: bool,
    /// Spieler auf der Insel (mehr Spieler = zähere Truppen)
    pub spieler: usize,
    /// Vorlagen der nächsten Welle je Straße (für die Vorschau schon ausgewählt)
    geplant: Vec<usize>,
    /// Ereignisse zum Anzeigen seit dem letzten Abholen
    pub ereignisse: Vec<Ereignis>,
    /// Stampfen des Golem-Bosses: Mitte, Radius, Dauer (die Türme dort sind lahm)
    pub stampfer: Vec<(Vec3, f32, f32)>,
    /// Schaden und Kills je Turm und je Spieler (seit Welle 1)
    pub turm_stats: HashMap<u32, (f32, u32)>,
    pub beitrag: BTreeMap<String, (f32, u32)>,
    zaehler: BTreeMap<u32, Zaehler>,
    /// Fertige Auswertungen: Bericht und der beste Turm (ID, Kills) – Namen setzt der Server ein
    pub berichte: Vec<(WellenBericht, Option<(u32, u32)>)>,
    /// Gift + Feuer: Explosionen, die noch Schaden machen (Mitte, Schaden, Spieler, Turm)
    explosionen: Vec<(Vec3, f32, String, Option<u32>)>,
    explodiert_gerade: bool,
    /// Leben, die in diesem Takt durch Durchbrüche verloren gingen (je Straße)
    pub durchbrueche: Vec<(usize, u32)>,
}

impl Heer {
    pub fn new(routes: Vec<Route>) -> Heer {
        let lanes = routes.len();
        let mut heer = Heer {
            routes,
            groups: Vec::new(),
            next_id: 1,
            timer: 3.0,
            enabled: false,
            rng: Rng::new(0x7E_E4),
            gefallen: Vec::new(),
            welle: 0,
            leben: vec![MAX_LEBEN; lanes],
            gefallene_lanes: Vec::new(),
            meldungen: Vec::new(),
            schwierigkeit: Schwierigkeit::Normal,
            endlos: false,
            sieg: false,
            sieg_neu: false,
            spieler: 1,
            geplant: Vec::new(),
            ereignisse: Vec::new(),
            stampfer: Vec::new(),
            turm_stats: HashMap::new(),
            beitrag: BTreeMap::new(),
            zaehler: BTreeMap::new(),
            berichte: Vec::new(),
            explosionen: Vec::new(),
            explodiert_gerade: false,
            durchbrueche: Vec::new(),
        };
        heer.plane(1);
        heer
    }

    /// Die Heerstraßen (ab dem Fuß der Rampe) als Linien (x, z) – für die Karte.
    pub fn strassen(&self) -> impl Iterator<Item = Vec<Vec2>> + '_ {
        self.routes.iter().map(|r| r.points.iter().skip(STRASSE_AB + 1).map(|p| vec2(p.x, p.z)).collect())
    }

    /// Namen der Straßen (Süd, Ost, Nord, West) in der Reihenfolge der Routen.
    pub fn strassen_namen(&self) -> Vec<&'static str> {
        self.routes.iter().map(Route::name).collect()
    }

    pub fn count(&self) -> usize {
        self.groups.iter().map(|g| g.members.len()).sum()
    }

    /// Wellen an/aus; beim Einschalten kommt sofort die erste.
    pub fn set_enabled(&mut self, on: bool) {
        if on && !self.enabled {
            if self.sieg && !self.endlos {
                self.reset();
            }
            self.spawn_wave();
        }
        self.enabled = on;
    }

    /// Sekunden bis zur nächsten Welle
    pub fn naechste_in(&self) -> f32 {
        self.timer.max(0.0)
    }

    /// Enden der Heerstraßen (dort stehen die Schutzsteine): Ort und Richtung der Straße.
    pub fn enden(&self) -> Vec<(Vec3, Vec2)> {
        self.routes.iter().map(|r| r.sample(r.length())).collect()
    }

    /// Nächster Punkt auf einer Heerstraße: Ort (Höhe der Route), Richtung, Strecke entlang der Route.
    pub fn naechster_strassenpunkt(&self, at: Vec2) -> Option<(Vec3, Vec2, f32)> {
        let mut best: Option<(f32, Vec3, Vec2, f32)> = None;
        for route in &self.routes {
            for i in (STRASSE_AB + 1)..route.points.len() {
                let (a, b) = (route.points[i - 1], route.points[i]);
                let (a2, b2) = (vec2(a.x, a.z), vec2(b.x, b.z));
                let ab = b2 - a2;
                let t = ((at - a2).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
                let d = (a2 + ab * t).distance(at);
                if best.is_none_or(|b| d < b.0) {
                    best = Some((d, a.lerp(b, t), ab.normalize_or(Vec2::Y), route.along[i - 1] + (route.along[i] - route.along[i - 1]) * t));
                }
            }
        }
        best.map(|(_, p, dir, along)| (p, dir, along))
    }

    /// Zähigkeit der Truppen in einer Welle: +12 % je Welle, mehr Spieler und Schwierigkeit.
    fn zaehigkeit(&self, welle: u32) -> f32 {
        let grund = 1.0 + 0.12 * welle.saturating_sub(1) as f32;
        let endlos = 1.0 + 0.1 * welle.saturating_sub(ZIEL_WELLE) as f32;
        let spieler = 1.0 + 0.3 * self.spieler.saturating_sub(1) as f32;
        grund * endlos * spieler * self.schwierigkeit.zaehigkeit()
    }

    fn boss_fuer(welle: u32) -> Option<EnemyKind> {
        (welle % 5 == 0 && welle > 0).then(|| BOSSE[((welle / 5 - 1) % BOSSE.len() as u32) as usize])
    }

    /// Vorlagen für die Welle `welle` auswählen (je Straße eine).
    fn plane(&mut self, welle: u32) {
        let boss = Self::boss_fuer(welle).is_some();
        let moeglich: Vec<usize> = GRUPPEN.iter().enumerate().filter(|(_, g)| g.ab <= welle && !(boss && g.fliegend())).map(|(i, _)| i).collect();
        self.geplant = (0..self.routes.len()).map(|_| moeglich[(self.rng.next_u32() % moeglich.len() as u32) as usize]).collect();
    }

    /// Einheiten einer Gruppe in der Welle `welle` (mehr mit jeder dritten Welle, der Boss vorneweg).
    fn aufstellung(vorlage: usize, welle: u32) -> Vec<(EnemyKind, bool)> {
        let einheiten = GRUPPEN[vorlage].einheiten;
        let extra = ((welle.saturating_sub(1)) / 3).min(6) as usize + (welle.saturating_sub(ZIEL_WELLE) / 2).min(8) as usize;
        let mut liste: Vec<(EnemyKind, bool)> = Self::boss_fuer(welle).iter().map(|&k| (k, true)).collect();
        liste.extend(einheiten.iter().chain(einheiten.iter().cycle().take(extra)).map(|&k| (k, false)));
        liste
    }

    /// Was in der nächsten Welle kommt: Einheiten je Art und der Boss.
    pub fn vorschau(&self) -> (Vec<(EnemyKind, u16)>, Option<EnemyKind>) {
        let welle = self.welle + 1;
        let mut anzahl: BTreeMap<EnemyKind, u16> = BTreeMap::new();
        for &vorlage in &self.geplant {
            for (kind, boss) in Self::aufstellung(vorlage, welle) {
                if !boss {
                    *anzahl.entry(kind).or_default() += 1;
                }
            }
        }
        (anzahl.into_iter().collect(), Self::boss_fuer(welle))
    }

    /// Wellen zurück auf Anfang (Admin): Welle 0, volle Leben, alle Truppen weg.
    pub fn reset(&mut self) {
        self.groups.clear();
        self.zaehler.clear();
        self.welle = 0;
        self.leben = vec![self.schwierigkeit.leben(); self.routes.len()];
        self.timer = 5.0;
        self.sieg = false;
        self.beitrag.clear();
        self.plane(1);
    }

    /// Schwierigkeit ändern (setzt die Wellen zurück).
    pub fn set_schwierigkeit(&mut self, schwierigkeit: Schwierigkeit) {
        self.schwierigkeit = schwierigkeit;
        self.reset();
        self.meldungen.push(format!("Schwierigkeit: {} – {} Leben. Die Wellen beginnen von vorn.", schwierigkeit.label(), schwierigkeit.leben()));
    }

    /// Die nächste Welle sofort (ein Spieler ruft sie): liefert die gesparten Sekunden.
    pub fn rufen(&mut self) -> Option<f32> {
        if !self.enabled || self.timer < 3.0 || (self.sieg && !self.endlos) {
            return None;
        }
        let gespart = self.timer;
        self.spawn_wave();
        Some(gespart)
    }

    /// Eine Welle: auf jeder Straße eine Gruppe. Mit jeder Welle werden die Truppen zäher
    /// (+12 % Leben), alle drei Wellen kommt je Gruppe eine Einheit dazu, jede fünfte Welle
    /// führt ein Boss an. Die nächste Welle kommt frühestens nach `WAVE_SECONDS`.
    pub fn spawn_wave(&mut self) {
        self.timer = WAVE_SECONDS;
        if self.count() > MAX_ENEMIES || (self.sieg && !self.endlos) {
            return;
        }
        self.welle += 1;
        let welle = self.welle;
        let zaeh = self.zaehigkeit(welle);
        let staerke = 1.0 + 0.04 * (welle - 1) as f32;
        let boss = Self::boss_fuer(welle);
        self.meldungen.push(match boss {
            Some(kind) => format!("Welle {welle}: Bosswelle! Ein gewaltiger {} führt die Truppen an.", kind.label()),
            None => format!("Welle {welle} bricht aus der Schattenfestung hervor."),
        });
        for route in 0..self.routes.len() {
            let vorlage = self.geplant.get(route).copied().unwrap_or(0);
            let template = Self::aufstellung(vorlage, welle);
            let columns = if template.len() >= 5 { 3.0 } else { 2.0 };
            let mut members = Vec::with_capacity(template.len());
            for (i, &(kind, boss)) in template.iter().enumerate() {
                let row = (i as f32 / columns).floor();
                let side = (i as f32 % columns) - (columns - 1.0) / 2.0;
                members.push(self.neues_mitglied(kind, boss, welle, zaeh, staerke, row, side));
            }
            let fliegend = members.iter().all(|m| m.kind.fliegt());
            let speed = members.iter().map(|m| m.kind.speed()).fold(f32::MAX, f32::min);
            // Die hinteren Reihen starten weiter drinnen im Hof
            let rows = members.iter().map(|m| m.row).fold(0.0, f32::max);
            self.groups.push(Group { route, distance: rows * 2.4, speed, halted: false, fliegend, members });
        }
        self.zaehler.insert(welle, Zaehler::default());
        self.plane(welle + 1);
    }

    #[allow(clippy::too_many_arguments)]
    fn neues_mitglied(&mut self, kind: EnemyKind, boss: bool, welle: u32, zaeh: f32, staerke: f32, row: f32, side: f32) -> Member {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let max_health = kind.max_health() * zaeh * if boss { 6.0 } else { 1.0 };
        Member {
            id,
            kind,
            boss,
            welle,
            zaeh,
            staerke,
            max_health,
            health: max_health,
            row,
            side,
            cooldown: self.rng.range(0.3, 1.5),
            progress: 0.0,
            slow: (0.0, 0.0),
            burn: (0.0, 0.0),
            burn_by: None,
            burn_ausbreiten: false,
            weak: (0.0, 0.0),
            gift: 0.0,
            seuche: false,
            geteert: 0.0,
            stun: 0.0,
            aufgedeckt: 0.0,
            heilsperre: 0.0,
            schild_bruch: 0.0,
            geerdet: 0.0,
            unverwundbar: 0.0,
            mark: (0.0, 0.0),
            mark_bonus: 0.0,
            geschuetzt: false,
            wut: false,
            faehigkeit: if boss { 6.0 } else { self.rng.range(2.0, 4.0) },
            heilen: self.rng.range(2.0, 4.0),
            hoehe: if kind.fliegt() { FLUGHOEHE } else { 0.0 },
            dying: None,
            offset: Vec2::ZERO,
            attacking: 0.0,
            position: Vec3::ZERO,
            facing: 0.0,
        }
    }

    /// Ein Takt: Wellen, Fähigkeiten, Marsch, Kampf. `blockers`: Spieler, Soldaten und Barrikaden;
    /// `boden`: Geländehöhe (die Einheiten stehen auf dem Boden, nicht auf der Mittellinie der Straße).
    pub fn tick(&mut self, dt: f32, blockers: &[Blocker], boden: &dyn Fn(Vec2) -> f32) -> Vec<Strike> {
        let mut strikes = Vec::new();
        if self.enabled {
            self.timer -= dt;
            if self.timer <= 0.0 {
                self.spawn_wave();
            }
        }
        self.schilde();
        self.faehigkeiten(dt);
        let gefallen = &mut self.gefallen;
        let mut spaltungen = Vec::new();
        for (gi, group) in self.groups.iter_mut().enumerate() {
            let route = &self.routes[group.route];
            let (lead, _) = route.sample(group.distance);
            // Stehen bleiben, sobald ein Spieler nah an der Spitze ist, oder vor Soldaten und Barrikaden
            group.halted = !group.fliegend
                && blockers.iter().any(|b| match b.ziel {
                    Ziel::Spieler => b.ort.distance(lead) < ENGAGE - 2.0,
                    _ => vec2(b.ort.x - lead.x, b.ort.z - lead.z).length() < SPERRE,
                });
            // Die Gruppe ist so schnell wie ihr langsamstes (ggf. vereistes oder betäubtes) Mitglied
            group.speed = group
                .members
                .iter()
                .filter(|m| m.dying.is_none())
                .map(|m| if m.stun > 0.0 { 0.0 } else { m.kind.speed() * (1.0 - m.slow.0) })
                .fold(f32::MAX, f32::min)
                .min(4.0);
            if !group.halted {
                group.distance += group.speed * dt;
            }
            for member in &mut group.members {
                if let Some(t) = &mut member.dying {
                    *t += dt;
                    // Besiegte Flieger stürzen ab
                    member.hoehe = (member.hoehe - dt * 9.0).max(0.0);
                    member.position.y = boden(vec2(member.position.x, member.position.z)) + member.hoehe;
                    continue;
                }
                // Nachwirkungen laufen ab, Brand zehrt
                for (wert, rest) in [&mut member.slow, &mut member.weak, &mut member.mark] {
                    *rest -= dt;
                    if *rest <= 0.0 {
                        *wert = 0.0;
                        *rest = 0.0;
                    }
                }
                for t in [
                    &mut member.gift,
                    &mut member.geteert,
                    &mut member.stun,
                    &mut member.aufgedeckt,
                    &mut member.heilsperre,
                    &mut member.schild_bruch,
                    &mut member.geerdet,
                    &mut member.unverwundbar,
                ] {
                    *t = (*t - dt).max(0.0);
                }
                if member.gift <= 0.0 {
                    member.seuche = false;
                }
                if member.burn.1 > 0.0 {
                    member.burn.1 -= dt;
                    let teer = if member.geteert > 0.0 { 1.5 } else { 1.0 };
                    let schaden = (member.burn.0 * dt * member.kind.factor(DamageKind::Fire, 0.0) * teer).min(member.health);
                    member.health -= schaden;
                    let (name, turm) = member.burn_by.clone().unwrap_or_default();
                    statistik(&mut self.beitrag, &mut self.turm_stats, &mut self.zaehler, &name, turm, member.welle, schaden, false);
                    if member.health <= 0.0 {
                        member.health = 0.0;
                        member.dying = Some(0.0);
                        statistik(&mut self.beitrag, &mut self.turm_stats, &mut self.zaehler, &name, turm, member.welle, 0.0, true);
                        gefallen.push(Gefallen {
                            von: name,
                            turm,
                            kind: member.kind,
                            boss: member.boss,
                            welle: member.welle,
                            ort: member.center(),
                            bonus: member.mark_bonus,
                            seuche: member.seuche,
                            ausbreiten: member.burn_ausbreiten,
                        });
                        if member.kind == Golem {
                            spaltungen.push((gi, member.row, member.side, member.boss, member.welle, member.zaeh, member.staerke, member.center()));
                        }
                        continue;
                    }
                } else {
                    member.burn_ausbreiten = false;
                }
                member.progress = group.distance - member.row * 2.4;
                let (spot, dir) = route.sample(member.progress);
                let side = vec2(-dir.y, dir.x) * member.side * 1.5;
                let home = vec2(spot.x, spot.z) + side;
                member.cooldown -= dt;
                member.attacking = (member.attacking - dt).max(0.0);
                let (range, pause) = member.kind.attack();
                let fliegt = member.fliegt_jetzt();
                let target = if member.stun > 0.0 {
                    None
                } else {
                    blockers
                        .iter()
                        .copied()
                        .filter(|b| !fliegt || b.ziel == Ziel::Spieler)
                        .filter(|b| vec2(b.ort.x, b.ort.z).distance(home + member.offset) < ENGAGE + range + 4.0)
                        .min_by(|a, b| a.ort.distance(member.position).total_cmp(&b.ort.distance(member.position)))
                };
                let mut facing_dir = dir;
                match target {
                    Some(ziel) => {
                        let to = vec2(ziel.ort.x, ziel.ort.z) - (home + member.offset);
                        let distance = to.length();
                        // Nahkämpfer gehen ein Stück auf das Ziel zu, Fernkämpfer bleiben stehen
                        if distance > range * 0.8 && range < 5.0 && !group.fliegend {
                            let step = to.normalize_or_zero() * member.kind.speed() * dt;
                            member.offset = (member.offset + step).clamp_length_max(ENGAGE);
                        }
                        facing_dir = to.normalize_or(dir);
                        if distance <= range && member.cooldown <= 0.0 {
                            member.cooldown = pause;
                            member.attacking = 0.9;
                            let schaden = member.kind.schlag() * member.staerke * if member.wut { 2.0 } else { 1.0 } * if member.boss { 2.5 } else { 1.0 };
                            strikes.push(Strike { kind: member.kind, from: member.position + Vec3::Y * 1.3, target: ziel.ort, ziel: ziel.ziel, schaden });
                        }
                    }
                    None => member.offset *= (1.0 - dt * 1.5).max(0.0),
                }
                let flat = home + member.offset;
                // Auf der Straße steht jede Einheit auf dem Gelände unter ihren Füßen; im Hof, am Tor
                // und auf der Rampe gilt die Höhe der Route (dort liegt der Boden nicht im Gelände)
                let grund = if route.auf_strasse(member.progress) { boden(flat) } else { spot.y };
                let ziel_hoehe = if fliegt { FLUGHOEHE } else { 0.0 };
                member.hoehe += (ziel_hoehe - member.hoehe).clamp(-dt * 6.0, dt * 4.0);
                let wippen = if member.kind.fliegt() { (member.progress * 0.9 + member.id as f32).sin() * 0.3 * (member.hoehe / FLUGHOEHE) } else { 0.0 };
                member.position = vec3(flat.x, grund + member.hoehe + wippen, flat.y);
                member.facing = facing_dir.x.atan2(facing_dir.y);
            }
            group.members.retain(|m| m.dying.is_none_or(|t| t < 2.0));
        }
        for (gi, row, side, boss, welle, zaeh, staerke, ort) in spaltungen {
            self.spalten(gi, row, side, boss, welle, zaeh, staerke, ort);
        }
        self.explodieren();
        // Am Ende der Straße: Durchbruch – diese Straße verliert Leben, die Einheit verschwindet
        self.durchbrueche.clear();
        for group in &mut self.groups {
            let route = &self.routes[group.route];
            let ende = route.length();
            let distance = group.distance;
            let mut hier = 0;
            group.members.retain(|m| {
                let durch = m.dying.is_none() && distance - m.row * 2.4 >= ende;
                if durch {
                    let wert = m.kind.durchbruch(m.boss);
                    hier += wert;
                    if let Some(z) = self.zaehler.get_mut(&m.welle) {
                        z.durch += 1;
                    }
                    self.meldungen.push(format!(
                        "{}{} ist auf der Straße {} durchgebrochen! (−{wert} Leben)",
                        if m.boss { "Der Boss " } else { "" },
                        m.kind.label(),
                        route.name()
                    ));
                }
                !durch
            });
            if hier > 0 {
                self.durchbrueche.push((group.route, hier));
            }
        }
        self.groups.retain(|g| !g.members.is_empty());
        // Jede Straße hat ihre eigenen Leben. Fällt eine, verschwinden ihre Truppen, ihre Siedlung
        // wird zerstört (Server) und sie fängt mit vollen Leben neu an – die anderen spielen weiter.
        let voll = self.schwierigkeit.leben();
        self.leben.resize(self.routes.len(), voll);
        for (lane, verlust) in std::mem::take(&mut self.durchbrueche).into_iter().fold(Vec::<(usize, u32)>::new(), |mut summe, (lane, n)| {
            match summe.iter_mut().find(|s| s.0 == lane) {
                Some(s) => s.1 += n,
                None => summe.push((lane, n)),
            }
            summe
        }) {
            self.durchbrueche.push((lane, verlust));
            self.leben[lane] = self.leben[lane].saturating_sub(verlust);
            if self.leben[lane] == 0 {
                let name = self.routes[lane].name();
                self.meldungen.push(format!("Die Straße {name} ist gefallen! Ihre Siedlung wird zerstört – die Straße fängt mit vollen Leben neu an."));
                self.groups.retain(|g| g.route != lane);
                self.leben[lane] = voll;
                self.gefallene_lanes.push(lane);
            }
        }
        self.auswerten();
        strikes
    }

    /// Ritter schirmen Einheiten im Umkreis von 5 m ab (solange ihr Schild nicht gebrochen ist).
    fn schilde(&mut self) {
        for group in &mut self.groups {
            let ritter: Vec<Vec3> = group.members.iter().filter(|m| m.dying.is_none() && m.kind == Knight && m.schild_bruch <= 0.0).map(|m| m.position).collect();
            for m in &mut group.members {
                m.geschuetzt = m.schild_bruch <= 0.0 && ritter.iter().any(|r| r.distance(m.position) < 5.0);
            }
        }
    }

    /// Heilung der Dunkelmagier und die Fähigkeiten der Bosse.
    fn faehigkeiten(&mut self, dt: f32) {
        let mut heilungen = Vec::new();
        let mut beschwoerungen = Vec::new();
        for (gi, group) in self.groups.iter_mut().enumerate() {
            for m in group.members.iter_mut().filter(|m| m.dying.is_none() && m.stun <= 0.0) {
                if m.kind == Warlock {
                    m.heilen -= dt;
                    if m.heilen <= 0.0 {
                        m.heilen = if m.boss { 3.0 } else { 4.0 };
                        heilungen.push((gi, m.position, if m.boss { 11.0 } else { 8.0 }, if m.boss { 0.1 } else { 0.06 }));
                    }
                }
                if !m.boss {
                    continue;
                }
                m.faehigkeit -= dt;
                match m.kind {
                    Knight if !m.wut && m.health < m.max_health * 0.5 => {
                        m.wut = true;
                        self.ereignisse.push(Ereignis::Wut(m.center()));
                        self.meldungen.push("Der Dunkle Ritter gerät in Wut: doppelte Schlagkraft, unaufhaltsam!".into());
                    }
                    Golem if m.faehigkeit <= 0.0 => {
                        m.faehigkeit = 9.0;
                        m.attacking = 0.9;
                        self.stampfer.push((m.position, 14.0, 3.0));
                        self.ereignisse.push(Ereignis::Stampfen(m.position, 14.0));
                    }
                    Ghost if m.faehigkeit <= 0.0 => {
                        m.faehigkeit = 13.0;
                        m.unverwundbar = 3.0;
                        self.ereignisse.push(Ereignis::Unverwundbar(m.center()));
                    }
                    Warlock if m.faehigkeit <= 0.0 => {
                        m.faehigkeit = 12.0;
                        m.attacking = 0.9;
                        beschwoerungen.push((gi, m.row, m.welle, m.zaeh, m.staerke, m.position));
                    }
                    _ => {}
                }
            }
        }
        for (gi, mitte, radius, anteil) in heilungen {
            let mut geheilt = false;
            for m in self.groups[gi].members.iter_mut().filter(|m| m.dying.is_none() && m.position.distance(mitte) < radius) {
                if m.gift > 0.0 || m.heilsperre > 0.0 || m.health >= m.max_health {
                    continue;
                }
                m.health = (m.health + m.max_health * anteil).min(m.max_health);
                geheilt = true;
            }
            if geheilt {
                self.ereignisse.push(Ereignis::Heilung(mitte));
            }
        }
        for (gi, row, welle, zaeh, staerke, ort) in beschwoerungen {
            if self.count() > MAX_ENEMIES + 20 {
                continue;
            }
            for i in 0..3 {
                let side = i as f32 - 1.0;
                let neu = self.neues_mitglied(Skeleton, false, welle, zaeh, staerke, row + 0.6, side);
                self.groups[gi].members.push(neu);
            }
            self.ereignisse.push(Ereignis::Beschwoerung(ort));
        }
    }

    /// Ein Golem zerfällt in drei Felslinge (der Golem-Boss in drei Golems).
    #[allow(clippy::too_many_arguments)]
    fn spalten(&mut self, gi: usize, row: f32, side: f32, boss: bool, welle: u32, zaeh: f32, staerke: f32, ort: Vec3) {
        if gi >= self.groups.len() {
            return;
        }
        let art = if boss { Golem } else { Felsling };
        for i in 0..3 {
            let mut neu = self.neues_mitglied(art, false, welle, zaeh, staerke, row + 0.3 * i as f32, side + i as f32 * 0.7 - 0.7);
            neu.position = ort - Vec3::Y * 0.8;
            self.groups[gi].members.push(neu);
        }
        self.ereignisse.push(Ereignis::Spaltung(ort));
    }

    /// Gift und Feuer: vergiftete Einheiten explodieren, wenn Feuer sie trifft (3 m Umkreis).
    fn explodieren(&mut self) {
        if self.explodiert_gerade {
            return;
        }
        self.explodiert_gerade = true;
        let mut runden = 0;
        while let Some((mitte, schaden, name, turm)) = self.explosionen.pop() {
            runden += 1;
            if runden > 40 {
                self.explosionen.clear();
                break;
            }
            self.ereignisse.push(Ereignis::Explosion(mitte, 3.0));
            for id in self.within(mitte, 3.0, Filter::BODEN) {
                self.damage(id, Hit { schaden, art: DamageKind::Fire, ..Default::default() }, Quelle { name: &name, turm });
            }
        }
        self.explodiert_gerade = false;
    }

    /// Wellen, deren letzte Einheit weg ist: Auswertung, bei Welle 30 der Sieg.
    fn auswerten(&mut self) {
        let unterwegs: BTreeSet<u32> = self.groups.iter().flat_map(|g| g.members.iter().map(|m| m.welle)).collect();
        let fertig: Vec<u32> = self.zaehler.keys().copied().filter(|w| !unterwegs.contains(w) && *w <= self.welle).collect();
        for welle in fertig {
            let Some(z) = self.zaehler.remove(&welle) else { continue };
            let mut schaden: Vec<(String, u32)> = z.schaden.iter().filter(|(n, _)| !n.is_empty()).map(|(n, &s)| (n.clone(), s.round() as u32)).collect();
            schaden.sort_by(|a, b| b.1.cmp(&a.1));
            let bester_turm = z.kills_turm.iter().max_by_key(|&(_, &k)| k).map(|(&id, &k)| (id, k));
            let bericht = WellenBericht {
                welle,
                besiegt: z.besiegt,
                durchgebrochen: z.durch,
                bester_spieler: schaden.first().cloned(),
                bester_turm: None,
                gold: 0,
                schaden,
            };
            self.berichte.push((bericht, bester_turm));
            if welle == ZIEL_WELLE && !self.sieg {
                self.sieg = true;
                self.sieg_neu = true;
                if self.endlos {
                    self.meldungen.push("Welle 30 überstanden – die Insel ist gerettet! Weiter im Endlosmodus …".into());
                } else {
                    self.enabled = false;
                    self.meldungen.push("SIEG! Welle 30 überstanden – die Schattenfestung ist geschlagen. Die Insel ist gerettet!".into());
                }
            }
        }
    }

    /// Alle Einheiten für den Schnappschuss.
    pub fn states(&self) -> Vec<EnemyState> {
        self.groups
            .iter()
            .flat_map(|g| {
                g.members.iter().map(move |m| EnemyState {
                    id: m.id,
                    kind: m.kind,
                    position: m.position,
                    facing: m.facing,
                    action: if m.dying.is_some() {
                        EnemyAction::Dying
                    } else if m.attacking > 0.0 {
                        EnemyAction::Attack
                    } else if (g.halted || m.stun > 0.0) && !m.kind.fliegt() {
                        EnemyAction::Idle
                    } else {
                        EnemyAction::Walk
                    },
                    health: if m.dying.is_some() { 0 } else { ((m.health / m.max_health) * 100.0).ceil().clamp(1.0, 100.0) as u8 },
                    boss: m.boss,
                    lp: m.health.ceil() as u32,
                    flags: m.flags(),
                })
            })
            .collect()
    }

    fn alive(&self) -> impl Iterator<Item = &Member> {
        self.groups.iter().flat_map(|g| &g.members).filter(|m| m.dying.is_none())
    }

    fn member_mut(&mut self, id: u16) -> Option<&mut Member> {
        self.groups.iter_mut().flat_map(|g| g.members.iter_mut()).find(|m| m.id == id && m.dying.is_none())
    }

    /// Ziel eines Turms nach Zielmodus unter den Einheiten, die er treffen kann.
    pub fn ziel(&self, from: Vec3, range: f32, minimum: f32, modus: Zielmodus, filter: Filter) -> Option<(u16, Vec3)> {
        self.ziele(from, range, minimum, modus, filter, 1).into_iter().next()
    }

    /// Die besten `n` Ziele nach Zielmodus.
    pub fn ziele(&self, from: Vec3, range: f32, minimum: f32, modus: Zielmodus, filter: Filter, n: usize) -> Vec<(u16, Vec3)> {
        let mut kandidaten: Vec<&Member> = self
            .alive()
            .filter(|m| {
                let d = m.center().distance(from);
                d <= range && d >= minimum && m.passt(filter)
            })
            .collect();
        let wert = |m: &Member| -> f32 {
            match modus {
                Zielmodus::Erster => m.progress,
                Zielmodus::Letzter => -m.progress,
                Zielmodus::Staerkster => m.health,
                Zielmodus::Schwaechster => -m.health,
                Zielmodus::Boss => m.progress + if m.boss { 10_000.0 } else { 0.0 },
            }
        };
        kandidaten.sort_by(|a, b| wert(b).total_cmp(&wert(a)));
        kandidaten.into_iter().take(n).map(|m| (m.id, m.center())).collect()
    }

    /// Nächste Einheit um `at` (Kettenblitz, Soldaten), ohne die schon getroffenen.
    pub fn nearest_except(&self, at: Vec3, radius: f32, except: &[u16], filter: Filter) -> Option<(u16, Vec3)> {
        self.alive()
            .filter(|m| !except.contains(&m.id) && m.passt(filter) && m.center().distance(at) <= radius)
            .min_by(|a, b| a.center().distance(at).total_cmp(&b.center().distance(at)))
            .map(|m| (m.id, m.center()))
    }

    /// Alle Einheiten im Umkreis (Flächenschaden).
    pub fn within(&self, at: Vec3, radius: f32, filter: Filter) -> Vec<u16> {
        self.alive()
            .filter(|m| m.passt(filter) && m.center().distance(at) <= radius + m.kind.hit_sphere().1 + 0.5)
            .map(|m| m.id)
            .collect()
    }

    /// Einheiten in einem Kegel (Drachenatem): Ursprung, Richtung, Reichweite, cos des halben Winkels.
    pub fn im_kegel(&self, from: Vec3, richtung: Vec3, range: f32, cos_halb: f32, filter: Filter) -> Vec<(u16, Vec3)> {
        let richtung = richtung.normalize_or(Vec3::Z);
        self.alive()
            .filter(|m| m.passt(filter))
            .filter(|m| {
                let to = m.center() - from;
                let d = to.length();
                d <= range && d > 0.1 && to.dot(richtung) / d >= cos_halb
            })
            .map(|m| (m.id, m.center()))
            .collect()
    }

    /// Einheiten auf einer Linie (Balliste: Durchbohren).
    pub fn auf_linie(&self, from: Vec3, richtung: Vec3, laenge: f32, breite: f32, filter: Filter) -> Vec<(u16, Vec3)> {
        let richtung = richtung.normalize_or(Vec3::Z);
        self.alive()
            .filter(|m| m.passt(filter))
            .filter(|m| {
                let to = m.center() - from;
                let along = to.dot(richtung);
                along > 0.0 && along <= laenge && (to - richtung * along).length() <= breite + m.kind.hit_sphere().1
            })
            .map(|m| (m.id, m.center()))
            .collect()
    }

    /// Einheit gesehen? (Ort, verlangsamt, betäubt, fliegt, Boss)
    pub fn info(&self, id: u16) -> Option<ZielInfo> {
        self.alive().find(|m| m.id == id).map(|m| ZielInfo {
            verlangsamt: m.slow.1 > 0.0,
            betaeubt: m.stun > 0.0,
            fliegt: m.fliegt_jetzt(),
            boss: m.boss,
        })
    }

    /// Getarnte im Umkreis sichtbar machen (Späherturm, Läuterung): für eine halbe Sekunde.
    pub fn aufdecken(&mut self, at: Vec3, radius: f32) {
        for m in self.groups.iter_mut().flat_map(|g| g.members.iter_mut()) {
            if m.dying.is_none() && m.center().distance(at) <= radius {
                m.aufgedeckt = m.aufgedeckt.max(0.5);
            }
        }
    }

    /// Läuterung: keine Heilung und kein Schild im Umkreis (für eine halbe Sekunde).
    pub fn laeutern(&mut self, at: Vec3, radius: f32) {
        for m in self.groups.iter_mut().flat_map(|g| g.members.iter_mut()) {
            if m.dying.is_none() && m.center().distance(at) <= radius {
                m.heilsperre = m.heilsperre.max(0.5);
                m.schild_bruch = m.schild_bruch.max(0.5);
                m.aufgedeckt = m.aufgedeckt.max(0.5);
            }
        }
    }

    /// Markieren (Späherturm): alle Treffer machen `anteil` mehr Schaden, dazu Kopfgeldbonus.
    pub fn markieren(&mut self, id: u16, anteil: f32, dauer: f32, bonus: f32) {
        if let Some(m) = self.member_mut(id) {
            m.mark = (m.mark.0.max(anteil), dauer);
            m.mark_bonus = m.mark_bonus.max(bonus);
        }
    }

    /// Gift schwächt die Rüstung (für eine Sekunde, wird in der Wolke laufend erneuert).
    pub fn schwaechen(&mut self, id: u16, anteil: f32) {
        if let Some(m) = self.member_mut(id) {
            m.weak = (m.weak.0.max(anteil), 1.0);
        }
    }

    /// Verlangsamen im Umkreis (Frostfeld, Teer, Runenfeld): für eine halbe Sekunde.
    pub fn bremsen(&mut self, at: Vec3, radius: f32, anteil: f32, filter: Filter, teer: bool) {
        for m in self.groups.iter_mut().flat_map(|g| g.members.iter_mut()) {
            if m.passt(filter) && vec2(m.position.x - at.x, m.position.z - at.z).length() <= radius {
                if m.wut {
                    continue;
                }
                let anteil = if m.kind.schwer() { anteil * 0.5 } else { anteil };
                m.slow = (m.slow.0.max(anteil), m.slow.1.max(0.5));
                if teer {
                    m.geteert = m.geteert.max(2.5);
                }
            }
        }
    }

    /// Windstoß: die ganze Gruppe der Einheit wird zurückgeworfen (Bosse halb so weit).
    pub fn stoss(&mut self, id: u16, meter: f32) -> Option<Vec3> {
        let group = self.groups.iter_mut().find(|g| g.members.iter().any(|m| m.id == id && m.dying.is_none()))?;
        let boss = group.members.iter().any(|m| m.boss && m.dying.is_none());
        let meter = if boss { meter * 0.5 } else { meter };
        let rows = group.members.iter().map(|m| m.row).fold(0.0, f32::max);
        group.distance = (group.distance - meter).max(rows * 2.4);
        let ort = group.members.iter().find(|m| m.id == id).map(|m| m.center());
        if let Some(ort) = ort {
            self.ereignisse.push(Ereignis::Windstoss(ort));
        }
        ort
    }

    /// Schaden an einer Einheit (Rüstung, Empfindlichkeiten, Schild, Markierung und Nachwirkungen).
    /// Liefert die Art, falls die Einheit besiegt ist.
    pub fn damage(&mut self, id: u16, hit: Hit, von: Quelle) -> Option<EnemyKind> {
        let (gi, mi) = self.groups.iter().enumerate().find_map(|(gi, g)| g.members.iter().position(|m| m.id == id && m.dying.is_none()).map(|mi| (gi, mi)))?;
        let member = &mut self.groups[gi].members[mi];
        if member.unverwundbar > 0.0 {
            return None;
        }
        let mut faktor = member.kind.factor(hit.art, member.weak.0);
        if member.geschuetzt && matches!(hit.art, DamageKind::Physical | DamageKind::Pierce) {
            faktor *= 0.7;
        }
        if member.mark.1 > 0.0 {
            faktor *= 1.0 + member.mark.0;
        }
        if member.wut {
            faktor *= 0.8;
        }
        if hit.art == DamageKind::Fire {
            if member.geteert > 0.0 {
                faktor *= 1.5;
            }
            // Kombo: Gift + Feuer = Explosion (verbraucht das Gift)
            if member.gift > 0.0 {
                member.gift = 0.0;
                member.weak = (0.0, 0.0);
                self.explosionen.push((member.center(), 25.0 + hit.schaden * 1.5, von.name.to_string(), von.turm));
            }
        }
        let schaden = (hit.schaden * faktor).max(0.0).min(member.health);
        member.health -= schaden;
        if hit.bremse > 0.0 && !member.wut {
            let bremse = if member.kind.schwer() { hit.bremse * 0.5 } else { hit.bremse };
            member.slow = (member.slow.0.max(bremse), 2.5);
        }
        if hit.stun > 0.0 && !member.wut {
            member.stun = member.stun.max(hit.stun * if member.boss { 0.4 } else { 1.0 });
        }
        if hit.brand > 0.0 {
            let teer = if member.geteert > 0.0 { 2.0 } else { 1.0 };
            member.burn = (member.burn.0.max(hit.brand * teer), hit.dauer);
            member.burn_by = Some((von.name.to_string(), von.turm));
            member.burn_ausbreiten |= hit.effekte & wirkung::AUSBREITEN != 0;
        }
        if hit.effekte & wirkung::SCHILDBRUCH != 0 {
            member.schild_bruch = member.schild_bruch.max(6.0);
        }
        if hit.effekte & wirkung::ERDEN != 0 && member.kind.fliegt() {
            member.geerdet = member.geerdet.max(5.0);
        }
        if hit.effekte & wirkung::GIFT != 0 {
            member.gift = member.gift.max(hit.dauer.max(1.0));
            member.seuche |= hit.effekte & wirkung::SEUCHE != 0;
        }
        let welle = member.welle;
        let tot = member.health <= 0.0;
        let info = tot.then(|| {
            member.health = 0.0;
            member.dying = Some(0.0);
            (member.kind, member.boss, member.center(), member.mark_bonus, member.seuche && member.gift > 0.0, member.burn.1 > 0.0 && member.burn_ausbreiten, member.row, member.side, member.zaeh, member.staerke)
        });
        statistik(&mut self.beitrag, &mut self.turm_stats, &mut self.zaehler, von.name, von.turm, welle, schaden, tot);
        let result = if let Some((kind, boss, ort, bonus, seuche, ausbreiten, row, side, zaeh, staerke)) = info {
            self.gefallen.push(Gefallen { von: von.name.to_string(), turm: von.turm, kind, boss, welle, ort, bonus, seuche, ausbreiten });
            if kind == Golem {
                self.spalten(gi, row, side, boss, welle, zaeh, staerke, ort);
            }
            Some(kind)
        } else {
            None
        };
        self.explodieren();
        result
    }

    /// Welche Einheit liegt auf dem Strahl zuerst (vor `nearest`)? Liefert (ID, Entfernung).
    pub fn ray_hit(&self, from: Vec3, direction: Vec3, nearest: f32) -> Option<(u16, f32)> {
        let mut best = None;
        let mut limit = nearest;
        for member in self.alive() {
            let radius = member.kind.hit_sphere().1 * member.kind.groesse() * if member.boss { 1.6 } else { 1.0 };
            let center = member.center();
            let along = (center - from).dot(direction);
            if along <= 0.0 || along - radius > limit {
                continue;
            }
            let miss = (from + direction * along).distance_squared(center);
            if miss < radius * radius {
                let entry = (along - (radius * radius - miss).sqrt()).max(0.0);
                if entry < limit {
                    limit = entry;
                    best = Some((member.id, entry));
                }
            }
        }
        best
    }

    /// Alle Einheiten entfernen (Admin).
    pub fn clear(&mut self) {
        self.groups.clear();
    }

    /// Nur für Tests und Demos: Welle setzen, ohne Truppen zu schicken.
    pub fn springe_zu_welle(&mut self, welle: u32) {
        self.welle = welle;
        self.plane(welle + 1);
    }
}

/// Schaden und Kills zählen: je Spieler, je Turm und für die Auswertung der Welle.
#[allow(clippy::too_many_arguments)]
fn statistik(
    beitrag: &mut BTreeMap<String, (f32, u32)>,
    turm_stats: &mut HashMap<u32, (f32, u32)>,
    zaehler: &mut BTreeMap<u32, Zaehler>,
    name: &str,
    turm: Option<u32>,
    welle: u32,
    schaden: f32,
    kill: bool,
) {
    if !name.is_empty() {
        let e = beitrag.entry(name.to_string()).or_default();
        e.0 += schaden;
        e.1 += kill as u32;
    }
    if let Some(t) = turm {
        let e = turm_stats.entry(t).or_default();
        e.0 += schaden;
        e.1 += kill as u32;
    }
    if let Some(z) = zaehler.get_mut(&welle) {
        *z.schaden.entry(name.to_string()).or_default() += schaden;
        if kill {
            z.besiegt += 1;
            if let Some(t) = turm {
                *z.kills_turm.entry(t).or_default() += 1;
            }
        }
    }
}

/// Was ein Turm über sein Ziel wissen muss (Kombos, Harpune).
#[derive(Clone, Copy, Debug)]
pub struct ZielInfo {
    pub verlangsamt: bool,
    pub betaeubt: bool,
    pub fliegt: bool,
    pub boss: bool,
}

// ---------- Darstellung (nur mit Fenster) ----------

/// Eine Figur zum Anzeigen: Einheit der Festung oder Soldat einer Kaserne.
struct Anzeige {
    id: u32,
    datei: &'static str,
    label: &'static str,
    position: Vec3,
    facing: f32,
    action: EnemyAction,
    health: u8,
    lp: u32,
    groesse: f32,
    trefferkugel: (f32, f32),
    flags: u16,
    freund: bool,
    paladin: bool,
}

struct Figur {
    entity: EntityId,
    animator: Animator,
    shown: Vec3,
    facing: f32,
    action: EnemyAction,
    label: &'static str,
    kind: Option<EnemyKind>,
    health: u8,
    lp: u32,
    groesse: f32,
    trefferkugel: (f32, f32),
    flash: f32,
    fallen: f32,
    flags: u16,
    /// Schaden, der noch als Zahl erscheinen soll, und wie lange schon gesammelt wird
    offen: f32,
    sammeln: f32,
    funken: f32,
}

/// Eine Schadenszahl über einer Einheit.
#[derive(Clone, Copy, Debug)]
pub struct Schadenszahl {
    pub ort: Vec3,
    pub wert: u32,
    pub alter: f32,
    pub gross: bool,
}

#[derive(Default)]
pub struct HeerAnsicht {
    models: HashMap<&'static str, Option<(Arc<Model>, MeshId)>>,
    figuren: HashMap<u32, Figur>,
    /// Schadenszahlen zum Einblenden (die Oberfläche zeichnet sie)
    pub zahlen: Vec<Schadenszahl>,
    rng: Option<Rng>,
}

/// Bis zu dieser Entfernung zur Kamera werden die Animationen gerechnet.
const ANIMATION_DISTANCE: f32 = 110.0;
/// Soldaten bekommen IDs ab hier (damit sie sich nicht mit den Einheiten der Festung überschneiden)
const SOLDAT_ID: u32 = 0x1_0000;

impl HeerAnsicht {
    fn model(&mut self, ctx: &mut Context, datei: &'static str) -> Option<(Arc<Model>, MeshId)> {
        self.models
            .entry(datei)
            .or_insert_with(|| {
                let path = asset_files::variants("gegner", datei).into_iter().next()?;
                let model = match Model::from_file(&path) {
                    Ok(model) => Arc::new(model),
                    Err(message) => {
                        log::warn!("{datei}: {message}");
                        return None;
                    }
                };
                let textures = model.register_textures(&mut ctx.assets);
                let texture = textures.first().copied();
                let mesh = ctx.assets.named_mesh(&format!("gegner_gpu_{datei}"), || model.skinned_gpu_mesh(texture));
                Some((model, mesh))
            })
            .clone()
    }

    /// Einmal pro Bild: Einheiten und Soldaten anlegen, bewegen, animieren, entfernen.
    pub fn update(&mut self, ctx: &mut Context, states: &[EnemyState], soldaten: &[SoldatState], sounds: &mut Vec<SoundEvent>) {
        let dt = ctx.time.delta;
        let anzeigen: Vec<Anzeige> = states
            .iter()
            .map(|s| Anzeige {
                id: s.id as u32,
                datei: s.kind.file_name(),
                label: s.kind.label(),
                position: s.position,
                facing: s.facing,
                action: s.action,
                health: s.health,
                lp: s.lp,
                groesse: s.kind.groesse() * if s.boss { 1.6 } else { 1.0 },
                trefferkugel: s.kind.hit_sphere(),
                flags: s.flags,
                freund: false,
                paladin: false,
            })
            .chain(soldaten.iter().map(|s| Anzeige {
                id: SOLDAT_ID + s.id as u32,
                datei: "soldat",
                label: if s.paladin { "Paladin" } else { "Soldat" },
                position: s.position,
                facing: s.facing,
                action: if s.health == 0 { EnemyAction::Dying } else { s.action },
                health: s.health,
                lp: s.health as u32,
                groesse: 1.0,
                trefferkugel: (1.0, 0.6),
                flags: 0,
                freund: true,
                paladin: s.paladin,
            }))
            .collect();
        // Weg, was nicht mehr gemeldet wird
        let gone: Vec<u32> = self.figuren.keys().copied().filter(|id| !anzeigen.iter().any(|s| s.id == *id)).collect();
        for id in gone {
            if let Some(figur) = self.figuren.remove(&id) {
                ctx.scene.despawn(figur.entity);
            }
        }
        let mut rng = self.rng.take().unwrap_or_else(|| Rng::new(0x5EE));
        for state in &anzeigen {
            if !self.figuren.contains_key(&state.id) {
                let Some((model, mesh)) = self.model(ctx, state.datei) else { continue };
                let mut animator = Animator::new(model);
                animator.play("Idle", true, 0.0);
                let mut entity = Entity::new(state.label, mesh).with_transform(Transform::from_position(state.position));
                entity.joints = animator.palette();
                let entity = ctx.scene.spawn(entity);
                self.figuren.insert(
                    state.id,
                    Figur {
                        entity,
                        animator,
                        shown: state.position,
                        facing: state.facing,
                        action: EnemyAction::Idle,
                        label: state.label,
                        kind: EnemyKind::ALL.into_iter().find(|k| k.label() == state.label),
                        health: state.health,
                        lp: state.lp,
                        groesse: state.groesse,
                        trefferkugel: state.trefferkugel,
                        flash: 0.0,
                        fallen: 0.0,
                        flags: state.flags,
                        offen: 0.0,
                        sammeln: 0.0,
                        funken: 0.0,
                    },
                );
            }
            let Some(figur) = self.figuren.get_mut(&state.id) else { continue };
            let (up, radius) = figur.trefferkugel;
            let up = up * figur.groesse;
            if state.health < figur.health || (state.action == EnemyAction::Dying && figur.action != EnemyAction::Dying) {
                figur.flash = 0.25;
                ctx.particles.burst(Burst {
                    position: figur.shown + Vec3::Y * up,
                    count: if state.health == 0 { 36 } else { 14 },
                    color: if state.freund { vec3(1.0, 0.85, 0.5) } else { vec3(0.6, 0.45, 1.0) },
                    color_variation: 0.35,
                    speed: 3.0,
                    direction: Vec3::Y * 0.5,
                    size: 0.1 + radius * 0.05,
                    life: 0.8,
                    gravity: 0.5,
                    glow: 4.0,
                    grow: 0.0,
                    round: true,
                });
                sounds.push(SoundEvent::Impact { at: figur.shown + Vec3::Y * up, animal: true, killed: state.health == 0 });
            }
            // Schadenszahlen: kurz sammeln (Brand und Gift ticken oft), dann als eine Zahl zeigen
            if !state.freund && state.lp < figur.lp {
                figur.offen += (figur.lp - state.lp) as f32;
            }
            figur.sammeln += dt;
            if figur.offen >= 1.0 && (figur.sammeln > 0.25 || state.health == 0) {
                self.zahlen.push(Schadenszahl { ort: figur.shown + Vec3::Y * (up * 2.0 + 0.4), wert: figur.offen.round() as u32, alter: 0.0, gross: figur.offen > 60.0 });
                figur.offen = 0.0;
                figur.sammeln = 0.0;
            }
            figur.health = state.health;
            figur.lp = state.lp;
            figur.flags = state.flags;
            // Weich hinterher (die Schnappschüsse kommen etwa 30-mal pro Sekunde)
            figur.shown = if figur.shown.distance(state.position) > 6.0 { state.position } else { figur.shown.lerp(state.position, (dt * 12.0).min(1.0)) };
            let turn = (state.facing - figur.facing + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            figur.facing += turn * (dt * 8.0).min(1.0);
            let near = figur.shown.distance(ctx.camera.position) < ANIMATION_DISTANCE;
            if state.action != figur.action {
                let (clip, looped, fade) = match state.action {
                    EnemyAction::Idle => ("Idle", true, 0.25),
                    EnemyAction::Walk => ("Laufen", true, 0.2),
                    EnemyAction::Attack => ("Angriff", false, 0.1),
                    EnemyAction::Dying => ("Idle", true, 0.2),
                };
                figur.animator.play(clip, looped, fade);
                figur.action = state.action;
            }
            if state.action == EnemyAction::Dying {
                figur.fallen = (figur.fallen + dt * 1.6).min(1.0);
            } else {
                figur.fallen = 0.0;
            }
            figur.flash = (figur.flash - dt).max(0.0);
            // Zustände als kleine Funken: brennt, vergiftet, betäubt, markiert, Heilung der Paladine
            figur.funken -= dt;
            if near && figur.funken <= 0.0 && state.action != EnemyAction::Dying {
                figur.funken = 0.18;
                let (schild_funke, paladin_funke) = (rng.chance(0.3), rng.chance(0.3));
                use zustand::*;
                let kopf = figur.shown + Vec3::Y * (up * 2.0 + 0.2);
                let mut funke = |farbe: Vec3, ort: Vec3, schwere: f32, glow: f32| {
                    ctx.particles.burst(Burst {
                        position: ort + vec3(rng.range(-0.3, 0.3), 0.0, rng.range(-0.3, 0.3)),
                        count: 2,
                        color: farbe,
                        color_variation: 0.2,
                        speed: 0.8,
                        direction: Vec3::Y,
                        size: 0.09,
                        life: 0.6,
                        gravity: schwere,
                        glow,
                        grow: 0.0,
                        round: true,
                    });
                };
                if state.flags & BRENNT != 0 {
                    funke(vec3(1.0, 0.5, 0.15), figur.shown + Vec3::Y * up, -1.5, 4.0);
                }
                if state.flags & VERGIFTET != 0 {
                    funke(vec3(0.45, 1.0, 0.3), figur.shown + Vec3::Y * up, -0.5, 1.5);
                }
                if state.flags & BETAEUBT != 0 {
                    funke(vec3(1.0, 0.95, 0.4), kopf, 0.0, 3.0);
                }
                if state.flags & MARKIERT != 0 {
                    funke(vec3(1.0, 0.25, 0.2), kopf + Vec3::Y * 0.3, 0.0, 5.0);
                }
                if state.flags & SCHILD != 0 && schild_funke {
                    funke(vec3(0.5, 0.7, 1.0), figur.shown + Vec3::Y * up, 0.0, 2.0);
                }
                if state.paladin && paladin_funke {
                    funke(vec3(1.0, 0.9, 0.5), figur.shown + Vec3::Y * up, -0.5, 3.0);
                }
            }
            if near {
                figur.animator.update(dt);
            }
            let Some(entity) = ctx.scene.try_get_mut(figur.entity) else { continue };
            entity.visible = figur.shown.distance(ctx.camera.position) < 320.0;
            if near {
                entity.joints = figur.animator.palette();
            }
            let fall = figur.fallen * figur.fallen;
            entity.transform.position = figur.shown - Vec3::Y * (fall * up * 0.6 + (figur.fallen - 0.5).max(0.0) * 1.2);
            entity.transform.rotation = Quat::from_rotation_y(figur.facing) * Quat::from_rotation_z(fall * std::f32::consts::FRAC_PI_2 * 0.95);
            entity.transform.scale = Vec3::splat(figur.groesse);
            // Farbe: Treffer blitzen rot, Getarnte fast schwarz, Unverwundbare weiß, Wut rot, Frost bläulich
            use zustand::*;
            let puls = 0.5 + 0.5 * (ctx.time.elapsed * 6.0).sin();
            let mut farbe = Vec4::ONE;
            if state.flags & GETARNT != 0 {
                farbe = vec4(0.18, 0.14, 0.26, 1.0) * (0.8 + 0.4 * puls);
            }
            if state.flags & VERLANGSAMT != 0 {
                farbe *= vec4(0.7, 0.9, 1.35, 1.0);
            }
            if state.flags & WUT != 0 {
                farbe *= vec4(1.5, 0.6, 0.5, 1.0);
            }
            if state.flags & UNVERWUNDBAR != 0 {
                farbe = Vec4::ONE.lerp(vec4(2.5, 2.5, 2.8, 1.0), 0.5 + 0.5 * puls);
            }
            if state.paladin {
                farbe *= vec4(1.15, 1.1, 0.9, 1.0);
            }
            entity.color = farbe.lerp(vec4(2.2, 0.35, 0.3, 1.0), figur.flash / 0.25);
        }
        for zahl in &mut self.zahlen {
            zahl.alter += dt;
            zahl.ort += Vec3::Y * dt * 1.2;
        }
        self.zahlen.retain(|z| z.alter < 1.1);
        self.rng = Some(rng);
    }

    /// Gegner unter dem Fadenkreuz (für die Lebensleiste): Art, Lebenspunkte, Mitte, Zustände.
    pub fn aimed(&self, from: Vec3, direction: Vec3, max: f32) -> Option<(EnemyKind, u8, Vec3, u16)> {
        let mut best: Option<(f32, &Figur)> = None;
        for figur in self.figuren.values().filter(|f| f.action != EnemyAction::Dying && f.kind.is_some()) {
            let (up, radius) = figur.trefferkugel;
            let center = figur.shown + Vec3::Y * up * figur.groesse;
            let along = (center - from).dot(direction);
            if along <= 0.0 || along > max {
                continue;
            }
            let radius = radius * figur.groesse;
            if (from + direction * along).distance_squared(center) < radius * radius * 1.4 && best.is_none_or(|(d, _)| along < d) {
                best = Some((along, figur));
            }
        }
        best.and_then(|(_, f)| f.kind.map(|k| (k, f.health, f.shown + Vec3::Y * f.trefferkugel.0 * f.groesse, f.flags)))
    }

    /// Lebensleisten über den Figuren: Oberkante, Lebenspunkte in Prozent, verbündet (Soldat), Boss.
    /// Getarnte und Gefallene haben keine.
    pub fn leisten(&self) -> Vec<(Vec3, u8, bool, bool)> {
        self.figuren
            .values()
            .filter(|f| f.action != EnemyAction::Dying && f.health > 0 && f.flags & zustand::GETARNT == 0)
            .map(|f| {
                let oben = f.shown + Vec3::Y * ((f.trefferkugel.0 * 2.0 + 0.3) * f.groesse + 0.25);
                (oben, f.health, f.kind.is_none(), f.kind.is_some() && f.groesse > 1.3)
            })
            .collect()
    }

    /// Bosse, die gerade zu sehen sind: Name, Lebenspunkte in Prozent (für die Bossleiste oben).
    pub fn bosse(&self) -> Vec<(&'static str, u8)> {
        let mut bosse: Vec<(&'static str, u8)> = self
            .figuren
            .values()
            .filter(|f| f.groesse > 1.3 && f.kind.is_some() && f.action != EnemyAction::Dying)
            .map(|f| (f.label, f.health))
            .collect();
        bosse.sort();
        bosse
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boden(world: &crate::world::World) -> impl Fn(Vec2) -> f32 + '_ {
        |p: Vec2| world.terrain.height_at(p.x, p.y)
    }

    #[test]
    fn strassen_gleich_lang_und_truppen_verschwinden_am_ende() {
        let mut ctx = Context::headless();
        let mut world = crate::world::World::new(&mut ctx);
        // Vier Heerstraßen, alle genau gleich lang (fair)
        let island = crate::island::STRASSEN_LAENGE;
        for (i, route) in world.heer.routes.iter().enumerate() {
            // Straße ab dem Fuß der Rampe (Punkt 5), dazu das kurze Stück vom Rampenfuß dorthin
            let strasse = route.length() - route.along[5];
            assert!((strasse - island).abs() < 0.01, "Straße {i}: {strasse} m statt {island} m");
        }
        let laengen: Vec<f32> = world.heer.routes.iter().map(Route::length).collect();
        assert!(laengen.iter().all(|l| (l - laengen[0]).abs() < 0.01), "Routen unterschiedlich lang: {laengen:?}");

        let mut heer = std::mem::replace(&mut world.heer, Heer::new(Vec::new()));
        heer.set_enabled(true);
        let anfang = heer.count();
        assert!(anfang >= 4 * 4, "zu wenige Einheiten: {anfang}");
        heer.enabled = false;
        // Ohne Spieler marschieren alle bis ans Ende und verschwinden dort – und stehen dabei auf dem Boden
        let mut unterwegs = false;
        let mut schwebt = 0.0f32;
        for _ in 0..3000 {
            heer.tick(0.1, &[], &boden(&world));
            let states = heer.states();
            unterwegs |= states.iter().any(|s| s.position.distance(Vec3::ZERO) > 150.0);
            for s in states.iter().filter(|s| s.position.distance(Vec3::ZERO) > 150.0 && s.flags & zustand::FLIEGT == 0) {
                schwebt = schwebt.max((s.position.y - world.terrain.height_at(s.position.x, s.position.z)).abs());
            }
            if states.is_empty() {
                break;
            }
        }
        assert!(unterwegs, "Truppen kommen nicht aus der Festung");
        assert!(schwebt < 0.05, "Einheiten schweben über dem Boden oder versinken: {schwebt} m");
        assert_eq!(heer.count(), 0, "Truppen verschwinden nicht am Ende der Straße");
    }

    #[test]
    fn wellen_werden_staerker_und_durchbrueche_kosten_leben() {
        let mut ctx = Context::headless();
        let mut world = crate::world::World::new(&mut ctx);
        let mut heer = std::mem::replace(&mut world.heer, Heer::new(Vec::new()));
        let boden = boden(&world);
        assert_eq!((heer.welle, heer.leben.clone()), (0, vec![MAX_LEBEN; heer.routes.len()]));
        // Welle 1: gewöhnliche Truppen mit Grundleben, kein Boss; die Vorschau stimmt
        let (vorschau, boss) = heer.vorschau();
        assert!(boss.is_none());
        heer.spawn_wave();
        assert_eq!(heer.welle, 1);
        let erste = heer.count();
        assert_eq!(vorschau.iter().map(|(_, n)| *n as usize).sum::<usize>(), erste, "Vorschau passt nicht zur Welle");
        assert!(heer.states().iter().all(|s| !s.boss));
        assert!(heer.meldungen.iter().any(|m| m.contains("Welle 1")));
        // Bis Welle 5: mehr Einheiten, zähere Einheiten, ein Boss je Straße
        heer.clear();
        for _ in 0..4 {
            heer.spawn_wave();
        }
        assert_eq!(heer.welle, 5);
        let bosse = heer.states().iter().filter(|s| s.boss).count();
        assert_eq!(bosse, heer.routes.len(), "jede Straße braucht in Welle 5 einen Boss");
        assert!(heer.meldungen.iter().any(|m| m.contains("Bosswelle")));
        let golem = heer.groups.iter().flat_map(|g| &g.members).find(|m| m.boss).unwrap();
        assert!((golem.max_health - EnemyKind::Golem.max_health() * 1.48 * 6.0).abs() < 0.1, "Boss-Leben {}", golem.max_health);
        assert!(heer.count() > erste, "Welle 5 ist nicht größer als Welle 1");

        // Durchbrüche: jede Einheit am Straßenende kostet Leben
        heer.reset();
        heer.spawn_wave();
        heer.enabled = false;
        // Viele Leben, damit die Insel nicht fällt (sonst gibt es keine Auswertung)
        heer.leben = vec![1000; heer.routes.len()];
        let mut minimum = 1000;
        for _ in 0..3000 {
            heer.tick(0.1, &[], &boden);
            minimum = minimum.min(*heer.leben.iter().min().unwrap());
            if heer.count() == 0 {
                break;
            }
        }
        assert!(minimum < 1000, "Durchbruch kostet keine Leben");
        assert!(heer.meldungen.iter().any(|m| m.contains("durchgebrochen")));
        // Die Welle ist vorbei: Auswertung liegt vor
        assert!(heer.berichte.iter().any(|(b, _)| b.welle == 1 && b.durchgebrochen > 0), "keine Auswertung");

        // Eine Straße ohne Leben fällt allein: ihre Truppen verschwinden, sie hat wieder volle Leben,
        // die anderen Straßen und die Wellen laufen weiter
        let welle = heer.welle;
        heer.leben = vec![1000; heer.routes.len()];
        heer.leben[2] = 1;
        heer.spawn_wave();
        heer.enabled = false;
        for _ in 0..3000 {
            heer.tick(0.1, &[], &boden);
            if !heer.gefallene_lanes.is_empty() {
                break;
            }
        }
        assert_eq!(heer.gefallene_lanes, vec![2], "Straße Nord fällt nicht");
        assert_eq!(heer.leben[2], MAX_LEBEN, "gefallene Straße bekommt keine vollen Leben");
        assert!(heer.leben[0] > MAX_LEBEN, "andere Straßen sind mitgefallen");
        assert!(heer.groups.iter().all(|g| g.route != 2), "Truppen der gefallenen Straße bleiben");
        assert!(heer.welle > welle, "die Wellen sind zurückgesetzt");
        assert!(heer.meldungen.iter().any(|m| m.contains("gefallen")));
    }

    #[test]
    fn truppen_bleiben_bei_spielern_stehen_und_greifen_an() {
        let mut ctx = Context::headless();
        let mut world = crate::world::World::new(&mut ctx);
        let mut heer = std::mem::replace(&mut world.heer, Heer::new(Vec::new()));
        heer.spawn_wave();
        // Ein Spieler steht auf der Südstraße ein Stück vor der Rampe
        let (spieler, _) = heer.routes[0].sample(120.0);
        let mut angriffe = 0;
        for _ in 0..1200 {
            angriffe += heer.tick(0.1, &[Blocker { ort: spieler, ziel: Ziel::Spieler }], &boden(&world)).len();
        }
        assert!(angriffe > 0, "niemand greift an");
        let nah = heer.states().iter().filter(|s| s.position.distance(spieler) < 30.0).count();
        assert!(nah > 0, "die Gruppe ist am Spieler vorbeigelaufen");
    }

    #[test]
    fn eigenschaften_der_einheiten() {
        let mut ctx = Context::headless();
        let mut world = crate::world::World::new(&mut ctx);
        let mut heer = std::mem::replace(&mut world.heer, Heer::new(Vec::new()));
        let boden = boden(&world);
        // Gezielt eine Gruppe aus Flieger, Meuchler, Ritter, Magier und Golem
        heer.springe_zu_welle(0);
        let zaeh = 1.0;
        let mut members = Vec::new();
        for (i, kind) in [Knight, Pikeman, Warlock, Golem, Assassin].into_iter().enumerate() {
            members.push(heer.neues_mitglied(kind, false, 1, zaeh, 1.0, (i / 2) as f32, (i % 2) as f32 - 0.5));
        }
        heer.groups.push(Group { route: 0, distance: 120.0, speed: 2.0, halted: false, fliegend: false, members });
        let harpyie = heer.neues_mitglied(Harpy, false, 1, zaeh, 1.0, 0.0, 0.0);
        heer.groups.push(Group { route: 1, distance: 120.0, speed: 3.0, halted: false, fliegend: true, members: vec![harpyie] });
        heer.zaehler.insert(1, Zaehler::default());
        heer.tick(0.05, &[], &boden);
        let states = heer.states();
        let finde = |k: EnemyKind| states.iter().find(|s| s.kind == k).copied().unwrap();
        // Harpyie fliegt hoch über dem Boden, Bodentürme treffen sie nicht
        let h = finde(Harpy);
        assert!(h.flags & zustand::FLIEGT != 0 && h.position.y - world.terrain.height_at(h.position.x, h.position.z) > 3.0);
        assert!(heer.ziel(h.position, 30.0, 0.0, Zielmodus::Erster, Filter::BODEN).is_none_or(|(id, _)| id != h.id));
        assert!(heer.ziel(h.position, 30.0, 0.0, Zielmodus::Erster, Filter::ALLE).is_some());
        // Meuchler ist getarnt, bis ihn etwas aufdeckt
        let m = finde(Assassin);
        assert!(m.flags & zustand::GETARNT != 0);
        assert!(!heer.within(m.position, 1.0, Filter::ALLE).contains(&m.id));
        heer.aufdecken(m.position, 5.0);
        assert!(heer.within(m.position, 1.0, Filter::ALLE).contains(&m.id));
        // Der Ritter schützt den Pikenier vor Pfeilen
        let p = finde(Pikeman);
        assert!(p.flags & zustand::SCHILD != 0, "Pikenier nicht im Schild");
        // Der Golem zerfällt in drei Felslinge
        let g = finde(Golem);
        heer.damage(g.id, Hit { schaden: 99_999.0, art: DamageKind::Arcane, ..Default::default() }, Quelle { name: "nils", turm: Some(7) });
        assert_eq!(heer.states().iter().filter(|s| s.kind == Felsling).count(), 3, "Golem zerfällt nicht");
        assert!(heer.gefallen.iter().any(|f| f.kind == Golem && f.von == "nils" && f.turm == Some(7)));
        assert_eq!(heer.turm_stats.get(&7).map(|s| s.1), Some(1));
        // Gift + Feuer = Explosion
        let p = finde(Pikeman);
        heer.damage(p.id, Hit { schaden: 1.0, art: DamageKind::Poison, dauer: 4.0, effekte: wirkung::GIFT, ..Default::default() }, Quelle { name: "nils", turm: None });
        heer.ereignisse.clear();
        heer.damage(p.id, Hit { schaden: 5.0, art: DamageKind::Fire, ..Default::default() }, Quelle { name: "nils", turm: None });
        assert!(heer.ereignisse.iter().any(|e| matches!(e, Ereignis::Explosion(..))), "keine Explosion");
        // Der Magier heilt Verletzte in seiner Nähe (aber keine Vergifteten)
        let r = finde(Knight);
        heer.damage(r.id, Hit { schaden: 60.0, art: DamageKind::Arcane, ..Default::default() }, Quelle { name: "", turm: None });
        let vorher = heer.states().iter().find(|s| s.id == r.id).unwrap().lp;
        for _ in 0..100 {
            heer.tick(0.05, &[], &boden);
        }
        let nachher = heer.states().iter().find(|s| s.id == r.id).map(|s| s.lp).unwrap_or(0);
        assert!(nachher > vorher, "Magier heilt nicht: {vorher} → {nachher}");
    }

    #[test]
    fn bosse_nutzen_faehigkeiten() {
        let mut ctx = Context::headless();
        let mut world = crate::world::World::new(&mut ctx);
        let mut heer = std::mem::replace(&mut world.heer, Heer::new(Vec::new()));
        let boden = boden(&world);
        // Welle 5: Golem-Boss stampft
        heer.springe_zu_welle(4);
        heer.spawn_wave();
        for _ in 0..300 {
            heer.tick(0.05, &[], &boden);
        }
        assert!(!heer.stampfer.is_empty(), "der Golem stampft nicht");
        // Welle 15: Magier-Boss ruft Skelette
        heer.clear();
        heer.springe_zu_welle(14);
        heer.spawn_wave();
        let vorher = heer.count();
        for _ in 0..300 {
            heer.tick(0.05, &[], &boden);
        }
        assert!(heer.count() > vorher, "der Magier ruft keine Skelette");
        assert!(heer.ereignisse.iter().any(|e| matches!(e, Ereignis::Beschwoerung(_))));
    }
}

#[cfg(test)]
mod ausgabe {
    /// Nur zum Nachsehen (Kamerapositionen für Bilder): `cargo test strassen_ausgeben -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn strassen_ausgeben() {
        let mut ctx = engine::prelude::Context::headless();
        let world = crate::world::World::new(&mut ctx);
        for (i, strasse) in world.heer.strassen().enumerate() {
            let punkte: Vec<String> = strasse.iter().step_by(5).map(|p| format!("({:.0},{:.1},{:.0})", p.x, world.terrain.height_at(p.x, p.y), p.y)).collect();
            println!("Straße {i} {}: {}", world.heer.strassen_namen()[i], punkte.join(" "));
        }
    }
}
