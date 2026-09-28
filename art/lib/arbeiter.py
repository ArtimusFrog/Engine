"""Arbeiter der Rohstoffgebäude (NPC) – Holzarbeiter, Steinarbeiter, Erzarbeiter.

Gebaut mit dem Figuren-Baukasten (figuren.py), kräftige Männer um 1,80 m:
- Holzarbeiter: rot-schwarz kariertes Flanellhemd mit hochgekrempelten Ärmeln, Hosenträger,
  olivgrüne Arbeitshose, Schnürstiefel, senfgelbe Strickmütze, voller brauner Bart; lange
  Fälleraxt mit rotem Kopf. Trägt drei Stämme auf der linken Schulter.
- Steinarbeiter: Leinenhemd, lange Lederschürze mit Meißeln in der Tasche, rotes Halstuch,
  graue Schiebermütze, Schnauzbart; schwerer Steinhammer. Trägt einen behauenen Block vor dem Bauch.
- Erzarbeiter: dunkelblauer Bergkittel mit Messingknöpfen, Arschleder, Knieleder, Messinghelm
  mit Grubenlampe, rußiges Gesicht, Kiepe auf dem Rücken; Keilhaue. Trägt Erz in der Kiepe.

Anbauteile (im Spiel ein- und ausblendbar): „Werkzeug“ (rechte Hand), „Werkzeug_Ruecken“ (auf
dem Rücken, beim Tragen) und „Last“ (die Ladung).
Animationen: Idle, Laufen, Arbeiten (Schlag in Schleife), Buecken (Aufheben/Abladen), Tragen.
Beim Arbeiten und Tragen greift die linke Hand (beim Stein beide) per IK an den Stiel bzw. die
Last; die IK wird in die Clips gebacken.

Koordinaten wie bei allen Figuren: Z oben, Blick nach -Y, Füße im Ursprung, links (L) = +X.
"""

import math

import bmesh
import bpy
from mathutils import Matrix, Quaternion, Vector

from figuren import HAENGT, HOCH, Figur, _achsen, _clip, _gehen, _mischen, _mit, _platte, _schlauch, _schleife, farbe, weich
from werkstatt import animation

X, Y, Z = Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))

# Gelenke (Meter). L = +X, R = -X, vorne = -Y.
SCHULTER = Vector((0.19, 0.0, 1.47))
ELLBOGEN = Vector((0.27, 0.02, 1.2))
HANDGELENK = Vector((0.315, -0.01, 0.96))
FINGER = Vector((0.335, -0.03, 0.87))
HUEFTE = Vector((0.1, 0.0, 0.93))
KNIE = Vector((0.1, -0.005, 0.5))
KNOECHEL = Vector((0.1, 0.02, 0.09))
ZEHEN = Vector((0.1, -0.14, 0.025))
# Gesicht wie beim Magier, 5,2 cm tiefer
DZ = -0.052
AUGE = Vector((0.038, -0.094, 1.752 + DZ))
KOPF = Vector((0.0, 0.005, 1.75 + DZ))
# Rechte Faust um den senkrechten Werkzeugstiel, linke Faust (Griff quer, entlang Y)
GRIFF_R = Vector((-0.325, -0.06, 0.915))
GRIFF_L = Vector((0.33, -0.045, 0.905))
# Werkzeug: Stiel vom Griff (oben) bis zum Kopf (unten)
STIEL_OBEN, KOPF_Z = 0.975, 0.16


def _spiegel(p, seite):
    return Vector((p.x * seite, p.y, p.z))


def _rumpf(co):
    z = co.z
    return _mischen(("Becken", 1 - weich(1.0, 1.13, z)), ("Bauch", weich(1.0, 1.13, z) * (1 - weich(1.26, 1.36, z))),
                    ("Brust", weich(1.26, 1.36, z) * (1 - weich(1.52, 1.57, z))), ("Hals", weich(1.52, 1.57, z)))


def _huefte_bein(co):
    if co.z > 0.95:
        return _rumpf(co)
    bein = weich(0.95, 0.78, co.z)
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


KOPF_GEWICHT = lambda co: {"Kopf": 1.0}
BRUST_GEWICHT = lambda co: {"Brust": 1.0}


def _kopf_und_hals(co):
    return _mischen(("Kopf", weich(1.55, 1.6, co.z)), ("Hals", 1 - weich(1.55, 1.6, co.z)))


def _faust(griff, achse, seite, dicke):
    """Faust aus Metaball-Formen, die sich um einen Stiel entlang `achse` schließt (Handschuh)."""
    achse = achse.normalized()
    q1 = achse.cross(Z if abs(achse.z) < 0.9 else X).normalized()
    q2 = achse.cross(q1).normalized()
    formen = [(griff + q1 * 0.028 * seite + q2 * 0.008, (0.03, 0.03, 0.03))]
    for i in range(4):
        versatz = achse * (0.021 - 0.014 * i)
        punkte = []
        for w in (0.3, -0.6, -1.5, -2.4, -3.1):
            punkte.append(griff + versatz + (q1 * math.cos(w) * seite + q2 * math.sin(w)) * 0.029)
        for a, b in zip(punkte, punkte[1:]):
            for t in (0.0, 0.33, 0.66):
                formen.append((a.lerp(b, t), (dicke,) * 3))
        dicke *= 0.96
    for t in (0.0, 0.5, 1.0):
        formen.append((griff + achse * 0.034 + q1 * (0.02 - 0.034 * t) * seite - q2 * 0.024, (dicke * 1.05,) * 3))
    return formen


# ---------------------------------------------------------------------------
# Werkzeuge (in Ruhelage: Stiel senkrecht in der rechten Faust, Kopf unten). `m` legt eine Kopie
# woanders hin (auf den Rücken).
# ---------------------------------------------------------------------------
def _stiel(f, m, gewicht, holz, holz_dunkel, leder, x, y):
    mr = m.to_3x3()
    ringe = []
    for i in range(18):
        t = i / 17
        z = STIEL_OBEN + (KOPF_Z - 0.05 - STIEL_OBEN) * t
        r = 0.019 + 0.004 * math.sin(t * math.pi) + (0.006 if t < 0.03 else 0.0)
        ringe.append((m @ Vector((x, y, z)), mr @ X, mr @ Y, r, r * 0.9))

    def farbe_von(i, k, p):
        if i > 0.8 and i < 3.2:
            return leder * (0.8 if int(i * 4) % 2 else 1.0)                    # Lederwicklung am Griff
        return (holz if (k + int(i * 2)) % 6 else holz_dunkel) * (0.92 + 0.08 * math.sin(i * 2.3))
    f.loft("Stiel", ringe, 10, farbe_von, gewicht, oben_zu=True, unten_zu=True, teilung=1, glatt=True)


def _profilkopf(f, name, m, gewicht, mitte, richtung, profil, farbe_von, hoch=Z):
    """Werkzeugkopf aus Querschnitten entlang `richtung` (Abstand, halbe Dicke, halbe Höhe, Versatz in Z)."""
    mr = m.to_3x3()
    quer = richtung.cross(hoch).normalized()
    ringe = [(m @ (mitte + richtung * d + hoch * dz), mr @ quer, mr @ hoch, dicke, hoehe) for d, dicke, hoehe, dz in profil]
    f.loft(name, ringe, 8, farbe_von, gewicht, oben_zu=True, unten_zu=True, teilung=2, glatt=False)


def faelleraxt(f, m, gewicht):
    holz, holz_dunkel, leder = farbe("#B07A44"), farbe("#80542C"), farbe("#3A2616")
    rot, rot_dunkel, stahl = farbe("#C23A2A"), farbe("#8A2418"), farbe("#D8DEE6")
    x, y = GRIFF_R.x, GRIFF_R.y
    _stiel(f, m, gewicht, holz, holz_dunkel, leder, x, y)
    auge = Vector((x, y, KOPF_Z + 0.02))
    # Schneide schräg nach vorne innen: führt beim schrägen Schlag von rechts oben
    richtung = Vector((0.55, -0.83, 0)).normalized()
    profil = [(-0.06, 0.022, 0.034, 0.0), (-0.03, 0.028, 0.044, 0.0), (0.0, 0.03, 0.046, 0.0), (0.035, 0.024, 0.04, -0.004),
              (0.075, 0.016, 0.046, -0.01), (0.11, 0.011, 0.068, -0.016), (0.145, 0.007, 0.09, -0.02), (0.175, 0.004, 0.104, -0.022),
              (0.192, 0.0015, 0.108, -0.022)]

    def kopf_farbe(i, k, p):
        return stahl if i > 6.6 else (rot if k % 4 else rot_dunkel)
    _profilkopf(f, "Axtkopf", m, gewicht, auge, richtung, profil, kopf_farbe)
    mr = m.to_3x3()
    f.loft("Axtring", [(m @ (auge + Z * dz), mr @ X, mr @ Y, 0.026, 0.028) for dz in (0.055, 0.075)], 10, lambda i, k, p: farbe("#3B3F47"), gewicht,
           oben_zu=True, unten_zu=True)


def keilhaue(f, m, gewicht):
    holz, holz_dunkel, leder = farbe("#8E5E30"), farbe("#65401E"), farbe("#2C221A")
    eisen, eisen_hell = farbe("#5A6068"), farbe("#C9D0DA")
    x, y = GRIFF_R.x, GRIFF_R.y
    _stiel(f, m, gewicht, holz, holz_dunkel, leder, x, y)
    auge = Vector((x, y, KOPF_Z))
    # Lange Spitze nach vorne (-Y), kurze Spitze nach hinten; beide Enden biegen sich nach oben
    profil = [(-0.19, 0.004, 0.004, 0.05), (-0.15, 0.012, 0.012, 0.03), (-0.09, 0.02, 0.022, 0.012), (-0.03, 0.03, 0.034, 0.0),
              (0.0, 0.034, 0.042, 0.0), (0.04, 0.028, 0.034, 0.0), (0.12, 0.018, 0.022, 0.012), (0.2, 0.012, 0.014, 0.03),
              (0.3, 0.005, 0.005, 0.055), (0.33, 0.001, 0.001, 0.066)]
    _profilkopf(f, "Haue", m, gewicht, auge, -Y, profil, lambda i, k, p: (eisen_hell if i < 1.2 or i > 7.6 else eisen) * (0.93 + 0.07 * (k % 2)))
    mr = m.to_3x3()
    f.loft("Beschlag", [(m @ (auge + Z * dz), mr @ X, mr @ Y, 0.027, 0.027) for dz in (-0.07, -0.05, 0.05, 0.07)], 10,
           lambda i, k, p: farbe("#3B3F47"), gewicht, oben_zu=True, unten_zu=True)


