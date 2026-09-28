//! Die Wildnis: Streuner, die in kleinen Trupps über die Insel ziehen (Goblins, Orks, Echsen,
//! Pilzlinge, Waldschrate, Minotauren, Keiler – je nach Gegend und Gefahr), und die Bewohner der
//! Dungeons, die ihren Raum bewachen.
//!
//! Ein Trupp wandert gemächlich durch sein Streifgebiet und rastet zwischendurch. Wer zu nahe kommt
//! (oder auf ihn schießt), wird angegriffen und verfolgt, bis zur Leine – dann kehrt der Trupp um
//! und heilt sich. Besiegte lassen Gold und mit etwas Glück ein Runenfragment fallen. Ein
//! aufgeriebener Trupp taucht nach einer Weile anderswo auf der Insel wieder auf.
//! Gezeigt werden sie wie die Truppen der Festung (`EnemyState`, IDs ab `WILD_ID`).

use engine::noise::Rng;
use engine::prelude::*;

use crate::heer::{zustand, EnemyAction, EnemyKind, EnemyState};
use crate::protocol::PlayerId;
use crate::tuerme::DamageKind;

/// IDs der Lagerbewohner (die Truppen der Festung bleiben darunter).
pub const WILD_ID: u16 = 0xF000;
/// So lange bleibt ein leeres Lager leer (Sekunden).
const NEU_BESETZEN: f32 = 150.0;
/// So lange dauert es, bis ein aufgeriebener Trupp anderswo wieder auftaucht (Sekunden).
const NEU_STREIFEN: f32 = 110.0;
/// So weit zieht ein Trupp um die Mitte seines Streifgebiets (Meter).
const STREIF_RADIUS: f32 = 45.0;
/// Gegenden der Insel (Bits in `LagerArt::gebiet`).
pub mod gebiet {
    pub const KUESTE: u8 = 1;
    pub const WIESE: u8 = 2;
    pub const WALD: u8 = 4;
    pub const BERG: u8 = 8;
}
/// Solange ein Spieler so nahe ist, wird ein leeres Lager nicht neu besetzt.
const BESETZEN_ABSTAND: f32 = 60.0;
/// Solange liegen Besiegte am Boden.
const STERBEN: f32 = 2.5;
/// Pilzling: so wahrscheinlich zündet er, wenn er getroffen wird; so lange glüht er, bevor er
/// platzt; so nah muss er dafür an sein Ziel; so weit reicht die Sporenwolke; so viel Schaden
pub const ZUENDEN_CHANCE: f32 = 0.4;
pub const ZUENDZEIT: f32 = 3.5;
const ZUENDEN_NAH: f32 = 1.6;
/// So lange steht er nach dem Zünden glühend still, bevor er losrennt (Sekunden)
const AUFLEUCHTEN: f32 = 0.7;
pub const SPOREN_RADIUS: f32 = 3.5;
const SPOREN_SCHADEN: f32 = 26.0;

/// Dungeons passen sich der Zahl der Spieler an, die gerade darin sind: allein deutlich
/// leichter, je weiterem Spieler mehr Leben und etwas mehr Schaden. (Leben, Schaden)
pub fn dungeon_faktor(spieler: usize) -> (f32, f32) {
    let weitere = spieler.max(1) as f32 - 1.0;
    (0.55 + 0.35 * weitere, 0.6 + 0.15 * weitere)
}
/// Keiler und Minotaurus: Ansturm (Anlauf, dann Lauf in gerader Linie)
const ANLAUF: f32 = 0.55;
const STURMLAUF: f32 = 1.1;
/// Waldschrat: so lange warnt der Ring, bevor die Wurzeln hervorbrechen; so groß ist der Kreis
pub const WURZEL_WARNUNG: f32 = 1.1;
pub const WURZEL_RADIUS: f32 = 2.3;

/// Wer in einem Lager haust.
pub struct LagerArt {
    pub name: &'static str,
    /// 1 = harmlos, 3 = gefährlich, 4–5 nur tief in Dungeons (mehr Leben, härtere Schläge, bessere Beute)
    pub gefahr: u8,
    pub einheiten: &'static [EnemyKind],
    pub anfuehrer: EnemyKind,
    /// Faktor auf die Leben (große Bosse der Festung sind für Dungeons zu zäh)
    pub leben: f32,
    /// Nur in Dungeons (`dungeon.rs`), nicht in der Wildnis
    pub dungeon: bool,
    /// Der Anführer ist ein Endgegner: sichere, wertvolle Beute
    pub boss: bool,
    /// Streuner: in welchen Gegenden der Trupp umherzieht (`gebiet::…`)
    pub gebiet: u8,
}

const fn streu(name: &'static str, gefahr: u8, einheiten: &'static [EnemyKind], anfuehrer: EnemyKind, gebiet: u8) -> LagerArt {
    LagerArt { name, gefahr, einheiten, anfuehrer, leben: 1.0, dungeon: false, boss: false, gebiet }
}

const fn tief(name: &'static str, gefahr: u8, einheiten: &'static [EnemyKind], anfuehrer: EnemyKind, leben: f32, boss: bool) -> LagerArt {
    LagerArt { name, gefahr, einheiten, anfuehrer, leben, dungeon: true, boss, gebiet: 0 }
}

use gebiet::{BERG, KUESTE, WALD, WIESE};

/// Streuner und Dungeonbewohner – deutlich schwächer als die Truppen der Festung.
/// Die Indizes 5–16 gehören den Dungeons (`dungeon.rs`), der Rest zieht über die Insel.
pub const ARTEN: [LagerArt; 23] = [
    streu("Goblinbande", 1, &[EnemyKind::Goblin, EnemyKind::Goblin], EnemyKind::Goblin, WIESE | WALD | KUESTE | BERG),
    streu("Pilzkreis", 1, &[EnemyKind::Pilzling, EnemyKind::Pilzling], EnemyKind::Pilzling, WALD),
    streu("Goblintrupp", 2, &[EnemyKind::Goblin, EnemyKind::Goblin, EnemyKind::GoblinSchamane], EnemyKind::Goblin, WIESE | WALD | BERG),
    streu("Echsenjäger", 2, &[EnemyKind::Echse], EnemyKind::Echse, KUESTE),
    streu("Orkspäher", 2, &[EnemyKind::Goblin, EnemyKind::Goblin], EnemyKind::Ork, WIESE | BERG),
    // ---------- Spinnengrotte ----------
    tief("Spinnenbrut", 2, &[EnemyKind::Waldspinne, EnemyKind::Waldspinne, EnemyKind::Waldspinne, EnemyKind::Waldspinne, EnemyKind::Waldspinne], EnemyKind::Spinnling, 1.0, false),
    tief("Gargoylenhorst", 3, &[EnemyKind::Harpy, EnemyKind::Harpy, EnemyKind::Waldspinne, EnemyKind::Waldspinne], EnemyKind::Harpy, 1.0, false),
    tief("Thron der Spinnenkönigin", 4, &[EnemyKind::Spinnling, EnemyKind::Spinnling, EnemyKind::Waldspinne, EnemyKind::Waldspinne, EnemyKind::Waldspinne],
         EnemyKind::Spinnenkoenigin, 0.16, true),
    // ---------- Gruft der Vergessenen ----------
    tief("Knochenhalle", 2, &[EnemyKind::Skeleton, EnemyKind::Skeleton, EnemyKind::Skeleton], EnemyKind::Skeleton, 1.0, false),
    tief("Geistergang", 3, &[EnemyKind::Ghost, EnemyKind::Skeleton, EnemyKind::Ghost], EnemyKind::Warlock, 1.0, false),
    tief("Zirkel der Totenbeschwörer", 3, &[EnemyKind::Warlock, EnemyKind::Skeleton, EnemyKind::Skeleton, EnemyKind::Assassin], EnemyKind::Warlock, 1.0, false),
    tief("Grab des Lichkönigs", 4, &[EnemyKind::Skeleton, EnemyKind::Ghost, EnemyKind::Skeleton, EnemyKind::Warlock], EnemyKind::Lich, 0.18, true),
    // ---------- Schmiede der Tiefe ----------
    tief("Höllenrudel", 3, &[EnemyKind::Wolf, EnemyKind::Wolf, EnemyKind::Wolf, EnemyKind::Wolf], EnemyKind::Wolf, 1.0, false),
    tief("Halle der Golems", 3, &[EnemyKind::Felsling, EnemyKind::Felsling, EnemyKind::Felsling], EnemyKind::Golem, 0.5, false),
    tief("Wacht der Dunklen Ritter", 4, &[EnemyKind::Knight, EnemyKind::Pikeman, EnemyKind::Knight, EnemyKind::Archer], EnemyKind::Knight, 0.8, false),
    tief("Trollhöhle", 4, &[EnemyKind::Felsling, EnemyKind::Felsling, EnemyKind::Wolf], EnemyKind::Troll, 0.16, true),
    tief("Thron des Dämonenfürsten", 5, &[EnemyKind::Knight, EnemyKind::Wolf, EnemyKind::Wolf, EnemyKind::Warlock], EnemyKind::Daemon, 0.14, true),
    // ---------- weitere Streuner ----------
    streu("Keilerrotte", 1, &[EnemyKind::Keiler], EnemyKind::Keiler, WALD | WIESE | KUESTE),
    streu("Schamanenzirkel", 2, &[EnemyKind::GoblinSchamane, EnemyKind::Pilzling], EnemyKind::GoblinSchamane, WALD),
    streu("Orkkriegstrupp", 3, &[EnemyKind::Ork, EnemyKind::GoblinSchamane], EnemyKind::Ork, WIESE | BERG),
    streu("Waldschrat", 3, &[EnemyKind::Pilzling, EnemyKind::Pilzling], EnemyKind::Waldschrat, WALD),
    streu("Minotaurus", 3, &[], EnemyKind::Minotaurus, BERG | WIESE),
    streu("Echsenkriegstrupp", 3, &[EnemyKind::Echse, EnemyKind::Echse], EnemyKind::Echse, KUESTE | WIESE),
];

