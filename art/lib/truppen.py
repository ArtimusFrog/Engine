"""Truppen und Bosse der Schattenfestung, der Soldat der Kaserne und die Waldspinne – modelliert
mit dem Bildhauer (bildhauer.py) wie die Streuner (streuner.py).

Menschliche Truppen teilen einen Grundkörper (\`mensch\`): Rumpf, Hals, Kopf mit Gesicht, Arme mit
Fäusten und Beine in einem Stück, Maße und Skelett wie beim Magier (Animationen aus gegner.py).
Rüstung sitzt eng an der Haut (\`bildhauer.huelle\`), Röcke und Umhänge werden am Körper
ausgemessen. Die großen Bosse werden in Menschengröße gebaut und dann vergrößert.
Koordinaten: Z oben, die Figur schaut nach -Y, Füße im Ursprung, links (L) ist +X.
"""

import math

import bmesh
from mathutils import Matrix, Vector, noise

import bildhauer as bh
from figuren import X, Y, Z, Figur, _achsen, _glocke, _mischen, _platte, _schale, farbe, weich
from gegner import _angriff_hieb, _angriff_schuss, _angriff_sense, _angriff_stampfen, _angriff_stoss, _angriff_zauber, _animationen
from streuner import KOPF, Bau, _falten, _kegel, _strecke, _veredeln, _zacken

GLUT = farbe("#D77BFF")
GIFT = farbe("#8CFF9E")
EIS = farbe("#8FD8FF")
FEUER = farbe("#FF7A1A")
FEUER_HELL = farbe("#FFD24A")
GOLD = farbe("#D8AE4A")
VIOLETT = farbe("#4A2466")
VIOLETT_DUNKEL = farbe("#2A1538")
VIOLETT_HELL = farbe("#8E4FC4")
STAHL = farbe("#8A909C")
STAHL_DUNKEL = farbe("#3E424C")
STAHL_SCHWARZ = farbe("#24262C")
LEDER = farbe("#5A3A24")
KNOCHEN = farbe("#DCD4C0")
HOLZ = farbe("#6A4A2C")

# Gelenke eines Menschen (wie der Magier, damit die Animationen passen)
MENSCH = dict(becken=0.95, bauch=1.1, brust=1.3, hals=1.56, kopf=1.65, scheitel=1.9, kopf_y=0.0,
              schulter=(0.2, 0.0, 1.5), ellbogen=(0.29, 0.02, 1.22), hand=(0.34, -0.01, 0.97), finger=(0.36, -0.03, 0.86),
              huefte=(0.1, 0.0, 0.95), knie=(0.1, 0.0, 0.52), knoechel=(0.1, 0.02, 0.1), zehen=(0.1, -0.13, 0.03))
AUGE = Vector((0.038, -0.094, 1.752))


# ---------------------------------------------------------------------------
# Grundkörper
# ---------------------------------------------------------------------------
def mensch(f, breite=1.0, dick=1.0, muskel=0.15, kopf=True, fuesse="stiefel", **aenderungen):
    """Bau (Skelett schon angelegt) und die Formen eines ganzen Menschen."""
    g = dict(MENSCH)
    g.update(aenderungen)
    b = Bau(f, **g)
    b.skelett()
    w, d = breite, dick
    formen = [
        ((0, 0.0, 0.97), (0.16 * w, 0.12 * d, 0.09)),
        ((0, 0.0, 1.1), (0.15 * w, 0.11 * d, 0.1)),
        ((0, -0.01, 1.22), (0.16 * w, 0.12 * d, 0.1)),
        ((0, -0.01, 1.36), (0.185 * w, 0.13 * d, 0.12)),
        ((0, 0.0, 1.46), (0.2 * w, 0.12 * d, 0.08)),
        ((0, 0.01, 1.5), (0.22 * w, 0.1 * d, 0.05)),
    ]
    for s in (1, -1):
        formen += [((0.08 * s * w, -0.09 * d, 1.4), (0.08 * w, 0.05 * d, 0.06)),          # Brust
                   ((0.1 * s * w, 0.06 * d, 1.42), (0.07 * w, 0.06 * d, 0.1))]            # Schulterblätter
        formen += b.arm_formen(s, (0.062 * w, 0.052 * w, 0.042, 0.046 * w, 0.035), muskel=muskel)
        formen += b.faust_formen(s, 1.0)
        formen += b.bein_formen(s, (0.088 * w, 0.072 * w, 0.056, 0.06 * w, 0.042), muskel=muskel * 0.8)
        x = b.p("knoechel", s).x
        if fuesse == "stiefel":
            formen += [((x, 0.01, 0.06), (0.05, 0.07, 0.06)), ((x, -0.07, 0.045), (0.048, 0.07, 0.042)), ((x, -0.14, 0.04), (0.04, 0.045, 0.035))]
    formen += bh.glied((0, 0.0, 1.5), (0, 0.0, 1.66), 0.056, 0.052)
    if kopf:
        formen += gesicht_formen()
    return b, formen


def gesicht_formen(knollnase=False):
    """Kopf mit ausgeformtem Gesicht (Schädel, Kiefer, Wangen, Brauen, Nase, Lippen, Ohren)."""
    formen = [
        ((0, 0.005, 1.75), (0.093, 0.105, 0.112)),
        ((0, -0.02, 1.685), (0.078, 0.075, 0.07)),
        ((0.05, -0.07, 1.72), (0.03, 0.025, 0.022)),
        ((-0.05, -0.07, 1.72), (0.03, 0.025, 0.022)),
        ((0, -0.082, 1.776), (0.068, 0.028, 0.02)),
        ((0, -0.102, 1.742), (0.014, 0.022, 0.03)),
        ((0, -0.115, 1.722), (0.017, 0.024, 0.021)),
        ((0, -0.132, 1.706), (0.021 if knollnase else 0.016, 0.021, 0.019)),
        ((0.019, -0.117, 1.701), (0.012, 0.013, 0.011)),
        ((-0.019, -0.117, 1.701), (0.012, 0.013, 0.011)),
        ((0, -0.075, 1.645), (0.035, 0.03, 0.03)),
        ((0, -0.098, 1.672), (0.02, 0.012, 0.007)),
        ((0, -0.106, 1.679), (0.022, 0.01, 0.0035), True),
    ]
    for s in (1, -1):
        auge = Vector((AUGE.x * s, AUGE.y, AUGE.z))
        formen += [(auge + Vector((0, -0.006, 0)), (0.02, 0.014, 0.014), True),
                   (auge + Vector((0, -0.007, 0.01)), (0.018, 0.01, 0.006)),
                   (auge + Vector((0, -0.006, -0.01)), (0.016, 0.008, 0.0045)),
                   ((0.094 * s, 0.008, 1.735), (0.014, 0.03, 0.042)),
                   ((0.097 * s, 0.002, 1.702), (0.011, 0.014, 0.013)),
                   ((0.101 * s, 0.006, 1.735), (0.006, 0.018, 0.026), True)]
    return formen


def augen_paar(b, iris, leuchten=False):
    """Augen: Augapfel, Iris, Pupille, Glanz – oder nur glühende Punkte in dunklen Höhlen."""
    orte = [Vector((AUGE.x * s, AUGE.y, AUGE.z)) for s in (1, -1)]
    if leuchten:
        for o in orte:
            b.f.kugel("Glutauge", o + Vector((0, -0.004, 0)), (0.012, 0.008, 0.009), iris, KOPF, 12, 8)
        return
    b.augen(orte, 0.0125, iris)


def ist_hand(b, p, weite=0.075):
    return min((p - b.griff(s)).length for s in (1, -1)) < weite


def ist_kopf(p):
    return p.z > 1.6 and abs(p.x) < 0.15


def kleidung(b, haut, oben, unten, stiefel, hand=None, gesicht=None, oben_rausch=12.0, streifen=None):
    """Farben des Grundkörpers: Gesicht (Haut oder dunkler Schatten), Hemd/Wams, Hose, Stiefel, Hände."""
    lippe = haut * farbe("#C88070") * 3.0

    def farbe_von(p, n, h):
        if ist_kopf(p) or (p.z > 1.54 and abs(p.x) < 0.07 and p.y > -0.08):
            if gesicht is not None:
                return gesicht(p, n, h)
            c = haut * (0.95 + 0.06 * bh.rausch(p, 25.0))
            if p.y < -0.06:
                c = c.lerp(haut * farbe("#E89080") * 1.3, weich(0.03, 0.0, abs(abs(p.x) - 0.05)) * weich(0.03, 0.0, abs(p.z - 1.705)) * 0.35)
            if (p - Vector((0, -0.1, 1.675))).length < 0.024 and p.y < -0.09:
                c = c.lerp(lippe, 0.5)
            return bh.schmutz(c, h, 0.35, 0.05)
        if ist_hand(b, p):
            return bh.schmutz((hand or haut) * (0.95 + 0.08 * bh.rausch(p, 30.0)), h, 0.4, 0.1)
        if p.z < 0.3 and abs(p.x) < 0.2:
            return bh.schmutz(stiefel * (0.88 + 0.16 * bh.rausch(p, 25.0)), h, 0.5, 0.2)
        if p.z < 0.98 and abs(p.x) < 0.22:
            c = unten * (0.88 + 0.14 * bh.rausch(Vector((p.x * 3, p.y * 3, p.z * 0.6)), 20.0))
            return bh.schmutz(c, h, 0.45, 0.1)
        c = oben * (0.88 + 0.16 * bh.rausch(p, oben_rausch))
        if streifen is not None:
            c = streifen(p, c)
        return bh.schmutz(c, h, 0.45, 0.12)
    return farbe_von


def stoff_falten(p, n):
    """Weiche Falten in Hemd und Hose (längs), Haut glatter."""
    if ist_kopf(p):
        return 0.0006 * bh.rausch(p, 40.0)
    return 0.0022 * noise.noise(Vector((p.x * 28, p.y * 28, p.z * 7))) + 0.0007 * bh.rausch(p, 40.0)


def koerper(f, b, formen, farbe_von, voxel=0.0065, ziel=11000, versatz=None):
    return bh.teil(f, "Koerper", formen, voxel, ziel, farbe_von, versatz=versatz or stoff_falten, knochen=f.knochen)


def rumpf_breite(koerper_obj, z, dz=0.03):
    return bh.umfang(koerper_obj, z, dz, nur=lambda p: abs(p.x) < 0.24)


def rock(f, b, koerper_obj, name, oben_z, unten_z, c, fransen=0.12, weite=0.06, seg=48, falten=0.05, saum=None):
    """Rock/Waffenrock/Schurz vom Gürtel abwärts, am Körper ausgemessen."""
    my, rx, ry = rumpf_breite(koerper_obj, oben_z)
    mitte_z = (oben_z + unten_z) / 2
    ringe = [(Vector((0, my, oben_z)), X, Y, rx + 0.015, ry + 0.015),
             (Vector((0, my - 0.005, mitte_z)), X, Y, rx + weite * 0.6, ry + weite * 0.5, _falten(f.rng, falten)),
             (Vector((0, my - 0.005, unten_z)), X, Y, rx + weite, ry + weite * 0.8, _zacken(f.rng, fransen, 13) if fransen else _falten(f.rng, falten))]
    obj = f.loft(name, ringe, seg, lambda i, k, p: c, b.rock, teilung=4, glatt=True)
    if saum is None:
        _veredeln(obj, c, 18.0, 0.3)
    else:
        bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz((saum if p.z < unten_z + 0.05 else c) * (0.88 + 0.16 * bh.rausch(p, 18.0)), h, 0.45, 0.1))
    bh.gewichte_uebertragen(obj, koerper_obj, lambda kn: kn in ("Becken", "Bauch") or kn.startswith("Oberschenkel"))
    return obj, (my, rx, ry)


def guertel(f, b, koerper_obj, z, c, schnalle=GOLD, hoehe=0.05):
    my, rx, ry = rumpf_breite(koerper_obj, z)
    obj = f.loft("Guertel", [(Vector((0, my, z - hoehe / 2)), X, Y, rx + 0.02, ry + 0.02), (Vector((0, my, z + hoehe / 2)), X, Y, rx + 0.022, ry + 0.022)],
                 40, lambda i, k, p: c, b.rumpf, glatt=True)
    _veredeln(obj, c, 40.0)
    b.f.kiste("Schnalle", (0, my - ry - 0.03, z), (0.05, 0.015, hoehe * 0.85), schnalle, b.rumpf)


def umhang(f, b, c, laenge=1.1, weite=1.0, fetzen=0.08, innen=None, rand=None):
    """Umhang von den Schultern über den Rücken, unten zerfetzt."""
    oben = 1.55
    ringe = [(Vector((0, 0.05, oben)), 0.2 * weite, 0.13), (Vector((0, 0.1, oben - laenge * 0.35)), 0.27 * weite, 0.17),
             (Vector((0, 0.15, oben - laenge * 0.7)), 0.31 * weite, 0.2), (Vector((0, 0.19, oben - laenge)), 0.34 * weite, 0.23)]
    gewicht = lambda co: _mischen(("Brust", weich(0.6, 1.3, co.z)), ("Becken", 1 - weich(0.6, 1.3, co.z)))
    obj = _glocke(f, "Umhang", ringe, (100, 260), lambda poly: c, gewicht, seg=36, welle=fetzen)
    bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz((rand if rand is not None and p.z < oben - laenge + 0.06 else c) * (0.8 + 0.25 * weich(0.3, 1.5, p.z)) *
                                                        (0.9 + 0.12 * bh.rausch(p, 12.0)), h, 0.4, 0.1))
    obj2 = _glocke(f, "Umhang innen", ringe, (100, 260), lambda poly: innen or c * 0.6, gewicht, innen=True, seg=36, welle=fetzen)
    bh.glatt_einfaerben(obj2, lambda p, n, h: (innen or c * 0.55) * (0.8 + 0.2 * p.z / 1.5))


def huelle(f, b, koerper_obj, name, auswahl, dicke, c, rand=None, ziel=1400, relief=None, glanz=0.12):
    """Rüstung eng an der Haut: Stahl mit hellen Kanten, Schmutz in den Fugen, optional Zierrand."""
    def farbe_von(p, n, h):
        basis = c
        if rand is not None:
            basis = rand(p, n, basis)
        return bh.schmutz(basis * (0.85 + 0.2 * max(0.0, n.z) + 0.08 * bh.rausch(p, 30.0)), h, 0.55, glanz)
    return bh.huelle(f, koerper_obj, name, auswahl, dicke, farbe_von, ziel=ziel, versatz=relief)


