//! Waffen: je Klasse eine Startwaffe (Wanderstab, Schmiedehammer) und fünf Waffen, die Gegner in
//! der Wildnis fallen lassen – in vier Seltenheiten, mit mehr Schaden, kürzeren Abklingzeiten und
//! einem Bonus auf eine Fähigkeit. Die Waffe sieht man in der Hand der Figur (Anbauteil im
//! Figurenmodell) und als kleines Modell am Boden, solange sie dort liegt.

use serde::{Deserialize, Serialize};

use crate::faehigkeiten::Faehigkeit;
use crate::protocol::CharacterClass;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Seltenheit {
    Gewoehnlich,
    Selten,
    Episch,
    Legendaer,
}

impl Seltenheit {
    pub fn label(self) -> &'static str {
        match self {
            Seltenheit::Gewoehnlich => "Gewöhnlich",
            Seltenheit::Selten => "Selten",
            Seltenheit::Episch => "Episch",
            Seltenheit::Legendaer => "Legendär",
        }
    }

    /// Farbe von Name, Rahmen und Lichtsäule (RGB 0..1).
    pub fn farbe(self) -> [f32; 3] {
        match self {
            Seltenheit::Gewoehnlich => [0.9, 0.9, 0.86],
            Seltenheit::Selten => [0.35, 0.62, 1.0],
            Seltenheit::Episch => [0.72, 0.38, 1.0],
            Seltenheit::Legendaer => [1.0, 0.62, 0.15],
        }
    }

    /// Gold, wenn man eine Waffe aufhebt, die man schon hat.
    pub fn gold_fuer_doppelte(self) -> u32 {
        match self {
            Seltenheit::Gewoehnlich => 15,
            Seltenheit::Selten => 40,
            Seltenheit::Episch => 90,
            Seltenheit::Legendaer => 200,
        }
    }
}

pub struct Waffe {
    /// 1–15 (0 ist die Startwaffe)
    pub id: u8,
    pub name: &'static str,
    pub klasse: CharacterClass,
    pub seltenheit: Seltenheit,
    /// Faktor auf allen Schaden und auf alle Abklingzeiten
    pub schaden: f32,
    pub abklingen: f32,
    /// Eine Fähigkeit wirkt stärker: Schaden und Nachwirkung (Brand, Verlangsamung, Betäubung) × Faktor
    pub bonus: Option<(Faehigkeit, f32)>,
    /// Name des Anbauteils im Figurenmodell und Dateiname von Bodenmodell und Symbol
    pub datei: &'static str,
    pub beschreibung: &'static str,
}

use Faehigkeit::*;
use Seltenheit::*;

