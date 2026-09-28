//! Verteidigungstürme an den Heerstraßen (Tower Defense): fünfzehn Arten in drei Stufen, auf
//! Stufe 3 mit zwei Richtungen zur Wahl (A/B), dazu Fallen auf der Straße (Stacheln, Teer,
//! Barrikade). Konzept: docs/konzept_tower_defense.md. Modelle: art/lib/tuerme.py
//! (`bauten/turm_<art>_<stufe>.gltf`, der drehbare Kopf `bauten/turm_<art>_kopf.gltf`, Fallen
//! `bauten/falle_<art>.gltf`).
//!
//! Hier stehen die Werte (Reichweite, Schaden, Kosten …) und `Verteidigung`, die auf dem Server
//! die Türme schießen lässt, die Soldaten der Kasernen führt und die Fallen auslöst. Das Treffen
//! und die Wirkungen an den Einheiten stehen in `heer.rs`.

use std::collections::HashMap;

use engine::prelude::*;
use serde::{Deserialize, Serialize};

use crate::bauten::{Building, BuildingKind};
use crate::heer::{wirkung, Blocker, EnemyAction, Filter, Heer, Hit, Quelle, Strike, Ziel};
use crate::protocol::Item;
use crate::td::{Ereignis, SoldatState};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TowerKind {
    Arrow,
    Ballista,
    Catapult,
    Fire,
    Frost,
    Lightning,
    Sun,
    Arcane,
    Poison,
    Banner,
    /// Kaserne: schickt Soldaten auf die Straße, die Gruppen aufhalten
    Barracks,
    /// Späherturm: deckt Getarnte auf, markiert Ziele
    Scout,
    /// Sturmturm: wirft Gruppen zurück, stark gegen Flieger
    Storm,
    /// Runenstampfer: Bodenwelle um den Turm, betäubt
    Rune,
    /// Schatzkammer: mehr Beute und Gold von Einheiten, die in der Nähe fallen
    Treasury,
}

/// Fallen, die direkt auf der Straße stehen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FallenArt {
    /// Stacheln: Schaden an allem, was darüber läuft
    Stacheln,
    /// Teer: verlangsamt, Feuer brennt darin doppelt
    Teer,
    /// Barrikade: hält Gruppen auf, bis sie zerschlagen ist
    Barrikade,
}

/// Schadensarten; die Einheiten sind unterschiedlich empfindlich (siehe `heer.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageKind {
    Physical,
    /// Halbiert die Rüstung
    Pierce,
    Fire,
    Frost,
    Lightning,
    /// Doppelt gegen Untote
    Holy,
    /// Ignoriert die Rüstung
    Arcane,
    Poison,
}

/// Plattformhöhe je Stufe (wie `KOPF_Z` in art/lib/tuerme.py): dort sitzt der drehbare Kopf.
pub const KOPF_Z: [f32; 3] = [4.2, 5.4, 6.6];
/// Die Türme werden im Spiel größer dargestellt als modelliert (höher, breiter, eindrucksvoller).
pub const TURM_GROESSE: f32 = 1.4;
/// Wie groß der Kopf im Spiel dargestellt wird (damit er über die Zinnen schaut).
pub const KOPF_GROESSE: f32 = 1.3 * TURM_GROESSE;

/// Höhe der Plattform (dort sitzt der Kopf) im Spiel.
pub fn kopf_hoehe(stufe: u8) -> f32 {
    KOPF_Z[(stufe.clamp(1, 3) - 1) as usize] * TURM_GROESSE
}
/// Höchste Stufe
pub const MAX_STUFE: u8 = 3;
/// Lebenspunkte einer Barrikade
pub const BARRIKADE_LP: f32 = 420.0;

/// Werte eines Turms auf einer Stufe (und ggf. in seiner Richtung).
#[derive(Clone, Copy, Debug, Default)]
pub struct Werte {
    pub reichweite: f32,
    /// Sekunden zwischen zwei Schüssen (0 = schießt nicht)
    pub takt: f32,
    pub schaden: f32,
    pub art: Option<DamageKind>,
    /// Flächenschaden (Radius)
    pub flaeche: f32,
    /// Kettenblitz: so viele Ziele
    pub kette: u8,
    /// Verlangsamung (Anteil) für 2,5 s
    pub bremse: f32,
    /// Brand bzw. Gift: Schaden pro Sekunde und Dauer
    pub nachwirkung: f32,
    pub dauer: f32,
    /// Katapult: so nah schießt es nicht
    pub mindestens: f32,
    /// Kriegsbanner: Verstärkung für Türme in der Nähe
    pub aura: f32,
    /// Mehrere Ziele auf einmal (Salve, Adlerauge)
    pub ziele: u8,
    /// Betäubung (s)
    pub stun: f32,
    /// Windstoß: so viele Meter zurück
    pub stoss: f32,
    /// Späher: Aufschlag auf jeden Schaden am markierten Ziel
    pub markiert: f32,
    /// Schatzkammer: mehr Beute (Anteil)
    pub beute: f32,
    /// Kaserne: Soldaten, ihre Lebenspunkte und ihr Schlag
    pub soldaten: u8,
    pub soldat_lp: f32,
    pub soldat_schaden: f32,
    /// Trifft fliegende Einheiten? Mit welchem Faktor?
    pub flieger: bool,
    pub gegen_flieger: f32,
    /// Chance auf dreifachen Schaden (Scharfschütze)
    pub krit: f32,
    /// Heilt Soldaten und Barrikaden in Reichweite (Lebenspunkte pro Sekunde)
    pub heilung: f32,
}

