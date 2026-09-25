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
    /// Wasserfall: obere Kante und Fuß (für Gischt, Nebel und Rauschen).
    pub waterfall: Option<(Vec3, Vec3)>,
    /// Brandung: Punkte auf der Wasserlinie flacher Strände und Richtung aufs Land.
    pub surf: Vec<(Vec3, Vec2)>,
    /// Wegelaternen (Fuß am Boden)
    pub lanterns: Vec<Vec3>,
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
