"""Zwerg – zweite Spielfigur (Baukasten: figuren.py).

Klein und stämmig (etwa 1,4 m, mit Helm 1,55 m): Kettenhemd mit rotem Waffenrock und Goldborte,
Schulterpanzer, breiter Gürtel mit Runenschnalle, lederne Armschienen und Handschuhe, schwere
Stiefel mit Stahlkappen. Runder Kopf mit Knollennase, gewaltiger roter Bart mit zwei Zöpfen und
Goldringen, Hörnerhelm mit Nasenschutz, Rundschild auf dem Rücken.
In der rechten Hand: Kriegshammer (Anbauteil „Hammer“), Spitzhacke oder Axt.

Gleiches Skelett (Knochennamen) wie der Magier, eigene Maße; die Animationen sind die des Magiers,
nur „Zaubern“ ist hier das Erdbeben: den Hammer mit beiden Händen hoch und auf den Boden.
Koordinaten: Z oben, die Figur schaut nach -Y, Füße im Ursprung, links (L) ist +X.
"""

import math

import bpy
from mathutils import Matrix, Quaternion, Vector

from figuren import (HAENGT, HOCH, Figur, _axt, _clip, _gehen, _mischen, _mit, _platte, _schleife, _spitzhacke, farbe, weich)
from werkstatt import animation

X, Y, Z = Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))

# Gelenke (Meter). L = +X, R = -X, vorne = -Y.
SCHULTER = Vector((0.25, 0.0, 1.06))
ELLBOGEN = Vector((0.33, 0.02, 0.84))
HANDGELENK = Vector((0.37, -0.01, 0.645))
FINGER = Vector((0.39, -0.03, 0.56))
HUEFTE = Vector((0.12, 0.0, 0.6))
KNIE = Vector((0.13, 0.0, 0.33))
KNOECHEL = Vector((0.13, 0.02, 0.09))
ZEHEN = Vector((0.13, -0.17, 0.03))
GRIFF_X, GRIFF_Y, GRIFF_Z = -0.39, -0.075, 0.6
KOPF = Vector((0.0, -0.005, 1.29))
AUGE = Vector((0.042, -0.1, 1.305))


def _spiegel(p, seite):
    return Vector((p.x * seite, p.y, p.z))


def _rumpf(co):
    z = co.z
    return _mischen(("Becken", 1 - weich(0.68, 0.76, z)), ("Bauch", weich(0.68, 0.76, z) * (1 - weich(0.84, 0.92, z))),
                    ("Brust", weich(0.84, 0.92, z) * (1 - weich(1.08, 1.13, z))), ("Hals", weich(1.08, 1.13, z)))


def _rock(co):
    """Oben am Rumpf, unten schwingt der Rock des Kettenhemds mit den Oberschenkeln."""
    if co.z > 0.62:
        return _rumpf(co)
    bein = weich(0.62, 0.36, co.z)
    links = weich(-0.12, 0.12, co.x)
    return _mischen(("Becken", 1 - bein), ("Oberschenkel.L", bein * links), ("Oberschenkel.R", bein * (1 - links)))


def _arm(seite):
    s = "L" if seite > 0 else "R"
    schulter, ellbogen, hand = _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite)

    def gewichte(co):
        oben = ellbogen - schulter
        t1 = (co - schulter).dot(oben) / oben.length_squared
        unten = hand - ellbogen
        t2 = (co - ellbogen).dot(unten) / unten.length_squared
        if t1 < 0.15:
            return _mischen(("Brust", 1 - weich(-0.1, 0.15, t1)), (f"Oberarm.{s}", weich(-0.1, 0.15, t1)))
        if t2 < 0.0:
            return _mischen((f"Oberarm.{s}", 1 - weich(-0.15, 0.1, t2)), (f"Unterarm.{s}", weich(-0.15, 0.1, t2)))
        if t2 > 0.95:
            return _mischen((f"Unterarm.{s}", 1 - weich(0.95, 1.1, t2)), (f"Hand.{s}", weich(0.95, 1.1, t2)))
        return {f"Unterarm.{s}": 1.0}
    return gewichte


def _kopf_und_brust(von, bis):
    return lambda co: _mischen(("Kopf", weich(von, bis, co.z)), ("Brust", 1 - weich(von, bis, co.z)))


def _verschieben(f, anfang, versatz, skala=1.0, um=Vector()):
    """Teile ab `anfang` um `um` skalieren und dann verschieben (Werkzeuge an die Zwergenhand)."""
    m = Matrix.Translation(um + versatz) @ Matrix.Scale(skala, 4) @ Matrix.Translation(-um)
    for obj in f.teile[anfang:]:
        obj.data.transform(m)


