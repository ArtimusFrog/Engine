"""Die Burganlage: die Burg aus burg.py, dazu Ringmauer mit Rundtürmen, Torhaus mit Fallgitter,
Innenhof mit Straße, Brunnenplatz, Statuen, Laternen, Hecken, Markt und Übungsplatz.

Alles landet in einem Mesh (ein Modell für die Insel). Maße in Metern, Z oben, das Tor zeigt
nach -Y. Die Burg steht mit ihrer Terrasse bei y = -19 … 19, das Tor der Ringmauer bei
y = TOR_Y; dazwischen liegt der Vorhof.
"""

import math
import random

from mathutils import Vector

from burg import (BANNER, DACH, DACH_DUNKEL, EISEN, GOLD, HOLZ, MAUER, PLATTEN, SOCKEL, STEIN_HELL, ZIEGEL, Bau, M,
                  banner, bogen, bogen_hoehe, bogenfries, burg_teile, drehkoerper, efeu, gesims, kreis, platte, prisma,
                  pyramide, quader, rahmen, satteldach, zinnen, zylinder, _blatt)

MAUERSTEIN = "#CBB48F"
MAUERFUSS = "#A6957F"
PFLASTER = "#A89C8C"
WASSER = "#3F8DB0"
STATUE = "#C9C2B4"
HECKE = "#4A8A3A"
HECKE_HELL = "#5E9E45"
ERDE = "#5E4430"
STROH = "#D8B45A"
LICHT = "#FFC56B"

# Ringmauer: Ecken gegen den Uhrzeigersinn (von oben), Tor vorne in der Mitte
MAUER_X = 60.0
TOR_Y = -75.0
HINTEN_Y = 32.0
MAUER_H = 9.0
MAUER_D = 2.6
TOR_HW = 5.2        # halbe Breite des Torbaus


# ---------------------------------------------------------------------------
# Mauer und Türme
# ---------------------------------------------------------------------------
def mauer(bau, p1, p2, scharten=True):
    """Mauerstück von p1 nach p2 (gegen den Uhrzeigersinn: außen liegt rechts): schräger Fuß,
    Schießscharten, Wehrgang mit Zinnen und Maschikulis außen, niedrige Brüstung innen."""
    dx, dy = p2[0] - p1[0], p2[1] - p1[1]
    laenge = math.hypot(dx, dy)
    m = M((p1[0], p1[1], 0), math.degrees(math.atan2(dy, dx)))
    d = MAUER_D
    bau.teil(quader(laenge, d, MAUER_H), MAUERSTEIN, MAUER, m=m @ M((laenge / 2, 0, 0)))
    bau.teil(prisma([(-d / 2 + 0.01, 0), (-d / 2 + 0.01, 2.4), (-d / 2 - 0.9, 0)], laenge), MAUERFUSS, MAUER, m=m)
    gesims(bau, m @ M((0, -d / 2, 2.4)), laenge, 0.22, 0.22)
    gesims(bau, m @ M((0, -d / 2, MAUER_H * 0.62)), laenge, 0.15, 0.18)
    bau.teil(quader(laenge, d - 0.3, 0.08), SOCKEL, PLATTEN, m=m @ M((laenge / 2, 0, MAUER_H)))
    bogenfries(bau, m @ M((0, -d / 2, MAUER_H - 0.1)), laenge, 1.1)
    zinnen(bau, m @ M((0, -d / 2 - 0.1, MAUER_H - 0.1)), laenge, 1.9, 0.65)
    bau.teil(quader(laenge, 0.45, 1.0), MAUERSTEIN, MAUER, m=m @ M((laenge / 2, d / 2 - 0.22, MAUER_H)))
    bau.teil(quader(laenge, 0.55, 0.12, 0.02), STEIN_HELL, m=m @ M((laenge / 2, d / 2 - 0.22, MAUER_H + 1.0)))
    if scharten:
        anzahl = max(int(laenge / 6.0), 1)
        for i in range(anzahl):
            x = (i + 0.5) * laenge / anzahl
            schiessscharte(bau, m @ M((x, -d / 2, 4.0)))


def schiessscharte(bau, m, hoehe=1.5):
    """Schmale Scharte mit Querschlitz und hellem Rahmen (Wand bei y = 0, außen = -Y)."""
    bau.teil(rahmen([(0.3, -0.1), (0.3, hoehe + 0.1), (-0.3, hoehe + 0.1), (-0.3, -0.1)],
                    [(0.1, 0.0), (0.1, hoehe), (-0.1, hoehe), (-0.1, 0.0)], 0.12), STEIN_HELL, m=m)
    bau.teil(platte([(0.1, 0.0), (0.1, hoehe), (-0.1, hoehe), (-0.1, 0.0)], 0.03), "#161412", m=m)
    bau.teil(platte([(0.28, hoehe * 0.55), (0.28, hoehe * 0.62), (-0.28, hoehe * 0.62), (-0.28, hoehe * 0.55)], 0.04), "#161412", m=m)


