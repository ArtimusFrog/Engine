"""Realistischer Braunbär – Spielfassung des Cycles-Renders (art/renders/baer_foto.py).

Gleiche Körperform, aber spieltauglich:
  - vereinfacht auf < 3000 Dreiecke, weich schattiert
  - Fell als Textur „gebacken“ (Cycles): Fellmaserung, dunkle Beine, helle Schnauze,
    Umgebungsverdeckung in Falten und unter dem Bauch
  - Skelett mit Ellbogen/Knien, weiche automatische Gewichtung
  - Animationen Idle, Laufen, Rennen
"""

import math

import bpy
from mathutils import Vector

import baer_form
from werkstatt import animation, ruhepose, skelett

ZIEL_DREIECKE = 2400
TEXTUR_GROESSE = 1024

# ---------------------------------------------------------------------------
# Form: Körper vereinfachen, Nase und Augen dazu
# ---------------------------------------------------------------------------
baer = baer_form.koerper("BaerRealistisch", aufloesung=0.035, beulen=0.02)
dreiecke = sum(len(p.vertices) - 2 for p in baer.data.polygons)
vereinfachen = baer.modifiers.new("Vereinfachen", "DECIMATE")
vereinfachen.ratio = min(1.0, ZIEL_DREIECKE / dreiecke)
bpy.ops.object.modifier_apply(modifier=vereinfachen.name)


def material(name):
    mat = bpy.data.materials.new(name)
    try:
        mat.use_nodes = True
    except (AttributeError, TypeError):
        pass
    return mat


fell = material("Fell")
nase_mat = material("Nase")
auge_mat = material("Auge")
baer.data.materials.append(fell)


def kleine_kugel(ort, radius, mat, skalierung=(1, 1, 1)):
    bpy.ops.mesh.primitive_uv_sphere_add(radius=radius, location=ort, segments=12, ring_count=7)
    obj = bpy.context.active_object
    obj.scale = skalierung
    obj.data.materials.append(mat)
    for polygon in obj.data.polygons:
        polygon.use_smooth = True
    return obj


teile = [kleine_kugel(baer_form.NASE, 0.042, nase_mat, (1.35, 0.75, 0.85))]
teile += [kleine_kugel(ort, 0.022, auge_mat) for ort in baer_form.augen_orte(baer)]
bpy.ops.object.select_all(action="DESELECT")
for teil in teile:
    teil.select_set(True)
baer.select_set(True)
bpy.context.view_layer.objects.active = baer
bpy.ops.object.join()

# UV-Abwicklung für die Textur
bpy.ops.object.mode_set(mode="EDIT")
bpy.ops.mesh.select_all(action="SELECT")
bpy.ops.uv.smart_project(angle_limit=math.radians(60), island_margin=0.004)
bpy.ops.object.mode_set(mode="OBJECT")

# ---------------------------------------------------------------------------
# Fell-Material zum Backen (prozedural), Nase und Augen einfarbig
# ---------------------------------------------------------------------------
knoten, links = fell.node_tree.nodes, fell.node_tree.links
bsdf = knoten["Principled BSDF"]
koordinaten = knoten.new("ShaderNodeTexCoord")
dehnen = knoten.new("ShaderNodeMapping")
dehnen.inputs["Scale"].default_value = (70.0, 14.0, 70.0)  # Maserung längs des Körpers
links.new(koordinaten.outputs["Object"], dehnen.inputs["Vector"])
maserung = knoten.new("ShaderNodeTexNoise")
maserung.inputs["Detail"].default_value = 10.0
maserung.inputs["Roughness"].default_value = 0.65
links.new(dehnen.outputs["Vector"], maserung.inputs["Vector"])
farben = knoten.new("ShaderNodeValToRGB")
farben.color_ramp.elements[0].position = 0.3
farben.color_ramp.elements[0].color = (0.035, 0.02, 0.01, 1)
farben.color_ramp.elements[1].position = 0.75
farben.color_ramp.elements[1].color = (0.2, 0.115, 0.055, 1)
links.new(maserung.outputs["Fac"], farben.inputs["Fac"])

# Beine nach unten dunkler
xyz = knoten.new("ShaderNodeSeparateXYZ")
links.new(koordinaten.outputs["Object"], xyz.inputs["Vector"])
beine = knoten.new("ShaderNodeMapRange")
beine.inputs["From Min"].default_value = 0.15
beine.inputs["From Max"].default_value = 0.65
beine.inputs["To Min"].default_value = 0.75
beine.inputs["To Max"].default_value = 0.0
links.new(xyz.outputs["Z"], beine.inputs["Value"])
dunkel = knoten.new("ShaderNodeMix")
dunkel.data_type = "RGBA"
dunkel.inputs["B"].default_value = (0.02, 0.012, 0.007, 1)
links.new(beine.outputs["Result"], dunkel.inputs["Factor"])
links.new(farben.outputs["Color"], dunkel.inputs["A"])

