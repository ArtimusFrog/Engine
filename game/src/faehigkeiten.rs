//! Fähigkeiten der Spielfiguren: jede Klasse hat vier, auf den Plätzen 3–6 der Auswahlleiste –
//! die vierte ist die ultimative Fähigkeit mit langer Abklingzeit.
//!
//! Magier: Arkangeschoss (lädt arkane Ladungen, mit dreien wird es zur Arkanlanze), Feuerball
//! (Explosion, Brand, Flammenteppich), Frostnova (Eiswelle, friert ein), **Meteorsturm**.
//! Zwerg: Hammerschlag (Dreierkombo mit Schmetterschlag), Wurfhammer (prallt ab und kehrt
//! zurück), Erdbeben (Sprung, zwei Ringe, Nachbeben), **Ahnenhammer**.
//! Bogenschütze: Pfeilschuss (kritische Treffer), Salve (fünf Pfeile im Fächer), Explosivpfeil
//! (zündet nach kurzer Zeit), **Pfeilregen**.
//! Schurke: Klingenhieb (Dreierkombo: zwei Hiebe, dann ein Stich), Wurfdolche (drei
//! Giftdolche im Fächer), Rauchbombe (betäubt alles um ihn), **Schattenklingen**.
//! Gefrorene zerschmettern beim nächsten Treffer (+50 %). Der Server rechnet alles nach; die
//! Werte stehen hier an einer Stelle (Konzept: `docs/konzept_faehigkeiten.md`).

use serde::{Deserialize, Serialize};

use crate::protocol::CharacterClass;
use crate::tuerme::DamageKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Faehigkeit {
    Arkangeschoss,
    Feuerball,
    Frostnova,
    Hammerschlag,
    Wurfhammer,
    Erdbeben,
    // Ultimative Fähigkeiten und der Bogenschütze (hinten angefügt: ältere Nachrichten bleiben lesbar)
    Meteorsturm,
    Ahnenhammer,
    Pfeilschuss,
    Salve,
    Explosivpfeil,
    Pfeilregen,
    // Schurke
    Klingenhieb,
    Wurfdolche,
    Rauchbombe,
    Schattenklingen,
}

/// Wie eine Fähigkeit wirkt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Form {
    /// Fliegt zum Ziel (Tempo m/s); `flaeche` > 0: trifft alles im Umkreis des Einschlags
    Geschoss { tempo: f32, flaeche: f32 },
    /// Trifft alles vor der Figur in `weite` Metern (halber Öffnungswinkel in Grad)
    Nahkampf { weite: f32, winkel: f32 },
    /// Trifft alles im Umkreis der Figur
    UmSich { radius: f32 },
    /// Wirkt auf eine Stelle am Boden unter dem Fadenkreuz (bis `reichweite`) im Umkreis `radius`
    Flaeche { reichweite: f32, radius: f32 },
}

/// Nachwirkungen eines Treffers.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Wirkung {
    /// Verlangsamung (Anteil)
    pub bremse: f32,
    /// Betäubung (s)
    pub stun: f32,
    /// Brand (Schaden/s) und seine Dauer (s)
    pub brand: f32,
    pub dauer: f32,
    /// Eingefroren (s): kann sich nicht rühren, der nächste Treffer zerschmettert das Eis
    pub frost: f32,
}

