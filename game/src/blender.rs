//! Assets mit Blender bauen: startet Blender ohne Fenster mit `art/lib/bauen.py`, das ein
//! Modell-Skript (oder eine .blend-Datei) aus `art/modelle/` nach `game/assets/` exportiert.

use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::SystemTime;

/// Wo das fertige Asset einer Quelle landet:
/// `…/art/modelle/natur/fels.py` → `…/game/assets/natur/fels.gltf`.
pub fn asset_for_source(source: &Path) -> Option<PathBuf> {
    let parts: Vec<Component> = source.components().collect();
    let at = parts.windows(2).position(|w| w[0].as_os_str() == "art" && w[1].as_os_str() == "modelle")?;
    let root: PathBuf = parts[..at].iter().collect();
    let relative: PathBuf = parts[at + 2..].iter().collect();
    Some(root.join("game").join("assets").join(relative).with_extension("gltf"))
}

/// Ist das eine Quelle (Blender-Skript oder .blend) statt eines fertigen Assets?
pub fn is_source(path: &Path) -> bool {
    matches!(path.extension().and_then(|e| e.to_str()), Some("py" | "blend"))
}

/// Sucht blender.exe: Umgebungsvariable `BLENDER`, sonst die neueste Version unter
/// „C:\Program Files\Blender Foundation“.
pub fn find_blender() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("BLENDER").map(PathBuf::from).filter(|p| p.is_file()) {
        return Some(path);
    }
    let base = PathBuf::from(std::env::var_os("ProgramFiles").unwrap_or_else(|| "C:\\Program Files".into())).join("Blender Foundation");
    let mut found: Vec<PathBuf> = std::fs::read_dir(base)
        .ok()?
        .flatten()
        .map(|entry| entry.path().join("blender.exe"))
        .filter(|exe| exe.is_file())
        .collect();
    found.sort();
    found.pop()
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Ein laufender oder abgeschlossener Bau.
pub struct Build {
    child: Child,
    log: PathBuf,
}

pub enum BuildState {
    Running,
    Done,
    Failed(String),
}

impl Build {
    /// Startet Blender im Hintergrund für eine Quelle.
    pub fn start(blender: &Path, source: &Path) -> Result<Build, String> {
        let root = repo_root(source).ok_or("Die Quelle liegt nicht unter art/modelle/")?;
        let log = std::env::temp_dir().join(format!("asset_bau_{}.log", std::process::id()));
        let out = std::fs::File::create(&log).map_err(|e| format!("Protokoll nicht anlegbar: {e}"))?;
        let err = out.try_clone().map_err(|e| e.to_string())?;
        let mut command = Command::new(blender);
        command
            .args(["--background", "--factory-startup", "--python-exit-code", "1", "--python"])
            .arg(root.join("art").join("lib").join("bauen.py"))
            .arg("--")
            .arg(source)
            .current_dir(&root)
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(err);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // Kein schwarzes Konsolenfenster aufpoppen lassen.
            command.creation_flags(0x0800_0000);
        }
        let child = command.spawn().map_err(|e| format!("Blender ließ sich nicht starten: {e}"))?;
        Ok(Build { child, log })
    }

    pub fn poll(&mut self) -> BuildState {
        match self.child.try_wait() {
            Ok(None) => BuildState::Running,
            Ok(Some(status)) if status.success() => BuildState::Done,
            Ok(Some(_)) => BuildState::Failed(self.error_excerpt()),
            Err(e) => BuildState::Failed(e.to_string()),
        }
    }

    /// Die aussagekräftigen Zeilen aus Blenders Ausgabe (ab dem Python-Fehler).
    fn error_excerpt(&self) -> String {
        let text = std::fs::read_to_string(&self.log).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        let start = lines.iter().position(|l| l.starts_with("Traceback")).unwrap_or(lines.len().saturating_sub(8));
        let excerpt: Vec<&str> = lines[start..].iter().copied().filter(|l| !l.trim().is_empty()).take(14).collect();
        if excerpt.is_empty() { "Blender hat abgebrochen (keine Ausgabe).".into() } else { excerpt.join("\n") }
    }
}

impl Drop for Build {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = std::fs::remove_file(&self.log);
    }
}

fn repo_root(source: &Path) -> Option<PathBuf> {
    let parts: Vec<Component> = source.components().collect();
    let at = parts.windows(2).position(|w| w[0].as_os_str() == "art" && w[1].as_os_str() == "modelle")?;
    Some(parts[..at].iter().collect())
}

/// Jüngste Änderung an der Quelle oder an der gemeinsamen Werkstatt-Bibliothek.
pub fn source_stamp(source: &Path) -> Option<SystemTime> {
    let own = modified(source)?;
    let library = repo_root(source).and_then(|root| modified(&root.join("art").join("lib").join("werkstatt.py")));
    Some(library.map_or(own, |l| l.max(own)))
}

/// Muss neu gebaut werden (Asset fehlt oder ist älter als die Quelle)?
pub fn needs_build(source: &Path) -> bool {
    let Some(asset) = asset_for_source(source) else { return false };
    match (modified(&asset), source_stamp(source)) {
        (Some(built), Some(changed)) => changed > built,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quelle_auf_asset_abbilden() {
        let asset = asset_for_source(Path::new("C:/Spiel/art/modelle/natur/fels.py")).unwrap();
        assert_eq!(asset, Path::new("C:/Spiel/game/assets/natur/fels.gltf"));
        assert!(asset_for_source(Path::new("C:/Spiel/anderswo/fels.py")).is_none());
        assert!(is_source(Path::new("baum.blend")));
        assert!(!is_source(Path::new("baum.gltf")));
    }
}
