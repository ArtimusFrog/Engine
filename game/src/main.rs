// Im Release-Build kein schwarzes Konsolenfenster neben dem Spiel öffnen.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod client;
mod playground;
mod protocol;
mod server;
mod world;

use engine::prelude::*;

use playground::{Mode, Playground};
use protocol::DEFAULT_PORT;

/// Kommandozeile:
/// - ohne Angabe: allein spielen
/// - `--host [--port 7777]`: spielen und Server für andere sein
/// - `--join <adresse[:port]>`: mit einem Server verbinden
/// - `--server [--port 7777]`: nur Server, ohne Fenster (z. B. auf dem VPS)
fn parse_mode(args: &[String]) -> Mode {
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let port = value("--port").and_then(|p| p.parse().ok()).unwrap_or(DEFAULT_PORT);
    if args.iter().any(|a| a == "--server") {
        Mode::Server { port }
    } else if args.iter().any(|a| a == "--host") {
        Mode::Host { port }
    } else if let Some(address) = value("--join") {
        let address = if address.contains(':') { address } else { format!("{address}:{DEFAULT_PORT}") };
        Mode::Join { address }
    } else {
        Mode::Offline
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = parse_mode(&args);
    let game = Playground::new(mode.clone(), args.iter().any(|a| a == "--autopilot"));
    if let Mode::Server { .. } = mode {
        run_headless(game);
    } else {
        run(EngineConfig { title: "Engine JN – Spielplatz".into(), ..Default::default() }, game);
    }
}
