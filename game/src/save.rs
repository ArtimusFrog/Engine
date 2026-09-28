//! Spielstand des Servers: was über einen Neustart hinweg erhalten bleibt.
//!
//! Gespeichert werden Tageszeit, abgebaute und beschädigte Rohstoffe und die Inventare
//! der Spieler (nach Namen, es gibt noch keine Konten). Die Insel selbst entsteht aus
//! dem Startwert und muss nicht gespeichert werden.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::island::WORLD_ID;
use crate::protocol::Inventory;
use crate::world::World;

/// Bei inkompatiblen Änderungen am Format hochzählen (alte Stände werden dann verworfen).
const FORMAT: u32 = 1;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorldSave {
    pub format: u32,
    /// Kennung der Insel (`island::WORLD_ID`) – passt sie nicht, gehören die Rohstoff-IDs zu einer
    /// anderen Welt. Der Feldname bleibt `seed`, damit alte Spielstände lesbar bleiben.
    pub seed: u32,
    pub hour: f32,
    pub day: u32,
    /// Inventare nach Spielername (klein geschrieben).
    pub inventories: BTreeMap<String, Inventory>,
    /// Abgebaute Rohstoffe: (ID, Takte bis zum Nachwachsen).
    pub gone: Vec<(u32, u64)>,
    /// Beschädigte Rohstoffe: (ID, verbleibende Schläge).
    pub damaged: Vec<(u32, u8)>,
    /// Von Spielern errichtete Gebäude (ältere Spielstände haben keine).
    #[serde(default)]
    pub buildings: Vec<crate::bauten::Building>,
    /// Wer sein Startgold schon bekommen hat
    #[serde(default)]
    pub startgold: std::collections::BTreeSet<String>,
    /// Wer seine Start-Heiltränke schon bekommen hat
    #[serde(default)]
    pub start_traenke: std::collections::BTreeSet<String>,
    /// Siedlungsplätze mit eingesetztem Runenstein: wem sie gehören (je Straße)
    #[serde(default)]
    pub runen: Vec<Option<String>>,
}

/// Schlüssel für Inventare: Name ohne Groß/klein-Unterschied.
pub fn player_key(name: &str) -> String {
    name.trim().to_lowercase()
}

impl WorldSave {
    /// Hält den aktuellen Stand fest. `inventories` enthält auch Spieler, die gerade nicht da sind.
    pub fn capture(world: &World, tick: u64, inventories: &BTreeMap<String, Inventory>) -> WorldSave {
        let mut inventories = inventories.clone();
        for (id, avatar) in &world.players {
            if let Some(inventory) = world.inventories.get(id) {
                inventories.insert(player_key(&avatar.name), *inventory);
            }
        }
        WorldSave {
            format: FORMAT,
            seed: WORLD_ID,
            hour: world.day.hour,
            day: world.day.day,
            inventories,
            gone: world.resources.iter().filter_map(|(&id, r)| r.regrows_at.map(|at| (id, at.saturating_sub(tick)))).collect(),
            damaged: world.resources.iter().filter(|(_, r)| r.is_present() && r.health < r.spec.max_health).map(|(&id, r)| (id, r.health)).collect(),
            buildings: world.buildings.clone(),
            startgold: Default::default(),
            start_traenke: Default::default(),
            runen: Vec::new(),
        }
    }

    /// Liest einen Spielstand; fehlt er oder passt das Format nicht, `None`. Gehört er zu einer
    /// anderen Insel, bleiben Inventare und Tageszeit erhalten, die Rohstoffe fangen neu an.
    pub fn load(path: &Path) -> Option<WorldSave> {
        let text = std::fs::read_to_string(path).ok()?;
        match serde_json::from_str::<WorldSave>(&text) {
            Ok(save) if save.format == FORMAT && save.seed == WORLD_ID => Some(save),
            Ok(save) if save.format == FORMAT => {
                log::warn!("Spielstand {} gehört zu einer anderen Insel – Inventare bleiben, Rohstoffe wachsen neu", path.display());
                Some(WorldSave { seed: WORLD_ID, gone: Vec::new(), damaged: Vec::new(), buildings: Vec::new(), ..save })
            }
            Ok(_) => {
                log::warn!("Spielstand {} gehört zu einer anderen Version – fange neu an", path.display());
                None
            }
            Err(e) => {
                log::error!("Spielstand {} ist beschädigt ({e}) – fange neu an", path.display());
                None
            }
        }
    }

    /// Schreibt den Spielstand sicher: erst in eine Hilfsdatei, dann umbenennen. So bleibt
    /// der alte Stand heil, falls der Rechner mittendrin ausgeht.
    pub fn store(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let temporary = path.with_extension("tmp");
        std::fs::write(&temporary, serde_json::to_string_pretty(self).map_err(std::io::Error::other)?)?;
        std::fs::rename(&temporary, path)
    }
}

/// Wo der Spielstand liegt: `--welt <datei>`, sonst im Einstellungsordner
/// (`welt.json` beim Spielen am eigenen Rechner, `welt_server.json` für den Server).
pub fn default_path(dedicated_server: bool) -> Option<PathBuf> {
    // Tests und automatische Screenshots (mit Demo-Beute) sollen keinen echten Spielstand verändern.
    if cfg!(test) || std::env::args().any(|a| a == "--screenshot") {
        return None;
    }
    let args: Vec<String> = std::env::args().collect();
    if let Some(path) = args.iter().position(|a| a == "--welt").and_then(|i| args.get(i + 1)) {
        return Some(PathBuf::from(path));
    }
    let name = if dedicated_server { "welt_server.json" } else { "welt.json" };
    Some(engine::storage::config_dir("EngineJN").join(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speichern_und_laden() {
        let path = std::env::temp_dir().join(format!("weltstand_{}.json", std::process::id()));
        let mut save = WorldSave { format: FORMAT, seed: WORLD_ID, hour: 13.5, day: 4, ..Default::default() };
        save.inventories.insert(player_key(" Nils "), Inventory { wood: 12, stone: 3, ..Default::default() });
        save.gone.push((42, 900));
        save.damaged.push((7, 2));
        save.store(&path).unwrap();
        assert_eq!(WorldSave::load(&path), Some(save.clone()));

        // Andere Insel: Inventare bleiben, aber keine falschen Bäume fällen.
        WorldSave { seed: WORLD_ID + 1, ..save.clone() }.store(&path).unwrap();
        assert_eq!(WorldSave::load(&path), Some(WorldSave { gone: Vec::new(), damaged: Vec::new(), buildings: Vec::new(), ..save.clone() }));
        // Altes Format: neu anfangen.
        WorldSave { format: FORMAT + 1, ..save }.store(&path).unwrap();
        assert_eq!(WorldSave::load(&path), None);
        std::fs::write(&path, "kaputt").unwrap();
        assert_eq!(WorldSave::load(&path), None);
        std::fs::remove_file(&path).ok();
    }
}
