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

## Steuerung (Spielplatz)

| Eingabe | Aktion |
|---|---|
| Linksklick ins Fenster | Maus einfangen |
| Maus | Kamera drehen |
| Mausrad | Zoom |
| W / A / S / D | Laufen |
| Shift | Rennen |
| Leertaste | Springen |
| Linksklick (Maus gefangen) | Ball werfen |
| Esc | Maus freigeben |
| F1 | Freie Kamera an/aus (rechte Maustaste + WASD/QE) |
| F12 | Screenshot nach `screenshots/` |

## Tests

    cargo test -p engine

Prüft die Physik ohne Fenster (Fallen, Landen, Laufen, Treppen, Springen, Sichtstrahlen).

## Automatischer Screenshot

    cargo run -p game -- --screenshot screenshots/test.png --frames 60

Startet das Spiel, speichert nach 60 Frames ein Bild und beendet sich. Mit `--autopilot` läuft die Figur dabei von allein los.