/// Arkane Ladungen: so viele machen das nächste Arkangeschoss zur Arkanlanze …
pub const LADUNG_MAX: u8 = 3;
/// … und so lange (Takte) halten sie ohne neuen Treffer.
pub const LADUNG_HAELT: u64 = 8 * 60;
/// Arkanlanze: Länge und halbe Breite des Strahls (m), Schaden gegenüber dem Geschoss.
pub const LANZE_WEITE: f32 = 45.0;
pub const LANZE_BREITE: f32 = 0.75;
pub const LANZE_FAKTOR: f32 = 2.2;
/// Feuerball: Flammenteppich (Radius m, Dauer s, Brand je s).
pub const FLAMMEN: (f32, f32, f32) = (3.0, 4.0, 6.0);
/// Frostnova: so schnell läuft die Eiswelle nach außen (m/s).
pub const EISWELLE_TEMPO: f32 = 18.0;
/// Gefrorene nehmen beim Zerschmettern so viel mehr Schaden.
pub const ZERSCHMETTERN: f32 = 1.5;
/// Hammerschlag: so lange (Takte) nach dem Bereitsein geht die Kombo weiter.
pub const KOMBO_FENSTER: u64 = 60;
/// Schmetterschlag: Druckwelle um den Einschlag (Radius m, Anteil am Schaden).
pub const SCHMETTERN_WELLE: (f32, f32) = (2.2, 0.5);
/// Wurfhammer: prallt so oft ab, sucht im Umkreis (m); Schaden und Betäubung je Treffer.
pub const ABPRALLE: usize = 2;
pub const ABPRALL_WEITE: f32 = 9.0;
pub const ABPRALL_SCHADEN: [f32; 3] = [1.0, 0.75, 0.55];
pub const ABPRALL_STUN: [f32; 3] = [1.2, 0.6, 0.6];
/// Wurfhammer: Tempo auf dem Rückweg (m/s).
pub const RUECKFLUG_TEMPO: f32 = 30.0;
/// Erdbeben: innerer Ring (m), Anteil außen; Nachbeben (nach s, Radius m, Schaden).
pub const BEBEN_INNEN: f32 = 3.0;
pub const BEBEN_AUSSEN_ANTEIL: f32 = 0.67;
pub const NACHBEBEN: [(f32, f32, f32); 2] = [(0.7, 5.0, 10.0), (1.4, 5.0, 10.0)];

/// Meteorsturm: so viele Meteore, wie lange sie fallen (s), Radius jedes Einschlags (m),
/// Fallzeit eines Meteors vom Himmel (s).
pub const METEORE: usize = 8;
pub const METEOR_DAUER: f32 = 2.4;
pub const METEOR_RADIUS: f32 = 2.6;
pub const METEOR_FALL: f32 = 0.55;
/// Wann Meteor `i` einschlägt (Sekunden nach dem Einsatz, Ausholen eingerechnet).
pub fn meteor_zeit(i: usize) -> f32 {
    Faehigkeit::Meteorsturm.ausholen(0) + METEOR_FALL + i as f32 * METEOR_DAUER / METEORE as f32
}
/// Ahnenhammer: innerer Ring (m) und Anteil des Schadens außen; so lange fällt der Hammer (s).
pub const AHNEN_INNEN: f32 = 3.0;
pub const AHNEN_AUSSEN_ANTEIL: f32 = 0.65;
pub const AHNEN_FALL: f32 = 0.35;
/// Pfeilschuss: Chance auf einen kritischen Treffer (doppelter Schaden).
pub const KRIT_CHANCE: f32 = 0.2;
/// Salve: so viele Pfeile, halber Fächer (Grad).
pub const SALVE_PFEILE: usize = 5;
pub const SALVE_FAECHER: f32 = 24.0;
/// Explosivpfeil: zündet so lange nach dem Einschlag (s).
pub const ZUENDER: f32 = 0.8;
/// Pfeilregen: so viele Wellen im Abstand von (s), die erste nach dem Ausholen plus Flugzeit (s).
pub const REGEN_WELLEN: usize = 10;
pub const REGEN_ABSTAND: f32 = 0.3;
pub const REGEN_FLUG: f32 = 0.6;
/// Wann Welle `i` des Pfeilregens trifft (Sekunden nach dem Einsatz).
/// Wurfdolche: so viele im Fächer (Grad zwischen den äußeren und der Mitte)
pub const DOLCHE: usize = 3;
pub const DOLCH_FAECHER: f32 = 13.0;
/// Schattenklingen: so viele Klingenwellen am Ziel, Abstand (s), danach der Schlussschlag
pub const KLINGEN_WELLEN: usize = 7;
pub const KLINGEN_ABSTAND: f32 = 0.28;
pub const KLINGEN_SCHLUSS: f32 = 2.5;

/// Wann die i-te Klingenwelle trifft (Sekunden nach dem Auslösen).
pub fn klingen_zeit(i: usize) -> f32 {
    Faehigkeit::Schattenklingen.ausholen(0) + 0.25 + i as f32 * KLINGEN_ABSTAND
}

/// Wann der Schlussschlag der Schattenklingen trifft.
pub fn klingen_schluss() -> f32 {
    klingen_zeit(KLINGEN_WELLEN - 1) + 0.45
}

