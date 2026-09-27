//! Rüstung: je Klasse sechs Teile für drei Plätze (Kopf, Brust, Füße) in vier Seltenheiten.
//! Gegner in der Wildnis lassen sie fallen wie Waffen; im Inventar zieht man sie auf den passenden
//! Ausrüstungsplatz. Rüstung gibt Leben und Schutz (weniger Schaden durch Gegner), bessere Teile
//! dazu Schaden, kürzere Abklingzeiten oder kritische Treffer – je nach Klasse unterschiedlich
//! gewichtet (der Zwerg setzt auf Schutz, Magier auf Tempo, Bogenschützin und Schurke auf Krit).
//!
//! Modelle am Boden und Symbole: `art/lib/ruestung.py` (Dateiname wie `datei`).

use serde::{Deserialize, Serialize};

use crate::protocol::CharacterClass;
use crate::waffen::Seltenheit::{self, *};

/// Wo ein Rüstungsteil getragen wird.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Platz {
    Kopf,
    Brust,
    Fuesse,
}

impl Platz {
    #[cfg(test)]
    pub const ALLE: [Platz; 3] = [Platz::Kopf, Platz::Brust, Platz::Fuesse];

    pub fn index(self) -> usize {
        match self {
            Platz::Kopf => 0,
            Platz::Brust => 1,
            Platz::Fuesse => 2,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Platz::Kopf => "Kopf",
            Platz::Brust => "Brust",
            Platz::Fuesse => "Füße",
        }
    }
}

pub struct Ruestung {
    /// 1–24
    pub id: u8,
    pub name: &'static str,
    pub klasse: CharacterClass,
    pub platz: Platz,
    pub seltenheit: Seltenheit,
    /// Zusätzliche Lebenspunkte
    pub leben: u16,
    /// Schutz: Anteil, um den Schaden durch Gegner sinkt (0,05 = 5 %)
    pub schutz: f32,
    /// Faktoren wie bei Waffen: Schaden, Abklingzeiten; dazu Chance auf kritische Treffer
    pub schaden: f32,
    pub abklingen: f32,
    pub krit: f32,
    /// Datei von Bodenmodell und Symbol
    pub datei: &'static str,
    pub beschreibung: &'static str,
}

use CharacterClass::{Bogenschuetze, Mage, Rogue, Zwerg};
use Platz::*;

const fn teil(
    id: u8,
    name: &'static str,
    klasse: CharacterClass,
    platz: Platz,
    seltenheit: Seltenheit,
    (leben, schutz): (u16, f32),
    (schaden, abklingen, krit): (f32, f32, f32),
    datei: &'static str,
    beschreibung: &'static str,
) -> Ruestung {
    Ruestung { id, name, klasse, platz, seltenheit, leben, schutz, schaden, abklingen, krit, datei, beschreibung }
}

