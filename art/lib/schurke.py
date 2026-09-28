"""Schurke – vierte Spielfigur (Baukasten: figuren.py, Kopf und Gewichte aus arbeiter.py).

Ein wettergegerbter Abenteurer mit schulterlangem, gewelltem braunem Haar, Mittelscheitel und
Kurzbart. Senfgelber Schal um den Hals, über die linke Schulter geworfen, dahinter ein langer
senfgelber Umhang. Burgunderrote Tunika mit Rautenmuster und geflochtener heller Borte über
dunklem Hemd, breiter Ledergürtel mit großen Silbernieten, darunter ein schräger Gürtel mit
Taschen, Fläschchen und runder Schnalle. Links: geschichtete Schulterplatte mit Rundbuckel,
verziertes Bruststück mit Mäanderschnallen, Lederband am Oberarm, Wickelbänder und ein Stahl-
handschuh. Rechts: Wickelbänder um den Unterarm, die bloße Hand um die Runenklinge (waffen.py).
Weite dunkle Hose, Wickelbänder über den Schnürstiefeln mit Stulpen, eine Schwertscheide an der
linken Hüfte.

Gleiche Knochennamen wie die anderen Figuren; der Umhang hängt am Oberkörper und schwingt mit
den Oberschenkeln. Animationen: wie der Magier (Springen, Hacken, Abbauen …), dazu Kampfstand,
Laufen und Rennen mit der Klinge sowie Hieb1, Hieb2, Stich, Dolchwurf, Rauchwurf, Schattenruf.
Koordinaten: Z oben, die Figur schaut nach -Y, Füße im Ursprung, links (L) = +X.
"""

import math

from mathutils import Vector

from arbeiter import (AUGE, BRUST_GEWICHT, DZ, ELLBOGEN, GRIFF_R, HANDGELENK, HUEFTE, KNIE, KNOECHEL, KOPF, KOPF_GEWICHT, SCHULTER, _arm, _bein,
                      _faust, _huefte_bein, _kopf, _kopf_und_hals, _rumpf, _skelett, _spiegel)
from figuren import Figur, _achsen, _axt, _clip, _gehen, _glocke, _mischen, _mit, _platte, _schlauch, _schleife, _spitzhacke, farbe, weich
from werkstatt import animation

X, Y, Z = Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))

# Gedeckte Farben: der Helden-Shader hebt die Sättigung noch an
HAUT, LIPPE, WANGE = farbe("#C99070"), farbe("#96584A"), farbe("#B87A62")
HAAR, HAAR_HELL, HAAR_DUNKEL = farbe("#4A3322"), farbe("#6A4A32"), farbe("#2E2016")
HEMD = farbe("#46241F")
TUNIKA, TUNIKA_DUNKEL, TUNIKA_HELL = farbe("#6A3A32"), farbe("#58302A"), farbe("#7A463C")
BORTE, BORTE_DUNKEL = farbe("#C8BAA0"), farbe("#9A8C74")
HOSE, HOSE_DUNKEL = farbe("#3E3D38"), farbe("#34332F")
WICKEL, WICKEL_DUNKEL = farbe("#BDB098"), farbe("#9A8E78")
STIEFEL, STIEFEL_DUNKEL, STIEFEL_HELL = farbe("#6A4A34"), farbe("#402C1E"), farbe("#7E5A40")
LEDER, LEDER_DUNKEL = farbe("#54382A"), farbe("#34241A")
STAHL, STAHL_HELL, STAHL_DUNKEL = farbe("#66625C"), farbe("#86817A"), farbe("#403D39")
SCHAL, SCHAL_DUNKEL, SCHAL_HELL = farbe("#9A7030"), farbe("#6E5020"), farbe("#B08840")
RUNE = farbe("#8FE0FF")


def _umhang_gewicht(co):
    """Oben am Oberkörper, unten schwingt er mit beiden Oberschenkeln."""
    if co.z > 1.02:
        return _rumpf(co)
    bein = weich(1.02, 0.4, co.z) * 0.75
    links = weich(-0.12, 0.12, co.x)
    return _mischen(("Becken", 1 - bein), ("Oberschenkel.L", bein * links), ("Oberschenkel.R", bein * (1 - links)))


def _tunika_gewicht(co):
    if co.z > 0.98:
        return _rumpf(co)
    bein = weich(0.98, 0.6, co.z)
    links = weich(-0.06, 0.06, co.x)
    return _mischen(("Becken", 1 - bein), ("Oberschenkel.L", bein * links), ("Oberschenkel.R", bein * (1 - links)))


