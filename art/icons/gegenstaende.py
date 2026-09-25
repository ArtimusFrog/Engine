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


GEGENSTAENDE = {"holz": holz, "stein": stein, "fleisch": fleisch, "fell": fell, "wolle": wolle}


# ---------------------------------------------------------------------------
# Bühne: Kamera von schräg oben, Licht, Rendern, Kontur
# ---------------------------------------------------------------------------
def buehne():
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
    richtung = Vector((0.55, -1.0, 0.75)).normalized()
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
        buehne()
        pfad = ZIEL / f"{name}.png"
        bpy.context.scene.render.filepath = str(pfad)
        bpy.ops.render.render(write_still=True)
        kontur(pfad)
        print(f"FERTIG {name} -> {pfad.relative_to(REPO)}")


main()
