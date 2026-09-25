//! Tiere auf der Insel: Hasen, Füchse, Hirsche und Bären.
//!
//! Der Server steuert sie (grasen, umherstreifen, vor Spielern fliehen) und schickt ihre
//! Lage in jedem Snapshot mit. Clients zeigen sie nur an.
//!
//! Aussehen: Liegt `game/assets/tiere/<name>.gltf` (aus Blender, mit Animationen `Idle`,
//! `Laufen`, `Rennen`), wird das Modell benutzt. Sonst ein einfacher Platzhalter aus
//! Grundformen, der beim Laufen hüpft.

use std::sync::Arc;

use engine::noise::Rng;
use engine::prelude::*;
use serde::{Deserialize, Serialize};

use crate::asset_files;
use crate::protocol::Item;

/// Um diesen Radius (Meter) zieht ein Tier beim Umherstreifen um sein Revier.
const WANDER_RADIUS: f32 = 16.0;
/// Weiter entfernt von der Kamera werden Animationen nicht mehr berechnet.
const ANIMATION_DISTANCE: f32 = 70.0;
/// Tiefer als das gilt als Wasser oder Strand – da gehen Tiere nicht hin.
const MIN_GROUND: f32 = 1.4;
/// So lange (Sekunden) bleibt ein erlegtes Tier weg, dann kommt ein neues in sein Revier.
const RESPAWN_SECONDS: f32 = 90.0;
/// Nach einem Treffer flieht ein Tier mindestens so lange (Sekunden), egal wie weit der Angreifer ist.
const PANIC_SECONDS: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnimalKind {
    Hare,
    Fox,
    /// Reh (früher Hirsch – der Name im Code bleibt, damit alte Spielstände passen)
    Deer,
    Bear,
    Sheep,
    Wolf,
}

/// Wie sich eine Tierart verhält.
struct Traits {
    walk: f32,
    run: f32,
    /// Näher als das kommt ein Spieler nicht, ohne dass das Tier flieht.
    flee: f32,
    /// So weit flieht es, bevor es sich beruhigt.
    calm: f32,
    /// Größe des Platzhalters (Schulterhöhe in Metern).
    height: f32,
    /// Abspieltempo der Lauf- und Renn-Animation, damit die Füße nicht rutschen.
    stride: (f32, f32),
}

impl AnimalKind {

    fn traits(self) -> Traits {
        match self {
            AnimalKind::Hare => Traits { walk: 1.6, run: 8.5, flee: 7.0, calm: 20.0, height: 0.35, stride: (1.0, 1.0) },
            // stride = Tempo / Fußtempo der Animation (art/lib/tiere.py gibt es beim Bauen aus)
            AnimalKind::Fox => Traits { walk: 1.5, run: 7.5, flee: 10.0, calm: 26.0, height: 0.45, stride: (1.7, 2.6) },
            AnimalKind::Deer => Traits { walk: 1.5, run: 10.0, flee: 14.0, calm: 32.0, height: 0.8, stride: (1.33, 2.06) },
            // Bären sind gemächlich und lassen Spieler nah heran, bevor sie davontrotten.
            AnimalKind::Bear => Traits { walk: 1.4, run: 5.0, flee: 5.0, calm: 18.0, height: 1.2, stride: (1.05, 1.26) },
            // Schafe sind zutraulich und rennen nur kurz weg.
            AnimalKind::Sheep => Traits { walk: 1.0, run: 4.5, flee: 4.0, calm: 12.0, height: 0.7, stride: (1.15, 1.6) },
            // Wölfe sind scheu und halten Abstand.
            AnimalKind::Wolf => Traits { walk: 1.5, run: 9.0, flee: 9.0, calm: 24.0, height: 0.8, stride: (1.36, 2.24) },
        }
    }

