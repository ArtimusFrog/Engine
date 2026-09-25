"""Der Marktplatz im Vorhof der Burg: Fachwerkhäuser mit Läden, Markthalle, Marktstände mit
ihren Waren, Händlerzelte, gotischer Marktbrunnen, Maibaum mit Wimpelgirlanden, Wirtsgarten,
Gauklerbühne und viel Kleinkram.

Eigenes Modell (bauwerke/marktplatz). Lokale Koordinaten: Mitte des Platzes im Ursprung, die
Straße zur Burg liegt bei +X, die Häuser stehen bei -X an der Ringmauer. In der Burganlage liegt
die Mitte bei MARKT_ORT (Blender-Koordinaten der Burganlage).
"""

import math
import random

from burg import (BANNER, EISEN, GLAS, GOLD, HOLZ, KOPFSTEIN, MAUER, STEIN_HELL, ZIEGEL, Bau, M, bogen, bogen_hoehe,
                  drehkoerper, fiale, kreis, platte, prisma, pyramide, quader, rahmen, satteldach, zylinder, _blatt)
from burganlage import (ERDE, LICHT, MAUERFUSS, PFLASTER, STROH, WASSER, fass, heuballen, karren, kiste, laterne, sack)

MARKT_ORT = (-32.25, -46.5)
MARKT_B = 49.5          # Ausdehnung in X
MARKT_T = 48.0          # Ausdehnung in Y

BALKEN = "#4A3322"
SOCKELSTEIN = "#BFAE92"
FENSTER = "#3B4A5C"
FENSTER_LICHT = "#F2C77E"
ROT_DACH = "#A5533A"
STOFFE = ("#B23A3A", "#3A5BB2", "#2E6B46", "#D9772F", "#7A3FA0", "#C9A227")
HELL_STOFF = "#F1E6CF"


def kugel(r, ecken=6):
    """Kleine eckige Kugel (Obst, Kohl, Käse-Laib …) als Rotationskörper, Mitte im Ursprung."""
    return drehkoerper([(0.0, -r), (r * 0.72, -r * 0.7), (r, 0.0), (r * 0.72, r * 0.7), (0.0, r)], ecken)


def balken(bau, m, x1, z1, x2, z2, dicke=0.2, tiefe=0.09, farbe=BALKEN):
    """Holzbalken auf einer Wand (Wand bei y = 0, außen = -Y) von (x1, z1) nach (x2, z2)."""
    laenge = math.hypot(x2 - x1, z2 - z1)
    winkel = math.degrees(math.atan2(z2 - z1, x2 - x1))
    bau.teil(quader(laenge + dicke * 0.5, tiefe, dicke), farbe, m=m @ M(((x1 + x2) / 2, -tiefe / 2, (z1 + z2) / 2), 0, 0, -winkel) @ M((0, 0, -dicke / 2)))


def girlande(bau, a, b, durchhang, zufall, abstand=0.55, laternen=False):
    """Wimpelkette von a nach b (3D), hängt in der Mitte um `durchhang` durch."""
    a, b = [float(v) for v in a], [float(v) for v in b]
    laenge = math.dist(a, b)
    teile = max(int(laenge / abstand), 2)
    richtung = math.degrees(math.atan2(b[1] - a[1], b[0] - a[0]))
    punkte = []
    for i in range(teile + 1):
        t = i / teile
        punkte.append((a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t - durchhang * 4 * t * (1 - t)))
    for p, q in zip(punkte, punkte[1:]):
        stueck = math.dist(p, q)
        neig = math.degrees(math.atan2(q[2] - p[2], math.hypot(q[0] - p[0], q[1] - p[1])))
        mitte = ((p[0] + q[0]) / 2, (p[1] + q[1]) / 2, (p[2] + q[2]) / 2)
        bau.teil(quader(stueck, 0.025, 0.025), "#3A2A1E", m=M(mitte, richtung, 0, -neig) @ M((0, 0, -0.0125)))
    for i, p in enumerate(punkte[1:-1]):
        farbe = STOFFE[(i + zufall.randrange(2)) % len(STOFFE)] if i % 3 else HELL_STOFF
        if laternen and i % 4 == 2:
            bau.teil(zylinder(0.012, 0.18, 4), EISEN, m=M((p[0], p[1], p[2] - 0.18)))
            bau.teil(kugel(0.13, 6), LICHT, m=M((p[0], p[1], p[2] - 0.3)), leuchten=True)
            continue
        # Wimpel: flaches Dreieck, von beiden Seiten sichtbar (zwei Flächen)
        wimpel = ([(-0.17, 0, 0), (0.17, 0, 0), (0, 0, -0.36)], [[0, 1, 2], [2, 1, 0]])
        bau.teil(wimpel, farbe, m=M(p, richtung), schwankung=0.05)


# ---------------------------------------------------------------------------
# Fachwerkhaus
# ---------------------------------------------------------------------------
def fenster_haus(bau, m, x, z, zufall, breite=0.9, hoehe=1.15, laden_farbe="#2E6B46"):
    """Fenster im Fachwerk: Rahmen, Sprossenkreuz, Klappläden, Blumenkasten."""
    licht = zufall.random() < 0.3
    bau.teil(platte([(x - breite / 2, z), (x + breite / 2, z), (x + breite / 2, z + hoehe), (x - breite / 2, z + hoehe)], 0.03, -0.02),
             FENSTER_LICHT if licht else FENSTER, m=m, leuchten=licht)
    for dx, dz, w, h in ((0, -0.05, breite + 0.2, 0.1), (0, hoehe - 0.05, breite + 0.2, 0.1)):
        bau.teil(quader(w, 0.14, h), "#E9DFC8", m=m @ M((x + dx, -0.07, z + dz)))
    for s in (-1, 1):
        bau.teil(quader(0.1, 0.14, hoehe), "#E9DFC8", m=m @ M((x + s * (breite / 2 + 0.05), -0.07, z)))
    bau.teil(quader(0.05, 0.08, hoehe), "#E9DFC8", m=m @ M((x, -0.06, z)))
    bau.teil(quader(breite, 0.08, 0.05), "#E9DFC8", m=m @ M((x, -0.06, z + hoehe * 0.55)))
    # Klappläden (offen, flach an der Wand) mit Querleisten
    for s in (-1, 1):
        lx = x + s * (breite * 0.75 + 0.12)
        bau.teil(quader(breite * 0.5, 0.06, hoehe), laden_farbe, m=m @ M((lx, -0.05, z)))
        for zz in (0.25, hoehe - 0.3):
            bau.teil(quader(breite * 0.5, 0.08, 0.07), "#E9DFC8", m=m @ M((lx, -0.06, z + zz)))
    # Blumenkasten
    bau.teil(quader(breite + 0.2, 0.3, 0.22, 0.02), "#7A5534", m=m @ M((x, -0.2, z - 0.32)))
    for i in range(7):
        bx = x - breite / 2 + (i + 0.5) * breite / 7
        bau.teil(_blatt(0.1), "#4E8A34", m=m @ M((bx, -0.3, z - 0.1)))
        bau.teil(pyramide(0.07, 0.07, 5, 0), zufall.choice(("#E0524F", "#F2D544", "#E88AB4", "#F1EDE4")), m=m @ M((bx, -0.25 - zufall.uniform(0, 0.1), z - 0.05)))


def fachwerk_wand(bau, m, hw, z0, z1, fenster_x, zufall, laden_farbe):
    """Fachwerk auf einer Wand zwischen z0 und z1: Schwelle, Rähm, Ständer, Riegel, Streben und
    Andreaskreuze, Fenster an den Stellen `fenster_x`."""
    brust = 0.9
    balken(bau, m, -hw, z0 + 0.1, hw, z0 + 0.1, 0.22)
    balken(bau, m, -hw, z1 - 0.1, hw, z1 - 0.1, 0.22)
    felder = max(int(round(2 * hw / 1.25)), 2)
    xs = [-hw + i * 2 * hw / felder for i in range(felder + 1)]
    for x in xs:
        balken(bau, m, x, z0, x, z1, 0.2)
    for a, b in zip(xs, xs[1:]):
        mitte = (a + b) / 2
        mit_fenster = any(abs(mitte - fx) < (b - a) / 2 for fx in fenster_x)
        balken(bau, m, a, z0 + brust, b, z0 + brust, 0.16)
        if mit_fenster:
            fenster_haus(bau, m, mitte, z0 + brust + 0.12, zufall, min(b - a - 0.35, 0.95), min(z1 - z0 - brust - 0.45, 1.2), laden_farbe)
            # Andreaskreuz unter dem Fenster
            balken(bau, m, a + 0.1, z0 + 0.2, b - 0.1, z0 + brust - 0.08, 0.13)
            balken(bau, m, a + 0.1, z0 + brust - 0.08, b - 0.1, z0 + 0.2, 0.13)
        else:
            balken(bau, m, a, z0 + 0.2, b, z1 - 0.2, 0.16)