# ---------------------------------------------------------------------------
# Waffen
# ---------------------------------------------------------------------------
def schwert(b, laenge=0.85, klinge=STAHL * 1.2, parier=STAHL_DUNKEL, griff_c=LEDER, rost=0.0, breite=0.07):
    """Schwert in der rechten Faust, in Ruhe schräg nach vorne unten."""
    f = b.f

    def bauen():
        g = b.griff(-1)
        richtung = Vector((0, -0.62, -0.78)).normalized()
        quer = X
        obj = _strecke(f, "Griff", g - richtung * 0.12, g + richtung * 0.05, 0.016, 0.016, griff_c, b.hand(-1), 12)
        bh.glatt_einfaerben(obj, lambda p, n, h: griff_c * (0.7 + 0.5 * max(0.0, math.sin((p - g).dot(richtung) * 180))))
        f.kugel("Knauf", g - richtung * 0.14, (0.026, 0.026, 0.026), parier, b.hand(-1), 12, 8)
        obj = f.loft("Parier", [(g + richtung * 0.07 - quer * 0.11, quer, richtung, 0.015, 0.02), (g + richtung * 0.07, quer, richtung, 0.02, 0.024),
                                (g + richtung * 0.07 + quer * 0.11, quer, richtung, 0.015, 0.02)], 10, lambda i, k, p: parier, b.hand(-1),
                     oben_zu=True, unten_zu=True, teilung=3, glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(parier, h, 0.5, 0.4))
        ringe = []
        for j in range(8):
            t = j / 7
            w = breite * 0.5 * (1.0 - 0.25 * t) * (1.0 - weich(0.82, 1.0, t))
            ringe.append((g + richtung * (0.08 + (laenge - 0.08) * t), Vector((1, 0, 0)), richtung.cross(quer).normalized(), w + 0.001, 0.007))
        obj = f.loft("Klinge", ringe, 8, lambda i, k, p: klinge, b.hand(-1), oben_zu=True, unten_zu=True, teilung=2)
        bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(klinge.lerp(farbe("#7A4A2E"), rost * weich(-0.2, 0.6, bh.rausch(p, 20.0))) *
                                                            (0.75 if abs(p.x - g.x) < 0.006 else 1.0), h, 0.4, 0.6))
    b.starr("Schwert", "Hand.R", bauen)


def schild(b, aussen, rand, zeichen=None, rund=False, kaputt=False, buckel=STAHL):
    """Schild am linken Unterarm: gewölbt, mit Rand, Buckel und Zeichen."""
    f = b.f
    arm = lambda co: {"Unterarm.L": 1.0}

    def bauen():
        mitte = Vector((0.43, -0.03, 1.02))
        ringe = []
        n = 11
        for j in range(n):
            t = j / (n - 1)
            z = 0.32 - 0.7 * t if not rund else 0.26 - 0.52 * t
            if rund:
                w = math.sqrt(max(0.0, 0.26 ** 2 - z ** 2)) + 0.004
            else:
                w = 0.2 if z > 0.1 else 0.2 * math.sqrt(max(0.0, (z + 0.38) / 0.48)) + 0.004
            ringe.append((mitte + Vector((0.0, 0.0, z)), Y, X, w, 0.028))
        obj = f.loft("Schild", ringe, 24, lambda i, k, p: aussen, arm, oben_zu=True, unten_zu=True, teilung=2, glatt=True)
        # Wölbung nach außen (+X); ein kaputter Schild hat eine abgebrochene Ecke
        for v in obj.data.vertices:
            rel = v.co - mitte
            v.co.x += 0.06 * max(0.0, 1 - (rel.y / 0.22) ** 2 - (rel.z / 0.36) ** 2) * (1 if v.co.x > mitte.x else 0.6)
            if kaputt and rel.y > 0.08 and rel.z > 0.05:
                v.co = mitte + Vector((rel.x, 0.08 + (rel.y - 0.08) * 0.2, 0.05 + (rel.z - 0.05) * 0.2))

        def schild_farbe(p, n, h):
            rel = p - mitte
            r_ = math.sqrt((rel.y / 0.21) ** 2 + (rel.z / (0.36 if not rund else 0.26)) ** 2)
            if n.x < -0.3:
                return LEDER * 0.8
            c = aussen * (0.85 + 0.2 * bh.rausch(p, 14.0))
            if r_ > 0.86:
                c = rand
            elif zeichen is not None and abs(rel.y) < 0.09 - abs(rel.z) * 0.25 and abs(rel.z) < 0.18:
                c = zeichen
            return bh.schmutz(c, h, 0.5, 0.3)
        bh.glatt_einfaerben(obj, schild_farbe)
        f.kugel("Schildbuckel", mitte + Vector((0.095, 0, 0.02)), (0.03, 0.055, 0.055), buckel, arm, 14, 10)
    b.starr("Schild", "Unterarm.L", bauen)


def stangenwaffe(b, laenge=2.5, spitze_c=STAHL, schaft=HOLZ, wimpel=None, name="Pike"):
    """Pike/Speer senkrecht durch die rechte Faust."""
    f = b.f

    def bauen():
        g = b.griff(-1)
        unten, oben = Vector((g.x, g.y, 0.02)), Vector((g.x, g.y, laenge))
        obj = _strecke(f, "Schaft", unten, oben, 0.02, 0.018, schaft, b.hand(-1), 12)
        bh.glatt_einfaerben(obj, lambda p, n, h: schaft * (0.8 + 0.3 * bh.rausch(Vector((p.x * 50, p.y * 50, p.z * 4)), 1.0)))
        obj = f.loft("Spitze", [(oben, X, Y, 0.026, 0.012), (oben + Z * 0.1, X, Y, 0.042, 0.014), (oben + Z * 0.32, X, Y, 0.002, 0.002)], 8,
                     lambda i, k, p: spitze_c, b.hand(-1), oben_zu=True, unten_zu=True, teilung=3)
        bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(spitze_c, h, 0.4, 0.6))
        f.loft("Tuelle", [(oben - Z * 0.08, X, Y, 0.028, 0.028), (oben + Z * 0.02, X, Y, 0.024, 0.024)], 12, lambda i, k, p: STAHL_DUNKEL, b.hand(-1), glatt=True)
        if wimpel is not None:
            ringe = [(oben - Z * (0.08 + 0.05 * j) + Y * 0.12, Y, Z, 0.12 * (1 - 0.15 * j), 0.004) for j in range(3)]
            obj = f.loft("Wimpel", ringe, 8, lambda i, k, p: wimpel, b.hand(-1), oben_zu=True, unten_zu=True, teilung=2, glatt=True)
            bh.glatt_einfaerben(obj, lambda p, n, h: wimpel * (0.85 + 0.2 * math.sin(p.y * 40)))
    b.starr(name, "Hand.R", bauen)


def schulterplatten(f, b, c, lagen=3, groesse=0.14, rand=None):
    for s in (1, -1):
        sl = b.p("schulter", s)
        for j in range(lagen):
            obj = _schale(f, "Schulterplatte", sl + Vector((0.02 * s, 0.0, 0.05 - 0.055 * j)), groesse - 0.012 * j, groesse - 0.012 * j, groesse * 0.7, (0, 360), (0, 80),
                          lambda poly: c, lambda co, s=s: _mischen(("Brust", 0.35), ("Oberarm.L" if s > 0 else "Oberarm.R", 0.65)), seg=(28, 8))
            bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz((rand if rand is not None and n.z < 0.25 else c).lerp(STAHL, weich(0.6, 1.0, n.z) * 0.25), h, 0.6, 0.6))


def kapuze(f, b, koerper_obj, c, spitz=0.0, schulter=True):
    """Kapuze: eine weite Hülle um Kopf und Nacken, vorne offen, das Gesicht liegt im Schatten."""
    def auswahl(p, n):
        vorne_offen = p.y < -0.05 and 1.64 < p.z < 1.83 and abs(p.x) < 0.075
        return (p.z > 1.6 and abs(p.x) < 0.16 or (schulter and p.z > 1.4 and abs(p.x) < 0.27 and not ist_hand(b, p))) and not vorne_offen

    def weite(p, n):
        return 0.012 * weich(1.7, 1.95, p.z) + 0.05 * weich(1.82, 1.95, p.z) * spitz * weich(-0.2, 0.3, p.y)
    obj = huelle(f, b, koerper_obj, "Kapuze", auswahl, 0.02, c, ziel=1800, relief=weite, glanz=0.05)
    if spitz > 0:
        _kegel(f, "Kapuzenspitze", Vector((0, 0.06, 1.9)), Vector((0, 0.6, 1)), 0.18 * spitz, 0.05, c, KOPF, 10, krumm=Vector((0, 1, -0.3)))
    return obj


def schatten_gesicht(p, n, h):
    return farbe("#16121A") * (0.8 + 0.2 * max(0.0, n.z))


# ---------------------------------------------------------------------------
# Dunkler Ritter: geschwärzter Plattenpanzer, Hörnerhelm mit glühendem Sehschlitz,
# violetter Waffenrock, zerschlissener Umhang, Schwert und Wappenschild
# ---------------------------------------------------------------------------
def dunkler_ritter(seed=101):
    f = Figur("DunklerRitter", seed)
    b, formen = mensch(f, breite=1.08, muskel=0.12)

    def ketten(p, c):
        return c * (0.8 if (int(p.z * 90) + int(p.x * 90)) % 2 else 1.0)
    k = koerper(f, b, formen, kleidung(b, farbe("#6A5A50"), STAHL_DUNKEL, VIOLETT_DUNKEL, STAHL_SCHWARZ, hand=STAHL_SCHWARZ, streifen=ketten,
                                       gesicht=lambda p, n, h: bh.schmutz(ketten(p, STAHL_DUNKEL), h, 0.5, 0.1)), ziel=9000)

    def kanten(zs):
        def rand(p, n, c):
            for z in zs:
                if abs(p.z - z) < 0.012:
                    return STAHL * 0.9
            return c
        return rand

    def brust_rand(p, n, c):
        if (p - Vector((0, -0.19, 1.33))).length < 0.035 and p.y < -0.12:
            return GLUT * 1.5
        if abs(p.z - 1.02) < 0.015 or (abs(p.x) < 0.006 and p.y < 0):
            return STAHL * 0.9
        return c
    huelle(f, b, k, "Brustpanzer", lambda p, n: 1.0 < p.z < 1.56 and abs(p.x) < 0.2 + 0.03 * weich(1.4, 1.5, p.z), 0.024, STAHL_SCHWARZ, rand=brust_rand,
           relief=lambda p, n: 0.006 * weich(0.03, 0.0, abs(p.x) - 0.01) * (1 if p.y < 0 else 0))

    def helm_rand(p, n, c):
        if abs(p.z - 1.755) < 0.009 and p.y < -0.07:
            return GLUT * 2.0
        if abs(p.x) < 0.012 and p.z > 1.8:
            return STAHL * 0.9
        return c
    huelle(f, b, k, "Helm", lambda p, n: p.z > 1.6 and abs(p.x) < 0.16, 0.026, STAHL_SCHWARZ, rand=helm_rand, ziel=1200,
           relief=lambda p, n: 0.01 * weich(0.03, 0.0, abs(p.x)) * weich(1.78, 1.86, p.z))
    for s in (1, -1):
        punkte = [Vector((0.1 * s, 0.0, 1.84)), Vector((0.2 * s, -0.02, 1.9)), Vector((0.27 * s, -0.06, 2.0)), Vector((0.28 * s, -0.1, 2.12))]
        obj = f.straehne("Horn", punkte, 0.035, 0.004, KNOCHEN, KOPF, 12, 0.0, 1.0, teilung=4, glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h, a=punkte[0]: farbe("#6A5A48").lerp(KNOCHEN, weich(0.04, 0.2, (p - a).length)))
        huelle(f, b, k, "Armschiene", lambda p, n, s=s: p.x * s > 0.22 and b.p("hand", s).z + 0.02 < p.z < b.p("ellbogen", s).z + 0.02, 0.016, STAHL_SCHWARZ,
               rand=kanten([b.p("ellbogen", s).z - 0.02]), ziel=700)
        huelle(f, b, k, "Beinschiene", lambda p, n, s=s: p.x * s > 0.02 and 0.14 < p.z < 0.62 and abs(p.x) < 0.2, 0.017, STAHL_SCHWARZ,
               rand=kanten([0.5, 0.56]), ziel=800)
        huelle(f, b, k, "Eisenschuh", lambda p, n, s=s: p.x * s > 0.02 and p.z < 0.15 and abs(p.x) < 0.2, 0.012, STAHL_SCHWARZ, ziel=600)
    schulterplatten(f, b, STAHL_SCHWARZ)
    rock(f, b, k, "Waffenrock", 1.02, 0.52, VIOLETT, fransen=0.1, saum=farbe("#7E2227"))
    guertel(f, b, k, 1.04, LEDER * 0.7, STAHL)
    umhang(f, b, VIOLETT_DUNKEL, laenge=1.15, rand=farbe("#1A0E22"))
    schwert(b, 0.88)
    schild(b, STAHL_SCHWARZ, STAHL_DUNKEL, zeichen=GLUT * 0.8)
    return f.fertig(_animationen(_angriff_hieb, arme_ruhe=((-20, -60), (-10, -25))))


# ---------------------------------------------------------------------------
# Soldat der Kaserne (verbündet): blanker Stahl, blauer Waffenrock mit Gold, offener Helm mit
# Nasenschutz und Federbusch, Speer mit Wimpel und Wappenschild
# ---------------------------------------------------------------------------
def soldat(seed=111):
    f = Figur("Soldat", seed)
    blau, blau_dunkel, stahl_hell = farbe("#3F6FB5"), farbe("#2D4F86"), farbe("#A4ACB8")
    b, formen = mensch(f)

    def steppung(p, c):
        return c * (0.85 if int(p.z * 26) % 2 else 1.0)
    k = koerper(f, b, formen, kleidung(b, farbe("#E2B894"), blau_dunkel, farbe("#3A3A48"), LEDER, hand=LEDER * 1.2, streifen=steppung))
    augen_paar(b, farbe("#4A6A8A"))
    huelle(f, b, k, "Harnisch", lambda p, n: 1.02 < p.z < 1.55 and abs(p.x) < 0.2 + 0.03 * weich(1.4, 1.5, p.z), 0.022, stahl_hell,
           rand=lambda p, n, c: GOLD if abs(p.z - 1.03) < 0.014 or (abs(p.x) < 0.007 and p.y < 0) else c)
    huelle(f, b, k, "Helm", lambda p, n: (p.z > 1.765 and abs(p.x) < 0.16) or (p.z > 1.64 and p.y > -0.02 and abs(p.x) < 0.16), 0.02, stahl_hell,
           rand=lambda p, n, c: GOLD if abs(p.z - 1.77) < 0.01 else c)
    f.kiste("Nasenschutz", (0, -0.128, 1.745), (0.016, 0.01, 0.08), stahl_hell, KOPF)
    busch = []
    for j in range(9):
        t = j / 8
        busch.append((Vector((0, -0.04 + 0.26 * t, 1.93 + 0.1 * math.sin(math.pi * t * 0.8) - 0.12 * t * t)), (0.03 + 0.02 * math.sin(math.pi * t), 0.045, 0.04)))
    obj = bh.ball_mesh("Federbusch", busch, 0.012)
    bh.modellieren(obj, 0.006, 1500, lambda p, n: 0.006 * noise.noise(Vector((p.x * 60, p.y * 12, p.z * 60))))
    bh.einfaerben(obj, lambda p, n, h: bh.schmutz(blau * (0.85 + 0.3 * noise.noise(Vector((p.x * 60, p.y * 12, p.z * 60)))), h, 0.5, 0.3))
    f._gewichten(obj, KOPF)
    bh.aufnehmen(f, obj)
    for s in (1, -1):
        huelle(f, b, k, "Beinschiene", lambda p, n, s=s: p.x * s > 0.02 and 0.16 < p.z < 0.55 and abs(p.x) < 0.2, 0.014, stahl_hell, ziel=600)
    schulterplatten(f, b, stahl_hell, lagen=2, groesse=0.12, rand=GOLD)
    rock(f, b, k, "Waffenrock", 1.02, 0.56, blau, fransen=0.0, saum=GOLD)
    guertel(f, b, k, 1.04, LEDER, GOLD)
    stangenwaffe(b, 2.2, stahl_hell, farbe("#9A6B3F"), wimpel=blau, name="Speer")
    schild(b, blau, GOLD, zeichen=GOLD, buckel=GOLD)
    return f.fertig(_animationen(_angriff_stoss, arme_ruhe=((-20, -60), (0, -12))))