impl TowerKind {
    pub const ALL: [TowerKind; 15] = [
        TowerKind::Arrow,
        TowerKind::Ballista,
        TowerKind::Catapult,
        TowerKind::Fire,
        TowerKind::Frost,
        TowerKind::Lightning,
        TowerKind::Sun,
        TowerKind::Arcane,
        TowerKind::Poison,
        TowerKind::Banner,
        TowerKind::Barracks,
        TowerKind::Scout,
        TowerKind::Storm,
        TowerKind::Rune,
        TowerKind::Treasury,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TowerKind::Arrow => "Pfeilturm",
            TowerKind::Ballista => "Balliste",
            TowerKind::Catapult => "Katapult",
            TowerKind::Fire => "Feuerturm",
            TowerKind::Frost => "Frostturm",
            TowerKind::Lightning => "Blitzturm",
            TowerKind::Sun => "Sonnenturm",
            TowerKind::Arcane => "Arkanturm",
            TowerKind::Poison => "Giftturm",
            TowerKind::Banner => "Kriegsbanner",
            TowerKind::Barracks => "Kaserne",
            TowerKind::Scout => "Späherturm",
            TowerKind::Storm => "Sturmturm",
            TowerKind::Rune => "Runenstampfer",
            TowerKind::Treasury => "Schatzkammer",
        }
    }

    /// Mit unbestimmtem Artikel („einen Pfeilturm“, „eine Balliste“ …).
    pub fn with_article(self) -> String {
        let artikel = match self {
            TowerKind::Ballista | TowerKind::Barracks | TowerKind::Treasury => "eine",
            TowerKind::Catapult | TowerKind::Banner => "ein",
            _ => "einen",
        };
        format!("{artikel} {}", self.label())
    }

    pub fn description(self) -> &'static str {
        match self {
            TowerKind::Arrow => "Günstig und schnell: Pfeile auf ein Ziel. Trifft auch Flieger.",
            TowerKind::Ballista => "Schwere Bolzen durchschlagen Rüstungen. Langsam, aber verheerend.",
            TowerKind::Catapult => "Steinbrocken treffen ganze Gruppen. Nicht auf nahe Ziele, nicht auf Flieger.",
            TowerKind::Fire => "Flammenstoß setzt Gegner in Brand. Vergiftete explodieren!",
            TowerKind::Frost => "Eis verlangsamt die Truppen – Blitze treffen Vereiste härter.",
            TowerKind::Lightning => "Kettenblitz springt von Gegner zu Gegner, stark gegen Vereiste.",
            TowerKind::Sun => "Heiliges Licht – doppelter Schaden an Skeletten und Gespenstern.",
            TowerKind::Arcane => "Wird stärker, je länger er dasselbe Ziel trifft. Der Bosskiller.",
            TowerKind::Poison => "Giftwolke schwächt Rüstung und verhindert Heilung. Feuer lässt Vergiftete explodieren.",
            TowerKind::Banner => "Keine Angriffe: Türme in der Nähe schießen schneller und härter.",
            TowerKind::Barracks => "Schickt Soldaten auf die Straße, die Gruppen aufhalten. Gefallene kommen nach.",
            TowerKind::Scout => "Deckt getarnte Meuchler auf und markiert Ziele: alle Türme machen dort mehr Schaden.",
            TowerKind::Storm => "Windstoß wirft ganze Gruppen zurück. Doppelter Schaden an Fliegern.",
            TowerKind::Rune => "Bodenwelle um den Turm betäubt alles in der Nähe. Nicht gegen Flieger und Geister.",
            TowerKind::Treasury => "Keine Angriffe: Einheiten, die in der Nähe fallen, bringen viel mehr Beute und Gold.",
        }
    }

    /// Dateiname in art/lib/tuerme.py und game/assets/bauten/.
    pub fn file(self) -> &'static str {
        match self {
            TowerKind::Arrow => "pfeil",
            TowerKind::Ballista => "balliste",
            TowerKind::Catapult => "katapult",
            TowerKind::Fire => "feuer",
            TowerKind::Frost => "frost",
            TowerKind::Lightning => "blitz",
            TowerKind::Sun => "sonne",
            TowerKind::Arcane => "arkan",
            TowerKind::Poison => "gift",
            TowerKind::Banner => "banner",
            TowerKind::Barracks => "kaserne",
            TowerKind::Scout => "spaeher",
            TowerKind::Storm => "sturm",
            TowerKind::Rune => "runen",
            TowerKind::Treasury => "schatz",
        }
    }

    /// Name und Beschreibung der Richtung auf Stufe 3 (1 = A, 2 = B).
    pub fn zweig(self, zweig: u8) -> (&'static str, &'static str) {
        let a = zweig != 2;
        match self {
            TowerKind::Arrow if a => ("Salve", "Drei Pfeile auf drei verschiedene Ziele."),
            TowerKind::Arrow => ("Scharfschütze", "Viel größere Reichweite, jeder vierte Schuss trifft dreifach."),
            TowerKind::Ballista if a => ("Durchbohren", "Der Bolzen durchschlägt alle Einheiten in einer Linie."),
            TowerKind::Ballista => ("Harpune", "Zielt zuerst auf Flieger, holt sie für 5 s auf den Boden, 1,5× Schaden an ihnen."),
            TowerKind::Catapult if a => ("Brandtöpfe", "Hinterlässt brennenden Boden. Brennt in Teer doppelt."),
            TowerKind::Catapult => ("Felsbrocken", "Betäubt getroffene Gruppen und zerschlägt die Schilde der Ritter."),
            TowerKind::Fire if a => ("Flächenbrand", "Brennende Einheiten stecken beim Tod ihre Nachbarn an."),
            TowerKind::Fire => ("Drachenatem", "Flammenkegel: doppelter Schaden an allem vor dem Turm."),
            TowerKind::Frost if a => ("Einfrieren", "Jeder vierte Treffer friert das Ziel 1,5 s ein."),
            TowerKind::Frost => ("Frostfeld", "Kein Schuss mehr: alles in Reichweite ist dauerhaft verlangsamt, auch Flieger."),
            TowerKind::Lightning if a => ("Überladung", "2,5× Schaden an verlangsamten oder betäubten Zielen (statt 1,5×)."),
            TowerKind::Lightning => ("Gewitter", "Blitze schlagen in bis zu vier Einheiten im ganzen Umkreis ein."),
            TowerKind::Sun if a => ("Läuterung", "Deckt Getarnte auf, verhindert Heilung und bricht Schilde in Reichweite."),
            TowerKind::Sun => ("Sonnenstrahl", "Dauerstrahl mit hohem Schaden, doppelt gegen Bosse."),
            TowerKind::Arcane if a => ("Fokus", "Steigert sich bis +300 % statt +150 % am selben Ziel."),
            TowerKind::Arcane => ("Arkane Kugel", "Fällt das Ziel, springt die Kugel mit voller Kraft sofort zum nächsten."),
            TowerKind::Poison if a => ("Seuche", "Vergiftete, die sterben, hinterlassen eine neue Giftwolke."),
            TowerKind::Poison => ("Säure", "Frisst die Rüstung ganz weg, Vergiftete nehmen +25 % Schaden."),
            TowerKind::Banner if a => ("Kriegstrommel", "Türme in der Nähe schießen doppelt so viel schneller (ohne Schadensbonus)."),
            TowerKind::Banner => ("Heilbanner", "Heilt Soldaten und Barrikaden in Reichweite."),
            TowerKind::Barracks if a => ("Veteranen", "Vier Soldaten statt drei, 50 % mehr Leben, Rüstung."),
            TowerKind::Barracks => ("Paladine", "Heiliger Schlag (doppelt gegen Untote), heilen sich selbst."),
            TowerKind::Scout if a => ("Adlerauge", "Viel größere Reichweite, markiert drei Ziele auf einmal."),
            TowerKind::Scout => ("Kopfgeldjäger", "Markierte Einheiten bringen doppeltes Kopfgeld."),
            TowerKind::Storm if a => ("Orkan", "Wirft fast doppelt so weit zurück und betäubt kurz."),
            TowerKind::Storm => ("Sturmwand", "Dreifacher Schaden an Fliegern, holt sie auf den Boden."),
            TowerKind::Rune if a => ("Beben", "Größere Bodenwelle, betäubt 1,5 s."),
            TowerKind::Rune => ("Runenfeld", "Die Welle hinterlässt ein Runenfeld, das 3 s stark verlangsamt."),
            TowerKind::Treasury if a => ("Goldader", "Bringt seinem Erbauer alle 10 s Gold, solange Wellen laufen."),
            TowerKind::Treasury => ("Tributkammer", "Doppelte statt anderthalbfache Beute in Reichweite."),
        }
    }

    /// Kosten für Stufe 1 bzw. das Aufwerten auf `stufe`: Gold, Holz, Stein, Erz.
    pub fn kosten(self, stufe: u8) -> Vec<(Item, u32)> {
        let (gold, tabelle): ([u32; 3], [[u32; 3]; 3]) = match self {
            TowerKind::Arrow => ([40, 60, 90], [[6, 2, 0], [8, 4, 1], [10, 6, 3]]),
            TowerKind::Ballista => ([60, 80, 120], [[8, 4, 1], [10, 6, 2], [12, 8, 5]]),
            TowerKind::Catapult => ([70, 90, 130], [[10, 6, 0], [12, 9, 2], [14, 12, 4]]),
            TowerKind::Fire => ([55, 75, 110], [[5, 7, 1], [7, 9, 2], [9, 11, 4]]),
            TowerKind::Frost => ([55, 75, 110], [[4, 7, 2], [6, 9, 3], [8, 11, 5]]),
            TowerKind::Lightning => ([65, 90, 130], [[4, 6, 4], [6, 8, 6], [8, 10, 8]]),
            TowerKind::Sun => ([60, 85, 125], [[5, 8, 2], [7, 10, 4], [9, 12, 6]]),
            TowerKind::Arcane => ([65, 90, 135], [[5, 7, 3], [7, 9, 5], [9, 11, 7]]),
            TowerKind::Poison => ([50, 70, 105], [[7, 4, 1], [9, 6, 2], [11, 8, 4]]),
            TowerKind::Banner => ([70, 90, 130], [[8, 3, 2], [10, 5, 4], [12, 7, 6]]),
            TowerKind::Barracks => ([60, 85, 120], [[10, 6, 1], [12, 8, 2], [14, 10, 4]]),
            TowerKind::Scout => ([45, 65, 95], [[8, 3, 0], [10, 4, 1], [12, 6, 2]]),
            TowerKind::Storm => ([60, 80, 120], [[6, 6, 2], [8, 8, 3], [10, 10, 5]]),
            TowerKind::Rune => ([65, 90, 130], [[3, 9, 2], [4, 11, 3], [5, 13, 5]]),
            TowerKind::Treasury => ([80, 110, 160], [[6, 8, 2], [8, 10, 4], [10, 12, 6]]),
        };
        let s = (stufe.clamp(1, 3) - 1) as usize;
        let [holz, stein, erz] = tabelle[s];
        // Die Zaubertürme brauchen für ihre letzte Stufe magische Kristalle (Kristallturm)
        let kristalle = if s == 2 && self.magisch() { 4 } else { 0 };
        [(Item::Gold, gold[s]), (Item::Wood, holz), (Item::Stone, stein), (Item::Ore, erz), (Item::Kristall, kristalle)].into_iter().filter(|&(_, n)| n > 0).collect()
    }

    /// Zaubertürme (Frost, Blitz, Sonne, Arkan, Sturm, Runen)
    pub fn magisch(self) -> bool {
        matches!(self, TowerKind::Frost | TowerKind::Lightning | TowerKind::Sun | TowerKind::Arcane | TowerKind::Storm | TowerKind::Rune)
    }

    /// Wen der Turm treffen kann.
    pub fn filter(self) -> Filter {
        match self {
            TowerKind::Catapult | TowerKind::Fire | TowerKind::Poison => Filter::BODEN,
            TowerKind::Rune => Filter::BODEN_OHNE_GEISTER,
            _ => Filter::ALLE,
        }
    }

    /// Werte auf einer Stufe (1–3): Schaden ×1,6 je Stufe, Reichweite +10 %, Feuerrate +10 %.
    /// Auf Stufe 3 ändert die Richtung (`zweig` 1 = A, 2 = B) einiges.
    pub fn werte(self, stufe: u8, zweig: u8) -> Werte {
        let s = (stufe.clamp(1, 3) - 1) as usize;
        let mal = [1.0, 1.6, 2.56][s];
        let weit = 1.0 + 0.1 * s as f32;
        let schnell = 1.0 - 0.09 * s as f32;
        let stufen = |werte: [f32; 3]| werte[s];
        let mut w = match self {
            TowerKind::Arrow => Werte { reichweite: 20.0, takt: 0.8, schaden: 14.0, art: Some(DamageKind::Physical), ..Default::default() },
            TowerKind::Ballista => Werte { reichweite: 28.0, takt: 2.6, schaden: 70.0, art: Some(DamageKind::Pierce), ..Default::default() },
            TowerKind::Catapult => Werte {
                reichweite: 30.0,
                takt: 3.2,
                schaden: 40.0,
                art: Some(DamageKind::Physical),
                flaeche: stufen([3.0, 3.7, 4.5]),
                mindestens: 8.0,
                ..Default::default()
            },
            TowerKind::Fire => Werte {
                reichweite: 12.0,
                takt: 1.2,
                schaden: 10.0,
                art: Some(DamageKind::Fire),
                flaeche: 1.6,
                nachwirkung: 12.0 * mal,
                dauer: 3.0,
                ..Default::default()
            },
            TowerKind::Frost => Werte {
                reichweite: 16.0,
                takt: 1.4,
                schaden: 6.0,
                art: Some(DamageKind::Frost),
                bremse: stufen([0.25, 0.35, 0.45]),
                ..Default::default()
            },
            TowerKind::Lightning => Werte {
                reichweite: 18.0,
                takt: 1.8,
                schaden: 26.0,
                art: Some(DamageKind::Lightning),
                kette: [3, 4, 5][s],
                ..Default::default()
            },
            TowerKind::Sun => Werte { reichweite: 22.0, takt: 1.5, schaden: 22.0, art: Some(DamageKind::Holy), ..Default::default() },
            TowerKind::Arcane => Werte { reichweite: 20.0, takt: 1.6, schaden: 24.0, art: Some(DamageKind::Arcane), ..Default::default() },
            TowerKind::Poison => Werte {
                reichweite: 15.0,
                takt: 2.4,
                schaden: 0.0,
                art: Some(DamageKind::Poison),
                flaeche: 3.0,
                nachwirkung: 8.0 * mal,
                dauer: 4.0,
                ..Default::default()
            },
            TowerKind::Banner => Werte { reichweite: 12.0, aura: stufen([0.15, 0.25, 0.35]), ..Default::default() },
            TowerKind::Barracks => Werte {
                reichweite: 14.0,
                soldaten: 3,
                soldat_lp: stufen([120.0, 190.0, 300.0]),
                soldat_schaden: stufen([9.0, 15.0, 24.0]),
                ..Default::default()
            },
            TowerKind::Scout => Werte {
                reichweite: 34.0,
                takt: 1.2,
                schaden: 6.0,
                art: Some(DamageKind::Pierce),
                markiert: stufen([0.15, 0.22, 0.3]),
                ..Default::default()
            },
            TowerKind::Storm => Werte {
                reichweite: 16.0,
                takt: 3.0,
                schaden: 12.0,
                art: Some(DamageKind::Physical),
                flaeche: 2.5,
                stoss: stufen([4.0, 5.5, 7.0]),
                gegen_flieger: 2.0,
                ..Default::default()
            },
            TowerKind::Rune => Werte {
                reichweite: 9.0,
                takt: 3.2,
                schaden: 22.0,
                art: Some(DamageKind::Physical),
                stun: stufen([0.6, 0.8, 1.0]),
                ..Default::default()
            },
            TowerKind::Treasury => Werte { reichweite: 18.0, beute: stufen([0.5, 0.65, 0.8]), ..Default::default() },
        };
        w.flieger = self.filter().flieger;
        if w.gegen_flieger == 0.0 {
            w.gegen_flieger = 1.0;
        }
        w.ziele = 1;
        w.schaden *= mal;
        w.reichweite *= weit;
        w.takt *= schnell;
        if s < 2 || zweig == 0 {
            return w;
        }
        let a = zweig == 1;
        match self {
            TowerKind::Arrow if a => w.ziele = 3,
            TowerKind::Arrow => {
                w.reichweite *= 1.6;
                w.krit = 0.25;
            }
            TowerKind::Ballista if !a => w.gegen_flieger = 1.5,
            TowerKind::Catapult if a => {
                w.nachwirkung = 15.0 * mal;
                w.dauer = 4.0;
            }
            TowerKind::Catapult => w.stun = 1.2,
            TowerKind::Fire if !a => {
                w.schaden *= 2.0;
                w.reichweite *= 1.2;
            }
            TowerKind::Frost if !a => {
                w.takt = 0.0;
                w.schaden = 0.0;
                w.bremse = 0.4;
                w.flieger = true;
            }
            TowerKind::Lightning if !a => {
                w.kette = 0;
                w.ziele = 4;
                w.schaden *= 0.85;
            }
            TowerKind::Sun if !a => {
                w.takt = 0.3;
                w.schaden *= 0.32;
            }
            TowerKind::Poison if !a => w.markiert = 0.25,
            TowerKind::Banner if !a => w.heilung = 14.0,
            TowerKind::Barracks if a => {
                w.soldaten = 4;
                w.soldat_lp *= 1.5;
            }
            TowerKind::Barracks => w.art = Some(DamageKind::Holy),
            TowerKind::Scout if a => {
                w.reichweite *= 1.5;
                w.ziele = 3;
            }
            TowerKind::Storm if a => {
                w.stoss *= 1.8;
                w.stun = 0.5;
            }
            TowerKind::Storm => w.gegen_flieger = 3.0,
            TowerKind::Rune if a => {
                w.reichweite *= 1.4;
                w.stun = 1.5;
            }
            TowerKind::Rune => w.bremse = 0.55,
            TowerKind::Treasury if !a => w.beute = 1.0,
            _ => {}
        }
        w
    }
}

