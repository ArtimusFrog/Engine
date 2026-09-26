//! Gebäude, die Spieler selbst errichten (Baumenü mit B): Holzfäller, Steinbruch und Erzmine sowie
//! zehn Verteidigungstürme an den Heerstraßen (siehe `tuerme.rs`).
//!
//! Der Server prüft den Bauplatz, zieht die Kosten ab und verteilt das neue Gebäude an alle.
//! Jeder Rechner lässt es dann in gleicher Geschwindigkeit entstehen: Schicht für Schicht von
//! unten nach oben, eingerüstet, mit Staub und Hammerschlägen. Fertige Gebäude liefern ihrem
//! Erbauer regelmäßig den passenden Rohstoff; Türme schießen auf die Truppen der Festung und
//! lassen sich in drei Stufen aufwerten.
//! Modelle: `game/assets/bauten/` (Blender: `art/lib/bauten.py`, `art/lib/tuerme.py`).

use std::collections::HashMap;

use engine::prelude::*;
use serde::{Deserialize, Serialize};

use crate::protocol::{Inventory, Item};
use crate::tuerme::{TowerKind, KOPF_GROESSE, KOPF_Z, MAX_STUFE};
use crate::world::SoundEvent;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildingKind {
    Lumberjack,
    Quarry,
    Mine,
    Tower(TowerKind),
}

impl BuildingKind {
    /// Die Wirtschaftsgebäude (Reiter „Gebäude“ im Baumenü)
    pub const ALL: [BuildingKind; 3] = [BuildingKind::Lumberjack, BuildingKind::Quarry, BuildingKind::Mine];

    pub fn tower(self) -> Option<TowerKind> {
        match self {
            BuildingKind::Tower(t) => Some(t),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            BuildingKind::Lumberjack => "Holzfäller",
            BuildingKind::Quarry => "Steinbruch",
            BuildingKind::Mine => "Erzmine",
            BuildingKind::Tower(t) => t.label(),
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            BuildingKind::Lumberjack => "Blockhütte mit Holzschuppen. Der Holzfäller schlägt Holz für dich.",
            BuildingKind::Quarry => "Felswand mit Kran und Werkstatt. Bricht Steinquader für dich.",
            BuildingKind::Mine => "Stollen in den Fels, Lore voller Erz. Fördert Eisenerz für dich.",
            BuildingKind::Tower(t) => t.description(),
        }
    }

    /// Modell in `game/assets/bauten/` (Türme je Stufe).
    pub fn model(self, level: u8) -> String {
        match self {
            BuildingKind::Lumberjack => "holzfaeller".into(),
            BuildingKind::Quarry => "steinbruch".into(),
            BuildingKind::Mine => "erzmine".into(),
            BuildingKind::Tower(t) => format!("turm_{}_{}", t.file(), level.clamp(1, MAX_STUFE)),
        }
    }

    /// Name für die Vorschaubilder im Baumenü (`icons/bau_<name>.png`).
    pub fn file_name(self) -> String {
        match self {
            BuildingKind::Tower(t) => format!("turm_{}", t.file()),
            other => other.model(1),
        }
    }

    /// Kosten für das Bauen (Stufe 1).
    pub fn cost(self) -> Vec<(Item, u32)> {
        match self {
            BuildingKind::Lumberjack => vec![(Item::Wood, 20), (Item::Stone, 8)],
            BuildingKind::Quarry => vec![(Item::Wood, 25), (Item::Stone, 10)],
            BuildingKind::Mine => vec![(Item::Wood, 30), (Item::Stone, 20)],
            BuildingKind::Tower(t) => t.kosten(1).into_iter().filter(|&(_, n)| n > 0).collect(),
        }
    }

    /// Kosten, um auf `level` aufzuwerten (nur Türme).
    pub fn upgrade_cost(self, level: u8) -> Vec<(Item, u32)> {
        match self {
            BuildingKind::Tower(t) if level <= MAX_STUFE => t.kosten(level).into_iter().filter(|&(_, n)| n > 0).collect(),
            _ => Vec::new(),
        }
    }

    /// Mit unbestimmtem Artikel („einen Holzfäller“, „eine Erzmine“).
    pub fn with_article(self) -> String {
        match self {
            BuildingKind::Lumberjack => "einen Holzfäller".into(),
            BuildingKind::Quarry => "einen Steinbruch".into(),
            BuildingKind::Mine => "eine Erzmine".into(),
            BuildingKind::Tower(t) => t.with_article(),
        }
    }

    /// Was das Gebäude liefert (Türme nichts).
    pub fn produces(self) -> Option<Item> {
        match self {
            BuildingKind::Lumberjack => Some(Item::Wood),
            BuildingKind::Quarry => Some(Item::Stone),
            BuildingKind::Mine => Some(Item::Ore),
            BuildingKind::Tower(_) => None,
        }
    }

