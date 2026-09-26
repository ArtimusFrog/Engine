"""Verteidigungstürme für die Heerstraßen (Tower Defense), zehn Arten in je drei Stufen.

Jeder Turm besteht aus zwei Modellen:
- `turm_<art>_<stufe>`: der feste Turm bis zur Plattform. Die Plattform liegt bei `KOPF_Z[stufe]`,
  jede Stufe ist höher und reicher verziert.
- `turm_<art>_kopf`: das bewegliche Oberteil (Armbrust, Balliste, Katapultarm, Kristall …), Fuß im
  Ursprung. Das Spiel setzt es auf die Plattform und dreht es zum Ziel. Die Vorderseite zeigt nach −Y.

Gebaut mit den Bauteilen der Burg (burg.py: Mauerwerk-Muster, leuchtende Teile), hell und
freundlich wie das Startlager: die Spieler verteidigen die Insel gegen die düstere Festung.
"""

import math
import random

from burg import MAUER, PLATTEN, ZIEGEL, Bau, M, drehkoerper, kreis, platte, prisma, pyramide, quader, zylinder

KOPF_Z = {1: 4.2, 2: 5.4, 3: 6.6}

HOLZ = "#9A6B3F"
HOLZ_DUNKEL = "#6E4A2A"
HOLZ_HELL = "#C0935A"
STEIN = "#A39C90"
STEIN_HELL = "#C8C1B4"
STEIN_DUNKEL = "#6F6A62"
DACH = "#B5563A"
GOLD = "#D8AE4A"
EISEN = "#4A4A50"
STAHL = "#9AA3AE"
KUPFER = "#C27A45"
MARMOR = "#E9E4DA"
TUCH_ROT = "#B23A3A"
TUCH_BLAU = "#3F6FB5"


def _achteck(r):
    return math.pi / 8


# ---------------------------------------------------------------------------
# Bausteine für die Schäfte
# ---------------------------------------------------------------------------
def _sockel(bau, r, h, farbe=STEIN_DUNKEL):
    """Achteckiger Bruchsteinsockel, reicht 0,6 m in den Boden."""
    bau.teil(zylinder(r * 1.12, h + 0.6, 8, r * 1.02, _achteck(r)), farbe, MAUER, m=M((0, 0, -0.6)))
    bau.teil(zylinder(r * 1.08, 0.18, 8, r * 1.02, _achteck(r)), STEIN_HELL, m=M((0, 0, h)))


def _schaft(bau, r_unten, r_oben, z0, z1, farbe=STEIN, muster=MAUER, ecken=8):
    bau.teil(zylinder(r_unten, z1 - z0, ecken, r_oben, math.pi / ecken), farbe, muster, m=M((0, 0, z0)))


def _plattform(bau, r, z, farbe=STEIN_HELL, zinnen=True, holz=False):
    """Plattform mit Rand (Zinnen oder Holzgeländer); die Oberkante liegt genau bei z."""
    bau.teil(zylinder(r, 0.35, 8, r, math.pi / 8), farbe, PLATTEN if not holz else 0, m=M((0, 0, z - 0.35)))
    if zinnen:
        for k in range(8):
            w = math.tau * k / 8 + math.pi / 8
            bau.teil(quader(0.55, 0.35, 0.7), farbe, MAUER, m=M((math.cos(w) * (r - 0.15), math.sin(w) * (r - 0.15), z), math.degrees(w) + 90))
    elif holz:
        for k in range(8):
            w = math.tau * k / 8
            bau.teil(quader(0.1, 0.1, 0.9), HOLZ_DUNKEL, m=M((math.cos(w) * (r - 0.1), math.sin(w) * (r - 0.1), z)))
        bau.teil(zylinder(r - 0.05, 0.08, 8, r - 0.05), HOLZ, m=M((0, 0, z + 0.85)))
        bau.teil(zylinder(r - 0.16, 0.1, 8, r - 0.16), HOLZ, m=M((0, 0, z + 0.8)))


