//! Arbeiter der Rohstoffgebäude: Holzfäller, Steinbruch, Erzmine, Lehmgrube und Kristallturm
//! schicken je drei Arbeiter los. Die Kristallmagier des Kristallturms schweben: Sie gleiten
//! schneller, suchen weiter und lösen die Kristalle mit einem Zauberstrahl.
//!
//! Der Server steuert sie: Sie gehen aus der Tür zum nächsten freien Baum bzw. Vorkommen, bauen
//! es ab (jeder Schlag trifft den Rohstoff wie der Schlag eines Spielers), tragen die Ladung
//! zurück und liefern sie dem Erbauer ab – dann geht es von vorn los. Ist in der Nähe nichts zu
//! holen, arbeiten sie am Gebäude selbst (Hackklotz, Felswand, Stollen), bringen dann aber weniger.
//!
//! Clients bekommen in jedem Schnappschuss Ort, Blick und Tätigkeit und zeigen die Modelle aus
//! `game/assets/npc/` (Blender: `art/lib/arbeiter.py`) mit den Clips Idle, Laufen, Arbeiten,
//! Buecken und Tragen.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use engine::prelude::*;
use serde::{Deserialize, Serialize};

use crate::asset_files;
use crate::bauten::{Building, BuildingKind};
use crate::island::ResourceKind;
use crate::world::Resource;

/// So viele Arbeiter schickt jedes Rohstoffgebäude los.
pub const JE_GEBAEUDE: u32 = 3;
/// So viel bringt ein Arbeiter von einem ganzen Baum bzw. Vorkommen zurück.
pub const LADUNG: u32 = 5;
/// Ohne Rohstoff in der Nähe: so viel je Gang und so lange (Sekunden) dauert die Arbeit am Gebäude.
pub const LADUNG_AM_HAUS: u32 = 2;
const ARBEIT_AM_HAUS: f32 = 14.0;
/// So weit (Meter, ab der Tür) suchen die Arbeiter nach Bäumen und Vorkommen.
pub const SUCHWEITE: f32 = 80.0;
/// Kristallvorkommen sind selten: die Magier suchen viel weiter.
pub const SUCHWEITE_KRISTALL: f32 = 220.0;
/// Gehen und Tragen (m/s); die Magier gleiten schneller
const TEMPO: f32 = 1.55;
const TEMPO_TRAGEN: f32 = 1.3;
const TEMPO_MAGIER: f32 = 2.6;
const TEMPO_MAGIER_TRAGEN: f32 = 2.2;

fn suchweite(art: ResourceKind) -> f32 {
    if art == ResourceKind::Kristall { SUCHWEITE_KRISTALL } else { SUCHWEITE }
}

fn tempo(art: ResourceKind, tragen: bool) -> f32 {
    match (art == ResourceKind::Kristall, tragen) {
        (true, false) => TEMPO_MAGIER,
        (true, true) => TEMPO_MAGIER_TRAGEN,
        (false, false) => TEMPO,
        (false, true) => TEMPO_TRAGEN,
    }
}
/// Aufheben der Ladung am Rohstoff und Abladen an der Tür (Sekunden)
const BUECKEN: f32 = 1.3;
/// Pause an der Tür nach dem Abliefern, und Abstand, in dem die drei zu Beginn losgehen
const PAUSE: f32 = 1.5;
const AUFBRUCH: f32 = 2.5;
/// Absender der Rohstoff-Treffer, die von Arbeitern stammen (kein Spieler)
pub const ARBEITER_ABSENDER: crate::protocol::PlayerId = u64::MAX;
/// Bis zu dieser Entfernung zur Kamera werden die Animationen gerechnet, bis hierher gezeichnet.
const ANIMATION_DISTANCE: f32 = 90.0;
const SICHTWEITE: f32 = 220.0;

/// Was ein Arbeiter gerade tut (bestimmt die Animation und was er trägt).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tun {
    #[default]
    Warten,
    Gehen,
    Arbeiten,
    /// Ladung aufheben (am Rohstoff)
    Aufheben,
    Tragen,
    /// Ladung an der Tür abladen
    Abladen,
}