def zwerg(seed=21, name="Zwerg"):
    f = Figur(name, seed)
    r = f.rng
    kette, kette_dunkel = farbe("#6A727C"), farbe("#454B54")
    rot, rot_dunkel = farbe("#A3282B"), farbe("#6E1A1C")
    gold, gold_dunkel = farbe("#D8AE4A"), farbe("#A9812E")
    bronze = farbe("#B07A3A")
    leder, leder_dunkel = farbe("#6A4526"), farbe("#43291A")
    hose, stiefel = farbe("#4E5A3A"), farbe("#3E2A1C")
    stahl, stahl_hell, stahl_dunkel = farbe("#6E7886"), farbe("#A9B3C0"), farbe("#3E444C")
    haut, wange, nase = farbe("#EDBB94"), farbe("#E08F74"), farbe("#E39A80")
    bart, bart_dunkel = farbe("#C2582A"), farbe("#8E3A1A")
    horn, horn_dunkel = farbe("#EDE3CB"), farbe("#A89878")
    holz, holz_dunkel = farbe("#8A5A30"), farbe("#5E3C20")
    kopf_gewicht = lambda co: {"Kopf": 1.0}

    def faltig(staerke, phase=0.0):
        falten = [r.uniform(0.6, 1.4) for _ in range(6)]
        return lambda w: 1.0 + staerke * sum(math.sin(w * (3 + i) + phase + falten[i]) * falten[i] / (3 + i) for i in range(6))

    def vorne_winkel(k, seg):
        w = math.tau * k / seg
        return abs(math.atan2(math.sin(w + math.pi / 2), math.cos(w + math.pi / 2)))

    # Zusätzliche Farben des Rüstungs-Looks
    stahl_mitte = farbe("#8A94A2")
    fell, fell_hell, fell_dunkel = farbe("#7A5638"), farbe("#A8835E"), farbe("#4E3422")
    rune = farbe("#7FE0FF")

    # ================= Kettenhemd (darunter, sichtbar an Armen und als Rock) =================
    hemd_ringe = [
        (Vector((0, 0.012, 1.14)), X, Y, 0.1, 0.085),
        (Vector((0, 0.0, 1.09)), X, Y, 0.21, 0.16),
        (Vector((0, 0.0, 1.0)), X, Y, 0.275, 0.2),
        (Vector((0, -0.012, 0.9)), X, Y, 0.3, 0.245),
        (Vector((0, -0.022, 0.8)), X, Y, 0.31, 0.265),
        (Vector((0, -0.012, 0.7)), X, Y, 0.295, 0.245),
        (Vector((0, 0.0, 0.62)), X, Y, 0.28, 0.22),
        (Vector((0, 0.005, 0.52)), X, Y, 0.3, 0.232, faltig(0.03)),
        (Vector((0, 0.01, 0.42)), X, Y, 0.322, 0.252, faltig(0.05)),
        (Vector((0, 0.01, 0.395)), X, Y, 0.318, 0.248, faltig(0.05)),
    ]
    SEG = 64

    def kette_farbe(i, k, poly):
        z = poly.center.z
        if i >= 8.3:
            return bronze * (0.85 if k % 2 else 1.0)                   # Saum aus Bronzeringen
        reihe = int(z * 70)
        glied = (0.82 if reihe % 2 else 1.0) * (0.9 if (k + reihe) % 2 else 1.0)
        return kette * glied
    f.loft("Kettenhemd", hemd_ringe, SEG, kette_farbe, _rock, teilung=3)
    f.loft("Futter", [(Vector((0, 0.01, 0.4)), X, Y, 0.31, 0.24), (Vector((0, 0.0, 0.55)), X, Y, 0.16, 0.12)], 24,
           lambda i, k, p: leder_dunkel, _rock, oben_zu=True)

    # ================= Brustpanzer mit Goldkanten und leuchtender Rune =================
    def brust_form(w):
        vorne = max(0.0, -math.sin(w))
        return 1.0 + 0.05 * vorne ** 2
    panzer = [
        (Vector((0, -0.01, 0.705)), X, Y, 0.322, 0.268),
        (Vector((0, -0.024, 0.8)), X, Y, 0.33, 0.285),
        (Vector((0, -0.016, 0.9)), X, Y, 0.318, 0.265),
        (Vector((0, -0.004, 1.0)), X, Y, 0.292, 0.226),
        (Vector((0, 0.004, 1.07)), X, Y, 0.245, 0.185),
        (Vector((0, 0.01, 1.105)), X, Y, 0.19, 0.152),
    ]

    def panzer_farbe(i, k, p):
        vorne = vorne_winkel(k + 0.5, 72)
        if i < 0.3 or i > 4.4:
            return gold if (k % 12) else gold_dunkel                     # Goldkanten
        if vorne < 0.05:
            return stahl_hell                                            # Mittelgrat
        # Brustplatten: oben heller, unten dunkler, dazu eine Zierlinie
        if abs(p.center.z - 0.86) < 0.012 and vorne < 1.2:
            return gold_dunkel
        return stahl.lerp(stahl_hell, max(0.0, p.normal.z) * 0.6) * (0.92 + 0.08 * weich(0.7, 1.1, p.center.z))
    f.loft("Brustpanzer", [r_ + (brust_form,) for r_ in panzer], 72, panzer_farbe, _rumpf, teilung=3, glatt=True)
    # Runenstein in goldener Fassung, darum ein Kranz aus Goldzacken
    f.kugel("Runenfassung", (0, -0.3, 0.93), (0.055, 0.02, 0.055), gold, _rumpf, 20, 8, glatt=True)
    f.kugel("Runenstein", (0, -0.318, 0.93), (0.034, 0.012, 0.034), rune, _rumpf, 16, 8, glatt=True)
    for k in range(8):
        w = math.tau * k / 8
        _platte(f, "Runenzacke", Vector((math.cos(w) * 0.065, -0.3, 0.93 + math.sin(w) * 0.065)), 0.04, 0.012, 0.006,
                Vector((math.cos(w), 0, math.sin(w))), -Y, lambda i, k2, p: gold_dunkel, _rumpf, spitz=0.95)

    # ================= Gürtel mit großer Runenschnalle, Taschen, Lederlaschen, Wappenschürze =================
    f.loft("Guertel", [(Vector((0, -0.005, 0.6)), X, Y, 0.298, 0.24), (Vector((0, -0.005, 0.69)), X, Y, 0.31, 0.255)], 56,
           lambda i, k, p: leder * (0.8 if k % 7 == 0 else 1.0), _rumpf, teilung=2, glatt=True)
    for k in range(14):
        w = math.tau * k / 14
        if abs(math.sin(w) + 1) < 0.25:
            continue
        f.kugel("Guertelniete", (math.cos(w) * 0.31, math.sin(w) * 0.255 - 0.005, 0.645), (0.012, 0.012, 0.012), gold, _rumpf, 8, 4)
    f.kiste("Schnalle", (0, -0.262, 0.645), (0.13, 0.022, 0.1), gold, _rumpf)
    f.kiste("Schnallenmitte", (0, -0.274, 0.645), (0.09, 0.012, 0.066), gold_dunkel, _rumpf)
    f.stern("Rune", Vector((0, -0.281, 0.645)), -Y, 0.026, rune, _rumpf, zacken=4)
    for s in (1, -1):
        f.kiste("Tasche", (0.25 * s, -0.12, 0.56), (0.075, 0.055, 0.085), leder * 1.1, _rumpf)
        f.kiste("Taschenklappe", (0.25 * s, -0.148, 0.595), (0.08, 0.012, 0.045), leder_dunkel, _rumpf)
        f.kugel("Taschenknopf", (0.25 * s, -0.156, 0.58), (0.01, 0.006, 0.01), gold, _rumpf, 8, 4)
    # Lederlaschen über den Hüften mit Stahlnieten
    for w in (-2.0, -1.1, 1.1, 2.0, 2.6, -2.6):
        aussen = Vector((math.sin(w), -math.cos(w), 0))
        mitte = Vector((math.sin(w) * 0.3, -math.cos(w) * 0.25, 0.53))
        _platte(f, "Lasche", mitte, 0.2, 0.07, 0.014, Vector((aussen.x * 0.25, aussen.y * 0.25, -1)), aussen,
                lambda i, k, p: leder_dunkel if (i < 0.4 or k in (0, 6)) else leder * (0.95 + 0.05 * (k % 2)), _rock, spitz=0.3, wolbung=0.3)
        f.kugel("Laschenniete", mitte + aussen * 0.02 + Vector((0, 0, 0.05)), (0.01, 0.01, 0.01), stahl_hell, _rock, 8, 4)
    # Rote Wappenschürze vorne mit goldener Borte und Hammer
    schuerze = [Vector((0, -0.285, 0.6)), Vector((0, -0.29, 0.48)), Vector((0, -0.285, 0.34))]
    ringe = [(p, X, Y, 0.1 - 0.01 * j, 0.008) for j, p in enumerate(schuerze)]
    f.loft("Wappenschuerze", ringe, 16, lambda i, k, p: gold if (k in (0, 1, 7, 8, 9, 15) or i > 1.7) else rot * (0.95 + 0.05 * (k % 2)),
           _rock, oben_zu=True, unten_zu=True, teilung=3, glatt=True)
    f.kiste("Wappenstiel", (0, -0.298, 0.47), (0.014, 0.006, 0.08), gold, _rock)
    f.kiste("Wappenkopf", (0, -0.3, 0.505), (0.055, 0.007, 0.024), gold, _rock)
    hinten = [Vector((0, 0.24, 0.6)), Vector((0, 0.25, 0.46)), Vector((0, 0.245, 0.36))]
    f.loft("Rueckenschuerze", [(p, X, Y, 0.09, 0.008) for p in hinten], 16, lambda i, k, p: rot_dunkel if k not in (0, 8) else gold_dunkel,
           _rock, oben_zu=True, unten_zu=True, teilung=3, glatt=True)

    # ================= Pelzkragen und geschichtete Schulterpanzer =================
    def fell_form(w):
        return 1.0 + 0.1 * abs(math.sin(w * 9)) + 0.04 * math.sin(w * 23)
    kragen = [(Vector((0, 0.01, 1.03)), X, Y, 0.27, 0.215, fell_form), (Vector((0, 0.01, 1.1)), X, Y, 0.25, 0.2, fell_form),
              (Vector((0, 0.015, 1.16)), X, Y, 0.16, 0.14, fell_form), (Vector((0, 0.015, 1.18)), X, Y, 0.1, 0.09)]
    f.loft("Pelzkragen", kragen, 54, lambda i, k, p: (fell_hell if (k % 9) in (0, 1) else fell) * (0.8 if i < 0.4 else 1.0),
           _rumpf, teilung=2, glatt=True)
    for seite in (1, -1):
        s = "L" if seite > 0 else "R"
        schulter = _spiegel(SCHULTER, seite)
        gewicht = lambda co, s=s: _mischen((f"Oberarm.{s}", 0.7), ("Brust", 0.3))
        aussen = Vector((seite, 0, 0))
        f.kugel("Schulterkuppel", schulter + Vector((0.03 * seite, 0, 0.05)), (0.15, 0.16, 0.09),
                lambda poly: stahl.lerp(stahl_hell, max(0.0, poly.normal.z)), gewicht, 24, 12, glatt=True)
        f.loft("Kuppelrand", [(schulter + Vector((0.03 * seite, 0, 0.03)), X, Y, 0.152, 0.162), (schulter + Vector((0.03 * seite, 0, 0.055)), X, Y, 0.147, 0.157)],
               28, lambda i, k, p: gold, gewicht, teilung=1, glatt=True)
        for j in range(3):
            mitte = schulter + Vector((0.11 * seite + 0.02 * seite * j, 0, -0.01 - 0.045 * j))
            _platte(f, "Schulterlamelle", mitte, 0.3 - 0.03 * j, 0.075 - 0.008 * j, 0.016, Y, aussen + Vector((0, 0, 0.8)),
                    lambda i, k, p, j=j: (gold if k in (0, 6, 11) or i > 5.3 else stahl_mitte * (0.95 - 0.05 * j)), gewicht, spitz=0.35, wolbung=1.6)
        f.stern("Schulterrune", schulter + Vector((0.07 * seite, -0.02, 0.135)), Vector((0.4 * seite, -0.1, 1)), 0.03, rune, gewicht, zacken=4)
        for dy in (-0.09, 0.0, 0.09):
            f.kugel("Kuppelniete", schulter + Vector((0.13 * seite, dy, 0.05)), (0.012, 0.012, 0.012), gold, gewicht, 8, 4)

    # ================= Arme: Kettenärmel, Armschienen mit Stahlplatte, Panzerhandschuhe =================
    for seite in (1, -1):
        s_, e, h = _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite)
        ringe = []
        for t, (a, b), radius in ((0.1, (s_, e), 0.09), (0.6, (s_, e), 0.085), (1.05, (s_, e), 0.082)):
            achse = (b - a).normalized()
            q = achse.cross(Y).normalized()
            ringe.append((a.lerp(b, t), q, achse.cross(q).normalized(), radius, radius * 0.95))
        f.loft("Aermel", ringe, 20, lambda i, k, p: kette * (0.82 if int(p.center.z * 70) % 2 else 1.0), _arm(seite), teilung=2)
        ringe = []
        for t, radius in ((0.0, 0.08), (0.35, 0.09), (0.8, 0.088), (1.02, 0.08)):
            achse = (h - e).normalized()
            q = achse.cross(Y).normalized()
            ringe.append((e.lerp(h, t), q, achse.cross(q).normalized(), radius, radius * 0.92))
        f.loft("Armschiene", ringe, 20, lambda i, k, p: leder_dunkel if i < 0.25 or i > 2.7 else leder * (0.88 if k % 5 == 0 else 1.0),
               _arm(seite), unten_zu=True, teilung=3, glatt=True)
        achse = (h - e).normalized()
        _platte(f, "Armplatte", e.lerp(h, 0.5) + Vector((0.07 * seite, 0.0, 0.0)), 0.2, 0.05, 0.014, -achse, Vector((seite, 0, 0)),
                lambda i, k, p: gold if i < 0.4 or i > 5.4 else stahl_mitte, _arm(seite), spitz=0.5, wolbung=1.0)

    def faust(griff, seite, dicke):
        """Handschuh als Faust: Handteller, vier gekrümmte Finger, Daumen darüber."""
        formen = [(griff + Vector((0.028 * seite, 0.01, 0.03)), (0.036, 0.044, 0.048))]
        for i in range(4):
            z = griff.z + 0.02 - 0.022 * i
            for j, w in enumerate((0.4, -0.5, -1.5, -2.5)):
                rad = 0.036 - 0.002 * j
                formen.append((Vector((griff.x + math.cos(w) * rad * seite, griff.y + math.sin(w) * rad, z)), (dicke,) * 3))
        for p in (griff + Vector((0.03 * seite, -0.03, 0.035)), griff + Vector((0.01 * seite, -0.045, 0.04)), griff + Vector((-0.012 * seite, -0.04, 0.042))):
            formen.append((p, (dicke * 1.1,) * 3))
        formen.append((griff + Vector((0.025 * seite, 0.012, 0.075)), (0.046, 0.048, 0.034)))    # Stulpe
        return formen

    for seite in (1, -1):
        s = "L" if seite > 0 else "R"
        griff = Vector((0.39, -0.03, 0.57)) if seite > 0 else Vector((GRIFF_X, GRIFF_Y, GRIFF_Z))
        f.metaball(f"Handschuh{s}", faust(griff, -seite, 0.011), 0.004, 2200,
                   lambda poly: leder_dunkel * (1.15 if poly.normal.z > 0.5 else 1.0), lambda co, s=s: {f"Hand.{s}": 1.0}, glatt=True)
        # Stahlplatte über den Knöcheln
        f.kugel("Knoechelplatte", griff + Vector((0.03 * seite, -0.005, 0.02)), (0.022, 0.04, 0.034),
                lambda poly: stahl.lerp(stahl_hell, max(0.0, poly.normal.z)), lambda co, s=s: {f"Hand.{s}": 1.0}, 12, 8, glatt=True)

    # ================= Beine: Hose, schwere Stiefel mit Pelzstulpe und Stahlkappe =================
    for seite in (1, -1):
        sn = "L" if seite > 0 else "R"
        k_, h_ = _spiegel(KNIE, seite), _spiegel(HUEFTE, seite)
        ringe = [(Vector((0.13 * seite, 0.01, 0.26)), X, Y, 0.078, 0.078), (k_, X, Y, 0.088, 0.088), (h_ + Vector((0, 0, -0.03)), X, Y, 0.112, 0.108)]
        f.loft("Hose", ringe, 16, lambda i, k, p: hose * (0.9 if k % 4 == 0 else 1.0),
               lambda co, sn=sn: _mischen((f"Unterschenkel.{sn}", 1 - weich(0.3, 0.38, co.z)), (f"Oberschenkel.{sn}", weich(0.3, 0.38, co.z))), glatt=True)
        schaft = [(Vector((0.13 * seite, 0.03, 0.06)), X, Y, 0.08, 0.085), (Vector((0.13 * seite, 0.02, 0.17)), X, Y, 0.084, 0.084),
                  (Vector((0.13 * seite, 0.01, 0.24)), X, Y, 0.09, 0.09)]
        f.loft("Stiefelschaft", schaft, 18, lambda i, k, p: stiefel * (0.9 if k % 6 == 0 else 1.0), lambda co, sn=sn: {f"Unterschenkel.{sn}": 1.0},
               unten_zu=True, teilung=2, glatt=True)
        stulpe = [(Vector((0.13 * seite, 0.01, 0.23)), X, Y, 0.1, 0.1, fell_form), (Vector((0.13 * seite, 0.01, 0.29)), X, Y, 0.112, 0.112, fell_form),
                  (Vector((0.13 * seite, 0.01, 0.31)), X, Y, 0.09, 0.09)]
        f.loft("Pelzstulpe", stulpe, 30, lambda i, k, p: fell_hell if (k % 7) in (0, 1) else fell, lambda co, sn=sn: {f"Unterschenkel.{sn}": 1.0},
               teilung=2, glatt=True)
        fuss = [(Vector((0.13 * seite, 0.075, 0.05)), X, Z, 0.074, 0.05), (Vector((0.13 * seite, 0.07, 0.022)), X, Z, 0.082, 0.03),
                (Vector((0.13 * seite, -0.02, 0.058)), X, Z, 0.09, 0.064), (Vector((0.13 * seite, -0.11, 0.052)), X, Z, 0.084, 0.05),
                (Vector((0.13 * seite, -0.165, 0.05)), X, Z, 0.062, 0.042), (Vector((0.13 * seite, -0.195, 0.055)), X, Z, 0.032, 0.03),
                (Vector((0.13 * seite, -0.205, 0.06)), X, Z, 0.004, 0.004)]

        def stiefel_farbe(i, k, p):
            if p.center.z < 0.012:
                return stiefel * 0.5
            if i >= 3.3:
                return stahl.lerp(stahl_hell, max(0.0, p.normal.z))          # Stahlkappe
            return stiefel
        f.loft("Stiefel", fuss, 18, stiefel_farbe, lambda co, sn=sn: {f"Fuss.{sn}": 1.0}, oben_zu=True, unten_zu=True, teilung=3, glatt=True)

    # ================= Kopf =================
    kopf = [
        (KOPF + Vector((0, 0.01, 0.0)), (0.1, 0.108, 0.112)),              # Schädel
        (KOPF + Vector((0, -0.025, -0.07)), (0.085, 0.078, 0.07)),         # Kiefer und Wangen
        (KOPF + Vector((0.055, -0.072, -0.035)), (0.035, 0.028, 0.025)),   # Wangenknochen
        (KOPF + Vector((-0.055, -0.072, -0.035)), (0.035, 0.028, 0.025)),
        (KOPF + Vector((0, -0.088, 0.03)), (0.074, 0.03, 0.022)),          # Brauenbogen
        (KOPF + Vector((0, -0.11, -0.01)), (0.018, 0.026, 0.03)),          # Nasenrücken
        (KOPF + Vector((0, -0.13, -0.035)), (0.03, 0.03, 0.028)),          # dicke Knollennase
        (KOPF + Vector((0.024, -0.122, -0.045)), (0.017, 0.018, 0.016)),   # Nasenflügel
        (KOPF + Vector((-0.024, -0.122, -0.045)), (0.017, 0.018, 0.016)),
        (KOPF + Vector((0, 0.0, -0.13)), (0.06, 0.06, 0.06)),              # Hals
    ]
    for s in (1, -1):
        auge = _spiegel(AUGE, s)
        kopf += [
            (auge + Vector((0, -0.006, 0)), (0.02, 0.014, 0.014), True),               # Augenhöhle
            (auge + Vector((0, -0.007, 0.011)), (0.019, 0.01, 0.006)),                 # Oberlid
            (Vector((0.102 * s, 0.01, KOPF.z - 0.005)), (0.016, 0.03, 0.04)),          # Ohr
        ]

    def gesicht(poly):
        p = poly.center
        c = haut
        nasenspitze = (p - (KOPF + Vector((0, -0.145, -0.035)))).length
        if nasenspitze < 0.03:
            return c.lerp(nase, 0.7)
        rosig = math.exp(-(((abs(p.x) - 0.058) ** 2 + (p.z - KOPF.z + 0.035) ** 2) / 0.02 ** 2)) if p.y < -0.05 else 0.0
        return c.lerp(wange, rosig * 0.75)

    f.metaball("Kopf", kopf, 0.0048, 4200, gesicht,
               lambda co: _mischen(("Kopf", weich(1.12, 1.18, co.z)), ("Hals", 1 - weich(1.12, 1.18, co.z))), glatt=True)
    for s in (1, -1):
        auge = _spiegel(AUGE, s)
        f.kugel("Augapfel", auge, (0.013, 0.013, 0.013), farbe("#EDE6DA"), kopf_gewicht, 16, 12)
        f.kugel("Iris", auge + Vector((-0.001 * s, -0.011, 0.0005)), (0.0072, 0.003, 0.0072), farbe("#3E6A4A"), kopf_gewicht, 14, 8)
        f.kugel("Pupille", auge + Vector((-0.001 * s, -0.0128, 0.0005)), (0.0036, 0.0016, 0.0036), farbe("#0E1116"), kopf_gewicht, 10, 6)
        f.kugel("Glanz", auge + Vector((-0.004 * s, -0.0134, 0.004)), (0.0017, 0.0008, 0.0017), farbe("#FFFFFF"), kopf_gewicht, 8, 4)
    # Buschige rote Brauen
    for s in (1, -1):
        for j in range(8):
            x = 0.014 + 0.01 * j
            basis = Vector((x * s, -0.108 + 1.2 * (x - 0.03) ** 2, KOPF.z + 0.026 + 0.006 * math.sin(math.pi * x / 0.09)))
            richtung = Vector((0.7 * s, -0.3 - r.uniform(0, 0.2), 0.35 + r.uniform(-0.1, 0.25))).normalized()
            laenge = 0.02 + 0.02 * (j / 7) + r.uniform(0, 0.006)
            f.straehne("Braue", [basis, basis + richtung * laenge * 0.55 + Vector((0, -0.004, 0.003)), basis + richtung * laenge],
                       0.0065, 0.001, bart * r.uniform(0.9, 1.05), kopf_gewicht, 6, 0.4, 0.7)

    # ================= Bart: voll und rund, von Ohr zu Ohr, in Locken über die Brust, zwei beringte Zöpfe =================
    bart_gewicht = _kopf_und_brust(0.95, 1.14)
    kupfer, kupfer_dunkel, kupfer_hell = farbe("#7E4028"), farbe("#4E2414"), farbe("#A8603A")
    # Ringe (Höhe, Mitte vorne/hinten, halbe Breite, halbe Tiefe): am Kiefer rund um das Kinn,
    # dann weit nach vorne über den Brustpanzer gewölbt und zur Spitze schmal
    profil = [(1.245, -0.035, 0.1, 0.1), (1.2, -0.06, 0.118, 0.105), (1.12, -0.12, 0.132, 0.1), (1.04, -0.18, 0.132, 0.094),
              (0.97, -0.23, 0.118, 0.082), (0.91, -0.262, 0.092, 0.066), (0.865, -0.28, 0.058, 0.046), (0.835, -0.288, 0.018, 0.014)]
    ringe = []
    for j, (z, y, rx, ry) in enumerate(profil):
        t = j / (len(profil) - 1)
        form = lambda w, t=t: 1.0 + 0.09 * math.sin(w * 6 + t * 2.5) + 0.04 * math.sin(w * 13 + t) + 0.05 * t * math.sin(w * 3)
        ringe.append((Vector((0, y, z)), X, Y, rx, ry, form))

    def bart_farbe(i, k, p):
        # breite Locken: jede zweite etwas dunkler, obenauf Glanzlichter
        locke = (k // 3) % 4
        grund = (kupfer, kupfer * 0.9, kupfer_dunkel * 1.25, kupfer * 1.05)[locke]
        if p.normal.z > 0.35 and locke != 2:
            grund = grund.lerp(kupfer_hell, 0.45)
        return grund * (0.9 + 0.1 * max(0.0, -p.normal.y))
    f.loft("Bart", ringe, 48, bart_farbe, bart_gewicht, oben_zu=False, unten_zu=False, teilung=3, glatt=True)
    # Große Locken auf der Oberfläche: brechen die Kontur auf und fallen unten und seitlich heraus
    for n in range(18):
        w = math.radians(r.uniform(-140, 140))
        hoehe = r.uniform(0.0, 0.7)
        j = int(hoehe * (len(profil) - 1))
        z, y, rx, ry = profil[j]
        start = Vector((math.sin(w) * rx * 0.95, y - math.cos(w) * ry * 0.95, z + 0.01))
        naechster = profil[min(j + 2, len(profil) - 1)]
        ziel = Vector((math.sin(w) * naechster[2] * 1.02, naechster[1] - math.cos(w) * naechster[3] * 1.02, naechster[0] - 0.03))
        mitte = start.lerp(ziel, 0.5) + Vector((math.sin(w) * 0.008, -math.cos(w) * 0.008, 0))
        spitze = ziel + Vector((math.sin(w) * 0.006, -0.004, -0.03))
        dicke = r.uniform(0.028, 0.04)
        farbe_locke = (kupfer, kupfer_hell * 0.92, kupfer * 0.88)[n % 3]
        f.straehne("Bartlocke", [start, mitte, ziel, spitze], dicke, 0.003, farbe_locke, bart_gewicht, 9, 0.6, 0.62, glatt=True)
    # Koteletten vom Helmrand bis zum Kiefer
    for s in (1, -1):
        punkte = [KOPF + Vector((0.098 * s, 0.0, 0.02)), KOPF + Vector((0.103 * s, -0.02, -0.04)), KOPF + Vector((0.098 * s, -0.045, -0.09))]
        f.straehne("Kotelette", punkte, 0.03, 0.02, kupfer * 0.95, kopf_gewicht, 10, 0.3, 0.6, glatt=True)
    # Kräftiger Schnurrbart, der seitlich weit herabhängt
    for s in (1, -1):
        for j, (breite, tief) in enumerate(((0.03, 0.0), (0.024, 0.012))):
            punkte = [KOPF + Vector((0.008 * s, -0.148, -0.045 - tief)), KOPF + Vector((0.05 * s, -0.15, -0.058 - tief)),
                      KOPF + Vector((0.095 * s, -0.13, -0.1 - tief)), KOPF + Vector((0.115 * s, -0.12, -0.16 - tief)),
                      KOPF + Vector((0.118 * s, -0.125, -0.21 - tief))]
            f.straehne("Schnurrbart", punkte, breite, 0.005, kupfer_hell * 0.95 if j == 0 else kupfer, kopf_gewicht, 10, 0.5, 0.75, glatt=True)
    # Zwei dicke, geflochtene Zöpfe aus der Bartspitze, je drei Goldringe und eine Goldkappe
    for s in (1, -1):
        oben = Vector((0.04 * s, -0.285, 0.9))
        punkte = [oben + Vector((0.012 * s * j, -0.006 * j, -0.045 * j)) for j in range(7)]
        f.straehne("Zopf", punkte, 0.042, 0.02, lambda i, k, p: kupfer_dunkel * 1.2 if (int(i * 5) + k) % 3 == 0 else kupfer, bart_gewicht, 12, 1.6, 0.9,
                   glatt=True)
        for j in (1.2, 2.9, 4.6):
            mitte = punkte[int(j)].lerp(punkte[int(j) + 1], j % 1)
            f.loft("Bartring", [(mitte + Vector((0, 0, -0.016)), X, Y, 0.04, 0.04), (mitte + Vector((0, 0, 0.016)), X, Y, 0.04, 0.04)], 18,
                   lambda i, k, p: gold if k % 5 else gold_dunkel, bart_gewicht, oben_zu=True, unten_zu=True, glatt=True)
        f.kugel("Zopfkappe", punkte[-1] + Vector((0, -0.004, -0.024)), (0.026, 0.026, 0.036), gold, bart_gewicht, 12, 8, glatt=True)
    # Zöpfe am Hinterkopf unter dem Helm
    for s in (-1, 0, 1):
        start = Vector((0.06 * s, 0.07, KOPF.z + 0.02))
        punkte = [start + Vector((0.01 * s * t, 0.03 * t + 0.02 * t * t, -0.3 * t)) for t in (0.0, 0.25, 0.5, 0.75, 1.0)]
        f.straehne("Haarzopf", punkte, 0.024, 0.01, lambda i, k, p: kupfer_dunkel * 1.2 if (int(i * 4) + k) % 3 == 0 else kupfer, _kopf_und_brust(1.0, 1.2), 8, 1.2, 0.85,
                   glatt=True)

    # ================= Helm: Stahlkuppel mit Goldkamm, Wangenschutz, Nasenschutz, große Hörner =================
    helm = []
    for i, (dz, rx, ry) in enumerate(((0.0, 0.122, 0.13), (0.012, 0.124, 0.132), (0.05, 0.12, 0.127), (0.09, 0.104, 0.11), (0.125, 0.078, 0.083),
                                      (0.15, 0.046, 0.05), (0.162, 0.012, 0.012))):
        helm.append((KOPF + Vector((0, 0.005, 0.03 + dz)), X, Y, rx, ry))

    def helm_farbe(i, k, p):
        if i < 1.0:
            return gold if i < 0.6 else gold_dunkel                          # Goldband am Rand
        if k % 16 in (4, 12):
            return stahl_mitte                                               # Nietenlinien
        return stahl.lerp(stahl_hell, max(0.0, p.normal.z) * 0.8)
    f.loft("Helm", helm, 40, helm_farbe, kopf_gewicht, oben_zu=True, teilung=3, glatt=True)
    for k in range(10):
        w = math.tau * k / 10
        f.kugel("Helmniete", KOPF + Vector((math.cos(w) * 0.125, 0.005 + math.sin(w) * 0.133, 0.042)), (0.009, 0.009, 0.009), gold, kopf_gewicht, 8, 4)
    # Goldkamm von der Stirn über den Scheitel nach hinten
    kamm = []
    for j in range(11):
        a = math.radians(-70 + 150 * j / 10)
        p = KOPF + Vector((0, 0.005 + math.sin(a) * 0.14, 0.035 + math.cos(a) * 0.15))
        hoehe = 0.035 * math.sin(math.pi * min(1.0, (j + 0.5) / 10)) + 0.008
        kamm.append((p, Y, Vector((0, math.sin(a), math.cos(a))), 0.012, hoehe))
    f.loft("Helmkamm", kamm, 8, lambda i, k, p: gold if k % 4 else gold_dunkel, kopf_gewicht, oben_zu=True, unten_zu=True, teilung=2, glatt=True)
    _platte(f, "Nasenschutz", KOPF + Vector((0, -0.134, -0.01)), 0.11, 0.018, 0.012, -Z, -Y, lambda i, k, p: stahl_hell if i < 5 else gold, kopf_gewicht,
            spitz=0.6, wolbung=0.5)
    for s in (1, -1):
        _platte(f, "Wangenschutz", KOPF + Vector((0.105 * s, -0.05, -0.02)), 0.13, 0.045, 0.012, Vector((0.1 * s, -0.25, -1)), Vector((s, -0.3, 0)),
                lambda i, k, p: gold if k in (0, 6) else stahl, kopf_gewicht, spitz=0.5, wolbung=0.6)
        wurzel = KOPF + Vector((0.11 * s, 0.0, 0.1))
        punkte = [wurzel, wurzel + Vector((0.08 * s, -0.005, 0.02)), wurzel + Vector((0.15 * s, -0.025, 0.08)), wurzel + Vector((0.18 * s, -0.05, 0.16)),
                  wurzel + Vector((0.175 * s, -0.07, 0.24)), wurzel + Vector((0.15 * s, -0.085, 0.29))]
        f.straehne("Horn", punkte, 0.042, 0.004, lambda i, k, p: horn_dunkel if i > 3.8 else horn * (0.93 + 0.07 * (int(i * 4) % 2)), kopf_gewicht, 14, 0.0, 1.0,
                   teilung=3, glatt=True)
        for t in (0.35, 1.1):
            mitte = punkte[int(t)].lerp(punkte[int(t) + 1], t % 1)
            richtung = (punkte[int(t) + 1] - punkte[int(t)]).normalized()
            q1 = richtung.cross(Z).normalized()
            q2 = richtung.cross(q1).normalized()
            dicke = 0.045 - 0.01 * t
            f.loft("Hornring", [(mitte - richtung * 0.012, q1, q2, dicke, dicke), (mitte + richtung * 0.012, q1, q2, dicke, dicke)], 16,
                   lambda i, k, p: gold, kopf_gewicht, oben_zu=True, unten_zu=True, glatt=True)

    # ================= Rundschild auf dem Rücken =================
    schild_mitte = Vector((0.0, 0.3, 0.86))
    neig = Quaternion(Z, math.radians(8)) @ Quaternion(X, math.radians(-12))
    achse = neig @ Y
    a1, a2 = neig @ X, neig @ Z
    ringe = [(schild_mitte - achse * 0.024, a1, a2, 0.29, 0.29), (schild_mitte - achse * 0.02, a1, a2, 0.3, 0.3),
             (schild_mitte + achse * 0.028, a1, a2, 0.3, 0.3), (schild_mitte + achse * 0.034, a1, a2, 0.285, 0.285)]

    def schild_farbe(i, k, p):
        d = (p.center - schild_mitte)
        radial = (d - achse * d.dot(achse)).length
        if radial > 0.27:
            return stahl.lerp(stahl_hell, 0.3)                                # Stahlrand
        planke = int((d.dot(a1) + 0.3) / 0.075)
        return (holz if planke % 2 else holz_dunkel) * (1.0 if radial < 0.2 or (planke + int(radial * 30)) % 3 else 0.9)
    f.loft("Schild", ringe, 40, schild_farbe, _rumpf, oben_zu=True, unten_zu=True, teilung=1)
    buckel = [(schild_mitte + achse * 0.03, a1, a2, 0.085, 0.085), (schild_mitte + achse * 0.062, a1, a2, 0.068, 0.068),
              (schild_mitte + achse * 0.085, a1, a2, 0.035, 0.035)]
    f.loft("Schildbuckel", buckel, 20, lambda i, k, p: gold if i > 1 else gold_dunkel, _rumpf, oben_zu=True, glatt=True)
    f.stern("Schildrune", schild_mitte + achse * 0.09, achse, 0.03, rune, _rumpf, zacken=4)
    for k in range(10):
        w = math.tau * k / 10
        f.kugel("Schildniete", schild_mitte + achse * 0.036 + (a1 * math.cos(w) + a2 * math.sin(w)) * 0.255, (0.013, 0.013, 0.013), gold, _rumpf, 8, 4)
    # Gurt über der Brust
    gurt = [Vector((0.24, -0.2, 1.04)), Vector((0.1, -0.31, 0.9)), Vector((-0.1, -0.32, 0.76)), Vector((-0.26, -0.24, 0.66))]
    f.loft("Gurt", [(p, X, Y, 0.03, 0.007) for p in gurt], 8, lambda i, k, p: leder_dunkel, _rumpf, oben_zu=True, unten_zu=True, teilung=3)

    # ================= Kriegshammer (rechte Hand, hängt wie die Spitzhacke mit dem Kopf nach unten) =================
    hammer_anfang = len(f.teile)
    hand_gewicht = lambda co: {"Hand.R": 1.0}
    stiel = []
    for i in range(14):
        z = 0.12 + 0.64 * i / 13
        stiel.append((Vector((GRIFF_X, GRIFF_Y, z)), X, Y, 0.021 - 0.002 * i / 13, 0.021 - 0.002 * i / 13))

    def stiel_farbe(i, k, p):
        z = p.center.z
        if 0.54 < z < 0.7:
            return leder_dunkel * (0.75 if int(z * 90) % 2 else 1.0)       # Ledergriff
        if any(abs(z - b) < 0.012 for b in (0.3, 0.52, 0.72)):
            return gold                                                  # Goldringe
        return holz * (0.82 + 0.18 * ((k * 3 + int(i * 5)) % 4) / 3)

    f.loft("Hammerstiel", stiel, 10, stiel_farbe, hand_gewicht, oben_zu=True, unten_zu=True, teilung=2)
    f.kugel("Knauf", (GRIFF_X, GRIFF_Y, 0.775), (0.03, 0.03, 0.03), gold, hand_gewicht, 10, 6, glatt=False)
    kopf_z = 0.2
    kopf_profil = [(-0.15, 0.07, 0.075), (-0.13, 0.08, 0.085), (-0.06, 0.07, 0.075), (0.0, 0.075, 0.08), (0.06, 0.07, 0.075),
                   (0.13, 0.08, 0.085), (0.15, 0.07, 0.075)]
    ringe = [(Vector((GRIFF_X, GRIFF_Y + dy, kopf_z)), X, Z, bx, bz, lambda w: 1.0 / max(abs(math.cos(w)), abs(math.sin(w))) ** 0.9)
             for dy, bx, bz in kopf_profil]

    def kopf_farbe(i, k, p):
        if 2.4 < i < 3.6:
            return gold if k % 2 else gold_dunkel                        # Runenband in der Mitte
        if i < 0.5 or i > 5.5:
            return stahl_hell                                            # Schlagflächen
        return stahl * (0.92 + 0.08 * (k % 2))

    f.loft("Hammerkopf", ringe, 16, kopf_farbe, hand_gewicht, oben_zu=True, unten_zu=True, teilung=2)
    f.stern("Hammerrune", Vector((GRIFF_X - 0.083, GRIFF_Y, kopf_z)), -X, 0.03, farbe("#7FD0FF"), hand_gewicht, zacken=4)
    f.stern("Hammerrune", Vector((GRIFF_X + 0.083, GRIFF_Y, kopf_z)), X, 0.03, farbe("#7FD0FF"), hand_gewicht, zacken=4)
    f.loft("Hammerspitze", [(Vector((GRIFF_X, GRIFF_Y, kopf_z - 0.075)), X, Y, 0.03, 0.03), (Vector((GRIFF_X, GRIFF_Y, kopf_z - 0.13)), X, Y, 0.003, 0.003)],
           8, lambda i, k, p: stahl_hell, hand_gewicht, unten_zu=True)
    f.als_starr("Hammer", "Hand.R", hammer_anfang)

    # ================= Erbeutbare Hämmer (waffen.py), im Spiel statt des Hammers sichtbar =================
    from waffen import HAEMMER, hammer as hammer_bauen
    for art in HAEMMER:
        anfang = len(f.teile)
        hammer_bauen(f, art, GRIFF_X, GRIFF_Y, hand_gewicht, GRIFF_Z)
        f.als_starr(art, "Hand.R", anfang)

    # ================= Spitzhacke und Axt (vom Magier, an die Zwergenhand gesetzt) =================
    magier_hand = Vector((-0.37, -0.075, 0.93))
    zwergen_hand = Vector((GRIFF_X, GRIFF_Y, GRIFF_Z + 0.02))
    hacke_anfang = len(f.teile)
    _spitzhacke(f, magier_hand.x, magier_hand.y, hand_gewicht)
    _verschieben(f, hacke_anfang, zwergen_hand - magier_hand, 0.85, magier_hand)
    f.als_starr("Spitzhacke", "Hand.R", hacke_anfang)
    axt_anfang = len(f.teile)
    _axt(f, magier_hand.x, magier_hand.y, hand_gewicht)
    _verschieben(f, axt_anfang, zwergen_hand - magier_hand, 0.85, magier_hand)
    f.als_starr("Axt", "Hand.R", axt_anfang)

    # ================= Skelett (Namen wie beim Magier) =================
    f.knochen_dazu("Becken", (0, 0, 0.6), (0, 0, 0.72), None, HOCH)
    f.knochen_dazu("Bauch", (0, 0, 0.72), (0, 0, 0.88), "Becken", HOCH)
    f.knochen_dazu("Brust", (0, 0, 0.88), (0, 0, 1.1), "Bauch", HOCH)
    f.knochen_dazu("Hals", (0, 0, 1.1), (0, 0, 1.18), "Brust", HOCH)
    f.knochen_dazu("Kopf", (0, 0, 1.18), (0, 0, 1.42), "Hals", HOCH)
    f.knochen_dazu("Hut", (0, 0.02, 1.46), (0, 0.1, 1.6), "Kopf", HOCH)
    for seite, sn in ((1, "L"), (-1, "R")):
        f.knochen_dazu(f"Oberarm.{sn}", _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), "Brust", HAENGT)
        f.knochen_dazu(f"Unterarm.{sn}", _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite), f"Oberarm.{sn}", HAENGT)
        f.knochen_dazu(f"Hand.{sn}", _spiegel(HANDGELENK, seite), _spiegel(FINGER, seite), f"Unterarm.{sn}", HAENGT)
        f.knochen_dazu(f"Oberschenkel.{sn}", _spiegel(HUEFTE, seite), _spiegel(KNIE, seite), "Becken", HAENGT)
        f.knochen_dazu(f"Unterschenkel.{sn}", _spiegel(KNIE, seite), _spiegel(KNOECHEL, seite), f"Oberschenkel.{sn}", HAENGT)
        f.knochen_dazu(f"Fuss.{sn}", _spiegel(KNOECHEL, seite), _spiegel(ZEHEN, seite), f"Unterschenkel.{sn}", (0, 0, 1))

    return f.fertig(_zwerg_animationen)