def steinhammer(f, m, gewicht):
    holz, holz_dunkel, leder = farbe("#9C6A3A"), farbe("#6E4824"), farbe("#4A2F1C")
    eisen, eisen_hell, eisen_dunkel = farbe("#5E646E"), farbe("#BFC6D0"), farbe("#3A3E46")
    x, y = GRIFF_R.x, GRIFF_R.y
    _stiel(f, m, gewicht, holz, holz_dunkel, leder, x, y)
    auge = Vector((x, y, KOPF_Z + 0.01))
    # Schwerer, vierkantiger Kopf mit abgeschrägten Bahnen vorne und hinten
    profil = [(-0.15, 0.036, 0.036, 0.0), (-0.14, 0.046, 0.046, 0.0), (-0.1, 0.05, 0.05, 0.0), (0.0, 0.046, 0.052, 0.0),
              (0.1, 0.05, 0.05, 0.0), (0.14, 0.046, 0.046, 0.0), (0.15, 0.036, 0.036, 0.0)]
    mr = m.to_3x3()
    ringe = []
    for d, dicke, hoehe, _ in profil:
        ringe.append((m @ (auge - Y * d), mr @ X, mr @ Z, dicke, hoehe, lambda w: 1.0 / max(abs(math.cos(w)), abs(math.sin(w))) ** 0.85))
    f.loft("Hammerkopf", ringe, 16, lambda i, k, p: eisen_hell if i < 1 or i > 5 else (eisen if k % 4 else eisen_dunkel), gewicht,
           oben_zu=True, unten_zu=True, teilung=1, glatt=False)


def spaten(f, m, gewicht):
    """Spaten: Stiel mit Holzgriff oben, flaches Eisenblatt unten (Schneide nach unten)."""
    holz, holz_dunkel, leder = farbe("#B8864E"), farbe("#8A5E30"), farbe("#6A4A2C")
    eisen, eisen_hell = farbe("#6A6E76"), farbe("#C8CED6")
    x, y = GRIFF_R.x, GRIFF_R.y
    _stiel(f, m, gewicht, holz, holz_dunkel, leder, x, y)
    mr = m.to_3x3()
    # Blatt: breit und flach, zur Schneide hin dünner, mit Tülle um den Stiel
    blatt = []
    for z, breite, dicke in ((KOPF_Z + 0.1, 0.03, 0.02), (KOPF_Z + 0.06, 0.085, 0.012), (KOPF_Z, 0.095, 0.009), (KOPF_Z - 0.12, 0.095, 0.007),
                             (KOPF_Z - 0.17, 0.085, 0.004), (KOPF_Z - 0.185, 0.07, 0.002)):
        blatt.append((m @ Vector((x, y, z)), mr @ X, mr @ Y, breite, dicke))
    f.loft("Spatenblatt", blatt, 8, lambda i, k, p: eisen_hell if i > 4.2 else eisen * (0.92 + 0.08 * (k % 2)), gewicht, oben_zu=True, unten_zu=True, teilung=1)
    f.loft("Tuelle", [(m @ Vector((x, y, z)), mr @ X, mr @ Y, 0.024, 0.024) for z in (KOPF_Z + 0.08, KOPF_Z + 0.2)], 10, lambda i, k, p: eisen, gewicht,
           oben_zu=True, unten_zu=True)
    # Querholz als Griff über der Faust
    f.loft("Spatengriff", [(m @ Vector((x - 0.07, y, STIEL_OBEN + 0.09)), mr @ Y, mr @ Z, 0.018, 0.018), (m @ Vector((x + 0.07, y, STIEL_OBEN + 0.09)), mr @ Y, mr @ Z, 0.018, 0.018)],
           8, lambda i, k, p: holz_dunkel, gewicht, oben_zu=True, unten_zu=True)
    f.loft("Griffstiel", [(m @ Vector((x, y, STIEL_OBEN - 0.02)), mr @ X, mr @ Y, 0.017, 0.017), (m @ Vector((x, y, STIEL_OBEN + 0.09)), mr @ X, mr @ Y, 0.015, 0.015)],
           8, lambda i, k, p: holz, gewicht)


WERKZEUGE = {"holz": faelleraxt, "stein": steinhammer, "erz": keilhaue, "lehm": spaten}


def _ruecken_matrix(grund, kopf_richtung):
    """Werkzeug auf den Rücken: Griff nach `grund`, der Stiel zeigt (vom Griff zum Kopf) nach `kopf_richtung`."""
    griff = Vector((GRIFF_R.x, GRIFF_R.y, STIEL_OBEN - 0.06))
    drehung = (-Z).rotation_difference(kopf_richtung.normalized()).to_matrix().to_4x4()
    return Matrix.Translation(grund) @ drehung @ Matrix.Translation(-griff)