# Schnauze heller (typisch für Braunbären)
abstand = knoten.new("ShaderNodeVectorMath")
abstand.operation = "DISTANCE"
abstand.inputs[1].default_value = (0, -1.36, 0.9)
links.new(koordinaten.outputs["Object"], abstand.inputs[0])
schnauze = knoten.new("ShaderNodeMapRange")
schnauze.inputs["From Min"].default_value = 0.06
schnauze.inputs["From Max"].default_value = 0.2
schnauze.inputs["To Min"].default_value = 0.8
schnauze.inputs["To Max"].default_value = 0.0
links.new(abstand.outputs["Value"], schnauze.inputs["Value"])
hell = knoten.new("ShaderNodeMix")
hell.data_type = "RGBA"
hell.inputs["B"].default_value = (0.24, 0.15, 0.085, 1)
links.new(schnauze.outputs["Result"], hell.inputs["Factor"])
links.new(dunkel.outputs["Result"], hell.inputs["A"])

# Umgebungsverdeckung: Falten, Achseln und unter dem Bauch dunkler
verdeckung = knoten.new("ShaderNodeAmbientOcclusion")
verdeckung.inputs["Distance"].default_value = 0.35
links.new(hell.outputs["Result"], verdeckung.inputs["Color"])
mischen = knoten.new("ShaderNodeMix")
mischen.data_type = "RGBA"
mischen.blend_type = "MULTIPLY"
mischen.inputs["Factor"].default_value = 0.85
links.new(hell.outputs["Result"], mischen.inputs["A"])
links.new(verdeckung.outputs["AO"], mischen.inputs["B"])
links.new(mischen.outputs["Result"], bsdf.inputs["Base Color"])

nase_mat.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (0.015, 0.012, 0.012, 1)
auge_mat.node_tree.nodes["Principled BSDF"].inputs["Base Color"].default_value = (0.02, 0.012, 0.006, 1)

# ---------------------------------------------------------------------------
# Backen: Farbe (ohne Licht) in eine Textur
# ---------------------------------------------------------------------------
bild = bpy.data.images.new("baer_realistisch_fell", TEXTUR_GROESSE, TEXTUR_GROESSE)
for mat in (fell, nase_mat, auge_mat):
    ziel = mat.node_tree.nodes.new("ShaderNodeTexImage")
    ziel.image = bild
    for n in mat.node_tree.nodes:
        n.select = False
    ziel.select = True
    mat.node_tree.nodes.active = ziel

szene = bpy.context.scene
szene.render.engine = "CYCLES"
szene.cycles.samples = 64
szene.render.bake.use_pass_direct = False
szene.render.bake.use_pass_indirect = False
szene.render.bake.use_pass_color = True
szene.render.bake.margin = 8
bpy.ops.object.select_all(action="DESELECT")
baer.select_set(True)
bpy.context.view_layer.objects.active = baer
bpy.ops.object.bake(type="DIFFUSE")

# Ein einziges einfaches Material mit der gebackenen Textur – das versteht die Engine.
bild.file_format = "PNG"
spiel = material("BaerRealistisch")
knoten, links = spiel.node_tree.nodes, spiel.node_tree.links
textur = knoten.new("ShaderNodeTexImage")
textur.image = bild
links.new(textur.outputs["Color"], knoten["Principled BSDF"].inputs["Base Color"])
knoten["Principled BSDF"].inputs["Roughness"].default_value = 0.85
baer.data.materials.clear()
baer.data.materials.append(spiel)
for polygon in baer.data.polygons:
    polygon.material_index = 0

