//! Fähigkeiten der Spielfiguren: jede Klasse hat drei, auf den Plätzen 3–5 der Auswahlleiste.
//!
//! Magier: Arkangeschoss (schnell, einzeln), Feuerball (Flächenschaden, Brand), Frostnova (um sich,
//! verlangsamt). Zwerg: Hammerschlag (Nahkampf, Kegel), Wurfhammer (betäubt), Erdbeben (um sich,
//! betäubt). Der Server rechnet alles nach; die Werte stehen hier an einer Stelle.

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
            Faehigkeit::Arkangeschoss => Form::Geschoss { tempo: 34.0, flaeche: 0.0 },
            Faehigkeit::Feuerball => Form::Geschoss { tempo: 24.0, flaeche: 3.8 },
            Faehigkeit::Frostnova => Form::UmSich { radius: 7.5 },
            Faehigkeit::Hammerschlag => Form::Nahkampf { weite: 3.4, winkel: 65.0 },
            Faehigkeit::Wurfhammer => Form::Geschoss { tempo: 26.0, flaeche: 0.0 },
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

    /// Abklingzeit in Takten (60 je Sekunde).
    pub fn abklingen(self) -> u64 {
        match self {
            Faehigkeit::Arkangeschoss => 42,
            Faehigkeit::Feuerball => 5 * 60,
            Faehigkeit::Frostnova => 12 * 60,
            Faehigkeit::Hammerschlag => 48,
            Faehigkeit::Wurfhammer => 5 * 60,
            Faehigkeit::Erdbeben => 12 * 60,
        }
    }

    /// So lange holt die Figur aus, bevor die Fähigkeit wirkt (Sekunden, passt zur Animation).
    pub fn ausholen(self) -> f32 {
        match self {
            Faehigkeit::Arkangeschoss | Faehigkeit::Feuerball => 0.22,
            Faehigkeit::Frostnova => 0.3,
            Faehigkeit::Hammerschlag => 0.32,
            Faehigkeit::Wurfhammer => 0.3,
            Faehigkeit::Erdbeben => 0.45,
        }
    }

    /// Animation (Clip im Figurenmodell) und Tempo.
    pub fn animation(self) -> (&'static str, f32) {
        match self {
            Faehigkeit::Arkangeschoss | Faehigkeit::Feuerball => ("Zaubern", 1.35),
            Faehigkeit::Frostnova => ("Zaubern", 1.1),
            Faehigkeit::Hammerschlag => ("Hieb", 1.5),
            Faehigkeit::Wurfhammer => ("Werfen", 1.4),
            Faehigkeit::Erdbeben => ("Zaubern", 1.2),
        }
    }

    pub fn schaden(self) -> f32 {
        match self {
            Faehigkeit::Arkangeschoss => 22.0,
            Faehigkeit::Feuerball => 34.0,
            Faehigkeit::Frostnova => 20.0,
            Faehigkeit::Hammerschlag => 36.0,
            Faehigkeit::Wurfhammer => 28.0,
            Faehigkeit::Erdbeben => 32.0,
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

    /// Nachwirkungen: Verlangsamung (Anteil), Betäubung (s), Brand (Schaden/s, Dauer s).
    pub fn wirkung(self) -> (f32, f32, f32, f32) {
        match self {
            Faehigkeit::Feuerball => (0.0, 0.0, 8.0, 4.0),
            Faehigkeit::Frostnova => (0.6, 0.0, 0.0, 0.0),
            Faehigkeit::Wurfhammer => (0.0, 1.2, 0.0, 0.0),
            Faehigkeit::Erdbeben => (0.0, 1.6, 0.0, 0.0),
            _ => (0.0, 0.0, 0.0, 0.0),
        }
    }

    /// Farbe von Geschoss, Licht und Funken.
    pub fn farbe(self) -> glam::Vec3 {
        match self {
            Faehigkeit::Arkangeschoss => glam::vec3(0.45, 0.4, 1.0),
            Faehigkeit::Feuerball => glam::vec3(1.0, 0.45, 0.12),
            Faehigkeit::Frostnova => glam::vec3(0.55, 0.85, 1.0),
            Faehigkeit::Hammerschlag | Faehigkeit::Wurfhammer => glam::vec3(0.95, 0.8, 0.55),
            Faehigkeit::Erdbeben => glam::vec3(0.75, 0.55, 0.35),
        }
    }

    /// Zeile für den Tooltip: Schaden, Abklingzeit und Wirkung.
    pub fn werte_zeile(self) -> String {
        let (bremse, stun, brand, dauer) = self.wirkung();
        let mut teile = vec![format!("{:.0} Schaden", self.schaden()), format!("{:.1} s Abklingzeit", self.abklingen() as f32 / 60.0)];
        match self.form() {
            Form::Geschoss { flaeche, .. } if flaeche > 0.0 => teile.push(format!("{flaeche:.1} m Umkreis")),
            Form::UmSich { radius } => teile.push(format!("{radius:.1} m um dich")),
            Form::Nahkampf { weite, .. } => teile.push(format!("{weite:.1} m vor dir")),
            _ => {}
        }
        if bremse > 0.0 {
            teile.push(format!("verlangsamt um {:.0} %", bremse * 100.0));
        }
        if stun > 0.0 {
            teile.push(format!("betäubt {stun:.1} s"));
        }
        if brand > 0.0 {
            teile.push(format!("Brand {brand:.0}/s für {dauer:.0} s"));
        }
        teile.join(" · ")
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
            assert!(f[0].abklingen() < 60, "{:?} zu langsam", f[0]);
            assert!(f[2].abklingen() > f[0].abklingen());
        }
    }
}
