"""Die Schattenfestung: eine düstere, verzauberte Festung auf einem schroffen Basaltfelsen.

Sechseckige Ringmauer mit Dornenzinnen und sechs Türmen unter steilen schwarzen Schieferhelmen,
Torhaus mit Runen-Fallgitter und Totenschädel, Bergfried mit Dornenkrone, darüber eine schwebende
magische Kugel mit Ringen, Kristallsplittern und einem Lichtstrahl in den Himmel. Im Hof eine
düstere Halle, ein leuchtender Runenkreis mit Altar und grüne Geisterfeuer; um die Festung
schwebende Felsinseln mit Kristallen und Ketten, zerrissene Banner, tote Bäume; im Fels
violett glühende Risse. Achteckige Ringmauer mit vier Toren (Nord, Ost, Süd, West), vor jedem eine
Rampe auf Bögen hinab zum Boden – aus diesen Toren marschieren die Truppen der Festung.

Baut auf dem Baukasten von burg.py auf (Mauerwerk und Schiefer zeichnet der Shader).
Z oben, das Haupttor zeigt nach -Y (Süden), Boden bei z = 0.
"""

import math
import random

from mathutils import Vector

from burg import (MAUER, PLATTEN, ZIEGEL, Bau, M, bogen, bogen_hoehe, drehkoerper, efeu, gesims, kreis, platte, prisma, pyramide,
                  quader, rahmen, wasserspeier, zylinder)

BASALT = "#34303B"
BASALT_DUNKEL = "#221F28"
STEIN = "#3E3946"
STEIN_HELL = "#5A5264"
DACH = "#24212E"
DACH_DUNKEL = "#16141C"
EISEN = "#1F1E23"
MAGIE = "#8E3DFF"
MAGIE_HELL = "#C08CFF"
RANKEN = ("#2A2426", "#342A2C", "#1F1B1D", "#3B2F2A")
GEISTERFEUER = "#7CFF9A"
KRISTALL = "#6FE3FF"
BANNER = "#3A1E4A"
KNOCHEN = "#CFC8B8"

SOCKEL_H = 11.0         # Höhe des Felsplateaus
MAUER_R = 28.0          # Umkreis der achteckigen Ringmauer (Ecken)
TORE = (0, 90, 180, 270)  # Drehung der Tore um die Hochachse: Süd (-Y), Ost (+X), Nord (+Y), West (-X)
MAUER_H = 10.0
MAUER_D = 2.2


# ---------------------------------------------------------------------------
# Fels, Kristalle, Magie
# ---------------------------------------------------------------------------
def fels(bau, m, r, h, zufall, farbe=BASALT, kopfueber=False):
    """Zerklüfteter Felsbrocken: unregelmäßige Kanten, schräge, gebrochene Oberseite."""
    import bmesh
    from burg import aus_bm
    n = zufall.randint(6, 8)
    bm = bmesh.new()
    ringe = []
    for stufe, (hoehe, breite) in enumerate(((0.0, 1.0), (0.45, 1.08), (0.85, 0.72))):
        ring = []
        for k in range(n):
            w = math.tau * (k + zufall.uniform(-0.2, 0.2)) / n
            rr = r * breite * zufall.uniform(0.8, 1.15)
            z = h * hoehe * zufall.uniform(0.85, 1.12) if stufe else 0.0
            ring.append(bm.verts.new((math.cos(w) * rr, math.sin(w) * rr, z)))
        ringe.append(ring)
    spitze = bm.verts.new((zufall.uniform(-0.2, 0.2) * r, zufall.uniform(-0.2, 0.2) * r, h * zufall.uniform(0.95, 1.15)))
    for a, b in zip(ringe, ringe[1:]):
        for k in range(n):
            bm.faces.new((a[k], a[(k + 1) % n], b[(k + 1) % n], b[k]))
    for k in range(n):
        bm.faces.new((ringe[-1][k], ringe[-1][(k + 1) % n], spitze))
    bm.faces.new(list(reversed(ringe[0])))
    if kopfueber:
        for v in bm.verts:
            v.co.z = -v.co.z
    bau.teil(aus_bm(bm), farbe, m=m, schwankung=0.08)


def kristall(bau, m, laenge, r, farbe=KRISTALL):
    """Sechseckiger Kristall mit Spitze (leuchtet), Fuß im Ursprung, zeigt nach +Z."""
    bau.teil(zylinder(r, laenge * 0.7, 6, r * 0.9), farbe, m=m, leuchten=True)
    bau.teil(pyramide(r * 0.9, laenge * 0.3, 6, 0.0), farbe, m=m @ M((0, 0, laenge * 0.7)), leuchten=True)


def kristallbuendel(bau, m, zufall, groesse=1.0, farbe=KRISTALL):
    for i in range(zufall.randint(4, 7)):
        laenge = groesse * zufall.uniform(0.8, 2.2)
        kristall(bau, m @ M((zufall.uniform(-0.4, 0.4) * groesse, zufall.uniform(-0.4, 0.4) * groesse, 0),
                            zufall.uniform(0, 360), zufall.uniform(-30, 30), zufall.uniform(-30, 30)), laenge, groesse * zufall.uniform(0.12, 0.2), farbe)


def riss(bau, m, laenge, zufall):
    """Violett glühender Riss im Fels: Zickzack aus schmalen Leuchtstreifen (Wand bei y = 0, außen -Y)."""
    x, z = 0.0, 0.0
    for _ in range(int(laenge / 0.7)):
        dx, dz = zufall.uniform(-0.35, 0.35), zufall.uniform(0.45, 0.8)
        stueck = math.hypot(dx, dz)
        w = math.degrees(math.atan2(dx, dz))
        bau.teil(quader(0.09, 0.08, stueck + 0.05), MAGIE, m=m @ M((x, -0.02, z), 0, 0, w), leuchten=True)
        x, z = x + dx, z + dz


def dorn(bau, m, h, r=0.18, farbe=EISEN):
    """Eiserner Dorn (Spitze nach oben)."""
    bau.teil(pyramide(r, h, 4), farbe, m=m)


