"""Bogenschütze – dritte Spielfigur (Baukasten: figuren.py).

Ein junger Waldläufer, gleiche Maße und gleiches Skelett wie der Magier: grünes Wams mit kurzem,
vorne geschlitztem Rock, Lederweste mit Schnürung, breiter Gürtel mit Tasche und Messer, grüner
Kapuzenumhang (Kapuze auf, das Gesicht frei), lederne Armschienen und Handschuhe, Hose und hohe
Stulpenstiefel. Auf dem Rücken ein Köcher voller Pfeile mit Gurt über der Brust. Braunes Haar,
kurzer Bart, grüne Augen.
In der linken Faust der Bogen (Anbauteil „Bogen“ bzw. die erbeutbaren Bögen aus waffen.py), in
der rechten Spitzhacke oder Axt.

Koordinaten wie in der Werkstatt: Z oben, die Figur schaut nach -Y, Füße im Ursprung, links (L) +X.
"""

import math

import bmesh
from mathutils import Quaternion, Vector

from figuren import (AUGE, ELLBOGEN, FINGER, HAENGT, HANDGELENK, HOCH, HUEFTE, KNIE, KNOECHEL, SCHULTER, STAB_X, STAB_Y, ZEHEN, Figur,
                     _abstand_zur_strecke, _arm_gewichte, _axt, _clip, _gehen, _kopf_und_brust, _mischen, _mit, _robe_gewichte,
                     _rumpf_gewichte, _schleife, _spiegel, _spitzhacke, farbe, weich)
from werkstatt import animation

X, Y, Z = Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))

# Der Bogen liegt in der linken Faust: Griff quer (entlang Y) bei
BOGEN_GRIFF = Vector((0.36, -0.03, 0.905))


def _schale(f, name, mitte, rx, ry, rz, theta, phi, farbe_von, gewicht, innen=False, seg=(40, 16), form=None):
    """Stück einer Ellipsoid-Schale (Kapuze, Umhang): Winkel `theta` (um Z, 0 = vorne) von–bis,
    `phi` (Höhe, -90 unten … 90 oben) von–bis. `form(theta, phi)` → Faktor auf den Radius."""
    bm = bmesh.new()
    nt, np_ = seg
    reihen = []
    for j in range(np_ + 1):
        ph = math.radians(phi[0] + (phi[1] - phi[0]) * j / np_)
        reihe = []
        for i in range(nt + 1):
            th = math.radians(theta[0] + (theta[1] - theta[0]) * i / nt)
            fak = form(th, ph) if form else 1.0
            p = mitte + Vector((math.sin(th) * rx * math.cos(ph), -math.cos(th) * ry * math.cos(ph), rz * math.sin(ph))) * fak
            reihe.append(bm.verts.new(p))
        reihen.append(reihe)
    for j in range(np_):
        for i in range(nt):
            a, b, c, d = reihen[j][i], reihen[j][i + 1], reihen[j + 1][i + 1], reihen[j + 1][i]
            bm.faces.new((a, b, c, d) if not innen else (d, c, b, a))
    obj = f._objekt(bm, name, farbe_von, gewicht)
    # Nach außen (bzw. innen) zeigen lassen: recalc kennt bei offenen Flächen die Seite nicht
    for poly in obj.data.polygons:
        nach_aussen = (poly.center - mitte).dot(poly.normal) > 0
        if nach_aussen == innen:
            poly.flip()
    return obj


def _faust(griff, achse, seite, dicke, haut):
    """Faust aus Metaball-Formen, die sich um einen Stab entlang `achse` schließt."""
    achse = achse.normalized()
    q1 = achse.cross(Z if abs(achse.z) < 0.9 else X).normalized()
    q2 = achse.cross(q1).normalized()
    formen = [(griff + q1 * 0.028 * seite + q2 * 0.01, (0.03, 0.03, 0.03))]
    for i in range(4):
        versatz = achse * (0.022 - 0.0145 * i)
        punkte = []
        for j, w in enumerate((0.3, -0.6, -1.5, -2.4, -3.1)):
            rad = 0.03 - 0.0015 * j
            punkte.append(griff + versatz + (q1 * math.cos(w) * seite + q2 * math.sin(w)) * rad)
        for a, b in zip(punkte, punkte[1:]):
            for t in (0.0, 0.33, 0.66):
                formen.append((a.lerp(b, t), (dicke,) * 3))
        dicke *= 0.97
    for t in (0.0, 0.5, 1.0):
        formen.append((griff + achse * 0.035 + q1 * (0.02 - 0.035 * t) * seite - q2 * 0.025, (dicke * 1.1,) * 3))   # Daumen
    return formen