pub fn regen_zeit(i: usize) -> f32 {
    Faehigkeit::Pfeilregen.ausholen(0) + REGEN_FLUG + i as f32 * REGEN_ABSTAND
}

impl Faehigkeit {
    /// Die vier Fähigkeiten einer Klasse (Platz 0–3, Platz 3 ist die ultimative).
    pub fn der_klasse(class: CharacterClass) -> [Faehigkeit; 4] {
        match class {
            CharacterClass::Zwerg => [Faehigkeit::Hammerschlag, Faehigkeit::Wurfhammer, Faehigkeit::Erdbeben, Faehigkeit::Ahnenhammer],
            CharacterClass::Bogenschuetze => [Faehigkeit::Pfeilschuss, Faehigkeit::Salve, Faehigkeit::Explosivpfeil, Faehigkeit::Pfeilregen],
            CharacterClass::Rogue => [Faehigkeit::Klingenhieb, Faehigkeit::Wurfdolche, Faehigkeit::Rauchbombe, Faehigkeit::Schattenklingen],
            _ => [Faehigkeit::Arkangeschoss, Faehigkeit::Feuerball, Faehigkeit::Frostnova, Faehigkeit::Meteorsturm],
        }
    }

    pub fn von(class: CharacterClass, platz: u8) -> Faehigkeit {
        Self::der_klasse(class)[(platz as usize).min(3)]
    }

    /// Die ultimative Fähigkeit (lange Abklingzeit, Platz 6)?
    pub fn ist_ultimativ(self) -> bool {
        matches!(self, Faehigkeit::Meteorsturm | Faehigkeit::Ahnenhammer | Faehigkeit::Pfeilregen | Faehigkeit::Schattenklingen)
    }

