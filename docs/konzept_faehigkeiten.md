# Konzept: Fähigkeiten der Helden (Überarbeitung)

Jede Fähigkeit soll sich **wuchtig** anfühlen und **Tiefe** haben: eine eigene Animation mit
Ausholen, Wirkung und Nachschwung, Effekte in mehreren Schichten (Blitz, Kern, Schweif, Druckwelle,
Spuren am Boden), Kamerawackeln und Treffer-Stopp bei schweren Schlägen, eigene Klänge – und
Mechaniken, die sich gegenseitig verstärken.

Code: `faehigkeiten.rs` (Werte), `server.rs` (`cast`, `land_hits`), `zauberbild.rs` (Effekte),
`heer.rs`/`wildnis.rs` (Einfrieren), `characters.rs` (Oberkörper-Animation), Animationen in
`art/lib/figuren.py` (Magier) und `art/lib/zwerg.py` (Zwerg).

## Gemeinsame Mechaniken

- **Einfrieren** (Frostnova): Der Gegner ist in Eis gefangen – bewegt sich nicht und greift nicht an.
  Der **nächste Treffer zerschmettert** das Eis und macht **+50 % Schaden** (Hammer auf Eis,
  Arkanlanze auf Eis …). Bosse bleiben nur 40 % so lange gefroren.
- **Kombos und Ladungen** werden vom Server gezählt; die eigene Figur zeigt sie sofort.
- Beim Laufen spielen die Fähigkeiten nur auf dem **Oberkörper** – die Beine laufen weiter, nichts
  rutscht. Im Stand wirkt der ganze Körper mit (Ausfallschritt, in die Knie gehen, Sprung).

## Magier

| Taste | Fähigkeit | Wirkung |
|---|---|---|
| 3 | **Arkangeschoss** | 20 Schaden, ignoriert Rüstung, 0,6 s. Jeder Treffer lädt eine **arkane Ladung** (bis 3, sichtbar als kreisende Lichter um den Magier, verfällt nach 8 s). Mit 3 Ladungen wird der nächste Schuss zur **Arkanlanze**: ein Strahl, der **alle Gegner in einer Linie** durchbohrt (44 Schaden). |
| 4 | **Feuerball** | 34 Schaden im Umkreis von 3,8 m, Brand 8/s für 4 s. Hinterlässt einen **Flammenteppich** (3 m, 4 s), der alle darin weiter brennen lässt. 5 s. |
| 5 | **Frostnova** | Eine **Eiswelle** breitet sich vom Magier aus (7,5 m): 18 Schaden, **friert 1,5 s ein**, danach verlangsamt. Eisdornen brechen aus dem Boden. 10 s. |

Animationen: *Arkan* (schneller Stoß mit dem Stab, die linke Hand schleudert nach), *Feuerball*
(beide Hände sammeln Glut vor der Brust, weit ausholen, mit Körperdrehung schleudern), *Frostnova*
(Stab hoch, in die Knie, mit dem Stabende auf den Boden stoßen, Arme auseinander).

## Zwerg

| Taste | Fähigkeit | Wirkung |
|---|---|---|
| 3 | **Hammerschlag** | **Dreierkombo:** Schwinger von rechts (28), Rückhand (28), dann **Schmetterschlag** von oben (50, betäubt 0,5 s, Druckwelle 2,2 m um den Einschlag mit halbem Schaden). Wer innerhalb von 1 s weiterschlägt, setzt die Kombo fort. |
| 4 | **Wurfhammer** | Der Zwerg **schleudert seinen Hammer** (das echte Modell, er kreiselt): 30 Schaden, betäubt 1,2 s, dann **prallt er** auf bis zu zwei weitere Gegner im Umkreis von 9 m ab (75 % / 55 %, betäubt 0,6 s) und **fliegt zurück** in die Hand. 5 s. |
| 5 | **Erdbeben** | Sprung und Aufschlag: innen (3 m) 36 Schaden und 1,8 s betäubt, außen (bis 6,5 m) 24 Schaden und 1 s betäubt. Felsdornen brechen in Wellen aus dem Boden, Risse glühen auf. Zwei **Nachbeben** (nach 0,7 s und 1,4 s, 5 m) machen je 10 Schaden und verlangsamen. 12 s. |

Animationen: *Schlag1/Schlag2* (waagerechte Schwünge mit Hüftdrehung), *Schlag3* (Hammer über den
Kopf, Ausfallschritt, Schmettern), *Wurf* (weit hinter den Kopf ausholen, schleudern, nachfedern),
*Fangen* (Hand hoch, Hammer kommt zurück), *Beben* (in die Knie, Sprung, Hammer mit beiden Händen
in den Boden).

## Effekte (nur Optik, `zauberbild.rs`)

- Geschosse aus mehreren Teilen: heller Kern, flackernde Hülle, Schweif, Funken, Licht.
- Einschläge: Lichtblitz, Druckwellenring am Boden, Splitter, Rauch; Brandflecken, Raureif und
  Risse bleiben eine Weile am Boden liegen und versinken dann.
- Eisdornen und Felsdornen wachsen in Wellen aus dem Boden und ziehen sich wieder zurück.
- Gefrorene Gegner stecken in Eiskristallen; zerschmettert das Eis, fliegen Splitter.
- Kamerawackeln nach Entfernung; kurzer Treffer-Stopp in der Animation bei schweren Schlägen.