def rundturm(bau, x, y, r, hoehe, dach, wimpel=True, ecken=12):
    """Runder Mauerturm: schräger Fuß, Scharten, Maschikulis, Zinnenkranz, grünes Kegeldach."""
    m = M((x, y, 0))
    dreh = math.pi / ecken
    bau.teil(zylinder(r + 1.0, 2.6, ecken, r, dreh), MAUERFUSS, MAUER, m=m)
    bau.teil(zylinder(r, hoehe, ecken, drehung=dreh), MAUERSTEIN, MAUER, m=m)
    for z in (2.6, hoehe * 0.55):
        bau.teil(zylinder(r + 0.2, 0.24, ecken, drehung=dreh), STEIN_HELL, m=m @ M((0, 0, z)))
    seite = r * math.cos(dreh)
    for k in range(ecken):
        w = 360 * k / ecken
        if k % 2 == 0:
            schiessscharte(bau, m @ M((0, 0, 0), w) @ M((0, -seite, 4.2 if k % 4 == 0 else hoehe * 0.62)))
        # Konsolen (Maschikulis) rundum unter dem Zinnenkranz
        bau.teil(prisma([(0.02, 0), (-0.5, 0), (-0.5, -0.2), (-0.1, -0.7), (0.02, -0.7)], 0.3), STEIN_HELL,
                 m=m @ M((0, 0, 0), w + 180 / ecken) @ M((-0.15, -seite, hoehe - 0.1)))
    bau.teil(zylinder(r + 0.55, 0.3, ecken, drehung=dreh), STEIN_HELL, m=m @ M((0, 0, hoehe - 0.3)))
    ra = r + 0.45
    for k in range(ecken):
        w1, w2 = math.tau * k / ecken, math.tau * (k + 1) / ecken
        p1 = Vector((math.cos(w1) * ra, math.sin(w1) * ra, 0))
        p2 = Vector((math.cos(w2) * ra, math.sin(w2) * ra, 0))
        zinnen(bau, m @ M((p1.x, p1.y, hoehe), math.degrees(math.atan2(p2.y - p1.y, p2.x - p1.x))), (p2 - p1).length, 1.8, 0.55)
    # Kegeldach mit Traufring, Knauf und Wimpel
    bau.teil(zylinder(r - 0.1, 0.3, ecken, drehung=dreh), DACH_DUNKEL, m=m @ M((0, 0, hoehe + 0.1)))
    bau.teil(pyramide(r - 0.2, dach, ecken, dreh), DACH, ZIEGEL, m=m @ M((0, 0, hoehe + 0.35)))
    spitze = hoehe + 0.35 + dach
    bau.teil(drehkoerper([(0.18, 0), (0.24, 0.15), (0.0, 0.4)], 8), GOLD, m=m @ M((0, 0, spitze - 0.1)))
    if wimpel:
        bau.teil(zylinder(0.04, 2.4, 6), EISEN, m=m @ M((0, 0, spitze + 0.2)))
        bau.teil(platte([(0, 0), (1.6, -0.3), (0, -0.7)], 0.03, 0.015), BANNER, m=m @ M((0.04, 0, spitze + 2.5)))
        bau.teil(drehkoerper([(0.0, 0), (0.08, 0.06), (0.0, 0.14)], 6), GOLD, m=m @ M((0, 0, spitze + 2.58)))


def fackel(bau, m):
    """Wandfackel: Eisenhalter, Stab, leuchtende Flamme (Wand bei y = 0, außen = -Y)."""
    bau.teil(quader(0.16, 0.1, 0.4), EISEN, m=m)
    bau.teil(quader(0.08, 0.35, 0.08), EISEN, m=m @ M((0, -0.2, 0.15)))
    bau.teil(zylinder(0.05, 0.7, 6, 0.07), HOLZ, m=m @ M((0, -0.38, 0.0), 0, 15))
    bau.teil(zylinder(0.1, 0.12, 6), EISEN, m=m @ M((0, -0.45, 0.62)))
    bau.teil(pyramide(0.12, 0.45, 5), "#FF9A2E", m=m @ M((0, -0.45, 0.72)), leuchten=True)
    bau.teil(pyramide(0.07, 0.3, 4), "#FFE08A", m=m @ M((0, -0.45, 0.74)), leuchten=True)


def wappen(bau, m, g=1.0):
    """Wappenschild: grünes Feld, goldener Rand und goldene Raute, auf einer Steinplatte."""
    umriss = [(1.0, 1.2), (-1.0, 1.2), (-1.0, 0.2), (-0.75, -0.7), (0.0, -1.3), (0.75, -0.7), (1.0, 0.2)]
    gross = [(x * g, z * g) for x, z in umriss]
    innen = [(x * g * 0.84, z * g * 0.84 + 0.05 * g) for x, z in umriss]
    bau.teil(platte([(x * 1.25, z * 1.2) for x, z in gross], 0.2), STEIN_HELL, m=m)
    bau.teil(platte(gross, 0.08, -0.2), GOLD, m=m)
    bau.teil(platte(innen, 0.04, -0.28), BANNER, m=m)
    raute = [(0, 0.75 * g), (-0.45 * g, 0.0), (0, -0.75 * g), (0.45 * g, 0.0)]
    bau.teil(platte(raute, 0.04, -0.3), GOLD, m=m)
    bau.teil(platte(kreis(0.16 * g, 8), 0.04, -0.34), "#B23A3A", m=m)