# ---------------------------------------------------------------------------
# Kopf: Gesicht wie beim Magier (etwas kräftiger), Bart und Haar je nach Arbeiter
# ---------------------------------------------------------------------------
def _kopf(f, haut, lippe, wange, haar, bart_art, russ=False):
    r = f.rng
    d = Vector((0, 0, DZ))
    kopf = [
        (Vector((0, 0.005, 1.75)) + d, (0.095, 0.106, 0.112)),
        (Vector((0, -0.02, 1.682)) + d, (0.084, 0.078, 0.074)),            # breiter Kiefer
        (Vector((0.052, -0.07, 1.72)) + d, (0.032, 0.026, 0.023)),
        (Vector((-0.052, -0.07, 1.72)) + d, (0.032, 0.026, 0.023)),
        (Vector((0, -0.084, 1.776)) + d, (0.072, 0.03, 0.022)),            # kräftiger Brauenbogen
        (Vector((0, -0.102, 1.742)) + d, (0.015, 0.023, 0.03)),
        (Vector((0, -0.116, 1.722)) + d, (0.018, 0.025, 0.022)),
        (Vector((0, -0.134, 1.705)) + d, (0.023, 0.022, 0.02)),            # knollige Nase
        (Vector((0.02, -0.118, 1.7)) + d, (0.013, 0.014, 0.012)),
        (Vector((-0.02, -0.118, 1.7)) + d, (0.013, 0.014, 0.012)),
        (Vector((0, -0.075, 1.642)) + d, (0.04, 0.032, 0.032)),            # breites Kinn
        (Vector((0, -0.098, 1.672)) + d, (0.021, 0.012, 0.007)),
        (Vector((0, -0.106, 1.679)) + d, (0.023, 0.01, 0.0035), True),
        (Vector((0, 0.0, 1.61)) + d, (0.058, 0.058, 0.07)),                # kräftiger Hals
    ]
    for s in (1, -1):
        kopf += [
            (_spiegel(AUGE, s) + Vector((0, -0.006, 0)), (0.02, 0.014, 0.014), True),
            (_spiegel(AUGE, s) + Vector((0, -0.007, 0.01)), (0.018, 0.01, 0.006)),
            (_spiegel(AUGE, s) + Vector((0, -0.006, -0.01)), (0.016, 0.008, 0.0045)),
            (Vector((0.096 * s, 0.008, 1.735)) + d, (0.015, 0.031, 0.043)),
            (Vector((0.099 * s, 0.002, 1.702)) + d, (0.012, 0.015, 0.014)),
            (Vector((0.103 * s, 0.006, 1.735)) + d, (0.006, 0.018, 0.026), True),
        ]
    mund = Vector((0, -0.098, 1.672)) + d

    def gesicht(poly):
        p, n = poly.center, poly.normal
        c = haut * (0.94 if p.z < 1.66 + DZ else 1.0)
        if (p - mund).length < 0.02 and p.y < -0.09:
            return lippe
        if abs(p.x) > 0.09 and n.x * p.x > 0 and abs(p.z - 1.73 - DZ) < 0.035:
            c = c.lerp(wange * 0.9, 0.5)
        rosig = math.exp(-(((abs(p.x) - 0.054) ** 2 + (p.z - 1.716 - DZ) ** 2) / 0.019 ** 2)) if p.y < -0.05 else 0.0
        rosig = max(rosig, 0.45 * math.exp(-((p - Vector((0, -0.134, 1.71 + DZ))).length_squared / 0.016 ** 2)))
        c = c.lerp(wange, rosig * 0.75)
        if russ:
            # Ruß auf Stirn, Wangen und Nasenrücken (fleckig)
            fleck = 0.5 + 0.5 * math.sin(p.x * 90 + p.z * 60) * math.sin(p.z * 110 - p.x * 40)
            if p.y < -0.04 and (p.z > 1.765 + DZ or abs(abs(p.x) - 0.06) < 0.025) and fleck > 0.55:
                c = c * 0.55
        return c
    f.metaball("Kopf", kopf, 0.0045, 5200, gesicht, _kopf_und_hals, glatt=True)

    for s in (1, -1):
        auge = _spiegel(AUGE, s)
        f.kugel("Augapfel", auge, (0.0125, 0.0125, 0.0125), farbe("#EDE6DA"), KOPF_GEWICHT, 16, 12)
        f.kugel("Iris", auge + Vector((-0.001 * s, -0.0105, 0.0005)), (0.0068, 0.0028, 0.0068), farbe("#4E6A3A") if russ else farbe("#5A7FA6"),
                KOPF_GEWICHT, 14, 8)
        f.kugel("Pupille", auge + Vector((-0.001 * s, -0.0122, 0.0005)), (0.0034, 0.0016, 0.0034), farbe("#0E1116"), KOPF_GEWICHT, 10, 6)
        f.kugel("Glanz", auge + Vector((-0.004 * s, -0.0128, 0.0035)), (0.0016, 0.0008, 0.0016), farbe("#FFFFFF"), KOPF_GEWICHT, 8, 4)
        f.kugel("Nasenloch", Vector((0.011 * s, -0.132, 1.69 + DZ)), (0.005, 0.004, 0.003), farbe("#6A3A30"), KOPF_GEWICHT, 10, 6)
        # Buschige Brauen
        for j in range(8):
            x = 0.012 + 0.0098 * j
            basis = Vector((x * s, -0.108 + 1.4 * (x - 0.03) ** 2, 1.773 + DZ + 0.007 * math.sin(math.pi * x / 0.085)))
            richtung = Vector((0.65 * s, -0.25 - r.uniform(0, 0.25), 0.4 + r.uniform(-0.15, 0.2))).normalized()
            laenge = 0.013 + 0.01 * (j / 7) + r.uniform(0, 0.004)
            f.straehne("Braue", [basis, basis + richtung * laenge * 0.55 + Vector((0, -0.002, 0.001)), basis + richtung * laenge], 0.0052, 0.001,
                       haar * r.uniform(0.85, 1.0), KOPF_GEWICHT, 6, 0.4, 0.7)

    # Kurzes Haar an Schläfen und Hinterkopf (oben sitzt immer eine Mütze oder ein Helm)
    f.metaball("Haar", [(Vector((0, 0.03, 1.745)) + d, (0.1, 0.1, 0.085)), (Vector((0, 0.06, 1.69)) + d, (0.092, 0.06, 0.05))], 0.008, 700,
               lambda poly: haar * (0.9 + 0.1 * (poly.index % 3) / 2), KOPF_GEWICHT)

    if bart_art == "voll":
        # Voller, kurzer Vollbart von Ohr zu Ohr, nach unten etwas zugespitzt
        ringe = []
        for t, (z, y, rx, ry) in enumerate(((1.705, -0.02, 0.1, 0.1), (1.672, -0.04, 0.098, 0.094), (1.635, -0.065, 0.082, 0.078),
                                             (1.6, -0.09, 0.058, 0.052), (1.572, -0.105, 0.03, 0.026), (1.555, -0.11, 0.006, 0.006))):
            ringe.append((Vector((0, y, z + DZ)), X, Y, rx, ry, lambda w, t=t: 1.0 + 0.08 * math.sin(w * 7 + t) + 0.04 * math.sin(w * 15)))

        def bart_farbe(i, k, p):
            # nur die Vorderseite: hinter dem Kinn verschwindet er im Hals
            return haar * (0.85 + 0.2 * ((k * 5 + int(i * 3)) % 4) / 3)
        f.loft("Bart", ringe, 32, bart_farbe, _kopf_und_hals, oben_zu=False, unten_zu=True, teilung=2, glatt=True)
        for n in range(18):
            w = math.radians(r.uniform(-120, 120))
            start = Vector((math.sin(w) * 0.09, -0.02 - math.cos(w) * 0.085, 1.675 + DZ + r.uniform(-0.03, 0.01)))
            ende = start + Vector((math.sin(w) * 0.01, -math.cos(w) * 0.03 - 0.02, -0.06 - 0.04 * math.cos(w)))
            f.straehne("Bartlocke", [start, start.lerp(ende, 0.5) + Vector((0, -0.012, 0)), ende], r.uniform(0.016, 0.022), 0.003,
                       haar * r.uniform(0.85, 1.1), _kopf_und_hals, 7, 0.5, 0.7, glatt=True)
        for s in (1, -1):
            punkte = [Vector((0.008 * s, -0.128, 1.69 + DZ)), Vector((0.035 * s, -0.126, 1.683 + DZ)), Vector((0.058 * s, -0.11, 1.662 + DZ))]
            f.straehne("Schnurrbart", punkte, 0.014, 0.006, haar * 0.95, KOPF_GEWICHT, 8, 0.4, 0.7, glatt=True)
    elif bart_art == "schnauzer":
        # Kräftiger Walross-Schnauzbart, dazu Stoppeln (dunklere Wangen über den Kopf-Farben)
        for s in (1, -1):
            for j in range(3):
                punkte = [Vector((0.006 * s, -0.13, 1.692 + DZ - 0.003 * j)), Vector((0.036 * s, -0.128, 1.684 + DZ - 0.004 * j)),
                          Vector((0.056 * s, -0.114, 1.66 + DZ - 0.006 * j)), Vector((0.064 * s, -0.106, 1.632 + DZ - 0.008 * j))]
                f.straehne("Schnurrbart", punkte, 0.017 - 0.003 * j, 0.004, haar * (1.0 - 0.06 * j), KOPF_GEWICHT, 8, 0.4, 0.75, glatt=True)
    elif bart_art == "kurz":
        # Kurzer, gepflegter Bart: unter der Lippe um Kinn und Kiefer, die Lippen bleiben frei,
        # schmale Koteletten und ein feiner Schnurrbart
        ringe = []
        for z, y, rx, ry in ((1.615, -0.035, 0.092, 0.092), (1.595, -0.045, 0.08, 0.08), (1.572, -0.058, 0.052, 0.05), (1.556, -0.064, 0.014, 0.014)):
            ringe.append((Vector((0, y, z)), X, Y, rx, ry))
        f.loft("Bart", ringe, 32, lambda i, k, p: haar * (0.88 + 0.12 * (k % 3) / 2), _kopf_und_hals, unten_zu=True, teilung=2, glatt=True)
        for s in (1, -1):
            wange = [Vector((0.094 * s, 0.0, 1.705 + DZ)), Vector((0.093 * s, -0.035, 1.665 + DZ)), Vector((0.08 * s, -0.07, 1.635 + DZ)),
                     Vector((0.05 * s, -0.098, 1.612 + DZ))]
            f.straehne("Kotelette", wange, 0.016, 0.01, haar * 0.95, KOPF_GEWICHT, 8, 0.2, 0.45, glatt=True)
            f.straehne("Schnurrbart", [Vector((0.004 * s, -0.126, 1.688 + DZ)), Vector((0.026 * s, -0.122, 1.684 + DZ)),
                                       Vector((0.046 * s, -0.11, 1.667 + DZ))], 0.008, 0.003, haar, KOPF_GEWICHT, 8, 0.3, 0.6, glatt=True)
            f.straehne("Kinnbart", [Vector((0.006 * s, -0.102, 1.66 + DZ)), Vector((0.004 * s, -0.104, 1.64 + DZ))], 0.007, 0.004, haar, KOPF_GEWICHT, 6, 0.0, 0.6,
                       glatt=True)
    elif bart_art == "keiner":
        pass
    else:
        # Kurzer Stoppelbart um Kinn und Mund
        ringe = []
        for t, (z, y, rx, ry) in enumerate(((1.69, -0.03, 0.094, 0.095), (1.662, -0.05, 0.088, 0.086), (1.63, -0.07, 0.06, 0.058),
                                             (1.612, -0.08, 0.025, 0.025))):
            ringe.append((Vector((0, y, z + DZ)), X, Y, rx, ry))
        f.loft("Bart", ringe, 32, lambda i, k, p: haar * (0.9 + 0.1 * (k % 3) / 2), _kopf_und_hals, unten_zu=True, teilung=2, glatt=True)
        for s in (1, -1):
            f.straehne("Schnurrbart", [Vector((0.006 * s, -0.128, 1.69 + DZ)), Vector((0.03 * s, -0.126, 1.684 + DZ)),
                                       Vector((0.05 * s, -0.114, 1.668 + DZ))], 0.011, 0.005, haar, KOPF_GEWICHT, 8, 0.4, 0.7, glatt=True)


