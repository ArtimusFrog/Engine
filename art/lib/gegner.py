"""Gegnerische Einheiten der Schattenfestung: Dunkler Ritter, Bogenschütze, Pikenier,
Skelettkrieger, Dunkelmagier, Steingolem, Schattenwolf und Gespenst.

Alle Zweibeiner teilen Skelett und Grundkörper (Gelenke wie beim Magier, figuren.py); Ausrüstung,
Farben und die Angriffsanimation unterscheiden sich. Der Schattenwolf ist ein Vierbeiner aus
tiere.py. Jede Einheit hat die Animationen Idle, Laufen und Angriff (dazu Rennen beim Wolf).
Farben der Festung: geschwärzter Stahl, dunkles Violett, glühende violette und grüne Akzente.
Koordinaten wie bei allen Figuren: Z oben, Blick nach -Y, Füße im Ursprung.
"""

import math

import bmesh
from mathutils import Matrix, Vector

import tiere
from figuren import (ELLBOGEN, FINGER, HAENGT, HANDGELENK, HOCH, HUEFTE, KNIE, KNOECHEL, SCHULTER, STAB_X, STAB_Y, X, Y, Z, ZEHEN,
                     Figur, _arm_gewichte, _gehen, _mischen, _robe_gewichte, _rumpf_gewichte, _schleife, _spiegel, farbe, weich)
from werkstatt import animation

STAHL_SCHWARZ = farbe("#3B3E47")
STAHL_DUNKEL = farbe("#555A66")
STAHL = farbe("#7E8594")
VIOLETT = farbe("#4A2466")
VIOLETT_DUNKEL = farbe("#2E163F")
VIOLETT_HELL = farbe("#8E4FC4")
GLUT = farbe("#D77BFF")
GIFT = farbe("#8CFF9E")
ROT = farbe("#7E2227")
LEDER = farbe("#4B3223")
LEDER_HELL = farbe("#6E4A32")
HOLZ = farbe("#5A3E28")
KNOCHEN = farbe("#DCD4C0")
KNOCHEN_DUNKEL = farbe("#A99F88")
ROST = farbe("#7A4A2E")
SCHATTEN = farbe("#0E0B12")

KOPF = lambda co: {"Kopf": 1.0}
HAND_L = lambda co: {"Hand.L": 1.0}
HAND_R = lambda co: {"Hand.R": 1.0}
GRIFF_R = Vector((STAB_X, STAB_Y, 0.9))      # wo die rechte Faust eine Waffe hält
GRIFF_L = Vector((0.37, -0.075, 0.9))


def _bein_gewichte(sn):
    return lambda co: _mischen((f"Unterschenkel.{sn}", 1 - weich(0.45, 0.6, co.z)), (f"Oberschenkel.{sn}", weich(0.45, 0.6, co.z)))


def _ring(mitte, r1, r2=None, form=None):
    ring = (Vector(mitte), X, Y, r1, r2 if r2 is not None else r1)
    return ring + (form,) if form else ring


def _strecke(f, name, a, b, r_a, r_b, c, gewichte, segmente=8, zu=True):
    """Rundes, sich verjüngendes Stück von a nach b (Schaft, Knochen, Stange)."""
    a, b = Vector(a), Vector(b)
    achse = (b - a).normalized()
    quer = achse.cross(Z if abs(achse.z) < 0.9 else X).normalized()
    zweite = achse.cross(quer).normalized()
    farbe_von = c if callable(c) else (lambda i, k, p: c)
    return f.loft(name, [(a, quer, zweite, r_a, r_a), (b, quer, zweite, r_b, r_b)], segmente, farbe_von, gewichte, oben_zu=zu, unten_zu=zu)


def _platte(f, name, umriss, dicke, ursprung, u, v, c, gewichte):
    """Flache Platte (Klinge, Schild) aus einem 2D-Umriss in der Ebene (u, v) um `ursprung`."""
    ursprung, u, v = Vector(ursprung), Vector(u), Vector(v)
    n = u.cross(v).normalized()
    bm = bmesh.new()
    vorne = [bm.verts.new(ursprung + u * a + v * b + n * dicke / 2) for a, b in umriss]
    hinten = [bm.verts.new(ursprung + u * a + v * b - n * dicke / 2) for a, b in umriss]
    for i in range(len(umriss)):
        j = (i + 1) % len(umriss)
        bm.faces.new((vorne[i], vorne[j], hinten[j], hinten[i]))
    bm.faces.new(vorne)
    bm.faces.new(list(reversed(hinten)))
    return f._objekt(bm, name, c if callable(c) else (lambda poly: c), gewichte)


# ---------------------------------------------------------------------------
# Grundkörper
# ---------------------------------------------------------------------------
def _beine(f, hose, stiefel, dick=1.0, schienen=None):
    for seite in (1, -1):
        sn = "L" if seite > 0 else "R"
        k_, h_ = _spiegel(KNIE, seite), _spiegel(HUEFTE, seite)
        x = 0.1 * seite
        ringe = [_ring((x, 0.02, 0.1), 0.055 * dick, 0.058 * dick), _ring((x, 0.015, 0.24), 0.06 * dick, 0.064 * dick),
                 _ring((x, 0.0, 0.4), 0.066 * dick, 0.07 * dick), _ring(k_ + Vector((0, 0, 0.02)), 0.066 * dick, 0.07 * dick),
                 _ring((x, 0.0, 0.75), 0.082 * dick, 0.086 * dick), _ring(h_ + Vector((0, 0, -0.03)), 0.094 * dick, 0.094 * dick)]
        farbe_bein = (lambda i, k, p: (schienen if (schienen is not None and i < 3.2) else hose) * (0.9 if 2.8 < i < 3.3 else 1.0))
        f.loft("Bein", ringe, 14, farbe_bein, _bein_gewichte(sn), teilung=2, glatt=True)
        schuh = [(Vector((x, 0.06, 0.05)), X, Z, 0.055 * dick, 0.05), (Vector((x, 0.05, 0.02)), X, Z, 0.062 * dick, 0.03),
                 (Vector((x, -0.02, 0.06)), X, Z, 0.064 * dick, 0.065), (Vector((x, -0.09, 0.045)), X, Z, 0.052 * dick, 0.042),
                 (Vector((x, -0.16, 0.04)), X, Z, 0.03 * dick, 0.024)]
        f.loft("Stiefel", schuh, 12, lambda i, k, p: stiefel, lambda co, sn=sn: {f"Fuss.{sn}": 1.0}, oben_zu=True, unten_zu=True)
        f.loft("Schaft", [_ring((x, 0.01, 0.08), 0.064 * dick), _ring((x, 0.01, 0.3), 0.07 * dick)], 12, lambda i, k, p: stiefel * 1.1,
               _bein_gewichte(sn), oben_zu=False, unten_zu=False)


def _rumpf(f, farbe_von, dick=1.0, robe=False):
    ringe = [_ring((0, 0.0, 0.92), 0.17 * dick, 0.13 * dick), _ring((0, 0.0, 1.08), 0.16 * dick, 0.125 * dick),
             _ring((0, -0.01, 1.25), 0.19 * dick, 0.15 * dick), _ring((0, -0.005, 1.42), 0.215 * dick, 0.155 * dick),
             _ring((0, 0.0, 1.53), 0.19 * dick, 0.135 * dick), _ring((0, 0.005, 1.6), 0.08, 0.075)]
    f.loft("Rumpf", ringe, 28, farbe_von, _rumpf_gewichte, oben_zu=True, teilung=2, glatt=True)
    f.loft("Hals", [_ring((0, 0.0, 1.55), 0.07), _ring((0, 0.0, 1.68), 0.06)], 12, lambda i, k, p: farbe_von(0, 0, p) * 0.8,
           lambda co: _mischen(("Kopf", weich(1.6, 1.66, co.z)), ("Hals", 1 - weich(1.6, 1.66, co.z))))


