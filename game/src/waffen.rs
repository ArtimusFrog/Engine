//! Waffen: je Klasse eine Startwaffe (Wanderstab, Schmiedehammer, Mondbogen, Runenklinge) und fünf Waffen, die Gegner in
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

    /// Grundwert in Gold (Waffen und Rüstung rechnen ihren Wert davon aus).
    pub fn grundwert(self) -> u32 {
        match self {
            Seltenheit::Gewoehnlich => 40,
            Seltenheit::Selten => 120,
            Seltenheit::Episch => 300,
            Seltenheit::Legendaer => 700,
        }
    }
}

pub struct Waffe {
    /// 1–20 (0 ist die Startwaffe)
    pub id: u8,
    pub name: &'static str,
    pub klasse: CharacterClass,
    pub seltenheit: Seltenheit,
    /// Faktor auf allen Schaden und auf alle Abklingzeiten
    pub schaden: f32,
    pub abklingen: f32,
    /// Chance auf kritische Treffer (×1,5 Schaden)
    pub krit: f32,
    /// Eine Fähigkeit wirkt stärker: Schaden und Nachwirkung (Brand, Verlangsamung, Betäubung) × Faktor
    pub bonus: Option<(Faehigkeit, f32)>,
    /// Name des Anbauteils im Figurenmodell und Dateiname von Bodenmodell und Symbol
    pub datei: &'static str,
    pub beschreibung: &'static str,
}

use Faehigkeit::*;
use Seltenheit::*;