def torhaus(bau, y0=TOR_Y):
    """Torhaus: zwei Tortürme, Durchfahrt mit Spitzbogen (begehbar), halb hochgezogenes
    Fallgitter, offene Torflügel, Wappen, Fackeln, Banner und Zinnen."""
    hw, tiefe, h = TOR_HW, 8.0, 13.0
    vorne, hinten = y0 - 4.5, y0 + 3.5
    oeff = 2.6
    r_bogen = oeff * 1.6
    hs = 4.8
    scheitel = hs + bogen_hoehe(oeff, r_bogen)
    for s in (-1, 1):
        bau.teil(quader(hw - oeff, tiefe, h), MAUERSTEIN, MAUER, m=M((s * (oeff + hw) / 2, (vorne + hinten) / 2, 0)))
        rundturm(bau, s * (hw + 2.4), y0 - 1.2, 4.6, 16.0, 9.5)
    # Bogenfeld über der Durchfahrt (die Unterseite bildet das Gewölbe)
    feld = bogen(oeff, hs, r_bogen, nur_bogen=True) + [(-oeff, h), (oeff, h)]
    bau.teil(platte(feld, tiefe, 0.0), MAUERSTEIN, MAUER, m=M((0, hinten, 0)))
    bau.teil(quader(2 * oeff, tiefe, 0.08), SOCKEL, PLATTEN, m=M((0, (vorne + hinten) / 2, 0)))
    # Bogenrahmen vorne und hinten
    for y, w in ((vorne, 0), (hinten, 180)):
        bau.teil(rahmen(bogen(oeff + 0.6, hs, r_bogen + 0.6), bogen(oeff, hs, r_bogen), 0.45), STEIN_HELL, m=M((0, y, 0), w))
        bau.teil(rahmen(bogen(oeff + 1.0, hs, r_bogen + 1.0, nur_bogen=True), bogen(oeff + 0.6, hs, r_bogen + 0.6, nur_bogen=True), 0.2, offen=True),
                 STEIN_HELL, m=M((0, y, 0), w))
    # Fallgitter (halb hochgezogen)
    unten = 3.9
    for i in range(9):
        x = -oeff + 0.2 + i * (2 * oeff - 0.4) / 8
        bau.teil(quader(0.13, 0.13, scheitel - unten + 0.5), EISEN, m=M((x, vorne + 1.4, unten)))
        bau.teil(pyramide(0.1, 0.35, 4), EISEN, m=M((x, vorne + 1.4, unten), 0, 180))
    for z in (unten + 0.4, unten + 1.5, unten + 2.6, unten + 3.7):
        bau.teil(quader(2 * oeff, 0.1, 0.12), EISEN, m=M((0, vorne + 1.4, z)))
    # Offene Torflügel an der Innenseite
    for s in (-1, 1):
        fluegel = M((s * (oeff - 0.15), hinten + 1.35, 0), 90)
        bau.teil(quader(2.6, 0.22, 6.4, 0.03), HOLZ, m=fluegel)
        for z in (0.8, 3.2, 5.6):
            bau.teil(quader(2.4, 0.28, 0.14), EISEN, m=fluegel @ M((0, 0, z)))
    # Wehrgang mit Zinnen, Maschikulis, Wappen, Fackeln
    bau.teil(quader(2 * hw, tiefe - 0.4, 0.1), SOCKEL, PLATTEN, m=M((0, (vorne + hinten) / 2, h)))
    front = M((-hw, vorne, 0))
    bogenfries(bau, front @ M((0, 0, h - 0.1)), 2 * hw, 1.0)
    zinnen(bau, front @ M((0, -0.1, h - 0.1)), 2 * hw, 1.9, 0.65)
    zinnen(bau, M((hw, hinten, 0), 180) @ M((0, -0.1, h - 0.1)), 2 * hw, 1.9, 0.65)
    gesims(bau, front @ M((0, 0, scheitel + 0.6)), 2 * hw, 0.25, 0.25)
    wappen(bau, M((0, vorne, scheitel + 2.4)), 1.1)
    for s in (-1, 1):
        fackel(bau, M((s * (oeff + 1.2), vorne, 3.4)))
        fackel(bau, M((s * (oeff + 1.2), hinten, 3.4), 180))
        banner(bau, M((s * (hw + 2.4), y0 - 1.2 - 4.6, 13.2)), 1.2, 4.2)


def treppe(bau, x0, y0, richtung, stufen=30):
    """Treppe an der Mauerinnenseite hinauf zum Wehrgang (steigt in Richtung `richtung` = ±1 in X)."""
    for i in range(stufen):
        hoehe = MAUER_H * (i + 1) / stufen
        bau.teil(quader(0.42, 1.8, hoehe), MAUERSTEIN, MAUER, m=M((x0 + richtung * (i * 0.4 + 0.2), y0, 0)))
        bau.teil(quader(0.44, 1.84, 0.06), STEIN_HELL, m=M((x0 + richtung * (i * 0.4 + 0.2), y0, hoehe)))


# ---------------------------------------------------------------------------
# Hof: Straße, Brunnen, Statuen, Laternen, Grün
# ---------------------------------------------------------------------------
def strasse(bau, y_von, y_bis, breite=7.0):
    laenge = y_bis - y_von
    mitte = (y_von + y_bis) / 2
    bau.teil(quader(breite, laenge, 0.08), PFLASTER, PLATTEN, m=M((0, mitte, 0)))
    for s in (-1, 1):
        bau.teil(quader(0.3, laenge, 0.18, 0.03), MAUERFUSS, m=M((s * (breite / 2 + 0.15), mitte, 0)))