def kaefig(bau, m, h=2.2, r=0.8):
    """Hängender Eisenkäfig (Aufhängung oben bei z = h): Gitterstäbe, Ringe, Kuppel, Totenkopf drin."""
    bau.teil(zylinder(r, 0.12, 8), EISEN, m=m)
    bau.teil(zylinder(r, 0.1, 8), EISEN, m=m @ M((0, 0, h * 0.5)))
    bau.teil(drehkoerper([(r, 0.0), (r * 0.7, 0.5), (0.1, 0.8), (0.0, 0.85)], 8), EISEN, m=m @ M((0, 0, h - 0.1)))
    for k in range(10):
        w = math.tau * k / 10
        bau.teil(quader(0.05, 0.05, h), EISEN, m=m @ M((math.cos(w) * r, math.sin(w) * r, 0)))
    schaedel(bau, m @ M((0.1, 0.1, 0.1), 30), 0.35)


def geisterfeuer(bau, m, zufall):
    """Feuerschale aus Eisen mit grünen Geisterflammen."""
    bau.teil(drehkoerper([(0.35, 0.0), (0.15, 0.1), (0.12, 0.9), (0.25, 1.0), (0.6, 1.15), (0.65, 1.35)], 8), EISEN, m=m)
    for k in range(4):
        dorn(bau, m @ M((0, 0, 1.25), k * 90 + 45) @ M((0.62, 0, 0)), 0.35, 0.05)
    for k in range(6):
        w = 60 * k
        bau.teil(pyramide(0.2, zufall.uniform(0.6, 1.0), 5), GEISTERFEUER, m=m @ M((0, 0, 1.25), w) @ M((0.2, 0, 0)), leuchten=True)
    bau.teil(pyramide(0.28, 1.3, 5), "#C8FFD4", m=m @ M((0, 0, 1.25)), leuchten=True)


def fenster_d(bau, m, breite, hoehe, glimmen=True):
    """Spitzbogenfenster mit dunklem Gewände, violett glimmend, mit Eisengitter (Wand bei y = 0)."""
    hw = breite / 2
    r = breite * 1.4
    hs = hoehe - bogen_hoehe(hw, r)
    bau.teil(rahmen(bogen(hw + 0.18, hs, r + 0.18), bogen(hw, hs, r), 0.3), STEIN_HELL, m=m)
    bau.teil(platte(bogen(hw, hs, r), 0.04, -0.02), MAGIE if glimmen else "#15141A", m=m, leuchten=glimmen)
    bau.teil(quader(0.05, 0.08, hoehe - 0.1), EISEN, m=m @ M((0, -0.08, 0)))
    for z in (hs * 0.4, hs * 0.8):
        bau.teil(quader(breite, 0.08, 0.05), EISEN, m=m @ M((0, -0.08, z)))


def schaedel(bau, m, g=1.0):
    """Stilisierter Totenschädel mit glühenden Augenhöhlen (schaut nach -Y)."""
    bau.teil(drehkoerper([(0.0, 0.0), (0.42 * g, 0.12 * g), (0.5 * g, 0.45 * g), (0.42 * g, 0.78 * g), (0.0, 0.92 * g)], 10), KNOCHEN,
             m=m @ M((0, 0, 0.35 * g)), schwankung=0.05)
    bau.teil(quader(0.55 * g, 0.4 * g, 0.4 * g, 0.05 * g), KNOCHEN, m=m @ M((0, -0.12 * g, 0.0)))
    for s in (-1, 1):
        bau.teil(drehkoerper([(0.0, -0.1 * g), (0.12 * g, -0.05 * g), (0.12 * g, 0.05 * g), (0.0, 0.1 * g)], 8), MAGIE,
                 m=m @ M((s * 0.18 * g, -0.4 * g, 0.72 * g), 0, 90), leuchten=True)
    bau.teil(pyramide(0.07 * g, 0.12 * g, 3), "#1A1820", m=m @ M((0, -0.47 * g, 0.5 * g), 0, 90))
    for i in range(6):
        bau.teil(quader(0.06 * g, 0.05 * g, 0.12 * g), KNOCHEN, m=m @ M(((-0.15 + i * 0.06) * g, -0.33 * g, 0.28 * g)))


def banner_d(bau, m, breite=1.4, hoehe=4.5, zufall=None):
    """Zerrissenes dunkles Banner mit glühendem Zeichen, an einer Eisenstange (hängt nach unten)."""
    zufall = zufall or random.Random(3)
    bau.teil(zylinder(0.05, breite + 0.4, 6), EISEN, m=m @ M((-(breite + 0.4) / 2, -0.35, 0), 0, 0, 90))
    for s in (-1, 1):
        dorn(bau, m @ M((s * (breite / 2 + 0.2), -0.35, 0), 0, 0, 90 * s), 0.3, 0.06)
    zacken = [(-breite / 2, 0)]
    for i in range(7):
        x = -breite / 2 + breite * i / 6
        zacken.append((x, -hoehe * zufall.uniform(0.75, 1.0) if i % 2 else -hoehe * zufall.uniform(0.55, 0.8)))
    zacken.append((breite / 2, 0))
    bau.teil(platte(list(reversed(zacken)), 0.05, -0.3), BANNER, m=m, schwankung=0.05)
    wz = -hoehe * 0.35
    for y in (-0.36, -0.29):
        bau.teil(rahmen(kreis(0.38, 12, 0, wz), kreis(0.3, 12, 0, wz), 0.015, y + 0.0075), MAGIE, m=m, leuchten=True)
        bau.teil(platte([(0, wz + 0.25), (-0.15, wz - 0.15), (0.15, wz - 0.15)], 0.015, y + 0.0075), MAGIE, m=m, leuchten=True)


