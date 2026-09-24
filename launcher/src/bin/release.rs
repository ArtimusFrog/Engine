//! Release-Werkzeug (nur für die Entwickler, nicht für Spieler).
//!
//! - `release schluessel` – legt einmalig den geheimen Signatur-Schlüssel an
//!   (`%APPDATA%\EngineJN\release_signatur.key`, NIE hochladen oder einchecken!) und schreibt
//!   den öffentlichen Schlüssel nach `launcher/release_public_key.txt`.
//! - `release bauen --version 0.2.0 [--neu "Text"]...` – stellt aus dem eigenen Release-Build
//!   (Windows) alles für den Webspace in `dist/web/` zusammen: Webseite, Download, Updates.
//! - `release mac --von <ordner>` – fügt die Mac-Version hinzu. Der Ordner ist das entpackte
//!   Ergebnis des GitHub-Workflows „Mac bauen“ (`game`, `launcher`, `EngineJN-macOS.dmg`).
//!   Signiert wird hier auf dem eigenen Rechner – der geheime Schlüssel kommt nie zu GitHub.
//!
//! Vorher bauen: `cargo build --release -p game -p launcher`.

use std::path::{Path, PathBuf};

use launcher::config::{platform, GAME_NAME};
use launcher::{manifest_from_dir, public_key, sha256_file, signed_manifest, walk, FileEntry};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn secret_key_path() -> PathBuf {
    let base = std::env::var_os("APPDATA").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    base.join("EngineJN").join("release_signatur.key")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("schluessel") => create_key(),
        Some("bauen") => build_local(&args[1..]),
        Some("mac") => add_mac(&args[1..]),
        _ => Err("Aufruf: release schluessel | release bauen --version <x.y.z> [--neu \"Text\"]... | release mac --von <ordner>".into()),
    };
    if let Err(message) = result {
        eprintln!("FEHLER: {message}");
        std::process::exit(1);
    }
}

fn create_key() -> Result<(), String> {
    let path = secret_key_path();
    if path.exists() {
        println!("Schlüssel existiert schon: {} (wird nicht überschrieben)", path.display());
    } else {
        std::fs::create_dir_all(path.parent().expect("hat Ordner")).map_err(|e| e.to_string())?;
        std::fs::write(&path, launcher::generate_secret_key()).map_err(|e| e.to_string())?;
        println!("Neuer geheimer Schlüssel: {}", path.display());
        println!("WICHTIG: Sichere diese Datei (z. B. im Passwortmanager). Ohne sie kann keine Version mehr veröffentlicht werden.");
    }
    let secret = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let public = public_key(secret.trim())?;
    std::fs::write(repo().join("launcher").join("release_public_key.txt"), &public).map_err(|e| e.to_string())?;
    println!("Öffentlicher Schlüssel: {public}");
    Ok(())
}

/// Geheimer Schlüssel – nur wenn er zum im Launcher eingebauten öffentlichen passt.
fn secret_key() -> Result<String, String> {
    let secret = std::fs::read_to_string(secret_key_path()).map_err(|_| "Kein Signatur-Schlüssel – erst `release schluessel` ausführen".to_string())?;
    let secret = secret.trim().to_string();
    if public_key(&secret)? != launcher::config::PUBLIC_KEY.trim() {
        return Err("Der eingebaute öffentliche Schlüssel passt nicht zum geheimen – Launcher neu bauen (cargo build --release -p launcher).".into());
    }
    Ok(secret)
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    for file in walk(from) {
        let target = to.join(file.strip_prefix(from).expect("liegt im Ordner"));
        std::fs::create_dir_all(target.parent().expect("hat Ordner")).map_err(|e| e.to_string())?;
        std::fs::copy(&file, &target).map_err(|e| format!("{}: {e}", file.display()))?;
    }
    Ok(())
}

/// Was für eine Plattform veröffentlicht wird.
struct Build<'a> {
    platform: &'a str,
    game: &'a Path,
    /// Name der Spieldatei im Spielordner (`game.exe` bzw. `game`).
    executable: &'a str,
    launcher: &'a Path,
    /// Datei für den Download-Knopf auf der Webseite.
    download: &'a Path,
    download_name: &'a str,
}