/// Gleiche Stufen für alle Klassen: Gewöhnlich +12 % Schaden; Selten +8 % und eine Fähigkeit +45 %;
/// Episch +15 %, −15 % Abklingzeiten und eine Fähigkeit +30 %; Legendär +28 %, −15 % und die
/// ultimative Fähigkeit +30 %. Dazu je nach Stufe 3–10 % Chance auf kritische Treffer (×1,5).
pub const WAFFEN: [Waffe; 20] = [
    Waffe { id: 1, name: "Eichenstab", klasse: CharacterClass::Mage, seltenheit: Gewoehnlich, schaden: 1.12, abklingen: 1.0, krit: 0.03, bonus: None,
            datei: "stab_eiche", beschreibung: "Ein knorriger Eichenstab mit Bernstein. Etwas mehr Kraft in jedem Zauber." },
    Waffe { id: 2, name: "Glutstab", klasse: CharacterClass::Mage, seltenheit: Selten, schaden: 1.08, abklingen: 1.0, krit: 0.03, bonus: Some((Feuerball, 1.45)),
            datei: "stab_glut", beschreibung: "In seiner Krone glimmt ein Feuerstein. Feuerbälle brennen heißer und länger." },
    Waffe { id: 3, name: "Froststab", klasse: CharacterClass::Mage, seltenheit: Selten, schaden: 1.08, abklingen: 1.0, krit: 0.03, bonus: Some((Frostnova, 1.45)),
            datei: "stab_frost", beschreibung: "Eiskristalle wachsen um die Spitze. Die Frostnova trifft härter und friert länger ein." },
    Waffe { id: 4, name: "Sturmstab", klasse: CharacterClass::Mage, seltenheit: Episch, schaden: 1.15, abklingen: 0.85, krit: 0.06, bonus: Some((Arkangeschoss, 1.3)),
            datei: "stab_sturm", beschreibung: "Blitze tanzen um den Silberring. Alle Zauber sind schneller bereit, Arkangeschosse treffen härter." },
    Waffe { id: 5, name: "Sternenstab", klasse: CharacterClass::Mage, seltenheit: Legendaer, schaden: 1.28, abklingen: 0.85, krit: 0.1, bonus: Some((Meteorsturm, 1.3)),
            datei: "stab_sternen", beschreibung: "Ein gefangener Stern in goldener Fassung. Mehr Schaden, schnellere Zauber, ein gewaltiger Meteorsturm." },
    Waffe { id: 6, name: "Eisenhammer", klasse: CharacterClass::Zwerg, seltenheit: Gewoehnlich, schaden: 1.12, abklingen: 1.0, krit: 0.03, bonus: None,
            datei: "hammer_eisen", beschreibung: "Grob geschmiedet, aber schwer. Etwas mehr Wucht in jedem Schlag." },
    Waffe { id: 7, name: "Runenhammer", klasse: CharacterClass::Zwerg, seltenheit: Selten, schaden: 1.08, abklingen: 1.0, krit: 0.03, bonus: Some((Wurfhammer, 1.45)),
            datei: "hammer_runen", beschreibung: "Leuchtende Runen lassen ihn zurückkehren. Der Wurfhammer trifft härter und betäubt länger." },
    Waffe { id: 8, name: "Streithammer", klasse: CharacterClass::Zwerg, seltenheit: Selten, schaden: 1.08, abklingen: 1.0, krit: 0.03, bonus: Some((Hammerschlag, 1.45)),
            datei: "hammer_streit", beschreibung: "Mit Dorn und Schlagfläche. Der Hammerschlag trifft viel härter." },
    Waffe { id: 9, name: "Donnerhammer", klasse: CharacterClass::Zwerg, seltenheit: Episch, schaden: 1.15, abklingen: 0.85, krit: 0.06, bonus: Some((Erdbeben, 1.3)),
            datei: "hammer_donner", beschreibung: "Er grollt bei jedem Schlag. Erdbeben sind stärker und alles ist schneller bereit." },
    Waffe { id: 10, name: "Drachenhammer", klasse: CharacterClass::Zwerg, seltenheit: Legendaer, schaden: 1.28, abklingen: 0.85, krit: 0.1, bonus: Some((Ahnenhammer, 1.3)),
            datei: "hammer_drachen", beschreibung: "Aus Drachenschuppe geschmiedet. Mehr Schaden, schnellere Fähigkeiten, ein wuchtigerer Ahnenhammer." },
    Waffe { id: 11, name: "Eibenbogen", klasse: CharacterClass::Bogenschuetze, seltenheit: Gewoehnlich, schaden: 1.12, abklingen: 1.0, krit: 0.03, bonus: None,
            datei: "bogen_eibe", beschreibung: "Zäh und biegsam, mit Hornspitzen. Etwas mehr Kraft in jedem Schuss." },
    Waffe { id: 12, name: "Langbogen", klasse: CharacterClass::Bogenschuetze, seltenheit: Selten, schaden: 1.08, abklingen: 1.0, krit: 0.03, bonus: Some((Pfeilschuss, 1.45)),
            datei: "bogen_lang", beschreibung: "Mannshoch und kraftvoll. Pfeilschüsse treffen härter." },
    Waffe { id: 13, name: "Glutbogen", klasse: CharacterClass::Bogenschuetze, seltenheit: Selten, schaden: 1.08, abklingen: 1.0, krit: 0.03, bonus: Some((Explosivpfeil, 1.45)),
            datei: "bogen_glut", beschreibung: "Die Sehne glimmt wie Kohle. Explosivpfeile zünden heftiger und brennen länger." },
    Waffe { id: 14, name: "Elfenbogen", klasse: CharacterClass::Bogenschuetze, seltenheit: Episch, schaden: 1.15, abklingen: 0.85, krit: 0.06, bonus: Some((Salve, 1.3)),
            datei: "bogen_elfen", beschreibung: "Aus hellem Silberholz mit Blattranken. Salven treffen härter, alles ist schneller bereit." },
    Waffe { id: 15, name: "Sturmbogen", klasse: CharacterClass::Bogenschuetze, seltenheit: Legendaer, schaden: 1.28, abklingen: 0.85, krit: 0.1, bonus: Some((Pfeilregen, 1.3)),
            datei: "bogen_sturm", beschreibung: "Blitze knistern in der Sehne. Mehr Schaden, schnellere Fähigkeiten, ein gewaltiger Pfeilregen." },
    Waffe { id: 16, name: "Eisenklinge", klasse: CharacterClass::Rogue, seltenheit: Gewoehnlich, schaden: 1.12, abklingen: 1.0, krit: 0.03, bonus: None,
            datei: "klinge_eisen", beschreibung: "Ein schlichtes Kurzschwert aus gutem Eisen. Etwas mehr Schärfe in jedem Hieb." },
    Waffe { id: 17, name: "Giftzahn", klasse: CharacterClass::Rogue, seltenheit: Selten, schaden: 1.08, abklingen: 1.0, krit: 0.03, bonus: Some((Wurfdolche, 1.45)),
            datei: "klinge_gift", beschreibung: "Die Klinge schwitzt grünes Gift. Wurfdolche treffen härter und lähmen länger." },
    Waffe { id: 18, name: "Rußklinge", klasse: CharacterClass::Rogue, seltenheit: Selten, schaden: 1.08, abklingen: 1.0, krit: 0.03, bonus: Some((Rauchbombe, 1.45)),
            datei: "klinge_russ", beschreibung: "Aus rußschwarzem Stahl geschmiedet. Rauchbomben treffen härter und betäuben länger." },
    Waffe { id: 19, name: "Mondsichel", klasse: CharacterClass::Rogue, seltenheit: Episch, schaden: 1.15, abklingen: 0.85, krit: 0.06, bonus: Some((Klingenhieb, 1.3)),
            datei: "klinge_mond", beschreibung: "Eine silberne Krummklinge, leicht wie ein Lufthauch. Hiebe treffen härter, alles ist schneller bereit." },
    Waffe { id: 20, name: "Schattenzahn", klasse: CharacterClass::Rogue, seltenheit: Legendaer, schaden: 1.28, abklingen: 0.85, krit: 0.1, bonus: Some((Schattenklingen, 1.3)),
            datei: "klinge_schatten", beschreibung: "In ihr wohnt ein Schatten. Mehr Schaden, schnellere Fähigkeiten, gewaltigere Schattenklingen." },
];