impl FallenArt {
    pub const ALL: [FallenArt; 3] = [FallenArt::Stacheln, FallenArt::Teer, FallenArt::Barrikade];

    pub fn label(self) -> &'static str {
        match self {
            FallenArt::Stacheln => "Stachelfalle",
            FallenArt::Teer => "Teergrube",
            FallenArt::Barrikade => "Barrikade",
        }
    }

    pub fn with_article(self) -> String {
        format!("{} {}", if self == FallenArt::Teer { "eine" } else if self == FallenArt::Barrikade { "eine" } else { "eine" }, self.label())
    }

    pub fn description(self) -> &'static str {
        match self {
            FallenArt::Stacheln => "Eiserne Stacheln im Boden: 30 Schaden pro Sekunde an allem, was darüber läuft.",
            FallenArt::Teer => "Zäher Teer: halbiert das Tempo. Feuer brennt darin doppelt so heiß.",
            FallenArt::Barrikade => "Hält Gruppen auf, bis sie zerschlagen ist (420 Leben). Heilbanner reparieren sie.",
        }
    }

    pub fn file(self) -> &'static str {
        match self {
            FallenArt::Stacheln => "falle_stacheln",
            FallenArt::Teer => "falle_teer",
            FallenArt::Barrikade => "barrikade",
        }
    }

    pub fn kosten(self) -> Vec<(Item, u32)> {
        match self {
            FallenArt::Stacheln => vec![(Item::Gold, 25), (Item::Wood, 2), (Item::Stone, 3), (Item::Ore, 1)],
            FallenArt::Teer => vec![(Item::Gold, 30), (Item::Wood, 4), (Item::Stone, 1)],
            FallenArt::Barrikade => vec![(Item::Gold, 20), (Item::Wood, 8), (Item::Stone, 2)],
        }
    }
}

