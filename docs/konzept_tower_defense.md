# Konzept: Tower Defense an den Heerstraßen

## Grundidee

Aus den vier Toren der Schattenfestung marschieren in Wellen Truppen über gleich lange Heerstraßen
(380 m) bis zu den Schutzsteinen am Straßenende. Jeder Durchbruch kostet die Insel Leben. Die Spieler
bauen Türme neben die Straßen und Fallen auf sie, werten sie auf, wählen auf Stufe 3 eine Richtung
und kämpfen selbst mit dem Zauberstab mit. Ziel: **Welle 30 überstehen** (danach wahlweise Endlosmodus).

Code: `heer.rs` (Truppen), `tuerme.rs` (Türme, Soldaten, Fallen), `td.rs` (geteilte Regeln),
`td_ui.rs` / `td_ansicht.rs` (Oberfläche und Effekte), `strassenbild.rs` (Pflaster und Laternen).

## Ablauf einer Partie

- Alle 45 s eine Welle, je Straße eine Gruppe. Mit **N** (oder im Fenster **T**) ruft man sie früher –
  die gesparten Sekunden bringen allen Gold (0,8 Gold je Sekunde).
- **Wellenvorschau**: Die nächste Welle steht vorher fest und wird angezeigt (Wellenleiste, Fenster T mit
  den Eigenschaften jeder Einheit).
- Zähigkeit: +12 % Leben je Welle, alle drei Wellen eine Einheit mehr je Gruppe, jede fünfte Welle ein
  Boss (Golem, Ritter, Magier, Gespenst reihum). Mehr Spieler = +30 % Leben je weiterem Spieler.
- **Schwierigkeit** (Admin-Panel): Leicht 30 Leben, Truppen ×0,7 · Normal 20 / ×1 · Schwer 15 / ×1,35 ·
  Albtraum 10 / ×1,8 (weniger Gold je höher).
- Nach Welle 30: Sieg, 500 Gold und 20 Erz für alle; im Endlosmodus geht es weiter (+10 % Zähigkeit je Welle darüber).
- **Auswertung** nach jeder überstandenen Welle: besiegt, durchgebrochen, bester Turm, Schaden je
  Spieler, Wellenbonus (10 + 3 × Welle Gold für alle).
- **Straßen aufteilen** (Fenster T): Wer eine Straße verteidigt, steht in der Übersicht und in der Wellenleiste.

## Gold

Gold ist die Währung für Türme und Fallen, dazu kommen Holz, Stein und Erz. Quellen: Kopfgeld für
jeden Sieg (4–15, Boss 60, +5 % je Welle), Wellenbonus, frühes Rufen, Goldader der Schatzkammer.
Jeder Spieler bekommt einmal 150 Startgold.

## Gegner und ihre Eigenschaften

| Einheit | Besonderheit | Antwort |
|---|---|---|
| Dunkler Ritter | Schild: Nachbarn im Umkreis von 5 m nehmen 30 % weniger von Pfeilen und Bolzen | Felsbrocken (Katapult B), Läuterung (Sonne A), Magie |
| Dunkelmagier | heilt Einheiten in der Nähe alle 4 s | Gift (Heilsperre), Läuterung |
| Steingolem | zerfällt beim Tod in drei Felslinge | Flächenschaden, Runenstampfer |
| Harpyie | fliegt – Katapult, Feuer, Gift, Runen, Fallen und Soldaten treffen sie nicht | Pfeil, Balliste B (holt sie runter), Sturm B, Frost, Blitz |
| Schattenmeuchler | getarnt – Türme sehen ihn nur aufgedeckt | Späherturm, Sonne A; Spieler und Soldaten sehen ihn immer |
| Gespenst | 75 % Rüstung, ×2 durch Heiliges, Runen treffen es nicht | Sonne, Arkan, Paladine |
| Skelett | immun gegen Gift, ×2 durch Heiliges | Sonne, Paladine |

**Bosse** (jede fünfte Welle, einer je Straße, eigene Leiste oben):

| Welle | Boss | Fähigkeiten |
|---|---|---|
| 5 | Steingolem (6-faches Leben, 1,6-fach groß) | stampft alle 9 s und legt Türme im Umkreis von 14 m für 3 s lahm, zerfällt in drei Golems |
| 10 | Bergtroll | heilt sich ständig (nicht solange er brennt – Feuer!), schleudert Felsen auf Soldaten und Barrikaden |
| 15 | Lichkönig | Frostnova: Türme im Umkreis schießen 5 s halb so schnell; Gefallene in seiner Nähe stehen als Skelette auf; anfällig für Heiliges |
| 20 | Spinnenkönigin | legt alle 7 s vier Spinnlinge, spinnt Türme im Umkreis von 18 m für 3,5 s ein |
| 25 | Dämonenfürst | immun gegen Feuer, ×1,5 durch Heiliges, Glutaura verbrennt Soldaten, ersteht bei halbem Leben einmal aus den Flammen |
| 30 | Schattendrache (Endgegner) | immun gegen Feuer, Flammenatem auf Soldaten und Barrikaden, steigt alle 16 s für 6 s auf (dann nur Anti-Luft, Harpune holt ihn runter) |

Im Endlosmodus kommen danach alle reihum, dazu Ritter, Magier und Gespenst als vergrößerte Anführer.
Die großen Bosse kosten beim Durchbruch 8 Leben ihrer Straße und bringen 150 (Drache 300) Gold.

## Leben je Straße

