// Im Release-Build kein schwarzes Konsolenfenster neben dem Spiel öffnen.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod client;
mod island;
mod models;
mod playground;
mod protocol;
mod server;
mod session;
mod settings;
mod ui;
mod world;

use engine::prelude::*;

use playground::Playground;
use protocol::DEFAULT_PORT;
use session::Mode;

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
    let mode = parse_mode(&args);
    let headless = matches!(mode, Some(Mode::Server { .. }));
    let game = Playground::new(mode, args.iter().any(|a| a == "--autopilot"));
    if headless {
        run_headless(game);
    } else {
        run(EngineConfig { title: "Spielplatz".into(), ..Default::default() }, game);
    }
}
