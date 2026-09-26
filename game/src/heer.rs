//! Die Truppen der Schattenfestung: Gruppen gegnerischer Einheiten entstehen im Festungshof,
//! marschieren durch die vier Tore, die Rampen hinab und die Straßen entlang und verschwinden am
//! Ende der Straße. Spieler in der Nähe greifen sie an; mit dem Zauberstab lassen sie sich besiegen.
//!
//! Der Server rechnet alles (`Heer`), die Clients bekommen mit jedem Schnappschuss die sichtbaren
//! Einheiten (`EnemyState`) und zeigen sie an (`HeerAnsicht`).
//! Modelle: `game/assets/gegner/` (Blender: art/lib/gegner.py).

use std::collections::HashMap;
use std::sync::Arc;

use engine::prelude::*;
use serde::{Deserialize, Serialize};

use crate::asset_files;
use crate::tuerme::DamageKind;
use crate::world::SoundEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnemyKind {
    Knight,
    Archer,
    Pikeman,
    Skeleton,
    Warlock,
    Golem,
    Wolf,
    Ghost,
}

impl EnemyKind {
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
        }
    }

    fn file_name(self) -> &'static str {
        match self {
            EnemyKind::Knight => "dunkler_ritter",
            EnemyKind::Archer => "bogenschuetze",
            EnemyKind::Pikeman => "pikenier",
            EnemyKind::Skeleton => "skelettkrieger",
            EnemyKind::Warlock => "dunkelmagier",
            EnemyKind::Golem => "steingolem",
            EnemyKind::Wolf => "schattenwolf",
            EnemyKind::Ghost => "gespenst",
        }
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
        }
    }

    /// Rüstung gegen physischen Schaden (Anteil, der abgehalten wird).
    pub fn armor(self) -> f32 {
        match self {
            EnemyKind::Knight => 0.4,
            EnemyKind::Archer | EnemyKind::Skeleton => 0.1,
            EnemyKind::Pikeman => 0.25,
            EnemyKind::Golem => 0.6,
            EnemyKind::Ghost => 0.75,
            EnemyKind::Warlock | EnemyKind::Wolf => 0.0,
        }
    }

    fn undead(self) -> bool {
        matches!(self, EnemyKind::Skeleton | EnemyKind::Ghost)
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
            DamageKind::Fire if self == EnemyKind::Golem => 0.5,
            DamageKind::Fire => 1.0,
        }
    }

    /// Marschtempo (m/s); eine Gruppe läuft so schnell wie ihr langsamstes Mitglied.
    fn speed(self) -> f32 {
        match self {
            EnemyKind::Golem => 2.0,
            EnemyKind::Warlock => 2.4,
            EnemyKind::Wolf => 3.6,
            EnemyKind::Ghost => 2.8,
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
            _ => (2.4, 1.8),
        }
    }

    /// Mittelpunkt und Radius der Trefferkugel über den Füßen.
    pub fn hit_sphere(self) -> (f32, f32) {
        match self {
            EnemyKind::Golem => (1.5, 1.2),
            EnemyKind::Wolf => (0.7, 0.8),
            _ => (1.0, 0.6),
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
    #[serde(default)]
    pub boss: bool,
}

/// Ein Treffer: Schaden, Art und Nachwirkungen (Verlangsamung, Brand).
#[derive(Clone, Copy, Debug, Default)]
pub struct Hit {
    pub schaden: f32,
    pub art: DamageKind,
    /// Verlangsamung (Anteil) für 2,5 s
    pub bremse: f32,
    /// Brand: Schaden pro Sekunde für `dauer` Sekunden
    pub brand: f32,
    pub dauer: f32,
}

impl Default for DamageKind {
    fn default() -> Self {
        DamageKind::Physical
    }
}

// ---------- Marschrouten ----------

/// Weg einer Gruppe: Festungshof → Tor → Rampe → Straße bis zum Ende.
#[derive(Clone, Debug)]
pub struct Route {
    points: Vec<Vec3>,
    /// Zurückgelegte Strecke bis zu jedem Punkt
    along: Vec<f32>,
}

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
        Route { points, along }
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
}