/// Werte je Seltenheit (Leben, Schutz) sind für alle Klassen gleich; die Nebenwerte haben dasselbe
/// Budget, nur anders verteilt.
pub const RUESTUNGEN: [Ruestung; 24] = [
    // ---------- Magier: Tempo und Zauberkraft ----------
    teil(1, "Lehrlingshut", Mage, Kopf, Gewoehnlich, (10, 0.03), (1.0, 0.97, 0.0), "hut_lehrling", "Ein spitzer Filzhut. Jeder Magier fängt so an."),
    teil(2, "Hut der Sterne", Mage, Kopf, Episch, (25, 0.06), (1.05, 0.92, 0.0), "hut_sterne",
         "Silberne Sterne kreisen um die Krempe. Zauber sind schneller wieder bereit."),
    teil(3, "Robe des Adepten", Mage, Brust, Selten, (20, 0.06), (1.03, 0.95, 0.0), "robe_adept", "Mit Schutzrunen bestickt."),
    teil(4, "Erzmagierrobe", Mage, Brust, Legendaer, (40, 0.1), (1.08, 0.9, 0.03), "robe_erzmagier",
         "Der Stoff schimmert wie der Nachthimmel. Mehr Kraft, schnellere Zauber."),
    teil(5, "Wanderschuhe", Mage, Fuesse, Gewoehnlich, (10, 0.03), (1.0, 0.98, 0.0), "schuhe_wander", "Bequem genug für jede Reise."),
    teil(6, "Mondschritt-Schuhe", Mage, Fuesse, Selten, (18, 0.05), (1.02, 0.95, 0.0), "schuhe_mondschritt", "Leise wie Mondlicht."),
    // ---------- Zwerg: viel Schutz, etwas Wucht ----------
    teil(7, "Eisenhaube", Zwerg, Kopf, Gewoehnlich, (12, 0.05), (1.0, 1.0, 0.0), "helm_eisen", "Schlicht, schwer, zuverlässig."),
    teil(8, "Runenhelm", Zwerg, Kopf, Episch, (30, 0.1), (1.05, 0.97, 0.0), "helm_runen", "Die Runen der Ahnen glühen, wenn der Träger zuschlägt."),
    teil(9, "Kettenhemd", Zwerg, Brust, Selten, (25, 0.09), (1.02, 1.0, 0.0), "brust_kette", "Tausend Ringe, von Hand geschmiedet."),
    teil(10, "Ahnenpanzer", Zwerg, Brust, Legendaer, (50, 0.15), (1.08, 0.95, 0.0), "brust_ahnen",
         "Gold und Stahl aus der Tiefe. Kaum ein Hieb dringt hindurch."),
    teil(11, "Grubenstiefel", Zwerg, Fuesse, Gewoehnlich, (12, 0.05), (1.0, 1.0, 0.0), "stiefel_gruben", "Mit Stahlkappen."),
    teil(12, "Eisenschritt", Zwerg, Fuesse, Selten, (20, 0.08), (1.02, 1.0, 0.0), "stiefel_eisen", "Jeder Schritt dröhnt."),
    // ---------- Bogenschützin: Krit und Tempo ----------
    teil(13, "Jägerkappe", Bogenschuetze, Kopf, Gewoehnlich, (10, 0.03), (1.0, 1.0, 0.02), "kappe_jaeger", "Mit einer Feder an der Seite."),
    teil(14, "Mondkrone", Bogenschuetze, Kopf, Episch, (25, 0.06), (1.03, 0.96, 0.05), "krone_mond", "Ein Silberreif mit Mondstein. Das Auge wird schärfer."),
    teil(15, "Lederwams", Bogenschuetze, Brust, Selten, (20, 0.06), (1.02, 1.0, 0.03), "wams_leder", "Weich gegerbt, lässt den Bogenarm frei."),
    teil(16, "Harnisch des Nachtwinds", Bogenschuetze, Brust, Legendaer, (40, 0.1), (1.06, 0.94, 0.06), "harnisch_nachtwind",
         "Leicht wie Wind, hart wie Stahl. Mehr Schaden, mehr Volltreffer."),
    teil(17, "Pirschstiefel", Bogenschuetze, Fuesse, Gewoehnlich, (10, 0.03), (1.0, 1.0, 0.02), "stiefel_pirsch", "Weiche Sohlen für die Jagd."),
    teil(18, "Elfenstiefel", Bogenschuetze, Fuesse, Selten, (18, 0.05), (1.02, 0.97, 0.03), "stiefel_elfen", "Mit Blattranken bestickt."),
    // ---------- Schurke: Schaden und Krit ----------
    teil(19, "Kapuze", Rogue, Kopf, Gewoehnlich, (10, 0.03), (1.02, 1.0, 0.01), "kapuze", "Verbirgt das Gesicht im Schatten."),
    teil(20, "Schattenmaske", Rogue, Kopf, Episch, (25, 0.06), (1.06, 0.97, 0.04), "maske_schatten", "Wer sie trägt, trifft, wo es wehtut."),
    teil(21, "Lederweste", Rogue, Brust, Selten, (20, 0.06), (1.04, 1.0, 0.02), "weste_leder", "Voller Taschen für Dolche und Rauchbomben."),
    teil(22, "Mantel der Dämmerung", Rogue, Brust, Legendaer, (40, 0.1), (1.1, 0.95, 0.05), "mantel_daemmerung",
         "Er verschluckt das Licht. Mehr Schaden, mehr Volltreffer."),
    teil(23, "Leisetreter", Rogue, Fuesse, Gewoehnlich, (10, 0.03), (1.02, 1.0, 0.0), "stiefel_leise", "Kein Laut auf Stein."),
    teil(24, "Schattenstiefel", Rogue, Fuesse, Selten, (18, 0.05), (1.03, 1.0, 0.02), "stiefel_schatten", "Schwarzes Leder mit Silberschnallen."),
];