# ---------------------------------------------------------------------------
# Pikenier: gesteppter Wams, violetter Überwurf mit Wappen, Eisenhut mit breiter Krempe,
# grimmiges graues Gesicht mit glühenden Augen, lange Pike mit Wimpel
# ---------------------------------------------------------------------------
def pikenier(seed=103):
    f = Figur("Pikenier", seed)
    wams = farbe("#4A4C3A")
    b, formen = mensch(f, breite=1.03)

    def steppung(p, c):
        return c * (0.8 if int(p.z * 24) % 2 else 1.0)

    def gesicht(p, n, h):
        return bh.schmutz(farbe("#7F766C") * (0.9 + 0.1 * bh.rausch(p, 20.0)), h, 0.5, 0.1)
    k = koerper(f, b, formen, kleidung(b, farbe("#8F8478"), wams, farbe("#3E3833"), LEDER * 0.8, hand=LEDER, gesicht=gesicht, streifen=steppung))
    augen_paar(b, GLUT * 1.4, leuchten=True)
    # Gesteppter Gambeson (dick, mit Wülsten), darüber der violette Waffenrock mit Wappen
    huelle(f, b, k, "Gambeson", lambda p, n: 1.0 < p.z < 1.54 and not ist_hand(b, p) and p.z > b.p("ellbogen", 1).z - 0.05 or (1.0 < p.z < 1.54 and abs(p.x) < 0.22),
           0.012, wams, rand=lambda p, n, c: c * (0.75 if int(p.z * 26) % 2 else 1.0), ziel=1800, relief=lambda p, n: 0.004 * abs(math.sin(p.z * 26 * math.pi)))
    huelle(f, b, k, "Waffenrock", lambda p, n: 0.98 < p.z < 1.5 and abs(p.x) < 0.14, 0.03, VIOLETT,
           rand=lambda p, n, c: VIOLETT_HELL if abs(p.x) < 0.012 or (p - Vector((0, -0.2, 1.3))).length < 0.045 and p.y < 0 else c, ziel=1000)
    rock(f, b, k, "Waffenrockschoss", 1.0, 0.62, VIOLETT, fransen=0.08, weite=0.05, saum=VIOLETT_HELL)
    guertel(f, b, k, 1.04, LEDER * 0.8, STAHL)
    huelle(f, b, k, "Eisenhut", lambda p, n: p.z > 1.8 and abs(p.x) < 0.16, 0.02, STAHL_DUNKEL, ziel=900)
    ringe = [(Vector((0, 0.0, 1.8)), X, Y, 0.21, 0.21, lambda w: 1.0 + 0.03 * math.cos(w * 2)), (Vector((0, 0.0, 1.815)), X, Y, 0.215, 0.215),
             (Vector((0, 0.0, 1.83)), X, Y, 0.12, 0.13)]
    obj = f.loft("Krempe", ringe, 40, lambda i, kk, p: STAHL_DUNKEL, KOPF, teilung=2, glatt=True)
    bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(STAHL_DUNKEL * (0.9 + 0.3 * max(0.0, n.z)), h, 0.5, 0.5))
    schulterplatten(f, b, STAHL_DUNKEL, lagen=2, groesse=0.12)
    stangenwaffe(b, 2.55, STAHL * 1.2, HOLZ, wimpel=VIOLETT_HELL)
    return f.fertig(_animationen(_angriff_stoss, arme_ruhe=((0, -15), (0, -12))))


# ---------------------------------------------------------------------------
# Bogenschütze: Lederwams über dunklem Hemd, violette Kapuze mit Schulterumhang (Gesicht im
# Schatten, grüne Augen), Köcher mit Pfeilen, Langbogen in der Linken
# ---------------------------------------------------------------------------
def bogenschuetze(seed=102):
    f = Figur("Bogenschuetze", seed)
    b, formen = mensch(f, breite=0.96)
    def schnuerung(p, c):
        if abs(p.x) < 0.01 and p.y < 0 and 1.1 < p.z < 1.48:
            return farbe("#1A1410")
        if abs(abs(p.x) - 0.018) < 0.006 and p.y < 0 and 1.1 < p.z < 1.48 and int(p.z * 40) % 2:
            return farbe("#C8B89A")
        return c
    k = koerper(f, b, formen, kleidung(b, farbe("#9A8A7A"), farbe("#3E3024"), farbe("#2A2A30"), LEDER * 0.9, hand=LEDER * 0.7, gesicht=schatten_gesicht,
                                       streifen=schnuerung))
    huelle(f, b, k, "Lederwams", lambda p, n: 1.0 < p.z < 1.5 and abs(p.x) < 0.2, 0.01, farbe("#4A3422"), rand=lambda p, n, c: schnuerung(p, c), ziel=1200,
           relief=lambda p, n: 0.003 * weich(0.02, 0.0, abs(abs(p.x) - 0.1)))
    augen_paar(b, GIFT * 1.5, leuchten=True)
    kapuze(f, b, k, VIOLETT_DUNKEL, spitz=0.4)
    rock(f, b, k, "Schoss", 1.02, 0.66, VIOLETT, fransen=0.14, saum=VIOLETT_HELL)
    guertel(f, b, k, 1.04, LEDER * 0.6, STAHL)
    riemen = [bh.auf_haut(k, p, 0.012) for p in (Vector((0.18, -0.1, 1.5)), Vector((0.05, -0.16, 1.35)), Vector((-0.1, -0.14, 1.18)), Vector((-0.16, -0.08, 1.06)))]
    obj = f.straehne("Riemen", riemen, 0.018, 0.018, LEDER * 0.6, b.rumpf, 8, 0.0, 0.3, teilung=3, glatt=True)
    _veredeln(obj, LEDER * 0.6, 40.0)
    brust = lambda co: {"Brust": 1.0}
    a, e = Vector((-0.1, 0.19, 1.05)), Vector((0.12, 0.21, 1.62))
    obj = _strecke(f, "Koecher", a, e, 0.07, 0.078, LEDER, brust, 16)
    bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz((LEDER * (0.8 + 0.3 * bh.rausch(p, 25.0))).lerp(STAHL_DUNKEL, weich(0.03, 0.0, abs((p - a).length - 0.1))), h, 0.5, 0.2))
    for i in range(6):
        spitze = e + Vector((0.025 * (i - 2.5), 0.012 * (i % 2), 0.16 + 0.02 * (i % 3)))
        _strecke(f, "Pfeil", e - Vector((0, 0, 0.05)), spitze, 0.007, 0.007, HOLZ * 1.2, brust, 6)
        for w in range(3):
            _platte(f, "Feder", spitze - Vector((0, 0, 0.035)) + Vector((math.cos(w * 2.1) * 0.008, math.sin(w * 2.1) * 0.008, 0)), 0.06, 0.012, 0.002, Z,
                    Vector((math.cos(w * 2.1), math.sin(w * 2.1), 0)), lambda ii, kk, p, i=i: VIOLETT_HELL if i % 2 else KNOCHEN, brust, spitz=0.3, wolbung=0.0)

    def bogen():
        g = b.griff(1)
        holz = farbe("#5A3E26")
        for vz in (1, -1):
            punkte = [g + Vector((0.05 * (t ** 1.4) * 2, vz * (0.06 + 0.66 * t), 0.02 * math.sin(math.pi * t))) for t in (0.0, 0.25, 0.5, 0.75, 1.0)]
            obj = f.straehne("Bogenarm", punkte, 0.021, 0.009, holz, b.hand(1), 10, 0.0, 0.65, teilung=3, glatt=True)
            bh.glatt_einfaerben(obj, lambda p, n, h: holz * (0.8 + 0.35 * bh.rausch(Vector((p.x * 30, p.y * 6, p.z * 30)), 1.0)))
            f.kugel("Bogenspitze", punkte[-1], (0.012, 0.018, 0.012), KNOCHEN, b.hand(1), 10, 6)
        ende = [g + Vector((0.1, vz * 0.72, 0.0)) for vz in (1, -1)]
        _strecke(f, "Sehne", ende[0], ende[1], 0.0035, 0.0035, KNOCHEN, b.hand(1), 5)
        obj = _strecke(f, "Griffwicklung", g + Vector((0, -0.06, 0)), g + Vector((0, 0.06, 0)), 0.025, 0.025, LEDER * 0.6, b.hand(1), 12)
        bh.glatt_einfaerben(obj, lambda p, n, h: LEDER * (0.5 + 0.3 * max(0.0, math.sin(p.y * 300))))
    b.starr("Bogen", "Hand.L", bogen)
    return f.fertig(_animationen(_angriff_schuss, arme_ruhe=((-5, -15), (0, -10))))


# ---------------------------------------------------------------------------
# Skelettkrieger: ein echtes Gerippe – Schädel mit Augenhöhlen und Zähnen, Rippenkorb, Wirbel,
# Becken, Knochenhände; rostiger Helm, zerfetzter Lendenschurz, rostiges Schwert, kaputter Schild
# ---------------------------------------------------------------------------
def skelettkrieger(seed=104):
    f = Figur("Skelettkrieger", seed)
    r = f.rng
    g = dict(MENSCH)
    b = Bau(f, **g)
    b.skelett()
    formen = []
    # Beine: Oberschenkel mit Gelenkköpfen, Kniescheibe, Schienbein, Fußknochen
    for s in (1, -1):
        hu, kn, ks = b.p("huefte", s), b.p("knie", s), b.p("knoechel", s)
        formen += [(hu + Vector((-0.02 * s, 0, 0.02)), (0.035, 0.035, 0.035))]
        formen += bh.glied(hu, kn, 0.024, 0.02, muskel=-0.2, lage=0.5)
        formen += [(kn, (0.034, 0.03, 0.03)), (kn + Vector((0, -0.025, 0)), (0.02, 0.012, 0.022))]
        formen += bh.glied(kn, ks, 0.021, 0.016, muskel=-0.15, lage=0.5)
        formen += [(ks, (0.024, 0.026, 0.02))]
        for j in (-1, 0, 1):
            formen += bh.glied(Vector((ks.x, 0.01, 0.04)), Vector((ks.x + 0.02 * j, -0.13, 0.015)), 0.013, 0.009)
        formen += [(Vector((ks.x, 0.03, 0.035)), (0.022, 0.03, 0.02))]
        # Arme: Schulterkugel, Oberarm, Ellbogen, Elle und Speiche, Knochenfaust
        sh, el, ha = b.p("schulter", s), b.p("ellbogen", s), b.p("hand", s)
        formen += [(sh, (0.034, 0.034, 0.034))]
        formen += bh.glied(sh, el, 0.021, 0.018, muskel=-0.2, lage=0.5)
        formen += [(el, (0.027, 0.027, 0.027))]
        for q in (-1, 1):
            formen += bh.glied(el + Vector((0.008 * q, 0, 0)), ha + Vector((0.008 * q, 0, 0)), 0.012, 0.011)
        formen += b.faust_formen(s, 0.85)
        # Schlüsselbein
        formen += bh.glied(Vector((0.02 * s, -0.04, 1.5)), sh + Vector((-0.02 * s, 0, 0.01)), 0.014, 0.014)
    # Becken: Schaufeln und Kreuzbein
    for s in (1, -1):
        formen += [(Vector((0.085 * s, 0.0, 1.0)), (0.06, 0.03, 0.055)), (Vector((0.05 * s, -0.03, 0.94)), (0.04, 0.025, 0.03))]
    formen += [(Vector((0, 0.035, 0.98)), (0.04, 0.025, 0.05)), (Vector((0, 0.0, 0.93)), (0.07, 0.03, 0.02))]
    # Wirbelsäule
    for j in range(15):
        z = 1.0 + j * 0.043
        formen.append((Vector((0, 0.045 - 0.02 * math.sin(j / 14 * math.pi), z)), (0.02, 0.02, 0.014)))
        formen.append((Vector((0, 0.07 - 0.02 * math.sin(j / 14 * math.pi), z)), (0.008, 0.015, 0.008)))
    # Rippen: Bögen vom Rückgrat nach vorne zum Brustbein
    for j in range(7):
        z = 1.2 + j * 0.045
        weite = 0.12 + 0.045 * math.sin(math.pi * (j + 0.8) / 7.5)
        tiefe = 0.1 + 0.02 * math.sin(math.pi * (j + 0.5) / 7)
        for s in (1, -1):
            punkte = []
            for t in range(9):
                w = math.pi * t / 8
                punkte.append(Vector((s * math.sin(w) * weite, 0.04 - (1 - math.cos(w)) * tiefe, z - 0.03 * t / 8)))
            for a_, e_ in zip(punkte, punkte[1:]):
                formen += bh.glied(a_, e_, 0.011, 0.011, dichte=0.8)
    formen += bh.glied(Vector((0, -0.165, 1.48)), Vector((0, -0.16, 1.2)), 0.017, 0.012)        # Brustbein
    formen += bh.glied(Vector((0, 0.03, 1.6)), Vector((0, 0.02, 1.67)), 0.02, 0.02)              # Halswirbel
    # Schädel: Hirnschale, Stirn, Wangenknochen, Oberkiefer, Unterkiefer, Höhlen
    k = Vector((0, 0.0, 1.76))
    formen += [(k + Vector((0, 0.01, 0.02)), (0.085, 0.1, 0.095)), (k + Vector((0, -0.06, -0.02)), (0.07, 0.05, 0.06)),
               (k + Vector((0, -0.07, -0.075)), (0.05, 0.04, 0.03)), (k + Vector((0, -0.05, -0.11)), (0.055, 0.045, 0.022)),
               (k + Vector((0, -0.1, -0.035)), (0.012, 0.015, 0.018), True)]
    for s in (1, -1):
        formen += [(k + Vector((0.035 * s, -0.085, 0.0)), (0.024, 0.022, 0.024), True),
                   (k + Vector((0.06 * s, -0.06, -0.03)), (0.025, 0.02, 0.018))]

    def knochen_farbe(p, n, h):
        c = KNOCHEN.lerp(farbe("#A89A7A"), weich(0.2, 0.9, bh.rausch(p, 9.0) * 0.5 + 0.5) * 0.6)
        c = c.lerp(farbe("#5A4A30"), weich(0.4, 0.0, n.z + 0.4) * 0.15)
        return bh.schmutz(c, h, 0.8, 0.2)
    bh.teil(f, "Gerippe", formen, 0.0042, 16000, knochen_farbe, versatz=lambda p, n: 0.001 * bh.rausch(p, 50.0), knochen=f.knochen,
            naechster_knochen=True, inseln=False)
    # Augenglut, Zähne
    for s in (1, -1):
        b.f.kugel("Glutauge", k + Vector((0.035 * s, -0.08, 0.0)), (0.01, 0.008, 0.01), GIFT * 1.6, KOPF, 10, 6)
    for j in range(8):
        x = -0.035 + j * 0.01
        for oben in (True, False):
            b.f.kiste("Zahn", (x, -0.095 + abs(x) * 0.4, 1.702 if oben else 1.66), (0.008, 0.008, 0.016), KNOCHEN, KOPF)
    # Rostiger Helm, zerfetzter Lendenschurz, Gürtel
    rost = farbe("#7A4A2E")
    ringe = [(k + Vector((0, 0.01, z)), X, Y, rx, ry) for z, rx, ry in ((0.02, 0.1, 0.112), (0.07, 0.096, 0.108), (0.11, 0.07, 0.08), (0.13, 0.02, 0.02))]
    obj = f.loft("Helm", ringe, 28, lambda i, kk, p: rost, KOPF, oben_zu=True, teilung=3, glatt=True)
    bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(rost.lerp(STAHL_DUNKEL, weich(0.0, 0.5, bh.rausch(p, 15.0))) , h, 0.5, 0.3))
    ringe = [(Vector((0, 0.0, 1.0)), X, Y, 0.14, 0.09), (Vector((0, -0.01, 0.85)), X, Y, 0.16, 0.11, _falten(r, 0.08)),
             (Vector((0, -0.01, 0.7)), X, Y, 0.17, 0.12, _zacken(r, 0.35, 9))]
    obj = f.loft("Lendenschurz", ringe, 36, lambda i, kk, p: VIOLETT_DUNKEL, b.rock, teilung=4, glatt=True)
    _veredeln(obj, VIOLETT_DUNKEL, 18.0, 0.4)
    obj = f.loft("Guertel", [(Vector((0, 0.0, 0.99)), X, Y, 0.142, 0.092), (Vector((0, 0.0, 1.03)), X, Y, 0.142, 0.092)], 28, lambda i, kk, p: LEDER * 0.6,
                 b.rumpf, glatt=True)
    schwert(b, 0.75, klinge=farbe("#8A7A6A"), rost=0.8)
    schild(b, rost * 0.9, rost * 0.6, rund=True, kaputt=True, buckel=rost)
    return f.fertig(_animationen(_angriff_hieb, arme_ruhe=((-18, -55), (-8, -25)), gehen=(24, 36, 12, 0.03, 8, 10)))