/// Ein möglicher Ort für einen Trupp: Mitte, Bodenhöhe, Gefahr (zur Inselmitte hin höher), Gegend.
#[derive(Clone, Copy, Debug)]
pub struct Streifort {
    pub mitte: Vec2,
    pub hoehe: f32,
    pub gefahr: u8,
    pub gebiet: u8,
}

/// Die Arten, die an diesem Ort umherziehen können (Gefahr passt, Gegend passt).
fn passend(ort: &Streifort) -> Vec<usize> {
    let alle: Vec<usize> = (0..ARTEN.len()).filter(|&i| !ARTEN[i].dungeon && ARTEN[i].gefahr == ort.gefahr).collect();
    let genau: Vec<usize> = alle.iter().copied().filter(|&i| ARTEN[i].gebiet & ort.gebiet != 0).collect();
    if genau.is_empty() { alle } else { genau }
}

/// Leben, Schlagkraft und Beute je Gefahrenstufe.
fn staerke(gefahr: u8) -> (f32, f32) {
    match gefahr {
        1 => (1.0, 1.0),
        2 => (1.25, 1.1),
        3 => (1.6, 1.25),
        4 => (2.0, 1.4),
        _ => (2.5, 1.55),
    }
}

/// Wahrscheinlichkeit, dass ein Besiegter ein Runenfragment fallen lässt.
pub fn fragment_chance(gefahr: u8, anfuehrer: bool) -> f32 {
    match (gefahr, anfuehrer) {
        (1, false) => 0.08,
        (1, true) => 0.4,
        (2, false) => 0.14,
        (2, true) => 0.6,
        (_, false) => 0.2,
        (_, true) => 0.85,
    }
}

/// Gold für einen Besiegten.
pub fn gold(gefahr: u8, anfuehrer: bool) -> u32 {
    let grund = [4, 7, 11, 16, 22][(gefahr.clamp(1, 5) - 1) as usize];
    if anfuehrer {
        grund * 4
    } else {
        grund
    }
}

/// Ein Lagerplatz in der Wildnis.
#[derive(Clone, Debug)]
pub struct Lager {
    pub mitte: Vec2,
    pub hoehe: f32,
    /// Index in `ARTEN`
    pub art: usize,
    /// Seit wann alle Bewohner besiegt sind
    leer_seit: Option<f32>,
    /// In Dungeons: der Raum (min, max), den die Bewohner nicht verlassen
    pub bereich: Option<(Vec2, Vec2)>,
    /// Ein Trupp, der umherzieht: wohin er gerade geht (`anker`) und wie lange er dort rastet
    pub streift: bool,
    pub anker: Vec2,
    rast: f32,
}

impl Lager {
    pub fn art(&self) -> &'static LagerArt {
        &ARTEN[self.art]
    }

    /// Ein Raum in einem Dungeon.
    pub fn im_dungeon(mitte: Vec2, hoehe: f32, art: usize, bereich: (Vec2, Vec2)) -> Lager {
        Lager { mitte, hoehe, art, leer_seit: None, bereich: Some(bereich), streift: false, anker: mitte, rast: 0.0 }
    }

    /// Ein Trupp, der um `mitte` umherzieht.
    pub fn streifend(mitte: Vec2, hoehe: f32, art: usize) -> Lager {
        Lager { mitte, hoehe, art, leer_seit: None, bereich: None, streift: true, anker: mitte, rast: 4.0 }
    }

    /// Bis hierhin (Meter vom Lager bzw. vom Trupp) verfolgen die Bewohner einen Spieler.
    fn leine(&self) -> f32 {
        (if self.streift { 40.0 } else { 32.0 }) + 6.0 * self.art().gefahr as f32
    }

    /// Wer näher kommt, wird bemerkt.
    fn wachsam(&self) -> f32 {
        if self.bereich.is_some() {
            return 18.0;
        }
        12.0 + 3.0 * self.art().gefahr as f32
    }

    /// Ist ein Spieler im Revier (in Dungeons: im Raum bzw. in der Tür, sonst an der Leine)?
    fn im_revier(&self, p: Vec3) -> bool {
        match self.bereich {
            Some((a, b)) => {
                let q = vec2(p.x, p.z);
                q.cmpge(a - Vec2::splat(3.0)).all() && q.cmple(b + Vec2::splat(3.0)).all() && (p.y - self.hoehe).abs() < 6.0
            }
            None if self.streift => vec2(p.x, p.z).distance(self.anker) <= self.leine(),
            None => p.distance(vec3(self.mitte.x, self.hoehe, self.mitte.y)) <= self.leine(),
        }
    }

    /// Im Dungeon bleiben die Bewohner in ihrem Raum.
    fn begrenzen(&self, p: Vec2, radius: f32) -> Vec2 {
        match self.bereich {
            Some((a, b)) => p.clamp(a + Vec2::splat(radius.min(2.0)), b - Vec2::splat(radius.min(2.0))),
            None => p,
        }
    }
}

struct Wilder {
    id: u16,
    kind: EnemyKind,
    lager: usize,
    anfuehrer: bool,
    /// Größer dargestellt (Anführer eines Dungeonlagers)
    gross: bool,
    /// Posten im Lager (bei Streunern: Platz im Trupp relativ zum Anker)
    heim: Vec2,
    versatz: Vec2,
    position: Vec3,
    facing: f32,
    health: f32,
    max_health: f32,
    schlag: f32,
    /// Im Dungeon: Leben und Schlag ohne Anpassung an die Spielerzahl, und für wie viele
    /// Spieler gerade angepasst ist
    grund: (f32, f32),
    fuer_spieler: usize,
    cooldown: f32,
    ziel: Option<PlayerId>,
    stun: f32,
    /// Eingefroren (Restzeit): wie betäubt, der nächste Treffer zerschmettert das Eis
    frost: f32,
    /// Verlangsamung (Anteil, Restzeit), Brand (Schaden/s, Restzeit)
    slow: (f32, f32),
    burn: (f32, f32),
    /// Wer zuletzt getroffen hat (bekommt die Beute)
    letzter: String,
    dying: Option<f32>,
    attacking: f32,
    laeuft: bool,
    /// Pilzling glüht und rennt los: Restzeit bis zur Explosion
    zuendet: Option<f32>,
    /// Abklingzeit der Eigenheit (Ansturm, Wurzeln, Heilung)
    faehigkeit: f32,
    /// Ansturm: Richtung und Restzeit (erst Anlauf, dann Lauf)
    sturm: Option<(Vec2, f32)>,
    /// Ork in Raserei
    wut: bool,
}

