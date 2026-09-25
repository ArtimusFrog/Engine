"""Das Startlager und andere gebaute Dinge im Mid-Poly-Stil: Lagerfeuer, Zelte, Holzstapel,
Karren, Wegweiser, Bänke, Kisten und Fässer.

Alles aus wenigen Bausteinen (Bretter mit Fase, Stämme mit Rinde und Hirnholz, Steine, Stoff),
Farben als Vertexfarben je Fläche mit leichter Maserung. Leuchtende Teile (Glut) bekommen ein
eigenes Material mit Emission – die Engine lässt sie glühen.
"""

import math
import random

import bmesh
import bpy
from mathutils import Matrix, Vector

from vorkommen import _brocken_bm, _material, _objekt, _setzen, _steinfarbe, farbe
from werkstatt import srgb_zu_linear, ursprung_unten, vereinen

X, Y, Z = Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))


# ---------------------------------------------------------------------------
# Farben
# ---------------------------------------------------------------------------
def holzfarbe(zufall, grund="#9A6B3F", dunkel="#6E4826", maserung=22.0):
    """Holz mit Maserung entlang der längsten Achse, Kanten (kleine Flächen) etwas heller."""
    g, d = farbe(grund), farbe(dunkel)
    ton = zufall.uniform(0.9, 1.1)

    def von(poly, fase=False):
        p = poly.center
        streifen = 0.5 + 0.5 * math.sin((p.x + p.y * 0.7 + p.z * 0.3) * maserung + zufall.random() * 0.8)
        c = g.lerp(d, streifen * 0.45) * ton * zufall.uniform(0.95, 1.05)
        if fase:
            c = c * 1.18
        return c
    return von


def einfarbig(hex_farbe, schwankung=0.05, zufall=None):
    c = farbe(hex_farbe)
    zufall = zufall or random.Random(1)
    return lambda poly, fase=False: c * zufall.uniform(1 - schwankung, 1 + schwankung)


# ---------------------------------------------------------------------------
# Bausteine (liefern bmesh, gesetzt in Weltkoordinaten)
# ---------------------------------------------------------------------------
def _fase_markieren(bm, ergebnis):
    schicht = bm.faces.layers.int.get("fase")
    for f in ergebnis.get("faces", []):
        f[schicht] = 1


def brett_bm(laenge, breite, dicke, fase=0.006):
    """Quader (Länge entlang X) mit schmaler Fase an allen Kanten, Mitte im Ursprung."""
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.transform(bm, matrix=Matrix.Diagonal((laenge, breite, dicke, 1.0)), verts=bm.verts)
    # Schicht vor dem Anfasen anlegen (später angelegt, würden die Flächen ungültig)
    bm.faces.layers.int.new("fase")
    if fase > 0:
        ergebnis = bmesh.ops.bevel(bm, geom=bm.edges[:], offset=min(fase, dicke * 0.3, breite * 0.3), offset_type="OFFSET",
                                   segments=1, profile=0.5, affect="EDGES", clamp_overlap=True)
        _fase_markieren(bm, ergebnis)
    return bm


def stamm_bm(radius, laenge, ecken=10, radius_ende=None, knorrig=0.08, seed=1):
    """Holzstamm entlang +X ab dem Ursprung, leicht unregelmäßig; Deckflächen markiert (Hirnholz)."""
    zufall = random.Random(seed)
    bm = bmesh.new()
    ende = radius if radius_ende is None else radius_ende
    ringe = []
    schritte = 4
    for i in range(schritte + 1):
        t = i / schritte
        r = radius + (ende - radius) * t
        ring = []
        for k in range(ecken):
            w = math.tau * k / ecken
            rr = r * (1 + zufall.uniform(-knorrig, knorrig))
            ring.append(bm.verts.new((laenge * t, math.cos(w) * rr, math.sin(w) * rr)))
        ringe.append(ring)
    for a, b in zip(ringe, ringe[1:]):
        for k in range(ecken):
            bm.faces.new((a[k], a[(k + 1) % ecken], b[(k + 1) % ecken], b[k]))
    schicht = bm.faces.layers.int.new("fase")
    for ring, umdrehen in ((ringe[0], True), (ringe[-1], False)):
        f = bm.faces.new(list(reversed(ring)) if umdrehen else ring)
        f[schicht] = 2  # Hirnholz
    return bm


