"""Ritter der Schlosswache (NPC) – gebaut mit dem Figuren-Baukasten aus figuren.py.

Plattenrüstung mit Kniekacheln und Eisenschuhen, grüner Wappenrock mit Goldrand und Wappen,
große Schulterstücke, Topfhelm mit Sehschlitz und Federbusch, Umhang, Schwert in der Scheide,
Wappenschild am linken Arm und eine Hellebarde in der rechten Hand.
Animationen: Idle (Wache stehen), Laufen (Marschieren).
Koordinaten wie bei allen Figuren: Z oben, Blick nach -Y, Füße im Ursprung.
"""

import math

import bmesh
from mathutils import Vector

from figuren import (ELLBOGEN, HANDGELENK, HAENGT, HOCH, HUEFTE, KNIE, KNOECHEL, SCHULTER, STAB_X, STAB_Y, X, Y, Z, ZEHEN,
                     Figur, FINGER, _arm_gewichte, _gehen, _mischen, _robe_gewichte, _rumpf_gewichte, _schleife, _spiegel,
                     farbe, weich)
from werkstatt import animation

STAHL = farbe("#A9B0BA")
STAHL_DUNKEL = farbe("#6C737D")
STAHL_HELL = farbe("#D2D7DE")
GOLD = farbe("#D8AE4A")
GOLD_DUNKEL = farbe("#A9812E")
ROCK = farbe("#2E6B46")
ROCK_DUNKEL = farbe("#1F4B31")
UMHANG = farbe("#234C35")
LEDER = farbe("#5A3A24")
HOLZ = farbe("#6E4B2E")
SCHLITZ = farbe("#121418")


def _bein_gewichte(sn):
    return lambda co: _mischen((f"Unterschenkel.{sn}", 1 - weich(0.45, 0.6, co.z)), (f"Oberschenkel.{sn}", weich(0.45, 0.6, co.z)))


