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

## Multiplayer

Am einfachsten über das Hauptmenü: „Spiel hosten“ bzw. „Beitreten“ mit Adresse.
Einstellungen (Name, Maus, Sichtfeld, Vollbild, VSync) werden unter
`%APPDATA%/EngineJN/einstellungen.json` gespeichert.

Alternativ zum Doppelklicken (nutzen den Release-Build):

| Datei | Was passiert |
|---|---|
| `Allein spielen.bat` | Einzelspieler |
| `Spiel hosten.bat` | Selbst spielen und gleichzeitig Server für andere sein |
| `Beitreten.bat` | Fragt nach der Adresse und verbindet sich |

Oder über die Kommandozeile:

    game.exe --host [--port 7777]      # spielen und hosten
    game.exe --join 100.64.1.2[:7777]  # beitreten
    game.exe --server [--port 7777]    # nur Server, ohne Fenster (z. B. auf dem VPS)

Das Spiel nutzt **UDP-Port 7777**. Beim ersten Hosten fragt die Windows-Firewall, ob
`game.exe` Verbindungen annehmen darf – mit „Zulassen“ bestätigen.

Übers Internet ohne eigenen Server: beide installieren [Tailscale](https://tailscale.com),
der Host startet `Spiel hosten.bat`, der andere `Beitreten.bat` mit der Tailscale-IP des Hosts.

**So funktioniert es:** Der Server rechnet die Physik für alle; Clients schicken nur ihre
Eingaben. Die eigene Figur bewegt sich trotzdem sofort (Vorhersage) und wird korrigiert,
falls der Server etwas anderes berechnet. Mitspieler und Objekte werden ~130 ms verzögert
zwischen zwei Server-Ständen interpoliert, damit sie flüssig laufen.

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
| Esc | Menü (Weiter, Einstellungen, Hauptmenü, Beenden) |
| Tab (halten) | Spielerliste |
| F3 | Debug-Anzeige (FPS, Objekte, Draw-Calls) |
| F6 | Eine Stunde vorspulen (nur allein oder als Host) |
| F1 | Freie Kamera an/aus (rechte Maustaste + WASD/QE) |
| F12 | Screenshot nach `screenshots/` |

## Tests

    cargo test --workspace

Prüft ohne Fenster: Physik (Fallen, Landen, Laufen, Treppen, Springen, Sichtstrahlen,
Nachspielen von Eingaben), Netzwerk und Multiplayer (Server und Client im selben
Prozess: Verbinden, Bewegung, Vorhersage ohne Korrekturen, synchrone Kisten).

## Automatischer Screenshot

    cargo run -p game -- --screenshot screenshots/test.png --frames 60

Startet das Spiel, speichert nach 60 Frames ein Bild und beendet sich. Mit `--autopilot` läuft die Figur dabei von allein los.

## Spielfiguren

Die Figuren (Ritter, Barbar, Magier, Schurkin) stammen aus dem
[KayKit Character Pack: Adventurers](https://github.com/KayKit-Game-Assets/KayKit-Character-Pack-Adventures-1.0)
von Kay Lousberg – Lizenz CC0 (frei nutzbar, auch kommerziell). Details in
`game/assets/characters/LICENSE-KayKit.txt`. Die Figur wählt man in den Einstellungen.

Die Engine lädt glTF-Modelle (`.glb`) mit Skelett, Textur und Animationen
(`engine::model::Model`, `Animator`) – eigene Modelle aus Blender funktionieren genauso.

Test-Schalter für Screenshots: `--figur barbar`, `--kamera-vorne`, `--demo-hacken`.

## Tag und Nacht

Ein Spieltag dauert etwa 20 Minuten (eine Stunde pro Minute, nachts doppelt so schnell).
Die Uhrzeit bestimmt der Server und schickt sie mit jedem Weltzustand an alle Spieler.
Die Leiste oben zeigt Tageszeit, Uhr, Tag und den Stand von Sonne bzw. Mond.
Test-Schalter: `--uhrzeit 22.5`, `--blick-hoch`.
