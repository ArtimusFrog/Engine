//! Installation auf dem Rechner des Spielers (ohne Admin-Rechte):
//! Launcher in den Installationsordner kopieren, Verknüpfungen, Eintrag unter
//! „Apps & Features“ zum Deinstallieren, Neustart nach Updates.

use std::path::{Path, PathBuf};
use std::process::Command;

use launcher::config::{install_dir, installed_launcher, GAME_NAME};

/// Registry-Schlüssel für „Apps & Features“ (nur für den aktuellen Benutzer).
#[cfg(windows)]
const UNINSTALL_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\EngineJN";

/// Wurde der Installationsort zum Testen überschrieben (`--ordner`)? Dann keine
/// Verknüpfungen und keine Registry-Einträge.
fn test_mode() -> bool {
    std::env::args().any(|a| a == "--ordner")
}

/// Argumente, die an einen neu gestarteten Launcher weitergegeben werden.
fn passthrough_args() -> Vec<String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut keep = Vec::new();
    for flag in ["--quelle", "--ordner", "--screenshot"] {
        if let Some(value) = args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)) {
            keep.push(flag.to_string());
            keep.push(value.clone());
        }
    }
    keep
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase(),
        _ => false,
    }
}

/// Läuft gerade der installierte Launcher (nicht der frisch heruntergeladene)?
pub fn is_installed_copy() -> bool {
    if !cfg!(windows) {
        // Auf dem Mac liegt der Launcher als App im Programme-Ordner; Selbst-Update folgt später.
        return false;
    }
    std::env::current_exe().is_ok_and(|exe| same_file(&exe, &installed_launcher()))
}

/// Muss erst installiert werden? (Nur Windows; auf dem Mac zieht man die App in „Programme“.)
pub fn needs_install() -> bool {
    cfg!(windows) && !is_installed_copy()
}

/// Installiert den Launcher und startet die installierte Kopie.
pub fn install(desktop_shortcut: bool) -> Result<(), String> {
    let target = installed_launcher();
    std::fs::create_dir_all(install_dir()).map_err(|e| format!("{}: {e}", install_dir().display()))?;
    let current = std::env::current_exe().map_err(|e| e.to_string())?;
    std::fs::copy(&current, &target).map_err(|e| format!("{}: {e}", target.display()))?;
    make_executable(&target);
    if !test_mode() {
        integrate(&target, desktop_shortcut);
    }
    Command::new(&target).args(passthrough_args()).current_dir(install_dir()).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(windows)]
fn powershell(script: &str) {
    use std::os::windows::process::CommandExt;
    let result = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x0800_0000)
        .status();
    if let Err(e) = result {
        log::warn!("PowerShell nicht ausführbar: {e}");
    }
}

#[cfg(windows)]
fn reg(args: &[&str]) {
    use std::os::windows::process::CommandExt;
    let _ = Command::new("reg").args(args).creation_flags(0x0800_0000).status();
}

/// Verknüpfungen (Startmenü, optional Desktop) und Eintrag zum Deinstallieren.
#[cfg(windows)]
fn integrate(target: &Path, desktop_shortcut: bool) {
    let exe = target.display().to_string().replace('\'', "''");
    let dir = install_dir().display().to_string().replace('\'', "''");
    let mut script = format!(
        "$w = New-Object -ComObject WScript.Shell; \
         foreach ($d in @([Environment]::GetFolderPath('Programs'){desktop})) {{ \
           $s = $w.CreateShortcut((Join-Path $d '{GAME_NAME}.lnk')); $s.TargetPath = '{exe}'; $s.WorkingDirectory = '{dir}'; $s.Save() }}",
        desktop = if desktop_shortcut { ", [Environment]::GetFolderPath('Desktop')" } else { "" },
    );
    script = script.replace("  ", " ");
    powershell(&script);

    let uninstall = format!("\"{}\" --deinstallieren", target.display());
    let icon = target.display().to_string();
    let location = install_dir().display().to_string();
    for (name, value) in [
        ("DisplayName", GAME_NAME),
        ("Publisher", GAME_NAME),
        ("UninstallString", uninstall.as_str()),
        ("DisplayIcon", icon.as_str()),
        ("InstallLocation", location.as_str()),
    ] {
        reg(&["add", UNINSTALL_KEY, "/v", name, "/t", "REG_SZ", "/d", value, "/f"]);
    }
    for name in ["NoModify", "NoRepair"] {
        reg(&["add", UNINSTALL_KEY, "/v", name, "/t", "REG_DWORD", "/d", "1", "/f"]);
    }
}

#[cfg(not(windows))]
fn integrate(_target: &Path, _desktop_shortcut: bool) {}

/// Entfernt Programmdateien, Verknüpfungen und den Registry-Eintrag.
/// Einstellungen und Spielstände (anderer Ordner) bleiben.
pub fn uninstall() {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if !test_mode() {
            powershell(&format!(
                "foreach ($d in @([Environment]::GetFolderPath('Programs'), [Environment]::GetFolderPath('Desktop'))) {{ \
                 Remove-Item -LiteralPath (Join-Path $d '{GAME_NAME}.lnk') -ErrorAction SilentlyContinue }}"
            ));
            reg(&["delete", UNINSTALL_KEY, "/f"]);
        }
        // Der Launcher läuft noch – den Ordner erst löschen, wenn er beendet ist.
        let dir = install_dir();
        let command = format!("ping 127.0.0.1 -n 3 > nul & rmdir /s /q \"{}\"", dir.display());
        let _ = Command::new("cmd").args(["/C", &command]).creation_flags(0x0800_0000).spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::fs::remove_dir_all(install_dir().join("spiel"));
    }
}

/// Unter Unix muss ein heruntergeladenes Programm ausführbar gemacht werden.
pub fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// Startet den (neuen) Launcher mit denselben Test-Schaltern.
pub fn restart(exe: &Path) {
    let _ = Command::new(exe).args(passthrough_args()).current_dir(install_dir()).spawn();
}

/// Nach einem Selbst-Update: die beiseitegelegte alte Version entfernen.
pub fn cleanup_after_update() {
    let Ok(current) = std::env::current_exe() else { return };
    let mut old = current.into_os_string();
    old.push(".alt");
    let old = PathBuf::from(old);
    // Die alte Version beendet sich gerade erst – kurz Zeit geben.
    for _ in 0..10 {
        if !old.exists() || std::fs::remove_file(&old).is_ok() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

pub fn open_folder(path: &Path) {
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = Command::new(program).arg(path).spawn();
}