    pub fn label(self) -> &'static str {
        match self {
            Faehigkeit::Arkangeschoss => "Arkangeschoss",
            Faehigkeit::Feuerball => "Feuerball",
            Faehigkeit::Frostnova => "Frostnova",
            Faehigkeit::Hammerschlag => "Hammerschlag",
            Faehigkeit::Wurfhammer => "Wurfhammer",
            Faehigkeit::Erdbeben => "Erdbeben",
            Faehigkeit::Meteorsturm => "Meteorsturm",
            Faehigkeit::Ahnenhammer => "Ahnenhammer",
            Faehigkeit::Pfeilschuss => "Pfeilschuss",
            Faehigkeit::Salve => "Salve",
            Faehigkeit::Explosivpfeil => "Explosivpfeil",
            Faehigkeit::Pfeilregen => "Pfeilregen",
            Faehigkeit::Klingenhieb => "Klingenhieb",
            Faehigkeit::Wurfdolche => "Wurfdolche",
            Faehigkeit::Rauchbombe => "Rauchbombe",
            Faehigkeit::Schattenklingen => "Schattenklingen",
        }
    }

    /// Symbol in `game/assets/icons/` (gerendert von `art/icons/gegenstaende.py`).
    pub fn icon_file(self) -> &'static str {
        match self {
            Faehigkeit::Arkangeschoss => "faehigkeit_arkangeschoss",
            Faehigkeit::Feuerball => "faehigkeit_feuerball",
            Faehigkeit::Frostnova => "faehigkeit_frostnova",
            Faehigkeit::Hammerschlag => "faehigkeit_hammerschlag",
            Faehigkeit::Wurfhammer => "faehigkeit_wurfhammer",
            Faehigkeit::Erdbeben => "faehigkeit_erdbeben",
            Faehigkeit::Meteorsturm => "faehigkeit_meteorsturm",
            Faehigkeit::Ahnenhammer => "faehigkeit_ahnenhammer",
            Faehigkeit::Pfeilschuss => "faehigkeit_pfeilschuss",
            Faehigkeit::Salve => "faehigkeit_salve",
            Faehigkeit::Explosivpfeil => "faehigkeit_explosivpfeil",
            Faehigkeit::Pfeilregen => "faehigkeit_pfeilregen",
            Faehigkeit::Klingenhieb => "faehigkeit_klingenhieb",
            Faehigkeit::Wurfdolche => "faehigkeit_wurfdolche",
            Faehigkeit::Rauchbombe => "faehigkeit_rauchbombe",
            Faehigkeit::Schattenklingen => "faehigkeit_schattenklingen",
        }
    }

    pub fn form(self) -> Form {
        match self {
            Faehigkeit::Arkangeschoss => Form::Geschoss { tempo: 40.0, flaeche: 0.0 },
            Faehigkeit::Feuerball => Form::Geschoss { tempo: 24.0, flaeche: 3.8 },
            Faehigkeit::Frostnova => Form::UmSich { radius: 7.5 },
            Faehigkeit::Hammerschlag => Form::Nahkampf { weite: 3.4, winkel: 65.0 },
            Faehigkeit::Wurfhammer => Form::Geschoss { tempo: 28.0, flaeche: 0.0 },
            Faehigkeit::Erdbeben => Form::UmSich { radius: 6.5 },
            Faehigkeit::Meteorsturm => Form::Flaeche { reichweite: 35.0, radius: 6.0 },
            Faehigkeit::Ahnenhammer => Form::Flaeche { reichweite: 22.0, radius: 6.0 },
            Faehigkeit::Pfeilschuss => Form::Geschoss { tempo: 60.0, flaeche: 0.0 },
            Faehigkeit::Salve => Form::Geschoss { tempo: 55.0, flaeche: 0.0 },
            Faehigkeit::Explosivpfeil => Form::Geschoss { tempo: 45.0, flaeche: 3.5 },
            Faehigkeit::Pfeilregen => Form::Flaeche { reichweite: 32.0, radius: 6.0 },
            Faehigkeit::Klingenhieb => Form::Nahkampf { weite: 3.1, winkel: 70.0 },
            Faehigkeit::Wurfdolche => Form::Geschoss { tempo: 42.0, flaeche: 0.0 },
            Faehigkeit::Rauchbombe => Form::UmSich { radius: 5.5 },
            Faehigkeit::Schattenklingen => Form::Flaeche { reichweite: 26.0, radius: 5.5 },
        }
    }

    /// Wie weit Geschosse fliegen bzw. Flächen gesetzt werden können (Meter).
    pub fn reichweite(self) -> f32 {
        match self.form() {
            Form::Flaeche { reichweite, .. } => reichweite,
            _ => match self {
                Faehigkeit::Arkangeschoss => 45.0,
                Faehigkeit::Feuerball => 40.0,
                Faehigkeit::Wurfhammer => 32.0,
                Faehigkeit::Pfeilschuss => 50.0,
                Faehigkeit::Salve => 35.0,
                Faehigkeit::Explosivpfeil => 42.0,
                Faehigkeit::Wurfdolche => 28.0,
                _ => 0.0,
            },
        }
    }

    /// Abklingzeit in Takten (60 je Sekunde) nach dem Einsatz in dieser Stufe.
    pub fn abklingen(self, stufe: u8) -> u64 {
        match (self, stufe) {
            (Faehigkeit::Arkangeschoss, 1) => 50,
            (Faehigkeit::Arkangeschoss, _) => 36,
            (Faehigkeit::Feuerball, _) => 5 * 60,
            (Faehigkeit::Frostnova, _) => 10 * 60,
            // Nach dem Schmetterschlag braucht der Zwerg einen Moment
            (Faehigkeit::Hammerschlag, 2) => 62,
            (Faehigkeit::Hammerschlag, _) => 30,
            (Faehigkeit::Wurfhammer, _) => 5 * 60,
            (Faehigkeit::Erdbeben, _) => 12 * 60,
            (Faehigkeit::Meteorsturm, _) => 45 * 60,
            (Faehigkeit::Ahnenhammer, _) => 40 * 60,
            (Faehigkeit::Pfeilschuss, _) => 33,
            (Faehigkeit::Salve, _) => 6 * 60,
            (Faehigkeit::Explosivpfeil, _) => 9 * 60,
            (Faehigkeit::Pfeilregen, _) => 40 * 60,
            (Faehigkeit::Klingenhieb, 2) => 44,
            (Faehigkeit::Klingenhieb, _) => 26,
            (Faehigkeit::Wurfdolche, _) => 5 * 60,
            (Faehigkeit::Rauchbombe, _) => 11 * 60,
            (Faehigkeit::Schattenklingen, _) => 40 * 60,
        }
    }

    /// Animation (Clip im Figurenmodell), Tempo und das Bild (30 je Sekunde), in dem die
    /// Fähigkeit wirkt – siehe `art/lib/figuren.py`, `zwerg.py` und `bogenschuetze.py`.
    pub fn animation(self, stufe: u8) -> (&'static str, f32, f32) {
        match (self, stufe) {
            (Faehigkeit::Arkangeschoss, 1) => ("Arkan", 0.75, 5.0),
            (Faehigkeit::Arkangeschoss, _) => ("Arkan", 1.0, 5.0),
            (Faehigkeit::Feuerball, _) => ("Feuerball", 1.0, 11.0),
            (Faehigkeit::Frostnova, _) => ("Frostnova", 1.0, 14.0),
            (Faehigkeit::Hammerschlag, 1) => ("Schlag2", 1.0, 7.0),
            (Faehigkeit::Hammerschlag, 2) => ("Schlag3", 1.0, 13.0),
            (Faehigkeit::Hammerschlag, _) => ("Schlag1", 1.0, 7.0),
            (Faehigkeit::Wurfhammer, _) => ("Wurf", 1.0, 9.0),
            (Faehigkeit::Erdbeben, _) => ("Beben", 1.0, 17.0),
            (Faehigkeit::Meteorsturm, _) => ("Meteor", 1.0, 20.0),
            (Faehigkeit::Ahnenhammer, _) => ("Ahnenruf", 1.0, 22.0),
            (Faehigkeit::Pfeilschuss, _) => ("Schuss", 1.0, 8.0),
            (Faehigkeit::Salve, _) => ("Salve", 1.0, 11.0),
            (Faehigkeit::Explosivpfeil, _) => ("Sprengschuss", 1.0, 13.0),
            (Faehigkeit::Pfeilregen, _) => ("Himmelsschuss", 1.0, 16.0),
            (Faehigkeit::Klingenhieb, 1) => ("Hieb2", 1.0, 6.0),
            (Faehigkeit::Klingenhieb, 2) => ("Stich", 1.0, 8.0),
            (Faehigkeit::Klingenhieb, _) => ("Hieb1", 1.0, 6.0),
            (Faehigkeit::Wurfdolche, _) => ("Dolchwurf", 1.0, 9.0),
            (Faehigkeit::Rauchbombe, _) => ("Rauchwurf", 1.0, 10.0),
            (Faehigkeit::Schattenklingen, _) => ("Schattenruf", 1.0, 18.0),
        }
    }

    /// So lange holt die Figur aus, bevor die Fähigkeit wirkt (Sekunden, passt zur Animation).
    pub fn ausholen(self, stufe: u8) -> f32 {
        let (_, tempo, bild) = self.animation(stufe);
        bild / 30.0 / tempo
    }

    /// Grundschaden (ohne Waffe) in dieser Stufe – bei Meteorsturm je Meteor, bei Salve je Pfeil,
    /// beim Pfeilregen je Welle.
    pub fn schaden(self, stufe: u8) -> f32 {
        match (self, stufe) {
            (Faehigkeit::Arkangeschoss, 1) => 20.0 * LANZE_FAKTOR,
            (Faehigkeit::Arkangeschoss, _) => 20.0,
            (Faehigkeit::Feuerball, _) => 34.0,
            (Faehigkeit::Frostnova, _) => 18.0,
            (Faehigkeit::Hammerschlag, 2) => 50.0,
            (Faehigkeit::Hammerschlag, _) => 28.0,
            (Faehigkeit::Wurfhammer, _) => 30.0,
            (Faehigkeit::Erdbeben, _) => 36.0,
            (Faehigkeit::Meteorsturm, _) => 30.0,
            (Faehigkeit::Ahnenhammer, _) => 95.0,
            // Kritischer Pfeilschuss: doppelter Schaden
            (Faehigkeit::Pfeilschuss, 1) => 36.0,
            (Faehigkeit::Pfeilschuss, _) => 18.0,
            (Faehigkeit::Salve, _) => 15.0,
            (Faehigkeit::Explosivpfeil, _) => 40.0,
            (Faehigkeit::Pfeilregen, _) => 10.0,
            (Faehigkeit::Klingenhieb, 2) => 50.0,
            (Faehigkeit::Klingenhieb, _) => 24.0,
            (Faehigkeit::Wurfdolche, _) => 16.0,
            (Faehigkeit::Rauchbombe, _) => 20.0,
            (Faehigkeit::Schattenklingen, _) => 14.0,
        }
    }

    pub fn art(self) -> DamageKind {
        match self {
            Faehigkeit::Arkangeschoss => DamageKind::Arcane,
            Faehigkeit::Feuerball | Faehigkeit::Meteorsturm | Faehigkeit::Explosivpfeil => DamageKind::Fire,
            Faehigkeit::Frostnova => DamageKind::Frost,
            Faehigkeit::Hammerschlag | Faehigkeit::Wurfhammer | Faehigkeit::Erdbeben | Faehigkeit::Ahnenhammer | Faehigkeit::Klingenhieb | Faehigkeit::Rauchbombe => {
                DamageKind::Physical
            }
            Faehigkeit::Pfeilschuss | Faehigkeit::Salve | Faehigkeit::Pfeilregen | Faehigkeit::Wurfdolche => DamageKind::Pierce,
            Faehigkeit::Schattenklingen => DamageKind::Arcane,
        }
    }

    /// Nachwirkungen des Haupttreffers in dieser Stufe.
    pub fn wirkung(self, stufe: u8) -> Wirkung {
        match (self, stufe) {
            (Faehigkeit::Feuerball, _) => Wirkung { brand: 8.0, dauer: 4.0, ..Default::default() },
            (Faehigkeit::Frostnova, _) => Wirkung { bremse: 0.5, frost: 1.5, ..Default::default() },
            (Faehigkeit::Hammerschlag, 2) => Wirkung { stun: 0.5, ..Default::default() },
            (Faehigkeit::Wurfhammer, _) => Wirkung { stun: ABPRALL_STUN[0], ..Default::default() },
            (Faehigkeit::Erdbeben, _) => Wirkung { stun: 1.8, ..Default::default() },
            (Faehigkeit::Meteorsturm, _) => Wirkung { brand: 6.0, dauer: 3.0, ..Default::default() },
            (Faehigkeit::Ahnenhammer, _) => Wirkung { stun: 2.5, ..Default::default() },
            (Faehigkeit::Explosivpfeil, _) => Wirkung { stun: 0.8, brand: 5.0, dauer: 2.0, ..Default::default() },
            (Faehigkeit::Pfeilregen, _) => Wirkung { bremse: 0.45, ..Default::default() },
            (Faehigkeit::Wurfdolche, _) => Wirkung { bremse: 0.35, ..Default::default() },
            (Faehigkeit::Rauchbombe, _) => Wirkung { stun: 1.8, ..Default::default() },
            (Faehigkeit::Schattenklingen, _) => Wirkung { bremse: 0.4, ..Default::default() },
            _ => Wirkung::default(),
        }
    }

    /// Farbe von Geschoss, Licht und Funken.
    pub fn farbe(self) -> glam::Vec3 {
        match self {
            Faehigkeit::Arkangeschoss => glam::vec3(0.55, 0.4, 1.0),
            Faehigkeit::Feuerball | Faehigkeit::Meteorsturm => glam::vec3(1.0, 0.45, 0.12),
            Faehigkeit::Frostnova => glam::vec3(0.55, 0.85, 1.0),
            Faehigkeit::Hammerschlag | Faehigkeit::Wurfhammer | Faehigkeit::Ahnenhammer => glam::vec3(1.0, 0.82, 0.5),
            Faehigkeit::Erdbeben => glam::vec3(0.75, 0.55, 0.35),
            Faehigkeit::Pfeilschuss | Faehigkeit::Salve | Faehigkeit::Pfeilregen => glam::vec3(0.9, 0.95, 0.75),
            Faehigkeit::Explosivpfeil => glam::vec3(1.0, 0.5, 0.2),
            Faehigkeit::Klingenhieb => glam::vec3(0.55, 0.85, 1.0),
            Faehigkeit::Wurfdolche => glam::vec3(0.45, 0.95, 0.35),
            Faehigkeit::Rauchbombe => glam::vec3(0.5, 0.45, 0.55),
            Faehigkeit::Schattenklingen => glam::vec3(0.6, 0.35, 1.0),
        }
    }

    /// Was die Fähigkeit besonders macht (für den Tooltip).
    pub fn beschreibung(self) -> &'static str {
        match self {
            Faehigkeit::Arkangeschoss => "Treffer laden arkane Ladungen. Mit dreien wird der nächste Schuss zur Arkanlanze, die alle Gegner in einer Linie durchbohrt.",
            Faehigkeit::Feuerball => "Explodiert und setzt Gegner in Brand. Hinterlässt einen Flammenteppich, der alle darin weiter brennen lässt.",
            Faehigkeit::Frostnova => "Eine Eiswelle friert Gegner ein. Der nächste Treffer zerschmettert das Eis (+50 % Schaden).",
            Faehigkeit::Hammerschlag => "Dreierkombo: zwei Schwünge, dann ein Schmetterschlag von oben, der betäubt und eine Druckwelle auslöst.",
            Faehigkeit::Wurfhammer => "Betäubt, prallt auf bis zu zwei weitere Gegner ab und kehrt in die Hand zurück.",
            Faehigkeit::Erdbeben => "Sprung und Aufschlag: innen mehr Schaden und längere Betäubung. Zwei Nachbeben verlangsamen.",
            Faehigkeit::Meteorsturm => "ULTIMATIV: Ein Feuerkreis öffnet sich am Himmel, acht Meteore stürzen auf das Ziel und setzen alles in Brand.",
            Faehigkeit::Ahnenhammer => "ULTIMATIV: Die Ahnen schleudern einen riesigen Geisterhammer vom Himmel. Innen gewaltiger Schaden, alle werden lange betäubt.",
            Faehigkeit::Pfeilschuss => "Schneller, weiter Schuss. Jeder fünfte Pfeil trifft im Schnitt kritisch (doppelter Schaden).",
            Faehigkeit::Salve => "Fünf Pfeile im Fächer – ideal gegen Gruppen, aus der Nähe treffen mehrere dasselbe Ziel.",
            Faehigkeit::Explosivpfeil => "Der Pfeil bleibt stecken und explodiert kurz darauf: Flächenschaden, Brand, kurze Betäubung.",
            Faehigkeit::Pfeilregen => "ULTIMATIV: Ein Schuss in den Himmel – drei Sekunden lang regnen Pfeile auf das Ziel und bremsen alles darin.",
            Faehigkeit::Klingenhieb => "Dreierkombo mit der Runenklinge: zwei schnelle Hiebe, dann ein Stich mit doppeltem Schaden.",
            Faehigkeit::Wurfdolche => "Drei vergiftete Dolche im Fächer. Das Gift verlangsamt jeden Getroffenen.",
            Faehigkeit::Rauchbombe => "Eine Rauchbombe zu deinen Füßen: Alles im Rauch wird getroffen und betäubt.",
            Faehigkeit::Schattenklingen => "ULTIMATIV: Ein Wirbel aus Schattenklingen zerschneidet alles am Ziel und bremst es. Zum Schluss explodiert er und betäubt.",
        }
    }

    /// Zeile für den Tooltip: Schaden, Abklingzeit und Wirkung.
    pub fn werte_zeile(self) -> String {
        let w = self.wirkung(0);
        let schaden = match self {
            Faehigkeit::Hammerschlag => format!("{:.0}/{:.0}/{:.0} Schaden", self.schaden(0), self.schaden(1), self.schaden(2)),
            Faehigkeit::Meteorsturm => format!("{METEORE} × {:.0} Schaden", self.schaden(0)),
            Faehigkeit::Salve => format!("{SALVE_PFEILE} × {:.0} Schaden", self.schaden(0)),
            Faehigkeit::Pfeilregen => format!("{REGEN_WELLEN} × {:.0} Schaden", self.schaden(0)),
            Faehigkeit::Klingenhieb => format!("{:.0} Schaden, Stich {:.0}", self.schaden(0), self.schaden(2)),
            Faehigkeit::Wurfdolche => format!("{DOLCHE} × {:.0} Schaden", self.schaden(0)),
            Faehigkeit::Schattenklingen => format!("{KLINGEN_WELLEN} × {:.0} + {:.0} Schaden", self.schaden(0), self.schaden(0) * KLINGEN_SCHLUSS),
            _ => format!("{:.0} Schaden", self.schaden(0)),
        };
        let mut teile = vec![schaden, format!("{:.1} s Abklingzeit", self.abklingen(0) as f32 / 60.0)];
        match self.form() {
            Form::Geschoss { flaeche, .. } if flaeche > 0.0 => teile.push(format!("{flaeche:.1} m Umkreis")),
            Form::UmSich { radius } => teile.push(format!("{radius:.1} m um dich")),
            Form::Nahkampf { weite, .. } => teile.push(format!("{weite:.1} m vor dir")),
            Form::Flaeche { radius, .. } => teile.push(format!("{radius:.0} m Umkreis am Ziel")),
            _ => {}
        }
        if w.frost > 0.0 {
            teile.push(format!("friert {:.1} s ein", w.frost));
        } else if w.bremse > 0.0 {
            teile.push(format!("verlangsamt um {:.0} %", w.bremse * 100.0));
        }
        if w.stun > 0.0 {
            teile.push(format!("betäubt {:.1} s", w.stun));
        }
        if w.brand > 0.0 {
            teile.push(format!("Brand {:.0}/s für {:.0} s", w.brand, w.dauer));
        }
        teile.join(" · ")
    }
}