def kette(bau, a, b, durchhang, glieder_laenge=0.35):
    """Schwere Eisenkette von a nach b, hängt in der Mitte durch."""
    a, b = Vector(a), Vector(b)
    teile = max(int((b - a).length / glieder_laenge), 2)
    punkte = [a.lerp(b, i / teile) - Vector((0, 0, durchhang * 4 * (i / teile) * (1 - i / teile))) for i in range(teile + 1)]
    for i, (p, q) in enumerate(zip(punkte, punkte[1:])):
        d = q - p
        rz = math.degrees(math.atan2(d.y, d.x))
        ry = -math.degrees(math.atan2(d.z, math.hypot(d.x, d.y)))
        mitte = (p + q) / 2
        glied = M(tuple(mitte), rz, 90 * (i % 2), ry)
        bau.teil(rahmen(kreis(0.13, 6, 0, 0), kreis(0.07, 6, 0, 0), 0.05, 0.025), EISEN, m=glied @ M((0, 0, 0), 0, 0, 90) @ M((0, 0, 0), 0, 90))


def toter_baum(bau, m, zufall, h=5.0):
    """Knorriger, toter Baum: dunkler Stamm, kahle, gekrümmte Äste."""
    def ast(m_, laenge, r, tiefe):
        bau.teil(zylinder(r, laenge, 6, r * 0.55), "#241F1E", m=m_, schwankung=0.06)
        if tiefe >= 3:
            return
        for k in range(2 if tiefe else 3):
            w = zufall.uniform(0, 360)
            neig = zufall.uniform(25, 55)
            ast(m_ @ M((0, 0, laenge * zufall.uniform(0.55, 0.95)), w, neig), laenge * zufall.uniform(0.5, 0.7), r * 0.55, tiefe + 1)
    ast(m @ M((0, 0, 0), 0, zufall.uniform(-6, 6)), h * 0.55, 0.28, 0)
    for k in range(4):
        w = 90 * k + zufall.uniform(-20, 20)
        bau.teil(zylinder(0.14, 1.3, 5, 0.03), "#241F1E", m=m @ M((0, 0, 0.3), w, 0, 75))


# ---------------------------------------------------------------------------
# Türme und Mauern
# ---------------------------------------------------------------------------
def dornzinnen(bau, m, laenge, h=1.8, d=0.7):
    """Brüstung entlang +X ab 0 mit spitzen, dornartigen Zinnen."""
    bau.teil(quader(laenge, d, h * 0.5), STEIN, MAUER, m=m @ M((laenge / 2, 0, 0)))
    anzahl = max(int(laenge / 1.4), 2)
    for i in range(anzahl):
        x = (i + 0.5) * laenge / anzahl
        bau.teil(quader(laenge / anzahl * 0.5, d, h * 0.35), STEIN, MAUER, m=m @ M((x, 0, h * 0.5)))
        bau.teil(pyramide(laenge / anzahl * 0.3, h * 0.55, 4, 0), STEIN_HELL, m=m @ M((x, 0, h * 0.85)))


def turmhelm_d(bau, m, r, hoehe):
    """Steiler schwarzer Schieferhelm mit ausgestellter Traufe, Dornenkranz und violetter Spitze."""
    profil = [(r + 1.0, 0.0), (r + 0.4, 0.5), (r * 0.8, 1.6), (r * 0.55, hoehe * 0.35), (r * 0.3, hoehe * 0.7), (0.12, hoehe * 0.97), (0.0, hoehe)]
    bau.teil(drehkoerper(profil, 12), DACH, ZIEGEL, m=m)
    bau.teil(zylinder(r + 1.05, 0.25, 12), DACH_DUNKEL, m=m @ M((0, 0, -0.1)))
    for k in range(12):
        w = 30 * k
        dorn(bau, m @ M((0, 0, 0.1), w) @ M((r + 1.0, 0, 0), 0, 0, 55), 0.9, 0.12)
    for k in range(4):
        # kleine Dachgauben mit glimmendem Fenster
        g = m @ M((0, 0, hoehe * 0.25), 90 * k + 45) @ M((0, -(r * 0.62), 0))
        bau.teil(quader(1.0, 1.2, 1.4), STEIN, MAUER, m=g @ M((0, 0.5, 0)))
        bau.teil(pyramide(0.9, 1.1, 4), DACH, ZIEGEL, m=g @ M((0, 0.5, 1.4)))
        bau.teil(platte(bogen(0.25, 0.5, 0.35), 0.04, -0.1), MAGIE, m=g @ M((0, 0, 0.3)), leuchten=True)
    bau.teil(drehkoerper([(0.0, 0.0), (0.3, 0.1), (0.35, 0.4), (0.0, 0.7)], 8), EISEN, m=m @ M((0, 0, hoehe - 0.2)))
    bau.teil(drehkoerper([(0.0, -0.3), (0.3, 0.0), (0.0, 0.3)], 8), MAGIE, m=m @ M((0, 0, hoehe + 0.8)), leuchten=True)
    dorn(bau, m @ M((0, 0, hoehe + 1.05)), 1.6, 0.06)