pub fn waffe(id: u8) -> Option<&'static Waffe> {
    WAFFEN.iter().find(|w| w.id == id)
}

/// Name der Startwaffe einer Klasse.
pub fn startwaffe(class: CharacterClass) -> &'static str {
    match class {
        CharacterClass::Zwerg => "Schmiedehammer",
        CharacterClass::Bogenschuetze => "Mondbogen",
        CharacterClass::Rogue => "Runenklinge",
        _ => "Wanderstab",
    }
}

/// Die Waffe in der Hand, wenn sie zur Klasse passt (sonst die Startwaffe).
pub fn ausgeruestet(id: u8, class: CharacterClass) -> Option<&'static Waffe> {
    waffe(id).filter(|w| w.klasse == class)
}

/// Kritischer Treffer: so viel mehr Schaden.
pub const KRIT_FAKTOR: f32 = 1.5;

/// Wie eine Fähigkeit mit Waffe und Rüstung wirkt: Schaden, Nachwirkung, Abklingzeit (Faktoren)
/// und die Chance auf einen kritischen Treffer.
pub fn faktoren(id: u8, ruestung: [u8; 3], class: CharacterClass, art: Faehigkeit) -> (f32, f32, f32, f32) {
    let r = crate::ruestung::summe(ruestung, class);
    let Some(w) = ausgeruestet(id, class) else { return (r.schaden, 1.0, r.abklingen, r.krit) };
    let bonus = w.bonus.filter(|b| b.0 == art).map_or(1.0, |b| b.1);
    (w.schaden * bonus * r.schaden, bonus, w.abklingen * r.abklingen, w.krit + r.krit)
}

impl Waffe {
    /// Wert in Gold.
    pub fn wert(&self) -> u32 {
        let bonus = self.bonus.map_or(0.0, |b| (b.1 - 1.0) * 150.0);
        let budget = (self.schaden - 1.0) * 800.0 + (1.0 - self.abklingen) * 700.0 + self.krit * 800.0 + bonus;
        (self.seltenheit.grundwert() as f32 + budget).round() as u32 / 5 * 5
    }

    /// Gold, wenn man die Waffe aufhebt, obwohl man sie schon hat (ein Viertel des Werts).
    pub fn gold_fuer_doppelte(&self) -> u32 {
        (self.wert() / 4).max(5)
    }

    /// Werte als Zeilen für den Tooltip.
    pub fn werte_zeilen(&self) -> Vec<String> {
        let mut z = vec![format!("Schaden +{:.0} %", (self.schaden - 1.0) * 100.0)];
        if self.abklingen < 1.0 {
            z.push(format!("Abklingzeiten −{:.0} %", (1.0 - self.abklingen) * 100.0));
        }
        z.push(format!("Kritische Treffer {:.0} % (×{KRIT_FAKTOR})", self.krit * 100.0));
        if let Some((art, f)) = self.bonus {
            z.push(format!("{} +{:.0} %", art.label(), (f - 1.0) * 100.0));
        }
        z
    }
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
        // Erbeutete Waffen stehen als Bits in einem u32
        assert!(WAFFEN.len() < 32);
    }

    #[test]
    fn gefaehrliche_lager_haben_bessere_waffen() {
        assert_eq!(seltenheit_fuer(1, 0.99), Seltenheit::Episch);
        assert_eq!(seltenheit_fuer(3, 0.99), Seltenheit::Legendaer);
        assert!(waffen_chance(3, true) > waffen_chance(1, true));
        // Fremde Waffen wirken nicht
        assert_eq!(faktoren(5, [0; 3], CharacterClass::Zwerg, Faehigkeit::Hammerschlag), (1.0, 1.0, 1.0, 0.0));
        let (s, n, a, _) = faktoren(2, [0; 3], CharacterClass::Mage, Faehigkeit::Feuerball);
        assert!(s > 1.5 && n > 1.4 && a == 1.0);
    }
}