pub fn ruestung(id: u8) -> Option<&'static Ruestung> {
    RUESTUNGEN.iter().find(|r| r.id == id)
}

/// Getragene Teile, die zur Klasse passen (Kopf, Brust, Füße).
pub fn getragen(ids: [u8; 3], class: CharacterClass) -> impl Iterator<Item = &'static Ruestung> {
    ids.into_iter().filter_map(ruestung).filter(move |r| r.klasse == class)
}

/// Summe aller getragenen Teile: Leben, Schutz (höchstens 40 %), Schaden, Abklingen, Krit.
pub fn summe(ids: [u8; 3], class: CharacterClass) -> Werte {
    let mut w = Werte { leben: 0, schutz: 0.0, schaden: 1.0, abklingen: 1.0, krit: 0.0 };
    for r in getragen(ids, class) {
        w.leben += r.leben;
        w.schutz += r.schutz;
        w.schaden *= r.schaden;
        w.abklingen *= r.abklingen;
        w.krit += r.krit;
    }
    w.schutz = w.schutz.min(0.4);
    w
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Werte {
    pub leben: u16,
    pub schutz: f32,
    pub schaden: f32,
    pub abklingen: f32,
    pub krit: f32,
}

impl Ruestung {
    /// Wert in Gold: nach Seltenheit und wie viel das Teil bringt.
    pub fn wert(&self) -> u32 {
        let budget = self.leben as f32 * 1.5 + self.schutz * 400.0 + (self.schaden - 1.0) * 600.0 + (1.0 - self.abklingen) * 600.0 + self.krit * 800.0;
        (self.seltenheit.grundwert() as f32 * 0.6 + budget).round() as u32 / 5 * 5
    }

    /// Werte als Zeilen für den Tooltip.
    pub fn werte_zeilen(&self) -> Vec<String> {
        let mut z = vec![format!("+{} Leben", self.leben), format!("Schutz +{:.0} %", self.schutz * 100.0)];
        if self.schaden > 1.0 {
            z.push(format!("Schaden +{:.0} %", (self.schaden - 1.0) * 100.0));
        }
        if self.abklingen < 1.0 {
            z.push(format!("Abklingzeiten −{:.0} %", (1.0 - self.abklingen) * 100.0));
        }
        if self.krit > 0.0 {
            z.push(format!("Kritische Treffer +{:.0} %", self.krit * 100.0));
        }
        z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jede_klasse_hat_sechs_teile_und_die_budgets_passen() {
        let mut ids: Vec<u8> = RUESTUNGEN.iter().map(|r| r.id).collect();
        ids.dedup();
        assert_eq!(ids, (1..=RUESTUNGEN.len() as u8).collect::<Vec<u8>>());
        assert!(RUESTUNGEN.len() < 32, "erbeutete Rüstung steht als Bits in einem u32");
        for class in CharacterClass::ALL {
            let eigene: Vec<&Ruestung> = RUESTUNGEN.iter().filter(|r| r.klasse == class).collect();
            assert_eq!(eigene.len(), 6, "{class:?}");
            for platz in Platz::ALLE {
                assert_eq!(eigene.iter().filter(|r| r.platz == platz).count(), 2, "{class:?} {platz:?}");
            }
            // Alle Klassen bekommen mit der besten Rüstung ähnlich viel (Wert in Gold als Maß)
            let bestes: u32 = Platz::ALLE.iter().map(|&p| eigene.iter().filter(|r| r.platz == p).map(|r| r.wert()).max().unwrap()).sum();
            assert!((700..1400).contains(&bestes), "{class:?}: {bestes} Gold");
        }
    }

    #[test]
    fn schutz_ist_begrenzt_und_fremde_teile_zaehlen_nicht() {
        let w = summe([8, 10, 12], Zwerg);
        assert!(w.schutz <= 0.4 && w.leben == 100);
        assert_eq!(summe([8, 10, 12], Mage).leben, 0);
    }
}
