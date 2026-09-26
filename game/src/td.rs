//! Regeln der Tower Defense, die alle Rechner kennen: Zielmodi der Türme, Schwierigkeit, Gold,
//! der Stand der Verteidigung im Schnappschuss (`TdStand`), Auswertungen am Ende einer Welle und
//! Ereignisse zum Anzeigen (Bossfähigkeiten, Explosionen …).
//!
//! Gerechnet wird auf dem Server: Truppen in `heer.rs`, Türme, Soldaten und Fallen in `tuerme.rs`.
//! Konzept: docs/konzept_tower_defense.md

use engine::prelude::*;
use serde::{Deserialize, Serialize};

use crate::heer::{EnemyAction, EnemyKind};

/// Nach dieser Welle ist die Insel gerettet (außer im Endlosmodus).
pub const ZIEL_WELLE: u32 = 30;
/// Gold, mit dem jeder Spieler anfängt.
pub const STARTGOLD: u32 = 150;

/// Worauf ein Turm zielt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Zielmodus {
    /// Wer auf der Straße am weitesten ist
    #[default]
    Erster,
    /// Wer am weitesten hinten ist
    Letzter,
    /// Die meisten Lebenspunkte
    Staerkster,
    /// Die wenigsten Lebenspunkte
    Schwaechster,
    /// Bosse zuerst, sonst der Erste
    Boss,
}

impl Zielmodus {
    pub const ALL: [Zielmodus; 5] = [Zielmodus::Erster, Zielmodus::Letzter, Zielmodus::Staerkster, Zielmodus::Schwaechster, Zielmodus::Boss];

    pub fn label(self) -> &'static str {
        match self {
            Zielmodus::Erster => "Erster",
            Zielmodus::Letzter => "Letzter",
            Zielmodus::Staerkster => "Stärkster",
            Zielmodus::Schwaechster => "Schwächster",
            Zielmodus::Boss => "Boss zuerst",
        }
    }

    pub fn beschreibung(self) -> &'static str {
        match self {
            Zielmodus::Erster => "Wer am weitesten gekommen ist – hält Durchbrüche auf.",
            Zielmodus::Letzter => "Wer ganz hinten läuft – gut für Verlangsamung und Gift.",
            Zielmodus::Staerkster => "Die meisten Lebenspunkte – schwere Türme gegen Golems und Ritter.",
            Zielmodus::Schwaechster => "Die wenigsten Lebenspunkte – räumt schnell auf.",
            Zielmodus::Boss => "Bosse zuerst, sonst wer am weitesten gekommen ist.",
        }
    }
}

/// Schwierigkeit: Leben der Insel, Zähigkeit der Truppen und Gold.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Schwierigkeit {
    Leicht,
    #[default]
    Normal,
    Schwer,
    Albtraum,
}

impl Schwierigkeit {
    pub const ALL: [Schwierigkeit; 4] = [Schwierigkeit::Leicht, Schwierigkeit::Normal, Schwierigkeit::Schwer, Schwierigkeit::Albtraum];

    pub fn label(self) -> &'static str {
        match self {
            Schwierigkeit::Leicht => "Leicht",
            Schwierigkeit::Normal => "Normal",
            Schwierigkeit::Schwer => "Schwer",
            Schwierigkeit::Albtraum => "Albtraum",
        }
    }

    pub fn leben(self) -> u32 {
        match self {
            Schwierigkeit::Leicht => 30,
            Schwierigkeit::Normal => 20,
            Schwierigkeit::Schwer => 15,
            Schwierigkeit::Albtraum => 10,
        }
    }

    /// Faktor auf die Lebenspunkte der Truppen
    pub fn zaehigkeit(self) -> f32 {
        match self {
            Schwierigkeit::Leicht => 0.7,
            Schwierigkeit::Normal => 1.0,
            Schwierigkeit::Schwer => 1.35,
            Schwierigkeit::Albtraum => 1.8,
        }
    }

    /// Faktor auf Kopfgeld und Wellenbonus
    pub fn gold(self) -> f32 {
        match self {
            Schwierigkeit::Leicht => 1.25,
            Schwierigkeit::Normal => 1.0,
            Schwierigkeit::Schwer => 0.9,
            Schwierigkeit::Albtraum => 0.8,
        }
    }
}