# ---------------------------------------------------------------------------
# Körper: Stiefel, Hose, Rumpf, Arme, Hände (Farben je Arbeiter)
# ---------------------------------------------------------------------------
def _koerper(f, haut, hose, hose_farbe, oben_farbe, stiefel, sohle, handschuh, aermel, aermel_farbe, hochgekrempelt, hand_ziel=2000):
    # Stiefel und Hose
    for seite in (1, -1):
        sn = "L" if seite > 0 else "R"
        h_, k_, a_ = _spiegel(HUEFTE, seite), _spiegel(KNIE, seite), _spiegel(KNOECHEL, seite)
        punkte = [h_ + Vector((0.012 * seite, 0.005, 0.03)), h_.lerp(k_, 0.35), h_.lerp(k_, 0.75), k_, k_.lerp(a_, 0.45), a_ + Vector((0, 0, 0.1))]
        radien = [0.098, 0.088, 0.07, 0.064, 0.062, 0.06]
        _schlauch(f, "Hose", punkte, radien, 20, lambda i, k, p, s=seite: hose_farbe(p, s), _bein(seite), zu=False, teilung=3)
        # Schnürstiefel: Schaft bis zur Wade, dicke Sohle, runde Kappe
        schaft = [a_ + Vector((0, 0.005, -0.03)), a_ + Vector((0, 0.0, 0.07)), a_ + Vector((0, -0.005, 0.2))]
        _schlauch(f, "Stiefelschaft", schaft, [0.056, 0.058, 0.066], 18,
                  lambda i, k, p: stiefel * (0.75 if i > 1.6 else 1.0) * (0.85 if k % 6 == 0 else 1.0), _bein(seite), zu=False, teilung=2)
        fuss = [(Vector((0.1 * seite, 0.06, 0.05)), X, Z, 0.05, 0.05), (Vector((0.1 * seite, 0.04, 0.02)), X, Z, 0.056, 0.03),
                (Vector((0.1 * seite, -0.03, 0.05)), X, Z, 0.058, 0.055), (Vector((0.1 * seite, -0.1, 0.045)), X, Z, 0.052, 0.042),
                (Vector((0.1 * seite, -0.15, 0.04)), X, Z, 0.036, 0.032), (Vector((0.1 * seite, -0.175, 0.04)), X, Z, 0.006, 0.006)]
        f.loft("Stiefel", fuss, 16, lambda i, k, p: sohle if p.center.z < 0.018 else stiefel * (1.1 if p.center.y < -0.1 else 1.0),
               lambda co, sn=sn: {f"Fuss.{sn}": 1.0}, oben_zu=True, unten_zu=True, teilung=3, glatt=True)
        # Schnürung
        for j in range(4):
            f.kiste("Schnuer", (0.1 * seite, -0.045 - 0.012 * j, 0.1 + 0.035 * j), (0.05, 0.006, 0.006), farbe("#D8C8A0"), _bein(seite))

    # Rumpf
    rumpf = [(0.86, 0.01, 0.125, 0.095), (0.92, 0.012, 0.168, 0.118), (0.98, 0.012, 0.178, 0.124), (1.05, 0.004, 0.175, 0.132),
             (1.14, -0.002, 0.176, 0.136), (1.24, -0.006, 0.184, 0.134), (1.33, -0.006, 0.198, 0.132), (1.41, 0.0, 0.204, 0.122),
             (1.47, 0.01, 0.17, 0.1), (1.52, 0.014, 0.092, 0.076), (1.57, 0.014, 0.058, 0.058)]
    ringe = [(Vector((0, y, z)), X, Y, rx, ry, lambda w: 1.0 - 0.05 * max(0.0, -math.sin(w))) for z, y, rx, ry in rumpf]
    f.loft("Koerper", ringe, 44, lambda i, k, p: oben_farbe(p) if p.center.z > 0.99 else hose, _huefte_bein, teilung=3, glatt=True)

    # Arme: Ärmel über dem Oberarm, darunter Haut (hochgekrempelt) oder Stoff, Handschuhe
    for seite in (1, -1):
        s_, e_, h_ = _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite)
        punkte = [s_ + Vector((-0.02 * seite, 0, 0.03)), s_.lerp(e_, 0.5), e_, e_.lerp(h_, 0.5), h_]
        _schlauch(f, "Arm", punkte, [0.066, 0.058, 0.05, 0.048, 0.038], 18,
                  lambda i, k, p: haut if hochgekrempelt and i > 2.0 else aermel_farbe(p), _arm(seite), zu=False, teilung=3)
        if hochgekrempelt:
            # Aufgerollter Ärmel knapp unter dem Ellbogen
            q1, q2 = _achsen(e_, h_)
            wulst = [(e_.lerp(h_, t), q1, q2, rad, rad) for t, rad in ((-0.02, 0.058), (0.08, 0.066), (0.18, 0.06))]
            f.loft("Aermelwulst", wulst, 18, lambda i, k, p: aermel * (0.85 if k % 5 == 0 else 1.0), _arm(seite), teilung=2, glatt=True)
        # Handschuhstulpe
        q1, q2 = _achsen(e_, h_)
        stulpe = [(e_.lerp(h_, t), q1, q2, rad, rad) for t, rad in ((0.82, 0.045), (0.95, 0.05), (1.05, 0.052))]
        f.loft("Stulpe", stulpe, 16, lambda i, k, p: handschuh * 0.9, _arm(seite), teilung=2, glatt=True)
    f.metaball("HandR", _faust(GRIFF_R, Z, -1, 0.0095), 0.003, hand_ziel, lambda poly: handschuh, lambda co: {"Hand.R": 1.0}, glatt=True)
    f.metaball("HandL", _faust(GRIFF_L, Y, 1, 0.0095), 0.003, hand_ziel, lambda poly: handschuh, lambda co: {"Hand.L": 1.0}, glatt=True)


def _skelett(f):
    f.knochen_dazu("Becken", (0, 0, 0.93), (0, 0, 1.07), None, HOCH)
    f.knochen_dazu("Bauch", (0, 0, 1.07), (0, 0, 1.27), "Becken", HOCH)
    f.knochen_dazu("Brust", (0, 0, 1.27), (0, 0, 1.52), "Bauch", HOCH)
    f.knochen_dazu("Hals", (0, 0, 1.52), (0, 0, 1.58), "Brust", HOCH)
    f.knochen_dazu("Kopf", (0, 0, 1.58), (0, 0, 1.82), "Hals", HOCH)
    f.knochen_dazu("Hut", (0, 0.02, 1.84), (0, 0.12, 1.9), "Kopf", HOCH)
    for seite, sn in ((1, "L"), (-1, "R")):
        f.knochen_dazu(f"Oberarm.{sn}", _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), "Brust", HAENGT)
        f.knochen_dazu(f"Unterarm.{sn}", _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite), f"Oberarm.{sn}", HAENGT)
        f.knochen_dazu(f"Hand.{sn}", _spiegel(HANDGELENK, seite), _spiegel(FINGER, seite), f"Unterarm.{sn}", HAENGT)
        f.knochen_dazu(f"Oberschenkel.{sn}", _spiegel(HUEFTE, seite), _spiegel(KNIE, seite), "Becken", HAENGT)
        f.knochen_dazu(f"Unterschenkel.{sn}", _spiegel(KNIE, seite), _spiegel(KNOECHEL, seite), f"Oberschenkel.{sn}", HAENGT)
        f.knochen_dazu(f"Fuss.{sn}", _spiegel(KNOECHEL, seite), _spiegel(ZEHEN, seite), f"Unterschenkel.{sn}", (0, 0, 1))


def _werkzeug(f, art, ruecken):
    hand = lambda co: {"Hand.R": 1.0}
    anfang = len(f.teile)
    WERKZEUGE[art](f, Matrix.Identity(4), hand)
    f.als_starr("Werkzeug", "Hand.R", anfang)
    anfang = len(f.teile)
    WERKZEUGE[art](f, ruecken, BRUST_GEWICHT)
    f.als_starr("Werkzeug_Ruecken", "Brust", anfang)


# ---------------------------------------------------------------------------
# Holzarbeiter
# ---------------------------------------------------------------------------
def holzarbeiter(seed=71, name="Holzarbeiter"):
    f = Figur(name, seed)
    r = f.rng
    haut, lippe, wange = farbe("#E8B894"), farbe("#B86A5A"), farbe("#E0907A")
    haar = farbe("#6A3E1E")
    rot, rot_dunkel, schwarz = farbe("#B8302A"), farbe("#7A1A16"), farbe("#241816")
    oliv, oliv_dunkel = farbe("#5A5A36"), farbe("#43432A")
    leder, leder_dunkel = farbe("#7A4A28"), farbe("#4A2C18")
    senf, senf_dunkel = farbe("#D9A030"), farbe("#A87420")

    def karo(p):
        # rot-schwarzes Büffelkaro (Flanell), Streifen um die Körperachse
        u = int((math.atan2(p.center.x, -p.center.y) + math.pi) / math.tau * 16) % 2
        v = int(p.center.z * 16) % 2
        if u and v:
            return schwarz
        if u or v:
            return rot_dunkel
        return rot

    def arm_karo(p):
        v = int(p.center.z * 18) % 2
        u = int((p.center.x * 30 + p.center.y * 30)) % 2
        return schwarz if u and v else (rot_dunkel if u or v else rot)

    def hose(p, s):
        return oliv_dunkel if p.normal.x * s > 0.85 else oliv * (0.94 + 0.08 * max(0.0, -p.normal.y))

    _koerper(f, haut, oliv, hose, karo, leder, farbe("#2A1C12"), farbe("#A8804E"), rot, arm_karo, True)
    # Offener Kragen mit Knopfleiste, Hosenträger vorne und hinten, Gürtel mit Tasche
    for s in (1, -1):
        _platte(f, "Kragen", Vector((0.05 * s, -0.07, 1.5)), 0.1, 0.045, 0.008, Vector((0.35 * s, -0.4, -1)), Vector((0.3 * s, -1, 0.3)),
                lambda i, k, p: rot_dunkel, _rumpf, spitz=0.7, wolbung=0.5)
        vorne = [Vector((0.09 * s, -0.12, 1.44)), Vector((0.1 * s, -0.142, 1.3)), Vector((0.095 * s, -0.14, 1.12)), Vector((0.09 * s, -0.135, 1.0))]
        f.loft("Hosentraeger", [(p, X, Y, 0.018, 0.006) for p in vorne], 6, lambda i, k, p: leder, _rumpf, oben_zu=True, unten_zu=True, teilung=3)
        f.kugel("Traegerknopf", Vector((0.09 * s, -0.142, 1.0)), (0.012, 0.006, 0.012), farbe("#C9A050"), _rumpf, 8, 4)
    hinten = [Vector((0.09, 0.13, 1.44)), Vector((0.0, 0.14, 1.28)), Vector((0.0, 0.14, 1.15)), Vector((0.0, 0.135, 1.0))]
    f.loft("Hosentraeger", [(p, X, Y, 0.02, 0.006) for p in hinten], 6, lambda i, k, p: leder, _rumpf, oben_zu=True, unten_zu=True, teilung=3)
    f.loft("Hosentraeger", [(Vector((-0.09, 0.13, 1.44)), X, Y, 0.02, 0.006), (Vector((0.0, 0.14, 1.28)), X, Y, 0.02, 0.006)], 6,
           lambda i, k, p: leder, _rumpf, oben_zu=True, unten_zu=True)
    for z in (1.42, 1.34, 1.26, 1.18):
        f.kugel("Hemdknopf", Vector((0, -0.14 + (z - 1.3) * 0.02, z)), (0.007, 0.004, 0.007), farbe("#E8DCC0"), _rumpf, 8, 4)
    f.loft("Guertel", [(Vector((0, 0.008, 0.99)), X, Y, 0.182, 0.13), (Vector((0, 0.008, 1.03)), X, Y, 0.18, 0.13)], 40,
           lambda i, k, p: leder_dunkel, _rumpf, teilung=2)
    f.kiste("Schnalle", (0, -0.134, 1.01), (0.05, 0.012, 0.04), farbe("#B8A070"), _rumpf)
    f.kiste("Guerteltasche", (0.15, -0.07, 0.97), (0.08, 0.05, 0.1), leder, _huefte_bein, drehung=Quaternion(Z, math.radians(30)).to_matrix())

    _kopf(f, haut, lippe, wange, haar, "voll")
    # Senfgelbe Strickmütze mit Umschlag, leicht nach hinten gerutscht
    kappe = []
    for t, (z, rx, ry, y) in enumerate(((1.735, 0.103, 0.113, 0.005), (1.77, 0.106, 0.115, 0.01), (1.8, 0.102, 0.11, 0.018),
                                        (1.83, 0.088, 0.095, 0.03), (1.855, 0.062, 0.068, 0.045), (1.868, 0.02, 0.022, 0.055))):
        kappe.append((Vector((0, y, z + 0.005)), X, Y, rx, ry))
    f.loft("Muetze", kappe, 36, lambda i, k, p: senf * (0.88 if k % 2 else 1.0), KOPF_GEWICHT, oben_zu=True, teilung=3, glatt=True)
    f.loft("Umschlag", [(Vector((0, 0.004, 1.728)), X, Y, 0.109, 0.119), (Vector((0, 0.006, 1.755)), X, Y, 0.112, 0.121),
                        (Vector((0, 0.008, 1.782)), X, Y, 0.108, 0.117)], 36, lambda i, k, p: senf_dunkel * (0.9 if k % 2 else 1.0), KOPF_GEWICHT,
           teilung=2, glatt=True)
    f.kugel("Bommel", Vector((0, 0.06, 1.885)), (0.035, 0.035, 0.032), lambda poly: senf.lerp(senf_dunkel, 0.3 + 0.3 * poly.normal.y),
            lambda co: {"Hut": 1.0}, 12, 8, glatt=True)

    # Last: drei Stämme auf der linken Schulter (entlang Y), Rinde mit hellen Schnittflächen
    anfang = len(f.teile)
    rinde, rinde_dunkel, holz_hell = farbe("#7A5232"), farbe("#4E321C"), farbe("#E8C88A")
    for j, (x, z, rad) in enumerate(((0.2, 1.575, 0.07), (0.335, 1.56, 0.066), (0.27, 1.685, 0.064))):
        laenge = 1.25 + 0.1 * j
        ringe = []
        for t in range(9):
            y = -0.5 - 0.04 * j + laenge * t / 8
            knorr = 1.0 + 0.06 * math.sin(t * 2.1 + j)
            ringe.append((Vector((x, y, z)), X, Z, rad * knorr, rad * knorr))

        def stamm_farbe(i, k, p, j=j):
            if abs(p.normal.y) > 0.8:
                return holz_hell
            return rinde_dunkel if (k + int(i * 1.7) + j) % 5 == 0 else rinde * (0.9 + 0.1 * ((k * 3 + j) % 3) / 2)
        f.loft("Stamm", ringe, 14, stamm_farbe, BRUST_GEWICHT, oben_zu=True, unten_zu=True, teilung=1, glatt=True)
        for y_ende in (ringe[0][0].y - 0.002, ringe[-1][0].y + 0.002):
            # Jahresringe auf den Schnittflächen
            f.loft("Jahresring", [(Vector((x, y_ende, z)), X, Z, rad * 0.55, rad * 0.55), (Vector((x, y_ende + (0.001 if y_ende > 0 else -0.001), z)), X, Z,
                                                                                             rad * 0.55, rad * 0.55)], 12, lambda i, k, p: farbe("#C89A58"),
                   BRUST_GEWICHT, oben_zu=True, unten_zu=True)
    f.loft("Seil", [(Vector((0.27, 0.35, 1.61)), Y, Z, 0.125, 0.105), (Vector((0.27, 0.37, 1.61)), Y, Z, 0.125, 0.105)], 16,
           lambda i, k, p: farbe("#C8B080"), BRUST_GEWICHT, teilung=1)
    f.als_starr("Last", "Brust", anfang)

    # Werkzeug: Fälleraxt in der Rechten, beim Tragen quer über dem Rücken
    _werkzeug(f, "holz", _ruecken_matrix(Vector((0.18, 0.2, 0.88)), Vector((-0.38, 0.05, 0.93))))
    _skelett(f)
    return f.fertig(lambda armatur: _animationen(armatur, "holz"))