def _rock(f, farbe_von, laenge=0.5, weite=1.0):
    """Waffenrock/Robe von der Taille abwärts (schwingt mit den Beinen)."""
    unten = 0.95 - laenge
    ringe = [_ring((0, 0.0, 1.0), 0.18, 0.14), _ring((0, 0.0, 0.85), 0.2 * weite, 0.16 * weite),
             _ring((0, 0.01, 0.95 - laenge * 0.6), 0.22 * weite, 0.18 * weite), _ring((0, 0.015, unten), 0.24 * weite, 0.2 * weite)]
    f.loft("Rock", ringe, 32, farbe_von, _robe_gewichte, teilung=2)


def _arme(f, farbe_arm, handschuh, dick=1.0, schulter=None):
    for seite in (1, -1):
        s, e, h = _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite)
        ringe = []
        for t, (a, b), radius in ((0.0, (s, e), 0.072), (0.6, (s, e), 0.066), (1.0, (s, e), 0.06), (0.4, (e, h), 0.055),
                                  (0.85, (e, h), 0.058), (1.0, (e, h), 0.064)):
            ax = (b - a).normalized()
            q = ax.cross(Y).normalized()
            ringe.append((a.lerp(b, t), q, ax.cross(q).normalized(), radius * dick, radius * dick * 0.95))
        f.loft("Arm", ringe, 12, lambda i, k, p: farbe_arm * (0.9 if i >= 3 else 1.0), _arm_gewichte(seite), teilung=2, glatt=True)
        if schulter is not None:
            for j in range(2):
                mitte = s + Vector((0.045 * seite, 0.0, 0.05 - j * 0.06))
                f.kugel("Schulter", mitte, ((0.12 - j * 0.01) * dick, (0.115 - j * 0.01) * dick, 0.07), schulter if j == 0 else schulter * 0.85,
                        lambda co, seite=seite: _mischen(("Brust", 0.45), (f"Oberarm.{'L' if seite > 0 else 'R'}", 0.55)), 12, 6)
    f.kugel("FaustL", Vector((0.352, -0.02, 0.9)), (0.045 * dick, 0.05 * dick, 0.065 * dick), handschuh, HAND_L, 10, 6, glatt=False)
    f.kugel("FaustR", Vector((STAB_X + 0.015, STAB_Y + 0.012, 0.915)), (0.045 * dick, 0.05 * dick, 0.06 * dick), handschuh, HAND_R, 10, 6,
            glatt=False)


def _augen(f, farbe_auge, z=1.76, abstand=0.042, vorne=-0.105, groesse=0.018):
    for sx in (-1, 1):
        f.kugel("Auge", Vector((sx * abstand, vorne, z)), (groesse, groesse * 0.6, groesse * 0.7), farbe_auge, KOPF, 8, 6)


def _kapuze(f, stoff, innen=SCHATTEN, spitz=0.0):
    """Kapuze mit tiefem Schatten vorne (das Gesicht bleibt im Dunkeln)."""
    def kapuze_farbe(i, k, p):
        return innen if (-p.normal.y > 0.55 and p.center.z < 1.86 and abs(p.center.x) < 0.09) else stoff * (0.9 + 0.12 * p.normal.z)
    ringe = [_ring((0, 0.02, 1.58), 0.16, 0.15), _ring((0, 0.03, 1.68), 0.14, 0.14), _ring((0, 0.03, 1.8), 0.135, 0.14),
             _ring((0, 0.05, 1.9), 0.11, 0.12), _ring((0, 0.08 + spitz * 0.1, 1.98 + spitz * 0.08), 0.05, 0.06)]
    f.loft("Kapuze", ringe, 20, kapuze_farbe, KOPF, oben_zu=True, teilung=2)
    f.kugel("Gesicht", Vector((0, -0.02, 1.75)), (0.09, 0.08, 0.11), SCHATTEN, KOPF, 10, 6)


def _skelett(f, groesse=1.0):
    g = lambda p: tuple(Vector(p) * groesse)
    f.knochen_dazu("Becken", g((0, 0, 0.95)), g((0, 0, 1.1)), None, HOCH)
    f.knochen_dazu("Bauch", g((0, 0, 1.1)), g((0, 0, 1.3)), "Becken", HOCH)
    f.knochen_dazu("Brust", g((0, 0, 1.3)), g((0, 0, 1.56)), "Bauch", HOCH)
    f.knochen_dazu("Hals", g((0, 0, 1.56)), g((0, 0, 1.65)), "Brust", HOCH)
    f.knochen_dazu("Kopf", g((0, 0, 1.65)), g((0, 0, 1.9)), "Hals", HOCH)
    f.knochen_dazu("Hut", g((0, 0.02, 1.98)), g((0, 0.2, 2.05)), "Kopf", HOCH)
    for seite, sn in ((1, "L"), (-1, "R")):
        f.knochen_dazu(f"Oberarm.{sn}", g(_spiegel(SCHULTER, seite)), g(_spiegel(ELLBOGEN, seite)), "Brust", HAENGT)
        f.knochen_dazu(f"Unterarm.{sn}", g(_spiegel(ELLBOGEN, seite)), g(_spiegel(HANDGELENK, seite)), f"Oberarm.{sn}", HAENGT)
        f.knochen_dazu(f"Hand.{sn}", g(_spiegel(HANDGELENK, seite)), g(_spiegel(FINGER, seite)), f"Unterarm.{sn}", HAENGT)
        f.knochen_dazu(f"Oberschenkel.{sn}", g(_spiegel(HUEFTE, seite)), g(_spiegel(KNIE, seite)), "Becken", HAENGT)
        f.knochen_dazu(f"Unterschenkel.{sn}", g(_spiegel(KNIE, seite)), g(_spiegel(KNOECHEL, seite)), f"Oberschenkel.{sn}", HAENGT)
        f.knochen_dazu(f"Fuss.{sn}", g(_spiegel(KNOECHEL, seite)), g(_spiegel(ZEHEN, seite)), f"Unterschenkel.{sn}", (0, 0, 1))


def _skalieren(f, groesse):
    """Ganze Figur (Teile und Anbauteile) vergrößern – vor `_skelett(f, groesse)`."""
    m = Matrix.Scale(groesse, 4)
    for obj in f.teile:
        obj.data.transform(m)
    for _, _, objekte in f.starr:
        for obj in objekte:
            obj.data.transform(m)


# ---------------------------------------------------------------------------
# Waffen (als starre Anbauteile an der Hand)
# ---------------------------------------------------------------------------
def _schwert(f, rostig=False, laenge=0.85):
    anfang = len(f.teile)
    richtung = Vector((0, -0.62, -0.78)).normalized()     # in Ruhe schräg nach vorne unten
    quer = Vector((1, 0, 0))
    griff = GRIFF_R
    klinge_farbe = ROST if rostig else STAHL
    _strecke(f, "Griff", griff - richtung * 0.12, griff + richtung * 0.05, 0.016, 0.016, LEDER, HAND_R)
    f.kugel("Knauf", griff - richtung * 0.13, (0.025, 0.025, 0.025), STAHL_DUNKEL, HAND_R, 8, 6)
    f.kiste("Parier", griff + richtung * 0.07, (0.2, 0.03, 0.03), STAHL_DUNKEL, HAND_R)
    v = richtung.cross(quer).normalized()
    umriss = [(-0.035, 0.08), (0.035, 0.08), (0.03, laenge - 0.1), (0.0, laenge), (-0.03, laenge - 0.1)]
    if rostig:
        umriss = [(-0.035, 0.08), (0.035, 0.08), (0.04, 0.4), (0.02, 0.45), (0.035, laenge - 0.15), (0.0, laenge - 0.02),
                  (-0.03, laenge - 0.2), (-0.04, 0.5)]
    _platte(f, "Klinge", [(a, b) for a, b in umriss], 0.012, griff, quer, richtung,
            lambda poly: klinge_farbe * (1.15 if abs(poly.normal.x) < 0.5 else 1.0), HAND_R)
    del v
    f.als_starr("Schwert", "Hand.R", anfang)