def _zwerg_animationen(armatur):
    """Wie der Magier (figuren.py), mit breitem Stand im Stehen und dem Erdbeben als „Zaubern“."""
    from figuren import _magier_animationen
    _magier_animationen(armatur)
    # Die folgenden ersetzen die des Magiers (sonst gäbe es „Idle“ und „Idle.001“)
    for alt in ("Idle", "Laufen", "Rennen", "Zaubern", "Arkan", "Feuerball", "Frostnova", "Meteor"):
        aktion = bpy.data.actions.get(alt)
        if aktion:
            bpy.data.actions.remove(aktion)

    # Idle: breitbeinig, Brust raus, der Hammer ruht in der Hand
    def idle(phi):
        return [
            ("Brust", "rot", (-3 + 1.5 * math.sin(phi * 2), 0, 0)), ("Bauch", "rot", (-0.8 * math.sin(phi * 2), 0, 0)),
            ("Kopf", "rot", (2 * math.sin(phi * 2 + 1), 12 * math.sin(phi), 2 * math.sin(phi))),
            ("Oberarm.L", "rot", (3 * math.sin(phi * 2), 0, 8)), ("Unterarm.L", "rot", (-14 - 3 * math.sin(phi * 2), 0, 0)),
            ("Oberarm.R", "rot", (-6, 0, -6)), ("Unterarm.R", "rot", (-20, 0, 0)),
            ("Oberschenkel.L", "rot", (0, 0, 6)), ("Oberschenkel.R", "rot", (0, 0, -6)),
            ("Becken", "pos", (0, -0.004 * (1 - math.cos(phi * 2)), 0)),
        ]
    animation(armatur, "Idle", 180, _schleife(180, 6, idle))
    animation(armatur, "Laufen", 30, _schleife(30, 2, lambda phi: _gehen(phi, 30, 48, 26, 0.025, 5, 18)))
    animation(armatur, "Rennen", 20, _schleife(20, 2, lambda phi: _gehen(phi, 46, 84, 44, 0.05, 16, 58)))

    # Zaubern = Erdbeben: Hammer mit beiden Händen hoch über den Kopf, kraftvoll auf den Boden
    # schlagen (Einschlag bei Bild 13, 0,45 s bei Tempo 1,2), in die Knie gehen, aufrichten.
    beben = []
    for bild, arm_r, ell_r, arm_l, ell_l, brust_x, knie, senken in (
            (0, -6, -20, 0, -14, 0, 6, 0.0), (8, -175, -25, -160, -45, -14, 12, 0.02), (11, -130, -10, -125, -25, 10, 20, -0.02),
            (13, -40, -4, -45, -10, 34, 42, -0.1), (18, -38, -8, -42, -14, 30, 38, -0.09), (30, -6, -20, 0, -14, 0, 6, 0.0)):
        beben += [(bild, "Oberarm.R", "rot", (arm_r, 0, 0)), (bild, "Unterarm.R", "rot", (ell_r, 0, 0)),
                  (bild, "Oberarm.L", "rot", (arm_l, 0, -14)), (bild, "Unterarm.L", "rot", (ell_l, 0, 0)),
                  (bild, "Brust", "rot", (brust_x, 0, 0)), (bild, "Bauch", "rot", (brust_x * 0.5, 0, 0)),
                  (bild, "Kopf", "rot", (-brust_x * 0.4, 0, 0)),
                  (bild, "Oberschenkel.L", "rot", (-knie * 0.8, 0, 8)), (bild, "Unterschenkel.L", "rot", (knie, 0, 0)),
                  (bild, "Oberschenkel.R", "rot", (-knie * 0.5, 0, -8)), (bild, "Unterschenkel.R", "rot", (knie * 0.8, 0, 0)),
                  (bild, "Becken", "pos", (0, senken, 0))]
    animation(armatur, "Zaubern", 30, beben)
    _zwerg_faehigkeiten(armatur)


