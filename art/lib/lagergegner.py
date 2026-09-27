"""Bewohner der Lager in der Wildnis (siehe game/src/wildnis.rs) – ganz normale Halunken statt der
Truppen der Schattenfestung: Bandit, Banditenschütze, Plünderer, dazu kleine Waldspinnen.

Menschen mit dem Baukasten aus gegner.py (Skelett wie beim Magier), aber mit sichtbarem Gesicht,
Haaren und Bart statt dunkler Kapuze; erdige Farben, rostige Waffen. Die Waldspinne ist wie die
Spinnenkönigin gebaut (bosse.py), nur klein und braun. Animationen: Idle, Laufen, Angriff.
"""

import bmesh
from mathutils import Vector

from bosse import _boden, _spinne_animationen, _spinne_bein
from figuren import Figur, X, Y, _rumpf_gewichte, farbe
from gegner import (GRIFF_L, GRIFF_R, HAND_L, HAND_R, HOLZ, KNOCHEN, KOPF, LEDER, ROST, SCHATTEN, _angriff_hieb, _angriff_schuss,
                    _animationen, _arme, _beine, _ring, _rock, _rumpf, _schild, _schwert, _skelett, _strecke)

WAFFE_RUHE = Vector((0, -0.62, -0.78)).normalized()     # Waffen in Ruhe schräg nach vorne unten
HAUT = farbe("#D9A57E")


def _kopf(f, haut, haar, bart=None):
    """Menschlicher Kopf: Gesicht mit Nase, Augen, Ohren, Haar oben und hinten, optional Bart."""
    f.kugel("Kopf", Vector((0, -0.005, 1.75)), (0.095, 0.105, 0.115), haut, KOPF, 16, 12)
    f.kugel("Nase", Vector((0, -0.108, 1.735)), (0.018, 0.026, 0.022), haut * 0.95, KOPF, 8, 6)
    for sx in (-1, 1):
        f.kugel("Auge", Vector((sx * 0.036, -0.094, 1.765)), (0.011, 0.006, 0.008), farbe("#2A1E16"), KOPF, 8, 4)
        f.kugel("Braue", Vector((sx * 0.037, -0.098, 1.785)), (0.022, 0.008, 0.006), haar * 0.8, KOPF, 8, 4)
        f.kugel("Ohr", Vector((sx * 0.096, 0.0, 1.75)), (0.014, 0.025, 0.032), haut * 0.95, KOPF, 8, 6)
    f.kugel("Haar", Vector((0, 0.025, 1.8)), (0.102, 0.1, 0.085), haar, KOPF, 14, 8)
    f.kugel("Hinterkopf", Vector((0, 0.045, 1.72)), (0.09, 0.07, 0.08), haar, KOPF, 12, 8)
    if bart is not None:
        f.kugel("Bart", Vector((0, -0.07, 1.67)), (0.075, 0.05, 0.06), bart, KOPF, 12, 8)


def _beil(f, holz):
    """Beil in der rechten Hand: Stiel und keilförmiges Eisenblatt."""
    anfang = len(f.teile)
    a = GRIFF_R - WAFFE_RUHE * 0.1
    b = GRIFF_R + WAFFE_RUHE * 0.5
    _strecke(f, "Beilstiel", a, b, 0.02, 0.018, holz, HAND_R, 8)
    quer = Vector((1, 0, 0))
    vorne = WAFFE_RUHE.cross(quer).normalized()
    kopf = GRIFF_R + WAFFE_RUHE * 0.44
    f.loft("Beilblatt", [(kopf, WAFFE_RUHE, quer, 0.05, 0.02), (kopf + vorne * 0.08, WAFFE_RUHE, quer, 0.07, 0.008),
                         (kopf + vorne * 0.13, WAFFE_RUHE, quer, 0.085, 0.003)], 8, lambda i, k, p: ROST if i < 1.5 else farbe("#A8A49C"),
           HAND_R, oben_zu=True, unten_zu=True)
    f.als_starr("Beil", "Hand.R", anfang)


def _bogen(f, holz):
    """Langbogen in der linken Hand."""
    anfang = len(f.teile)
    griff = GRIFF_L
    punkte_vorne = [griff + Vector((0.05 * t ** 1.5 * 3, -0.08 * t - 0.62 * t * t, 0.0)) for t in (0.0, 0.3, 0.6, 1.0)]
    punkte_hinten = [griff + Vector((0.05 * t ** 1.5 * 3, 0.08 * t + 0.62 * t * t, 0.0)) for t in (0.0, 0.3, 0.6, 1.0)]
    for punkte in (punkte_vorne, punkte_hinten):
        f.straehne("Bogenarm", punkte, 0.02, 0.008, holz, HAND_L, 6, 0.0, 0.7, 2)
    _strecke(f, "Sehne", punkte_vorne[-1], punkte_hinten[-1], 0.004, 0.004, KNOCHEN, HAND_L, 4)
    _strecke(f, "Griffwicklung", griff + Vector((0, -0.06, 0)), griff + Vector((0, 0.06, 0)), 0.024, 0.024, LEDER * 0.6, HAND_L, 8)
    f.als_starr("Bogen", "Hand.L", anfang)


