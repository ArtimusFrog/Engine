"""Sehenswürdigkeiten der Insel im Mid-Poly-Stil: Wachturm-Ruine, Steinkreis, Schrein mit Steg
und Ruderboot, Schiffswrack, Leuchtturm, Höhleneingang, dazu Treibholz und Muscheln.

Baut auf den Bausteinen aus lager.py auf (Bretter, Stämme, Steine, Vertexfarben, Glut-Material).
Maße in Metern; wo das Spiel Kollisionen nachbildet (Turm, Steinkreis), stehen die Maße als
Konstanten auch in game/src/orte.rs.
"""

import math
import random

import bmesh
from mathutils import Matrix, Vector

from lager import brett_bm, einfarbig, fertig, holzfarbe, objekt, setzen, stamm_bm, stammfarbe, stein_bm
from vorkommen import _steinfarbe, farbe


def _moosstein(zufall, hell="#A39E93"):
    """Verwitterter Stein: grau, oben mit Moos."""
    grau = _steinfarbe(zufall, farbe("#5C5953"), farbe("#7E7A72"), farbe(hell), farbe("#C4BEB2"))
    moos = farbe("#5A6A3C")

    def von(poly, fase=False):
        if poly.normal.z > 0.7 and zufall.random() < 0.55:
            return moos * zufall.uniform(0.85, 1.1)
        return grau(poly, fase)
    return von


# ---------------------------------------------------------------------------
# Wachturm-Ruine
# ---------------------------------------------------------------------------
TURM_RADIUS = 2.4
TURM_WAND = 0.55


def wachturm(seed=31):
    """Runder Turm aus Steinquadern, oben gebrochen (auf einer Seite hoch, auf der anderen
    niedrig), Tor nach +X, Schießscharte, Schutt drinnen und draußen, ein morscher Balken."""
    zufall = random.Random(seed)
    teile = []
    stein = _moosstein(zufall)
    reihen, hoehe_reihe = 11, 0.42
    for reihe in range(reihen):
        umfang = math.tau * TURM_RADIUS
        anzahl = int(umfang / 0.85)
        versatz = 0.5 if reihe % 2 else 0.0
        for k in range(anzahl):
            w = math.tau * (k + versatz) / anzahl
            # gebrochene Krone: nach Westen (−X) hoch, nach Osten niedrig, dazu Zacken
            hoch = 5 + 6 * (0.5 - 0.5 * math.cos(w)) + 2 * math.sin(w * 3 + 1.0)
            if reihe > hoch:
                continue
            # Tor (Richtung +X) und Schießscharte
            if (reihe < 6 and abs(math.atan2(math.sin(w), math.cos(w))) < 0.28) or (reihe in (7, 8) and abs(w - math.pi) < 0.12):
                continue
            laenge = umfang / anzahl - 0.03
            bm = brett_bm(laenge * zufall.uniform(0.96, 1.0), TURM_WAND * zufall.uniform(0.92, 1.0), hoehe_reihe * 0.96, fase=0.0)
            setzen(bm, (0, 0, 0), (0, 0, math.degrees(w) + 90))
            r = TURM_RADIUS + zufall.uniform(-0.03, 0.03)
            setzen(bm, (math.cos(w) * r, math.sin(w) * r, hoehe_reihe * (reihe + 0.5)))
            teile.append(objekt("Quader", bm, stein))
    # Torsturz über dem Eingang
    bm = brett_bm(TURM_WAND + 0.1, 1.5, 0.35, fase=0.02)
    setzen(bm, (TURM_RADIUS, 0, hoehe_reihe * 6 + 0.15))
    teile.append(objekt("Sturz", bm, stein))
    # Boden drinnen und Schutt
    bm = bmesh.new()
    bmesh.ops.create_circle(bm, cap_ends=True, segments=20, radius=TURM_RADIUS - 0.2)
    setzen(bm, (0, 0, 0.03))
    teile.append(objekt("Boden", bm, einfarbig("#6E6A62", 0.08, zufall)))
    for i in range(22):
        innen = i < 10
        w = zufall.uniform(0, math.tau)
        d = zufall.uniform(0.2, TURM_RADIUS - 0.6) if innen else zufall.uniform(TURM_RADIUS + 0.5, TURM_RADIUS + 3.0)
        r = zufall.uniform(0.15, 0.32)
        bm = brett_bm(r * 2, r * 1.4, r, fase=0.0) if zufall.random() < 0.6 else stein_bm(zufall, r)
        setzen(bm, (math.cos(w) * d, math.sin(w) * d, r * 0.4), (zufall.uniform(-20, 20), zufall.uniform(-20, 20), zufall.uniform(0, 360)))
        teile.append(objekt("Schutt", bm, stein))
    # Morscher Balken, der aus der Wand ragt
    bm = stamm_bm(0.1, 2.6, ecken=8, seed=seed)
    setzen(bm, (-TURM_RADIUS - 0.3, -0.4, 3.3), (0, 8, 25))
    teile.append(objekt("Balken", bm, stammfarbe(zufall, "#4A3A2C", "#2E241B", "#8C7A60", "#6A5A44")))
    return fertig("Wachturm", teile)