def stammfarbe(zufall, rinde="#5E3D22", rinde_dunkel="#3F2816", hirn="#D8A868", hirn_dunkel="#9C6A38"):
    """Rinde außen (mit Längsrillen), Hirnholz an den Enden hell mit Ringen."""
    r, rd, h, hd = farbe(rinde), farbe(rinde_dunkel), farbe(hirn), farbe(hirn_dunkel)

    def von(poly, fase=False):
        if fase == 2:
            return h.lerp(hd, zufall.uniform(0.0, 0.5))
        return r.lerp(rd, zufall.uniform(0.0, 0.7)) * zufall.uniform(0.92, 1.08)
    return von


def stein_bm(zufall, radius, flach=0.6):
    """Kleiner, kantiger Stein (wenige Flächen, für Feuerstellen und Mauern)."""
    bm = bmesh.new()
    bmesh.ops.create_icosphere(bm, subdivisions=1, radius=radius)
    bmesh.ops.transform(bm, matrix=Matrix.Diagonal((1.0, zufall.uniform(0.8, 1.1), flach, 1.0)), verts=bm.verts)
    for _ in range(zufall.randint(3, 5)):
        n = Vector((zufall.uniform(-1, 1), zufall.uniform(-1, 1), zufall.uniform(-0.2, 1))).normalized()
        bmesh.ops.bisect_plane(bm, geom=bm.verts[:] + bm.edges[:] + bm.faces[:], plane_co=n * radius * zufall.uniform(0.6, 0.85) * max(flach, 0.6),
                               plane_no=n, clear_outer=True)
        rand = [e for e in bm.edges if e.is_boundary]
        if rand:
            bmesh.ops.holes_fill(bm, edges=rand, sides=0)
    bmesh.ops.triangulate(bm, faces=bm.faces[:])
    for v in bm.verts:
        v.co += Vector((zufall.uniform(-1, 1), zufall.uniform(-1, 1), zufall.uniform(-1, 1))) * radius * 0.05
    return bm


def setzen(bm, ort=(0, 0, 0), drehung=(0, 0, 0)):
    """Drehen (Grad um X, Y, Z) und verschieben."""
    rot = (Matrix.Rotation(math.radians(drehung[2]), 4, "Z") @ Matrix.Rotation(math.radians(drehung[1]), 4, "Y")
           @ Matrix.Rotation(math.radians(drehung[0]), 4, "X"))
    bmesh.ops.transform(bm, matrix=Matrix.Translation(Vector(ort)) @ rot, verts=bm.verts)
    return bm


def objekt(name, bm, farbe_von):
    return _objekt(name, bm, farbe_von)


def leucht_material(name, hex_farbe, staerke):
    mat = bpy.data.materials.get(name)
    if mat is None:
        mat = bpy.data.materials.new(name)
        try:
            mat.use_nodes = True
        except (AttributeError, TypeError):
            pass
        knoten, links = mat.node_tree.nodes, mat.node_tree.links
        bsdf = next(k for k in knoten if k.type == "BSDF_PRINCIPLED")
        vc = knoten.new("ShaderNodeVertexColor")
        vc.layer_name = "Farbe"
        links.new(vc.outputs["Color"], bsdf.inputs["Base Color"])
        if "Emission Color" in bsdf.inputs:
            bsdf.inputs["Emission Color"].default_value = srgb_zu_linear(hex_farbe)
            bsdf.inputs["Emission Strength"].default_value = staerke
    return mat