// ---------- Die Truppen (nur auf dem Server) ----------

/// Zusammensetzungen der Gruppen (vorne → hinten).
const GRUPPEN: [&[EnemyKind]; 6] = [
    &[EnemyKind::Knight, EnemyKind::Knight, EnemyKind::Pikeman, EnemyKind::Pikeman, EnemyKind::Archer, EnemyKind::Archer],
    &[EnemyKind::Skeleton, EnemyKind::Skeleton, EnemyKind::Skeleton, EnemyKind::Skeleton, EnemyKind::Ghost],
    &[EnemyKind::Wolf, EnemyKind::Wolf, EnemyKind::Wolf, EnemyKind::Wolf],
    &[EnemyKind::Knight, EnemyKind::Knight, EnemyKind::Warlock, EnemyKind::Warlock],
    &[EnemyKind::Golem, EnemyKind::Pikeman, EnemyKind::Pikeman, EnemyKind::Archer],
    &[EnemyKind::Ghost, EnemyKind::Ghost, EnemyKind::Skeleton, EnemyKind::Warlock, EnemyKind::Skeleton],
];
/// Abstand zwischen zwei Wellen (s) und Obergrenze gleichzeitiger Einheiten.
pub const WAVE_SECONDS: f32 = 45.0;
/// Leben der Insel: so viele Durchbrüche hält sie aus.
pub const MAX_LEBEN: u32 = 20;
/// Pause nach einer Niederlage, bevor es wieder bei Welle 1 losgeht (s).
const NEUSTART_SECONDS: f32 = 60.0;
/// Anführer der Bosswellen (jede fünfte Welle, reihum)
const BOSSE: [EnemyKind; 4] = [EnemyKind::Golem, EnemyKind::Knight, EnemyKind::Warlock, EnemyKind::Ghost];
const MAX_ENEMIES: usize = 90;
/// Ab dieser Entfernung zu einem Spieler bleibt eine Gruppe stehen und kämpft.
const ENGAGE: f32 = 16.0;

struct Member {
    id: u16,
    kind: EnemyKind,
    /// Anführer einer Bosswelle und die (mit der Welle gewachsenen) vollen Lebenspunkte
    boss: bool,
    max_health: f32,
    /// Platz in der Formation (Reihe hinter der Spitze, seitlich)
    row: f32,
    side: f32,
    health: f32,
    cooldown: f32,
    /// Wie weit die Einheit auf ihrer Route ist (für die Zielwahl der Türme: „erstes“)
    progress: f32,
    /// Verlangsamung (Anteil, Restzeit), Brand (Schaden/s, Restzeit, von wem), Rüstung geschwächt (Anteil, Restzeit)
    slow: (f32, f32),
    burn: (f32, f32),
    burn_by: Option<String>,
    weak: (f32, f32),
    /// Seit wann besiegt (dann nach kurzer Zeit weg)
    dying: Option<f32>,
    /// Weicht zum Kämpfen von seinem Platz ab
    offset: Vec2,
    attacking: f32,
    position: Vec3,
    facing: f32,
}

struct Group {
    route: usize,
    distance: f32,
    speed: f32,
    halted: bool,
    members: Vec<Member>,
}

/// Ein Angriff einer Einheit auf einen Spieler (für Geschosse und Klang).
#[derive(Clone, Copy, Debug)]
pub struct Strike {
    pub kind: EnemyKind,
    pub from: Vec3,
    pub target: Vec3,
}