# ---------------------------------------------------------------------------
# Steinkreis
# ---------------------------------------------------------------------------
KREIS_RADIUS = 5.0
KREIS_STEINE = 9


def steinkreis(seed=32):
    """Neun hohe Steine im Kreis, zwei davon mit Deckstein, in der Mitte ein flacher Altarstein
    mit leuchtender Rune."""
    zufall = random.Random(seed)
    teile, leuchtend = [], []
    stein = _moosstein(zufall, "#9A968C")
    hoehen = []
    for i in range(KREIS_STEINE):
        w = math.tau * i / KREIS_STEINE
        h = zufall.uniform(2.2, 3.0)
        hoehen.append(h)
        # Stehender Stein: grobe Platte, oben schmaler und schräg gebrochen, fest im Boden
        bm = brett_bm(1.0, 0.6, h + 0.3, fase=0.06)
        for v in bm.verts:
            oben = (v.co.z + (h + 0.3) / 2) / (h + 0.3)
            v.co.x *= 1.0 - 0.3 * oben
            v.co.y *= 1.0 - 0.2 * oben
            v.co.z -= oben * v.co.x * 0.35
            v.co += Vector((zufall.uniform(-1, 1), zufall.uniform(-1, 1), zufall.uniform(-1, 1))) * 0.04
        setzen(bm, (0, 0, (h + 0.3) / 2 - 0.3), (zufall.uniform(-4, 4), zufall.uniform(-4, 4), math.degrees(w) + 90))
        setzen(bm, (math.cos(w) * KREIS_RADIUS, math.sin(w) * KREIS_RADIUS, 0))
        teile.append(objekt(f"Stein{i}", bm, stein))
    # Decksteine über zwei Paaren
    for i in (0, 4):
        w1, w2 = math.tau * i / KREIS_STEINE, math.tau * (i + 1) / KREIS_STEINE
        mitte = (Vector((math.cos(w1), math.sin(w1), 0)) + Vector((math.cos(w2), math.sin(w2), 0))) / 2 * KREIS_RADIUS
        spann = (Vector((math.cos(w2), math.sin(w2), 0)) - Vector((math.cos(w1), math.sin(w1), 0))) * KREIS_RADIUS
        z = min(hoehen[i], hoehen[(i + 1) % KREIS_STEINE]) - 0.1
        bm = brett_bm(spann.length + 0.9, 0.55, 0.45, fase=0.05)
        setzen(bm, (0, 0, 0), (0, zufall.uniform(-3, 3), math.degrees(math.atan2(spann.y, spann.x))))
        setzen(bm, (mitte.x, mitte.y, z + 0.2))
        teile.append(objekt("Deckstein", bm, stein))
    # Altarstein mit Rune
    bm = stein_bm(zufall, 0.9, flach=0.45)
    setzen(bm, (0, 0, 0.2))
    teile.append(objekt("Altar", bm, stein))
    bm = bmesh.new()
    # Rune: Ring und drei Striche, flach auf dem Altar
    for k in range(3):
        w = math.tau * k / 3 + 0.3
        b = brett_bm(0.7, 0.05, 0.02, fase=0.0)
        setzen(b, (0, 0, 0), (0, 0, math.degrees(w)))
        b.to_mesh(tmp := __import__("bpy").data.meshes.new("tmp"))
        bm.from_mesh(tmp)
        b.free()
    ring = bmesh.new()
    bmesh.ops.create_circle(ring, cap_ends=False, segments=24, radius=0.38)
    geom = bmesh.ops.extrude_edge_only(ring, edges=ring.edges[:])
    for v in [e for e in geom["geom"] if isinstance(e, bmesh.types.BMVert)]:
        v.co *= 0.88
    ring.to_mesh(tmp2 := __import__("bpy").data.meshes.new("tmp2"))
    bm.from_mesh(tmp2)
    ring.free()
    setzen(bm, (0, 0, 0.62))
    leuchtend.append(objekt("Rune", bm, einfarbig("#5FE0FF", 0.05, zufall)))
    return fertig("Steinkreis", teile, leuchtend, ("#6FE8FF", 3.0))