    /// Radius des Bauplatzes in Metern (Hof bzw. Vorplatz eingeschlossen).
    pub fn radius(self) -> f32 {
        match self {
            BuildingKind::Lumberjack => 7.4,
            BuildingKind::Quarry => 8.0,
            BuildingKind::Mine => 7.4,
            BuildingKind::Tower(TowerKind::Catapult) => 3.3,
            BuildingKind::Tower(_) => 2.6,
        }
    }

    /// So lange dauert der Bau bzw. das Aufwerten auf `level` (Sekunden).
    pub fn build_seconds(self, level: u8) -> f32 {
        match self {
            BuildingKind::Lumberjack => 30.0,
            BuildingKind::Quarry => 36.0,
            BuildingKind::Mine => 42.0,
            BuildingKind::Tower(_) if level > 1 => 8.0,
            BuildingKind::Tower(_) => 14.0,
        }
    }

    /// Feste Hindernisse (Mitte, Größe) im Raum des Modells – der Rest bleibt begehbar.
    fn colliders(self, level: u8) -> Vec<([f32; 3], [f32; 3])> {
        match self {
            BuildingKind::Lumberjack => vec![([0.0, 2.6, 0.0], [6.9, 5.2, 5.3]), ([4.3, 1.2, 0.0], [2.0, 2.4, 4.8])],
            BuildingKind::Quarry => vec![([0.0, 1.6, -4.3], [11.0, 3.2, 3.2]), ([2.6, 2.7, -0.2], [0.6, 5.4, 0.6])],
            BuildingKind::Mine => vec![([0.0, 1.8, -4.3], [10.0, 3.6, 4.2])],
            BuildingKind::Tower(t) => {
                let h = KOPF_Z[(level.clamp(1, MAX_STUFE) - 1) as usize] + 1.0;
                let breite = if t == TowerKind::Catapult { 4.4 } else { 3.3 };
                vec![([0.0, h / 2.0, 0.0], [breite, h, breite])]
            }
        }
    }
}

/// Wie oft ein fertiges Gebäude liefert (Sekunden).
pub const PRODUCTION_SECONDS: f32 = 40.0;
/// Wie weit man vom Bauplatz entfernt stehen darf.
pub const BUILD_REACH: f32 = 32.0;
/// Türme stehen mit ihrer Mitte so weit neben einer Heerstraße (Mittellinie, Meter).
pub const TURM_ABSTAND: (f32, f32) = (4.5, 14.0);