def _schild(f, farbe_aussen, farbe_rand, zeichen=None, rund=False, kaputt=False):
    anfang = len(f.teile)
    mitte = Vector((0.43, -0.03, 1.02))
    if rund:
        umriss = [(math.cos(math.tau * k / 14) * 0.26, math.sin(math.tau * k / 14) * 0.26) for k in range(14)]
        if kaputt:
            umriss = [(y, z) for y, z in umriss if not (y > 0.1 and z > 0.05)] + [(0.05, 0.05)]
    else:
        umriss = [(0.0, 0.32), (0.19, 0.28), (0.21, 0.08), (0.14, -0.2), (0.0, -0.38), (-0.14, -0.2), (-0.21, 0.08), (-0.19, 0.28)]
    schild = lambda co: {"Unterarm.L": 1.0}
    _platte(f, "Schild", umriss, 0.045, mitte, Y, Z,
            lambda poly: farbe_aussen if poly.normal.x > 0.5 else (LEDER if poly.normal.x < -0.5 else farbe_rand), schild)
    if zeichen is not None:
        _platte(f, "Zeichen", [(0.0, 0.14), (0.08, 0.0), (0.0, -0.18), (-0.08, 0.0)], 0.01, mitte + Vector((0.028, 0, 0.02)), Y, Z,
                lambda poly: zeichen, schild)
    f.als_starr("Schild", "Unterarm.L", anfang)


# ---------------------------------------------------------------------------
# Animationen
# ---------------------------------------------------------------------------
def _schluessel(bilder, abbildung):
    """Schlüsselbilder aus Zeilen (Bild, Werte…) über eine Abbildung Werte → [(Knochen, art, xyz)]."""
    liste = []
    for zeile in bilder:
        bild, werte = zeile[0], zeile[1:]
        for knochen, art, wert in abbildung(*werte):
            liste.append((bild, knochen, art, wert))
    return liste


def _angriff_hieb():
    # Schwert über die rechte Schulter heben und schräg niederschlagen, linker Arm (Schild) vor dem Körper
    return 30, _schluessel(((0, 0, -10, 0, 0, 6), (9, -150, -40, -8, 25, 10), (13, -160, -30, -10, 30, 12), (17, -40, -10, 22, -25, 25),
                            (22, -30, -20, 16, -18, 20), (30, 0, -10, 0, 0, 6)),
                           lambda arm, ell, bx, by, knie: [
                               ("Oberarm.R", "rot", (arm, 0, 0)), ("Unterarm.R", "rot", (ell, 0, 0)),
                               ("Oberarm.L", "rot", (-20, 0, 8)), ("Unterarm.L", "rot", (-60, 0, 0)),
                               ("Brust", "rot", (bx, by, 0)), ("Bauch", "rot", (bx * 0.4, by * 0.4, 0)),
                               ("Oberschenkel.L", "rot", (-knie * 0.6, 0, 0)), ("Unterschenkel.L", "rot", (knie, 0, 0)),
                               ("Oberschenkel.R", "rot", (knie * 0.3, 0, 0)), ("Unterschenkel.R", "rot", (knie * 0.6, 0, 0))])


def _angriff_stoss():
    # Speer zurücknehmen, dann mit einem Ausfallschritt nach vorne stoßen
    return 30, _schluessel(((0, 0, -10, 0, 0, 0), (8, 25, -70, -5, 25, 0), (13, -65, -15, 18, -25, 1), (18, -60, -12, 16, -22, 1),
                            (30, 0, -10, 0, 0, 0)),
                           lambda arm, ell, bx, by, schritt: [
                               ("Oberarm.R", "rot", (arm, 0, 0)), ("Unterarm.R", "rot", (ell, 0, 0)),
                               ("Oberarm.L", "rot", (arm * 0.6, 0, 10)), ("Unterarm.L", "rot", (-40, 0, 0)),
                               ("Brust", "rot", (bx, by, 0)), ("Bauch", "rot", (bx * 0.4, by * 0.4, 0)),
                               ("Oberschenkel.L", "rot", (-35 * schritt, 0, 0)), ("Unterschenkel.L", "rot", (6 + 30 * schritt, 0, 0)),
                               ("Oberschenkel.R", "rot", (20 * schritt, 0, 0)), ("Unterschenkel.R", "rot", (6 + 10 * schritt, 0, 0)),
                               ("Becken", "pos", (0, 0, -0.06 * schritt))])


def _angriff_schuss():
    # Bogen heben (links nach vorne), Sehne mit rechts zum Kinn ziehen, loslassen, nachhalten
    return 40, _schluessel(((0, 0, -10, 0, -10, 0), (10, -86, -4, -80, -40, 30), (20, -88, -4, -82, -125, 34), (25, -88, -4, -70, -95, 30),
                            (32, -86, -4, -60, -60, 28), (40, 0, -10, 0, -10, 0)),
                           lambda arm_l, ell_l, arm_r, ell_r, dreh: [
                               ("Oberarm.L", "rot", (arm_l, 0, 8)), ("Unterarm.L", "rot", (ell_l, 0, 0)),
                               ("Oberarm.R", "rot", (arm_r, 0, -12)), ("Unterarm.R", "rot", (ell_r, 0, 0)),
                               ("Brust", "rot", (0, dreh, 0)), ("Bauch", "rot", (0, dreh * 0.4, 0)), ("Kopf", "rot", (0, -dreh * 0.8, 0))])


def _angriff_zauber():
    # Stab hochreißen, Kraft sammeln, nach vorne stoßen; die linke Hand zeigt aufs Ziel
    return 32, _schluessel(((0, 0, -6, 0, -12, 0, 0), (6, -120, -55, -35, -40, -8, 18), (11, -85, -8, -82, -8, 12, -10),
                            (17, -80, -6, -78, -10, 14, -12), (24, -40, -10, -35, -20, 6, -5), (32, 0, -6, 0, -12, 0, 0)),
                           lambda arm_r, ell_r, arm_l, ell_l, bx, by: [
                               ("Oberarm.R", "rot", (arm_r, 0, 0)), ("Unterarm.R", "rot", (ell_r, 0, 0)),
                               ("Oberarm.L", "rot", (arm_l, 0, 12)), ("Unterarm.L", "rot", (ell_l, 0, 0)),
                               ("Brust", "rot", (bx, by, 0)), ("Bauch", "rot", (bx * 0.4, by * 0.4, 0)), ("Hut", "rot", (-bx * 0.6, 0, 0))])


def _angriff_stampfen():
    # Beide Fäuste hoch über den Kopf, dann mit ganzer Wucht auf den Boden schmettern
    return 36, _schluessel(((0, 0, -10, 0, 0), (12, -165, -25, -14, 4), (17, -150, -20, -16, 6), (21, -40, -6, 34, 28), (27, -35, -8, 30, 25),
                            (36, 0, -10, 0, 0)),
                           lambda arm, ell, bx, knie: [
                               ("Oberarm.R", "rot", (arm, 0, 10)), ("Unterarm.R", "rot", (ell, 0, 0)),
                               ("Oberarm.L", "rot", (arm, 0, -10)), ("Unterarm.L", "rot", (ell, 0, 0)),
                               ("Brust", "rot", (bx, 0, 0)), ("Bauch", "rot", (bx * 0.5, 0, 0)), ("Kopf", "rot", (-bx * 0.3, 0, 0)),
                               ("Oberschenkel.L", "rot", (-knie * 0.7, 0, 0)), ("Unterschenkel.L", "rot", (knie, 0, 0)),
                               ("Oberschenkel.R", "rot", (-knie * 0.7, 0, 0)), ("Unterschenkel.R", "rot", (knie, 0, 0)),
                               ("Becken", "pos", (0, 0, -0.004 * knie))])