def bogenschuetze(seed=31, name="Bogenschuetze"):
    f = Figur(name, seed)
    r = f.rng
    gruen, gruen_dunkel, gruen_hell = farbe("#3F6B34"), farbe("#2A4A24"), farbe("#6E9A4A")
    umhang, umhang_dunkel, umhang_innen = farbe("#2F5A3A"), farbe("#1F3E28"), farbe("#1A2A1C")
    leder, leder_dunkel, leder_hell = farbe("#7A4E2C"), farbe("#4A2E1A"), farbe("#A06A3A")
    hose, stiefel = farbe("#5A4A3C"), farbe("#4A3020")
    gold, messing = farbe("#D8AE4A"), farbe("#B8904A")
    haut, wange, lippe, nasenloch = farbe("#EDBF98"), farbe("#E4A080"), farbe("#C4826E"), farbe("#5A3A34")
    haar, haar_dunkel = farbe("#6A4226"), farbe("#4A2C18")
    holz, stahl = farbe("#6E4B2E"), farbe("#A8B0BA")
    kopf_gewicht = lambda co: {"Kopf": 1.0}

    def vorne_winkel(k, seg):
        w = math.tau * k / seg
        return abs(math.atan2(math.sin(w + math.pi / 2), math.cos(w + math.pi / 2)))

    # ================= Wams mit kurzem Rock =================
    falten = [r.uniform(0.6, 1.4) for _ in range(8)]

    def faltig(staerke, phase=0.0):
        return lambda w: 1.0 + staerke * sum(math.sin(w * (3 + i) + phase + falten[i]) * falten[i] / (3 + i) for i in range(6))

    wams = [
        (Vector((0, 0.005, 1.58)), X, Y, 0.1, 0.085),
        (Vector((0, 0.0, 1.52)), X, Y, 0.19, 0.13),
        (Vector((0, 0.0, 1.42)), X, Y, 0.185, 0.132),
        (Vector((0, -0.005, 1.28)), X, Y, 0.17, 0.128),
        (Vector((0, 0.0, 1.12)), X, Y, 0.158, 0.12),
        (Vector((0, 0.0, 0.98)), X, Y, 0.182, 0.14, faltig(0.02)),
        (Vector((0, 0.005, 0.86)), X, Y, 0.21, 0.165, faltig(0.05)),
        (Vector((0, 0.01, 0.76)), X, Y, 0.232, 0.185, faltig(0.07)),
        (Vector((0, 0.01, 0.74)), X, Y, 0.228, 0.182, faltig(0.07)),
    ]
    SEG = 64

    def wams_farbe(i, k, p):
        vorne = vorne_winkel(k + 0.5, SEG)
        if i >= 6.6:
            return leder_hell                                                   # Saum
        if p.center.z < 0.97 and vorne < 0.1:
            return gruen_dunkel * 0.7                                            # vorne geschlitzt
        return gruen * (0.9 + 0.12 * weich(0.7, 1.6, p.center.z)) * (0.96 if k % 8 == 0 else 1.0)

    f.loft("Wams", wams, SEG, wams_farbe, _robe_gewichte, teilung=3)
    f.loft("Futter", [(Vector((0, 0.01, 0.745)), X, Y, 0.22, 0.175), (Vector((0, 0.0, 0.9)), X, Y, 0.14, 0.1)], 24,
           lambda i, k, p: gruen_dunkel * 0.6, _robe_gewichte, oben_zu=True)

    # ================= Lederweste mit Schnürung =================
    weste = [
        (Vector((0, 0.005, 1.54)), X, Y, 0.195, 0.138),
        (Vector((0, 0.0, 1.42)), X, Y, 0.192, 0.14),
        (Vector((0, -0.005, 1.28)), X, Y, 0.177, 0.136),
        (Vector((0, 0.0, 1.14)), X, Y, 0.166, 0.128),
        (Vector((0, 0.0, 1.07)), X, Y, 0.17, 0.132),
    ]

    def weste_farbe(i, k, p):
        vorne = vorne_winkel(k + 0.5, 48)
        if vorne < 0.14:
            # Schnürung: Kreuze aus hellem Band über der dunklen Öffnung
            z = p.center.z
            return leder_hell if abs(((z * 26) % 1.0) - 0.5) < 0.18 else gruen_dunkel * 0.8
        if i < 0.3 or i > 3.7:
            return leder_dunkel
        return leder * (0.9 + 0.1 * ((k + int(i * 4)) % 3) / 2)

    f.loft("Weste", weste, 48, weste_farbe, _rumpf_gewichte, teilung=3)
    for s in (1, -1):
        for z in (1.46, 1.34, 1.22, 1.12):
            f.kugel("Oese", (0.03 * s, -0.142 + 0.004 * (1.46 - z), z), (0.006, 0.004, 0.006), messing, _rumpf_gewichte, 6, 4)

    # ================= Gürtel, Tasche, Messer =================
    f.loft("Guertel", [(Vector((0, 0.0, 0.98)), X, Y, 0.188, 0.146), (Vector((0, 0.0, 1.05)), X, Y, 0.184, 0.143)], 56,
           lambda i, k, p: leder_dunkel * (0.8 if k % 7 == 0 else 1.0), _rumpf_gewichte, teilung=2)
    f.kiste("Schnalle", (0, -0.148, 1.015), (0.05, 0.012, 0.045), messing, _rumpf_gewichte)
    f.kiste("Schnallenmitte", (0, -0.154, 1.015), (0.028, 0.008, 0.024), leder_dunkel, _rumpf_gewichte)
    f.kiste("Tasche", (0.17, -0.06, 0.93), (0.07, 0.045, 0.085), leder * 1.1, _rumpf_gewichte)
    f.kiste("Taschenklappe", (0.17, -0.085, 0.965), (0.075, 0.01, 0.045), leder_dunkel, _rumpf_gewichte)
    f.kugel("Knopf", (0.17, -0.092, 0.95), (0.007, 0.004, 0.007), messing, _rumpf_gewichte, 8, 4)
    messer = Quaternion(Y, math.radians(20))
    f.kiste("Scheide", (-0.17, -0.07, 0.9), (0.03, 0.02, 0.15), leder_dunkel, _rumpf_gewichte, messer)
    f.kiste("Messergriff", (-0.185, -0.07, 1.0), (0.02, 0.018, 0.07), holz, _rumpf_gewichte, messer)
    f.kugel("Messerknauf", (-0.2, -0.07, 1.04), (0.015, 0.015, 0.015), messing, _rumpf_gewichte, 8, 4)

    # ================= Ärmel, Armschienen =================
    for seite in (1, -1):
        s, e, h = _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite)
        ringe = []
        for t, (a, b), radius in ((0.0, (s + Vector((-0.04 * seite, 0, 0.03)), e), 0.08), (0.5, (s, e), 0.078), (1.0, (s, e), 0.07),
                                  (0.35, (e, h), 0.064)):
            achse = (b - a).normalized()
            q = achse.cross(Y).normalized()
            ringe.append((a.lerp(b, t), q, achse.cross(q).normalized(), radius, radius * 0.92, faltig(0.03, seite)))
        f.loft("Aermel", ringe, 28, lambda i, k, p: gruen * (0.92 + 0.08 * math.sin(k / 28 * math.tau * 3)), _arm_gewichte(seite), teilung=3)
        ringe = []
        for t, radius in ((0.3, 0.06), (0.55, 0.066), (0.85, 0.058), (1.02, 0.052)):
            achse = (h - e).normalized()
            q = achse.cross(Y).normalized()
            ringe.append((e.lerp(h, t), q, achse.cross(q).normalized(), radius, radius * 0.9))

        def schiene(i, k, p):
            if (k % 6) == 0:
                return leder_hell                                                # Nähte
            return leder_dunkel * (1.1 if i < 0.4 or i > 2.6 else 1.0)

        f.loft("Armschiene", ringe, 18, schiene, _arm_gewichte(seite), unten_zu=True, teilung=2)
        # Schnürriemen über der Armschiene
        for t in (0.5, 0.72):
            f.loft("Riemen", [(e.lerp(h, t) - (h - e).normalized() * 0.006, q, achse.cross(q).normalized(), 0.069, 0.064),
                              (e.lerp(h, t) + (h - e).normalized() * 0.006, q, achse.cross(q).normalized(), 0.069, 0.064)], 16,
                   lambda i, k, p: leder_hell, _arm_gewichte(seite), oben_zu=True, unten_zu=True)

    # ================= Hände =================
    # Links: Faust um den Bogengriff (quer, entlang Y); rechts: Faust um einen senkrechten Stiel
    # (Spitzhacke, Axt) – so hält sie auch die Sehne
    handschuh = leder_dunkel * 1.2
    f.metaball("HandL", _faust(BOGEN_GRIFF, Y, 1, 0.0092, haut), 0.003, 2200,
               lambda poly: handschuh if poly.center.z > BOGEN_GRIFF.z + 0.02 else haut * (0.95 if poly.normal.x < 0 else 1.0), lambda co: {"Hand.L": 1.0}, glatt=True)
    griff_r = Vector((STAB_X, STAB_Y, 0.915))
    f.metaball("HandR", _faust(griff_r, Z, -1, 0.0092, haut), 0.003, 2200,
               lambda poly: handschuh if poly.center.z > griff_r.z + 0.03 else haut, lambda co: {"Hand.R": 1.0}, glatt=True)

    # ================= Beine: Hose und hohe Stulpenstiefel =================
    for seite in (1, -1):
        sn = "L" if seite > 0 else "R"
        k_, h_ = _spiegel(KNIE, seite), _spiegel(HUEFTE, seite)
        ringe = [(Vector((0.1 * seite, 0.015, 0.4)), X, Y, 0.058, 0.058), (k_, X, Y, 0.066, 0.066), (Vector((0.1 * seite, 0.0, 0.72)), X, Y, 0.078, 0.078),
                 (h_ + Vector((0, 0, -0.03)), X, Y, 0.092, 0.09)]
        f.loft("Hose", ringe, 16, lambda i, k, p: hose * (0.9 if k % 5 == 0 else 1.0),
               lambda co, sn=sn: _mischen((f"Unterschenkel.{sn}", 1 - weich(0.46, 0.58, co.z)), (f"Oberschenkel.{sn}", weich(0.46, 0.58, co.z))), teilung=2)
        schaft = [(Vector((0.1 * seite, 0.03, 0.06)), X, Y, 0.056, 0.06), (Vector((0.1 * seite, 0.02, 0.2)), X, Y, 0.058, 0.058),
                  (Vector((0.1 * seite, 0.015, 0.4)), X, Y, 0.066, 0.066), (Vector((0.1 * seite, 0.01, 0.47)), X, Y, 0.078, 0.08),
                  (Vector((0.1 * seite, 0.005, 0.5)), X, Y, 0.083, 0.085), (Vector((0.1 * seite, 0.005, 0.49)), X, Y, 0.066, 0.066)]
        f.loft("Stiefelschaft", schaft, 18, lambda i, k, p: stiefel * (1.3 if i >= 3.0 else 1.0) * (0.92 if k % 9 == 0 else 1.0),
               lambda co, sn=sn: _mischen((f"Unterschenkel.{sn}", 1 - weich(0.5, 0.56, co.z)), (f"Oberschenkel.{sn}", weich(0.5, 0.56, co.z))),
               unten_zu=True, teilung=2)
        f.kugel("Stiefelschnalle", (0.1 * seite + 0.05 * seite, -0.02, 0.3), (0.006, 0.02, 0.016), messing, lambda co, sn=sn: {f"Unterschenkel.{sn}": 1.0}, 8, 4)
        fuss = [(Vector((0.1 * seite, 0.06, 0.045)), X, Z, 0.052, 0.045), (Vector((0.1 * seite, 0.055, 0.02)), X, Z, 0.058, 0.03),
                (Vector((0.1 * seite, -0.02, 0.05)), X, Z, 0.06, 0.055), (Vector((0.1 * seite, -0.09, 0.045)), X, Z, 0.052, 0.04),
                (Vector((0.1 * seite, -0.14, 0.045)), X, Z, 0.036, 0.03), (Vector((0.1 * seite, -0.17, 0.055)), X, Z, 0.014, 0.012),
                (Vector((0.1 * seite, -0.175, 0.07)), X, Z, 0.003, 0.003)]
        f.loft("Stiefel", fuss, 16, lambda i, k, p: stiefel * (0.55 if p.center.z < 0.012 else 1.0),
               lambda co, sn=sn: {f"Fuss.{sn}": 1.0}, oben_zu=True, unten_zu=True, teilung=3)

    # ================= Kopf: junges Gesicht =================
    kopf = [
        (Vector((0, 0.005, 1.75)), (0.09, 0.102, 0.11)),                 # Schädel
        (Vector((0, -0.022, 1.685)), (0.076, 0.072, 0.068)),             # Kiefer und Wangen
        (Vector((0.048, -0.07, 1.72)), (0.028, 0.024, 0.02)),            # Wangenknochen
        (Vector((-0.048, -0.07, 1.72)), (0.028, 0.024, 0.02)),
        (Vector((0, -0.082, 1.776)), (0.064, 0.026, 0.018)),             # Brauenbogen
        (Vector((0, -0.1, 1.742)), (0.012, 0.02, 0.028)),                # gerade Nase
        (Vector((0, -0.114, 1.718)), (0.014, 0.02, 0.018)),
        (Vector((0, -0.124, 1.706)), (0.016, 0.016, 0.014)),
        (Vector((0.016, -0.114, 1.702)), (0.01, 0.011, 0.009)),          # Nasenflügel
        (Vector((-0.016, -0.114, 1.702)), (0.01, 0.011, 0.009)),
        (Vector((0, -0.078, 1.642)), (0.036, 0.03, 0.028)),              # kantiges Kinn
        (Vector((0, -0.098, 1.672)), (0.019, 0.011, 0.007)),             # Unterlippe
        (Vector((0, -0.105, 1.679)), (0.021, 0.01, 0.0032), True),       # Mundspalte
        (Vector((0, 0.0, 1.61)), (0.05, 0.05, 0.06)),                    # Hals
    ]
    for s in (1, -1):
        kopf += [
            (_spiegel(AUGE, s) + Vector((0, -0.006, 0)), (0.019, 0.013, 0.013), True),       # Augenhöhle
            (_spiegel(AUGE, s) + Vector((0, -0.007, 0.01)), (0.017, 0.009, 0.005)),          # Oberlid
            (Vector((0.092 * s, 0.008, 1.735)), (0.013, 0.028, 0.04)),                        # Ohr
        ]

    def gesicht(poly):
        p = poly.center
        c = haut * (0.95 if p.z < 1.66 else 1.0)
        if (p - Vector((0, -0.098, 1.672))).length < 0.018 and p.y < -0.09:
            return lippe
        # Kurzer Bart an Kinn, Kiefer und über der Lippe
        bart = p.y < -0.02 and (p.z < 1.672 or (abs(p.x) < 0.03 and 1.683 < p.z < 1.695 and p.y < -0.1)) and p.z > 1.6
        if bart and not (p - Vector((0, -0.098, 1.672))).length < 0.02:
            return haar * (0.9 + 0.1 * ((poly.index * 7) % 3) / 2)
        rosig = math.exp(-(((abs(p.x) - 0.05) ** 2 + (p.z - 1.718) ** 2) / 0.016 ** 2)) if p.y < -0.05 else 0.0
        return c.lerp(wange, rosig * 0.6)

    f.metaball("Kopf", kopf, 0.0045, 5000, gesicht,
               lambda co: _mischen(("Kopf", weich(1.6, 1.66, co.z)), ("Hals", 1 - weich(1.6, 1.66, co.z))), glatt=True)
    for s in (1, -1):
        auge = _spiegel(AUGE, s)
        f.kugel("Augapfel", auge, (0.012, 0.012, 0.012), farbe("#EDE6DA"), kopf_gewicht, 16, 12)
        f.kugel("Iris", auge + Vector((-0.001 * s, -0.0103, 0.0005)), (0.0068, 0.0028, 0.0068), farbe("#4E8A4A"), kopf_gewicht, 14, 8)
        f.kugel("Pupille", auge + Vector((-0.001 * s, -0.012, 0.0005)), (0.0034, 0.0016, 0.0034), farbe("#0E1116"), kopf_gewicht, 10, 6)
        f.kugel("Glanz", auge + Vector((-0.004 * s, -0.0126, 0.0035)), (0.0016, 0.0008, 0.0016), farbe("#FFFFFF"), kopf_gewicht, 8, 4)
        f.kugel("Nasenloch", Vector((0.009 * s, -0.122, 1.694)), (0.004, 0.0035, 0.0025), nasenloch, kopf_gewicht, 10, 6)
        # Gerade, kräftige Brauen
        for j in range(7):
            x = 0.014 + 0.0095 * j
            basis = Vector((x * s, -0.104 + 1.3 * (x - 0.03) ** 2, 1.772 + 0.004 * math.sin(math.pi * x / 0.08)))
            richtung = Vector((0.7 * s, -0.2, 0.3 + r.uniform(-0.1, 0.15))).normalized()
            laenge = 0.016 + 0.01 * (j / 6)
            f.straehne("Braue", [basis, basis + richtung * laenge * 0.6, basis + richtung * laenge], 0.0055, 0.0012, haar_dunkel, kopf_gewicht, 6, 0.4, 0.6)
    # Schnurrbart
    for s in (1, -1):
        for j in range(4):
            start = Vector((s * (0.004 + 0.005 * j), -0.116, 1.69))
            ende = Vector((s * (0.028 + 0.004 * j), -0.106, 1.678 - 0.003 * j))
            f.straehne("Schnurrbart", [start, start.lerp(ende, 0.5) + Vector((0, -0.003, 0)), ende], 0.0045, 0.0012, haar, kopf_gewicht, 5, 0.3, 0.7)
    # Haar: Grundmasse und Strähnen, die unter der Kapuze in die Stirn und an den Seiten hervorkommen
    f.metaball("Haar", [(Vector((0, 0.02, 1.775)), (0.098, 0.1, 0.1)), (Vector((0, 0.06, 1.68)), (0.09, 0.06, 0.07))], 0.009, 800,
               lambda poly: haar, kopf_gewicht)
    for n in range(14):
        theta = math.radians(-60 + 120 * n / 13 + r.uniform(-4, 4))
        aussen = Vector((math.sin(theta), -math.cos(theta), 0))
        start = aussen * 0.085 + Vector((0, 0.0, 1.82))
        punkte = [start, start + aussen * 0.02 + Vector((0, -0.01, -0.03)), start + aussen * 0.03 + Vector((0, -0.005, -0.06 - 0.02 * abs(math.sin(theta))))]
        f.straehne("Strähne", punkte, r.uniform(0.01, 0.014), 0.003, haar * r.uniform(0.85, 1.05), kopf_gewicht, 6, 0.4, 0.7)

    # ================= Kapuze (auf) und Umhang =================
    hauptmitte = Vector((0, 0.012, 1.745))

    def kapuzen_form(th, ph):
        # Vorne weiter (Gesichtsöffnung), oben leicht spitz nach hinten
        vorne = max(0.0, math.cos(th))
        return 1.0 + 0.1 * vorne * max(0.0, math.sin(ph)) + 0.05 * math.sin(ph) ** 8

    def kapuzen_farbe(poly):
        rand = abs(math.degrees(math.atan2(poly.center.x, -(poly.center.y - hauptmitte.y)))) < 50
        return umhang_dunkel if rand else umhang * (0.92 + 0.08 * ((poly.index * 13) % 5) / 4)

    kapuzen_gewicht = lambda co: _mischen(("Kopf", weich(1.6, 1.7, co.z)), ("Brust", 1 - weich(1.6, 1.7, co.z)))
    _schale(f, "Kapuze", hauptmitte, 0.128, 0.138, 0.145, (48, 312), (-55, 88), kapuzen_farbe, kapuzen_gewicht, seg=(40, 14), form=kapuzen_form)
    _schale(f, "KapuzeInnen", hauptmitte, 0.12, 0.13, 0.137, (48, 312), (-55, 88), lambda poly: umhang_innen, kapuzen_gewicht, innen=True, seg=(32, 12),
            form=kapuzen_form)
    # Zipfel der Kapuze fällt über den Rücken
    zipfel = [hauptmitte + Vector((0, 0.13, 0.07)), hauptmitte + Vector((0, 0.17, 0.0)), hauptmitte + Vector((0, 0.2, -0.1)), hauptmitte + Vector((0, 0.22, -0.2))]
    f.straehne("Zipfel", zipfel, 0.05, 0.012, umhang, kapuzen_gewicht, 10, 0.0, 0.5)
    # Kragen und Umhang über Schultern und Rücken
    f.loft("Kragen", [(Vector((0, 0.01, 1.52)), X, Y, 0.215, 0.16, faltig(0.03)), (Vector((0, 0.01, 1.6)), X, Y, 0.12, 0.11, faltig(0.02))], 48,
           lambda i, k, p: umhang * (0.95 if k % 6 else 0.85), _rumpf_gewichte, teilung=2)

    def umhang_form(th, ph):
        # Nach unten weiter, sanfte Falten
        unten = max(0.0, -math.sin(ph))
        return 1.0 + 0.35 * unten + 0.04 * math.sin(th * 9 + ph * 2) * unten

    umhang_gewicht = lambda co: _mischen(("Brust", weich(1.05, 1.35, co.z)), ("Bauch", 1 - weich(1.05, 1.35, co.z)))
    mitte_umhang = Vector((0, 0.03, 1.5))
    _schale(f, "Umhang", mitte_umhang, 0.2, 0.15, 0.62, (95, 265), (-80, 5), lambda poly: umhang * (0.9 + 0.1 * ((poly.index * 7) % 5) / 4),
            umhang_gewicht, seg=(36, 16), form=umhang_form)
    _schale(f, "UmhangInnen", mitte_umhang, 0.192, 0.142, 0.61, (95, 265), (-80, 5), lambda poly: umhang_innen, umhang_gewicht, innen=True,
            seg=(28, 12), form=umhang_form)
    f.kugel("Spange", (0.13, -0.11, 1.52), (0.02, 0.008, 0.02), gold, _rumpf_gewichte, 12, 6, glatt=False)
    f.stern("Blatt", Vector((0.13, -0.119, 1.52)), -Y, 0.014, gruen_hell, _rumpf_gewichte, zacken=3)

    # ================= Köcher mit Pfeilen und Gurt =================
    unten_k, oben_k = Vector((0.1, 0.26, 1.02)), Vector((-0.14, 0.24, 1.6))
    achse = (oben_k - unten_k).normalized()
    q = achse.cross(Y).normalized()
    q2 = achse.cross(q).normalized()
    ringe = [(unten_k.lerp(oben_k, t), q, q2, rad, rad * 0.85) for t, rad in ((0.0, 0.05), (0.05, 0.062), (0.9, 0.068), (1.0, 0.07))]

    def koecher_farbe(i, k, p):
        t = (p.center - unten_k).dot(achse) / (oben_k - unten_k).length
        if t > 0.9 or t < 0.08:
            return leder_dunkel
        if abs(t - 0.5) < 0.03:
            return messing
        return leder * (0.9 + 0.1 * ((k + int(t * 8)) % 3) / 2)

    f.loft("Koecher", ringe, 16, koecher_farbe, _rumpf_gewichte, unten_zu=True, teilung=2)
    for n in range(7):
        w = math.tau * n / 7
        basis = oben_k + (q * math.cos(w) + q2 * math.sin(w)) * 0.035 - achse * 0.02
        spitze = basis + achse * r.uniform(0.12, 0.18) + (q * math.cos(w) + q2 * math.sin(w)) * 0.01
        f.loft("Pfeilschaft", [(basis, q, q2, 0.006, 0.006), (spitze, q, q2, 0.006, 0.006)], 5, lambda i, k, p: holz, _rumpf_gewichte, oben_zu=True)
        feder = farbe("#C8322A") if n % 3 else farbe("#F0EADA")
        for j in range(3):
            v = w + math.tau * j / 3
            aus = (q * math.cos(v) + q2 * math.sin(v))
            bm = bmesh.new()
            for seite in (1, -1):
                # Beide Seiten der Feder (eigene Eckpunkte, sonst gäbe es die Fläche doppelt)
                a = bm.verts.new(spitze - achse * 0.09)
                b = bm.verts.new(spitze - achse * 0.01)
                c = bm.verts.new(spitze - achse * 0.07 + aus * 0.022)
                bm.faces.new((a, b, c) if seite > 0 else (a, c, b))
            f._objekt(bm, "Feder", lambda poly, feder=feder: feder, _rumpf_gewichte)
    gurt = [Vector((-0.17, -0.1, 1.5)), Vector((-0.08, -0.155, 1.36)), Vector((0.06, -0.16, 1.2)), Vector((0.17, -0.12, 1.06))]
    f.loft("Gurt", [(p, X, Y, 0.024, 0.006) for p in gurt], 8, lambda i, k, p: leder_dunkel, _rumpf_gewichte, oben_zu=True, unten_zu=True, teilung=3)
    f.kugel("Gurtschnalle", (0.0, -0.162, 1.27), (0.018, 0.006, 0.016), messing, _rumpf_gewichte, 8, 4, glatt=False)

    # ================= Bogen (linke Hand) =================
    from waffen import BOEGEN, bogen as bogen_bauen
    bogen_gewicht = lambda co: {"Hand.L": 1.0}
    anfang = len(f.teile)
    bogen_bauen(f, "Bogen", BOGEN_GRIFF, bogen_gewicht)
    f.als_starr("Bogen", "Hand.L", anfang)
    for art in BOEGEN:
        anfang = len(f.teile)
        bogen_bauen(f, art, BOGEN_GRIFF, bogen_gewicht)
        f.als_starr(art, "Hand.L", anfang)

    # ================= Spitzhacke und Axt (rechte Hand, wie beim Magier) =================
    hand_gewicht = lambda co: {"Hand.R": 1.0}
    anfang = len(f.teile)
    _spitzhacke(f, STAB_X, STAB_Y, hand_gewicht)
    f.als_starr("Spitzhacke", "Hand.R", anfang)
    anfang = len(f.teile)
    _axt(f, STAB_X, STAB_Y, hand_gewicht)
    f.als_starr("Axt", "Hand.R", anfang)

    # ================= Skelett (wie der Magier) =================
    f.knochen_dazu("Becken", (0, 0, 0.95), (0, 0, 1.1), None, HOCH)
    f.knochen_dazu("Bauch", (0, 0, 1.1), (0, 0, 1.3), "Becken", HOCH)
    f.knochen_dazu("Brust", (0, 0, 1.3), (0, 0, 1.56), "Bauch", HOCH)
    f.knochen_dazu("Hals", (0, 0, 1.56), (0, 0, 1.65), "Brust", HOCH)
    f.knochen_dazu("Kopf", (0, 0, 1.65), (0, 0, 1.9), "Hals", HOCH)
    f.knochen_dazu("Hut", (0, 0.02, 1.9), (0, 0.15, 2.0), "Kopf", HOCH)
    for seite, sn in ((1, "L"), (-1, "R")):
        f.knochen_dazu(f"Oberarm.{sn}", _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), "Brust", HAENGT)
        f.knochen_dazu(f"Unterarm.{sn}", _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite), f"Oberarm.{sn}", HAENGT)
        f.knochen_dazu(f"Hand.{sn}", _spiegel(HANDGELENK, seite), _spiegel(FINGER, seite), f"Unterarm.{sn}", HAENGT)
        f.knochen_dazu(f"Oberschenkel.{sn}", _spiegel(HUEFTE, seite), _spiegel(KNIE, seite), "Becken", HAENGT)
        f.knochen_dazu(f"Unterschenkel.{sn}", _spiegel(KNIE, seite), _spiegel(KNOECHEL, seite), f"Oberschenkel.{sn}", HAENGT)
        f.knochen_dazu(f"Fuss.{sn}", _spiegel(KNOECHEL, seite), _spiegel(ZEHEN, seite), f"Unterschenkel.{sn}", (0, 0, 1))

    return f.fertig(_bogen_animationen)


