"""Waldspinne – kleine braune Spinne mit gelber Zeichnung, gebaut wie die Spinnenkönigin (bosse.py).
Sie haust in der Spinnengrotte (dungeon.rs). Animationen: Idle, Laufen, Angriff.

(Früher standen hier auch die Banditen der Wildnislager; die Wildnis bevölkern jetzt die Streuner,
siehe streuner.py.)
"""

import bmesh
from mathutils import Vector

from bosse import _boden, _spinne_animationen, _spinne_bein
from figuren import Figur, farbe
from gegner import SCHATTEN, _strecke


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