def turm_d(bau, x, y, z0, r, h, helm, zufall, speier=True):
    """Runder dunkler Turm: schräger Fuß, Gesimse, glimmende Scharten und Fenster, Maschikulis,
    Dornenzinnen, steiler Helm, Wasserspeier."""
    m = M((x, y, z0))
    ecken = 12
    bau.teil(zylinder(r + 1.2, 3.0, ecken, r), BASALT, MAUER, m=m)
    bau.teil(zylinder(r, h, ecken), STEIN, MAUER, m=m)
    for z in (3.0, h * 0.5):
        bau.teil(zylinder(r + 0.25, 0.3, ecken), STEIN_HELL, m=m @ M((0, 0, z)))
    seite = r * math.cos(math.pi / ecken)
    for k in range(ecken):
        w = 360 * k / ecken
        f = m @ M((0, 0, 0), w) @ M((0, -seite, 0))
        if k % 3 == 0:
            fenster_d(bau, f @ M((0, 0, h * 0.62)), 0.9, 2.2)
        elif k % 3 == 1:
            bau.teil(platte([(-0.08, 0), (0.08, 0), (0.08, 1.3), (-0.08, 1.3)], 0.03), MAGIE, m=f @ M((0, 0, h * 0.3)), leuchten=True)
        bau.teil(prisma([(0.02, 0), (-0.55, 0), (-0.55, -0.2), (-0.1, -0.8), (0.02, -0.8)], 0.32), STEIN_HELL,
                 m=m @ M((0, 0, 0), w + 180 / ecken) @ M((-0.16, -seite, h - 0.1)))
    for k in range(ecken):
        if k % 2 == 0:
            f = m @ M((0, 0, 0), 360 * k / ecken) @ M((0, -seite - 0.04, 0))
            for j in range(3):
                bau.teil(platte([(-0.14, 0), (0.14, 0.12), (0.0, 0.38), (-0.1, 0.2)], 0.03), MAGIE, m=f @ M((0, 0, 3.6 + j * 0.55)), leuchten=True)
    efeu(bau, m @ M((0, 0, 0), zufall.uniform(0, 360)) @ M((0, -seite, 0)), 2.0, h * 0.7, zufall.randint(1, 999), 0.8, RANKEN)
    bau.teil(zylinder(r + 0.6, 0.35, ecken), STEIN_HELL, m=m @ M((0, 0, h - 0.35)))
    ra = r + 0.5
    for k in range(ecken):
        w1, w2 = math.tau * k / ecken, math.tau * (k + 1) / ecken
        p1 = Vector((math.cos(w1) * ra, math.sin(w1) * ra, 0))
        p2 = Vector((math.cos(w2) * ra, math.sin(w2) * ra, 0))
        dornzinnen(bau, m @ M((p1.x, p1.y, h), math.degrees(math.atan2(p2.y - p1.y, p2.x - p1.x))), (p2 - p1).length, 1.8, 0.6)
    turmhelm_d(bau, m @ M((0, 0, h + 0.2)), r - 0.1, helm)
    if speier:
        for k in range(3):
            wasserspeier(bau, m @ M((0, 0, 0), 120 * k + zufall.uniform(0, 40)) @ M((0, -seite - 0.3, h - 1.3)), 1.2)


def mauer_d(bau, p1, p2, zufall, scharten=True):
    """Mauerstück von p1 nach p2 (gegen den Uhrzeigersinn; außen = rechts)."""
    dx, dy = p2[0] - p1[0], p2[1] - p1[1]
    laenge = math.hypot(dx, dy)
    m = M((p1[0], p1[1], SOCKEL_H), math.degrees(math.atan2(dy, dx)))
    d = MAUER_D
    bau.teil(quader(laenge, d, MAUER_H), STEIN, MAUER, m=m @ M((laenge / 2, 0, 0)))
    bau.teil(prisma([(-d / 2 + 0.01, 0), (-d / 2 + 0.01, 3.0), (-d / 2 - 1.2, 0)], laenge), BASALT, MAUER, m=m)
    gesims(bau, m @ M((0, -d / 2, MAUER_H * 0.6)), laenge, 0.2, 0.2, STEIN_HELL)
    bau.teil(quader(laenge, d - 0.3, 0.08), BASALT_DUNKEL, PLATTEN, m=m @ M((laenge / 2, 0, MAUER_H)))
    anzahl = max(int(laenge / 1.1), 1)
    for i in range(anzahl):
        x = (i + 0.5) * laenge / anzahl
        bau.teil(prisma([(0.02, 0), (-0.5, 0), (-0.5, -0.2), (-0.1, -0.7), (0.02, -0.7)], 0.26), STEIN_HELL, m=m @ M((x - 0.13, -d / 2, MAUER_H - 0.1)))
    dornzinnen(bau, m @ M((0, -d / 2 - 0.1, MAUER_H - 0.1)), laenge, 2.0, 0.7)
    bau.teil(quader(laenge, 0.45, 1.0), STEIN, MAUER, m=m @ M((laenge / 2, d / 2 - 0.22, MAUER_H)))
    if scharten:
        for i in range(max(int(laenge / 5.0), 1)):
            x = (i + 0.5) * laenge / max(int(laenge / 5.0), 1)
            bau.teil(platte([(-0.09, 0), (0.09, 0), (0.09, 1.4), (-0.09, 1.4)], 0.03), MAGIE, m=m @ M((x, -d / 2, 4.2)), leuchten=True)
    # Strebepfeiler mit Dornen außen
    for i in range(1, max(int(laenge / 7.0), 1)):
        x = i * laenge / max(int(laenge / 7.0), 1)
        bau.teil(prisma([(0.0, 0.0), (-2.2, 0.0), (0.0, MAUER_H * 0.8)], 1.0), STEIN, MAUER, m=m @ M((x - 0.5, -d / 2 + 0.01, 0)))
        dorn(bau, m @ M((x, -d / 2 - 0.4, MAUER_H * 0.78), 0, -35), 1.2, 0.15)


