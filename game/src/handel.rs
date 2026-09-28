//! Der Händler auf dem Marktplatz der Burg: steht vor dem Waffenladen, kauft Beute, Rohstoffe und
//! Ausrüstung für Gold an und verkauft ein Tagesangebot – Waffen und Rüstung für die eigene Figur
//! (bis Episch; Legendäres gibt es nur als Beute) und Baumaterial.
//!
//! Der Server prüft jeden Handel (Entfernung, Besitz, Gold, Angebot des Tages); die Figur selbst
//! ist reine Kulisse und steht bei allen an derselben Stelle. Modell: `game/assets/npc/haendler.gltf`
//! (Blender: art/lib/haendler.py).

use std::sync::Arc;

use engine::prelude::*;
use serde::{Deserialize, Serialize};

use crate::protocol::{CharacterClass, Inventory, Item};
use crate::waffen::Seltenheit;

/// So nah muss man am Händler stehen (m, waagerecht).
pub const HANDEL_WEITE: f32 = 4.0;
/// Wie viele Waffen und Rüstungsteile täglich im Angebot sind.
pub const ANGEBOT_GROESSE: usize = 4;

/// Was gehandelt wird.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ware {
    Waffe(u8),
    Ruestung(u8),
    Gegenstand(Item),
}

/// Ein Handel mit dem Händler: kaufen bzw. verkaufen (Anzahl nur bei Gegenständen).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HandelBefehl {
    Kaufen(Ware, u32),
    Verkaufen(Ware, u32),
}

/// Standort vor dem Waffenladen (Blender-Koordinaten der Burganlage: x, y) und Blickrichtung
/// (zum Platz, Blender +X).
const STANDORT: (f32, f32) = (-48.25, -37.9);

/// Wo der Händler steht (Füße, Welt).
pub fn ort() -> Vec3 {
    let p = crate::island::burg_welt(vec2(STANDORT.0, -STANDORT.1));
    vec3(p.x, crate::island::BURG_HOEHE + 0.08, p.y)
}

/// Blickrichtung zum Platz (Welt x, z).
fn blick_zum_platz() -> Vec2 {
    (crate::island::burg_welt(vec2(STANDORT.0 + 1.0, -STANDORT.1)) - crate::island::burg_welt(vec2(STANDORT.0, -STANDORT.1))).normalize_or(Vec2::Y)
}

/// Steht jemand an `p` (Füße oder Körpermitte) nah genug zum Handeln?
pub fn in_reichweite(p: Vec3) -> bool {
    let o = ort();
    vec2(p.x, p.z).distance(vec2(o.x, o.z)) <= HANDEL_WEITE && (p.y - o.y).abs() < 3.0
}

/// Was der Händler für ein Stück zahlt (None = kauft er nicht an).
pub fn ankauf(item: Item) -> Option<u32> {
    match item {
        Item::Wood | Item::Stone => Some(1),
        Item::Ore | Item::Wool => Some(3),
        Item::Meat => Some(2),
        Item::Pelt => Some(4),
        Item::Runenfragment => Some(35),
        Item::Gold | Item::Runenstein => None,
    }
}

/// Was ein Stück beim Händler kostet (None = hat er nicht).
pub fn verkauf(item: Item) -> Option<u32> {
    match item {
        Item::Wood | Item::Stone => Some(4),
        Item::Ore => Some(10),
        _ => None,
    }
}

/// Rohstoffe, die der Händler immer vorrätig hat.
pub const ROHSTOFFE: [Item; 3] = [Item::Wood, Item::Stone, Item::Ore];

/// Wert einer Waffe oder eines Rüstungsteils in Gold (Kaufpreis beim Händler).
pub fn wert(ware: Ware) -> Option<u32> {
    match ware {
        Ware::Waffe(id) => crate::waffen::waffe(id).map(|w| w.wert()),
        Ware::Ruestung(id) => crate::ruestung::ruestung(id).map(|r| r.wert()),
        Ware::Gegenstand(item) => verkauf(item),
    }
}

/// Was der Händler für eine Waffe oder ein Rüstungsteil zahlt: die Hälfte des Werts
/// (mehr als das Einschmelzen doppelter Beute).
pub fn ankauf_ausruestung(ware: Ware) -> Option<u32> {
    match ware {
        Ware::Gegenstand(item) => ankauf(item),
        _ => wert(ware).map(|w| (w / 2 / 5 * 5).max(5)),
    }
}