# ---------------------------------------------------------------------------
# Dunkelmagier: lange violette Robe mit Goldsaum und Runen, spitze Kapuze, glühende Augen,
# weite Ärmel, Schärpe, Knochenstab mit schwebendem Kristall
# ---------------------------------------------------------------------------
def aermel(f, b, s, c, saum=None, weite=0.1, fetzen=0.0):
    e, h = b.p("ellbogen", s), b.p("hand", s)
    ax = (h - e).normalized()
    q = ax.cross(Y).normalized()
    ringe = [(e.lerp(h, 0.25), q, ax.cross(q).normalized(), 0.058, 0.058), (e.lerp(h, 0.7), q, ax.cross(q).normalized(), weite * 0.75, weite * 0.7),
             (h + ax * 0.03, q, ax.cross(q).normalized(), weite, weite * 0.9, _zacken(f.rng, fetzen, 9) if fetzen else _falten(f.rng, 0.05))]
    obj = f.loft("Aermel", ringe, 24, lambda i, k, p: c, b.arm(s), teilung=3, glatt=True)
    bh.glatt_einfaerben(obj, lambda p, n, hh: bh.schmutz((saum if saum is not None and (p - h).dot(ax) > -0.02 else c) * (0.85 + 0.2 * bh.rausch(p, 18.0)), hh, 0.4, 0.1))


def dunkelmagier(seed=105):
    f = Figur("Dunkelmagier", seed)
    b, formen = mensch(f, breite=0.95)
    k = koerper(f, b, formen, kleidung(b, farbe("#9C8FA8"), VIOLETT, VIOLETT_DUNKEL, farbe("#16121A"), hand=farbe("#8C7F98"), gesicht=schatten_gesicht))
    augen_paar(b, GLUT * 1.6, leuchten=True)
    kapuze(f, b, k, VIOLETT_DUNKEL, spitz=1.0)

    def robe_saum(p, n, c):
        return c
    obj, _ = rock(f, b, k, "Robe", 1.02, 0.03, VIOLETT, fransen=0.0, weite=0.16, falten=0.08, saum=GOLD * 0.8)
    # Runen am Saum
    my, rx, ry = rumpf_breite(k, 1.02)
    for j in range(8):
        w = math.tau * j / 8
        f.stern("Rune", Vector((math.sin(w) * (rx + 0.13), my - math.cos(w) * (ry + 0.1), 0.18)), Vector((math.sin(w), -math.cos(w), 0)), 0.035, GLUT, b.rock, zacken=3)
    obj = f.loft("Schaerpe", [(Vector((0, my, 1.0)), X, Y, rx + 0.02, ry + 0.02), (Vector((0, my, 1.06)), X, Y, rx + 0.022, ry + 0.022)], 36,
                 lambda i, kk, p: GLUT * 0.7, b.rumpf, glatt=True)
    for s in (1, -1):
        aermel(f, b, s, VIOLETT_DUNKEL, saum=VIOLETT_HELL, weite=0.1)
    # Runen, die um die Brust kreisen (hängen an der Brust)
    for j in range(3):
        w = math.tau * j / 3
        f.stern("Schwebrune", Vector((math.cos(w) * 0.32, math.sin(w) * 0.32 - 0.05, 1.25)), Vector((math.cos(w), math.sin(w), 0)), 0.045, GLUT,
                lambda co: {"Brust": 1.0}, zacken=3)

    def stab():
        g_ = b.griff(-1)
        unten, oben = Vector((g_.x, g_.y, 0.05)), Vector((g_.x, g_.y, 2.0))
        punkte = [unten.lerp(oben, t) + Vector((0.012 * math.sin(t * 9), 0.01 * math.cos(t * 7), 0)) for t in (0.0, 0.25, 0.5, 0.75, 1.0)]
        obj = f.straehne("Stab", punkte, 0.022, 0.018, farbe("#8A7A68"), b.hand(-1), 10, 0.0, 1.0, teilung=4, glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(farbe("#8A7A68") * (0.7 + 0.4 * bh.rausch(Vector((p.x * 40, p.y * 40, p.z * 5)), 1.0)), h, 0.5, 0.2))
        for i in range(4):
            w = math.tau * i / 4
            punkte = [oben, oben + Vector((math.cos(w) * 0.07, math.sin(w) * 0.07, 0.08)), oben + Vector((math.cos(w) * 0.045, math.sin(w) * 0.045, 0.22))]
            f.straehne("Klaue", punkte, 0.015, 0.003, KNOCHEN, b.hand(-1), 8, 0.0, 1.0, glatt=True)
        f.loft("Kristall", [(oben + Z * 0.05, X, Y, 0.001, 0.001), (oben + Z * 0.15, X, Y, 0.055, 0.055), (oben + Z * 0.28, X, Y, 0.001, 0.001)], 6,
               lambda i, kk, p: GLUT * 1.4, b.hand(-1), teilung=1)
    b.starr("Stab", "Hand.R", stab)
    return f.fertig(_animationen(_angriff_zauber, arme_ruhe=((-5, -20), (0, -8)), gehen=(22, 34, 12, 0.02, 4, 12)))


# ---------------------------------------------------------------------------
# Gespenst: schwebende, zerfetzte Robe ohne Beine, fahl leuchtend, Kapuze mit grünen Augen,
# Ketten um die Brust, Sense
# ---------------------------------------------------------------------------
def gespenst(seed=108):
    f = Figur("Gespenst", seed)
    r = f.rng
    geist, geist_hell = farbe("#5E6F7A"), farbe("#A8D8D4")
    b, formen = mensch(f, breite=0.9)
    formen = [fo for fo in formen if fo[0][2] > 0.95 or abs(fo[0][0]) > 0.2]        # ohne Beine: nur Rumpf, Arme, Kopf
    k = koerper(f, b, formen, kleidung(b, farbe("#B8D0CC"), geist, geist, geist, hand=farbe("#C7D8D2"), gesicht=schatten_gesicht))
    augen_paar(b, GIFT * 1.8, leuchten=True)
    kapuze(f, b, k, geist * 0.75, spitz=0.6)
    ringe = [(Vector((0, 0.0, 1.05)), X, Y, 0.17, 0.13), (Vector((0, 0.02, 0.8)), X, Y, 0.23, 0.18, _falten(r, 0.08)),
             (Vector((0, 0.05, 0.5)), X, Y, 0.26, 0.2, _falten(r, 0.1)), (Vector((0, 0.08, 0.0)), X, Y, 0.2, 0.15, _zacken(r, 0.45, 11))]
    obj = f.loft("Geisterrobe", ringe, 48, lambda i, kk, p: geist, b.rumpf, teilung=4, glatt=True)
    bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(geist.lerp(geist_hell, weich(0.55, 0.25, p.z) * 0.8) * (0.85 + 0.25 * bh.rausch(p, 10.0)), h, 0.4, 0.2))
    for s in (1, -1):
        aermel(f, b, s, geist, saum=geist_hell, weite=0.12, fetzen=0.35)
    # Ketten um die Brust aus echten Gliedern
    for i in range(16):
        w = math.tau * i / 16
        mitte = Vector((math.cos(w) * 0.22, math.sin(w) * 0.16, 1.22 + 0.05 * math.sin(w * 2)))
        tang = Vector((-math.sin(w), math.cos(w), 0.1 * math.cos(w * 2)))
        quer = Z if i % 2 else Vector((math.cos(w), math.sin(w), 0))
        f.loft("Kettenglied", [(mitte - tang * 0.004, quer, tang.cross(quer).normalized(), 0.022, 0.014), (mitte + tang * 0.004, quer, tang.cross(quer).normalized(), 0.022, 0.014)],
               10, lambda ii, kk, p: STAHL_DUNKEL, lambda co: {"Brust": 1.0}, glatt=True)

    def sense():
        g_ = b.griff(-1)
        unten, oben = Vector((g_.x, g_.y + 0.11, 0.1)), Vector((g_.x, g_.y - 0.05, 2.05))
        obj = _strecke(f, "Stiel", unten, oben, 0.02, 0.018, HOLZ * 0.7, b.hand(-1), 12)
        bh.glatt_einfaerben(obj, lambda p, n, h: HOLZ * 0.7 * (0.8 + 0.3 * bh.rausch(Vector((p.x * 50, p.y * 50, p.z * 5)), 1.0)))
        ringe = []
        for j in range(12):
            t = j / 11
            w = math.pi * 0.95 * t
            mitte = oben + Vector((0, -math.sin(w) * 0.42, -0.05 + (1 - math.cos(w)) * 0.12 - 0.1 * t))
            ringe.append((mitte, X, Vector((0, -math.cos(w), math.sin(w))), 0.004, 0.07 * (1 - t) ** 0.7 + 0.004))
        obj = f.loft("Sensenblatt", ringe, 6, lambda i, kk, p: STAHL, b.hand(-1), oben_zu=True, unten_zu=True, teilung=2)
        bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(farbe("#B9C3C8"), h, 0.5, 0.6))
    b.starr("Sense", "Hand.R", sense)
    return f.fertig(_animationen(_angriff_sense, arme_ruhe=((-30, -45), (-25, -35)), schweben=True))


# ---------------------------------------------------------------------------
# Schattenmeuchler: schlankes dunkles Leder, Kapuze und violette Maske, gekreuzte Gurte mit
# Wurfmessern, kurzer zerfetzter Umhang, zwei Dolche
# ---------------------------------------------------------------------------
def dolch(b, seite, name):
    f = b.f

    def bauen():
        g_ = b.griff(seite)
        unten = Vector((0, -0.25, -1)).normalized()
        obj = _strecke(f, "Griff", g_ + Vector((0, 0, 0.07)), g_ - Vector((0, 0, 0.05)), 0.014, 0.014, LEDER * 0.6, b.hand(seite), 10)
        f.loft("Parier", [(g_ - Z * 0.055 - X * 0.05, X, Z, 0.012, 0.012), (g_ - Z * 0.055 + X * 0.05, X, Z, 0.012, 0.012)], 8, lambda i, k, p: STAHL_DUNKEL,
               b.hand(seite), oben_zu=True, unten_zu=True, glatt=True)
        ringe = [(g_ - Z * 0.06 + unten * 0.3 * t, Y, X, 0.022 * (1 - t ** 1.5) + 0.001, 0.006) for t in (0.0, 0.3, 0.6, 0.85, 1.0)]
        obj = f.loft("Klinge", ringe, 6, lambda i, k, p: STAHL, b.hand(seite), oben_zu=True, unten_zu=True, teilung=2)
        bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(farbe("#B9C0CC"), h, 0.4, 0.6))
    b.starr(name, "Hand.R" if seite < 0 else "Hand.L", bauen)


def schattenmeuchler(seed=110):
    f = Figur("Schattenmeuchler", seed)
    stoff, leder, violett = farbe("#24202C"), farbe("#3E322C"), farbe("#4A2466")
    b, formen = mensch(f, breite=0.9, muskel=0.1)
    k = koerper(f, b, formen, kleidung(b, farbe("#6A5A60"), stoff, stoff, leder * 0.7, hand=leder * 0.6, gesicht=schatten_gesicht))
    augen_paar(b, GLUT * 1.6, leuchten=True)
    kapuze(f, b, k, stoff * 1.3, spitz=0.3)
    huelle(f, b, k, "Maske", lambda p, n: ist_kopf(p) and p.y < -0.02 and p.z < 1.735, 0.012, violett * 0.8, ziel=500)
    huelle(f, b, k, "Lederharnisch", lambda p, n: 1.02 < p.z < 1.48 and abs(p.x) < 0.19, 0.01, leder,
           rand=lambda p, n, c: c * (0.7 if int(p.z * 30) % 3 == 0 else 1.0), ziel=1000)
    for s in (1, -1):
        punkte = [bh.auf_haut(k, p, 0.018) for p in (Vector((0.17 * s, -0.1, 1.5)), Vector((0.02 * s, -0.17, 1.28)), Vector((-0.15 * s, -0.12, 1.04)))]
        obj = f.straehne("Gurt", punkte, 0.016, 0.016, leder * 0.7, b.rumpf, 8, 0.0, 0.3, teilung=3, glatt=True)
        _veredeln(obj, leder * 0.7, 40.0)
        huelle(f, b, k, "Armband", lambda p, n, s=s: p.x * s > 0.24 and b.p("hand", s).z + 0.02 < p.z < b.p("hand", s).z + 0.14, 0.01, leder, ziel=400)
    for j in range(3):
        o = bh.auf_haut(k, Vector((0.05 - j * 0.05, -0.2, 1.33 - j * 0.07)), 0.022)
        f.loft("Wurfmesser", [(o + Z * 0.04, X, Y, 0.012, 0.004), (o - Z * 0.05, X, Y, 0.001, 0.001)], 6, lambda i, kk, p: farbe("#8A909C"), b.rumpf, oben_zu=True, glatt=True)
    rock(f, b, k, "Schoss", 1.02, 0.7, stoff, fransen=0.25, saum=violett * 0.7)
    guertel(f, b, k, 1.04, leder * 0.7, STAHL_DUNKEL)
    umhang(f, b, stoff * 1.1, laenge=0.85, fetzen=0.12, rand=violett * 0.6)
    dolch(b, -1, "DolchR")
    dolch(b, 1, "DolchL")
    return f.fertig(_animationen(_angriff_hieb, arme_ruhe=((-40, -75), (-40, -75)), gehen=(30, 45, 18, 0.03, 12, 25)))