def _holzturm(bau, breite, z0, z1, zufall):
    """Vier Pfosten mit Kreuzstreben (Holzturm auf Steinsockel)."""
    for sx in (-1, 1):
        for sy in (-1, 1):
            bau.teil(quader(0.28, 0.28, z1 - z0), HOLZ_DUNKEL, m=M((sx * breite / 2, sy * breite / 2, z0)))
    etagen = max(int((z1 - z0) / 1.6), 1)
    for e in range(etagen):
        za, zb = z0 + (z1 - z0) * e / etagen, z0 + (z1 - z0) * (e + 1) / etagen
        laenge = math.hypot(breite, zb - za)
        winkel = math.degrees(math.atan2(zb - za, breite))
        for seite in range(4):
            rz = 90 * seite
            dreh = M((0, 0, 0), rz)
            mitte = dreh @ M((0, -breite / 2, (za + zb) / 2))
            bau.teil(quader(laenge, 0.1, 0.16), HOLZ, m=mitte @ M((0, 0, -0.08), 0, 0, winkel if e % 2 else -winkel))
            bau.teil(quader(breite + 0.3, 0.16, 0.18), HOLZ_HELL, m=dreh @ M((0, -breite / 2, zb - 0.1)))
    # Leiter
    bau.teil(quader(0.08, 0.08, z1 - z0), HOLZ, m=M((-0.3, -breite / 2 - 0.25, z0)))
    bau.teil(quader(0.08, 0.08, z1 - z0), HOLZ, m=M((0.3, -breite / 2 - 0.25, z0)))
    for i in range(int((z1 - z0) / 0.4)):
        bau.teil(quader(0.6, 0.05, 0.05), HOLZ_HELL, m=M((0, -breite / 2 - 0.25, z0 + 0.3 + i * 0.4)))


def _fenster(bau, r, z, anzahl=4, leuchten_farbe=None, farbe="#2A2A30"):
    for k in range(anzahl):
        w = math.tau * k / anzahl + math.pi / 4
        m = M((math.cos(w) * r, math.sin(w) * r, z), math.degrees(w) - 90)
        bau.teil(platte([(-0.18, 0.0), (0.18, 0.0), (0.18, 0.6), (0.0, 0.8), (-0.18, 0.6)], 0.08, 0.02), leuchten_farbe or farbe, m=m,
                 leuchten=leuchten_farbe is not None)


def _wimpel(bau, m, farbe=TUCH_ROT, hoehe=1.4):
    bau.teil(zylinder(0.04, hoehe, 6), GOLD, m=m)
    bau.teil(platte([(0.0, hoehe - 0.05), (0.6, hoehe - 0.25), (0.0, hoehe - 0.45)], 0.03, 0.015), farbe, m=m @ M((0.03, 0, 0), 90))


def _stufen_zier(bau, stufe, r, z, farbe_band=GOLD, wimpel=TUCH_ROT):
    """Was mit der Stufe dazukommt: Zierband, Wimpel an der Plattform, bei Stufe 3 goldene Kappen."""
    if stufe >= 2:
        bau.teil(zylinder(r + 0.06, 0.16, 8, r + 0.06, math.pi / 8), farbe_band, m=M((0, 0, z - 1.3)))
        for k in (0, 2, 4, 6):
            w = math.tau * k / 8 + math.pi / 8
            _wimpel(bau, M((math.cos(w) * (r + 0.25), math.sin(w) * (r + 0.25), z - 0.2)), wimpel)
    if stufe >= 3:
        bau.teil(zylinder(r + 0.1, 0.2, 8, r + 0.1, math.pi / 8), farbe_band, m=M((0, 0, z - 2.6)))
        for k in range(8):
            w = math.tau * k / 8 + math.pi / 8
            bau.teil(pyramide(0.18, 0.35, 4), GOLD, m=M((math.cos(w) * (r - 0.15), math.sin(w) * (r - 0.15), z + 0.7)))


# ---------------------------------------------------------------------------
# Die zehn Türme (feste Teile)
# ---------------------------------------------------------------------------
def _turm_pfeil(bau, stufe, z, zufall):
    _sockel(bau, 1.7, 1.0)
    _holzturm(bau, 2.2, 1.0, z - 0.35, zufall)
    _plattform(bau, 1.8, z, HOLZ_HELL, zinnen=False, holz=True)
    if stufe >= 2:
        for k in range(4):
            w = math.tau * k / 4 + math.pi / 4
            _wimpel(bau, M((math.cos(w) * 1.7, math.sin(w) * 1.7, z)), TUCH_ROT, 1.6 + 0.3 * stufe)
    if stufe >= 3:
        for k in range(4):
            w = math.tau * k / 4
            bau.teil(quader(1.0, 0.08, 0.6), TUCH_ROT, m=M((math.cos(w) * 1.78, math.sin(w) * 1.78, z + 0.15), math.degrees(w) + 90))


