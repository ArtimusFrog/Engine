"""Werkstatt: Hilfsfunktionen, mit denen unsere Blender-Skripte Low-Poly-Modelle bauen.

Alle Skripte unter art/modelle/ benutzen diese Funktionen, damit alle Assets gleich aussehen:
dieselbe Farbpalette, flache Schattierung, Maße in Metern, Ursprung am Boden.

Koordinaten in Blender: Z zeigt nach oben, die Vorderseite eines Modells schaut nach -Y
(Blender-Ansicht „Vorne“). Der Export dreht das automatisch in die Achsen der Engine.
"""

import math
import random

import bmesh
import bpy
from mathutils import Matrix, Vector

# ---------------------------------------------------------------------------
# Farbpalette (sRGB-Hex wie im Malprogramm). Neue Farben bitte hier eintragen,
# nicht in einzelnen Skripten, damit die Welt einheitlich bleibt.
# ---------------------------------------------------------------------------
PALETTE = {
    # Pflanzen
    "laub_hell": "#8BC34A",
    "laub": "#5E9E3A",
    "laub_dunkel": "#3F7A2E",
    "nadel": "#2F6B4F",
    "gras": "#7CB342",
    "moos": "#6E8B3D",
    "herbst": "#E0A43A",
    "bluete_rot": "#E0524F",
    "bluete_gelb": "#F2D544",
    "bluete_blau": "#6A8FE0",
    # Holz
    "rinde": "#7A5534",
    "rinde_dunkel": "#5A3D26",
    "holz_hell": "#B98A56",
    "holz": "#94693F",
    # Stein und Erde
    "stein_hell": "#A9A49A",
    "stein": "#8A857C",
    "stein_dunkel": "#66625C",
    "erde": "#7B5B3E",
    "sand": "#E3CF9A",
    # Metall
    "eisen": "#8C939C",
    "eisen_dunkel": "#4B5058",
    "gold": "#E8B64A",
    # Stoff, Fell und Sonstiges
    "stoff_rot": "#B23A3A",
    "stoff_blau": "#3A5BB2",
    "stoff_beige": "#D8C7A0",
    "fell_orange": "#D9772F",
    "fell_weiss": "#F1EDE4",
    "fell_braun": "#8A5A36",
    "baer": "#6B4428",
    "baer_dunkel": "#46291A",
    "baer_schnauze": "#B68A5E",
    "schwarz": "#1E1E22",
    "kristall": "#7FE0F0",
    "magie": "#B07CFF",
}


def srgb_zu_linear(hex_farbe):
    """'#RRGGBB' → lineares RGBA, wie Blender und glTF es erwarten."""
    h = hex_farbe.lstrip("#")
    kanal = [int(h[i : i + 2], 16) / 255.0 for i in (0, 2, 4)]
    linear = [c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in kanal]
    return (*linear, 1.0)


# ---------------------------------------------------------------------------
# Szene
# ---------------------------------------------------------------------------
def neu():
    """Leert die Szene komplett (Objekte, Meshes, Materialien, Animationen)."""
    for sammlung in (bpy.data.objects, bpy.data.meshes, bpy.data.materials, bpy.data.armatures, bpy.data.actions):
        for eintrag in list(sammlung):
            sammlung.remove(eintrag)
    szene = bpy.context.scene
    szene.unit_settings.system = "METRIC"
    szene.unit_settings.scale_length = 1.0
    szene.render.fps = 30