pub const WAFFEN: [Waffe; 15] = [
    Waffe { id: 1, name: "Eichenstab", klasse: CharacterClass::Mage, seltenheit: Gewoehnlich, schaden: 1.12, abklingen: 1.0, bonus: None, datei: "stab_eiche",
            beschreibung: "Ein knorriger Eichenstab mit Bernstein. Etwas mehr Kraft in jedem Zauber." },
    Waffe { id: 2, name: "Glutstab", klasse: CharacterClass::Mage, seltenheit: Selten, schaden: 1.1, abklingen: 1.0, bonus: Some((Feuerball, 1.45)), datei: "stab_glut",
            beschreibung: "In seiner Krone glimmt ein Feuerstein. Feuerbälle brennen heißer und länger." },
    Waffe { id: 3, name: "Froststab", klasse: CharacterClass::Mage, seltenheit: Selten, schaden: 1.1, abklingen: 1.0, bonus: Some((Frostnova, 1.45)), datei: "stab_frost",
            beschreibung: "Eiskristalle wachsen um die Spitze. Die Frostnova trifft härter und bremst stärker." },
    Waffe { id: 4, name: "Sturmstab", klasse: CharacterClass::Mage, seltenheit: Episch, schaden: 1.15, abklingen: 0.75, bonus: None, datei: "stab_sturm",
            beschreibung: "Blitze tanzen um den Silberring. Alle Zauber sind viel schneller wieder bereit." },
    Waffe { id: 5, name: "Sternenstab", klasse: CharacterClass::Mage, seltenheit: Legendaer, schaden: 1.35, abklingen: 0.85, bonus: Some((Arkangeschoss, 1.25)),
            datei: "stab_sternen", beschreibung: "Ein gefangener Stern in goldener Fassung. Mehr Schaden, schnellere Zauber, stärkere Arkangeschosse." },
    Waffe { id: 6, name: "Eisenhammer", klasse: CharacterClass::Zwerg, seltenheit: Gewoehnlich, schaden: 1.12, abklingen: 1.0, bonus: None, datei: "hammer_eisen",
            beschreibung: "Grob geschmiedet, aber schwer. Etwas mehr Wucht in jedem Schlag." },
    Waffe { id: 7, name: "Runenhammer", klasse: CharacterClass::Zwerg, seltenheit: Selten, schaden: 1.1, abklingen: 1.0, bonus: Some((Wurfhammer, 1.45)), datei: "hammer_runen",
            beschreibung: "Leuchtende Runen lassen ihn zurückkehren. Der Wurfhammer trifft härter und betäubt länger." },
    Waffe { id: 8, name: "Streithammer", klasse: CharacterClass::Zwerg, seltenheit: Selten, schaden: 1.1, abklingen: 1.0, bonus: Some((Hammerschlag, 1.45)), datei: "hammer_streit",
            beschreibung: "Mit Dorn und Schlagfläche. Der Hammerschlag trifft viel härter." },
    Waffe { id: 9, name: "Donnerhammer", klasse: CharacterClass::Zwerg, seltenheit: Episch, schaden: 1.15, abklingen: 0.85, bonus: Some((Erdbeben, 1.4)), datei: "hammer_donner",
            beschreibung: "Er grollt bei jedem Schlag. Erdbeben sind stärker und alles ist schneller bereit." },
    Waffe { id: 10, name: "Drachenhammer", klasse: CharacterClass::Zwerg, seltenheit: Legendaer, schaden: 1.35, abklingen: 0.85, bonus: Some((Hammerschlag, 1.25)),
            datei: "hammer_drachen", beschreibung: "Aus Drachenschuppe geschmiedet. Mehr Schaden, schnellere Fähigkeiten, wuchtigere Hammerschläge." },
    Waffe { id: 11, name: "Eibenbogen", klasse: CharacterClass::Bogenschuetze, seltenheit: Gewoehnlich, schaden: 1.12, abklingen: 1.0, bonus: None, datei: "bogen_eibe",
            beschreibung: "Zäh und biegsam, mit Hornspitzen. Etwas mehr Kraft in jedem Schuss." },
    Waffe { id: 12, name: "Langbogen", klasse: CharacterClass::Bogenschuetze, seltenheit: Selten, schaden: 1.1, abklingen: 1.0, bonus: Some((Pfeilschuss, 1.4)), datei: "bogen_lang",
            beschreibung: "Mannshoch und kraftvoll. Pfeilschüsse treffen härter." },
    Waffe { id: 13, name: "Glutbogen", klasse: CharacterClass::Bogenschuetze, seltenheit: Selten, schaden: 1.1, abklingen: 1.0, bonus: Some((Explosivpfeil, 1.45)), datei: "bogen_glut",
            beschreibung: "Die Sehne glimmt wie Kohle. Explosivpfeile zünden heftiger und brennen länger." },
    Waffe { id: 14, name: "Elfenbogen", klasse: CharacterClass::Bogenschuetze, seltenheit: Episch, schaden: 1.15, abklingen: 0.8, bonus: Some((Salve, 1.35)), datei: "bogen_elfen",
            beschreibung: "Aus hellem Silberholz mit Blattranken. Salven treffen härter, alles ist schneller bereit." },
    Waffe { id: 15, name: "Sturmbogen", klasse: CharacterClass::Bogenschuetze, seltenheit: Legendaer, schaden: 1.35, abklingen: 0.85, bonus: Some((Pfeilregen, 1.3)),
            datei: "bogen_sturm", beschreibung: "Blitze knistern in der Sehne. Mehr Schaden, schnellere Fähigkeiten, ein gewaltiger Pfeilregen." },
];

pub fn waffe(id: u8) -> Option<&'static Waffe> {
    WAFFEN.iter().find(|w| w.id == id)
}