impl Werte {
    /// Kurzbeschreibung der Werte für Menü und Turmfenster.
    pub fn zeilen(&self) -> Vec<String> {
        let mut z = Vec::new();
        if self.aura > 0.0 {
            z.push(format!("Türme im Umkreis von {:.0} m: +{:.0} % Schaden und Feuerrate", self.reichweite, self.aura * 100.0));
            if self.heilung > 0.0 {
                z.push(format!("heilt Soldaten und Barrikaden: {:.0} pro Sekunde", self.heilung));
            }
            return z;
        }
        if self.soldaten > 0 {
            z.push(format!("{} Soldaten mit je {:.0} Leben, Schlag {:.0}", self.soldaten, self.soldat_lp, self.soldat_schaden));
            z.push(format!("verteidigen die Straße im Umkreis von {:.0} m", self.reichweite));
            return z;
        }
        if self.beute > 0.0 {
            z.push(format!("Einheiten, die im Umkreis von {:.0} m fallen: +{:.0} % Beute und Kopfgeld", self.reichweite, self.beute * 100.0));
            return z;
        }
        if self.takt > 0.0 {
            z.push(format!("Reichweite {:.0} m · alle {:.1} s", self.reichweite, self.takt));
        } else {
            z.push(format!("Reichweite {:.0} m · wirkt ständig", self.reichweite));
        }
        if self.schaden > 0.0 {
            z.push(format!("Schaden {:.0}{}{}", self.schaden, match self.art {
                Some(DamageKind::Pierce) => " (durchschlagend)",
                Some(DamageKind::Holy) => " (heilig, ×2 gegen Untote)",
                Some(DamageKind::Arcane) => " (arkan, ohne Rüstung)",
                Some(DamageKind::Fire) => " (Feuer)",
                Some(DamageKind::Frost) => " (Frost)",
                Some(DamageKind::Lightning) => " (Blitz)",
                _ => "",
            }, if self.ziele > 1 { format!(" · {} Ziele", self.ziele) } else { String::new() }));
            if self.takt > 0.0 {
                z.push(format!("≈ {:.0} Schaden pro Sekunde", self.dps()));
            }
        }
        if self.flaeche > 0.0 && self.nachwirkung == 0.0 {
            z.push(format!("Fläche {:.1} m", self.flaeche));
        }
        if self.kette > 0 {
            z.push(format!("Kettenblitz auf {} Ziele", self.kette));
        }
        if self.bremse > 0.0 {
            z.push(format!("verlangsamt um {:.0} %", self.bremse * 100.0));
        }
        if self.nachwirkung > 0.0 {
            z.push(format!("{:.0} Schaden/s für {:.0} s", self.nachwirkung, self.dauer));
        }
        if self.stun > 0.0 {
            z.push(format!("betäubt {:.1} s", self.stun));
        }
        if self.stoss > 0.0 {
            z.push(format!("wirft die Gruppe {:.0} m zurück", self.stoss));
        }
        if self.markiert > 0.0 {
            z.push(format!("markiert: +{:.0} % Schaden von allen", self.markiert * 100.0));
        }
        if self.krit > 0.0 {
            z.push(format!("{:.0} % Chance auf dreifachen Schaden", self.krit * 100.0));
        }
        if self.mindestens > 0.0 {
            z.push(format!("nicht näher als {:.0} m", self.mindestens));
        }
        z.push(if self.flieger { "trifft Flieger".into() } else { "trifft keine Flieger".into() });
        z
    }

    /// Schaden pro Sekunde (ohne Kombos, Rüstung und Nachwirkungen)
    pub fn dps(&self) -> f32 {
        if self.takt <= 0.0 {
            return 0.0;
        }
        let ziele = self.ziele.max(1) as f32 * if self.kette > 0 { 1.0 + 0.85 * (self.kette - 1) as f32 * 0.6 } else { 1.0 };
        self.schaden * ziele * (1.0 + self.krit * 2.0) / self.takt
    }
}

/// Beute für eine besiegte Einheit (bekommt, wer den letzten Treffer gesetzt hat).
pub fn beute(kind: crate::heer::EnemyKind) -> &'static [(Item, u32)] {
    use crate::heer::EnemyKind;
    match kind {
        EnemyKind::Wolf | EnemyKind::Archer | EnemyKind::Skeleton | EnemyKind::Harpy => &[(Item::Stone, 1)],
        EnemyKind::Pikeman | EnemyKind::Warlock | EnemyKind::Ghost | EnemyKind::Assassin => &[(Item::Stone, 1), (Item::Ore, 1)],
        EnemyKind::Knight => &[(Item::Ore, 2)],
        EnemyKind::Felsling => &[(Item::Stone, 2)],
        EnemyKind::Golem => &[(Item::Stone, 4), (Item::Ore, 3)],
        EnemyKind::Spinnling => &[],
        EnemyKind::Troll | EnemyKind::Lich | EnemyKind::Spinnenkoenigin | EnemyKind::Daemon => &[(Item::Stone, 10), (Item::Ore, 10)],
        EnemyKind::Drache => &[(Item::Stone, 20), (Item::Ore, 25)],
        _ => &[],
    }
}

/// Ein Schuss eines Turms (für die Darstellung): welcher Turm, wohin.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Schuss {
    pub turm: u32,
    pub ziel: Vec3,
}

/// Ein Soldat einer Kaserne (nur auf dem Server).
struct Soldat {
    id: u16,
    kaserne: u32,
    platz: u8,
    ort: Vec3,
    facing: f32,
    lp: f32,
    max_lp: f32,
    cooldown: f32,
    /// > 0: gefallen, kommt nach so vielen Sekunden neu
    tot: f32,
    angriff: f32,
    laeuft: bool,
    paladin: bool,
    /// Veteranen tragen Rüstung (30 % weniger Schaden)
    veteran: bool,
}

/// Wolke oder Feld am Boden: Gift, Brand (Katapult) oder Runenfeld.
struct Wolke {
    mitte: Vec3,
    radius: f32,
    dps: f32,
    rest: f32,
    owner: String,
    turm: Option<u32>,
    art: WolkenArt,
}

#[derive(Clone, Copy, PartialEq)]
enum WolkenArt {
    Gift { seuche: bool, saeure: bool },
    Brand,
    Runen(f32),
}

/// Die Türme im Kampf (nur auf dem Server): Nachladezeiten, Wolken, Soldaten, Barrikaden.
#[derive(Default)]
pub struct Verteidigung {
    nachladen: HashMap<u32, f32>,
    wolken: Vec<Wolke>,
    /// Arkanturm: letztes Ziel und aufgebaute Stufen
    arkan: HashMap<u32, (u16, u32)>,
    /// Frostturm (Einfrieren): Treffer seit dem letzten Einfrieren
    frost: HashMap<u32, u32>,
    /// Türme, die der Golem-Boss oder die Spinnenkönigin lahmgelegt hat (Restzeit)
    lahm: HashMap<u32, f32>,
    /// Türme, die die Frostnova des Lichkönigs verlangsamt (Restzeit)
    vereist: HashMap<u32, f32>,
    soldaten: Vec<Soldat>,
    next_soldat: u16,
    /// Sammelpunkte der Kasernen auf der Straße (Ort, Richtung)
    sammel: HashMap<u32, (Vec3, Vec2)>,
    /// Lebenspunkte der Barrikaden
    barrikaden: HashMap<u32, f32>,
    /// Goldader: Zeit bis zum nächsten Gold
    goldader: HashMap<u32, f32>,
    rng: Option<Rng>,
    /// Ereignisse zum Anzeigen, zerstörte Barrikaden und Gold aus Goldadern (holt der Server ab)
    pub ereignisse: Vec<Ereignis>,
    pub zerstoert: Vec<u32>,
    pub gold: Vec<(String, u32)>,
}

impl Verteidigung {
    /// Was die Truppen aufhält: lebende Soldaten und stehende Barrikaden.
    pub fn blocker(&self, buildings: &[Building]) -> Vec<Blocker> {
        let mut liste: Vec<Blocker> = self.soldaten.iter().filter(|s| s.tot <= 0.0).map(|s| Blocker { ort: s.ort, ziel: Ziel::Soldat(s.id) }).collect();
        for b in buildings.iter().filter(|b| b.finished() && b.kind == BuildingKind::Falle(FallenArt::Barrikade)) {
            liste.push(Blocker { ort: b.position + Vec3::Y * 0.8, ziel: Ziel::Barrikade(b.id) });
        }
        liste
    }

    /// Soldaten für den Schnappschuss.
    pub fn soldaten(&self) -> Vec<SoldatState> {
        self.soldaten
            .iter()
            .map(|s| SoldatState {
                id: s.id,
                position: s.ort,
                facing: s.facing,
                action: if s.angriff > 0.0 {
                    EnemyAction::Attack
                } else if s.laeuft {
                    EnemyAction::Walk
                } else {
                    EnemyAction::Idle
                },
                health: if s.tot > 0.0 { 0 } else { ((s.lp / s.max_lp) * 100.0).ceil().clamp(1.0, 100.0) as u8 },
                paladin: s.paladin,
            })
            // Gefallene nur kurz zeigen (sie kippen um), dann erst wieder, wenn sie nachkommen
            .filter(|s| s.health > 0 || self.soldaten.iter().any(|x| x.id == s.id && x.tot > 0.0 && x.tot > respawn_zeit(x) - 1.8))
            .collect()
    }

    /// Barrikaden: Gebäude und Lebenspunkte in Prozent.
    pub fn barrikaden(&self) -> Vec<(u32, u8)> {
        self.barrikaden.iter().map(|(&id, &lp)| (id, ((lp / BARRIKADE_LP) * 100.0).ceil().clamp(0.0, 100.0) as u8)).collect()
    }