def _angriff_sense():
    # Sense weit nach rechts hinten ausholen und waagerecht von rechts nach links durchziehen
    return 32, _schluessel(((0, -30, -40, 0), (9, -70, -30, 65), (14, -80, -10, -45), (19, -70, -15, -65), (32, -30, -40, 0)),
                           lambda arm, ell, dreh: [
                               ("Oberarm.R", "rot", (arm, 0, 0)), ("Unterarm.R", "rot", (ell, 0, 0)),
                               ("Oberarm.L", "rot", (arm * 0.9, 0, -20)), ("Unterarm.L", "rot", (ell - 20, 0, 0)),
                               ("Brust", "rot", (8, dreh, 0)), ("Bauch", "rot", (4, dreh * 0.5, 0)), ("Kopf", "rot", (0, -dreh * 0.3, 0))])


def _animationen(angriff, arme_ruhe=((0, -10), (0, -10)), gehen=(26, 40, 16, 0.025, 3, 12), schweben=False):
    """Idle, Laufen (Marsch) und Angriff. `arme_ruhe`: (Oberarm, Unterarm) links und rechts im Stand."""
    (ol, ul), (orr, ur) = arme_ruhe

    def anwenden(armatur):
        def idle(phi):
            werte = [("Brust", "rot", (1.4 * math.sin(phi * 2), 0, 0)), ("Bauch", "rot", (-0.6 * math.sin(phi * 2), 0, 0)),
                     ("Kopf", "rot", (1.5 * math.sin(phi * 2 + 1), 16 * math.sin(phi), 0)),
                     ("Oberarm.L", "rot", (ol + 2 * math.sin(phi * 2), 0, 4)), ("Unterarm.L", "rot", (ul, 0, 0)),
                     ("Oberarm.R", "rot", (orr, 0, -4)), ("Unterarm.R", "rot", (ur, 0, 0)),
                     ("Becken", "pos", (0, 0, -0.003 * (1 - math.cos(phi * 2))))]
            if schweben:
                werte.append(("Becken", "pos", (0, 0, 0.06 * math.sin(phi * 2))))
            return werte
        animation(armatur, "Idle", 180, _schleife(180, 6, idle))

        def laufen(phi):
            if schweben:
                # Gleiten: kein Schritt, nur Wogen und leichtes Vorbeugen
                return [("Becken", "pos", (0, 0, 0.05 * math.sin(phi))), ("Brust", "rot", (10, 4 * math.sin(phi), 0)),
                        ("Oberarm.L", "rot", (ol - 10, 0, 6)), ("Unterarm.L", "rot", (ul, 0, 0)),
                        ("Oberarm.R", "rot", (orr - 10, 0, -6)), ("Unterarm.R", "rot", (ur, 0, 0)), ("Kopf", "rot", (-6, 0, 0))]
            werte = _gehen(phi, *gehen)
            # Arme mit Waffe/Schild schwingen weniger
            werte = [w for w in werte if not w[0].startswith(("Oberarm", "Unterarm"))]
            s = math.sin(phi)
            return werte + [("Oberarm.L", "rot", (ol - 10 * s, 0, 4)), ("Unterarm.L", "rot", (ul, 0, 0)),
                            ("Oberarm.R", "rot", (orr + 10 * s, 0, -4)), ("Unterarm.R", "rot", (ur, 0, 0))]
        animation(armatur, "Laufen", 30, _schleife(30, 2, laufen))
        laenge, schluessel = angriff()
        animation(armatur, "Angriff", laenge, schluessel)
    return anwenden


# ---------------------------------------------------------------------------
# 1. Dunkler Ritter: geschwärzte Plattenrüstung, violetter Waffenrock, Hörnerhelm, Schwert und Schild
# ---------------------------------------------------------------------------
def dunkler_ritter(seed=101):
    f = Figur("DunklerRitter", seed)
    _beine(f, STAHL_SCHWARZ, STAHL_SCHWARZ * 0.8, dick=1.1, schienen=STAHL_DUNKEL)
    _rumpf(f, lambda i, k, p: STAHL_SCHWARZ * (0.9 + 0.2 * max(0.0, -p.normal.y)), dick=1.1)
    _rock(f, lambda i, k, p: ROT if i >= 2.6 else (VIOLETT if -p.normal.y > 0.3 else VIOLETT_DUNKEL), laenge=0.48)
    f.loft("Guertel", [_ring((0, 0.0, 1.03), 0.19, 0.15), _ring((0, 0.0, 1.08), 0.188, 0.148)], 28, lambda i, k, p: LEDER, _rumpf_gewichte)
    _arme(f, STAHL_SCHWARZ, STAHL_DUNKEL, dick=1.1, schulter=STAHL_DUNKEL)
    # Helm mit Sehschlitz (glüht) und zwei Hörnern
    def helm_farbe(i, k, p):
        if -p.normal.y > 0.4 and 1.75 < p.center.z < 1.785:
            return GLUT
        return STAHL_SCHWARZ * (0.9 + 0.2 * max(0.0, p.normal.z))
    helm = [_ring((0, 0.0, 1.6), 0.1, 0.105), _ring((0, -0.005, 1.68), 0.122, 0.128), _ring((0, -0.005, 1.78), 0.124, 0.13),
            _ring((0, 0.0, 1.86), 0.115, 0.12), _ring((0, 0.01, 1.93), 0.07, 0.075)]
    f.loft("Helm", helm, 28, helm_farbe, KOPF, oben_zu=True, teilung=2)
    for sx in (-1, 1):
        punkte = [Vector((sx * 0.1, 0.0, 1.86)), Vector((sx * 0.2, -0.02, 1.9)), Vector((sx * 0.27, -0.06, 2.0)), Vector((sx * 0.28, -0.1, 2.1))]
        f.straehne("Horn", punkte, 0.035, 0.004, KNOCHEN_DUNKEL, KOPF, 8)
    f.stern("Brustzeichen", Vector((0, -0.172, 1.33)), -Y, 0.05, GLUT, _rumpf_gewichte, zacken=4)
    # Zerschlissener Umhang
    umhang = []
    for i, z in enumerate((1.56, 1.35, 1.05, 0.75, 0.45)):
        t = i / 4
        umhang.append((Vector((0, 0.14 + 0.12 * t, z)), X, Y, 0.22 + 0.1 * t, 0.018, lambda w, t=t: 1.0 + 0.05 * math.sin(w * 6 + t * 5)))
    f.loft("Umhang", umhang, 20, lambda i, k, p: VIOLETT_DUNKEL * (0.8 + 0.25 * weich(0.4, 1.5, p.center.z)), _robe_gewichte,
           oben_zu=True, unten_zu=True, teilung=2)
    _schild(f, STAHL_SCHWARZ, STAHL_DUNKEL, zeichen=GLUT)
    _schwert(f)
    _skelett(f)
    return f.fertig(_animationen(_angriff_hieb, arme_ruhe=((-20, -60), (-10, -25))))