/// Lage eines Arbeiters im Schnappschuss.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArbeiterState {
    pub id: u32,
    pub art: ResourceKind,
    pub position: Vec3,
    /// Blickrichtung als Winkel um die Hochachse (0 = +Z)
    pub blick: f32,
    pub tun: Tun,
}

/// Wie lange ein Schlag dauert (eine Runde der Animation „Arbeiten“) und wann in der Runde die
/// Meldung an den Rohstoff geht – so, dass die Splitter beim Client mit dem Auftreffen kommen
/// (die Clients zeigen Treffer um `characters::CHOP_STRIKE`/`MINE_STRIKE` verzögert).
pub fn schlag_takt(art: ResourceKind) -> (f32, f32) {
    match art {
        // Axt: 36 Bilder, Einschlag bei Bild 22
        ResourceKind::Wood => (36.0 / 30.0, 22.0 / 30.0 - crate::characters::CHOP_STRIKE),
        // Spitzhacke: 42 Bilder, Einschlag bei Bild 27
        ResourceKind::Stone | ResourceKind::Ore => (42.0 / 30.0, 27.0 / 30.0 - crate::characters::MINE_STRIKE),
        // Spaten: 42 Bilder, Stich bei Bild 14
        ResourceKind::Lehm => (42.0 / 30.0, 14.0 / 30.0 - crate::characters::MINE_STRIKE),
        // Zauber: 48 Bilder, Stoß bei Bild 30
        ResourceKind::Kristall => (48.0 / 30.0, 30.0 / 30.0 - crate::characters::MINE_STRIKE),
    }
}

/// Dateiname des Modells in `game/assets/npc/`.
pub fn modell(art: ResourceKind) -> &'static str {
    match art {
        ResourceKind::Wood => "holzarbeiter",
        ResourceKind::Stone => "steinarbeiter",
        ResourceKind::Ore => "erzarbeiter",
        ResourceKind::Lehm => "lehmarbeiter",
        ResourceKind::Kristall => "kristallmagier",
    }
}

pub fn label(art: ResourceKind) -> &'static str {
    match art {
        ResourceKind::Wood => "Holzarbeiter",
        ResourceKind::Stone => "Steinarbeiter",
        ResourceKind::Ore => "Erzarbeiter",
        ResourceKind::Lehm => "Lehmstecher",
        ResourceKind::Kristall => "Kristallmagier",
    }
}

/// Welchen Rohstoff das Gebäude abbauen lässt.
pub fn rohstoff(kind: BuildingKind) -> Option<ResourceKind> {
    match kind {
        BuildingKind::Lumberjack => Some(ResourceKind::Wood),
        BuildingKind::Quarry => Some(ResourceKind::Stone),
        BuildingKind::Mine => Some(ResourceKind::Ore),
        BuildingKind::Lehmgrube => Some(ResourceKind::Lehm),
        BuildingKind::Kristallturm => Some(ResourceKind::Kristall),
        _ => None,
    }
}

/// Vor der Tür (Welt, am Boden): Holzfäller und Steinbruch auf dem Hof vor dem Haus, bei der
/// Erzmine direkt im Stollenportal. Die Modelle schauen nach +Z.
fn tuer(b: &Building) -> Vec3 {
    let vorne = match b.kind {
        BuildingKind::Lumberjack => 3.4,
        BuildingKind::Quarry => 2.4,
        BuildingKind::Lehmgrube => 2.6,
        BuildingKind::Kristallturm => 1.6,
        _ => 0.9,
    };
    b.position + b.rotation() * vec3(0.0, 0.0, vorne)
}

