"""Waffen der Spielfiguren (siehe game/src/waffen.rs): fünf Stäbe für den Magier, fünf Hämmer für
den Zwerg. Jede Waffe wird zweimal gebraucht: als Anbauteil in der Hand der Figur (figuren.py,
zwerg.py) und als liegendes Modell am Boden, wenn ein Gegner sie fallen lässt (beute.py).

Stäbe: senkrecht, unten bei z ≈ 0, gegriffen bei z ≈ 0,93 (Hand des Magiers), Kopf oben.
Hämmer: hängen aus der Faust (Griff bei z ≈ 0,6) mit dem Kopf nach unten (z ≈ 0,2).
Bausteine aus figuren.py (Lofts, Kugeln, Strähnen, Sterne); Farben als Vertexfarben.
"""

import math

from mathutils import Vector

from figuren import farbe

X, Y, Z = Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))

STAEBE = ["stab_eiche", "stab_glut", "stab_frost", "stab_sturm", "stab_sternen"]
HAEMMER = ["hammer_eisen", "hammer_runen", "hammer_streit", "hammer_donner", "hammer_drachen"]


def _schaft(f, name, x, y, von, bis, dicke, farbe_von, gewicht, wackeln=0.006, ringe=18, knoten=()):
    punkte = []
    for i in range(ringe):
        t = i / (ringe - 1)
        z = von + (bis - von) * t
        w = Vector((wackeln * math.sin(i * 0.9), wackeln * math.cos(i * 1.2), 0))
        r = dicke * (1.0 - 0.2 * t) + (0.006 if i in knoten else 0.0)
        punkte.append((Vector((x, y, z)) + w, X, Y, r, r))
    f.loft(name, punkte, 12, farbe_von, gewicht, unten_zu=True, oben_zu=True, teilung=2)


def _griff(holz, leder, z_von=0.84, z_bis=1.02):
    def farbe_von(i, k, p):
        z = p.center.z
        if z_von < z < z_bis:
            return leder * (0.75 if int(z * 90) % 2 else 1.0)
        return holz * (0.82 + 0.18 * ((k * 3 + int(i * 5)) % 4) / 3)
    return farbe_von