impl Wilder {
    fn groesse(&self) -> f32 {
        self.kind.groesse() * if self.gross { 1.35 } else { 1.0 }
    }

    fn center(&self) -> Vec3 {
        self.position + Vec3::Y * self.kind.hit_sphere().0 * self.groesse()
    }

    fn radius(&self) -> f32 {
        self.kind.hit_sphere().1 * self.groesse()
    }

    fn lebt(&self) -> bool {
        self.dying.is_none()
    }
}

/// Ein Treffer eines Spielers.
#[derive(Clone, Copy, Debug, Default)]
pub struct Treffer {
    pub schaden: f32,
    pub art: DamageKind,
    /// Verlangsamung (Anteil, 4 s), Betäubung (s), Brand (Schaden/s, Dauer)
    pub bremse: f32,
    pub stun: f32,
    pub brand: f32,
    pub dauer: f32,
    /// Einfrieren (s)
    pub frost: f32,
}

/// Ein Besiegter (für Beute und Meldungen).
#[derive(Clone, Debug)]
pub struct WildGefallen {
    pub von: String,
    pub kind: EnemyKind,
    pub anfuehrer: bool,
    pub gefahr: u8,
    pub ort: Vec3,
    pub lager: &'static str,
    /// Ein Endgegner (sichere, wertvolle Beute)
    pub boss: bool,
}

/// Ein Angriff auf einen Spieler.
#[derive(Clone, Copy, Debug)]
pub struct WildAngriff {
    pub kind: EnemyKind,
    pub von: Vec3,
    pub ziel: Vec3,
    pub spieler: PlayerId,
    pub schaden: f32,
    /// Eine Explosion (kein Hieb und kein Geschoss zu zeigen)
    pub explosion: bool,
    /// Rückstoß (Geschwindigkeit, mit der der Spieler weggeschleudert wird)
    pub stoss: Vec3,
}

pub struct Wildnis {
    pub lager: Vec<Lager>,
    /// Wo Trupps (wieder) auftauchen können, und wohin sie nicht ziehen (Siedlungen, Burg, Start)
    orte: Vec<Streifort>,
    sperren: Vec<(Vec2, f32)>,
    wilde: Vec<Wilder>,
    next_id: u16,
    rng: Rng,
    uhr: f32,
    pub gefallen: Vec<WildGefallen>,
    /// Geplatzte Pilzlinge seit dem letzten Abholen (Ort)
    pub explosionen: Vec<Vec3>,
    /// Was die Streuner sonst noch tun und alle sehen sollen (Heilung, Raserei, Wurzeln, Ansturm)
    pub ereignisse: Vec<crate::td::Ereignis>,
    /// Wurzeln, die gleich hervorbrechen: Ort, Restzeit, Schaden
    wurzeln: Vec<(Vec3, f32, f32)>,
    /// Neu besetzte Lager seit dem letzten Abholen (Name, Ort)
    pub besetzt: Vec<(&'static str, Vec2)>,
}

impl Wildnis {
    pub fn new(lager: Vec<Lager>, boden: &dyn Fn(Vec2) -> f32) -> Wildnis {
        let mut wildnis = Wildnis {
            lager,
            orte: Vec::new(),
            sperren: Vec::new(),
            wilde: Vec::new(),
            next_id: WILD_ID,
            rng: Rng::new(0x_57_11D),
            uhr: 0.0,
            gefallen: Vec::new(),
            explosionen: Vec::new(),
            ereignisse: Vec::new(),
            wurzeln: Vec::new(),
            besetzt: Vec::new(),
        };
        for i in 0..wildnis.lager.len() {
            wildnis.besetzen(i, boden);
        }
        wildnis.besetzt.clear();
        wildnis
    }

    /// Wo Trupps (wieder) auftauchen können und wohin sie nicht ziehen.
    pub fn mit_streifgebieten(mut self, orte: Vec<Streifort>, sperren: Vec<(Vec2, f32)>) -> Wildnis {
        self.orte = orte;
        self.sperren = sperren;
        self
    }

    /// Mögliche Orte für Trupps: fester Zufall (auf allen Rechnern gleich), Land, nicht zu steil, weit
    /// weg von Heerstraßen, Siedlungsplätzen, Burg, Festung und Startlager. Die Gefahr steigt zur
    /// Inselmitte (zur Festung) hin; die Gegend (Küste, Wiese, Wald, Berg) bestimmt, wer dort lebt.
    pub fn streifgebiete(terrain: &Terrain, meiden: &[(Vec2, f32)], strassen: &[Vec<Vec2>]) -> Vec<Streifort> {
        let mut rng = Rng::new(0x57_2E1F);
        let mut orte: Vec<Streifort> = Vec::new();
        let radius = crate::island::ISLAND_RADIUS;
        for _ in 0..9000 {
            let p = vec2(rng.range(-radius, radius), rng.range(-radius, radius));
            let r = p.length();
            if !(160.0..radius * 0.88).contains(&r) {
                continue;
            }
            let h = terrain.height_at(p.x, p.y);
            if h < 2.4 {
                continue;
            }
            let steil = [vec2(4.0, 0.0), vec2(-4.0, 0.0), vec2(0.0, 4.0), vec2(0.0, -4.0)]
                .iter()
                .map(|d| (terrain.height_at(p.x + d.x, p.y + d.y) - h).abs())
                .fold(0.0f32, f32::max);
            if steil > 2.4 {
                continue;
            }
            if meiden.iter().any(|&(q, weite)| q.distance(p) < weite) || strassen.iter().flatten().any(|q| q.distance(p) < 30.0) {
                continue;
            }
            if crate::island::burg_rand(p) < 60.0 || crate::island::burg_weg(p).0 < 25.0 || crate::island::festung_rand(p) < 80.0 {
                continue;
            }
            if orte.iter().any(|o| o.mitte.distance(p) < 55.0) {
                continue;
            }
            let gefahr = if r > radius * 0.64 { 1 } else if r > radius * 0.42 { 2 } else { 3 };
            let gebiet = if h < 5.0 {
                KUESTE
            } else if h > 26.0 {
                BERG
            } else if crate::island::moisture(p) > 0.56 {
                WALD
            } else {
                WIESE
            };
            orte.push(Streifort { mitte: p, hoehe: h, gefahr, gebiet });
        }
        orte
    }

    /// Verteilt `anzahl` Trupps über die Insel (weit auseinander).
    pub fn streuner(orte: &[Streifort], anzahl: usize) -> Vec<Lager> {
        let mut rng = Rng::new(0x57_2E20);
        let mut lager: Vec<Lager> = Vec::new();
        for abstand in [150.0, 115.0, 85.0] {
            for _ in 0..4000 {
                if lager.len() >= anzahl || orte.is_empty() {
                    return lager;
                }
                let ort = orte[(rng.next_u32() % orte.len() as u32) as usize];
                if lager.iter().any(|l| l.mitte.distance(ort.mitte) < abstand) {
                    continue;
                }
                let arten = passend(&ort);
                let art = arten[(rng.next_u32() % arten.len() as u32) as usize];
                lager.push(Lager::streifend(ort.mitte, ort.hoehe, art));
            }
        }
        lager
    }

