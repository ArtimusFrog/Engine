"""Symbole für das Inventar: jeder Gegenstand als kleines 3D-Modell, gerendert wie ein
Fantasy-MMO-Icon (schräg von oben, warmes Hauptlicht, kühles Kantenlicht, dunkle Kontur).

Aufruf (Ergebnis: game/assets/icons/<name>.png, 256 × 256, durchsichtiger Hintergrund):
  blender --background --factory-startup --python art/icons/gegenstaende.py [-- holz stein ...]
"""

import math
import pathlib
import random
import sys

import bmesh
import bpy
import numpy as np
from mathutils import Matrix, Vector

REPO = pathlib.Path(__file__).resolve().parents[2]
ZIEL = REPO / "game" / "assets" / "icons"
GROESSE = 256
SAMPLES = 64


# ---------------------------------------------------------------------------
# Hilfen
# ---------------------------------------------------------------------------
def setze(obj, **werte):
    """Eigenschaften setzen, die es in dieser Blender-Version gibt (andere still überspringen)."""
    for name, wert in werte.items():
        if hasattr(obj, name):
            try:
                setattr(obj, name, wert)
            except (TypeError, ValueError):
                pass


def linear(hex_farbe):
    h = hex_farbe.lstrip("#")
    kanal = [int(h[i : i + 2], 16) / 255.0 for i in (0, 2, 4)]
    return tuple(c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in kanal) + (1.0,)


def leeren():
    for sammlung in (bpy.data.objects, bpy.data.meshes, bpy.data.materials, bpy.data.lights, bpy.data.cameras, bpy.data.images):
        for eintrag in list(sammlung):
            sammlung.remove(eintrag)


def objekt(name, bm, *materialien, glatt=False):
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    for mat in materialien:
        mesh.materials.append(mat)
    for polygon in mesh.polygons:
        polygon.use_smooth = glatt
    return obj