def _bogen_animationen(armatur):
    """Wie der Magier (figuren.py), aber der Bogen bleibt in der Linken vor dem Körper (Unterarm
    gebeugt, dann steht er senkrecht) und eigene Schüsse."""
    import bpy
    from figuren import _magier_animationen
    _magier_animationen(armatur)
    for alt in ("Idle", "Laufen", "Rennen", "Arkan", "Feuerball", "Frostnova", "Meteor"):
        aktion = bpy.data.actions.get(alt)
        if aktion:
            bpy.data.actions.remove(aktion)

    def idle(phi):
        return [
            ("Brust", "rot", (1.5 * math.sin(phi * 2), 0, 0)), ("Bauch", "rot", (-0.8 * math.sin(phi * 2), 0, 0)),
            ("Kopf", "rot", (2 * math.sin(phi * 2 + 1), 14 * math.sin(phi), 2 * math.sin(phi))),
            ("Oberarm.L", "rot", (-8 + 2 * math.sin(phi * 2), 0, 10)), ("Unterarm.L", "rot", (-78 - 2 * math.sin(phi * 2), 0, 0)),
            ("Hand.L", "rot", (0, 0, 0)),
            ("Oberarm.R", "rot", (3 * math.sin(phi * 2), 0, 0)), ("Unterarm.R", "rot", (-14, 0, 0)),
            ("Oberschenkel.L", "rot", (0, 0, 3)), ("Oberschenkel.R", "rot", (0, 0, -3)),
            ("Becken", "pos", (0, -0.004 * (1 - math.cos(phi * 2)), 0)),
        ]
    animation(armatur, "Idle", 180, _schleife(180, 6, idle))

    def gehen(phi, schwung, knie, arm, huepfen, vorbeugen, ellbogen):
        werte = _gehen(phi, schwung, knie, arm, huepfen, vorbeugen, ellbogen)
        # Linker Arm hält den Bogen: Unterarm gebeugt, schwingt nur wenig
        s = math.sin(phi)
        werte = [w for w in werte if w[0] not in ("Oberarm.L", "Unterarm.L")]
        werte += [("Oberarm.L", "rot", (-6 - arm * 0.25 * s, 0, 10)), ("Unterarm.L", "rot", (-78 - ellbogen * 0.2, 0, 0))]
        return werte
    animation(armatur, "Laufen", 30, _schleife(30, 2, lambda phi: gehen(phi, 28, 45, 22, 0.03, 4, 14)))
    animation(armatur, "Rennen", 20, _schleife(20, 2, lambda phi: gehen(phi, 44, 80, 40, 0.06, 14, 30)))
    _bogen_schuesse(armatur)