def stab(f, art, x, y, gewicht):
    """Einer der fünf Stäbe (`art` aus STAEBE) mit dem Griff bei (x, y, 0,93)."""
    gold, gold_dunkel = farbe("#D8AE4A"), farbe("#A9812E")
    leder = farbe("#4A2F1C")
    oben = Vector((x, y, 1.93))
    if art == "stab_eiche":
        holz = farbe("#6B4726")
        _schaft(f, "Eichenschaft", x, y, 0.03, 1.9, 0.024, _griff(holz, leder), gewicht, wackeln=0.014, knoten=(3, 7, 11, 14))
        # Wurzelkrone, die einen Bernstein umklammert
        for n in range(4):
            w = math.tau * n / 4 + 0.3
            aussen = Vector((math.cos(w), math.sin(w), 0))
            punkte = [oben - Z * 0.02, oben + aussen * 0.05 + Z * 0.04, oben + aussen * 0.065 + Z * 0.12, oben + aussen * 0.03 + Z * 0.19]
            f.straehne("Wurzel", punkte, 0.014, 0.004, holz * 0.9, gewicht, 8, 0.3)
        f.kugel("Bernstein", oben + Z * 0.1, (0.05, 0.05, 0.055), farbe("#F0A030"), gewicht, 12, 8, glatt=False)
        for n in range(3):
            w = math.tau * n / 3
            f.kugel("Blatt", oben + Vector((math.cos(w) * 0.06, math.sin(w) * 0.06, -0.06)), (0.025, 0.012, 0.035), farbe("#5E9E45"), gewicht, 8, 4, glatt=False)
    elif art == "stab_glut":
        holz = farbe("#2E2320")
        rot = farbe("#B8321E")

        def glut_farbe(i, k, p):
            if any(abs(p.center.z - b) < 0.015 for b in (0.5, 1.2, 1.6)):
                return rot
            return _griff(holz, leder)(i, k, p)
        _schaft(f, "Glutschaft", x, y, 0.03, 1.88, 0.022, glut_farbe, gewicht)
        for n in range(3):
            w = math.tau * n / 3
            aussen = Vector((math.cos(w), math.sin(w), 0))
            punkte = [oben - Z * 0.03, oben + aussen * 0.06 + Z * 0.05, oben + aussen * 0.05 + Z * 0.16, oben + aussen * 0.015 + Z * 0.22]
            f.straehne("Eisenkralle", punkte, 0.011, 0.003, farbe("#3A3634"), gewicht, 6, 0.2)
        f.loft("Glutstein", [(oben + Z * 0.04, X, Y, 0.005, 0.005), (oben + Z * 0.1, X, Y, 0.05, 0.05), (oben + Z * 0.16, X, Y, 0.04, 0.04),
                             (oben + Z * 0.24, X, Y, 0.003, 0.003)], 6, lambda i, k, p: farbe("#FF6A1E") * (1.3 if k % 2 else 1.0), gewicht,
               oben_zu=True, unten_zu=True)
        for n in range(4):
            w = math.tau * n / 4 + 0.4
            f.straehne("Flamme", [oben + Z * 0.14 + Vector((math.cos(w), math.sin(w), 0)) * 0.03, oben + Z * 0.28 + Vector((math.cos(w), math.sin(w), 0)) * 0.02],
                       0.012, 0.002, farbe("#FFB040"), gewicht, 5, 0.0)
    elif art == "stab_frost":
        holz = farbe("#C9D6DE")
        _schaft(f, "Frostschaft", x, y, 0.03, 1.9, 0.021, _griff(holz, farbe("#5A7890")), gewicht)
        for n, (neig, laenge) in enumerate(((0, 0.3), (28, 0.2), (-26, 0.22), (40, 0.14), (-38, 0.16), (15, 0.12))):
            w = math.tau * n / 6
            richtung = (Z * math.cos(math.radians(neig)) + Vector((math.cos(w), math.sin(w), 0)) * math.sin(math.radians(abs(neig)))).normalized()
            fuss = oben + Z * 0.02
            f.loft("Eiskristall", [(fuss, X, Y, 0.018, 0.018), (fuss + richtung * laenge * 0.8, X, Y, 0.022, 0.022), (fuss + richtung * laenge, X, Y, 0.002, 0.002)],
                   6, lambda i, k, p: farbe("#A8E4FF") * (1.25 if k % 2 else 1.0), gewicht, oben_zu=True, unten_zu=True)
    elif art == "stab_sturm":
        silber = farbe("#B8C0CA")

        def spirale(i, k, p):
            if (k + int(p.center.z * 30)) % 12 < 2:
                return farbe("#4A6AC8")
            return _griff(silber, leder)(i, k, p)
        _schaft(f, "Sturmschaft", x, y, 0.03, 1.86, 0.02, spirale, gewicht)
        ring = []
        for n in range(24):
            w = math.tau * n / 24
            ring.append((oben + Z * 0.12 + Vector((math.cos(w) * 0.1, 0, math.sin(w) * 0.1)), Y, Vector((math.cos(w), 0, math.sin(w))), 0.012, 0.012))
        ring.append(ring[0])
        f.loft("Silberring", ring, 8, lambda i, k, p: silber, gewicht)
        f.kugel("Sturmkugel", oben + Z * 0.12, (0.045, 0.045, 0.045), farbe("#8A7CFF"), gewicht, 12, 8, glatt=False)
        for n in range(3):
            w = math.tau * n / 3
            a = oben + Z * 0.12 + Vector((math.cos(w), 0.2, math.sin(w))) * 0.05
            punkte = [a, a + Vector((math.cos(w) * 0.03, -0.02, math.sin(w) * 0.03 + 0.02)), a + Vector((math.cos(w) * 0.07, 0.01, math.sin(w) * 0.07))]
            f.straehne("Blitz", punkte, 0.006, 0.002, farbe("#FFE66A"), gewicht, 4, 0.0)
    else:  # stab_sternen
        weiss = farbe("#EDE8DA")

        def baender(i, k, p):
            if any(abs(p.center.z - b) < 0.014 for b in (0.3, 0.7, 1.1, 1.45, 1.75)):
                return gold
            return _griff(weiss, farbe("#3A2A5A"))(i, k, p)
        _schaft(f, "Sternenschaft", x, y, 0.03, 1.9, 0.021, baender, gewicht)
        # Goldene Mondsichel mit einem Stern darin
        mond = []
        for n in range(12):
            w = math.radians(-30 + 240 * n / 11)
            mond.append((oben + Z * 0.13 + Vector((math.cos(w) * 0.11, 0, math.sin(w) * 0.11)), Y, Vector((math.cos(w), 0, math.sin(w))), 0.012, 0.012 + 0.014 * math.sin(math.pi * n / 11)))
        f.loft("Mondsichel", mond, 8, lambda i, k, p: gold, gewicht, oben_zu=True, unten_zu=True)
        f.kugel("Sternkern", oben + Z * 0.13, (0.04, 0.04, 0.04), farbe("#FFF3C0"), gewicht, 12, 8, glatt=False)
        for seite in (Y, -Y):
            f.stern("Stern", oben + Z * 0.13 + seite * 0.042, seite, 0.07, gold_dunkel * 1.3, gewicht, zacken=5)