    /// Platz-Kennung für Markierungen in der Asset-Galerie.
    pub fn slot(self) -> &'static str {
        match self {
            AnimalKind::Hare => "hase",
            AnimalKind::Fox => "fuchs",
            AnimalKind::Deer => "reh",
            AnimalKind::Bear => "baer",
            AnimalKind::Sheep => "schaf",
            AnimalKind::Wolf => "wolf",
        }
    }

    /// Dateiname in `game/assets/tiere/`.
    pub fn file_name(self) -> &'static str {
        match self {
            AnimalKind::Hare => "hase",
            AnimalKind::Fox => "fuchs",
            AnimalKind::Deer => "reh",
            AnimalKind::Bear => "baer",
            AnimalKind::Sheep => "schaf",
            AnimalKind::Wolf => "wolf",
        }
    }

    /// Wie viele Treffer das Tier aushält.
    pub fn max_health(self) -> u8 {
        match self {
            AnimalKind::Hare => 2,
            AnimalKind::Fox => 3,
            AnimalKind::Deer => 4,
            AnimalKind::Sheep => 3,
            AnimalKind::Wolf => 5,
            AnimalKind::Bear => 10,
        }
    }

    /// Was ein erlegtes Tier einbringt.
    pub fn loot(self) -> &'static [(Item, u32)] {
        match self {
            AnimalKind::Hare => &[(Item::Meat, 1), (Item::Pelt, 1)],
            AnimalKind::Fox => &[(Item::Meat, 1), (Item::Pelt, 2)],
            AnimalKind::Deer => &[(Item::Meat, 3), (Item::Pelt, 2)],
            AnimalKind::Sheep => &[(Item::Meat, 2), (Item::Wool, 3)],
            AnimalKind::Wolf => &[(Item::Meat, 2), (Item::Pelt, 2)],
            AnimalKind::Bear => &[(Item::Meat, 5), (Item::Pelt, 3)],
        }
    }

    /// Trefferkugel relativ zu den Füßen: (Höhe des Mittelpunkts, Radius).
    pub fn hit_sphere(self) -> (f32, f32) {
        let h = self.traits().height;
        (h * 0.65, (h * 0.8).max(0.4))
    }

    pub fn label(self) -> &'static str {
        match self {
            AnimalKind::Hare => "Hase",
            AnimalKind::Fox => "Fuchs",
            AnimalKind::Deer => "Reh",
            AnimalKind::Bear => "Bär",
            AnimalKind::Sheep => "Schaf",
            AnimalKind::Wolf => "Wolf",
        }
    }
}

/// Was ein Tier gerade tut (bestimmt auch die Animation).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Gait {
    #[default]
    Idle,
    Walk,
    Run,
}

/// Lage eines Tiers im Snapshot.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimalState {
    pub id: u16,
    pub position: Vec3,
    pub facing: f32,
    pub gait: Gait,
    pub health: u8,
}

/// Was ein Tier vorhat (nur auf dem Server).
#[derive(Clone, Copy, Debug)]
enum Plan {
    Rest { until: f32 },
    Wander { target: Vec2, until: f32 },
    Flee { from: Vec2 },
}

pub struct Animal {
    /// Server: Ziel, zu dem das Tier gerade gern hinzieht (Rehe morgens zum Trinken an den See).
    pub attraction: Option<Vec2>,
    pub kind: AnimalKind,
    pub position: Vec3,
    /// Blickrichtung wie bei Spielern: 0 = nach -Z.
    pub facing: f32,
    pub gait: Gait,
    /// Hat sich seit dem letzten Snapshot bewegt?
    pub moved: bool,
    /// Verbleibende Treffer; 0 = erlegt (unsichtbar, bis es nachkommt).
    pub health: u8,
    /// Server: Uhrzeit (`clock`), zu der ein erlegtes Tier wiederkommt.
    respawn_at: Option<f32>,
    /// Server: bis wann das Tier nach einem Treffer auf jeden Fall flieht.
    panic_until: f32,
    /// Wo das Tier am Anfang stand (dort kommt es nach dem Erlegen wieder).
    origin: Vec3,
    home: Vec2,
    plan: Plan,
    clock: f32,
    rng: Rng,
    visual: Option<Visual>,
}

/// Darstellung (nur mit Fenster).
struct Visual {
    entity: EntityId,
    body: Body,
    /// Dargestellte Position (weich nachgeführt) und Hüpf-Phase des Platzhalters.
    shown: Vec3,
    phase: f32,
    /// Restzeit des roten Aufleuchtens nach einem Treffer (Sekunden).
    flash: f32,
    /// Zeit seit dem Erlegen: das Tier kippt um und versinkt, bevor es verschwindet.
    dying: Option<f32>,
}

enum Body {
    /// Modell aus Blender mit Skelett-Animationen.
    Animated { animator: Animator, mesh: MeshId, texture: Option<TextureId> },
    /// Platzhalter aus Grundformen.
    Placeholder,
}

impl Animal {
    fn new(kind: AnimalKind, position: Vec3, seed: u64) -> Animal {
        let mut rng = Rng::new(seed);
        let facing = rng.range(0.0, std::f32::consts::TAU);
        let until = rng.range(0.0, 4.0);
        Animal {
            attraction: None,
            kind,
            position,
            facing,
            gait: Gait::Idle,
            moved: true,
            health: kind.max_health(),
            respawn_at: None,
            panic_until: 0.0,
            origin: position,
            home: vec2(position.x, position.z),
            plan: Plan::Rest { until },
            clock: 0.0,
            rng,
            visual: None,
        }
    }