def fertig(name, teile, leuchtend=None, leucht=("#FF8A2A", 2.0)):
    """Teile vereinen, Material zuweisen, Ursprung unten in die Mitte."""
    obj = vereinen(name, teile)
    _material(obj)
    if leuchtend:
        glut = vereinen(name + "Glut", leuchtend)
        glut.data.materials.clear()
        glut.data.materials.append(leucht_material(name + "Leuchten", *leucht))
        for poly in glut.data.polygons:
            poly.material_index = 0
        obj = vereinen(name, [obj, glut])
    ursprung_unten(obj)
    dreiecke = sum(len(p.vertices) - 2 for p in obj.data.polygons)
    print(f"MODELL {name}: {dreiecke} Dreiecke")
    return obj


# ---------------------------------------------------------------------------
# Lagerfeuer
# ---------------------------------------------------------------------------
def lagerfeuer(seed=1):
    zufall = random.Random(seed)
    teile, glut = [], []
    stein = _steinfarbe(zufall, farbe("#57534D"), farbe("#7C776E"), farbe("#A59E92"), farbe("#C7C0B4"))
    # Steinkreis
    anzahl = 11
    for i in range(anzahl):
        w = math.tau * i / anzahl + zufall.uniform(-0.1, 0.1)
        r = zufall.uniform(0.12, 0.17)
        bm = stein_bm(zufall, r)
        _setzen(bm, (math.cos(w) * 0.62, math.sin(w) * 0.62, r * 0.35), w + zufall.uniform(-0.4, 0.4))
        teile.append(objekt(f"Stein{i}", bm, stein))
    # Asche und Glut am Boden
    bm = bmesh.new()
    bmesh.ops.create_circle(bm, cap_ends=True, segments=16, radius=0.5)
    setzen(bm, (0, 0, 0.015))
    teile.append(objekt("Asche", bm, einfarbig("#3A3431", 0.1, zufall)))
    for i in range(14):
        w, d = zufall.uniform(0, math.tau), zufall.uniform(0.0, 0.38)
        k = bmesh.new()
        bmesh.ops.create_icosphere(k, subdivisions=1, radius=zufall.uniform(0.03, 0.06))
        bmesh.ops.transform(k, matrix=Matrix.Diagonal((1.0, 1.0, 0.5, 1.0)), verts=k.verts)
        setzen(k, (math.cos(w) * d, math.sin(w) * d, 0.03))
        glut.append(objekt(f"Glut{i}", k, einfarbig(zufall.choice(["#FF6A1A", "#FF9A30", "#FFC24A"]), 0.1, zufall)))
    # Scheite als Pyramide, unten verkohlt
    rinde = stammfarbe(zufall)
    for i in range(5):
        w = math.tau * i / 5 + zufall.uniform(-0.15, 0.15)
        laenge = zufall.uniform(0.62, 0.75)
        bm = stamm_bm(zufall.uniform(0.045, 0.06), laenge, ecken=8, seed=seed + i)
        # Fuß außen am Rand, Spitze zur Mitte oben
        neigung = math.degrees(math.atan2(0.42, 0.34))
        setzen(bm, (0, 0, 0), (0, -neigung, 0))
        setzen(bm, (math.cos(w) * 0.42, math.sin(w) * 0.42, 0.02), (0, 0, math.degrees(w) + 180))
        verkohlt = farbe("#1E1714")

        def scheit(poly, fase=False, rinde=rinde):
            c = rinde(poly, fase)
            # oben (in der Glut) verkohlt
            return c.lerp(verkohlt, min(1.0, max(0.0, (poly.center.z - 0.05) / 0.25)))
        teile.append(objekt(f"Scheit{i}", bm, scheit))
    return fertig("Lagerfeuer", teile, glut, ("#FF8A2A", 2.5))