def _bogen_schuesse(armatur):
    """Schuss, Salve, Sprengschuss, Himmelsschuss – siehe game/src/faehigkeiten.rs (Bild der Wirkung)."""
    ruhe = {"Oberarm.L": (-8, 0, 10), "Unterarm.L": (-78, 0, 0), "Oberarm.R": (0, 0, 0), "Unterarm.R": (-14, 0, 0)}
    # Schussstellung: seitlich zum Ziel (die linke Schulter vorne), der linke Arm gestreckt zum Ziel,
    # die rechte Hand zieht die Sehne bis ans Kinn
    def anschlag(dreh=62, zug=1.0, hoch=0.0):
        # dreh: so weit dreht sich der Oberkörper nach rechts (+Y dreht nach links)
        return {"Becken": (0, -dreh * 0.3, 0), "Bauch": (0, -dreh * 0.3, 0), "Brust": (-2 - hoch * 0.3, -dreh * 0.4, 0), "Kopf": (-hoch * 0.5, dreh * 0.85, 0),
                "Oberarm.L": (-88 - hoch, 0, dreh * 0.55), "Unterarm.L": (-4, 0, 0), "Hand.L": (0, 0, 0),
                "Oberarm.R": (-80 - hoch * 0.8, 0, -45 - 25 * zug), "Unterarm.R": (-40 - 105 * zug, 0, 0),
                "Oberschenkel.L": (-10, 0, 8), "Unterschenkel.L": (10, 0, 0), "Oberschenkel.R": (8, 0, -8), "Unterschenkel.R": (6, 0, 0)}

    def los(pose, rueck=10):
        # Die Zughand schnellt nach hinten, der Bogenarm federt nach
        p = dict(pose)
        x, y, z = p["Oberarm.R"]
        p["Oberarm.R"] = (x + rueck, y, z - 10)
        p["Unterarm.R"] = (-80, 0, 0)
        x, y, z = p["Oberarm.L"]
        p["Oberarm.L"] = (x + 6, y, z)
        return p

    # Schuss: schnell anlegen (Bild 5), voll gespannt (7), Schuss (8), nachhalten, zurück
    _clip(armatur, "Schuss", 20, [(0, ruhe), (5, anschlag(zug=0.7)), (7, anschlag()), (8, los(anschlag())), (12, los(anschlag(), 14)), (20, ruhe)])
    # Salve: den Bogen etwas schräg (Fächer), weit ausziehen, Schuss (11)
    schraeg = lambda z: _mit(z, Hand_L=(0, 25, 0))
    _clip(armatur, "Salve", 26, [(0, ruhe), (6, schraeg(anschlag(zug=0.6))), (10, schraeg(anschlag(zug=1.1))), (11, los(schraeg(anschlag(zug=1.1)), 16)),
                                 (16, los(schraeg(anschlag(zug=1.1)), 16)), (26, ruhe)])
    # Sprengschuss: tief in den Stand, lange spannen und zielen, Schuss (13)
    tief = lambda p: {**p, "Oberschenkel.L": (-22, 0, 10), "Unterschenkel.L": (26, 0, 0), "Oberschenkel.R": (14, 0, -10), "Unterschenkel.R": (22, 0, 0),
                      "Becken.pos": (0, 0, -0.06)}
    _clip(armatur, "Sprengschuss", 30, [(0, ruhe), (6, tief(anschlag(zug=0.5))), (11, tief(anschlag(zug=1.15))), (13, los(tief(anschlag(zug=1.15)), 18)),
                                        (19, los(tief(anschlag(zug=1.15)), 18)), (30, ruhe)])
    # Himmelsschuss (Pfeilregen): steil nach oben zielen, zurücklehnen, Schuss (16)
    oben = lambda zug: anschlag(dreh=55, zug=zug, hoch=45)
    _clip(armatur, "Himmelsschuss", 32, [(0, ruhe), (8, oben(0.6)), (14, oben(1.15)), (16, los(oben(1.15), 20)), (22, los(oben(1.15), 20)), (32, ruhe)])