def _turm_balliste(bau, stufe, z, zufall):
    _sockel(bau, 2.0, 0.8)
    _schaft(bau, 1.8, 1.6, 0.8, z - 0.35)
    _fenster(bau, 1.66, z - 2.2, 4)
    _plattform(bau, 2.0, z)
    _stufen_zier(bau, stufe, 1.7, z)


def _turm_katapult(bau, stufe, z, zufall):
    # Breiter, wuchtiger Unterbau mit Rampe und Vorratskisten
    bau.teil(quader(4.2, 4.2, z - 0.35 + 0.6), STEIN, MAUER, m=M((0, 0, -0.6)))
    bau.teil(quader(4.4, 4.4, 0.35), STEIN_HELL, PLATTEN, m=M((0, 0, z - 0.35)))
    for sx in (-1, 1):
        for sy in (-1, 1):
            bau.teil(quader(0.6, 0.6, z + 0.5), STEIN_DUNKEL, MAUER, m=M((sx * 1.9, sy * 1.9, 0)))
            bau.teil(pyramide(0.45, 0.5, 4), DACH if stufe < 3 else GOLD, m=M((sx * 1.9, sy * 1.9, z + 0.5)))
    for i in range(2 + stufe):
        bau.teil(drehkoerper([(0.0, 0.0), (0.22, 0.05), (0.26, 0.25), (0.2, 0.42), (0.0, 0.45)], 6), STEIN_HELL,
                 m=M((1.6 - i * 0.45, -2.5, 0)))
    _stufen_zier(bau, stufe, 2.1, z)


def _turm_feuer(bau, stufe, z, zufall):
    _sockel(bau, 1.8, 0.8, "#5C534C")
    _schaft(bau, 1.55, 1.3, 0.8, z - 0.35, "#7A6F66")
    for k in range(4):
        w = math.tau * k / 4 + math.pi / 4
        m = M((math.cos(w) * 1.4, math.sin(w) * 1.4, 1.6), math.degrees(w) - 90)
        bau.teil(platte([(-0.1, 0.0), (0.1, 0.0), (0.1, z - 3.0), (-0.1, z - 3.0)], 0.1, 0.03), "#FF8A2A", m=m, leuchten=True)
    _plattform(bau, 1.75, z, "#8C8076")
    _stufen_zier(bau, stufe, 1.45, z, GOLD, "#E0582A")


def _turm_frost(bau, stufe, z, zufall):
    _sockel(bau, 1.7, 0.8, "#8E9AA6")
    _schaft(bau, 1.45, 1.1, 0.8, z - 0.35, "#C9D6E2")
    for k in range(5 + stufe * 2):
        w = zufall.uniform(0, math.tau)
        h = zufall.uniform(0.6, 1.6)
        bau.teil(pyramide(0.18, h, 5, 0.0), "#9FE0FF", m=M((math.cos(w) * 1.75, math.sin(w) * 1.75, zufall.uniform(0.0, 0.6)),
                                                     0, zufall.uniform(-25, 25), zufall.uniform(-25, 25)), leuchten=True)
    _fenster(bau, 1.3, z - 2.0, 4, "#9FE0FF")
    _plattform(bau, 1.6, z, "#DDE6EE")
    _stufen_zier(bau, stufe, 1.25, z, "#E8F4FF", TUCH_BLAU)


def _turm_blitz(bau, stufe, z, zufall):
    _sockel(bau, 1.6, 0.8, "#55565E")
    _schaft(bau, 1.3, 1.0, 0.8, z - 0.35, "#6B6D78")
    for i in range(3 + stufe):
        zi = 1.4 + i * (z - 2.2) / (3 + stufe)
        r = 1.3 - 0.3 * (zi / z) + 0.08
        bau.teil(zylinder(r, 0.14, 12, r), KUPFER, m=M((0, 0, zi)))
    for k in range(3):
        w = math.tau * k / 3
        bau.teil(zylinder(0.05, z - 1.0, 6), KUPFER, m=M((math.cos(w) * 1.25, math.sin(w) * 1.25, 0.8)))
        bau.teil(drehkoerper([(0.0, 0.0), (0.14, 0.14), (0.0, 0.28)], 6), "#7FD3FF", m=M((math.cos(w) * 1.25, math.sin(w) * 1.25, z - 0.25)),
                 leuchten=True)
    _plattform(bau, 1.4, z, "#7C7E88", zinnen=False)
    _stufen_zier(bau, stufe, 1.05, z, KUPFER, TUCH_BLAU)