# ---------------------------------------------------------------------------
# 2. Bogenschütze: Lederwams, Kapuze, Köcher, Langbogen
# ---------------------------------------------------------------------------
def bogenschuetze(seed=102):
    f = Figur("Bogenschuetze", seed)
    _beine(f, LEDER * 0.8, LEDER, dick=0.95)
    _rumpf(f, lambda i, k, p: LEDER_HELL * (0.85 + 0.2 * max(0.0, -p.normal.y)) if abs(p.center.x) > 0.05 else LEDER, dick=0.95)
    _rock(f, lambda i, k, p: VIOLETT_DUNKEL if i < 2.5 else VIOLETT, laenge=0.36, weite=0.95)
    f.loft("Guertel", [_ring((0, 0.0, 1.02), 0.175, 0.14), _ring((0, 0.0, 1.06), 0.173, 0.138)], 24, lambda i, k, p: LEDER * 0.7, _rumpf_gewichte)
    _arme(f, LEDER_HELL, LEDER * 0.7, dick=0.92)
    _kapuze(f, VIOLETT_DUNKEL)
    _augen(f, GIFT, z=1.765, abstand=0.035, vorne=-0.1, groesse=0.014)
    # Umhang über den Schultern (kurz) und Köcher auf dem Rücken
    f.loft("Schulterumhang", [_ring((0, 0.02, 1.6), 0.2, 0.16), _ring((0, 0.03, 1.45), 0.26, 0.2), _ring((0, 0.04, 1.3), 0.28, 0.22)], 24,
           lambda i, k, p: VIOLETT * (0.85 + 0.2 * p.normal.z), _rumpf_gewichte, teilung=2)
    brust = lambda co: {"Brust": 1.0}
    koecher_a, koecher_b = Vector((-0.1, 0.2, 1.05)), Vector((0.12, 0.22, 1.62))
    _strecke(f, "Koecher", koecher_a, koecher_b, 0.07, 0.075, LEDER, brust, 10)
    for i in range(5):
        spitze = koecher_b + Vector((0.02 * (i - 2), 0.01 * (i % 2), 0.14 + 0.02 * (i % 3)))
        _strecke(f, "Pfeil", koecher_b - Vector((0, 0, 0.05)), spitze, 0.008, 0.008, HOLZ, brust, 5)
        f.kiste("Feder", spitze, (0.012, 0.03, 0.06), VIOLETT_HELL if i % 2 else KNOCHEN, brust)
    _strecke(f, "Riemen", Vector((0.18, -0.12, 1.5)), Vector((-0.16, -0.14, 1.05)), 0.018, 0.018, LEDER * 0.7, _rumpf_gewichte, 6)
    # Langbogen in der linken Hand: in Ruhe waagerecht nach vorne, beim Schuss aufrecht
    anfang = len(f.teile)
    griff = GRIFF_L
    punkte_vorne = [griff + Vector((0, -0.08 * t - 0.62 * t * t, 0.12 * math.sin(math.pi * t) * 0.3)) for t in (0.0, 0.3, 0.6, 1.0)]
    punkte_hinten = [griff + Vector((0, 0.08 * t + 0.62 * t * t, 0.12 * math.sin(math.pi * t) * 0.3)) for t in (0.0, 0.3, 0.6, 1.0)]
    for punkte in (punkte_vorne, punkte_hinten):
        # Wurfarme nach außen gebogen (Richtung +X, weg vom Körper)
        gebogen = [p + Vector((0.12 * math.sin(math.pi * 0.5 * (j / 3)) * (1 - j / 3) * 0 + 0.05 * (j / 3) ** 1.5 * 3, 0, 0))
                   for j, p in enumerate(punkte)]
        f.straehne("Bogenarm", gebogen, 0.02, 0.008, HOLZ * 1.2, HAND_L, 6, 0.0, 0.7, 2)
    ende_v, ende_h = punkte_vorne[-1] + Vector((0.15, 0, 0)), punkte_hinten[-1] + Vector((0.15, 0, 0))
    _strecke(f, "Sehne", ende_v, ende_h, 0.004, 0.004, KNOCHEN, HAND_L, 4)
    _strecke(f, "Griffwicklung", griff + Vector((0, -0.06, 0)), griff + Vector((0, 0.06, 0)), 0.024, 0.024, LEDER * 0.6, HAND_L, 8)
    f.als_starr("Bogen", "Hand.L", anfang)
    _skelett(f)
    return f.fertig(_animationen(_angriff_schuss, arme_ruhe=((-5, -15), (0, -10))))


# ---------------------------------------------------------------------------
# 3. Pikenier: gepolsterter Wams, Eisenhut, violetter Überwurf, lange Pike
# ---------------------------------------------------------------------------
def pikenier(seed=103):
    f = Figur("Pikenier", seed)
    wams = farbe("#6B6258")
    _beine(f, farbe("#3E3833"), LEDER * 0.8, dick=1.0)

    def wams_farbe(i, k, p):
        # gesteppte Streifen
        return wams * (0.82 if int(p.center.z * 22) % 2 else 1.0) * (0.9 + 0.15 * max(0.0, -p.normal.y))
    _rumpf(f, wams_farbe, dick=1.02)
    _rock(f, lambda i, k, p: VIOLETT if abs(p.center.x) < 0.12 else VIOLETT_DUNKEL, laenge=0.42)
    f.loft("Ueberwurf", [_ring((0, -0.005, 1.52), 0.2, 0.148), _ring((0, -0.01, 1.2), 0.2, 0.16), _ring((0, 0.0, 1.0), 0.19, 0.152)], 28,
           lambda i, k, p: VIOLETT if abs(p.center.x) < 0.1 else VIOLETT_DUNKEL, _rumpf_gewichte, teilung=2)
    f.stern("Wappen", Vector((0, -0.18, 1.3)), -Y, 0.045, KNOCHEN, _rumpf_gewichte, zacken=5)
    _arme(f, wams * 0.9, LEDER, dick=1.0, schulter=STAHL_DUNKEL)
    # Gesicht (grau, grimmig) mit Eisenhut (breite Krempe)
    f.kugel("Kopf", Vector((0, -0.01, 1.76)), (0.09, 0.1, 0.115), farbe("#8F8478"), KOPF, 12, 8)
    _augen(f, GLUT, z=1.77, abstand=0.035, vorne=-0.1, groesse=0.013)
    f.kiste("Mund", (0, -0.105, 1.71), (0.05, 0.01, 0.008), SCHATTEN, KOPF)
    f.loft("Eisenhut", [_ring((0, 0.0, 1.8), 0.2, 0.2), _ring((0, 0.0, 1.82), 0.21, 0.21), _ring((0, 0.0, 1.84), 0.12, 0.125),
                        _ring((0, 0.0, 1.93), 0.1, 0.105), _ring((0, 0.0, 1.97), 0.02, 0.02)], 20,
           lambda i, k, p: STAHL_DUNKEL * (0.9 + 0.2 * max(0.0, p.normal.z)), KOPF, oben_zu=True, unten_zu=True)
    # Pike: senkrecht durch die rechte Faust, Spitze oben
    anfang = len(f.teile)
    unten, oben = Vector((STAB_X, STAB_Y, 0.02)), Vector((STAB_X, STAB_Y, 2.55))
    _strecke(f, "Schaft", unten, oben, 0.02, 0.018, HOLZ, HAND_R, 8)
    _platte(f, "Spitze", [(-0.035, 0.0), (0.035, 0.0), (0.02, 0.2), (0.0, 0.32), (-0.02, 0.2)], 0.014, oben, X, Z,
            lambda poly: STAHL * 1.1, HAND_R)
    f.kiste("Tuelle", oben + Vector((0, 0, -0.03)), (0.035, 0.035, 0.08), STAHL_DUNKEL, HAND_R)
    f.kiste("Wimpel", oben + Vector((0.0, 0.1, -0.2)), (0.01, 0.18, 0.12), VIOLETT_HELL, HAND_R)
    f.als_starr("Pike", "Hand.R", anfang)
    _skelett(f)
    return f.fertig(_animationen(_angriff_stoss, arme_ruhe=((0, -15), (0, -12))))


