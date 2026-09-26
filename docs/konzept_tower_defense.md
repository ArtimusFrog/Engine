# Konzept: Tower Defense an den Heerstraßen

## Grundidee

Die Schattenfestung schickt in Wellen Truppen über vier gleich lange Heerstraßen. Die Spieler bauen
Türme an den Straßenrand, um sie aufzuhalten, bevor sie das Ende der Straße erreichen. Türme kosten
die Rohstoffe, die die Spieler auf der Insel sammeln (Holz, Stein, Eisenerz). Besiegte Truppen
bringen Beute, mit der sich Türme aufwerten lassen. So greifen Sammeln, Bauen und Verteidigen
ineinander.

## Bauen

- **Baumenü (B)** mit zwei Reitern: *Gebäude* (Holzfäller, Steinbruch, Erzmine) und *Türme*.
- **Nur an der Straße:** Die Turmmitte muss 4–14 m vom Rand einer Heerstraße entfernt sein. Auf der
  Straße selbst geht es nicht, sie muss frei bleiben. Außerdem gilt ein Mindestabstand zu anderen
  Türmen und Gebäuden (Grundfläche 3 m Radius).
- **Vorschau** wie bei den Gebäuden: grün oder rot mit Grund. Dazu ein Ring, der die Reichweite zeigt.
- **Bauzeit:** 10–20 s mit der vorhandenen Bauanimation (Schichten, Gerüst, Staub). Erst wenn der Turm
  fertig ist, schießt er.
- **Besitz:** Jeder Turm gehört dem Spieler, der ihn gebaut hat. Aufwerten darf jeder, abreißen nur
  der Besitzer.

## Aufwerten

- Jeder Turm hat **3 Stufen**. Auf einen Turm zielen und **E** drücken öffnet das Turmfenster mit
  Werten, nächster Stufe, Kosten, *Aufwerten* und *Abreißen* (50 % der Kosten zurück).
- Beim Aufwerten wird der Turm sichtbar größer (neues Modell je Stufe) und kurz erneut
  eingerüstet (5 s). In der Zeit schießt er nicht.
- **Werte je Stufe** (Richtwerte): Schaden ×1,6, Reichweite +10 %, Feuerrate +10 %. Dazu wird der
  Spezialeffekt stärker (siehe Tabelle).

## Die zehn Türme

| # | Turm | Rolle | Schaden | Besonderheit (Stufe 1 → 3) |
|---|------|-------|---------|------------------------------|
| 1 | **Pfeilturm** | günstiger Allrounder | physisch | schnelle Pfeile, Einzelziel |
| 2 | **Balliste** | Panzerbrecher | Durchschlag | langsam, sehr hoher Einzelschaden, halbiert die Rüstung |
| 3 | **Katapult** | Flächenschaden | physisch | Steinbrocken, Wirkung auf 3 → 4,5 m, Mindestreichweite 8 m |
| 4 | **Feuerturm** | Brand | Feuer | Flammenstoß setzt Ziele 3 s in Brand (Schaden über Zeit) |
| 5 | **Frostturm** | Kontrolle | Frost | verlangsamt um 25 → 45 % (Golems nur halb so stark) |
| 6 | **Blitzturm** | Gruppen | Blitz | Kettenblitz springt auf 3 → 5 Ziele |
| 7 | **Sonnenturm** | gegen Untote | heilig | Lichtstrahl, doppelter Schaden an Skeletten und Gespenstern |
| 8 | **Arkanturm** | gegen Rüstung | arkan | ignoriert Rüstung, trifft Gespenster voll |
| 9 | **Giftturm** | Zermürbung | Gift | Giftwolke am Boden (Fläche, Schaden über Zeit), senkt die Rüstung |
| 10 | **Kriegsbanner** | Unterstützung | – | Türme im Umkreis von 12 m: +15 → 35 % Schaden und Feuerrate |

### Richtwerte Stufe 1

| Turm | Reichweite | Schuss alle | Schaden |
|------|-----------:|------------:|--------:|
| Pfeilturm | 20 m | 0,8 s | 14 |
| Balliste | 28 m | 2,6 s | 70 |
| Katapult | 30 m | 3,2 s | 40 (Fläche) |
| Feuerturm | 12 m | 1,2 s | 10 + 12/s Brand |
| Frostturm | 16 m | 1,4 s | 6 + Verlangsamung |
| Blitzturm | 18 m | 1,8 s | 26 je Ziel |
| Sonnenturm | 22 m | 1,5 s | 22 (×2 gegen Untote) |
| Arkanturm | 20 m | 1,6 s | 30 (ohne Rüstung) |
| Giftturm | 15 m | 2,4 s | 8/s Gift, Wolke 4 s |
| Kriegsbanner | 12 m (Aura) | – | – |