    /// Stellt die Bewohner eines Lagers auf: der Anführer in der Mitte, die anderen im Kreis.
    fn besetzen(&mut self, index: usize, boden: &dyn Fn(Vec2) -> f32) {
        let lager = &self.lager[index];
        let art = lager.art();
        let (leben, schlag) = staerke(art.gefahr);
        let mitte = lager.mitte;
        let mut neu = Vec::new();
        let mut liste: Vec<(EnemyKind, bool)> = vec![(art.anfuehrer, true)];
        liste.extend(art.einheiten.iter().map(|&k| (k, false)));
        let n = liste.len().max(2) - 1;
        for (i, (kind, anfuehrer)) in liste.into_iter().enumerate() {
            let heim = if anfuehrer {
                mitte
            } else {
                let w = std::f32::consts::TAU * (i - 1) as f32 / n as f32 + self.rng.range(-0.2, 0.2);
                mitte + vec2(w.cos(), w.sin()) * if lager.streift { self.rng.range(2.2, 3.6) } else { self.rng.range(4.5, 6.5) }
            };
            let id = self.next_id;
            self.next_id = if self.next_id == u16::MAX { WILD_ID } else { self.next_id + 1 };
            let heim = lager.begrenzen(heim, 1.5);
            let versatz = heim - mitte;
            let fuehrung = if !anfuehrer { 1.0 } else if lager.streift { 1.6 } else { 2.2 };
            let max_health = kind.max_health() * leben * art.leben * fuehrung;
            neu.push(Wilder {
                id,
                kind,
                lager: index,
                anfuehrer,
                gross: anfuehrer && !lager.streift,
                heim,
                versatz,
                position: vec3(heim.x, boden(heim), heim.y),
                facing: self.rng.range(0.0, std::f32::consts::TAU),
                health: max_health * if lager.bereich.is_some() { dungeon_faktor(1).0 } else { 1.0 },
                max_health: max_health * if lager.bereich.is_some() { dungeon_faktor(1).0 } else { 1.0 },
                grund: (max_health, kind.schlag() * schlag * if anfuehrer { 1.3 } else { 1.0 }),
                fuer_spieler: 1,
                schlag: kind.schlag() * schlag * if anfuehrer { 1.3 } else { 1.0 } * if lager.bereich.is_some() { dungeon_faktor(1).1 } else { 1.0 },
                cooldown: self.rng.range(0.5, 1.5),
                ziel: None,
                stun: 0.0,
                frost: 0.0,
                slow: (0.0, 0.0),
                burn: (0.0, 0.0),
                letzter: String::new(),
                dying: None,
                attacking: 0.0,
                laeuft: false,
                zuendet: None,
                // Keiler und Minotaurus stürmen los, sobald sie jemanden bemerken
                faehigkeit: if matches!(kind, EnemyKind::Keiler | EnemyKind::Minotaurus) { 0.0 } else { self.rng.range(2.0, 5.0) },
                sturm: None,
                wut: false,
            });
        }
        self.wilde.extend(neu);
        self.lager[index].leer_seit = None;
        self.lager[index].anker = mitte;
        self.besetzt.push((art.name, mitte));
    }

    /// Alle Lager sofort neu besetzen (Admin).
    pub fn alle_neu(&mut self, boden: &dyn Fn(Vec2) -> f32) {
        self.wilde.clear();
        for i in 0..self.lager.len() {
            self.besetzen(i, boden);
        }
    }

    #[cfg(test)]
    fn anzahl(&self) -> usize {
        self.wilde.iter().filter(|w| w.lebt()).count()
    }