def ritter(seed=21, name="Ritter"):
    f = Figur(name, seed)
    r = f.rng
    kopf = lambda co: {"Kopf": 1.0}

    # ================= Beine: Beinschienen, Kniekacheln, Eisenschuhe =================
    for seite in (1, -1):
        sn = "L" if seite > 0 else "R"
        k_, h_ = _spiegel(KNIE, seite), _spiegel(HUEFTE, seite)
        x = 0.1 * seite
        ringe = [(Vector((x, 0.02, 0.07)), X, Y, 0.055, 0.058), (Vector((x, 0.015, 0.2)), X, Y, 0.06, 0.064),
                 (Vector((x, 0.0, 0.38)), X, Y, 0.068, 0.072), (k_ + Vector((0, 0, -0.04)), X, Y, 0.066, 0.07),
                 (k_ + Vector((0, 0, 0.06)), X, Y, 0.074, 0.076), (Vector((x, 0.0, 0.75)), X, Y, 0.085, 0.088),
                 (h_ + Vector((0, 0, -0.04)), X, Y, 0.095, 0.095)]
        f.loft("Beinschiene", ringe, 20, lambda i, k, p: STAHL * (0.86 if i in (2, 4) else 1.0) * (1.08 if 4 <= k <= 6 else 1.0),
               _bein_gewichte(sn), teilung=2, glatt=True)
        f.kugel("Kniekachel", k_ + Vector((0, -0.07, 0.01)), (0.058, 0.035, 0.06), STAHL_HELL, _bein_gewichte(sn), 14, 8)
        f.stern("Kniezier", k_ + Vector((0, -0.105, 0.01)), -Y, 0.022, GOLD, _bein_gewichte(sn), zacken=4)
        schuh = [(Vector((x, 0.06, 0.05)), X, Z, 0.055, 0.05), (Vector((x, 0.05, 0.02)), X, Z, 0.062, 0.03),
                 (Vector((x, -0.02, 0.055)), X, Z, 0.064, 0.06), (Vector((x, -0.09, 0.045)), X, Z, 0.052, 0.042),
                 (Vector((x, -0.15, 0.045)), X, Z, 0.032, 0.026), (Vector((x, -0.2, 0.05)), X, Z, 0.01, 0.01)]
        f.loft("Eisenschuh", schuh, 16, lambda i, k, p: STAHL_DUNKEL * (0.8 if int(p.center.y * 50) % 2 else 1.0),
               lambda co, sn=sn: {f"Fuss.{sn}": 1.0}, oben_zu=True, unten_zu=True, teilung=2)

    # ================= Rumpf: Brustpanzer =================
    rumpf = [(Vector((0, 0.0, 0.94)), X, Y, 0.17, 0.13), (Vector((0, 0.0, 1.08)), X, Y, 0.16, 0.125),
             (Vector((0, -0.01, 1.25)), X, Y, 0.19, 0.15), (Vector((0, -0.005, 1.42)), X, Y, 0.215, 0.155),
             (Vector((0, 0.0, 1.53)), X, Y, 0.19, 0.135), (Vector((0, 0.005, 1.6)), X, Y, 0.08, 0.075)]
    f.loft("Brustpanzer", rumpf, 36, lambda i, k, p: STAHL * (0.92 + 0.12 * max(0.0, -p.normal.y)), _rumpf_gewichte, oben_zu=True, teilung=3,
           glatt=True)

    # ================= Wappenrock =================
    falten = [r.uniform(0.6, 1.4) for _ in range(6)]

    def faltig(staerke):
        return lambda w: 1.0 + staerke * sum(math.sin(w * (3 + i) + falten[i]) * falten[i] / (3 + i) for i in range(5))

    rock = [(Vector((0, -0.005, 1.5)), X, Y, 0.205, 0.148), (Vector((0, -0.012, 1.3)), X, Y, 0.2, 0.158),
            (Vector((0, 0.0, 1.1)), X, Y, 0.172, 0.138), (Vector((0, 0.0, 0.93)), X, Y, 0.19, 0.15, faltig(0.03)),
            (Vector((0, 0.01, 0.75)), X, Y, 0.215, 0.17, faltig(0.05)), (Vector((0, 0.015, 0.55)), X, Y, 0.235, 0.19, faltig(0.07))]

    def rock_farbe(i, k, p):
        if i >= 4.6:
            return GOLD                                                     # Saum
        vorne = -p.normal.y > 0.55
        if vorne and 1.18 < p.center.z < 1.44 and abs(p.center.x) < 0.075:
            return GOLD if abs(p.center.x) + abs(p.center.z - 1.31) * 0.6 < 0.07 else ROCK   # Wappen (Raute)
        if abs(p.center.x) > 0.17 and p.center.z > 1.0:
            return ROCK_DUNKEL
        return ROCK * (0.92 + 0.1 * weich(0.5, 1.5, p.center.z))

    f.loft("Wappenrock", rock, 48, rock_farbe, _robe_gewichte, teilung=3)
    f.stern("Wappenstern", Vector((0, -0.172, 1.31)), -Y, 0.035, farbe("#B23A3A"), _rumpf_gewichte, zacken=4)

    # ================= Gürtel, Schwert =================
    f.loft("Guertel", [(Vector((0, 0.0, 1.05)), X, Y, 0.178, 0.143), (Vector((0, 0.0, 1.1)), X, Y, 0.176, 0.141)], 40,
           lambda i, k, p: LEDER * (0.8 if k % 6 == 0 else 1.0), _rumpf_gewichte, teilung=2)
    f.kiste("Schnalle", (0, -0.146, 1.075), (0.05, 0.012, 0.045), GOLD, _rumpf_gewichte)
    scheide_oben, scheide_unten = Vector((0.2, -0.02, 1.02)), Vector((0.27, 0.12, 0.38))
    achse = (scheide_unten - scheide_oben).normalized()
    quer = achse.cross(Y).normalized()
    f.loft("Scheide", [(scheide_oben, quer, achse.cross(quer), 0.028, 0.014), (scheide_unten, quer, achse.cross(quer), 0.022, 0.012)], 8,
           lambda i, k, p: LEDER * 0.85, _robe_gewichte, oben_zu=True, unten_zu=True, teilung=4)
    f.kugel("Ortband", scheide_unten, (0.026, 0.016, 0.02), GOLD, _robe_gewichte, 8, 6)
    griff_o = scheide_oben - achse * 0.2
    f.loft("Griff", [(scheide_oben - achse * 0.02, quer, achse.cross(quer), 0.014, 0.014), (griff_o, quer, achse.cross(quer), 0.012, 0.012)], 8,
           lambda i, k, p: LEDER * 0.6, _rumpf_gewichte, oben_zu=True, unten_zu=True)
    f.kiste("Parierstange", scheide_oben - achse * 0.02, (0.14, 0.02, 0.02), GOLD, _rumpf_gewichte)
    f.kugel("Knauf", griff_o - achse * 0.015, (0.022, 0.022, 0.022), GOLD, _rumpf_gewichte, 10, 6)

    # ================= Umhang =================
    umhang = []
    for i, z in enumerate((1.56, 1.4, 1.1, 0.8, 0.5, 0.3)):
        t = i / 5
        umhang.append((Vector((0, 0.13 + 0.12 * t, z)), X, Y, 0.2 + 0.12 * t, 0.018, lambda w, t=t: 1.0 + 0.04 * math.sin(w * 5 + t * 4)))

    def umhang_farbe(i, k, p):
        return GOLD if i >= 4.7 else UMHANG * (0.85 + 0.2 * weich(0.3, 1.5, p.center.z))

    f.loft("Umhang", umhang, 24, umhang_farbe, _robe_gewichte, oben_zu=True, unten_zu=True, teilung=3)

    # ================= Arme: Schulterstücke, Armzeug, Handschuhe =================
    for seite in (1, -1):
        s, e, h = _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite)
        ringe = []
        for t, (a, b), radius in ((0.0, (s, e), 0.075), (0.6, (s, e), 0.068), (1.0, (s, e), 0.064), (0.4, (e, h), 0.058),
                                  (0.85, (e, h), 0.062), (1.0, (e, h), 0.07)):
            ax = (b - a).normalized()
            q = ax.cross(Y).normalized()
            ringe.append((a.lerp(b, t), q, ax.cross(q).normalized(), radius, radius * 0.95))
        f.loft("Armzeug", ringe, 16, lambda i, k, p: STAHL * (0.88 if i >= 3 else 1.0), _arm_gewichte(seite), teilung=2, glatt=True)
        f.kugel("Ellbogenkachel", e + Vector((0.01 * seite, 0.05, 0)), (0.05, 0.045, 0.05), STAHL_HELL, _arm_gewichte(seite), 12, 8)
        # Schulterstück aus drei übereinanderliegenden Schalen
        for j in range(3):
            mitte = s + Vector((0.045 * seite, 0.0, 0.05 - j * 0.055))
            groesse = (0.12 - j * 0.008, 0.115 - j * 0.008, 0.07)
            f.kugel("Schulter", mitte, groesse, STAHL_HELL if j == 0 else STAHL, lambda co, seite=seite: _mischen(
                ("Brust", 0.45), (f"Oberarm.{'L' if seite > 0 else 'R'}", 0.55)), 16, 8)
        f.kugel("Schulterniete", s + Vector((0.1 * seite, -0.07, 0.07)), (0.014, 0.01, 0.014), GOLD, _arm_gewichte(seite), 8, 4)

    hand_l = lambda co: {"Hand.L": 1.0}
    f.kugel("HandschuhL", Vector((0.352, -0.02, 0.9)), (0.04, 0.05, 0.065), STAHL_DUNKEL, hand_l, 12, 8, glatt=False)
    f.kugel("StulpeL", Vector((0.345, -0.012, 0.965)), (0.05, 0.052, 0.035), STAHL, hand_l, 12, 6, glatt=False)
    hand_r = lambda co: {"Hand.R": 1.0}
    f.kugel("HandschuhR", Vector((STAB_X + 0.015, STAB_Y + 0.012, 0.915)), (0.045, 0.05, 0.06), STAHL_DUNKEL, hand_r, 12, 8, glatt=False)
    f.kugel("StulpeR", Vector((STAB_X + 0.03, STAB_Y + 0.03, 0.975)), (0.05, 0.052, 0.035), STAHL, hand_r, 12, 6, glatt=False)

    # ================= Hals und Helm =================
    f.loft("Halsberge", [(Vector((0, 0.0, 1.55)), X, Y, 0.13, 0.11), (Vector((0, 0.0, 1.6)), X, Y, 0.09, 0.085),
                         (Vector((0, 0.0, 1.66)), X, Y, 0.075, 0.075)], 20, lambda i, k, p: STAHL_DUNKEL,
           lambda co: _mischen(("Kopf", weich(1.6, 1.66, co.z)), ("Hals", 1 - weich(1.6, 1.66, co.z))))

    def helm_form(w):
        # vorne leicht spitz (Grat), hinten rund
        vorne = max(0.0, math.cos(w + math.pi / 2))
        return 1.0 + 0.06 * vorne ** 4

    helm = [(Vector((0, 0.0, 1.62)), X, Y, 0.1, 0.105, helm_form), (Vector((0, -0.005, 1.68)), X, Y, 0.118, 0.125, helm_form),
            (Vector((0, -0.005, 1.76)), X, Y, 0.12, 0.128, helm_form), (Vector((0, 0.0, 1.84)), X, Y, 0.118, 0.124, helm_form),
            (Vector((0, 0.005, 1.9)), X, Y, 0.1, 0.105), (Vector((0, 0.008, 1.95)), X, Y, 0.06, 0.064)]

    def helm_farbe(i, k, p):
        vorne = -p.normal.y > 0.35
        if vorne and 1.76 < p.center.z < 1.79:
            return SCHLITZ                                                  # Sehschlitz
        if vorne and 1.67 < p.center.z < 1.73 and p.center.x < -0.02 and int(p.center.z * 180) % 2 == 0:
            return SCHLITZ                                                  # Luftlöcher rechts
        if 1.835 < p.center.z < 1.86:
            return GOLD                                                     # Goldreif
        if abs(p.center.x) < 0.012 and vorne:
            return GOLD_DUNKEL                                              # Grat vorne
        return STAHL * (0.9 + 0.15 * max(0.0, p.normal.z))

    f.loft("Helm", helm, 40, helm_farbe, kopf, oben_zu=True, teilung=3, glatt=False)
    # Federbusch: Strähnen vom Scheitel nach hinten unten (am Knochen „Hut“, wippt beim Gehen)
    busch = lambda co: _mischen(("Kopf", 1 - weich(1.93, 2.0, co.z)), ("Hut", weich(1.93, 2.0, co.z)))
    f.kiste("Federhalter", (0, 0.02, 1.96), (0.03, 0.05, 0.04), GOLD, kopf)
    for n in range(9):
        w = (n - 4) * 0.09
        start = Vector((w * 0.15, 0.0, 1.98))
        punkte = []
        for j in range(6):
            t = j / 5
            punkte.append(start + Vector((w * 0.25 * t, 0.06 + 0.26 * t, 0.12 * math.sin(math.pi * t * 0.8) - 0.28 * t ** 2)))
        c = GOLD if n in (0, 8) else (ROCK if n % 2 else farbe("#E9E4D8"))
        f.straehne("Feder", punkte, 0.028, 0.004, c, busch, 7, 0.4, 0.45)

    # ================= Wappenschild am linken Unterarm =================
    schild_anfang = len(f.teile)
    schild_gewicht = lambda co: {"Unterarm.L": 1.0}
    umriss = [(0.0, 0.32), (0.19, 0.28), (0.21, 0.08), (0.14, -0.2), (0.0, -0.36), (-0.14, -0.2), (-0.21, 0.08), (-0.19, 0.28)]
    mitte = Vector((0.43, -0.03, 1.02))
    bm = bmesh.new()
    vorne = [bm.verts.new(mitte + Vector((0.025, y * 1.0, z))) for y, z in umriss]
    hinten = [bm.verts.new(mitte + Vector((-0.015, y * 1.0, z))) for y, z in umriss]
    n = len(umriss)
    for i in range(n):
        j = (i + 1) % n
        bm.faces.new((vorne[i], vorne[j], hinten[j], hinten[i]))
    bm.faces.new(vorne)
    bm.faces.new(list(reversed(hinten)))

    def schild_farbe(poly):
        if poly.normal.x > 0.5:
            return ROCK
        if poly.normal.x < -0.5:
            return LEDER
        return GOLD
    f._objekt(bm, "Schild", schild_farbe, schild_gewicht)
    # Goldrand und Wappen (Raute mit rotem Stein) auf der Außenseite
    for (y0, z0), (y1, z1) in zip(umriss, umriss[1:] + umriss[:1]):
        a, b = mitte + Vector((0.03, y0 * 0.97, z0 * 0.97)), mitte + Vector((0.03, y1 * 0.97, z1 * 0.97))
        laenge = (b - a).length
        richtung = (b - a).normalized()
        f.loft("Schildrand", [(a, X, richtung.cross(X).normalized(), 0.006, 0.014), (b, X, richtung.cross(X).normalized(), 0.006, 0.014)], 4,
               lambda i, k, p: GOLD, schild_gewicht, oben_zu=True, unten_zu=True)
        del laenge
    f.stern("Schildwappen", mitte + Vector((0.03, 0.0, 0.03)), X, 0.1, GOLD, schild_gewicht, zacken=4)
    f.kugel("Schildstein", mitte + Vector((0.036, 0.0, 0.03)), (0.01, 0.028, 0.028), farbe("#B23A3A"), schild_gewicht, 10, 6)
    f.als_starr("Schild", "Unterarm.L", schild_anfang)

    # ================= Hellebarde in der rechten Hand =================
    waffe_anfang = len(f.teile)
    waffe = lambda co: {"Hand.R": 1.0}
    schaft = [(Vector((STAB_X, STAB_Y, z)), X, Y, 0.018, 0.018) for z in (0.02, 0.6, 1.2, 1.8, 2.25)]
    f.loft("Schaft", schaft, 10, lambda i, k, p: HOLZ * (0.85 + 0.15 * ((k + int(i * 3)) % 3) / 2), waffe, unten_zu=True, teilung=2)
    for z in (0.9, 0.95, 2.05):
        f.loft("Wicklung", [(Vector((STAB_X, STAB_Y, z)), X, Y, 0.022, 0.022), (Vector((STAB_X, STAB_Y, z + 0.03)), X, Y, 0.022, 0.022)], 10,
               lambda i, k, p: LEDER if z < 1.5 else GOLD, waffe)
    kopf_z = 2.1
    # Axtblatt (nach vorne, -Y), Haken (nach hinten), Spitze (oben)
    klinge = [(0.0, 0.0), (-0.05, -0.02), (-0.2, -0.08), (-0.26, 0.02), (-0.24, 0.16), (-0.18, 0.24), (-0.05, 0.2), (0.0, 0.16)]
    bm = bmesh.new()
    a_ = [bm.verts.new(Vector((STAB_X - 0.008, STAB_Y + y, kopf_z + z))) for y, z in klinge]
    b_ = [bm.verts.new(Vector((STAB_X + 0.008, STAB_Y + y, kopf_z + z))) for y, z in klinge]
    for i in range(len(klinge)):
        j = (i + 1) % len(klinge)
        bm.faces.new((a_[i], a_[j], b_[j], b_[i]))
    bm.faces.new(list(reversed(a_)))
    bm.faces.new(b_)
    f._objekt(bm, "Axtblatt", lambda poly: STAHL_HELL if abs(poly.normal.x) > 0.5 else STAHL, waffe)
    haken = [(0.0, 0.06), (0.14, 0.1), (0.2, 0.02), (0.12, 0.08), (0.0, 0.12)]
    bm = bmesh.new()
    a_ = [bm.verts.new(Vector((STAB_X - 0.006, STAB_Y + y, kopf_z + z))) for y, z in haken]
    b_ = [bm.verts.new(Vector((STAB_X + 0.006, STAB_Y + y, kopf_z + z))) for y, z in haken]
    for i in range(len(haken)):
        j = (i + 1) % len(haken)
        bm.faces.new((a_[i], a_[j], b_[j], b_[i]))
    bm.faces.new(list(reversed(a_)))
    bm.faces.new(b_)
    f._objekt(bm, "Haken", lambda poly: STAHL, waffe)
    f.loft("Spitze", [(Vector((STAB_X, STAB_Y, kopf_z + 0.15)), X, Y, 0.024, 0.016), (Vector((STAB_X, STAB_Y, kopf_z + 0.3)), X, Y, 0.018, 0.012),
                      (Vector((STAB_X, STAB_Y, kopf_z + 0.46)), X, Y, 0.002, 0.002)], 6, lambda i, k, p: STAHL_HELL, waffe, unten_zu=True)
    f.kugel("Troddel", Vector((STAB_X, STAB_Y, kopf_z - 0.04)), (0.03, 0.03, 0.03), ROCK, waffe, 8, 6)
    f.als_starr("Hellebarde", "Hand.R", waffe_anfang)

    # ================= Skelett (wie beim Magier; „Hut“ trägt hier den Federbusch) =================
    f.knochen_dazu("Becken", (0, 0, 0.95), (0, 0, 1.1), None, HOCH)
    f.knochen_dazu("Bauch", (0, 0, 1.1), (0, 0, 1.3), "Becken", HOCH)
    f.knochen_dazu("Brust", (0, 0, 1.3), (0, 0, 1.56), "Bauch", HOCH)
    f.knochen_dazu("Hals", (0, 0, 1.56), (0, 0, 1.65), "Brust", HOCH)
    f.knochen_dazu("Kopf", (0, 0, 1.65), (0, 0, 1.9), "Hals", HOCH)
    f.knochen_dazu("Hut", (0, 0.02, 1.98), (0, 0.2, 2.05), "Kopf", HOCH)
    for seite, sn in ((1, "L"), (-1, "R")):
        f.knochen_dazu(f"Oberarm.{sn}", _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), "Brust", HAENGT)
        f.knochen_dazu(f"Unterarm.{sn}", _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite), f"Oberarm.{sn}", HAENGT)
        f.knochen_dazu(f"Hand.{sn}", _spiegel(HANDGELENK, seite), _spiegel(FINGER, seite), f"Unterarm.{sn}", HAENGT)
        f.knochen_dazu(f"Oberschenkel.{sn}", _spiegel(HUEFTE, seite), _spiegel(KNIE, seite), "Becken", HAENGT)
        f.knochen_dazu(f"Unterschenkel.{sn}", _spiegel(KNIE, seite), _spiegel(KNOECHEL, seite), f"Oberschenkel.{sn}", HAENGT)
        f.knochen_dazu(f"Fuss.{sn}", _spiegel(KNOECHEL, seite), _spiegel(ZEHEN, seite), f"Unterschenkel.{sn}", (0, 0, 1))

    return f.fertig(_ritter_animationen)