# ---------------------------------------------------------------------------
# Steingolem: massiger Körper aus verwachsenen Felsbrocken mit Rissen, violett glühende Adern,
# Moos auf Schultern und Kopf, Kristalle auf dem Rücken (1,7-fach groß)
# ---------------------------------------------------------------------------
def skalieren(f, faktor):
    m = Matrix.Scale(faktor, 4)
    for obj in f.teile + [o for _, _, objs in f.starr for o in objs]:
        obj.data.transform(m)
    f.knochen = [(n, tuple(Vector(a) * faktor), tuple(Vector(e) * faktor), p, o) for n, a, e, p, o in f.knochen]


def steingolem(seed=106):
    f = Figur("Steingolem", seed)
    r = f.rng
    b = Bau(f, **MENSCH)
    b.skelett()
    fels, fels_dunkel, moos = farbe("#6A6570"), farbe("#3E3A44"), farbe("#4E6E3A")
    formen = []

    def brocken(mitte, groesse):
        mitte = Vector(mitte)
        formen.append((mitte, tuple(g * r.uniform(0.9, 1.1) for g in groesse)))
        for _ in range(2):
            formen.append((mitte + Vector((r.uniform(-1, 1) * groesse[0] * 0.5, r.uniform(-1, 1) * groesse[1] * 0.5, r.uniform(-1, 1) * groesse[2] * 0.4)),
                           tuple(g * r.uniform(0.5, 0.7) for g in groesse)))
    for s in (1, -1):
        hu, kn, ks = b.p("huefte", s), b.p("knie", s), b.p("knoechel", s)
        brocken(hu.lerp(kn, 0.45) + Vector((0.02 * s, 0, 0)), (0.13, 0.13, 0.22))
        brocken(kn + Vector((0, -0.03, 0)), (0.1, 0.1, 0.09))
        brocken(kn.lerp(ks, 0.5), (0.11, 0.11, 0.2))
        brocken(Vector((ks.x, -0.05, 0.08)), (0.13, 0.18, 0.09))
        sh, el, ha = b.p("schulter", s), b.p("ellbogen", s), b.p("hand", s)
        brocken(sh + Vector((0.05 * s, 0, 0.04)), (0.14, 0.13, 0.12))
        brocken(sh.lerp(el, 0.5), (0.1, 0.1, 0.17))
        brocken(el, (0.085, 0.085, 0.08))
        brocken(el.lerp(ha, 0.5), (0.11, 0.11, 0.16))
        brocken(b.griff(s) + Vector((0, -0.01, 0.02)), (0.12, 0.12, 0.12))
    brocken((0, 0, 0.98), (0.22, 0.16, 0.13))
    brocken((0, 0, 1.15), (0.2, 0.15, 0.15))
    brocken((0, 0.0, 1.38), (0.29, 0.2, 0.19))
    brocken((0, 0.06, 1.55), (0.16, 0.13, 0.1))
    brocken((0, -0.04, 1.66), (0.1, 0.1, 0.09))

    def risse(p, n):
        z = bh.zellen(p, 7.0)
        return -0.012 * weich(0.07, 0.0, z) + 0.01 * bh.rausch(p, 5.0) + 0.003 * bh.rausch(p, 22.0)

    def fels_farbe(p, n, h):
        z = bh.zellen(p, 7.0)
        c = fels.lerp(fels_dunkel, 0.3 + 0.3 * bh.rausch(p, 4.0)) * (0.9 + 0.15 * bh.rausch(p, 25.0))
        c = c.lerp(moos, weich(0.55, 0.85, n.z) * weich(-0.1, 0.3, bh.rausch(p, 3.0)) * 0.9)
        if z < 0.04 and bh.rausch(p + Vector((3, 3, 3)), 2.5) > 0.05:
            return GLUT * (1.2 + bh.rausch(p, 40.0) * 0.4)                  # glühende Adern in den Rissen
        return bh.schmutz(c, h, 0.6, 0.25)
    bh.teil(f, "Fels", formen, 0.009, 16000, fels_farbe, versatz=risse, knochen=f.knochen, naechster_knochen=True, inseln=False)
    for s in (1, -1):
        b.f.kugel("Glutauge", Vector((0.037 * s, -0.135, 1.68)), (0.02, 0.01, 0.014), GLUT * 1.6, KOPF, 12, 8)
    for i in range(6):
        basis = Vector((r.uniform(-0.16, 0.16), 0.17, r.uniform(1.3, 1.58)))
        richtung = Vector((basis.x * 2, 1.0, 0.9)).normalized()
        obj = f.loft("Kristall", [(basis, X, Y, 0.04, 0.035), (basis + richtung * 0.1, X, Y, 0.045, 0.04), (basis + richtung * r.uniform(0.2, 0.28), X, Y, 0.001, 0.001)],
                     6, lambda ii, kk, p: GLUT, lambda co: {"Brust": 1.0}, oben_zu=True, unten_zu=True)
        bh.glatt_einfaerben(obj, lambda p, n, h: GLUT * (0.8 + 0.6 * max(0.0, n.z)))
    skalieren(f, 1.7)
    return f.fertig(_animationen(_angriff_stampfen, arme_ruhe=((-5, -20), (-5, -20)), gehen=(18, 25, 10, 0.02, 6, 20)))

OHNE = lambda co: {}


# ---------------------------------------------------------------------------
# Tiere (tiere.py) mit modelliertem Körper statt facettierter Metaball-Hülle
# ---------------------------------------------------------------------------
def _modellierter_tierkoerper(versatz, voxel=0.012, ziel=9000):
    def koerper(self, zonen, aufloesung=0.03, beulen=0.012, ziel_alt=2200, **_):
        formen = [(m, h, False, d) for m, h, d in self.formen]
        obj = bh.ball_mesh(self.name, formen, voxel * 1.6 * self.s)
        bh.modellieren(obj, voxel * self.s, ziel, lambda p, n: versatz(p / self.s, n) * self.s, glaetten=3)
        for v in obj.data.vertices:
            v.co.z = max(v.co.z, 0.0)
        bh.einfaerben(obj, lambda p, n, h: bh.schmutz(zonen(p / self.s, n) * (0.9 + 0.14 * noise.noise(Vector((p.x * 70, p.y * 12, p.z * 70)) / self.s)), h, 0.45, 0.12))
        self.rumpf = obj
        return obj
    return koerper


def fell_strich(p, n):
    """Fell, nach hinten gekämmt: längliche Strähnen, auf dem Rücken länger."""
    straehne = noise.noise(Vector((p.x * 70, p.y * 11, p.z * 70)))
    return 0.004 * straehne + 0.005 * weich(0.6, 0.9, p.z) * max(0.0, straehne) + 0.0015 * bh.rausch(p, 30.0)


def schattenwolf(seed=107):
    """Großer Schattenwolf: dichte Mähne um Hals und Schultern, gesträubter Rückenkamm mit
    Knochenstacheln, lange schmale Schnauze mit gefletschten Zähnen, kleine glühende Augen."""
    import tiere
    from gegner import _schleife  # noqa: F401  (Animationen kommen aus tiere._animieren)
    t = tiere.Tier("Schattenwolf", seed, massstab=1.35)
    kz = tiere._hundeartig(t, schnauze=1.3, ohr=0.8, schwanz=1.3, bein=1.0)
    # Mähne und Rückenkamm, tiefe Brust
    t.form((0, -0.5, 0.86), (0.19, 0.2, 0.2))
    t.form((0, -0.62, 0.93), (0.16, 0.14, 0.16))
    for i in range(6):
        y = -0.4 + i * 0.13
        t.form((0, y, 0.8 - 0.035 * i), (0.08, 0.09, 0.05))
    t.form((0, -0.36, 0.52), (0.13, 0.16, 0.14))
    fell, ruecken, bauch = farbe("#2E2A34"), farbe("#141218"), farbe("#5A5462")

    def zonen(p, n):
        c = fell.lerp(ruecken, weich(0.3, 0.8, n.z) * weich(0.62, 0.8, p.z))
        c = c.lerp(bauch, weich(-0.2, -0.7, n.z) * 0.8)
        c = c.lerp(bauch * 1.2, weich(-0.62, -0.75, p.y) * weich(0.75, 0.6, p.z) * 0.6)          # helle Kehle
        c = c.lerp(VIOLETT_DUNKEL * 1.5, weich(0.8, 1.05, p.y) * 0.8)                           # Schwanzspitze
        return c

    def mähne(p, n):
        lang = weich(-0.25, -0.45, p.y) * weich(0.7, 0.85, p.z) + weich(0.65, 0.85, p.z) * weich(0.3, -0.2, p.y) * 0.6
        return fell_strich(p, n) + 0.008 * lang * max(0.0, noise.noise(Vector((p.x * 60, p.y * 10, p.z * 60))))
    alt = tiere.Tier.koerper
    tiere.Tier.koerper = _modellierter_tierkoerper(mähne, voxel=0.011, ziel=9000)
    try:
        t.koerper(zonen)
    finally:
        tiere.Tier.koerper = alt
    t.ohr(t.v(0.06, -0.73, kz + 0.07), t.v(0.085, -0.7, kz + 0.16), 0.06, 0.025, "#141218", "#3A1E48")
    t.ohr(t.v(-0.06, -0.73, kz + 0.07), t.v(-0.085, -0.7, kz + 0.16), 0.06, 0.025, "#141218", "#3A1E48")
    for ort in t.augen_orte(0.05, kz + 0.035):
        t.kugel(ort, 0.012 * t.s, GLUT * 1.6, name="Auge")
    t.kugel(t.v(0, -1.18, kz - 0.03), 0.026, farbe("#0B0A0D"), groesse=(1.2, 0.9, 0.8), name="Nase")
    for i in range(7):
        y = -0.42 + i * 0.13
        basis = t.v(0, y, 0.86 - 0.03 * i)
        bm = bmesh.new()
        bmesh.ops.create_cone(bm, cap_ends=True, segments=6, radius1=0.035 * t.s, radius2=0.0, depth=(0.17 - 0.012 * i) * t.s)
        for v in bm.verts:
            v.co = Matrix.Rotation(-0.55, 3, "X") @ v.co + basis + Vector((0, 0, 0.07 * t.s))
        t._teil(bm, "Stachel", lambda p, n: KNOCHEN * 0.75, "Koerper")
    for sx in (-1, 1):
        for j in range(2):
            bm = bmesh.new()
            bmesh.ops.create_cone(bm, cap_ends=True, segments=5, radius1=0.012 * t.s, radius2=0.0, depth=0.055 * t.s)
            for v in bm.verts:
                v.co = Matrix.Rotation(math.pi, 3, "X") @ v.co + t.v(sx * (0.028 - 0.01 * j), -1.08 + 0.06 * j, kz - 0.085)
            t._teil(bm, "Zahn", lambda p, n: KNOCHEN, "Kopf")
    stil = dict(schritt=22, schwung=28, knick=45, wippen=0.025, sprung=14, schwung_renn=44, huepfen=0.1, wedeln=0.3)
    original = tiere._animieren

    def mit_angriff(armatur, stil, knochen):
        from werkstatt import animation
        original(armatur, stil, knochen)
        biss = []
        for bild, koerper_x, hoehe, hals, kopf, vorne, hinten in ((0, 0, 0.0, 0, 0, 0, 0), (8, 8, -0.1, -15, 10, 20, 25),
                                                                  (13, -12, 0.12, 10, -25, -55, -35), (17, -8, 0.06, 5, 20, -40, -20),
                                                                  (21, -4, 0.02, 0, -10, -10, 0), (28, 0, 0.0, 0, 0, 0, 0)):
            biss += [tiere._rot(bild, "Koerper", x=koerper_x), tiere._pos(bild, "Koerper", hoehe), tiere._rot(bild, "Hals", x=hals),
                     tiere._rot(bild, "Kopf", x=kopf)]
            for s in ("L", "R"):
                biss += [tiere._rot(bild, f"Bein.V{s}.oben", x=vorne), tiere._rot(bild, f"Bein.V{s}.unten", x=-vorne * 0.3),
                         tiere._rot(bild, f"Bein.H{s}.oben", x=hinten), tiere._rot(bild, f"Bein.H{s}.unten", x=abs(hinten) * 0.5)]
        animation(armatur, "Angriff", 28, biss)

    tiere._animieren = mit_angriff
    try:
        t.fertig(stil)
    finally:
        tiere._animieren = original