/// Arbeitsplatz am Gebäude, falls in der Nähe nichts zu holen ist (je Arbeiter etwas versetzt):
/// Hackklotz neben dem Holzfällerhaus, die eigene Felswand im Steinbruch, der Stollen der Mine.
fn am_haus(b: &Building, k: u32) -> (Vec3, Vec3) {
    let versatz = (k as f32 - 1.0) * 1.3;
    let (stand, ziel) = match b.kind {
        BuildingKind::Lumberjack => (vec3(-4.2 + versatz * 0.3, 0.0, 1.2 + versatz), vec3(-5.2, 0.0, 1.2 + versatz)),
        BuildingKind::Quarry => (vec3(versatz * 2.0, 0.0, -1.2), vec3(versatz * 2.0, 0.0, -2.4)),
        // Am Lehmwall hinter der Grube
        BuildingKind::Lehmgrube => (vec3(versatz * 1.5, 0.0, -2.3), vec3(versatz * 1.5, 0.0, -3.5)),
        // Um den Kristallsockel neben dem Turm
        BuildingKind::Kristallturm => {
            let w = (k as f32 - 1.0) * 1.2;
            let sockel = vec3(3.3, 0.0, 1.2);
            (sockel + vec3(-w.cos(), 0.0, w.sin()) * 1.9, sockel)
        }
        _ => (vec3(versatz * 0.5, 0.0, -0.8), vec3(versatz * 0.5, 0.0, -2.5)),
    };
    (b.position + b.rotation() * stand, b.position + b.rotation() * ziel)
}

struct Arbeiter {
    id: u32,
    gebaeude: u32,
    nummer: u32,
    art: ResourceKind,
    position: Vec3,
    blick: f32,
    tun: Tun,
    /// Wegpunkte (nur x, z), der letzte ist das Ziel
    weg: Vec<Vec2>,
    /// Baum bzw. Vorkommen, an dem er arbeitet (`None`: am Gebäude)
    ziel: Option<u32>,
    /// Restzeit der aktuellen Tätigkeit bzw. bis zum nächsten Schlag
    zeit: f32,
    /// Arbeitszeit am Gebäude
    arbeit: f32,
    /// Schläge am aktuellen Rohstoff und wie viele er zu Beginn aushielt
    schlaege: u32,
    gesamt: u32,
    /// Was er trägt
    ladung: u32,
}

/// Was die Arbeiter in einem Takt bewirkt haben (der Server setzt es um).
#[derive(Clone, Debug, PartialEq)]
pub enum Ereignis {
    /// Ein Schlag trifft den Rohstoff (ein Leben weniger)
    Schlag(u32),
    /// Ladung an der Tür abgeliefert: an den Erbauer des Gebäudes
    Lieferung { gebaeude: u32, menge: u32 },
}

/// Alle Arbeiter (nur beim Server).
#[derive(Default)]
pub struct Arbeiterschaft {
    arbeiter: Vec<Arbeiter>,
}

