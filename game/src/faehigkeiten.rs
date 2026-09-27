//! Fähigkeiten der Spielfiguren: jede Klasse hat drei, auf den Plätzen 3–5 der Auswahlleiste.
//!
//! Magier: Arkangeschoss (lädt arkane Ladungen, mit dreien wird es zur Arkanlanze), Feuerball
//! (Explosion, Brand, Flammenteppich), Frostnova (Eiswelle, friert ein). Zwerg: Hammerschlag
//! (Dreierkombo mit Schmetterschlag), Wurfhammer (prallt ab und kehrt zurück), Erdbeben (Sprung,
//! Aufschlag in zwei Ringen, Nachbeben). Gefrorene zerschmettern beim nächsten Treffer (+50 %).
//! Der Server rechnet alles nach; die Werte stehen hier an einer Stelle (Konzept:
//! `docs/konzept_faehigkeiten.md`).

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

impl Faehigkeit {
    /// Die drei Fähigkeiten einer Klasse (Platz 0, 1, 2).
    pub fn der_klasse(class: CharacterClass) -> [Faehigkeit; 3] {
        match class {
            CharacterClass::Zwerg => [Faehigkeit::Hammerschlag, Faehigkeit::Wurfhammer, Faehigkeit::Erdbeben],
            _ => [Faehigkeit::Arkangeschoss, Faehigkeit::Feuerball, Faehigkeit::Frostnova],
        }
    }

    pub fn von(class: CharacterClass, platz: u8) -> Faehigkeit {
        Self::der_klasse(class)[(platz as usize).min(2)]
    }

    pub fn label(self) -> &'static str {
        match self {
            Faehigkeit::Arkangeschoss => "Arkangeschoss",
            Faehigkeit::Feuerball => "Feuerball",
            Faehigkeit::Frostnova => "Frostnova",
            Faehigkeit::Hammerschlag => "Hammerschlag",
            Faehigkeit::Wurfhammer => "Wurfhammer",
            Faehigkeit::Erdbeben => "Erdbeben",
        }
    }

    /// Symbol in `game/assets/icons/` (gerendert von `art/icons/faehigkeiten.py`).
    pub fn icon_file(self) -> &'static str {
        match self {
            Faehigkeit::Arkangeschoss => "faehigkeit_arkangeschoss",
            Faehigkeit::Feuerball => "faehigkeit_feuerball",
            Faehigkeit::Frostnova => "faehigkeit_frostnova",
            Faehigkeit::Hammerschlag => "faehigkeit_hammerschlag",
            Faehigkeit::Wurfhammer => "faehigkeit_wurfhammer",
            Faehigkeit::Erdbeben => "faehigkeit_erdbeben",
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
        }
    }

    /// Wie weit Geschosse fliegen (Meter).
    pub fn reichweite(self) -> f32 {
        match self {
            Faehigkeit::Arkangeschoss => 45.0,
            Faehigkeit::Feuerball => 40.0,
            Faehigkeit::Wurfhammer => 32.0,
            _ => 0.0,
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
        }
    }

    /// Animation (Clip im Figurenmodell), Tempo und das Bild (30 je Sekunde), in dem die
    /// Fähigkeit wirkt – siehe `art/lib/figuren.py` und `art/lib/zwerg.py`.
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
        }
    }

    /// So lange holt die Figur aus, bevor die Fähigkeit wirkt (Sekunden, passt zur Animation).
    pub fn ausholen(self, stufe: u8) -> f32 {
        let (_, tempo, bild) = self.animation(stufe);
        bild / 30.0 / tempo
    }

    /// Grundschaden (ohne Waffe) in dieser Stufe.
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
        }
    }

    pub fn art(self) -> DamageKind {
        match self {
            Faehigkeit::Arkangeschoss => DamageKind::Arcane,
            Faehigkeit::Feuerball => DamageKind::Fire,
            Faehigkeit::Frostnova => DamageKind::Frost,
            Faehigkeit::Hammerschlag | Faehigkeit::Wurfhammer | Faehigkeit::Erdbeben => DamageKind::Physical,
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
            _ => Wirkung::default(),
        }
    }

    /// Farbe von Geschoss, Licht und Funken.
    pub fn farbe(self) -> glam::Vec3 {
        match self {
            Faehigkeit::Arkangeschoss => glam::vec3(0.55, 0.4, 1.0),
            Faehigkeit::Feuerball => glam::vec3(1.0, 0.45, 0.12),
            Faehigkeit::Frostnova => glam::vec3(0.55, 0.85, 1.0),
            Faehigkeit::Hammerschlag | Faehigkeit::Wurfhammer => glam::vec3(1.0, 0.82, 0.5),
            Faehigkeit::Erdbeben => glam::vec3(0.75, 0.55, 0.35),
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
        }
    }

    /// Zeile für den Tooltip: Schaden, Abklingzeit und Wirkung.
    pub fn werte_zeile(self) -> String {
        let w = self.wirkung(0);
        let schaden = match self {
            Faehigkeit::Hammerschlag => format!("{:.0}/{:.0}/{:.0} Schaden", self.schaden(0), self.schaden(1), self.schaden(2)),
            _ => format!("{:.0} Schaden", self.schaden(0)),
        };
        let mut teile = vec![schaden, format!("{:.1} s Abklingzeit", self.abklingen(0) as f32 / 60.0)];
        match self.form() {
            Form::Geschoss { flaeche, .. } if flaeche > 0.0 => teile.push(format!("{flaeche:.1} m Umkreis")),
            Form::UmSich { radius } => teile.push(format!("{radius:.1} m um dich")),
            Form::Nahkampf { weite, .. } => teile.push(format!("{weite:.1} m vor dir")),
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
    match letzter {
        Some((tick, stufe)) if jetzt <= tick + Faehigkeit::Hammerschlag.abklingen(stufe) + KOMBO_FENSTER => (stufe + 1) % 3,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jede_klasse_hat_drei_verschiedene_faehigkeiten() {
        for class in CharacterClass::ALL {
            let f = Faehigkeit::der_klasse(class);
            assert!(f[0] != f[1] && f[1] != f[2] && f[0] != f[2], "{class:?}");
            // Die erste ist der Standardangriff: schnell wieder bereit
            assert!(f[0].abklingen(0) < 60, "{:?} zu langsam", f[0]);
            assert!(f[2].abklingen(0) > f[0].abklingen(0));
            for art in f {
                for stufe in 0..3 {
                    let aus = art.ausholen(stufe);
                    assert!((0.1..0.7).contains(&aus), "{art:?} holt {aus} s aus");
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
}