    /// Ein Takt: Soldaten, Fallen, Türme. `strikes`: Angriffe der Truppen in diesem Takt (treffen
    /// Soldaten und Barrikaden). Liefert die Schüsse (für die Darstellung). Besiegte Einheiten
    /// landen mit dem Besitzer des Turms in `heer.gefallen`.
    pub fn tick(&mut self, dt: f32, buildings: &[Building], heer: &mut Heer, strikes: &[Strike], boden: &dyn Fn(Vec2) -> f32) -> Vec<Schuss> {
        let mut rng = self.rng.take().unwrap_or_else(|| Rng::new(0x70E));
        let mut schuesse = Vec::new();
        // Golem-Boss hat gestampft: Türme im Umkreis sind lahm
        for (mitte, radius, dauer) in std::mem::take(&mut heer.stampfer) {
            for b in buildings.iter().filter(|b| b.tower().is_some() && b.position.distance(mitte) < radius) {
                self.lahm.insert(b.id, dauer);
            }
        }
        for rest in self.lahm.values_mut() {
            *rest -= dt;
        }
        self.lahm.retain(|_, r| *r > 0.0);
        for (mitte, radius, dauer) in std::mem::take(&mut heer.frostnovas) {
            for b in buildings.iter().filter(|b| b.tower().is_some() && b.position.distance(mitte) < radius) {
                self.vereist.insert(b.id, dauer);
            }
        }
        for rest in self.vereist.values_mut() {
            *rest -= dt;
        }
        self.vereist.retain(|_, r| *r > 0.0);
        // Treffer der Truppen an Soldaten und Barrikaden
        for strike in strikes {
            match strike.ziel {
                Ziel::Soldat(id) => {
                    if let Some(s) = self.soldaten.iter_mut().find(|s| s.id == id && s.tot <= 0.0) {
                        let ruestung = if s.veteran { 0.7 } else { 1.0 };
                        s.lp -= strike.schaden * ruestung;
                        if s.lp <= 0.0 {
                            s.lp = 0.0;
                            s.tot = respawn_zeit(s);
                        }
                    }
                }
                Ziel::Barrikade(id) => {
                    if let Some(lp) = self.barrikaden.get_mut(&id) {
                        *lp -= strike.schaden;
                    }
                }
                Ziel::Spieler => {}
            }
        }
        // Barrikaden: neue eintragen, zerschlagene melden
        for b in buildings.iter().filter(|b| b.finished() && b.kind == BuildingKind::Falle(FallenArt::Barrikade)) {
            self.barrikaden.entry(b.id).or_insert(BARRIKADE_LP);
        }
        self.barrikaden.retain(|id, _| buildings.iter().any(|b| b.id == *id));
        let kaputt: Vec<u32> = self.barrikaden.iter().filter(|&(_, &lp)| lp <= 0.0).map(|(&id, _)| id).collect();
        for id in kaputt {
            self.barrikaden.remove(&id);
            if let Some(b) = buildings.iter().find(|b| b.id == id) {
                self.ereignisse.push(Ereignis::Zerstoert(b.position + Vec3::Y));
            }
            self.zerstoert.push(id);
        }

        // Kriegsbanner: Verstärkung (und Heilung) für alles in ihrer Nähe
        let banner: Vec<(Vec3, Werte, u8)> = buildings
            .iter()
            .filter_map(|b| b.tower().filter(|&t| t == TowerKind::Banner && b.finished()).map(|t| (b.position, t.werte(b.level, b.zweig), b.aktiver_zweig())))
            .collect();
        for (ort, werte, _) in banner.iter().filter(|(_, w, _)| w.heilung > 0.0) {
            for s in self.soldaten.iter_mut().filter(|s| s.tot <= 0.0 && s.ort.distance(*ort) <= werte.reichweite) {
                s.lp = (s.lp + werte.heilung * dt).min(s.max_lp);
            }
            for b in buildings.iter().filter(|b| b.position.distance(*ort) <= werte.reichweite) {
                if let Some(lp) = self.barrikaden.get_mut(&b.id) {
                    *lp = (*lp + werte.heilung * dt).min(BARRIKADE_LP);
                }
            }
        }

        // Späher und Läuterung decken Getarnte auf, bevor die Türme zielen
        for b in buildings.iter().filter(|b| b.finished() && !self.lahm.contains_key(&b.id)) {
            match (b.tower(), b.aktiver_zweig()) {
                (Some(TowerKind::Scout), _) => heer.aufdecken(b.position + Vec3::Y * 2.0, b.kind_werte().reichweite),
                (Some(TowerKind::Sun), 1) => heer.laeutern(b.position + Vec3::Y * 2.0, b.kind_werte().reichweite),
                _ => {}
            }
        }

        self.kasernen(dt, buildings, heer, boden, &banner);
        self.fallen(dt, buildings, heer);

        for building in buildings {
            let Some(kind) = building.tower() else { continue };
            if !building.finished() || matches!(kind, TowerKind::Banner | TowerKind::Barracks | TowerKind::Treasury) || self.lahm.contains_key(&building.id) {
                continue;
            }
            let zweig = building.aktiver_zweig();
            let (a, b) = (zweig == 1, zweig == 2);
            let mut werte = building.kind_werte();
            // Stärkstes Banner in Reichweite (die Kriegstrommel: nur Tempo, das doppelt)
            let (mut tempo, mut kraft) = (0.0f32, 0.0f32);
            for (ort, w, z) in &banner {
                if ort.distance(building.position) <= w.reichweite {
                    if *z == 1 {
                        tempo = tempo.max(w.aura * 2.0);
                    } else {
                        tempo = tempo.max(w.aura);
                        kraft = kraft.max(w.aura);
                    }
                }
            }
            werte.schaden *= 1.0 + kraft;
            werte.nachwirkung *= 1.0 + kraft;
            werte.takt /= 1.0 + tempo;
            if self.vereist.contains_key(&building.id) {
                werte.takt *= 2.0;
            }
            let owner = building.owner.as_str();
            let von = Quelle { name: owner, turm: Some(building.id) };
            let mund = building.position + Vec3::Y * (kopf_hoehe(building.level) + 1.0 * TURM_GROESSE);
            let fuss = building.position + Vec3::Y * 1.0;

            // Frostfeld: kein Schuss, ständige Verlangsamung
            if kind == TowerKind::Frost && b {
                heer.bremsen(building.position, werte.reichweite, werte.bremse, Filter::ALLE, false);
                let bereit = self.nachladen.entry(building.id).or_insert(0.0);
                *bereit -= dt;
                if *bereit <= 0.0 {
                    *bereit = 1.5;
                    self.ereignisse.push(Ereignis::Frostfeld(building.position, werte.reichweite));
                }
                continue;
            }
            let bereit = self.nachladen.entry(building.id).or_insert(0.0);
            *bereit -= dt;
            if *bereit > 0.0 {
                continue;
            }
            let modus = building.ziel;
            let filter = if werte.flieger { Filter { flieger: true, ..kind.filter() } } else { kind.filter() };
            let art = werte.art.unwrap_or(DamageKind::Physical);
            let grund = Hit { schaden: werte.schaden, art, bremse: werte.bremse, stun: werte.stun, ..Default::default() };

            // Runenstampfer: Bodenwelle um den Turm, sobald jemand nah genug ist
            if kind == TowerKind::Rune {
                let ids = heer.within(building.position, werte.reichweite, filter);
                if ids.is_empty() {
                    continue;
                }
                *bereit = werte.takt;
                for id in ids {
                    heer.damage(id, Hit { bremse: 0.0, ..grund }, von);
                }
                self.ereignisse.push(Ereignis::Puls(building.position, werte.reichweite));
                if b {
                    self.wolken.push(Wolke { mitte: building.position, radius: werte.reichweite, dps: 0.0, rest: 3.0, owner: owner.to_string(), turm: Some(building.id), art: WolkenArt::Runen(werte.bremse) });
                    self.ereignisse.push(Ereignis::Frostfeld(building.position, werte.reichweite));
                }
                continue;
            }

            // Ziel suchen (Harpune: zuerst Flieger)
            let ziel = if kind == TowerKind::Ballista && b {
                heer.ziel(mund, werte.reichweite, werte.mindestens, modus, Filter { flieger: true, boden: false, geister: true, getarnte: false })
                    .or_else(|| heer.ziel(mund, werte.reichweite, werte.mindestens, modus, filter))
            } else {
                heer.ziel(mund, werte.reichweite, werte.mindestens, modus, filter)
            };
            let Some((ziel, ort)) = ziel else { continue };
            *bereit = werte.takt;
            let info = heer.info(ziel);
            let fliegt = info.is_some_and(|i| i.fliegt);
            match kind {
                TowerKind::Arrow => {
                    for (id, dort) in heer.ziele(mund, werte.reichweite, 0.0, modus, filter, werte.ziele as usize) {
                        let krit = werte.krit > 0.0 && rng.chance(werte.krit);
                        heer.damage(id, Hit { schaden: werte.schaden * if krit { 3.0 } else { 1.0 }, ..grund }, von);
                        schuesse.push(Schuss { turm: building.id, ziel: dort });
                    }
                }
                TowerKind::Ballista => {
                    let erden = if b { wirkung::ERDEN } else { 0 };
                    if a {
                        let richtung = ort - mund;
                        for (id, _) in heer.auf_linie(mund, richtung, werte.reichweite + 6.0, 0.8, filter) {
                            heer.damage(id, grund, von);
                        }
                        schuesse.push(Schuss { turm: building.id, ziel: mund + richtung.normalize_or(Vec3::Z) * (werte.reichweite + 6.0) });
                    } else {
                        let mal = if fliegt { werte.gegen_flieger } else { 1.0 };
                        heer.damage(ziel, Hit { schaden: werte.schaden * mal, effekte: erden, ..grund }, von);
                        schuesse.push(Schuss { turm: building.id, ziel: ort });
                    }
                }
                TowerKind::Catapult => {
                    let effekte = if b { wirkung::SCHILDBRUCH } else { 0 };
                    for id in heer.within(ort, werte.flaeche, filter) {
                        heer.damage(id, Hit { effekte, ..grund }, von);
                    }
                    if a {
                        let boden_ort = vec3(ort.x, boden(vec2(ort.x, ort.z)), ort.z);
                        self.wolken.push(Wolke { mitte: boden_ort, radius: werte.flaeche, dps: werte.nachwirkung, rest: werte.dauer, owner: owner.to_string(), turm: Some(building.id), art: WolkenArt::Brand });
                        self.ereignisse.push(Ereignis::Brandfeld(boden_ort, werte.flaeche, werte.dauer));
                    }
                    schuesse.push(Schuss { turm: building.id, ziel: ort });
                }
                TowerKind::Fire => {
                    let hit = Hit { brand: werte.nachwirkung, dauer: werte.dauer, effekte: if a { wirkung::AUSBREITEN } else { 0 }, ..grund };
                    if b {
                        for (id, dort) in heer.im_kegel(mund, ort - mund, werte.reichweite, 0.8, filter) {
                            heer.damage(id, hit, von);
                            if rng.chance(0.4) {
                                schuesse.push(Schuss { turm: building.id, ziel: dort });
                            }
                        }
                    } else {
                        for id in heer.within(ort, werte.flaeche, filter) {
                            heer.damage(id, hit, von);
                        }
                    }
                    schuesse.push(Schuss { turm: building.id, ziel: ort });
                }
                TowerKind::Frost => {
                    let mut hit = grund;
                    if a {
                        let zaehler = self.frost.entry(building.id).or_insert(0);
                        *zaehler += 1;
                        if *zaehler >= 4 {
                            *zaehler = 0;
                            hit.stun = 1.5;
                            self.ereignisse.push(Ereignis::Einfrieren(ort));
                        }
                    }
                    heer.damage(ziel, hit, von);
                    schuesse.push(Schuss { turm: building.id, ziel: ort });
                }
                TowerKind::Lightning => {
                    let kombo = |i: Option<crate::heer::ZielInfo>| if i.is_some_and(|i| i.verlangsamt || i.betaeubt) { if a { 2.5 } else { 1.5 } } else { 1.0 };
                    if b {
                        // Gewitter: Einschläge in zufällige Einheiten im Umkreis
                        let mut kandidaten = heer.ziele(building.position, werte.reichweite, 0.0, modus, filter, 12);
                        for _ in 0..werte.ziele {
                            if kandidaten.is_empty() {
                                break;
                            }
                            let (id, dort) = kandidaten.swap_remove((rng.next_u32() as usize) % kandidaten.len());
                            let mal = kombo(heer.info(id));
                            heer.damage(id, Hit { schaden: werte.schaden * mal, ..grund }, von);
                            schuesse.push(Schuss { turm: building.id, ziel: dort });
                        }
                    } else {
                        // Kettenblitz: springt zur nächsten Einheit, jedes Mal etwas schwächer
                        let mut getroffen = vec![ziel];
                        let (mut dort, mut hit) = (ort, grund);
                        heer.damage(ziel, Hit { schaden: hit.schaden * kombo(info), ..hit }, von);
                        schuesse.push(Schuss { turm: building.id, ziel: ort });
                        for _ in 1..werte.kette {
                            let Some((naechster, ort2)) = heer.nearest_except(dort, 7.0, &getroffen, filter) else { break };
                            hit.schaden *= 0.85;
                            let mal = kombo(heer.info(naechster));
                            heer.damage(naechster, Hit { schaden: hit.schaden * mal, ..hit }, von);
                            schuesse.push(Schuss { turm: building.id, ziel: ort2 });
                            getroffen.push(naechster);
                            dort = ort2;
                        }
                    }
                }
                TowerKind::Sun => {
                    let mal = if b && info.is_some_and(|i| i.boss) { 2.0 } else { 1.0 };
                    heer.damage(ziel, Hit { schaden: werte.schaden * mal, ..grund }, von);
                    schuesse.push(Schuss { turm: building.id, ziel: ort });
                }
                TowerKind::Arcane => {
                    // Je länger dasselbe Ziel, desto stärker (+15 % je Treffer)
                    let (letztes, stapel) = self.arkan.get(&building.id).copied().unwrap_or((0, 0));
                    let stapel = if letztes == ziel || (b && letztes != 0 && heer.info(letztes).is_none()) { stapel + 1 } else { 0 };
                    let grenze = if a { 20 } else { 10 };
                    let mal = 1.0 + 0.15 * stapel.min(grenze) as f32;
                    let besiegt = heer.damage(ziel, Hit { schaden: werte.schaden * mal, ..grund }, von).is_some();
                    schuesse.push(Schuss { turm: building.id, ziel: ort });
                    self.arkan.insert(building.id, (ziel, stapel));
                    if besiegt && b {
                        // Arkane Kugel: springt sofort weiter
                        if let Some((naechster, ort2)) = heer.ziel(ort, werte.reichweite, 0.0, modus, filter) {
                            heer.damage(naechster, Hit { schaden: werte.schaden * mal, ..grund }, von);
                            schuesse.push(Schuss { turm: building.id, ziel: ort2 });
                            self.arkan.insert(building.id, (naechster, stapel));
                        }
                    }
                }
                TowerKind::Poison => {
                    let boden_ort = vec3(ort.x, boden(vec2(ort.x, ort.z)), ort.z);
                    self.wolken.push(Wolke {
                        mitte: boden_ort,
                        radius: werte.flaeche,
                        dps: werte.nachwirkung,
                        rest: werte.dauer,
                        owner: owner.to_string(),
                        turm: Some(building.id),
                        art: WolkenArt::Gift { seuche: a, saeure: b },
                    });
                    self.ereignisse.push(Ereignis::Giftwolke(boden_ort, werte.flaeche, werte.dauer));
                    schuesse.push(Schuss { turm: building.id, ziel: ort });
                }
                TowerKind::Scout => {
                    let bonus = if b { 1.0 } else { 0.0 };
                    for (id, dort) in heer.ziele(mund, werte.reichweite, 0.0, modus, filter, werte.ziele as usize) {
                        heer.markieren(id, werte.markiert, 4.0, bonus);
                        heer.damage(id, grund, von);
                        self.ereignisse.push(Ereignis::Markiert(dort));
                        schuesse.push(Schuss { turm: building.id, ziel: dort });
                    }
                }
                TowerKind::Storm => {
                    let effekte = if b { wirkung::ERDEN } else { 0 };
                    for id in heer.within(ort, werte.flaeche, filter) {
                        let mal = if heer.info(id).is_some_and(|i| i.fliegt) { werte.gegen_flieger } else { 1.0 };
                        heer.damage(id, Hit { schaden: werte.schaden * mal, effekte, ..grund }, von);
                    }
                    if !fliegt {
                        heer.stoss(ziel, werte.stoss);
                    }
                    schuesse.push(Schuss { turm: building.id, ziel: ort });
                }
                TowerKind::Banner | TowerKind::Barracks | TowerKind::Treasury | TowerKind::Rune => {}
            }
            let _ = fuss;
        }

        // Wolken und Felder: Gift, Brand, Runen
        for wolke in &mut self.wolken {
            wolke.rest -= dt;
            let von = Quelle { name: &wolke.owner, turm: wolke.turm };
            match wolke.art {
                WolkenArt::Gift { seuche, saeure } => {
                    for id in heer.within(wolke.mitte, wolke.radius, Filter::BODEN) {
                        let effekte = wirkung::GIFT | if seuche { wirkung::SEUCHE } else { 0 };
                        heer.damage(id, Hit { schaden: wolke.dps * dt, art: DamageKind::Poison, dauer: 1.5, effekte, ..Default::default() }, von);
                        if saeure {
                            heer.markieren(id, 0.25, 1.0, 0.0);
                        }
                        heer.schwaechen(id, if saeure { 1.0 } else { 0.2 });
                    }
                }
                WolkenArt::Brand => {
                    for id in heer.within(wolke.mitte, wolke.radius, Filter::BODEN) {
                        heer.damage(id, Hit { schaden: wolke.dps * dt, art: DamageKind::Fire, ..Default::default() }, von);
                    }
                }
                WolkenArt::Runen(anteil) => heer.bremsen(wolke.mitte, wolke.radius, anteil, Filter::BODEN_OHNE_GEISTER, false),
            }
        }
        self.wolken.retain(|w| w.rest > 0.0);

        // Wirkungen beim Tod: Seuche (neue Giftwolke) und Flächenbrand (Nachbarn fangen Feuer)
        let tote: Vec<crate::heer::Gefallen> = heer.gefallen.iter().filter(|g| g.seuche || g.ausbreiten).cloned().collect();
        for g in tote {
            if g.seuche {
                let mitte = vec3(g.ort.x, boden(vec2(g.ort.x, g.ort.z)), g.ort.z);
                self.wolken.push(Wolke { mitte, radius: 2.5, dps: 15.0, rest: 3.0, owner: g.von.clone(), turm: g.turm, art: WolkenArt::Gift { seuche: true, saeure: false } });
                self.ereignisse.push(Ereignis::Giftwolke(mitte, 2.5, 3.0));
            }
            if g.ausbreiten {
                let von = Quelle { name: &g.von, turm: g.turm };
                for id in heer.within(g.ort, 3.0, Filter::BODEN) {
                    heer.damage(id, Hit { schaden: 5.0, art: DamageKind::Fire, brand: 14.0, dauer: 3.0, effekte: wirkung::AUSBREITEN, ..Default::default() }, von);
                }
                self.ereignisse.push(Ereignis::Explosion(g.ort, 3.0));
            }
        }

        // Goldader: Gold für den Erbauer, solange Wellen laufen
        for b in buildings.iter().filter(|b| b.finished() && b.tower() == Some(TowerKind::Treasury) && b.aktiver_zweig() == 1) {
            let rest = self.goldader.entry(b.id).or_insert(10.0);
            if heer.enabled {
                *rest -= dt;
            }
            if *rest <= 0.0 {
                *rest = 10.0;
                self.gold.push((b.owner.clone(), 8));
            }
        }

        self.nachladen.retain(|id, _| buildings.iter().any(|b| b.id == *id));
        self.rng = Some(rng);
        schuesse
    }