# ---------------------------------------------------------------------------
# 4. Skelettkrieger: Knochen, Rippen, Schädel mit grünen Augen, rostiges Schwert, zerbrochener Rundschild
# ---------------------------------------------------------------------------
def skelettkrieger(seed=104):
    f = Figur("Skelettkrieger", seed)
    knochen_farbe = lambda i, k, p: KNOCHEN * (0.85 + 0.2 * max(0.0, p.normal.z))
    for seite in (1, -1):
        sn = "L" if seite > 0 else "R"
        h_, k_, f_ = _spiegel(HUEFTE, seite), _spiegel(KNIE, seite), _spiegel(KNOECHEL, seite)
        _strecke(f, "Oberschenkelknochen", h_, k_, 0.03, 0.026, knochen_farbe, _bein_gewichte(sn), 8)
        _strecke(f, "Schienbein", k_, f_, 0.026, 0.02, knochen_farbe, _bein_gewichte(sn), 8)
        f.kugel("Knie", k_, (0.04, 0.04, 0.04), KNOCHEN, _bein_gewichte(sn), 8, 6)
        f.loft("Fussknochen", [(Vector((0.1 * seite, 0.03, 0.06)), X, Z, 0.04, 0.03), (Vector((0.1 * seite, -0.14, 0.02)), X, Z, 0.035, 0.012)],
               8, knochen_farbe, lambda co, sn=sn: {f"Fuss.{sn}": 1.0}, oben_zu=True, unten_zu=True)
        s_, e_, w_ = _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite)
        _strecke(f, "Oberarmknochen", s_, e_, 0.026, 0.022, knochen_farbe, _arm_gewichte(seite), 8)
        _strecke(f, "Elle", e_, w_, 0.022, 0.018, knochen_farbe, _arm_gewichte(seite), 8)
        f.kugel("Ellbogen", e_, (0.032, 0.032, 0.032), KNOCHEN, _arm_gewichte(seite), 8, 6)
        f.kugel("Schulterkugel", s_, (0.045, 0.045, 0.045), KNOCHEN, _arm_gewichte(seite), 8, 6)
    f.kugel("KnochenhandL", Vector((0.352, -0.02, 0.9)), (0.035, 0.045, 0.055), KNOCHEN_DUNKEL, HAND_L, 8, 6, glatt=False)
    f.kugel("KnochenhandR", Vector((STAB_X + 0.015, STAB_Y + 0.012, 0.915)), (0.035, 0.045, 0.05), KNOCHEN_DUNKEL, HAND_R, 8, 6, glatt=False)
    # Becken, Wirbelsäule, Rippen, Schlüsselbein
    f.loft("Becken", [_ring((0, 0.0, 0.9), 0.13, 0.08), _ring((0, 0.0, 1.0), 0.15, 0.09), _ring((0, 0.02, 1.05), 0.08, 0.06)], 12,
           knochen_farbe, _rumpf_gewichte, oben_zu=True, unten_zu=True)
    _strecke(f, "Wirbelsaeule", Vector((0, 0.05, 1.0)), Vector((0, 0.04, 1.62)), 0.025, 0.02, KNOCHEN_DUNKEL, _rumpf_gewichte, 6)
    for i in range(6):
        z = 1.2 + i * 0.065
        r = 0.13 + 0.04 * math.sin(math.pi * (i + 0.5) / 6)
        f.loft("Rippe", [(Vector((0, -0.005, z)), X, Y, r, r * 0.75), (Vector((0, -0.005, z + 0.022)), X, Y, r, r * 0.75)], 14,
               lambda ii, k, p: SCHATTEN if -p.normal.y > 0.97 else KNOCHEN * 0.95, _rumpf_gewichte)
    _strecke(f, "Schluesselbein", _spiegel(SCHULTER, 1), _spiegel(SCHULTER, -1), 0.022, 0.022, KNOCHEN, _rumpf_gewichte, 6)
    _strecke(f, "Halswirbel", Vector((0, 0.03, 1.58)), Vector((0, 0.02, 1.68)), 0.02, 0.02, KNOCHEN_DUNKEL,
             lambda co: _mischen(("Kopf", weich(1.6, 1.66, co.z)), ("Hals", 1 - weich(1.6, 1.66, co.z))), 6)
    # Schädel: Hirnschale, Kiefer, dunkle Augenhöhlen mit grünem Glimmen, rostiger Helm
    def schaedel_farbe(i, k, p):
        return KNOCHEN * (0.88 + 0.18 * max(0.0, p.normal.z))
    f.loft("Schaedel", [_ring((0, 0.0, 1.69), 0.07, 0.08), _ring((0, -0.01, 1.74), 0.095, 0.105), _ring((0, 0.0, 1.82), 0.1, 0.11),
                        _ring((0, 0.01, 1.88), 0.085, 0.095), _ring((0, 0.01, 1.92), 0.04, 0.045)], 16, schaedel_farbe, KOPF, oben_zu=True,
           unten_zu=True, teilung=2)
    f.kiste("Kiefer", (0, -0.05, 1.68), (0.1, 0.07, 0.04), KNOCHEN_DUNKEL, KOPF)
    for sx in (-1, 1):
        f.kugel("Augenhoehle", Vector((sx * 0.038, -0.095, 1.775)), (0.028, 0.02, 0.026), SCHATTEN, KOPF, 8, 6)
    _augen(f, GIFT, z=1.775, abstand=0.038, vorne=-0.108, groesse=0.011)
    for i in range(4):
        f.kiste("Zahn", (-0.03 + i * 0.02, -0.088, 1.705), (0.012, 0.01, 0.018), KNOCHEN, KOPF)
    f.loft("Helm", [_ring((0, 0.0, 1.8), 0.11, 0.12), _ring((0, 0.0, 1.88), 0.105, 0.115), _ring((0, 0.0, 1.95), 0.05, 0.055)], 16,
           lambda i, k, p: ROST * (0.8 + 0.3 * max(0.0, p.normal.z)), KOPF, oben_zu=True, unten_zu=True)
    # Zerfetzter Lendenschurz
    _rock(f, lambda i, k, p: VIOLETT_DUNKEL * (0.6 if k % 5 == 0 else 1.0), laenge=0.3, weite=0.8)
    _schild(f, ROST * 0.9, ROST * 0.7, rund=True, kaputt=True)
    _schwert(f, rostig=True, laenge=0.75)
    _skelett(f)
    return f.fertig(_animationen(_angriff_hieb, arme_ruhe=((-18, -55), (-8, -25)), gehen=(24, 36, 12, 0.03, 8, 10)))