# ---------------------------------------------------------------------------
# Spinnen: die kleine Waldspinne und die Spinnenkönigin – modellierter Chitinpanzer, Beine mit
# Gelenken und Borsten, Augenkranz, Beißklauen; Skelett wie bisher (bosse.py)
# ---------------------------------------------------------------------------
def spinne(name, seed, chitin, chitin_hell, zeichnung, augen, groesse=1.0, krone=False, sanduhr=True, ziel=18000, borsten=True):
    from bosse import _boden, _spinne_animationen, _spinne_bein
    f = Figur(name, seed)
    r = f.rng
    formen = [((0, -0.25, 1.45), (0.7, 0.95, 0.48)), ((0, -0.5, 1.6), (0.5, 0.5, 0.3)),         # Vorderkörper
              ((0, -1.1, 1.4), (0.42, 0.45, 0.35)), ((0, -1.35, 1.35), (0.3, 0.2, 0.25)),        # Kopf
              ((0, 0.55, 1.6), (0.35, 0.35, 0.3)),                                              # Stiel
              ((0, 1.55, 1.8), (1.15, 1.45, 1.0)), ((0, 2.2, 1.7), (0.8, 0.8, 0.7))]           # Hinterleib
    for i in range(4):
        for seite in (1, -1):
            ansatz, knie, fuss, aussen = _spinne_bein(i, seite)
            formen += [(ansatz, (0.2, 0.2, 0.2))]
            formen += bh.glied(ansatz, knie, 0.17, 0.12, muskel=0.1)
            formen += [(knie, (0.15, 0.15, 0.15))]
            mitte = knie.lerp(fuss, 0.45)
            formen += bh.glied(knie, mitte, 0.11, 0.08) + bh.glied(mitte, fuss, 0.08, 0.035)
            formen += [(mitte, (0.095, 0.095, 0.095))]
    for s in (1, -1):
        formen += bh.glied(Vector((0.14 * s, -1.4, 1.25)), Vector((0.18 * s, -1.62, 1.0)), 0.09, 0.06) + bh.glied(Vector((0.18 * s, -1.62, 1.0)), Vector((0.08 * s, -1.64, 0.8)), 0.06, 0.02)

    def panzer(p, n):
        platten = 0.02 * weich(0.0, 0.2, bh.zellen(p, 3.5)) if p.y < 0.4 else 0.0
        ringe = 0.015 * abs(math.sin(p.y * 7)) if p.y > 0.6 else 0.0
        return platten + ringe + 0.012 * noise.noise(Vector((p.x * 18, p.y * 18, p.z * 18)))

    def farbe_von(p, n, h):
        c = chitin.lerp(chitin_hell, weich(-0.2, 0.8, n.z) * 0.5) * (0.9 + 0.15 * bh.rausch(p, 6.0))
        if p.y > 0.6 and n.z > 0.35:
            if sanduhr and abs(p.x) < 0.3 - abs(p.y - 1.6) * 0.2:
                c = zeichnung
            elif not sanduhr and math.sin(p.y * 6) > 0.4 and abs(p.x) < 0.7:
                c = zeichnung
        if p.z < 1.3 and abs(p.y) < 3.5 and (abs(p.x) > 0.8 or p.z < 0.6):
            c = c.lerp(zeichnung * 0.6, 0.35 * (math.sin((p - Vector((0, 0, 1.45))).length * 9) > 0.6))       # geringelte Beine
        return bh.schmutz(c, h, 0.55, 0.35)
    k = bh.teil(f, "Chitin", formen, 0.03, ziel, farbe_von, versatz=panzer, knochen=None, inseln=False)
    # Borsten an den Beinen
    for i in range(4 if borsten else 0):
        for seite in (1, -1):
            ansatz, knie, fuss, aussen = _spinne_bein(i, seite)
            for j in range(4):
                p = ansatz.lerp(knie, 0.25 + j * 0.2) + Vector((0, 0, 0.14))
                f.straehne("Borste", [p, p + Vector((0, 0, 0.16)) + aussen * 0.08], 0.018, 0.003, chitin_hell * 1.3, OHNE, 4, 0.0, 1.0)
    # Augenkranz: acht Augen, glänzend
    for j in range(8):
        x = (j % 4 - 1.5) * 0.13
        z = 1.52 + (j // 4) * 0.13 - abs(x) * 0.2
        g = 0.07 if j in (1, 2) else 0.05
        f.kugel("Auge", Vector((x, -1.47 + abs(x) * 0.25, z)), (g, g * 0.7, g), augen, OHNE, 14, 10)
        f.kugel("Glanz", Vector((x + 0.015, -1.47 + abs(x) * 0.25 - g * 0.65, z + g * 0.4)), (g * 0.25, g * 0.1, g * 0.25), farbe("#FFFFFF"), OHNE, 8, 6)
    if krone:
        for j in range(7):
            w = (j - 3) * 0.13
            basis = Vector((w, -1.02, 1.72 - abs(w) * 0.3))
            spitze = basis + Vector((w * 0.4, 0.08, 0.5 - abs(w) * 0.6))
            obj = f.loft("Kronkristall", [(basis, X, Y, 0.05, 0.05), (basis.lerp(spitze, 0.4), X, Y, 0.07, 0.07), (spitze, X, Y, 0.001, 0.001)], 6,
                         lambda i2, kk, p: GLUT, OHNE, oben_zu=True, unten_zu=True)
            bh.glatt_einfaerben(obj, lambda p, n, h: GLUT * (0.9 + 0.6 * max(0.0, n.z)))
        for j in range(10):
            w = r.uniform(-0.9, 0.9)
            basis = Vector((w * 0.8, 1.2 + r.uniform(-0.3, 0.9), 2.6 - abs(w) * 0.4))
            _kegel(f, "Leibdorn", basis, Vector((w * 0.4, 0.3, 1)), 0.5, 0.08, chitin_hell, OHNE, 8)
    # Skelett wie die bisherigen Spinnen
    for i in range(4):
        for seite, sn in ((1, "L"), (-1, "R")):
            ansatz, knie, fuss, aussen = _spinne_bein(i, seite)
            f.knochen_dazu(f"Bein{i}{sn}.oben", ansatz, knie, "Koerper", (0, 0, 1))
            f.knochen_dazu(f"Bein{i}{sn}.unten", knie, fuss, f"Bein{i}{sn}.oben", tuple(aussen))
    f.knochen_dazu("Koerper", (0, 0.5, 1.45), (0, -0.8, 1.45), None, (0, 0, 1))
    f.knochen_dazu("Hinterleib", (0, 0.45, 1.6), (0, 2.9, 1.9), "Koerper", (0, 0, 1))
    f.knochen_dazu("Kopf", (0, -0.8, 1.45), (0, -1.6, 1.35), "Koerper", (0, 0, 1))
    f.knochen = sorted(f.knochen, key=lambda kn: (0 if kn[3] is None else 1 if kn[3] == "Koerper" and not kn[0].startswith("Bein") else 2 if kn[0].endswith(".oben") else 3))
    # Alle Teile nach nächstem Knochen gewichten (Augen und Krone am Kopf)
    for obj in f.teile:
        obj.vertex_groups.clear()
        bh.knochen_gewichte(obj, f.knochen)
    if groesse != 1.0:
        skalieren(f, groesse)
        _boden(f)
    return f.fertig(_spinne_animationen)


def waldspinne(seed=306):
    return spinne("Waldspinne", seed, farbe("#4A3A26"), farbe("#7A6440"), farbe("#C8B83A"), farbe("#1A1410"), groesse=0.24, sanduhr=False, ziel=7000, borsten=False)


def spinnenkoenigin(seed=203):
    return spinne("Spinnenkoenigin", seed, farbe("#241C2C"), farbe("#4A3A60"), VIOLETT_HELL, farbe("#FF3A2A"), krone=True)


# ===========================================================================
# Bosse
# ===========================================================================
def _boden_setzen(f):
    for obj in f.teile + [o for _, _, objs in f.starr for o in objs]:
        for v in obj.data.vertices:
            v.co.z = max(v.co.z, -0.02)


# ---------------------------------------------------------------------------
# Bergtroll: gebeugter Riese, graugrüne warzige Haut mit Flechten und Moos, gewaltiger
# Unterbiss mit Hauern, Knollnase, Felsbrocken auf den Schultern, Fellschurz, Stachelkeule
# ---------------------------------------------------------------------------
def bergtroll(seed=201):
    f = Figur("Bergtroll", seed)
    r = f.rng
    haut, haut_hell, haut_dunkel = farbe("#6E7C5C"), farbe("#96A07A"), farbe("#46503A")
    fell, moos = farbe("#5E4430"), farbe("#4E6A2E")
    b, formen = mensch(f, breite=1.55, dick=1.4, muskel=0.3, kopf=False, fuesse=None, kopf_y=-0.08)
    formen += [((0, 0.08, 1.45), (0.26, 0.18, 0.16)), ((0, -0.06, 1.18), (0.24, 0.2, 0.17))]            # Buckel und Bauch
    for s in (1, -1):
        ks = b.p("knoechel", s)
        formen += [((ks.x, -0.04, 0.05), (0.09, 0.14, 0.055))]
        for j in (-1, 0, 1):
            formen += [((ks.x + 0.04 * j, -0.16, 0.03), (0.028, 0.035, 0.03))]
    k = Vector((0, -0.14, 1.72))
    formen += [
        (k + Vector((0, 0.04, 0.03)), (0.12, 0.13, 0.12)),
        (k + Vector((0, -0.05, -0.07)), (0.14, 0.11, 0.08)),                   # riesiger Unterkiefer
        (k + Vector((0, -0.12, -0.1)), (0.1, 0.05, 0.05)),
        (k + Vector((0, -0.1, 0.04)), (0.12, 0.04, 0.03)),                     # Brauenwulst
        (k + Vector((0, -0.15, -0.005)), (0.045, 0.05, 0.045)),                # Knollnase
        (k + Vector((0, -0.19, -0.03)), (0.04, 0.035, 0.035)),
        (k + Vector((0, -0.14, -0.065)), (0.07, 0.025, 0.008), True),          # Maul
        (k + Vector((0, 0.06, -0.12)), (0.14, 0.1, 0.1)),
    ]
    for s in (1, -1):
        formen += [(k + Vector((0.05 * s, -0.11, 0.015)), (0.024, 0.018, 0.02), True), (k + Vector((0.09 * s, -0.08, -0.03)), (0.045, 0.04, 0.035))]

    def warzen(p, n):
        z = bh.zellen(p, 26.0)
        warze = 0.004 * max(0.0, 0.14 - z) / 0.14 if bh.rausch(p, 4.0) > 0.2 else 0.0
        return warze + 0.0025 * bh.rausch(p, 18.0) + 0.001 * bh.rausch(p, 60.0)

    def haut_farbe(p, n, h):
        c = haut.lerp(haut_hell, weich(-0.2, -0.8, n.y) * 0.3).lerp(haut_dunkel, weich(0.2, 0.9, n.y) * 0.35) * (0.88 + 0.16 * bh.rausch(p, 5.0))
        c = c.lerp(moos, weich(0.6, 0.9, n.z) * weich(1.35, 1.5, p.z) * weich(-0.1, 0.3, bh.rausch(p, 6.0)))
        if bh.zellen(p, 26.0) < 0.04 and bh.rausch(p, 4.0) > 0.2:
            c = haut_dunkel
        if ist_hand(b, p, 0.1):
            c = c * 0.85
        return bh.schmutz(c, h, 0.55, 0.15)
    kk = koerper(f, b, formen, haut_farbe, voxel=0.0085, ziel=16000, versatz=warzen)
    for s in (1, -1):
        b.f.kugel("Glutauge", k + Vector((0.05 * s, -0.115, 0.017)), (0.012, 0.008, 0.01), farbe("#FFB020"), KOPF, 10, 6)
        obj = _kegel(f, "Hauer", k + Vector((0.055 * s, -0.15, -0.085)), Vector((0.2 * s, -0.2, 1)), 0.09, 0.02, KNOCHEN, KOPF, 10, krumm=Vector((0.3 * s, 0.3, 0)))
        bh.glatt_einfaerben(obj, lambda p, n, h: KNOCHEN.lerp(farbe("#7A6A4A"), weich(k.z - 0.02, k.z - 0.1, p.z) * 0.6))
        wurzel = k + Vector((0.11 * s, 0.02, 0.0))
        ringe = [(wurzel + Vector((0.08 * s * t, 0.04 * t, 0.05 * t)), Vector((0, 0.3, -1)).normalized(), Vector((0, 1, 0.3)).normalized(),
                  0.05 * (1 - t) + 0.004, 0.012) for t in (0.0, 0.5, 1.0)]
        obj = f.loft("Ohr", ringe, 14, lambda i, k2, p: haut, KOPF, oben_zu=True, unten_zu=True, teilung=3, glatt=True)
        _veredeln(obj, haut, 30.0, 0.15)
        # Felsbrocken, die auf den Schultern gewachsen sind
        for j in range(3):
            o = b.p("schulter", s) + Vector((0.05 * s + r.uniform(-0.04, 0.04), r.uniform(-0.03, 0.08), 0.1 + 0.02 * j))
            fels = bh.ball_mesh("Schulterfels", [(o, (0.07, 0.06, 0.05))], 0.01)
            bh.modellieren(fels, 0.008, 600, lambda p, n: 0.01 * bh.rausch(p, 12.0))
            bh.einfaerben(fels, lambda p, n, h: bh.schmutz(farbe("#6A6570").lerp(moos, weich(0.5, 0.9, n.z) * 0.8), h, 0.6, 0.3))
            f._gewichten(fels, lambda co, s=s: _mischen(("Brust", 0.5), ("Oberarm.L" if s > 0 else "Oberarm.R", 0.5)))
            bh.aufnehmen(f, fels)
    for j in range(7):
        x = (j - 3) * 0.025
        f.straehne("Haar", [k + Vector((x, 0.0, 0.14)), k + Vector((x * 1.5, 0.1, 0.16)), k + Vector((x * 2, 0.18, 0.06))], 0.02, 0.004, farbe("#2E2A22"), KOPF, 6, 0.3, 1.0, glatt=True)
    obj, _ = rock(f, b, kk, "Fellschurz", 1.02, 0.62, fell, fransen=0.25, weite=0.08, saum=fell * 0.6)
    bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(fell.lerp(farbe("#8A6A4A"), 0.5 + 0.5 * bh.rausch(Vector((p.x * 60, p.y * 60, p.z * 8)), 1.0)), h, 0.5, 0.1))
    guertel(f, b, kk, 1.04, LEDER * 0.6, KNOCHEN, hoehe=0.07)

    def keule():
        g_ = b.griff(-1)
        richtung = Vector((0, -0.35, -0.94)).normalized()
        obj = _strecke(f, "Keulengriff", g_ - richtung * 0.15, g_ + richtung * 0.2, 0.035, 0.04, LEDER, b.hand(-1), 12)
        _veredeln(obj, LEDER, 50.0)
        knuppel = bh.ball_mesh("Keule", bh.glied(g_ + richtung * 0.15, g_ + richtung * 0.95, 0.05, 0.13, muskel=0.15, lage=0.8), 0.012)
        bh.modellieren(knuppel, 0.008, 2000, lambda p, n: 0.006 * noise.noise(Vector((p.x * 30, p.y * 30, p.z * 4))) + 0.004 * bh.rausch(p, 15.0))
        bh.einfaerben(knuppel, lambda p, n, h: bh.schmutz(farbe("#6A4A2A") * (0.75 + 0.4 * bh.rausch(Vector((p.x * 40, p.y * 40, p.z * 5)), 1.0)), h, 0.6, 0.2))
        f._gewichten(knuppel, b.hand(-1))
        bh.aufnehmen(f, knuppel)
        quer = richtung.cross(X).normalized()
        for j in range(12):
            t = 0.5 + (j % 6) * 0.09
            w = math.tau * j / 12 * 3.3
            basis = g_ + richtung * t
            nach = (X * math.cos(w) + quer * math.sin(w)).normalized()
            _kegel(f, "Stachel", basis + nach * 0.07, nach, 0.14, 0.022, STAHL_DUNKEL, b.hand(-1), 8)
    b.starr("Keule", "Hand.R", keule)
    skalieren(f, 2.4)
    _boden_setzen(f)
    return f.fertig(_animationen(_angriff_hieb, arme_ruhe=((-8, -30), (-25, -45)), gehen=(20, 30, 12, 0.05, 16, 20)))


