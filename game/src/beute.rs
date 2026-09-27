//! Beute am Boden: was Gegner und Tiere fallen lassen, liegt sichtbar auf dem Boden (kleines Modell,
//! das sanft schwebt und sich dreht; Waffen und Runenfragmente mit einer Lichtsäule in der Farbe
//! ihrer Seltenheit) und wird mit E aufgehoben. Nach `LIEGT_SEKUNDEN` verschwindet sie.
//! Modelle: `game/assets/gegenstaende/` (Blender: `art/lib/beute.py`).

use std::collections::{BTreeMap, HashMap};

use engine::prelude::*;
use serde::{Deserialize, Serialize};

use crate::protocol::Item;

/// So nah muss man stehen, um etwas aufzuheben (Meter, waagerecht).
pub const AUFHEBEN_WEITE: f32 = 2.8;
/// So lange bleibt Beute liegen.
pub const LIEGT_SEKUNDEN: f32 = 300.0;

/// Was dort liegt.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Fund {
    Gegenstand(Item, u32),
    /// Waffe (ID aus `waffen.rs`)
    Waffe(u8),
    /// Rüstungsteil (ID aus `ruestung.rs`)
    Ruestung(u8),
}

impl Fund {
    pub fn name(&self) -> String {
        match *self {
            Fund::Gegenstand(item, 1) => item.label().to_string(),
            Fund::Gegenstand(item, n) => format!("{n} {}", item.label()),
            Fund::Waffe(id) => crate::waffen::waffe(id).map_or("Waffe".into(), |w| w.name.to_string()),
            Fund::Ruestung(id) => crate::ruestung::ruestung(id).map_or("Rüstung".into(), |r| r.name.to_string()),
        }
    }

    /// Farbe von Name und Lichtsäule (RGB 0..1).
    pub fn farbe(&self) -> [f32; 3] {
        match *self {
            Fund::Waffe(id) => crate::waffen::waffe(id).map_or([1.0; 3], |w| w.seltenheit.farbe()),
            Fund::Ruestung(id) => crate::ruestung::ruestung(id).map_or([1.0; 3], |r| r.seltenheit.farbe()),
            Fund::Gegenstand(Item::Runenfragment | Item::Runenstein, _) => [0.45, 0.8, 1.0],
            Fund::Gegenstand(Item::Gold, _) => [1.0, 0.82, 0.35],
            Fund::Gegenstand(..) => [0.92, 0.9, 0.85],
        }
    }

    /// Modelldatei in `game/assets/gegenstaende/`.
    pub fn modell(&self) -> &'static str {
        match *self {
            Fund::Waffe(id) => crate::waffen::waffe(id).map_or("beute_gold", |w| w.datei),
            Fund::Ruestung(id) => crate::ruestung::ruestung(id).map_or("beute_gold", |r| r.datei),
            Fund::Gegenstand(Item::Gold, _) => "beute_gold",
            Fund::Gegenstand(Item::Runenfragment | Item::Runenstein, _) => "beute_runenfragment",
            Fund::Gegenstand(Item::Meat, _) => "beute_fleisch",
            Fund::Gegenstand(Item::Pelt, _) => "beute_fell",
            Fund::Gegenstand(Item::Wool, _) => "beute_wolle",
            Fund::Gegenstand(..) => "beute_gold",
        }
    }

    /// Wertvolles bekommt eine Lichtsäule.
    fn strahl(&self) -> bool {
        matches!(self, Fund::Waffe(_) | Fund::Ruestung(_) | Fund::Gegenstand(Item::Runenfragment | Item::Runenstein, _))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bodenbeute {
    pub id: u32,
    /// Am Boden
    pub ort: Vec3,
    pub fund: Fund,
}

/// Was von einer Beute zu sehen ist.
struct Teile {
    modell: Vec<EntityId>,
    strahl: Option<EntityId>,
    phase: f32,
}

/// Darstellung der Beute (nur mit Fenster).
#[derive(Default)]
pub struct BeuteAnsicht {
    teile: HashMap<u32, Teile>,
}

impl BeuteAnsicht {
    pub fn update(&mut self, ctx: &mut Context, beute: &BTreeMap<u32, Bodenbeute>) {
        // Weg, was nicht mehr da ist
        let weg: Vec<u32> = self.teile.keys().filter(|id| !beute.contains_key(id)).copied().collect();
        for id in weg {
            if let Some(t) = self.teile.remove(&id) {
                for e in t.modell.into_iter().chain(t.strahl) {
                    ctx.scene.despawn(e);
                }
            }
        }
        let zeit = ctx.time.elapsed;
        for b in beute.values() {
            let teile = self.teile.entry(b.id).or_insert_with(|| {
                let mut modell = Vec::new();
                match crate::asset_files::load_variants(ctx, "gegenstaende", b.fund.modell(), Vec3::ONE, 0.0).first() {
                    Some(&(mesh, glow)) => {
                        modell.push(ctx.scene.spawn(Entity::new("Beute", mesh)));
                        if let Some(glow) = glow {
                            modell.push(ctx.scene.spawn(Entity::new("Beute (leuchtet)", glow).with_material(Material::Emissive { glow: 2.2 })));
                        }
                    }
                    None => {
                        let f = b.fund.farbe();
                        let e = Entity::new("Beute", ctx.assets.cube()).with_transform(Transform::from_position(Vec3::ZERO).with_scale(Vec3::splat(0.3))).with_color(vec4(f[0], f[1], f[2], 1.0));
                        modell.push(ctx.scene.spawn(e));
                    }
                }
                let strahl = b.fund.strahl().then(|| {
                    let f = Vec3::from(b.fund.farbe());
                    let t = Transform::from_position(b.ort + Vec3::Y * 2.2).with_scale(vec3(0.045, 4.4, 0.045));
                    ctx.scene.spawn(Entity::new("Lichtsäule", ctx.assets.cube()).with_transform(t).with_color((f * 0.5).extend(1.0)).with_material(Material::Emissive { glow: 0.9 }))
                });
                Teile { modell, strahl, phase: (b.id % 97) as f32 * 0.37 }
            });
            let schweben = 0.18 + (zeit * 2.0 + teile.phase).sin() * 0.06;
            let drehung = Quat::from_rotation_y(zeit * 0.8 + teile.phase);
            for &e in &teile.modell {
                if let Some(entity) = ctx.scene.try_get_mut(e) {
                    entity.transform.position = b.ort + Vec3::Y * schweben;
                    entity.transform.rotation = drehung;
                }
            }
            if b.fund.strahl() {
                let f = Vec3::from(b.fund.farbe());
                ctx.lights.push(PointLight { position: b.ort + Vec3::Y * 0.8, color: f * 1.6, radius: 4.5 });
            }
        }
    }
}
