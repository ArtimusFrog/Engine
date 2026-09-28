"""Händler auf dem Marktplatz der Burg (NPC) – Baukasten der Arbeiter (arbeiter.py).

Ein wohlgenährter, freundlicher Kaufmann um die fünfzig: weinrotes Wams mit Goldborten über dem
Bauch, gepuffte Leinenärmel, Pelzkragen, Gildenkette mit Medaillon, breiter Gürtel mit prallem
Geldbeutel und Schlüsselbund, weiches Barett mit zwei Federn, gepflegter grauer Bart. In der
Linken eine versiegelte Schriftrolle (Kassenbuch).

Animationen: Idle (steht, wippt, schaut sich um), Laufen, Gruessen (verbeugt sich und lädt mit
der Rechten ein).
"""

import math

from mathutils import Vector

from arbeiter import (GRIFF_L, HUEFTE, KNIE, KOPF_GEWICHT, SCHULTER, X, Y, Z, _huefte_bein, _huefte_bein_schuerze, _kopf, _koerper, _rumpf, _skelett, _spiegel)
from figuren import Figur, _achsen, _clip, _gehen, _mischen, _mit, _platte, _schale, _schleife, farbe, weich
from werkstatt import animation


def haendler(seed=81, name="Haendler"):
    f = Figur(name, seed)
    r = f.rng
    haut, lippe, wange = farbe("#E8B08C"), farbe("#B0645A"), farbe("#E48A78")
    haar = farbe("#8C8278")
    wein, wein_dunkel, wein_hell = farbe("#6A1E2C"), farbe("#44121C"), farbe("#8A2C3A")
    gold, gold_dunkel = farbe("#C8983E"), farbe("#8A6424")
    leinen, leinen_dunkel = farbe("#EFE6D2"), farbe("#C8BCA0")
    hose_c = farbe("#3A3430")
    leder, leder_dunkel = farbe("#7A4A2A"), farbe("#4A2C18")
    pelz, pelz_dunkel = farbe("#8A6A4A"), farbe("#5A4230")
    samt = farbe("#2A1E2E")

    def wams_farbe(p):
        # Weinrot mit dunkleren Seitenbahnen und hellem Schimmer vorne
        c = wein * (0.9 + 0.14 * max(0.0, -p.normal.y))
        if abs(p.center.x) > 0.15:
            c = wein_dunkel
        return c

    def aermel_farbe(p):
        # Gepuffte Leinenärmel mit Schlitzen, durch die weinroter Stoff scheint
        streifen = int((math.atan2(p.normal.x, p.normal.y) + math.pi) / math.tau * 10) % 2
        if p.center.z > 1.3 and streifen:
            return wein
        return leinen * (0.9 + 0.1 * max(0.0, p.normal.z + 0.3))

    def hose(p, s):
        return hose_c * (0.9 + 0.1 * max(0.0, -p.normal.y))

    _koerper(f, haut, hose_c, hose, wams_farbe, leder_dunkel, farbe("#20160E"), haut, leinen, aermel_farbe, False, hand_ziel=1400)

    # Puffärmel an den Oberarmen (bauschig, mit Schlitzen)
    for s in (1, -1):
        sch = _spiegel(SCHULTER, s)
        ell = _spiegel(Vector((0.27, 0.02, 1.2)), s)
        q1, q2 = _achsen(sch, ell)
        ringe = []
        for t, rad in ((0.0, 0.07), (0.2, 0.086), (0.45, 0.092), (0.7, 0.084), (0.88, 0.064), (0.95, 0.054)):
            ringe.append((sch.lerp(ell, t) + Vector((0.01 * s, 0, 0.02 * (1 - t))), q1, q2, rad, rad,
                          lambda w, t=t: 1.0 + 0.08 * math.sin(w * 8) * math.sin(t * math.pi)))
        wg = lambda co, sn="L" if s > 0 else "R": _mischen(("Brust", 1 - weich(1.42, 1.3, co.z)), (f"Oberarm.{sn}", weich(1.42, 1.3, co.z)))
        f.loft("Puffaermel", ringe, 20, lambda i, k, p: wein if k % 5 == 0 else leinen * (0.9 + 0.1 * (k % 2)), wg, oben_zu=True, teilung=2, glatt=True)

    # Wams über dem Bauch: vorne gewölbt, Goldborte am Saum, Knopfleiste
    wams = [(0.9, 0.0, 0.2, 0.15), (1.0, -0.02, 0.222, 0.18), (1.1, -0.034, 0.228, 0.194), (1.2, -0.026, 0.222, 0.18),
            (1.3, -0.012, 0.215, 0.156), (1.4, 0.0, 0.212, 0.134), (1.46, 0.01, 0.182, 0.112), (1.51, 0.014, 0.115, 0.085)]
    ringe = [(Vector((0, y, z)), X, Y, rx, ry) for z, y, rx, ry in wams]

    def vorne(z, x=0.0):
        """Vorderseite des Wamses (y) in Höhe z, x seitlich von der Mitte."""
        for (z0, y0, rx0, ry0), (z1, y1, rx1, ry1) in zip(wams, wams[1:]):
            if z0 <= z <= z1:
                t = (z - z0) / (z1 - z0)
                y, rx, ry = y0 + (y1 - y0) * t, rx0 + (rx1 - rx0) * t, ry0 + (ry1 - ry0) * t
                return y - ry * math.sqrt(max(0.0, 1 - (x / rx) ** 2))
        return -0.14
    f.loft("Wams", ringe, 44, lambda i, k, p: wams_farbe(p), _rumpf, teilung=3, glatt=True)
    schoss = [(Vector((0, -0.004, 0.93)), X, Y, 0.206, 0.158), (Vector((0, 0.0, 0.88)), X, Y, 0.222, 0.17), (Vector((0, 0.004, 0.82)), X, Y, 0.236, 0.182)]

    def schoss_farbe(i, k, p):
        if i > 1.8:
            return gold if k % 3 else gold_dunkel                                  # Goldborte am Saum
        if abs(p.center.x) < 0.012 and p.normal.y < 0:
            return wein_dunkel                                                     # vorne geschlitzt
        return wein * (0.88 + 0.12 * ((k // 2) % 2))                               # Falten
    f.loft("Wamsschoss", [(p, a, b, rx, ry, lambda w: 1.0 + 0.035 * math.sin(w * 10)) for p, a, b, rx, ry in schoss], 44, schoss_farbe,
           _huefte_bein_schuerze, teilung=3, glatt=True)
    # Pluderhose: bauschige, geschlitzte Oberschenkel unter dem Schoß
    for s in (1, -1):
        h_, k_ = _spiegel(HUEFTE, s), _spiegel(KNIE, s)
        ringe = []
        for t, rad in ((0.0, 0.1), (0.2, 0.125), (0.45, 0.132), (0.68, 0.118), (0.8, 0.09), (0.84, 0.075)):
            ringe.append((h_.lerp(k_, t) + Vector((0.012 * s, 0, 0.03 * (1 - t))), X, Y, rad, rad * 0.95,
                          lambda w, t=t: 1.0 + 0.1 * abs(math.sin(w * 5)) * math.sin(t * math.pi / 0.84)))

        def pluder(i, k, p):
            return wein_dunkel if k % 4 == 0 else (gold_dunkel if k % 4 == 2 and i > 1 and i < 4 else hose_c * (0.9 + 0.2 * max(0.0, -p.normal.y)))
        f.loft("Pluderhose", ringe, 28, pluder, _huefte_bein, teilung=2, glatt=True)
    for j in range(7):
        z = 1.44 - 0.07 * j
        f.kugel("Knopf", Vector((0, vorne(z) - 0.003, z)), (0.011, 0.006, 0.011), gold, _rumpf, 8, 5)
    # Goldene Borten links und rechts der Knopfleiste
    for s in (1, -1):
        punkte = [Vector((x * s, vorne(z, x) - 0.002, z)) for x, z in ((0.03, 1.45), (0.035, 1.3), (0.035, 1.12), (0.03, 0.96))]
        f.straehne("Borte", punkte, 0.012, 0.012, gold, _rumpf, 6, 0.0, 0.25, teilung=3, glatt=True)

    # Breiter Gürtel über dem Bauch mit großer Schnalle
    f.loft("Guertel", [(Vector((0, -0.016, 0.975)), X, Y, 0.216, 0.17), (Vector((0, -0.02, 1.035)), X, Y, 0.226, 0.182)], 44,
           lambda i, k, p: leder_dunkel * (0.9 if k % 8 == 0 else 1.0), _rumpf, teilung=2)
    f.kiste("Schnalle", (0, -0.2, 1.005), (0.07, 0.014, 0.058), gold, _rumpf)
    f.kiste("Schnallendorn", (0, -0.208, 1.005), (0.008, 0.006, 0.05), gold_dunkel, _rumpf)

    # Praller Geldbeutel an der rechten Hüfte, mit Zugband und herausblitzenden Münzen
    beutel = Vector((-0.2, -0.11, 0.9))
    f.metaball("Geldbeutel", [(beutel, (0.06, 0.05, 0.065)), (beutel + Vector((0, 0, 0.06)), (0.03, 0.028, 0.03)),
                              (beutel + Vector((0.01, -0.01, -0.03)), (0.058, 0.048, 0.045))], 0.006, 900,
               lambda poly: leder * (0.8 + 0.25 * max(0.0, poly.normal.z) + 0.08 * math.sin(poly.center.z * 200)), _huefte_bein_schuerze, glatt=True)
    f.loft("Zugband", [(beutel + Vector((0, 0, 0.075)), X, Y, 0.028, 0.026), (beutel + Vector((0, 0, 0.085)), X, Y, 0.03, 0.028)], 12,
           lambda i, k, p: gold_dunkel, _huefte_bein_schuerze)
    f.straehne("Beutelriemen", [beutel + Vector((0.0, 0.0, 0.09)), beutel + Vector((0.01, 0.0, 0.11)), Vector((-0.21, -0.08, 0.99))], 0.01, 0.01, leder_dunkel,
               _huefte_bein_schuerze, 6, 0.0, 0.4)
    for j in range(3):
        f.loft("Muenze", [(beutel + Vector((-0.02 + 0.02 * j, -0.03, 0.098 + 0.004 * j)), Vector((0.3, 1, 0)).normalized(), Z, 0.012, 0.012),
                          (beutel + Vector((-0.02 + 0.02 * j, -0.034, 0.098 + 0.004 * j)), Vector((0.3, 1, 0)).normalized(), Z, 0.012, 0.012)],
               10, lambda i, k, p: gold, _huefte_bein_schuerze, oben_zu=True, unten_zu=True)
    # Schlüsselbund links
    ring = Vector((0.22, -0.07, 0.96))
    f.loft("Schluesselring", [(ring + Vector((0, 0, -0.002)), Y, Z, 0.026, 0.026), (ring + Vector((0, 0, 0.002)), Y, Z, 0.026, 0.026)], 14,
           lambda i, k, p: farbe("#5A5E66"), _huefte_bein_schuerze)
    for j, w in enumerate((-0.4, 0.0, 0.35)):
        oben = ring + Vector((0.004, math.sin(w) * 0.02, -0.02))
        unten = oben + Vector((0.004, math.sin(w) * 0.03, -0.08))
        f.straehne("Schluessel", [oben, oben.lerp(unten, 0.5), unten], 0.007, 0.007, farbe("#8A8E96") if j != 1 else gold_dunkel, _huefte_bein_schuerze, 6, 0.0, 1.0)
        f.kiste("Bart", tuple(unten + Vector((0, -0.008, 0.012))), (0.006, 0.016, 0.014), farbe("#8A8E96"), _huefte_bein_schuerze)

    # Weiße Halskrause, darüber ein Pelzkragen über den Schultern
    krause = [(Vector((0, 0.01, 1.525)), X, Y, 0.078, 0.07), (Vector((0, 0.01, 1.555)), X, Y, 0.09, 0.082), (Vector((0, 0.01, 1.575)), X, Y, 0.072, 0.066)]
    f.loft("Halskrause", [(p, a, b, rx, ry, lambda w: 1.0 + 0.12 * abs(math.sin(w * 9))) for p, a, b, rx, ry in krause], 54,
           lambda i, k, p: leinen if k % 2 else leinen_dunkel, _rumpf, teilung=1, glatt=True)
    pelzkragen = _schale(f, "Pelzkragen", Vector((0, 0.01, 1.43)), 0.25, 0.18, 0.12, (-180, 180), (-10, 72),
                         lambda poly: pelz_dunkel if poly.normal.z < -0.3 else pelz * (0.82 + 0.18 * max(0.0, poly.normal.z)) * (0.94 + 0.06 * math.sin(poly.center.x * 40 + poly.center.z * 30)),
                         lambda co: _mischen(("Brust", 1.0 - 0.35 * weich(0.14, 0.25, abs(co.x))),
                                             ("Oberarm.L" if co.x > 0 else "Oberarm.R", 0.35 * weich(0.14, 0.25, abs(co.x)))), seg=(44, 10))
    for v in pelzkragen.data.vertices:
        v.co += v.normal * 0.01 * (0.5 + 0.5 * math.sin(v.co.x * 70 + v.co.y * 40) * math.sin(v.co.z * 60))

    # Gildenkette mit Medaillon
    for j in range(15):
        t = j / 14
        w = math.pi * (0.1 + 0.8 * t)
        p = Vector((math.cos(w) * 0.13, -0.13 - math.sin(w) * 0.065, 1.47 - math.sin(w) * 0.17))
        f.kugel("Kettenglied", p, (0.012, 0.008, 0.01), gold if j % 2 else gold_dunkel, _rumpf, 6, 4)
    medaillon = Vector((0, -0.2, 1.28))
    f.loft("Medaillon", [(medaillon, X, Z, 0.038, 0.038), (medaillon + Vector((0, -0.008, 0)), X, Z, 0.038, 0.038)], 18,
           lambda i, k, p: gold if i < 0.5 else gold_dunkel, _rumpf, oben_zu=True, unten_zu=True)
    f.stern("Waage", medaillon + Vector((0, -0.01, 0)), -Y, 0.022, farbe("#FFF0C0"), _rumpf, zacken=6)

    _kopf(f, haut, lippe, wange, haar, "kurz")

    # Weiches Barett, etwas schräg nach rechts, mit Goldband und zwei Federn
    barett = []
    for z, rx, ry, dx in ((1.72, 0.106, 0.116, 0.0), (1.745, 0.118, 0.128, -0.004), (1.772, 0.158, 0.162, -0.014), (1.795, 0.172, 0.172, -0.02),
                          (1.815, 0.162, 0.162, -0.022), (1.832, 0.12, 0.122, -0.02), (1.842, 0.05, 0.05, -0.016), (1.845, 0.006, 0.006, -0.014)):
        barett.append((Vector((dx, 0.008, z)), X, Y, rx, ry, lambda w: 1.0 + 0.03 * math.sin(w * 6)))
    f.loft("Barett", barett, 40, lambda i, k, p: samt * (0.85 + 0.25 * max(0.0, p.normal.z)) if i > 0.7 else gold_dunkel * (0.9 if k % 3 else 0.7), KOPF_GEWICHT,
           oben_zu=True, teilung=3, glatt=True)
    f.kugel("Hutbrosche", Vector((0.125, -0.08, 1.77)), (0.018, 0.012, 0.018), farbe("#C8303A"), KOPF_GEWICHT, 10, 6)
    feder = lambda i, k, p, c: c * (0.85 + 0.15 * (k % 2))
    _platte(f, "Feder", Vector((0.15, 0.05, 1.86)), 0.34, 0.035, 0.006, Vector((0.25, 1.0, 0.55)), Vector((1, 0, 0.2)),
            lambda i, k, p: feder(i, k, p, farbe("#F4EEDC")), lambda co: {"Hut": 1.0}, spitz=0.85, wolbung=2.0)
    _platte(f, "Feder", Vector((0.14, 0.02, 1.84)), 0.24, 0.026, 0.006, Vector((0.35, 1.0, 0.3)), Vector((1, 0, 0.3)),
            lambda i, k, p: feder(i, k, p, farbe("#C8303A")), lambda co: {"Hut": 1.0}, spitz=0.85, wolbung=2.0)

    # Versiegelte Schriftrolle in der linken Faust (entlang Y)
    anfang = len(f.teile)
    pergament = farbe("#EADBB4")
    rolle = [(GRIFF_L + Vector((0, -0.14, 0)), X, Z, 0.022, 0.022), (GRIFF_L + Vector((0, -0.13, 0)), X, Z, 0.024, 0.024),
             (GRIFF_L + Vector((0, 0.13, 0)), X, Z, 0.024, 0.024), (GRIFF_L + Vector((0, 0.14, 0)), X, Z, 0.022, 0.022)]
    f.loft("Rolle", rolle, 14, lambda i, k, p: pergament * (0.85 if abs(p.normal.y) > 0.7 else 1.0), lambda co: {"Hand.L": 1.0}, oben_zu=True, unten_zu=True)
    f.loft("Band", [(GRIFF_L + Vector((0, -0.08, 0)), X, Z, 0.026, 0.026), (GRIFF_L + Vector((0, -0.065, 0)), X, Z, 0.026, 0.026)], 14,
           lambda i, k, p: farbe("#B02A30"), lambda co: {"Hand.L": 1.0})
    f.kugel("Siegel", GRIFF_L + Vector((0.026, -0.072, 0)), (0.008, 0.016, 0.016), farbe("#8A1A20"), lambda co: {"Hand.L": 1.0}, 10, 6)
    f.als_starr("Rolle", "Hand.L", anfang)
    # Goldring an der rechten Hand
    f.loft("Ring", [(Vector((-0.34, -0.085, 0.905)), X, Z, 0.012, 0.012), (Vector((-0.332, -0.085, 0.905)), X, Z, 0.012, 0.012)], 10,
           lambda i, k, p: gold, lambda co: {"Hand.R": 1.0})

    _skelett(f)
    return f.fertig(_animationen)


def _animationen(armatur):
    # Idle: die Rolle vor dem Bauch, wippt auf den Fersen, schaut sich nach Kundschaft um
    def idle(phi):
        return [
            ("Brust", "rot", (-2 + 1.5 * math.sin(phi * 2), 0, 0)), ("Bauch", "rot", (-3 - 0.8 * math.sin(phi * 2), 0, 0)),
            ("Kopf", "rot", (2 * math.sin(phi * 2 + 1), 26 * math.sin(phi), 0)),
            ("Oberarm.L", "rot", (-6, 0, 8)), ("Unterarm.L", "rot", (-22 - 3 * math.sin(phi * 2), 0, 0)),
            ("Oberarm.R", "rot", (-4, 0, 0)), ("Unterarm.R", "rot", (-12 - 3 * math.sin(phi * 2), 0, 0)),
            ("Oberschenkel.L", "rot", (-2, 0, 0)), ("Oberschenkel.R", "rot", (2, 0, 0)),
            ("Becken", "pos", (0, -0.005 * (1 - math.cos(phi * 2)), 0)), ("Becken", "rot", (0, 3 * math.sin(phi), 0)),
            ("Hut", "rot", (2 * math.sin(phi * 2), 0, 0)),
        ]
    animation(armatur, "Idle", 180, _schleife(180, 6, idle))

    # Laufen: gemächlicher Schritt, die Rolle bleibt vor dem Bauch
    def laufen(phi):
        werte = [w for w in _gehen(phi, 24, 36, 16, 0.025, -2, 12) if w[0] not in ("Oberarm.L", "Unterarm.L")]
        return werte + [("Oberarm.L", "rot", (-6 + 3 * math.sin(phi), 0, 8)), ("Unterarm.L", "rot", (-24, 0, 0))]
    animation(armatur, "Laufen", 32, _schleife(32, 2, laufen))

    # Grüßen: leichte Verbeugung, dann mit der Rechten einladend zum Laden weisen
    stehen = {"Oberarm.L": (-6, 0, 8), "Unterarm.L": (-22, 0, 0), "Oberarm.R": (-4, 0, 0), "Unterarm.R": (-12, 0, 0), "Brust": (-2, 0, 0), "Kopf": (0, 0, 0)}
    verbeugt = _mit(stehen, Brust=(16, 0, 0), Bauch=(8, 0, 0), Kopf=(8, 0, 0), Oberarm_R=(-30, 0, 0), Unterarm_R=(-70, 0, 0), Hut=(0, 0, 0))
    einladen = _mit(stehen, Brust=(0, -8, 0), Kopf=(-4, -10, 0), Oberarm_R=(-62, 0, -38), Unterarm_R=(-22, 0, 0), Hand_R=(0, -30, 0))
    _clip(armatur, "Gruessen", 72, [(0, stehen), (14, verbeugt), (24, verbeugt), (38, einladen), (54, _mit(einladen, Oberarm_R=(-58, 0, -44))), (72, stehen)])
