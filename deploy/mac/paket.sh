#!/usr/bin/env bash
# Baut aus den Universal-Programmen in <ordner> (game, launcher) die Mac-App „Engine JN.app“
# und das Download-Abbild EngineJN-macOS.dmg. Läuft auf einem Mac (GitHub-Workflow „Mac bauen“).
#
# Ergebnis in <ordner>:
#   game                  – das Spiel (ad-hoc signiert), wird vom Launcher heruntergeladen
#   launcher              – der Launcher aus der App (für das Selbst-Update)
#   EngineJN-macOS.dmg    – der Download für die Webseite
set -euo pipefail

OUT="$1"
NAME="Engine JN"
APP="$OUT/$NAME.app"
ICON_SRC="launcher/assets/icon.png"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$OUT/launcher" "$APP/Contents/MacOS/launcher"
chmod 755 "$APP/Contents/MacOS/launcher" "$OUT/game"

# Symbol (.icns) in allen Größen
ICONSET="$(mktemp -d)/AppIcon.iconset"
mkdir -p "$ICONSET"
for s in 16 32 128 256 512; do
  sips -z "$s" "$s" "$ICON_SRC" --out "$ICONSET/icon_${s}x${s}.png" >/dev/null
  d=$((s * 2))
  sips -z "$d" "$d" "$ICON_SRC" --out "$ICONSET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"

# Info.plist bleibt über Versionen gleich: Sonst passt die Signatur des Launchers nach
# einem Selbst-Update nicht mehr zur App.
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$NAME</string>
  <key>CFBundleDisplayName</key><string>$NAME</string>
  <key>CFBundleIdentifier</key><string>de.enginejn.launcher</string>
  <key>CFBundleExecutable</key><string>launcher</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.games</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# Ad-hoc-Signatur (Pflicht auf Apple Silicon). Ohne Apple-Entwicklerkonto nicht beglaubigt –
# beim ersten Start einmal „Dennoch öffnen“ (Systemeinstellungen → Datenschutz & Sicherheit).
codesign --force --sign - "$OUT/game"
codesign --force --deep --sign - "$APP"
# Der signierte Launcher aus der App ist auch die Datei für Selbst-Updates.
cp "$APP/Contents/MacOS/launcher" "$OUT/launcher"

# DMG mit Verknüpfung auf „Programme“ zum Hineinziehen
STAGE="$(mktemp -d)"
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Programme"
hdiutil create -volname "$NAME" -srcfolder "$STAGE" -ov -format UDZO "$OUT/EngineJN-macOS.dmg"

ls -la "$OUT"
