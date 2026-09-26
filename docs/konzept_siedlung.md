# Konzept: Siedlungen am Ende der Heerstraßen

Am Ende jeder der vier Heerstraßen liegt ein **Siedlungsplatz**: ebener Boden (50 m, weich auslaufend
bis 90 m), ohne Bäume und Felsen im Kern, mit einem Dorfweg vom Straßenende bis in die Mitte.

## Die Dorfhalle

- Jeder Spieler baut genau **eine** Dorfhalle, jeder Siedlungsplatz trägt genau **eine** Siedlung.
  Wer am Ende einer Straße siedelt, verteidigt diese Straße (Fenster T).
- Stufen: **Dorfhalle** (70 m Bauradius, 30 Holz + 15 Stein) → **Rathaus** (95 m, schaltet die Erzmine
  frei; 120 Gold, 40 Holz, 30 Stein, 8 Erz) → **Burgfried** (120 m; 300 Gold, 60 Holz, 60 Stein, 25 Erz).
  Nur der Besitzer baut aus (E auf die Halle).
- Holzfäller, Steinbruch und Erzmine gehen nur im Radius der eigenen Dorfhalle; Türme und Fallen
  weiterhin an der ganzen Straße.
- **R** zeigt die Radien (eigene golden, fremde hell) und freie Siedlungsplätze (grün); beim Platzieren
  erscheinen sie von selbst.
- Vorteile: Start und Wiedereinstieg vor der eigenen Dorfhalle, +5/10/20 Gold je überstandener Welle,
  Gebäude im Radius arbeiten 10 % schneller. Die Dorfhalle ersetzt den Schutzstein ihrer Straße.
- Aussehen (`art/lib/siedlung.py`): Langhaus aus Fachwerk mit Vorlaube, Brunnen und Glockengalgen →
  Rathaus mit Steingeschoss, Uhrturm, Balkon und zwei Marktständen → zusätzlich der Burgfried mit
  Erkertürmen und Pyramidendach, dazu Mauern mit zwei Rundtürmen am Dorfplatz.

## Später

Durchgebrochene Truppen greifen die Gebäude der Siedlung und zuletzt die Dorfhalle an (Lebenspunkte je
Stufe); die Leben der Insel werden dann durch die Siedlungen ersetzt.