def _hammerkopf(f, name, mitte, halb_x, halb_y, halb_z, farbe_von, gewicht, rund=False):
    """Hammerkopf längs der Y-Achse (Schlagflächen vorne und hinten)."""
    profil = [(-1.0, 0.9), (-0.9, 1.0), (-0.3, 0.92), (0.0, 0.96), (0.3, 0.92), (0.9, 1.0), (1.0, 0.9)]
    form = None if rund else (lambda w: 1.0 / max(abs(math.cos(w)), abs(math.sin(w))) ** 0.9)
    ringe = []
    for t, s in profil:
        ring = (mitte + Y * halb_y * t, X, Z, halb_x * s, halb_z * s)
        ringe.append(ring + ((form,) if form else ()))
    f.loft(name, ringe, 16, farbe_von, gewicht, oben_zu=True, unten_zu=True, teilung=2)


def hammer(f, art, x, y, gewicht, griff_z=0.6):
    """Einer der fünf Hämmer (`art` aus HAEMMER), hängt aus der Faust bei (x, y, griff_z)."""
    gold, gold_dunkel = farbe("#D8AE4A"), farbe("#A9812E")
    leder = farbe("#43291A")
    stahl, stahl_hell, stahl_dunkel = farbe("#8E97A2"), farbe("#C9D1DA"), farbe("#4E555E")
    oben, unten = griff_z + 0.18, griff_z - 0.46
    kopf = Vector((x, y, griff_z - 0.4))
    holz = farbe({"hammer_eisen": "#7A5230", "hammer_runen": "#5E4A3A", "hammer_streit": "#6A4526", "hammer_donner": "#2E2A2E", "hammer_drachen": "#3A1E1A"}[art])

    def stiel(i, k, p):
        z = p.center.z
        if griff_z - 0.06 < z < griff_z + 0.1:
            return leder * (0.75 if int(z * 90) % 2 else 1.0)
        if art != "hammer_eisen" and any(abs(z - b) < 0.012 for b in (griff_z - 0.2, griff_z + 0.13)):
            return gold
        return holz * (0.85 + 0.15 * ((k * 3 + int(i * 5)) % 4) / 3)
    _schaft(f, "Hammerstiel", x, y, unten, oben, 0.021, stiel, gewicht, wackeln=0.0, ringe=12)
    f.kugel("Knauf", (x, y, oben + 0.01), (0.03, 0.03, 0.03), gold if art != "hammer_eisen" else stahl_dunkel, gewicht, 10, 6, glatt=False)

    if art == "hammer_eisen":
        _hammerkopf(f, "Eisenkopf", kopf, 0.06, 0.12, 0.065, lambda i, k, p: (stahl_hell if i < 0.5 or i > 5.5 else stahl_dunkel * 1.3) * (0.95 + 0.05 * (k % 2)), gewicht)
    elif art == "hammer_runen":
        stein = farbe("#7E8494")
        _hammerkopf(f, "Runenkopf", kopf, 0.075, 0.14, 0.08, lambda i, k, p: gold if 2.6 < i < 3.4 else stein * (0.92 + 0.08 * (k % 2)), gewicht)
        for seite in (X, -X):
            for dy in (-0.07, 0.07):
                f.stern("Rune", kopf + seite * 0.077 + Y * dy, seite, 0.028, farbe("#6FC8FF"), gewicht, zacken=4)
    elif art == "hammer_streit":
        _hammerkopf(f, "Streitkopf", kopf + Y * 0.03, 0.06, 0.09, 0.07, lambda i, k, p: stahl_hell if i < 0.5 else stahl * (0.92 + 0.08 * (k % 2)), gewicht)
        # Dorn nach hinten und Spitze nach unten
        f.loft("Dorn", [(kopf - Y * 0.05, X, Z, 0.045, 0.05), (kopf - Y * 0.2, X, Z, 0.004, 0.004)], 8, lambda i, k, p: stahl_hell, gewicht, unten_zu=True, oben_zu=True)
        f.loft("Spitze", [(kopf - Z * 0.06, X, Y, 0.03, 0.03), (kopf - Z * 0.16, X, Y, 0.003, 0.003)], 8, lambda i, k, p: stahl_hell, gewicht, unten_zu=True, oben_zu=True)
        f.kugel("Niete", kopf + X * 0.062 + Y * 0.03, (0.012, 0.012, 0.012), gold, gewicht, 8, 4)
    elif art == "hammer_donner":
        dunkel = farbe("#3A3D46")

        def donner(i, k, p):
            if i < 0.5 or i > 5.5:
                return stahl_hell
            if (k + int(i * 2)) % 8 in (0, 1) and 1.2 < i < 4.8:
                return farbe("#FFD24A")                                       # Blitz-Einlagen
            return dunkel * (0.95 + 0.08 * (k % 2))
        _hammerkopf(f, "Donnerkopf", kopf, 0.095, 0.15, 0.095, donner, gewicht, rund=True)
        for dy in (-0.15, 0.15):
            f.loft("Goldring", [(kopf + Y * dy * 0.92, X, Z, 0.1, 0.1), (kopf + Y * dy * 1.02, X, Z, 0.1, 0.1)], 16, lambda i, k, p: gold, gewicht, oben_zu=True, unten_zu=True)
    else:  # hammer_drachen
        schuppe, schuppe_dunkel = farbe("#A8261E"), farbe("#5A1410")

        def drache(i, k, p):
            if i < 0.5 or i > 5.5:
                return farbe("#2A2226")
            return (schuppe if (k + int(i * 3)) % 3 else schuppe_dunkel) * (0.92 + 0.1 * (k % 2))
        _hammerkopf(f, "Drachenkopf", kopf, 0.085, 0.15, 0.085, drache, gewicht)
        for s in (1, -1):
            punkte = [kopf + X * 0.07 * s + Z * 0.05, kopf + X * 0.12 * s + Z * 0.1, kopf + X * 0.13 * s + Z * 0.17]
            f.straehne("Horn", punkte, 0.022, 0.003, farbe("#E8DCC0"), gewicht, 8, 0.0)
        f.kugel("Drachenauge", kopf - X * 0.088, (0.012, 0.03, 0.03), farbe("#FFB020"), gewicht, 10, 6, glatt=False)
        f.kugel("Drachenauge", kopf + X * 0.088, (0.012, 0.03, 0.03), farbe("#FFB020"), gewicht, 10, 6, glatt=False)
        f.loft("Goldband", [(kopf - Z * 0.087, X, Y, 0.03, 0.03), (kopf + Z * 0.087, X, Y, 0.03, 0.03)], 10, lambda i, k, p: gold_dunkel * 1.2, gewicht,
               oben_zu=True, unten_zu=True)