def torhaus_d(bau, y0, zufall, tuerme=True):
    """Torhaus: Spitzbogen mit Runen-Fallgitter (halb offen), Totenschädel, Feuer; mit `tuerme`
    zwei eigene Flankentürme."""
    hw, tiefe, h = 5.5, 7.0, 14.0
    vorne, hinten = y0 - 3.5, y0 + 3.5
    oeff = 2.3
    r_bogen = oeff * 1.7
    hs = 4.6
    scheitel = hs + bogen_hoehe(oeff, r_bogen)
    for s in (-1, 1):
        bau.teil(quader(hw - oeff, tiefe, h), STEIN, MAUER, m=M((s * (oeff + hw) / 2, y0, SOCKEL_H)))
        if tuerme:
            turm_d(bau, s * (hw + 2.0), y0 - 1.0, SOCKEL_H, 3.8, 20.0, 13.0, zufall)
    feld = bogen(oeff, hs, r_bogen, nur_bogen=True) + [(-oeff, h), (oeff, h)]
    bau.teil(platte(feld, tiefe, 0.0), STEIN, MAUER, m=M((0, hinten, SOCKEL_H)))
    bau.teil(quader(2 * oeff, tiefe, 0.08), BASALT_DUNKEL, PLATTEN, m=M((0, y0, SOCKEL_H)))
    for y, w in ((vorne, 0), (hinten, 180)):
        bau.teil(rahmen(bogen(oeff + 0.7, hs, r_bogen + 0.7), bogen(oeff, hs, r_bogen), 0.5), STEIN_HELL, m=M((0, y, SOCKEL_H), w))
    # Fallgitter mit glühenden Runen, halb hochgezogen
    unten = 3.4
    for i in range(8):
        x = -oeff + 0.25 + i * (2 * oeff - 0.5) / 7
        bau.teil(quader(0.14, 0.14, scheitel - unten + 0.5), EISEN, m=M((x, vorne + 1.2, SOCKEL_H + unten)))
        dorn(bau, M((x, vorne + 1.2, SOCKEL_H + unten), 0, 180), 0.45, 0.1)
    for z in (unten + 0.5, unten + 1.6, unten + 2.7):
        bau.teil(quader(2 * oeff, 0.1, 0.12), EISEN, m=M((0, vorne + 1.2, SOCKEL_H + z)))
    for i in range(5):
        x = -oeff + 0.6 + i * (2 * oeff - 1.2) / 4
        bau.teil(platte([(-0.12, 0), (0.12, 0.2), (0.0, 0.45), (-0.1, 0.22)], 0.03, -0.06), MAGIE, m=M((x, vorne + 1.2, SOCKEL_H + unten + 1.0)), leuchten=True)
    # Wehrgang, Zinnen, Schädel, Feuer
    bau.teil(quader(2 * hw, tiefe - 0.4, 0.1), BASALT_DUNKEL, PLATTEN, m=M((0, y0, SOCKEL_H + h)))
    dornzinnen(bau, M((-hw, vorne - 0.1, SOCKEL_H + h - 0.1)), 2 * hw, 2.0, 0.7)
    dornzinnen(bau, M((hw, hinten + 0.1, SOCKEL_H + h - 0.1), 180), 2 * hw, 2.0, 0.7)
    schaedel(bau, M((0, vorne - 0.4, SOCKEL_H + scheitel + 1.0)), 1.6)
    for s in (-1, 1):
        bau.teil(quader(0.5, 0.8, 0.5), STEIN_HELL, m=M((s * 1.4, vorne - 0.3, SOCKEL_H + scheitel + 1.2)))
        for k in range(3):
            dorn(bau, M((s * 1.4, vorne - 0.3, SOCKEL_H + scheitel + 1.7), 0, k * 25 - 25), 1.4, 0.12)
        geisterfeuer(bau, M((s * (oeff + 1.6), vorne - 1.6, SOCKEL_H)), zufall)
        if tuerme:
            banner_d(bau, M((s * (hw + 2.0), y0 - 1.0 - 4.0, SOCKEL_H + 16.5)), 1.4, 5.0, zufall)
        else:
            banner_d(bau, M((s * 3.9, vorne - 0.15, SOCKEL_H + 13.2)), 1.3, 4.6, zufall)
    return vorne


# ---------------------------------------------------------------------------
# Bergfried mit schwebender Kugel
# ---------------------------------------------------------------------------
def bergfried(bau, x, y, zufall):
    z = SOCKEL_H
    b = 15.0
    h1, h2 = 24.0, 17.0
    m = M((x, y, z))
    bau.teil(quader(b + 2.0, b + 2.0, 3.0, 0.1), BASALT, MAUER, m=m)
    bau.teil(quader(b, b, h1), STEIN, MAUER, m=m)
    for sx, sy in ((1, 1), (-1, 1), (-1, -1), (1, -1)):
        ecke = m @ M((sx * b / 2, sy * b / 2, 0), math.degrees(math.atan2(sy, sx)) + 90)
        bau.teil(quader(2.2, 2.2, h1 + 3.0), STEIN, MAUER, m=ecke)
        for zz in (8.0, 16.0):
            bau.teil(pyramide(1.7, 2.0, 4, 0), DACH, ZIEGEL, m=ecke @ M((0, 0, zz)))
        for k in range(3):
            dorn(bau, ecke @ M((0, 0, h1 + 3.0), k * 120) @ M((0.6, 0, 0), 0, 0, 20), 3.0, 0.25)
    for k in range(4):
        f = m @ M((0, 0, 0), 90 * k) @ M((0, -b / 2, 0))
        for zz, br, hh in ((4.0, 1.4, 3.2), (11.0, 2.2, 5.5), (18.0, 1.4, 3.4)):
            fenster_d(bau, f @ M((0, 0, zz)), br, hh)
        gesims(bau, f @ M((-b / 2 - 0.4, 0, 9.5)), b + 0.8, 0.35, 0.3, STEIN_HELL)
        gesims(bau, f @ M((-b / 2 - 0.6, 0, h1 - 0.4)), b + 1.2, 0.6, 0.45, STEIN_HELL)
        dornzinnen(bau, f @ M((-b / 2, -0.2, h1)), b, 2.2, 0.8)
        banner_d(bau, f @ M((0, 0, 16.5)), 2.0, 6.5, zufall)
        riss(bau, f @ M((b * 0.3, -0.02, 1.0)), 6.0, zufall)
    # Achteckiger Oberbau mit Balkon, Dornenkrone und Spitzhelm
    r = 6.0
    ob = m @ M((0, 0, h1 + 0.1))
    bau.teil(zylinder(r + 1.6, 0.6, 8, drehung=math.pi / 8), STEIN_HELL, m=ob)
    bau.teil(zylinder(r, h2, 8, drehung=math.pi / 8), STEIN, MAUER, m=ob)
    seite = r * math.cos(math.pi / 8)
    for k in range(8):
        f = ob @ M((0, 0, 0), 45 * k) @ M((0, -seite, 0))
        fenster_d(bau, f @ M((0, 0, 3.0)), 1.6, 4.5)
        if k % 2 == 0:
            fenster_d(bau, f @ M((0, 0, 10.0)), 1.1, 3.0)
        dorn(bau, ob @ M((0, 0, 0.6), 45 * k + 22.5) @ M((r + 1.4, 0, 0), 0, 0, 30), 1.4, 0.15)
    krone = ob @ M((0, 0, h2))
    bau.teil(zylinder(r + 0.9, 0.5, 8, drehung=math.pi / 8), STEIN_HELL, m=krone)
    for k in range(16):
        w = 360 * k / 16
        hoehe = 3.5 if k % 2 == 0 else 2.0
        bau.teil(pyramide(0.45, hoehe, 4), STEIN_HELL if k % 2 else DACH_DUNKEL, m=krone @ M((0, 0, 0.5), w) @ M((r + 0.3, 0, 0), 0, 0, 12))
    turmhelm_d(bau, krone @ M((0, 0, 0.5)), r - 0.4, 20.0)
    spitze = krone @ M((0, 0, 0.5 + 20.0))
    # Schwebende Kugel mit Ringen, Splittern und einem Lichtstrahl in den Himmel
    kugel_m = spitze @ M((0, 0, 9.0))
    bau.teil(drehkoerper([(0.0, -3.2), (1.9, -2.6), (3.2, 0.0), (1.9, 2.6), (0.0, 3.2)], 16), MAGIE, m=kugel_m, leuchten=True)
    bau.teil(drehkoerper([(0.0, -1.8), (1.2, -1.4), (1.8, 0.0), (1.2, 1.4), (0.0, 1.8)], 12), MAGIE_HELL, m=kugel_m @ M((0.9, -1.3, 0.8)), leuchten=True)
    for neig, dreh, rr in ((70, 20, 5.2), (60, 110, 6.4), (80, 200, 7.4)):
        bau.teil(rahmen(kreis(rr + 0.12, 32), kreis(rr - 0.12, 32), 0.12), MAGIE_HELL, m=kugel_m @ M((0, 0, 0), dreh, neig), leuchten=True)
    for k in range(8):
        w = 45 * k
        kristall(bau, kugel_m @ M((0, 0, 0), w) @ M((8.5 + (k % 3) * 1.2, 0, (k % 3 - 1) * 2.6), 0, zufall.uniform(-40, 40), zufall.uniform(150, 210)),
                 zufall.uniform(2.0, 3.4), 0.45, KRISTALL if k % 2 else MAGIE)
    bau.teil(zylinder(0.7, 70.0, 10, 0.3), MAGIE_HELL, m=kugel_m @ M((0, 0, 3.0)), leuchten=True)
    bau.teil(zylinder(0.3, 9.0, 8), MAGIE_HELL, m=spitze @ M((0, 0, 0.5)), leuchten=True)
    # Käfige an Ketten am Oberbau
    for k in range(3):
        w = 120 * k + 30
        arm = ob @ M((0, 0, 8.0), w)
        bau.teil(quader(3.2, 0.3, 0.3), EISEN, m=arm @ M((r + 1.4, 0, 0)))
        kette(bau, tuple(arm @ Vector((r + 2.8, 0, 0))), tuple(arm @ Vector((r + 2.8, 0, -3.5))), 0.02)
        kaefig(bau, arm @ M((r + 2.8, 0, -6.2)))
    bau.teil(zylinder(0.18, 3.6, 6), MAGIE_HELL, m=spitze @ M((0, 0, 1.5)), leuchten=True)
    return kugel_m