# ---------------------------------------------------------------------------
# Schrein, Steg, Ruderboot
# ---------------------------------------------------------------------------
def schrein(seed=33):
    """Kleiner offener Schrein: Steinsockel mit Stufen, vier Säulen, Holzdach mit Ziegeln, innen
    eine Laterne, die nachts leuchtet."""
    zufall = random.Random(seed)
    teile, leuchtend = [], []
    stein = _moosstein(zufall, "#B0AA9E")
    for stufe, (breite, hoehe) in enumerate(((3.2, 0.2), (2.8, 0.2), (2.5, 0.2))):
        bm = brett_bm(breite, breite, hoehe, fase=0.03)
        setzen(bm, (0, 0, hoehe * (stufe + 0.5)))
        teile.append(objekt("Stufe", bm, stein))
    for x in (-0.95, 0.95):
        for y in (-0.95, 0.95):
            bm = stamm_bm(0.16, 2.1, ecken=10, knorrig=0.02, seed=seed)
            setzen(bm, (0, 0, 0), (0, -90, 0))
            setzen(bm, (x, y, 0.6))
            teile.append(objekt("Saeule", bm, stein))
    # Dach: zwei geneigte Holzflächen mit Ziegelreihen
    holz = holzfarbe(zufall, "#6E4A2C", "#4A301C")
    ziegel = einfarbig("#8A3B2A", 0.08, zufall)
    for seite in (-1, 1):
        bm = brett_bm(3.0, 1.8, 0.08, fase=0.02)
        setzen(bm, (0, 0, 0), (-seite * 28, 0, 0))
        setzen(bm, (0, seite * 0.75, 3.1))
        teile.append(objekt("Dach", bm, ziegel))
        for r in range(4):
            bm = brett_bm(3.05, 0.06, 0.06, fase=0.0)
            setzen(bm, (0, 0, 0), (-seite * 28, 0, 0))
            setzen(bm, (0, seite * (0.2 + r * 0.4), 3.47 - r * 0.21))
            teile.append(objekt("Ziegelreihe", bm, einfarbig("#6E2C1F", 0.05, zufall)))
    bm = brett_bm(3.2, 0.14, 0.14, fase=0.02)
    setzen(bm, (0, 0, 3.55))
    teile.append(objekt("First", bm, holz))
    for y in (-0.95, 0.95):
        bm = brett_bm(2.3, 0.16, 0.2, fase=0.02)
        setzen(bm, (0, y, 2.75))
        teile.append(objekt("Rahmen", bm, holz))
    # Laterne auf einem kleinen Sockel
    bm = brett_bm(0.4, 0.4, 0.6, fase=0.03)
    setzen(bm, (0, 0, 0.9))
    teile.append(objekt("Sockel", bm, stein))
    bm = brett_bm(0.28, 0.28, 0.36, fase=0.02)
    setzen(bm, (0, 0, 1.38))
    leuchtend.append(objekt("Licht", bm, einfarbig("#FFD27A", 0.04, zufall)))
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, segments=4, radius1=0.3, radius2=0.02, depth=0.25)
    setzen(bm, (0, 0, 1.68), (0, 0, 45))
    teile.append(objekt("Laternendach", bm, einfarbig("#3E3A36", 0.05, zufall)))
    return fertig("Schrein", teile, leuchtend, ("#FFC060", 2.5))