/// Legt `dist/web/updates/<plattform>/` und den Download an und trägt ihn in version.json ein.
fn publish(build: &Build, version: &str, date: &str, news: Vec<String>, secret: &str) -> Result<(), String> {
    for needed in [build.game, build.launcher, build.download] {
        if !needed.exists() {
            return Err(format!("{} fehlt", needed.display()));
        }
    }
    let dist = repo().join("dist");
    let web = dist.join("web");

    // Spieldateien: Programm + assets/ (die Assets sind für alle Plattformen gleich).
    let payload = dist.join(format!("spiel-{}", build.platform));
    let _ = std::fs::remove_dir_all(&payload);
    std::fs::create_dir_all(&payload).map_err(|e| e.to_string())?;
    std::fs::copy(build.game, payload.join(build.executable)).map_err(|e| e.to_string())?;
    copy_dir(&repo().join("game/assets"), &payload.join("assets"))?;

    let mut manifest = manifest_from_dir(&payload, version, date, news, build.executable).map_err(|e| e.to_string())?;
    let launcher_size = std::fs::metadata(build.launcher).map_err(|e| e.to_string())?.len();
    let launcher_hash = sha256_file(build.launcher).map_err(|e| e.to_string())?;
    manifest.launcher = Some(FileEntry { path: "launcher".into(), size: launcher_size, sha256: launcher_hash.clone() });

    let updates = web.join("updates").join(build.platform);
    let _ = std::fs::remove_dir_all(&updates);
    let blobs = updates.join("dateien");
    std::fs::create_dir_all(&blobs).map_err(|e| e.to_string())?;
    for entry in &manifest.files {
        std::fs::copy(payload.join(&entry.path), blobs.join(&entry.sha256)).map_err(|e| e.to_string())?;
    }
    std::fs::copy(build.launcher, blobs.join(&launcher_hash)).map_err(|e| e.to_string())?;
    std::fs::write(updates.join("manifest.signed"), signed_manifest(&manifest, secret)?).map_err(|e| e.to_string())?;

    let downloads = web.join("downloads");
    std::fs::create_dir_all(&downloads).map_err(|e| e.to_string())?;
    std::fs::copy(build.download, downloads.join(build.download_name)).map_err(|e| e.to_string())?;
    let download_size = std::fs::metadata(build.download).map_err(|e| e.to_string())?.len();

    // version.json für die Webseite – Einträge anderer Plattformen bleiben erhalten.
    let info_path = web.join("version.json");
    let mut info: serde_json::Value = std::fs::read(&info_path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_else(|| serde_json::json!({}));
    info["name"] = GAME_NAME.into();
    info["version"] = manifest.version.clone().into();
    info["date"] = manifest.date.clone().into();
    info["news"] = manifest.news.clone().into();
    info["downloads"][build.platform] = serde_json::json!({ "file": format!("downloads/{}", build.download_name), "size": download_size });
    std::fs::write(&info_path, serde_json::to_string_pretty(&info).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;

    let total: u64 = manifest.files.iter().map(|f| f.size).sum();
    println!("Release {version} ({date}) für {}: {} Dateien, {:.1} MB", build.platform, manifest.files.len(), total as f64 / 1e6);
    Ok(())
}

/// Release für die eigene Plattform (Windows): baut die Webseite neu auf.
fn build_local(args: &[String]) -> Result<(), String> {
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let version = value("--version").ok_or("--version fehlt, z. B. --version 0.2.0")?;
    let news: Vec<String> = args.windows(2).filter(|w| w[0] == "--neu").map(|w| w[1].clone()).collect();
    let date = value("--datum").unwrap_or_else(today);
    let secret = secret_key()?;

    let repo = repo();
    let web = repo.join("dist").join("web");
    let _ = std::fs::remove_dir_all(&web);
    copy_dir(&repo.join("web"), &web)?;

    let exe = if cfg!(windows) { ".exe" } else { "" };
    let game = repo.join("target/release").join(format!("game{exe}"));
    let launcher = repo.join("target/release").join(format!("launcher{exe}"));
    let executable = format!("game{exe}");
    let download_name = if cfg!(windows) { format!("{}-Windows.exe", GAME_NAME.replace(' ', "")) } else { format!("{}-{}", GAME_NAME.replace(' ', ""), platform()) };
    // Unter Windows ist der Launcher selbst der Download (er installiert sich beim ersten Start).
    let build = Build { platform: platform(), game: &game, executable: &executable, launcher: &launcher, download: &launcher, download_name: &download_name };
    publish(&build, &version, &date, news, &secret)?;
    println!("Fertig in {}", web.display());
    Ok(())
}

/// Mac-Version aus dem GitHub-Workflow hinzufügen (gleiche Version wie das letzte `bauen`).
fn add_mac(args: &[String]) -> Result<(), String> {
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let dir = PathBuf::from(value("--von").ok_or("--von <ordner> fehlt (entpacktes Ergebnis von „Mac bauen“)")?);
    let secret = secret_key()?;
    let info_path = repo().join("dist/web/version.json");
    let info: serde_json::Value = std::fs::read(&info_path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .ok_or("dist/web/version.json fehlt – erst die Windows-Version mit `release bauen` zusammenstellen")?;
    let version = info["version"].as_str().unwrap_or_default().to_string();
    let date = info["date"].as_str().unwrap_or_default().to_string();
    let news: Vec<String> = info["news"].as_array().map(|a| a.iter().filter_map(|n| n.as_str().map(String::from)).collect()).unwrap_or_default();

    let download_name = format!("{}-macOS.dmg", GAME_NAME.replace(' ', ""));
    let build = Build {
        platform: "macos",
        game: &dir.join("game"),
        executable: "game",
        launcher: &dir.join("launcher"),
        download: &dir.join(&download_name),
        download_name: &download_name,
    };
    publish(&build, &version, &date, news, &secret)?;
    println!("Mac-Version {version} hinzugefügt.");
    Ok(())
}

/// Heutiges Datum (UTC) als JJJJ-MM-TT, ohne zusätzliche Bibliothek.
fn today() -> String {
    let days = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() / 86_400).unwrap_or(0) as i64;
    // Umrechnung nach Howard Hinnant (civil_from_days)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}