pub struct Heer {
    routes: Vec<Route>,
    groups: Vec<Group>,
    next_id: u16,
    timer: f32,
    /// Wellen aus der Festung an (Admin-Panel)
    pub enabled: bool,
    rng: Rng,
    /// Besiegte Einheiten seit dem letzten Abholen: wer den letzten Treffer gesetzt hat, welche Art
    pub gefallen: Vec<(String, EnemyKind)>,
    /// Nummer der zuletzt losgeschickten Welle (0 = noch keine) und die Leben der Insel
    pub welle: u32,
    pub leben: u32,
    /// Meldungen für alle Spieler (neue Welle, Durchbruch, Niederlage)
    pub meldungen: Vec<String>,
}

impl Heer {
    pub fn new(routes: Vec<Route>) -> Heer {
        Heer { routes, groups: Vec::new(), next_id: 1, timer: 3.0, enabled: false, rng: Rng::new(0x7E_E4), gefallen: Vec::new(), welle: 0, leben: MAX_LEBEN, meldungen: Vec::new() }
    }

    /// Die Heerstraßen (ab dem Fuß der Rampe) als Linien (x, z) – für die Karte.
    pub fn strassen(&self) -> impl Iterator<Item = Vec<Vec2>> + '_ {
        self.routes.iter().map(|r| r.points.iter().skip(5).map(|p| vec2(p.x, p.z)).collect())
    }

    pub fn count(&self) -> usize {
        self.groups.iter().map(|g| g.members.len()).sum()
    }

    /// Wellen an/aus; beim Einschalten kommt sofort die erste.
    pub fn set_enabled(&mut self, on: bool) {
        if on && !self.enabled {
            self.spawn_wave();
        }
        self.enabled = on;
    }

    /// Sekunden bis zur nächsten Welle
    pub fn naechste_in(&self) -> f32 {
        self.timer.max(0.0)
    }

    /// Stand der Wellen vom Server übernehmen (Client).
    pub fn sync(&mut self, welle: u32, leben: u32, naechste: f32) {
        (self.welle, self.leben, self.timer) = (welle, leben, naechste);
    }

    /// Enden der Heerstraßen (dort stehen die Schutzsteine): Ort und Richtung der Straße.
    pub fn enden(&self) -> Vec<(Vec3, Vec2)> {
        self.routes.iter().map(|r| r.sample(r.length())).collect()
    }

    /// Wellen zurück auf Anfang (Admin): Welle 0, volle Leben, alle Truppen weg.
    pub fn reset(&mut self) {
        self.groups.clear();
        self.welle = 0;
        self.leben = MAX_LEBEN;
        self.timer = 5.0;
    }

    /// Eine Welle: auf jeder Straße eine Gruppe. Mit jeder Welle werden die Truppen zäher
    /// (+15 % Leben), alle drei Wellen kommt je Gruppe eine Einheit dazu, jede fünfte Welle
    /// führt ein Boss an. Die nächste Welle kommt frühestens nach `WAVE_SECONDS`.
    pub fn spawn_wave(&mut self) {
        self.timer = WAVE_SECONDS;
        if self.count() > MAX_ENEMIES {
            return;
        }
        self.welle += 1;
        let welle = self.welle;
        let zaeh = 1.0 + 0.15 * (welle - 1) as f32;
        let boss = (welle % 5 == 0).then(|| BOSSE[((welle / 5 - 1) % BOSSE.len() as u32) as usize]);
        self.meldungen.push(match boss {
            Some(kind) => format!("Welle {welle}: Bosswelle! Ein gewaltiger {} führt die Truppen an.", kind.label()),
            None => format!("Welle {welle} bricht aus der Schattenfestung hervor."),
        });
        for route in 0..self.routes.len() {
            let vorlage = GRUPPEN[(self.rng.next_u32() % GRUPPEN.len() as u32) as usize];
            // Mehr Einheiten mit jeder dritten Welle (reihum aus der Vorlage), der Boss vorneweg
            let extra = ((welle - 1) / 3).min(6) as usize;
            let mut template: Vec<(EnemyKind, bool)> = boss.iter().map(|&k| (k, true)).collect();
            template.extend(vorlage.iter().chain(vorlage.iter().cycle().take(extra)).map(|&k| (k, false)));
            let columns = if template.len() >= 5 { 3.0 } else { 2.0 };
            let members: Vec<Member> = template
                .iter()
                .enumerate()
                .map(|(i, &(kind, boss))| {
                    let row = (i as f32 / columns).floor();
                    let side = (i as f32 % columns) - (columns - 1.0) / 2.0;
                    let id = self.next_id;
                    self.next_id = self.next_id.wrapping_add(1).max(1);
                    let max_health = kind.max_health() * zaeh * if boss { 6.0 } else { 1.0 };
                    Member {
                        id,
                        kind,
                        boss,
                        max_health,
                        row,
                        side,
                        health: max_health,
                        cooldown: self.rng.range(0.3, 1.5),
                        progress: 0.0,
                        slow: (0.0, 0.0),
                        burn: (0.0, 0.0),
                        burn_by: None,
                        weak: (0.0, 0.0),
                        dying: None,
                        offset: Vec2::ZERO,
                        attacking: 0.0,
                        position: Vec3::ZERO,
                        facing: 0.0,
                    }
                })
                .collect();
            let speed = members.iter().map(|m| m.kind.speed()).fold(f32::MAX, f32::min);
            // Die hinteren Reihen starten weiter drinnen im Hof
            let rows = members.iter().map(|m| m.row).fold(0.0, f32::max);
            self.groups.push(Group { route, distance: rows * 2.4, speed, halted: false, members });
        }
    }

    /// Ein Takt: Wellen, Marsch, Kampf. Liefert die Angriffe dieses Takts.
    pub fn tick(&mut self, dt: f32, players: &[Vec3]) -> Vec<Strike> {
        let mut strikes = Vec::new();
        if self.enabled {
            self.timer -= dt;
            if self.timer <= 0.0 {
                self.timer = WAVE_SECONDS;
                self.spawn_wave();
            }
        }
        let gefallen = &mut self.gefallen;
        for group in &mut self.groups {
            let route = &self.routes[group.route];
            let (lead, _) = route.sample(group.distance);
            // Stehen bleiben, sobald ein Spieler nah an der Spitze ist – die vorderen Reihen kämpfen
            group.halted = players.iter().any(|p| p.distance(lead) < ENGAGE - 2.0);
            // Die Gruppe ist so schnell wie ihr langsamstes (ggf. vereistes) Mitglied
            group.speed = group
                .members
                .iter()
                .filter(|m| m.dying.is_none())
                .map(|m| m.kind.speed() * (1.0 - m.slow.0))
                .fold(f32::MAX, f32::min)
                .min(4.0);
            if !group.halted {
                group.distance += group.speed * dt;
            }
            for member in &mut group.members {
                if let Some(t) = &mut member.dying {
                    *t += dt;
                    continue;
                }
                // Nachwirkungen: Verlangsamung und Schwächung laufen ab, Brand zehrt
                member.slow.1 -= dt;
                if member.slow.1 <= 0.0 {
                    member.slow = (0.0, 0.0);
                }
                member.weak.1 -= dt;
                if member.weak.1 <= 0.0 {
                    member.weak = (0.0, 0.0);
                }
                if member.burn.1 > 0.0 {
                    member.burn.1 -= dt;
                    member.health -= member.burn.0 * dt * member.kind.factor(DamageKind::Fire, 0.0);
                    if member.health <= 0.0 {
                        member.health = 0.0;
                        member.dying = Some(0.0);
                        gefallen.push((member.burn_by.clone().unwrap_or_default(), member.kind));
                        continue;
                    }
                }
                member.progress = group.distance - member.row * 2.4;
                let (spot, dir) = route.sample(group.distance - member.row * 2.4);
                let side = vec2(-dir.y, dir.x) * member.side * 1.5;
                let home = vec2(spot.x, spot.z) + side;
                member.cooldown -= dt;
                member.attacking = (member.attacking - dt).max(0.0);
                let (range, pause) = member.kind.attack();
                let target = players
                    .iter()
                    .copied()
                    .filter(|p| vec2(p.x, p.z).distance(home + member.offset) < ENGAGE + range + 4.0)
                    .min_by(|a, b| a.distance(member.position).total_cmp(&b.distance(member.position)));
                let mut facing_dir = dir;
                match target {
                    Some(player) => {
                        let to = vec2(player.x, player.z) - (home + member.offset);
                        let distance = to.length();
                        // Nahkämpfer gehen ein Stück auf den Spieler zu, Fernkämpfer bleiben stehen
                        if distance > range * 0.8 && range < 5.0 {
                            let step = to.normalize_or_zero() * member.kind.speed() * dt;
                            member.offset = (member.offset + step).clamp_length_max(ENGAGE);
                        }
                        facing_dir = to.normalize_or(dir);
                        if distance <= range && member.cooldown <= 0.0 {
                            member.cooldown = pause;
                            member.attacking = 0.9;
                            strikes.push(Strike { kind: member.kind, from: member.position + Vec3::Y * 1.3, target: player });
                        }
                    }
                    None => member.offset *= (1.0 - dt * 1.5).max(0.0),
                }
                let flat = home + member.offset;
                // Höhe: auf der Route (Hof, Rampe) bzw. dem Gelände – die Route kennt beides
                member.position = vec3(flat.x, spot.y, flat.y);
                member.facing = facing_dir.x.atan2(facing_dir.y);
            }
            group.members.retain(|m| m.dying.is_none_or(|t| t < 2.0));
        }
        // Am Ende der Straße: Durchbruch – die Insel verliert Leben, die Einheit verschwindet
        let mut verlust = 0;
        for group in &mut self.groups {
            let ende = self.routes[group.route].length();
            let distance = group.distance;
            group.members.retain(|m| {
                let durch = m.dying.is_none() && distance - m.row * 2.4 >= ende;
                if durch {
                    let wert = if m.boss { 5 } else if m.kind == EnemyKind::Golem { 3 } else { 1 };
                    verlust += wert;
                    self.meldungen.push(format!("{}{} ist durchgebrochen! (−{wert} {})", if m.boss { "Der Boss " } else { "" }, m.kind.label(), "Leben"));
                }
                !durch
            });
        }
        self.groups.retain(|g| !g.members.is_empty());
        if verlust > 0 {
            self.leben = self.leben.saturating_sub(verlust);
            if self.leben == 0 {
                self.meldungen.push(format!("Die Insel ist gefallen – nach {} Wellen. In {:.0} Sekunden geht es von vorn los.", self.welle, NEUSTART_SECONDS));
                self.groups.clear();
                self.welle = 0;
                self.leben = MAX_LEBEN;
                self.timer = NEUSTART_SECONDS;
            }
        }
        strikes
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
                    } else if g.halted {
                        EnemyAction::Idle
                    } else {
                        EnemyAction::Walk
                    },
                    health: if m.dying.is_some() { 0 } else { ((m.health / m.max_health) * 100.0).ceil().clamp(1.0, 100.0) as u8 },
                    boss: m.boss,
                })
            })
            .collect()
    }

    fn alive(&self) -> impl Iterator<Item = &Member> {
        self.groups.iter().flat_map(|g| &g.members).filter(|m| m.dying.is_none())
    }

    fn center(m: &Member) -> Vec3 {
        m.position + Vec3::Y * m.kind.hit_sphere().0
    }

    /// Ziel eines Turms: die Einheit in Reichweite, die auf der Straße am weitesten gekommen ist.
    pub fn first_in_range(&self, from: Vec3, range: f32, minimum: f32) -> Option<(u16, Vec3)> {
        self.alive()
            .filter(|m| {
                let d = Self::center(m).distance(from);
                d <= range && d >= minimum
            })
            .max_by(|a, b| a.progress.total_cmp(&b.progress))
            .map(|m| (m.id, Self::center(m)))
    }

    /// Nächste Einheit um `at` (für den Kettenblitz), ohne die schon getroffenen.
    pub fn nearest_except(&self, at: Vec3, radius: f32, except: &[u16]) -> Option<(u16, Vec3)> {
        self.alive()
            .filter(|m| !except.contains(&m.id) && Self::center(m).distance(at) <= radius)
            .min_by(|a, b| Self::center(a).distance(at).total_cmp(&Self::center(b).distance(at)))
            .map(|m| (m.id, Self::center(m)))
    }

    /// Alle Einheiten im Umkreis (Flächenschaden).
    pub fn within(&self, at: Vec3, radius: f32) -> Vec<u16> {
        self.alive().filter(|m| m.position.distance(at) <= radius + m.kind.hit_sphere().1).map(|m| m.id).collect()
    }

    /// Schaden an einer Einheit (mit Rüstung und Empfindlichkeiten, Verlangsamung, Brand).
    /// `von`: wer getroffen hat – bekommt bei einem Sieg die Beute. Liefert die Art, falls besiegt.
    pub fn damage(&mut self, id: u16, hit: Hit, von: &str) -> Option<EnemyKind> {
        let member = self.groups.iter_mut().flat_map(|g| g.members.iter_mut()).find(|m| m.id == id && m.dying.is_none())?;
        member.health -= hit.schaden * member.kind.factor(hit.art, member.weak.0);
        if hit.bremse > 0.0 {
            let bremse = if member.kind == EnemyKind::Golem { hit.bremse * 0.5 } else { hit.bremse };
            member.slow = (member.slow.0.max(bremse), 2.5);
        }
        if hit.brand > 0.0 {
            member.burn = (member.burn.0.max(hit.brand), hit.dauer);
            member.burn_by = Some(von.to_string());
        }
        if member.health > 0.0 {
            return None;
        }
        member.health = 0.0;
        member.dying = Some(0.0);
        self.gefallen.push((von.to_string(), member.kind));
        Some(member.kind)
    }

    /// Gift schwächt die Rüstung (für eine Sekunde, wird in der Wolke laufend erneuert).
    pub fn weaken(&mut self, id: u16, amount: f32) {
        if let Some(member) = self.groups.iter_mut().flat_map(|g| g.members.iter_mut()).find(|m| m.id == id) {
            member.weak = (member.weak.0.max(amount), 1.0);
        }
    }

    /// Welche Einheit liegt auf dem Strahl zuerst (vor `nearest`)? Liefert (ID, Entfernung).
    pub fn ray_hit(&self, from: Vec3, direction: Vec3, nearest: f32) -> Option<(u16, f32)> {
        let mut best = None;
        let mut limit = nearest;
        for member in self.groups.iter().flat_map(|g| &g.members).filter(|m| m.dying.is_none()) {
            let (up, radius) = member.kind.hit_sphere();
            let center = member.position + Vec3::Y * up;
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
}

// ---------- Darstellung (nur mit Fenster) ----------

struct Figur {
    entity: EntityId,
    animator: Animator,
    shown: Vec3,
    facing: f32,
    action: EnemyAction,
    kind: EnemyKind,
    health: u8,
    flash: f32,
    fallen: f32,
}

#[derive(Default)]
pub struct HeerAnsicht {
    models: HashMap<EnemyKind, Option<(Arc<Model>, MeshId)>>,
    figuren: HashMap<u16, Figur>,
}

/// Bis zu dieser Entfernung zur Kamera werden die Animationen gerechnet.
const ANIMATION_DISTANCE: f32 = 110.0;

impl HeerAnsicht {
    fn model(&mut self, ctx: &mut Context, kind: EnemyKind) -> Option<(Arc<Model>, MeshId)> {
        self.models
            .entry(kind)
            .or_insert_with(|| {
                let path = asset_files::variants("gegner", kind.file_name()).into_iter().next()?;
                let model = match Model::from_file(&path) {
                    Ok(model) => Arc::new(model),
                    Err(message) => {
                        log::warn!("{}: {message}", kind.label());
                        return None;
                    }
                };
                let textures = model.register_textures(&mut ctx.assets);
                let texture = textures.first().copied();
                let mesh = ctx.assets.named_mesh(&format!("gegner_gpu_{}", kind.file_name()), || model.skinned_gpu_mesh(texture));
                Some((model, mesh))
            })
            .clone()
    }

    /// Einmal pro Bild: Einheiten anlegen, bewegen, animieren, entfernen.
    pub fn update(&mut self, ctx: &mut Context, states: &[EnemyState], sounds: &mut Vec<SoundEvent>) {
        let dt = ctx.time.delta;
        // Weg, was nicht mehr gemeldet wird
        let gone: Vec<u16> = self.figuren.keys().copied().filter(|id| !states.iter().any(|s| s.id == *id)).collect();
        for id in gone {
            if let Some(figur) = self.figuren.remove(&id) {
                ctx.scene.despawn(figur.entity);
            }
        }
        for state in states {
            if !self.figuren.contains_key(&state.id) {
                let Some((model, mesh)) = self.model(ctx, state.kind) else { continue };
                let mut animator = Animator::new(model);
                animator.play("Idle", true, 0.0);
                let mut entity = Entity::new(state.kind.label(), mesh).with_transform(Transform::from_position(state.position));
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
                        kind: state.kind,
                        health: state.health,
                        flash: 0.0,
                        fallen: 0.0,
                    },
                );
            }
            let Some(figur) = self.figuren.get_mut(&state.id) else { continue };
            if state.health < figur.health {
                figur.flash = 0.25;
                let (up, radius) = figur.kind.hit_sphere();
                ctx.particles.burst(Burst {
                    position: figur.shown + Vec3::Y * up,
                    count: if state.health == 0 { 36 } else { 14 },
                    color: vec3(0.6, 0.45, 1.0),
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
            figur.health = state.health;
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
            }
            figur.flash = (figur.flash - dt).max(0.0);
            if near {
                figur.animator.update(dt);
            }
            let Some(entity) = ctx.scene.try_get_mut(figur.entity) else { continue };
            entity.visible = figur.shown.distance(ctx.camera.position) < 320.0;
            if near {
                entity.joints = figur.animator.palette();
            }
            let fall = figur.fallen * figur.fallen;
            let (up, _) = figur.kind.hit_sphere();
            entity.transform.position = figur.shown - Vec3::Y * (fall * up * 0.6 + (figur.fallen - 0.5).max(0.0) * 1.2);
            entity.transform.rotation = Quat::from_rotation_y(figur.facing) * Quat::from_rotation_z(fall * std::f32::consts::FRAC_PI_2 * 0.95);
            entity.transform.scale = Vec3::splat(if state.boss { 1.6 } else { 1.0 });
            entity.color = Vec4::ONE.lerp(vec4(2.2, 0.35, 0.3, 1.0), figur.flash / 0.25);
        }
    }

    /// Gegner unter dem Fadenkreuz (für die Lebensleiste): Name, Lebenspunkte, Mitte.
    pub fn aimed(&self, from: Vec3, direction: Vec3, max: f32) -> Option<(EnemyKind, u8, Vec3)> {
        let mut best: Option<(f32, &Figur)> = None;
        for figur in self.figuren.values().filter(|f| f.action != EnemyAction::Dying) {
            let (up, radius) = figur.kind.hit_sphere();
            let center = figur.shown + Vec3::Y * up;
            let along = (center - from).dot(direction);
            if along <= 0.0 || along > max {
                continue;
            }
            if (from + direction * along).distance_squared(center) < radius * radius * 1.4 && best.is_none_or(|(d, _)| along < d) {
                best = Some((along, figur));
            }
        }
        best.map(|(_, f)| (f.kind, f.health, f.shown + Vec3::Y * f.kind.hit_sphere().0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        world.heer.set_enabled(true);
        let anfang = world.heer.count();
        assert!(anfang >= 4 * 4, "zu wenige Einheiten: {anfang}");
        world.heer.enabled = false;
        // Ohne Spieler marschieren alle bis ans Ende und verschwinden dort
        let mut unterwegs = false;
        for _ in 0..3000 {
            world.heer.tick(0.1, &[]);
            let states = world.heer.states();
            unterwegs |= states.iter().any(|s| s.position.distance(Vec3::ZERO) > 150.0);
            if states.is_empty() {
                break;
            }
        }
        assert!(unterwegs, "Truppen kommen nicht aus der Festung");
        assert_eq!(world.heer.count(), 0, "Truppen verschwinden nicht am Ende der Straße");
    }

    #[test]
    fn wellen_werden_staerker_und_durchbrueche_kosten_leben() {
        let mut ctx = Context::headless();
        let mut world = crate::world::World::new(&mut ctx);
        let heer = &mut world.heer;
        assert_eq!((heer.welle, heer.leben), (0, MAX_LEBEN));
        // Welle 1: gewöhnliche Truppen mit Grundleben, kein Boss
        heer.spawn_wave();
        assert_eq!(heer.welle, 1);
        let erste = heer.count();
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
        assert!((golem.max_health - EnemyKind::Golem.max_health() * 1.6 * 6.0).abs() < 0.1, "Boss-Leben {}", golem.max_health);
        assert!(heer.count() > erste, "Welle 5 ist nicht größer als Welle 1");

        // Durchbrüche: jede Einheit am Straßenende kostet Leben
        heer.clear();
        heer.reset();
        heer.spawn_wave();
        heer.enabled = false;
        let mut minimum = heer.leben;
        for _ in 0..3000 {
            heer.tick(0.1, &[]);
            minimum = minimum.min(heer.leben);
            if heer.count() == 0 {
                break;
            }
        }
        assert!(minimum < MAX_LEBEN, "Durchbruch kostet keine Leben");
        assert!(heer.meldungen.iter().any(|m| m.contains("durchgebrochen")));

        // Niederlage: bei 0 Leben geht es nach einer Pause wieder bei Welle 1 los
        heer.leben = 1;
        heer.spawn_wave();
        heer.enabled = false;
        for _ in 0..3000 {
            heer.tick(0.1, &[]);
            if heer.welle == 0 {
                break;
            }
        }
        assert_eq!((heer.welle, heer.leben, heer.count()), (0, MAX_LEBEN, 0), "Niederlage setzt nicht zurück");
        assert!(heer.meldungen.iter().any(|m| m.contains("gefallen")));
        assert!(heer.naechste_in() > 30.0);
    }

    #[test]
    fn truppen_bleiben_bei_spielern_stehen_und_greifen_an() {
        let mut ctx = Context::headless();
        let mut world = crate::world::World::new(&mut ctx);
        world.heer.spawn_wave();
        // Ein Spieler steht auf der Südstraße ein Stück vor der Rampe
        let (spieler, _) = world.heer.routes[0].sample(120.0);
        let mut angriffe = 0;
        for _ in 0..1200 {
            angriffe += world.heer.tick(0.1, &[spieler]).len();
        }
        assert!(angriffe > 0, "niemand greift an");
        let nah = world.heer.states().iter().filter(|s| s.position.distance(spieler) < 30.0).count();
        assert!(nah > 0, "die Gruppe ist am Spieler vorbeigelaufen");
    }
}