    pub fn state(&self, id: u16) -> AnimalState {
        AnimalState { id, position: self.position, facing: self.facing, gait: self.gait, health: self.health }
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0
    }

    /// Übernimmt die Lage vom Server (Clients).
    pub fn apply(&mut self, position: Vec3, facing: f32, gait: Gait, health: u8) {
        self.position = position;
        self.facing = facing;
        self.gait = gait;
        self.set_health(health, false);
    }

    /// Neuer Lebensstand; `effects` = rot aufleuchten lassen (Treffer gerade gesehen).
    pub fn set_health(&mut self, health: u8, effects: bool) {
        if let Some(visual) = &mut self.visual {
            if effects && health < self.health {
                visual.flash = 0.25;
            }
            if health == 0 && self.health > 0 {
                visual.dying = Some(0.0);
            } else if health > 0 && self.health == 0 {
                // Nachgekommen: ohne Umfallen sofort an der neuen Stelle zeigen.
                visual.dying = None;
                visual.shown = self.position;
            }
        }
        self.health = health;
    }

    /// Mittelpunkt und Radius der Trefferkugel.
    pub fn hit_sphere(&self) -> (Vec3, f32) {
        let (height, radius) = self.kind.hit_sphere();
        (self.position + Vec3::Y * height, radius)
    }

    /// Wo das Tier gerade zu sehen ist (weich nachgeführt, für Lebensbalken).
    pub fn shown_position(&self) -> Vec3 {
        self.visual.as_ref().map_or(self.position, |v| v.shown)
    }

    /// Ein Treffer (nur auf dem Server): ein Leben weniger, dann weg vom Angreifer.
    /// Liefert den neuen Lebensstand.
    pub fn hit(&mut self, from: Vec3) -> u8 {
        if !self.is_alive() {
            return 0;
        }
        self.set_health(self.health - 1, true);
        if self.health == 0 {
            self.respawn_at = Some(self.clock + RESPAWN_SECONDS);
            self.gait = Gait::Idle;
        } else {
            self.plan = Plan::Flee { from: vec2(from.x, from.z) };
            self.panic_until = self.clock + PANIC_SECONDS;
        }
        self.moved = true;
        self.health
    }