def _turm_sonne(bau, stufe, z, zufall):
    _sockel(bau, 1.7, 0.8, STEIN_HELL)
    _schaft(bau, 1.4, 1.2, 0.8, z - 0.35, MARMOR, 0, 12)
    for k in range(6):
        w = math.tau * k / 6
        bau.teil(zylinder(0.14, z - 1.2, 8), MARMOR, m=M((math.cos(w) * 1.45, math.sin(w) * 1.45, 0.8)))
        bau.teil(zylinder(0.2, 0.15, 8), GOLD, m=M((math.cos(w) * 1.45, math.sin(w) * 1.45, z - 0.5)))
    _fenster(bau, 1.22, z - 2.0, 4, "#FFE9A0")
    _plattform(bau, 1.7, z, MARMOR, zinnen=False)
    bau.teil(zylinder(1.72, 0.12, 12, 1.72), GOLD, m=M((0, 0, z - 0.1)))
    _stufen_zier(bau, stufe, 1.3, z, GOLD, "#F2C94C")


def _turm_arkan(bau, stufe, z, zufall):
    _sockel(bau, 1.7, 0.8, "#4B4666")
    _schaft(bau, 1.35, 1.05, 0.8, z - 0.35, "#5E5A8A")
    for k in range(3 + stufe):
        w = math.tau * k / (3 + stufe)
        zr = 1.8 + (k % 3) * 0.9
        bau.teil(quader(0.35, 0.12, 0.5, 0.03), "#A98BFF", m=M((math.cos(w) * 1.7, math.sin(w) * 1.7, zr), math.degrees(w) + 90), leuchten=True)
    _fenster(bau, 1.2, z - 2.1, 4, "#B69CFF")
    _plattform(bau, 1.55, z, "#6F6A9E")
    bau.teil(pyramide(0.3, 0.9, 4), "#B69CFF", m=M((0, 1.45, z - 0.35)), leuchten=True)
    _stufen_zier(bau, stufe, 1.2, z, "#C9B8FF", "#6A4FC8")


def _turm_gift(bau, stufe, z, zufall):
    _sockel(bau, 1.7, 0.8, "#5E6B4E")
    _holzturm(bau, 2.0, 0.8, z - 0.35, zufall)
    # Moos und Rohre, aus denen es grün tropft
    for k in range(3):
        w = math.tau * k / 3 + 0.4
        bau.teil(zylinder(0.12, z - 0.6, 6), KUPFER, m=M((math.cos(w) * 1.25, math.sin(w) * 1.25, 0.1)))
        bau.teil(drehkoerper([(0.0, 0.0), (0.25, 0.04), (0.28, 0.12), (0.0, 0.14)], 6), "#8CFF6A", m=M((math.cos(w) * 1.25, math.sin(w) * 1.25, 0.0)),
                 leuchten=True)
    _plattform(bau, 1.65, z, HOLZ, zinnen=False, holz=True)
    _stufen_zier(bau, stufe, 1.3, z, "#8CC152", "#4E8A3A")


def _turm_banner(bau, stufe, z, zufall):
    _sockel(bau, 1.6, 0.8, STEIN)
    _holzturm(bau, 1.8, 0.8, z - 0.35, zufall)
    # Kriegstrommeln am Fuß
    for k in range(2 + stufe):
        w = math.tau * k / (2 + stufe) + 0.3
        m = M((math.cos(w) * 2.2, math.sin(w) * 2.2, 0.0))
        bau.teil(zylinder(0.4, 0.55, 10), TUCH_ROT, m=m)
        bau.teil(zylinder(0.42, 0.06, 10), GOLD, m=m @ M((0, 0, 0.55)))
        bau.teil(zylinder(0.38, 0.02, 10), "#E8DCC0", m=m @ M((0, 0, 0.6)))
    _plattform(bau, 1.5, z, HOLZ_HELL, zinnen=False, holz=True)
    _stufen_zier(bau, stufe, 1.1, z, GOLD, TUCH_ROT)


