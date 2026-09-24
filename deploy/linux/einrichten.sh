#!/usr/bin/env bash
# Richtet einen frischen Linux-Server (Ubuntu/Debian, z. B. Strato VPS) für den Spielserver ein.
# Einmal als root ausführen – macht "Server einrichten.bat" automatisch.
#
# Danach läuft der Spielserver als Dienst "spielserver" unter dem Benutzer "spiel":
#   systemctl status spielserver     – läuft er?
#   journalctl -u spielserver -f     – Log live ansehen
#   systemctl restart spielserver    – neu starten (speichert vorher)
set -euo pipefail

PORT="${PORT:-7777}"
QUELLE="/opt/spiel/src"

echo "== Pakete =="
export DEBIAN_FRONTEND=noninteractive
apt-get update -q
apt-get install -y -q build-essential pkg-config curl ufw libasound2-dev

echo "== Benutzer und Ordner =="
id spiel &>/dev/null || useradd --system --home-dir /opt/spiel --shell /usr/sbin/nologin spiel
mkdir -p /opt/spiel /var/lib/spiel
chown spiel:spiel /var/lib/spiel

echo "== Auslagerungsdatei (der Rust-Compiler braucht beim Bauen viel Speicher) =="
if ! swapon --show | grep -q .; then
    fallocate -l 4G /swapfile
    chmod 600 /swapfile
    mkswap /swapfile
    swapon /swapfile
    grep -q '^/swapfile' /etc/fstab || echo '/swapfile none swap sw 0 0' >> /etc/fstab
fi

echo "== Rust =="
if ! command -v cargo &>/dev/null && [ ! -x "$HOME/.cargo/bin/cargo" ]; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi

echo "== Firewall: SSH und Spiel-Port (UDP $PORT) =="
# Erst SSH erlauben, dann einschalten – sonst sperrt man sich selbst aus.
ufw allow OpenSSH
ufw allow "$PORT/udp"
ufw --force enable

echo "== Dienst =="
sed "s/--port 7777/--port $PORT/" "$QUELLE/deploy/linux/spielserver.service" > /etc/systemd/system/spielserver.service
systemctl daemon-reload
systemctl enable spielserver

echo "== Bauen und starten =="
bash "$QUELLE/deploy/linux/bauen_und_starten.sh"

echo
echo "Fertig. Der Spielserver läuft an UDP-Port $PORT."
echo "Falls Strato zusätzlich eine Firewall im Kundenbereich hat: dort ebenfalls UDP $PORT freigeben."
