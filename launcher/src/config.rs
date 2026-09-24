//! Was für alle Spieler gleich ist: Name, Adressen, Schlüssel und Installationsort.

use std::path::PathBuf;

pub const GAME_NAME: &str = "Engine JN";
pub const WEBSITE: &str = "https://buddysagainstbets.com";
/// Hier liegen die Updates, je Plattform ein Unterordner (`windows`, `macos`).
pub const UPDATE_BASE: &str = "https://buddysagainstbets.com/updates";

/// Öffentlicher Schlüssel für die Signatur der Update-Liste. Den passenden geheimen
/// Schlüssel hat nur, wer Releases veröffentlicht (`release schluessel`).
pub const PUBLIC_KEY: &str = include_str!("../release_public_key.txt");

pub fn platform() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

/// Woher Updates kommen. Zum Testen überschreibbar: `--quelle <ordner oder adresse>`.
pub fn update_location() -> String {
    let args: Vec<String> = std::env::args().collect();
    if let Some(source) = args.iter().position(|a| a == "--quelle").and_then(|i| args.get(i + 1)) {
        return source.clone();
    }
    format!("{UPDATE_BASE}/{}", platform())
}

/// Installationsordner (ohne Admin-Rechte beschreibbar).
/// Windows: `%LOCALAPPDATA%\Engine JN`, macOS: `~/Library/Application Support/Engine JN`.
pub fn install_dir() -> PathBuf {
    let args: Vec<String> = std::env::args().collect();
    if let Some(dir) = args.iter().position(|a| a == "--ordner").and_then(|i| args.get(i + 1)) {
        return PathBuf::from(dir);
    }
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    };
    base.unwrap_or_else(|| PathBuf::from(".")).join(GAME_NAME)
}

/// Hier liegen die Spieldateien, die der Launcher verwaltet.
pub fn game_dir() -> PathBuf {
    install_dir().join("spiel")
}

/// So heißt der installierte Launcher (für Spieler ist er „das Spiel“).
pub fn installed_launcher() -> PathBuf {
    let name = if cfg!(windows) { format!("{GAME_NAME}.exe") } else { GAME_NAME.to_string() };
    install_dir().join(name)
}