TUERME = {
    "pfeil": _turm_pfeil, "balliste": _turm_balliste, "katapult": _turm_katapult, "feuer": _turm_feuer, "frost": _turm_frost,
    "blitz": _turm_blitz, "sonne": _turm_sonne, "arkan": _turm_arkan, "gift": _turm_gift, "banner": _turm_banner,
}


def turm(art, stufe):
    bau = Bau(200 + stufe * 13 + len(art))
    zufall = random.Random(sum(map(ord, art)) * 7 + stufe)
    TUERME[art](bau, stufe, KOPF_Z[stufe], zufall)
    return bau.fertig(f"Turm_{art}_{stufe}", glas_leuchten=2.0, ursprung=(0.0, 0.0))


# ---------------------------------------------------------------------------
# Köpfe (drehbar, Vorderseite −Y, Fuß im Ursprung)
# ---------------------------------------------------------------------------
def _drehteller(bau, r=0.7, farbe=HOLZ_DUNKEL):
    bau.teil(zylinder(r, 0.2, 12, r * 0.9), farbe, m=M((0, 0, 0)))


def _kopf_pfeil(bau):
    _drehteller(bau, 0.6)
    bau.teil(quader(0.2, 0.3, 0.6), HOLZ_DUNKEL, m=M((0, 0, 0.2)))
    # Armbrust: Säule, Bogen quer, Bolzen nach vorne
    bau.teil(quader(0.22, 1.5, 0.18), HOLZ, m=M((0, -0.3, 0.8)))
    bau.teil(quader(1.6, 0.12, 0.1), HOLZ_HELL, m=M((0, -0.95, 0.88)))
    for s in (-1, 1):
        bau.teil(quader(0.5, 0.1, 0.09), STAHL, m=M((s * 0.9, -0.85, 0.88), s * 20))
    bau.teil(zylinder(0.03, 1.3, 6), HOLZ_HELL, m=M((0, -0.2, 1.0), 0, 90))
    bau.teil(pyramide(0.06, 0.2, 4), STAHL, m=M((0, -1.52, 1.0), 0, 90))
    bau.teil(quader(0.6, 0.08, 0.5), TUCH_ROT, m=M((0, 0.25, 0.75)))


def _kopf_balliste(bau):
    _drehteller(bau, 0.9, HOLZ_DUNKEL)
    for s in (-1, 1):
        bau.teil(quader(0.2, 0.9, 0.9), HOLZ, m=M((s * 0.45, 0, 0.2)))
    bau.teil(quader(0.35, 2.4, 0.25), HOLZ, m=M((0, -0.3, 1.0)))
    bau.teil(quader(2.6, 0.18, 0.18), HOLZ_DUNKEL, m=M((0, -1.2, 1.05)))
    for s in (-1, 1):
        bau.teil(quader(0.8, 0.16, 0.14), STAHL, m=M((s * 1.4, -1.0, 1.08), s * 25))
        bau.teil(zylinder(0.18, 0.3, 8), EISEN, m=M((s * 0.35, 0.6, 0.9), 0, 0, 90))
    bau.teil(zylinder(0.05, 2.2, 6), HOLZ_HELL, m=M((0, 0.5, 1.3), 0, 90))
    bau.teil(pyramide(0.12, 0.35, 4), STAHL, m=M((0, -1.7, 1.3), 0, 90))


