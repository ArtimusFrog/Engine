//! Kern des Launchers: Dateiliste (Manifest), Prüfsummen, Signatur und das sichere
//! Aktualisieren der Spieldateien. Ohne Fenster, damit alles testbar ist.
//!
//! Aufbau auf dem Webspace (je Plattform, z. B. `updates/windows/`):
//! - `manifest.signed` – erste Zeile: Ed25519-Signatur (hex), danach das Manifest als JSON
//!   (Version, Neuigkeiten, alle Dateien mit Größe und SHA-256). Eine einzige Datei, damit
//!   Liste und Signatur beim Hochladen nie kurz auseinanderlaufen.
//! - `dateien/<sha256>` – der Inhalt jeder Datei, benannt nach ihrer Prüfsumme
//!
//! Weil Dateien nach ihrem Inhalt benannt sind, kann man eine neue Version komplett
//! hochladen, bevor das Manifest umgestellt wird – Spieler sehen nie einen halben Stand.

pub mod config;

use std::io::Read;
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Eine Datei des Spiels.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    /// Pfad relativ zum Spielordner, immer mit `/`.
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// Anzeige-Version, z. B. "0.3.0".
    pub version: String,
    /// Datum der Veröffentlichung (JJJJ-MM-TT).
    pub date: String,
    /// Neuigkeiten für den Launcher (eine Zeile je Punkt).
    pub news: Vec<String>,
    /// Programm, das der Launcher startet (relativ zum Spielordner).
    pub executable: String,
    pub files: Vec<FileEntry>,
    /// Der Launcher selbst (für seine eigene Aktualisierung).
    pub launcher: Option<FileEntry>,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 16];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn unhex(text: &str) -> Option<Vec<u8>> {
    let text = text.trim();
    if text.len() % 2 != 0 {
        return None;
    }
    (0..text.len()).step_by(2).map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok()).collect()
}

// ---------------------------------------------------------------------------
// Signatur
// ---------------------------------------------------------------------------

/// Neuer geheimer Schlüssel (32 Byte, hex).
pub fn generate_secret_key() -> String {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).expect("Zufallszahlen des Systems nicht verfügbar");
    hex(&seed)
}

fn signing_key(secret_hex: &str) -> Result<SigningKey, String> {
    let bytes: [u8; 32] = unhex(secret_hex).and_then(|b| b.try_into().ok()).ok_or("Geheimer Schlüssel ist ungültig")?;
    Ok(SigningKey::from_bytes(&bytes))
}

/// Öffentlicher Schlüssel (hex) zu einem geheimen.
pub fn public_key(secret_hex: &str) -> Result<String, String> {
    Ok(hex(signing_key(secret_hex)?.verifying_key().as_bytes()))
}

pub fn sign(data: &[u8], secret_hex: &str) -> Result<String, String> {
    Ok(hex(&signing_key(secret_hex)?.sign(data).to_bytes()))
}

/// Prüft, ob `data` wirklich mit dem geheimen Schlüssel zu `public_hex` signiert wurde.
pub fn verify(data: &[u8], signature_hex: &str, public_hex: &str) -> bool {
    let Some(key) = unhex(public_hex).and_then(|b| <[u8; 32]>::try_from(b).ok()).and_then(|b| VerifyingKey::from_bytes(&b).ok()) else {
        return false;
    };
    let Some(signature) = unhex(signature_hex).and_then(|b| <[u8; 64]>::try_from(b).ok()).map(|b| Signature::from_bytes(&b)) else {
        return false;
    };
    key.verify(data, &signature).is_ok()
}

/// Manifest mit vorangestellter Signatur (Inhalt von `manifest.signed`).
pub fn signed_manifest(manifest: &Manifest, secret_hex: &str) -> Result<Vec<u8>, String> {
    let json = serde_json::to_vec_pretty(manifest).map_err(|e| e.to_string())?;
    let mut out = sign(&json, secret_hex)?.into_bytes();
    out.push(b'\n');
    out.extend_from_slice(&json);
    Ok(out)
}