/// Das Angebot eines Tages für eine Figur: vier Waffen und Rüstungsteile bis Episch, jeden Tag
/// anders gemischt (für alle Spieler derselben Klasse gleich).
pub fn angebot(tag: u32, klasse: CharacterClass) -> Vec<Ware> {
    let waffen = crate::waffen::WAFFEN.iter().filter(|w| w.klasse == klasse && w.seltenheit < Seltenheit::Legendaer).map(|w| Ware::Waffe(w.id));
    let teile = crate::ruestung::RUESTUNGEN.iter().filter(|r| r.klasse == klasse && r.seltenheit < Seltenheit::Legendaer).map(|r| Ware::Ruestung(r.id));
    let mut alle: Vec<(u32, Ware)> = waffen
        .chain(teile)
        .map(|ware| {
            let id = match ware {
                Ware::Waffe(id) => id as u32,
                Ware::Ruestung(id) => 100 + id as u32,
                Ware::Gegenstand(_) => 0,
            };
            (mischen(tag.wrapping_mul(7919).wrapping_add(id)), ware)
        })
        .collect();
    alle.sort_by_key(|&(h, _)| h);
    let mut auswahl: Vec<Ware> = alle.into_iter().take(ANGEBOT_GROESSE).map(|(_, w)| w).collect();
    auswahl.sort_by_key(|w| match *w {
        Ware::Waffe(id) => (0, id),
        Ware::Ruestung(id) => (1, crate::ruestung::ruestung(id).map_or(0, |r| r.platz.index() as u8)),
        Ware::Gegenstand(_) => (2, 0),
    });
    auswahl
}