# ---------------------------------------------------------------------------
# Bögen des Bogenschützen (bogenschuetze.py): liegen in der linken Faust, der Griff bei `griff`.
# In Ruhehaltung (Arm hängt) liegt der Bogen waagerecht entlang Y; beugt die Figur den Unterarm
# nach vorne, steht er senkrecht. Der Bauch des Bogens zeigt nach -Z (später zum Ziel), die Sehne
# spannt sich auf der +Z-Seite (zur Figur hin).
# ---------------------------------------------------------------------------
BOEGEN = ["bogen_eibe", "bogen_lang", "bogen_glut", "bogen_elfen", "bogen_sturm"]

# Länge, Standhöhe (Abstand Griff–Sehne), Recurve (wie stark sich die Enden zurückbiegen),
# Breite der Wurfarme, Holzfarbe, zweite Farbe
_BOGEN_ART = {
    "Bogen": (1.36, 0.15, 0.0, 0.03, "#7A5230", "#5E3C20"),
    "bogen_eibe": (1.46, 0.15, 0.0, 0.032, "#C98A4A", "#8A4E28"),
    "bogen_lang": (1.72, 0.16, 0.0, 0.034, "#5A3A22", "#C8B48A"),
    "bogen_glut": (1.4, 0.15, 0.05, 0.034, "#2A2224", "#FF7A2A"),
    "bogen_elfen": (1.48, 0.15, 0.06, 0.03, "#E6DFCB", "#5EA85A"),
    "bogen_sturm": (1.52, 0.16, 0.07, 0.036, "#3C4A62", "#FFD24A"),
}