# ---------------------------------------------------------------------------
# Lichkönig: schwebender Totenkönig – Schädel mit Eisaugen und Krone, Knochenhände, reich
# verzierte Robe mit Eisrunen, stachelige Schulterstücke, hoher Kragen, zerfetzter Umhang,
# Knochenstab mit Eiskristall
# ---------------------------------------------------------------------------
def lichkoenig(seed=202):
    f = Figur("Lichkoenig", seed)
    r = f.rng
    robe, robe_hell = farbe("#26263A"), farbe("#4A4A70")
    b, formen = mensch(f, breite=0.95, kopf=False)
    formen = [fo for fo in formen if fo[0][2] > 0.95 or abs(fo[0][0]) > 0.2]
    k = Vector((0, -0.01, 1.76))
    formen += [(k + Vector((0, 0.01, 0.02)), (0.085, 0.1, 0.095)), (k + Vector((0, -0.06, -0.02)), (0.07, 0.05, 0.06)),
               (k + Vector((0, -0.07, -0.075)), (0.05, 0.04, 0.03)), (k + Vector((0, -0.05, -0.11)), (0.055, 0.045, 0.022)),
               (k + Vector((0, -0.1, -0.035)), (0.012, 0.015, 0.018), True)]
    for s in (1, -1):
        formen += [(k + Vector((0.035 * s, -0.085, 0.0)), (0.024, 0.022, 0.024), True), (k + Vector((0.06 * s, -0.06, -0.03)), (0.025, 0.02, 0.018))]

    def farbe_von(p, n, h):
        if ist_kopf(p) or ist_hand(b, p, 0.09):
            return bh.schmutz(KNOCHEN.lerp(farbe("#8A8AA0"), 0.35 + 0.2 * bh.rausch(p, 12.0)), h, 0.8, 0.2)
        return bh.schmutz(robe * (0.85 + 0.2 * bh.rausch(p, 12.0)), h, 0.4, 0.1)
    kk = koerper(f, b, formen, farbe_von)
    for s in (1, -1):
        b.f.kugel("Eisauge", k + Vector((0.035 * s, -0.08, 0.0)), (0.012, 0.008, 0.012), EIS * 2.0, KOPF, 12, 8)
    for j in range(8):
        x = -0.035 + j * 0.01
        for oben in (True, False):
            b.f.kiste("Zahn", (x, -0.095 + abs(x) * 0.4, k.z - 0.058 if oben else k.z - 0.1), (0.008, 0.008, 0.016), KNOCHEN, KOPF)
    # Krone
    ringe = [(k + Vector((0, 0.01, 0.07)), X, Y, 0.1, 0.11), (k + Vector((0, 0.01, 0.1)), X, Y, 0.102, 0.112)]
    f.loft("Kronreif", ringe, 32, lambda i, kk_, p: GOLD, KOPF, glatt=True)
    for j in range(9):
        w = math.tau * j / 9
        basis = k + Vector((math.cos(w) * 0.1, 0.01 + math.sin(w) * 0.11, 0.1))
        _kegel(f, "Kronzacke", basis, Vector((math.cos(w) * 0.2, math.sin(w) * 0.2, 1)), 0.1 + 0.05 * (j % 2), 0.018, GOLD, KOPF, 6)
        if j % 2 == 0:
            b.f.kugel("Kronstein", basis + Vector((math.cos(w) * 0.004, math.sin(w) * 0.004, 0.02)), (0.012, 0.012, 0.012), EIS * 1.5, KOPF, 8, 6)
    # Robe mit Eisrunen am Saum, weite Ärmel, hoher Kragen, Schulterdornen
    rock(f, b, kk, "Robe", 1.02, 0.02, robe, fransen=0.35, weite=0.2, falten=0.1, saum=robe_hell)
    my, rx, ry = rumpf_breite(kk, 1.02)
    for j in range(10):
        w = math.tau * j / 10
        f.stern("Eisrune", Vector((math.sin(w) * (rx + 0.17), my - math.cos(w) * (ry + 0.13), 0.28)), Vector((math.sin(w), -math.cos(w), 0)), 0.04, EIS * 1.4, b.rock, zacken=4)
    f.loft("Schaerpe", [(Vector((0, my, 1.0)), X, Y, rx + 0.02, ry + 0.02), (Vector((0, my, 1.07)), X, Y, rx + 0.022, ry + 0.022)], 36, lambda i, kk_, p: EIS * 0.6,
           b.rumpf, glatt=True)
    huelle(f, b, kk, "Brustpanzer", lambda p, n: 1.1 < p.z < 1.54 and abs(p.x) < 0.2 and not ist_hand(b, p), 0.02, STAHL_SCHWARZ,
           rand=lambda p, n, c: EIS if (p - Vector((0, -0.18, 1.35))).length < 0.03 and p.y < 0 else c, ziel=1200)
    for s in (1, -1):
        aermel(f, b, s, robe, saum=robe_hell, weite=0.13, fetzen=0.3)
        sl = b.p("schulter", s)
        obj = _schale(f, "Schulterstueck", sl + Vector((0.02 * s, 0.0, 0.05)), 0.14, 0.14, 0.1, (0, 360), (0, 80), lambda poly: STAHL_SCHWARZ,
                      lambda co, s=s: _mischen(("Brust", 0.4), ("Oberarm.L" if s > 0 else "Oberarm.R", 0.6)), seg=(28, 8))
        bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(STAHL_SCHWARZ, h, 0.6, 0.6))
        for j in range(3):
            _kegel(f, "Schulterdorn", sl + Vector((0.07 * s, (j - 1) * 0.06, 0.1)), Vector((s * (0.35 + j * 0.1), (j - 1) * 0.3, 1)), 0.22 + 0.04 * j, 0.03,
                   STAHL_DUNKEL, lambda co, s=s: _mischen(("Brust", 0.4), ("Oberarm.L" if s > 0 else "Oberarm.R", 0.6)), 8)
    obj = f.loft("Kragen", [(Vector((0, 0.03, 1.5)), X, Y, 0.21, 0.16), (Vector((0, 0.08, 1.75)), X, Y, 0.22, 0.15, _zacken(r, 0.1, 9)),
                            (Vector((0, 0.13, 1.98)), X, Y, 0.19, 0.1, _zacken(r, 0.25, 9))], 36, lambda i, kk_, p: robe_hell, b.rumpf, teilung=3, glatt=True)
    bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz((robe_hell if n.y > 0 else robe) * (0.85 + 0.2 * bh.rausch(p, 15.0)), h, 0.4, 0.2))
    umhang(f, b, robe * 0.9, laenge=1.45, weite=1.2, fetzen=0.12, rand=farbe("#0E0E18"))

    def stab():
        g_ = b.griff(-1)
        unten, oben = Vector((g_.x, g_.y, 0.0)), Vector((g_.x, g_.y, 2.2))
        punkte = [unten.lerp(oben, t) + Vector((0.01 * math.sin(t * 11), 0.008 * math.cos(t * 7), 0)) for t in (0.0, 0.25, 0.5, 0.75, 1.0)]
        obj = f.straehne("Stab", punkte, 0.026, 0.02, farbe("#9A9280"), b.hand(-1), 10, 0.0, 1.0, teilung=4, glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(farbe("#9A9280") * (0.7 + 0.4 * bh.rausch(Vector((p.x * 40, p.y * 40, p.z * 6)), 1.0)), h, 0.5, 0.2))
        schaedel = bh.ball_mesh("Stabschaedel", [(oben + Z * 0.06, (0.07, 0.075, 0.08)), (oben + Vector((0, -0.05, 0.02)), (0.05, 0.04, 0.04)),
                                                  (oben + Vector((0.03, -0.07, 0.07)), (0.018, 0.015, 0.018), True),
                                                  (oben + Vector((-0.03, -0.07, 0.07)), (0.018, 0.015, 0.018), True)], 0.008)
        bh.modellieren(schaedel, 0.006, 1200)
        bh.einfaerben(schaedel, lambda p, n, h: bh.schmutz(KNOCHEN, h, 0.8, 0.2))
        f._gewichten(schaedel, b.hand(-1))
        bh.aufnehmen(f, schaedel)
        for j in range(3):
            w = math.tau * j / 3
            a = oben + Z * 0.14 + Vector((math.cos(w) * 0.03, math.sin(w) * 0.03, 0))
            e = a + Vector((math.cos(w) * 0.04, math.sin(w) * 0.04, 0.35 - 0.08 * j))
            obj = f.loft("Eiskristall", [(a, X, Y, 0.03, 0.03), (a.lerp(e, 0.5), X, Y, 0.045, 0.045), (e, X, Y, 0.001, 0.001)], 6, lambda i2, kk_, p: EIS,
                         b.hand(-1), oben_zu=True, unten_zu=True)
            bh.glatt_einfaerben(obj, lambda p, n, h: EIS * (1.0 + 0.8 * max(0.0, n.z)))
    b.starr("Stab", "Hand.R", stab)
    skalieren(f, 2.1)
    return f.fertig(_animationen(_angriff_zauber, arme_ruhe=((-8, -25), (0, -10)), schweben=True))


# ---------------------------------------------------------------------------
# Dämonenfürst: muskulöser Leib aus rotem, rissigem Fleisch mit glühenden Adern, schwarze
# Widderhörner, Fledermausflügel mit Flughaut, Bocksbeine mit Hufen, Schwanz, Flammenschwert
# ---------------------------------------------------------------------------
def daemonenfuerst(seed=204):
    f = Figur("Daemonenfuerst", seed)
    rot, rot_dunkel, schwarz = farbe("#8A2A22"), farbe("#3E0E0C"), farbe("#1A1214")
    b, formen = mensch(f, breite=1.35, dick=1.2, muskel=0.35, kopf=False, fuesse=None)
    for s in (1, -1):
        formen += [((0.13 * s, -0.12, 1.4), (0.12, 0.06, 0.09)), ((0.17 * s, 0.03, 1.47), (0.1, 0.09, 0.07))]
        for j in range(3):
            formen.append(((0.045 * s, -0.14, 1.25 - 0.065 * j), (0.04, 0.025, 0.03)))
        ks = b.p("knoechel", s)
        formen += [((ks.x, ks.y - 0.02, 0.06), (0.06, 0.07, 0.06))]
    formen += bh.glied(Vector((0, 0.12, 0.98)), Vector((0, 0.45, 0.7)), 0.06, 0.045) + bh.glied(Vector((0, 0.45, 0.7)), Vector((0.1, 0.72, 0.35)), 0.045, 0.03) \
        + bh.glied(Vector((0.1, 0.72, 0.35)), Vector((0.2, 0.86, 0.2)), 0.03, 0.018)
    k = Vector((0, -0.02, 1.74))
    formen += [(k + Vector((0, 0.01, 0.02)), (0.095, 0.11, 0.11)), (k + Vector((0, -0.05, -0.06)), (0.08, 0.07, 0.06)),
               (k + Vector((0, -0.09, 0.03)), (0.085, 0.03, 0.025)), (k + Vector((0, -0.11, -0.01)), (0.022, 0.03, 0.025)),
               (k + Vector((0, -0.08, -0.1)), (0.04, 0.035, 0.03)), (k + Vector((0, -0.1, -0.055)), (0.04, 0.012, 0.005), True)]
    for s in (1, -1):
        formen += [(k + Vector((0.04 * s, -0.09, 0.005)), (0.02, 0.014, 0.014), True), (k + Vector((0.06 * s, -0.07, -0.03)), (0.03, 0.025, 0.02))]

    def rissig(p, n):
        z = bh.zellen(p, 9.0)
        return -0.004 * weich(0.05, 0.0, z) + 0.0015 * bh.rausch(p, 30.0)

    def farbe_von(p, n, h):
        c = rot.lerp(rot_dunkel, weich(0.1, 0.8, n.y) * 0.5 + weich(1.0, 0.5, p.z) * 0.4) * (0.88 + 0.16 * bh.rausch(p, 7.0))
        if p.z < 0.2 and abs(p.x) < 0.25:
            c = schwarz
        z = bh.zellen(p, 9.0)
        if z < 0.03 and not ist_kopf(p) and bh.rausch(p + Vector((2, 2, 2)), 3.0) > 0.1:
            c = FEUER * (1.4 + 0.5 * bh.rausch(p, 30.0))
        return bh.schmutz(c, h, 0.5, 0.15)
    kk = koerper(f, b, formen, farbe_von, voxel=0.0075, ziel=15000, versatz=rissig)
    for s in (1, -1):
        b.f.kugel("Feuerauge", k + Vector((0.04 * s, -0.085, 0.005)), (0.014, 0.009, 0.01), FEUER_HELL * 2.0, KOPF, 10, 6)
        punkte = [k + Vector((0.07 * s, -0.02, 0.1)), k + Vector((0.17 * s, 0.02, 0.21)), k + Vector((0.26 * s, 0.12, 0.21)), k + Vector((0.27 * s, 0.16, 0.08)),
                  k + Vector((0.22 * s, 0.08, 0.0))]
        obj = f.straehne("Horn", punkte, 0.065, 0.008, schwarz, KOPF, 14, 0.0, 1.0, teilung=4, glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(schwarz * (0.7 + 0.8 * max(0.0, math.sin(((p - k).length) * 260))), h, 0.5, 0.3))
    for j in range(6):
        x = (j - 2.5) * 0.013
        for oben in (True, False):
            _kegel(f, "Zahn", k + Vector((x, -0.108, -0.05 if oben else -0.062)), Vector((0, -0.2, -1 if oben else 1)), 0.022, 0.006, KNOCHEN, KOPF, 5)
    # Fledermausflügel: Knochenfinger und Flughaut, am Rücken
    for s in (1, -1):
        wurzel = Vector((s * 0.1, 0.15, 1.45))
        finger = [wurzel + Vector((s * 0.9, 0.35, 0.75)), wurzel + Vector((s * 1.1, 0.4, 0.2)), wurzel + Vector((s * 0.95, 0.35, -0.35)),
                  wurzel + Vector((s * 0.55, 0.25, -0.65))]
        brust = lambda co: {"Brust": 1.0}
        for spitze in finger:
            f.straehne("Fluegelfinger", [wurzel, wurzel.lerp(spitze, 0.5) + Vector((0, 0.05, 0.05)), spitze], 0.035, 0.008, schwarz, brust, 8, 0.0, 1.0, teilung=3, glatt=True)
        bm = bmesh.new()
        punkte = [bm.verts.new(wurzel)]
        for a, e in zip(finger, finger[1:]):
            for t in (0.0, 0.33, 0.66):
                mitte = a.lerp(e, t) + (wurzel - a.lerp(e, t)) * 0.12 * math.sin(math.pi * t)
                punkte.append(bm.verts.new(mitte + Vector((0, 0.01, 0))))
        punkte.append(bm.verts.new(finger[-1]))
        for a, e in zip(punkte[1:], punkte[2:]):
            bm.faces.new((punkte[0], a, e))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        from bosse import _doppelseitig
        _doppelseitig(bm)
        obj = f._objekt(bm, "Flughaut", lambda poly: rot_dunkel, brust)
        bh.glatt_einfaerben(obj, lambda p, n, h: rot_dunkel.lerp(rot, weich(0.3, 0.0, (p - wurzel).length) * 0.4) * (0.8 + 0.25 * bh.rausch(p, 10.0)))
    _kegel(f, "Schwanzspitze", Vector((0.2, 0.86, 0.2)), Vector((0.5, 0.8, -0.3)), 0.14, 0.05, schwarz, lambda co: {"Becken": 1.0}, 6)
    rock(f, b, kk, "Schurz", 1.02, 0.62, schwarz, fransen=0.3, weite=0.05, saum=rot_dunkel)
    guertel(f, b, kk, 1.04, GOLD * 0.8, farbe("#FF4A1A"), hoehe=0.07)

    def flammenschwert():
        g_ = b.griff(-1)
        richtung = Vector((0, -0.62, -0.78)).normalized()
        _strecke(f, "Schwertgriff", g_ - richtung * 0.14, g_ + richtung * 0.06, 0.02, 0.02, schwarz, b.hand(-1), 10)
        obj = f.loft("Parier", [(g_ + richtung * 0.08 - X * 0.14, X, richtung, 0.02, 0.03), (g_ + richtung * 0.08, X, richtung, 0.03, 0.035),
                                (g_ + richtung * 0.08 + X * 0.14, X, richtung, 0.02, 0.03)], 10, lambda i, kk_, p: GOLD, b.hand(-1), oben_zu=True, unten_zu=True, teilung=3, glatt=True)
        ringe = []
        for j in range(12):
            t = j / 11
            w = 0.055 * (1 + 0.35 * math.sin(t * 14)) * (1 - weich(0.75, 1.0, t))
            ringe.append((g_ + richtung * (0.1 + 1.05 * t), X, richtung.cross(X).normalized(), w + 0.002, 0.012))
        obj = f.loft("Flammenklinge", ringe, 8, lambda i, kk_, p: FEUER, b.hand(-1), oben_zu=True, unten_zu=True, teilung=2)
        bh.glatt_einfaerben(obj, lambda p, n, h: FEUER.lerp(FEUER_HELL * 1.6, weich(0.02, 0.0, abs(p.x - g_.x))) * 1.3)
    b.starr("Flammenschwert", "Hand.R", flammenschwert)
    skalieren(f, 2.3)
    _boden_setzen(f)
    return f.fertig(_animationen(_angriff_hieb, arme_ruhe=((-15, -40), (-10, -25)), gehen=(24, 34, 14, 0.04, 6, 14)))


