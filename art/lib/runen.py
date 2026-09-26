"""Runen: der Runenstein (schwebt über einem Schutzstein, dessen Siedlungsplatz vergeben ist, und
groß über dem Brunnen im Burghof) und der Runenbrunnen – vier Runensäulen und ein leuchtender
Runenkreis um den Brunnen der Burg Grünfels. Dort vereinen Spieler vier Runenfragmente zu einem
Runenstein. Gebaut mit dem Baukasten aus burg.py (leuchtende Teile: `leuchten=True`).
"""

import math
import random

from burg import MAUER, Bau, M, drehkoerper, platte, pyramide, quader, zylinder

STEIN, HELL, DUNKEL, GOLD, RUNE = "#6E7BA0", "#CFC8BA", "#6F6A62", "#D8AE4A", "#8FD8FF"
RUNENSTEIN = "#5F6C94"


def _rune(bau, m, groesse=1.0, art=0):
    """Ein leuchtendes Runenzeichen (flach, auf einer Fläche, die nach -Y schaut)."""
    g = groesse
    formen = [
        [(-0.03 * g, 0.0), (0.03 * g, 0.0), (0.03 * g, 0.3 * g), (-0.03 * g, 0.3 * g)],
        [(-0.12 * g, 0.12 * g), (0.12 * g, 0.2 * g), (0.12 * g, 0.25 * g), (-0.12 * g, 0.17 * g)],
        [(-0.1 * g, 0.0), (0.1 * g, 0.0), (0.0, 0.26 * g)],
    ]
    teile = [formen[0], formen[1]] if art % 2 == 0 else [formen[0], formen[2]]
    for form in teile:
        bau.teil(platte(form, 0.02, 0.0), RUNE, m=m, leuchten=True)


def runenstein(seed=41):
    """Sechskantiger Runenstein mit Goldband, leuchtenden Runen und drei schwebenden Splittern."""
    zufall = random.Random(seed)
    bau = Bau(seed)
    profil = [(0.0, 0.0), (0.26, 0.12), (0.34, 0.45), (0.31, 0.95), (0.18, 1.22), (0.0, 1.34)]
    bau.teil(drehkoerper(profil, 6), RUNENSTEIN, MAUER)
    bau.teil(zylinder(0.355, 0.07, 6, 0.345), GOLD, m=M((0, 0, 0.5)))
    bau.teil(zylinder(0.33, 0.05, 6, 0.32), GOLD, m=M((0, 0, 0.95)))
    for k in range(6):
        w = math.tau * (k + 0.5) / 6
        r = 0.3
        m = M((math.cos(w) * r, math.sin(w) * r, 0.62), math.degrees(w) - 90)
        _rune(bau, m, 0.9, k)
    for k in range(3):
        w = math.tau * k / 3 + 0.4
        z = zufall.uniform(0.3, 1.0)
        m = M((math.cos(w) * 0.6, math.sin(w) * 0.6, z), zufall.uniform(0, 90), zufall.uniform(-20, 20))
        bau.teil(drehkoerper([(0.0, 0.0), (0.07, 0.1), (0.0, 0.26)], 5), RUNE, m=m, leuchten=True)
    bau.fertig("Runenstein", glas_leuchten=3.0, ursprung=(0.0, 0.0))


def runenbrunnen(seed=42):
    """Vier Runensäulen und ein Runenkreis um den Brunnen im Burghof (Ursprung = Brunnenmitte)."""
    bau = Bau(seed)
    # Runenkreis im Pflaster: zwei Goldringe, dazwischen leuchtende Runenplatten
    for r, n in ((5.6, 40), (7.2, 48)):
        for k in range(n):
            w = math.tau * (k + 0.5) / n
            laenge = math.tau * r / n * 1.02
            bau.teil(quader(laenge, 0.12, 0.03), GOLD, m=M((math.cos(w) * r, math.sin(w) * r, 0.085), math.degrees(w) + 90))
    for k in range(16):
        w = math.tau * k / 16
        m = M((math.cos(w) * 6.4, math.sin(w) * 6.4, 0.095), math.degrees(w) + 90, -90)
        _rune(bau, m, 1.6, k)
    # Vier Runensäulen (schräg zur Straße, die den Platz von vorn nach hinten quert)
    for k in range(4):
        w = math.tau * k / 4 + math.pi / 4
        mitte = (math.cos(w) * 8.4, math.sin(w) * 8.4, 0.0)
        m = M(mitte, math.degrees(w) + 90)
        bau.teil(quader(1.3, 1.3, 0.35, 0.05), DUNKEL, MAUER, m=m)
        bau.teil(quader(1.05, 1.05, 0.25, 0.04), HELL, MAUER, m=m @ M((0, 0, 0.35)))
        bau.teil(zylinder(0.46, 2.7, 4, 0.34, math.pi / 4), HELL, MAUER, m=m @ M((0, 0, 0.6)))
        bau.teil(quader(0.8, 0.8, 0.14, 0.03), GOLD, m=m @ M((0, 0, 3.3)))
        bau.teil(pyramide(0.36, 0.5, 4), GOLD, m=m @ M((0, 0, 3.44)))
        # Runen auf allen vier Seiten, die zur Brunnenmitte leuchtet am hellsten (Fassung für ein Fragment)
        for seite in range(4):
            sm = m @ M((0, 0, 0), 90 * seite)
            for i, z in enumerate((1.1, 1.8, 2.5)):
                r = 0.44 - (z - 0.6) / 2.7 * 0.12 + 0.015
                _rune(bau, sm @ M((0, -r, z)), 1.1, i + seite)
        bau.teil(drehkoerper([(0.0, 0.0), (0.18, 0.28), (0.0, 0.7)], 6), RUNE, m=m @ M((0, 0, 4.15)), leuchten=True)
    bau.fertig("Runenbrunnen", glas_leuchten=2.6, ursprung=(0.0, 0.0))
