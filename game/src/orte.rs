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