def brunnen(bau, x, y):
    """Achteckiger Brunnen mit Wasserbecken, Säule mit Schale, goldener Spitze und Wasserstrahlen."""
    m = M((x, y, 0))
    bau.teil(zylinder(11.0, 0.09, 24), PFLASTER, PLATTEN, m=m)
    for k in range(24):
        bau.teil(quader(2.95, 0.35, 0.2, 0.03), MAUERFUSS, m=m @ M((0, 0, 0), 360 * k / 24 + 7.5) @ M((0, -11.0, 0)))
    ecken, r = 8, 4.4
    seite = r * math.cos(math.pi / ecken)
    kante = 2 * r * math.sin(math.pi / ecken)
    for k in range(ecken):
        w = 360 * k / ecken + 180 / ecken
        bau.teil(quader(kante + 0.1, 0.5, 0.95), STEIN_HELL, MAUER, m=m @ M((0, 0, 0), w) @ M((0, -seite + 0.25, 0)))
        bau.teil(quader(kante + 0.35, 0.75, 0.16, 0.04), STEIN_HELL, m=m @ M((0, 0, 0), w) @ M((0, -seite + 0.25, 0.95)))
    bau.teil(zylinder(r - 0.4, 0.72, ecken, drehung=0.0), WASSER, m=m)
    profil = [(1.0, 0.0), (1.0, 0.9), (0.55, 1.2), (0.42, 2.2), (0.6, 2.35), (1.6, 2.5), (1.65, 2.75), (0.4, 2.85),
              (0.3, 3.9), (0.55, 4.05), (0.5, 4.25), (0.15, 4.4)]
    bau.teil(drehkoerper(profil, 8), STEIN_HELL, m=m)
    bau.teil(zylinder(1.45, 0.06, 8), WASSER, m=m @ M((0, 0, 2.7)))
    bau.teil(drehkoerper([(0.28, 0), (0.35, 0.3), (0.2, 0.7), (0.26, 0.9), (0.0, 1.3)], 8), GOLD, m=m @ M((0, 0, 4.35)))
    # Wasserstrahlen: aus der oberen Schale in das Becken
    for k in range(8):
        w = 45 * k
        for i in range(4):
            t0, t1 = i / 4, (i + 1) / 4
            p0 = (1.6 + t0 * 1.3, 2.6 - (t0 ** 2) * 1.9)
            p1 = (1.6 + t1 * 1.3, 2.6 - (t1 ** 2) * 1.9)
            laenge = math.hypot(p1[0] - p0[0], p1[1] - p0[1])
            neig = math.degrees(math.atan2(p1[1] - p0[1], p1[0] - p0[0]))
            bau.teil(quader(laenge + 0.05, 0.07, 0.07), "#A8DDF0", m=m @ M((0, 0, 0), w) @ M((p0[0], 0, p0[1]), 0, 0, -neig) @ M((laenge / 2, 0, 0)),
                     leuchten=False)


def ritterstatue(bau, m):
    """Ritterstatue auf einem Sockel: stützt sich auf ein Schwert, Umhang, Helm (Blick nach -Y)."""
    bau.teil(quader(1.7, 1.7, 0.4, 0.05), MAUERFUSS, m=m)
    bau.teil(quader(1.25, 1.25, 1.5), STEIN_HELL, MAUER, m=m @ M((0, 0, 0.4)))
    bau.teil(quader(1.55, 1.55, 0.25, 0.05), STEIN_HELL, m=m @ M((0, 0, 1.9)))
    f = m @ M((0, 0, 2.15))
    for s in (-1, 1):
        bau.teil(quader(0.26, 0.32, 0.95, 0.03), STATUE, m=f @ M((s * 0.17, 0, 0)))
        bau.teil(quader(0.3, 0.42, 0.14, 0.03), STATUE, m=f @ M((s * 0.17, -0.05, 0)))
        bau.teil(drehkoerper([(0.0, -0.12), (0.2, -0.08), (0.24, 0.05), (0.0, 0.14)], 6), STATUE, m=f @ M((s * 0.42, 0, 1.84)))
        bau.teil(quader(0.19, 0.22, 0.62, 0.03), STATUE, m=f @ M((s * 0.34, -0.12, 1.2), 0, 20 * s))
    bau.teil(zylinder(0.4, 0.5, 8, 0.36), STATUE, m=f @ M((0, 0, 0.78)))
    bau.teil(quader(0.7, 0.42, 0.62, 0.05), STATUE, m=f @ M((0, 0, 1.25)))
    bau.teil(zylinder(0.19, 0.34, 8), STATUE, m=f @ M((0, 0, 1.9)))
    bau.teil(pyramide(0.2, 0.22, 8, 0), STATUE, m=f @ M((0, 0, 2.24)))
    bau.teil(quader(0.26, 0.04, 0.05), "#5E5850", m=f @ M((0, -0.19, 2.08)))
    # Schwert, Spitze auf dem Boden, Hände auf dem Knauf
    bau.teil(quader(0.1, 0.035, 1.15), STATUE, m=f @ M((0, -0.42, 0.0)))
    bau.teil(quader(0.5, 0.08, 0.08), STATUE, m=f @ M((0, -0.42, 1.15)))
    bau.teil(quader(0.07, 0.07, 0.22), STATUE, m=f @ M((0, -0.42, 1.23)))
    bau.teil(drehkoerper([(0.0, 0), (0.07, 0.04), (0.0, 0.1)], 6), STATUE, m=f @ M((0, -0.42, 1.45)))
    bau.teil(quader(0.32, 0.18, 0.16, 0.03), STATUE, m=f @ M((0, -0.36, 1.32)))
    # Umhang
    bau.teil(platte([(-0.45, 1.95), (0.45, 1.95), (0.6, 0.15), (-0.6, 0.15)], 0.08, 0.0), STATUE, m=f @ M((0, 0.3, 0)))