## Gegner-Eigenschaften (neu)

Die Lebenspunkte werden feiner (vorher 3–14 Treffer, jetzt echte Werte). Der Zauberstab des Spielers
macht 20 Schaden (arkan).

| Einheit | Leben | Rüstung (physisch) | Besonderheiten |
|---------|------:|-------------------:|----------------|
| Dunkler Ritter | 160 | 40 % | – |
| Bogenschütze | 70 | 10 % | – |
| Pikenier | 120 | 25 % | – |
| Skelettkrieger | 80 | 10 % | untot: heilig ×2, immun gegen Gift |
| Dunkelmagier | 90 | 0 % | 40 % Widerstand gegen arkan und Blitz |
| Steingolem | 420 | 60 % | Verlangsamung nur halb so stark, Feuer ×0,5 |
| Schattenwolf | 60 | 0 % | schnell |
| Gespenst | 100 | 75 % | untot: heilig ×2; arkan trifft voll |

Durchschlag halbiert die Rüstung, arkan ignoriert sie. Feuer, Frost, Blitz, heilig und Gift werden
nicht von der Rüstung gebremst.

## Zielwahl

Jeder Turm nimmt das Ziel, das **am weitesten auf der Straße** gekommen ist („erstes“). Das ist in
Tower Defense üblich und für alle nachvollziehbar. Später lässt sich im Turmfenster umstellen:
*erstes / stärkstes / schwächstes*.

## Beute

Besiegte Einheiten geben dem Besitzer des Turms (bzw. dem Spieler mit dem Stab), der den letzten
Treffer gesetzt hat, Rohstoffe:

| Einheit | Beute |
|---|---|
| Wolf, Bogenschütze, Skelett | 1 Stein |
| Pikenier, Magier, Gespenst | 1 Stein + 1 Erz |
| Ritter | 2 Erz |
| Golem | 4 Stein + 3 Erz |

## Kosten (Holz / Stein / Erz)

| Turm | Stufe 1 | Stufe 2 | Stufe 3 |
|------|---------|---------|---------|
| Pfeilturm | 12 / 4 / 0 | 16 / 8 / 2 | 20 / 12 / 6 |
| Balliste | 18 / 8 / 2 | 22 / 12 / 5 | 26 / 16 / 10 |
| Katapult | 20 / 12 / 0 | 24 / 18 / 4 | 28 / 24 / 8 |
| Feuerturm | 10 / 14 / 2 | 14 / 18 / 5 | 18 / 22 / 9 |
| Frostturm | 8 / 14 / 4 | 12 / 18 / 7 | 16 / 22 / 11 |
| Blitzturm | 8 / 12 / 8 | 12 / 16 / 12 | 16 / 20 / 16 |
| Sonnenturm | 10 / 16 / 4 | 14 / 20 / 8 | 18 / 24 / 12 |
| Arkanturm | 10 / 14 / 6 | 14 / 18 / 10 | 18 / 22 / 14 |
| Giftturm | 14 / 8 / 2 | 18 / 12 / 5 | 22 / 16 / 9 |
| Kriegsbanner | 16 / 6 / 4 | 20 / 10 / 8 | 24 / 14 / 12 |

## Technik

- **Server-autoritativ:** Der Server wählt Ziele, rechnet Schaden und Effekte (Brand, Frost, Gift als
  Zustände an der Einheit). Die Clients bekommen Schüsse als kurze Ereignisse, zeigen Geschosse und
  Treffer und drehen den Turmkopf zum Ziel.
- **Türme sind Gebäude** (`bauten.rs`): gleiche Platzierung, Speicherung, Bauanimation und
  Kollision. Neu sind Art = Turm, Stufe und Richtung des Kopfs.
- **Modelle** (`art/lib/tuerme.py`): ein Turm je Stufe (`bauten/turm_<art>_<stufe>.gltf`) und ein
  beweglicher Kopf je Art (`bauten/turm_<art>_kopf.gltf`), der sich zum Ziel dreht (Armbrust,
  Balliste, Katapultarm, Kristall …).
- **Gleiche Schnittstelle für Spielerangriffe:** Zauberstab und Türme machen Schaden über dieselbe
  Funktion (`Heer::damage(id, menge, art)`).

## Später (nicht in diesem Schritt)

- **Ziel am Straßenende:** Eine Dorf- oder Burg-„Lebensleiste“. Durchgebrochene Truppen ziehen
  Leben ab, bei 0 ist die Runde verloren.
- **Wellen mit steigender Stärke** (Welle 1, 2, 3 … mit Anzeige und Countdown) und Boss-Wellen.
- **Lebenspunkte für Spieler**, damit die Angriffe der Truppen wehtun.
- **Zielwahl** im Turmfenster umstellbar.