# ---------------------------------------------------------------------------
# Schattendrache: Körper modelliert mit Schuppen und Bauchschilden, sonst wie bisher (bosse.py)
# ---------------------------------------------------------------------------
def schattendrache(seed=205):
    import bosse
    import tiere

    def schuppen(p, n):
        z = bh.zellen(p, 9.0)
        bauch = weich(-0.3, -0.7, n.z)
        return 0.02 * weich(0.0, 0.2, z) * (1 - bauch) + 0.012 * bauch * abs(math.sin(p.y * 10)) + 0.01 * bh.rausch(p, 6.0)
    alt = tiere.Tier.koerper

    def koerper_neu(self, zonen, **kw):
        def zonen_mit_fugen(p, n):
            c = zonen(p, n)
            return c.lerp(c * 0.5, weich(0.06, 0.0, bh.zellen(p, 9.0)) * 0.6)
        return _modellierter_tierkoerper(schuppen, voxel=0.045, ziel=14000)(self, zonen_mit_fugen)
    tiere.Tier.koerper = koerper_neu
    try:
        return bosse.schattendrache(seed)
    finally:
        tiere.Tier.koerper = alt


# ---------------------------------------------------------------------------
# Gargoyle (ersetzt die Harpyie): geduckter Flugdämon aus verwittertem Stein mit Rissen und
# Moos, gebogene Hörner, spitze Ohren, Fratze mit Reißzähnen, glühende Augen; Fledermaus-
# schwingen spannen sich von Arm und Hand bis zur Hüfte (schlagen mit den Armen), Krallenfüße,
# Schwanz mit Pfeilspitze
# ---------------------------------------------------------------------------
def gargoyle(seed=109):
    from bosse import _doppelseitig
    from einheiten import _harpyie_animationen
    f = Figur("Gargoyle", seed)
    r = f.rng
    stein, stein_hell, stein_dunkel = farbe("#6E6A74"), farbe("#9A96A0"), farbe("#35323C")
    moos = farbe("#4E6A34")
    b, formen = mensch(f, breite=1.05, dick=1.05, muskel=0.3, kopf=False, fuesse=None, kopf_y=-0.08,
                       hand=(0.36, -0.03, 0.99), finger=(0.39, -0.08, 0.9))
    # breiter Brustkorb, Buckel zwischen den Schultern
    formen += [((0, 0.08, 1.46), (0.2, 0.12, 0.12)), ((0, -0.1, 1.38), (0.17, 0.06, 0.1))]
    for s in (1, -1):
        ks = b.p("knoechel", s)
        # Krallenfüße: drei lange Zehen nach vorne, eine nach hinten
        for j in (-1, 0, 1):
            formen += bh.glied(Vector((ks.x, 0.0, 0.05)), Vector((ks.x + 0.045 * j, -0.15, 0.025)), 0.022, 0.014)
        formen += bh.glied(Vector((ks.x, 0.0, 0.05)), Vector((ks.x, 0.08, 0.02)), 0.018, 0.012)
    # Schwanz
    schwanz = [Vector((0, 0.1, 0.96)), Vector((0, 0.38, 0.72)), Vector((0.08, 0.62, 0.42)), Vector((0.18, 0.78, 0.22))]
    for a, e, ra, rb in zip(schwanz, schwanz[1:], (0.06, 0.045, 0.032), (0.045, 0.032, 0.02)):
        formen += bh.glied(a, e, ra, rb)
    # Fratze: wulstige Stirn, breite Schnauze, Kiefer, Höhlen
    k = Vector((0, -0.08, 1.76))
    formen += [(k + Vector((0, 0.03, 0.02)), (0.1, 0.11, 0.1)), (k + Vector((0, -0.07, -0.03)), (0.075, 0.07, 0.055)),
               (k + Vector((0, -0.12, -0.02)), (0.05, 0.05, 0.04)), (k + Vector((0, -0.07, -0.085)), (0.06, 0.05, 0.03)),
               (k + Vector((0, -0.095, 0.035)), (0.09, 0.04, 0.03)), (k + Vector((0, -0.13, -0.05)), (0.05, 0.03, 0.008), True),
               (k + Vector((0.02, -0.165, -0.01)), (0.008, 0.008, 0.007), True), (k + Vector((-0.02, -0.165, -0.01)), (0.008, 0.008, 0.007), True)]
    for s in (1, -1):
        formen += [(k + Vector((0.042 * s, -0.1, 0.012)), (0.022, 0.016, 0.017), True), (k + Vector((0.07 * s, -0.07, -0.03)), (0.035, 0.03, 0.025))]

    def verwittert(p, n):
        z = bh.zellen(p, 11.0)
        return -0.005 * weich(0.05, 0.0, z) + 0.004 * bh.rausch(p, 14.0) + 0.0015 * bh.rausch(p, 50.0)

    def stein_farbe(p, n, h):
        c = stein.lerp(stein_dunkel, 0.25 + 0.3 * bh.rausch(p, 5.0)).lerp(stein_hell, weich(0.3, 0.9, n.z) * 0.3)
        c = c.lerp(moos, weich(0.55, 0.9, n.z) * weich(0.0, 0.4, bh.rausch(p, 4.0)) * 0.8)
        c = c.lerp(stein_dunkel * 0.5, weich(0.05, 0.0, bh.zellen(p, 11.0)) * 0.7)
        return bh.schmutz(c, h, 0.65, 0.3)
    kk = koerper(f, b, formen, stein_farbe, voxel=0.0065, ziel=13000, versatz=verwittert)
    # Der Schwanz hängt am Becken (sonst zerren ihn die Beine mit)
    becken_g = kk.vertex_groups.get("Becken")
    for v in kk.data.vertices:
        if v.co.y > 0.16 and 0.15 < v.co.z < 0.98:
            for e in list(v.groups):
                kk.vertex_groups[e.group].remove([v.index])
            becken_g.add([v.index], 1.0, "REPLACE")
    for s in (1, -1):
        b.f.kugel("Glutauge", k + Vector((0.042 * s, -0.112, 0.012)), (0.014, 0.009, 0.011), farbe("#FF8A2A") * 2.0, KOPF, 12, 8)
        punkte = [k + Vector((0.06 * s, 0.0, 0.1)), k + Vector((0.12 * s, 0.06, 0.2)), k + Vector((0.13 * s, 0.16, 0.24)), k + Vector((0.1 * s, 0.24, 0.18))]
        obj = f.straehne("Horn", punkte, 0.035, 0.004, stein_dunkel, KOPF, 12, 0.0, 1.0, teilung=4, glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(stein_dunkel * (0.7 + 0.6 * max(0.0, math.sin((p - k).length * 240))), h, 0.5, 0.3))
        wurzel = k + Vector((0.095 * s, 0.02, 0.01))
        ringe = [(wurzel + Vector((0.09 * s * t, 0.05 * t, 0.07 * t)), Vector((0, 0.3, -1)).normalized(), Vector((0, 1, 0.3)).normalized(),
                  0.045 * (1 - t) + 0.003, 0.01) for t in (0.0, 0.5, 1.0)]
        obj = f.loft("Ohr", ringe, 14, lambda i, kk_, p: stein, KOPF, oben_zu=True, unten_zu=True, teilung=3, glatt=True)
        bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(stein, h, 0.5, 0.3))
        for j, x in enumerate((0.022, 0.04)):
            _kegel(f, "Reisszahn", k + Vector((x * s, -0.14 + 0.015 * j, -0.045)), Vector((0, -0.2, -1)), 0.035 - 0.01 * j, 0.008, KNOCHEN * 0.9, KOPF, 6)
            _kegel(f, "Reisszahn", k + Vector((x * s, -0.135 + 0.015 * j, -0.06)), Vector((0, -0.2, 1)), 0.025, 0.007, KNOCHEN * 0.9, KOPF, 6)
        # Krallen an Händen und Füßen
        g_ = b.griff(s)
        for j in range(4):
            a = g_ + Vector((0.01 * s, -0.035, 0.02 - 0.02 * j))
            f.straehne("Kralle", [a, a + Vector((0, -0.03, -0.01)), a + Vector((0, -0.035, -0.035))], 0.008, 0.001, stein_dunkel * 0.6, b.hand(s), 6, 0.0, 1.0, glatt=True)
        ks = b.p("knoechel", s)
        for j in (-1, 0, 1):
            sp = Vector((ks.x + 0.045 * j, -0.16, 0.025))
            f.straehne("Kralle", [sp, sp + Vector((0.008 * j, -0.035, 0.0)), sp + Vector((0.012 * j, -0.045, -0.025))], 0.012, 0.002, stein_dunkel * 0.6, b.fuss(s), 6, 0.0, 1.0, glatt=True)
    # Rückenkamm und Schwanzspitze
    for j in range(6):
        basis = bh.auf_haut(kk, Vector((0, 0.2, 1.62 - j * 0.1)), -0.005)
        _kegel(f, "Rueckenzacke", basis, Vector((0, 1, 0.6)), 0.07 - 0.006 * j, 0.022, stein_dunkel, b.rumpf, 6)
    obj = f.loft("Schwanzspitze", [(schwanz[-1], X, Z, 0.02, 0.012), (schwanz[-1] + Vector((0.04, 0.05, -0.03)), X, Z, 0.07, 0.012),
                                   (schwanz[-1] + Vector((0.1, 0.13, -0.08)), X, Z, 0.001, 0.001)], 6, lambda i, kk_, p: stein_dunkel,
                 lambda co: {"Becken": 1.0}, oben_zu=True, unten_zu=True, teilung=2)
    # Schwingen: Knochenfinger aus der Hand, Flughaut bis zur Hüfte
    for s in (1, -1):
        sh, el, ha = b.p("schulter", s), b.p("ellbogen", s), b.p("hand", s)
        huefte = Vector((0.16 * s, 0.05, 1.0))
        finger = [ha + Vector((0.28 * s, 0.1, 0.5)), ha + Vector((0.42 * s, 0.2, 0.15)), ha + Vector((0.38 * s, 0.28, -0.22)), ha + Vector((0.2 * s, 0.25, -0.42))]
        hand_g = lambda co, s=s: {"Hand.L" if s > 0 else "Hand.R": 1.0}
        for spitze in finger:
            obj = f.straehne("Fluegelfinger", [ha, ha.lerp(spitze, 0.5) + Vector((0, 0.04, 0.03)), spitze], 0.02, 0.006, stein_dunkel, hand_g, 8, 0.0, 1.0,
                             teilung=3, glatt=True)
            bh.glatt_einfaerben(obj, lambda p, n, h: bh.schmutz(stein_dunkel, h, 0.5, 0.3))
        # Flughaut als Fächer: Schulter → Ellbogen → Hand → Fingerspitzen → Hüfte
        rand = [sh + Vector((0, 0.05, 0.03)), el + Vector((0, 0.04, 0.02)), ha]
        for a, e in zip([ha] + finger, finger):
            pass
        rand += finger + [huefte]
        mitte = (sh + huefte + ha) / 3 + Vector((0, 0.1, 0))
        bm = bmesh.new()
        vm = bm.verts.new(mitte)
        kette = []
        for a, e in zip(rand, rand[1:]):
            for t in (0.0, 0.34, 0.67):
                p = a.lerp(e, t)
                durchhang = 0.1 * math.sin(math.pi * t) if a in finger and e in finger + [huefte] else 0.0
                kette.append(bm.verts.new(p + (mitte - p).normalized() * durchhang))
        kette.append(bm.verts.new(rand[-1]))
        for a, e in zip(kette, kette[1:]):
            bm.faces.new((vm, a, e))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        _doppelseitig(bm, 0.008)
        obj = f._objekt(bm, "Flughaut", lambda poly: stein_dunkel, OHNE)
        obj.vertex_groups.clear()
        bh.knochen_gewichte(obj, [kn for kn in f.knochen if kn[0] in ("Brust", "Bauch", "Becken", f"Oberarm.{'L' if s > 0 else 'R'}",
                                                                  f"Unterarm.{'L' if s > 0 else 'R'}", f"Hand.{'L' if s > 0 else 'R'}")], schaerfe=3.0)
        bh.glatt_einfaerben(obj, lambda p, n, h, m=mitte: farbe("#4A3E4E").lerp(farbe("#7A5A6A"), weich(0.4, 0.0, (p - m).length) * 0.4) *
                            (0.8 + 0.25 * bh.rausch(p, 14.0)))
    return f.fertig(_harpyie_animationen)