STEG_LAENGE = 7.0
STEG_HOEHE = 0.45


def steg(seed=34):
    """Holzsteg entlang +X (Ursprung am Ufer), Laufbretter auf zwei Balken, Pfähle ins Wasser."""
    zufall = random.Random(seed)
    teile = []
    holz = holzfarbe(zufall, "#8A6440", "#5E4128")
    for i in range(int(STEG_LAENGE / 0.26)):
        bm = brett_bm(0.24, 1.3 + zufall.uniform(-0.05, 0.05), 0.05, fase=0.008)
        setzen(bm, (0.13 + i * 0.26, zufall.uniform(-0.03, 0.03), STEG_HOEHE), (0, 0, zufall.uniform(-2, 2)))
        teile.append(objekt("Brett", bm, holzfarbe(zufall, "#9A7048", "#6A4A2C")))
    for y in (-0.5, 0.5):
        bm = brett_bm(STEG_LAENGE, 0.12, 0.14, fase=0.01)
        setzen(bm, (STEG_LAENGE / 2, y, STEG_HOEHE - 0.1))
        teile.append(objekt("Balken", bm, holz))
    for x in (0.4, STEG_LAENGE / 2, STEG_LAENGE - 0.2):
        for y in (-0.62, 0.62):
            bm = stamm_bm(0.08, 2.2, ecken=8, seed=int(x * 10 + y * 5))
            setzen(bm, (0, 0, 0), (0, 90, 0))
            setzen(bm, (x, y, STEG_HOEHE + 0.35))
            teile.append(objekt("Pfahl", bm, stammfarbe(zufall)))
    return fertig("Steg", teile)