def halle_d(bau, m, laenge, breite, zufall):
    """Düstere Halle entlang +X (Mitte bei x = 0): steiles Schieferdach, Strebepfeiler mit Dornen,
    glimmende Spitzbogenfenster (Fenster nach -Y)."""
    hoehe = 12.0
    bau.teil(quader(laenge, breite, hoehe), STEIN, MAUER, m=m)
    for s in (-1, 1):
        seite = m @ M((0, s * breite / 2, 0), 0 if s < 0 else 180)
        for i in range(5):
            x = -laenge / 2 + (i + 0.5) * laenge / 5
            fenster_d(bau, seite @ M((x if s < 0 else -x, 0, 2.5)), 1.8, 6.5)
        for i in range(6):
            x = -laenge / 2 + i * laenge / 5
            p = seite @ M((x if s < 0 else -x, 0, 0))
            bau.teil(prisma([(0.0, 0.0), (-2.4, 0.0), (-0.6, hoehe + 1.0), (0.0, hoehe + 1.0)], 1.0), STEIN, MAUER, m=p @ M((-0.5, 0, 0)))
            dorn(bau, p @ M((0, -0.3, hoehe + 1.0)), 3.2, 0.3)
    tan = math.tan(math.radians(62))
    for s in (-1, 1):
        profil = [(s * (breite / 2 + 0.8), hoehe - 0.8 * tan), (0.0, hoehe + breite / 2 * tan), (0.0, hoehe + breite / 2 * tan + 0.4),
                  (s * (breite / 2 + 0.8), hoehe - 0.8 * tan + 0.4)]
        bau.teil(prisma(profil, laenge + 1.0), DACH, ZIEGEL, m=m @ M((-laenge / 2 - 0.5, 0, 0)))
    first = hoehe + breite / 2 * tan
    for i in range(int(laenge / 2) + 1):
        dorn(bau, m @ M((-laenge / 2 + i * laenge / int(laenge / 2), 0, first + 0.3)), 1.2, 0.1)
    for s in (-1, 1):
        giebel = m @ M((s * laenge / 2, 0, 0), 90 * s)
        bau.teil(platte([(-breite / 2, hoehe), (breite / 2, hoehe), (0.0, first)], 0.8, 0.4), STEIN, MAUER, m=giebel)
        fenster_d(bau, giebel @ M((0, -0.4, hoehe + 1.5)), 2.0, 6.0)
        schaedel(bau, giebel @ M((0, -0.6, first - 3.2)), 0.8)


