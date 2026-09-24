# Eigene Assets mit Blender

Alle Modelle des Spiels entstehen hier. Die Quelle ist entweder ein **Python-Skript**, das
Blender das Modell bauen lässt, oder eine **.blend-Datei**, die von Hand modelliert wurde.
Ergebnisse (`.gltf` + `.bin`) landen in `game/assets/` und werden mit eingecheckt, damit das
Spiel auch ohne Blender baut.

```
art/lib/werkstatt.py            gemeinsame Hilfsfunktionen und Farbpalette
art/lib/bauen.py                führt eine Quelle in Blender aus und exportiert sie
art/modelle/natur/fels.py   →   game/assets/natur/fels.gltf
art/modelle/tiere/fuchs.blend → game/assets/tiere/fuchs.gltf
```

## Arbeiten

- **Ansehen mit Live-Aktualisierung:** Quelle oder fertiges Modell auf `Asset ansehen.bat`
  ziehen (oder `game.exe --ansehen art/modelle/natur/fels_test.py`). Der Betrachter baut die
  Quelle mit Blender neu, sobald sie gespeichert wird, und lädt das Ergebnis automatisch.
- **Live in Blender zuschauen:** Skript auf `Blender live.bat` ziehen. Blender öffnet sich mit
  dem Modell und baut es bei jedem Speichern des Skripts (oder von `art/lib/*.py`) sofort neu –
  inklusive Export, sodass Spiel-Betrachter und Asset-Galerie mitziehen. Leertaste spielt die
  Animation ab. Fehler stehen oben im 3D-Fenster.
- **Alles bauen:** `Assets bauen.bat` (oder einzelne Dateien daraufziehen).
- Blender wird unter `C:\Program Files\Blender Foundation\` gesucht, oder per
  Umgebungsvariable `BLENDER=<Pfad zu blender.exe>`.

## Regeln, damit alles zusammenpasst

| Thema | Regel |
|---|---|
| Maßstab | 1 Blender-Einheit = 1 Meter. Spielfigur = 1,8 m (Vergleichsfigur im Betrachter). |
| Ursprung | Mittig unter dem Modell auf dem Boden (Z = 0). Der Betrachter warnt sonst rot. |
| Richtung | Vorderseite schaut in Blender nach **-Y** (Ansicht „Vorne“, Ziffernblock 1). |
| Stil | Low-Poly, flach schattiert, Farben nur aus der Palette in `werkstatt.py`. |
| Materialien | Eine Grundfarbe je Material, keine Texturen nötig. Beidseitig nur für dünne Flächen (Blätter, Stoff). |
| Größe | Bäume ≤ ~1500 Dreiecke, Deko ≤ ~300, Figuren/Tiere ≤ ~3000. |
| Animationen | Eine Aktion je Clip, Namen wie `Idle`, `Laufen`, `Rennen`, `Angriff`, `Tod`. Schleifen enden in der Startpose. 30 Bilder/s. |
| Knochen | Ein Skelett je Modell, Befestigungspunkte heißen `handslot.r`, `handslot.l`, `kopf`. |
| Namen | Dateien klein, mit Unterstrich: `eiche_gross.py`, `fuchs.blend`. |

## Eine .blend-Datei von Hand bearbeiten

Einfach in Blender öffnen, bearbeiten, speichern. Liegt sie unter `art/modelle/`, baut der
Betrachter sie beim Speichern neu. Soll ein Skript-Modell von Hand weiterbearbeitet werden,
wandelt Claude es in eine `.blend` um und löscht das Skript, damit nicht zwei Quellen
dasselbe Asset erzeugen.