def ruderboot(seed=35):
    """Kleines Ruderboot entlang +X: Rumpf aus Planken, zwei Sitzbänke, zwei Ruder."""
    zufall = random.Random(seed)
    teile = []
    laenge, breite, tiefe = 3.0, 1.1, 0.45
    bm = bmesh.new()
    ringe = []
    for i in range(9):
        t = i / 8
        x = (t - 0.5) * laenge
        form = math.sin(t * math.pi) ** 0.55
        spitz = 0.06
        ring = []
        for k in range(9):
            s = k / 8  # 0 = linke Kante oben, 0.5 = Kiel, 1 = rechte Kante oben
            w = (s - 0.5) * math.pi
            y = math.sin(w) * breite / 2 * max(form, spitz)
            z = tiefe - math.cos(w) * tiefe * max(form, 0.35) + (abs(t - 0.5) ** 2) * 0.5
            ring.append(bm.verts.new((x, y, z)))
        ringe.append(ring)
    for a, b in zip(ringe, ringe[1:]):
        for k in range(8):
            bm.faces.new((a[k], b[k], b[k + 1], a[k + 1]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bmesh.ops.solidify(bm, geom=bm.faces[:], thickness=0.04)
    rumpf = holzfarbe(zufall, "#7A5234", "#4E3420", 60.0)
    teile.append(objekt("Rumpf", bm, lambda poly, fase=False: rumpf(poly) if poly.center.z > 0.12 else rumpf(poly) * 0.55))
    for x in (-0.5, 0.5):
        bm = brett_bm(0.25, breite * 0.8, 0.04, fase=0.008)
        setzen(bm, (x, 0, tiefe * 0.75))
        teile.append(objekt("Bank", bm, holzfarbe(zufall, "#9A7048", "#6A4A2C")))
    for seite in (-1, 1):
        bm = stamm_bm(0.025, 2.2, ecken=6, knorrig=0.0)
        setzen(bm, (-1.1, seite * 0.3, tiefe * 0.8), (0, -4, seite * 6))
        teile.append(objekt("Ruder", bm, holzfarbe(zufall, "#A8804F", "#7E5A33")))
        bm = brett_bm(0.45, 0.02, 0.14, fase=0.004)
        setzen(bm, (1.0, seite * 0.3 + seite * 0.23, tiefe * 0.8), (0, 0, seite * 6))
        teile.append(objekt("Ruderblatt", bm, holzfarbe(zufall, "#A8804F", "#7E5A33")))
    return fertig("Ruderboot", teile)


# ---------------------------------------------------------------------------
# Schiffswrack
# ---------------------------------------------------------------------------
def schiffswrack(seed=36):
    """Halb im Sand versunkener Rumpf, auf die Seite gekippt: Kiel, Spanten, lückenhafte Planken,
    gebrochener Mast mit Segelfetzen, verstreute Fracht."""
    zufall = random.Random(seed)
    teile = []
    laenge, breite, hoehe = 9.0, 3.0, 2.4
    holz = holzfarbe(zufall, "#6E5236", "#433222", 40.0)
    alt = holzfarbe(zufall, "#8A7A64", "#5E5244", 40.0)

    def spant_punkt(t, s):
        x = (t - 0.5) * laenge
        form = math.sin(t * math.pi) ** 0.5
        w = (s - 0.5) * math.pi
        return Vector((x, math.sin(w) * breite / 2 * max(form, 0.08), hoehe - math.cos(w) * hoehe * max(form, 0.3) + (t - 0.5) ** 2 * 1.2))

    # Kiel
    bm = brett_bm(laenge * 0.95, 0.22, 0.3, fase=0.02)
    setzen(bm, (0, 0, 0.1))
    teile.append(objekt("Kiel", bm, holz))
    # Spanten (Rippen)
    for i in range(1, 12):
        t = i / 12
        punkte = [spant_punkt(t, s / 10) for s in range(11)]
        for a, b in zip(punkte, punkte[1:]):
            richtung = b - a
            bm = stamm_bm(0.07, richtung.length, ecken=6, knorrig=0.0, schritte=1)
            dreh = Vector((1, 0, 0)).rotation_difference(richtung.normalized()).to_matrix().to_4x4()
            bmesh.ops.transform(bm, matrix=Matrix.Translation(a) @ dreh, verts=bm.verts)
            teile.append(objekt("Spant", bm, holz))
    # Planken mit Lücken (auf der rechten Seite mehr erhalten)
    for reihe in range(9):
        s = 0.06 + reihe * 0.11
        for seg in range(8):
            t0, t1 = 0.06 + seg * 0.11, 0.06 + (seg + 1) * 0.11
            erhalten = 0.85 if s > 0.5 else 0.45
            if zufall.random() > erhalten:
                continue
            a, b = spant_punkt(t0, s), spant_punkt(t1, s)
            richtung = b - a
            bm = brett_bm(richtung.length + 0.05, 0.05, 0.28, fase=0.0)
            normale = Vector((0, 1 if s > 0.5 else -1, 0.3)).normalized()
            dreh = Vector((1, 0, 0)).rotation_difference(richtung.normalized()).to_matrix().to_4x4()
            bmesh.ops.transform(bm, matrix=Matrix.Translation((a + b) / 2 + normale * 0.04) @ dreh, verts=bm.verts)
            teile.append(objekt("Planke", bm, alt))
    # Gebrochener Mast mit Segelfetzen
    bm = stamm_bm(0.16, 3.2, ecken=10, radius_ende=0.12, seed=seed)
    setzen(bm, (0, 0, 0), (0, -70, 0))
    setzen(bm, (0.5, 0, 0.3))
    teile.append(objekt("Mast", bm, holz))
    bm = bmesh.new()
    v = [bm.verts.new(p) for p in ((1.2, 0.05, 2.6), (1.2, 1.4, 1.6), (2.4, 0.9, 2.1), (1.9, 0.1, 3.2))]
    bm.faces.new(v)
    bmesh.ops.solidify(bm, geom=bm.faces[:], thickness=0.02)
    teile.append(objekt("Segel", bm, einfarbig("#CFC3A4", 0.06, zufall)))
    # Fracht im Sand
    for i in range(3):
        bm = brett_bm(0.5, 0.5, 0.45, fase=0.02)
        setzen(bm, (zufall.uniform(-4.5, 4.5), zufall.uniform(2.2, 3.8) * (1 if i % 2 else -1), 0.12), (zufall.uniform(-15, 15), zufall.uniform(-15, 15), zufall.uniform(0, 90)))
        teile.append(objekt("Kiste", bm, alt))
    # Alles auf die Seite gekippt und halb im Sand
    obj = fertig("Schiffswrack", teile)
    obj.data.transform(Matrix.Rotation(math.radians(24), 4, "X"))
    obj.data.transform(Matrix.Translation((0, 0, -0.55)))
    # Was unter dem Sand liegt, abschneiden: dann beginnt das Modell am Boden (Asset-Regel)
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bmesh.ops.bisect_plane(bm, geom=bm.verts[:] + bm.edges[:] + bm.faces[:], plane_co=(0, 0, 0), plane_no=(0, 0, 1), clear_inner=True)
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.update()
    return obj


# ---------------------------------------------------------------------------
# Leuchtturm
# ---------------------------------------------------------------------------
LEUCHTTURM_HOEHE = 9.0


def leuchtturm(seed=37):
    """Rot-weiß gestreifter Turm auf einem Steinsockel, Galerie mit Geländer, Laternenhaus mit
    Licht (leuchtet), rotes Kegeldach, Tür."""
    zufall = random.Random(seed)
    teile, leuchtend = [], []
    stein = _moosstein(zufall, "#A8A296")
    bm = stein_bm(zufall, 2.4, flach=0.35)
    setzen(bm, (0, 0, 0.2))
    teile.append(objekt("Sockel", bm, stein))
    # Turm: Kegelstumpf, Farbe nach Höhe in Streifen
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, segments=20, radius1=1.35, radius2=0.95, depth=LEUCHTTURM_HOEHE)
    bmesh.ops.subdivide_edges(bm, edges=[e for e in bm.edges if abs(e.verts[0].co.z - e.verts[1].co.z) > 1.0], cuts=11, use_grid_fill=True)
    setzen(bm, (0, 0, LEUCHTTURM_HOEHE / 2 + 0.5))
    rot, weiss = farbe("#B23A2E"), farbe("#EFEBE2")
    teile.append(objekt("Turm", bm, lambda poly, fase=False: (rot if int((poly.center.z - 0.5) / 1.5) % 2 else weiss) * zufall.uniform(0.95, 1.04)))
    # Tür
    bm = brett_bm(0.1, 0.8, 1.6, fase=0.02)
    setzen(bm, (1.33, 0, 1.3))
    teile.append(objekt("Tuer", bm, holzfarbe(zufall, "#5E3E22", "#3E2814")))
    top = LEUCHTTURM_HOEHE + 0.5
    # Galerie mit Geländer
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, segments=20, radius1=1.45, radius2=1.45, depth=0.18)
    setzen(bm, (0, 0, top + 0.09))
    teile.append(objekt("Galerie", bm, einfarbig("#3C3E42", 0.04, zufall)))
    for k in range(16):
        w = math.tau * k / 16
        bm = stamm_bm(0.025, 0.7, ecken=5, knorrig=0.0)
        setzen(bm, (0, 0, 0), (0, -90, 0))
        setzen(bm, (math.cos(w) * 1.38, math.sin(w) * 1.38, top + 0.18))
        teile.append(objekt("Stab", bm, einfarbig("#2E3034", 0.04, zufall)))
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=False, segments=20, radius1=1.4, radius2=1.4, depth=0.05)
    setzen(bm, (0, 0, top + 0.88))
    teile.append(objekt("Handlauf", bm, einfarbig("#2E3034", 0.04, zufall)))
    # Laternenhaus (leuchtet) und Dach
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, segments=8, radius1=0.8, radius2=0.8, depth=1.1)
    setzen(bm, (0, 0, top + 0.75))
    leuchtend.append(objekt("Laterne", bm, einfarbig("#FFE7A0", 0.03, zufall)))
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, segments=8, radius1=1.0, radius2=0.05, depth=0.9)
    setzen(bm, (0, 0, top + 1.75))
    teile.append(objekt("Dach", bm, einfarbig("#9E2E24", 0.05, zufall)))
    return fertig("Leuchtturm", teile, leuchtend, ("#FFE08A", 3.0))