# ---------------------------------------------------------------------------
# 5. Dunkelmagier: lange violette Robe, spitze Kapuze, leuchtende Augen, Knochenstab mit Kristall
# ---------------------------------------------------------------------------
def dunkelmagier(seed=105):
    f = Figur("Dunkelmagier", seed)
    _beine(f, SCHATTEN, SCHATTEN, dick=0.9)
    robe_farbe = lambda i, k, p: VIOLETT_HELL if i >= 2.7 else (VIOLETT_DUNKEL if k % 8 in (0, 1) else VIOLETT)
    _rumpf(f, lambda i, k, p: VIOLETT * (0.85 + 0.2 * max(0.0, -p.normal.y)), dick=0.95)
    _rock(f, robe_farbe, laenge=0.9, weite=1.15)
    f.loft("Schaerpe", [_ring((0, 0.0, 1.02), 0.18, 0.145), _ring((0, 0.0, 1.07), 0.178, 0.143)], 24, lambda i, k, p: GLUT * 0.8,
           _rumpf_gewichte)
    _arme(f, VIOLETT_DUNKEL, farbe("#9C8FA8"), dick=0.95)
    # Weite Ärmelenden
    for seite in (1, -1):
        e, h = _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite)
        ax = (h - e).normalized()
        q = ax.cross(Y).normalized()
        f.loft("Aermel", [(e.lerp(h, 0.5), q, ax.cross(q).normalized(), 0.07, 0.07), (h + ax * 0.02, q, ax.cross(q).normalized(), 0.1, 0.1)], 12,
               lambda i, k, p: VIOLETT_HELL if i > 0.6 else VIOLETT, _arm_gewichte(seite), teilung=2)
    _kapuze(f, VIOLETT_DUNKEL, spitz=1.0)
    _augen(f, GLUT, z=1.765, abstand=0.036, vorne=-0.1, groesse=0.016)
    # Schwebende Runen um die Brust (am Brustknochen)
    for k in range(3):
        w = math.tau * k / 3
        f.stern("Rune", Vector((math.cos(w) * 0.3, math.sin(w) * 0.3 - 0.05, 1.25)), Vector((math.cos(w), math.sin(w), 0)), 0.04, GLUT,
                lambda co: {"Brust": 1.0}, zacken=3)
    # Knochenstab mit violettem Kristall in der rechten Hand
    anfang = len(f.teile)
    unten, oben = Vector((STAB_X, STAB_Y, 0.05)), Vector((STAB_X, STAB_Y, 2.0))
    _strecke(f, "Stab", unten, oben, 0.022, 0.018, KNOCHEN_DUNKEL, HAND_R, 8)
    for i in range(4):
        w = math.tau * i / 4
        punkte = [oben, oben + Vector((math.cos(w) * 0.07, math.sin(w) * 0.07, 0.08)), oben + Vector((math.cos(w) * 0.05, math.sin(w) * 0.05, 0.2))]
        f.straehne("Klaue", punkte, 0.014, 0.004, KNOCHEN, HAND_R, 5)
    f.loft("Kristall", [(oben + Vector((0, 0, 0.04)), X, Y, 0.001, 0.001), (oben + Vector((0, 0, 0.13)), X, Y, 0.055, 0.055),
                        (oben + Vector((0, 0, 0.26)), X, Y, 0.001, 0.001)], 6, lambda i, k, p: GLUT, HAND_R, teilung=1)
    f.als_starr("Stab", "Hand.R", anfang)
    _skelett(f)
    return f.fertig(_animationen(_angriff_zauber, arme_ruhe=((-5, -20), (0, -8)), gehen=(22, 34, 12, 0.02, 4, 12)))


# ---------------------------------------------------------------------------
# 6. Steingolem: massiger Körper aus Felsblöcken, violette Adern, riesige Fäuste (1,7-fach groß)
# ---------------------------------------------------------------------------
def steingolem(seed=106):
    f = Figur("Steingolem", seed)
    r = f.rng
    fels = farbe("#6A6570")
    fels_dunkel = farbe("#4A4652")
    moos = farbe("#4E6E3A")

    def fels_farbe(poly):
        c = fels * (0.8 + 0.3 * max(0.0, poly.normal.z)) * r.uniform(0.9, 1.08)
        if poly.normal.z > 0.75 and r.random() < 0.25:
            c = moos
        if r.random() < 0.012:
            c = GLUT
        return c

    def brocken(name, ort, groesse, gewichte, drehung=None):
        bm = bmesh.new()
        bmesh.ops.create_icosphere(bm, subdivisions=2, radius=1.0)
        for v in bm.verts:
            v.co = Vector((v.co.x * groesse[0] * r.uniform(0.85, 1.1), v.co.y * groesse[1] * r.uniform(0.85, 1.1),
                           v.co.z * groesse[2] * r.uniform(0.85, 1.1)))
            if drehung:
                v.co = drehung @ v.co
            v.co += Vector(ort)
        return f._objekt(bm, name, fels_farbe, gewichte)

    for seite in (1, -1):
        sn = "L" if seite > 0 else "R"
        h_, k_, f_ = _spiegel(HUEFTE, seite), _spiegel(KNIE, seite), _spiegel(KNOECHEL, seite)
        brocken("Schenkel", h_.lerp(k_, 0.45) + Vector((0.02 * seite, 0, 0)), (0.14, 0.14, 0.27), _bein_gewichte(sn))
        brocken("Knie", k_ + Vector((0, -0.03, 0)), (0.1, 0.1, 0.09), _bein_gewichte(sn))
        brocken("Wade", k_.lerp(f_, 0.5), (0.12, 0.12, 0.25), _bein_gewichte(sn))
        brocken("Fuss", Vector((0.1 * seite, -0.05, 0.08)), (0.13, 0.18, 0.09), lambda co, sn=sn: {f"Fuss.{sn}": 1.0})
        s_, e_, w_ = _spiegel(SCHULTER, seite), _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite)
        brocken("Schulterfels", s_ + Vector((0.05 * seite, 0, 0.04)), (0.14, 0.13, 0.12), lambda co, sn=sn: _mischen(("Brust", 0.4), (f"Oberarm.{sn}", 0.6)))
        brocken("Oberarmfels", s_.lerp(e_, 0.5), (0.11, 0.11, 0.2), _arm_gewichte(seite))
        brocken("Ellbogenfels", e_, (0.09, 0.09, 0.08), _arm_gewichte(seite))
        brocken("Unterarmfels", e_.lerp(w_, 0.5), (0.12, 0.12, 0.19), _arm_gewichte(seite))
    brocken("FaustL", Vector((0.352, -0.02, 0.86)), (0.13, 0.13, 0.13), HAND_L)
    brocken("FaustR", Vector((STAB_X + 0.015, STAB_Y + 0.012, 0.88)), (0.13, 0.13, 0.13), HAND_R)
    brocken("Becken", (0, 0, 0.98), (0.23, 0.16, 0.14), _rumpf_gewichte)
    brocken("Bauch", (0, 0, 1.15), (0.21, 0.16, 0.16), _rumpf_gewichte)
    brocken("Brust", (0, 0.0, 1.38), (0.3, 0.21, 0.21), lambda co: {"Brust": 1.0})
    brocken("Nacken", (0, 0.06, 1.55), (0.16, 0.13, 0.1), lambda co: {"Brust": 1.0})
    brocken("Kopf", (0, -0.04, 1.66), (0.1, 0.1, 0.09), KOPF)
    _augen(f, GLUT, z=1.68, abstand=0.037, vorne=-0.13, groesse=0.02)
    # Glühende Risse auf der Brust
    for i in range(4):
        a = Vector((r.uniform(-0.12, 0.12), -0.17, r.uniform(1.28, 1.48)))
        _strecke(f, "Ader", a, a + Vector((r.uniform(-0.08, 0.08), -0.01, r.uniform(-0.1, 0.1))), 0.012, 0.006, GLUT, lambda co: {"Brust": 1.0}, 4)
    # Kristalle auf dem Rücken
    for i in range(4):
        basis = Vector((r.uniform(-0.15, 0.15), 0.15, r.uniform(1.35, 1.55)))
        f.loft("Kristall", [(basis, X, Y, 0.035, 0.035), (basis + Vector((0, 0.1, 0.12)), X, Y, 0.001, 0.001)], 5,
               lambda ii, k, p: GLUT * 0.9, lambda co: {"Brust": 1.0})
    _skalieren(f, 1.7)
    _skelett(f, 1.7)
    return f.fertig(_animationen(_angriff_stampfen, arme_ruhe=((-5, -20), (-5, -20)), gehen=(18, 25, 10, 0.02, 6, 20)))