    /// Ein Takt Verhalten (nur auf dem Server). `players` = Positionen aller Spieler.
    pub fn think(&mut self, dt: f32, players: &[Vec3], terrain: &Terrain) {
        self.clock += dt;
        if let Some(at) = self.respawn_at {
            if self.clock >= at {
                // Ein neues Tier derselben Art kommt dort, wo es am Anfang stand.
                self.respawn_at = None;
                self.position = self.origin;
                self.home = vec2(self.origin.x, self.origin.z);
                self.plan = Plan::Rest { until: self.clock + 2.0 };
                self.set_health(self.kind.max_health(), false);
                self.moved = true;
            }
            return;
        }
        let traits = self.kind.traits();
        let here = vec2(self.position.x, self.position.z);
        let nearest = players.iter().map(|p| vec2(p.x, p.z)).min_by(|a, b| a.distance_squared(here).total_cmp(&b.distance_squared(here)));
        let threat = nearest.filter(|p| p.distance(here) < traits.flee);

        self.plan = match (self.plan, threat) {
            (_, Some(from)) => Plan::Flee { from },
            (Plan::Flee { from }, None) if self.clock < self.panic_until => Plan::Flee { from },
            (Plan::Flee { from }, None) => {
                let from = nearest.unwrap_or(from);
                if from.distance(here) > traits.calm {
                    // In Sicherheit: hier ist das neue Revier.
                    self.home = here;
                    Plan::Rest { until: self.clock + self.rng.range(2.0, 5.0) }
                } else {
                    Plan::Flee { from }
                }
            }
            (Plan::Rest { until }, None) if self.clock > until => match self.pick_target(terrain) {
                Some(target) => Plan::Wander { target, until: self.clock + 12.0 },
                None => Plan::Rest { until: self.clock + 2.0 },
            },
            (Plan::Wander { target, until }, None) if target.distance(here) < 0.6 || self.clock > until => {
                Plan::Rest { until: self.clock + self.rng.range(3.0, 9.0) }
            }
            (plan, None) => plan,
        };

        let (direction, speed, gait) = match self.plan {
            Plan::Rest { .. } => (Vec2::ZERO, 0.0, Gait::Idle),
            Plan::Wander { target, .. } => ((target - here).normalize_or_zero(), traits.walk, Gait::Walk),
            Plan::Flee { from } => ((here - from).normalize_or(Vec2::X), traits.run, Gait::Run),
        };
        self.gait = gait;
        if speed == 0.0 {
            return;
        }

        // Nicht ins Wasser oder steile Hänge hinauf: notfalls seitlich ausweichen.
        let step = speed * dt;
        let walkable = |d: Vec2| {
            let next = here + d * step * 8.0;
            terrain.height_at(next.x, next.y) > MIN_GROUND && !crate::island::in_lake(next) && terrain.normal_at(next.x, next.y).y > 0.8
        };
        let turned = [0.0f32, 0.6, -0.6, 1.2, -1.2, 1.9, -1.9]
            .into_iter()
            .map(|angle| Vec2::from_angle(angle).rotate(direction))
            .find(|&d| walkable(d));
        let Some(direction) = turned else {
            self.plan = Plan::Rest { until: self.clock + 1.0 };
            self.gait = Gait::Idle;
            return;
        };

        let next = here + direction * step;
        self.position = vec3(next.x, terrain.height_at(next.x, next.y), next.y);
        // Weich in Laufrichtung drehen.
        let wanted = direction.x.atan2(-direction.y);
        let diff = (wanted - self.facing + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        self.facing += diff * (dt * 8.0).min(1.0);
        self.moved = true;
    }

    /// Ein begehbarer Punkt im Revier – oder, wenn es einen Anziehungspunkt gibt und er nicht zu
    /// weit ist, meist dorthin.
    fn pick_target(&mut self, terrain: &Terrain) -> Option<Vec2> {
        let here = vec2(self.position.x, self.position.z);
        if let Some(goal) = self.attraction.filter(|g| g.distance(here) < 160.0) {
            if self.rng.chance(0.7) {
                // In Etappen hinziehen (höchstens 25 m je Ziel), am Ziel ein wenig verteilt
                let step = (goal - here).clamp_length_max(25.0);
                let target = here + step + Vec2::from_angle(self.rng.range(0.0, std::f32::consts::TAU)) * self.rng.range(0.0, 3.0);
                if terrain.height_at(target.x, target.y) > MIN_GROUND && !crate::island::in_lake(target) {
                    return Some(target);
                }
            }
        }
        for _ in 0..8 {
            let angle = self.rng.range(0.0, std::f32::consts::TAU);
            let distance = self.rng.range(3.0, WANDER_RADIUS);
            let target = self.home + Vec2::from_angle(angle) * distance;
            if terrain.height_at(target.x, target.y) > MIN_GROUND && !crate::island::in_lake(target) && terrain.normal_at(target.x, target.y).y > 0.85 {
                return Some(target);
            }
        }
        None
    }

    /// Einmal pro Bild: Modell an die Lage anpassen und animieren.
    pub fn update_visual(&mut self, ctx: &mut Context) {
        if ctx.is_headless() {
            return;
        }
        if self.visual.is_none() {
            self.visual = Some(Visual::new(ctx, self.kind, self.position));
        }
        let Some(visual) = &mut self.visual else { return };
        let dt = ctx.time.delta;
        // Erlegt: umkippen, kurz liegen bleiben, im Boden versinken, dann ausblenden.
        let mut fallen = 0.0f32;
        let mut sink = 0.0f32;
        if let Some(time) = &mut visual.dying {
            *time += dt;
            fallen = (*time / 0.45).min(1.0);
            sink = ((*time - 2.5) / 1.5).clamp(0.0, 1.0);
            let hidden = sink >= 1.0;
            if let Some(entity) = ctx.scene.try_get_mut(visual.entity) {
                entity.visible = !hidden;
            }
            if hidden {
                return;
            }
        } else if self.health == 0 {
            // Schon tot, als wir es zum ersten Mal gesehen haben.
            if let Some(entity) = ctx.scene.try_get_mut(visual.entity) {
                entity.visible = false;
            }
            return;
        } else if let Some(entity) = ctx.scene.try_get_mut(visual.entity) {
            entity.visible = true;
        }
        visual.shown = visual.shown.lerp(self.position, (dt * 15.0).min(1.0));
        if visual.shown.distance(self.position) > 5.0 {
            visual.shown = self.position;
        }
        let near = visual.shown.distance(ctx.camera.position) < ANIMATION_DISTANCE && visual.dying.is_none();

        let mut position = visual.shown;
        let mut tilt = Quat::IDENTITY;
        match &mut visual.body {
            Body::Animated { animator, mesh, texture, .. } => {
                if near {
                    let (clips, speed): (&[&str], f32) = match self.gait {
                        Gait::Idle => (&["Idle", "Stehen"], 1.0),
                        Gait::Walk => (&["Laufen", "Walk"], self.kind.traits().stride.0),
                        Gait::Run => (&["Rennen", "Run", "Laufen"], self.kind.traits().stride.1),
                    };
                    if !clips.iter().any(|clip| animator.current() == Some(*clip)) {
                        for clip in clips {
                            if animator.play(clip, true, 0.2) {
                                break;
                            }
                        }
                    }
                    animator.set_speed(speed);
                    animator.update(dt);
                    ctx.assets.update_mesh(*mesh, animator.skinned_mesh(*texture));
                }
            }
            Body::Placeholder => {
                // Hüpfen beim Laufen, leichtes Atmen im Stand.
                let (rate, height) = match self.gait {
                    Gait::Idle => (1.5, 0.0),
                    Gait::Walk => (7.0, 0.05),
                    Gait::Run => (13.0, 0.14),
                };
                visual.phase += dt * rate;
                let hop = visual.phase.sin().abs() * height * self.kind.traits().height * 4.0;
                position += Vec3::Y * hop;
                tilt = Quat::from_rotation_x(if self.gait == Gait::Run { visual.phase.cos() * 0.12 } else { 0.0 });
            }
        }

        // Treffer: kurz rot aufleuchten.
        visual.flash = (visual.flash - dt).max(0.0);
        let flash = visual.flash / 0.25;
        let Some(entity) = ctx.scene.try_get_mut(visual.entity) else { return };
        let height = self.kind.traits().height;
        let eased = 1.0 - (1.0 - fallen) * (1.0 - fallen);
        entity.transform.position = position + Vec3::Y * (eased * height * 0.1 - sink * height * 1.4);
        entity.color = Vec4::ONE.lerp(vec4(2.2, 0.35, 0.3, 1.0), flash);
        let base = match &visual.body {
            // Blender-Modelle schauen nach +Z, unsere Tiere nach -Z.
            Body::Animated { .. } => Quat::from_rotation_y(std::f32::consts::PI),
            Body::Placeholder => Quat::IDENTITY,
        };
        let fall = Quat::from_rotation_z(eased * std::f32::consts::FRAC_PI_2 * 0.95);
        entity.transform.rotation = Quat::from_rotation_y(-self.facing) * fall * tilt * base;
    }
}

impl Visual {
    fn new(ctx: &mut Context, kind: AnimalKind, position: Vec3) -> Visual {
        let body = match load_model(kind) {
            Some(model) => {
                let textures = model.register_textures(&mut ctx.assets);
                let texture = textures.first().copied();
                let mut animator = Animator::new(model);
                animator.play("Idle", true, 0.0);
                let mesh = ctx.assets.add_mesh(animator.skinned_mesh(texture));
                Body::Animated { animator, mesh, texture }
            }
            None => Body::Placeholder,
        };
        let mesh = match &body {
            Body::Animated { mesh, .. } => *mesh,
            Body::Placeholder => ctx.assets.named_mesh(&format!("tier_platzhalter_{}", kind.file_name()), || placeholder(kind)),
        };
        let entity = ctx.scene.spawn(Entity::new(kind.label(), mesh).with_transform(Transform::from_position(position)));
        Visual { entity, body, shown: position, phase: 0.0, flash: 0.0, dying: None }
    }
}

/// Liest das Blender-Modell einer Tierart: in der Galerie markiert, sonst `tiere/<name>.gltf`.
/// Jede Datei wird nur einmal pro Programmlauf gelesen.
fn load_model(kind: AnimalKind) -> Option<Arc<Model>> {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Mutex;
    static CACHE: Mutex<Option<HashMap<PathBuf, Option<Arc<Model>>>>> = Mutex::new(None);

    let path = crate::markierungen::fuer_platz(kind.slot())
        .into_iter()
        .next()
        .or_else(|| asset_files::variants("tiere", kind.file_name()).into_iter().next())?;
    let mut cache = CACHE.lock().ok()?;
    cache
        .get_or_insert_with(HashMap::new)
        .entry(path.clone())
        .or_insert_with(|| match Model::from_file(&path) {
            Ok(model) => Some(Arc::new(model)),
            Err(message) => {
                log::warn!("{message} – nehme den Platzhalter");
                None
            }
        })
        .clone()
}

/// Einfaches Tier aus Grundformen (schaut nach -Z), bis das Blender-Modell fertig ist.
fn placeholder(kind: AnimalKind) -> MeshData {
    let mut mesh = MeshData::default();
    let ball = |color: Vec3| MeshData::icosphere(1, color);
    let mut add = |part: MeshData, position: Vec3, scale: Vec3| {
        mesh.append(&part, Mat4::from_scale_rotation_translation(scale, Quat::IDENTITY, position));
    };
    let h = kind.traits().height;
    match kind {
        AnimalKind::Hare => {
            let fur = vec3(0.32, 0.22, 0.14);
            let light = vec3(0.75, 0.7, 0.62);
            add(ball(fur), vec3(0.0, h * 0.55, 0.05), vec3(0.3, 0.3, 0.42));
            add(ball(fur), vec3(0.0, h * 0.95, -0.2), vec3(0.2, 0.2, 0.22));
            for side in [-1.0, 1.0] {
                add(ball(fur), vec3(side * 0.05, h * 1.35, -0.18), vec3(0.06, 0.28, 0.05));
                add(ball(fur), vec3(side * 0.1, h * 0.2, 0.12), vec3(0.1, 0.12, 0.2));
            }
            add(ball(light), vec3(0.0, h * 0.7, 0.28), vec3(0.12, 0.12, 0.12));
        }
        AnimalKind::Fox => {
            let fur = vec3(0.65, 0.2, 0.03);
            let white = vec3(0.85, 0.83, 0.78);
            let dark = vec3(0.05, 0.04, 0.04);
            add(ball(fur), vec3(0.0, h * 0.85, 0.0), vec3(0.3, 0.28, 0.6));
            add(ball(fur), vec3(0.0, h * 1.15, -0.38), vec3(0.24, 0.22, 0.26));
            add(ball(white), vec3(0.0, h * 1.05, -0.55), vec3(0.1, 0.09, 0.14));
            for side in [-1.0, 1.0] {
                add(MeshData::cylinder(0.05, 0.0, 0.14, 4, fur), vec3(side * 0.07, h * 1.35, -0.38), Vec3::ONE);
                for end in [-0.2, 0.2] {
                    add(MeshData::cylinder(0.035, 0.03, h * 0.75, 5, dark), vec3(side * 0.09, 0.0, end), Vec3::ONE);
                }
            }
            add(ball(fur), vec3(0.0, h * 0.9, 0.42), vec3(0.14, 0.14, 0.4));
            add(ball(white), vec3(0.0, h * 0.95, 0.62), vec3(0.09, 0.09, 0.12));
        }
        AnimalKind::Deer => {
            let fur = vec3(0.35, 0.19, 0.08);
            let light = vec3(0.7, 0.6, 0.45);
            let antler = vec3(0.6, 0.52, 0.4);
            add(ball(fur), vec3(0.0, h * 1.0, 0.0), vec3(0.45, 0.45, 1.0));
            add(MeshData::cylinder(0.1, 0.08, 0.45, 6, fur), vec3(0.0, h * 1.1, -0.42), Vec3::ONE);
            add(ball(fur), vec3(0.0, h * 1.55, -0.55), vec3(0.22, 0.22, 0.34));
            add(ball(light), vec3(0.0, h * 0.95, 0.5), vec3(0.14, 0.18, 0.1));
            for side in [-1.0, 1.0] {
                for end in [-0.32, 0.32] {
                    add(MeshData::cylinder(0.05, 0.04, h * 0.85, 5, fur), vec3(side * 0.13, 0.0, end), Vec3::ONE);
                }
                add(MeshData::cylinder(0.025, 0.012, 0.4, 4, antler), vec3(side * 0.08, h * 1.62, -0.5), Vec3::ONE);
                add(MeshData::cylinder(0.02, 0.01, 0.2, 4, antler), vec3(side * 0.16, h * 1.8, -0.48), Vec3::ONE);
            }
        }
        AnimalKind::Bear => {
            let fur = vec3(0.14, 0.07, 0.03);
            let muzzle = vec3(0.45, 0.28, 0.14);
            add(ball(fur), vec3(0.0, h * 0.85, 0.0), vec3(1.0, 0.9, 1.9));
            add(ball(fur), vec3(0.0, h * 1.05, -1.0), vec3(0.62, 0.6, 0.65));
            add(ball(muzzle), vec3(0.0, h * 0.95, -1.3), vec3(0.3, 0.26, 0.36));
            for side in [-1.0, 1.0] {
                add(ball(fur), vec3(side * 0.2, h * 1.3, -0.95), vec3(0.18, 0.18, 0.1));
                for end in [-0.6, 0.6] {
                    add(MeshData::cylinder(0.16, 0.2, h * 0.75, 6, fur), vec3(side * 0.3, 0.0, end), Vec3::ONE);
                }
            }
        }
        AnimalKind::Sheep => {
            let wool = vec3(0.85, 0.82, 0.76);
            let face = vec3(0.05, 0.04, 0.035);
            add(ball(wool), vec3(0.0, h * 0.9, 0.0), vec3(0.5, 0.45, 0.8));
            add(ball(face), vec3(0.0, h * 1.05, -0.55), vec3(0.16, 0.18, 0.22));
            for side in [-1.0, 1.0] {
                for end in [-0.28, 0.28] {
                    add(MeshData::cylinder(0.04, 0.035, h * 0.6, 5, face), vec3(side * 0.12, 0.0, end), Vec3::ONE);
                }
            }
        }
        AnimalKind::Wolf => {
            let fur = vec3(0.24, 0.26, 0.28);
            let light = vec3(0.68, 0.66, 0.6);
            add(ball(fur), vec3(0.0, h * 0.8, 0.0), vec3(0.32, 0.34, 0.75));
            add(ball(fur), vec3(0.0, h * 1.1, -0.55), vec3(0.24, 0.24, 0.3));
            add(ball(light), vec3(0.0, h * 1.02, -0.78), vec3(0.1, 0.1, 0.16));
            for side in [-1.0, 1.0] {
                add(MeshData::cylinder(0.05, 0.0, 0.14, 4, fur), vec3(side * 0.07, h * 1.3, -0.55), Vec3::ONE);
                for end in [-0.33, 0.33] {
                    add(MeshData::cylinder(0.05, 0.04, h * 0.7, 5, fur), vec3(side * 0.1, 0.0, end), Vec3::ONE);
                }
            }
            add(ball(fur), vec3(0.0, h * 0.7, 0.6), vec3(0.12, 0.12, 0.35));
        }
    }
    mesh.flat_shaded()
}

/// Verteilt die Tiere auf der Insel – auf allen Rechnern gleich (die IDs müssen passen).
pub fn populate(terrain: &Terrain, spawn: Vec3, seed: u32, moisture: impl Fn(Vec2) -> f32) -> Vec<Animal> {
    let mut rng = Rng::new(seed as u64 ^ 0xA41A_A15);
    let mut animals = Vec::new();
    // (Art, Anzahl, davon beim Startpunkt, Gruppengröße, passt der Ort?) – Hasen auf Wiesen,
    // Füchse im Wald, Rehe überall im Grünen, Bären im Wald, Schafe in Herden auf Wiesen,
    // Wölfe als Rudel im Wald und am Berg.
    let groups: [(AnimalKind, usize, usize, usize, &dyn Fn(Vec2, f32) -> bool); 6] = [
        (AnimalKind::Hare, 80, 2, 1, &|p, h| h < 14.0 && moisture(p) < 0.52),
        (AnimalKind::Fox, 30, 2, 1, &|p, h| h < 16.0 && moisture(p) >= 0.48),
        (AnimalKind::Deer, 40, 2, 2, &|_, h| (4.0..20.0).contains(&h)),
        (AnimalKind::Bear, 16, 1, 1, &|p, h| (3.0..18.0).contains(&h) && moisture(p) >= 0.5),
        (AnimalKind::Sheep, 60, 4, 4, &|p, h| (3.0..12.0).contains(&h) && moisture(p) < 0.5),
        (AnimalKind::Wolf, 24, 0, 3, &|p, h| (6.0..24.0).contains(&h) && moisture(p) >= 0.45),
    ];
    for (kind, count, near, herd, fits) in groups {
        let mut placed = 0;
        let mut tries = 0;
        while placed < count && tries < 9000 {
            tries += 1;
            // Ein paar Tiere in Sichtweite des Startpunkts, der Rest verteilt.
            let near_spawn = placed < near;
            let p = if near_spawn {
                vec2(spawn.x, spawn.z) + Vec2::from_angle(rng.range(0.0, std::f32::consts::TAU)) * rng.range(18.0, 40.0)
            } else {
                {
                let r = crate::island::ISLAND_RADIUS;
                vec2(rng.range(-r, r), rng.range(-r, r))
            }
            };
            let h = terrain.height_at(p.x, p.y);
            if h < MIN_GROUND + 1.0 || crate::island::in_lake(p) || terrain.normal_at(p.x, p.y).y < 0.9 || !(near_spawn || fits(p, h)) {
                continue;
            }
            // Herden und Rudel: die übrigen Tiere der Gruppe dicht daneben.
            for member in 0..herd.min(count - placed) {
                let q = if member == 0 { p } else { p + Vec2::from_angle(rng.range(0.0, std::f32::consts::TAU)) * rng.range(2.0, 6.0) };
                let hq = terrain.height_at(q.x, q.y);
                if hq < MIN_GROUND + 1.0 || crate::island::in_lake(q) {
                    continue;
                }
                let seed = ((seed as u64) << 16) ^ animals.len() as u64;
                animals.push(Animal::new(kind, vec3(q.x, hq, q.y), seed));
                placed += 1;
            }
        }
    }
    animals
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_island() -> Terrain {
        Terrain::generate(Vec2::ZERO, 200.0, 50, |p| if p.length() < 80.0 { 5.0 } else { -5.0 })
    }

    #[test]
    fn tier_flieht_vor_spieler_und_bleibt_an_land() {
        let terrain = flat_island();
        let mut hare = Animal::new(AnimalKind::Hare, vec3(70.0, 5.0, 0.0), 1);
        // Spieler direkt daneben, landeinwärts: der Hase flieht Richtung Küste und muss ausweichen.
        let player = vec3(66.0, 5.0, 0.0);
        let start = hare.position;
        for _ in 0..240 {
            hare.think(1.0 / 60.0, &[player], &terrain);
        }
        assert!(hare.position.distance(player) > start.distance(player) + 3.0, "Hase ist nicht geflohen: {:?}", hare.position);
        assert!(terrain.height_at(hare.position.x, hare.position.z) > MIN_GROUND, "Hase ist ins Wasser gelaufen");
    }

    #[test]
    fn getroffenes_tier_flieht_stirbt_und_kommt_wieder() {
        let terrain = flat_island();
        let mut deer = Animal::new(AnimalKind::Deer, vec3(0.0, 5.0, 0.0), 3);
        let start = deer.position;
        // Angreifer weit weg: das Reh flieht trotzdem.
        let attacker = vec3(-30.0, 5.0, 0.0);
        assert_eq!(deer.hit(attacker), AnimalKind::Deer.max_health() - 1);
        for _ in 0..60 {
            deer.think(1.0 / 60.0, &[], &terrain);
        }
        assert_eq!(deer.gait, Gait::Run, "Reh flieht nach dem Treffer nicht");
        assert!(deer.position.x > start.x + 3.0, "Reh flieht nicht vom Angreifer weg: {:?}", deer.position);

        while deer.is_alive() {
            deer.hit(attacker);
        }
        assert_eq!(deer.hit(attacker), 0, "Totes Tier nimmt weiter Schaden");
        let dead_at = deer.position;
        for _ in 0..60 * 10 {
            deer.think(1.0 / 60.0, &[], &terrain);
        }
        assert_eq!(deer.position, dead_at, "Totes Tier läuft weiter");
        for _ in 0..(60.0 * RESPAWN_SECONDS) as usize {
            deer.think(1.0 / 60.0, &[], &terrain);
        }
        assert!(deer.is_alive(), "Reh kommt nicht wieder");
        assert_eq!(deer.health, AnimalKind::Deer.max_health());
        assert!(deer.position.distance(start) < WANDER_RADIUS + 2.0, "Neues Reh erscheint nicht in seinem alten Revier: {:?}", deer.position);
    }

    #[test]
    fn tier_streift_umher_und_ruht() {
        let terrain = flat_island();
        let mut fox = Animal::new(AnimalKind::Fox, vec3(0.0, 5.0, 0.0), 2);
        let mut walked = false;
        for _ in 0..60 * 30 {
            fox.think(1.0 / 60.0, &[], &terrain);
            walked |= fox.gait == Gait::Walk;
            assert!(vec2(fox.position.x, fox.position.z).length() < WANDER_RADIUS + 2.0, "Fuchs verlässt sein Revier");
        }
        assert!(walked, "Fuchs ist in 30 Sekunden nie gelaufen");
    }


    #[test]
    fn baeren_leben_im_wald_und_einer_beim_start() {
        let mut ctx = Context::headless();
        let world = crate::world::World::new(&mut ctx);
        let bears: Vec<_> = world.animals.iter().filter(|a| a.kind == AnimalKind::Bear).collect();
        assert_eq!(bears.len(), 16, "Bären fehlen");
        let spawn = world.spawn;
        for bear in &bears {
            let offset = bear.position - spawn;
            println!("Bär bei {:.0} / {:.0} m vom Start (Winkel {:.0}°)", offset.x, offset.z, offset.x.atan2(-offset.z).to_degrees());
        }
        assert!(bears.iter().any(|b| b.position.distance(spawn) < 45.0), "kein Bär in Sichtweite des Startpunkts");
    }
    #[test]
    fn verteilung_ist_deterministisch() {
        let terrain = flat_island();
        let a = populate(&terrain, vec3(0.0, 5.0, 0.0), 7, |_| 0.5);
        let b = populate(&terrain, vec3(0.0, 5.0, 0.0), 7, |_| 0.5);
        assert!(!a.is_empty());
        assert_eq!(a.len(), b.len());
        assert!(a.iter().zip(&b).all(|(x, y)| x.position == y.position && x.kind == y.kind));
    }
}