# ---------------------------------------------------------------------------
# Höhleneingang
# ---------------------------------------------------------------------------
def hoehle(seed=38):
    """Felstor vor einem dunklen Loch: große Brocken bilden einen Bogen, dahinter eine dunkle
    Mulde (wirkt wie ein tiefer Gang). Öffnung nach +X."""
    zufall = random.Random(seed)
    teile = []
    fels = _steinfarbe(zufall, farbe("#4E4A45"), farbe("#6E6961"), farbe("#8E887D"), farbe("#A8A195"))
    # Bogen aus Brocken
    for i in range(9):
        t = i / 8
        w = math.pi * t
        r = zufall.uniform(0.7, 1.0)
        bm = stein_bm(zufall, r, flach=0.9)
        setzen(bm, (0.3, math.cos(w) * 2.2, 0.3 + math.sin(w) * 2.6), (zufall.uniform(0, 360), zufall.uniform(0, 360), zufall.uniform(0, 360)))
        teile.append(objekt(f"Bogen{i}", bm, fels))
    # Felsmasse drumherum und dahinter
    for i in range(8):
        r = zufall.uniform(1.2, 1.8)
        bm = stein_bm(zufall, r, flach=0.8)
        setzen(bm, (zufall.uniform(-2.5, -0.8), zufall.uniform(-3.5, 3.5), zufall.uniform(0.5, 3.8)), (0, 0, zufall.uniform(0, 360)))
        teile.append(objekt(f"Fels{i}", bm, fels))
    # Dunkler Gang: nach innen gewölbte Halbkugel, fast schwarz
    bm = bmesh.new()
    bmesh.ops.create_uvsphere(bm, u_segments=14, v_segments=8, radius=1.0)
    bmesh.ops.delete(bm, geom=[v for v in bm.verts if v.co.x > 0.05 or v.co.z < -0.1], context="VERTS")
    bmesh.ops.transform(bm, matrix=Matrix.Diagonal((1.6, 1.9, 2.3, 1.0)), verts=bm.verts)
    bmesh.ops.reverse_faces(bm, faces=bm.faces[:])
    setzen(bm, (0.4, 0, 0.05))
    teile.append(objekt("Dunkel", bm, einfarbig("#0B0A0A", 0.1, zufall)))
    # Geröll vor dem Eingang
    for i in range(10):
        r = zufall.uniform(0.12, 0.35)
        bm = stein_bm(zufall, r)
        setzen(bm, (zufall.uniform(0.8, 3.0), zufall.uniform(-2.5, 2.5), r * 0.3))
        teile.append(objekt("Geroell", bm, fels))
    return fertig("Hoehle", teile)