def _zwerg_faehigkeiten(armatur):
    """Hammerschlag (drei Schläge), Wurf und Fangen, Beben – siehe game/src/faehigkeiten.rs."""
    # Kampfstand: breitbeinig, der Hammer hängt in der Rechten
    ruhe = {"Oberarm.R": (-6, 0, -6), "Unterarm.R": (-20, 0, 0), "Oberarm.L": (0, 0, 8), "Unterarm.L": (-14, 0, 0),
            "Oberschenkel.L": (0, 0, 6), "Oberschenkel.R": (0, 0, -6), "Brust": (-3, 0, 0)}

    # Schlag 1: weit nach rechts ausholen, waagerecht von rechts nach links durchziehen (Bild 7)
    aus1 = {"Oberarm.R": (-92, 0, -10), "Unterarm.R": (-35, 0, 0), "Oberarm.L": (-78, 0, -20), "Unterarm.L": (-50, 0, 0),
            "Brust": (-4, 62, 0), "Bauch": (-2, 28, 0), "Becken": (0, 14, 0), "Kopf": (0, -40, 0),
            "Oberschenkel.L": (-10, 0, 8), "Unterschenkel.L": (14, 0, 0), "Oberschenkel.R": (4, 0, -8), "Unterschenkel.R": (16, 0, 0),
            "Becken.pos": (0, 0, -0.03)}
    treffer1 = {"Oberarm.R": (-90, 0, -4), "Unterarm.R": (-6, 0, 0), "Oberarm.L": (-85, 0, -18), "Unterarm.L": (-18, 0, 0),
                "Brust": (8, -12, 0), "Bauch": (4, -6, 0), "Becken": (0, -6, 0), "Kopf": (-4, 8, 0),
                "Oberschenkel.L": (-22, 0, 8), "Unterschenkel.L": (26, 0, 0), "Oberschenkel.R": (12, 0, -8), "Unterschenkel.R": (16, 0, 0),
                "Becken.pos": (0, 0, -0.05)}
    nach1 = _mit(treffer1, Oberarm_R=(-80, 0, -4), Unterarm_R=(-12, 0, 0), Oberarm_L=(-78, 0, -18), Unterarm_L=(-22, 0, 0),
                 Brust=(10, -50, 0), Bauch=(5, -22, 0), Becken=(0, -12, 0), Kopf=(-4, 30, 0))
    _clip(armatur, "Schlag1", 20, [(0, ruhe), (4, aus1), (7, treffer1), (10, nach1), (20, ruhe)])

    # Schlag 2: Rückhand – nach links ausholen, von links nach rechts zurückschlagen (Bild 7)
    aus2 = {"Oberarm.R": (-88, 0, -8), "Unterarm.R": (-45, 0, 0), "Oberarm.L": (-72, 0, -22), "Unterarm.L": (-55, 0, 0),
            "Brust": (-2, -58, 0), "Bauch": (-1, -26, 0), "Becken": (0, -12, 0), "Kopf": (0, 36, 0),
            "Oberschenkel.L": (4, 0, 8), "Unterschenkel.L": (16, 0, 0), "Oberschenkel.R": (-12, 0, -8), "Unterschenkel.R": (16, 0, 0),
            "Becken.pos": (0, 0, -0.03)}
    treffer2 = {"Oberarm.R": (-92, 0, -6), "Unterarm.R": (-6, 0, 0), "Oberarm.L": (-84, 0, -20), "Unterarm.L": (-18, 0, 0),
                "Brust": (8, 14, 0), "Bauch": (4, 6, 0), "Becken": (0, 6, 0), "Kopf": (-4, -10, 0),
                "Oberschenkel.L": (8, 0, 8), "Unterschenkel.L": (16, 0, 0), "Oberschenkel.R": (-22, 0, -8), "Unterschenkel.R": (26, 0, 0),
                "Becken.pos": (0, 0, -0.05)}
    nach2 = _mit(treffer2, Oberarm_R=(-82, 0, -6), Unterarm_R=(-14, 0, 0), Oberarm_L=(-76, 0, -20), Unterarm_L=(-24, 0, 0),
                 Brust=(10, 48, 0), Bauch=(5, 22, 0), Becken=(0, 12, 0), Kopf=(-4, -30, 0))
    _clip(armatur, "Schlag2", 20, [(0, ruhe), (4, aus2), (7, treffer2), (10, nach2), (20, ruhe)])

    # Schlag 3 (Schmetterschlag): Hammer mit beiden Händen hoch über den Kopf, zurücklehnen,
    # dann mit Ausfallschritt vor sich in den Boden schmettern (Bild 13), nachfedern
    hoch = {"Oberarm.R": (-176, 0, -6), "Unterarm.R": (-28, 0, 0), "Oberarm.L": (-165, 0, -12), "Unterarm.L": (-45, 0, 0),
            "Brust": (-16, 0, 0), "Bauch": (-6, 0, 0), "Kopf": (-10, 0, 0),
            "Oberschenkel.L": (-8, 0, 6), "Unterschenkel.L": (10, 0, 0), "Oberschenkel.R": (4, 0, -6), "Unterschenkel.R": (8, 0, 0),
            "Becken.pos": (0, 0, 0.03)}
    fallen = _mit(hoch, Oberarm_R=(-150, 0, -6), Unterarm_R=(-15, 0, 0), Oberarm_L=(-140, 0, -12), Unterarm_L=(-30, 0, 0),
                  Brust=(0, 0, 0), Bauch=(0, 0, 0), Kopf=(-4, 0, 0))
    fallen["Becken.pos"] = (0, 0, 0.0)
    # Der Rumpf ist weit vorgebeugt: die Arme müssen das ausgleichen, damit der Hammer vor dem Zwerg einschlägt
    schmettern = {"Oberarm.R": (-96, 0, -4), "Unterarm.R": (-4, 0, 0), "Oberarm.L": (-102, 0, -12), "Unterarm.L": (-10, 0, 0),
                  "Brust": (36, 0, 0), "Bauch": (16, 0, 0), "Kopf": (-24, 0, 0),
                  "Oberschenkel.L": (-40, 0, 8), "Unterschenkel.L": (48, 0, 0), "Oberschenkel.R": (18, 0, -8), "Unterschenkel.R": (30, 0, 0),
                  "Fuss.L": (-8, 0, 0), "Becken.pos": (0, 0, -0.11)}
    _clip(armatur, "Schlag3", 30, [(0, ruhe), (7, hoch), (10, fallen), (13, schmettern),
                                   (18, _mit(schmettern, Brust=(33, 0, 0), Oberarm_R=(-93, 0, -4))), (30, ruhe)])

    # Wurf: den Hammer hinter den Kopf, die Linke zeigt aufs Ziel, mit Drehung und
    # Ausfallschritt schleudern (Bild 9), der Arm schwingt quer vor dem Körper aus
    zielen = {"Oberarm.R": (-150, 0, -20), "Unterarm.R": (-105, 0, 0), "Oberarm.L": (-85, 0, 10), "Unterarm.L": (-8, 0, 0),
              "Brust": (-10, 40, 0), "Bauch": (-4, 18, 0), "Becken": (0, 10, 0), "Kopf": (4, -32, 0),
              "Oberschenkel.L": (-18, 0, 8), "Unterschenkel.L": (8, 0, 0), "Oberschenkel.R": (10, 0, -8), "Unterschenkel.R": (22, 0, 0),
              "Becken.pos": (0, 0, -0.02)}
    los = {"Oberarm.R": (-105, 0, -8), "Unterarm.R": (-8, 0, 0), "Oberarm.L": (-25, 0, 14), "Unterarm.L": (-50, 0, 0),
           "Brust": (18, -30, 0), "Bauch": (8, -14, 0), "Becken": (0, -8, 0), "Kopf": (-12, 22, 0),
           "Oberschenkel.L": (-32, 0, 8), "Unterschenkel.L": (36, 0, 0), "Oberschenkel.R": (18, 0, -8), "Unterschenkel.R": (12, 0, 0),
           "Becken.pos": (0, 0, -0.06)}
    aus = _mit(los, Oberarm_R=(-55, 0, -4), Unterarm_R=(-12, 0, 0), Oberarm_L=(-15, 0, 14), Unterarm_L=(-40, 0, 0),
               Brust=(24, -34, 0), Bauch=(10, -16, 0), Becken=(0, -10, 0), Kopf=(-14, 24, 0))
    _clip(armatur, "Wurf", 24, [(0, ruhe), (5, zielen), (9, los), (13, aus), (24, ruhe)])

    # Fangen: die Rechte greift nach dem zurückkehrenden Hammer, federt die Wucht ab
    greifen = _mit(ruhe, Oberarm_R=(-115, 0, -10), Unterarm_R=(-45, 0, 0), Brust=(-4, -6, 0), Kopf=(-6, 0, 0))
    abfedern = _mit(ruhe, Oberarm_R=(-60, 0, -8), Unterarm_R=(-55, 0, 0), Brust=(8, 4, 0),
                    Oberschenkel_L=(-8, 0, 6), Unterschenkel_L=(14, 0, 0), Oberschenkel_R=(-8, 0, -6), Unterschenkel_R=(14, 0, 0))
    abfedern["Becken.pos"] = (0, 0, -0.04)
    _clip(armatur, "Fangen", 14, [(0, ruhe), (3, greifen), (6, abfedern), (14, ruhe)])

    # Beben: tief in die Knie, abspringen mit dem Hammer über dem Kopf, im Fallen ausholen und
    # mit beiden Händen in den Boden schmettern (Bild 17), in der Hocke nachfedern, aufrichten
    hocke = {"Oberarm.R": (-55, 0, -8), "Unterarm.R": (-50, 0, 0), "Oberarm.L": (-45, 0, -10), "Unterarm.L": (-60, 0, 0),
             "Brust": (24, 0, 0), "Bauch": (10, 0, 0), "Kopf": (-16, 0, 0),
             "Oberschenkel.L": (-50, 0, 8), "Unterschenkel.L": (70, 0, 0), "Oberschenkel.R": (-50, 0, -8), "Unterschenkel.R": (70, 0, 0),
             "Fuss.L": (-20, 0, 0), "Fuss.R": (-20, 0, 0), "Becken.pos": (0, 0, -0.16)}
    sprung = {"Oberarm.R": (-178, 0, -6), "Unterarm.R": (-25, 0, 0), "Oberarm.L": (-168, 0, -12), "Unterarm.L": (-45, 0, 0),
              "Brust": (-16, 0, 0), "Bauch": (-6, 0, 0), "Kopf": (-8, 0, 0),
              "Oberschenkel.L": (-45, 0, 8), "Unterschenkel.L": (65, 0, 0), "Oberschenkel.R": (-30, 0, -8), "Unterschenkel.R": (55, 0, 0),
              "Fuss.L": (10, 0, 0), "Fuss.R": (10, 0, 0), "Becken.pos": (0, 0, 0.3)}
    fall = {"Oberarm.R": (-140, 0, -6), "Unterarm.R": (-12, 0, 0), "Oberarm.L": (-132, 0, -12), "Unterarm.L": (-25, 0, 0),
            "Brust": (4, 0, 0), "Bauch": (2, 0, 0), "Kopf": (-6, 0, 0),
            "Oberschenkel.L": (-30, 0, 8), "Unterschenkel.L": (35, 0, 0), "Oberschenkel.R": (-20, 0, -8), "Unterschenkel.R": (30, 0, 0),
            "Becken.pos": (0, 0, 0.16)}
    aufschlag = {"Oberarm.R": (-100, 0, -4), "Unterarm.R": (-4, 0, 0), "Oberarm.L": (-106, 0, -12), "Unterarm.L": (-8, 0, 0),
                 "Brust": (40, 0, 0), "Bauch": (18, 0, 0), "Kopf": (-26, 0, 0),
                 "Oberschenkel.L": (-58, 0, 10), "Unterschenkel.L": (72, 0, 0), "Oberschenkel.R": (-42, 0, -10), "Unterschenkel.R": (68, 0, 0),
                 "Fuss.L": (-14, 0, 0), "Fuss.R": (-26, 0, 0), "Becken.pos": (0, 0, -0.18)}
    halten = _mit(aufschlag, Brust=(36, 0, 0))
    halten["Becken.pos"] = (0, 0, -0.17)
    _clip(armatur, "Beben", 36, [(0, ruhe), (5, hocke), (10, sprung), (14, fall), (17, aufschlag), (23, halten), (36, ruhe)])

    # Ahnenruf (ultimativ): den Hammer zum Himmel recken, die Linke zur Faust geballt, die Ahnen
    # rufen – dann mit aller Kraft vor sich in den Boden schmettern (Bild 22), der Geisterhammer
    # fällt im selben Augenblick
    ruf = {"Oberarm.R": (-178, 0, -4), "Unterarm.R": (-5, 0, 0), "Oberarm.L": (-150, 0, 20), "Unterarm.L": (-60, 0, 0),
           "Brust": (-18, 0, 0), "Bauch": (-6, 0, 0), "Kopf": (-25, 0, 0),
           "Oberschenkel.L": (-6, 0, 8), "Unterschenkel.L": (8, 0, 0), "Oberschenkel.R": (4, 0, -8), "Unterschenkel.R": (8, 0, 0),
           "Becken.pos": (0, 0, 0.02)}
    ausholen = _mit(ruf, Oberarm_R=(-150, 0, -4), Oberarm_L=(-140, 0, -8), Unterarm_L=(-35, 0, 0), Brust=(-6, 0, 0), Kopf=(-10, 0, 0))
    _clip(armatur, "Ahnenruf", 36, [(0, ruhe), (7, ruf), (15, _mit(ruf, Oberarm_R=(-175, 0, -4), Brust=(-20, 5, 0))), (19, ausholen),
                                    (22, schmettern), (28, _mit(schmettern, Brust=(33, 0, 0))), (36, ruhe)])