def bank(seed=2, laenge=1.6):
    """Halbierter Stamm als Sitzbank auf zwei Klötzen."""
    zufall = random.Random(seed)
    teile = []
    rinde = stammfarbe(zufall)
    for x in (-laenge * 0.35, laenge * 0.35):
        bm = stamm_bm(0.13, 0.28, ecken=10, seed=seed + int(x * 10))
        setzen(bm, (0, 0, 0), (0, -90, 0))
        setzen(bm, (x, 0, 0))
        teile.append(objekt("Klotz", bm, rinde))
    # Halber Stamm: Stamm, oben flach abgeschnitten
    bm = stamm_bm(0.17, laenge, ecken=14, seed=seed)
    setzen(bm, (-laenge / 2, 0, 0.28))
    geom = bm.verts[:] + bm.edges[:] + bm.faces[:]
    bmesh.ops.bisect_plane(bm, geom=geom, plane_co=(0, 0, 0.34), plane_no=(0, 0, 1), clear_outer=True)
    rand = [e for e in bm.edges if e.is_boundary]
    if rand:
        ergebnis = bmesh.ops.holes_fill(bm, edges=rand, sides=0)
        schicht = bm.faces.layers.int.get("fase")
        for f in ergebnis["faces"]:
            f[schicht] = 3  # Sitzfläche: helles Holz
    sitz = holzfarbe(zufall, "#C8955A", "#9A6A3A", 30.0)

    def von(poly, fase=False):
        return sitz(poly) if fase == 3 else rinde(poly, fase)
    teile.append(objekt("Sitz", bm, von))
    return fertig("Bank", teile)


# ---------------------------------------------------------------------------
# Zelt
# ---------------------------------------------------------------------------
def zelt(seed=3, stoff="#D8C8A0", streifen="#A8412F"):
    """Giebelzelt aus Segeltuch: Firststange, zwei Stützen, Plane mit leichtem Durchhang,
    geschlossene Rückwand, vorne aufgeschlagene Klappen, Spannseile und Heringe."""
    zufall = random.Random(seed)
    teile = []
    laenge, breite, hoehe = 2.2, 1.9, 1.55
    holz = holzfarbe(zufall, "#8A5E36", "#5E3E22")
    tuch, band, innen = farbe(stoff), farbe(streifen), farbe(stoff) * 0.55

    # Stangen
    for x in (-laenge / 2, laenge / 2):
        bm = stamm_bm(0.03, hoehe + 0.1, ecken=8, knorrig=0.02, seed=seed)
        setzen(bm, (0, 0, 0), (0, -90, 0))
        setzen(bm, (x, 0, 0))
        teile.append(objekt("Stuetze", bm, holz))
    bm = stamm_bm(0.03, laenge + 0.3, ecken=8, knorrig=0.02, seed=seed + 1)
    setzen(bm, (-laenge / 2 - 0.15, 0, hoehe))
    teile.append(objekt("First", bm, holz))

    # Plane: Gitter über beide Dachseiten, in der Mitte leicht durchhängend, mit Dicke
    teilung_x, teilung_s = 12, 8
    bm = bmesh.new()
    zeilen = []
    for j in range(teilung_s * 2 + 1):
        s = j / (teilung_s * 2) * 2 - 1  # -1 (linke Unterkante) .. 0 (First) .. 1
        zeile = []
        for i in range(teilung_x + 1):
            t = i / teilung_x * 2 - 1
            durchhang = (1 - t * t) * 0.07 * (1 - abs(s) ** 0.5) + (1 - t * t) * 0.03
            y = s * breite / 2 * 1.02
            z = hoehe * (1 - abs(s)) - durchhang * (1 if abs(s) < 0.98 else 0)
            zeile.append(bm.verts.new((t * laenge / 2 * 1.04, y, max(0.02, z))))
        zeilen.append(zeile)
    for a, b in zip(zeilen, zeilen[1:]):
        for i in range(teilung_x):
            bm.faces.new((a[i], a[i + 1], b[i + 1], b[i]))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    for f in bm.faces:
        if f.normal.z < 0:
            f.normal_flip()
    bmesh.ops.solidify(bm, geom=bm.faces[:], thickness=0.015)

    def plane(poly, fase=False):
        # Rotes Band knapp über der Unterkante, innen dunkler
        if poly.normal.z < -0.1:
            return innen
        c = band if 0.05 < poly.center.z < 0.24 else tuch
        return c * zufall.uniform(0.94, 1.04)
    teile.append(objekt("Plane", bm, plane))

    # Rückwand (Dreieck) und zwei aufgeschlagene Klappen vorne
    bm = bmesh.new()
    v = [bm.verts.new(p) for p in ((-laenge / 2 - 0.02, -breite / 2, 0.02), (-laenge / 2 - 0.02, breite / 2, 0.02), (-laenge / 2 - 0.02, 0, hoehe - 0.03))]
    bm.faces.new(v)
    bmesh.ops.solidify(bm, geom=bm.faces[:], thickness=0.015)
    teile.append(objekt("Rueckwand", bm, lambda poly, fase=False: tuch * 0.92))
    for seite in (-1, 1):
        bm = bmesh.new()
        oben = Vector((laenge / 2 + 0.02, 0, hoehe - 0.05))
        unten_innen = Vector((laenge / 2 + 0.02, seite * breite * 0.5, 0.02))
        aussen = Vector((laenge / 2 - 0.35, seite * (breite / 2 + 0.25), 0.05))
        v = [bm.verts.new(p) for p in (oben, unten_innen, aussen)]
        bm.faces.new(v if seite > 0 else list(reversed(v)))
        bmesh.ops.solidify(bm, geom=bm.faces[:], thickness=0.015)
        teile.append(objekt("Klappe", bm, lambda poly, fase=False: tuch * 0.97))

    # Spannseile und Heringe an den Ecken
    seil = einfarbig("#8C7A58", 0.05, zufall)
    for x in (-laenge / 2, laenge / 2):
        for seite in (-1, 1):
            start = Vector((x, 0, hoehe))
            ziel = Vector((x + (0.5 if x > 0 else -0.5), seite * (breite / 2 + 0.55), 0.05))
            richtung = ziel - start
            bm = stamm_bm(0.008, richtung.length, ecken=5, knorrig=0.0)
            dreh = X.rotation_difference(richtung.normalized()).to_matrix().to_4x4()
            bmesh.ops.transform(bm, matrix=Matrix.Translation(start) @ dreh, verts=bm.verts)
            teile.append(objekt("Seil", bm, seil))
            bm = brett_bm(0.03, 0.03, 0.18, fase=0.0)
            setzen(bm, (ziel.x, ziel.y, 0.05), (0, 15 if x > 0 else -15, 0))
            teile.append(objekt("Hering", bm, holz))
    # Bodenplane im Eingang
    bm = brett_bm(laenge * 0.9, breite * 0.8, 0.012, fase=0.0)
    setzen(bm, (0.1, 0, 0.008))
    teile.append(objekt("Boden", bm, einfarbig("#6F6A4C", 0.05, zufall)))
    return fertig("Zelt", teile)


