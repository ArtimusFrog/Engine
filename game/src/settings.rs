//! Einstellungen, die zwischen zwei Spielstarts erhalten bleiben.

use std::path::PathBuf;

use engine::prelude::*;
use serde::{Deserialize, Serialize};

use crate::protocol::{clean_name, CharacterClass, Hello};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Stand des Einstellungsformats (für einmalige Umstellungen alter Dateien).
    #[serde(default)]
    pub settings_version: u32,
    pub name: String,
    /// Gewählte Figur.
    pub character: CharacterClass,
    /// Faktor auf die Grundempfindlichkeit der Maus (1 = Standard).
    pub mouse_sensitivity: f32,
    pub invert_y: bool,
    /// Vertikales Sichtfeld in Grad.
    pub fov_degrees: f32,
    pub fullscreen: bool,
    pub vsync: bool,
    /// Zuletzt benutzte Server-Adresse.
    pub last_address: String,
    /// Lautstärken 0..1.
    pub volume_master: f32,
    pub volume_effects: f32,
    pub volume_ambient: f32,
    pub volume_music: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            settings_version: SETTINGS_VERSION,
            name: default_name(),
            character: CharacterClass::default(),
            mouse_sensitivity: 1.0,
            invert_y: false,
            fov_degrees: 70.0,
            fullscreen: true,
            vsync: true,
            last_address: "127.0.0.1".into(),
            volume_master: 0.8,
            volume_effects: 0.8,
            volume_ambient: 0.7,
            volume_music: 0.45,
        }
    }
}

/// Windows-Benutzername als Vorschlag für den Spielernamen.
fn default_name() -> String {
    std::env::var("USERNAME").or_else(|_| std::env::var("USER")).map(|n| clean_name(&n)).unwrap_or_else(|_| "Spieler".into())
}

/// 1: Vollbild wird Standard (ab Version 0.2.1).
const SETTINGS_VERSION: u32 = 1;

fn path() -> PathBuf {
    engine::storage::config_dir("EngineJN").join("einstellungen.json")
}

impl Settings {
    pub fn load() -> Self {
        let path = path();
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let mut settings: Settings = serde_json::from_str(&text).unwrap_or_else(|e| {
                    log::warn!("Einstellungen in {} sind beschädigt ({e}), nehme Standardwerte", path.display());
                    Settings::default()
                });
                // Ältere Einstellungen: einmalig auf Vollbild umstellen, danach gilt die eigene Wahl.
                if settings.settings_version < 1 {
                    settings.fullscreen = true;
                }
                settings.settings_version = SETTINGS_VERSION;
                settings
            }
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
        ctx.audio.set_master_volume(self.volume_master);
        ctx.audio.set_volume(Bus::Effects, self.volume_effects);
        ctx.audio.set_volume(Bus::Ambient, self.volume_ambient);
        ctx.audio.set_volume(Bus::Music, self.volume_music);
    }
}

impl Settings {
    /// Was beim Verbinden an den Server geht.
    pub fn hello(&self) -> Hello {
        Hello { name: clean_name(&self.name), class: self.character }
    }
}