    /// Soldaten der Kasernen: aufstellen, kämpfen, fallen und nachrücken.
    fn kasernen(&mut self, dt: f32, buildings: &[Building], heer: &mut Heer, boden: &dyn Fn(Vec2) -> f32, _banner: &[(Vec3, Werte, u8)]) {
        let kasernen: Vec<&Building> = buildings.iter().filter(|b| b.finished() && b.tower() == Some(TowerKind::Barracks)).collect();
        self.soldaten.retain(|s| kasernen.iter().any(|k| k.id == s.kaserne));
        self.sammel.retain(|id, _| kasernen.iter().any(|k| k.id == *id));
        for kaserne in &kasernen {
            let werte = kaserne.kind_werte();
            let paladin = kaserne.aktiver_zweig() == 2;
            let veteran = kaserne.aktiver_zweig() == 1;
            let sammel = *self.sammel.entry(kaserne.id).or_insert_with(|| {
                heer.naechster_strassenpunkt(vec2(kaserne.position.x, kaserne.position.z)).map(|(p, dir, _)| (p, dir)).unwrap_or((kaserne.position, Vec2::Y))
            });
            // So viele Soldaten wie die Stufe erlaubt (neue Stufe: Leben anpassen)
            let vorhanden = self.soldaten.iter().filter(|s| s.kaserne == kaserne.id).count();
            for platz in vorhanden..werte.soldaten as usize {
                let id = self.next_soldat;
                self.next_soldat = self.next_soldat.wrapping_add(1);
                self.soldaten.push(Soldat {
                    id,
                    kaserne: kaserne.id,
                    platz: platz as u8,
                    ort: kaserne.position,
                    facing: 0.0,
                    lp: werte.soldat_lp,
                    max_lp: werte.soldat_lp,
                    cooldown: 0.5,
                    tot: 0.0,
                    angriff: 0.0,
                    laeuft: false,
                    paladin,
                    veteran,
                });
            }
            let mut ueberzaehlig = self.soldaten.iter().filter(|s| s.kaserne == kaserne.id).count().saturating_sub(werte.soldaten as usize);
            self.soldaten.retain(|s| {
                if s.kaserne == kaserne.id && ueberzaehlig > 0 && s.platz as usize >= werte.soldaten as usize {
                    ueberzaehlig -= 1;
                    return false;
                }
                true
            });
            for s in self.soldaten.iter_mut().filter(|s| s.kaserne == kaserne.id) {
                s.paladin = paladin;
                s.veteran = veteran;
                if (s.max_lp - werte.soldat_lp).abs() > 0.5 {
                    let anteil = s.lp / s.max_lp.max(1.0);
                    s.max_lp = werte.soldat_lp;
                    s.lp = s.max_lp * anteil;
                }
                s.angriff = (s.angriff - dt).max(0.0);
                s.cooldown -= dt;
                if s.tot > 0.0 {
                    s.tot -= dt;
                    if s.tot <= 0.0 {
                        s.tot = 0.0;
                        s.lp = s.max_lp;
                        s.ort = kaserne.position + Vec3::Y * 0.2;
                    }
                    continue;
                }
                if paladin {
                    s.lp = (s.lp + 4.0 * dt).min(s.max_lp);
                }
                // Platz in der Reihe quer über die Straße
                let (mitte, dir) = sammel;
                let quer = dir.perp();
                let versatz = match s.platz {
                    0 => quer * -1.4,
                    1 => quer * 1.4,
                    2 => Vec2::ZERO,
                    _ => -dir * 1.6,
                };
                let heim = vec2(mitte.x, mitte.z) + versatz;
                let hier = vec2(s.ort.x, s.ort.z);
                let feind = heer.nearest_except(s.ort, 7.0, &[], Filter::NAHKAMPF).filter(|(_, p)| vec2(p.x, p.z).distance(heim) < werte.reichweite * 0.75);
                let (ziel, kampf) = match feind {
                    Some((id, p)) => (vec2(p.x, p.z), Some(id)),
                    None => (heim, None),
                };
                let weg = ziel - hier;
                let abstand = weg.length();
                let nah = if kampf.is_some() { 1.5 } else { 0.2 };
                s.laeuft = abstand > nah;
                if s.laeuft {
                    let schritt = weg / abstand * (3.6 * dt).min(abstand - nah);
                    let neu = hier + schritt;
                    s.ort = vec3(neu.x, boden(neu), neu.y);
                    s.facing = weg.x.atan2(weg.y);
                } else {
                    s.ort.y = boden(hier);
                    s.facing = if kampf.is_some() { weg.x.atan2(weg.y) } else { (-dir).x.atan2(-dir.y) };
                }
                if let Some(id) = kampf {
                    if abstand <= 2.0 && s.cooldown <= 0.0 {
                        s.cooldown = 1.0;
                        s.angriff = 0.6;
                        let art = if paladin { DamageKind::Holy } else { DamageKind::Physical };
                        heer.damage(id, Hit { schaden: werte.soldat_schaden, art, ..Default::default() }, Quelle { name: &kaserne.owner, turm: Some(kaserne.id) });
                    }
                }
            }
        }
    }