# ---------------------------------------------------------------------------
# Holzstapel mit Hackklotz
# ---------------------------------------------------------------------------
def holzstapel(seed=4):
    zufall = random.Random(seed)
    teile = []
    rinde = stammfarbe(zufall)
    # Stapel: Reihen versetzt übereinander, zwischen zwei Pfosten
    reihen = [(5, 0.0), (4, 0.5), (4, 0.0), (3, 0.5)]
    r = 0.09
    for ebene, (anzahl, versatz) in enumerate(reihen):
        for i in range(anzahl):
            y = (i + versatz - (anzahl - 1) / 2 - versatz / 2) * r * 2.05
            bm = stamm_bm(r * zufall.uniform(0.9, 1.1), 1.0 + zufall.uniform(-0.08, 0.08), ecken=9, seed=seed * 10 + ebene * 7 + i)
            setzen(bm, (-0.5 + zufall.uniform(-0.04, 0.04), y, r + ebene * r * 1.75))
            teile.append(objekt("Scheit", bm, rinde))
    holz = holzfarbe(zufall, "#7A5230", "#553820")
    for y in (-0.52, 0.52):
        bm = brett_bm(0.07, 0.07, 0.8, fase=0.01)
        setzen(bm, (0, y, 0.4))
        teile.append(objekt("Pfosten", bm, holz))
    # Hackklotz mit Beil
    bm = stamm_bm(0.22, 0.42, ecken=14, seed=seed + 50)
    setzen(bm, (0, 0, 0), (0, -90, 0))
    setzen(bm, (0.95, 0.15, 0))
    teile.append(objekt("Hackklotz", bm, rinde))
    bm = brett_bm(0.5, 0.035, 0.035, fase=0.005)
    setzen(bm, (0.95 + 0.18, 0.15, 0.62), (0, -35, 0))
    teile.append(objekt("Beilstiel", bm, holzfarbe(zufall, "#A8763F", "#7E5427")))
    bm = brett_bm(0.1, 0.02, 0.14, fase=0.004)
    setzen(bm, (0.95 - 0.02, 0.15, 0.46), (0, -35, 0))
    teile.append(objekt("Beilkopf", bm, einfarbig("#7C838D", 0.04, zufall)))
    # ein paar gespaltene Scheite am Boden
    for i in range(4):
        bm = stamm_bm(0.06, 0.38, ecken=6, seed=seed + 80 + i)
        setzen(bm, (0.7 + zufall.uniform(-0.2, 0.3), -0.35 + zufall.uniform(-0.15, 0.15), 0.05), (0, 0, zufall.uniform(0, 360)))
        teile.append(objekt("Scheit", bm, rinde))
    return fertig("Holzstapel", teile)