    /// Ein Takt: bemerken, verfolgen, angreifen, heimkehren, neu besetzen. `spieler`: lebende
    /// Spieler und ihre Position. Liefert die Angriffe auf Spieler.
    pub fn tick(&mut self, dt: f32, spieler: &[(PlayerId, Vec3)], boden: &dyn Fn(Vec2) -> f32) -> Vec<WildAngriff> {
        self.uhr += dt;
        let mut angriffe = Vec::new();
        let finde = |id: PlayerId| spieler.iter().find(|s| s.0 == id).map(|s| s.1);

        // Alarm: wer von einem Lager verfolgt wird, den verfolgen alle aus dem Lager
        let mut alarm: Vec<Option<PlayerId>> = vec![None; self.lager.len()];
        for w in self.wilde.iter().filter(|w| w.lebt()) {
            if let Some(ziel) = w.ziel {
                alarm[w.lager].get_or_insert(ziel);
            }
        }
        self.umherziehen(dt, boden);
        let mut heilen: Vec<(usize, Vec3)> = Vec::new();
        // Dungeons: je Raum zählen, wie viele Spieler im selben Dungeon sind, und die Bewohner anpassen
        let mut im_dungeon: Vec<usize> = vec![0; self.lager.len()];
        for (i, l) in self.lager.iter().enumerate() {
            if l.bereich.is_some() {
                im_dungeon[i] = spieler.iter().filter(|(_, p)| (p.y - l.hoehe).abs() < 20.0 && vec2(p.x, p.z).distance(l.mitte) < 320.0).count();
            }
        }
        for w in self.wilde.iter_mut().filter(|w| w.lebt()) {
            let n = im_dungeon[w.lager];
            if n > 0 && n != w.fuer_spieler && self.lager[w.lager].bereich.is_some() {
                let (leben, schlag) = dungeon_faktor(n);
                let anteil = w.health / w.max_health;
                w.max_health = w.grund.0 * leben;
                w.health = w.max_health * anteil;
                w.schlag = w.grund.1 * schlag;
                w.fuer_spieler = n;
            }
        }
        for w in &mut self.wilde {
            if let Some(seit) = &mut w.dying {
                *seit += dt;
                continue;
            }
            let lager = &self.lager[w.lager];
            if lager.streift {
                w.heim = lager.anker + w.versatz;
            }
            // Brand, Betäubung, Verlangsamung
            if w.burn.1 > 0.0 {
                w.burn.1 -= dt;
                w.health -= w.burn.0 * dt;
                if w.health <= 0.0 {
                    w.health = 0.0;
                    w.dying = Some(0.0);
                    self.gefallen.push(WildGefallen {
                        von: w.letzter.clone(),
                        kind: w.kind,
                        anfuehrer: w.anfuehrer,
                        gefahr: lager.art().gefahr,
                        ort: w.position,
                        lager: lager.art().name,
                        boss: w.anfuehrer && lager.art().boss,
                    });
                    continue;
                }
            }
            w.slow.1 -= dt;
            if w.slow.1 <= 0.0 {
                w.slow.0 = 0.0;
            }
            w.cooldown -= dt;
            w.attacking -= dt;
            w.laeuft = false;
            w.frost = (w.frost - dt).max(0.0);
            if let Some(rest) = &mut w.zuendet {
                *rest -= dt;
                let nah = ZUENDZEIT - *rest > AUFLEUCHTEN + 0.3
                    && w.ziel.and_then(finde).is_some_and(|p| vec2(p.x, p.z).distance(vec2(w.position.x, w.position.z)) <= ZUENDEN_NAH);
                if *rest <= 0.0 || nah {
                    // Platzen: Sporenwolke trifft alle Spieler im Umkreis, der Pilzling vergeht
                    let mitte = w.center();
                    for &(id, p) in spieler {
                        let abstand = p.distance(mitte);
                        if abstand <= SPOREN_RADIUS {
                            let schaden = SPOREN_SCHADEN * staerke(lager.art().gefahr).1 * (1.0 - 0.5 * abstand / SPOREN_RADIUS);
                            angriffe.push(WildAngriff { kind: w.kind, von: mitte, ziel: p, spieler: id, schaden, explosion: true, stoss: (p - mitte).with_y(0.0).normalize_or_zero() * 5.0 + Vec3::Y * 4.0 });
                        }
                    }
                    self.explosionen.push(mitte);
                    w.health = 0.0;
                    w.dying = Some(STERBEN);
                    w.zuendet = None;
                    self.gefallen.push(WildGefallen {
                        von: w.letzter.clone(),
                        kind: w.kind,
                        anfuehrer: w.anfuehrer,
                        gefahr: lager.art().gefahr,
                        ort: w.position,
                        lager: lager.art().name,
                        boss: false,
                    });
                    continue;
                }
            }
            if w.stun > 0.0 && w.zuendet.is_none() {
                w.stun -= dt;
                continue;
            }
            // Ziel prüfen: noch da und nicht zu weit vom Lager?
            if let Some(ziel) = w.ziel {
                match finde(ziel) {
                    Some(p) if lager.im_revier(p) => {}
                    _ => w.ziel = None,
                }
            }
            if w.ziel.is_none() {
                w.ziel = alarm[w.lager].filter(|&z| finde(z).is_some_and(|p| lager.im_revier(p)));
            }
            if w.ziel.is_none() {
                w.ziel = spieler
                    .iter()
                    .filter(|(_, p)| p.distance(w.position) < lager.wachsam() && lager.im_revier(*p))
                    .min_by(|a, b| a.1.distance(w.position).total_cmp(&b.1.distance(w.position)))
                    .map(|s| s.0);
            }
            // ---- Eigenheiten der Streuner ----
            w.faehigkeit -= dt;
            let ziel_p = w.ziel.and_then(finde);
            let hier = vec2(w.position.x, w.position.z);
            if let Some((mut richtung, mut rest)) = w.sturm {
                rest -= dt;
                let (radius, mitte) = (w.radius(), w.center());
                if rest > STURMLAUF {
                    // Anlauf: mit den Hufen scharren und auf das Ziel ausrichten
                    if let Some(p) = ziel_p {
                        richtung = (vec2(p.x, p.z) - hier).normalize_or(richtung);
                    }
                } else {
                    let tempo = if w.kind == EnemyKind::Minotaurus { 10.0 } else { 11.5 };
                    let neu = lager.begrenzen(hier + richtung * tempo * dt, radius);
                    w.position = vec3(neu.x, boden(neu), neu.y);
                    w.laeuft = true;
                    // Wer im Weg steht, wird umgerannt und weggeschleudert
                    if let Some(&(id, p)) = spieler.iter().find(|(_, p)| vec2(p.x, p.z).distance(neu) < radius + 0.75) {
                        let wucht = if w.kind == EnemyKind::Minotaurus { 11.0 } else { 8.0 };
                        angriffe.push(WildAngriff {
                            kind: w.kind,
                            von: mitte,
                            ziel: p,
                            spieler: id,
                            schaden: w.schlag * 2.2,
                            explosion: false,
                            stoss: vec3(richtung.x, 0.0, richtung.y) * wucht + Vec3::Y * 5.5,
                        });
                        w.attacking = 0.6;
                        rest = 0.0;
                    }
                }
                w.facing = richtung.x.atan2(-richtung.y);
                w.sturm = (rest > 0.0).then_some((richtung, rest));
                continue;
            }
            if let Some(p) = ziel_p {
                let abstand = vec2(p.x, p.z).distance(hier);
                match w.kind {
                    EnemyKind::Keiler | EnemyKind::Minotaurus if w.faehigkeit <= 0.0 && (4.0..15.0).contains(&abstand) => {
                        w.sturm = Some(((vec2(p.x, p.z) - hier).normalize_or(Vec2::Y), ANLAUF + STURMLAUF));
                        w.faehigkeit = if w.kind == EnemyKind::Minotaurus { 8.0 } else { 6.0 };
                        self.ereignisse.push(crate::td::Ereignis::Ansturm(w.position));
                        continue;
                    }
                    EnemyKind::Waldschrat if w.faehigkeit <= 0.0 && abstand < 14.0 => {
                        let unter = vec3(p.x, boden(vec2(p.x, p.z)), p.z);
                        self.wurzeln.push((unter, WURZEL_WARNUNG, w.schlag * 1.6));
                        self.ereignisse.push(crate::td::Ereignis::Wurzelwarnung(unter));
                        w.faehigkeit = 7.0;
                        w.attacking = 0.8;
                    }
                    EnemyKind::GoblinSchamane if w.faehigkeit <= 0.0 => {
                        heilen.push((w.lager, w.position));
                        w.faehigkeit = 9.0;
                        w.attacking = 0.7;
                    }
                    _ => {}
                }
            }
            if w.kind == EnemyKind::Ork && !w.wut && w.health < w.max_health * 0.5 {
                w.wut = true;
                self.ereignisse.push(crate::td::Ereignis::Wut(w.center()));
            }
            // Streuner ohne Ziel schlendern
            let schlendern = if lager.streift && w.ziel.is_none() { 0.42 } else { 1.0 };
            let tempo = w.kind.speed() * 1.3 * (1.0 - w.slow.0) * schlendern * if w.zuendet.is_some() { 2.2 } else if w.wut { 1.35 } else { 1.0 };
            let hier = vec2(w.position.x, w.position.z);
            let (weg, zu) = match w.ziel.and_then(finde) {
                // Glühend rennt er stur auf sein Ziel zu
                Some(p) if w.zuendet.is_some_and(|rest| ZUENDZEIT - rest < AUFLEUCHTEN) => (None, Some(vec2(p.x, p.z))),
                Some(p) if w.zuendet.is_some() => (Some(vec2(p.x, p.z)), Some(vec2(p.x, p.z))),
                Some(p) => {
                    let ziel = vec2(p.x, p.z);
                    let (reichweite, pause) = w.kind.attack();
                    let reichweite = reichweite.min(16.0) + w.radius() * 0.5;
                    let abstand = hier.distance(ziel);
                    if abstand <= reichweite {
                        if w.cooldown <= 0.0 {
                            w.cooldown = pause * if w.wut { 0.65 } else { 1.0 };
                            w.attacking = 0.7;
                            angriffe.push(WildAngriff { kind: w.kind, von: w.center(), ziel: p, spieler: w.ziel.unwrap_or_default(), schaden: w.schlag * if w.wut { 1.5 } else { 1.0 }, explosion: false, stoss: Vec3::ZERO });
                        }
                        (None, Some(ziel))
                    } else {
                        (Some(ziel), Some(ziel))
                    }
                }
                None => {
                    // Heimkehren und dabei heilen
                    w.health = (w.health + w.max_health * 0.25 * dt).min(w.max_health);
                    if hier.distance(w.heim) > 0.6 {
                        (Some(w.heim), None)
                    } else {
                        (None, None)
                    }
                }
            };
            if let Some(nach) = weg {
                let d = nach - hier;
                let schritt = (tempo * dt).min(d.length());
                let neu = lager.begrenzen(hier + d.normalize_or_zero() * schritt, w.radius());
                w.position = vec3(neu.x, boden(neu), neu.y);
                w.laeuft = true;
                if d.length_squared() > 0.01 {
                    w.facing = d.x.atan2(-d.y);
                }
            }
            if let Some(blick) = zu {
                let d = blick - vec2(w.position.x, w.position.z);
                if d.length_squared() > 0.01 {
                    w.facing = d.x.atan2(-d.y);
                }
            }
        }
        // Der Schamane heilt seinen Trupp
        for (lager, ort) in heilen {
            for w in self.wilde.iter_mut().filter(|w| w.lager == lager && w.lebt() && w.position.distance(ort) < 10.0) {
                w.health = (w.health + w.max_health * 0.3).min(w.max_health);
            }
            self.ereignisse.push(crate::td::Ereignis::Heilung(ort + Vec3::Y));
        }
        // Wurzeln brechen hervor: wer noch im Kreis steht, wird getroffen und hochgeschleudert
        for (ort, rest, schaden) in &mut self.wurzeln {
            *rest -= dt;
            if *rest <= 0.0 {
                for &(id, p) in spieler {
                    if vec2(p.x, p.z).distance(vec2(ort.x, ort.z)) <= WURZEL_RADIUS && (p.y - ort.y).abs() < 3.0 {
                        angriffe.push(WildAngriff {
                            kind: EnemyKind::Waldschrat,
                            von: *ort,
                            ziel: p,
                            spieler: id,
                            schaden: *schaden,
                            explosion: true,
                            stoss: Vec3::Y * 7.0,
                        });
                    }
                }
                self.ereignisse.push(crate::td::Ereignis::Wurzeln(*ort));
            }
        }
        self.wurzeln.retain(|w| w.1 > 0.0);
        // Nicht ineinander stehen
        let orte: Vec<(Vec2, f32, bool)> = self.wilde.iter().map(|w| (vec2(w.position.x, w.position.z), w.radius(), w.lebt())).collect();
        for (i, w) in self.wilde.iter_mut().enumerate() {
            if !w.lebt() {
                continue;
            }
            let hier = vec2(w.position.x, w.position.z);
            let mut schub = Vec2::ZERO;
            for (j, &(p, r, lebt)) in orte.iter().enumerate() {
                if i == j || !lebt {
                    continue;
                }
                let d = hier - p;
                let min = (w.radius() + r) * 0.8;
                if d.length_squared() < min * min && d.length_squared() > 1e-6 {
                    schub += d.normalize() * (min - d.length()) * 0.5;
                }
            }
            if schub != Vec2::ZERO {
                let neu = hier + schub;
                w.position = vec3(neu.x, boden(neu), neu.y);
            }
        }
        self.wilde.retain(|w| w.dying.is_none_or(|t| t < STERBEN));
        // Leere Lager nach einer Weile neu besetzen (nicht vor den Augen der Spieler); ein
        // aufgeriebener Trupp taucht anderswo auf der Insel wieder auf
        for i in 0..self.lager.len() {
            if self.wilde.iter().any(|w| w.lager == i) {
                continue;
            }
            let seit = *self.lager[i].leer_seit.get_or_insert(self.uhr);
            let warten = if self.lager[i].streift { NEU_STREIFEN } else { NEU_BESETZEN };
            if self.uhr - seit <= warten {
                continue;
            }
            if self.lager[i].streift && !self.orte.is_empty() {
                let andere: Vec<Vec2> = self.lager.iter().enumerate().filter(|&(j, _)| j != i).map(|(_, l)| l.anker).collect();
                for _ in 0..30 {
                    let ort = self.orte[(self.rng.next_u32() % self.orte.len() as u32) as usize];
                    let frei = spieler.iter().all(|(_, p)| vec2(p.x, p.z).distance(ort.mitte) > 90.0) && andere.iter().all(|a| a.distance(ort.mitte) > 90.0);
                    if frei {
                        let arten = passend(&ort);
                        let lager = &mut self.lager[i];
                        lager.mitte = ort.mitte;
                        lager.hoehe = ort.hoehe;
                        lager.art = arten[(self.rng.next_u32() % arten.len() as u32) as usize];
                        self.besetzen(i, boden);
                        break;
                    }
                }
                continue;
            }
            let mitte = self.lager[i].mitte;
            let beobachtet = spieler.iter().any(|(_, p)| vec2(p.x, p.z).distance(mitte) < BESETZEN_ABSTAND);
            if !beobachtet {
                self.besetzen(i, boden);
            }
        }
        angriffe
    }