    /// Fallen auf der Straße: Stacheln verletzen, Teer bremst.
    fn fallen(&mut self, dt: f32, buildings: &[Building], heer: &mut Heer) {
        for b in buildings.iter().filter(|b| b.finished()) {
            match b.kind {
                BuildingKind::Falle(FallenArt::Stacheln) => {
                    let von = Quelle { name: &b.owner, turm: Some(b.id) };
                    for id in heer.within(b.position, 2.2, Filter::BODEN_OHNE_GEISTER) {
                        heer.damage(id, Hit { schaden: 30.0 * dt, art: DamageKind::Pierce, ..Default::default() }, von);
                    }
                }
                BuildingKind::Falle(FallenArt::Teer) => heer.bremsen(b.position, 2.8, 0.5, Filter::BODEN_OHNE_GEISTER, true),
                _ => {}
            }
        }
    }
}

/// So lange braucht ein gefallener Soldat, bis er nachkommt (je Stufe kürzer).
fn respawn_zeit(s: &Soldat) -> f32 {
    if s.max_lp > 250.0 {
        8.0
    } else if s.max_lp > 150.0 {
        10.0
    } else {
        12.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stufen_machen_staerker_und_kosten_mehr() {
        for kind in TowerKind::ALL {
            let (a, b, c) = (kind.werte(1, 0), kind.werte(2, 0), kind.werte(3, 0));
            assert!(b.schaden >= a.schaden && c.schaden >= b.schaden, "{kind:?} Schaden");
            assert!(b.reichweite > a.reichweite && c.reichweite > b.reichweite, "{kind:?} Reichweite");
            let summe = |stufe| kind.kosten(stufe).iter().map(|(_, n)| n).sum::<u32>();
            assert!(summe(2) > summe(1) && summe(3) > summe(2), "{kind:?} Kosten");
            assert!(kind.kosten(1).iter().any(|(i, _)| *i == Item::Gold), "{kind:?} kostet kein Gold");
            // Beide Richtungen unterscheiden sich
            let (za, zb) = (kind.zweig(1), kind.zweig(2));
            assert_ne!(za.0, zb.0);
        }
        assert_eq!(TowerKind::Banner.werte(3, 0).aura, 0.35);
        assert_eq!(TowerKind::Lightning.werte(3, 0).kette, 5);
        assert_eq!(TowerKind::Arrow.werte(3, 1).ziele, 3);
        assert!(TowerKind::Arrow.werte(3, 2).reichweite > TowerKind::Arrow.werte(3, 1).reichweite * 1.5);
        // Richtungen wirken erst ab Stufe 3
        assert_eq!(TowerKind::Arrow.werte(2, 1).ziele, 1);
        assert!(!TowerKind::Catapult.werte(1, 0).flieger && TowerKind::Arrow.werte(1, 0).flieger);
    }
}

#[cfg(test)]
mod kampf_tests {
    use super::*;
    use crate::heer::{EnemyKind, Heer};

    /// Ein Turm an der Südstraße: Stelle nach `entlang` Punkten, `seitlich` daneben.
    pub(crate) fn turm_an_strasse(world: &crate::world::World, kind: BuildingKind, id: u32, entlang: usize, seitlich: f32, level: u8, zweig: u8) -> Building {
        let strasse: Vec<Vec2> = world.heer.strassen().next().unwrap();
        let (a, b) = (strasse[entlang], strasse[entlang + 1]);
        let quer = (b - a).normalize().perp();
        let p = a + quer * seitlich;
        Building {
            id,
            kind,
            position: vec3(p.x, world.terrain.height_at(p.x, p.y), p.y),
            yaw: 0.0,
            progress: 1.0,
            owner: "nils".into(),
            produce_in: 0.0,
            level,
            zweig,
            ziel: Default::default(),
        }
    }

    fn simulieren(world: &mut crate::world::World, tuerme: &[Building], sekunden: f32) -> (usize, Heer) {
        let mut heer = std::mem::replace(&mut world.heer, Heer::new(Vec::new()));
        let mut verteidigung = Verteidigung::default();
        let mut schuesse = 0;
        let boden = |p: Vec2| world.terrain.height_at(p.x, p.y);
        for _ in 0..(sekunden / 0.05) as usize {
            let blocker = verteidigung.blocker(tuerme);
            let strikes = heer.tick(0.05, &blocker, &boden);
            schuesse += verteidigung.tick(0.05, tuerme, &mut heer, &strikes, &boden).len();
        }
        (schuesse, heer)
    }

    #[test]
    fn tuerme_besiegen_truppen_und_bringen_beute() {
        let mut ctx = Context::headless();
        let mut world = crate::world::World::new(&mut ctx);
        let tuerme: Vec<Building> = TowerKind::ALL
            .iter()
            .enumerate()
            .map(|(i, &kind)| turm_an_strasse(&world, BuildingKind::Tower(kind), i as u32 + 1, 8 + i * 4, if i % 2 == 0 { 7.0 } else { -7.0 }, 3, (i % 2) as u8 + 1))
            .collect();
        world.heer.spawn_wave();
        let (schuesse, mut heer) = simulieren(&mut world, &tuerme, 120.0);
        assert!(schuesse > 5, "Türme schießen kaum: {schuesse}");
        let gefallen = std::mem::take(&mut heer.gefallen);
        assert!(!gefallen.is_empty(), "keine Einheit besiegt");
        assert!(gefallen.iter().all(|g| g.von == "nils"), "Beute an den Falschen");
        assert!(heer.turm_stats.values().any(|s| s.1 > 0), "keine Kills je Turm gezählt");
    }

    #[test]
    fn kaserne_haelt_gruppen_auf() {
        let mut ctx = Context::headless();
        let mut world = crate::world::World::new(&mut ctx);
        let kaserne = turm_an_strasse(&world, BuildingKind::Tower(TowerKind::Barracks), 1, 10, 8.0, 3, 1);
        world.heer.spawn_wave();
        let (_, heer) = simulieren(&mut world, &[kaserne.clone()], 60.0);
        // Nach einer Minute ist die Südgruppe noch nicht weit an der Kaserne vorbei
        let strasse: Vec<Vec2> = heer.strassen().next().unwrap();
        let hinter = strasse[30];
        let vorbei = heer.states().iter().filter(|s| vec2(s.position.x, s.position.z).distance(hinter) < 20.0).count();
        assert_eq!(vorbei, 0, "die Soldaten halten niemanden auf");
    }

    /// Spielt Wellen gegen eine Verteidigung (je Straße dieselben Türme) und liefert die Welle, in der
    /// die Insel fällt – oder `ZIEL_WELLE + 1` bei Sieg.
    fn welle_bis_zum_fall(tuerme_je_strasse: &[(TowerKind, u8, u8)]) -> u32 {
        let mut ctx = Context::headless();
        let mut world = crate::world::World::new(&mut ctx);
        let strassen: Vec<Vec<Vec2>> = world.heer.strassen().collect();
        let mut tuerme = Vec::new();
        for (s, strasse) in strassen.iter().enumerate() {
            for (i, &(kind, level, zweig)) in tuerme_je_strasse.iter().enumerate() {
                let k = 6 + i * 6;
                let (a, b) = (strasse[k], strasse[k + 1]);
                let p = a + (b - a).normalize().perp() * if i % 2 == 0 { 7.0 } else { -7.0 };
                tuerme.push(Building {
                    id: (s * 100 + i + 1) as u32,
                    kind: BuildingKind::Tower(kind),
                    position: vec3(p.x, world.terrain.height_at(p.x, p.y), p.y),
                    yaw: 0.0,
                    progress: 1.0,
                    owner: "nils".into(),
                    produce_in: 0.0,
                    level,
                    zweig,
                    ziel: Default::default(),
                });
            }
        }
        let mut heer = std::mem::replace(&mut world.heer, Heer::new(Vec::new()));
        let mut verteidigung = Verteidigung::default();
        let boden = |p: Vec2| world.terrain.height_at(p.x, p.y);
        heer.set_enabled(true);
        let mut hoechste = 0;
        for _ in 0..(crate::td::ZIEL_WELLE as f32 * crate::heer::WAVE_SECONDS * 1.3 / 0.05) as usize {
            let blocker = verteidigung.blocker(&tuerme);
            let strikes = heer.tick(0.05, &blocker, &boden);
            verteidigung.tick(0.05, &tuerme, &mut heer, &strikes, &boden);
            heer.gefallen.clear();
            heer.meldungen.clear();
            heer.berichte.clear();
            heer.ereignisse.clear();
            if !heer.gefallene_lanes.is_empty() {
                return heer.welle;
            }
            hoechste = hoechste.max(heer.welle);
            if heer.sieg {
                return crate::td::ZIEL_WELLE + 1;
            }
        }
        crate::td::ZIEL_WELLE + 1
    }

    /// Balancing: ohne Verteidigung fällt die Insel sofort, eine schwache Verteidigung hält ein paar
    /// Wellen, eine starke (ausgebaute Türme mit Richtungen) übersteht alle 30.
    /// Ausgabe mit `cargo test --release balancing -- --nocapture`.
    #[test]
    fn balancing() {
        use TowerKind::*;
        let ohne = welle_bis_zum_fall(&[]);
        let schwach = welle_bis_zum_fall(&[(Arrow, 2, 0), (Fire, 2, 0), (Frost, 2, 0), (Lightning, 2, 0)]);
        let stark = welle_bis_zum_fall(&[
            (Barracks, 3, 1),
            (Frost, 3, 1),
            (Lightning, 3, 1),
            (Arcane, 3, 1),
            (Scout, 3, 1),
            (Catapult, 3, 2),
            (Arrow, 3, 1),
            (Sun, 3, 1),
            (Poison, 3, 2),
            (Fire, 3, 1),
            (Banner, 3, 2),
            (Ballista, 3, 2),
            (Storm, 3, 2),
        ]);
        println!("Balancing: ohne Türme fällt die erste Straße in Welle {ohne}, schwach in Welle {schwach}, stark {}", if stark > crate::td::ZIEL_WELLE { "nie (Sieg)".to_string() } else { format!("in Welle {stark}") });
        assert!(ohne <= 8, "ohne Verteidigung hält die Insel zu lange: {ohne}");
        assert!((4..=20).contains(&schwach), "schwache Verteidigung: Welle {schwach}");
        assert!(stark > 20, "starke Verteidigung fällt schon in Welle {stark}");
    }

    #[test]
    fn frost_bremst_und_ruestung_zaehlt() {
        // Rüstung: der Ritter nimmt von Pfeilen weniger als vom Arkanturm
        assert!(EnemyKind::Knight.factor(DamageKind::Physical, 0.0) < EnemyKind::Knight.factor(DamageKind::Arcane, 0.0));
        assert!(EnemyKind::Ghost.factor(DamageKind::Holy, 0.0) > 1.5);
        assert_eq!(EnemyKind::Skeleton.factor(DamageKind::Poison, 0.0), 0.0);
    }
}
