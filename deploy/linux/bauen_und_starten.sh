#!/usr/bin/env bash
# Baut den Spielserver aus /opt/spiel/src und startet den Dienst neu.
# Läuft auf dem Server (als root) – "Server aktualisieren.bat" ruft es auf.
set -euo pipefail

QUELLE="/opt/spiel/src"
export PATH="$HOME/.cargo/bin:$PATH"
# Build-Ordner außerhalb der Quelle: bleibt bei Updates erhalten, dann geht es viel schneller.
export CARGO_TARGET_DIR="/opt/spiel/target"

cd "$QUELLE"
echo "== Bauen (beim ersten Mal ca. 10–20 Minuten, danach meist 1–3) =="
cargo build --release -p game

echo "== Installieren =="
install -m 755 "$CARGO_TARGET_DIR/release/game" /opt/spiel/game.neu
# Der Dienst speichert beim Stoppen die Welt; danach die neue Version einsetzen.
systemctl stop spielserver || true
mv /opt/spiel/game.neu /opt/spiel/game
systemctl start spielserver

sleep 2
systemctl --no-pager --lines 5 status spielserver