    /// Trupps ohne Ziel ziehen weiter: sind alle angekommen, rasten sie eine Weile und suchen sich
    /// dann ein neues Ziel in ihrem Streifgebiet (an Land, nicht in Siedlungen oder an der Burg).
    fn umherziehen(&mut self, dt: f32, boden: &dyn Fn(Vec2) -> f32) {
        for i in 0..self.lager.len() {
            if !self.lager[i].streift {
                continue;
            }
            let mut lebende = self.wilde.iter().filter(|w| w.lager == i && w.lebt()).peekable();
            if lebende.peek().is_none() {
                continue;
            }
            let anker = self.lager[i].anker;
            let mut ruhig = true;
            for w in lebende {
                if w.ziel.is_some() || vec2(w.position.x, w.position.z).distance(anker + w.versatz) > 1.2 {
                    ruhig = false;
                }
            }
            if !ruhig {
                continue;
            }
            self.lager[i].rast -= dt;
            if self.lager[i].rast > 0.0 {
                continue;
            }
            let mitte = self.lager[i].mitte;
            for _ in 0..10 {
                let w = self.rng.range(0.0, std::f32::consts::TAU);
                let d = self.rng.range(12.0, STREIF_RADIUS);
                let ziel = mitte + vec2(w.cos(), w.sin()) * d;
                if boden(ziel) < 1.5 || self.sperren.iter().any(|&(q, weite)| q.distance(ziel) < weite) {
                    continue;
                }
                self.lager[i].anker = ziel;
                break;
            }
            self.lager[i].rast = self.rng.range(7.0, 20.0);
        }
    }

    /// Schaden an einem Bewohner. `spieler`: wer getroffen hat (das Lager nimmt ihn ins Visier).
    /// Liefert die Art, wenn er dabei fällt.
    pub fn damage(&mut self, id: u16, treffer: Treffer, von: &str, spieler: Option<PlayerId>) -> Option<EnemyKind> {
        let wurf = self.rng.range(0.0, 1.0);
        let w = self.wilde.iter_mut().find(|w| w.id == id && w.lebt())?;
        // Ein getroffener Pilzling kann zünden: er glüht auf und rennt auf den Angreifer los
        if w.kind == EnemyKind::Pilzling && w.zuendet.is_none() && spieler.is_some() && wurf < ZUENDEN_CHANCE {
            w.zuendet = Some(ZUENDZEIT);
            w.ziel = spieler;
        }
        let mut schaden = treffer.schaden * w.kind.factor(treffer.art, 0.0);
        // Eingefroren: der nächste Treffer zerschmettert das Eis
        if w.frost > 0.0 && treffer.schaden > 0.0 {
            schaden *= crate::faehigkeiten::ZERSCHMETTERN;
            w.frost = 0.0;
            w.stun = 0.0;
        }
        w.health -= schaden;
        w.letzter = von.to_string();
        if treffer.bremse > 0.0 {
            w.slow = (treffer.bremse.max(w.slow.0), 4.0);
        }
        if treffer.stun > 0.0 {
            w.stun = w.stun.max(treffer.stun);
        }
        if treffer.frost > 0.0 {
            let frost = treffer.frost * if w.anfuehrer { 0.6 } else { 1.0 };
            w.frost = w.frost.max(frost);
            w.stun = w.stun.max(frost);
            w.slow.1 = w.slow.1.max(4.0 + frost);
        }
        if treffer.brand > 0.0 && w.kind.factor(DamageKind::Fire, 0.0) > 0.0 {
            w.burn = (treffer.brand, treffer.dauer);
        }
        if w.ziel.is_none() {
            w.ziel = spieler;
        }
        if w.health > 0.0 {
            return None;
        }
        w.health = 0.0;
        w.dying = Some(0.0);
        let lager = &self.lager[w.lager];
        self.gefallen.push(WildGefallen {
            von: von.to_string(),
            kind: w.kind,
            anfuehrer: w.anfuehrer,
            gefahr: lager.art().gefahr,
            ort: w.position,
            lager: lager.art().name,
            boss: w.anfuehrer && lager.art().boss,
        });
        Some(w.kind)
    }

    /// Nur für Screenshots: einen Trupp samt Bewohnern an einen anderen Ort versetzen.
    pub fn verlegen(&mut self, index: usize, mitte: Vec2, boden: &dyn Fn(Vec2) -> f32) {
        let Some(lager) = self.lager.get_mut(index) else { return };
        lager.mitte = mitte;
        lager.anker = mitte;
        lager.hoehe = boden(mitte);
        lager.rast = 60.0;
        for w in self.wilde.iter_mut().filter(|w| w.lager == index) {
            let p = mitte + w.versatz;
            w.heim = p;
            w.position = vec3(p.x, boden(p), p.y);
        }
    }

