//! Besondere Orte auf der Insel: das Startlager (Lagerfeuer, Zelte, Holzstapel, Karren,
//! Wegweiser) und später weitere Sehenswürdigkeiten. Modelle aus `game/assets/gebaeude/`
//! (Blender-Skripte in `art/modelle/gebaeude/`, gebaut mit `art/lib/lager.py`).
//!
//! Alles ist Kulisse mit Kollision: man kann nicht hindurchlaufen, aber nichts abbauen.

use engine::prelude::*;

use crate::asset_files;

/// Was an Orten steht und die Welt später noch braucht (Feuer, Schilder).
#[derive(Default)]
pub struct Places {
    /// Mitte der Lagerfeuer (am Boden): Flammen, Rauch, Licht, Knistern.
    pub fires: Vec<Vec3>,
    /// Spitze eines Wegweiser-Bretts und sein Ziel (Beschriftung aus der Nähe).
    pub signs: Vec<(Vec3, &'static str)>,
    /// Namen auf der Karte (Sehenswürdigkeiten).
    pub labels: Vec<(&'static str, Vec2)>,
    /// Lampen und Leuchtendes: Ort, Farbe mal Helligkeit, Reichweite (nachts kräftiger).
    pub lights: Vec<(Vec3, Vec3, f32)>,
    /// Boote auf dem Wasser (schaukeln): Objekt, Ruhelage, Drehung.
    pub boats: Vec<(EntityId, Vec3, Quat)>,
    /// Brandung: Punkte auf der Wasserlinie flacher Strände und Richtung aufs Land.
    pub surf: Vec<(Vec3, Vec2)>,
    /// Wegelaternen (Fuß am Boden)
    pub lanterns: Vec<Vec3>,
    /// Windmühlenflügel: Objekt, Nabe, Grunddrehung (die Flügel drehen um die eigene X-Achse)
    pub windmills: Vec<(EntityId, Vec3, Quat)>,
}

/// Mitte des Startlagers: ein Stück vom Startpunkt landeinwärts.
pub fn camp_center(spawn: Vec3) -> Vec2 {
    let at = vec2(spawn.x, spawn.z);
    at + (-at).normalize_or(Vec2::NEG_Y) * 7.0
}

/// Halbmesser des Lagers: hier wachsen keine Bäume, Büsche und kein hohes Gras.
pub const CAMP_RADIUS: f32 = 10.0;

/// Dreht +X (die Vorderseite unserer Kulissen-Modelle) in Richtung `dir` (x, z).
fn facing(dir: Vec2) -> Quat {
    Quat::from_rotation_y((-dir.y).atan2(dir.x))
}

/// Stellt ein Modell auf den Boden, optional mit Kollisionskasten (volle Größe in Modell-Achsen).
fn prop(ctx: &mut Context, terrain: &Terrain, name: &str, at: Vec2, rotation: Quat, collider: Option<Vec3>) -> Option<EntityId> {
    let (mesh, glow) = *asset_files::load_variants(ctx, "gebaeude", name, Vec3::ONE, 0.0).first()?;
    let base = vec3(at.x, terrain.height_at(at.x, at.y) - 0.03, at.y);
    let transform = Transform::from_position(base).with_rotation(rotation);
    let entity = ctx.scene.spawn(Entity::new(name.to_string(), mesh).with_transform(transform));
    if let Some(glow) = glow {
        ctx.scene.spawn(Entity::new(format!("{name} (leuchtet)"), glow).with_transform(transform).with_material(Material::Emissive { glow: 2.0 }));
    }
    if let Some(size) = collider {
        let body = Transform::from_position(base + Vec3::Y * size.y / 2.0).with_rotation(rotation);
        ctx.physics.add_body(entity, &body, BodyDesc::fixed(Shape::Box { size }));
    }
    Some(entity)
}

/// Das Startlager rund um `camp_center`: Feuer mit drei Bänken, zwei Zelte, Holzstapel, Karren,
/// Kisten und Fässer, am Startpunkt ein Wegweiser zu den großen Orten der Insel.
pub fn build_camp(ctx: &mut Context, terrain: &Terrain, spawn: Vec3, places: &mut Places, landmarks: &[(&'static str, Vec2)]) {
    let center = camp_center(spawn);
    let start = vec2(spawn.x, spawn.z);
    let forward = (center - start).normalize_or(Vec2::NEG_Y);
    let side = forward.perp();
    let at = |f: f32, s: f32| center + forward * f + side * s;
    let toward = |from: Vec2, to: Vec2| facing((to - from).normalize_or(Vec2::X));

    // Feuer und Bänke (längs zum Feuer)
    if prop(ctx, terrain, "lagerfeuer", center, Quat::IDENTITY, Some(vec3(1.4, 0.5, 1.4))).is_some() {
        places.fires.push(vec3(center.x, terrain.height_at(center.x, center.y), center.y));
    }
    for (f, s) in [(0.0, 2.1), (0.0, -2.1), (2.1, 0.0)] {
        let spot = at(f, s);
        let tangent = (spot - center).perp().normalize_or(Vec2::X);
        prop(ctx, terrain, "bank", spot, facing(tangent), Some(vec3(1.6, 0.45, 0.4)));
    }
    // Zelte mit dem Eingang zum Feuer
    for (name, f, s) in [("zelt_1", 4.4, 3.2), ("zelt_2", 4.8, -3.0)] {
        let spot = at(f, s);
        prop(ctx, terrain, name, spot, toward(spot, center), Some(vec3(2.3, 1.6, 2.0)));
    }
    // Holzstapel, Karren, Kisten, Fässer
    let pile = at(0.8, -4.6);
    prop(ctx, terrain, "holzstapel", pile, facing(forward), Some(vec3(1.2, 0.8, 1.2)));
    let cart = at(-0.6, 4.8);
    prop(ctx, terrain, "karren", cart, facing(-forward * 0.3 + side), Some(vec3(1.8, 1.0, 1.4)));
    for (name, f, s, size) in [("kiste", 6.2, 1.2, 0.6), ("kiste", 6.6, 0.4, 0.6), ("fass", 6.0, -0.6, 0.55), ("fass", 2.6, 5.9, 0.55)] {
        prop(ctx, terrain, name, at(f, s), Quat::from_rotation_y(f * 1.7 + s), Some(Vec3::splat(size)));
    }

    // Wegweiser am Startpunkt: ein Brett je großem Ort, jeweils in dessen Richtung gedreht
    let post = start + side * 2.4 - forward * 0.6;
    if prop(ctx, terrain, "wegweiser", post, Quat::IDENTITY, Some(vec3(0.2, 2.2, 0.2))).is_some() {
        let ground = terrain.height_at(post.x, post.y);
        for (i, (name, target)) in landmarks.iter().enumerate() {
            let direction = (*target - post).normalize_or(Vec2::X);
            let height = 1.76 - i as f32 * 0.28;
            let Some((mesh, _)) = asset_files::load_variants(ctx, "gebaeude", "schild", Vec3::ONE, 0.0).first().copied() else { break };
            let transform = Transform::from_position(vec3(post.x, ground + height - 0.09, post.y)).with_rotation(facing(direction));
            ctx.scene.spawn(Entity::new(format!("Schild {name}"), mesh).with_transform(transform));
            let tip = post + direction * 0.55;
            places.signs.push((vec3(tip.x, ground + height + 0.05, tip.y), name));
        }
    }
}

// ---------------------------------------------------------------------------
// Burg Grünfels (Modelle aus art/lib/burganlage.py und art/lib/marktplatz.py)
// ---------------------------------------------------------------------------

/// Lage des Marktplatzes in der Burganlage (Modellkoordinaten, wie MARKT_ORT in marktplatz.py).
const MARKT_VERSATZ: Vec3 = vec3(-32.25, 0.0, 46.5);

/// Stellt Burganlage und Marktplatz auf das Plateau des Tafelbergs. Die Kollision kommt aus den
/// Modellen selbst: Mauern, Tor, Treppen, Wehrgang, Stände – alles ist fest und begehbar.
/// Fackeln und Laternen werfen nachts Licht.
pub fn build_castle(ctx: &mut Context, places: &mut Places) {
    use crate::island::{burg_drehung, BURG_HOEHE, BURG_ORT};
    let rotation = burg_drehung();
    let origin = vec3(BURG_ORT.x, BURG_HOEHE, BURG_ORT.y);
    let welt = |p: Vec3| origin + rotation * p;
    for (name, offset) in [("burganlage", Vec3::ZERO), ("marktplatz", MARKT_VERSATZ)] {
        let Some(&(mesh, glow)) = asset_files::load_variants(ctx, "bauwerke", name, Vec3::ONE, 0.0).first() else {
            log::warn!("Bauwerk {name} fehlt");
            continue;
        };
        let transform = Transform::from_position(welt(offset)).with_rotation(rotation);
        let entity = ctx.scene.spawn(Entity::new(name.to_string(), mesh).with_transform(transform));
        if let Some(glow) = glow {
            ctx.scene.spawn(Entity::new(format!("{name} (leuchtet)"), glow).with_transform(transform).with_material(Material::Emissive { glow: 1.2 }));
        }
        let (vertices, triangles) = collision_mesh(ctx.assets.mesh(mesh), &transform);
        ctx.physics.add_static_mesh(Some(entity), vertices, triangles);
        // Aus der Ferne vereinfacht; der Markt (viel Kleinkram) verschwindet ganz
        let stufen = if name == "marktplatz" { vec![(140.0, Some(0.5)), (420.0, None)] } else { vec![(160.0, Some(0.8)), (420.0, Some(2.0))] };
        fernstufen(ctx, mesh, glow, &stufen);
    }
    // Namen auf der Karte (die Burg selbst steht bei den großen Orten)
    places.labels.push(("Burgtor", crate::island::burg_welt(vec2(0.0, 75.0))));
    places.labels.push(("Marktplatz", crate::island::burg_welt(vec2(MARKT_VERSATZ.x, MARKT_VERSATZ.z))));
    places.labels.push(("Thronsaal", crate::island::burg_welt(vec2(0.0, -2.0))));
    // Licht: Fackeln am Tor, Laternen an der Straße und auf dem Markt (Blender-Koordinaten der
    // Burganlage: x, y, Höhe → Modell: x, Höhe, -y)
    let blender = |x: f32, y: f32, z: f32| welt(vec3(x, z, -y));
    for s in [-1.0, 1.0] {
        for y in [-79.95, -71.05] {
            places.lights.push((blender(s * 3.8, y, 4.3), vec3(2.4, 1.3, 0.45), 9.0));
        }
        for y in [-87.0, -69.0, -36.0, -24.0] {
            places.lights.push((blender(s * 4.4, y, 3.4), vec3(2.2, 1.35, 0.55), 8.0));
        }
    }
    for (x, y) in [(-14.0, -20.0), (14.0, -20.5), (22.0, 10.0), (-14.0, 21.5), (22.0, -12.0), (0.0, 4.5)] {
        places.lights.push((blender(x - 32.25, y - 46.5, 3.4), vec3(2.2, 1.35, 0.55), 8.0));
    }
    // Im Schloss: drei Kronleuchter, Kamin, Feuerschalen am Thron (siehe innenraum in burg.py)
    for x in [-12.25, 0.0, 12.25] {
        places.lights.push((blender(x, 0.0, 14.0), vec3(2.4, 1.7, 0.9), 16.0));
    }
    places.lights.push((blender(-22.5, 0.0, 2.2), vec3(3.0, 1.5, 0.5), 12.0));
    for s in [-1.0, 1.0] {
        places.lights.push((blender(s * 3.6, 8.8, 2.6), vec3(2.6, 1.4, 0.5), 9.0));
    }
}

/// Detailstufen für große Bauwerke: ab der Entfernung vereinfacht (Zellgröße in Metern) oder,
/// bei `None`, gar nicht mehr gezeichnet. Leuchtende Teile verschwinden bei der doppelten
/// Entfernung der ersten Stufe. Nur mit Fenster (der Server zeichnet nichts).
fn fernstufen(ctx: &mut Context, mesh: MeshId, glow: Option<MeshId>, stufen: &[(f32, Option<f32>)]) {
    if ctx.is_headless() || ctx.assets.has_lods(mesh) {
        return;
    }
    let lods = stufen
        .iter()
        .map(|&(distance, zelle)| Lod {
            distance,
            mesh: zelle.map(|zelle| {
                let grob = ctx.assets.mesh(mesh).simplified(zelle);
                ctx.assets.add_mesh(grob)
            }),
        })
        .collect();
    ctx.assets.set_lods(mesh, lods);
    if let (Some(glow), Some(&(erste, _))) = (glow, stufen.first()) {
        ctx.assets.set_lods(glow, vec![Lod { distance: erste * 2.0, mesh: None }]);
    }
}

/// Dreiecke eines Modells in Weltkoordinaten, gleiche Eckpunkte zusammengefasst (flach
/// schattierte Modelle haben jede Ecke mehrfach).
fn collision_mesh(mesh: &MeshData, transform: &Transform) -> (Vec<Vec3>, Vec<[u32; 3]>) {
    let matrix = transform.matrix();
    let mut index = std::collections::HashMap::new();
    let mut vertices = Vec::new();
    let mut remap = Vec::with_capacity(mesh.vertices.len());
    for v in &mesh.vertices {
        let key = v.position.map(f32::to_bits);
        let i = *index.entry(key).or_insert_with(|| {
            vertices.push(matrix.transform_point3(Vec3::from(v.position)));
            vertices.len() as u32 - 1
        });
        remap.push(i);
    }
    let triangles = mesh
        .indices
        .chunks_exact(3)
        .map(|t| [remap[t[0] as usize], remap[t[1] as usize], remap[t[2] as usize]])
        .filter(|t| t[0] != t[1] && t[1] != t[2] && t[0] != t[2])
        .collect();
    (vertices, triangles)
}

// ---------------------------------------------------------------------------
// Schattenfestung (Modell aus art/lib/festung.py) in der Inselmitte
// ---------------------------------------------------------------------------

/// Stellt die Schattenfestung in die Inselmitte (Tor nach Süden), mit Kollision aus dem Modell
/// und Licht von Geisterfeuern und Runenkreis.
pub fn build_festung(ctx: &mut Context, places: &mut Places) {
    use crate::island::festung_hoehe;
    let origin = vec3(0.0, festung_hoehe(), 0.0);
    log::info!("Schattenfestung auf {:.1} m Höhe", origin.y);
    let Some(&(mesh, glow)) = asset_files::load_variants(ctx, "bauwerke", "schattenfestung", Vec3::ONE, 0.0).first() else {
        log::warn!("Bauwerk schattenfestung fehlt");
        return;
    };
    let transform = Transform::from_position(origin);
    let entity = ctx.scene.spawn(Entity::new("Schattenfestung", mesh).with_transform(transform));
    if let Some(glow) = glow {
        ctx.scene.spawn(Entity::new("Schattenfestung (leuchtet)", glow).with_transform(transform).with_material(Material::Emissive { glow: 1.1 }));
    }
    let (vertices, triangles) = collision_mesh(ctx.assets.mesh(mesh), &transform);
    ctx.physics.add_static_mesh(Some(entity), vertices, triangles);
    fernstufen(ctx, mesh, None, &[(150.0, Some(0.8)), (420.0, Some(2.0))]);
    // Blender-Koordinaten der Festung (x, y, Höhe) → Welt
    let blender = |x: f32, y: f32, z: f32| origin + vec3(x, z, -y);
    let gruen = vec3(0.7, 2.6, 1.1);
    let violett = vec3(1.8, 0.6, 2.8);
    // Geisterfeuer an den vier Toren und Rampen (für das Südtor berechnet, dann gedreht)
    for grad in [0.0f32, 90.0, 180.0, 270.0] {
        let (sin, cos) = grad.to_radians().sin_cos();
        let drehen = |x: f32, y: f32| (x * cos - y * sin, x * sin + y * cos);
        for s in [-1.0, 1.0] {
            for (x, y, z) in [(s * 3.9, -30.97, 12.6), (s * 2.7, -59.07, 4.7), (s * 2.7, -42.87, 9.65)] {
                let (x, y) = drehen(x, y);
                places.lights.push((blender(x, y, z), gruen, 9.0));
            }
        }
    }
    for (x, y) in [(-4.0, -16.0), (4.0, -16.0), (6.5, 6.5), (-6.5, 6.5), (16.0, 4.0), (16.0, -4.0), (-16.0, 4.0), (-16.0, -4.0)] {
        places.lights.push((blender(x, y, 12.5), gruen, 10.0));
    }
    places.labels.push(("Festungstor", vec2(0.0, 30.0)));
    places.lights.push((blender(11.0, -11.0, 12.0), violett, 12.0));
}

// ---------------------------------------------------------------------------
// Sehenswürdigkeiten (Modelle aus art/lib/sehenswert.py)
// ---------------------------------------------------------------------------

/// Maße wie in art/lib/sehenswert.py (für die Kollisionen).
const TOWER_RADIUS: f32 = 2.4;
const TOWER_WALL: f32 = 0.55;
const CIRCLE_RADIUS: f32 = 5.0;
const CIRCLE_STONES: usize = 9;
const PIER_LENGTH: f32 = 7.0;
const PIER_HEIGHT: f32 = 0.45;

/// Wo die Sehenswürdigkeiten stehen (vom Inselaufbau gesucht) – Punkt und Blickrichtung (x, z).
pub struct SightSpots {
    pub tower: Option<(Vec2, Vec2)>,
    pub circle: Option<Vec2>,
    /// Ufer am Bergsee: Punkt am Wasser und Richtung zur Seemitte, dazu der Wasserspiegel
    pub lake_shore: Option<(Vec2, Vec2, f32)>,
    pub wreck: Option<(Vec2, Vec2)>,
    pub lighthouse: Option<(Vec2, Vec2)>,
    pub cave: Option<(Vec2, Vec2)>,
    /// Mühlenhof: Mitte (Windmühle) und Richtung zum Lager
    pub farm: Option<(Vec2, Vec2)>,
    /// Tempelruine: Mitte und Richtung der Treppe
    pub ruin: Option<(Vec2, Vec2)>,
}

/// Fester Kasten relativ zu einem aufgestellten Modell (`offset` und `yaw` in dessen Achsen).
fn solid(ctx: &mut Context, entity: EntityId, base: Vec3, rotation: Quat, offset: Vec3, size: Vec3, yaw: f32) {
    let transform = Transform::from_position(base + rotation * offset).with_rotation(rotation * Quat::from_rotation_y(yaw));
    ctx.physics.add_body(entity, &transform, BodyDesc::fixed(Shape::Box { size }));
}

fn ground(terrain: &Terrain, at: Vec2) -> Vec3 {
    vec3(at.x, terrain.height_at(at.x, at.y), at.y)
}

/// Stellt alle gefundenen Sehenswürdigkeiten auf. Liefert Sperrzonen (Mitte, Radius), in denen
/// keine Bäume und kein Kleinkram wachsen sollen.
pub fn build_sights(ctx: &mut Context, terrain: &Terrain, spots: &SightSpots, places: &mut Places) -> Vec<(Vec2, f32)> {
    let mut blocked = Vec::new();

    if let Some((at, look)) = spots.tower {
        let rotation = facing(look);
        if let Some(entity) = prop(ctx, terrain, "wachturm", at, rotation, None) {
            let base = ground(terrain, at);
            // Mauer als Ring aus Kästen, das Tor (+X) bleibt frei
            for k in 0..12 {
                let w = std::f32::consts::TAU * k as f32 / 12.0;
                if w.cos() > 0.92 {
                    continue;
                }
                let offset = vec3(w.cos() * TOWER_RADIUS, 1.8, -w.sin() * TOWER_RADIUS);
                solid(ctx, entity, base, rotation, offset, vec3(TOWER_WALL, 3.6, 1.3), w);
            }
            places.labels.push(("Alter Wachturm", at));
            blocked.push((at, 8.0));
        }
    }
    if let Some(at) = spots.circle {
        if let Some(entity) = prop(ctx, terrain, "steinkreis", at, Quat::IDENTITY, None) {
            let base = ground(terrain, at);
            for i in 0..CIRCLE_STONES {
                let w = std::f32::consts::TAU * i as f32 / CIRCLE_STONES as f32;
                let offset = vec3(w.cos() * CIRCLE_RADIUS, 1.3, -w.sin() * CIRCLE_RADIUS);
                solid(ctx, entity, base, Quat::IDENTITY, offset, vec3(0.6, 2.6, 1.0), w);
            }
            solid(ctx, entity, base, Quat::IDENTITY, Vec3::Y * 0.3, vec3(1.5, 0.6, 1.2), 0.0);
            places.labels.push(("Steinkreis", at));
            places.lights.push((base + Vec3::Y * 1.0, vec3(0.4, 1.6, 2.2), 7.0));
            blocked.push((at, 8.0));
        }
    }
    if let Some((shore, inward, level)) = spots.lake_shore {
        let side = inward.perp();
        // Steg vom Ufer auf den See hinaus
        let pier_start = shore - inward * 1.2;
        let rotation = facing(inward);
        let pier_base = vec3(pier_start.x, level, pier_start.y);
        if let Some((mesh, _)) = asset_files::load_variants(ctx, "gebaeude", "steg", Vec3::ONE, 0.0).first().copied() {
            let transform = Transform::from_position(pier_base - Vec3::Y * 0.02).with_rotation(rotation);
            let entity = ctx.scene.spawn(Entity::new("Steg", mesh).with_transform(transform));
            solid(ctx, entity, pier_base, rotation, vec3(PIER_LENGTH / 2.0, PIER_HEIGHT - 0.05, 0.0), vec3(PIER_LENGTH, 0.12, 1.3), 0.0);
        }
        // Ruderboot neben dem Steg, schaukelt auf dem Wasser
        let boat_at = pier_start + inward * 4.6 + side * 1.5;
        if let Some((mesh, _)) = asset_files::load_variants(ctx, "gebaeude", "ruderboot", Vec3::ONE, 0.0).first().copied() {
            let base = vec3(boat_at.x, level - 0.18, boat_at.y);
            let rotation = facing(inward) * Quat::from_rotation_y(0.12);
            let entity = ctx.scene.spawn(Entity::new("Ruderboot", mesh).with_transform(Transform::from_position(base).with_rotation(rotation)));
            places.boats.push((entity, base, rotation));
        }
        // Schrein ein paar Schritte landeinwärts, Blick auf den See
        let shrine = shore - inward * 6.0 - side * 3.5;
        if let Some(entity) = prop(ctx, terrain, "schrein", shrine, facing(inward), None) {
            let base = ground(terrain, shrine);
            solid(ctx, entity, base, facing(inward), Vec3::Y * 0.3, vec3(3.2, 0.6, 3.2), 0.0);
            for (x, z) in [(-0.95, -0.95), (0.95, -0.95), (-0.95, 0.95), (0.95, 0.95)] {
                solid(ctx, entity, base, facing(inward), vec3(x, 1.7, z), vec3(0.35, 2.3, 0.35), 0.0);
            }
            places.lights.push((base + Vec3::Y * 1.5, vec3(2.4, 1.5, 0.6), 7.0));
            places.labels.push(("Seeschrein", shrine));
            blocked.push((shrine, 5.5));
        }
        blocked.push((shore, 4.0));
    }
    if let Some((at, along)) = spots.wreck {
        let rotation = facing(along);
        if let Some(entity) = prop(ctx, terrain, "schiffswrack", at, rotation, None) {
            let base = ground(terrain, at);
            solid(ctx, entity, base, rotation, Vec3::Y * 0.7, vec3(8.0, 1.4, 2.6), 0.0);
            places.labels.push(("Schiffswrack", at));
            blocked.push((at, 7.0));
        }
    }
    if let Some((at, sea)) = spots.lighthouse {
        let rotation = facing(-sea);
        if let Some(entity) = prop(ctx, terrain, "leuchtturm", at, rotation, None) {
            let base = ground(terrain, at);
            solid(ctx, entity, base, rotation, Vec3::Y * 5.0, vec3(2.7, 10.0, 2.7), 0.0);
            places.lights.push((base + Vec3::Y * 10.3, vec3(3.0, 2.4, 1.2), 16.0));
            places.labels.push(("Leuchtturm", at));
            blocked.push((at, 5.0));
        }
    }
    if let Some((at, look)) = spots.farm {
        let rotation = facing(look);
        let side = look.perp();
        if let Some(entity) = prop(ctx, terrain, "windmuehle", at, rotation, None) {
            let base = ground(terrain, at);
            solid(ctx, entity, base, rotation, Vec3::Y * 4.0, vec3(4.6, 8.0, 4.6), 0.0);
            if let Some((mesh, _)) = asset_files::load_variants(ctx, "teile", "windmuehle_fluegel", Vec3::ONE, 0.0).first().copied() {
                // Nabe vorne oben an der Kappe (Blender 2,05 / 0 / 8,35)
                let hub = base + rotation * vec3(2.05, 8.35, 0.0);
                let sails = ctx.scene.spawn(Entity::new("Windmühlenflügel", mesh).with_transform(Transform::from_position(hub).with_rotation(rotation)));
                places.windmills.push((sails, hub, rotation));
            }
            places.labels.push(("Mühlenhof", at));
            // Rund um die Mühle frei von Bäumen und hohem Gras
            blocked.push((at, 13.0));
        }
        // Brunnen, Bienenstöcke, Weizenfeld mit Vogelscheuche rund um die Mühle
        let spots_around: [(&str, Vec2, Vec2, Option<Vec3>, f32); 4] = [
            ("brunnen", at + side * 9.0 + look * 4.0, look, Some(vec3(2.4, 1.0, 2.4)), 3.0),
            ("bienenstoecke", at - side * 8.0 + look * 5.0, side, None, 3.0),
            ("weizenfeld", at - look * 11.0 + side * 3.0, side, None, 7.0),
            ("vogelscheuche", at - look * 11.5 + side * 2.0, look, None, 1.0),
        ];
        for (name, spot, dir, collider, radius) in spots_around {
            if prop(ctx, terrain, name, spot, facing(dir), collider).is_some() {
                blocked.push((spot, radius));
            }
        }
        places.lights.push((ground(terrain, at) + rotation * vec3(2.6, 2.4, 0.0), vec3(2.2, 1.5, 0.7), 8.0));
    }
    if let Some((at, look)) = spots.ruin {
        let rotation = facing(look);
        if let Some(entity) = prop(ctx, terrain, "tempelruine", at, rotation, None) {
            // Die Plattform sitzt tiefer im Boden, damit man bequem hinaufgeht
            ctx.scene.get_mut(entity).transform.position.y -= 0.35;
            let base = ground(terrain, at) - Vec3::Y * 0.35;
            // Säulen (Blender x/y → Spiel x/-z) und der Sockel mit dem Runenstein
            for y in [-3.0f32, 3.0] {
                for x in [-4.4f32, -2.2, 0.0, 2.2, 4.4] {
                    solid(ctx, entity, base, rotation, vec3(x, 1.6, -y), vec3(0.75, 2.4, 0.75), 0.0);
                }
            }
            solid(ctx, entity, base, rotation, vec3(0.0, 1.0, 0.0), vec3(1.4, 1.3, 1.4), 0.0);
            places.lights.push((base + Vec3::Y * 2.2, vec3(0.5, 1.6, 2.4), 9.0));
            places.labels.push(("Tempelruine", at));
            blocked.push((at, 9.0));
        }
    }
    if let Some((at, out)) = spots.cave {
        let rotation = facing(out);
        if let Some(entity) = prop(ctx, terrain, "hoehle", at, rotation, None) {
            let base = ground(terrain, at);
            for side in [-1.0, 1.0] {
                solid(ctx, entity, base, rotation, vec3(-0.5, 2.0, side * 2.8), vec3(3.0, 4.0, 1.6), 0.0);
            }
            solid(ctx, entity, base, rotation, vec3(-1.8, 2.0, 0.0), vec3(1.2, 4.0, 3.6), 0.0);
            places.labels.push(("Höhle", at));
            blocked.push((at, 6.0));
        }
    }
    blocked
}