# ---------------------------------------------------------------------------
# Skelett: Rumpf, Hals, Kopf, Beine mit Ellbogen/Knie, Schwanz
# ---------------------------------------------------------------------------
VORNE = (0, 0, 1)
UNTEN = (0, 1, 0)
knochen = [
    ("Koerper", (0, 0.62, 0.97), (0, -0.5, 1.05), None, VORNE),
    ("Hals", (0, -0.5, 1.05), (0, -0.86, 1.02), "Koerper", VORNE),
    ("Kopf", (0, -0.86, 1.02), (0, -1.42, 0.92), "Hals", VORNE),
    ("Schwanz", (0, 0.92, 0.92), (0, 1.1, 0.9), "Koerper", VORNE),
]
for seite, x in (("L", 1), ("R", -1)):
    knochen += [
        (f"Bein.V{seite}.oben", (0.24 * x, -0.42, 0.92), (0.24 * x, -0.46, 0.5), "Koerper", UNTEN),
        (f"Bein.V{seite}.unten", (0.24 * x, -0.46, 0.5), (0.24 * x, -0.5, 0.07), f"Bein.V{seite}.oben", UNTEN),
        (f"Bein.H{seite}.oben", (0.23 * x, 0.6, 0.92), (0.24 * x, 0.63, 0.5), "Koerper", UNTEN),
        (f"Bein.H{seite}.unten", (0.24 * x, 0.63, 0.5), (0.24 * x, 0.6, 0.07), f"Bein.H{seite}.oben", UNTEN),
    ]
armatur = skelett("BaerRealistischSkelett", knochen)

# Weiche automatische Gewichtung: das Fell biegt sich mit, statt zu knicken.
bpy.ops.object.select_all(action="DESELECT")
baer.select_set(True)
armatur.select_set(True)
bpy.context.view_layer.objects.active = armatur
bpy.ops.object.parent_set(type="ARMATURE_AUTO")

# ---------------------------------------------------------------------------
# Animationen (30 Bilder pro Sekunde; Schleifen: letztes Bild = erstes)
# Beine: Drehung um X, positiv = Fuß nach hinten. Hebt ein Bein ab (Schwung nach vorne),
# knickt der untere Knochen ein.
# ---------------------------------------------------------------------------
def rot(bild, name, x=0.0, y=0.0, z=0.0):
    return (bild, name, "rot", (x, y, z))


def pos(bild, name, z=0.0):
    return (bild, name, "pos", (0.0, 0.0, z))


def beinzyklus(laenge, bein, versatz, schwung, knick):
    """Ein Bein: hinten (+schwung) → abheben und nach vorne (-schwung) → am Boden zurück."""
    schluessel = []
    for i in range(5):
        t = (i / 4 + versatz) % 1.0
        bild = round(i * laenge / 4)
        # Erste Hälfte: in der Luft nach vorne, zweite: am Boden nach hinten
        winkel = schwung * (1 - 4 * t) if t < 0.5 else schwung * (4 * t - 3)
        beuge = knick * math.sin(t * 2 * math.pi) if t < 0.5 else 0.0
        schluessel += [rot(bild, f"{bein}.oben", x=winkel), rot(bild, f"{bein}.unten", x=beuge)]
    return schluessel


idle = []
for bild, atmen, blick, nicken in ((0, 0.0, -10, 2), (30, 0.018, 0, -4), (60, 0.0, 12, 2), (90, 0.018, 0, -3), (120, 0.0, -10, 2)):
    idle += [pos(bild, "Koerper", atmen), rot(bild, "Kopf", x=nicken, z=blick * 0.6), rot(bild, "Hals", z=blick * 0.4)]
animation(armatur, "Idle", 120, idle)

SCHRITT = 24
laufen = []
for bein, versatz in (("Bein.VL", 0.0), ("Bein.HR", 0.0), ("Bein.VR", 0.5), ("Bein.HL", 0.5)):
    laufen += beinzyklus(SCHRITT, bein, versatz, 26, 40)
for i, hoehe in enumerate((0.0, 0.025, 0.0, 0.025, 0.0)):
    laufen += [pos(round(i * SCHRITT / 4), "Koerper", hoehe), rot(round(i * SCHRITT / 4), "Kopf", x=3 if i % 2 else -2)]
animation(armatur, "Laufen", SCHRITT, laufen)

SPRUNG = 18
rennen = []
for bein, versatz in (("Bein.VL", 0.0), ("Bein.VR", 0.08), ("Bein.HL", 0.5), ("Bein.HR", 0.58)):
    rennen += beinzyklus(SPRUNG, bein, versatz, 34, 60)
for i, (hoehe, neigung) in enumerate(((0.0, 4), (0.07, 0), (0.0, -4), (0.05, 0), (0.0, 4))):
    bild = round(i * SPRUNG / 4)
    rennen += [pos(bild, "Koerper", hoehe), rot(bild, "Koerper", x=neigung), rot(bild, "Kopf", x=-neigung)]
animation(armatur, "Rennen", SPRUNG, rennen)

ruhepose(armatur)