# ---------------------------------------------------------------------------
# Steinarbeiter
# ---------------------------------------------------------------------------
def steinarbeiter(seed=72, name="Steinarbeiter"):
    f = Figur(name, seed)
    haut, lippe, wange = farbe("#DDA880"), farbe("#A86050"), farbe("#D48870")
    haar = farbe("#8A7A64")
    leinen, leinen_dunkel = farbe("#DCCFB2"), farbe("#B8A888")
    grau, grau_dunkel = farbe("#5E6670"), farbe("#434950")
    leder, leder_dunkel = farbe("#9A6A3C"), farbe("#6A4424")
    staub = farbe("#C8C0B0")
    rot = farbe("#B8342C")

    def hemd(p):
        return leinen * (0.92 + 0.08 * max(0.0, -p.normal.y)) * (0.93 if int(p.center.z * 40) % 5 == 0 else 1.0)

    def hose(p, s):
        c = grau_dunkel if p.normal.x * s > 0.85 else grau
        # Flicken aufs linke Knie, Staub unten
        if s > 0 and abs(p.center.z - 0.5) < 0.06 and p.normal.y < -0.4:
            return farbe("#7A6A50")
        return c.lerp(staub, 0.35 * weich(0.35, 0.1, p.center.z))

    _koerper(f, haut, grau, hose, hemd, farbe("#4A3A2C"), farbe("#221A14"), farbe("#8A6A48"), leinen, lambda p: leinen * 0.95, True)

    # Lange Lederschürze: vorne vom Brustbein bis unter die Knie, Nackenriemen, Tasche mit Meißeln
    schuerze = []
    for t, (z, y, breite) in enumerate(((1.4, -0.132, 0.12), (1.3, -0.142, 0.13), (1.15, -0.148, 0.15), (1.0, -0.142, 0.17),
                                        (0.85, -0.15, 0.18), (0.7, -0.16, 0.18), (0.55, -0.168, 0.175))):
        schuerze.append((Vector((0, y, z)), X, Y, breite, 0.012, lambda w: 1.0 + 0.03 * math.sin(w * 3 + t)))

    def schuerze_farbe(i, k, p):
        c = leder * (0.9 + 0.1 * math.sin(p.center.z * 30 + p.center.x * 20))
        if i > 5.6:
            c = leder_dunkel
        return c.lerp(staub, 0.25 * weich(1.0, 0.6, p.center.z) * (0.5 + 0.5 * math.sin(p.center.x * 50)))
    f.loft("Schuerze", schuerze, 16, schuerze_farbe, _huefte_bein_schuerze, oben_zu=True, unten_zu=True, teilung=3, glatt=True)
    f.kiste("Schuerzentasche", (0.0, -0.168, 1.08), (0.2, 0.02, 0.12), leder_dunkel, _rumpf)
    for j, x in enumerate((-0.06, -0.035, 0.04)):
        f.loft("Meissel", [(Vector((x, -0.17, 1.1)), X, Y, 0.009, 0.009), (Vector((x + 0.004 * j, -0.172, 1.2 + 0.02 * j)), X, Y, 0.011, 0.011)], 8,
               lambda i, k, p: farbe("#8E5E30") if i < 0.6 else farbe("#A8B0BA"), _rumpf, oben_zu=True, unten_zu=True)
    for s in (1, -1):
        f.loft("Nackenriemen", [(Vector((0.1 * s, -0.13, 1.4)), X, Y, 0.012, 0.005), (Vector((0.085 * s, -0.06, 1.52)), X, Y, 0.012, 0.005),
                                (Vector((0.05 * s, 0.04, 1.56)), X, Y, 0.012, 0.005)], 6, lambda i, k, p: leder_dunkel, _rumpf, oben_zu=True, unten_zu=True,
               teilung=2)
    f.loft("Schuerzenband", [(Vector((0, 0.008, 1.05)), X, Y, 0.182, 0.14), (Vector((0, 0.008, 1.075)), X, Y, 0.181, 0.139)], 40,
           lambda i, k, p: leder_dunkel, _rumpf)
    # Rotes Halstuch, vorne geknotet
    f.loft("Halstuch", [(Vector((0, 0.012, 1.53)), X, Y, 0.1, 0.085), (Vector((0, 0.012, 1.565)), X, Y, 0.075, 0.07)], 24,
           lambda i, k, p: rot * (0.85 if k % 4 == 0 else 1.0), _rumpf, teilung=2, glatt=True)
    f.kugel("Knoten", Vector((0, -0.085, 1.525)), (0.025, 0.02, 0.022), rot * 0.9, _rumpf, 10, 6)
    _platte(f, "Tuchzipfel", Vector((0.0, -0.1, 1.46)), 0.12, 0.05, 0.008, -Z, -Y, lambda i, k, p: rot, _rumpf, spitz=0.9, wolbung=0.3)

    _kopf(f, haut, lippe, wange, haar, "schnauzer")
    # Graue Schiebermütze mit kurzem Schirm
    kappe = []
    for t, (z, rx, ry, y) in enumerate(((1.735, 0.104, 0.114, 0.0), (1.765, 0.11, 0.122, -0.008), (1.8, 0.108, 0.126, -0.02),
                                        (1.82, 0.09, 0.114, -0.03), (1.83, 0.04, 0.06, -0.035))):
        kappe.append((Vector((0, y, z)), X, Y, rx, ry))
    kappe_farbe = farbe("#6E6A62")
    f.loft("Muetze", kappe, 36, lambda i, k, p: kappe_farbe * (0.9 if (k // 3) % 2 else 1.0), KOPF_GEWICHT, oben_zu=True, teilung=3, glatt=True)
    _platte(f, "Schirm", Vector((0, -0.14, 1.745)), 0.09, 0.09, 0.008, -Y + Z * -0.25, Z, lambda i, k, p: kappe_farbe * 0.75, KOPF_GEWICHT,
            spitz=0.2, wolbung=0.4)
    f.kugel("Knopf", Vector((0, -0.04, 1.832)), (0.012, 0.012, 0.006), kappe_farbe * 0.7, KOPF_GEWICHT, 8, 4)

    # Last: behauener Quader, vor dem Bauch mit beiden Händen gehalten
    anfang = len(f.teile)
    block_farbe = farbe("#BDB6A6")
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.bevel(bm, geom=bm.edges[:], offset=0.02, segments=1, affect="EDGES")
    for v in bm.verts:
        v.co = Vector((v.co.x * 0.38, v.co.y * 0.26, v.co.z * 0.26)) + Vector((0, -0.3, 1.02))

    def block(poly):
        n, p = poly.normal, poly.center
        c = block_farbe * (0.85 + 0.15 * max(0.0, n.z)) * (0.9 + 0.1 * math.sin(p.x * 60 + p.z * 40))
        return c
    f._objekt(bm, "Block", block, BRUST_GEWICHT)
    f.als_starr("Last", "Brust", anfang)

    _werkzeug(f, "stein", _ruecken_matrix(Vector((0.18, 0.2, 0.9)), Vector((-0.38, 0.05, 0.93))))
    _skelett(f)
    return f.fertig(lambda armatur: _animationen(armatur, "stein"))


def _huefte_bein_schuerze(co):
    """Die Schürze schwingt unten mit beiden Oberschenkeln."""
    if co.z > 0.98:
        return _rumpf(co)
    bein = weich(0.98, 0.5, co.z) * 0.6
    links = weich(-0.1, 0.1, co.x)
    return _mischen(("Becken", 1 - bein), ("Oberschenkel.L", bein * links), ("Oberschenkel.R", bein * (1 - links)))


# ---------------------------------------------------------------------------
# Erzarbeiter
# ---------------------------------------------------------------------------
def erzarbeiter(seed=73, name="Erzarbeiter"):
    f = Figur(name, seed)
    haut, lippe, wange = farbe("#E0AE8C"), farbe("#A8604E"), farbe("#D08870")
    haar = farbe("#2A2220")
    kittel, kittel_dunkel, kittel_hell = farbe("#26324A"), farbe("#1A2234"), farbe("#34466A")
    hose_c = farbe("#3C3C42")
    leder, leder_dunkel = farbe("#3A2A20"), farbe("#221812")
    messing, messing_dunkel = farbe("#D8B050"), farbe("#9A7A30")
    weide, weide_dunkel = farbe("#B08A50"), farbe("#7A5A30")

    def kittel_farbe(p):
        c = kittel * (0.92 + 0.1 * max(0.0, -p.normal.y))
        if p.center.z > 1.38 and abs(p.center.x) > 0.13:
            c = kittel_hell                                                   # Schulterstücke
        if p.normal.y < -0.5 and abs(p.center.x) < 0.012:
            c = kittel_dunkel                                                 # Knopfleiste
        return c

    def hose(p, s):
        c = hose_c * (0.94 + 0.08 * max(0.0, -p.normal.y))
        if abs(p.center.z - 0.5) < 0.07 and p.normal.y < -0.3:
            return leder                                                      # Knieleder
        return c

    _koerper(f, haut, kittel, hose, kittel_farbe, farbe("#1E1A18"), farbe("#0E0C0A"), leder, kittel, lambda p: kittel * 0.95, False)
    # Bergkittel: Rockschoß bis zur Hüfte, Messingknöpfe, breiter Gürtel mit Schloss
    schoss = [(Vector((0, 0.012, 1.0)), X, Y, 0.188, 0.14), (Vector((0, 0.018, 0.9)), X, Y, 0.2, 0.15), (Vector((0, 0.022, 0.8)), X, Y, 0.21, 0.158)]
    f.loft("Kittelschoss", schoss, 40, lambda i, k, p: kittel if abs(p.center.x) > 0.015 or p.normal.y > 0 else kittel_dunkel, _huefte_bein_schuerze,
           teilung=2, glatt=True)
    for j in range(8):
        z = 1.45 - 0.07 * j
        f.kugel("Knopf", Vector((0, -0.145 + 0.004 * abs(j - 4), z)), (0.011, 0.006, 0.011), messing, _rumpf, 8, 5)
    f.loft("Guertel", [(Vector((0, 0.01, 1.0)), X, Y, 0.192, 0.143), (Vector((0, 0.01, 1.05)), X, Y, 0.19, 0.142)], 40, lambda i, k, p: leder_dunkel,
           _rumpf, teilung=2)
    f.kiste("Schloss", (0, -0.146, 1.025), (0.06, 0.012, 0.05), messing, _rumpf)
    # Arschleder: schwarzes Leder hinten über dem Gesäß
    _platte(f, "Arschleder", Vector((0, 0.16, 0.88)), 0.3, 0.2, 0.012, -Z, Y, lambda i, k, p: leder * (0.85 if k % 3 == 0 else 1.0), _huefte_bein_schuerze,
            spitz=0.2, wolbung=1.5)

    _kopf(f, haut, lippe, wange, haar, "stoppeln", russ=True)
    # Messinghelm mit Krempe und Grubenlampe
    helm = []
    for t, (z, rx, ry) in enumerate(((1.738, 0.112, 0.122), (1.77, 0.115, 0.125), (1.81, 0.105, 0.113), (1.845, 0.08, 0.087),
                                     (1.866, 0.045, 0.05), (1.873, 0.01, 0.01))):
        helm.append((Vector((0, 0.005, z)), X, Y, rx, ry))
    f.loft("Helm", helm, 36, lambda i, k, p: (messing if k % 9 else messing_dunkel) * (0.85 + 0.2 * max(0.0, p.normal.z)), KOPF_GEWICHT, oben_zu=True,
           teilung=3, glatt=True)
    f.loft("Krempe", [(Vector((0, 0.005, 1.742)), X, Y, 0.114, 0.124), (Vector((0, 0.0, 1.736)), X, Y, 0.15, 0.162), (Vector((0, 0.0, 1.728)), X, Y, 0.152, 0.164)],
           36, lambda i, k, p: messing_dunkel, KOPF_GEWICHT, teilung=1, glatt=True)
    f.loft("Lampe", [(Vector((0, -0.12, 1.8)), X, Z, 0.028, 0.028), (Vector((0, -0.15, 1.8)), X, Z, 0.034, 0.034), (Vector((0, -0.158, 1.8)), X, Z, 0.034, 0.034)],
           16, lambda i, k, p: messing_dunkel, KOPF_GEWICHT, oben_zu=False, unten_zu=True)
    f.kugel("Licht", Vector((0, -0.158, 1.8)), (0.029, 0.008, 0.029), farbe("#FFF2B0"), KOPF_GEWICHT, 14, 6)
    # Riemen vom Helm unters Kinn? Nein – ein Lederband ums Handgelenk genügt: Zunftabzeichen auf der Brust
    f.stern("Abzeichen", Vector((0.1, -0.142, 1.36)), Vector((0.2, -1, 0)), 0.022, messing, _rumpf, zacken=4)

    # Kiepe (Weidenkorb) auf dem Rücken mit Tragriemen
    kiepe_ringe = []
    for t, (z, rx, ry) in enumerate(((0.95, 0.13, 0.09), (1.1, 0.16, 0.11), (1.3, 0.19, 0.13), (1.52, 0.21, 0.14), (1.55, 0.215, 0.145))):
        kiepe_ringe.append((Vector((0, 0.27, z)), X, Y, rx, ry))

    def kiepe_farbe(i, k, p):
        if i > 3.3:
            return weide_dunkel                                               # Rand
        geflecht = (k + int(p.center.z * 40)) % 2
        return weide if geflecht else weide_dunkel * 1.15
    f.loft("Kiepe", kiepe_ringe, 28, kiepe_farbe, BRUST_GEWICHT, unten_zu=True, teilung=2)
    for s in (1, -1):
        riemen = [Vector((0.12 * s, 0.2, 1.46)), Vector((0.13 * s, 0.05, 1.53)), Vector((0.14 * s, -0.12, 1.42)), Vector((0.15 * s, -0.12, 1.2)),
                  Vector((0.14 * s, 0.1, 1.02))]
        f.loft("Tragriemen", [(p, X, Y, 0.018, 0.006) for p in riemen], 6, lambda i, k, p: leder, _rumpf, oben_zu=True, unten_zu=True, teilung=3)

    # Last: Erzbrocken in der Kiepe (rotbraun mit metallischem Glanz)
    anfang = len(f.teile)
    erz, erz_hell, erz_glanz = farbe("#8A4A2C"), farbe("#B8683C"), farbe("#E0C080")
    r = f.rng
    for n in range(11):
        w = r.uniform(0, math.tau)
        d = r.uniform(0, 0.13)
        mitte = Vector((math.cos(w) * d, 0.27 + math.sin(w) * d * 0.6, 1.53 + r.uniform(-0.02, 0.06)))
        groesse = r.uniform(0.045, 0.07)
        f.kugel("Erz", mitte, (groesse, groesse * r.uniform(0.8, 1.1), groesse * r.uniform(0.7, 1.0)),
                lambda poly, n=n: erz_glanz if poly.index % 7 == 0 else (erz_hell if (poly.index + n) % 3 == 0 else erz), BRUST_GEWICHT, 7, 5, glatt=False)
    f.als_starr("Last", "Brust", anfang)

    # Keilhaue: beim Tragen steckt sie hinten in der Kiepe
    _werkzeug(f, "erz", _ruecken_matrix(Vector((-0.12, 0.3, 1.0)), Vector((-0.1, 0.05, 1.0))))
    _skelett(f)
    return f.fertig(lambda armatur: _animationen(armatur, "erz"))


# ---------------------------------------------------------------------------
# Animationen
# ---------------------------------------------------------------------------
def _leeres(armatur, name, knochen, ort):
    """Leeres Objekt an einem Knochen, in Ruhelage an `ort` (Weltkoordinaten)."""
    e = bpy.data.objects.new(name, None)
    bpy.context.scene.collection.objects.link(e)
    e.parent = armatur
    e.parent_type = "BONE"
    e.parent_bone = knochen
    b = armatur.data.bones[knochen]
    e.matrix_parent_inverse = (armatur.matrix_world @ b.matrix_local @ Matrix.Translation((0, b.length, 0))).inverted()
    e.location = ort
    return e


def _ik_backen(armatur, laenge, arme):
    """Legt die Hände per IK an Punkte (am Werkzeugstiel oder an der Last) und backt das Ergebnis
    in die aktuelle Aktion. `arme` = Liste (Seite "L"/"R", Knochen des Ziels, Ziel in Ruhelage,
    Pol für den Ellbogen in Ruhelage)."""
    szene = bpy.context.scene
    hilfen, beschraenkungen = [], []
    for seite, knochen, ziel, pol in arme:
        z = _leeres(armatur, "IKZiel" + seite, knochen, ziel)
        p = _leeres(armatur, "IKPol" + seite, "Brust", pol)
        hilfen += [z, p]
        pb = armatur.pose.bones[f"Unterarm.{seite}"]
        c = pb.constraints.new("IK")
        c.target, c.pole_target, c.chain_count = z, p, 2
        # Polwinkel so wählen, dass der Ellbogen wirklich zum Pol zeigt
        bester = None
        for w in (-90, 0, 90, 180):
            c.pole_angle = math.radians(w)
            fehler = 0.0
            for bild in range(0, laenge + 1, max(1, laenge // 6)):
                szene.frame_set(bild)
                ellbogen = armatur.matrix_world @ pb.head
                fehler += (ellbogen - p.matrix_world.translation).length
            if bester is None or fehler < bester[0]:
                bester = (fehler, w)
        c.pole_angle = math.radians(bester[1])
        beschraenkungen.append((seite, c))
    # Erst alle Bilder auswerten (mit IK), dann die IK entfernen und als Schlüssel setzen
    werte = {}
    for bild in range(0, laenge + 1):
        szene.frame_set(bild)
        for seite, _ in beschraenkungen:
            for name in (f"Oberarm.{seite}", f"Unterarm.{seite}"):
                pb = armatur.pose.bones[name]
                lokal = armatur.convert_space(pose_bone=pb, matrix=pb.matrix, from_space="POSE", to_space="LOCAL")
                werte[(bild, name)] = lokal.to_quaternion()
    for seite, c in beschraenkungen:
        armatur.pose.bones[f"Unterarm.{seite}"].constraints.remove(c)
    for e in hilfen:
        bpy.data.objects.remove(e)
    vorher = {}
    for bild in range(0, laenge + 1):
        for seite, _ in beschraenkungen:
            for name in (f"Oberarm.{seite}", f"Unterarm.{seite}"):
                pb = armatur.pose.bones[name]
                euler = werte[(bild, name)].to_euler("XYZ", vorher[name]) if name in vorher else werte[(bild, name)].to_euler("XYZ")
                vorher[name] = euler
                pb.rotation_euler = euler
                pb.keyframe_insert(data_path="rotation_euler", frame=bild)


def _treffpunkt(armatur, bild, punkt):
    """Wo ein Punkt des Werkzeugs (Ruhelage) im Bild `bild` liegt (für die Abstände im Spiel)."""
    bpy.context.scene.frame_set(bild)
    pb = armatur.pose.bones["Hand.R"]
    b = armatur.data.bones["Hand.R"]
    return armatur.matrix_world @ pb.matrix @ b.matrix_local.inverted() @ punkt


def _animationen(armatur, art):
    stiel = lambda t: Vector((GRIFF_R.x, GRIFF_R.y, STIEL_OBEN + (KOPF_Z - STIEL_OBEN) * t))
    pol_l = Vector((0.55, 0.35, 1.05))
    pol_r = Vector((-0.55, 0.35, 1.05))

    # Idle: auf das Werkzeug gestützt stehen (Kopf am Boden neben dem rechten Fuß), atmen, umsehen
    def idle(phi):
        return [
            ("Brust", "rot", (1.5 * math.sin(phi * 2), 0, 0)), ("Bauch", "rot", (-0.8 * math.sin(phi * 2), 0, 0)),
            ("Kopf", "rot", (2 * math.sin(phi * 2 + 1), 22 * math.sin(phi), 0)),
            ("Oberarm.L", "rot", (-4, 0, 4)), ("Unterarm.L", "rot", (-14 - 3 * math.sin(phi * 2), 0, 0)),
            ("Oberarm.R", "rot", (-8, 0, 0)), ("Unterarm.R", "rot", (-6, 0, 0)),
            ("Oberschenkel.L", "rot", (-3, 0, 0)), ("Oberschenkel.R", "rot", (3, 0, 0)),
            ("Becken", "pos", (0, -0.004 * (1 - math.cos(phi * 2)), 0)),
        ]
    animation(armatur, "Idle", 180, _schleife(180, 6, idle))

    # Laufen: kräftiger Schritt, das Werkzeug auf der rechten Schulter
    def laufen(phi):
        werte = [w for w in _gehen(phi, 28, 42, 20, 0.03, 4, 14) if w[0] not in ("Oberarm.R", "Unterarm.R")]
        return werte + [("Oberarm.R", "rot", (-28 + 3 * math.sin(phi), 0, 0)), ("Unterarm.R", "rot", (-105, 0, 0)), ("Hand.R", "rot", (-82, 0, 0))]
    animation(armatur, "Laufen", 30, _schleife(30, 2, laufen))

    # Tragen: langsamer, schwerer Schritt; die Hände halten die Last (IK)
    def tragen(phi):
        werte = _gehen(phi, 20, 34, 12, 0.02, 2 if art not in ("stein", "lehm") else -4, 12)
        if art == "holz":
            werte = [w for w in werte if w[0] not in ("Oberarm.L", "Unterarm.L")] + [("Oberarm.L", "rot", (-40, 0, 0)), ("Unterarm.L", "rot", (-120, 0, 0))]
        else:
            werte = [w for w in werte if "arm" not in w[0]] + [("Oberarm.L", "rot", (-30, 0, 0)), ("Unterarm.L", "rot", (-80, 0, 0)),
                                                               ("Oberarm.R", "rot", (-30, 0, 0)), ("Unterarm.R", "rot", (-80, 0, 0))]
        return werte
    animation(armatur, "Tragen", 36, _schleife(36, 2, tragen))
    if art == "holz":
        _ik_backen(armatur, 36, [("L", "Brust", Vector((0.25, -0.32, 1.6)), pol_l + Vector((0, 0, -0.1)))])
    elif art in ("stein", "lehm"):
        _ik_backen(armatur, 36, [("L", "Brust", Vector((0.2, -0.3, 1.0)), pol_l), ("R", "Brust", Vector((-0.2, -0.3, 1.0)), pol_r)])
    else:
        _ik_backen(armatur, 36, [("L", "Brust", Vector((0.14, -0.15, 1.34)), pol_l), ("R", "Brust", Vector((-0.14, -0.15, 1.34)), pol_r)])

    # Bücken: in die Knie, Oberkörper vor, mit den Armen zum Boden, wieder hoch (Aufheben/Abladen)
    stehen = {"Oberarm.L": (0, 0, 0), "Unterarm.L": (-10, 0, 0), "Oberarm.R": (-5, 0, 0), "Unterarm.R": (-8, 0, 0)}
    unten = {"Becken.pos": (0, 0, -0.3), "Oberschenkel.L": (-80, 0, 0), "Oberschenkel.R": (-70, 0, 0), "Unterschenkel.L": (112, 0, 0),
             "Unterschenkel.R": (104, 0, 0), "Fuss.L": (-32, 0, 0), "Fuss.R": (-34, 0, 0), "Bauch": (22, 0, 0), "Brust": (24, 0, 0), "Kopf": (-26, 0, 0),
             "Oberarm.L": (-45, 0, 0), "Unterarm.L": (-25, 0, 0), "Oberarm.R": (-40, 0, 0), "Unterarm.R": (-25, 0, 0)}
    _clip(armatur, "Buecken", 40, [(0, stehen), (13, unten), (25, _mit(unten, Brust=(20, 0, 0))), (40, stehen)])

    # Arbeiten: der Schlag in Schleife, die linke Hand greift den Stiel (IK)
    beine = {"Oberschenkel.L": (-14, 0, 0), "Unterschenkel.L": (16, 0, 0), "Oberschenkel.R": (8, 0, 0), "Unterschenkel.R": (12, 0, 0),
             "Fuss.L": (-2, 0, 0), "Fuss.R": (-6, 0, 0)}
    if art == "holz":
        # Schräger Schlag von rechts oben in den Stamm (Einschlag bei Bild 22)
        ausgeholt = dict(beine, **{"Brust": (4, -38, 0), "Bauch": (2, -18, 0), "Becken": (0, -10, 0), "Kopf": (-4, 30, 0),
                                   "Oberarm.R": (-150, 0, 0), "Unterarm.R": (-55, 0, 0), "Hand.R": (-10, 0, 0)})
        oben = _mit(ausgeholt, Brust=(0, -46, 0), Oberarm_R=(-162, 0, 0), Unterarm_R=(-62, 0, 0))
        schlag = dict(beine, **{"Brust": (14, -6, 0), "Bauch": (8, -3, 0), "Becken": (0, 0, 0), "Kopf": (-12, 4, 0),
                                "Oberarm.R": (-82, 0, 0), "Unterarm.R": (-10, 0, 0), "Hand.R": (8, 0, 0), "Becken.pos": (0, 0, -0.04)})
        nach = _mit(schlag, Brust=(16, 0, 0), Oberarm_R=(-78, 0, 0))
        zurueck = _mit(ausgeholt, Brust=(8, -12, 0), Oberarm_R=(-118, 0, 0), Unterarm_R=(-30, 0, 0))
        posen = [(0, ausgeholt), (15, oben), (19, _mit(oben, Oberarm_R=(-140, 0, 0), Brust=(4, -20, 0))), (22, schlag), (26, nach), (31, zurueck),
                 (36, ausgeholt)]
        laenge, einschlag = 36, 22
    elif art == "lehm":
        # Spaten: ansetzen, mit dem Fuß hineintreten, Lehm heraushebeln und zur Seite werfen (Stich bei Bild 14)
        bereit = dict(beine, **{"Brust": (16, 0, 0), "Bauch": (6, 0, 0), "Oberarm.R": (-38, 0, 0), "Unterarm.R": (-32, 0, 0), "Hand.R": (8, 0, 0)})
        stich = {"Oberschenkel.L": (-42, 0, 0), "Unterschenkel.L": (58, 0, 0), "Fuss.L": (6, 0, 0), "Oberschenkel.R": (6, 0, 0), "Unterschenkel.R": (14, 0, 0),
                 "Brust": (26, 0, 0), "Bauch": (10, 0, 0), "Kopf": (-14, 0, 0), "Oberarm.R": (-22, 0, 0), "Unterarm.R": (-14, 0, 0), "Hand.R": (10, 0, 0),
                 "Becken.pos": (0, 0, -0.05)}
        hebeln = dict(beine, **{"Brust": (4, 0, 0), "Bauch": (0, 0, 0), "Kopf": (-4, 0, 0), "Oberarm.R": (-58, 0, 0), "Unterarm.R": (-48, 0, 0), "Hand.R": (-12, 0, 0)})
        werfen = dict(beine, **{"Brust": (6, 26, 0), "Bauch": (2, 10, 0), "Kopf": (-4, 12, 0), "Oberarm.R": (-86, 0, 0), "Unterarm.R": (-30, 0, 0), "Hand.R": (-30, 0, 0)})
        posen = [(0, bereit), (8, _mit(bereit, Brust=(20, 0, 0), Oberarm_R=(-30, 0, 0))), (14, stich), (18, _mit(stich, Oberarm_R=(-26, 0, 0))), (26, hebeln),
                 (32, werfen), (36, _mit(werfen, Oberarm_R=(-70, 0, 0))), (42, bereit)]
        laenge, einschlag = 42, 14
    else:
        # Über den Kopf ausholen und mit ganzer Kraft nach vorne unten schlagen (Einschlag bei Bild 27)
        bereit = dict(beine, **{"Brust": (12, 0, 0), "Bauch": (5, 0, 0), "Oberarm.R": (-42, 0, 0), "Unterarm.R": (-22, 0, 0)})
        hoch = dict(beine, **{"Brust": (-12, 0, 0), "Bauch": (-5, 0, 0), "Kopf": (-4, 0, 0), "Oberarm.R": (-168, 0, 0), "Unterarm.R": (-38, 0, 0)})
        schlag = {"Oberschenkel.L": (-28, 0, 0), "Unterschenkel.L": (34, 0, 0), "Oberschenkel.R": (-8, 0, 0), "Unterschenkel.R": (26, 0, 0),
                  "Fuss.L": (-6, 0, 0), "Fuss.R": (-14, 0, 0), "Brust": (20, 0, 0), "Bauch": (8, 0, 0), "Kopf": (-20, 0, 0),
                  "Oberarm.R": (-78, 0, 0), "Unterarm.R": (-6, 0, 0), "Becken.pos": (0, 0, -0.08)}
        posen = [(0, bereit), (14, hoch), (21, _mit(hoch, Oberarm_R=(-174, 0, 0), Brust=(-14, 0, 0))), (27, schlag),
                 (31, _mit(schlag, Oberarm_R=(-82, 0, 0), Brust=(18, 0, 0))), (37, _mit(bereit, Oberarm_R=(-62, 0, 0), Brust=(16, 0, 0))), (42, bereit)]
        laenge, einschlag = 42, 27
    _clip(armatur, "Arbeiten", laenge, posen)
    _ik_backen(armatur, laenge, [("L", "Hand.R", stiel(0.34), pol_l)])
    treff = _treffpunkt(armatur, einschlag, stiel(1.0))
    print(f"TREFFPUNKT {art}: vorne {-treff.y:.2f} m, seitlich {treff.x:.2f} m, Höhe {treff.z:.2f} m")


# ---------------------------------------------------------------------------
# Lehmarbeiter
# ---------------------------------------------------------------------------
def lehmarbeiter(seed=74, name="Lehmarbeiter"):
    """Junger Lehmstecher: breiter Strohhut, Leinenhemd mit hochgekrempelten Ärmeln, grünes
    Halstuch, hochgekrempelte braune Hose, lehmverschmierte Stiefel und Schürze; Spaten. Trägt
    einen gestochenen Lehmblock vor dem Bauch."""
    f = Figur(name, seed)
    r = f.rng
    haut, lippe, wange = farbe("#D8A07A"), farbe("#B8806A"), farbe("#D8866E")
    haar = farbe("#C88A3A")
    leinen = farbe("#E8DEC8")
    braun, braun_dunkel = farbe("#7A5A3A"), farbe("#5A402A")
    lehm, lehm_dunkel = farbe("#D8B070"), farbe("#A87E4A")
    gruen = farbe("#4E8A4A")
    stroh, stroh_dunkel = farbe("#E8CC7A"), farbe("#B89A4A")

    def hemd(p):
        c = leinen * (0.92 + 0.08 * max(0.0, -p.normal.y))
        # Lehmspritzer
        if math.sin(p.center.x * 90 + p.center.z * 70) * math.sin(p.center.z * 55 - p.center.x * 30) > 0.92:
            c = lehm
        return c

    def hose(p, s):
        c = braun_dunkel if p.normal.x * s > 0.85 else braun * (0.94 + 0.08 * max(0.0, -p.normal.y))
        # Unten hochgekrempelt und voller Lehm
        if p.center.z < 0.32:
            return lehm.lerp(lehm_dunkel, 0.3 + 0.3 * math.sin(p.center.x * 80))
        if 0.32 <= p.center.z < 0.4:
            return braun * 1.15
        return c

    _koerper(f, haut, braun, hose, hemd, lehm_dunkel, farbe("#3A2A1A"), farbe("#B08A5A"), leinen, lambda p: leinen * 0.95, True)
    # Umgeschlagene Hosenbeine unter dem Knie
    for s in (1, -1):
        mitte = _spiegel(Vector((0.1, 0.0, 0.36)), s)
        f.loft("Hosenumschlag", [(mitte + Vector((0, 0.0, -0.02)), X, Y, 0.068, 0.068), (mitte + Vector((0, 0.0, 0.03)), X, Y, 0.07, 0.07)], 18,
               lambda i, k, p: braun * 1.1, _bein(s), teilung=1, glatt=True)
    # Kurze Schürze voller Lehm, Nackenriemen, grünes Halstuch
    schuerze = []
    for t, (z, y, breite) in enumerate(((1.25, -0.142, 0.14), (1.1, -0.15, 0.16), (0.95, -0.146, 0.175), (0.8, -0.156, 0.18), (0.66, -0.166, 0.176))):
        schuerze.append((Vector((0, y, z)), X, Y, breite, 0.011, lambda w, t=t: 1.0 + 0.03 * math.sin(w * 3 + t)))

    def schuerze_farbe(i, k, p):
        c = farbe("#8A7A62") * (0.92 + 0.08 * math.sin(p.center.z * 30))
        return c.lerp(lehm, 0.55 * weich(1.0, 0.7, p.center.z) + 0.3 * max(0.0, math.sin(p.center.x * 60 + p.center.z * 20)))
    f.loft("Schuerze", schuerze, 16, schuerze_farbe, _huefte_bein_schuerze, oben_zu=True, unten_zu=True, teilung=3, glatt=True)
    for s in (1, -1):
        f.loft("Nackenriemen", [(Vector((0.11 * s, -0.135, 1.25)), X, Y, 0.012, 0.005), (Vector((0.1 * s, -0.12, 1.42)), X, Y, 0.012, 0.005),
                                (Vector((0.06 * s, 0.03, 1.56)), X, Y, 0.012, 0.005)], 6, lambda i, k, p: braun_dunkel, _rumpf, oben_zu=True, unten_zu=True, teilung=2)
    f.loft("Schuerzenband", [(Vector((0, 0.008, 1.05)), X, Y, 0.182, 0.14), (Vector((0, 0.008, 1.075)), X, Y, 0.181, 0.139)], 40, lambda i, k, p: braun_dunkel, _rumpf)
    f.loft("Halstuch", [(Vector((0, 0.012, 1.53)), X, Y, 0.1, 0.085), (Vector((0, 0.012, 1.565)), X, Y, 0.075, 0.07)], 24,
           lambda i, k, p: gruen * (0.85 if k % 4 == 0 else 1.0), _rumpf, teilung=2, glatt=True)
    f.kugel("Knoten", Vector((0.04, -0.08, 1.52)), (0.022, 0.018, 0.02), gruen * 0.9, _rumpf, 10, 6)
    # Lehm an den Unterarmen
    for s in (1, -1):
        f.kugel("Lehmfleck", _spiegel(Vector((0.3, -0.03, 1.02)), s), (0.035, 0.03, 0.05), lambda poly: lehm * (0.9 + 0.1 * (poly.index % 2)), _arm(s), 8, 5)

    _kopf(f, haut, lippe, wange, haar, "keiner")
    # Breiter Strohhut mit Band
    hut = []
    for z, rx, ry, y in ((1.73, 0.105, 0.115, 0.004), (1.77, 0.108, 0.117, 0.008), (1.81, 0.1, 0.11, 0.012), (1.84, 0.08, 0.088, 0.016), (1.852, 0.03, 0.034, 0.018)):
        hut.append((Vector((0, y, z)), X, Y, rx, ry))
    f.loft("Hut", hut, 32, lambda i, k, p: stroh * (0.85 + 0.15 * ((k + int(p.center.z * 120)) % 2)), KOPF_GEWICHT, oben_zu=True, teilung=2, glatt=True)
    f.loft("Krempe", [(Vector((0, 0.004, 1.74)), X, Y, 0.108, 0.118), (Vector((0, 0.0, 1.73)), X, Y, 0.2, 0.21), (Vector((0, 0.0, 1.705)), X, Y, 0.235, 0.245)],
           32, lambda i, k, p: stroh_dunkel if k % 3 == 0 else stroh * 0.95, KOPF_GEWICHT, teilung=1, glatt=True)
    f.loft("Hutband", [(Vector((0, 0.004, 1.745)), X, Y, 0.109, 0.119), (Vector((0, 0.006, 1.772)), X, Y, 0.11, 0.12)], 32, lambda i, k, p: gruen, KOPF_GEWICHT)

    # Last: ein gestochener Lehmblock vor dem Bauch
    anfang = len(f.teile)
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.bevel(bm, geom=bm.edges[:], offset=0.03, segments=2, affect="EDGES")
    for v in bm.verts:
        v.co = Vector((v.co.x * 0.36, v.co.y * 0.24, v.co.z * 0.22)) + Vector((0, -0.3, 1.02))
        v.co += Vector((r.uniform(-1, 1), r.uniform(-1, 1), r.uniform(-1, 1))) * 0.006

    def block(poly):
        n = poly.normal
        return (lehm if n.z > 0.5 else lehm_dunkel.lerp(farbe("#A8B6BC"), 0.3)) * (0.9 + 0.1 * max(0.0, n.z))
    f._objekt(bm, "Lehmblock", block, BRUST_GEWICHT)
    f.als_starr("Last", "Brust", anfang)

    _werkzeug(f, "lehm", _ruecken_matrix(Vector((0.18, 0.2, 0.9)), Vector((-0.38, 0.05, 0.93))))
    _skelett(f)
    return f.fertig(lambda armatur: _animationen(armatur, "lehm"))