/// Liest `manifest.signed` – nur wenn die Signatur stimmt.
pub fn parse_signed(bytes: &[u8], public_hex: &str) -> Result<Manifest, String> {
    let split = bytes.iter().position(|&b| b == b'\n').ok_or("Update-Liste unvollständig")?;
    parse_signed_manifest(&bytes[split + 1..], &String::from_utf8_lossy(&bytes[..split]), public_hex)
}

/// Liest ein Manifest nur, wenn die Signatur stimmt.
pub fn parse_signed_manifest(json: &[u8], signature_hex: &str, public_hex: &str) -> Result<Manifest, String> {
    if !verify(json, signature_hex, public_hex) {
        return Err("Die Update-Liste ist nicht korrekt signiert – aus Sicherheitsgründen wird nichts installiert.".into());
    }
    serde_json::from_slice(json).map_err(|e| format!("Update-Liste unlesbar: {e}"))
}

// ---------------------------------------------------------------------------
// Aktualisieren
// ---------------------------------------------------------------------------

/// Woher Dateien kommen: Webserver oder (zum Testen) ein Ordner.
pub trait Source: Send + Sync {
    /// Liest eine Datei relativ zur Update-Adresse (z. B. "manifest.json").
    fn fetch(&self, relative: &str, progress: &mut dyn FnMut(u64)) -> Result<Vec<u8>, String>;
}

/// Webserver (HTTPS).
pub struct HttpSource {
    pub base: String,
}

impl Source for HttpSource {
    fn fetch(&self, relative: &str, progress: &mut dyn FnMut(u64)) -> Result<Vec<u8>, String> {
        let url = format!("{}/{relative}", self.base.trim_end_matches('/'));
        let response = ureq::get(&url).call().map_err(|e| format!("{url}: {e}"))?;
        let mut reader = response.into_body().into_reader();
        let mut data = Vec::new();
        let mut buffer = vec![0u8; 1 << 16];
        loop {
            let read = reader.read(&mut buffer).map_err(|e| format!("{url}: {e}"))?;
            if read == 0 {
                break;
            }
            data.extend_from_slice(&buffer[..read]);
            progress(read as u64);
        }
        Ok(data)
    }
}

/// Ordner auf der Festplatte (Tests, Probelauf vor dem Hochladen).
pub struct DirSource {
    pub dir: PathBuf,
}

impl Source for DirSource {
    fn fetch(&self, relative: &str, progress: &mut dyn FnMut(u64)) -> Result<Vec<u8>, String> {
        let path = self.dir.join(relative);
        let data = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        progress(data.len() as u64);
        Ok(data)
    }
}

/// Webadresse oder Ordner.
pub fn source_for(location: &str) -> Box<dyn Source> {
    if location.starts_with("http://") || location.starts_with("https://") {
        Box::new(HttpSource { base: location.to_string() })
    } else {
        Box::new(DirSource { dir: PathBuf::from(location) })
    }
}

/// Holt Manifest und Signatur und prüft sie.
pub fn fetch_manifest(source: &dyn Source, public_hex: &str) -> Result<Manifest, String> {
    parse_signed(&source.fetch("manifest.signed", &mut |_| {})?, public_hex)
}

/// Was fehlt oder anders ist als im Manifest.
pub fn missing_files(manifest: &Manifest, game_dir: &Path) -> Vec<FileEntry> {
    manifest
        .files
        .iter()
        .filter(|entry| {
            let path = game_dir.join(&entry.path);
            // Größe zuerst (schnell), dann Prüfsumme – so werden auch beschädigte Dateien repariert.
            match std::fs::metadata(&path) {
                Ok(meta) if meta.len() == entry.size => sha256_file(&path).map_or(true, |hash| hash != entry.sha256),
                _ => true,
            }
        })
        .cloned()
        .collect()
}

/// Fortschritt beim Aktualisieren.
#[derive(Clone, Debug, Default)]
pub struct Progress {
    pub files_done: usize,
    pub files_total: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
}

