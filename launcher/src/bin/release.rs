//! Release-Werkzeug (nur für die Entwickler, nicht für Spieler).
//!
//! - `release schluessel`  – legt einmalig den geheimen Signatur-Schlüssel an
//!   (`%APPDATA%\EngineJN\release_signatur.key`, NIE hochladen oder einchecken!) und schreibt
//!   den öffentlichen Schlüssel nach `launcher/release_public_key.txt`.
//! - `release bauen --version 0.2.0 [--neu "Text"]...` – stellt aus dem Release-Build alles
//!   für den Webspace in `dist/web/` zusammen: Webseite, Launcher-Download und signierte Updates.
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
        Some("bauen") => build(&args[1..]),
        _ => Err("Aufruf: release schluessel | release bauen --version <x.y.z> [--neu \"Text\"]...".into()),
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

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    for file in walk(from) {
        let target = to.join(file.strip_prefix(from).expect("liegt im Ordner"));
        std::fs::create_dir_all(target.parent().expect("hat Ordner")).map_err(|e| e.to_string())?;
        std::fs::copy(&file, &target).map_err(|e| format!("{}: {e}", file.display()))?;
    }
    Ok(())
}

fn build(args: &[String]) -> Result<(), String> {
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let version = value("--version").ok_or("--version fehlt, z. B. --version 0.2.0")?;
    let news: Vec<String> = args.windows(2).filter(|w| w[0] == "--neu").map(|w| w[1].clone()).collect();
    let date = value("--datum").unwrap_or_else(today);

    let secret = std::fs::read_to_string(secret_key_path()).map_err(|_| "Kein Signatur-Schlüssel – erst `release schluessel` ausführen".to_string())?;
    let public = public_key(secret.trim())?;
    if public != launcher::config::PUBLIC_KEY.trim() {
        return Err("Der eingebaute öffentliche Schlüssel passt nicht zum geheimen – Launcher neu bauen (cargo build --release -p launcher).".into());
    }

    let repo = repo();
    let exe = if cfg!(windows) { ".exe" } else { "" };
    let game_exe = repo.join("target/release").join(format!("game{exe}"));
    let launcher_exe = repo.join("target/release").join(format!("launcher{exe}"));
    for needed in [&game_exe, &launcher_exe] {
        if !needed.exists() {
            return Err(format!("{} fehlt – erst `cargo build --release -p game -p launcher`", needed.display()));
        }
    }

    // 1. Spieldateien zusammenstellen: game.exe + assets/
    let dist = repo.join("dist");
    let payload = dist.join(format!("spiel-{}", platform()));
    let _ = std::fs::remove_dir_all(&payload);
    std::fs::create_dir_all(&payload).map_err(|e| e.to_string())?;
    let executable = format!("game{exe}");
    std::fs::copy(&game_exe, payload.join(&executable)).map_err(|e| e.to_string())?;
    copy_dir(&repo.join("game/assets"), &payload.join("assets"))?;

    // 2. Manifest mit allen Dateien und dem Launcher selbst
    let mut manifest = manifest_from_dir(&payload, &version, &date, news, &executable).map_err(|e| e.to_string())?;
    let launcher_size = std::fs::metadata(&launcher_exe).map_err(|e| e.to_string())?.len();
    let launcher_hash = sha256_file(&launcher_exe).map_err(|e| e.to_string())?;
    manifest.launcher = Some(FileEntry { path: "launcher".into(), size: launcher_size, sha256: launcher_hash.clone() });

    // 3. Webseite + Updates: dist/web/
    let web = dist.join("web");
    let _ = std::fs::remove_dir_all(&web);
    copy_dir(&repo.join("web"), &web)?;
    let updates = web.join("updates").join(platform());
    let blobs = updates.join("dateien");
    std::fs::create_dir_all(&blobs).map_err(|e| e.to_string())?;
    for entry in &manifest.files {
        std::fs::copy(payload.join(&entry.path), blobs.join(&entry.sha256)).map_err(|e| e.to_string())?;
    }
    std::fs::copy(&launcher_exe, blobs.join(&launcher_hash)).map_err(|e| e.to_string())?;
    std::fs::write(updates.join("manifest.signed"), signed_manifest(&manifest, secret.trim())?).map_err(|e| e.to_string())?;

    // Download für die Webseite: der Launcher (installiert sich beim ersten Start selbst).
    let downloads = web.join("downloads");
    std::fs::create_dir_all(&downloads).map_err(|e| e.to_string())?;
    let download_name = download_file_name();
    std::fs::copy(&launcher_exe, downloads.join(&download_name)).map_err(|e| e.to_string())?;

    // Infos für die Webseite (Version, Datum, Neuigkeiten, Download-Größe).
    let info = serde_json::json!({
        "name": GAME_NAME,
        "version": manifest.version,
        "date": manifest.date,
        "news": manifest.news,
        "downloads": { platform(): { "file": format!("downloads/{download_name}"), "size": launcher_size } },
    });
    std::fs::write(web.join("version.json"), serde_json::to_string_pretty(&info).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;

    let total: u64 = manifest.files.iter().map(|f| f.size).sum();
    println!("Release {version} ({date}) für {}: {} Dateien, {:.1} MB", platform(), manifest.files.len(), total as f64 / 1e6);
    println!("Fertig in {}", web.display());
    Ok(())
}

fn download_file_name() -> String {
    let base = GAME_NAME.replace(' ', "");
    match platform() {
        "windows" => format!("{base}-Windows.exe"),
        other => format!("{base}-{other}"),
    }
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