    /// Nur für Screenshots: den nächsten Pilzling zünden lassen.
    pub fn zuenden(&mut self, bei: Vec3, ziel: PlayerId) -> bool {
        let Some(w) = self
            .wilde
            .iter_mut()
            .filter(|w| w.lebt() && w.kind == EnemyKind::Pilzling)
            .min_by(|a, b| a.position.distance(bei).total_cmp(&b.position.distance(bei)))
        else {
            return false;
        };
        w.zuendet = Some(ZUENDZEIT);
        w.ziel = Some(ziel);
        true
    }

    /// Lebende Bewohner im Umkreis (Mitte der Trefferkugel).
    pub fn within(&self, at: Vec3, radius: f32) -> Vec<u16> {
        self.wilde.iter().filter(|w| w.lebt() && w.center().distance(at) <= radius + w.radius()).map(|w| w.id).collect()
    }

    /// Lebende Bewohner auf einer Linie (Arkanlanze): von `from` in `richtung`, bis `laenge`,
    /// höchstens `breite` neben dem Strahl.
    pub fn auf_linie(&self, from: Vec3, richtung: Vec3, laenge: f32, breite: f32) -> Vec<u16> {
        let richtung = richtung.normalize_or(Vec3::NEG_Z);
        self.wilde
            .iter()
            .filter(|w| w.lebt())
            .filter(|w| {
                let to = w.center() - from;
                let along = to.dot(richtung);
                along > 0.0 && along <= laenge && (to - richtung * along).length() <= breite + w.radius()
            })
            .map(|w| w.id)
            .collect()
    }

    /// Nächster lebender Bewohner um `at` (bis `radius`), ohne die in `ausser` – für den
    /// abprallenden Wurfhammer.
    pub fn naechster(&self, at: Vec3, radius: f32, ausser: &[u16]) -> Option<(u16, Vec3)> {
        self.wilde
            .iter()
            .filter(|w| w.lebt() && !ausser.contains(&w.id) && w.center().distance(at) <= radius)
            .min_by(|a, b| a.center().distance(at).total_cmp(&b.center().distance(at)))
            .map(|w| (w.id, w.center()))
    }

    /// Lebende Bewohner vor `from` in Blickrichtung (bis `weite`, halber Winkel als Kosinus).
    pub fn im_kegel(&self, from: Vec3, richtung: Vec3, weite: f32, cos_halb: f32) -> Vec<u16> {
        let richtung = richtung.with_y(0.0).normalize_or(Vec3::NEG_Z);
        self.wilde
            .iter()
            .filter(|w| w.lebt())
            .filter(|w| {
                let d = (w.center() - from).with_y(0.0);
                let abstand = d.length();
                abstand <= weite + w.radius() && (abstand < w.radius() || d.normalize_or_zero().dot(richtung) >= cos_halb)
            })
            .map(|w| w.id)
            .collect()
    }

    /// Erster Bewohner auf einem Strahl (ID, Entfernung), höchstens `nearest` weit.
    pub fn ray_hit(&self, from: Vec3, direction: Vec3, nearest: f32) -> Option<(u16, f32)> {
        let mut best = None;
        let mut nearest = nearest;
        for w in self.wilde.iter().filter(|w| w.lebt()) {
            let (center, radius) = (w.center(), w.radius());
            let along = (center - from).dot(direction);
            if along <= 0.0 || along - radius > nearest {
                continue;
            }
            let miss = (from + direction * along).distance_squared(center);
            if miss < radius * radius {
                let entry = (along - (radius * radius - miss).sqrt()).max(0.0);
                if entry < nearest {
                    nearest = entry;
                    best = Some((w.id, entry));
                }
            }
        }
        best
    }