/// Kopfgeld für eine besiegte Einheit (Gold für den, der sie besiegt hat).
pub fn kopfgeld(kind: EnemyKind, boss: bool, welle: u32, schwierigkeit: Schwierigkeit) -> u32 {
    let grund = match kind {
        EnemyKind::Wolf | EnemyKind::Skeleton | EnemyKind::Archer | EnemyKind::Harpy => 4.0,
        EnemyKind::Felsling => 3.0,
        EnemyKind::Pikeman | EnemyKind::Warlock | EnemyKind::Ghost | EnemyKind::Assassin => 6.0,
        EnemyKind::Knight => 8.0,
        EnemyKind::Golem => 15.0,
        EnemyKind::Spinnling => 1.0,
        EnemyKind::Troll | EnemyKind::Lich | EnemyKind::Spinnenkoenigin | EnemyKind::Daemon => 150.0,
        EnemyKind::Drache => 300.0,
    };
    let grund = if boss && !kind.ist_boss_art() { 60.0 } else { grund };
    (grund * (1.0 + 0.05 * welle.saturating_sub(1) as f32) * schwierigkeit.gold()).round() as u32
}

/// Gold für jeden Spieler, wenn eine Welle ganz überstanden ist.
pub fn wellenbonus(welle: u32, schwierigkeit: Schwierigkeit) -> u32 {
    ((10 + 3 * welle) as f32 * schwierigkeit.gold()).round() as u32
}

/// Eine Einheit der Kaserne (für die Darstellung).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoldatState {
    pub id: u16,
    pub position: Vec3,
    pub facing: f32,
    pub action: EnemyAction,
    /// Lebenspunkte in Prozent (0 = gefallen, wartet auf Ersatz)
    pub health: u8,
    /// Paladin (Kaserne, Zweig B): leuchtet golden
    pub paladin: bool,
}

/// Stand der Verteidigung für alle Spieler (in jedem Schnappschuss).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TdStand {
    /// Schickt die Festung Wellen?
    pub aktiv: bool,
    /// Nummer der zuletzt losgeschickten Welle (0 = noch keine)
    pub welle: u32,
    /// Leben jeder Straße (Reihenfolge wie `strassen`) und die vollen Leben
    pub leben: Vec<u32>,
    pub max_leben: u32,
    /// Sekunden bis zur nächsten Welle
    pub naechste: f32,
    /// Was als Nächstes kommt: Einheiten je Art (alle Straßen zusammen), dazu der Boss
    pub vorschau: Vec<(EnemyKind, u16)>,
    pub vorschau_boss: Option<EnemyKind>,
    pub schwierigkeit: Schwierigkeit,
    pub endlos: bool,
    /// Welle 30 überstanden
    pub sieg: bool,
    /// Die Heerstraßen: Name (Nord, Ost …) und wer sie verteidigt
    pub strassen: Vec<(String, Option<String>)>,
    /// Soldaten der Kasernen
    pub soldaten: Vec<SoldatState>,
    /// Barrikaden: Gebäude und Lebenspunkte in Prozent
    pub barrikaden: Vec<(u32, u8)>,
    /// Nur etwa einmal pro Sekunde gefüllt (sonst leer): Kills und Schaden je Turm …
    pub turm_stats: Vec<(u32, u32, u32)>,
    /// … und Schaden und Kills je Spieler (seit Welle 1)
    pub beitrag: Vec<(String, u32, u32)>,
    /// Siedlungsplätze (Reihenfolge wie `strassen`), in deren Schutzstein ein Runenstein sitzt: wem sie gehören
    #[serde(default)]
    pub runen: Vec<Option<String>>,
}

