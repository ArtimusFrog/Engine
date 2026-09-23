//! Einstellungen, die zwischen zwei Spielstarts erhalten bleiben.

use std::path::PathBuf;

use engine::prelude::*;
use serde::{Deserialize, Serialize};

use crate::protocol::clean_name;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub name: String,
    /// Faktor auf die Grundempfindlichkeit der Maus (1 = Standard).
    pub mouse_sensitivity: f32,
    pub invert_y: bool,
    /// Vertikales Sichtfeld in Grad.
    pub fov_degrees: f32,
    pub fullscreen: bool,
    pub vsync: bool,
    /// Zuletzt benutzte Server-Adresse.
    pub last_address: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            name: default_name(),
            mouse_sensitivity: 1.0,
            invert_y: false,
            fov_degrees: 70.0,
            fullscreen: false,
            vsync: true,
            last_address: "127.0.0.1".into(),
        }
    }
}

/// Windows-Benutzername als Vorschlag für den Spielernamen.
fn default_name() -> String {
    std::env::var("USERNAME").or_else(|_| std::env::var("USER")).map(|n| clean_name(&n)).unwrap_or_else(|_| "Spieler".into())
}

fn path() -> PathBuf {
    engine::storage::config_dir("EngineJN").join("einstellungen.json")
}

impl Settings {
    pub fn load() -> Self {
        let path = path();
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                log::warn!("Einstellungen in {} sind beschädigt ({e}), nehme Standardwerte", path.display());
                Settings::default()
            }),
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self) {
        let path = path();
        let text = serde_json::to_string_pretty(self).expect("Einstellungen lassen sich immer als JSON schreiben");
        if let Err(e) = std::fs::write(&path, text) {
            log::warn!("Einstellungen konnten nicht gespeichert werden ({}): {e}", path.display());
        }
    }

    /// Überträgt die Einstellungen auf die Engine.
    pub fn apply(&self, ctx: &mut Context) {
        ctx.camera.fov_y = self.fov_degrees.clamp(50.0, 120.0).to_radians();
        ctx.display = Display { fullscreen: self.fullscreen, vsync: self.vsync };
    }
}