def ladenfront(bau, m, breite, ware, zufall):
    """Laden im Erdgeschoss: Rundbogen, dunkles Innere, Klappladen als Vordach und Theke mit Waren."""
    hw = breite / 2
    hs = 1.6
    bau.teil(rahmen(bogen(hw + 0.3, hs, hw + 0.3), bogen(hw, hs, hw), 0.25), STEIN_HELL, m=m)
    bau.teil(platte(bogen(hw, hs, hw), 0.03, -0.01), "#1E1814", m=m)
    # Klappladen oben als schräges Vordach, unten als Theke
    bau.teil(quader(breite + 0.3, 1.0, 0.08), "#8A5A34", m=m @ M((0, -0.5, hs + hw + 0.1), 0, 25))
    for s in (-1, 1):
        bau.teil(zylinder(0.02, 0.9, 4), EISEN, m=m @ M((s * hw, -0.85, hs + hw - 0.55), 0, 30))
    bau.teil(quader(breite + 0.2, 0.8, 0.9, 0.03), "#8A5A34", m=m @ M((0, -0.4, 0)))
    bau.teil(quader(breite + 0.3, 0.9, 0.07), "#B98A56", m=m @ M((0, -0.45, 0.9)))
    WAREN[ware](bau, m @ M((0, -0.45, 0.97)), breite, zufall)


def schild(bau, m, zeichen):
    """Zunftschild an einem Eisenausleger (ragt nach -Y aus der Wand)."""
    bau.teil(quader(0.06, 1.3, 0.06), EISEN, m=m @ M((0, -0.65, 0)))
    bau.teil(quader(0.04, 0.9, 0.04), EISEN, m=m @ M((0, -0.45, -0.45), 0, 45))
    for y in (-0.35, -1.05):
        bau.teil(zylinder(0.01, 0.2, 4), EISEN, m=m @ M((0, y, -0.2)))
    brett = m @ M((0, -0.7, -0.95), 90)
    bau.teil(quader(0.95, 0.07, 0.75, 0.02), "#6E4826", m=brett)
    for y in (-0.036, 0.036 + 0.02):
        z = zeichen
        seite = brett @ M((0, y, 0))
        if z == "brot":
            bau.teil(platte([(-0.3, 0.3), (-0.15, 0.5), (0.15, 0.5), (0.3, 0.3), (0.15, 0.2), (-0.15, 0.2)], 0.02), "#D9A55A", m=seite)
        elif z == "fisch":
            bau.teil(platte([(-0.3, 0.37), (0.0, 0.5), (0.2, 0.4), (0.33, 0.52), (0.33, 0.22), (0.2, 0.34), (0.0, 0.24)], 0.02), "#9FB6C4", m=seite)
        elif z == "krug":
            bau.teil(platte([(-0.15, 0.15), (0.15, 0.15), (0.2, 0.45), (0.1, 0.6), (-0.1, 0.6), (-0.2, 0.45)], 0.02), "#C87A4A", m=seite)
        elif z == "schwert":
            bau.teil(platte([(-0.03, 0.12), (0.03, 0.12), (0.03, 0.5), (0.0, 0.62), (-0.03, 0.5)], 0.02), "#D0D6DE", m=seite)
            bau.teil(platte([(-0.16, 0.18), (0.16, 0.18), (0.16, 0.22), (-0.16, 0.22)], 0.02), GOLD, m=seite)
        else:
            bau.teil(platte(kreis(0.18, 8, 0, 0.37), 0.02), GOLD, m=seite)


def fachwerkhaus(bau, m, breite, tiefe, putz, dach, laden_farbe, ware, zeichen, zufall, stockwerke=2):
    """Giebelhaus mit Steinsockel und Laden, auskragenden Fachwerk-Stockwerken, Giebel mit
    Fachwerk, Ziegeldach, Kamin, Gaube und Zunftschild (vorne = -Y)."""
    hw = breite / 2
    eg = 3.2
    bau.teil(quader(breite, tiefe, eg), SOCKELSTEIN, MAUER, m=m @ M((0, tiefe / 2, 0)))
    ladenfront(bau, m @ M((-hw * 0.28, 0, 0)), breite * 0.46, ware, zufall)
    # Haustür mit Stufe
    tx = hw * 0.62
    bau.teil(rahmen(bogen(0.65, 1.7, 0.65), bogen(0.5, 1.7, 0.5), 0.18), STEIN_HELL, m=m @ M((tx, 0, 0.2)))
    bau.teil(platte(bogen(0.5, 1.7, 0.5), 0.05, 0.0), "#5A3A22", m=m @ M((tx, 0, 0.2)))
    bau.teil(quader(1.5, 0.5, 0.2, 0.03), STEIN_HELL, m=m @ M((tx, -0.25, 0)))
    bau.teil(zylinder(0.05, 0.02, 6), GOLD, m=m @ M((tx + 0.3, -0.07, 1.1), 0, 90))
    z = eg
    vor = 0.0
    hoehe_og = 2.8
    for k in range(stockwerke):
        vor += 0.45
        bw = breite + 0.2 * (k + 1)
        # Knaggen (Stützbalken) unter der Auskragung
        for i in range(5):
            x = -bw / 2 + 0.2 + i * (bw - 0.4) / 4
            bau.teil(prisma([(0.0, 0.0), (-0.45, 0.0), (0.0, -0.5)], 0.16), BALKEN, m=m @ M((x - 0.08, -vor + 0.45, z)))
        bau.teil(quader(bw, tiefe + vor, hoehe_og), putz, m=m @ M((0, (tiefe - vor) / 2, z)), schwankung=0.01)
        wand = m @ M((0, -vor, 0))
        fachwerk_wand(bau, wand, bw / 2, z, z + hoehe_og, [-bw * 0.28, bw * 0.28] if bw > 6 else [0.0], zufall, laden_farbe)
        # Seitenwände: nur Ständer und Streben
        for s in (-1, 1):
            seite = m @ M((s * bw / 2, (tiefe - vor) / 2, 0), 90 * s)
            fachwerk_wand(bau, seite, (tiefe + vor) / 2, z, z + hoehe_og, [(tiefe + vor) * 0.2] if k == 0 else [], zufall, laden_farbe)
        z += hoehe_og
    # Giebel mit Fachwerk und Satteldach (First läuft nach hinten)
    bw = breite + 0.2 * stockwerke
    neigung = 52.0
    tan = math.tan(math.radians(neigung))
    first = z + (bw / 2) * tan
    bau.teil(platte([(-bw / 2, z), (bw / 2, z), (0, first)], tiefe + vor - 0.05, tiefe), putz, m=m)
    giebel = m @ M((0, -vor, 0))
    balken(bau, giebel, -bw / 2, z + 0.1, 0, first - 0.1, 0.22)
    balken(bau, giebel, bw / 2, z + 0.1, 0, first - 0.1, 0.22)
    balken(bau, giebel, 0, z, 0, first - 0.2, 0.2)
    zm = z + (first - z) * 0.42
    balken(bau, giebel, -bw / 2 + (zm - z) / tan, zm, bw / 2 - (zm - z) / tan, zm, 0.16)
    for s in (-1, 1):
        balken(bau, giebel, s * bw * 0.34, z, s * bw * 0.1, zm, 0.14)
    fenster_haus(bau, giebel, 0.0 - 0.0, zm + 0.25, zufall, 0.6, 0.8, laden_farbe) if (first - zm) > 1.6 else None
    satteldach(bau, m @ M((0, -vor - 0.5, z), 90), tiefe + vor + 0.9, bw / 2, 0.0, neigung, 0.45, 0.22, False, dach, "#6E3426")
    # Kamin und Gaube auf der Seite
    kx = bw * 0.22
    bau.teil(quader(0.7, 0.7, first - z + 1.4), "#9A5A44", MAUER, m=m @ M((kx, tiefe * 0.62, z)))
    bau.teil(quader(0.9, 0.9, 0.18, 0.03), STEIN_HELL, m=m @ M((kx, tiefe * 0.62, first + 1.4)))
    for s in (-1, 1):
        bau.teil(zylinder(0.14, 0.4, 6), "#8A5A44", m=m @ M((kx + s * 0.16, tiefe * 0.62, first + 1.55)))
    # Zunftschild am ersten Obergeschoss
    schild(bau, m @ M((-hw - 0.1, -0.45, eg + 1.9)), zeichen)
    # Kleinkram vor dem Haus
    if zufall.random() < 0.7:
        fass(bau, m @ M((hw - 0.3, -0.9, 0)))
    if zufall.random() < 0.6:
        kiste(bau, m @ M((-hw + 0.3, -1.6, 0), zufall.uniform(0, 30)), 0.6)