impl TdStand {
    /// Braucht es die Wellenanzeige?
    pub fn sichtbar(&self) -> bool {
        self.aktiv || self.welle > 0 || self.sieg
    }

    /// Die Straße, die ein Spieler verteidigt (Index).
    pub fn lane_von(&self, name: &str) -> Option<usize> {
        self.strassen.iter().position(|(_, v)| v.as_deref() == Some(name))
    }

}

/// Auswertung am Ende einer Welle.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WellenBericht {
    pub welle: u32,
    pub besiegt: u32,
    pub durchgebrochen: u32,
    /// Wer am meisten Schaden gemacht hat (Name, Schaden)
    pub bester_spieler: Option<(String, u32)>,
    /// Welcher Turm am meisten besiegt hat (Bezeichnung, Besitzer, Kills)
    pub bester_turm: Option<(String, String, u32)>,
    /// Gold, das jeder dafür bekommen hat
    pub gold: u32,
    /// Schaden je Spieler in dieser Welle
    pub schaden: Vec<(String, u32)>,
}

/// Was die Clients zusätzlich zeigen sollen (Bossfähigkeiten, Kombos, Fallen …).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Ereignis {
    /// Ein Dunkelmagier heilt Einheiten um sich
    Heilung(Vec3),
    /// Der Golem-Boss stampft: Türme im Umkreis sind kurz lahm
    Stampfen(Vec3, f32),
    /// Der Magier-Boss ruft Skelette
    Beschwoerung(Vec3),
    /// Das Geister-Boss wird kurz unverwundbar
    Unverwundbar(Vec3),
    /// Der Ritter-Boss gerät in Wut
    Wut(Vec3),
    /// Ein Golem zerfällt in Felslinge
    Spaltung(Vec3),
    /// Gift und Feuer: Explosion (Mitte, Radius)
    Explosion(Vec3, f32),
    /// Runenstampfer: Bodenwelle (Mitte, Radius)
    Puls(Vec3, f32),
    /// Frostturm: Einheit eingefroren
    Einfrieren(Vec3),
    /// Sturmturm: Windstoß zurück
    Windstoss(Vec3),
    /// Späherturm: Ziel markiert
    Markiert(Vec3),
    /// Barrikade zerstört
    Zerstoert(Vec3),
    /// Brandfeld (Katapult mit Brandtöpfen): Mitte, Radius, Dauer
    Brandfeld(Vec3, f32, f32),
    /// Giftwolke (Mitte, Radius, Dauer)
    Giftwolke(Vec3, f32, f32),
    /// Runenfeld oder Frostfeld verlangsamt (Mitte, Radius)
    Frostfeld(Vec3, f32),
    /// Bergtroll schleudert einen Felsen (von, nach)
    Felswurf(Vec3, Vec3),
    /// Lichkönig: Frostnova lähmt Türme (Mitte, Radius)
    Frostnova(Vec3, f32),
    /// Spinnenkönigin spinnt Türme ein (Mitte, Radius)
    Netz(Vec3, f32),
    /// Spinnenkönigin legt Spinnlinge
    Brut(Vec3),
    /// Dämonenfürst ersteht aus den Flammen
    Wiedergeburt(Vec3),
    /// Schattendrache speit Feuer (Maul, Richtung)
    Flammenatem(Vec3, Vec2),
    /// Schattendrache steigt auf
    Auffliegen(Vec3),
}

/// Befehle der Spieler zur Verteidigung.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TdBefehl {
    /// Turm aufwerten; auf Stufe 3 mit Richtung (1 = A, 2 = B)
    Aufwerten(u32, u8),
    /// Eigenes Gebäude abreißen
    Abreissen(u32),
    /// Zielmodus eines Turms
    Zielen(u32, Zielmodus),
    /// Die nächste Welle sofort rufen (Bonusgold für die gesparte Zeit)
    WelleRufen,
    /// Eine Straße verteidigen (Index) – 255 = keine mehr
    Strasse(u8),
}