def laterne(bau, m, hoehe=3.2):
    """Laternenmast aus Eisen mit leuchtender Laterne."""
    bau.teil(drehkoerper([(0.26, 0), (0.26, 0.18), (0.14, 0.32), (0.08, 0.5)], 8), EISEN, m=m)
    bau.teil(zylinder(0.06, hoehe, 6), EISEN, m=m)
    bau.teil(zylinder(0.1, 0.1, 6), GOLD, m=m @ M((0, 0, hoehe * 0.6)))
    k = m @ M((0, 0, hoehe))
    bau.teil(quader(0.42, 0.42, 0.06), EISEN, m=k)
    bau.teil(quader(0.3, 0.3, 0.46), LICHT, m=k @ M((0, 0, 0.06)), leuchten=True)
    for sx, sy in ((1, 1), (-1, 1), (-1, -1), (1, -1)):
        bau.teil(quader(0.05, 0.05, 0.5), EISEN, m=k @ M((sx * 0.17, sy * 0.17, 0.05)))
    bau.teil(pyramide(0.34, 0.32, 4), EISEN, m=k @ M((0, 0, 0.55)))
    bau.teil(drehkoerper([(0.0, 0), (0.06, 0.05), (0.0, 0.12)], 6), GOLD, m=k @ M((0, 0, 0.86)))


def hecke(bau, m, laenge, zufall):
    """Geschnittene Hecke entlang +X ab 0 mit Blättern an den Seiten."""
    bau.teil(quader(laenge, 1.0, 1.05, 0.14), HECKE, m=m @ M((laenge / 2, 0, 0)), schwankung=0.06)
    bau.teil(quader(laenge - 0.2, 0.8, 0.14, 0.06), HECKE_HELL, m=m @ M((laenge / 2, 0, 1.0)))
    for _ in range(int(laenge * 5)):
        x = zufall.uniform(0.1, laenge - 0.1)
        s = zufall.choice((-1, 1))
        bau.teil(_blatt(zufall.uniform(0.1, 0.16)), zufall.choice((HECKE, HECKE_HELL, "#3E7A30")),
                 m=m @ M((x, s * 0.5, zufall.uniform(0.15, 1.0)), 0 if s < 0 else 180) @ M((0, 0, 0), 0, 0, zufall.uniform(-40, 40)))


def blumenbeet(bau, m, laenge, breite, zufall):
    bau.teil(quader(laenge, breite, 0.18), ERDE, m=m @ M((laenge / 2, 0, 0)))
    farben = ("#E0524F", "#F2D544", "#B07CFF", "#F1EDE4", "#E88AB4")
    for _ in range(int(laenge * breite * 5)):
        x, y = zufall.uniform(0.1, laenge - 0.1), zufall.uniform(-breite / 2 + 0.1, breite / 2 - 0.1)
        h = zufall.uniform(0.25, 0.45)
        bau.teil(quader(0.03, 0.03, h), "#4E8A34", m=m @ M((x, y, 0.15)))
        bau.teil(pyramide(0.07, 0.08, 5, 0), zufall.choice(farben), m=m @ M((x, y, 0.15 + h)))


def zierbaum(bau, m, zufall, form="kugel"):
    """Formschnittbaum im Steinkübel (Kugeln oder Kegel)."""
    bau.teil(quader(1.3, 1.3, 0.75, 0.06), STEIN_HELL, m=m)
    bau.teil(quader(1.12, 1.12, 0.05), ERDE, m=m @ M((0, 0, 0.72)))
    bau.teil(zylinder(0.09, 1.2, 6), "#6A4A2E", m=m @ M((0, 0, 0.75)))
    if form == "kugel":
        for z, r in ((1.95, 0.75), (2.95, 0.5)):
            bau.teil(drehkoerper([(0.0, -r), (r * 0.7, -r * 0.7), (r, 0.0), (r * 0.7, r * 0.7), (0.0, r)], 8), HECKE,
                     m=m @ M((0, 0, z), zufall.uniform(0, 45)), schwankung=0.08)
    else:
        bau.teil(zylinder(0.85, 2.6, 8, 0.05), HECKE, m=m @ M((0, 0, 1.2)), schwankung=0.08)


def baum(bau, m, zufall):
    """Runder Laubbaum (Stamm mit zwei Ästen, Krone aus Kugeln)."""
    bau.teil(zylinder(0.28, 3.2, 7, 0.18), "#6A4A2E", m=m)
    for s in (-1, 1):
        bau.teil(zylinder(0.12, 1.6, 6, 0.06), "#6A4A2E", m=m @ M((0, 0, 2.4), 0, 0, 40 * s))
    for _ in range(5):
        r = zufall.uniform(1.3, 1.9)
        ort = (zufall.uniform(-1.2, 1.2), zufall.uniform(-1.2, 1.2), zufall.uniform(3.6, 5.0))
        bau.teil(drehkoerper([(0.0, -r), (r * 0.75, -r * 0.6), (r, 0.1), (r * 0.7, r * 0.7), (0.0, r)], 7),
                 zufall.choice(("#5E9E3A", "#6FA844", "#4E8A34")), m=m @ M(ort, zufall.uniform(0, 60)), schwankung=0.06)