/// Kleiner Ganzzahl-Mischer (für ein gleichbleibendes Tagesangebot ohne Zufallsquelle).
fn mischen(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

fn name(ware: Ware) -> String {
    match ware {
        Ware::Waffe(id) => crate::waffen::waffe(id).map_or("Waffe", |w| w.name).to_string(),
        Ware::Ruestung(id) => crate::ruestung::ruestung(id).map_or("Rüstung", |r| r.name).to_string(),
        Ware::Gegenstand(item) => item.label().to_string(),
    }
}

/// Führt einen Handel am Inventar aus (Entfernung prüft der Aufrufer). Liefert die Meldung für
/// den Spieler oder den Grund, warum es nicht geht.
pub fn handeln(inventory: &mut Inventory, befehl: HandelBefehl, klasse: CharacterClass, tag: u32) -> Result<String, String> {
    match befehl {
        HandelBefehl::Kaufen(ware @ Ware::Gegenstand(item), anzahl) => {
            let anzahl = anzahl.clamp(1, 100);
            let preis = verkauf(item).ok_or_else(|| format!("{} hat der Händler nicht", item.label()))? * anzahl;
            if inventory.gold < preis {
                return Err(format!("Zu wenig Gold: {anzahl} {} kosten {preis} Gold", name(ware)));
            }
            inventory.gold -= preis;
            inventory.add_item(item, anzahl);
            Ok(format!("Gekauft: {anzahl} {} für {preis} Gold", name(ware)))
        }
        HandelBefehl::Kaufen(ware, _) => {
            if !angebot(tag, klasse).contains(&ware) {
                return Err(format!("{} hat der Händler heute nicht im Angebot", name(ware)));
            }
            let (bits, bit) = match ware {
                Ware::Waffe(id) => (&mut inventory.waffen, 1u32 << id),
                Ware::Ruestung(id) => (&mut inventory.ruestungen, 1u32 << id),
                Ware::Gegenstand(_) => unreachable!(),
            };
            if *bits & bit != 0 {
                return Err(format!("{} hast du schon", name(ware)));
            }
            let preis = wert(ware).ok_or("Diese Ware gibt es nicht")?;
            if inventory.gold < preis {
                return Err(format!("Zu wenig Gold: {} kostet {preis} Gold", name(ware)));
            }
            inventory.gold -= preis;
            match ware {
                Ware::Waffe(_) => inventory.waffen |= bit,
                _ => inventory.ruestungen |= bit,
            }
            Ok(format!("Gekauft: {} für {preis} Gold", name(ware)))
        }
        HandelBefehl::Verkaufen(ware @ Ware::Gegenstand(item), anzahl) => {
            let preis = ankauf(item).ok_or_else(|| format!("{} kauft der Händler nicht", item.label()))?;
            let anzahl = anzahl.min(inventory.count(item));
            if anzahl == 0 || !inventory.remove_item(item, anzahl) {
                return Err(format!("Du hast kein {}", name(ware)));
            }
            inventory.gold += preis * anzahl;
            Ok(format!("Verkauft: {anzahl} {} für {} Gold", name(ware), preis * anzahl))
        }
        HandelBefehl::Verkaufen(ware, _) => {
            let preis = ankauf_ausruestung(ware).ok_or("Diese Ware gibt es nicht")?;
            match ware {
                Ware::Waffe(id) => {
                    if inventory.waffen & (1u32 << id) == 0 {
                        return Err(format!("{} hast du nicht", name(ware)));
                    }
                    if inventory.waffe == id {
                        return Err(format!("{} hast du in der Hand – leg sie erst ab", name(ware)));
                    }
                    inventory.waffen &= !(1u32 << id);
                }
                Ware::Ruestung(id) => {
                    if inventory.ruestungen & (1u32 << id) == 0 {
                        return Err(format!("{} hast du nicht", name(ware)));
                    }
                    if inventory.ruestung.contains(&id) {
                        return Err(format!("{} trägst du – leg es erst ab", name(ware)));
                    }
                    inventory.ruestungen &= !(1u32 << id);
                }
                Ware::Gegenstand(_) => unreachable!(),
            }
            inventory.gold += preis;
            Ok(format!("Verkauft: {} für {preis} Gold", name(ware)))
        }
    }
}

// ---------------------------------------------------------------------------
// Die Figur (nur mit Fenster)
// ---------------------------------------------------------------------------

/// Bis hierhin dreht er sich zu Kunden um (m).
const KUNDEN_WEITE: f32 = 9.0;
/// So oft grüßt er höchstens (s).
const GRUSS_PAUSE: f32 = 25.0;

pub struct HaendlerFigur {
    entity: EntityId,
    animator: Animator,
    /// Blickrichtung (Winkel um Y) und seit wann er nicht mehr gegrüßt hat
    blick: f32,
    seit_gruss: f32,
    /// Wer zuletzt in Grußweite war (damit er nur beim Herankommen grüßt)
    kunde_nah: bool,
}

impl HaendlerFigur {
    pub fn new(ctx: &mut Context) -> Option<HaendlerFigur> {
        let path = crate::asset_files::variants("npc", "haendler").into_iter().next()?;
        let model = match Model::from_file(&path) {
            Ok(model) => Arc::new(model),
            Err(message) => {
                log::warn!("Händler: {message}");
                return None;
            }
        };
        let textures = model.register_textures(&mut ctx.assets);
        let texture = textures.first().copied();
        let mesh = ctx.assets.named_mesh("haendler_gpu", || model.skinned_gpu_mesh(texture));
        let mut animator = Animator::new(model);
        animator.play("Idle", true, 0.0);
        let blick = winkel(blick_zum_platz());
        let mut entity = Entity::new("Händler", mesh).with_transform(Transform::from_position(ort()).with_rotation(Quat::from_rotation_y(blick)));
        entity.joints = animator.palette();
        let entity = ctx.scene.spawn(entity);
        Some(HaendlerFigur { entity, animator, blick, seit_gruss: GRUSS_PAUSE, kunde_nah: false })
    }

    /// `spieler`: wo die Spieler gerade stehen. Er schaut zum nächsten Kunden und grüßt, wenn
    /// jemand herankommt.
    pub fn update(&mut self, ctx: &mut Context, spieler: &[Vec3]) {
        let dt = ctx.time.delta;
        let o = ort();
        if o.distance(ctx.camera.position) > 90.0 {
            return;
        }
        let naechster = spieler
            .iter()
            .map(|p| (vec2(p.x - o.x, p.z - o.z), (p.y - o.y).abs()))
            .filter(|(d, h)| d.length() < KUNDEN_WEITE && *h < 4.0)
            .min_by(|a, b| a.0.length().total_cmp(&b.0.length()))
            .map(|(d, _)| d);
        // Nicht ganz herumdrehen: höchstens 80° von der Ladenfront weg
        let grund = winkel(blick_zum_platz());
        let ziel = naechster.map_or(grund, |d| grund + wrap(winkel(d) - grund).clamp(-1.4, 1.4));
        self.blick += wrap(ziel - self.blick) * (1.0 - (-dt * 3.0).exp());
        self.seit_gruss += dt;
        let nah = naechster.is_some_and(|d| d.length() < HANDEL_WEITE + 1.5);
        if nah && !self.kunde_nah && self.seit_gruss > GRUSS_PAUSE {
            self.animator.play("Gruessen", false, 0.2);
            self.seit_gruss = 0.0;
        }
        self.kunde_nah = nah;
        if self.animator.current() != Some("Idle") && self.animator.finished() {
            self.animator.play("Idle", true, 0.3);
        }
        self.animator.update(dt);
        if let Some(entity) = ctx.scene.try_get_mut(self.entity) {
            entity.joints = self.animator.palette();
            entity.transform.rotation = Quat::from_rotation_y(self.blick);
        }
    }
}

/// Drehwinkel um Y für eine Blickrichtung (Welt x, z); das Modell schaut nach +Z.
fn winkel(richtung: Vec2) -> f32 {
    richtung.x.atan2(richtung.y)
}

fn wrap(w: f32) -> f32 {
    (w + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reich() -> Inventory {
        Inventory { gold: 5000, wood: 20, pelt: 3, ..Default::default() }
    }

    #[test]
    fn das_angebot_wechselt_taeglich_und_passt_zur_figur() {
        for klasse in CharacterClass::ALL {
            let heute = angebot(3, klasse);
            assert_eq!(heute.len(), ANGEBOT_GROESSE);
            assert_eq!(heute, angebot(3, klasse), "gleicher Tag, gleiches Angebot");
            for ware in &heute {
                let (k, s) = match *ware {
                    Ware::Waffe(id) => crate::waffen::waffe(id).map(|w| (w.klasse, w.seltenheit)).unwrap(),
                    Ware::Ruestung(id) => crate::ruestung::ruestung(id).map(|r| (r.klasse, r.seltenheit)).unwrap(),
                    Ware::Gegenstand(_) => panic!("keine Gegenstände im Tagesangebot"),
                };
                assert_eq!(k, klasse);
                assert!(s < Seltenheit::Legendaer, "Legendäres gibt es nur als Beute");
            }
            assert!((0..10).any(|tag| angebot(tag, klasse) != heute), "das Angebot wechselt nie");
        }
    }

    #[test]
    fn kaufen_und_verkaufen() {
        let klasse = CharacterClass::Zwerg;
        let ware = angebot(1, klasse)[0];
        let mut inv = reich();
        let preis = wert(ware).unwrap();
        handeln(&mut inv, HandelBefehl::Kaufen(ware, 1), klasse, 1).unwrap();
        assert_eq!(inv.gold, 5000 - preis);
        assert!(handeln(&mut inv, HandelBefehl::Kaufen(ware, 1), klasse, 1).is_err(), "doppelt gekauft");
        // Wieder verkaufen bringt die Hälfte
        handeln(&mut inv, HandelBefehl::Verkaufen(ware, 1), klasse, 1).unwrap();
        assert_eq!(inv.gold, 5000 - preis + ankauf_ausruestung(ware).unwrap());
        assert!(ankauf_ausruestung(ware).unwrap() * 2 <= preis);
        // Was nicht im Angebot ist, gibt es nicht
        let fremd = crate::waffen::WAFFEN.iter().find(|w| w.klasse == klasse && !angebot(1, klasse).contains(&Ware::Waffe(w.id))).unwrap();
        assert!(handeln(&mut inv, HandelBefehl::Kaufen(Ware::Waffe(fremd.id), 1), klasse, 1).is_err());
        // Rohstoffe
        let gold = inv.gold;
        handeln(&mut inv, HandelBefehl::Verkaufen(Ware::Gegenstand(Item::Wood), 50), klasse, 1).unwrap();
        assert_eq!((inv.wood, inv.gold), (0, gold + 20));
        handeln(&mut inv, HandelBefehl::Kaufen(Ware::Gegenstand(Item::Stone), 10), klasse, 1).unwrap();
        assert_eq!(inv.stone, 10);
        assert!(handeln(&mut inv, HandelBefehl::Verkaufen(Ware::Gegenstand(Item::Runenstein), 1), klasse, 1).is_err());
    }

    #[test]
    fn angelegtes_verkauft_man_nicht_und_ohne_gold_gibt_es_nichts() {
        let klasse = CharacterClass::Mage;
        let mut inv = Inventory { waffen: 1 << 2, waffe: 2, ruestungen: 1 << 1, ruestung: [1, 0, 0], ..Default::default() };
        assert!(handeln(&mut inv, HandelBefehl::Verkaufen(Ware::Waffe(2), 1), klasse, 0).is_err());
        assert!(handeln(&mut inv, HandelBefehl::Verkaufen(Ware::Ruestung(1), 1), klasse, 0).is_err());
        inv.waffe = 0;
        handeln(&mut inv, HandelBefehl::Verkaufen(Ware::Waffe(2), 1), klasse, 0).unwrap();
        assert_eq!(inv.waffen, 0);
        let ware = angebot(0, klasse)[0];
        let mut arm = Inventory::default();
        assert!(handeln(&mut arm, HandelBefehl::Kaufen(ware, 1), klasse, 0).is_err());
        assert_eq!(arm, Inventory::default());
    }
}