def _ritter_animationen(armatur):
    # Wache stehen: aufrecht, ruhiges Atmen, der Blick schweift langsam, Federbusch wippt
    def idle(phi):
        return [
            ("Brust", "rot", (1.2 * math.sin(phi * 2), 0, 0)), ("Bauch", "rot", (-0.6 * math.sin(phi * 2), 0, 0)),
            ("Kopf", "rot", (1.5 * math.sin(phi * 2 + 1), 18 * math.sin(phi), 0)),
            ("Hut", "rot", (4 * math.sin(phi * 2 + 1.5), 0, 3 * math.sin(phi + 0.8))),
            ("Oberarm.L", "rot", (-8, 0, 6)), ("Unterarm.L", "rot", (-40, 0, 0)),
            ("Oberarm.R", "rot", (0, 0, 0)), ("Unterarm.R", "rot", (-8, 0, 0)),
            ("Becken", "pos", (0, 0, -0.003 * (1 - math.cos(phi * 2)))),
        ]
    animation(armatur, "Idle", 180, _schleife(180, 6, idle))

    # Marschieren: gleichmäßiger, etwas steifer Schritt; links hält den Schild vor dem Körper
    def marsch(phi):
        werte = _gehen(phi, 26, 40, 8, 0.025, 2, 12)
        werte = [w for w in werte if w[0] not in ("Oberarm.L", "Unterarm.L")]
        return werte + [("Oberarm.L", "rot", (-10 + 3 * math.sin(phi), 0, 6)), ("Unterarm.L", "rot", (-42, 0, 0))]
    animation(armatur, "Laufen", 30, _schleife(30, 2, marsch))