def _kopf_katapult(bau):
    _drehteller(bau, 1.2, HOLZ_DUNKEL)
    for s in (-1, 1):
        bau.teil(quader(0.2, 1.8, 0.2), HOLZ, m=M((s * 0.6, 0, 0.2)))
        bau.teil(prisma([(-0.6, 0.0), (0.6, 0.0), (0.0, 1.5)], 0.18), HOLZ, m=M((s * 0.6 - 0.09, 0, 0.2)))
    bau.teil(zylinder(0.12, 1.4, 8), HOLZ_DUNKEL, m=M((-0.7, 0, 1.6), 0, 0, 90))
    # Wurfarm nach hinten geneigt, Schale mit Stein
    bau.teil(quader(0.2, 2.6, 0.2), HOLZ_HELL, m=M((0, 0.6, 1.3), 0, 32))
    bau.teil(drehkoerper([(0.0, 0.0), (0.35, 0.05), (0.4, 0.25), (0.0, 0.2)], 8), HOLZ_DUNKEL, m=M((0, 1.7, 0.7)))
    bau.teil(drehkoerper([(0.0, 0.0), (0.25, 0.1), (0.28, 0.3), (0.0, 0.45)], 6), STEIN, m=M((0, 1.7, 0.85)))
    bau.teil(quader(0.6, 0.6, 0.5), STEIN_DUNKEL, m=M((0, -0.7, 0.2)))


def _kopf_feuer(bau):
    bau.teil(zylinder(0.5, 0.3, 8, 0.35), EISEN, m=M((0, 0, 0)))
    bau.teil(drehkoerper([(0.1, 0.3), (0.55, 0.45), (0.85, 0.9), (0.9, 1.05), (0.75, 1.0)], 10), EISEN, m=M((0, 0, 0)))
    for s in (-1, 1):
        bau.teil(pyramide(0.1, 0.5, 4), EISEN, m=M((s * 0.85, 0, 1.0)))
    # Drachenkopf-Düse nach vorne
    bau.teil(quader(0.3, 0.8, 0.3), "#3A3A40", m=M((0, -0.8, 0.7)))
    bau.teil(pyramide(0.2, 0.4, 4), "#3A3A40", m=M((0, -1.2, 0.85), 0, 90))
    for i, (h, r) in enumerate(((1.2, 0.55), (1.6, 0.4), (1.9, 0.25))):
        bau.teil(pyramide(r, h - 0.6, 6, 0.3 * i), "#FF8A2A" if i < 2 else "#FFD36A", m=M((0.05 * i, 0.03 * i, 0.9)), leuchten=True)


def _kopf_frost(bau):
    bau.teil(zylinder(0.55, 0.25, 8, 0.4), "#DDE6EE", m=M((0, 0, 0)))
    for k in range(6):
        w = math.tau * k / 6
        bau.teil(pyramide(0.16, 0.9, 5), "#BDEBFF", m=M((math.cos(w) * 0.35, math.sin(w) * 0.35, 0.25), 0, math.cos(w) * 25, math.sin(w) * -25),
                 leuchten=True)
    bau.teil(drehkoerper([(0.0, 0.3), (0.35, 0.9), (0.0, 2.1)], 6), "#9FE0FF", m=M((0, 0, 0)), leuchten=True)


def _kopf_blitz(bau):
    bau.teil(zylinder(0.45, 0.3, 10, 0.35), "#55565E", m=M((0, 0, 0)))
    bau.teil(zylinder(0.12, 1.2, 8), KUPFER, m=M((0, 0, 0.3)))
    for i in range(5):
        bau.teil(zylinder(0.32 - i * 0.03, 0.08, 12), KUPFER, m=M((0, 0, 0.45 + i * 0.18)))
    bau.teil(drehkoerper([(0.0, 0.0), (0.3, 0.2), (0.36, 0.45), (0.3, 0.7), (0.0, 0.9)], 10), "#7FD3FF", m=M((0, 0, 1.4)), leuchten=True)
    for k in range(4):
        w = math.tau * k / 4 + math.pi / 4
        bau.teil(pyramide(0.05, 0.5, 4), KUPFER, m=M((math.cos(w) * 0.35, math.sin(w) * 0.35, 1.6), 0, math.cos(w) * 40, -math.sin(w) * 40))


def _kopf_sonne(bau):
    bau.teil(zylinder(0.5, 0.25, 12, 0.4), GOLD, m=M((0, 0, 0)))
    for s in (-1, 1):
        bau.teil(quader(0.12, 0.12, 1.2), GOLD, m=M((s * 0.7, 0, 0.2)))
    # Sonnenscheibe (Spiegel) nach vorne gerichtet, mit Strahlenkranz
    m = M((0, 0, 1.3), 0, 90)
    bau.teil(zylinder(0.62, 0.1, 16), GOLD, m=m)
    bau.teil(zylinder(0.5, 0.12, 16), "#FFF1B0", m=m @ M((0, 0, -0.02)), leuchten=True)
    for k in range(12):
        w = math.tau * k / 12
        bau.teil(pyramide(0.07, 0.35, 4), GOLD, m=M((math.cos(w) * 0.78, 0.03, 1.3 + math.sin(w) * 0.78), 0, 0, math.degrees(-w) + 90))