    /// Was alle von den Bewohnern sehen.
    pub fn states(&self) -> Vec<EnemyState> {
        self.wilde
            .iter()
            .map(|w| {
                let mut flags = 0;
                if w.stun > 0.0 {
                    flags |= zustand::BETAEUBT;
                }
                if w.burn.1 > 0.0 {
                    flags |= zustand::BRENNT;
                }
                if w.slow.1 > 0.0 {
                    flags |= zustand::VERLANGSAMT;
                }
                if w.frost > 0.0 {
                    flags |= zustand::GEFROREN;
                }
                if w.zuendet.is_some() {
                    flags |= zustand::ZUENDET;
                }
                if w.wut {
                    flags |= zustand::WUT;
                }
                EnemyState {
                    id: w.id,
                    kind: w.kind,
                    position: w.position,
                    facing: w.facing,
                    action: if w.dying.is_some() {
                        EnemyAction::Dying
                    } else if w.attacking > 0.0 {
                        EnemyAction::Attack
                    } else if w.laeuft {
                        EnemyAction::Walk
                    } else {
                        EnemyAction::Idle
                    },
                    health: ((w.health / w.max_health).clamp(0.0, 1.0) * 100.0).ceil() as u8,
                    boss: w.gross,
                    lp: w.health.max(0.0).round() as u32,
                    flags,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ein Goblintrupp (Anführer und drei Streuner) auf ebenem Land.
    fn wildnis() -> Wildnis {
        let lager = vec![Lager::streifend(vec2(0.0, 0.0), 5.0, 2)];
        Wildnis::new(lager, &|_| 5.0)
    }

    #[test]
    fn trupp_bemerkt_verfolgt_und_laesst_los() {
        let mut w = wildnis();
        assert_eq!(w.anzahl(), 4, "Anführer und drei Streuner");
        // Weit weg: niemand greift an
        let weit = [(7u64, vec3(120.0, 5.0, 0.0))];
        for _ in 0..120 {
            assert!(w.tick(1.0 / 60.0, &weit, &|_| 5.0).is_empty());
        }
        // Nah dran: sie kommen und schlagen zu
        let hier = w.lager[0].anker;
        let nah = [(7u64, vec3(hier.x + 9.0, 5.0, hier.y))];
        let mut angriffe = 0;
        for _ in 0..600 {
            angriffe += w.tick(1.0 / 60.0, &nah, &|_| 5.0).len();
        }
        assert!(angriffe > 3, "zu wenige Angriffe: {angriffe}");
        // Außerhalb der Leine: sie lassen los und sammeln sich wieder um ihren Anker
        let weg = [(7u64, vec3(hier.x + 150.0, 5.0, hier.y))];
        for _ in 0..1200 {
            w.tick(1.0 / 60.0, &weg, &|_| 5.0);
        }
        assert!(w.wilde.iter().all(|x| x.ziel.is_none()), "verfolgen noch");
        let weiteste = w.wilde.iter().map(|x| vec2(x.position.x, x.position.z).length()).fold(0.0f32, f32::max);
        assert!(weiteste < STREIF_RADIUS + 10.0, "nicht ins Streifgebiet zurückgekehrt: {weiteste}");
        assert!(w.lager[0].anker.length() <= STREIF_RADIUS + 0.1, "Anker außerhalb des Streifgebiets");
    }

    #[test]
    fn trupp_zieht_umher_und_bleibt_im_gebiet() {
        let mut w = wildnis();
        let niemand: [(PlayerId, Vec3); 0] = [];
        let mut ziele = Vec::new();
        let mut weiteste = 0.0f32;
        for _ in 0..(120 * 20) {
            w.tick(0.05, &niemand, &|_| 5.0);
            let a = w.lager[0].anker;
            if ziele.last() != Some(&a) {
                ziele.push(a);
            }
            for x in &w.wilde {
                weiteste = weiteste.max(vec2(x.position.x, x.position.z).length());
            }
        }
        assert!(ziele.len() >= 4, "zieht nicht umher: {} Ziele", ziele.len());
        assert!(weiteste > 10.0, "bleibt auf der Stelle: {weiteste}");
        assert!(weiteste < STREIF_RADIUS + 8.0, "verlässt das Streifgebiet: {weiteste}");
        // Kein Ziel im Wasser
        let mut nass = Wildnis::new(vec![Lager::streifend(vec2(0.0, 0.0), 5.0, 2)], &|_| 5.0);
        for _ in 0..(60 * 20) {
            nass.tick(0.05, &niemand, &|p: Vec2| if p.x > 5.0 { 0.0 } else { 5.0 });
        }
        assert!(nass.lager[0].anker.x <= 5.0, "zieht ins Wasser: {}", nass.lager[0].anker);
    }

    #[test]
    fn besiegte_melden_sich_und_der_trupp_kommt_wieder() {
        let mut w = wildnis();
        let ids: Vec<u16> = w.states().iter().map(|s| s.id).collect();
        for id in ids {
            let treffer = Treffer { schaden: 10_000.0, art: DamageKind::Arcane, ..Default::default() };
            assert!(w.damage(id, treffer, "anna", Some(1)).is_some());
        }
        assert_eq!(w.gefallen.len(), 4);
        assert!(w.gefallen.iter().all(|g| g.von == "anna"));
        assert_eq!(w.anzahl(), 0);
        let niemand: [(PlayerId, Vec3); 0] = [];
        for _ in 0..((NEU_BESETZEN + 5.0) * 10.0) as usize {
            w.tick(0.1, &niemand, &|_| 0.0);
        }
        assert_eq!(w.anzahl(), 4, "Trupp nicht neu aufgetaucht");
    }

    #[test]
    fn jede_gegend_hat_eigene_streuner() {
        for gefahr in 1..=3u8 {
            for g in [KUESTE, WIESE, WALD, BERG] {
                let ort = Streifort { mitte: Vec2::ZERO, hoehe: 5.0, gefahr, gebiet: g };
                let arten = passend(&ort);
                assert!(!arten.is_empty() && arten.iter().all(|&i| ARTEN[i].gebiet & g != 0 && !ARTEN[i].dungeon), "Gefahr {gefahr}, Gegend {g}: {arten:?}");
            }
        }
    }

    #[test]
    fn pilzling_zuendet_rennt_los_und_platzt() {
        let pilzkreis = ARTEN.iter().position(|a| a.name == "Pilzkreis").unwrap();
        let mut w = Wildnis::new(vec![Lager::streifend(vec2(0.0, 0.0), 5.0, pilzkreis)], &|_| 5.0);
        let spieler = [(7u64, vec3(10.0, 5.0, 0.0))];
        // Treffen, bis einer zündet (40 % je Treffer)
        let ids: Vec<u16> = w.states().iter().map(|s| s.id).collect();
        let mut gezuendet = None;
        for _ in 0..40 {
            for &id in &ids {
                w.damage(id, Treffer { schaden: 0.1, art: DamageKind::Arcane, ..Default::default() }, "anna", Some(7));
                if w.states().iter().any(|s| s.id == id && s.flags & zustand::ZUENDET != 0) {
                    gezuendet = Some(id);
                    break;
                }
            }
            if gezuendet.is_some() {
                break;
            }
        }
        let id = gezuendet.expect("kein Pilzling zündet");
        let start = w.states().iter().find(|s| s.id == id).unwrap().position;
        let mut explosion = Vec::new();
        for _ in 0..(ZUENDZEIT * 60.0) as usize + 5 {
            explosion.extend(w.tick(1.0 / 60.0, &spieler, &|_| 5.0).into_iter().filter(|a| a.explosion));
            if !explosion.is_empty() {
                break;
            }
        }
        assert!(!explosion.is_empty(), "keine Explosion am Spieler");
        assert!(explosion.iter().all(|a| a.spieler == 7 && a.schaden > 10.0));
        assert_eq!(w.explosionen.len(), 1);
        assert!(w.gefallen.iter().any(|g| g.kind == EnemyKind::Pilzling && g.von == "anna"), "keine Beute für den Angreifer");
        let _ = start;
    }

    /// Ein Trupp einer bestimmten Art auf ebenem Boden.
    fn trupp(name: &str) -> Wildnis {
        let art = ARTEN.iter().position(|a| a.name == name).unwrap();
        Wildnis::new(vec![Lager::streifend(vec2(0.0, 0.0), 5.0, art)], &|_| 5.0)
    }

    #[test]
    fn keiler_nimmt_anlauf_und_rammt() {
        let mut w = trupp("Keilerrotte");
        let spieler = [(7u64, vec3(9.0, 5.0, 0.0))];
        let mut stoss = None;
        for _ in 0..(12 * 60) {
            for a in w.tick(1.0 / 60.0, &spieler, &|_| 5.0) {
                if a.stoss.length() > 5.0 {
                    stoss = Some(a);
                }
            }
            if stoss.is_some() {
                break;
            }
        }
        let a = stoss.expect("kein Ansturm");
        assert!(a.stoss.x > 3.0 && a.stoss.y > 3.0, "Rückstoß falsch: {}", a.stoss);
        assert!(w.ereignisse.iter().any(|e| matches!(e, crate::td::Ereignis::Ansturm(_))));
    }

    #[test]
    fn schamane_heilt_seinen_trupp() {
        let mut w = trupp("Goblintrupp");
        for x in w.wilde.iter_mut().filter(|x| x.kind == EnemyKind::Goblin) {
            x.health = x.max_health * 0.3;
        }
        let spieler = [(7u64, vec3(8.0, 5.0, 0.0))];
        for _ in 0..(12 * 60) {
            w.tick(1.0 / 60.0, &spieler, &|_| 5.0);
        }
        assert!(w.ereignisse.iter().any(|e| matches!(e, crate::td::Ereignis::Heilung(_))), "keine Heilung");
    }

    #[test]
    fn waldschrat_laesst_wurzeln_hervorbrechen() {
        let mut w = trupp("Waldschrat");
        let spieler = [(7u64, vec3(10.0, 5.0, 0.0))];
        let mut getroffen = false;
        for _ in 0..(12 * 60) {
            getroffen |= w.tick(1.0 / 60.0, &spieler, &|_| 5.0).iter().any(|a| a.kind == EnemyKind::Waldschrat && a.explosion && a.stoss.y > 5.0);
        }
        assert!(w.ereignisse.iter().any(|e| matches!(e, crate::td::Ereignis::Wurzelwarnung(_))), "keine Warnung");
        assert!(getroffen, "Wurzeln treffen den stehenden Spieler nicht");
    }

    #[test]
    fn ork_geraet_in_raserei() {
        let mut w = trupp("Orkkriegstrupp");
        let ork = w.wilde.iter().find(|x| x.kind == EnemyKind::Ork).unwrap().id;
        let treffer = Treffer { schaden: w.wilde.iter().find(|x| x.id == ork).unwrap().max_health * 0.6, art: DamageKind::Arcane, ..Default::default() };
        w.damage(ork, treffer, "anna", Some(7));
        w.tick(1.0 / 60.0, &[(7u64, vec3(5.0, 5.0, 0.0))], &|_| 5.0);
        assert!(w.states().iter().any(|s| s.id == ork && s.flags & zustand::WUT != 0), "keine Raserei");
    }

    #[test]
    fn dungeons_passen_sich_der_spielerzahl_an() {
        let lager = vec![Lager::im_dungeon(vec2(0.0, 0.0), 60.0, 8, (vec2(-15.0, -15.0), vec2(15.0, 15.0)))];
        let mut w = Wildnis::new(lager, &|_| 60.0);
        let allein = w.wilde[0].max_health;
        let drei = [(1u64, vec3(40.0, 61.0, 0.0)), (2, vec3(41.0, 61.0, 0.0)), (3, vec3(42.0, 61.0, 0.0))];
        w.tick(1.0 / 60.0, &drei, &|_| 60.0);
        let zu_dritt = w.wilde[0].max_health;
        assert!(zu_dritt > allein * 2.0, "zu dritt nicht zäher: {allein} → {zu_dritt}");
        assert!(dungeon_faktor(1).0 < 0.6 && dungeon_faktor(1).1 < 0.7, "allein nicht leichter");
    }

    #[test]
    fn fragmente_sind_selten_bei_den_kleinen() {
        assert!(fragment_chance(1, false) < 0.1);
        assert!(fragment_chance(3, true) > fragment_chance(1, true));
    }
}