/// Hammerschlag: welche Stufe der nächste Schlag hat. `letzter` = Takt des letzten Schlags und
/// seine Stufe, `jetzt` = aktueller Takt.
pub fn kombo_stufe(letzter: Option<(u64, u8)>, jetzt: u64) -> u8 {
    kombo_stufe_von(Faehigkeit::Hammerschlag, letzter, jetzt)
}

/// Dreierkombo einer Fähigkeit (Hammerschlag, Klingenhieb).
pub fn kombo_stufe_von(art: Faehigkeit, letzter: Option<(u64, u8)>, jetzt: u64) -> u8 {
    match letzter {
        Some((tick, stufe)) if jetzt <= tick + art.abklingen(stufe) + KOMBO_FENSTER => (stufe + 1) % 3,
        _ => 0,
    }
}

impl Faehigkeit {
    /// Hat die Fähigkeit eine Dreierkombo?
    pub fn ist_kombo(self) -> bool {
        matches!(self, Faehigkeit::Hammerschlag | Faehigkeit::Klingenhieb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jede_klasse_hat_vier_verschiedene_faehigkeiten() {
        for class in CharacterClass::ALL {
            let f = Faehigkeit::der_klasse(class);
            for i in 0..4 {
                for j in 0..i {
                    assert!(f[i] != f[j], "{class:?}: doppelt {:?}", f[i]);
                }
            }
            // Die erste ist der Standardangriff: schnell wieder bereit; die vierte ist ultimativ
            assert!(f[0].abklingen(0) < 60, "{:?} zu langsam", f[0]);
            assert!(f[2].abklingen(0) > f[0].abklingen(0));
            assert!(f[3].ist_ultimativ() && f[3].abklingen(0) >= 30 * 60, "{:?} ist keine ultimative Fähigkeit", f[3]);
            assert!(f[..3].iter().all(|a| !a.ist_ultimativ()));
            for art in f {
                for stufe in 0..3 {
                    let aus = art.ausholen(stufe);
                    let grenze = if art.ist_ultimativ() { 0.9 } else { 0.7 };
                    assert!((0.1..grenze).contains(&aus), "{art:?} holt {aus} s aus");
                }
            }
        }
    }

    #[test]
    fn kombo_zaehlt_hoch_und_verfaellt() {
        assert_eq!(kombo_stufe(None, 100), 0);
        assert_eq!(kombo_stufe(Some((100, 0)), 140), 1);
        assert_eq!(kombo_stufe(Some((140, 1)), 180), 2);
        assert_eq!(kombo_stufe(Some((180, 2)), 250), 0, "nach dem Schmetterschlag beginnt sie neu");
        assert_eq!(kombo_stufe(Some((100, 0)), 100 + 30 + KOMBO_FENSTER + 1), 0, "zu spät: neu anfangen");
    }

    #[test]
    fn meteore_und_pfeilregen_fallen_nacheinander() {
        assert!(meteor_zeit(0) > Faehigkeit::Meteorsturm.ausholen(0));
        assert!((meteor_zeit(METEORE - 1) - meteor_zeit(0) - METEOR_DAUER * (METEORE - 1) as f32 / METEORE as f32).abs() < 1e-4);
        assert!(regen_zeit(REGEN_WELLEN - 1) < 5.0);
        assert!(klingen_zeit(0) > Faehigkeit::Schattenklingen.ausholen(0) && klingen_schluss() < 4.0);
    }
}