impl Arbeiterschaft {
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.arbeiter.len()
    }

    /// Ein Server-Takt: Arbeiter zu neuen Gebäuden schicken, verschwundene abziehen, alle bewegen.
    pub fn takt(&mut self, dt: f32, buildings: &[Building], resources: &BTreeMap<u32, Resource>, terrain: &Terrain) -> Vec<Ereignis> {
        // Wer zu keinem fertigen Rohstoffgebäude mehr gehört, geht
        self.arbeiter.retain(|a| buildings.iter().any(|b| b.id == a.gebaeude && b.finished() && rohstoff(b.kind) == Some(a.art)));
        for b in buildings.iter().filter(|b| b.finished()) {
            let Some(art) = rohstoff(b.kind) else { continue };
            for nummer in 0..JE_GEBAEUDE {
                let id = b.id * 4 + nummer;
                if self.arbeiter.iter().any(|a| a.id == id) {
                    continue;
                }
                let start = tuer(b);
                let position = vec3(start.x, terrain.height_at(start.x, start.z), start.z);
                self.arbeiter.push(Arbeiter {
                    id,
                    gebaeude: b.id,
                    nummer,
                    art,
                    position,
                    blick: b.yaw,
                    tun: Tun::Warten,
                    weg: Vec::new(),
                    ziel: None,
                    zeit: 0.5 + nummer as f32 * AUFBRUCH,
                    arbeit: 0.0,
                    schlaege: 0,
                    gesamt: 1,
                    ladung: 0,
                });
            }
        }

        let mut ereignisse = Vec::new();
        let mut belegt: HashSet<u32> = self.arbeiter.iter().filter_map(|a| a.ziel).collect();
        for a in &mut self.arbeiter {
            let Some(b) = buildings.iter().find(|b| b.id == a.gebaeude) else { continue };
            a.zeit -= dt;
            let art = a.art;
            match a.tun {
                Tun::Warten => {
                    if a.zeit <= 0.0 {
                        a.losschicken(b, resources, &mut belegt);
                    }
                }
                Tun::Gehen | Tun::Tragen => {
                    let tempo = tempo(art, a.tun == Tun::Tragen);
                    if a.gehen(dt * tempo, terrain) {
                        if a.tun == Tun::Tragen {
                            a.tun = Tun::Abladen;
                            a.zeit = BUECKEN;
                            a.blick = (b.position - a.position).x.atan2((b.position - a.position).z);
                        } else {
                            // Angekommen – ist der Rohstoff noch da?
                            match a.ziel {
                                Some(id) if resources.get(&id).is_some_and(|r| r.is_present()) => {
                                    let r = &resources[&id];
                                    let mitte = r.spec.transform.position;
                                    a.blick = (mitte - a.position).x.atan2((mitte - a.position).z);
                                    a.tun = Tun::Arbeiten;
                                    a.zeit = schlag_takt(a.art).1.max(0.05);
                                    a.schlaege = 0;
                                    a.gesamt = r.health.max(1) as u32;
                                }
                                Some(_) => {
                                    belegt.remove(&a.ziel.take().unwrap_or_default());
                                    a.losschicken(b, resources, &mut belegt);
                                }
                                None => {
                                    a.tun = Tun::Arbeiten;
                                    a.arbeit = ARBEIT_AM_HAUS;
                                    a.zeit = schlag_takt(a.art).1.max(0.05);
                                    let (_, ziel) = am_haus(b, a.nummer);
                                    a.blick = (ziel - a.position).x.atan2((ziel - a.position).z);
                                }
                            }
                        }
                    }
                }
                Tun::Arbeiten => match a.ziel {
                    Some(id) => {
                        let Some(r) = resources.get(&id).filter(|r| r.is_present()) else {
                            // Jemand anderes war schneller: mitnehmen, was schon abgeschlagen ist
                            belegt.remove(&id);
                            a.ziel = None;
                            if a.schlaege > 0 {
                                a.aufheben((LADUNG * a.schlaege).div_ceil(a.gesamt).max(1));
                            } else {
                                a.losschicken(b, resources, &mut belegt);
                            }
                            continue;
                        };
                        if a.zeit <= 0.0 {
                            a.zeit += schlag_takt(a.art).0;
                            a.schlaege += 1;
                            ereignisse.push(Ereignis::Schlag(id));
                            if r.health <= 1 {
                                // Der letzte Schlag: der Baum fällt bzw. das Vorkommen zerfällt
                                belegt.remove(&id);
                                a.ziel = None;
                                a.aufheben(LADUNG);
                            }
                        }
                    }
                    None => {
                        a.arbeit -= dt;
                        if a.zeit <= 0.0 {
                            a.zeit += schlag_takt(a.art).0;
                        }
                        if a.arbeit <= 0.0 {
                            a.aufheben(LADUNG_AM_HAUS);
                        }
                    }
                },
                Tun::Aufheben => {
                    if a.zeit <= 0.0 {
                        a.tun = Tun::Tragen;
                        a.weg = weg(b, a.position, tuer(b));
                    }
                }
                Tun::Abladen => {
                    if a.zeit <= 0.0 {
                        ereignisse.push(Ereignis::Lieferung { gebaeude: a.gebaeude, menge: a.ladung });
                        a.ladung = 0;
                        a.tun = Tun::Warten;
                        a.zeit = PAUSE;
                    }
                }
            }
        }
        ereignisse
    }

    pub fn states(&self) -> Vec<ArbeiterState> {
        self.arbeiter.iter().map(|a| ArbeiterState { id: a.id, art: a.art, position: a.position, blick: a.blick, tun: a.tun }).collect()
    }
}