fn stufe_eins() -> u8 {
    1
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Building {
    pub id: u32,
    pub kind: BuildingKind,
    /// Mitte am Boden
    pub position: Vec3,
    pub yaw: f32,
    /// Baufortschritt 0..1 (nach dem Aufwerten wieder von vorn)
    pub progress: f32,
    /// Name des Erbauers (klein geschrieben, wie die Inventare im Spielstand)
    pub owner: String,
    /// Sekunden bis zur nächsten Lieferung (zählt nur der Server)
    #[serde(default)]
    pub produce_in: f32,
    /// Stufe (Türme 1–3, sonst 1)
    #[serde(default = "stufe_eins")]
    pub level: u8,
}

impl Building {
    pub fn finished(&self) -> bool {
        self.progress >= 1.0
    }

    pub fn rotation(&self) -> Quat {
        Quat::from_rotation_y(self.yaw)
    }

    pub fn tower(&self) -> Option<TowerKind> {
        self.kind.tower()
    }

    /// Was bisher insgesamt bezahlt wurde (fürs Abreißen: die Hälfte kommt zurück).
    pub fn paid(&self) -> Vec<(Item, u32)> {
        let mut summe: Vec<(Item, u32)> = Vec::new();
        let mut dazu = |kosten: Vec<(Item, u32)>| {
            for (item, n) in kosten {
                match summe.iter_mut().find(|(i, _)| *i == item) {
                    Some((_, m)) => *m += n,
                    None => summe.push((item, n)),
                }
            }
        };
        dazu(self.kind.cost());
        for stufe in 2..=self.level {
            dazu(self.kind.upgrade_cost(stufe));
        }
        summe
    }
}

/// Reicht das Inventar für diese Kosten?
pub fn can_pay(inventory: &Inventory, cost: &[(Item, u32)]) -> bool {
    cost.iter().all(|&(item, amount)| inventory.count(item) >= amount)
}

/// Reicht das Inventar für den Bau?
pub fn affordable(inventory: &Inventory, kind: BuildingKind) -> bool {
    can_pay(inventory, &kind.cost())
}

/// Abstand eines Punkts zur nächsten Heerstraße (Mittellinie, Meter).
pub fn road_distance(world: &crate::world::World, at: Vec2) -> f32 {
    let mut best = f32::MAX;
    for strasse in world.heer.strassen() {
        for pair in strasse.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let ab = b - a;
            let t = ((at - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
            best = best.min((a + ab * t).distance(at));
        }
    }
    best
}

/// Bauplatz prüfen: Liefert die Bodenhöhe für das Gebäude oder den Grund, warum es hier nicht geht.
pub fn check_site(world: &crate::world::World, kind: BuildingKind, at: Vec2, builder: Option<Vec3>) -> Result<f32, &'static str> {
    let radius = kind.radius();
    if let Some(builder) = builder {
        if vec2(builder.x, builder.z).distance(at) > BUILD_REACH {
            return Err("Zu weit weg");
        }
    }
    let strasse = road_distance(world, at);
    if kind.tower().is_some() {
        if strasse < TURM_ABSTAND.0 {
            return Err("Nicht auf die Straße");
        }
        if strasse > TURM_ABSTAND.1 {
            return Err("Türme nur direkt an einer Heerstraße");
        }
    } else if strasse < radius + 2.5 {
        return Err("Nicht auf die Heerstraße");
    }
    let terrain = &world.terrain;
    // Hang: Höhen auf zwei Ringen um die Mitte
    let (mut low, mut high) = (f32::MAX, f32::MIN);
    for ring in [0.0, 0.45, 0.8] {
        let steps = if ring == 0.0 { 1 } else { 12 };
        for k in 0..steps {
            let p = at + Vec2::from_angle(k as f32 / steps as f32 * std::f32::consts::TAU) * radius * ring;
            let h = terrain.height_at(p.x, p.y);
            low = low.min(h);
            high = high.max(h);
        }
    }
    if low < 0.8 {
        return Err("Zu nah am Wasser");
    }
    if high - low > 2.6 {
        return Err("Zu steil");
    }
    let spawn = vec2(world.spawn.x, world.spawn.z);
    if at.distance(spawn) < 28.0 + radius {
        return Err("Zu nah am Startlager");
    }
    for (_, place) in crate::island::landmarks().iter().take(2) {
        if at.distance(*place) < 95.0 + radius {
            return Err("Zu nah an der Festung oder Burg");
        }
    }
    for other in &world.buildings {
        if vec2(other.position.x, other.position.z).distance(at) < radius + other.kind.radius() {
            return Err("Zu nah an einem anderen Gebäude");
        }
    }
    let blocked = world.resources.values().any(|r| {
        let p = r.spec.transform.position;
        r.is_present() && vec2(p.x, p.z).distance(at) < radius * 0.7
    });
    if blocked {
        return Err("Bäume oder Felsen im Weg");
    }
    // Etwas unter die Mitte setzen: der Sockel reicht unter den Boden, an Hängen verschwindet
    // die höhere Seite ein Stück im Gelände statt dass die tiefere schwebt.
    Ok(low + (high - low) * 0.4)
}

/// Feste Hindernisse eines Gebäudes in die Physik eintragen (auf allen Rechnern gleich).
/// `marker`: unsichtbares Objekt, an dem die Körper hängen (zum Anvisieren und Entfernen).
pub fn add_colliders(ctx: &mut Context, building: &Building, marker: EntityId) -> Vec<RigidBodyHandle> {
    let rotation = building.rotation();
    building
        .kind
        .colliders(building.level)
        .into_iter()
        .map(|(center, size)| {
            let transform = Transform::from_position(building.position + rotation * Vec3::from(center)).with_rotation(rotation);
            ctx.physics.add_body(marker, &transform, BodyDesc::fixed(Shape::Box { size: Vec3::from(size) }))
        })
        .collect()
}

// ---------- Darstellung (nur mit Fenster) ----------

/// So viele Schichten wächst ein Gebäude beim Bauen.
const BANDS: usize = 16;

/// Meshes einer Gebäudeart: ganz, leuchtende Teile, und in waagerechte Schichten zerlegt.
struct KindVisual {
    full: MeshId,
    glow: Option<MeshId>,
    bands: Vec<MeshId>,
    /// Unterkante des Modells und Höhe je Schicht
    bottom: f32,
    band_height: f32,
    /// Grundriss oberhalb des Bodens (min, max in x/z), für das Gerüst
    min: Vec2,
    max: Vec2,
    height: f32,
}

struct Site {
    bands: Vec<EntityId>,
    full: EntityId,
    glow: Option<EntityId>,
    /// Modell (Art und Stufe), mit dem die Baustelle angelegt wurde
    model: String,
    /// Turmkopf: Objekt, aktuelle und gewünschte Drehung, Zeit seit dem letzten Schuss
    head: Option<(EntityId, f32, f32, f32)>,
    /// Gerüst: Objekt und die Höhe, ab der es zu sehen ist
    scaffold: Vec<(EntityId, f32)>,
    shown_bands: usize,
    hammer: f32,
    finished: bool,
}

struct Ghost {
    kind: BuildingKind,
    entity: EntityId,
    /// Reichweite (Ring um die Vorschau eines Turms)
    ring: Option<EntityId>,
}

#[derive(Default)]
pub struct BauVisuals {
    kinds: HashMap<String, Option<KindVisual>>,
    sites: HashMap<u32, Site>,
    ghost: Option<Ghost>,
    rng: Option<Rng>,
    /// Giftwolken der Türme: Mitte, Radius, Restzeit
    clouds: Vec<(Vec3, f32, f32)>,
}

impl BauVisuals {
    fn kind(&mut self, ctx: &mut Context, model: &str) -> Option<&KindVisual> {
        self.kinds.entry(model.to_string()).or_insert_with(|| load_kind(ctx, model)).as_ref()
    }

    /// Eine Baustelle ganz entfernen (abgerissen oder vor dem Aufwerten).
    pub fn remove_site(&mut self, ctx: &mut Context, id: u32) {
        let Some(site) = self.sites.remove(&id) else { return };
        for entity in site.bands.iter().chain([&site.full]).chain(site.glow.iter()).chain(site.scaffold.iter().map(|(e, _)| e)) {
            ctx.scene.despawn(*entity);
        }
        if let Some((head, ..)) = site.head {
            ctx.scene.despawn(head);
        }
    }

    /// Ein Turm hat geschossen: Kopf zum Ziel drehen, Geschoss und Treffer zeigen.
    pub fn shot(&mut self, ctx: &mut Context, building: &Building, target: Vec3, sounds: &mut Vec<SoundEvent>) {
        let Some(kind) = building.tower() else { return };
        let mund = building.position + Vec3::Y * (KOPF_Z[(building.level.clamp(1, MAX_STUFE) - 1) as usize] + 1.1);
        if let Some(site) = self.sites.get_mut(&building.id) {
            if let Some(head) = &mut site.head {
                let to = target - building.position;
                head.2 = to.x.atan2(to.z);
                head.3 = 0.0;
            }
        }
        if mund.distance(ctx.camera.position) > 160.0 {
            return;
        }
        let (farbe, glow, dicke, bogen) = match kind {
            TowerKind::Arrow => (vec3(0.5, 0.35, 0.2), 0.0, 0.05, 0.05),
            TowerKind::Ballista => (vec3(0.45, 0.32, 0.2), 0.0, 0.1, 0.02),
            TowerKind::Catapult => (vec3(0.55, 0.52, 0.48), 0.0, 0.22, 0.35),
            TowerKind::Fire => (vec3(1.0, 0.45, 0.1), 4.0, 0.2, 0.0),
            TowerKind::Frost => (vec3(0.55, 0.85, 1.0), 3.0, 0.12, 0.0),
            TowerKind::Lightning => (vec3(0.6, 0.85, 1.0), 6.0, 0.08, 0.0),
            TowerKind::Sun => (vec3(1.0, 0.9, 0.5), 5.0, 0.1, 0.0),
            TowerKind::Arcane => (vec3(0.75, 0.5, 1.0), 4.0, 0.12, 0.0),
            TowerKind::Poison => (vec3(0.45, 0.95, 0.35), 1.5, 0.16, 0.25),
            TowerKind::Banner => return,
        };
        let weite = mund.distance(target);
        let schritte = (weite / 0.7).clamp(4.0, 40.0) as usize;
        let rng = self.rng.get_or_insert_with(|| Rng::new(0xBA0));
        for i in 0..=schritte {
            let t = i as f32 / schritte as f32;
            let mut p = mund.lerp(target, t) + Vec3::Y * (t * (1.0 - t) * weite * bogen);
            if kind == TowerKind::Lightning {
                // Zickzack
                p += vec3(rng.range(-0.35, 0.35), rng.range(-0.35, 0.35), rng.range(-0.35, 0.35)) * (t * (1.0 - t) * 4.0);
            }
            ctx.particles.burst(Burst {
                position: p,
                count: 1,
                color: farbe,
                color_variation: 0.1,
                speed: 0.05,
                direction: Vec3::ZERO,
                size: dicke,
                life: 0.12 + t * 0.25,
                gravity: 0.0,
                glow,
                grow: 0.0,
                round: glow > 0.0,
            });
        }
        let (anzahl, flaeche) = match kind {
            TowerKind::Catapult => (26, 2.2),
            TowerKind::Fire => (18, 1.2),
            _ => (10, 0.6),
        };
        ctx.particles.burst(Burst {
            position: target,
            count: anzahl,
            color: farbe,
            color_variation: 0.25,
            speed: 2.0 + flaeche,
            direction: Vec3::Y * 0.5,
            size: 0.08 + flaeche * 0.04,
            life: 0.5,
            gravity: if glow > 0.0 { 1.0 } else { 8.0 },
            glow,
            grow: if kind == TowerKind::Catapult { 1.0 } else { 0.0 },
            round: kind == TowerKind::Catapult || glow > 0.0,
        });
        if kind == TowerKind::Poison {
            self.clouds.push((target, kind.werte(building.level).flaeche, kind.werte(building.level).dauer));
        }
        sounds.push(SoundEvent::Impact { at: target, animal: false, killed: false });
    }

    /// Neues Gebäude (oder beim Beitreten ein schon stehendes) sichtbar machen. `hidden` sind
    /// Gräser und Büsche im Bauplatz, die verschwinden.
    pub fn add_site(&mut self, ctx: &mut Context, building: &Building, keep: &dyn Fn(EntityId) -> bool) {
        let model = building.kind.model(building.level);
        let Some(visual) = self.kind(ctx, &model) else { return };
        let (full_mesh, glow_mesh, band_meshes) = (visual.full, visual.glow, visual.bands.clone());
        let (min, max, height) = (visual.min, visual.max, visual.height);
        let transform = Transform::from_position(building.position).with_rotation(building.rotation());
        let bands = band_meshes.iter().map(|&m| ctx.scene.spawn(Entity::new("Bauabschnitt", m).with_transform(transform))).collect();
        let full = ctx.scene.spawn(Entity::new(building.kind.label(), full_mesh).with_transform(transform));
        let glow = glow_mesh.map(|m| ctx.scene.spawn(Entity::new("Licht", m).with_transform(transform).with_material(Material::Emissive { glow: 2.2 })));
        let mut scaffold = spawn_scaffold(ctx, building, min, max, height);
        // Baumaterial vor der Baustelle: Holzstapel, Kisten und ein Fass (verschwinden mit dem Gerüst)
        let rotation = building.rotation();
        for (name, local, turn) in [("holzstapel", vec2(max.x + 1.6, max.y + 2.2), 0.3), ("kiste", vec2(min.x - 1.2, max.y + 2.0), 0.9), ("fass", vec2(min.x - 1.4, max.y + 3.0), 0.0)] {
            if let Some(&(mesh, _)) = crate::asset_files::load_variants(ctx, "gebaeude", name, Vec3::ONE, 0.0).first() {
                let at = building.position + rotation * vec3(local.x, 0.0, local.y);
                let transform = Transform::from_position(at).with_rotation(rotation * Quat::from_rotation_y(turn));
                scaffold.push((ctx.scene.spawn(Entity::new("Baumaterial", mesh).with_transform(transform)), 0.0));
            }
        }
        // Turmkopf oben auf der Plattform (dreht sich zum Ziel)
        let head = building.tower().and_then(|t| {
            let (mesh, _) = crate::asset_files::load_variants(ctx, "bauten", &format!("turm_{}_kopf", t.file()), Vec3::ONE, 0.0).into_iter().next()?;
            let z = KOPF_Z[(building.level.clamp(1, MAX_STUFE) - 1) as usize];
            let transform = Transform::from_position(building.position + Vec3::Y * z)
                .with_rotation(building.rotation())
                .with_scale(Vec3::splat(KOPF_GROESSE));
            let mut entity = Entity::new("Turmkopf", mesh).with_transform(transform);
            entity.visible = building.finished();
            Some((ctx.scene.spawn(entity), building.yaw, building.yaw, 10.0))
        });
        let mut site = Site { bands, full, glow, model, head, scaffold, shown_bands: usize::MAX, hammer: 0.0, finished: false };
        site.show(ctx, building.progress, false);
        self.sites.insert(building.id, site);

        // Gras und Büsche auf dem Bauplatz verschwinden (Bäume und Felsen stehen dort nicht).
        let at = vec2(building.position.x, building.position.z);
        let radius = building.kind.radius() * 0.95;
        let hidden: Vec<EntityId> = ctx
            .scene
            .iter_entities()
            .filter(|(id, e)| {
                e.parent.is_none()
                    && matches!(e.material, Material::Foliage { .. } | Material::Leaves { .. })
                    && vec2(e.transform.position.x, e.transform.position.z).distance(at) < radius
                    && keep(*id)
            })
            .map(|(id, _)| id)
            .collect();
        for id in hidden {
            ctx.scene.get_mut(id).visible = false;
        }
    }

    /// Einmal pro Bild: Baufortschritt zeigen, Staub und Hammerschläge. Liefert die Gebäude,
    /// die gerade fertig geworden sind.
    pub fn update(&mut self, ctx: &mut Context, buildings: &[Building], sounds: &mut Vec<SoundEvent>) -> Vec<BuildingKind> {
        let mut done = Vec::new();
        let rng = self.rng.get_or_insert_with(|| Rng::new(0xBA0));
        let dt = ctx.time.delta;
        // Giftwolken wabern am Boden
        for (mitte, radius, rest) in &mut self.clouds {
            *rest -= dt;
            if rng.chance((dt * 14.0).min(1.0)) {
                let p = *mitte + vec3(rng.range(-*radius, *radius), 0.2, rng.range(-*radius, *radius));
                ctx.particles.burst(Burst {
                    position: p,
                    count: 1,
                    color: vec3(0.35, 0.8, 0.25),
                    color_variation: 0.2,
                    speed: 0.3,
                    direction: Vec3::Y,
                    size: 0.4,
                    life: 1.4,
                    gravity: -0.2,
                    glow: 0.6,
                    grow: 1.5,
                    round: true,
                });
            }
        }
        self.clouds.retain(|c| c.2 > 0.0);
        for building in buildings {
            let Some(site) = self.sites.get_mut(&building.id) else { continue };
            // Turmköpfe drehen sich zum Ziel; ohne Ziel schauen sie sich langsam um
            if let Some((entity, yaw, want, idle)) = &mut site.head {
                *idle += dt;
                if *idle > 4.0 {
                    *want += dt * 0.25;
                }
                let turn = (*want - *yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                *yaw += turn * (dt * 6.0).min(1.0);
                if let Some(e) = ctx.scene.try_get_mut(*entity) {
                    e.transform.rotation = Quat::from_rotation_y(*yaw);
                    e.visible = building.finished();
                }
            }
            let Some(Some(visual)) = self.kinds.get(&site.model) else { continue };
            let was_finished = site.finished;
            let new_band = site.show(ctx, building.progress, true);
            let rotation = building.rotation();
            let corner = |rng: &mut Rng, y: f32| {
                let local = vec3(rng.range(visual.min.x, visual.max.x), y, rng.range(visual.min.y, visual.max.y));
                building.position + rotation * local
            };
            if site.finished {
                if !was_finished {
                    // Fertig: Staubwolke, Funkeln, Glockenton
                    for _ in 0..8 {
                        let y = rng.range(0.0, visual.height * 0.6);
                        let at = corner(rng, y);
                        ctx.particles.burst(dust(at, 10, 1.4));
                    }
                    ctx.particles.burst(Burst {
                        position: building.position + Vec3::Y * visual.height * 0.6,
                        count: 60,
                        color: vec3(1.0, 0.85, 0.45),
                        color_variation: 0.3,
                        speed: 6.0,
                        direction: Vec3::Y,
                        size: 0.12,
                        life: 1.4,
                        gravity: 3.0,
                        glow: 3.0,
                        grow: 0.0,
                        round: true,
                    });
                    sounds.push(SoundEvent::Built { at: building.position, done: true });
                    done.push(building.kind);
                }
                continue;
            }
            let top = visual.bottom + visual.band_height * (building.progress * BANDS as f32);
            if new_band {
                for _ in 0..3 {
                    let at = corner(rng, top.max(0.2));
                    ctx.particles.burst(dust(at, 6, 1.0));
                }
            }
            // Hammerschläge in unregelmäßigem Takt (nur aus der Nähe zu hören)
            site.hammer -= ctx.time.delta;
            if site.hammer <= 0.0 {
                site.hammer = rng.range(0.35, 0.9);
                let at = corner(rng, top.clamp(0.3, visual.height));
                if at.distance(ctx.camera.position) < 70.0 {
                    sounds.push(SoundEvent::Built { at, done: false });
                    ctx.particles.burst(Burst {
                        position: at,
                        count: 4,
                        color: vec3(0.75, 0.62, 0.45),
                        color_variation: 0.2,
                        speed: 2.0,
                        size: 0.05,
                        life: 0.5,
                        gravity: 9.0,
                        ..Default::default()
                    });
                }
            }
            for (entity, from) in &site.scaffold {
                if let Some(e) = ctx.scene.try_get_mut(*entity) {
                    e.visible = top + 1.2 >= *from;
                }
            }
        }
        done
    }

    /// Vorschau beim Platzieren: Modell in Grün (passt) oder Rot (passt nicht). `None` = weg.
    pub fn set_ghost(&mut self, ctx: &mut Context, ghost: Option<(BuildingKind, Vec3, f32, bool)>) {
        if let Some(old) = &self.ghost {
            if ghost.is_none_or(|(kind, ..)| kind != old.kind) {
                ctx.scene.despawn(old.entity);
                if let Some(ring) = old.ring {
                    ctx.scene.despawn(ring);
                }
                self.ghost = None;
            }
        }
        let Some((kind, position, yaw, valid)) = ghost else { return };
        if self.ghost.is_none() {
            let Some(mesh) = self.kind(ctx, &kind.model(1)).map(|v| v.full) else { return };
            let entity = ctx.scene.spawn(Entity::new("Bauvorschau", mesh));
            // Reichweite eines Turms als leuchtender Ring am Boden
            let ring = kind.tower().filter(|t| t.werte(1).reichweite > 0.0).map(|_| {
                let mesh = ctx.assets.named_mesh("reichweite_ring", ring_mesh);
                ctx.scene.spawn(Entity::new("Reichweite", mesh).with_material(Material::Emissive { glow: 1.2 }))
            });
            self.ghost = Some(Ghost { kind, entity, ring });
        }
        let Some(ghost) = &self.ghost else { return };
        let pulse = 0.5 + 0.5 * (ctx.time.elapsed * 4.0).sin();
        let farbe = if valid { vec4(0.35, 1.7, 0.45, 1.0) } else { vec4(1.9, 0.35, 0.3, 1.0) };
        let entity = ctx.scene.get_mut(ghost.entity);
        entity.transform = Transform::from_position(position).with_rotation(Quat::from_rotation_y(yaw));
        entity.color = farbe;
        entity.material = Material::Emissive { glow: 0.3 + pulse * 0.25 };
        if let (Some(ring), Some(tower)) = (ghost.ring, kind.tower()) {
            let weite = tower.werte(1).reichweite;
            let e = ctx.scene.get_mut(ring);
            e.transform = Transform::from_position(position + Vec3::Y * 0.3).with_scale(vec3(weite, 1.0, weite));
            e.color = farbe;
        }
    }
}

impl Site {
    /// Zeigt den Stand `progress`; liefert, ob gerade eine neue Schicht dazugekommen ist.
    fn show(&mut self, ctx: &mut Context, progress: f32, animate: bool) -> bool {
        let finished = progress >= 1.0;
        let bands = ((progress * BANDS as f32).floor() as usize + 1).min(BANDS);
        let mut new_band = false;
        if finished != self.finished || self.shown_bands == usize::MAX {
            self.finished = finished;
            ctx.scene.get_mut(self.full).visible = finished;
            if let Some(glow) = self.glow {
                ctx.scene.get_mut(glow).visible = finished;
            }
            for (entity, _) in &self.scaffold {
                ctx.scene.get_mut(*entity).visible = !finished;
            }
        }
        if finished {
            for &entity in &self.bands {
                ctx.scene.get_mut(entity).visible = false;
            }
            self.shown_bands = BANDS;
            return false;
        }
        if bands != self.shown_bands {
            new_band = animate && self.shown_bands != usize::MAX && bands > self.shown_bands;
            let base = self.base_position(ctx);
            for (i, &entity) in self.bands.iter().enumerate() {
                let e = ctx.scene.get_mut(entity);
                e.visible = i < bands;
                e.transform.position = base;
            }
            self.shown_bands = bands;
        }
        // Die oberste Schicht schiebt sich von unten in ihre Lage
        if let Some(&entity) = self.bands.get(bands - 1) {
            let within = (progress * BANDS as f32).fract();
            let base = self.base_position(ctx);
            let e = ctx.scene.get_mut(entity);
            e.transform.position = base - Vec3::Y * (1.0 - within).powi(2) * 0.45;
        }
        new_band
    }

    fn base_position(&self, ctx: &Context) -> Vec3 {
        ctx.scene.get(self.full).transform.position
    }
}

fn dust(at: Vec3, count: u32, size: f32) -> Burst {
    Burst {
        position: at,
        count,
        color: vec3(0.78, 0.7, 0.58),
        color_variation: 0.12,
        speed: 1.2,
        direction: Vec3::Y * 0.4,
        size: 0.35 * size,
        life: 1.6,
        gravity: -0.3,
        glow: 0.0,
        grow: 1.8,
        round: true,
    }
}

/// Flacher Ring mit Radius 1 (wird auf die Reichweite skaliert).
fn ring_mesh() -> MeshData {
    let mut mesh = MeshData::default();
    let n = 96;
    for k in 0..n {
        let a = std::f32::consts::TAU * k as f32 / n as f32;
        let b = std::f32::consts::TAU * (k + 1) as f32 / n as f32;
        let p = |w: f32, r: f32| vec3(w.cos() * r, 0.0, w.sin() * r);
        let (a0, a1, b0, b1) = (p(a, 0.985), p(a, 1.0), p(b, 0.985), p(b, 1.0));
        mesh.push_triangle(a0, b1, a1, Vec3::ONE);
        mesh.push_triangle(a0, b0, b1, Vec3::ONE);
    }
    mesh.double_sided = true;
    mesh
}

/// Modell laden und in Schichten zerlegen.
fn load_kind(ctx: &mut Context, model: &str) -> Option<KindVisual> {
    let (full, glow) = crate::asset_files::load_variants(ctx, "bauten", model, Vec3::ONE, 0.0).into_iter().next()?;
    let mesh = ctx.assets.mesh(full).clone();
    let (bottom, top) = mesh.vertices.iter().fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(v.position[1]), hi.max(v.position[1])));
    let band_height = (top - bottom) / BANDS as f32;
    // Jedes Dreieck in die Schicht seiner Mitte
    let mut parts: Vec<MeshData> = (0..BANDS)
        .map(|_| MeshData { texture: mesh.texture, double_sided: mesh.double_sided, alpha_cutout: mesh.alpha_cutout, ..Default::default() })
        .collect();
    let mut remap: Vec<HashMap<u32, u32>> = vec![HashMap::new(); BANDS];
    for tri in mesh.indices.chunks_exact(3) {
        let center = tri.iter().map(|&i| mesh.vertices[i as usize].position[1]).sum::<f32>() / 3.0;
        let band = (((center - bottom) / band_height).floor() as usize).min(BANDS - 1);
        let part = &mut parts[band];
        for &i in tri {
            let index = *remap[band].entry(i).or_insert_with(|| {
                part.vertices.push(mesh.vertices[i as usize]);
                part.vertices.len() as u32 - 1
            });
            part.indices.push(index);
        }
    }
    let bands = parts.into_iter().map(|part| ctx.assets.add_mesh(part)).collect();
    // Grundriss des eigentlichen Gebäudes (was höher als 2 m ist – ohne Hof, Stapel und Schild)
    let (mut min, mut max) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
    for v in mesh.vertices.iter().filter(|v| v.position[1] > 2.0) {
        let p = vec2(v.position[0], v.position[2]);
        min = min.min(p);
        max = max.max(p);
    }
    if min.x > max.x {
        (min, max) = (Vec2::splat(-3.0), Vec2::splat(3.0));
    }
    Some(KindVisual { full, glow, bands, bottom, band_height, min, max, height: top })
}