def runenkreis(bau, m, zufall, r=5.0):
    """Leuchtender Runenkreis im Boden mit Altar und schwebendem Kristall."""
    bau.teil(zylinder(r + 0.6, 0.12, 32), BASALT_DUNKEL, m=m)
    for rr in (r, r * 0.7):
        bau.teil(rahmen(kreis(rr + 0.12, 32), kreis(rr - 0.12, 32), 0.05, 0.0), MAGIE, m=m @ M((0, 0, 0.13), 0, 90), leuchten=True)
    for k in range(5):
        w1, w2 = math.tau * k / 5, math.tau * (k + 2) / 5
        a = Vector((math.cos(w1) * r * 0.7, math.sin(w1) * r * 0.7, 0))
        b = Vector((math.cos(w2) * r * 0.7, math.sin(w2) * r * 0.7, 0))
        d = b - a
        bau.teil(quader(d.length, 0.14, 0.05), MAGIE, m=m @ M(tuple((a + b) / 2 + Vector((0, 0, 0.13))), math.degrees(math.atan2(d.y, d.x))), leuchten=True)
    for k in range(16):
        w = 360 * k / 16
        bau.teil(platte([(-0.16, 0), (0.16, 0.15), (0.0, 0.4), (-0.12, 0.2)], 0.03), MAGIE_HELL, m=m @ M((0, 0, 0.14), w) @ M((0, -r * 0.85, 0), 0, -90),
                 leuchten=True)
    for k in range(6):
        w = 60 * k
        stein = m @ M((0, 0, 0), w) @ M((r + 1.5, 0, 0))
        fels(bau, stein, 0.6, zufall.uniform(2.5, 3.5), zufall, BASALT)
        bau.teil(platte([(-0.1, 0), (0.1, 0), (0.0, 1.2)], 0.03), MAGIE, m=stein @ M((0, 0, 0.8), 180) @ M((0, -0.62, 0)), leuchten=True)
    bau.teil(quader(1.8, 1.2, 1.0, 0.08), BASALT, m=m)
    bau.teil(quader(2.0, 1.4, 0.15, 0.05), STEIN_HELL, m=m @ M((0, 0, 1.0)))
    kristall(bau, m @ M((0, 0, 2.4), 20, 0, 180), 1.4, 0.35, MAGIE)
    kristall(bau, m @ M((0, 0, 2.4), 20), 1.4, 0.35, MAGIE)


def schwebender_fels(bau, m, zufall, groesse=3.0):
    """Schwebende Felsinsel: Fels nach unten spitz, oben flach mit Kristallen und hängenden Ketten."""
    fels(bau, m, groesse, groesse * 2.2, zufall, BASALT, kopfueber=True)
    bau.teil(zylinder(groesse * 0.9, 0.4, 8), BASALT_DUNKEL, m=m @ M((0, 0, -0.2)))
    kristallbuendel(bau, m @ M((0, 0, 0.2)), zufall, groesse * 0.5, KRISTALL if zufall.random() < 0.5 else MAGIE)
    for k in range(2):
        w = zufall.uniform(0, math.tau)
        a = Vector(m.translation) + Vector((math.cos(w) * groesse * 0.6, math.sin(w) * groesse * 0.6, -groesse * 1.2))
        kette(bau, a, a + Vector((0.3, 0.2, -zufall.uniform(2.0, 4.0))), 0.05)


# ---------------------------------------------------------------------------
# Die Festung
# ---------------------------------------------------------------------------
class _Gedreht:
    """Gibt Bauteile mit einer zusätzlichen Drehung um die Hochachse an `bau` weiter – so lassen
    sich Torhaus und Rampe, die für das Südtor geschrieben sind, an jede Seite stellen."""

    def __init__(self, bau, grad):
        self._bau = bau
        self._r = M((0, 0, 0), grad)

    def teil(self, geo, *args, m=None, **kw):
        return self._bau.teil(geo, *args, m=self._r @ (m if m is not None else M()), **kw)

    def __getattr__(self, name):
        return getattr(self._bau, name)


def _bei_tor(w, breite):
    """Liegt der Winkel `w` (rad, 0 = +X) näher als `breite` an einer Torrichtung?"""
    for grad in TORE:
        tor = math.radians(grad - 90)
        if abs(math.atan2(math.sin(w - tor), math.cos(w - tor))) < breite:
            return True
    return False


def _rampe(bau, tor_y, z, zufall):
    """Rampe vom Boden hinauf zum Tor (für das Südtor gebaut, `bau` ggf. gedreht):
    Keil aus Mauerwerk, gepflastert, Dornengeländer, Geisterfeuer, Blendbögen, Vorplatz."""
    rampe_von, rampe_bis = tor_y - 8.0, tor_y - 44.0
    laenge = rampe_von - rampe_bis
    bau.teil(prisma([(rampe_bis, 0.0), (rampe_von, 0.0), (rampe_von, z)], 7.0), STEIN, MAUER, m=M((-3.5, 0, 0)))
    bau.teil(prisma([(rampe_bis, 0.0), (rampe_von, z), (rampe_von, z + 0.12), (rampe_bis, 0.12)], 6.4), BASALT_DUNKEL, PLATTEN, m=M((-3.2, 0, 0)))
    for s in (-1, 1):
        for i in range(7):
            t = i / 6
            dorn(bau, M((s * 3.4, rampe_bis + laenge * t, z * t)), 1.8, 0.14)
        for t in (0.3, 0.75):
            geisterfeuer(bau, M((s * 2.7, rampe_bis + laenge * t, z * t)), zufall)
        for i in range(4):
            t = (i + 0.5) / 4
            hoehe = z * t - 1.0
            if hoehe > 1.5:
                bau.teil(platte(bogen(1.6, hoehe - 1.6, 1.6), 0.06, 0.0), "#15141A", m=M((s * 3.52, rampe_bis + laenge * t, 0), 90 * s))
    bau.teil(quader(14.0, 10.0, z), BASALT_DUNKEL, m=M((0, tor_y - 4.0, 0)))
    bau.teil(quader(14.0, 10.0, 0.1), "#3E3B45", PLATTEN, m=M((0, tor_y - 4.0, z - 0.02)))


