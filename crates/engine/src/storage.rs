//! Ablageorte für Einstellungen und Spielstände.

use std::path::PathBuf;

/// Ordner für Einstellungen des Spiels, wird bei Bedarf angelegt.
/// Windows: `%APPDATA%\<app>`, macOS: `~/Library/Application Support/<app>`,
/// Linux: `$XDG_CONFIG_HOME/<app>` bzw. `~/.config/<app>`.
pub fn config_dir(app: &str) -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
    };
    let dir = base.unwrap_or_else(|| PathBuf::from(".")).join(app);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        log::warn!("Einstellungsordner {} lässt sich nicht anlegen: {e}", dir.display());
    }
    dir
}
