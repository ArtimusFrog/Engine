//! Lager in der Wildnis: kleine Trupps der Schattenfestung, die abseits der Heerstraßen lagern.
//!
//! Sie bewachen ihr Lager, greifen Spieler an, die zu nahe kommen (oder auf sie schießen), und
//! verfolgen sie bis zur Leine – dann kehren sie um und heilen sich. Besiegte lassen Gold und mit
//! etwas Glück ein Runenfragment fallen. Ein leeres Lager wird nach einer Weile neu besetzt.
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
/// Solange ein Spieler so nahe ist, wird ein leeres Lager nicht neu besetzt.
const BESETZEN_ABSTAND: f32 = 60.0;
/// Solange liegen Besiegte am Boden.
const STERBEN: f32 = 2.5;

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
}

const fn wild(name: &'static str, gefahr: u8, einheiten: &'static [EnemyKind], anfuehrer: EnemyKind) -> LagerArt {
    LagerArt { name, gefahr, einheiten, anfuehrer, leben: 1.0, dungeon: false, boss: false }
}

const fn tief(name: &'static str, gefahr: u8, einheiten: &'static [EnemyKind], anfuehrer: EnemyKind, leben: f32, boss: bool) -> LagerArt {
    LagerArt { name, gefahr, einheiten, anfuehrer, leben, dungeon: true, boss }
}

/// Eigene Bewohner der Wildnis – deutlich schwächer als die Truppen der Festung.
pub const ARTEN: [LagerArt; 17] = [
    wild("Spinnennest", 1, &[EnemyKind::Waldspinne, EnemyKind::Waldspinne, EnemyKind::Waldspinne, EnemyKind::Waldspinne], EnemyKind::Waldspinne),
    wild("Plündererlager", 1, &[EnemyKind::Pluenderer, EnemyKind::Pluenderer, EnemyKind::Pluenderer], EnemyKind::Pluenderer),
    wild("Banditenlager", 2, &[EnemyKind::Bandit, EnemyKind::Banditenschuetze, EnemyKind::Bandit, EnemyKind::Banditenschuetze], EnemyKind::Bandit),
    wild("Plündererbande", 2, &[EnemyKind::Pluenderer, EnemyKind::Pluenderer, EnemyKind::Banditenschuetze, EnemyKind::Pluenderer], EnemyKind::Bandit),
    wild("Banditenfestung", 3, &[EnemyKind::Bandit, EnemyKind::Banditenschuetze, EnemyKind::Pluenderer, EnemyKind::Bandit], EnemyKind::Bandit),
    // ---------- Spinnengrotte ----------
    tief("Spinnenbrut", 2, &[EnemyKind::Waldspinne, EnemyKind::Waldspinne, EnemyKind::Waldspinne, EnemyKind::Waldspinne, EnemyKind::Waldspinne], EnemyKind::Spinnling, 1.0, false),
    tief("Harpyiennest", 3, &[EnemyKind::Harpy, EnemyKind::Harpy, EnemyKind::Waldspinne, EnemyKind::Waldspinne], EnemyKind::Harpy, 1.0, false),
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
];

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
}

impl Lager {
    pub fn art(&self) -> &'static LagerArt {
        &ARTEN[self.art]
    }

    /// Ein Raum in einem Dungeon.
    pub fn im_dungeon(mitte: Vec2, hoehe: f32, art: usize, bereich: (Vec2, Vec2)) -> Lager {
        Lager { mitte, hoehe, art, leer_seit: None, bereich: Some(bereich) }
    }

    /// Bis hierhin (Meter vom Lager) verfolgen die Bewohner einen Spieler.
    fn leine(&self) -> f32 {
        32.0 + 6.0 * self.art().gefahr as f32
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
    /// Posten im Lager
    heim: Vec2,
    position: Vec3,
    facing: f32,
    health: f32,
    max_health: f32,
    schlag: f32,
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
}