Es gibt keinen gemeinsamen Pool: jede Straße hat ihre eigenen Leben. Fällt eine Straße, verschwinden ihre
Truppen, die Siedlung an ihrem Ende (Dorfhalle und Gebäude des Besitzers im Radius) wird zerstört und die
Straße fängt mit vollen Leben neu an. Die Wellen laufen für alle weiter.

## Zielmodi

Jeder zielende Turm: **Erster** (Standard), **Letzter**, **Stärkster**, **Schwächster**, **Boss zuerst** –
im Turmfenster (E) wählbar.

## Die fünfzehn Türme und ihre Richtungen auf Stufe 3

| Turm | Rolle | A | B |
|---|---|---|---|
| Pfeilturm | günstig, trifft Flieger | Salve (3 Ziele) | Scharfschütze (Reichweite ×1,6, 25 % dreifach) |
| Balliste | durchschlagend | Durchbohren (ganze Linie) | Harpune (Flieger zuerst, holt sie 5 s runter, ×1,5) |
| Katapult | Gruppen, nicht nah, keine Flieger | Brandtöpfe (brennender Boden) | Felsbrocken (betäubt, bricht Schilde) |
| Feuerturm | Brand | Flächenbrand (springt beim Tod über) | Drachenatem (Kegel, doppelter Schaden) |
| Frostturm | verlangsamen | Einfrieren (jeder 4. Treffer 1,5 s) | Frostfeld (ständige Aura, auch Flieger) |
| Blitzturm | Kette, ×1,5 an Vereisten | Überladung (×2,5 an Vereisten/Betäubten) | Gewitter (4 Einschläge im Umkreis) |
| Sonnenturm | heilig | Läuterung (aufdecken, Heilsperre, Schilde weg) | Sonnenstrahl (Dauerstrahl, ×2 an Bossen) |
| Arkanturm | +15 % je Treffer am selben Ziel | Fokus (bis +300 %) | Arkane Kugel (springt beim Sieg weiter) |
| Giftturm | Wolke: Heilsperre, Rüstung −20 % | Seuche (neue Wolke beim Tod) | Säure (Rüstung 0, +25 % Schaden) |
| Kriegsbanner | Türme in der Nähe schneller und härter | Kriegstrommel (doppelt Tempo) | Heilbanner (heilt Soldaten, Barrikaden) |
| **Kaserne** | 3 Soldaten halten Gruppen auf | Veteranen (4, +50 % Leben, Rüstung) | Paladine (heilig, heilen sich) |
| **Späherturm** | deckt Getarnte auf, markiert (+15–30 % Schaden) | Adlerauge (3 Ziele, Reichweite ×1,5) | Kopfgeldjäger (doppeltes Kopfgeld) |
| **Sturmturm** | wirft Gruppen zurück, ×2 an Fliegern | Orkan (fast doppelt so weit, betäubt) | Sturmwand (×3 an Fliegern, holt sie runter) |
| **Runenstampfer** | Bodenwelle um den Turm, betäubt | Beben (größer, 1,5 s) | Runenfeld (3 s starke Verlangsamung) |
| **Schatzkammer** | +50–80 % Beute und Kopfgeld in Reichweite | Goldader (8 Gold alle 10 s) | Tributkammer (doppelte Beute) |

Stufen: Schaden ×1,6 je Stufe, Reichweite +10 %, Feuerrate +10 %. Kosten in Gold, Holz, Stein, Erz;
Abreißen gibt die Hälfte zurück.

## Kombos

- **Gift + Feuer**: Vergiftete explodieren bei einem Feuertreffer (3 m Umkreis, verbraucht das Gift).
- **Frost + Blitz**: Blitze machen an verlangsamten Zielen 1,5-fachen Schaden (Überladung 2,5-fach).
- **Späher-Markierung**: alle Treffer auf markierte Ziele machen mehr Schaden.
- **Teer + Feuer**: geteerte Einheiten nehmen 1,5-fachen Feuerschaden und brennen doppelt.

## Fallen (direkt auf der Straße, rasten mittig und quer ein)

Stachelfalle (30 Schaden/s) · Teergrube (halbes Tempo, Feuer doppelt) · Barrikade (hält Gruppen auf,
420 Leben, Heilbanner reparieren sie). Nicht gegen Flieger und Gespenster.

## Übersicht und Rückmeldung

- Wellenleiste oben: Welle x/30, Countdown, Leben, Vorschau, Straße, Hinweis N/T.
- Bossleisten, Schadenszahlen (abschaltbar in den Einstellungen), Zustände unter dem Fadenkreuz
  (fliegt, getarnt, geschützt, markiert, betäubt, brennt, vergiftet …).
- Turmfenster: Werte mit Schaden pro Sekunde, Kills und Schaden bisher, Zielmodus, nächste Stufe bzw.
  beide Richtungen nebeneinander.
- Fenster T: nächste Welle mit Eigenschaften, früh rufen, Straßen, Beitrag je Spieler, letzte Auswertung.

## Balancing

`cargo test --release balancing -- --nocapture` spielt die Wellen gegen drei Verteidigungen:
ohne Türme fällt die Insel in den ersten Wellen, vier Türme auf Stufe 2 je Straße halten bis etwa
Welle 9, eine voll ausgebaute Verteidigung (13 Türme je Straße auf Stufe 3) gewinnt.

## Später

- Spieler-Lebenspunkte (Truppen greifen Spieler an, bisher nur Optik)
- Eigene Kampagnenkarten, Belohnungen je Schwierigkeit, Bestenliste
