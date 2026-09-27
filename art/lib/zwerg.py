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

from figuren import (HAENGT, HOCH, Figur, _axt, _clip, _gehen, _mischen, _mit, _schleife, _spitzhacke, farbe, weich)
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
    kette, kette_dunkel = farbe("#8C939C"), farbe("#5E656E")
    rot, rot_dunkel = farbe("#A3282B"), farbe("#6E1A1C")
    gold, gold_dunkel = farbe("#D8AE4A"), farbe("#A9812E")
    bronze = farbe("#B07A3A")
    leder, leder_dunkel = farbe("#6A4526"), farbe("#43291A")
    hose, stiefel = farbe("#4E5A3A"), farbe("#3E2A1C")
    stahl, stahl_hell, stahl_dunkel = farbe("#8E97A2"), farbe("#C9D1DA"), farbe("#4E555E")
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

    # ================= Kettenhemd mit Waffenrock =================
    hemd_ringe = [
        (Vector((0, 0.012, 1.14)), X, Y, 0.1, 0.085),
        (Vector((0, 0.0, 1.09)), X, Y, 0.21, 0.16),
        (Vector((0, 0.0, 1.0)), X, Y, 0.275, 0.2),
        (Vector((0, -0.012, 0.9)), X, Y, 0.3, 0.245),
        (Vector((0, -0.022, 0.8)), X, Y, 0.31, 0.265),
        (Vector((0, -0.012, 0.7)), X, Y, 0.295, 0.245),
        (Vector((0, 0.0, 0.62)), X, Y, 0.275, 0.215),
        (Vector((0, 0.005, 0.52)), X, Y, 0.295, 0.225, faltig(0.03)),
        (Vector((0, 0.01, 0.43)), X, Y, 0.315, 0.245, faltig(0.05)),
        (Vector((0, 0.01, 0.405)), X, Y, 0.31, 0.24, faltig(0.05)),
    ]
    SEG = 64

    def hemd_farbe(i, k, poly):
        vorne = vorne_winkel(k + 0.5, SEG)
        z = poly.center.z
        if i >= 8.3:
            return bronze * (0.9 if k % 2 else 1.0)                    # Saum aus Bronzeringen
        if 0.44 < z < 1.02 and vorne < 0.36:
            if vorne > 0.3 or z < 0.47:
                return gold                                             # Goldborte am Waffenrock
            return rot * (0.92 + 0.08 * math.sin(z * 30)) if not (0.86 < z < 0.95 and vorne < 0.12) else gold_dunkel
        if 0.44 < z < 1.02 and math.pi - vorne < 0.3:
            return rot_dunkel                                           # Rückenteil
        # Kettenglieder: versetzte helle und dunkle Reihen
        reihe = int(z * 55)
        glied = 0.9 if reihe % 2 else 1.0
        glied *= 0.95 if (k + reihe) % 3 == 0 else 1.0
        return kette * glied * (0.88 + 0.12 * weich(0.4, 1.1, z))

    f.loft("Kettenhemd", hemd_ringe, SEG, hemd_farbe, _rock, teilung=3)
    f.loft("Futter", [(Vector((0, 0.01, 0.41)), X, Y, 0.3, 0.23), (Vector((0, 0.0, 0.55)), X, Y, 0.16, 0.12)], 24,
           lambda i, k, p: leder_dunkel, _rock, oben_zu=True)
    # Wappen auf der Brust: goldener Hammer
    f.kiste("Wappenstiel", (0, -0.268, 0.9), (0.014, 0.006, 0.08), gold, _rumpf)
    f.kiste("Wappenkopf", (0, -0.27, 0.935), (0.05, 0.007, 0.022), gold, _rumpf)

    # ================= Gürtel mit Runenschnalle und Taschen =================
    f.loft("Guertel", [(Vector((0, -0.005, 0.595)), X, Y, 0.29, 0.23), (Vector((0, -0.005, 0.68)), X, Y, 0.298, 0.24)], 56,
           lambda i, k, p: leder * (0.78 if k % 7 == 0 else 1.0), _rumpf, teilung=2)
    f.kiste("Schnalle", (0, -0.24, 0.638), (0.085, 0.016, 0.07), gold, _rumpf)
    f.kiste("Schnallenmitte", (0, -0.25, 0.638), (0.05, 0.012, 0.04), gold_dunkel, _rumpf)
    f.stern("Rune", Vector((0, -0.257, 0.638)), -Y, 0.018, farbe("#7FD0FF"), _rumpf, zacken=4)
    for s in (1, -1):
        f.kiste("Tasche", (0.24 * s, -0.12, 0.56), (0.07, 0.05, 0.08), leder * 1.1, _rumpf)
        f.kiste("Taschenklappe", (0.24 * s, -0.145, 0.595), (0.075, 0.012, 0.045), leder_dunkel, _rumpf)
    f.kiste("Horn", (-0.2, 0.16, 0.6), (0.03, 0.03, 0.09), horn, _rumpf, Quaternion(X, math.radians(30)))

    # ================= Schulterpanzer =================
    for seite in (1, -1):
        s = "L" if seite > 0 else "R"
        schulter = _spiegel(SCHULTER, seite)
        gewicht = lambda co, s=s: _mischen((f"Oberarm.{s}", 0.7), ("Brust", 0.3))
        for lage, (dz, grosse) in enumerate(((0.035, (0.14, 0.15, 0.08)), (-0.03, (0.13, 0.14, 0.06)))):
            mitte = schulter + Vector((0.035 * seite, 0, dz))
            f.kugel("Schulterpanzer", mitte, grosse, lambda poly, lage=lage: (stahl_hell if poly.normal.z > 0.75 else stahl) * (0.95 - 0.08 * lage),
                    gewicht, 16, 8, glatt=False)
            f.loft("Panzerrand", [(mitte + Vector((0, 0, -grosse[2] * 0.05)), X, Y, grosse[0] * 1.01, grosse[1] * 1.01),
                                  (mitte + Vector((0, 0, grosse[2] * 0.2)), X, Y, grosse[0] * 0.99, grosse[1] * 0.99)],
                   20, lambda i, k, p: gold, gewicht, teilung=1)
        f.kugel("Niete", schulter + Vector((0.07 * seite, -0.08, 0.1)), (0.014, 0.014, 0.014), gold, gewicht, 8, 4)

    # ================= Arme: Kettenärmel, Armschienen, Handschuhe =================
    for seite in (1, -1):
        s_, e, h = _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite)
        ringe = []
        for t, (a, b), radius in ((0.1, (s_, e), 0.085), (0.6, (s_, e), 0.08), (1.05, (s_, e), 0.078)):
            achse = (b - a).normalized()
            q = achse.cross(Y).normalized()
            ringe.append((a.lerp(b, t), q, achse.cross(q).normalized(), radius, radius * 0.95))
        f.loft("Aermel", ringe, 20, lambda i, k, p: kette * (0.9 if int(p.center.z * 55) % 2 else 1.0), _arm(seite), teilung=2)
        ringe = []
        for t, radius in ((0.0, 0.074), (0.35, 0.082), (0.8, 0.078), (1.02, 0.07)):
            achse = (h - e).normalized()
            q = achse.cross(Y).normalized()
            ringe.append((e.lerp(h, t), q, achse.cross(q).normalized(), radius, radius * 0.92))

        def schiene(i, k, p):
            if i < 0.25 or i > 2.7:
                return stahl                                               # Stahlbänder an den Enden
            return leder * (0.85 if k % 5 == 0 else 1.0)

        f.loft("Armschiene", ringe, 18, schiene, _arm(seite), unten_zu=True, teilung=3)

    def faust(griff, seite, dicke):
        """Handschuh als Faust: Handteller, vier gekrümmte Finger, Daumen darüber."""
        formen = []
        auf = Vector((0, 0, 1))
        formen.append((griff + Vector((0.028 * seite, 0.01, 0.03)), (0.034, 0.042, 0.046)))
        for i in range(4):
            z = griff.z + 0.02 - 0.022 * i
            for j, w in enumerate((0.4, -0.5, -1.5, -2.5)):
                rad = 0.034 - 0.002 * j
                p = Vector((griff.x + math.cos(w) * rad * seite, griff.y + math.sin(w) * rad, z))
                formen.append((p, (dicke,) * 3))
        daumen = [griff + Vector((0.03 * seite, -0.03, 0.035)), griff + Vector((0.01 * seite, -0.045, 0.04)), griff + Vector((-0.012 * seite, -0.04, 0.042))]
        for p in daumen:
            formen.append((p, (dicke * 1.1,) * 3))
        formen.append((griff + Vector((0.025 * seite, 0.012, 0.075)), (0.04, 0.042, 0.03)))    # Stulpe
        del auf
        return formen

    for seite in (1, -1):
        s = "L" if seite > 0 else "R"
        if seite > 0:
            griff = Vector((0.39, -0.03, 0.57))
        else:
            griff = Vector((GRIFF_X, GRIFF_Y, GRIFF_Z))
        f.metaball(f"Handschuh{s}", faust(griff, -seite, 0.0105), 0.004, 2200,
                   lambda poly: leder_dunkel * (1.15 if poly.normal.z > 0.5 else 1.0), lambda co, s=s: {f"Hand.{s}": 1.0}, glatt=True)

    # ================= Beine: Hose, Stiefel mit Stulpe und Stahlkappe =================
    for seite in (1, -1):
        sn = "L" if seite > 0 else "R"
        k_, h_ = _spiegel(KNIE, seite), _spiegel(HUEFTE, seite)
        ringe = [(Vector((0.13 * seite, 0.01, 0.26)), X, Y, 0.075, 0.075), (k_, X, Y, 0.085, 0.085), (h_ + Vector((0, 0, -0.03)), X, Y, 0.11, 0.105)]
        f.loft("Hose", ringe, 14, lambda i, k, p: hose * (0.9 if k % 4 == 0 else 1.0),
               lambda co, sn=sn: _mischen((f"Unterschenkel.{sn}", 1 - weich(0.3, 0.38, co.z)), (f"Oberschenkel.{sn}", weich(0.3, 0.38, co.z))))
        schaft = [(Vector((0.13 * seite, 0.03, 0.06)), X, Y, 0.075, 0.08), (Vector((0.13 * seite, 0.02, 0.17)), X, Y, 0.08, 0.08),
                  (Vector((0.13 * seite, 0.01, 0.28)), X, Y, 0.098, 0.098), (Vector((0.13 * seite, 0.01, 0.31)), X, Y, 0.104, 0.104),
                  (Vector((0.13 * seite, 0.01, 0.305)), X, Y, 0.08, 0.08)]
        f.loft("Stiefelschaft", schaft, 16, lambda i, k, p: stiefel * (1.25 if i >= 2.5 else 1.0), lambda co, sn=sn: {f"Unterschenkel.{sn}": 1.0},
               unten_zu=True, teilung=2)
        fuss = [(Vector((0.13 * seite, 0.075, 0.05)), X, Z, 0.07, 0.05), (Vector((0.13 * seite, 0.07, 0.022)), X, Z, 0.078, 0.03),
                (Vector((0.13 * seite, -0.02, 0.055)), X, Z, 0.085, 0.06), (Vector((0.13 * seite, -0.11, 0.05)), X, Z, 0.078, 0.048),
                (Vector((0.13 * seite, -0.16, 0.05)), X, Z, 0.058, 0.04), (Vector((0.13 * seite, -0.19, 0.055)), X, Z, 0.03, 0.028),
                (Vector((0.13 * seite, -0.2, 0.06)), X, Z, 0.004, 0.004)]

        def stiefel_farbe(i, k, p):
            if p.center.z < 0.012:
                return stiefel * 0.5                                    # Sohle
            if i >= 3.6:
                return stahl                                            # Stahlkappe
            return stiefel

        f.loft("Stiefel", fuss, 16, stiefel_farbe, lambda co, sn=sn: {f"Fuss.{sn}": 1.0}, oben_zu=True, unten_zu=True, teilung=3)

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

    # ================= Bart: Grundmasse, Zöpfe mit Goldringen, Schnurrbart =================
    bart_gewicht = _kopf_und_brust(0.95, 1.14)
    ringe = []
    for t in (0.0, 0.12, 0.3, 0.5, 0.7, 0.88):
        z = KOPF.z - 0.06 - 0.36 * t
        breite = 0.1 * math.sin(math.pi * min(1.0, 0.4 + t * 0.8)) ** 0.5 + 0.02
        tiefe = 0.05 * (1 - t) ** 0.5 + 0.02
        ringe.append((Vector((0, -0.085 - 0.13 * t ** 0.8, z)), X, Y, breite, tiefe, lambda w: 1.0 + 0.12 * math.sin(w * 7) + 0.05 * math.sin(w * 13)))
    ringe.append((Vector((0, -0.21, KOPF.z - 0.46)), X, Y, 0.012, 0.01))
    f.loft("Bart", ringe, 32, lambda i, k, p: bart * (0.86 + 0.14 * ((k * 7 + int(i * 3)) % 5) / 4), bart_gewicht, oben_zu=True, teilung=3)
    for n in range(18):
        x0 = r.uniform(-0.09, 0.09)
        z0 = KOPF.z - 0.06 - abs(x0) * 0.3 - r.uniform(0, 0.03)
        laenge = r.uniform(0.2, 0.36) * (1 - abs(x0) * 3)
        start = Vector((x0, -0.1 - 0.02 * (1 - abs(x0) / 0.09), z0))
        punkte = []
        for j in range(5):
            t = j / 4
            welle = math.sin(t * math.pi * 2 + n) * 0.01
            punkte.append(start + Vector((-x0 * 0.5 * t + welle, -0.1 * t - 0.02 * math.sin(math.pi * t), -laenge * t)))
        f.straehne("Bartlocke", punkte, r.uniform(0.016, 0.024), 0.002, bart * r.uniform(0.82, 1.02), bart_gewicht, 7, 0.5, 0.75)
    # Zwei Zöpfe unten am Bart, je mit zwei Goldringen
    for s in (1, -1):
        oben = Vector((0.035 * s, -0.2, KOPF.z - 0.4))
        punkte = [oben + Vector((0.004 * s * j, -0.008 * j, -0.045 * j)) for j in range(6)]
        f.straehne("Zopf", punkte, 0.022, 0.008, lambda i, k, p: bart_dunkel if (int(i * 4) + k) % 3 == 0 else bart, bart_gewicht, 8, 1.2, 0.9)
        for j in (1.6, 3.4):
            mitte = punkte[int(j)] .lerp(punkte[int(j) + 1], j % 1)
            f.loft("Bartring", [(mitte + Vector((0, 0, -0.012)), X, Y, 0.022, 0.022), (mitte + Vector((0, 0, 0.012)), X, Y, 0.022, 0.022)], 12,
                   lambda i, k, p: gold, bart_gewicht, oben_zu=True, unten_zu=True)
    for s in (1, -1):
        for j in range(7):
            start = Vector((s * (0.008 + 0.006 * j), -0.14 + 0.003 * j, KOPF.z - 0.05 - 0.002 * j))
            ende = Vector((s * (0.1 + 0.008 * j), -0.1 + 0.005 * j, KOPF.z - 0.12 - 0.01 * j))
            punkte = [start, start + Vector((s * 0.025, 0.003, -0.004)), start.lerp(ende, 0.6) + Vector((s * 0.012, -0.01, 0.006)), ende]
            f.straehne("Schnurrbart", punkte, 0.011, 0.0015, bart * r.uniform(0.9, 1.05), kopf_gewicht, 6, 0.4, 0.8)
    # Zöpfe am Hinterkopf unter dem Helm
    for s in (-1, 0, 1):
        start = Vector((0.06 * s, 0.07, KOPF.z + 0.02))
        punkte = [start + Vector((0.01 * s * t, 0.03 * t + 0.02 * t * t, -0.3 * t)) for t in (0.0, 0.25, 0.5, 0.75, 1.0)]
        f.straehne("Haarzopf", punkte, 0.022, 0.01, lambda i, k, p: bart_dunkel if (int(i * 4) + k) % 3 == 0 else bart, _kopf_und_brust(1.0, 1.2), 8, 1.2, 0.85)

    # ================= Hörnerhelm =================
    helm = []
    for i, (dz, rx, ry) in enumerate(((0.0, 0.118, 0.125), (0.012, 0.12, 0.127), (0.05, 0.116, 0.122), (0.09, 0.1, 0.106), (0.125, 0.075, 0.08),
                                      (0.15, 0.045, 0.048), (0.162, 0.012, 0.012))):
        helm.append((KOPF + Vector((0, 0.005, 0.03 + dz)), X, Y, rx, ry))

    def helm_farbe(i, k, p):
        if i < 1.0:
            return gold if i < 0.6 else gold_dunkel                     # Goldband am Rand
        if k % 16 in (4, 12) or (abs(p.center.x) < 0.012 and p.center.z > KOPF.z + 0.06):
            return gold_dunkel                                            # Kamm und Nieten-Linien
        return stahl_hell if p.normal.z > 0.6 else stahl

    f.loft("Helm", helm, 32, helm_farbe, kopf_gewicht, oben_zu=True, teilung=2)
    f.kiste("Nasenschutz", KOPF + Vector((0, -0.128, 0.0)), (0.012, 0.012, 0.075), stahl_hell, kopf_gewicht)
    for s in (1, -1):
        f.kugel("Helmniete", KOPF + Vector((0.085 * s, -0.085, 0.05)), (0.009, 0.009, 0.009), gold, kopf_gewicht, 8, 4)
        wurzel = KOPF + Vector((0.105 * s, 0.0, 0.1))
        punkte = [wurzel, wurzel + Vector((0.07 * s, -0.005, 0.02)), wurzel + Vector((0.13 * s, -0.02, 0.07)), wurzel + Vector((0.155 * s, -0.04, 0.14)),
                  wurzel + Vector((0.15 * s, -0.055, 0.2))]
        f.straehne("Horn", punkte, 0.034, 0.004, lambda i, k, p: horn_dunkel if i > 3.2 or (int(i * 3) % 2 and k % 2) else horn, kopf_gewicht, 12, 0.0, 1.0,
                   teilung=3)
        f.loft("Hornring", [(wurzel + Vector((0.012 * s, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1)), 0.038, 0.038),
                            (wurzel + Vector((0.03 * s, 0, 0.004)), Vector((0, 1, 0)), Vector((0, 0, 1)), 0.036, 0.036)], 12,
               lambda i, k, p: gold, kopf_gewicht, oben_zu=True, unten_zu=True)

    # ================= Rundschild auf dem Rücken =================
    schild_mitte = Vector((0.0, 0.29, 0.88))
    neig = Quaternion(Z, math.radians(8)) @ Quaternion(X, math.radians(-12))
    achse = neig @ Y
    a1, a2 = neig @ X, neig @ Z
    ringe = [(schild_mitte - achse * 0.022, a1, a2, 0.28, 0.28), (schild_mitte - achse * 0.02, a1, a2, 0.29, 0.29),
             (schild_mitte + achse * 0.02, a1, a2, 0.29, 0.29), (schild_mitte + achse * 0.022, a1, a2, 0.28, 0.28)]

    def schild_farbe(i, k, p):
        d = (p.center - schild_mitte)
        radial = (d - achse * d.dot(achse)).length
        if radial > 0.265:
            return stahl_dunkel                                            # Eisenrand
        planke = int((d.dot(a1) + 0.3) / 0.075)
        return (holz if planke % 2 else holz_dunkel) * (1.0 if radial < 0.2 or (planke + int(radial * 30)) % 3 else 0.9)

    f.loft("Schild", ringe, 36, schild_farbe, _rumpf, oben_zu=True, unten_zu=True, teilung=1)
    buckel = [(schild_mitte + achse * 0.02, a1, a2, 0.075, 0.075), (schild_mitte + achse * 0.05, a1, a2, 0.06, 0.06),
              (schild_mitte + achse * 0.075, a1, a2, 0.03, 0.03)]
    f.loft("Schildbuckel", buckel, 16, lambda i, k, p: stahl_hell if i > 1 else stahl, _rumpf, oben_zu=True)
    for k in range(8):
        w = math.tau * k / 8
        f.kugel("Schildniete", schild_mitte + achse * 0.024 + (a1 * math.cos(w) + a2 * math.sin(w)) * 0.245, (0.012, 0.012, 0.012), gold, _rumpf, 8, 4)
    # Gurt über der Brust
    gurt = [Vector((0.22, -0.2, 1.02)), Vector((0.08, -0.29, 0.86)), Vector((-0.1, -0.3, 0.72)), Vector((-0.24, -0.22, 0.64))]
    f.loft("Gurt", [(p, X, Y, 0.028, 0.006) for p in gurt], 8, lambda i, k, p: leder_dunkel, _rumpf, oben_zu=True, unten_zu=True, teilung=3)

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