def festung(seed=66):
    bau = Bau(seed)
    zufall = random.Random(seed)
    z = SOCKEL_H

    # Basaltfelsen: Kern, zerklüfteter Rand aus vielen Brocken, glühende Risse, Plateau oben
    bau.teil(zylinder(33.0, z, 10, 31.0), BASALT_DUNKEL, m=M((0, 0, 0)))
    for k in range(34):
        w = math.tau * k / 34 + zufall.uniform(-0.05, 0.05)
        rr = zufall.uniform(31.0, 35.0)
        if _bei_tor(w, 0.4):
            continue    # vor Toren und Rampen keine Felsen
        fels(bau, M((math.cos(w) * rr, math.sin(w) * rr, 0), zufall.uniform(0, 360)), zufall.uniform(3.5, 6.0), z * zufall.uniform(0.85, 1.25),
             zufall, BASALT if k % 3 else BASALT_DUNKEL)
        if k % 4 == 0:
            riss(bau, M((math.cos(w) * (rr + 3.0), math.sin(w) * (rr + 3.0), 0.5), math.degrees(w) + 90), z * 0.9, zufall)
    bau.teil(zylinder(32.0, 0.12, 10), "#35313C", PLATTEN, m=M((0, 0, z - 0.02)))
    for k in range(14):
        w = math.tau * (k + 0.5) / 14 + zufall.uniform(-0.08, 0.08)
        rr = zufall.uniform(34.0, 38.0)
        if _bei_tor(w, 0.62):
            continue    # vor den Rampen frei
        nadel = M((math.cos(w) * rr, math.sin(w) * rr, 0), zufall.uniform(0, 360))
        fels(bau, nadel, zufall.uniform(1.8, 2.8), z + zufall.uniform(6.0, 13.0), zufall, BASALT)
        if k % 2 == 0:
            ader = M((math.cos(w) * (rr - 1.0), math.sin(w) * (rr - 1.0), zufall.uniform(2.0, z - 2.0)), math.degrees(w) - 90)
            kristallbuendel(bau, ader @ M((0, 0, 0), 0, 70), zufall, 1.3, KRISTALL if k % 4 else MAGIE)

    # Achteckige Ringmauer: Ecken bei 22,5° + k·45°, die geraden Seiten nach Süden, Osten,
    # Norden und Westen tragen die Tore, die schrägen Seiten sind Mauer.
    ecken = [(math.cos(math.radians(22.5 + 45 * k)) * MAUER_R, math.sin(math.radians(22.5 + 45 * k)) * MAUER_R) for k in range(8)]
    tor_y = -MAUER_R * math.cos(math.radians(22.5))
    halbe_seite = MAUER_R * math.sin(math.radians(22.5))
    for k in range(8):
        if k % 2 == 0:
            mauer_d(bau, ecken[k], ecken[(k + 1) % 8], zufall)
    for grad in TORE:
        gedreht = _Gedreht(bau, grad)
        mauer_d(gedreht, (-halbe_seite, tor_y), (-5.5, tor_y), zufall)
        mauer_d(gedreht, (5.5, tor_y), (halbe_seite, tor_y), zufall)
        torhaus_d(gedreht, tor_y, zufall, tuerme=False)
        _rampe(gedreht, tor_y, z, zufall)
    for i, (x, y) in enumerate(ecken):
        turm_d(bau, x, y, z, 4.2, 18.0 + (i % 2) * 5.0, 14.0 + (i % 3) * 2.0, zufall)

    # Bergfried hinten, Halle links, Runenkreis rechts, Geisterfeuer im Hof
    bergfried(bau, 0.0, 9.0, zufall)
    # (Halle und Runenkreis in den schrägen Ecken, damit die vier Wege zu den Toren frei bleiben)
    halle_d(bau, M((-11.0, -11.0, z), 45), 11.0, 7.0, zufall)
    runenkreis(bau, M((11.0, -11.0, z)), zufall)
    for x, y in ((-4.0, -16.0), (4.0, -16.0), (6.5, 6.5), (-6.5, 6.5), (16.0, 4.0), (16.0, -4.0), (-16.0, 4.0), (-16.0, -4.0)):
        geisterfeuer(bau, M((x, y, z)), zufall)
    for i in range(6):
        x = zufall.choice((-1, 1)) * zufall.uniform(5.0, 9.0)
        y = zufall.choice((-1, 1)) * zufall.uniform(15.0, 20.0)
        if i % 2:
            x, y = y, x
        kristallbuendel(bau, M((x, y, z), zufall.uniform(0, 360)), zufall, 0.8, KRISTALL if i % 2 else MAGIE)

    # Ketten zwischen Bergfried und den hinteren Türmen
    for k in (1, 2):
        x, y = ecken[k]
        kette(bau, (x * 0.85, y * 0.85, z + 21.0), (math.copysign(7.5, x), 9.0 + math.copysign(1.0, y) * 6.0, z + 22.0), 2.5)

    # Schwebende Felsinseln rund um die Festung
    for k in range(6):
        w = math.tau * k / 6 + 0.4
        rr = zufall.uniform(40.0, 52.0)
        schwebender_fels(bau, M((math.cos(w) * rr, math.sin(w) * rr + 6.0, zufall.uniform(22.0, 38.0)), zufall.uniform(0, 360)), zufall,
                         zufall.uniform(2.0, 3.8))

    # Tote Bäume und Felsen am Fuß
    for k in range(9):
        w = math.tau * k / 9 + 0.2
        rr = zufall.uniform(38.0, 46.0)
        p = M((math.cos(w) * rr, math.sin(w) * rr, 0), zufall.uniform(0, 360))
        if _bei_tor(w, 0.45):
            continue    # vor den Rampen frei lassen
        if k % 2 == 0:
            toter_baum(bau, p, zufall, zufall.uniform(5.0, 7.5))
        else:
            fels(bau, p, zufall.uniform(1.5, 2.8), zufall.uniform(2.0, 4.0), zufall)
            kristallbuendel(bau, p @ M((0, 0, 0.5)), zufall, 0.6, MAGIE)

    # Tote Ranken an der Ringmauer
    for i, (x, y) in enumerate(ecken):
        nx, ny = ecken[(i + 1) % 8]
        mx, my = (x + nx) / 2, (y + ny) / 2
        if i % 2:
            continue    # Torseiten
        aussen = math.degrees(math.atan2(my, mx)) - 90
        efeu(bau, M((mx * 1.04, my * 1.04, z), aussen), 3.5, 8.0, 500 + i, 0.8, RANKEN)
    return bau.fertig("Schattenfestung", glas_leuchten=1.0, ursprung=(0.0, 0.0))