# ---------------------------------------------------------------------------
# Markt und Übungsplatz
# ---------------------------------------------------------------------------
def fass(bau, m):
    profil = [(0.34, 0.0), (0.4, 0.25), (0.43, 0.5), (0.4, 0.75), (0.34, 1.0)]
    bau.teil(drehkoerper(profil, 10), "#8A5A34", m=m, schwankung=0.05)
    for z in (0.12, 0.88):
        bau.teil(zylinder(0.385, 0.06, 10), EISEN, m=m @ M((0, 0, z - 0.03)))
    bau.teil(zylinder(0.3, 0.02, 10), "#6E4826", m=m @ M((0, 0, 0.99)))


def kiste(bau, m, g=0.8):
    bau.teil(quader(g, g, g, 0.03), "#A0703F", m=m, schwankung=0.05)
    for s in (-1, 1):
        bau.teil(quader(g + 0.02, 0.06, 0.1), "#6E4826", m=m @ M((0, s * g / 2, g * 0.2)))
        bau.teil(quader(g + 0.02, 0.06, 0.1), "#6E4826", m=m @ M((0, s * g / 2, g * 0.75)))


def sack(bau, m):
    bau.teil(drehkoerper([(0.0, 0.0), (0.3, 0.05), (0.34, 0.3), (0.22, 0.6), (0.08, 0.7), (0.12, 0.8), (0.0, 0.82)], 7), "#C9B48A", m=m, schwankung=0.05)


def marktstand(bau, m, stoff, zufall):
    """Marktstand: Holzgerüst, Theke, gestreifte Markise, Waren (lokal: Kunden stehen bei -Y)."""
    for sx in (-1, 1):
        for sy, h in ((-1, 2.3), (1, 2.9)):
            bau.teil(quader(0.14, 0.14, h), HOLZ, m=m @ M((sx * 1.45, sy * 0.95, 0)))
    bau.teil(quader(2.9, 0.9, 0.95, 0.03), "#8A5A34", m=m @ M((0, -0.55, 0)))
    bau.teil(quader(3.0, 1.0, 0.08, 0.02), "#B98A56", m=m @ M((0, -0.55, 0.95)))
    # Markise aus Streifen
    neig = math.degrees(math.atan2(0.6, 2.2))
    for i in range(6):
        x = -1.5 + (i + 0.5) * 0.5
        bau.teil(quader(0.5, 2.4, 0.05), stoff if i % 2 == 0 else "#F1E6CF", m=m @ M((x, 0, 2.6), 0, neig) @ M((0, 0, 0)))
    for i in range(6):
        x = -1.5 + (i + 0.5) * 0.5
        bau.teil(platte([(-0.25, 0), (0.25, 0), (0, -0.3)], 0.03), stoff if i % 2 == 0 else "#F1E6CF", m=m @ M((x, -1.2, 2.25)))
    # Waren: Körbe mit Obst, kleine Kisten
    for i in range(3):
        k = m @ M((-0.9 + i * 0.9, -0.55, 1.03))
        bau.teil(zylinder(0.3, 0.2, 8, 0.36), "#B98A56", m=k)
        farbe_ = zufall.choice(("#E0524F", "#F2D544", "#7CB342", "#E0A43A"))
        for _ in range(6):
            bau.teil(drehkoerper([(0.0, -0.07), (0.07, 0.0), (0.0, 0.07)], 5), farbe_,
                     m=k @ M((zufall.uniform(-0.18, 0.18), zufall.uniform(-0.18, 0.18), 0.24)))
    fass(bau, m @ M((1.9, 0.4, 0)))
    kiste(bau, m @ M((-1.95, 0.5, 0), 20), 0.7)
    sack(bau, m @ M((-2.0, -0.4, 0)))


def karren(bau, m):
    bau.teil(quader(1.6, 2.6, 0.12), HOLZ, m=m @ M((0, 0, 0.7)))
    for s in (-1, 1):
        bau.teil(quader(0.08, 2.6, 0.5), "#8A5A34", m=m @ M((s * 0.78, 0, 0.8)))
        bau.teil(zylinder(0.55, 0.12, 12, drehung=0.1), "#6E4826", m=m @ M((s * 0.92, 0.3, 0.55), 0, 0, 90) @ M((0, 0, -0.06)))
        bau.teil(zylinder(0.08, 0.2, 6), EISEN, m=m @ M((s * 0.92, 0.3, 0.55), 0, 0, 90) @ M((0, 0, -0.1)))
        bau.teil(quader(0.08, 2.0, 0.08), HOLZ, m=m @ M((s * 0.4, -2.2, 0.55), 0, -8))
    bau.teil(quader(1.9, 0.08, 0.08), EISEN, m=m @ M((0, 0.3, 0.52)))
    for i in range(3):
        sack(bau, m @ M((-0.4 + i * 0.4, 0.5 - i * 0.3, 0.82), i * 30))