# ---------------------------------------------------------------------------
# Karren
# ---------------------------------------------------------------------------
def _rad(zufall, radius=0.42, speichen=8):
    holz = holzfarbe(zufall, "#7E5530", "#56381E")
    eisen = einfarbig("#4A4E55", 0.04, zufall)
    teile = []
    # Felge aus Segmenten, mit Eisenreifen
    bm = bmesh.new()
    segmente, dicke, tiefe = 20, 0.05, 0.07
    ringe = []
    for k in range(segmente):
        w = math.tau * k / segmente
        aussen, innen_r = radius, radius - dicke
        ringe.append([bm.verts.new((math.cos(w) * r, y, math.sin(w) * r)) for r, y in ((innen_r, -tiefe / 2), (aussen, -tiefe / 2), (aussen, tiefe / 2), (innen_r, tiefe / 2))])
    for k in range(segmente):
        a, b = ringe[k], ringe[(k + 1) % segmente]
        for j in range(4):
            bm.faces.new((a[j], b[j], b[(j + 1) % 4], a[(j + 1) % 4]))
    teile.append(objekt("Felge", bm, lambda poly, fase=False: eisen(poly) if (poly.center.x ** 2 + poly.center.z ** 2) ** 0.5 > radius - 0.012 else holz(poly)))
    for s in range(speichen):
        w = math.degrees(math.tau * s / speichen)
        bm = brett_bm(radius - 0.05, 0.035, 0.03, fase=0.005)
        setzen(bm, ((radius - 0.05) / 2, 0, 0))
        setzen(bm, (0, 0, 0), (0, w, 0))
        teile.append(objekt("Speiche", bm, holz))
    bm = stamm_bm(0.07, 0.14, ecken=10, knorrig=0.0)
    setzen(bm, (0, 0, 0), (0, 0, 90))
    setzen(bm, (0, 0.07, 0))
    teile.append(objekt("Nabe", bm, holz))
    return teile