impl Arbeiter {
    /// Zum nächsten freien Rohstoff in Reichweite schicken, sonst zum Arbeitsplatz am Gebäude.
    fn losschicken(&mut self, b: &Building, resources: &BTreeMap<u32, Resource>, belegt: &mut HashSet<u32>) {
        let von = tuer(b);
        let naechster = resources
            .iter()
            .filter(|(id, r)| r.spec.kind == self.art && r.is_present() && !belegt.contains(id))
            .map(|(&id, r)| (id, r, flach(r.spec.transform.position).distance(flach(von))))
            .filter(|&(_, _, d)| d <= suchweite(self.art))
            .min_by(|a, b| a.2.total_cmp(&b.2));
        match naechster {
            Some((id, r, _)) => {
                let mitte = flach(r.spec.transform.position);
                // Vom Weg her an den Rohstoff treten; Abstand so, dass Axt bzw. Hacke gerade trifft
                // (Treffpunkt laut art/lib/arbeiter.py: Axt 1,24 m, Hacke und Hammer 1,18 m vor den Füßen)
                let richtung = (flach(self.position) - mitte).normalize_or(Vec2::X);
                let abstand = r.radius()
                    + match self.art {
                        ResourceKind::Wood => 1.1,
                        ResourceKind::Lehm => 0.45,
                        // Der Magier zaubert aus etwas Abstand
                        ResourceKind::Kristall => 1.7,
                        _ => 0.9,
                    };
                self.ziel = Some(id);
                belegt.insert(id);
                self.weg = weg(b, self.position, { let p = mitte + richtung * abstand; vec3(p.x, 0.0, p.y) });
            }
            None => {
                self.ziel = None;
                self.weg = weg(b, self.position, am_haus(b, self.nummer).0);
            }
        }
        self.tun = Tun::Gehen;
    }

    fn aufheben(&mut self, menge: u32) {
        self.ladung = menge;
        self.tun = Tun::Aufheben;
        self.zeit = BUECKEN;
    }

    /// Ein Stück den Wegpunkten nach; `true`, wenn er am Ziel ist.
    fn gehen(&mut self, mut strecke: f32, terrain: &Terrain) -> bool {
        while strecke > 0.0 {
            let Some(&naechster) = self.weg.first() else { return true };
            let hier = flach(self.position);
            let rest = naechster - hier;
            let d = rest.length();
            let neu = if d <= strecke {
                self.weg.remove(0);
                strecke -= d;
                naechster
            } else {
                let p = hier + rest / d * strecke;
                strecke = 0.0;
                p
            };
            if d > 0.01 {
                self.blick = rest.x.atan2(rest.y);
            }
            self.position = vec3(neu.x, terrain.height_at(neu.x, neu.y), neu.y);
        }
        self.weg.is_empty()
    }
}

fn flach(p: Vec3) -> Vec2 {
    vec2(p.x, p.z)
}

/// Kern des Gebäudes, um den die Arbeiter herumgehen: Mitte und Radius (Welt, x/z). Beim
/// Steinbruch und bei der Mine ist es die Felswand bzw. der Hügel hinter dem Platz.
fn kern(b: &Building) -> (Vec2, f32) {
    let (hinten, radius) = match b.kind {
        BuildingKind::Lumberjack => (0.0, 3.3),
        BuildingKind::Quarry => (4.2, 3.6),
        BuildingKind::Lehmgrube => (3.6, 3.4),
        BuildingKind::Kristallturm => (1.6, 2.7),
        _ => (3.6, 3.2),
    };
    (flach(b.position + b.rotation() * vec3(0.0, 0.0, -hinten)), radius)
}