# ---------------------------------------------------------------------------
# Waren (auf einer Theke: Ursprung = Mitte der Oberkante, vorne = -Y)
# ---------------------------------------------------------------------------
def _korb(bau, m, r=0.3, farbe="#B98A56"):
    bau.teil(drehkoerper([(r * 0.8, 0.0), (r, 0.18), (r * 1.05, 0.22), (r * 0.95, 0.22), (r * 0.9, 0.05), (0.0, 0.05)], 8), farbe, m=m)


def ware_obst(bau, m, breite, zufall):
    for i, farbe in enumerate(("#E0524F", "#8FBF3A", "#F29A2E", "#F2D544")):
        x = -breite / 2 + (i + 0.5) * breite / 4
        kiste_m = m @ M((x, -0.05, 0.1), 0, -20)
        bau.teil(quader(breite / 4 - 0.08, 0.6, 0.22), "#A0703F", m=kiste_m @ M((0, 0, -0.1)))
        for k in range(14):
            bau.teil(kugel(0.085, 6), farbe, m=kiste_m @ M((zufall.uniform(-1, 1) * (breite / 8 - 0.1), zufall.uniform(-0.22, 0.22), 0.12 + (k > 8) * 0.08)),
                     schwankung=0.1)


def ware_gemuese(bau, m, breite, zufall):
    for i in range(3):
        x = -breite / 2 + (i + 0.5) * breite / 3
        _korb(bau, m @ M((x, 0, 0)), 0.3)
        for _ in range(5 if i == 0 else 10):
            if i == 0:
                bau.teil(kugel(0.17, 7), "#8FBF3A", m=m @ M((x + zufall.uniform(-0.12, 0.12), zufall.uniform(-0.12, 0.12), 0.26)), schwankung=0.08)
            elif i == 1:
                bau.teil(pyramide(0.035, 0.3, 5, 0), "#E88A2E", m=m @ M((x + zufall.uniform(-0.15, 0.15), zufall.uniform(-0.1, 0.1), 0.2), 0, 80, zufall.uniform(0, 360)))
            else:
                bau.teil(kugel(0.08, 5), "#C4A060", m=m @ M((x + zufall.uniform(-0.15, 0.15), zufall.uniform(-0.12, 0.12), 0.22)), schwankung=0.1)


def ware_brot(bau, m, breite, zufall):
    for i in range(7):
        x = -breite / 2 + 0.3 + i * (breite - 0.6) / 6
        if i % 2:
            bau.teil(drehkoerper([(0.0, 0.0), (0.16, 0.02), (0.15, 0.1), (0.0, 0.16)], 7), "#C98A42", m=m @ M((x, -0.1, 0)), schwankung=0.06)
        else:
            bau.teil(kugel(0.1, 6), "#D9A55A", m=m @ M((x, 0.12, 0.08), zufall.uniform(0, 90), 0, 0, (2.2, 1.0, 0.8)), schwankung=0.06)
    _korb(bau, m @ M((breite / 2 - 0.35, 0.2, 0)), 0.28)
    # Brezeln an einer Stange
    bau.teil(zylinder(0.02, breite * 0.8, 4), HOLZ, m=m @ M((-breite * 0.4, 0.35, 1.25), 0, 0, 90))
    for i in range(5):
        x = -breite * 0.35 + i * breite * 0.175
        bau.teil(rahmen(kreis(0.12, 8, 0, 0), kreis(0.07, 8, 0, 0), 0.04), "#A8642C", m=m @ M((x, 0.35, 1.05)))