def _banditenkoerper(f, tuch, haar):
    """Lederwams über farbigem Hemd, Gürtel mit Beutel, Kappe und Tuch vor dem Mund."""
    wams, hose = farbe("#6A5036"), farbe("#4A3C2E")
    _beine(f, hose, LEDER * 0.75, dick=1.0)
    _rumpf(f, lambda i, k, p: (wams if abs(p.center.x) > 0.05 or p.center.z < 1.3 else tuch) * (0.85 + 0.2 * max(0.0, -p.normal.y)), dick=1.0)
    _rock(f, lambda i, k, p: wams * (0.8 if k % 6 == 0 else 0.95), laenge=0.3, weite=1.0)
    f.loft("Guertel", [_ring((0, 0.0, 1.02), 0.178, 0.142), _ring((0, 0.0, 1.06), 0.176, 0.14)], 24, lambda i, k, p: LEDER * 0.6, _rumpf_gewichte)
    f.kiste("Schnalle", (0, -0.145, 1.04), (0.04, 0.01, 0.03), farbe("#A08040"), _rumpf_gewichte)
    f.kiste("Beutel", (0.15, -0.06, 0.96), (0.06, 0.05, 0.07), LEDER, _rumpf_gewichte)
    _arme(f, tuch * 0.9, LEDER * 0.7, dick=1.0)
    _kopf(f, HAUT, haar)
    # Lederkappe und Tuch vor Mund und Nase
    f.loft("Kappe", [_ring((0, 0.01, 1.77), 0.105, 0.115), _ring((0, 0.015, 1.83), 0.1, 0.108), _ring((0, 0.02, 1.87), 0.06, 0.065)], 16,
           lambda i, k, p: LEDER * 0.85, KOPF, oben_zu=True)
    f.loft("Maske", [_ring((0, -0.012, 1.66), 0.088, 0.1), _ring((0, -0.018, 1.735), 0.093, 0.108)], 16, lambda i, k, p: tuch * 0.8, KOPF)


def bandit(seed=302):
    f = Figur("Bandit", seed)
    _banditenkoerper(f, farbe("#8A2A22"), farbe("#3A2A1E"))
    _schild(f, HOLZ, LEDER, rund=True)
    _schwert(f, rostig=True, laenge=0.62)
    _skelett(f)
    return f.fertig(_animationen(_angriff_hieb, arme_ruhe=((-20, -60), (-10, -25))))


def banditenschuetze(seed=303):
    f = Figur("Banditenschuetze", seed)
    _banditenkoerper(f, farbe("#4E6A2E"), farbe("#6A4A2A"))
    brust = lambda co: {"Brust": 1.0}
    koecher_a, koecher_b = Vector((-0.1, 0.2, 1.05)), Vector((0.12, 0.22, 1.62))
    _strecke(f, "Koecher", koecher_a, koecher_b, 0.07, 0.075, LEDER, brust, 10)
    for i in range(4):
        spitze = koecher_b + Vector((0.02 * (i - 1.5), 0.01 * (i % 2), 0.14 + 0.02 * (i % 3)))
        _strecke(f, "Pfeil", koecher_b - Vector((0, 0, 0.05)), spitze, 0.008, 0.008, HOLZ, brust, 5)
        f.kiste("Feder", spitze, (0.012, 0.03, 0.06), farbe("#D8D0B8") if i % 2 else farbe("#8A2A22"), brust)
    _bogen(f, HOLZ * 1.25)
    _skelett(f)
    return f.fertig(_animationen(_angriff_schuss, arme_ruhe=((-5, -15), (0, -10))))


def pluenderer(seed=301):
    """Zerlumpter Plünderer: offene Weste, nackte Arme, Stirnband, struppiger Bart, Beil."""
    f = Figur("Pluenderer", seed)
    lumpen, weste = farbe("#7A6446"), farbe("#5A4230")
    _beine(f, farbe("#5E5040") * (0.9), LEDER * 0.6, dick=1.02)

    def oberkoerper(i, k, p):
        vorne = -p.normal.y > 0.5 and abs(p.center.x) < 0.07 and p.center.z > 1.12
        return HAUT * 0.95 if vorne else weste * (0.85 + 0.2 * max(0.0, -p.normal.y))
    _rumpf(f, oberkoerper, dick=1.02)
    _rock(f, lambda i, k, p: lumpen * (0.75 if (k + int(i * 3)) % 5 == 0 else 1.0), laenge=0.34, weite=1.02)
    f.loft("Strick", [_ring((0, 0.0, 1.02), 0.18, 0.143), _ring((0, 0.0, 1.05), 0.178, 0.141)], 20, lambda i, k, p: farbe("#8A7048"), _rumpf_gewichte)
    _arme(f, HAUT * 0.95, LEDER * 0.6, dick=1.02)
    haar = farbe("#2E2620")
    _kopf(f, HAUT, haar, bart=haar * 1.2)
    f.loft("Stirnband", [_ring((0, -0.005, 1.785), 0.1, 0.11), _ring((0, -0.005, 1.805), 0.1, 0.11)], 16, lambda i, k, p: farbe("#6A2A1E"), KOPF)
    f.kiste("Narbe", (0.04, -0.103, 1.72), (0.004, 0.003, 0.03), HAUT * 0.7, KOPF)
    _beil(f, HOLZ * 1.1)
    _skelett(f)
    return f.fertig(_animationen(_angriff_hieb, arme_ruhe=((-5, -20), (-10, -30)), gehen=(28, 42, 18, 0.03, 5, 14)))