def karren(seed=5):
    zufall = random.Random(seed)
    teile = []
    holz = holzfarbe(zufall, "#8E6238", "#5E3F22")
    hoehe = 0.62
    # Ladefläche aus Brettern
    for i in range(6):
        bm = brett_bm(1.6, 0.16, 0.035)
        setzen(bm, (0, -0.42 + i * 0.168, hoehe))
        teile.append(objekt("Boden", bm, holzfarbe(zufall, "#946840", "#63432A")))
    # Seitenwände (zwei Bretter hoch) mit Eckpfosten
    for y in (-0.5, 0.5):
        for z in (hoehe + 0.1, hoehe + 0.26):
            bm = brett_bm(1.62, 0.03, 0.13)
            setzen(bm, (0, y, z))
            teile.append(objekt("Wand", bm, holz))
    for x in (-0.8, 0.8):
        for z in (hoehe + 0.1, hoehe + 0.26):
            bm = brett_bm(0.03, 1.0, 0.13)
            setzen(bm, (x, 0, z))
            teile.append(objekt("Stirnwand", bm, holz))
        for y in (-0.5, 0.5):
            bm = brett_bm(0.05, 0.05, 0.4)
            setzen(bm, (x, y, hoehe + 0.16))
            teile.append(objekt("Eckpfosten", bm, holz))
    # Achse, Räder, Deichsel
    bm = stamm_bm(0.035, 1.3, ecken=8, knorrig=0.0)
    setzen(bm, (0, 0, 0), (0, 0, 90))
    setzen(bm, (0, -0.65, 0.42))
    teile.append(objekt("Achse", bm, einfarbig("#4A4E55", 0.04, zufall)))
    for y in (-0.62, 0.62):
        for teil in _rad(zufall):
            teil.data.transform(Matrix.Translation((0, y, 0.42)))
            teile.append(teil)
    for y in (-0.28, 0.28):
        bm = brett_bm(1.5, 0.06, 0.06)
        setzen(bm, (1.45, y, hoehe - 0.05), (0, 18, 0))
        teile.append(objekt("Deichsel", bm, holz))
    # Ladung: Säcke, Kiste, Fass
    sack = einfarbig("#C9B48A", 0.06, zufall)
    for i, (x, y) in enumerate(((-0.45, -0.2), (-0.15, -0.25), (-0.35, 0.18))):
        bm = bmesh.new()
        bmesh.ops.create_uvsphere(bm, u_segments=12, v_segments=8, radius=0.22)
        bmesh.ops.transform(bm, matrix=Matrix.Diagonal((1.1, 0.85, 0.7, 1.0)), verts=bm.verts)
        # oben zugebunden
        for v in bm.verts:
            if v.co.z > 0.1:
                v.co.x *= 0.7
                v.co.y *= 0.7
        setzen(bm, (x, y, hoehe + 0.2), (0, zufall.uniform(-15, 15), zufall.uniform(0, 360)))
        teile.append(objekt(f"Sack{i}", bm, sack))
    teile += _kiste_teile(zufall, (0.35, -0.15, hoehe + 0.02), 0.36)
    teile += _fass_teile(zufall, (0.45, 0.25, hoehe + 0.02), 0.5)
    return fertig("Karren", teile)


def _kiste_teile(zufall, ort, groesse):
    holz = holzfarbe(zufall, "#A87A48", "#7A5430")
    eisen = einfarbig("#50555C", 0.04, zufall)
    teile = []
    s = groesse
    bm = brett_bm(s, s, s, fase=0.012)
    setzen(bm, (ort[0], ort[1], ort[2] + s / 2))
    teile.append(objekt("Kiste", bm, holz))
    # Leisten an den Kanten
    for dz in (0.08, s - 0.08):
        for dy in (-s / 2, s / 2):
            bm = brett_bm(s + 0.01, 0.015, 0.05, fase=0.003)
            setzen(bm, (ort[0], ort[1] + dy, ort[2] + dz))
            teile.append(objekt("Leiste", bm, eisen))
    return teile


