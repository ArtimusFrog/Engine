"""Bogenschützin – dritte Spielfigur (Baukasten: figuren.py), im Stil einer Mondlicht-Elfe.

Schlank und hochgewachsen (etwa 1,76 m): silberweißes Haar mit hohem, weit schwingendem
Pferdeschwanz und Kristallschmuck, lange Elfenohren, große violette Augen. Blauer Schulterumhang
mit hohem Stehkragen und weißem Saum, dahinter zwei lange, hellblaue Umhangbahnen. Dunkelblaues,
kurzes Mieder mit weißen Brustschalen und einer leuchtenden Mondsichel, freier Bauch, gekreuzte
Ledergürtel mit weißer Schnalle, weiße Federplatten an den Hüften, dunkelblaue Hose, hohe braune
Stiefel mit weißen Knie- und Spitzenpanzern, weiße Armschienen, braune Handschuhe, ein weißer
Schulterpanzer rechts. In der linken Faust der Mondbogen (oder ein erbeuteter Bogen aus waffen.py),
in der rechten Spitzhacke oder Axt.

Gleiche Knochennamen wie der Magier, eigene Maße. Der Pferdeschwanz hängt am Knochen „Hut“ und
schwingt so in allen Animationen nach.
Koordinaten wie in der Werkstatt: Z oben, die Figur schaut nach -Y, Füße im Ursprung, links (L) +X.
"""

import math

import bmesh
from mathutils import Quaternion, Vector

from figuren import HAENGT, HOCH, Figur, _achsen, _axt, _clip, _gehen, _glocke, _mischen, _mit, _platte, _schale, _schlauch, _schleife, _spitzhacke, farbe, weich
from werkstatt import animation

X, Y, Z = Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))

# Gelenke (Meter). L = +X, R = -X, vorne = -Y.
SCHULTER = Vector((0.165, 0.0, 1.45))
ELLBOGEN = Vector((0.235, 0.02, 1.19))
HANDGELENK = Vector((0.28, -0.01, 0.955))
FINGER = Vector((0.3, -0.03, 0.87))
HUEFTE = Vector((0.092, 0.0, 0.93))
KNIE = Vector((0.09, -0.005, 0.5))
KNOECHEL = Vector((0.088, 0.02, 0.085))
ZEHEN = Vector((0.088, -0.13, 0.025))
KOPF = Vector((0.0, -0.005, 1.66))
AUGE = Vector((0.033, -0.083, 1.672))
# Griffe: links der Bogen (quer, entlang Y), rechts ein senkrechter Stiel (Werkzeuge)
BOGEN_GRIFF = Vector((0.3, -0.035, 0.915))
GRIFF_R = Vector((-0.305, -0.07, 0.915))


def _spiegel(p, seite):
    return Vector((p.x * seite, p.y, p.z))


def _rumpf(co):
    z = co.z
    return _mischen(("Becken", 1 - weich(1.0, 1.12, z)), ("Bauch", weich(1.0, 1.12, z) * (1 - weich(1.24, 1.34, z))),
                    ("Brust", weich(1.24, 1.34, z) * (1 - weich(1.5, 1.56, z))), ("Hals", weich(1.5, 1.56, z)))


def _huefte_bein(co):
    """Unten am Becken gehen die Hüften weich in die Oberschenkel über."""
    if co.z > 0.95:
        return _rumpf(co)
    bein = weich(0.95, 0.8, co.z)
    links = weich(-0.04, 0.04, co.x)
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


def _bein(seite):
    sn = "L" if seite > 0 else "R"
    return lambda co: _mischen((f"Unterschenkel.{sn}", 1 - weich(0.45, 0.56, co.z)), (f"Oberschenkel.{sn}", weich(0.45, 0.56, co.z)))


def _faust(griff, achse, seite, dicke):
    """Faust aus Metaball-Formen, die sich um einen Stab entlang `achse` schließt."""
    achse = achse.normalized()
    q1 = achse.cross(Z if abs(achse.z) < 0.9 else X).normalized()
    q2 = achse.cross(q1).normalized()
    formen = [(griff + q1 * 0.024 * seite + q2 * 0.008, (0.026, 0.026, 0.026))]
    for i in range(4):
        versatz = achse * (0.019 - 0.0125 * i)
        punkte = []
        for w in (0.3, -0.6, -1.5, -2.4, -3.1):
            punkte.append(griff + versatz + (q1 * math.cos(w) * seite + q2 * math.sin(w)) * 0.026)
        for a, b in zip(punkte, punkte[1:]):
            for t in (0.0, 0.33, 0.66):
                formen.append((a.lerp(b, t), (dicke,) * 3))
        dicke *= 0.96
    for t in (0.0, 0.5, 1.0):
        formen.append((griff + achse * 0.03 + q1 * (0.018 - 0.03 * t) * seite - q2 * 0.022, (dicke * 1.05,) * 3))
    return formen