/// Name der Startwaffe einer Klasse.
pub fn startwaffe(class: CharacterClass) -> &'static str {
    match class {
        CharacterClass::Zwerg => "Schmiedehammer",
        CharacterClass::Bogenschuetze => "Jagdbogen",
        _ => "Wanderstab",
    }
}

/// Die Waffe in der Hand, wenn sie zur Klasse passt (sonst die Startwaffe).
pub fn ausgeruestet(id: u8, class: CharacterClass) -> Option<&'static Waffe> {
    waffe(id).filter(|w| w.klasse == class)
}

/// Wie eine Fähigkeit mit einer Waffe wirkt: Schaden, Nachwirkung, Abklingzeit (Faktoren).
pub fn faktoren(id: u8, class: CharacterClass, art: Faehigkeit) -> (f32, f32, f32) {
    let Some(w) = ausgeruestet(id, class) else { return (1.0, 1.0, 1.0) };
    let bonus = w.bonus.filter(|b| b.0 == art).map_or(1.0, |b| b.1);
    (w.schaden * bonus, bonus, w.abklingen)
}

/// Werte einer Waffe für Tooltip und Hinweise.
pub fn werte_zeile(w: &Waffe) -> String {
    let mut teile = vec![format!("Schaden +{:.0} %", (w.schaden - 1.0) * 100.0)];
    if w.abklingen < 1.0 {
        teile.push(format!("Abklingzeiten −{:.0} %", (1.0 - w.abklingen) * 100.0));
    }
    if let Some((art, f)) = w.bonus {
        teile.push(format!("{} +{:.0} %", art.label(), (f - 1.0) * 100.0));
    }
    teile.join(" · ")
}

/// Welche Seltenheit ein Lager der Gefahr 1–3 fallen lässt (Wurf 0..1).
pub fn seltenheit_fuer(gefahr: u8, wurf: f32) -> Seltenheit {
    let grenzen: [f32; 3] = match gefahr {
        1 => [0.65, 0.95, 1.0],
        2 => [0.35, 0.8, 0.97],
        _ => [0.15, 0.55, 0.85],
    };
    if wurf < grenzen[0] {
        Gewoehnlich
    } else if wurf < grenzen[1] {
        Selten
    } else if wurf < grenzen[2] {
        Episch
    } else {
        Legendaer
    }
}

/// Chance, dass ein Besiegter eine Waffe fallen lässt.
pub fn waffen_chance(gefahr: u8, anfuehrer: bool) -> f32 {
    match (gefahr, anfuehrer) {
        (1, false) => 0.03,
        (1, true) => 0.15,
        (2, false) => 0.05,
        (2, true) => 0.25,
        (_, false) => 0.08,
        (_, true) => 0.4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuenf_waffen_je_klasse_in_allen_seltenheiten() {
        for class in CharacterClass::ALL {
            let eigene: Vec<&Waffe> = WAFFEN.iter().filter(|w| w.klasse == class).collect();
            assert_eq!(eigene.len(), 5, "{class:?}");
            assert!(eigene.iter().any(|w| w.seltenheit == Legendaer));
            assert!(eigene.iter().all(|w| w.schaden > 1.0 && w.abklingen <= 1.0));
        }
        let mut ids: Vec<u8> = WAFFEN.iter().map(|w| w.id).collect();
        ids.dedup();
        assert_eq!(ids, (1..=WAFFEN.len() as u8).collect::<Vec<u8>>());
        // Erbeutete Waffen stehen als Bits in einem u16
        assert!(WAFFEN.len() < 16);
    }

    #[test]
    fn gefaehrliche_lager_haben_bessere_waffen() {
        assert_eq!(seltenheit_fuer(1, 0.99), Seltenheit::Episch);
        assert_eq!(seltenheit_fuer(3, 0.99), Seltenheit::Legendaer);
        assert!(waffen_chance(3, true) > waffen_chance(1, true));
        // Fremde Waffen wirken nicht
        assert_eq!(faktoren(5, CharacterClass::Zwerg, Faehigkeit::Hammerschlag), (1.0, 1.0, 1.0));
        let (s, n, a) = faktoren(2, CharacterClass::Mage, Faehigkeit::Feuerball);
        assert!(s > 1.5 && n > 1.4 && a == 1.0);
    }
}