# ---------------------------------------------------------------------------
# Strandgut
# ---------------------------------------------------------------------------
def treibholz(seed=39):
    """Ausgebleichter, knorriger Ast mit Seitenzweigen, flach im Sand."""
    zufall = random.Random(seed)
    teile = []
    grau = stammfarbe(zufall, "#A89E8E", "#8A8070", "#C8BEA8", "#A89C84")
    laenge = zufall.uniform(1.6, 2.4)
    bm = stamm_bm(0.1, laenge, ecken=8, radius_ende=0.05, knorrig=0.18, seed=seed)
    setzen(bm, (-laenge / 2, 0, 0.07), (0, 0, 0))
    teile.append(objekt("Ast", bm, grau))
    for i in range(3):
        bm = stamm_bm(0.04, zufall.uniform(0.4, 0.8), ecken=6, radius_ende=0.015, knorrig=0.2, seed=seed + i)
        setzen(bm, (zufall.uniform(-0.6, 0.6), 0, 0.08), (0, zufall.uniform(-15, 5), zufall.uniform(30, 150) * (1 if i % 2 else -1)))
        teile.append(objekt("Zweig", bm, grau))
    return fertig("Treibholz", teile)


def muscheln(seed=40):
    """Ein paar Muscheln und Schneckenhäuser in Weiß, Rosa und Beige."""
    zufall = random.Random(seed)
    teile = []
    for i in range(6):
        farbe_hex = zufall.choice(["#F2E6DA", "#E8B8A8", "#D8C4A0", "#F0D8C8"])
        if i % 2:
            # Fächermuschel: flacher Fächer mit Rippen
            bm = bmesh.new()
            mitte = bm.verts.new((0, 0, 0.02))
            rand = [bm.verts.new((math.cos(w) * 0.08, math.sin(w) * 0.08 + 0.02, 0.01 + 0.012 * (k % 2)))
                    for k, w in enumerate([math.radians(20 + 14 * j) for j in range(11)])]
            for a, b in zip(rand, rand[1:]):
                bm.faces.new((mitte, a, b))
        else:
            # Schneckenhaus: kleiner, spitzer Kegel
            bm = bmesh.new()
            bmesh.ops.create_cone(bm, cap_ends=True, segments=7, radius1=0.04, radius2=0.0, depth=0.09)
            setzen(bm, (0, 0, 0.03), (80, 0, 0))
        setzen(bm, (zufall.uniform(-0.6, 0.6), zufall.uniform(-0.6, 0.6), 0), (0, 0, zufall.uniform(0, 360)))
        teile.append(objekt(f"Muschel{i}", bm, einfarbig(farbe_hex, 0.06, zufall)))
    return fertig("Muscheln", teile)