def _kopf_arkan(bau):
    bau.teil(zylinder(0.45, 0.2, 8, 0.35), "#4B4666", m=M((0, 0, 0)))
    bau.teil(drehkoerper([(0.0, 0.4), (0.3, 0.9), (0.0, 1.8)], 6), "#B69CFF", m=M((0, 0, 0)), leuchten=True)
    for ring, (rx, ry) in enumerate(((0, 0), (60, 0), (-60, 30))):
        punkte = kreis(0.7, 16)
        for (x0, z0), (x1, z1) in zip(punkte, punkte[1:] + punkte[:1]):
            laenge = math.hypot(x1 - x0, z1 - z0)
            w = math.degrees(math.atan2(z1 - z0, x1 - x0))
            bau.teil(quader(laenge, 0.05, 0.05), "#D8C8FF", m=M((0, 0, 1.1), ry, rx) @ M(((x0 + x1) / 2, 0, (z0 + z1) / 2), 0, 0, -w),
                     leuchten=ring == 0)


def _kopf_gift(bau):
    bau.teil(zylinder(0.4, 0.25, 8, 0.3), EISEN, m=M((0, 0, 0)))
    bau.teil(drehkoerper([(0.1, 0.25), (0.6, 0.4), (0.72, 0.8), (0.62, 1.1), (0.55, 1.12)], 10), "#3A3A40", m=M((0, 0, 0)))
    bau.teil(zylinder(0.56, 0.06, 10), "#8CFF6A", m=M((0, 0, 1.05)), leuchten=True)
    for k in range(5):
        w = math.tau * k / 5
        bau.teil(drehkoerper([(0.0, 0.0), (0.12, 0.08), (0.0, 0.18)], 6), "#B8FF8C", m=M((math.cos(w) * 0.3, math.sin(w) * 0.3, 1.08)),
                 leuchten=True)
    bau.teil(zylinder(0.1, 0.9, 8), KUPFER, m=M((0, -0.55, 0.7), 0, 60))
    bau.teil(drehkoerper([(0.1, 0.0), (0.22, 0.2), (0.0, 0.22)], 8), KUPFER, m=M((0, -1.0, 1.0), 0, 60))


def _kopf_banner(bau):
    bau.teil(zylinder(0.3, 0.2, 8), HOLZ_DUNKEL, m=M((0, 0, 0)))
    bau.teil(zylinder(0.07, 4.2, 8), HOLZ_DUNKEL, m=M((0, 0, 0.2)))
    bau.teil(drehkoerper([(0.0, 0.0), (0.14, 0.1), (0.0, 0.45)], 6), GOLD, m=M((0, 0, 4.4)))
    bau.teil(quader(1.8, 0.08, 0.08), GOLD, m=M((0.9, 0, 4.0)))
    umriss = [(0.0, 4.0), (1.8, 4.0), (1.8, 1.9), (0.9, 2.4), (0.0, 1.9)]
    bau.teil(platte(umriss, 0.05, 0.025), TUCH_ROT, m=M((0.05, 0, 0)))
    raute = [(0.9, 3.6), (1.3, 3.1), (0.9, 2.6), (0.5, 3.1)]
    for y in (-0.03, 0.08):
        bau.teil(platte(raute, 0.012, y), GOLD, m=M((0.05, 0, 0)))


KOEPFE = {
    "pfeil": _kopf_pfeil, "balliste": _kopf_balliste, "katapult": _kopf_katapult, "feuer": _kopf_feuer, "frost": _kopf_frost,
    "blitz": _kopf_blitz, "sonne": _kopf_sonne, "arkan": _kopf_arkan, "gift": _kopf_gift, "banner": _kopf_banner,
}


def kopf(art):
    bau = Bau(400 + len(art))
    KOEPFE[art](bau)
    return bau.fertig(f"Kopf_{art}", glas_leuchten=2.5, ursprung=(0.0, 0.0))