def _fass_teile(zufall, ort, hoehe):
    teile = []
    holz = holzfarbe(zufall, "#8E5E32", "#613D1E", 40.0)
    eisen = einfarbig("#4B4F56", 0.04, zufall)
    r = hoehe * 0.36
    bm = bmesh.new()
    ringe = []
    for i in range(7):
        t = i / 6
        bauch = 1.0 + 0.16 * math.sin(t * math.pi)
        ringe.append([bm.verts.new((math.cos(math.tau * k / 16) * r * bauch, math.sin(math.tau * k / 16) * r * bauch, t * hoehe)) for k in range(16)])
    for a, b in zip(ringe, ringe[1:]):
        for k in range(16):
            bm.faces.new((a[k], a[(k + 1) % 16], b[(k + 1) % 16], b[k]))
    bm.faces.new(list(reversed(ringe[0])))
    bm.faces.new(ringe[-1])
    setzen(bm, ort)

    def fass(poly, fase=False):
        z = poly.center.z - ort[2]
        if any(abs(z - h) < 0.035 for h in (hoehe * 0.12, hoehe * 0.88)):
            return eisen(poly)
        return holz(poly) * (0.9 if int(math.atan2(poly.center.y - ort[1], poly.center.x - ort[0]) / math.tau * 16) % 2 else 1.0)
    teile.append(objekt("Fass", bm, fass))
    return teile


def kiste(seed=6):
    return fertig("Kiste", _kiste_teile(random.Random(seed), (0, 0, 0), 0.55))


def fass(seed=7):
    return fertig("Fass", _fass_teile(random.Random(seed), (0, 0, 0), 0.8))


# ---------------------------------------------------------------------------
# Wegweiser
# ---------------------------------------------------------------------------
def schild(seed=9):
    """Ein Pfeilbrett für den Wegweiser (Spitze zeigt nach +X, Ursprung am Pfosten).
    Im Spiel werden die Bretter einzeln so gedreht, dass sie zu ihrem Ziel zeigen."""
    zufall = random.Random(seed)
    bm = bmesh.new()
    umriss = [(0.05, -0.09), (0.62, -0.09), (0.74, 0.0), (0.62, 0.09), (0.05, 0.09)]
    oben = [bm.verts.new((x, 0.02, y + 0.09)) for x, y in umriss]
    unten = [bm.verts.new((x, -0.02, y + 0.09)) for x, y in umriss]
    bm.faces.new(oben)
    bm.faces.new(list(reversed(unten)))
    for k in range(len(umriss)):
        bm.faces.new((oben[k], unten[k], unten[(k + 1) % len(umriss)], oben[(k + 1) % len(umriss)]))
    obj = objekt("Schild", bm, holzfarbe(zufall, "#B68A55", "#8C6538", 35.0))
    obj = vereinen("Schild", [obj])
    _material(obj)
    return obj


def wegweiser(seed=8, richtungen=()):
    """Pfosten mit Kappe; Pfeilbretter (Winkel in Grad um die Hochachse) optional."""
    zufall = random.Random(seed)
    teile = []
    holz = holzfarbe(zufall, "#7A5332", "#523620")
    bm = stamm_bm(0.06, 2.1, ecken=8, knorrig=0.05, seed=seed)
    setzen(bm, (0, 0, 0), (0, -90, 0))
    teile.append(objekt("Pfosten", bm, holz))
    # Kappe
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, segments=8, radius1=0.09, radius2=0.0, depth=0.12)
    setzen(bm, (0, 0, 2.16))
    teile.append(objekt("Kappe", bm, holzfarbe(zufall, "#5E3E22", "#40291A")))
    schild = holzfarbe(zufall, "#B68A55", "#8C6538", 35.0)
    for i, winkel in enumerate(richtungen):
        z = 1.85 - i * 0.28
        bm = bmesh.new()
        # Pfeilform: Rechteck mit Spitze
        umriss = [(0.05, -0.09), (0.62, -0.09), (0.74, 0.0), (0.62, 0.09), (0.05, 0.09)]
        oben = [bm.verts.new((x, 0.02, y)) for x, y in umriss]
        unten = [bm.verts.new((x, -0.02, y)) for x, y in umriss]
        bm.faces.new(oben)
        bm.faces.new(list(reversed(unten)))
        for k in range(len(umriss)):
            bm.faces.new((oben[k], unten[k], unten[(k + 1) % len(umriss)], oben[(k + 1) % len(umriss)]))
        setzen(bm, (0, 0, z), (0, zufall.uniform(-4, 4), winkel))
        teile.append(objekt(f"Schild{i}", bm, schild))
    return fertig("Wegweiser", teile)