def seerosen(seed=42):
    """Ein Teppich Seerosenblätter (flache, eingeschnittene Scheiben) mit zwei, drei Blüten."""
    zufall = random.Random(seed)
    teile = []
    for i in range(zufall.randint(6, 9)):
        r = zufall.uniform(0.18, 0.32)
        w0 = zufall.uniform(0, math.tau)
        bm = bmesh.new()
        mitte = bm.verts.new((0, 0, 0))
        rand = [bm.verts.new((math.cos(w0 + t) * r, math.sin(w0 + t) * r, 0)) for t in [0.35 + k * (math.tau - 0.7) / 12 for k in range(13)]]
        for a, b in zip(rand, rand[1:]):
            bm.faces.new((mitte, a, b))
        setzen(bm, (zufall.uniform(-0.9, 0.9), zufall.uniform(-0.9, 0.9), 0.01))
        teile.append(objekt(f"Blatt{i}", bm, einfarbig(zufall.choice(["#3F7A2E", "#4E8A34", "#5A9A3A"]), 0.06, zufall)))
    for i in range(zufall.randint(2, 3)):
        at = (zufall.uniform(-0.7, 0.7), zufall.uniform(-0.7, 0.7), 0.02)
        blume = zufall.choice(["#F4F0F2", "#F2B8D0", "#F7E08A"])
        for ring, (anzahl, laenge, hoch) in enumerate(((7, 0.13, 25), (5, 0.09, 55))):
            for k in range(anzahl):
                w = math.degrees(math.tau * k / anzahl + ring * 0.4)
                bm = brett_bm(laenge, 0.045, 0.008, fase=0.0)
                setzen(bm, (laenge / 2, 0, 0), (0, -hoch, 0))
                setzen(bm, at, (0, 0, w))
                teile.append(objekt("Bluetenblatt", bm, einfarbig(blume, 0.05, zufall)))
        bm = brett_bm(0.05, 0.05, 0.04, fase=0.0)
        setzen(bm, (at[0], at[1], at[2] + 0.03))
        teile.append(objekt("Stempel", bm, einfarbig("#E8B83A", 0.05, zufall)))
    return fertig("Seerosen", teile)
