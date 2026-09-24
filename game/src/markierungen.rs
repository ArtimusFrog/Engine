//! Entwickler-Markierungen aus der Asset-Galerie: welches Modell auf der Insel wofür
//! verwendet wird (z. B. `natur/fels_test.gltf` als „Fels“).
//!
//! Gilt nur auf diesem Rechner (Datei im Einstellungsordner) – zum Gegentesten im Spiel.
//! Ohne Markierung nimmt die Insel wie gehabt `natur/<platz>[_n].gltf` bzw. die eingebauten Formen.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::asset_files::asset_dir;

/// Ein Platz auf der Insel, den ein Modell einnehmen kann: (Kennung, Anzeigename, Ordner).
pub const PLAETZE: &[(&str, &str, &str)] = &[
    ("eiche", "Eiche", "natur"),
    ("tanne", "Tanne", "natur"),
    ("palme", "Palme", "natur"),
    ("zauberbaum", "Zauberbaum", "natur"),
    ("fels", "Fels", "natur"),
    ("busch", "Busch", "natur"),
    ("gras", "Gras", "natur"),
    ("zaubergras", "Zaubergras", "natur"),
    ("blume", "Blume", "natur"),
    ("zauberblume", "Zauberblume", "natur"),
    ("fliegenpilz", "Fliegenpilz", "natur"),
    ("leuchtpilz", "Leuchtpilz", "natur"),
    ("kristall", "Kristall", "natur"),
    ("hase", "Hase", "tiere"),
    ("fuchs", "Fuchs", "tiere"),
    ("hirsch", "Hirsch", "tiere"),
    ("baer", "Bär", "tiere"),
];

pub fn platz_name(kennung: &str) -> &str {
    PLAETZE.iter().find(|(k, _, _)| *k == kennung).map_or(kennung, |(_, name, _)| name)
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Markierungen {
    /// Modell (Pfad relativ zum Asset-Ordner, mit `/`) → Platz-Kennung.
    pub zuordnung: BTreeMap<String, String>,
}

fn datei() -> Option<PathBuf> {
    if cfg!(test) {
        return None;
    }
    Some(engine::storage::config_dir("EngineJN").join("asset_markierungen.json"))
}

impl Markierungen {
    pub fn laden() -> Markierungen {
        datei()
            .and_then(|pfad| std::fs::read_to_string(pfad).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn speichern(&self) {
        let Some(pfad) = datei() else { return };
        match serde_json::to_string_pretty(self) {
            Ok(text) => {
                if let Err(e) = std::fs::write(&pfad, text) {
                    log::warn!("Markierungen lassen sich nicht speichern ({}): {e}", pfad.display());
                }
            }
            Err(e) => log::warn!("Markierungen nicht speicherbar: {e}"),
        }
    }

    pub fn platz(&self, modell: &str) -> Option<&str> {
        self.zuordnung.get(modell).map(String::as_str)
    }

    pub fn setzen(&mut self, modell: &str, platz: Option<&str>) {
        match platz {
            Some(platz) => self.zuordnung.insert(modell.to_string(), platz.to_string()),
            None => self.zuordnung.remove(modell),
        };
    }

    /// Alle für einen Platz markierten Modelle (volle Pfade, sortiert).
    pub fn modelle_fuer(&self, platz: &str) -> Vec<PathBuf> {
        let Some(dir) = asset_dir() else { return Vec::new() };
        self.zuordnung.iter().filter(|(_, p)| *p == platz).map(|(modell, _)| dir.join(modell)).filter(|p| p.is_file()).collect()
    }
}

/// Markierte Modelle für einen Platz (liest die Datei jedes Mal – ändert sich in der Galerie).
pub fn fuer_platz(platz: &str) -> Vec<PathBuf> {
    Markierungen::laden().modelle_fuer(platz)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markieren_und_aufheben() {
        let mut m = Markierungen::default();
        m.setzen("natur/fels_test.gltf", Some("fels"));
        m.setzen("tiere/baer.gltf", Some("baer"));
        assert_eq!(m.platz("natur/fels_test.gltf"), Some("fels"));
        m.setzen("natur/fels_test.gltf", None);
        assert_eq!(m.platz("natur/fels_test.gltf"), None);
        assert_eq!(platz_name("baer"), "Bär");
        // Nur existierende Dateien zählen.
        m.setzen("tiere/gibt_es_nicht.gltf", Some("baer"));
        assert!(m.modelle_fuer("baer").iter().all(|p| p.is_file()));
    }
}