# ---------------------------------------------------------------------------
# 7. Schattenwolf: großer, fast schwarzer Wolf mit violetten Augen und Rückenstacheln
# ---------------------------------------------------------------------------
def schattenwolf(seed=107):
    t = tiere.Tier("Schattenwolf", seed, massstab=1.35)
    kz = tiere._hundeartig(t, schnauze=1.1, ohr=1.1, schwanz=1.2, bein=1.0)
    fell, ruecken, bauch = farbe("#2B2830"), farbe("#17151C"), farbe("#4A4452")

    def zonen(p, n):
        c = fell
        if p.z > 0.72 and n.z > 0.35 and -0.45 < p.y < 0.5:
            c = ruecken
        if n.z < -0.35:
            c = bauch
        if p.y > 0.82:
            c = VIOLETT_DUNKEL
        return c * (0.9 + 0.15 * tiere._weich(0.2, 0.9, p.z))

    t.koerper(zonen, ziel=2600)
    t.ohr(t.v(0.065, -0.72, kz + 0.07), t.v(0.1, -0.69, kz + 0.19), 0.075, 0.03, "#17151C", "#241430")
    t.ohr(t.v(-0.065, -0.72, kz + 0.07), t.v(-0.1, -0.69, kz + 0.19), 0.075, 0.03, "#17151C", "#241430")
    for ort in t.augen_orte(0.058, kz + 0.035):
        t.kugel(ort, 0.022 * t.s, GLUT, name="Auge")
    t.kugel(t.v(0, -1.1, kz - 0.02), 0.028, farbe("#0B0A0D"), groesse=(1.2, 0.9, 0.8), name="Nase")
    # Knochenstacheln auf dem Rücken
    for i in range(6):
        y = -0.4 + i * 0.14
        basis = t.v(0, y, 0.8 + 0.05 * math.cos(y * 3))
        bm = bmesh.new()
        bmesh.ops.create_cone(bm, cap_ends=True, segments=5, radius1=0.04 * t.s, radius2=0.0, depth=0.16 * t.s)
        for v in bm.verts:
            v.co = Matrix.Rotation(-0.5, 3, "X") @ v.co + basis + Vector((0, 0, 0.06 * t.s))
        t._teil(bm, "Stachel", lambda p, n: KNOCHEN_DUNKEL, "Koerper")
    # Zähne (Lefzen hochgezogen)
    for sx in (-1, 1):
        bm = bmesh.new()
        bmesh.ops.create_cone(bm, cap_ends=True, segments=4, radius1=0.012 * t.s, radius2=0.0, depth=0.05 * t.s)
        for v in bm.verts:
            v.co = Matrix.Rotation(math.pi, 3, "X") @ v.co + t.v(sx * 0.03, -1.03, kz - 0.09)
        t._teil(bm, "Zahn", lambda p, n: KNOCHEN, "Kopf")

    stil = dict(schritt=22, schwung=28, knick=45, wippen=0.025, sprung=14, schwung_renn=44, huepfen=0.1, wedeln=0.3)
    original = tiere._animieren

    def mit_angriff(armatur, stil, knochen):
        original(armatur, stil, knochen)
        # Sprung-Biss: ducken, nach vorne schnellen (Vorderbeine gestreckt), zuschnappen, zurück
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
# 8. Gespenst: schwebende, zerfetzte Robe ohne Beine, Kapuze mit grünem Leuchten, Sense
# ---------------------------------------------------------------------------
def gespenst(seed=108):
    f = Figur("Gespenst", seed)
    geist = farbe("#5E6F7A")
    geist_hell = farbe("#9FC7C9")

    def robe_farbe(i, k, p):
        if i >= 3.4:
            return geist_hell * 0.9                                    # ausgefranster, heller Saum
        return (geist * 0.7 if k % 6 in (0, 1) else geist) * (0.8 + 0.3 * weich(0.3, 1.6, p.center.z))

    fetzen = [f.rng.uniform(0.7, 1.3) for _ in range(7)]

    def zerfetzt(w):
        return 1.0 + 0.12 * sum(math.sin(w * (4 + i) + fetzen[i]) * fetzen[i] / (4 + i) for i in range(6))

    ringe = [_ring((0, 0.0, 1.55), 0.18, 0.13), _ring((0, 0.0, 1.3), 0.21, 0.16), _ring((0, 0.02, 1.0), 0.23, 0.18),
             _ring((0, 0.04, 0.7), 0.25, 0.2, zerfetzt), _ring((0, 0.07, 0.42), 0.2, 0.16, zerfetzt)]
    f.loft("Geisterrobe", ringe, 32, robe_farbe, _rumpf_gewichte, teilung=2)
    # Spitze Fetzen unten
    for k in range(9):
        w = math.tau * k / 9
        start = Vector((math.cos(w) * 0.19, math.sin(w) * 0.15 + 0.07, 0.45))
        punkte = [start, start + Vector((math.cos(w) * 0.03, math.sin(w) * 0.03, -0.12)), start + Vector((math.cos(w) * 0.05, math.sin(w) * 0.05, -0.28))]
        f.straehne("Fetzen", punkte, 0.05, 0.004, geist, lambda co: {"Becken": 1.0}, 5, 0.0, 0.3)
    _arme(f, geist * 0.8, farbe("#C7D8D2"), dick=0.85)
    for seite in (1, -1):
        e, h = _spiegel(ELLBOGEN, seite), _spiegel(HANDGELENK, seite)
        ax = (h - e).normalized()
        q = ax.cross(Y).normalized()
        f.loft("Aermel", [(e.lerp(h, 0.3), q, ax.cross(q).normalized(), 0.07, 0.07), (h + ax * 0.03, q, ax.cross(q).normalized(), 0.12, 0.1, zerfetzt)],
               12, lambda i, k, p: geist_hell * 0.8 if i > 0.6 else geist, _arm_gewichte(seite), teilung=2)
    _kapuze(f, geist * 0.75, spitz=0.6)
    _augen(f, GIFT, z=1.765, abstand=0.04, vorne=-0.1, groesse=0.02)
    # Ketten um die Brust
    for i in range(10):
        w = math.tau * i / 10
        f.kiste("Kettenglied", (math.cos(w) * 0.23, math.sin(w) * 0.17, 1.2 + 0.05 * math.sin(w * 2)), (0.03, 0.03, 0.015), STAHL_DUNKEL,
                lambda co: {"Brust": 1.0})
    # Sense mit beiden Händen: Stiel schräg, Blatt oben nach vorne gekrümmt
    anfang = len(f.teile)
    unten, oben = Vector((STAB_X, STAB_Y + 0.11, 0.0)), Vector((STAB_X, STAB_Y - 0.05, 2.05))
    _strecke(f, "Stiel", unten, oben, 0.02, 0.018, HOLZ * 0.7, HAND_R, 8)
    blatt = [(0.0, 0.0), (0.06, -0.12), (0.1, -0.35), (0.08, -0.6), (0.02, -0.78), (0.0, -0.7), (0.02, -0.45), (0.0, -0.2), (-0.05, -0.05)]
    _platte(f, "Sensenblatt", [(b, a) for a, b in blatt], 0.012, oben + Vector((0, 0, -0.02)), Y, Z,
            lambda poly: farbe("#B9C3C8") if abs(poly.normal.x) > 0.5 else STAHL_DUNKEL, HAND_R)
    f.als_starr("Sense", "Hand.R", anfang)
    _skelett(f)
    return f.fertig(_animationen(_angriff_sense, arme_ruhe=((-30, -45), (-25, -35)), schweben=True))