def ziehbrunnen(bau, m):
    for k in range(12):
        bau.teil(quader(0.5, 0.35, 0.85), MAUERSTEIN, MAUER, m=m @ M((0, 0, 0), k * 30) @ M((0, -0.95, 0)))
    bau.teil(zylinder(0.85, 0.05, 12), "#1E2A30", m=m @ M((0, 0, 0.5)))
    for s in (-1, 1):
        bau.teil(quader(0.16, 0.16, 2.4), HOLZ, m=m @ M((s * 1.0, 0, 0.85)))
    bau.teil(zylinder(0.1, 2.2, 8), HOLZ, m=m @ M((-1.1, 0, 2.5), 0, 0, 90))
    satteldach(bau, M((0, 0, 0)) @ m @ M((-1.3, 0, 3.2)), 2.6, 0.9, 0.0, 40.0, 0.25, 0.1, False)
    bau.teil(zylinder(0.015, 1.3, 4), "#C9B48A", m=m @ M((0, 0, 1.2)))
    bau.teil(zylinder(0.18, 0.3, 8, 0.22), "#8A5A34", m=m @ M((0, 0, 0.95)))


def strohpuppe(bau, m):
    bau.teil(zylinder(0.07, 2.3, 6), HOLZ, m=m)
    bau.teil(quader(1.4, 0.1, 0.1), HOLZ, m=m @ M((0, 0, 1.55)))
    bau.teil(drehkoerper([(0.25, 0.8), (0.38, 1.0), (0.4, 1.4), (0.3, 1.75), (0.15, 1.85)], 8), STROH, m=m, schwankung=0.06)
    bau.teil(drehkoerper([(0.0, 0.0), (0.22, 0.05), (0.24, 0.25), (0.15, 0.42), (0.0, 0.46)], 7), "#C9B48A", m=m @ M((0, 0, 1.85)))
    for z in (1.1, 1.5):
        bau.teil(zylinder(0.41, 0.05, 8), "#6E4826", m=m @ M((0, 0, z)))
    for s in (-1, 1):
        bau.teil(drehkoerper([(0.1, 0.0), (0.13, 0.2), (0.1, 0.4)], 6), STROH, m=m @ M((s * 0.65, 0, 1.6), 0, 0, 90))


def zielscheibe(bau, m):
    for sx in (-1, 1):
        bau.teil(quader(0.1, 0.1, 2.0), HOLZ, m=m @ M((sx * 0.5, 0.1, 0), 0, sx * -10))
    bau.teil(quader(0.1, 0.1, 1.9), HOLZ, m=m @ M((0, 0.6, 0), 0, -20))
    mitte = m @ M((0, -0.05, 1.3), 0, 90)
    for r, farbe_, d in ((0.75, STROH, 0.14), (0.62, "#F1EDE4", 0.16), (0.46, "#B23A3A", 0.18), (0.3, "#F1EDE4", 0.2), (0.14, GOLD, 0.22)):
        bau.teil(zylinder(r, d, 14), farbe_, m=mitte @ M((0, 0, -d / 2)))
    bau.teil(zylinder(0.015, 0.6, 4), HOLZ, m=m @ M((0.2, -0.3, 1.45), 0, 80, 10))


def waffenstaender(bau, m, zufall):
    for s in (-1, 1):
        bau.teil(quader(0.12, 0.5, 1.8), HOLZ, m=m @ M((s * 1.1, 0, 0)))
    for z in (0.4, 1.5):
        bau.teil(quader(2.3, 0.12, 0.12), HOLZ, m=m @ M((0, 0, z)))
    for i in range(6):
        x = -0.9 + i * 0.36
        if i % 2 == 0:
            bau.teil(zylinder(0.03, 2.3, 5), "#8A5A34", m=m @ M((x, 0.05, 0.1), 0, -8))
            bau.teil(pyramide(0.06, 0.28, 4), "#B8BEC6", m=m @ M((x + 0.32, 0.05, 2.35), 0, -8))
        else:
            bau.teil(quader(0.07, 0.02, 1.2), "#B8BEC6", m=m @ M((x, 0.05, 0.4)))
            bau.teil(quader(0.28, 0.06, 0.05), GOLD, m=m @ M((x, 0.05, 1.6)))
            bau.teil(quader(0.05, 0.05, 0.25), HOLZ, m=m @ M((x, 0.05, 1.65)))
    for s in (-1, 1):
        schild = m @ M((s * 0.55, -0.12, 0.9), 0, 90)
        bau.teil(zylinder(0.36, 0.06, 12), "#3A5BB2" if s < 0 else "#B23A3A", m=schild)
        bau.teil(zylinder(0.1, 0.1, 8), GOLD, m=schild @ M((0, 0, 0.02)))


def zaun(bau, p1, p2):
    dx, dy = p2[0] - p1[0], p2[1] - p1[1]
    laenge = math.hypot(dx, dy)
    m = M((p1[0], p1[1], 0), math.degrees(math.atan2(dy, dx)))
    felder = max(int(laenge / 2.0), 1)
    for i in range(felder + 1):
        bau.teil(quader(0.14, 0.14, 1.2, 0.02), HOLZ, m=m @ M((i * laenge / felder, 0, 0)))
    for z in (0.45, 0.95):
        bau.teil(quader(laenge, 0.07, 0.12), "#8A5A34", m=m @ M((laenge / 2, 0, z)))


