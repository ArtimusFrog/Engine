"""Schutzstein am Ende jeder Heerstraße: Runenobelisk auf rundem Sockel mit leuchtendem Kristall.
Bricht eine Einheit der Festung bis hierher durch, verliert die Insel Leben (Tower Defense)."""

import math
import random

from burg import MAUER, PLATTEN, Bau, M, drehkoerper, platte, pyramide, quader, zylinder

zufall = random.Random(91)
bau = Bau(91)
STEIN, HELL, DUNKEL, GOLD, RUNE = "#A7A195", "#CFC8BA", "#6F6A62", "#D8AE4A", "#8FD8FF"

# Runder, gestufter Sockel
bau.teil(zylinder(2.6, 0.9, 12, 2.5), DUNKEL, MAUER, m=M((0, 0, -0.6)))
bau.teil(zylinder(2.4, 0.2, 12, 2.3), HELL, PLATTEN, m=M((0, 0, 0.3)))
bau.teil(zylinder(1.5, 0.35, 8, 1.4), STEIN, MAUER, m=M((0, 0, 0.5)))
# Obelisk, leicht verjüngt, mit Goldspitze
bau.teil(zylinder(0.75, 3.6, 4, 0.5, math.pi / 4), STEIN, MAUER, m=M((0, 0, 0.85)))
bau.teil(pyramide(0.52, 0.7, 4), GOLD, m=M((0, 0, 4.45)))
# Leuchtende Runen auf allen vier Seiten
for k in range(4):
    w = math.tau * k / 4
    for i, z in enumerate((1.5, 2.3, 3.1)):
        r = 0.72 - (z - 0.85) / 3.6 * 0.25 + 0.02
        m = M((math.cos(w) * r, math.sin(w) * r, z), math.degrees(w) - 90)
        form = [(-0.14, 0.0), (0.14, 0.0), (0.0, 0.35)] if i % 2 else [(-0.12, 0.0), (0.12, 0.0), (0.12, 0.3), (-0.12, 0.3)]
        bau.teil(platte(form, 0.03, 0.015), RUNE, m=m, leuchten=True)
# Schwebender Kristall über der Spitze
bau.teil(drehkoerper([(0.0, 5.4), (0.32, 5.85), (0.0, 6.6)], 6), RUNE, m=M((0, 0, 0)), leuchten=True)
# Vier kleine Wächtersteine mit Goldkappe rundherum
for k in range(4):
    w = math.tau * k / 4 + math.pi / 4
    m = M((math.cos(w) * 2.0, math.sin(w) * 2.0, 0.3), zufall.uniform(0, 90))
    bau.teil(quader(0.45, 0.45, 1.0, 0.05), STEIN, MAUER, m=m)
    bau.teil(pyramide(0.3, 0.25, 4), GOLD, m=m @ M((0, 0, 1.0)))
bau.fertig("Schutzstein", glas_leuchten=2.5, ursprung=(0.0, 0.0))