/// Kreuzt die Strecke a–b den Kreis?
fn kreuzt(a: Vec2, b: Vec2, mitte: Vec2, radius: f32) -> bool {
    let ab = b - a;
    let t = ((mitte - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    (a + ab * t).distance(mitte) < radius
}

/// Weg von `von` nach `nach`: Liegt das Gebäude dazwischen, geht es an der Seite vorbei (über
/// eine vordere und eine hintere Ecke), nicht quer durch die Wände.
fn weg(b: &Building, von: Vec3, nach: Vec3) -> Vec<Vec2> {
    let (mitte, r) = kern(b);
    let vorne = flach(b.rotation() * Vec3::Z);
    let seite = vec2(vorne.y, -vorne.x);
    let (a, z) = (flach(von), flach(nach));
    // Arbeitsplätze am Gebäude selbst (Felswand, Stollen) liegen im Kern: direkt hin
    if z.distance(mitte) < r || a.distance(mitte) < r || !kreuzt(a, z, mitte, r) {
        return vec![z];
    }
    let s = if (z - mitte).dot(seite) + (a - mitte).dot(seite) >= 0.0 { 1.0 } else { -1.0 };
    let vorn_ecke = mitte + seite * s * (r + 1.0) + vorne * r * 0.8;
    let hinten_ecke = mitte + seite * s * (r + 1.0) - vorne * r * 0.8;
    let (erst, dann) = if a.distance(vorn_ecke) <= a.distance(hinten_ecke) { (vorn_ecke, hinten_ecke) } else { (hinten_ecke, vorn_ecke) };
    let mut punkte = vec![erst];
    if kreuzt(erst, z, mitte, r) {
        punkte.push(dann);
    }
    punkte.push(z);
    punkte
}

// ---------------------------------------------------------------------------
// Ansicht (nur mit Fenster)
// ---------------------------------------------------------------------------

struct Figur {
    entity: EntityId,
    animator: Animator,
    art: ResourceKind,
    shown: Vec3,
    blick: f32,
    tun: Tun,
    /// Zeit bis zum nächsten Funken des Zauberstrahls (Kristallmagier)
    funken: f32,
}

/// Was Clients von den Arbeitern sehen: Modelle anlegen, nachführen, animieren, entfernen.
#[derive(Default)]
pub struct ArbeiterAnsicht {
    modelle: HashMap<&'static str, Option<(Arc<Model>, MeshId)>>,
    figuren: HashMap<u32, Figur>,
}

impl ArbeiterAnsicht {
    fn modell(&mut self, ctx: &mut Context, datei: &'static str) -> Option<(Arc<Model>, MeshId)> {
        self.modelle
            .entry(datei)
            .or_insert_with(|| {
                let path = asset_files::variants("npc", datei).into_iter().next()?;
                let model = match Model::from_file(&path) {
                    Ok(model) => Arc::new(model),
                    Err(message) => {
                        log::warn!("{datei}: {message}");
                        return None;
                    }
                };
                let textures = model.register_textures(&mut ctx.assets);
                let texture = textures.first().copied();
                // Alle Arbeiter einer Art teilen ein Mesh; die Grafikkarte verformt es je Figur.
                let mesh = ctx.assets.named_mesh(&format!("npc_gpu_{datei}"), || model.skinned_gpu_mesh(texture));
                Some((model, mesh))
            })
            .clone()
    }

    pub fn update(&mut self, ctx: &mut Context, states: &[ArbeiterState]) {
        let dt = ctx.time.delta;
        let weg: Vec<u32> = self.figuren.keys().copied().filter(|id| !states.iter().any(|s| s.id == *id)).collect();
        for id in weg {
            if let Some(figur) = self.figuren.remove(&id) {
                ctx.scene.despawn(figur.entity);
            }
        }
        for state in states {
            if !self.figuren.contains_key(&state.id) {
                let Some((model, mesh)) = self.modell(ctx, modell(state.art)) else { continue };
                let mut animator = Animator::new(model);
                animator.play("Idle", true, 0.0);
                let mut entity = Entity::new(label(state.art), mesh)
                    .with_transform(Transform::from_position(state.position))
                    .with_material(Material::Figur { rim: 0.35 });
                entity.joints = animator.palette();
                let entity = ctx.scene.spawn(entity);
                self.figuren.insert(state.id, Figur { entity, animator, art: state.art, shown: state.position, blick: state.blick, tun: Tun::Warten, funken: 0.0 });
            }
            let Some(figur) = self.figuren.get_mut(&state.id) else { continue };
            // Weich nachführen (Schnappschüsse kommen nur alle paar Bilder)
            figur.shown = figur.shown.lerp(state.position, (dt * 12.0).min(1.0));
            if figur.shown.distance(state.position) > 4.0 {
                figur.shown = state.position;
            }
            let dreh = (state.blick - figur.blick + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            figur.blick += dreh * (dt * 9.0).min(1.0);
            let abstand = figur.shown.distance(ctx.camera.position);
            let sichtbar = abstand < SICHTWEITE;
            if sichtbar && abstand < ANIMATION_DISTANCE {
                if state.tun != figur.tun {
                    figur.tun = state.tun;
                    let (clip, schleife, blende) = match state.tun {
                        Tun::Warten => ("Idle", true, 0.3),
                        Tun::Gehen => ("Laufen", true, 0.25),
                        Tun::Tragen => ("Tragen", true, 0.3),
                        Tun::Arbeiten => ("Arbeiten", true, 0.15),
                        Tun::Aufheben | Tun::Abladen => ("Buecken", false, 0.2),
                    };
                    figur.animator.play(clip, schleife, blende);
                    // Beim Tragen hängt das Werkzeug auf dem Rücken, die Ladung ist zu sehen
                    let traegt = matches!(state.tun, Tun::Tragen | Tun::Abladen);
                    figur.animator.set_visible("Last", traegt);
                    figur.animator.set_visible("Werkzeug", !traegt);
                    figur.animator.set_visible("Werkzeug_Ruecken", traegt);
                }
                figur.animator.set_speed(match (figur.tun, figur.art) {
                    // Die Magier gleiten: kein Schritt, der zum Tempo passen muss
                    (_, ResourceKind::Kristall) => 1.0,
                    // Schrittweite der Clips: Laufen 1,75 m/s, Tragen 1,06 m/s bei Tempo 1
                    (Tun::Gehen, _) => TEMPO / 1.75,
                    (Tun::Tragen, _) => TEMPO_TRAGEN / 1.06,
                    _ => 1.0,
                });
                figur.animator.update(dt);
                if figur.art == ResourceKind::Kristall {
                    zauber_funken(ctx, figur, dt);
                }
            }
            let Some(entity) = ctx.scene.try_get_mut(figur.entity) else { continue };
            entity.visible = sichtbar;
            if sichtbar {
                if abstand < ANIMATION_DISTANCE {
                    entity.joints = figur.animator.palette();
                }
                entity.transform.position = figur.shown;
                entity.transform.rotation = Quat::from_rotation_y(figur.blick);
            }
            let _ = figur.art;
        }
    }
}

/// Funken der Kristallmagier: beim Arbeiten ein Strahl vom Stab zum Vorkommen, beim Tragen
/// glitzern die schwebenden Kristalle, beim Gleiten eine feine Spur.
fn zauber_funken(ctx: &mut Context, figur: &mut Figur, dt: f32) {
    figur.funken -= dt;
    if figur.funken > 0.0 {
        return;
    }
    let vorne = vec3(figur.blick.sin(), 0.0, figur.blick.cos());
    let blau = vec3(0.35, 0.75, 1.0);
    let funke = |position: Vec3, richtung: Vec3, speed: f32, size: f32, life: f32, gravity: f32| Burst {
        position,
        count: 1,
        color: blau,
        color_variation: 0.25,
        speed,
        direction: richtung,
        size,
        life,
        gravity,
        glow: 3.0,
        grow: 0.0,
        round: true,
    };
    match figur.tun {
        Tun::Arbeiten => {
            figur.funken = 0.05;
            let stab = figur.shown + vorne * 0.9 + Vec3::Y * 2.1;
            let ziel = figur.shown + vorne * 2.3 + Vec3::Y * 0.7;
            let lauf = (ctx.time.elapsed * 3.0).fract();
            for k in 0..4 {
                let t = (k as f32 + lauf) / 4.0;
                ctx.particles.burst(funke(stab.lerp(ziel, t), (ziel - stab).normalize_or_zero() * 0.5, 0.4, 0.09, 0.35, 0.0));
            }
            ctx.particles.burst(Burst { count: 2, ..funke(ziel, Vec3::Y * 0.5, 1.6, 0.07, 0.6, 1.0) });
        }
        Tun::Tragen | Tun::Gehen => {
            let tragen = figur.tun == Tun::Tragen;
            figur.funken = if tragen { 0.12 } else { 0.2 };
            let at = if tragen { figur.shown + vorne * 0.34 + Vec3::Y * 1.3 } else { figur.shown + Vec3::Y * 0.2 };
            ctx.particles.burst(funke(at, Vec3::Y * 0.2, 0.3, 0.06, 0.8, -0.2));
        }
        _ => figur.funken = 0.3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn haus(kind: BuildingKind) -> Building {
        Building {
            id: 7,
            kind,
            position: vec3(0.0, 0.0, 0.0),
            yaw: 0.0,
            progress: 1.0,
            owner: "nils".into(),
            produce_in: 0.0,
            level: 1,
            zweig: 0,
            ziel: Default::default(),
        }
    }

    #[test]
    fn lehmgrube_und_kristallturm_schicken_ihre_arbeiter() {
        let terrain = Terrain::generate(Vec2::ZERO, 200.0, 50, |_| 5.0);
        let mut lehm = haus(BuildingKind::Lehmgrube);
        lehm.id = 3;
        let mut turm = haus(BuildingKind::Kristallturm);
        turm.id = 5;
        turm.position = vec3(60.0, 0.0, 0.0);
        let buildings = vec![lehm, turm];
        let mut schaft = Arbeiterschaft::default();
        let mut geliefert = 0;
        for _ in 0..(60 * 90) {
            for e in schaft.takt(1.0 / 60.0, &buildings, &BTreeMap::new(), &terrain) {
                if let Ereignis::Lieferung { menge, .. } = e {
                    geliefert += menge;
                }
            }
        }
        let arten: Vec<ResourceKind> = schaft.states().iter().map(|s| s.art).collect();
        assert_eq!(arten.iter().filter(|&&a| a == ResourceKind::Lehm).count(), JE_GEBAEUDE as usize);
        assert_eq!(arten.iter().filter(|&&a| a == ResourceKind::Kristall).count(), JE_GEBAEUDE as usize);
        assert!(geliefert > 0, "nichts geliefert");
        // Die Magier gleiten schneller und suchen weiter als die übrigen Arbeiter
        assert!(tempo(ResourceKind::Kristall, false) > tempo(ResourceKind::Lehm, false));
        assert!(suchweite(ResourceKind::Kristall) > suchweite(ResourceKind::Lehm));
        // Kristalle baut kein Spieler von Hand ab
        assert!(!ResourceKind::Kristall.von_hand() && ResourceKind::Lehm.von_hand());
    }

    #[test]
    fn weg_fuehrt_um_das_haus_herum() {
        let b = haus(BuildingKind::Lumberjack);
        // Ziel hinter dem Haus: erst zur Seite, dann nach hinten
        let punkte = weg(&b, tuer(&b), vec3(0.0, 0.0, -25.0));
        assert!(punkte.len() >= 2, "{punkte:?}");
        assert!(punkte[0].x.abs() > kern(&b).1, "{punkte:?}");
        // Ziel vorne: direkt hin
        assert_eq!(weg(&b, tuer(&b), vec3(3.0, 0.0, 20.0)).len(), 1);
    }

    #[test]
    fn am_haus_ohne_rohstoffe() {
        let terrain = Terrain::generate(Vec2::ZERO, 200.0, 50, |_| 5.0);
        let buildings = vec![haus(BuildingKind::Quarry)];
        let mut schaft = Arbeiterschaft::default();
        let mut geliefert = 0;
        for _ in 0..(60 * 90) {
            for e in schaft.takt(1.0 / 60.0, &buildings, &BTreeMap::new(), &terrain) {
                if let Ereignis::Lieferung { menge, .. } = e {
                    geliefert += menge;
                }
            }
        }
        assert_eq!(schaft.len(), JE_GEBAEUDE as usize);
        assert!(geliefert >= LADUNG_AM_HAUS * JE_GEBAEUDE, "nur {geliefert} geliefert");
        // Gebäude weg: Arbeiter weg
        schaft.takt(1.0 / 60.0, &[], &BTreeMap::new(), &terrain);
        assert_eq!(schaft.len(), 0);
    }
}
