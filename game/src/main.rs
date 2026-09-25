// Im Release-Build kein schwarzes Konsolenfenster neben dem Spiel öffnen.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod animals;
mod asset_files;
mod blender;
mod characters;
mod chat;
mod client;
mod inventar;
mod hauptmenue;
mod island;
mod karte;
mod leben;
mod markierungen;
mod models;
mod orte;
mod playground;
mod protocol;
mod server;
mod save;
mod session;
mod sounds;
mod settings;
mod ui;
mod viewer;
mod wetter;
mod world;

use engine::prelude::*;

use playground::Playground;
use protocol::DEFAULT_PORT;
use session::Mode;

/// Wird gesetzt, wenn der Server beendet werden soll (Strg+C oder `systemctl stop`).
/// Das Spiel beendet dann den Takt sauber und speichert.
pub static STOP_REQUESTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Kommandozeile (ohne Angabe startet das Hauptmenü):
/// - `--offline`: direkt allein spielen
/// - `--host [--port 7777]`: direkt spielen und Server für andere sein
/// - `--join <adresse[:port]>`: direkt mit einem Server verbinden
/// - `--server [--port 7777]`: nur Server, ohne Fenster (z. B. auf dem VPS)
fn parse_mode(args: &[String]) -> Option<Mode> {
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let port = value("--port").and_then(|p| p.parse().ok()).unwrap_or(DEFAULT_PORT);
    if args.iter().any(|a| a == "--server") {
        Some(Mode::Server { port })
    } else if args.iter().any(|a| a == "--host") {
        Some(Mode::Host { port })
    } else if let Some(address) = value("--join") {
        let address = if address.contains(':') { address } else { format!("{address}:{DEFAULT_PORT}") };
        Some(Mode::Join { address })
    } else if args.iter().any(|a| a == "--offline" || a == "--autopilot") {
        Some(Mode::Offline)
    } else {
        None
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // `--klaenge-exportieren <ordner>`: erzeugte Klänge als WAV speichern (zum Anhören).
    if let Some(dir) = args.iter().position(|a| a == "--klaenge-exportieren").and_then(|i| args.get(i + 1)) {
        match sounds::export_all(std::path::Path::new(dir)) {
            Ok(files) => println!("{} Klänge gespeichert in {dir}", files.len()),
            Err(e) => eprintln!("Speichern fehlgeschlagen: {e}"),
        }
        return;
    }
    // `--ansehen <datei.gltf>`: nur den Asset-Betrachter öffnen.
    if let Some(path) = args.iter().position(|a| a == "--ansehen").and_then(|i| args.get(i + 1)) {
        run(EngineConfig { title: "Asset-Betrachter".into(), ..Default::default() }, viewer::Viewer::new(path.into()));
        return;
    }
    let mode = parse_mode(&args);
    let headless = matches!(mode, Some(Mode::Server { .. }));
    let game = Playground::new(mode, args.iter().any(|a| a == "--autopilot"));
    if headless {
        if let Err(e) = ctrlc::set_handler(|| STOP_REQUESTED.store(true, std::sync::atomic::Ordering::SeqCst)) {
            log::warn!("Strg+C lässt sich nicht abfangen: {e}");
        }
        run_headless(game);
        log::info!("Server beendet");
    } else {
        let fullscreen = settings::Settings::load().fullscreen;
        run(EngineConfig { title: "Engine JN".into(), fullscreen, ..Default::default() }, game);
    }
}