/// Lädt fehlende Dateien, prüft jede einzeln und setzt sie ein. Ein Abbruch schadet nicht:
/// Beim nächsten Mal fehlen die Dateien eben noch und werden neu geholt.
pub fn apply_update(source: &dyn Source, manifest: &Manifest, game_dir: &Path, progress: &mut dyn FnMut(&Progress)) -> Result<(), String> {
    let missing = missing_files(manifest, game_dir);
    let mut state = Progress { files_total: missing.len(), bytes_total: missing.iter().map(|f| f.size).sum(), ..Default::default() };
    progress(&state);
    std::fs::create_dir_all(game_dir).map_err(|e| e.to_string())?;
    for entry in &missing {
        let target = safe_join(game_dir, &entry.path)?;
        let data = source.fetch(&format!("dateien/{}", entry.sha256), &mut |read| {
            state.bytes_done += read;
            progress(&state);
        })?;
        if sha256_hex(&data) != entry.sha256 {
            return Err(format!("{} ist beim Herunterladen beschädigt worden – bitte erneut versuchen.", entry.path));
        }
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        // Erst daneben schreiben, dann austauschen: nie eine halbe Datei am Zielort.
        let temporary = with_suffix(&target, ".download");
        std::fs::write(&temporary, &data).map_err(|e| format!("{}: {e}", temporary.display()))?;
        replace_file(&temporary, &target)?;
        state.files_done += 1;
        progress(&state);
    }
    remove_stale(manifest, game_dir);
    Ok(())
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Ersetzt `target` durch `source`. Läuft `target` gerade (Windows sperrt laufende
/// Programme gegen Überschreiben, erlaubt aber Umbenennen), wird es vorher beiseitegelegt.
pub fn replace_file(source: &Path, target: &Path) -> Result<(), String> {
    if std::fs::rename(source, target).is_ok() {
        return Ok(());
    }
    let old = with_suffix(target, ".alt");
    let _ = std::fs::remove_file(&old);
    std::fs::rename(target, &old).map_err(|e| format!("{} lässt sich nicht ersetzen: {e}", target.display()))?;
    std::fs::rename(source, target).map_err(|e| format!("{}: {e}", target.display()))
}

/// Pfade aus dem Manifest dürfen nicht aus dem Spielordner heraus zeigen.
fn safe_join(base: &Path, relative: &str) -> Result<PathBuf, String> {
    let ok = !relative.is_empty()
        && !relative.starts_with('/')
        && !relative.contains('\\')
        && relative.split('/').all(|part| !part.is_empty() && part != ".." && part != "." && !part.contains(':'));
    if ok { Ok(base.join(relative)) } else { Err(format!("Ungültiger Pfad in der Update-Liste: {relative}")) }
}

/// Löscht Dateien im Spielordner, die es in der neuen Version nicht mehr gibt
/// (nur Programmdateien – Einstellungen und Spielstände liegen woanders).
fn remove_stale(manifest: &Manifest, game_dir: &Path) {
    let wanted: std::collections::HashSet<PathBuf> = manifest.files.iter().filter_map(|f| safe_join(game_dir, &f.path).ok()).collect();
    for path in walk(game_dir) {
        if !wanted.contains(&path) {
            let _ = std::fs::remove_file(&path);
        }
    }
}

pub fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(walk(&path));
        } else {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// Erstellt ein Manifest aus einem Ordner (für das Release-Werkzeug).
pub fn manifest_from_dir(dir: &Path, version: &str, date: &str, news: Vec<String>, executable: &str) -> std::io::Result<Manifest> {
    let mut files = Vec::new();
    for path in walk(dir) {
        let relative = path
            .strip_prefix(dir)
            .expect("liegt im Ordner")
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        files.push(FileEntry { size: std::fs::metadata(&path)?.len(), sha256: sha256_file(&path)?, path: relative });
    }
    Ok(Manifest { version: version.into(), date: date.into(), news, executable: executable.into(), files, launcher: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("launcher_test_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn signatur_schuetzt_die_update_liste() {
        let secret = generate_secret_key();
        let public = public_key(&secret).unwrap();
        let json = br#"{"version":"1"}"#;
        let signature = sign(json, &secret).unwrap();
        assert!(verify(json, &signature, &public));
        assert!(!verify(br#"{"version":"2"}"#, &signature, &public), "veränderte Liste muss auffallen");
        let other = public_key(&generate_secret_key()).unwrap();
        assert!(!verify(json, &signature, &other), "fremder Schlüssel muss abgelehnt werden");
        assert!(!verify(json, "kaputt", &public));
    }

    /// Kompletter Ablauf: Release erstellen, erstinstallieren, aktualisieren, reparieren.
    #[test]
    fn installieren_aktualisieren_und_reparieren() {
        let secret = generate_secret_key();
        let public = public_key(&secret).unwrap();
        let release_dir = temp("release");
        let game_dir = temp("spiel");

        let publish = |files: &[(&str, &[u8])], version: &str| {
            let build = temp(&format!("build_{version}"));
            for (path, data) in files {
                let full = build.join(path);
                std::fs::create_dir_all(full.parent().unwrap()).unwrap();
                std::fs::write(full, data).unwrap();
            }
            let manifest = manifest_from_dir(&build, version, "2026-09-24", vec!["Neu".into()], "game.exe").unwrap();
            std::fs::create_dir_all(release_dir.join("dateien")).unwrap();
            for entry in &manifest.files {
                std::fs::copy(build.join(&entry.path), release_dir.join("dateien").join(&entry.sha256)).unwrap();
            }
            std::fs::write(release_dir.join("manifest.signed"), signed_manifest(&manifest, &secret).unwrap()).unwrap();
        };
        let source = DirSource { dir: release_dir.clone() };

        // Version 1: Erstinstallation
        publish(&[("game.exe", b"programm v1"), ("assets/natur/baum.gltf", b"baum"), ("assets/alt.txt", b"weg damit")], "1");
        let manifest = fetch_manifest(&source, &public).unwrap();
        assert_eq!(missing_files(&manifest, &game_dir).len(), 3);
        apply_update(&source, &manifest, &game_dir, &mut |_| {}).unwrap();
        assert!(missing_files(&manifest, &game_dir).is_empty());

        // Version 2: eine Datei geändert, eine entfernt – nur die Änderung wird geladen.
        publish(&[("game.exe", b"programm v2"), ("assets/natur/baum.gltf", b"baum")], "2");
        let manifest = fetch_manifest(&source, &public).unwrap();
        assert_eq!(missing_files(&manifest, &game_dir).len(), 1);
        let mut loaded = 0;
        apply_update(&source, &manifest, &game_dir, &mut |p| loaded = p.files_done).unwrap();
        assert_eq!(loaded, 1);
        assert_eq!(std::fs::read(game_dir.join("game.exe")).unwrap(), b"programm v2");
        assert!(!game_dir.join("assets/alt.txt").exists(), "veraltete Datei muss weg");

        // Beschädigte Datei wird beim nächsten Start repariert.
        std::fs::write(game_dir.join("assets/natur/baum.gltf"), b"baux").unwrap();
        assert_eq!(missing_files(&manifest, &game_dir).len(), 1);
        apply_update(&source, &manifest, &game_dir, &mut |_| {}).unwrap();
        assert_eq!(std::fs::read(game_dir.join("assets/natur/baum.gltf")).unwrap(), b"baum");

        // Manipulierte Liste wird abgelehnt.
        let mut forged = std::fs::read(release_dir.join("manifest.signed")).unwrap();
        let pattern = b"\"version\": \"2\"";
        let at = forged.windows(pattern.len()).position(|w| w == pattern).expect("Version steht in der Liste");
        forged[at + pattern.len() - 2] = b'9';
        std::fs::write(release_dir.join("manifest.signed"), forged).unwrap();
        assert!(fetch_manifest(&source, &public).is_err());
    }

    #[test]
    fn pfade_duerfen_nicht_ausbrechen() {
        let base = Path::new("C:/Spiel");
        assert!(safe_join(base, "assets/a.png").is_ok());
        for bad in ["../boese.exe", "/etc/passwd", "a/../../b", "C:/Windows/x", "a\\b", "", "a/./b"] {
            assert!(safe_join(base, bad).is_err(), "{bad}");
        }
    }
}