def bogenschuetze(seed=31, name="Bogenschuetze"):
    f = Figur(name, seed)
    r = f.rng
    haut, haut_schatten = farbe("#F6D6C4"), farbe("#E8B8A4")
    lippe, wange = farbe("#D98A94"), farbe("#F0A8A8")
    navy, navy_dunkel, navy_hell = farbe("#1E2A5A"), farbe("#141C40"), farbe("#34479A")
    blau, blau_hell = farbe("#4E6FD8"), farbe("#8AB0F0")
    eisblau, kristall = farbe("#A8D8FF"), farbe("#7FE0FF")
    weiss, weiss_schatten = farbe("#F2F4FA"), farbe("#C8D0E4")
    silber = farbe("#E6E8F2")
    haar, haar_schatten, haar_glanz = farbe("#ECEAF4"), farbe("#BFC0D8"), farbe("#FFFFFF")
    leder, leder_dunkel = farbe("#7A4A2E"), farbe("#4E2E1C")
    iris = farbe("#8A6AE0")
    kopf_gewicht = lambda co: {"Kopf": 1.0}

    # ================= Körper: Beine, Hüften, Taille, Brustkorb, Hals =================
    def rumpf_form(w):
        # Vorne etwas flacher, Rücken gerundet
        return 1.0 - 0.06 * max(0.0, -math.sin(w))

    rumpf = [
        (Vector((0, 0.005, 0.84)), X, Y, 0.1, 0.085),
        (Vector((0, 0.01, 0.9)), X, Y, 0.158, 0.11),
        (Vector((0, 0.012, 0.97)), X, Y, 0.168, 0.115),
        (Vector((0, 0.01, 1.04)), X, Y, 0.145, 0.1),
        (Vector((0, 0.005, 1.12)), X, Y, 0.112, 0.082),
        (Vector((0, 0.0, 1.2)), X, Y, 0.118, 0.085),
        (Vector((0, -0.005, 1.28)), X, Y, 0.135, 0.095),
        (Vector((0, 0.0, 1.37)), X, Y, 0.145, 0.098),
        (Vector((0, 0.005, 1.44)), X, Y, 0.15, 0.09),
        (Vector((0, 0.01, 1.5)), X, Y, 0.075, 0.06),
        (Vector((0, 0.012, 1.56)), X, Y, 0.042, 0.042),
        (Vector((0, 0.01, 1.6)), X, Y, 0.04, 0.042),
    ]

    def rumpf_farbe(i, k, p):
        z = p.center.z
        if z < 1.02:
            return navy * (0.92 if k % 7 == 0 else 1.0)                            # Hose
        if z < 1.2:
            return haut if z > 1.04 else navy_dunkel                               # freier Bauch
        if z < 1.46:
            return navy * (0.95 + 0.05 * math.sin(k * 0.8))                        # Mieder
        return haut                                                                # Hals
    f.loft("Koerper", [ring + (rumpf_form,) for ring in rumpf], 48, rumpf_farbe, _huefte_bein, teilung=3, glatt=True)
    # Nabel als kleine Vertiefung
    f.kugel("Nabel", (0, -0.084, 1.1), (0.006, 0.003, 0.008), haut_schatten, _rumpf, 8, 4)

    # Beine: Hose bis unters Knie, darüber die hohen Stiefel
    for seite in (1, -1):
        sn = "L" if seite > 0 else "R"
        h_, k_, a_ = _spiegel(HUEFTE, seite), _spiegel(KNIE, seite), _spiegel(KNOECHEL, seite)
        punkte = [h_ + Vector((0.01 * seite, 0.005, 0.02)), h_.lerp(k_, 0.35), h_.lerp(k_, 0.75), k_, k_.lerp(a_, 0.4), a_ + Vector((0, 0, 0.05))]
        radien = [0.085, 0.075, 0.058, 0.05, 0.047, 0.036]
        _schlauch(f, "Bein", punkte, radien, 20, lambda i, k, p: navy_hell if p.normal.x * seite > 0.85 else navy * (0.94 + 0.08 * max(0.0, -p.normal.y)),
                  _bein(seite), zu=False, teilung=3)
        # Hoher Stiefel: Schaft bis übers Knie, weißer Kniepanzer, Absatz, weiße Spitze
        schaft = [a_ + Vector((0, 0.005, -0.02)), a_ + Vector((0, 0.0, 0.1)), k_.lerp(a_, 0.45), k_ + Vector((0, 0, -0.02)), k_ + Vector((0, 0.0, 0.045))]
        _schlauch(f, "Stiefelschaft", schaft, [0.042, 0.043, 0.052, 0.058, 0.063], 20,
                  lambda i, k, p: leder * (0.88 if k % 5 == 0 else 1.0) * (1.15 if i > 3.4 else 1.0), _bein(seite), zu=False, teilung=2)
        _platte(f, "Kniepanzer", k_ + Vector((0, -0.055, 0.02)), 0.17, 0.045, 0.02, Z, -Y,
                lambda i, k, p: weiss if p.normal.dot(-Y) > 0.3 else weiss_schatten, _bein(seite), spitz=0.95, wolbung=1.2)
        _platte(f, "Kniezacke", k_ + Vector((0.035 * seite, -0.04, -0.01)), 0.1, 0.02, 0.012, Vector((0.3 * seite, 0, 1)), Vector((seite, -1, 0)),
                lambda i, k, p: weiss_schatten, _bein(seite), spitz=0.95)
        fuss = [(Vector((0.088 * seite, 0.05, 0.035)), X, Z, 0.036, 0.035), (Vector((0.088 * seite, 0.03, 0.012)), X, Z, 0.04, 0.02),
                (Vector((0.088 * seite, -0.03, 0.04)), X, Z, 0.042, 0.04), (Vector((0.088 * seite, -0.09, 0.035)), X, Z, 0.036, 0.03),
                (Vector((0.088 * seite, -0.14, 0.03)), X, Z, 0.022, 0.02), (Vector((0.088 * seite, -0.165, 0.028)), X, Z, 0.004, 0.004)]

        def stiefel_farbe(i, k, p):
            if p.center.y < -0.075:
                return weiss if p.normal.z > -0.3 else weiss_schatten                 # weiße Spitze
            if p.center.z < 0.012:
                return leder_dunkel * 0.6
            return leder
        f.loft("Stiefel", fuss, 16, stiefel_farbe, lambda co, sn=sn: {f"Fuss.{sn}": 1.0}, oben_zu=True, unten_zu=True, teilung=3, glatt=True)
        f.kiste("Absatz", (0.088 * seite, 0.045, 0.02), (0.03, 0.03, 0.04), leder_dunkel, lambda co, sn=sn: {f"Fuss.{sn}": 1.0})

    # ================= Mieder: Brustschalen, Mondsichel, Rüschen =================
    for s in (1, -1):
        mitte = Vector((0.052 * s, -0.082, 1.31))
        f.kugel("Brustschale", mitte, (0.055, 0.04, 0.05), lambda poly: blau * (0.85 + 0.25 * max(0.0, -poly.normal.y)), _rumpf, 16, 10)
        # weiße Panzerfassung um die Schale, spitz zur Mitte
        _platte(f, "Schalenrand", mitte + Vector((0.008 * s, -0.03, 0.035)), 0.11, 0.018, 0.01, Vector((-s * 0.9, 0, -0.35)), Vector((0, -1, 0.5)),
                lambda i, k, p: weiss, _rumpf, spitz=0.9, wolbung=0.6)
    f.kiste("Miedersteg", (0, -0.106, 1.34), (0.018, 0.012, 0.09), navy_dunkel, _rumpf)
    # Mondsichel: leuchtender Halbmond auf der Brust
    bm = bmesh.new()
    rand = []
    for k in range(13):
        w = math.radians(40 + k * 280 / 12)
        aussen = Vector((math.cos(w) * 0.026, -0.118, 1.39 + math.sin(w) * 0.026))
        innen = Vector((math.cos(w) * 0.017 + 0.009, -0.119, 1.39 + math.sin(w) * 0.019))
        rand.append((bm.verts.new(aussen), bm.verts.new(innen)))
    for (a, b), (c, d) in zip(rand, rand[1:]):
        bm.faces.new((a, c, d, b))
    mond = f._objekt(bm, "Mond", lambda poly: kristall * 1.3, _rumpf)
    for poly in mond.data.polygons:
        if poly.normal.y > 0:
            poly.flip()
    f.kugel("Mondfassung", (0, -0.11, 1.39), (0.032, 0.008, 0.032), navy_dunkel, _rumpf, 16, 6, glatt=False)
    # Saum des Mieders: gezackter dunkler Rand
    for k in range(10):
        w = math.pi * (0.2 + 0.6 * k / 9)
        p = Vector((math.cos(w) * 0.12, -math.sin(w) * 0.09, 1.215))
        _platte(f, "Zacke", p, 0.05, 0.016, 0.006, -Z, Vector((math.cos(w), -math.sin(w), 0)), lambda i, k2, p2: navy_dunkel, _rumpf, spitz=0.95)

    # ================= Gürtel: gekreuzte Lederriemen, weiße Schnalle, Federplatten =================
    for neig, dz in ((8, 0.0), (-10, 0.03)):
        q = Quaternion(Y, math.radians(neig))
        ringe = [(Vector((0, 0.005, 0.965 + dz)) + q @ Vector((0, 0, -0.018)), q @ X, Y, 0.17, 0.118),
                 (Vector((0, 0.005, 0.965 + dz)) + q @ Vector((0, 0, 0.018)), q @ X, Y, 0.172, 0.12)]
        f.loft("Riemen", ringe, 48, lambda i, k, p: leder * (0.85 if k % 6 == 0 else 1.0), _rumpf, teilung=1)
    _platte(f, "Schnalle", Vector((0, -0.125, 0.98)), 0.06, 0.028, 0.012, X, -Y, lambda i, k, p: weiss, _rumpf, spitz=0.3, wolbung=0.5)
    f.kugel("Schnallenstein", (0, -0.134, 0.98), (0.012, 0.005, 0.012), kristall, _rumpf, 10, 6, glatt=False)
    for s in (1, -1):
        for j, (dz, laenge) in enumerate(((0.0, 0.22), (-0.03, 0.18), (-0.06, 0.14))):
            mitte = Vector((0.16 * s, -0.02 + 0.012 * j, 0.86 + dz))
            _platte(f, "Hueftfeder", mitte, laenge, 0.035 - 0.005 * j, 0.01, Vector((0.25 * s, 0.1, -1)), Vector((s, -0.3, 0)),
                    lambda i, k, p, j=j: (weiss if j != 1 else weiss_schatten) * (0.95 + 0.05 * (k % 2)), _huefte_bein, spitz=0.95, wolbung=0.8)

    # ================= Arme: Haut, kurze Ärmel, Armschienen, Handschuhe =================
    for seite in (1, -1):
        s_, e_, h_ = _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite)
        punkte = [s_ + Vector((-0.02 * seite, 0, 0.02)), s_.lerp(e_, 0.5), e_, e_.lerp(h_, 0.5), h_]
        _schlauch(f, "Arm", punkte, [0.05, 0.042, 0.036, 0.034, 0.028], 16, lambda i, k, p: haut, _arm(seite), zu=False, teilung=3)
        # kurzer, dunkler Ärmel
        _schlauch(f, "Aermel", [s_ + Vector((-0.02 * seite, 0, 0.03)), s_.lerp(e_, 0.18), s_.lerp(e_, 0.36)], [0.058, 0.05, 0.047], 16,
                  lambda i, k, p: navy_dunkel * (1.15 if i > 1.6 else 1.0), _arm(seite), zu=False, teilung=2)
        # Armschiene: weiß, zur Hand hin weiter, mit spitzem Flügel
        schiene = [e_.lerp(h_, 0.2), e_.lerp(h_, 0.55), e_.lerp(h_, 0.92)]
        _schlauch(f, "Armschiene", schiene, [0.04, 0.042, 0.038], 16, lambda i, k, p: weiss if k % 8 else weiss_schatten, _arm(seite), zu=False, teilung=2)
        q1, q2 = _achsen(e_, h_)
        _platte(f, "Schienenfluegel", e_.lerp(h_, 0.45) + (Vector((seite, 0, 0)) - Vector((seite, 0, 0)).project(h_ - e_)).normalized() * 0.04,
                0.16, 0.022, 0.01, (e_ - h_), Vector((seite, 0, 0)), lambda i, k, p: weiss, _arm(seite), spitz=0.95, wolbung=0.6)
        f.loft("Schienenband", [(e_.lerp(h_, 0.62), q1, q2, 0.044, 0.044), (e_.lerp(h_, 0.68), q1, q2, 0.044, 0.044)], 16,
               lambda i, k, p: blau, _arm(seite), oben_zu=True, unten_zu=True)
    # Weißer Schulterpanzer rechts, geschichtet
    sr = _spiegel(SCHULTER, -1)
    schulter_gewicht = lambda co: _mischen(("Oberarm.R", 0.65), ("Brust", 0.35))
    f.kugel("Schulterkappe", sr + Vector((-0.02, 0, 0.035)), (0.07, 0.068, 0.045), lambda poly: weiss if poly.normal.z > 0.3 else weiss_schatten,
            schulter_gewicht, 20, 10, glatt=True)
    for j in range(3):
        mitte = sr + Vector((-0.06 - 0.012 * j, 0.0, 0.0 - 0.035 * j))
        _platte(f, "Schulterplatte", mitte, 0.16 - 0.025 * j, 0.06 - 0.008 * j, 0.012, Vector((0, 1, -0.25)), Vector((-1, 0, 0.35)),
                lambda i, k, p, j=j: (weiss if j % 2 == 0 else weiss_schatten) * (0.96 + 0.04 * (k % 2)), schulter_gewicht, spitz=0.6, wolbung=1.4)
    f.stern("Schulterstein", sr + Vector((-0.03, -0.05, 0.07)), Vector((-0.3, -1, 0.6)), 0.014, kristall, lambda co: _mischen(("Oberarm.R", 0.6), ("Brust", 0.4)), zacken=4)

    # Silberne Armreifen am Oberarm, eingelegt mit einem Kristall
    for seite in (1, -1):
        s_, e_ = _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite)
        q1, q2 = _achsen(s_, e_)
        for t in (0.5, 0.56):
            f.loft("Armreif", [(s_.lerp(e_, t), q1, q2, 0.046, 0.046), (s_.lerp(e_, t + 0.03), q1, q2, 0.046, 0.046)], 16,
                   lambda i, k, p: silber if k % 4 else weiss_schatten, _arm(seite), oben_zu=True, unten_zu=True, glatt=True)
        f.kugel("Armreifstein", s_.lerp(e_, 0.545) + Vector((0.044 * seite, -0.01, 0)), (0.006, 0.009, 0.009), kristall, _arm(seite), 8, 6)
    koecher_unten, koecher_oben = Vector((-0.2, 0.06, 0.64)), Vector((-0.23, 0.02, 0.98))
    q1, q2 = _achsen(koecher_unten, koecher_oben)
    f.loft("Koecher", [(koecher_unten.lerp(koecher_oben, t), q1, q2, rad, rad * 0.8) for t, rad in ((0.0, 0.032), (0.1, 0.045), (0.85, 0.05), (1.0, 0.054))],
           16, lambda i, k, p: weiss if i < 0.3 or i > 2.6 else (blau if k % 8 in (0, 1) else navy), _huefte_bein, unten_zu=True, teilung=2, glatt=True)
    richtung = (koecher_oben - koecher_unten).normalized()
    for n in range(5):
        w = math.tau * n / 5
        basis = koecher_oben + (q1 * math.cos(w) + q2 * math.sin(w)) * 0.025
        spitze = basis + richtung * (0.1 + 0.02 * (n % 2))
        f.loft("Pfeilschaft", [(basis - richtung * 0.05, q1, q2, 0.005, 0.005), (spitze, q1, q2, 0.005, 0.005)], 5, lambda i, k, p: silber,
               _huefte_bein, oben_zu=True)
        _platte(f, "Pfeilfeder", spitze - richtung * 0.03, 0.07, 0.014, 0.004, richtung, q1 * math.cos(w) + q2 * math.sin(w),
                lambda i, k, p: kristall if n % 2 else blau_hell, _huefte_bein, spitz=0.6)
    f.loft("Koechergurt", [(Vector((-0.12, -0.08, 1.0)), X, Y, 0.012, 0.006), (Vector((-0.2, -0.02, 0.9)), X, Y, 0.012, 0.006)], 6,
           lambda i, k, p: leder, _huefte_bein, oben_zu=True, unten_zu=True)

    # Hände: links Faust um den Bogengriff, rechts um einen senkrechten Stiel; braune, fingerlose Handschuhe
    handschuh = leder
    f.metaball("HandL", _faust(BOGEN_GRIFF, Y, 1, 0.0082), 0.0028, 2000,
               lambda poly: handschuh if poly.center.z > BOGEN_GRIFF.z + 0.012 else haut, lambda co: {"Hand.L": 1.0}, glatt=True)
    f.metaball("HandR", _faust(GRIFF_R, Z, -1, 0.0082), 0.0028, 2000,
               lambda poly: handschuh if poly.center.z > GRIFF_R.z + 0.022 else haut, lambda co: {"Hand.R": 1.0}, glatt=True)

    # ================= Kopf: schmales Gesicht, große Augen, Elfenohren =================
    kopf = [
        (KOPF + Vector((0, 0.01, 0.02)), (0.078, 0.092, 0.098)),            # Schädel
        (KOPF + Vector((0, -0.028, -0.04)), (0.064, 0.064, 0.056)),         # Wangen
        (KOPF + Vector((0, -0.07, -0.085)), (0.024, 0.024, 0.02)),          # spitzes Kinn
        (KOPF + Vector((0, -0.05, -0.07)), (0.045, 0.04, 0.03)),            # Kiefer
        (KOPF + Vector((0, -0.08, 0.02)), (0.056, 0.02, 0.016)),            # Brauenbogen
        (KOPF + Vector((0, -0.088, -0.012)), (0.008, 0.014, 0.02)),         # kleine Nase
        (KOPF + Vector((0, -0.096, -0.026)), (0.009, 0.01, 0.008)),
        (KOPF + Vector((0, -0.085, -0.058)), (0.016, 0.01, 0.007)),         # volle Unterlippe
        (KOPF + Vector((0, -0.087, -0.045)), (0.015, 0.008, 0.005)),        # Oberlippe
        (KOPF + Vector((0, -0.086, -0.05)), (0.016, 0.009, 0.0026), True),  # Mundspalte
        (KOPF + Vector((0, 0.005, -0.1)), (0.04, 0.04, 0.05)),              # Hals
    ]
    for s in (1, -1):
        kopf.append((_spiegel(AUGE, s) + Vector((0, -0.004, 0)), (0.021, 0.013, 0.015), True))    # große Augenhöhle
    lippe_mitte = KOPF + Vector((0, -0.084, -0.056))

    def gesicht(poly):
        p = poly.center
        if (p - lippe_mitte).length < 0.016 and p.y < -0.075:
            return lippe
        rosig = math.exp(-(((abs(p.x) - 0.042) ** 2 + (p.z - KOPF.z + 0.035) ** 2) / 0.014 ** 2)) if p.y < -0.04 else 0.0
        return haut.lerp(wange, rosig * 0.55)
    f.metaball("Kopf", kopf, 0.004, 5200, gesicht,
               lambda co: _mischen(("Kopf", weich(1.52, 1.58, co.z)), ("Hals", 1 - weich(1.52, 1.58, co.z))), glatt=True)
    for s in (1, -1):
        auge = _spiegel(AUGE, s)
        f.kugel("Augapfel", auge, (0.0165, 0.012, 0.015), farbe("#F4F0FA"), kopf_gewicht, 16, 12)
        f.kugel("Iris", auge + Vector((-0.001 * s, -0.0105, 0.0005)), (0.0105, 0.003, 0.0115), iris, kopf_gewicht, 16, 8)
        f.kugel("Pupille", auge + Vector((-0.001 * s, -0.0125, 0.0005)), (0.0045, 0.0016, 0.0055), farbe("#1A1030"), kopf_gewicht, 10, 6)
        f.kugel("Glanz", auge + Vector((-0.004 * s, -0.0132, 0.0045)), (0.002, 0.0008, 0.002), farbe("#FFFFFF"), kopf_gewicht, 8, 4)
        # Wimpern: dunkler Bogen über dem Auge, außen hochgezogen
        punkte = [auge + Vector((0.014 * s, -0.006, 0.007)), auge + Vector((0.0, -0.011, 0.012)), auge + Vector((-0.016 * s, -0.004, 0.009)),
                  auge + Vector((-0.024 * s, 0.002, 0.016))]
        f.straehne("Wimpern", list(reversed(punkte)) if s > 0 else punkte, 0.0035, 0.001, farbe("#2A2040"), kopf_gewicht, 6, 0.0, 0.5)
        # feine Braue
        brauen = [auge + Vector((0.012 * s, -0.008, 0.027)), auge + Vector((-0.004 * s, -0.012, 0.032)), auge + Vector((-0.022 * s, -0.006, 0.03))]
        f.straehne("Braue", brauen, 0.0028, 0.001, haar_schatten * 0.8, kopf_gewicht, 5, 0.0, 0.5)
        # Lange, spitze Elfenohren nach hinten oben
        ohr = [KOPF + Vector((0.074 * s, 0.005, -0.012)), KOPF + Vector((0.09 * s, 0.022, 0.012)), KOPF + Vector((0.108 * s, 0.04, 0.04)),
               KOPF + Vector((0.125 * s, 0.058, 0.075))]
        f.straehne("Ohr", ohr, 0.02, 0.002, haut, kopf_gewicht, 10, 0.0, 0.45)
        f.kugel("Ohrring", KOPF + Vector((0.078 * s, 0.0, -0.03)), (0.006, 0.006, 0.006), kristall, kopf_gewicht, 8, 4)
    f.kugel("Stirnstein", KOPF + Vector((0, -0.088, 0.045)), (0.007, 0.004, 0.01), kristall, kopf_gewicht, 8, 6, glatt=False)

    # ================= Haar: Kappe, Seitensträhnen, Pony, hoher Pferdeschwanz =================
    f.metaball("Haar", [(KOPF + Vector((0, 0.018, 0.035)), (0.086, 0.098, 0.096)), (KOPF + Vector((0, 0.05, 0.0)), (0.08, 0.07, 0.08))], 0.008, 1200,
               lambda poly: haar if poly.normal.z > 0.2 else haar_schatten, kopf_gewicht)
    # Glatt nach hinten gestrichenes Haar: eine Schale mit sanften Rillen zum Pferdeschwanz hin
    def haar_form(th, ph):
        return 1.0 + 0.025 * math.sin(th * 14) * math.cos(ph) + 0.06 * max(0.0, -math.cos(th)) * max(0.0, math.sin(ph))

    def haar_farbe(poly):
        p, n = poly.center, poly.normal
        glanz = abs(math.sin(math.atan2(p.x, -p.y + 0.02) * 7)) < 0.25 and n.z > 0.3
        return haar_glanz if glanz else (haar if n.z > -0.1 else haar_schatten)
    _schale(f, "Haarschale", KOPF + Vector((0, 0.015, 0.03)), 0.088, 0.1, 0.1, (-150, 150), (-10, 90), haar_farbe, kopf_gewicht, seg=(40, 12), form=haar_form)
    _schale(f, "Hinterkopf", KOPF + Vector((0, 0.02, 0.025)), 0.086, 0.098, 0.1, (110, 250), (-60, 0), haar_farbe, kopf_gewicht, seg=(20, 8), form=haar_form)
    # Seitensträhnen, die das Gesicht rahmen; ein Pony fällt schräg übers rechte Auge
    for s in (1, -1):
        for j in range(3):
            start = KOPF + Vector((0.07 * s, -0.05 + 0.02 * j, 0.06))
            punkte = [start, start + Vector((0.012 * s, -0.012, -0.06)), start + Vector((0.006 * s, -0.01, -0.12 - 0.02 * j)),
                      start + Vector((-0.004 * s, -0.004, -0.17 - 0.03 * j))]
            f.straehne("Seitenstraehne", punkte, 0.016, 0.003, haar * r.uniform(0.92, 1.02), kopf_gewicht, 8, 0.3, 0.6, glatt=True)
    pony = [KOPF + Vector((0.02, -0.08, 0.1)), KOPF + Vector((-0.02, -0.1, 0.06)), KOPF + Vector((-0.05, -0.102, 0.02)), KOPF + Vector((-0.07, -0.085, -0.03))]
    f.straehne("Pony", pony, 0.024, 0.004, haar_glanz * 0.96, kopf_gewicht, 10, 0.2, 0.5, glatt=True)
    # Pferdeschwanz: vom Hinterkopf steil hoch, dann in großem Bogen nach hinten und weit hinab
    ansatz = KOPF + Vector((0, 0.07, 0.11))
    schwanz_gewicht = lambda co: _mischen(("Kopf", 1 - weich(1.8, 1.55, co.z) * weich(0.05, 0.2, co.y)), ("Hut", weich(1.8, 1.55, co.z) * weich(0.05, 0.2, co.y)))
    def schwanz_punkt(t, seitlich=0.0, tief=0.0):
        # hoch (bis ~1,98 m), in großem Bogen zurück (bis ~0,45 m hinter dem Kopf) und hinab bis ~0,9 m
        y = 0.07 + 0.38 * math.sin(t * math.pi * 0.62) ** 0.9 + 0.05 * t
        z = 1.77 + 0.21 * math.sin(t * math.pi * 1.05) - 0.9 * t ** 1.8 - tief * t
        x = seitlich * (0.015 + 0.06 * t)
        return Vector((x, y, z))

    linie = [schwanz_punkt(j / 16) for j in range(17)]
    ringe = []
    for j, p in enumerate(linie):
        t = j / 16
        richtung = (linie[min(j + 1, 16)] - linie[max(j - 1, 0)]).normalized()
        quer = X
        normale = richtung.cross(quer).normalized()
        # am Band schmal, dann voll und breit, zur Spitze fein auslaufend
        r_ = 0.034 + 0.05 * math.sin(math.pi * min(1.0, t * 1.6)) ** 0.7 * (1 - t) ** 0.35 + 0.004
        form = lambda w, t=t: 1.0 + 0.1 * math.sin(w * 9 + t * 4) + 0.05 * math.sin(w * 17)
        ringe.append((p, quer, normale, r_ * 1.15, r_ * 0.85, form))

    def schwanz_farbe(i, k, p):
        glanz = (k % 9) in (2, 3) and p.normal.z > 0.0
        return haar_glanz if glanz else (haar if p.normal.z > -0.2 else haar_schatten)
    f.loft("Pferdeschwanz", ringe, 28, schwanz_farbe, schwanz_gewicht, oben_zu=True, unten_zu=True, teilung=2, glatt=True)
    # Locken, die sich oben und an den Seiten aus der Masse lösen
    for n in range(7):
        seitlich = (n - 3) / 3.0
        punkte = [schwanz_punkt(0.15 + 0.1 * j, seitlich * 1.6, 0.02 * n) + Vector((0, 0, 0.03 * (1 - j / 8))) for j in range(9)]
        f.straehne("Locke", punkte, 0.02, 0.003, haar * (0.95 + 0.05 * (n % 2)), schwanz_gewicht, 8, 0.4, 0.55, glatt=True)
    f.loft("Haarband", [(ansatz + Vector((0, 0, -0.01)), X, Y, 0.036, 0.034), (ansatz + Vector((0, 0.005, 0.025)), X, Y, 0.034, 0.032)], 16,
           lambda i, k, p: navy_hell, kopf_gewicht, oben_zu=True, unten_zu=True)
    # Kristallschmuck am Pferdeschwanz: ein Fächer leuchtender Kristalle
    for j, (w, laenge) in enumerate(((-35, 0.1), (-12, 0.13), (12, 0.13), (35, 0.1), (0, 0.16))):
        a = math.radians(w)
        richtung = Vector((math.sin(a), -0.35, math.cos(a))).normalized()
        basis = ansatz + Vector((0, -0.03, 0.02))
        _platte(f, "Haarkristall", basis + richtung * laenge * 0.5, laenge, 0.018, 0.01, richtung, Vector((0, -1, 0.3)),
                lambda i, k, p, j=j: kristall if j % 2 == 0 else blau_hell, kopf_gewicht, spitz=0.95, wolbung=0.3)

    # ================= Schulterumhang mit Stehkragen, lange Umhangbahnen =================
    hals = Vector((0, 0.012, 1.5))

    def kragen_form(th, ph):
        # Nach oben weit aufstehend, vorne offen
        return 1.0 + 0.35 * max(0.0, math.sin(ph))

    kragen_gewicht = lambda co: _mischen(("Brust", 1 - weich(1.55, 1.64, co.z) * 0.4), ("Hals", weich(1.55, 1.64, co.z) * 0.4))
    _schale(f, "Stehkragen", hals + Vector((0, 0, 0.05)), 0.075, 0.07, 0.1, (38, 322), (-10, 70),
            lambda poly: blau_hell if (poly.center - hals).length > 0.1 else blau, kragen_gewicht, seg=(32, 6), form=kragen_form)
    _schale(f, "KragenInnen", hals + Vector((0, 0, 0.05)), 0.07, 0.065, 0.097, (38, 322), (-10, 70), lambda poly: navy_dunkel, kragen_gewicht, innen=True,
            seg=(24, 5), form=kragen_form)

    def capelet_form(th, ph):
        unten = max(0.0, -math.sin(ph))
        return 1.0 + 0.2 * unten + 0.05 * math.sin(th * 6) * unten

    def capelet_farbe(poly):
        p = poly.center
        if p.z < 1.305:
            return weiss                                                           # weißer Saum
        # vorne an der Öffnung eine weiße Zierkante, zum Saum hin heller, oben satt blau
        winkel = abs(math.degrees(math.atan2(p.x, -(p.y - 0.01))))
        if winkel < 36:
            return weiss
        return blau.lerp(blau_hell, weich(1.42, 1.32, p.z) * 0.6) * (0.92 + 0.12 * max(0.0, poly.normal.z))
    # Glocke vom Kragen über die Schultern, vorne offen (dort sitzt die Mondsichel)
    glocke = [(Vector((0, 0.012, 1.535)), 0.085, 0.075), (Vector((0, 0.01, 1.5)), 0.15, 0.11), (Vector((0, 0.008, 1.46)), 0.2, 0.135),
              (Vector((0, 0.008, 1.41)), 0.215, 0.145), (Vector((0, 0.01, 1.35)), 0.222, 0.152), (Vector((0, 0.012, 1.3)), 0.226, 0.156)]
    _glocke(f, "Schulterumhang", glocke, (28, 332), capelet_farbe, _rumpf, seg=48, welle=0.05)
    _glocke(f, "UmhangFutter", [(m, rx * 0.97, ry * 0.97) for m, rx, ry in glocke], (28, 332), lambda poly: navy_dunkel, _rumpf, innen=True, seg=36,
            welle=0.05)
    f.kugel("Kragenschloss", (0, -0.1, 1.49), (0.018, 0.008, 0.018), weiss, _rumpf, 12, 6, glatt=False)
    # Zwei lange Umhangbahnen hinten, hellblau, zur Spitze schmal
    umhang_gewicht = lambda co: _mischen(("Brust", weich(1.0, 1.35, co.z)), ("Becken", 1 - weich(1.0, 1.35, co.z)))
    for s in (1, -1):
        punkte = [Vector((0.08 * s, 0.13, 1.4)), Vector((0.12 * s, 0.17, 1.1)), Vector((0.14 * s, 0.2, 0.8)), Vector((0.15 * s, 0.22, 0.5)),
                  Vector((0.15 * s, 0.23, 0.36))]
        ringe = []
        for j, p in enumerate(punkte):
            b = 0.085 * (1 - 0.55 * (j / 4) ** 2)
            ringe.append((p, X, Y, b * 0.85, 0.006, lambda w: 1.0 + 0.25 * abs(math.cos(w)) * 0.0))

        def bahn_farbe(i, k, p):
            # oben kräftig blau, nach unten eisig hell
            return blau.lerp(eisblau, min(1.0, i / 4.0))
        f.loft("Umhangbahn", ringe, 16, bahn_farbe, umhang_gewicht, oben_zu=True, unten_zu=True, teilung=3, glatt=True)

    # ================= Mondbogen (linke Hand) und erbeutbare Bögen =================
    from waffen import BOEGEN, bogen as bogen_bauen
    bogen_gewicht = lambda co: {"Hand.L": 1.0}
    anfang = len(f.teile)
    bogen_bauen(f, "Bogen", BOGEN_GRIFF, bogen_gewicht)
    f.als_starr("Bogen", "Hand.L", anfang)
    for art in BOEGEN:
        anfang = len(f.teile)
        bogen_bauen(f, art, BOGEN_GRIFF, bogen_gewicht)
        f.als_starr(art, "Hand.L", anfang)

    # ================= Spitzhacke und Axt (rechte Hand) =================
    hand_gewicht = lambda co: {"Hand.R": 1.0}
    anfang = len(f.teile)
    _spitzhacke(f, GRIFF_R.x, GRIFF_R.y, hand_gewicht)
    f.als_starr("Spitzhacke", "Hand.R", anfang)
    anfang = len(f.teile)
    _axt(f, GRIFF_R.x, GRIFF_R.y, hand_gewicht)
    f.als_starr("Axt", "Hand.R", anfang)

    # ================= Skelett (Namen wie beim Magier; „Hut“ trägt den Pferdeschwanz) =================
    f.knochen_dazu("Becken", (0, 0, 0.93), (0, 0, 1.06), None, HOCH)
    f.knochen_dazu("Bauch", (0, 0, 1.06), (0, 0, 1.26), "Becken", HOCH)
    f.knochen_dazu("Brust", (0, 0, 1.26), (0, 0, 1.5), "Bauch", HOCH)
    f.knochen_dazu("Hals", (0, 0, 1.5), (0, 0, 1.58), "Brust", HOCH)
    f.knochen_dazu("Kopf", (0, 0, 1.58), (0, 0, 1.8), "Hals", HOCH)
    f.knochen_dazu("Hut", (0, 0.1, 1.82), (0, 0.3, 1.7), "Kopf", HOCH)
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