/// Baugerüst um den Grundriss: Stangen an den Ecken und alle paar Meter, Laufbretter in
/// mehreren Höhen (erst sichtbar, wenn der Bau so hoch gekommen ist).
fn spawn_scaffold(ctx: &mut Context, building: &Building, min: Vec2, max: Vec2, height: f32) -> Vec<(EntityId, f32)> {
    let cube = ctx.assets.cube();
    let rotation = building.rotation();
    let wood = vec4(0.62, 0.43, 0.24, 1.0);
    let plank = vec4(0.78, 0.6, 0.36, 1.0);
    let (min, max) = (min - Vec2::splat(0.6), max + Vec2::splat(0.6));
    let top = (height * 0.9).clamp(2.5, 8.0);
    let mut parts = Vec::new();
    let mut add = |ctx: &mut Context, local: Vec3, size: Vec3, color: Vec4, from: f32| {
        let transform = Transform::from_position(building.position + rotation * local).with_rotation(rotation).with_scale(size);
        let entity = ctx.scene.spawn(Entity::new("Gerüst", cube).with_transform(transform).with_color(color));
        parts.push((entity, from));
    };
    // Stangen entlang des Umfangs
    let posts = |a: f32, b: f32| {
        let n = ((b - a) / 2.8).ceil().max(1.0) as usize;
        (0..=n).map(move |i| a + (b - a) * i as f32 / n as f32)
    };
    let mut spots = Vec::new();
    for x in posts(min.x, max.x) {
        spots.push(vec2(x, min.y));
        spots.push(vec2(x, max.y));
    }
    for z in posts(min.y, max.y) {
        spots.push(vec2(min.x, z));
        spots.push(vec2(max.x, z));
    }
    spots.dedup_by(|a, b| a.distance(*b) < 0.1);
    for p in spots {
        add(ctx, vec3(p.x, top / 2.0, p.y), vec3(0.12, top, 0.12), wood, 0.0);
    }
    // Laufbretter und Querstangen je Ebene
    let mut level = 1.4;
    while level < top - 0.3 {
        let (w, d) = (max.x - min.x, max.y - min.y);
        let (cx, cz) = ((min.x + max.x) / 2.0, (min.y + max.y) / 2.0);
        for (center, size) in [
            (vec3(cx, level, min.y), vec3(w, 0.06, 0.55)),
            (vec3(cx, level, max.y), vec3(w, 0.06, 0.55)),
            (vec3(min.x, level, cz), vec3(0.55, 0.06, d)),
            (vec3(max.x, level, cz), vec3(0.55, 0.06, d)),
        ] {
            add(ctx, center, size, plank, level);
            // Geländer
            let rail = if size.x > 1.0 { vec3(size.x, 0.07, 0.07) } else { vec3(0.07, 0.07, size.z) };
            add(ctx, center + Vec3::Y * 0.9, rail, wood, level);
        }
        level += 1.6;
    }
    parts
}
