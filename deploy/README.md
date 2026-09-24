# Spielserver auf dem Strato-VPS

Der Server läuft dort als Linux-Dienst ohne Fenster (`game --server`), startet nach einem
Neustart des VPS von selbst und speichert die Welt in `/var/lib/spiel/welt.json`.

## Einmalig einrichten

1. **Betriebssystem:** Im Strato-Kundenbereich den VPS mit **Ubuntu 24.04** aufsetzen. Notiere
   dir die **IP-Adresse**. Das root-Passwort brauchst du nur einmal, für Schritt 2.
2. **SSH-Schlüssel** (damit kein Passwort mehr nötig ist). In der Eingabeaufforderung:
   ```
   ssh-keygen -t ed25519
   ```
   Alle Fragen mit Enter bestätigen. Danach den Schlüssel auf den Server bringen (`IP` ersetzen, dabei
   einmal das root-Passwort eingeben):
   ```
   type %USERPROFILE%\.ssh\id_ed25519.pub | ssh root@IP "mkdir -p ~/.ssh && cat >> ~/.ssh/authorized_keys"
   ```
   Test: `ssh root@IP` muss jetzt ohne Passwort klappen.
3. **Adresse eintragen:** Datei `deploy\server.txt` anlegen mit genau einer Zeile, z. B.
   `root@85.215.123.45`. Die Datei wird nicht eingecheckt.
4. **`Server einrichten.bat`** doppelklicken. Das installiert die Pakete, richtet die Firewall ein
   (SSH + UDP 7777), legt den Dienst an, baut den Server und startet ihn. Das erste Bauen
   dauert 10–20 Minuten.
5. Hat Strato im Kundenbereich eine eigene Firewall, dort ebenfalls **UDP 7777** freigeben.

Mitspieler verbinden sich dann im Spiel über **Beitreten** mit der IP-Adresse.

## Neue Version aufspielen

Änderungen committen, dann **`Server aktualisieren.bat`**. Hochgeladen wird der letzte Commit.
Der Server speichert vor dem Neustart, niemand verliert etwas.

## Nützliche Befehle auf dem Server (`ssh root@IP`)

| Befehl | Wozu |
|---|---|
| `systemctl status spielserver` | Läuft er? |
| `journalctl -u spielserver -f` | Log live ansehen (Strg+C beendet nur die Anzeige) |
| `systemctl restart spielserver` | Neu starten (speichert vorher) |
| `cp /var/lib/spiel/welt.json ~/welt-sicherung.json` | Spielstand sichern |