def heuballen(bau, m):
    bau.teil(quader(1.2, 0.8, 0.6, 0.08), STROH, m=m, schwankung=0.08)
    for x in (-0.3, 0.3):
        bau.teil(quader(0.04, 0.82, 0.62), "#9C7A3A", m=m @ M((x, 0, -0.005)))


# ---------------------------------------------------------------------------
# Die ganze Anlage
# ---------------------------------------------------------------------------
def umgebung(bau, seed=5):
    zufall = random.Random(seed)
    ecken = [(-MAUER_X, TOR_Y), (MAUER_X, TOR_Y), (MAUER_X, HINTEN_Y), (-MAUER_X, HINTEN_Y)]

    # Ringmauer (vorne vom Torhaus unterbrochen) mit Ecktürmen und Zwischentürmen
    mauer(bau, ecken[0], (-TOR_HW, TOR_Y))
    mauer(bau, (TOR_HW, TOR_Y), ecken[1])
    for a, b in zip(ecken[1:], ecken[2:] + ecken[:1]):
        mauer(bau, a, b)
    for x, y in ecken:
        rundturm(bau, x, y, 5.2, 15.0, 9.5)
    for x, y in ((-34.0, TOR_Y), (34.0, TOR_Y), (-MAUER_X, -22.0), (MAUER_X, -22.0), (0.0, HINTEN_Y)):
        rundturm(bau, x, y, 4.0, 12.5, 7.0, wimpel=False)
    torhaus(bau)
    innen = TOR_Y + MAUER_D / 2 + 0.9
    treppe(bau, -28.5, innen, -1)
    treppe(bau, 28.5, innen, 1)
    # Efeu außen an der Mauer
    for i, x in enumerate((-46.0, -20.0, 18.0, 44.0)):
        efeu(bau, M((x, TOR_Y - MAUER_D / 2, 0)), 2.6, 6.5, 400 + i, 0.6)
    for i, y in enumerate((-50.0, 5.0)):
        efeu(bau, M((-MAUER_X - MAUER_D / 2, y, 0), -90), 2.2, 5.5, 410 + i, 0.6)
        efeu(bau, M((MAUER_X + MAUER_D / 2, y, 0), 90), 2.2, 5.5, 420 + i, 0.6)

    # Straße vom Tor zur Burgtreppe und hinaus, Brunnenplatz in der Mitte
    strasse(bau, TOR_Y - 14.0, TOR_Y - 4.5)
    strasse(bau, TOR_Y + 3.5, -21.9)
    platz = (TOR_Y - 21.9) / 2 + 0.5
    brunnen(bau, 0.0, platz)
    for s in (-1, 1):
        for y in (TOR_Y + 9.0, -30.0):
            ritterstatue(bau, M((s * 5.4, y, 0), -90 * s))
        for y in (TOR_Y - 12.0, TOR_Y - 7.0, TOR_Y + 6.0, TOR_Y + 14.0, -36.0, -24.0):
            laterne(bau, M((s * 4.4, y, 0)))
        # Hecken und Blumenbeete entlang der Straße (sie enden am Rand des Brunnenplatzes)
        # (links bleibt der Zugang zum Marktplatz offen)
        if s > 0:
            for y0, y1 in ((TOR_Y + 4.5, platz - 5.8), (platz + 5.8, -22.5)):
                hecke(bau, M((s * 9.9, y0, 0), 90), y1 - y0, zufall)
            for y0, y1 in ((TOR_Y + 4.5, platz - 8.4), (platz + 8.4, -22.5)):
                blumenbeet(bau, M((s * 8.0, y0, 0), 90), y1 - y0, 1.5, zufall)
        for y in (platz - 8.0, platz + 8.0):
            zierbaum(bau, M((s * 8.0, y, 0)), zufall, "kugel" if y < platz else "kegel")

    # Links im Vorhof liegt der Marktplatz (eigenes Modell, siehe marktplatz.py: MARKT_ORT)

    # Übungsplatz rechts: Zaun, Strohpuppen, Zielscheiben, Waffenständer, Heu
    x0, x1, y0, y1 = 16.0, 52.0, -68.0, -32.0
    for a, b in (((x0, y0), (x1, y0)), ((x1, y0), (x1, y1)), ((x1, y1), (x0, y1)), ((x0, y1), (x0, y0 + 10.0))):
        zaun(bau, a, b)
    for i in range(4):
        strohpuppe(bau, M((24.0 + i * 5.0, -58.0, 0), zufall.uniform(-20, 20)))
    for i in range(3):
        zielscheibe(bau, M((24.0 + i * 7.0, -38.0, 0), 180))
    waffenstaender(bau, M((47.0, -50.0, 0), -90), zufall)
    for i in range(4):
        heuballen(bau, M((45.0 + (i % 2) * 1.4, -62.0 + (i // 2) * 1.0, 0.6 * (i // 3)), zufall.uniform(-10, 10)))

    # Bäume an den Seiten und hinter der Burg
    for x, y in ((-52.0, -8.0), (-50.0, 12.0), (-40.0, 25.0), (52.0, -10.0), (50.0, 14.0), (40.0, 25.0), (-20.0, 26.0), (22.0, 26.0)):
        baum(bau, M((x, y, 0), zufall.uniform(0, 360)), zufall)


def burganlage(seed=77):
    bau = Bau(seed)
    burg_teile(bau)
    umgebung(bau)
    return bau.fertig("Burganlage", ursprung=(0.0, 0.0))
