# Engine

Eigene Spiele-Engine in Rust, nativ für Windows (DirectX 12 / Vulkan über wgpu).

## Aufbau

- `crates/engine` – die Engine als Bibliothek (Fenster, Rendering, Physik, Netzwerk, Editor)
- `game` – das Spiel, das auf der Engine aufbaut (erzeugt die `.exe`)

## Voraussetzungen

- Visual Studio Build Tools mit „Desktopentwicklung mit C++“
- Rust über `rustup`

## Starten

    cargo run -p game

## Release-Build (.exe)

    cargo build -p game --release

Die fertige Datei liegt danach unter `target/release/game.exe`.