def material(name, farbe=None, rau=0.9, metall=0.0, beidseitig=False, leuchten=0.0):
    """Einfaches Material. `farbe` = Palettenname oder '#RRGGBB' (Standard: `name`).

    Die Engine liest davon die Grundfarbe und ob es beidseitig ist. Gleichnamige
    Materialien werden wiederverwendet.
    """
    if name in bpy.data.materials:
        return bpy.data.materials[name]
    hex_farbe = PALETTE.get(farbe or name, farbe or name)
    rgba = srgb_zu_linear(hex_farbe)
    mat = bpy.data.materials.new(name)
    try:
        mat.use_nodes = True  # ab Blender 5 immer an, dort nur noch veraltet
    except (AttributeError, TypeError):
        pass
    mat.diffuse_color = rgba
    mat.use_backface_culling = not beidseitig
    bsdf = next((n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED"), None)
    if bsdf is not None:
        bsdf.inputs["Base Color"].default_value = rgba
        bsdf.inputs["Roughness"].default_value = rau
        bsdf.inputs["Metallic"].default_value = metall
        if leuchten > 0.0 and "Emission Strength" in bsdf.inputs:
            bsdf.inputs["Emission Color"].default_value = rgba
            bsdf.inputs["Emission Strength"].default_value = leuchten
    return mat


def _objekt_aus_bmesh(name, bm, mat):
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.scene.collection.objects.link(obj)
    if mat is not None:
        mesh.materials.append(mat)
    flach(obj)
    return obj


def _matrix(ort, drehung, groesse):
    rot = (
        Matrix.Rotation(math.radians(drehung[2]), 4, "Z")
        @ Matrix.Rotation(math.radians(drehung[1]), 4, "Y")
        @ Matrix.Rotation(math.radians(drehung[0]), 4, "X")
    )
    if isinstance(groesse, (int, float)):
        skala = Matrix.Scale(groesse, 4)
    else:
        skala = Matrix.Diagonal((*groesse, 1.0))
    return Matrix.Translation(Vector(ort)) @ rot @ skala


# ---------------------------------------------------------------------------
# Grundformen. `ort` = Mittelpunkt der Unterseite (Formen stehen auf dem Boden),
# `drehung` = Grad um X, Y, Z.
# ---------------------------------------------------------------------------
def kiste(name, breite, tiefe, hoehe, mat=None, ort=(0, 0, 0), drehung=(0, 0, 0)):
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.transform(bm, matrix=Matrix.Translation((0, 0, 0.5)), verts=bm.verts)
    bmesh.ops.transform(bm, matrix=_matrix(ort, drehung, (breite, tiefe, hoehe)), verts=bm.verts)
    return _objekt_aus_bmesh(name, bm, mat)


def zylinder(name, radius, hoehe, mat=None, ecken=8, radius_oben=None, ort=(0, 0, 0), drehung=(0, 0, 0)):
    """Zylinder oder Kegelstumpf (`radius_oben`), z. B. Stämme, Äste, Pfosten."""
    bm = bmesh.new()
    oben = radius if radius_oben is None else radius_oben
    bmesh.ops.create_cone(bm, cap_ends=True, cap_tris=False, segments=ecken, radius1=radius, radius2=oben, depth=hoehe)
    bmesh.ops.transform(bm, matrix=Matrix.Translation((0, 0, hoehe / 2)), verts=bm.verts)
    bmesh.ops.transform(bm, matrix=_matrix(ort, drehung, 1.0), verts=bm.verts)
    return _objekt_aus_bmesh(name, bm, mat)


def kegel(name, radius, hoehe, mat=None, ecken=8, ort=(0, 0, 0), drehung=(0, 0, 0)):
    return zylinder(name, radius, hoehe, mat, ecken, radius_oben=0.0, ort=ort, drehung=drehung)


def kugel(name, radius, mat=None, stufen=1, ort=(0, 0, 0), groesse=(1, 1, 1), drehung=(0, 0, 0)):
    """Eckige Kugel (Ikosaeder) – Baumkronen, Büsche, Felsen. Hier ist `ort` der Mittelpunkt."""
    bm = bmesh.new()
    bmesh.ops.create_icosphere(bm, subdivisions=stufen, radius=radius)
    bmesh.ops.transform(bm, matrix=_matrix(ort, drehung, groesse), verts=bm.verts)
    return _objekt_aus_bmesh(name, bm, mat)


# ---------------------------------------------------------------------------
# Bearbeiten
# ---------------------------------------------------------------------------
def flach(obj):
    """Flache Schattierung (jede Fläche eine Farbe) – unser Low-Poly-Stil."""
    for polygon in obj.data.polygons:
        polygon.use_smooth = False


def verbeulen(obj, staerke, seed=1):
    """Verschiebt jeden Eckpunkt zufällig, damit nichts zu glatt aussieht.
    Gleicher `seed` = gleiches Ergebnis."""
    zufall = random.Random(seed)
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
    for v in bm.verts:
        v.co += Vector((zufall.uniform(-1, 1), zufall.uniform(-1, 1), zufall.uniform(-1, 1))) * staerke
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.update()
    flach(obj)
    return obj


def boden_abflachen(obj, hoehe=0.0):
    """Drückt alles unter `hoehe` platt, damit das Modell sauber auf dem Boden steht."""
    inverse = obj.matrix_world.inverted()
    for v in obj.data.vertices:
        welt = obj.matrix_world @ v.co
        if welt.z < hoehe:
            welt.z = hoehe
            v.co = inverse @ welt
    obj.data.update()
    return obj


def einfaerben_nach_richtung(obj, mat_oben, grenze=0.6):
    """Flächen, die nach oben zeigen, bekommen `mat_oben` (Moos, Schnee, Gras auf Felsen)."""
    if mat_oben.name not in obj.data.materials:
        obj.data.materials.append(mat_oben)
    index = list(obj.data.materials).index(mat_oben)
    obj.data.update()
    for polygon in obj.data.polygons:
        if polygon.normal.z > grenze:
            polygon.material_index = index
    return obj


def vereinen(name, objekte):
    """Fügt mehrere Objekte zu einem Mesh zusammen (Materialien bleiben erhalten)."""
    objekte = [o for o in objekte if o is not None]
    ziel = objekte[0]
    bpy.ops.object.select_all(action="DESELECT")
    for o in objekte:
        o.select_set(True)
    bpy.context.view_layer.objects.active = ziel
    bpy.ops.object.join()
    ziel.name = name
    ziel.data.name = name
    return ziel


def ursprung_unten(obj):
    """Setzt den Ursprung mittig unter das Modell, auf den Boden."""
    ecken = [obj.matrix_world @ v.co for v in obj.data.vertices]
    unten = Vector((
        (min(e.x for e in ecken) + max(e.x for e in ecken)) / 2,
        (min(e.y for e in ecken) + max(e.y for e in ecken)) / 2,
        min(e.z for e in ecken),
    ))
    obj.data.transform(Matrix.Translation(obj.location - unten))
    obj.location = Vector((0, 0, 0))
    obj.data.update()
    return obj


# ---------------------------------------------------------------------------
# Skelett und Animation (für Tiere und Figuren)
#
# Vorgehen: Körperteile als einzelne Objekte bauen, jedes mit `knochen_zuweisen` einem
# Knochen zuordnen (starre Zuordnung – passt zum Low-Poly-Stil), mit `vereinen` zu einem
# Mesh machen, `skelett` anlegen, `binden`, dann je Animation `animation`.
# ---------------------------------------------------------------------------
def knochen_zuweisen(obj, knochen):
    """Alle Eckpunkte von `obj` bewegen sich mit dem Knochen `knochen`."""
    gruppe = obj.vertex_groups.new(name=knochen)
    gruppe.add([v.index for v in obj.data.vertices], 1.0, "REPLACE")
    return obj


def skelett(name, knochen):
    """Legt ein Skelett an. `knochen` = Liste von (Name, Kopf, Ende, Eltern, Oben).

    `Oben` ist die Weltrichtung, in die die lokale Z-Achse des Knochens zeigen soll –
    so ist klar, um welche Achse sich ein Knochen dreht:
    - Beine (zeigen nach unten), Oben = (0, 1, 0): Drehung um X schwingt sie vor und zurück.
    - Rumpf/Kopf (zeigen nach vorne, -Y), Oben = (0, 0, 1): Drehung um X nickt, um Z schaut zur Seite.
    """
    daten = bpy.data.armatures.new(name)
    obj = bpy.data.objects.new(name, daten)
    bpy.context.scene.collection.objects.link(obj)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    angelegt = {}
    for knochen_name, kopf, ende, eltern, oben in knochen:
        k = daten.edit_bones.new(knochen_name)
        k.head = Vector(kopf)
        k.tail = Vector(ende)
        k.align_roll(Vector(oben))
        if eltern:
            k.parent = angelegt[eltern]
        angelegt[knochen_name] = k
    bpy.ops.object.mode_set(mode="OBJECT")
    for pose in obj.pose.bones:
        pose.rotation_mode = "XYZ"
    return obj


def binden(mesh_obj, armatur):
    """Hängt das Mesh an das Skelett (Vertex-Gruppen = Knochennamen)."""
    mesh_obj.parent = armatur
    modifier = mesh_obj.modifiers.new("Skelett", "ARMATURE")
    modifier.object = armatur
    return mesh_obj


def animation(armatur, name, laenge, schluessel):
    """Eine Animation (glTF-Clip) als Blender-Aktion.

    `laenge` in Bildern (30 pro Sekunde). `schluessel` = Liste von
    (Bild, Knochen, "rot" oder "pos", (x, y, z)) – Drehung in Grad (lokale Achsen), Position in
    Metern. Für Schleifen muss das letzte Bild dem ersten gleichen.
    """
    armatur.animation_data_create()
    aktion = bpy.data.actions.new(name)
    aktion.use_fake_user = True
    armatur.animation_data.action = aktion
    for pose in armatur.pose.bones:
        pose.rotation_euler = (0, 0, 0)
        pose.location = (0, 0, 0)
    for bild, knochen_name, art, wert in sorted(schluessel, key=lambda s: s[0]):
        pose = armatur.pose.bones[knochen_name]
        if art == "rot":
            pose.rotation_euler = [math.radians(w) for w in wert]
            pose.keyframe_insert(data_path="rotation_euler", frame=bild)
        else:
            pose.location = wert
            pose.keyframe_insert(data_path="location", frame=bild)
    aktion.frame_range = (0, laenge)
    return aktion


def ruhepose(armatur):
    """Nach dem Animieren: Skelett in Grundstellung, keine aktive Aktion."""
    for pose in armatur.pose.bones:
        pose.rotation_euler = (0, 0, 0)
        pose.location = (0, 0, 0)
    if armatur.animation_data:
        armatur.animation_data.action = None


# ---------------------------------------------------------------------------
# Export
# ---------------------------------------------------------------------------
def exportieren(pfad):
    """Schreibt die ganze Szene als .gltf (+ .bin) für die Engine."""
    pfad.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=str(pfad),
        export_format="GLTF_SEPARATE",
        export_yup=True,
        export_apply=True,
        export_animations=True,
        export_materials="EXPORT",
    )