def material(name, farbe, rau=0.7, muster=None, muster_farbe=None, muster_skala=8.0, ringe=False, glanz=0.0):
    """Principled-Material, optional mit Rauschmuster (zweite Farbe) oder Jahresringen."""
    mat = bpy.data.materials.new(name)
    setze(mat, use_nodes=True)
    knoten, links = mat.node_tree.nodes, mat.node_tree.links
    bsdf = next(n for n in knoten if n.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Roughness"].default_value = rau
    if "Coat Weight" in bsdf.inputs:
        bsdf.inputs["Coat Weight"].default_value = glanz
    if muster is None and not ringe:
        bsdf.inputs["Base Color"].default_value = linear(farbe)
        return mat
    koordinaten = knoten.new("ShaderNodeTexCoord")
    if ringe:
        textur = knoten.new("ShaderNodeTexWave")
        textur.wave_type = "RINGS"
        setze(textur, rings_direction="Z")
        textur.inputs["Scale"].default_value = muster_skala
        textur.inputs["Distortion"].default_value = 3.0
        links.new(koordinaten.outputs["Object"], textur.inputs["Vector"])
        faktor = textur.outputs["Fac"]
    else:
        textur = knoten.new("ShaderNodeTexNoise")
        textur.inputs["Scale"].default_value = muster_skala
        textur.inputs["Detail"].default_value = 6.0
        links.new(koordinaten.outputs["Object"], textur.inputs["Vector"])
        faktor = textur.outputs["Fac"]
    rampe = knoten.new("ShaderNodeValToRGB")
    rampe.color_ramp.elements[0].position = 0.35 if not ringe else 0.2
    rampe.color_ramp.elements[1].position = 0.65 if not ringe else 0.8
    rampe.color_ramp.elements[0].color = linear(farbe)
    rampe.color_ramp.elements[1].color = linear(muster_farbe or farbe)
    links.new(faktor, rampe.inputs["Fac"])
    links.new(rampe.outputs["Color"], bsdf.inputs["Base Color"])
    return mat


def verbeulen(bm, staerke, seed, frequenz=None):
    zufall = random.Random(seed)
    for v in bm.verts:
        if frequenz:
            welle = math.sin(v.co.x * frequenz + seed) * math.cos(v.co.y * frequenz * 1.3) * math.sin(v.co.z * frequenz * 0.7 + 1)
            v.co += v.normal * welle * staerke
        else:
            v.co += Vector((zufall.uniform(-1, 1), zufall.uniform(-1, 1), zufall.uniform(-1, 1))) * staerke


def zylinder(radius, laenge, ecken, radius2=None):
    bm = bmesh.new()
    bmesh.ops.create_cone(bm, cap_ends=True, cap_tris=False, segments=ecken, radius1=radius,
                          radius2=radius if radius2 is None else radius2, depth=laenge)
    return bm


def bewegen(bm, ort=(0, 0, 0), drehung=(0, 0, 0), groesse=1.0):
    rot = (Matrix.Rotation(math.radians(drehung[2]), 4, "Z") @ Matrix.Rotation(math.radians(drehung[1]), 4, "Y")
           @ Matrix.Rotation(math.radians(drehung[0]), 4, "X"))
    skala = Matrix.Scale(groesse, 4) if isinstance(groesse, (int, float)) else Matrix.Diagonal((*groesse, 1.0))
    bmesh.ops.transform(bm, matrix=Matrix.Translation(Vector(ort)) @ rot @ skala, verts=bm.verts)
    return bm


def seil_ring(radius, dicke, ort, drehung, mat, name="Seil"):
    """Ein gedrehter Strick (Torus mit Windungen) um etwas herum."""
    bm = bmesh.new()
    ringe = []
    for i in range(48):
        w = math.tau * i / 48
        mitte = Vector((math.cos(w) * radius, math.sin(w) * radius, 0))
        aussen = Vector((math.cos(w), math.sin(w), 0))
        ring = []
        for k in range(8):
            u = math.tau * k / 8
            drall = 1 + 0.25 * math.cos(u * 2 + w * 14)
            ring.append(bm.verts.new(mitte + (aussen * math.cos(u) + Vector((0, 0, 1)) * math.sin(u)) * dicke * drall))
        ringe.append(ring)
    for i in range(48):
        a, b = ringe[i], ringe[(i + 1) % 48]
        for k in range(8):
            bm.faces.new((a[k], b[k], b[(k + 1) % 8], a[(k + 1) % 8]))
    bewegen(bm, ort, drehung)
    return objekt(name, bm, mat, glatt=True)


# ---------------------------------------------------------------------------
# Gegenstände (jeweils grob in einem Würfel von ±1 um den Ursprung)
# ---------------------------------------------------------------------------
def holz():
    rinde = material("Rinde", "#5A3A22", rau=0.9, muster=True, muster_farbe="#2E1D12", muster_skala=14.0)
    hirnholz = material("Hirnholz", "#E2B57A", rau=0.8, ringe=True, muster_farbe="#A8713D", muster_skala=9.0)
    seil = material("Seil", "#C9A66B", rau=0.9, muster=True, muster_farbe="#8C6A3A", muster_skala=30.0)
    for i, (y, z, r) in enumerate(((-0.33, 0.0, 0.34), (0.33, 0.0, 0.32), (0.0, 0.55, 0.33))):
        bm = zylinder(r, 1.9, 14)
        # Deckel bekommen das Hirnholz-Material, der Mantel die Rinde.
        for f in bm.faces:
            f.material_index = 1 if abs(f.normal.z) > 0.9 else 0
        verbeulen(bm, 0.018, seed=i + 1)
        bewegen(bm, (0, y, z), (0, 90, 8 * (i - 1)))
        objekt(f"Scheit{i}", bm, rinde, hirnholz)
    for x in (-0.55, 0.55):
        seil_ring(0.7, 0.055, (x, 0.0, 0.18), (0, 90, 0), seil)


def stein():
    fels = material("Fels", "#8B8E96", rau=0.85, muster=True, muster_farbe="#5C5F68", muster_skala=5.0)
    moos = material("Moos", "#6E8B3D", rau=0.9, muster=True, muster_farbe="#4B6628", muster_skala=12.0)
    bm = bmesh.new()
    bmesh.ops.create_icosphere(bm, subdivisions=2, radius=0.9)
    bewegen(bm, groesse=(1.15, 0.95, 0.75))
    zufall = random.Random(4)
    # Kantige Bruchflächen: Punkte auf ein paar Ebenen zurückschneiden.
    ebenen = [Vector((zufall.uniform(-1, 1), zufall.uniform(-1, 1), zufall.uniform(-0.3, 1))).normalized() for _ in range(9)]
    for v in bm.verts:
        for n in ebenen:
            abstand = v.co.dot(n) - 0.62
            if abstand > 0:
                v.co -= n * abstand
    verbeulen(bm, 0.03, seed=2)
    for f in bm.faces:
        f.material_index = 1 if f.normal.z > 0.75 and f.calc_center_median().x < 0.1 else 0
    objekt("Stein", bm, fels, moos)


def fleisch():
    roh = material("Fleisch", "#B8423A", rau=0.45, muster=True, muster_farbe="#E27A6A", muster_skala=6.0, glanz=0.3)

    knochen = material("Knochen", "#EFE6D2", rau=0.55)
    bm = bmesh.new()
    bmesh.ops.create_uvsphere(bm, u_segments=28, v_segments=18, radius=0.72)
    # Keulenform: dick am einen Ende, zum Knochen hin schlanker.
    for v in bm.verts:
        t = (v.co.x + 0.72) / 1.44
        v.co.y *= 1.05 - 0.35 * t
        v.co.z *= 1.0 - 0.3 * t
        v.co.x *= 1.25
    verbeulen(bm, 0.05, seed=3, frequenz=6.0)
    fleisch_obj = objekt("Keule", bm, roh, glatt=True)
    bm = zylinder(0.11, 0.9, 12, radius2=0.09)
    bewegen(bm, (1.25, 0, 0.05), (0, 90, 0))
    objekt("Knochen", bm, knochen, glatt=True)
    for dy in (-0.1, 0.1):
        bm = bmesh.new()
        bmesh.ops.create_uvsphere(bm, u_segments=14, v_segments=10, radius=0.14)
        bewegen(bm, (1.72, dy, 0.07))
        objekt("Knauf", bm, knochen, glatt=True)
    fleisch_obj.rotation_euler.z = 0


def fell():
    haar = material("Fellhaar", "#8A5A32", rau=0.95, muster=True, muster_farbe="#5A3620", muster_skala=18.0)
    innen = material("Fellleder", "#D9B48A", rau=0.8)
    # Umriss eines ausgebreiteten Fells (Rumpf, vier Beine, Hals, Schwanz) als Radius je Richtung
    def umriss(w):
        r = 0.78
        for mitte, breite, laenge in ((40, 0.22, 0.42), (140, 0.22, 0.42), (220, 0.2, 0.4), (320, 0.2, 0.4), (90, 0.25, 0.3), (270, 0.12, 0.45)):
            d = math.atan2(math.sin(w - math.radians(mitte)), math.cos(w - math.radians(mitte)))
            r += laenge * math.exp(-(d / breite) ** 2)
        return r + 0.03 * math.sin(w * 23)

    # Feines Gitter, nur was innerhalb des Umrisses liegt, bleibt stehen.
    bm = bmesh.new()
    bmesh.ops.create_grid(bm, x_segments=70, y_segments=70, size=1.4)
    innerhalb = lambda v: math.hypot(v.co.x / 0.85, v.co.y) <= umriss(math.atan2(v.co.y, v.co.x / 0.85))
    bmesh.ops.delete(bm, geom=[f for f in bm.faces if not all(innerhalb(v) for v in f.verts)], context="FACES")
    bmesh.ops.delete(bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS")
    # Treppenstufen am Rand glätten: Randpunkte genau auf den Umriss schieben.
    for v in bm.verts:
        if v.is_boundary:
            w = math.atan2(v.co.y, v.co.x / 0.85)
            r = umriss(w)
            v.co.x, v.co.y = math.cos(w) * r * 0.85, math.sin(w) * r
    for _ in range(3):
        for v in [v for v in bm.verts if not v.is_boundary]:
            nachbarn = [e.other_vert(v).co for e in v.link_edges]
            v.co = v.co * 0.5 + sum(nachbarn, Vector()) / len(nachbarn) * 0.5
    for v in bm.verts:
        v.co.z = 0.12 * math.sin(v.co.x * 3.1 + 0.5) * math.cos(v.co.y * 2.3) + 0.08 * math.sin(v.co.y * 5.0)
    obj = objekt("Fell", bm, haar, innen, glatt=True)
    dicke = obj.modifiers.new("Dicke", "SOLIDIFY")
    dicke.thickness = 0.07
    dicke.material_offset = 1
    dicke.material_offset_rim = 0
    obj.rotation_euler = (math.radians(42), 0, math.radians(-15))


def wolle():
    faden = material("Wolle", "#EFE8D8", rau=0.95, muster=True, muster_farbe="#CFC3A8", muster_skala=40.0)
    kern = bmesh.new()
    bmesh.ops.create_icosphere(kern, subdivisions=3, radius=0.72)
    objekt("Knaeuelkern", kern, faden, glatt=True)
    zufall = random.Random(9)
    for i in range(16):
        drehung = (zufall.uniform(0, 180), zufall.uniform(0, 180), zufall.uniform(0, 180))
        seil_ring(0.74 + 0.012 * (i % 3), 0.055, (0, 0, 0), drehung, faden, name=f"Faden{i}")
    # Loses Fadenende
    punkte = [Vector((0.55, -0.45, -0.25)), Vector((0.95, -0.7, -0.5)), Vector((1.3, -0.6, -0.72)), Vector((1.55, -0.85, -0.8))]
    bm = bmesh.new()
    ringe = []
    for i in range(24):
        t = i / 23
        s = t * (len(punkte) - 1)
        k = min(int(s), len(punkte) - 2)
        mitte = punkte[k].lerp(punkte[k + 1], s - k)
        ringe.append([bm.verts.new(mitte + Vector((0, math.cos(u), math.sin(u))) * 0.05) for u in (math.tau * j / 8 for j in range(8))])
    for a, b in zip(ringe, ringe[1:]):
        for j in range(8):
            bm.faces.new((a[j], b[j], b[(j + 1) % 8], a[(j + 1) % 8]))
    objekt("Fadenende", bm, faden, glatt=True)


def erz():
    """Eisenerz wie die Erzvorkommen: kantig gebrochener, dunkler Basaltbrocken, durchzogen von
    rostroten Erzadern mit blanken Eisenstellen."""
    fels = material("Basalt", "#3E3D46", rau=0.8, muster=True, muster_farbe="#26252C", muster_skala=6.0)
    rost = material("Erzrost", "#C06A30", rau=0.6, muster=True, muster_farbe="#8A4520", muster_skala=9.0)
    rost_rand = material("Erzrand", "#5A2E18", rau=0.75)
    glanz = bpy.data.materials.new("Erzglanz")
    setze(glanz, use_nodes=True)
    bsdf = next(n for n in glanz.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
    bsdf.inputs["Base Color"].default_value = linear("#C9D1DC")
    bsdf.inputs["Metallic"].default_value = 1.0
    bsdf.inputs["Roughness"].default_value = 0.3

    zufall = random.Random(21)
    bm = bmesh.new()
    bmesh.ops.create_icosphere(bm, subdivisions=3, radius=0.8)
    bewegen(bm, groesse=(1.15, 0.95, 0.85))
    # Große, ebene Bruchflächen (wie bei den Vorkommen)
    for _ in range(11):
        n = Vector((zufall.uniform(-1, 1), zufall.uniform(-1, 1), zufall.uniform(-0.5, 1))).normalized()
        bmesh.ops.bisect_plane(bm, geom=bm.verts[:] + bm.edges[:] + bm.faces[:], plane_co=n * zufall.uniform(0.5, 0.7),
                               plane_no=n, clear_outer=True)
        rand = [e for e in bm.edges if e.is_boundary]
        if rand:
            bmesh.ops.holes_fill(bm, edges=rand, sides=0)
    # Flächen fein unterteilen, damit die Adern als schmale Bänder erscheinen
    bmesh.ops.triangulate(bm, faces=bm.faces[:], quad_method="BEAUTY", ngon_method="BEAUTY")
    for _ in range(3):
        lang = [e for e in bm.edges if e.calc_length() > 0.12]
        if not lang:
            break
        bmesh.ops.subdivide_edges(bm, edges=lang, cuts=1, use_grid_fill=True)
        bmesh.ops.triangulate(bm, faces=bm.faces[:], quad_method="BEAUTY", ngon_method="BEAUTY")
    bm.normal_update()
    # Drei Adern quer durch den Brocken; im Kern hier und da blankes Eisen
    adern = [(Vector((zufall.uniform(-1, 1), zufall.uniform(-1, 1), zufall.uniform(-0.5, 0.5))).normalized(), zufall.uniform(-0.2, 0.25))
             for _ in range(3)]
    for f in bm.faces:
        mitte = f.calc_center_median()
        f.material_index = 0
        for normale, abstand in adern:
            d = abs(mitte.dot(normale) - abstand)
            if d < 0.045:
                f.material_index = 3 if zufall.random() < 0.22 else 1
                break
            if d < 0.09:
                f.material_index = 2
    brocken = objekt("Erzbrocken", bm, fels, rost, rost_rand, glanz)
    brocken.rotation_euler.z = 0.4


def _aus_magier(knoten, drehung, nur_oben=None, figur="magier"):
    """Holt ein Anbauteil (Stab, Spitzhacke, Hammer) aus dem fertigen Figurenmodell – so sieht das
    Symbol genau aus wie im Spiel. `drehung` legt es schräg ins Bild."""
    bpy.ops.import_scene.gltf(filepath=str(REPO / "game" / "assets" / "figuren" / f"{figur}.gltf"))
    for obj in bpy.context.scene.objects:
        if obj.type == "ARMATURE":
            obj.data.pose_position = "REST"
    bpy.context.view_layer.update()
    ziel = bpy.data.objects[knoten]
    welt = ziel.matrix_world.copy()
    daten = ziel.data.copy()
    daten.transform(welt)
    if nur_oben:
        # Nur das obere Stück zeigen (beim langen Stab: Kristall und Krallen)
        bm = bmesh.new()
        bm.from_mesh(daten)
        oben = max(v.co.z for v in bm.verts)
        bmesh.ops.delete(bm, geom=[v for v in bm.verts if v.co.z < oben - nur_oben], context="VERTS")
        bm.to_mesh(daten)
        bm.free()
    mitte = sum((Vector(v.co) for v in daten.vertices), Vector()) / len(daten.vertices)
    daten.transform(Matrix.Translation(-mitte))
    daten.transform(drehung)
    for obj in list(bpy.context.scene.objects):
        bpy.data.objects.remove(obj)
    teil = bpy.data.objects.new(knoten, daten)
    bpy.context.scene.collection.objects.link(teil)
    # Vertexfarben als Grundfarbe
    mat = bpy.data.materials.new("Vertexfarben")
    setze(mat, use_nodes=True)
    knoten_baum = mat.node_tree.nodes
    bsdf = next(n for n in knoten_baum if n.type == "BSDF_PRINCIPLED")
    attribut = knoten_baum.new("ShaderNodeAttribute")
    attribut.attribute_name = daten.color_attributes[0].name if daten.color_attributes else "Col"
    mat.node_tree.links.new(attribut.outputs["Color"], bsdf.inputs["Base Color"])
    bsdf.inputs["Roughness"].default_value = 0.6
    daten.materials.clear()
    daten.materials.append(mat)


def spitzhacke():
    # Stiel (im Modell nach unten hängend) schräg von links unten nach rechts oben, Spitze nach rechts unten
    s = 0.7071
    drehung = Matrix(((0, -s, -s), (-1, 0, 0), (0, s, -s))).to_4x4()
    _aus_magier("Spitzhacke", drehung)


def axt():
    # Stiel schräg von links unten nach rechts oben, Schneide zeigt nach links oben
    s = 0.7071
    drehung = Matrix(((-s, 0, -s), (0, 1, 0), (s, 0, -s))).to_4x4()
    _aus_magier("Axt", drehung)


def zauberstab():
    _aus_magier("Stab", Matrix.Rotation(math.radians(45), 4, "Y"), nur_oben=0.75)


def gold():
    """Zwei Stapel Goldmünzen und eine angelehnte Münze."""
    metall = material("Gold", "#E8B53A", rau=0.25, muster=True, muster_farbe="#FFE08A", muster_skala=14.0, glanz=0.6)
    rand = material("Goldrand", "#B8841E", rau=0.3)
    zufall = random.Random(9)
    for sx, anzahl in ((-0.42, 5), (0.38, 3)):
        for i in range(anzahl):
            bm = zylinder(0.42, 0.11, 24)
            for f in bm.faces:
                f.material_index = 0 if abs(f.normal.z) > 0.5 else 1
            bewegen(bm, (sx + zufall.uniform(-0.04, 0.04), zufall.uniform(-0.04, 0.04), -0.55 + i * 0.12), (0, 0, zufall.uniform(0, 30)))
            objekt("Muenze", bm, metall, rand)
    bm = zylinder(0.42, 0.11, 24)
    for f in bm.faces:
        f.material_index = 0 if abs(f.normal.z) > 0.5 else 1
    bewegen(bm, (0.2, -0.45, -0.18), (70, 0, 20))
    objekt("Muenze", bm, metall, rand)


def leucht(name, farbe, staerke=4.0):
    """Leuchtendes Material (Magie, Glut, Funken)."""
    mat = bpy.data.materials.new(name)
    setze(mat, use_nodes=True)
    knoten, links = mat.node_tree.nodes, mat.node_tree.links
    for k in list(knoten):
        knoten.remove(k)
    ausgabe = knoten.new("ShaderNodeOutputMaterial")
    emission = knoten.new("ShaderNodeEmission")
    emission.inputs["Color"].default_value = linear(farbe)
    emission.inputs["Strength"].default_value = staerke
    links.new(emission.outputs["Emission"], ausgabe.inputs["Surface"])
    return mat


def glimmen(name, farbe, staerke=0.6):
    """Schattiertes Material, das zusätzlich leicht von innen leuchtet (Zauberkugeln)."""
    mat = material(name, farbe, rau=0.35, glanz=0.8)
    bsdf = next(n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED")
    if "Emission Color" in bsdf.inputs:
        bsdf.inputs["Emission Color"].default_value = linear(farbe)
        bsdf.inputs["Emission Strength"].default_value = staerke
    return mat


def kugel(radius, ort, mat, name="Kugel", unterteilung=3, groesse=(1, 1, 1)):
    bm = bmesh.new()
    bmesh.ops.create_icosphere(bm, subdivisions=unterteilung, radius=radius)
    bewegen(bm, ort, (0, 0, 0), groesse)
    return objekt(name, bm, mat, glatt=True)


def kegel(radius, laenge, ort, drehung, mat, ecken=6, name="Kegel"):
    bm = zylinder(radius, laenge, ecken, 0.0)
    bewegen(bm, (0, 0, laenge / 2))
    bewegen(bm, ort, drehung)
    return objekt(name, bm, mat)


def brocken(radius, ort, mat, seed, streckung=(1, 1, 1), name="Brocken"):
    """Kantiger Stein: Ikosaeder, ein paar Schnitte, leicht verbeult."""
    zufall = random.Random(seed)
    bm = bmesh.new()
    bmesh.ops.create_icosphere(bm, subdivisions=1, radius=radius)
    for _ in range(4):
        n = Vector((zufall.uniform(-1, 1), zufall.uniform(-1, 1), zufall.uniform(-1, 1))).normalized()
        bmesh.ops.bisect_plane(bm, geom=bm.verts[:] + bm.edges[:] + bm.faces[:], plane_co=n * radius * zufall.uniform(0.55, 0.8), plane_no=n, clear_outer=True)
        rand = [e for e in bm.edges if e.is_boundary]
        if rand:
            bmesh.ops.holes_fill(bm, edges=rand, sides=0)
    bewegen(bm, ort, (zufall.uniform(0, 90), zufall.uniform(0, 90), zufall.uniform(0, 90)), streckung)
    return objekt(name, bm, mat)


def runenfragment():
    """Drei kantige Splitter eines Runensteins, in einem leuchtet eine Rune."""
    stein = material("Runenstein", "#5F6C94", rau=0.55, muster=True, muster_farbe="#8290B8", muster_skala=10.0)
    rune = leucht("Rune", "#6FC8FF", 2.2)
    for i, (ort, r, streckung) in enumerate((((0.0, 0.0, 0.0), 0.62, (0.8, 0.55, 1.25)), ((-0.62, 0.2, -0.35), 0.36, (1.0, 0.8, 0.9)),
                                             ((0.6, 0.1, -0.42), 0.3, (0.9, 1.0, 0.8)))):
        brocken(r, ort, stein, 11 + i, streckung, "Splitter")
    for dz, breite, drehung in ((0.2, 0.06, 0), (-0.05, 0.28, 30)):
        bm = zylinder(0.1, 1.0, 4)
        bewegen(bm, (0, -0.36, dz), (90, drehung, 45), (breite, 1.0, 0.06 if breite > 0.1 else 0.45))
        objekt("Rune", bm, rune)


def runenstein():
    """Sechskantiger Runenstein mit Goldband und leuchtenden Runen (wie das Modell im Spiel)."""
    stein = material("Runenstein", "#5F6C94", rau=0.5, muster=True, muster_farbe="#7F8DB5", muster_skala=8.0)
    metall = material("Gold", "#E8B53A", rau=0.25, glanz=0.6)
    rune = leucht("Rune", "#6FC8FF", 2.2)
    bm = zylinder(0.55, 1.4, 6, 0.42)
    bewegen(bm, (0, 0, 0))
    objekt("Stein", bm, stein)
    for z, r in ((0.7, 0.44), (-0.7, 0.55)):
        kegel(r, 0.35 if z > 0 else 0.25, (0, 0, z), (0, 0, 0) if z > 0 else (180, 0, 0), stein, 6, "Spitze")
    bm = zylinder(0.53, 0.09, 6)
    bewegen(bm, (0, 0, 0.1))
    objekt("Band", bm, metall)
    for k in range(3):
        w = math.tau * (k + 0.5) / 6 - math.pi / 2
        bm = zylinder(0.1, 1.0, 4)
        bewegen(bm, (math.cos(w) * 0.47, math.sin(w) * 0.47, -0.3), (90, 0, math.degrees(w) + 90), (0.35, 0.06, 0.35))
        objekt("Rune", bm, rune)
        bm = zylinder(0.1, 1.0, 4)
        bewegen(bm, (math.cos(w) * 0.47, math.sin(w) * 0.47, -0.25), (90, 45, math.degrees(w) + 90), (0.05, 0.06, 0.25))
        objekt("Rune", bm, rune)
    for i, w in enumerate((0.6, 2.4, 4.4)):
        kugel(0.08, (math.cos(w) * 0.85, math.sin(w) * 0.85, -0.2 + i * 0.35), rune, "Funke", 1, (0.8, 0.8, 1.6))


def faehigkeit_arkangeschoss():
    """Leuchtende Kugel reiner Magie mit einer Spur kleinerer Kugeln und Funken."""
    kern = glimmen("Arkan", "#6A5CFF", 0.7)
    hell = leucht("ArkanHell", "#C8C0FF", 2.2)
    kugel(0.42, (0.3, 0, 0.3), kern)
    kugel(0.22, (0.3, -0.2, 0.3), hell)
    for i, (x, z, r) in enumerate(((-0.25, -0.2, 0.2), (-0.62, -0.5, 0.13), (-0.88, -0.74, 0.08))):
        kugel(r, (x, 0.05 * i, z), kern)
    zufall = random.Random(5)
    for _ in range(9):
        kugel(0.035, (zufall.uniform(-0.8, 0.8), zufall.uniform(-0.2, 0.2), zufall.uniform(-0.8, 0.8)), hell, "Funke", 1)


def faehigkeit_feuerball():
    """Glühender Feuerball mit Flammenzungen nach hinten."""
    glut = glimmen("Glut", "#FF7A1E", 0.8)
    kern = leucht("Kern", "#FFD060", 2.2)
    flamme = leucht("Flamme", "#E8401A", 1.6)
    kugel(0.48, (0.3, 0, 0.3), glut)
    kugel(0.3, (0.35, -0.15, 0.35), kern)
    zufall = random.Random(8)
    for i in range(9):
        w = math.radians(200 + i * 10 + zufall.uniform(-6, 6))
        laenge = zufall.uniform(0.7, 1.15)
        kegel(0.2 - 0.01 * i, laenge, (0.3 + math.cos(w) * 0.25, 0.1, 0.3 + math.sin(w) * 0.25), (0, math.degrees(-w) - 90, 0), flamme, 6, "Zunge")


def faehigkeit_frostnova():
    """Ein Ring aus Eisstacheln, die aus der Mitte nach außen brechen, darüber ein Eiskristall."""
    eis = material("Eis", "#6FB8E8", rau=0.2, glanz=1.0)
    frost = leucht("Frost", "#4FA8E0", 1.3)
    for i in range(12):
        w = math.tau * i / 12
        laenge = 0.55 if i % 2 else 0.85
        kegel(0.13, laenge, (math.cos(w) * 0.45, math.sin(w) * 0.45, -0.3), (0, 70, math.degrees(w)), eis, 5, "Stachel")
    bm = zylinder(0.95, 0.06, 32)
    bewegen(bm, (0, 0, -0.4))
    objekt("Frostboden", bm, frost)
    for k in range(3):
        bm = zylinder(0.06, 1.1, 4)
        bewegen(bm, (0, 0, 0.35), (90, 60 * k, 0))
        objekt("Kristall", bm, eis)
    kugel(0.14, (0, 0, 0.35), frost)


def faehigkeit_hammerschlag():
    """Der Kriegshammer des Zwergs im Schlag, mit Funken am Kopf."""
    s = 0.7071
    _aus_magier("Hammer", Matrix(((-s, 0, -s), (0, 1, 0), (s, 0, -s))).to_4x4(), figur="zwerg")
    funke = leucht("Funke", "#FFC040", 2.0)
    # Funken sprühen vom Hammerkopf (das äußerste Ende in Schlagrichtung) nach außen
    hammer = bpy.data.objects["Hammer"]
    achse = Vector((s, 0, s))
    kopf = max((Vector(v.co) for v in hammer.data.vertices), key=lambda p: p.dot(achse))
    zufall = random.Random(3)
    for i in range(9):
        w = math.radians(-35 + i * 14)
        richtung = Vector((math.cos(w) * s - math.sin(w) * s, 0, math.cos(w) * s + math.sin(w) * s))
        kegel(0.022, zufall.uniform(0.12, 0.22), kopf + richtung * 0.05 + Vector((0, -0.12, 0)), (0, math.degrees(math.atan2(richtung.x, richtung.z)), 0), funke, 4, "Funke")


def faehigkeit_wurfhammer():
    """Ein fliegender Hammer mit Wirbelspuren."""
    _aus_magier("Hammer", Matrix.Rotation(math.radians(120), 4, "Y"), figur="zwerg")
    spur = leucht("Spur", "#BBD8FF", 1.4)
    for i, r in enumerate((0.55, 0.7)):
        bm = bmesh.new()
        ringe = []
        for k in range(13):
            w = math.radians(120 + k * 12 + i * 10)
            ringe.append([bm.verts.new(Vector((math.cos(w) * r, dy, math.sin(w) * r))) for dy in (-0.02, 0.02)])
        for a, b in zip(ringe, ringe[1:]):
            bm.faces.new((a[0], b[0], b[1], a[1]))
        bewegen(bm, (0, -0.2, 0), (0, 0, 0), (1, 1, 1))
        objekt("Spur", bm, spur)


def faehigkeit_erdbeben():
    """Aufgebrochener Boden mit glühenden Rissen und hochspringenden Felsbrocken."""
    erde = material("Erde", "#6E5238", rau=0.9, muster=True, muster_farbe="#8A6A48", muster_skala=6.0)
    fels = material("Fels", "#8C8478", rau=0.8)
    riss = leucht("Riss", "#FF7A2A", 2.0)
    zufall = random.Random(4)
    for i in range(6):
        w = math.tau * i / 6 + zufall.uniform(-0.2, 0.2)
        bm = zylinder(0.55, 0.22, 3)
        bewegen(bm, (math.cos(w) * 0.5, math.sin(w) * 0.5, -0.55 + zufall.uniform(-0.05, 0.12)), (zufall.uniform(-10, 10), zufall.uniform(-10, 10), math.degrees(w)), (1.0, 0.8, 1.0))
        objekt("Scholle", bm, erde)
    for i in range(6):
        w = math.tau * (i + 0.5) / 6
        bm = zylinder(0.03, 0.9, 4)
        bewegen(bm, (math.cos(w) * 0.45, math.sin(w) * 0.45, -0.52), (90, 0, math.degrees(w) + 90), (1, 1, 1))
        objekt("Riss", bm, riss)
    for i, (x, y, z, r) in enumerate(((0.0, 0.0, 0.15, 0.24), (-0.5, 0.1, 0.45, 0.16), (0.55, -0.1, 0.35, 0.18), (0.2, 0.2, 0.75, 0.12), (-0.25, -0.2, 0.8, 0.1))):
        brocken(r, (x, y, z), fels, 30 + i)


def _pfeil(mitte, winkel, laenge, holz, stahl, feder, dicke=0.035):
    """Pfeil in der Bildebene (X nach rechts, Z nach oben), `winkel` in Grad gegen +X, Mitte bei `mitte`."""
    a = math.radians(winkel)
    richtung = Vector((math.cos(a), 0, math.sin(a)))
    drehung = (0, 90 - winkel, 0)
    mitte = Vector(mitte)
    bm = zylinder(dicke, laenge, 8)
    bewegen(bm, mitte, drehung)
    objekt("Schaft", bm, holz)
    kegel(dicke * 2.6, laenge * 0.2, mitte + richtung * laenge * 0.48, drehung, stahl, 4, "Spitze")
    for k in range(3):
        bm = bmesh.new()
        seite = Vector((-math.sin(a), 0, math.cos(a))) * (1 if k != 1 else -1)
        tiefe = Vector((0, 0.06 if k == 2 else 0.0, 0))
        hinten = mitte - richtung * laenge * 0.5
        v = [bm.verts.new(hinten), bm.verts.new(hinten + richtung * laenge * 0.22),
             bm.verts.new(hinten + richtung * 0.02 + (seite * 0.12 if k != 2 else tiefe * 2) + tiefe)]
        bm.faces.new(v)
        objekt("Feder", bm, feder)
    return richtung


def _pfeil_mats():
    return (material("Holz", "#8A5A30", rau=0.7), material("Stahl", "#B8C0CA", rau=0.3, glanz=1.0), material("Feder", "#C8322A", rau=0.8))


def faehigkeit_pfeilschuss():
    """Ein Pfeil schräg nach oben rechts, mit hellen Bewegungsstreifen."""
    holz, stahl, feder = _pfeil_mats()
    richtung = _pfeil((0.05, 0, 0.05), 35, 1.9, holz, stahl, feder)
    streifen = leucht("Streifen", "#FFF2C8", 1.4)
    for i, (dx, dz, l) in enumerate(((-0.1, 0.18, 0.7), (-0.2, -0.16, 0.55), (-0.35, 0.02, 0.45))):
        bm = zylinder(0.015, l, 6)
        bewegen(bm, Vector((dx, 0.1, dz)) - richtung * 0.75, (0, 55, 0))
        objekt("Streifen", bm, streifen)


def faehigkeit_salve():
    """Fünf Pfeile im Fächer."""
    holz, stahl, feder = _pfeil_mats()
    for i, w in enumerate((10, 30, 50, 70, 90)):
        a = math.radians(w)
        _pfeil((-0.55 + math.cos(a) * 0.55, -0.05 * i, -0.55 + math.sin(a) * 0.55), w, 1.3, holz, stahl, feder, 0.03)


def faehigkeit_explosivpfeil():
    """Pfeil mit glühender Sprengkugel an der Spitze und Funken."""
    holz, stahl, feder = _pfeil_mats()
    richtung = _pfeil((-0.15, 0, -0.15), 40, 1.6, holz, stahl, feder)
    glut = glimmen("Glut", "#FF6A1A", 1.0)
    kern = leucht("Kern", "#FFD060", 2.5)
    spitze = Vector((-0.15, 0, -0.15)) + richtung * 0.72
    kugel(0.26, spitze, glut)
    kugel(0.14, spitze + Vector((0.04, -0.12, 0.04)), kern)
    funke = leucht("Funke", "#FFC040", 2.2)
    zufall = random.Random(7)
    for i in range(10):
        w = math.radians(zufall.uniform(0, 360))
        kegel(0.02, zufall.uniform(0.15, 0.3), spitze + Vector((math.cos(w) * 0.28, -0.1, math.sin(w) * 0.28)), (0, math.degrees(math.atan2(math.cos(w), math.sin(w))), 0), funke, 4, "Funke")


def faehigkeit_pfeilregen():
    """Viele Pfeile fallen schräg vom Himmel auf einen goldenen Zielkreis."""
    holz, stahl, feder = _pfeil_mats()
    zufall = random.Random(11)
    for i in range(9):
        x = -0.8 + 0.2 * i + zufall.uniform(-0.05, 0.05)
        z = zufall.uniform(-0.3, 0.7)
        _pfeil((x, -0.02 * i, z), -70, 0.8, holz, stahl, feder, 0.022)
    ring = leucht("Ring", "#FFD24A", 1.8)
    for k in range(28):
        w = math.tau * k / 28
        kugel(0.045, (math.cos(w) * 0.95, 0.2 + math.sin(w) * 0.95, -0.8), ring, "Ring", 1)


def faehigkeit_meteorsturm():
    """Drei glühende Meteore mit Feuerschweifen stürzen schräg herab."""
    stein = glimmen("Stein", "#8A3A1A", 0.7)
    flamme = leucht("Flamme", "#FF6A1A", 1.8)
    kern = leucht("Kern", "#FFD060", 2.2)
    for i, (x, z, r) in enumerate(((0.35, -0.3, 0.34), (-0.45, 0.25, 0.22), (0.6, 0.55, 0.16))):
        brocken(r, (x, -0.05 * i, z), stein, 70 + i)
        kugel(r * 0.55, (x + r * 0.2, -0.05 * i - r * 0.6, z - r * 0.2), kern)
        for k in range(5):
            laenge = r * (3.2 - k * 0.35)
            kegel(r * (0.8 - k * 0.1), laenge, (x + math.cos(math.radians(55 + k * 4)) * r * 0.2, -0.05 * i + 0.02 * k, z + r * 0.2), (0, -145 + k * 6, 0), flamme, 6, "Schweif")


def faehigkeit_ahnenhammer():
    """Ein riesiger goldener Geisterhammer mit Strahlenkranz."""
    s = 0.7071
    _aus_magier("Hammer", Matrix(((-s, 0, -s), (0, 1, 0), (s, 0, -s))).to_4x4() @ Matrix.Rotation(math.radians(90), 4, "Z"), figur="zwerg")
    hammer = bpy.data.objects["Hammer"]
    hammer.data.transform(Matrix.Scale(2.4, 4))
    geist = glimmen("Geist", "#FFD060", 1.2)
    hammer.data.materials.clear()
    hammer.data.materials.append(geist)
    strahl = leucht("Strahl", "#FFE9A0", 1.5)
    for k in range(12):
        w = math.tau * k / 12
        kegel(0.05, 0.5, (math.cos(w) * 0.75, 0.3, math.sin(w) * 0.75), (0, math.degrees(math.atan2(math.cos(w), math.sin(w))), 0), strahl, 4, "Strahl")


# ---------------------------------------------------------------------------
# Schurke: Klingen aus dem Figurenmodell, Symbole der Fähigkeiten
# ---------------------------------------------------------------------------
def _klinge_symbol(knoten):
    # Die Klinge hängt im Modell nach unten: schräg ins Bild legen, die Spitze nach rechts oben
    return lambda: _aus_magier(knoten, Matrix.Rotation(math.radians(-135), 4, "Y") @ Matrix.Rotation(math.radians(90), 4, "Z"), figur="schurke")


def _bogen_linie_leucht(name, farbe, mitte, radius, von, bis, dicke, staerke=2.0):
    """Leuchtende Sichel in der Bildebene (Winkel in Grad)."""
    mat = leucht(name, farbe, staerke)
    for k in range(18):
        t = k / 17
        w = math.radians(von + (bis - von) * t)
        p = Vector(mitte) + Vector((math.cos(w) * radius, -0.2, math.sin(w) * radius))
        kugel(dicke * (0.3 + math.sin(t * math.pi)), p, mat, name, 2)


def faehigkeit_klingenhieb():
    """Die Runenklinge im Schwung, dahinter eine eisblaue Sichel."""
    _aus_magier("Klinge", Matrix.Rotation(math.radians(-135), 4, "Y") @ Matrix.Rotation(math.radians(90), 4, "Z"), figur="schurke")
    bpy.data.objects["Klinge"].data.transform(Matrix.Scale(1.8, 4))
    _bogen_linie_leucht("Sichel", "#8FE0FF", (0.0, 0, -0.1), 0.8, 165, 15, 0.07, 2.2)


def _dolch(mitte, winkel, laenge, stahl, griff):
    a = math.radians(winkel)
    richtung = Vector((math.cos(a), 0, math.sin(a)))
    drehung = (0, 90 - winkel, 0)
    mitte = Vector(mitte)
    klinge = kegel(0.07, laenge * 0.62, mitte, drehung, stahl, 4, "Klinge")
    klinge.data.transform(Matrix.Translation(-mitte) @ Matrix.Scale(1, 4) )
    klinge.data.transform(Matrix.Translation(mitte))
    bm = zylinder(0.03, laenge * 0.3, 8)
    bewegen(bm, mitte - richtung * laenge * 0.15, drehung)
    objekt("Griff", bm, griff)
    bm = zylinder(0.012, 0.22, 6)
    bewegen(bm, mitte, (0, -winkel, 0))
    objekt("Parier", bm, griff)
    return richtung


def faehigkeit_wurfdolche():
    """Drei Dolche im Fächer mit grünen Giftspuren."""
    stahl = material("Stahl", "#C8CED6", rau=0.25, glanz=1.0)
    griff = material("Griff", "#4A3222", rau=0.8)
    spur = leucht("Gift", "#6CFF4A", 1.8)
    for i, w in enumerate((20, 40, 60)):
        a = math.radians(w)
        mitte = (-0.35 + math.cos(a) * 0.5, -0.05 * i, -0.35 + math.sin(a) * 0.5)
        richtung = _dolch(mitte, w, 0.9, stahl, griff)
        for k in range(5):
            p = Vector(mitte) - richtung * (0.45 + 0.12 * k)
            kugel(0.05 - 0.008 * k, p, spur, "Spur", 1)


def faehigkeit_rauchbombe():
    """Eine schwarze Bombe mit glimmender Lunte, dahinter Rauchwolken."""
    bombe = material("Bombe", "#2A2830", rau=0.4, glanz=0.4)
    rauch = material("Rauch", "#9A94A8", rau=1.0)
    kugel(0.42, (0.1, 0, -0.15), bombe, "Bombe")
    bm = zylinder(0.12, 0.12, 12)
    bewegen(bm, (0.32, 0, 0.18), (0, 35, 0))
    objekt("Hals", bm, material("Messing", "#B08A3A", rau=0.4, glanz=0.8))
    funke = leucht("Funke", "#FFB040", 2.5)
    kugel(0.08, (0.45, -0.05, 0.36), funke, "Funke")
    zufall = random.Random(5)
    for i in range(7):
        w = math.radians(zufall.uniform(0, 360))
        kegel(0.018, zufall.uniform(0.1, 0.2), (0.45 + math.cos(w) * 0.08, -0.06, 0.36 + math.sin(w) * 0.08),
              (0, math.degrees(math.atan2(math.cos(w), math.sin(w))), 0), funke, 4, "Funke")
    for i, (x, z, r) in enumerate(((-0.55, 0.3, 0.3), (-0.3, 0.6, 0.24), (-0.7, -0.1, 0.26), (0.05, 0.62, 0.2), (-0.5, 0.65, 0.18))):
        kugel(r, (x, 0.25 + 0.03 * i, z), rauch, "Rauch")


def faehigkeit_schattenklingen():
    """Ein violetter Kranz aus Schattenklingen um ein leuchtendes Zentrum."""
    klinge = leucht("Klinge", "#B488FF", 1.8)
    kern = leucht("Kern", "#F0E0FF", 3.0)
    ring = leucht("Ring", "#8A4AFF", 1.4)
    for k in range(6):
        w = math.tau * k / 6
        mitte = (math.cos(w) * 0.55, 0, math.sin(w) * 0.55)
        # Klinge tangential, leicht nach außen gebogen
        kegel(0.09, 0.62, mitte, (0, math.degrees(-w), 0), klinge, 4, "Klinge")
    kugel(0.2, (0, -0.1, 0), kern, "Kern")
    for k in range(32):
        w = math.tau * k / 32
        kugel(0.03, (math.cos(w) * 0.95, 0.1, math.sin(w) * 0.95), ring, "Ring", 1)


def _aus_gegenstand(datei, drehung=None):
    """Symbol aus einem Bodenmodell (game/assets/gegenstaende/<datei>.gltf), z. B. Rüstungsteile."""
    bpy.ops.import_scene.gltf(filepath=str(REPO / "game" / "assets" / "gegenstaende" / f"{datei}.gltf"))
    teile = [o for o in bpy.context.scene.objects if o.type == "MESH"]
    daten = teile[0].data.copy()
    daten.transform(teile[0].matrix_world)
    mitte = sum((Vector(v.co) for v in daten.vertices), Vector()) / len(daten.vertices)
    daten.transform(Matrix.Translation(-mitte))
    if drehung is not None:
        daten.transform(drehung)
    for obj in list(bpy.context.scene.objects):
        bpy.data.objects.remove(obj)
    teil = bpy.data.objects.new(datei, daten)
    bpy.context.scene.collection.objects.link(teil)
    mat = bpy.data.materials.new("Vertexfarben")
    setze(mat, use_nodes=True)
    knoten_baum = mat.node_tree.nodes
    bsdf = next(n for n in knoten_baum if n.type == "BSDF_PRINCIPLED")
    attribut = knoten_baum.new("ShaderNodeAttribute")
    attribut.attribute_name = daten.color_attributes[0].name if daten.color_attributes else "Col"
    mat.node_tree.links.new(attribut.outputs["Color"], bsdf.inputs["Base Color"])
    bsdf.inputs["Roughness"].default_value = 0.6
    daten.materials.clear()
    daten.materials.append(mat)


RUESTUNG = ["hut_lehrling", "hut_sterne", "robe_adept", "robe_erzmagier", "schuhe_wander", "schuhe_mondschritt", "helm_eisen", "helm_runen",
            "brust_kette", "brust_ahnen", "stiefel_gruben", "stiefel_eisen", "kappe_jaeger", "krone_mond", "wams_leder", "harnisch_nachtwind",
            "stiefel_pirsch", "stiefel_elfen", "kapuze", "maske_schatten", "weste_leder", "mantel_daemmerung", "stiefel_leise", "stiefel_schatten"]


def _bogen_symbol(knoten):
    # Der Bogen liegt im Modell waagerecht (entlang Y): aufrichten und schräg ins Bild legen
    return lambda: _aus_magier(knoten, Matrix.Rotation(math.radians(-40), 4, "Y") @ Matrix.Rotation(math.radians(90), 4, "X"), figur="bogenschuetze")


def lehm():
    """Zwei gestochene Lehmblöcke (hell, weich, feucht glänzend) und ein Klumpen mit Fingerspuren."""
    ocker = material("Lehm", "#E0B878", rau=0.45, muster=True, muster_farbe="#C8985E", muster_skala=7.0, glanz=0.15)
    ton = material("Ton", "#A8B6BC", rau=0.5, muster=True, muster_farbe="#8C9AA2", muster_skala=9.0)
    for i, (ort, drehung, groesse) in enumerate((((-0.3, 0.15, -0.2), (0, 0, 12), (1.0, 0.7, 0.55)), ((0.35, -0.2, -0.28), (0, 0, -20), (0.8, 0.6, 0.45)))):
        bm = bmesh.new()
        bmesh.ops.create_cube(bm, size=1.0)
        bmesh.ops.bevel(bm, geom=bm.edges[:], offset=0.12, segments=3, affect="EDGES")
        for f in bm.faces:
            f.material_index = 1 if f.calc_center_median().z < -0.3 else 0
        verbeulen(bm, 0.03, seed=5 + i)
        bewegen(bm, ort, drehung, groesse)
        objekt(f"Lehmblock{i}", bm, ocker, ton, glatt=True)
    kugel(0.34, (0.05, -0.05, 0.28), ocker, name="Klumpen", groesse=(1.1, 1.0, 0.8))


def kristall():
    """Drei blau leuchtende Kristalle wie aus den Kristallvorkommen."""
    blau = glimmen("Kristall", "#3C9CFF", 0.9)
    hell = glimmen("Kristallhell", "#BFE8FF", 1.1)
    for i, (ort, drehung, r, laenge, mat) in enumerate((((0.0, 0.0, -0.75), (0, 0, 0), 0.3, 1.5, blau), ((-0.35, 0.1, -0.7), (0, -28, 10), 0.2, 1.0, hell),
                                                          ((0.35, -0.05, -0.72), (0, 30, -15), 0.18, 0.9, blau))):
        bm = zylinder(r, laenge * 0.75, 6)
        bewegen(bm, (0, 0, laenge * 0.375))
        spitze = zylinder(r, laenge * 0.25, 6, 0.0)
        bewegen(spitze, (0, 0, laenge * 0.75 + laenge * 0.125))
        bm2 = bmesh.new()
        for teil in (bm, spitze):
            mesh = bpy.data.meshes.new("tmp")
            teil.to_mesh(mesh)
            bm2.from_mesh(mesh)
            bpy.data.meshes.remove(mesh)
        bewegen(bm2, ort, drehung)
        objekt(f"Kristall{i}", bm2, mat)


GEGENSTAENDE = {"gold": gold, "lehm": lehm, "kristall": kristall, "holz": holz, "stein": stein, "erz": erz, "fleisch": fleisch, "fell": fell, "wolle": wolle,
                "spitzhacke": spitzhacke, "axt": axt, "zauberstab": zauberstab, "runenfragment": runenfragment, "runenstein": runenstein,
                "faehigkeit_arkangeschoss": faehigkeit_arkangeschoss, "faehigkeit_feuerball": faehigkeit_feuerball,
                "faehigkeit_frostnova": faehigkeit_frostnova, "faehigkeit_hammerschlag": faehigkeit_hammerschlag,
                "faehigkeit_wurfhammer": faehigkeit_wurfhammer, "faehigkeit_erdbeben": faehigkeit_erdbeben}
# Waffen (waffen.py) direkt aus den Figurenmodellen: Stäbe mit ihrer Krone, Hämmer schräg
def _stab_symbol(art):
    return lambda: _aus_magier(art, Matrix.Rotation(math.radians(45), 4, "Y"), nur_oben=0.8)


def _hammer_symbol(knoten):
    s = 0.7071
    return lambda: _aus_magier(knoten, Matrix(((-s, 0, -s), (0, 1, 0), (s, 0, -s))).to_4x4() @ Matrix.Rotation(math.radians(90), 4, "Z"), figur="zwerg")


for _art in ("stab_eiche", "stab_glut", "stab_frost", "stab_sturm", "stab_sternen"):
    GEGENSTAENDE[_art] = _stab_symbol(_art)
for _art in ("hammer_eisen", "hammer_runen", "hammer_streit", "hammer_donner", "hammer_drachen"):
    GEGENSTAENDE[_art] = _hammer_symbol(_art)
GEGENSTAENDE["schmiedehammer"] = _hammer_symbol("Hammer")
for _art in ("bogen_eibe", "bogen_lang", "bogen_glut", "bogen_elfen", "bogen_sturm"):
    GEGENSTAENDE[_art] = _bogen_symbol(_art)
GEGENSTAENDE["jagdbogen"] = _bogen_symbol("Bogen")
for _art in ("klinge_eisen", "klinge_gift", "klinge_russ", "klinge_mond", "klinge_schatten"):
    GEGENSTAENDE[_art] = _klinge_symbol(_art)
GEGENSTAENDE["runenklinge"] = _klinge_symbol("Klinge")
for _art in RUESTUNG:
    GEGENSTAENDE[_art] = (lambda a: lambda: _aus_gegenstand(a))(_art)
for _name, _bau in (("faehigkeit_klingenhieb", faehigkeit_klingenhieb), ("faehigkeit_wurfdolche", faehigkeit_wurfdolche),
                    ("faehigkeit_rauchbombe", faehigkeit_rauchbombe), ("faehigkeit_schattenklingen", faehigkeit_schattenklingen)):
    GEGENSTAENDE[_name] = _bau
for _name, _bau in (("faehigkeit_meteorsturm", faehigkeit_meteorsturm), ("faehigkeit_ahnenhammer", faehigkeit_ahnenhammer),
                    ("faehigkeit_pfeilschuss", faehigkeit_pfeilschuss), ("faehigkeit_salve", faehigkeit_salve),
                    ("faehigkeit_explosivpfeil", faehigkeit_explosivpfeil), ("faehigkeit_pfeilregen", faehigkeit_pfeilregen)):
    GEGENSTAENDE[_name] = _bau

# Werkzeuge von vorne ansehen (liegen flach im Bild), Gegenstände schräg von oben
BLICK = {**{n: (0.0, -1.0, 0.25) for n in GEGENSTAENDE if n.startswith(("stab_", "hammer_", "bogen_", "klinge_", "faehigkeit_")) or n in ("schmiedehammer", "jagdbogen", "runenklinge")}, "spitzhacke": (0.0, -1.0, 0.25), "axt": (0.0, -1.0, 0.25), "zauberstab": (0.0, -1.0, 0.25),
         "faehigkeit_arkangeschoss": (0.0, -1.0, 0.2), "faehigkeit_feuerball": (0.0, -1.0, 0.2), "faehigkeit_hammerschlag": (0.0, -1.0, 0.2),
         "faehigkeit_wurfhammer": (0.0, -1.0, 0.2), **{n: (0.45, -1.0, 0.5) for n in RUESTUNG}, "faehigkeit_frostnova": (0.3, -1.0, 0.9), "faehigkeit_erdbeben": (0.4, -1.0, 0.8)}


# ---------------------------------------------------------------------------
# Bühne: Kamera von schräg oben, Licht, Rendern, Kontur
# ---------------------------------------------------------------------------
def buehne(blick=(0.55, -1.0, 0.75)):
    szene = bpy.context.scene
    try:
        szene.render.engine = "CYCLES"
        setze(szene.cycles, samples=SAMPLES, use_denoising=True, device="CPU")
    except TypeError:
        szene.render.engine = "BLENDER_EEVEE"
    szene.render.resolution_x = szene.render.resolution_y = GROESSE
    szene.render.film_transparent = True
    szene.render.image_settings.file_format = "PNG"
    szene.render.image_settings.color_mode = "RGBA"
    setze(szene.view_settings, view_transform="Standard", look="None")

    welt = bpy.data.worlds.new("Welt") if not szene.world else szene.world
    szene.world = welt
    setze(welt, use_nodes=True)
    hintergrund = next((n for n in welt.node_tree.nodes if n.type == "BACKGROUND"), None)
    if hintergrund:
        hintergrund.inputs["Color"].default_value = (0.32, 0.3, 0.34, 1)
        hintergrund.inputs["Strength"].default_value = 0.6

    # Alles auf die Mitte ausrichten und so skalieren, dass es ins Bild passt.
    objekte = [o for o in szene.objects if o.type == "MESH"]
    bpy.context.view_layer.update()
    ecken = [o.matrix_world @ Vector(e) for o in objekte for e in o.bound_box]
    tief = Vector((min(e.x for e in ecken), min(e.y for e in ecken), min(e.z for e in ecken)))
    hoch = Vector((max(e.x for e in ecken), max(e.y for e in ecken), max(e.z for e in ecken)))
    mitte = (tief + hoch) / 2

    kamera_daten = bpy.data.cameras.new("Kamera")
    kamera_daten.type = "ORTHO"
    kamera = bpy.data.objects.new("Kamera", kamera_daten)
    szene.collection.objects.link(kamera)
    richtung = Vector(blick).normalized()
    kamera.location = mitte + richtung * 10
    kamera.rotation_euler = (-richtung).to_track_quat("-Z", "Y").to_euler()
    szene.camera = kamera
    bpy.context.view_layer.update()
    # Größe aus der Projektion der Eckpunkte in Kamerarichtung
    inverse = kamera.matrix_world.inverted()
    projiziert = [inverse @ e for e in ecken]
    ausdehnung = max(max(abs(p.x) for p in projiziert), max(abs(p.y) for p in projiziert))
    kamera_daten.ortho_scale = ausdehnung * 2 * 1.05

    def licht(name, art, energie, farbe, ort, groesse=2.0):
        daten = bpy.data.lights.new(name, art)
        daten.energy = energie
        daten.color = farbe
        setze(daten, size=groesse, shape="DISK")
        obj = bpy.data.objects.new(name, daten)
        szene.collection.objects.link(obj)
        obj.location = mitte + Vector(ort)
        obj.rotation_euler = (-Vector(ort)).to_track_quat("-Z", "Y").to_euler()

    licht("Hauptlicht", "AREA", 900, (1.0, 0.9, 0.78), (-3.5, -4.0, 6.0), 4.0)
    licht("Kante", "AREA", 700, (0.6, 0.75, 1.0), (4.0, 5.0, 3.0), 3.0)
    licht("Fuellung", "AREA", 250, (1.0, 0.95, 0.9), (5.0, -5.0, 1.0), 5.0)


def kontur(pfad):
    """Dunkle Kontur und weicher Schatten um das Motiv – liest sich auf jedem Hintergrund."""
    bild = bpy.data.images.load(str(pfad))
    breite, hoehe = bild.size
    pixel = np.array(bild.pixels[:], dtype=np.float32).reshape(hoehe, breite, 4)
    alpha = pixel[..., 3]

    def ausdehnen(maske, schritte):
        ergebnis = maske.copy()
        for _ in range(schritte):
            neu = ergebnis.copy()
            for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)):
                neu = np.maximum(neu, np.roll(np.roll(ergebnis, dy, 0), dx, 1) * (0.85 if dx and dy else 1.0))
            ergebnis = neu
        return ergebnis

    rand = np.clip(ausdehnen(alpha, 3), 0, 1)
    schatten = np.roll(np.roll(ausdehnen(alpha, 6), -5, 0), 4, 1) * 0.45
    unten = np.zeros_like(pixel)
    unten[..., 3] = np.maximum(schatten, rand)
    kontur_farbe = np.array([0.02, 0.015, 0.01])
    unten[..., :3] = kontur_farbe
    # Motiv (vormultipliziert) über Kontur und Schatten legen
    a = alpha[..., None]
    farbe = pixel[..., :3] * a + unten[..., :3] * unten[..., 3:4] * (1 - a)
    gesamt = a[..., 0] + unten[..., 3] * (1 - a[..., 0])
    ergebnis = np.zeros_like(pixel)
    ergebnis[..., :3] = np.where(gesamt[..., None] > 1e-4, farbe / np.maximum(gesamt[..., None], 1e-4), 0)
    ergebnis[..., 3] = gesamt
    bild.pixels[:] = ergebnis.ravel()
    bild.filepath_raw = str(pfad)
    bild.file_format = "PNG"
    bild.save()


def main():
    namen = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else list(GEGENSTAENDE)
    ZIEL.mkdir(parents=True, exist_ok=True)
    for name in namen:
        leeren()
        GEGENSTAENDE[name]()
        buehne(BLICK.get(name, (0.55, -1.0, 0.75)))
        pfad = ZIEL / f"{name}.png"
        bpy.context.scene.render.filepath = str(pfad)
        bpy.ops.render.render(write_still=True)
        kontur(pfad)
        print(f"FERTIG {name} -> {pfad.relative_to(REPO)}")


main()