def _bogen_linie(y, laenge, stand, recurve):
    """Mittellinie des Bogens: z-Versatz an der Stelle y (Griff bei y = 0)."""
    s = min(1.0, abs(y) / (laenge / 2))
    z = stand * s ** 1.7
    if recurve > 0.0 and s > 0.8:
        z -= recurve * ((s - 0.8) / 0.2) ** 2
    return z


def bogen(f, art, griff, gewicht):
    """Einer der Bögen (`art` aus BOEGEN oder der Jagdbogen „Bogen“), gegriffen bei `griff`."""
    laenge, stand, recurve, breite, holz_hex, zweit_hex = _BOGEN_ART[art]
    holz, zweit = farbe(holz_hex), farbe(zweit_hex)
    leder = farbe("#43291A")
    gold, horn = farbe("#D8AE4A"), farbe("#EDE3CB")
    sehne = farbe("#E8E0CC") if art != "bogen_sturm" else farbe("#BFE6FF")
    n = 26
    ringe = []
    for i in range(n):
        y = (i / (n - 1) - 0.5) * laenge
        z = _bogen_linie(y, laenge, stand, recurve)
        dz = (_bogen_linie(y + 0.01, laenge, stand, recurve) - _bogen_linie(y - 0.01, laenge, stand, recurve)) / 0.02
        tangente = Vector((0, 1, dz)).normalized()
        quer = tangente.cross(X).normalized()
        s = abs(y) / (laenge / 2)
        griffstueck = abs(y) < 0.07
        b = breite * (1.25 if griffstueck else (1.0 - 0.6 * s))
        d = breite * (0.85 if griffstueck else (0.55 - 0.3 * s))
        ringe.append((griff + Vector((0, y, z)), X, quer, b, d))

    def bogen_farbe(i, k, p):
        y = p.center.y - griff.y
        s = abs(y) / (laenge / 2)
        if abs(y) < 0.07:
            return leder * (0.75 if int(y * 90) % 2 else 1.0)                  # Ledergriff
        if s > 0.93:
            return gold if art in ("bogen_elfen", "bogen_sturm") else horn       # Spitzen
        if art == "bogen_eibe":
            return holz if k in (0, 1, 2, 7, 8, 9) else zweit                    # helles Splintholz, dunkles Kernholz
        if art == "bogen_lang":
            return zweit if int(s * 12) % 4 == 0 else holz                        # Sehnenwicklungen
        if art == "bogen_glut":
            return zweit if (k + int(s * 20)) % 7 == 0 else holz * (0.9 + 0.2 * (k % 2))   # glimmende Risse
        if art == "bogen_elfen":
            return holz * (0.95 + 0.05 * (k % 2))
        if art == "bogen_sturm":
            return zweit if abs(((s * 10) % 1.0) - 0.5) < 0.08 else holz          # Blitzbänder
        return holz * (0.88 + 0.12 * ((k + int(s * 8)) % 3) / 2)

    f.loft("Bogen" + art, ringe, 10, bogen_farbe, gewicht, oben_zu=True, unten_zu=True, teilung=2)
    # Sehne von Spitze zu Spitze
    oben = griff + Vector((0, laenge / 2, _bogen_linie(laenge / 2, laenge, stand, recurve)))
    unten = griff + Vector((0, -laenge / 2, _bogen_linie(-laenge / 2, laenge, stand, recurve)))
    f.loft("Sehne", [(unten, X, Z, 0.0028, 0.0028), (oben, X, Z, 0.0028, 0.0028)], 6, lambda i, k, p: sehne, gewicht, oben_zu=True, unten_zu=True)
    for spitze in (oben, unten):
        f.kugel("Nocke", spitze, (0.012, 0.014, 0.012), gold if art in ("bogen_elfen", "bogen_sturm") else horn, gewicht, 8, 4, glatt=False)
    # Pfeilauflage und Verzierungen
    f.kiste("Auflage", griff + Vector((breite * 1.1, 0.075, -0.004)), (0.01, 0.02, 0.02), leder * 1.2, gewicht)
    if art == "bogen_glut":
        f.kugel("Glutstein", griff + Vector((0, 0.0, -0.035)), (0.022, 0.03, 0.02), farbe("#FF8A2A"), gewicht, 10, 6, glatt=False)
    elif art == "bogen_elfen":
        for i in range(10):
            y = (i / 9 - 0.5) * laenge * 0.8
            if abs(y) < 0.1:
                continue
            z = _bogen_linie(y, laenge, stand, recurve)
            seite = 1 if i % 2 else -1
            f.kugel("Blatt", griff + Vector((seite * breite * 0.9, y, z - 0.004)), (0.018, 0.03, 0.006), zweit, gewicht, 8, 4, glatt=False)
        f.kugel("Mondstein", griff + Vector((0, 0.0, -0.034)), (0.018, 0.025, 0.016), farbe("#CFF4FF"), gewicht, 10, 6, glatt=False)
    elif art == "bogen_sturm":
        f.kugel("Sturmstein", griff + Vector((0, 0.0, -0.038)), (0.024, 0.032, 0.02), farbe("#6FD8FF"), gewicht, 10, 6, glatt=False)
        for spitze in (oben, unten):
            f.stern("Blitz", spitze + Vector((breite * 0.5, 0, 0)), X, 0.03, zweit, gewicht, zacken=4)
            f.stern("Blitz", spitze - Vector((breite * 0.5, 0, 0)), -X, 0.03, zweit, gewicht, zacken=4)
    elif art == "bogen_lang":
        for y in (-0.5, 0.5):
            p = griff + Vector((0, y * laenge * 0.5, _bogen_linie(y * laenge * 0.5, laenge, stand, recurve)))
            f.loft("Wicklung", [(p - Y * 0.02, X, Z, breite * 0.9, breite * 0.6), (p + Y * 0.02, X, Z, breite * 0.9, breite * 0.6)], 10,
                   lambda i, k, p: zweit, gewicht, oben_zu=True, unten_zu=True)