def ware_fleisch(bau, m, breite, zufall):
    bau.teil(quader(0.8, 0.5, 0.12), "#8A5A34", m=m @ M((-breite * 0.25, 0, 0)))
    bau.teil(quader(0.4, 0.02, 0.18), "#B8BEC6", m=m @ M((-breite * 0.25, 0, 0.12), 20))
    for i in range(8):
        bau.teil(quader(0.3, 0.22, 0.1, 0.03), zufall.choice(("#B8453F", "#C9605A", "#E0A090")), m=m @ M((0.05 + (i % 4) * 0.32, -0.15 + (i // 4) * 0.3, 0), zufall.uniform(-20, 20)))
    # Schinken und Würste hängen unter dem Vordach
    bau.teil(zylinder(0.025, breite, 4), HOLZ, m=m @ M((-breite / 2, 0.3, 1.3), 0, 0, 90))
    for i in range(4):
        x = -breite / 2 + 0.35 + i * (breite - 0.7) / 3
        if i % 2 == 0:
            bau.teil(drehkoerper([(0.0, 0.0), (0.12, 0.08), (0.14, 0.3), (0.06, 0.45), (0.0, 0.48)], 7), "#9A4A34", m=m @ M((x, 0.3, 0.72)))
        else:
            for k in range(4):
                bau.teil(zylinder(0.035, 0.18, 5), "#8A3A2A", m=m @ M((x + (k % 2) * 0.05, 0.3, 1.25 - (k + 1) * 0.19)))


def ware_fisch(bau, m, breite, zufall):
    bau.teil(quader(breite - 0.1, 0.8, 0.1), "#DCEBF2", m=m)
    for i in range(10):
        x = -breite / 2 + 0.3 + (i % 5) * (breite - 0.6) / 4
        y = -0.18 if i < 5 else 0.18
        fisch = m @ M((x, y, 0.13), zufall.uniform(-20, 20) + 90)
        bau.teil(kugel(0.07, 5), zufall.choice(("#9FB6C4", "#B8C4CC", "#C9A08A")), m=fisch @ M((0, 0, 0), 0, 0, 0, (3.0, 1.0, 0.6)))
        bau.teil(platte([(0.0, 0.0), (0.12, 0.07), (0.12, -0.07)], 0.02), "#8FA4B2", m=fisch @ M((0.2, 0.01, 0), 0, -90))
    fass(bau, m @ M((breite / 2 + 0.5, -0.4, -0.97)))


def ware_stoff(bau, m, breite, zufall):
    for i in range(8):
        x = -breite / 2 + 0.25 + (i % 4) * (breite - 0.5) / 3
        z = 0.1 + (i // 4) * 0.2
        bau.teil(zylinder(0.1, 0.8, 8), STOFFE[i % len(STOFFE)], m=m @ M((x, -0.4 + (i // 4) * 0.1, z), 0, 90, 0), schwankung=0.05)
    # Teppich hängt an der Rückwand
    for i in range(6):
        bau.teil(quader(0.2 * breite / 1.2, 0.03, 1.2), STOFFE[(i * 2) % len(STOFFE)] if i % 2 else HELL_STOFF,
                 m=m @ M((-breite * 0.3 + i * 0.2 * breite / 1.2, 0.45, 0.2)))


def ware_toepfer(bau, m, breite, zufall):
    profile = (
        [(0.08, 0.0), (0.16, 0.12), (0.17, 0.25), (0.1, 0.38), (0.08, 0.45), (0.1, 0.48)],
        [(0.1, 0.0), (0.2, 0.1), (0.22, 0.18), (0.2, 0.24)],
        [(0.06, 0.0), (0.12, 0.2), (0.1, 0.4), (0.05, 0.55), (0.08, 0.62)],
    )
    for i in range(9):
        x = -breite / 2 + 0.25 + i * (breite - 0.5) / 8
        bau.teil(drehkoerper(profile[i % 3], 8), zufall.choice(("#C87A4A", "#B0603A", "#3A6BB2", "#E8D8B8")), m=m @ M((x, zufall.uniform(-0.2, 0.2), 0)), schwankung=0.06)


def ware_waffen(bau, m, breite, zufall):
    for i in range(3):
        x = -breite * 0.35 + i * breite * 0.3
        bau.teil(quader(0.08, 0.9, 0.02), "#D0D6DE", m=m @ M((x, 0.0, 0.02)))
        bau.teil(quader(0.3, 0.06, 0.05), GOLD, m=m @ M((x, 0.45, 0.02)))
        bau.teil(quader(0.06, 0.2, 0.05), HOLZ, m=m @ M((x, 0.58, 0.02)))
    for s in (-1, 1):
        helm = m @ M((s * breite * 0.42, 0.25, 0))
        bau.teil(drehkoerper([(0.16, 0.0), (0.17, 0.12), (0.14, 0.24), (0.0, 0.3)], 8), "#9AA2AC", m=helm)
        bau.teil(quader(0.04, 0.05, 0.16), "#9AA2AC", m=helm @ M((0, -0.16, 0.02)))
    for i, farbe in enumerate(("#3A5BB2", "#B23A3A")):
        schild_m = m @ M((-0.4 + i * 0.8, 0.45, 0.55), 0, 80)
        bau.teil(zylinder(0.36, 0.06, 12), farbe, m=schild_m)
        bau.teil(zylinder(0.1, 0.1, 8), GOLD, m=schild_m @ M((0, 0, -0.03)))
    # Amboss vor dem Stand
    amboss = m @ M((breite / 2 + 0.6, -0.9, -0.97))
    bau.teil(quader(0.4, 0.4, 0.5), "#6E4826", m=amboss)
    bau.teil(quader(0.25, 0.2, 0.2), "#3A3A40", m=amboss @ M((0, 0, 0.5)))
    bau.teil(quader(0.6, 0.26, 0.14), "#3A3A40", m=amboss @ M((0, 0, 0.7)))
    bau.teil(pyramide(0.12, 0.3, 4), "#3A3A40", m=amboss @ M((0.3, 0, 0.77), 0, 0, 90))


def ware_traenke(bau, m, breite, zufall):
    for stufe in range(3):
        bau.teil(quader(breite - 0.2, 0.25, 0.04 + stufe * 0.22), "#6E4826", m=m @ M((0, -0.25 + stufe * 0.25, 0)))
        for i in range(6):
            x = -breite / 2 + 0.3 + i * (breite - 0.6) / 5
            farbe = zufall.choice(("#E0524F", "#6AD1E8", "#8FE06A", "#B07CFF", "#F2D544"))
            flasche = m @ M((x, -0.25 + stufe * 0.25, 0.04 + stufe * 0.22))
            bau.teil(drehkoerper([(0.0, 0.0), (0.07, 0.01), (0.075, 0.1), (0.03, 0.17), (0.025, 0.24), (0.0, 0.24)], 6), farbe, m=flasche, leuchten=True)
            bau.teil(zylinder(0.028, 0.04, 5), "#8A5A34", m=flasche @ M((0, 0, 0.24)))
    kessel = m @ M((breite / 2 + 0.6, -0.8, -0.97))
    for k in range(3):
        bau.teil(zylinder(0.03, 0.9, 4), EISEN, m=kessel @ M((0, 0, 0), k * 120) @ M((0.35, 0, 0), 0, 0, -15))
    bau.teil(drehkoerper([(0.0, 0.2), (0.3, 0.25), (0.36, 0.45), (0.3, 0.62)], 10), "#2A2A30", m=kessel)
    bau.teil(zylinder(0.3, 0.03, 10), "#7CF08A", m=kessel @ M((0, 0, 0.58)), leuchten=True)


def ware_kaese(bau, m, breite, zufall):
    for i in range(7):
        x = -breite / 2 + 0.3 + (i % 4) * (breite - 0.6) / 3
        z = (i // 4) * 0.16
        bau.teil(zylinder(0.22, 0.15, 10), zufall.choice(("#F2C94C", "#E8B64A", "#F5DD8A")), m=m @ M((x, -0.15 + (i // 4) * 0.3, z)))
    bau.teil(prisma([(0.0, 0.0), (0.3, 0.0), (0.0, 0.12)], 0.15), "#F2C94C", m=m @ M((breite / 2 - 0.4, 0.2, 0)))


def ware_gewuerze(bau, m, breite, zufall):
    farben = ("#C0392B", "#E6A627", "#8E5A2E", "#D35400", "#6E8B3D", "#F1C40F")
    for i in range(6):
        x = -breite / 2 + 0.3 + (i % 3) * (breite - 0.6) / 2
        y = -0.18 if i < 3 else 0.2
        schale = m @ M((x, y, 0))
        bau.teil(drehkoerper([(0.0, 0.0), (0.2, 0.02), (0.24, 0.12), (0.22, 0.12)], 8), "#B98A56", m=schale)
        bau.teil(pyramide(0.2, 0.18, 8, 0), farben[i], m=schale @ M((0, 0, 0.08)))
    # Waage
    waage = m @ M((0, 0.35, 0))
    bau.teil(zylinder(0.03, 0.55, 5), GOLD, m=waage)
    bau.teil(quader(0.7, 0.03, 0.03), GOLD, m=waage @ M((0, 0, 0.55)))
    for s in (-1, 1):
        bau.teil(zylinder(0.12, 0.02, 8), GOLD, m=waage @ M((s * 0.33, 0, 0.25)))
        bau.teil(zylinder(0.005, 0.3, 3), GOLD, m=waage @ M((s * 0.33, 0, 0.27)))


def ware_blumen(bau, m, breite, zufall):
    for i in range(6):
        x = -breite / 2 + 0.3 + (i % 3) * (breite - 0.6) / 2
        y = -0.15 if i < 3 else 0.2
        eimer = m @ M((x, y, 0.2 * (i >= 3)))
        bau.teil(zylinder(0.16, 0.3, 8, 0.19), "#6E7A86", m=eimer)
        farbe = zufall.choice(("#E0524F", "#F2D544", "#B07CFF", "#F1EDE4", "#E88AB4"))
        for _ in range(7):
            dx, dy = zufall.uniform(-0.12, 0.12), zufall.uniform(-0.12, 0.12)
            h = zufall.uniform(0.35, 0.55)
            bau.teil(zylinder(0.01, h, 3), "#4E8A34", m=eimer @ M((dx, dy, 0.25)))
            bau.teil(kugel(0.05, 5), farbe, m=eimer @ M((dx, dy, 0.25 + h)))


def ware_schmuck(bau, m, breite, zufall):
    bau.teil(quader(breite - 0.2, 0.8, 0.03), "#5A2A6A", m=m)
    for i in range(12):
        x, y = zufall.uniform(-breite / 2 + 0.2, breite / 2 - 0.2), zufall.uniform(-0.3, 0.3)
        if i % 3 == 0:
            bau.teil(rahmen(kreis(0.05, 8), kreis(0.035, 8), 0.015), GOLD, m=m @ M((x, y, 0.04), 0, 90))
        else:
            bau.teil(pyramide(0.05, 0.08, 4), zufall.choice(("#E0524F", "#6AD1E8", "#8FE06A", "#B07CFF")), m=m @ M((x, y, 0.03)), leuchten=True)


WAREN = {
    "obst": ware_obst, "gemuese": ware_gemuese, "brot": ware_brot, "fleisch": ware_fleisch, "fisch": ware_fisch,
    "stoff": ware_stoff, "toepfer": ware_toepfer, "waffen": ware_waffen, "traenke": ware_traenke, "kaese": ware_kaese,
    "gewuerze": ware_gewuerze, "blumen": ware_blumen, "schmuck": ware_schmuck,
}


# ---------------------------------------------------------------------------
# Stände, Zelte, Halle
# ---------------------------------------------------------------------------
def stand(bau, m, ware, stoff, zufall, breite=3.0):
    """Marktstand: Holzgerüst mit Rückwand und Seitentüchern, Theke mit Waren, gestreifte
    Markise mit Zackenrand, Preistafel, Laterne; Kisten und Körbe drumherum (Kunden bei -Y)."""
    hw = breite / 2
    for sx in (-1, 1):
        for sy, h in ((-1, 2.35), (1, 2.9)):
            bau.teil(quader(0.14, 0.14, h), HOLZ, m=m @ M((sx * hw, sy * 0.95, 0)))
        bau.teil(platte([(-0.95, 0.9), (0.95, 0.9), (0.95, 2.2), (-0.95, 2.4)], 0.03), stoff, m=m @ M((sx * hw, 0, 0), 90) @ M((0, -0.01 * sx, 0)))
    bau.teil(quader(breite, 0.05, 1.7), "#8A5A34", m=m @ M((0, 0.95, 0.9)))
    bau.teil(quader(breite - 0.2, 0.9, 0.95, 0.03), "#8A5A34", m=m @ M((0, -0.5, 0)))
    bau.teil(quader(breite, 1.0, 0.07, 0.02), "#B98A56", m=m @ M((0, -0.52, 0.95)))
    for i in range(4):
        bau.teil(quader(0.03, 0.02, 0.85), "#6E4826", m=m @ M((-hw + 0.3 + i * (breite - 0.6) / 3, -0.96, 0.05)))
    WAREN[ware](bau, m @ M((0, -0.5, 1.02)), breite - 0.2, zufall)
    # Markise
    neig = math.degrees(math.atan2(0.62, 2.3))
    streifen = 6
    for i in range(streifen):
        x = -hw - 0.15 + (i + 0.5) * (breite + 0.3) / streifen
        farbe = stoff if i % 2 == 0 else HELL_STOFF
        bau.teil(quader((breite + 0.3) / streifen, 2.45, 0.05), farbe, m=m @ M((x, 0, 2.64), 0, neig))
        for k in range(2):
            zx = x - (breite + 0.3) / streifen / 4 + k * (breite + 0.3) / streifen / 2
            bau.teil(([(-0.13, 0, 0), (0.13, 0, 0), (0, 0, -0.26)], [[0, 1, 2], [2, 1, 0]]), farbe, m=m @ M((zx, -1.22, 2.3)))
    # Preistafel und Laterne
    bau.teil(quader(0.6, 0.04, 0.4, 0.02), "#2A2A2E", m=m @ M((hw - 0.2, -1.02, 0.35), 0, 8))
    bau.teil(quader(0.4, 0.05, 0.05), HELL_STOFF, m=m @ M((hw - 0.2, -1.05, 0.55)))
    bau.teil(zylinder(0.012, 0.3, 4), EISEN, m=m @ M((-hw + 0.3, -0.8, 1.95)))
    bau.teil(quader(0.18, 0.18, 0.26), LICHT, m=m @ M((-hw + 0.3, -0.8, 1.7)), leuchten=True)
    bau.teil(pyramide(0.16, 0.12, 4), EISEN, m=m @ M((-hw + 0.3, -0.8, 1.96)))
    # Kisten und Körbe neben dem Stand
    kiste(bau, m @ M((hw + 0.55, 0.3, 0), zufall.uniform(-20, 20)), 0.6)
    if zufall.random() < 0.6:
        kiste(bau, m @ M((hw + 0.55, 0.3, 0.6), zufall.uniform(-20, 20)), 0.45)
    if zufall.random() < 0.6:
        sack(bau, m @ M((-hw - 0.5, -0.3, 0)))
    if zufall.random() < 0.5:
        _korb(bau, m @ M((-hw - 0.5, 0.5, 0)), 0.3)


def zelt(bau, m, farbe, zufall, r=3.2, hoehe=4.6):
    """Rundes Händlerzelt mit gestreiftem Dach, Zackenrand, Wimpel; drinnen Teppiche, Kissen,
    niedriger Tisch mit Teekanne."""
    ecken = 12
    wand = 2.2
    for k in range(ecken):
        w1, w2 = math.tau * k / ecken, math.tau * (k + 1) / ecken
        p1 = (math.cos(w1) * r, math.sin(w1) * r)
        p2 = (math.cos(w2) * r, math.sin(w2) * r)
        f = farbe if k % 2 == 0 else HELL_STOFF
        punkte = [(0, 0, hoehe), (p1[0], p1[1], wand), (p2[0], p2[1], wand)]
        bau.teil((punkte, [[0, 1, 2], [2, 1, 0]]), f, m=m, schwankung=0.02)
        mitte = ((p1[0] + p2[0]) / 2, (p1[1] + p2[1]) / 2)
        bau.teil(([(p1[0], p1[1], wand), (p2[0], p2[1], wand), (mitte[0] * 1.0, mitte[1] * 1.0, wand - 0.4)], [[0, 1, 2], [2, 1, 0]]), f, m=m)
        bau.teil(zylinder(0.07, wand, 6), HOLZ, m=m @ M((p1[0], p1[1], 0)))
        # hintere Hälfte mit Stoffwand, vorne offen
        if math.sin(w1) > 0.2 and math.sin(w2) > 0.2:
            bau.teil(([(p1[0], p1[1], 0), (p2[0], p2[1], 0), (p2[0], p2[1], wand), (p1[0], p1[1], wand)], [[0, 1, 2, 3], [3, 2, 1, 0]]), HELL_STOFF if k % 2 else farbe, m=m)
    bau.teil(zylinder(0.1, hoehe + 0.6, 6), HOLZ, m=m)
    bau.teil(zylinder(0.03, 1.2, 4), EISEN, m=m @ M((0, 0, hoehe)))
    bau.teil(platte([(0, 0), (1.2, -0.25), (0, -0.55)], 0.03, 0.015), farbe, m=m @ M((0.03, 0, hoehe + 1.15)))
    bau.teil(kugel(0.12, 6), GOLD, m=m @ M((0, 0, hoehe + 1.25)))
    # Teppiche, Kissen, Tisch
    for i, (dx, dy, w) in enumerate(((0.0, 0.8, 20), (-0.8, -0.6, -15), (1.0, -0.5, 40))):
        teppich = m @ M((dx, dy, 0.01), w)
        bau.teil(quader(2.2, 1.4, 0.03), STOFFE[(i + 3) % len(STOFFE)], m=teppich)
        bau.teil(quader(1.8, 1.0, 0.035), STOFFE[(i + 1) % len(STOFFE)], m=teppich)
        bau.teil(quader(1.2, 0.5, 0.04), GOLD, m=teppich)
    for k in range(5):
        w = math.tau * k / 5
        bau.teil(quader(0.55, 0.55, 0.22, 0.08), STOFFE[k % len(STOFFE)], m=m @ M((math.cos(w) * 1.3, math.sin(w) * 1.3 + 0.4, 0.03), math.degrees(w)))
    bau.teil(zylinder(0.5, 0.35, 8), "#8A5A34", m=m @ M((0, 0.4, 0)))
    bau.teil(drehkoerper([(0.0, 0.0), (0.12, 0.02), (0.13, 0.12), (0.06, 0.2), (0.07, 0.24), (0.0, 0.26)], 8), GOLD, m=m @ M((0, 0.4, 0.35)))
    for k in range(3):
        bau.teil(zylinder(0.04, 0.07, 6), GOLD, m=m @ M((0.25 * math.cos(k * 2.1), 0.4 + 0.25 * math.sin(k * 2.1), 0.35)))
    del zufall


def markthalle(bau, m, laenge=16.0, tiefe=6.5, zufall=None):
    """Offene Markthalle: Steinpfeiler, Holzbalken, Ziegeldach; darunter lange Tische mit Käse,
    Brot und Körben (lokal: Halle entlang X, offen nach -Y)."""
    hl = laenge / 2
    hoehe = 3.6
    felder = 5
    for i in range(felder + 1):
        x = -hl + i * laenge / felder
        for y in (-tiefe / 2, tiefe / 2):
            bau.teil(quader(0.6, 0.6, 0.4, 0.04), MAUERFUSS, m=m @ M((x, y, 0)))
            bau.teil(quader(0.42, 0.42, hoehe - 0.4), "#8A5A34" if y > 0 else SOCKELSTEIN, MAUER if y < 0 else 0, m=m @ M((x, y, 0.4)))
            bau.teil(quader(0.7, 0.7, 0.2, 0.03), BALKEN, m=m @ M((x, y, hoehe)))
        bau.teil(quader(0.28, tiefe + 0.6, 0.35), BALKEN, m=m @ M((x, 0, hoehe + 0.2)))
    for y in (-tiefe / 2, tiefe / 2):
        bau.teil(quader(laenge + 0.8, 0.3, 0.4), BALKEN, m=m @ M((0, y, hoehe + 0.2)))
        for i in range(felder):
            x = -hl + (i + 0.5) * laenge / felder
            for s in (-1, 1):
                balken(bau, m @ M((0, y, 0)), x + s * laenge / felder / 2, hoehe - 0.9, x + s * (laenge / felder / 2 - 0.9), hoehe + 0.1, 0.16, 0.2)
    # Rückwand aus Brettern
    bau.teil(quader(laenge, 0.12, hoehe * 0.55), "#7A5534", m=m @ M((0, tiefe / 2 + 0.15, 0)))
    satteldach(bau, m @ M((-hl - 0.4, 0, hoehe + 0.55)), laenge + 0.8, tiefe / 2 + 0.3, 0.0, 38.0, 0.6, 0.2, False, ROT_DACH, "#6E3426")
    for s in (-1, 1):
        bau.teil(platte([(-(tiefe / 2 + 0.3), 0), (tiefe / 2 + 0.3, 0), (0, (tiefe / 2 + 0.3) * math.tan(math.radians(38)))], 0.15),
                 "#8A5A34", m=m @ M((s * (hl + 0.2), 0, hoehe + 0.55), 90 * s))
    # Tische mit Waren
    for i, ware in enumerate(("kaese", "brot", "gemuese", "toepfer", "kaese")):
        x = -hl + (i + 0.5) * laenge / felder
        tisch = m @ M((x, 0.8, 0))
        bau.teil(quader(laenge / felder - 0.6, 1.2, 0.08, 0.02), "#B98A56", m=tisch @ M((0, 0, 0.85)))
        for sx in (-1, 1):
            for sy in (-1, 1):
                bau.teil(quader(0.08, 0.08, 0.85), HOLZ, m=tisch @ M((sx * (laenge / felder / 2 - 0.45), sy * 0.5, 0)))
        WAREN[ware](bau, tisch @ M((0, 0, 0.93)), laenge / felder - 0.8, zufall)
    for i in range(6):
        (fass if i % 2 else kiste)(bau, m @ M((-hl + 1.0 + i * (laenge - 2) / 5, tiefe / 2 - 0.6, 0), zufall.uniform(0, 40)))


# ---------------------------------------------------------------------------
# Platzmitte und Kleinkram
# ---------------------------------------------------------------------------
def marktbrunnen(bau, m):
    """Gotischer Marktbrunnen: rundes Becken, Pfeiler mit vier Wasserspeiern, Fialenspitze."""
    ecken = 12
    r = 3.2
    seite = r * math.cos(math.pi / ecken)
    kante = 2 * r * math.sin(math.pi / ecken)
    for k in range(ecken):
        w = 360 * k / ecken + 180 / ecken
        bau.teil(quader(kante + 0.08, 0.45, 0.85), STEIN_HELL, MAUER, m=m @ M((0, 0, 0), w) @ M((0, -seite + 0.22, 0)))
        bau.teil(quader(kante + 0.3, 0.7, 0.14, 0.03), STEIN_HELL, m=m @ M((0, 0, 0), w) @ M((0, -seite + 0.22, 0.85)))
    bau.teil(zylinder(r - 0.3, 0.65, ecken), WASSER, m=m)
    bau.teil(quader(1.3, 1.3, 3.2), STEIN_HELL, MAUER, m=m)
    bau.teil(quader(1.6, 1.6, 0.25, 0.04), STEIN_HELL, m=m @ M((0, 0, 3.2)))
    for k in range(4):
        speier = m @ M((0, 0, 0), 90 * k) @ M((0, -0.65, 1.9))
        bau.teil(drehkoerper([(0.0, 0.0), (0.16, 0.05), (0.14, 0.3), (0.0, 0.35)], 6), GOLD, m=speier @ M((0, 0, 0), 0, 90))
        for i in range(4):
            t0, t1 = i / 4, (i + 1) / 4
            p0 = (0.3 + t0 * 1.1, -(t0 ** 2) * 1.2)
            p1 = (0.3 + t1 * 1.1, -(t1 ** 2) * 1.2)
            laenge = math.hypot(p1[0] - p0[0], p1[1] - p0[1])
            neig = math.degrees(math.atan2(p1[1] - p0[1], p1[0] - p0[0]))
            bau.teil(quader(laenge + 0.04, 0.06, 0.06), "#A8DDF0", m=speier @ M((0, 0, 0), -90) @ M((p0[0], 0, p0[1]), 0, 0, -neig) @ M((laenge / 2, 0, 0)))
    fiale(bau, m @ M((0, 0, 3.45)), 1.2, 5.5, schaft=0.5)
    for sx, sy in ((1, 1), (-1, 1), (-1, -1), (1, -1)):
        fiale(bau, m @ M((sx * 0.62, sy * 0.62, 3.45)), 0.35, 2.4, schaft=0.5, krabben=False)


def maibaum(bau, m, hoehe=16.0):
    """Maibaum: blau-weiß gebändert, Kranz mit Bändern, Zunftschilder, grüne Spitze."""
    teile = 16
    for i in range(teile):
        z = i * hoehe / teile
        r0 = 0.2 - 0.1 * i / teile
        r1 = 0.2 - 0.1 * (i + 1) / teile
        bau.teil(zylinder(r0, hoehe / teile, 8, r1), "#3A6BB2" if i % 2 else "#F1EDE4", m=m @ M((0, 0, z)))
    bau.teil(quader(0.8, 0.8, 0.5, 0.05), STEIN_HELL, m=m)
    # Kranz und Bänder
    kranz = hoehe * 0.78
    for k in range(16):
        w = math.tau * k / 16
        bau.teil(kugel(0.22, 5), "#3F7A2E", m=m @ M((math.cos(w) * 0.9, math.sin(w) * 0.9, kranz)), schwankung=0.08)
        if k % 2 == 0:
            bau.teil(quader(0.08, 0.02, 2.2), STOFFE[k // 2 % len(STOFFE)], m=m @ M((math.cos(w) * 0.95, math.sin(w) * 0.95, kranz - 2.3), math.degrees(w) + 90))
    for k in range(4):
        bau.teil(zylinder(0.03, 0.9, 4), HOLZ, m=m @ M((0, 0, kranz), 45 + 90 * k) @ M((0.0, 0, 0), 0, 0, 90))
    # Zunftschilder an Querstangen
    for i, z in enumerate((hoehe * 0.35, hoehe * 0.5, hoehe * 0.65)):
        for s in (-1, 1):
            arm = m @ M((0, 0, z), 90 * i)
            bau.teil(quader(1.6, 0.05, 0.05), HOLZ, m=arm)
            tafel = arm @ M((s * 0.75, 0, -0.55))
            bau.teil(quader(0.55, 0.04, 0.5, 0.02), "#3A6BB2", m=tafel)
            bau.teil(quader(0.45, 0.06, 0.4), "#F1EDE4", m=tafel @ M((0, 0, 0.05)))
            for y in (-0.035, 0.035):
                bau.teil(platte(kreis(0.13, 6, 0, 0.25), 0.02, y + 0.01 * (y > 0)), STOFFE[(i * 2 + (s > 0)) % len(STOFFE)], m=tafel)
    bau.teil(zylinder(0.6, 1.8, 7, 0.0), "#3F7A2E", m=m @ M((0, 0, hoehe - 0.2)), schwankung=0.08)
    bau.teil(zylinder(0.45, 1.2, 7, 0.0), "#4E8A34", m=m @ M((0, 0, hoehe + 0.7)), schwankung=0.08)


def tisch_mit_baenken(bau, m, zufall):
    bau.teil(quader(2.2, 0.9, 0.07, 0.02), "#B98A56", m=m @ M((0, 0, 0.78)))
    for sx in (-1, 1):
        bau.teil(quader(0.08, 0.8, 0.78), HOLZ, m=m @ M((sx * 0.9, 0, 0)))
        for sy in (-1, 1):
            pass
    for sy in (-1, 1):
        bau.teil(quader(2.2, 0.32, 0.06), "#B98A56", m=m @ M((0, sy * 0.85, 0.45)))
        for sx in (-1, 1):
            bau.teil(quader(0.07, 0.28, 0.45), HOLZ, m=m @ M((sx * 0.9, sy * 0.85, 0)))
    for i in range(zufall.randint(2, 4)):
        x, y = zufall.uniform(-0.9, 0.9), zufall.uniform(-0.3, 0.3)
        bau.teil(zylinder(0.07, 0.18, 7), "#8A6A4A", m=m @ M((x, y, 0.85)))
        bau.teil(zylinder(0.06, 0.03, 7), "#F4E8C8", m=m @ M((x, y, 1.03)))
    bau.teil(zylinder(0.16, 0.03, 8), "#D8D0C4", m=m @ M((zufall.uniform(-0.6, 0.6), 0.1, 0.85)))
    bau.teil(zylinder(0.03, 0.12, 5), "#F4ECD8", m=m @ M((0, 0, 0.85)))
    bau.teil(pyramide(0.02, 0.05, 4), "#FFC56B", m=m @ M((0, 0, 0.97)), leuchten=True)


def ausschank(bau, m, zufall):
    """Ausschank: Theke mit Zapffass und Krügen unter einem kleinen Ziegeldach, Schild mit Krug."""
    bau.teil(quader(3.0, 0.9, 1.05, 0.03), "#8A5A34", m=m)
    bau.teil(quader(3.2, 1.0, 0.08, 0.02), "#B98A56", m=m @ M((0, 0, 1.05)))
    for sx in (-1, 1):
        for sy in (-1, 1):
            bau.teil(quader(0.14, 0.14, 2.7), HOLZ, m=m @ M((sx * 1.55, sy * 0.55 + 0.2, 0)))
    satteldach(bau, m @ M((-1.8, 0.2, 2.7)), 3.6, 0.9, 0.0, 32.0, 0.35, 0.12, False, ROT_DACH, "#6E3426")
    fassliegend = m @ M((-0.9, 0.1, 1.55), 0, 0, 90) @ M((0, 0, -0.5))
    fass(bau, fassliegend)
    bau.teil(zylinder(0.04, 0.2, 5), GOLD, m=m @ M((-1.42, 0.1, 1.5), 0, 0, -90))
    for i in range(7):
        krug = m @ M((0.0 + i * 0.2, -0.2 + (i % 2) * 0.2, 1.13))
        bau.teil(zylinder(0.07, 0.2, 7), "#8A6A4A", m=krug)
        bau.teil(zylinder(0.065, 0.04, 7), "#F4E8C8", m=krug @ M((0, 0, 0.19)))
        bau.teil(quader(0.03, 0.05, 0.12), "#8A6A4A", m=krug @ M((0.08, 0, 0.04)))
    schild(bau, m @ M((1.7, -0.3, 2.5), -90), "krug")
    for i in range(2):
        fass(bau, m @ M((1.9 + i * 0.1, 0.9, 0), zufall.uniform(0, 90)))


def schirm(bau, m, farbe):
    bau.teil(zylinder(0.04, 2.6, 5), HOLZ, m=m)
    bau.teil(zylinder(1.4, 0.35, 8, 0.05), farbe, m=m @ M((0, 0, 2.25)))
    bau.teil(quader(0.4, 0.4, 0.06), STEIN_HELL, m=m)


def fassgestell(bau, m):
    for s in (-1, 1):
        bau.teil(quader(2.4, 0.1, 0.1), HOLZ, m=m @ M((0, s * 0.35, 0.2)))
        for x in (-1.1, 1.1):
            bau.teil(quader(0.1, 0.1, 0.25), HOLZ, m=m @ M((x, s * 0.35, 0)))
    for i in range(3):
        fassliegend = m @ M((-0.8 + i * 0.8, 0, 0.62), 0, 0, 90) @ M((0, 0, -0.5))
        fass(bau, fassliegend)
        bau.teil(zylinder(0.03, 0.12, 5), "#6E4826", m=m @ M((-0.8 + i * 0.8 - 0.55, 0, 0.5), 0, 0, -90))


def buehne(bau, m, zufall):
    """Gauklerbühne: Holzpodest mit Treppe, Rahmen mit rotem Vorhang, Banner, Requisiten."""
    b, t, h = 5.0, 3.4, 1.0
    bau.teil(quader(b, t, h, 0.03), "#8A5A34", m=m)
    for i in range(9):
        bau.teil(quader(b, 0.02, 0.02), "#5A3A22", m=m @ M((0, -t / 2 + (i + 0.5) * t / 9, h)))
    for i in range(3):
        bau.teil(quader(1.2, 0.4, h * (i + 1) / 4), "#8A5A34", m=m @ M((b / 2 - 0.9, -t / 2 - 0.2 - (2 - i) * 0.4, 0)))
    for s in (-1, 1):
        bau.teil(quader(0.2, 0.2, 3.2), HOLZ, m=m @ M((s * b / 2, t / 2 - 0.2, h)))
        bau.teil(platte([(-0.9, 0.0), (0.0, 0.0), (0.2, 2.9), (-0.9, 2.9)], 0.06), "#9E2A2A", m=m @ M((s * (b / 2 - 0.9) * 1.0 + (0.0 if s < 0 else 0.0), t / 2 - 0.3, h), 0 if s < 0 else 0, 0, 0, (-s, 1, 1)))
    bau.teil(quader(b + 0.3, 0.25, 0.3), HOLZ, m=m @ M((0, t / 2 - 0.2, h + 3.2)))
    bau.teil(platte([(-1.8, 0), (1.8, 0), (1.8, 0.6), (-1.8, 0.6)], 0.04), BANNER, m=m @ M((0, t / 2 - 0.35, h + 3.2)))
    bau.teil(platte([(-1.6, 0.1), (1.6, 0.1), (1.6, 0.5), (-1.6, 0.5)], 0.02), GOLD, m=m @ M((0, t / 2 - 0.39, h + 3.2)))
    bau.teil(platte([(-2.4, 0.0), (2.4, 0.0), (2.4, 2.9), (-2.4, 2.9)], 0.05), "#6E2020", m=m @ M((0, t / 2 - 0.1, h)))
    # Requisiten: Kiste, Jonglierbälle, Laute
    kiste(bau, m @ M((-1.5, 0.5, h), 15), 0.6)
    for i, farbe in enumerate(("#E0524F", "#F2D544", "#3A5BB2")):
        bau.teil(kugel(0.08, 6), farbe, m=m @ M((-1.3 + i * 0.2, 0.4, h + 0.68)))
    laute = m @ M((1.0, 0.8, h + 0.05), 30, 0, 70)
    bau.teil(kugel(0.25, 8), "#B98A56", m=laute @ M((0, 0, 0), 0, 0, 0, (1.0, 0.5, 1.25)))
    bau.teil(quader(0.07, 0.04, 0.6), "#6E4826", m=laute @ M((0, 0, 0.25)))
    for s in (-1, 1):
        fackel_m = m @ M((s * (b / 2 + 0.1), -t / 2 + 0.1, 0))
        bau.teil(zylinder(0.05, h + 1.5, 5), HOLZ, m=fackel_m)
        bau.teil(zylinder(0.1, 0.12, 6), EISEN, m=fackel_m @ M((0, 0, h + 1.5)))
        bau.teil(pyramide(0.12, 0.45, 5), "#FF9A2E", m=fackel_m @ M((0, 0, h + 1.6)), leuchten=True)
    del zufall


def anschlagbrett(bau, m, zufall):
    for s in (-1, 1):
        bau.teil(quader(0.14, 0.14, 2.4), HOLZ, m=m @ M((s * 0.9, 0, 0)))
    bau.teil(quader(1.9, 0.08, 1.1, 0.02), "#8A5A34", m=m @ M((0, 0, 1.0)))
    satteldach(bau, m @ M((-1.1, 0, 2.3), 0), 2.2, 0.3, 0.0, 30.0, 0.1, 0.08, False)
    for _ in range(6):
        x, z = zufall.uniform(-0.7, 0.7), zufall.uniform(1.15, 1.85)
        bau.teil(quader(0.28, 0.02, 0.36), zufall.choice(("#F4ECD8", "#EFE2C0", "#E8D8B8")), m=m @ M((x, -0.05, z), 0, 0, zufall.uniform(-8, 8)))
        bau.teil(kugel(0.02, 4), "#B23A3A", m=m @ M((x, -0.07, z + 0.3)))


def huehnerkiste(bau, m, zufall):
    bau.teil(quader(1.0, 0.7, 0.06), "#A0703F", m=m)
    for x in (-0.48, -0.16, 0.16, 0.48):
        bau.teil(quader(0.04, 0.72, 0.6), "#A0703F", m=m @ M((x, 0, 0.06)))
    bau.teil(quader(1.0, 0.7, 0.05), "#A0703F", m=m @ M((0, 0, 0.66)))
    for i in range(2):
        huhn = m @ M((-0.2 + i * 0.4, 0, 0.06), zufall.uniform(0, 360))
        bau.teil(kugel(0.14, 6), "#F1EDE4", m=huhn @ M((0, 0, 0.16), 0, 0, 0, (1.3, 1.0, 1.0)))
        bau.teil(kugel(0.08, 5), "#F1EDE4", m=huhn @ M((0.15, 0, 0.32)))
        bau.teil(pyramide(0.03, 0.08, 3), "#E8B64A", m=huhn @ M((0.22, 0, 0.32), 0, 0, 90))
        bau.teil(quader(0.02, 0.06, 0.06), "#D03A2A", m=huhn @ M((0.15, 0, 0.4)))


def gemuesekarren(bau, m, zufall):
    karren(bau, m)
    for _ in range(10):
        bau.teil(kugel(0.14, 6), zufall.choice(("#E88A2E", "#8FBF3A", "#E0524F")), m=m @ M((zufall.uniform(-0.55, 0.55), zufall.uniform(-1.0, 1.0), 0.9)), schwankung=0.08)


# ---------------------------------------------------------------------------
# Der ganze Platz
# ---------------------------------------------------------------------------
def marktplatz(seed=11):
    bau = Bau(seed)
    zufall = random.Random(seed)
    hb, ht = MARKT_B / 2, MARKT_T / 2

    # Kopfsteinpflaster mit Randsteinen und einer Rinne in der Mitte
    bau.teil(quader(MARKT_B, MARKT_T, 0.06), "#9E948A", KOPFSTEIN)
    for s in (-1, 1):
        bau.teil(quader(MARKT_B, 0.3, 0.14, 0.02), MAUERFUSS, m=M((0, s * (ht - 0.15), 0)))

    # Fachwerkhäuser an der Mauer (Front nach +X)
    haeuser = (
        ("#EFE6D2", "#2E6B46", "brot", "brot"),
        ("#F2DFA8", "#B23A3A", "fisch", "fisch"),
        ("#EBCFC4", "#3A5BB2", "toepfer", "krug"),
        ("#D9E3D0", "#7A3FA0", "waffen", "schwert"),
    )
    for i, (putz, laden, ware, zeichen) in enumerate(haeuser):
        y = -15.0 + i * 8.2
        fachwerkhaus(bau, M((-hb + 7.4, y, 0), 90), 7.2, 6.4, putz, ROT_DACH, laden, ware, zeichen, zufall, 2 if i % 2 == 0 else 1)

    # Markthalle an der Vorderseite (zur Toranlage), offen zum Platz
    markthalle(bau, M((-3.0, -ht + 4.2, 0), 180), 16.0, 6.5, zufall)

    # Marktbrunnen und Maibaum
    marktbrunnen(bau, M((2.0, -1.0, 0)))
    maibaum(bau, M((11.5, 3.5, 0)))

    # Stände in zwei Reihen um den Brunnen
    reihe_vorne = (("obst", 0), ("fleisch", 1), ("kaese", 3), ("gewuerze", 5), ("stoff", 2))
    for i, (ware, stoff) in enumerate(reihe_vorne):
        stand(bau, M((-9.0 + i * 5.2, -9.5, 0), 180), ware, STOFFE[stoff], zufall)
    reihe_hinten = (("gemuese", 2), ("blumen", 4), ("traenke", 4), ("schmuck", 5))
    for i, (ware, stoff) in enumerate(reihe_hinten):
        x = (-9.0, -3.8, 7.2, 12.4)[i]
        stand(bau, M((x, 8.0, 0), 0), ware, STOFFE[stoff], zufall)

    # Händlerzelte
    zelt(bau, M((-8.0, 17.5, 0), 180), "#7A3FA0", zufall)
    zelt(bau, M((18.5, -16.5, 0), 30), "#B23A3A", zufall)

    # Wirtsgarten mit Fassgestell und Sonnenschirmen
    for i, (x, y) in enumerate(((2.0, 16.0), (5.5, 19.0), (9.0, 16.0), (12.5, 19.5))):
        tisch_mit_baenken(bau, M((x, y, 0), zufall.uniform(-10, 10)), zufall)
        if i % 2 == 0:
            schirm(bau, M((x + 1.5, y + 0.3, 0)), STOFFE[i % len(STOFFE)])
    fassgestell(bau, M((7.0, 22.3, 0)))
    ausschank(bau, M((2.5, 21.8, 0)), zufall)

    # Gauklerbühne, Anschlagbrett, Karren, Hühner, Heu, Laternen
    buehne(bau, M((17.0, 13.0, 0), 90 + 180), zufall)
    anschlagbrett(bau, M((21.5, -1.0, 0), 90), zufall)
    gemuesekarren(bau, M((-14.5, 13.0, 0), 20), zufall)
    karren(bau, M((20.0, 3.0, 0), 160))
    for i in range(3):
        huehnerkiste(bau, M((-13.2 + i * 0.3, -3.5 + i * 1.1, 0.7 * (i == 2)), zufall.uniform(-10, 10)), zufall)
    for i in range(3):
        heuballen(bau, M((-13.0, 3.0 + i * 1.3, 0), zufall.uniform(-10, 10)))
    for x, y in ((-14.0, -20.0), (14.0, -20.5), (22.0, 10.0), (-14.0, 21.5), (22.0, -12.0), (0.0, 4.5)):
        laterne(bau, M((x, y, 0)))
    for i in range(12):
        x, y = zufall.uniform(-12.0, 20.0), zufall.choice((zufall.uniform(-14.5, -13.0), zufall.uniform(12.0, 13.5)))
        [fass, kiste, sack][i % 3](bau, M((x, y, 0), zufall.uniform(0, 90)))

    # Wimpelgirlanden: vom Maibaum zu den Häusern, dazu Lichterketten über den Gängen
    spitze = (11.5, 3.5, 12.0)
    for i in range(4):
        y = -15.0 + i * 8.2
        girlande(bau, spitze, (-hb + 7.4 + 0.6, y + 2.0, 6.2), 1.4, zufall)
    girlande(bau, spitze, (21.0, -20.0, 5.0), 1.2, zufall)
    girlande(bau, spitze, (21.5, 22.0, 5.0), 1.2, zufall)
    for s in (-1, 1):
        for x in (-14.0, 22.0):
            bau.teil(zylinder(0.07, 5.0, 6), HOLZ, m=M((x, s * 4.0, 0)))
        girlande(bau, (-14.0, s * 4.0, 4.9), (22.0, s * 4.0, 4.9), 1.0, zufall, 0.6, laternen=True)

    return bau.fertig("Marktplatz", ursprung=(0.0, 0.0))