impl Wilder {
    fn groesse(&self) -> f32 {
        self.kind.groesse() * if self.anfuehrer { 1.35 } else { 1.0 }
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
}

pub struct Wildnis {
    pub lager: Vec<Lager>,
    wilde: Vec<Wilder>,
    next_id: u16,
    rng: Rng,
    uhr: f32,
    pub gefallen: Vec<WildGefallen>,
    /// Neu besetzte Lager seit dem letzten Abholen (Name, Ort)
    pub besetzt: Vec<(&'static str, Vec2)>,
}

impl Wildnis {
    pub fn new(lager: Vec<Lager>, boden: &dyn Fn(Vec2) -> f32) -> Wildnis {
        let mut wildnis = Wildnis { lager, wilde: Vec::new(), next_id: WILD_ID, rng: Rng::new(0x_57_11D), uhr: 0.0, gefallen: Vec::new(), besetzt: Vec::new() };
        for i in 0..wildnis.lager.len() {
            wildnis.besetzen(i, boden);
        }
        wildnis.besetzt.clear();
        wildnis
    }

    /// Sucht Lagerplätze: fester Zufall (auf allen Rechnern gleich), ebener Boden an Land, weit weg
    /// von Heerstraßen, Siedlungsplätzen, Burg, Festung, Startlager und voneinander. Die Gefahr
    /// steigt zur Inselmitte (zur Festung) hin.
    pub fn plaetze(terrain: &Terrain, meiden: &[(Vec2, f32)], strassen: &[Vec<Vec2>], anzahl: usize) -> Vec<Lager> {
        let mut rng = Rng::new(0x1A6E_2);
        let mut lager: Vec<Lager> = Vec::new();
        let radius = crate::island::ISLAND_RADIUS;
        for _ in 0..6000 {
            if lager.len() >= anzahl {
                break;
            }
            let p = vec2(rng.range(-radius, radius), rng.range(-radius, radius));
            let r = p.length();
            if !(170.0..radius * 0.82).contains(&r) {
                continue;
            }
            let h = terrain.height_at(p.x, p.y);
            if h < 3.0 {
                continue;
            }
            let steil = [vec2(5.0, 0.0), vec2(-5.0, 0.0), vec2(0.0, 5.0), vec2(0.0, -5.0)]
                .iter()
                .map(|d| (terrain.height_at(p.x + d.x, p.y + d.y) - h).abs())
                .fold(0.0f32, f32::max);
            if steil > 1.6 {
                continue;
            }
            if meiden.iter().any(|&(q, weite)| q.distance(p) < weite) {
                continue;
            }
            if strassen.iter().flatten().any(|q| q.distance(p) < 45.0) {
                continue;
            }
            if crate::island::burg_rand(p) < 60.0 || crate::island::burg_weg(p).0 < 30.0 || crate::island::festung_rand(p) < 60.0 {
                continue;
            }
            if lager.iter().any(|l| l.mitte.distance(p) < 150.0) {
                continue;
            }
            // Gefahr: außen harmlos, zur Festung hin gefährlich
            let gefahr = if r > radius * 0.64 { 1 } else if r > radius * 0.44 { 2 } else { 3 };
            let passend: Vec<usize> = (0..ARTEN.len()).filter(|&i| ARTEN[i].gefahr == gefahr && !ARTEN[i].dungeon).collect();
            let art = passend[lager.len() % passend.len()];
            lager.push(Lager { mitte: p, hoehe: h, art, leer_seit: None, bereich: None });
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
                mitte + vec2(w.cos(), w.sin()) * self.rng.range(4.5, 6.5)
            };
            let id = self.next_id;
            self.next_id = if self.next_id == u16::MAX { WILD_ID } else { self.next_id + 1 };
            let heim = lager.begrenzen(heim, 1.5);
            let max_health = kind.max_health() * leben * art.leben * if anfuehrer { 2.2 } else { 1.0 };
            neu.push(Wilder {
                id,
                kind,
                lager: index,
                anfuehrer,
                heim,
                position: vec3(heim.x, boden(heim), heim.y),
                facing: self.rng.range(0.0, std::f32::consts::TAU),
                health: max_health,
                max_health,
                schlag: kind.schlag() * schlag * if anfuehrer { 1.3 } else { 1.0 },
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
            });
        }
        self.wilde.extend(neu);
        self.lager[index].leer_seit = None;
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
        for w in &mut self.wilde {
            if let Some(seit) = &mut w.dying {
                *seit += dt;
                continue;
            }
            let lager = &self.lager[w.lager];
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
            if w.stun > 0.0 {
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
            let tempo = w.kind.speed() * 1.3 * (1.0 - w.slow.0);
            let hier = vec2(w.position.x, w.position.z);
            let (weg, zu) = match w.ziel.and_then(finde) {
                Some(p) => {
                    let ziel = vec2(p.x, p.z);
                    let (reichweite, pause) = w.kind.attack();
                    let reichweite = reichweite.min(16.0) + w.radius() * 0.5;
                    let abstand = hier.distance(ziel);
                    if abstand <= reichweite {
                        if w.cooldown <= 0.0 {
                            w.cooldown = pause;
                            w.attacking = 0.7;
                            angriffe.push(WildAngriff { kind: w.kind, von: w.center(), ziel: p, spieler: w.ziel.unwrap_or_default(), schaden: w.schlag });
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
        // Leere Lager nach einer Weile neu besetzen (nicht vor den Augen der Spieler)
        for i in 0..self.lager.len() {
            if self.wilde.iter().any(|w| w.lager == i) {
                continue;
            }
            let seit = *self.lager[i].leer_seit.get_or_insert(self.uhr);
            let mitte = self.lager[i].mitte;
            let beobachtet = spieler.iter().any(|(_, p)| vec2(p.x, p.z).distance(mitte) < BESETZEN_ABSTAND);
            if self.uhr - seit > NEU_BESETZEN && !beobachtet {
                self.besetzen(i, boden);
            }
        }
        angriffe
    }

    /// Schaden an einem Bewohner. `spieler`: wer getroffen hat (das Lager nimmt ihn ins Visier).
    /// Liefert die Art, wenn er dabei fällt.
    pub fn damage(&mut self, id: u16, treffer: Treffer, von: &str, spieler: Option<PlayerId>) -> Option<EnemyKind> {
        let w = self.wilde.iter_mut().find(|w| w.id == id && w.lebt())?;
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
                    boss: w.anfuehrer,
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

    fn wildnis() -> Wildnis {
        let lager = vec![Lager { mitte: vec2(0.0, 0.0), hoehe: 0.0, art: 1, leer_seit: None, bereich: None }];
        Wildnis::new(lager, &|_| 0.0)
    }

    #[test]
    fn lager_bemerkt_verfolgt_und_laesst_los() {
        let mut w = wildnis();
        let n = w.anzahl();
        assert_eq!(n, 4, "Anführer und drei Bewohner");
        // Weit weg: niemand greift an
        let weit = [(7u64, vec3(80.0, 0.0, 0.0))];
        for _ in 0..120 {
            assert!(w.tick(1.0 / 60.0, &weit, &|_| 0.0).is_empty());
        }
        // Nah dran: sie kommen und schlagen zu
        let nah = [(7u64, vec3(9.0, 0.0, 0.0))];
        let mut angriffe = 0;
        for _ in 0..600 {
            angriffe += w.tick(1.0 / 60.0, &nah, &|_| 0.0).len();
        }
        assert!(angriffe > 3, "zu wenige Angriffe: {angriffe}");
        // Außerhalb der Leine: sie kehren heim
        let weg = [(7u64, vec3(70.0, 0.0, 0.0))];
        for _ in 0..1800 {
            w.tick(1.0 / 60.0, &weg, &|_| 0.0);
        }
        let weiteste = w.wilde.iter().map(|x| vec2(x.position.x, x.position.z).length()).fold(0.0f32, f32::max);
        assert!(weiteste < 8.0, "nicht heimgekehrt: {weiteste}");
    }

    #[test]
    fn besiegte_melden_sich_und_das_lager_kommt_wieder() {
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
        assert_eq!(w.anzahl(), 4, "Lager nicht neu besetzt");
    }

    #[test]
    fn fragmente_sind_selten_bei_den_kleinen() {
        assert!(fragment_chance(1, false) < 0.1);
        assert!(fragment_chance(3, true) > fragment_chance(1, true));
    }
}