def schurke(seed=41, name="Schurke"):
    f = Figur(name, seed)
    r = f.rng

    # ================= Beine: weite Hose, Wickelbänder, Schnürstiefel mit Stulpen =================
    for seite in (1, -1):
        sn = "L" if seite > 0 else "R"
        h_, k_, a_ = _spiegel(HUEFTE, seite), _spiegel(KNIE, seite), _spiegel(KNOECHEL, seite)
        punkte = [h_ + Vector((0.012 * seite, 0.005, 0.03)), h_.lerp(k_, 0.35), h_.lerp(k_, 0.75), k_, k_.lerp(a_, 0.35), k_.lerp(a_, 0.6)]
        radien = [0.1, 0.092, 0.08, 0.078, 0.07, 0.06]

        def hose(i, k, p, s=seite):
            # Falten: dunkle Streifen, die sich um das Knie legen
            return HOSE.lerp(HOSE_DUNKEL, 0.5 + 0.5 * math.sin(p.center.z * 24 + s)) * (0.93 + 0.1 * max(0.0, -p.normal.y))
        _schlauch(f, "Hose", punkte, radien, 22, hose, _bein(seite), zu=False, teilung=3)
        # Wickelbänder vom Stiefelrand bis unters Knie (schräg gewickelt)
        wickel = [a_ + Vector((0, 0.005, 0.2)), k_.lerp(a_, 0.62), k_.lerp(a_, 0.4)]

        def wickel_farbe(i, k, p):
            band = int(p.center.z * 55 + k * 0.35) % 3
            return (WICKEL, WICKEL * 0.92, WICKEL_DUNKEL)[band]
        _schlauch(f, "Wickel", wickel, [0.058, 0.064, 0.068], 20, wickel_farbe, _bein(seite), zu=False, teilung=3)
        # Stiefel: Schaft bis zur Wade mit umgeschlagener Stulpe, Schnürung vorne
        schaft = [a_ + Vector((0, 0.008, -0.04)), a_ + Vector((0, 0.0, 0.06)), a_ + Vector((0, -0.004, 0.17))]
        _schlauch(f, "Stiefelschaft", schaft, [0.058, 0.06, 0.066], 20, lambda i, k, p: STIEFEL * (0.9 if k % 5 == 0 else 1.0), _bein(seite), zu=False, teilung=2)
        stulpe = [a_ + Vector((0, 0.0, 0.15)), a_ + Vector((0, -0.004, 0.2)), a_ + Vector((0, -0.006, 0.235))]
        _schlauch(f, "Stulpe", stulpe, [0.078, 0.082, 0.08], 20, lambda i, k, p: STIEFEL_HELL * (0.85 if i < 0.5 else 1.0), _bein(seite), zu=False, teilung=2)
        fuss = [(Vector((0.1 * seite, 0.06, 0.05)), X, Z, 0.052, 0.05), (Vector((0.1 * seite, 0.04, 0.02)), X, Z, 0.058, 0.03),
                (Vector((0.1 * seite, -0.03, 0.05)), X, Z, 0.06, 0.056), (Vector((0.1 * seite, -0.1, 0.045)), X, Z, 0.054, 0.042),
                (Vector((0.1 * seite, -0.155, 0.04)), X, Z, 0.038, 0.032), (Vector((0.1 * seite, -0.18, 0.04)), X, Z, 0.006, 0.006)]
        f.loft("Stiefel", fuss, 16, lambda i, k, p: STIEFEL_DUNKEL * 0.6 if p.center.z < 0.018 else STIEFEL * (1.08 if p.center.y < -0.1 else 1.0),
               lambda co, sn=sn: {f"Fuss.{sn}": 1.0}, oben_zu=True, unten_zu=True, teilung=3, glatt=True)
        for j in range(5):
            f.kiste("Schnuer", (0.1 * seite, -0.056 - 0.006 * j, 0.1 + 0.03 * j), (0.05, 0.006, 0.006), LEDER_DUNKEL, _bein(seite))

    # ================= Rumpf: dunkles Hemd, darüber die Tunika =================
    rumpf = [(0.86, 0.01, 0.13, 0.1), (0.92, 0.012, 0.165, 0.118), (0.98, 0.012, 0.17, 0.12), (1.05, 0.006, 0.158, 0.118),
             (1.14, 0.0, 0.16, 0.12), (1.24, -0.006, 0.18, 0.126), (1.33, -0.008, 0.2, 0.13), (1.41, 0.0, 0.208, 0.122),
             (1.47, 0.01, 0.172, 0.1), (1.52, 0.014, 0.092, 0.076), (1.57, 0.014, 0.058, 0.058)]
    ringe = [(Vector((0, y, z)), X, Y, rx, ry, lambda w: 1.0 - 0.05 * max(0.0, -math.sin(w))) for z, y, rx, ry in rumpf]

    def rumpf_farbe(i, k, p):
        z = p.center.z
        if z > 1.49:
            return HAUT * 0.95                                                    # Hals
        if z < 0.99:
            return HOSE
        # Tunika mit Rautenmuster (dunkle Linien), vorne ein V-Ausschnitt, der das Hemd zeigt
        vorne = p.normal.y < -0.4
        if vorne and z > 1.3 and abs(p.center.x) < (z - 1.3) * 0.5:
            return HEMD
        return TUNIKA * (0.92 + 0.1 * max(0.0, -p.normal.y))
    f.loft("Koerper", ringe, 44, rumpf_farbe, _huefte_bein, teilung=3, glatt=True)

    # Tunikarock: vorne und hinten je eine Bahn bis zur Mitte der Oberschenkel, seitlich geschlitzt
    for name, theta in (("Tunika vorne", (-62, 62)), ("Tunika hinten", (118, 242))):
        rock = [(Vector((0, 0.01, 1.0)), 0.182, 0.13), (Vector((0, 0.012, 0.85)), 0.2, 0.145), (Vector((0, 0.014, 0.7)), 0.215, 0.158),
                (Vector((0, 0.016, 0.58)), 0.222, 0.165)]

        def rock_farbe(poly):
            z = poly.center.z
            if z < 0.585 + 0.075:
                # Geflochtene Borte: helles Zopfmuster
                w = math.atan2(poly.center.x, -poly.center.y) * 18 + z * 60
                return BORTE if math.sin(w) > -0.2 else BORTE_DUNKEL
            if 0.66 < z < 0.672:
                return BORTE_DUNKEL
            return TUNIKA.lerp(TUNIKA_DUNKEL, weich(0.9, 0.62, z) * 0.6)
        _glocke(f, name, rock, theta, rock_farbe, _tunika_gewicht, seg=34, welle=0.04)
        _glocke(f, name + " innen", rock, theta, lambda poly: TUNIKA_DUNKEL * 0.8, _tunika_gewicht, innen=True, seg=34, welle=0.04)

    # ================= Gürtel: breit mit Silbernieten, darunter schräg mit Taschen =================
    f.loft("Guertel", [(Vector((0, 0.01, 0.95)), X, Y, 0.186, 0.134), (Vector((0, 0.01, 1.0)), X, Y, 0.184, 0.132), (Vector((0, 0.01, 1.06)), X, Y, 0.172, 0.128)],
           40, lambda i, k, p: LEDER_DUNKEL * (1.15 if 0.8 < i < 1.2 else 1.0), _rumpf, teilung=2, glatt=True)
    for j in range(7):
        w = math.radians(-60 + 20 * j)
        ort = Vector((math.sin(w) * 0.186, 0.01 - math.cos(w) * 0.136, 1.005))
        groesse = 0.03 if j != 3 else 0.036
        f.kugel("Niete", ort, (groesse, groesse * 0.5, groesse), lambda poly: STAHL_HELL if poly.normal.z > 0.3 else STAHL, _rumpf, 12, 8)
    # Schräger Gürtel von rechts oben nach links unten, runde Schnalle links, Taschen und Fläschchen
    schraeg = []
    for k in range(25):
        w = math.tau * k / 24
        z = 0.9 - 0.035 * math.sin(w - 0.3)
        schraeg.append(Vector((math.sin(w) * 0.2, 0.012 - math.cos(w) * 0.148, z)))
    _schlauch(f, "Taschengurt", schraeg, [0.014] * len(schraeg), 6, lambda i, k, p: LEDER, _tunika_gewicht, zu=False, form=lambda w: 1.0 + 0.6 * abs(math.sin(w)))
    schnalle = Vector((0.12, -0.14, 0.89))
    f.loft("Schnalle", [(schnalle, X, Z, 0.045, 0.045), (schnalle + Vector((0, -0.012, 0)), X, Z, 0.045, 0.045), (schnalle + Vector((0, -0.016, 0)), X, Z, 0.03, 0.03)],
           20, lambda i, k, p: STAHL_HELL if i > 1.5 else STAHL, _tunika_gewicht, oben_zu=True, unten_zu=True)
    f.kugel("Schnallenknopf", schnalle + Vector((0, -0.018, 0)), (0.014, 0.008, 0.014), STAHL_DUNKEL, _tunika_gewicht, 10, 6)
    for x, breite in ((-0.1, 0.07), (-0.16, 0.06)):
        f.kiste("Tasche", (x, -0.13 + abs(x) * 0.25, 0.86), (breite, 0.04, 0.08), LEDER, _tunika_gewicht)
        f.kiste("Taschendeckel", (x, -0.152 + abs(x) * 0.25, 0.895), (breite + 0.006, 0.01, 0.03), LEDER_DUNKEL, _tunika_gewicht)
    f.loft("Flaeschchen", [(Vector((-0.2, -0.08, 0.8)), X, Y, 0.016, 0.016), (Vector((-0.2, -0.08, 0.85)), X, Y, 0.02, 0.02), (Vector((-0.2, -0.08, 0.87)), X, Y, 0.008, 0.008)],
           10, lambda i, k, p: farbe("#4A8ACC") if i < 1.2 else farbe("#D8D0B8"), _tunika_gewicht, oben_zu=True, unten_zu=True)

    # ================= Schwertscheide an der linken Hüfte (schräg nach vorne) =================
    oben_s, unten_s = Vector((0.04, -0.17, 0.93)), Vector((0.46, -0.3, 0.6))
    q1, q2 = _achsen(oben_s, unten_s)
    scheide = [(oben_s.lerp(unten_s, t), q1, q2, 0.052 * (1 - 0.5 * t ** 2) + 0.004, 0.016) for t in (0.0, 0.3, 0.7, 0.95, 1.0)]
    f.loft("Scheide", scheide, 12, lambda i, k, p: LEDER * (0.85 if int(i * 3) % 2 else 1.0), _tunika_gewicht, oben_zu=True, unten_zu=True, teilung=3)
    f.loft("Scheidenmund", [(oben_s, q1, q2, 0.058, 0.022), (oben_s.lerp(unten_s, 0.06), q1, q2, 0.058, 0.022)], 12, lambda i, k, p: STAHL, _tunika_gewicht,
           oben_zu=True, unten_zu=True)
    f.kugel("Ortband", unten_s, (0.025, 0.025, 0.025), STAHL, _tunika_gewicht, 8, 6)

    # ================= Brustpanzer links: verzierte Platte, zwei Riemen mit Mäanderschnallen =================
    platte_mitte = Vector((0.09, -0.125, 1.33))
    _platte(f, "Brustplatte", platte_mitte, 0.24, 0.1, 0.02, Z, Vector((0.35, -1, 0.1)),
            lambda i, k, p: STAHL_HELL if (k % 3 == 0 and 1 < i < 5) else STAHL * (0.9 + 0.1 * math.sin(i * 3)), _rumpf, spitz=0.25, wolbung=1.2)
    for j, z in enumerate((1.27, 1.2)):
        riemen = [Vector((-0.19, -0.02, z + 0.06)), Vector((-0.12, -0.13, z + 0.03)), Vector((0.0, -0.148, z)), Vector((0.14, -0.13, z - 0.02)),
                  Vector((0.2, -0.02, z - 0.03))]
        _schlauch(f, "Riemen", riemen, [0.018] * 5, 6, lambda i, k, p: LEDER_DUNKEL, _rumpf, zu=False, form=lambda w: 1.0 + 0.9 * abs(math.cos(w)))
        mitte = Vector((-0.07 + 0.02 * j, -0.152, z + 0.018))
        f.kiste("Maeander", mitte, (0.05, 0.012, 0.042), STAHL_HELL, _rumpf)
        f.kiste("Maeanderkern", mitte + Vector((0, -0.007, 0)), (0.026, 0.006, 0.018), STAHL_DUNKEL, _rumpf)

    # ================= Arme =================
    for seite in (1, -1):
        s_, e_, h_ = _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite)
        punkte = [s_ + Vector((-0.02 * seite, 0, 0.03)), s_.lerp(e_, 0.5), e_, e_.lerp(h_, 0.5), h_]

        def arm_farbe(i, k, p):
            if i < 2.1:
                return HEMD * (0.9 + 0.12 * max(0.0, p.normal.z))              # Ärmel des Hemds
            # Wickelbänder über dem Unterarm
            band = int((p.center - h_).length * 70 + k * 0.4) % 3
            return (WICKEL, WICKEL * 0.9, WICKEL_DUNKEL)[band]
        _schlauch(f, "Arm", punkte, [0.066, 0.058, 0.05, 0.048, 0.04], 18, arm_farbe, _arm(seite), zu=False, teilung=3)
        # Aufgekrempelter Ärmelrand am Ellbogen
        q1, q2 = _achsen(e_, h_)
        f.loft("Aermelrand", [(e_.lerp(h_, t), q1, q2, rad, rad) for t, rad in ((-0.05, 0.058), (0.05, 0.064), (0.14, 0.058))], 18,
               lambda i, k, p: HEMD * 1.1, _arm(seite), teilung=2, glatt=True)
    # Lederband am linken Oberarm
    sl, el, hl = SCHULTER, ELLBOGEN, HANDGELENK
    q1, q2 = _achsen(sl, el)
    f.loft("Armband", [(sl.lerp(el, t), q1, q2, 0.064, 0.064) for t in (0.45, 0.62)], 18, lambda i, k, p: LEDER * (0.85 if k % 6 == 0 else 1.0), _arm(1),
           oben_zu=True, unten_zu=True, glatt=True)
    # Stahlhandschuh links: Stulpe aus Platten mit Nieten
    q1, q2 = _achsen(el, hl)
    f.loft("Stulpe", [(el.lerp(hl, t), q1, q2, rad, rad) for t, rad in ((0.72, 0.052), (0.9, 0.058), (1.04, 0.06))], 18,
           lambda i, k, p: STAHL_HELL if k % 6 == 0 else STAHL, _arm(1), oben_zu=True, unten_zu=True, glatt=True)
    for k in range(5):
        w = math.tau * k / 5
        f.kugel("Handschuhniete", el.lerp(hl, 0.9) + (q1 * math.cos(w) + q2 * math.sin(w)) * 0.058, (0.008, 0.008, 0.008), STAHL_HELL, _arm(1), 6, 4)
    griff_l = Vector((0.335, -0.04, 0.905))
    f.metaball("HandL", _faust(griff_l, Y, 1, 0.0105), 0.003, 2000, lambda poly: STAHL_HELL if poly.normal.z > 0.2 else STAHL,
               lambda co: {"Hand.L": 1.0}, glatt=True)
    for k in range(3):
        f.kugel("Knoechelplatte", griff_l + Vector((0.03, -0.022 + 0.022 * k, 0.012)), (0.012, 0.012, 0.012), STAHL_HELL, lambda co: {"Hand.L": 1.0}, 8, 6)
    # Rechte Hand: bloß, fest um die Klinge
    f.metaball("HandR", _faust(GRIFF_R, Z, -1, 0.0095), 0.003, 2000, lambda poly: HAUT * (0.92 if poly.normal.x > 0.3 else 1.0),
               lambda co: {"Hand.R": 1.0}, glatt=True)

    # ================= Schulterplatte links: drei Lagen, Rundbuckel, Nieten =================
    sl_ = SCHULTER + Vector((0.03, 0.0, 0.04))
    schulter_gewicht = lambda co: _mischen(("Oberarm.L", 0.6), ("Brust", 0.4))
    f.kugel("Schulterkappe", sl_, (0.1, 0.105, 0.07), lambda poly: STAHL_HELL if poly.normal.z > 0.5 else STAHL, schulter_gewicht, 22, 12, glatt=True)
    for j in range(3):
        mitte = sl_ + Vector((0.07 + 0.012 * j, 0.0, -0.04 - 0.045 * j))
        _platte(f, "Schulterplatte", mitte, 0.22 - 0.03 * j, 0.075 - 0.006 * j, 0.014, Vector((0, 1, -0.2)), Vector((1, 0, 0.45)),
                lambda i, k, p, j=j: (STAHL if j % 2 else STAHL_HELL * 0.92) * (0.95 + 0.05 * (k % 2)), schulter_gewicht, spitz=0.35, wolbung=1.4)
    buckel = sl_ + Vector((0.07, -0.06, 0.02))
    normale = Vector((0.55, -1.0, 0.3)).normalized()
    b1, b2 = _achsen(buckel, buckel + normale)
    f.loft("Rundbuckel", [(buckel, b1, b2, 0.062, 0.062), (buckel + normale * 0.012, b1, b2, 0.064, 0.064), (buckel + normale * 0.02, b1, b2, 0.05, 0.05),
                          (buckel + normale * 0.03, b1, b2, 0.022, 0.022)], 24, lambda i, k, p: STAHL_HELL if 0.8 < i < 1.4 or i > 2.4 else STAHL,
           schulter_gewicht, oben_zu=True, unten_zu=True, glatt=True)
    for k in range(6):
        w = math.tau * k / 6
        f.kugel("Schulterniete", sl_ + Vector((0.1, 0.0, -0.02)) + Vector((0, math.cos(w) * 0.06, math.sin(w) * 0.04)), (0.009, 0.009, 0.009), STAHL_HELL,
                schulter_gewicht, 6, 4)
    # Spitze Zierkante oben auf der Schulterplatte
    _platte(f, "Schulterkamm", sl_ + Vector((0.02, 0.02, 0.07)), 0.2, 0.03, 0.01, Y, Vector((0.3, 0, 1)), lambda i, k, p: STAHL_DUNKEL, schulter_gewicht,
            spitz=0.8, wolbung=0.6)

    # ================= Kopf, langes Haar =================
    _kopf(f, HAUT, LIPPE, WANGE, HAAR, "kurz")
    # Haarkappe: nur oben und hinten sichtbar (vorne liegt sie hinter der Stirn)
    f.metaball("Haarkappe", [(KOPF + Vector((0, 0.022, 0.035)), (0.102, 0.108, 0.1)), (KOPF + Vector((0, 0.05, -0.02)), (0.1, 0.09, 0.09))], 0.008, 1400,
               lambda poly: (HAAR if math.sin(math.atan2(poly.center.x, poly.center.y - KOPF.y) * 26) > -0.3 else HAAR_DUNKEL) * (0.9 + 0.12 * max(0.0, poly.normal.z)),
               KOPF_GEWICHT, glatt=True)
    # Gewellte Strähnen vom Mittelscheitel nach unten: vorne bis zum Kinn, hinten bis in den Nacken
    for n in range(48):
        seite = 1 if n % 2 == 0 else -1
        theta = math.radians(72 + 112 * (n // 2) / 23 + r.uniform(-3, 3)) * seite
        aussen = Vector((math.sin(theta), -math.cos(theta), 0))
        hinten = abs(math.cos(theta)) if math.cos(theta) < 0 else 0.0
        ende_z = KOPF.z - 0.12 - 0.09 * hinten - r.uniform(0.0, 0.04)
        start = KOPF + aussen * 0.03 + Vector((0, 0.005, 0.118))
        punkte = []
        for j in range(8):
            t = j / 7
            radius = 0.105 + 0.02 * math.sin(min(1.0, t * 1.6) * math.pi / 2) + 0.018 * t
            welle = aussen.cross(Z) * math.sin(t * math.pi * 2.4 + n) * 0.012 * t
            hoehe = KOPF.z + 0.118 - (KOPF.z + 0.118 - ende_z) * (t ** 0.85)
            p = KOPF + aussen * radius * min(1.0, 0.35 + t * 1.5)
            punkte.append(Vector((p.x, p.y, hoehe)) + welle + (start - KOPF) * (1 - min(1.0, t * 3)) * 0.0)
        c = (HAAR, HAAR_HELL * 0.85, HAAR * 0.85)[n % 3]
        f.straehne("Haar", punkte, r.uniform(0.028, 0.036), 0.005, c, KOPF_GEWICHT, 8, 0.4, 0.45, glatt=True)
    # Zwei lose Strähnen in der Stirn
    for s in (1, -1):
        punkte = [KOPF + Vector((0.012 * s, -0.08, 0.11)), KOPF + Vector((0.055 * s, -0.1, 0.075)), KOPF + Vector((0.095 * s, -0.075, 0.03)),
                  KOPF + Vector((0.112 * s, -0.05, -0.04))]
        f.straehne("Stirnhaar", punkte, 0.018, 0.003, HAAR_HELL * 0.9, KOPF_GEWICHT, 7, 0.3, 0.5, glatt=True)

    # ================= Schal: dicke Wülste um den Hals, ein Ende über die linke Schulter =================
    schal_gewicht = lambda co: _mischen(("Brust", 0.75), ("Hals", 0.25)) if co.z > 1.45 else {"Brust": 1.0}
    for j, (z, rx, ry, dicke) in enumerate(((1.49, 0.14, 0.125, 0.055), (1.545, 0.115, 0.105, 0.045))):
        weg = []
        for k in range(33):
            w = math.tau * k / 32
            weg.append(Vector((math.sin(w) * rx, 0.012 - math.cos(w) * ry, z + 0.02 * math.sin(w * 2 + j) - 0.03 * max(0.0, math.sin(w)))))

        def schal_farbe(i, k, p, j=j):
            return SCHAL.lerp(SCHAL_DUNKEL, 0.35 + 0.35 * math.sin(i * 0.9 + j)) * (0.85 + 0.2 * max(0.0, p.normal.z))
        _schlauch(f, "Schal", weg, [dicke] * len(weg), 16, schal_farbe, schal_gewicht, zu=False,
                  form=lambda w: 1.0 + 0.12 * math.sin(w * 3))
    # Überwurf: breite Bahn von der linken Schulter vorne herab, gefaltet
    for k in range(3):
        mitte = Vector((0.14 + 0.02 * k, -0.11 + 0.01 * k, 1.36 - 0.03 * k))
        _platte(f, "Schalbahn", mitte, 0.34 - 0.06 * k, 0.07 - 0.01 * k, 0.012, Vector((0.25, -0.1, -1)), Vector((0.2, -1, 0.1)),
                lambda i, k2, p, k=k: SCHAL.lerp(SCHAL_DUNKEL, 0.2 + 0.2 * k) * (0.9 + 0.12 * max(0.0, -p.normal.y)), BRUST_GEWICHT, spitz=0.35, wolbung=1.2)
    f.loft("Schalende", [(Vector((0.19, -0.13, 1.14)), X, Y, 0.06, 0.014), (Vector((0.2, -0.13, 1.1)), X, Y, 0.05, 0.012)], 10, lambda i, k, p: SCHAL_DUNKEL,
           BRUST_GEWICHT, oben_zu=True, unten_zu=True)

    # ================= Umhang: vom Schal über den Rücken bis zu den Waden =================
    umhang = [(Vector((0, 0.03, 1.5)), 0.2, 0.14), (Vector((0, 0.06, 1.3)), 0.26, 0.17), (Vector((0, 0.08, 1.0)), 0.3, 0.2),
              (Vector((0, 0.1, 0.7)), 0.33, 0.23), (Vector((0, 0.12, 0.38)), 0.36, 0.26)]

    def umhang_farbe(poly):
        z = poly.center.z
        c = SCHAL.lerp(SCHAL_HELL, weich(1.4, 0.5, z) * 0.3) * (0.88 + 0.14 * max(0.0, -poly.normal.y))
        if z < 0.42:
            return SCHAL_DUNKEL * 1.05                                             # Saum
        falte = math.sin(math.atan2(poly.center.x, poly.center.y) * 9 + z * 2)
        return c * (0.92 + 0.08 * falte)
    _glocke(f, "Umhang", umhang, (95, 265), umhang_farbe, _umhang_gewicht, seg=40, welle=0.06)
    _glocke(f, "Umhang innen", umhang, (95, 265), lambda poly: SCHAL_DUNKEL * 0.75, _umhang_gewicht, innen=True, seg=40, welle=0.06)

    # ================= Waffen: Runenklinge (Startwaffe) und die Beuteklingen, Werkzeuge =================
    import waffen
    hand_gewicht = lambda co: {"Hand.R": 1.0}
    for art in ["Klinge"] + waffen.KLINGEN:
        anfang = len(f.teile)
        waffen.klinge(f, art, GRIFF_R.x, GRIFF_R.y, hand_gewicht, GRIFF_R.z)
        f.als_starr(art, "Hand.R", anfang)
    anfang = len(f.teile)
    _spitzhacke(f, GRIFF_R.x, GRIFF_R.y, hand_gewicht)
    f.als_starr("Spitzhacke", "Hand.R", anfang)
    anfang = len(f.teile)
    _axt(f, GRIFF_R.x, GRIFF_R.y, hand_gewicht)
    f.als_starr("Axt", "Hand.R", anfang)

    _skelett(f)
    import anlegen
    f.ruestungen = anlegen.fuer_klasse("schurke")
    return f.fertig(_schurken_animationen)


def _schurken_animationen(armatur):
    """Wie der Magier (Springen, Hieb, Werfen, Abbauen, Hacken), eigener Kampfstand und Angriffe."""
    import bpy
    from figuren import _magier_animationen
    _magier_animationen(armatur)
    for alt in ("Idle", "Laufen", "Rennen", "Arkan", "Feuerball", "Frostnova", "Meteor"):
        aktion = bpy.data.actions.get(alt)
        if aktion:
            bpy.data.actions.remove(aktion)

    # Kampfstand: die Klinge schräg nach vorne unten, die gepanzerte Linke locker neben der Hüfte
    def idle(phi):
        return [
            ("Brust", "rot", (1.5 * math.sin(phi * 2), -4, 0)), ("Bauch", "rot", (-0.8 * math.sin(phi * 2), 0, 0)),
            ("Kopf", "rot", (2 * math.sin(phi * 2 + 1), 10 + 12 * math.sin(phi), 0)),
            ("Oberarm.R", "rot", (-14 + 2 * math.sin(phi * 2), 0, -6)), ("Unterarm.R", "rot", (-38, 0, 0)), ("Hand.R", "rot", (-18, 0, 0)),
            ("Oberarm.L", "rot", (-4, 0, 6)), ("Unterarm.L", "rot", (-22 - 3 * math.sin(phi * 2), 0, 0)),
            ("Oberschenkel.L", "rot", (-4, 0, 4)), ("Oberschenkel.R", "rot", (4, 0, -4)), ("Unterschenkel.L", "rot", (4, 0, 0)),
            ("Becken", "pos", (0, -0.004 * (1 - math.cos(phi * 2)), 0)),
        ]
    animation(armatur, "Idle", 180, _schleife(180, 6, idle))

    def gehen(phi, schwung, knie, arm, huepfen, vorbeugen, ellbogen):
        werte = [w for w in _gehen(phi, schwung, knie, arm, huepfen, vorbeugen, ellbogen) if w[0] not in ("Oberarm.R", "Unterarm.R")]
        s = math.sin(phi)
        return werte + [("Oberarm.R", "rot", (-12 + arm * 0.35 * s, 0, -6)), ("Unterarm.R", "rot", (-40 - ellbogen * 0.3, 0, 0)), ("Hand.R", "rot", (-15, 0, 0))]
    animation(armatur, "Laufen", 30, _schleife(30, 2, lambda phi: gehen(phi, 28, 45, 22, 0.03, 4, 14)))
    animation(armatur, "Rennen", 20, _schleife(20, 2, lambda phi: gehen(phi, 44, 80, 40, 0.06, 14, 40)))

    ruhe = {"Oberarm.R": (-14, 0, -6), "Unterarm.R": (-38, 0, 0), "Hand.R": (-18, 0, 0), "Oberarm.L": (-4, 0, 6), "Unterarm.L": (-22, 0, 0)}
    stand = {"Oberschenkel.L": (-14, 0, 6), "Unterschenkel.L": (16, 0, 0), "Oberschenkel.R": (10, 0, -6), "Unterschenkel.R": (10, 0, 0), "Becken.pos": (0, 0, -0.03)}

    # Hieb1: von rechts oben schräg nach links unten (Treffer Bild 6)
    h1_aus = {**stand, "Brust": (-4, -32, 0), "Bauch": (0, -14, 0), "Kopf": (0, 26, 0), "Oberarm.R": (-128, 0, -28), "Unterarm.R": (-62, 0, 0), "Hand.R": (-20, 0, 0),
              "Oberarm.L": (-40, 0, 20), "Unterarm.L": (-60, 0, 0)}
    h1_schlag = {**stand, "Brust": (10, 26, 0), "Bauch": (5, 12, 0), "Kopf": (-6, -20, 0), "Oberarm.R": (-72, 0, 38), "Unterarm.R": (-12, 0, 0), "Hand.R": (6, 0, 0),
                 "Oberarm.L": (-10, 0, 26), "Unterarm.L": (-40, 0, 0), "Oberschenkel.L": (-24, 0, 6), "Unterschenkel.L": (26, 0, 0)}
    _clip(armatur, "Hieb1", 16, [(0, ruhe), (3, h1_aus), (6, h1_schlag), (9, _mit(h1_schlag, Oberarm_R=(-66, 0, 44))), (16, ruhe)])
    # Hieb2: Rückhand von links nach rechts (Treffer Bild 6)
    h2_aus = {**stand, "Brust": (0, 30, 0), "Bauch": (0, 14, 0), "Kopf": (0, -24, 0), "Oberarm.R": (-78, 0, 52), "Unterarm.R": (-112, 0, 0), "Hand.R": (-10, 0, 0),
              "Oberarm.L": (-20, 0, 10), "Unterarm.L": (-50, 0, 0)}
    h2_schlag = {**stand, "Brust": (8, -30, 0), "Bauch": (4, -14, 0), "Kopf": (-4, 24, 0), "Oberarm.R": (-86, 0, -48), "Unterarm.R": (-8, 0, 0), "Hand.R": (4, 0, 0),
                 "Oberarm.L": (-30, 0, 28), "Unterarm.L": (-70, 0, 0), "Oberschenkel.R": (-16, 0, -6), "Unterschenkel.R": (22, 0, 0)}
    _clip(armatur, "Hieb2", 16, [(0, ruhe), (3, h2_aus), (6, h2_schlag), (9, _mit(h2_schlag, Oberarm_R=(-84, 0, -54))), (16, ruhe)])
    # Stich: zurückziehen, dann mit Ausfallschritt weit nach vorne stoßen (Treffer Bild 8)
    zurueck = {**stand, "Brust": (-2, -28, 0), "Bauch": (0, -12, 0), "Kopf": (0, 24, 0), "Oberarm.R": (-24, 0, -18), "Unterarm.R": (-108, 0, 0), "Hand.R": (38, 0, 0),
               "Oberarm.L": (-60, 0, 30), "Unterarm.L": (-50, 0, 0)}
    stoss = {"Brust": (12, 22, 0), "Bauch": (6, 10, 0), "Kopf": (-10, -18, 0), "Oberarm.R": (-90, 0, 6), "Unterarm.R": (-2, 0, 0), "Hand.R": (2, 0, 0),
             "Oberarm.L": (-6, 0, 40), "Unterarm.L": (-20, 0, 0), "Oberschenkel.L": (-46, 0, 6), "Unterschenkel.L": (52, 0, 0), "Fuss.L": (-6, 0, 0),
             "Oberschenkel.R": (26, 0, -6), "Unterschenkel.R": (12, 0, 0), "Becken.pos": (0, -0.1, -0.1)}
    _clip(armatur, "Stich", 24, [(0, ruhe), (5, zurueck), (8, stoss), (13, _mit(stoss, Oberarm_R=(-88, 0, 4))), (24, ruhe)])
    # Dolchwurf: die Linke greift zum Gürtel, holt weit aus und schleudert die Dolche (Bild 9)
    greifen = {**stand, "Oberarm.L": (10, 0, -10), "Unterarm.L": (-70, 0, 0), "Brust": (6, 10, 0)}
    ausholen = {**stand, "Brust": (-4, 34, 0), "Bauch": (0, 14, 0), "Kopf": (0, -30, 0), "Oberarm.L": (-86, 0, 54), "Unterarm.L": (-110, 0, 0),
                "Oberarm.R": (-30, 0, -20), "Unterarm.R": (-40, 0, 0), "Hand.R": (22, 0, 0)}
    wurf = {**stand, "Brust": (8, -26, 0), "Bauch": (4, -12, 0), "Kopf": (-4, 22, 0), "Oberarm.L": (-92, 0, -22), "Unterarm.L": (-6, 0, 0),
            "Oberarm.R": (-10, 0, -30), "Unterarm.R": (-30, 0, 0), "Hand.R": (22, 0, 0), "Oberschenkel.R": (-18, 0, -6), "Unterschenkel.R": (22, 0, 0)}
    _clip(armatur, "Dolchwurf", 24, [(0, ruhe), (3, greifen), (7, ausholen), (9, wurf), (13, _mit(wurf, Oberarm_L=(-80, 0, -32))), (24, ruhe)])
    # Rauchwurf: die Linke hoch, dann die Bombe kraftvoll vor die Füße schmettern (Bild 10), in die Hocke
    hoch = {**stand, "Brust": (-10, 12, 0), "Oberarm.L": (-168, 0, 10), "Unterarm.L": (-40, 0, 0), "Oberarm.R": (-30, 0, -20), "Unterarm.R": (-50, 0, 0)}
    runter = {"Brust": (26, 0, 0), "Bauch": (14, 0, 0), "Kopf": (-24, 0, 0), "Oberarm.L": (-30, 0, 16), "Unterarm.L": (-8, 0, 0),
              "Oberarm.R": (-40, 0, -24), "Unterarm.R": (-60, 0, 0), "Hand.R": (-20, 0, 0),
              "Oberschenkel.L": (-50, 0, 8), "Unterschenkel.L": (70, 0, 0), "Fuss.L": (-20, 0, 0), "Oberschenkel.R": (-20, 0, -8), "Unterschenkel.R": (60, 0, 0),
              "Fuss.R": (-40, 0, 0), "Becken.pos": (0, 0, -0.2)}
    _clip(armatur, "Rauchwurf", 28, [(0, ruhe), (6, hoch), (10, runter), (16, _mit(runter, Brust=(22, 0, 0))), (28, ruhe)])
    # Schattenruf: die Klinge hoch über den Kopf, Kraft sammeln, dann auf das Ziel richten (Bild 18)
    oben = {**stand, "Brust": (-12, 0, 0), "Bauch": (-5, 0, 0), "Kopf": (-12, 0, 0), "Oberarm.R": (-172, 0, 4), "Unterarm.R": (-8, 0, 0), "Hand.R": (-4, 0, 0),
            "Oberarm.L": (-40, 0, 60), "Unterarm.L": (-20, 0, 0)}
    zeigen = {**stand, "Brust": (14, 8, 0), "Bauch": (6, 4, 0), "Kopf": (-8, -6, 0), "Oberarm.R": (-100, 0, 4), "Unterarm.R": (-4, 0, 0), "Hand.R": (12, 0, 0),
              "Oberarm.L": (20, 0, 30), "Unterarm.L": (-30, 0, 0), "Oberschenkel.L": (-32, 0, 6), "Unterschenkel.L": (36, 0, 0), "Becken.pos": (0, -0.05, -0.07)}
    _clip(armatur, "Schattenruf", 36, [(0, ruhe), (8, oben), (14, _mit(oben, Oberarm_R=(-178, 0, 0), Brust=(-14, 0, 0))), (18, zeigen),
                                       (26, _mit(zeigen, Oberarm_R=(-96, 0, 4))), (36, ruhe)])