# ---------------------------------------------------------------------------
# Waldspinne: wie die Spinnenkönigin gebaut, aber klein, braun mit gelber Zeichnung
# ---------------------------------------------------------------------------
def waldspinne(seed=306, groesse=0.24):
    from gegner import _skalieren
    f = Figur("Waldspinne", seed)
    r = f.rng
    chitin, chitin_hell, zeichnung = farbe("#4A3A26"), farbe("#6A5634"), farbe("#C8B83A")
    koerper = lambda co: {"Koerper": 1.0}
    hinterleib = lambda co: {"Hinterleib": 1.0}
    kopf = lambda co: {"Kopf": 1.0}
    f.kugel("Vorderkoerper", Vector((0, -0.25, 1.45)), (0.7, 0.95, 0.5), chitin, koerper, 14, 10)
    f.kugel("Kopf", Vector((0, -1.1, 1.4)), (0.42, 0.45, 0.35), chitin_hell, kopf, 12, 8)

    def leib_farbe(poly):
        p = poly.center
        if poly.normal.z > 0.45 and (int((p.y - 0.5) * 3.0) % 2 == 0) and abs(p.x) < 0.5:
            return zeichnung
        return chitin * (0.8 + 0.35 * max(0.0, poly.normal.z)) * r.uniform(0.92, 1.08)
    bm = bmesh.new()
    bmesh.ops.create_uvsphere(bm, u_segments=16, v_segments=10, radius=1.0)
    for v in bm.verts:
        v.co = Vector((v.co.x * 1.0, v.co.y * 1.3, v.co.z * 0.9)) + Vector((0, 1.5, 1.75))
    f._objekt(bm, "Hinterleib", leib_farbe, hinterleib)
    for k in range(8):
        x = (k % 4 - 1.5) * 0.12
        z = 1.5 + (k // 4) * 0.12
        f.kugel("Auge", Vector((x, -1.45, z)), (0.05, 0.035, 0.05), SCHATTEN, kopf, 6, 4)
    for sx in (-1, 1):
        f.straehne("Klaue", [Vector((sx * 0.14, -1.4, 1.25)), Vector((sx * 0.18, -1.6, 1.0)), Vector((sx * 0.08, -1.62, 0.8))], 0.08, 0.01, farbe("#2A1E14"), kopf, 6)
    for i in range(4):
        for seite, sn in ((1, "L"), (-1, "R")):
            ansatz, knie, fuss, aussen = _spinne_bein(i, seite)
            oben = lambda co, n=f"Bein{i}{sn}.oben": {n: 1.0}
            unten = lambda co, n=f"Bein{i}{sn}.unten": {n: 1.0}
            _strecke(f, "Oberbein", ansatz, knie, 0.15, 0.11, chitin_hell, oben, 7)
            f.kugel("Kniegelenk", knie, (0.13, 0.13, 0.13), chitin, oben, 8, 6)
            _strecke(f, "Unterbein", knie, fuss, 0.1, 0.035, chitin, unten, 7)
            f.knochen_dazu(f"Bein{i}{sn}.oben", ansatz, knie, "Koerper", (0, 0, 1))
            f.knochen_dazu(f"Bein{i}{sn}.unten", knie, fuss, f"Bein{i}{sn}.oben", tuple(aussen))
    f.knochen_dazu("Koerper", (0, 0.5, 1.45), (0, -0.8, 1.45), None, (0, 0, 1))
    f.knochen_dazu("Hinterleib", (0, 0.45, 1.6), (0, 2.7, 1.8), "Koerper", (0, 0, 1))
    f.knochen_dazu("Kopf", (0, -0.8, 1.45), (0, -1.6, 1.35), "Koerper", (0, 0, 1))
    f.knochen = sorted(f.knochen, key=lambda k: (0 if k[3] is None else 1 if k[3] == "Koerper" and not k[0].startswith("Bein") else 2 if k[0].endswith(".oben") else 3))
    # Alles verkleinern: Teile und Knochen
    _skalieren(f, groesse)
    f.knochen = [(n, tuple(Vector(a) * groesse), tuple(Vector(e) * groesse), p, o) for n, a, e, p, o in f.knochen]
    _boden(f)
    return f.fertig(_spinne_animationen)
