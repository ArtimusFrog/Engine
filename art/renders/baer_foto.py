"""Fotorealistischer Braunbär als Standbild (Cycles), um zu zeigen, was mit Blender per Skript geht.

Aufruf:
  blender --background --factory-startup --python art/renders/baer_foto.py -- <ausgabe.png> [samples]

Aufbau:
  - Körper: Skin-Modifier auf einem „Knochengerüst“ aus Punkten mit Radien → Unterteilung →
    leichte Unebenheiten (Muskeln, Fettpolster)
  - Fell: Haar-Partikel mit Kind-Haaren, Principled Hair BSDF (Melanin), Büschel und Unordnung
  - Umgebung: physikalischer Himmel mit tiefer Sonne, Boden mit Gras-Haaren, Tiefenschärfe
"""

import math
import sys

import bpy
from mathutils import Vector

args = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
AUSGABE = args[0] if args else "baer_foto.png"
SAMPLES = int(args[1]) if len(args) > 1 else 128

# ---------------------------------------------------------------------------
# Szene leeren
# ---------------------------------------------------------------------------
for sammlung in (bpy.data.objects, bpy.data.meshes, bpy.data.materials, bpy.data.particles, bpy.data.lights, bpy.data.cameras):
    for eintrag in list(sammlung):
        sammlung.remove(eintrag)
szene = bpy.context.scene


def setze(obj, **werte):
    """Eigenschaften setzen, die es in dieser Blender-Version gibt (andere still überspringen)."""
    for name, wert in werte.items():
        if hasattr(obj, name):
            setattr(obj, name, wert)


def verlinken(obj):
    szene.collection.objects.link(obj)
    return obj


# ---------------------------------------------------------------------------
# Körper: gemeinsame Form aus art/lib/baer_form.py (auch fürs Spielmodell)
# ---------------------------------------------------------------------------
sys.path.insert(0, str(__import__("pathlib").Path(__file__).resolve().parents[1] / "lib"))
import baer_form  # noqa: E402

koerper = baer_form.koerper("Baer", aufloesung=0.03)

# ---------------------------------------------------------------------------
# Materialien
# ---------------------------------------------------------------------------
def material(name):
    mat = bpy.data.materials.new(name)
    setze(mat, use_nodes=True)
    return mat, mat.node_tree.nodes, mat.node_tree.links


# Haut unter dem Fell (sieht man nur an lichten Stellen): dunkelbraun
haut, knoten, _ = material("Haut")
knoten["Principled BSDF"].inputs["Base Color"].default_value = (0.03, 0.018, 0.01, 1)
knoten["Principled BSDF"].inputs["Roughness"].default_value = 0.8
koerper.data.materials.append(haut)

# Fell: Principled Hair mit Melanin; Farbe variiert zufällig je Haar (Random Color).
fell, knoten, verbinde = material("Fell")
for n in list(knoten):
    knoten.remove(n)
haar = knoten.new("ShaderNodeBsdfHairPrincipled")
setze(haar, parametrization="MELANIN")
# Entlang des Haares heller werdend: Wurzel dunkel, Spitzen goldbraun (typisch Braunbär).
haar_info = knoten.new("ShaderNodeHairInfo")
verlauf = knoten.new("ShaderNodeMapRange")
verlauf.inputs["To Min"].default_value = 0.9
verlauf.inputs["To Max"].default_value = 0.6
verbinde.new(haar_info.outputs["Intercept"], verlauf.inputs["Value"])
verbinde.new(verlauf.outputs["Result"], haar.inputs["Melanin"])
for eingang, wert in (("Melanin Redness", 0.66), ("Roughness", 0.5), ("Radial Roughness", 0.6), ("Random Color", 0.25), ("Random Roughness", 0.2), ("Coat", 0.05)):
    if eingang in haar.inputs:
        haar.inputs[eingang].default_value = wert
ausgabe = knoten.new("ShaderNodeOutputMaterial")
verbinde.new(haar.outputs[0], ausgabe.inputs["Surface"])
koerper.data.materials.append(fell)

# Nase: feucht glänzend, fast schwarz
nase, knoten, _ = material("Nase")
knoten["Principled BSDF"].inputs["Base Color"].default_value = (0.012, 0.01, 0.01, 1)
knoten["Principled BSDF"].inputs["Roughness"].default_value = 0.32

# Augen: dunkel, mit Glanzlicht
auge, knoten, _ = material("Auge")
knoten["Principled BSDF"].inputs["Base Color"].default_value = (0.02, 0.012, 0.006, 1)
knoten["Principled BSDF"].inputs["Roughness"].default_value = 0.05


def kugel(name, ort, radius, mat, skalierung=(1, 1, 1)):
    bpy.ops.mesh.primitive_uv_sphere_add(radius=radius, location=ort, segments=32, ring_count=16)
    obj = bpy.context.active_object
    obj.name = name
    obj.scale = skalierung
    obj.data.materials.append(mat)
    bpy.ops.object.shade_smooth()
    return obj


kugel("Nase", (0, -1.475, 0.915), 0.04, nase, (1.35, 0.75, 0.85))
# Augen genau auf die Kopfoberfläche
augen_orte = baer_form.augen_orte(koerper)
for i, ort in enumerate(augen_orte):
    kugel(f"Auge{i}", ort, 0.021, auge)

# ---------------------------------------------------------------------------
# Fell als Haar-Partikel
# ---------------------------------------------------------------------------
# Wo Fell wächst und wie lang: kurz an Schnauze, Tatzen und rund um die Augen.
nasen_ort = baer_form.NASE
gruppe = koerper.vertex_groups.new(name="Felllaenge")
for v in koerper.data.vertices:
    p = v.co
    laenge = 1.0
    if p.y < -1.3:  # Schnauze
        laenge = 0.22
    elif p.y < -1.15:
        laenge = 0.5
    if p.z < 0.12:  # Tatzen
        laenge = min(laenge, 0.35)
    naehe = min((p - a).length for a in augen_orte + [nasen_ort])
    if naehe < 0.07:  # Augen und Nase frei lassen
        laenge = min(laenge, max(0.0, (naehe - 0.035) / 0.035) * 0.3)
    gruppe.add([v.index], laenge, "REPLACE")

system = koerper.modifiers.new("Fell", "PARTICLE_SYSTEM").particle_system
s = system.settings
s.type = "HAIR"
setze(s, count=26000, use_advanced_hair=True, material_slot="Fell")
setze(s, child_type="INTERPOLATED", child_percent=4, rendered_child_count=22, child_radius=0.03, child_roundness=0.5)
setze(s, length_random=0.4, clump_factor=0.65, clump_shape=-0.15, roughness_1=0.015, roughness_2=0.035, roughness_2_size=0.6, roughness_endpoint=0.008)
setze(s, root_radius=1.0, tip_radius=0.0, radius_scale=0.012, use_close_tip=True)
setze(s, child_length=0.9, child_length_threshold=0.2)
system.vertex_group_length = "Felllaenge"
system.vertex_group_density = "Felllaenge"
# Länge und Richtung in Metern: etwas von der Haut weg, vor allem nach hinten (+Y) und
# unten – so liegt das Fell an wie bei einem echten Bären, statt abzustehen.
setze(s, normal_factor=0.05, tangent_factor=0.0, object_align_factor=(0.0, 0.065, -0.035))
setze(s, use_rotations=True, rotation_mode="NOR_TAN", phase_factor_random=0.5)

# ---------------------------------------------------------------------------
# Umgebung: Boden mit Gras, physikalischer Himmel, Kamera
# ---------------------------------------------------------------------------
bpy.ops.mesh.primitive_plane_add(size=60, location=(0, 0, 0))
boden = bpy.context.active_object
boden_mat, knoten, verbinde = material("Boden")
rauschen = knoten.new("ShaderNodeTexNoise")
rauschen.inputs["Scale"].default_value = 2.5
rampe = knoten.new("ShaderNodeValToRGB")
rampe.color_ramp.elements[0].color = (0.035, 0.028, 0.015, 1)
rampe.color_ramp.elements[1].color = (0.06, 0.09, 0.025, 1)
verbinde.new(rauschen.outputs["Fac"], rampe.inputs["Fac"])
verbinde.new(rampe.outputs["Color"], knoten["Principled BSDF"].inputs["Base Color"])
knoten["Principled BSDF"].inputs["Roughness"].default_value = 0.95
boden.data.materials.append(boden_mat)

# Gras in der Nähe (weiter weg fällt es in der Unschärfe nicht auf)
bpy.ops.mesh.primitive_plane_add(size=9, location=(0.5, -0.8, 0.0))
wiese = bpy.context.active_object
bpy.ops.object.mode_set(mode="EDIT")
bpy.ops.mesh.subdivide(number_cuts=20)
bpy.ops.object.mode_set(mode="OBJECT")
gras_mat, knoten, verbinde = material("Gras")
zufall = knoten.new("ShaderNodeHairInfo")
gras_rampe = knoten.new("ShaderNodeValToRGB")
gras_rampe.color_ramp.elements[0].color = (0.05, 0.1, 0.02, 1)
gras_rampe.color_ramp.elements[1].color = (0.16, 0.2, 0.05, 1)
verbinde.new(zufall.outputs["Random"], gras_rampe.inputs["Fac"])
verbinde.new(gras_rampe.outputs["Color"], knoten["Principled BSDF"].inputs["Base Color"])
knoten["Principled BSDF"].inputs["Roughness"].default_value = 0.6
wiese.data.materials.append(boden_mat)
wiese.data.materials.append(gras_mat)
gras = wiese.modifiers.new("Gras", "PARTICLE_SYSTEM").particle_system.settings
gras.type = "HAIR"
setze(gras, count=9000, hair_length=0.09, use_advanced_hair=True, material_slot="Gras")
setze(gras, child_type="INTERPOLATED", child_percent=2, rendered_child_count=12, child_radius=0.25)
setze(gras, roughness_2=0.08, roughness_endpoint=0.05, length_random=0.6)
setze(gras, root_radius=1.0, tip_radius=0.0, radius_scale=0.035, use_close_tip=True)

welt = bpy.data.worlds.new("Himmel") if not szene.world else szene.world
szene.world = welt
setze(welt, use_nodes=True)
knoten, verbinde = welt.node_tree.nodes, welt.node_tree.links
himmel = knoten.new("ShaderNodeTexSky")
setze(himmel, sky_type="MULTIPLE_SCATTERING", sun_elevation=math.radians(28), sun_disc=False, altitude=100.0)
hintergrund = knoten.get("Background") or knoten.new("ShaderNodeBackground")
hintergrund.inputs["Strength"].default_value = 0.14
verbinde.new(himmel.outputs["Color"], hintergrund.inputs["Color"])
welt_ausgabe = knoten.get("World Output") or knoten.new("ShaderNodeOutputWorld")
verbinde.new(hintergrund.outputs["Background"], welt_ausgabe.inputs["Surface"])

sonne_daten = bpy.data.lights.new("Sonne", "SUN")
sonne_daten.energy = 3.8
sonne_daten.angle = math.radians(2.5)
sonne_daten.color = (1.0, 0.9, 0.78)
sonne = verlinken(bpy.data.objects.new("Sonne", sonne_daten))
sonne.rotation_euler = (Vector((0, -0.4, 0.9)) - Vector((2.6, -6.0, 3.4))).to_track_quat("-Z", "Y").to_euler()

kamera_daten = bpy.data.cameras.new("Kamera")
kamera_daten.lens = 65
kamera_daten.dof.use_dof = True
kamera_daten.dof.aperture_fstop = 2.8
kamera = verlinken(bpy.data.objects.new("Kamera", kamera_daten))
kamera.location = (3.3, -5.4, 0.95)
ziel = Vector((0.1, -0.3, 0.72))
kamera.rotation_euler = (ziel - kamera.location).to_track_quat("-Z", "Y").to_euler()
kamera_daten.dof.focus_distance = (Vector((0, -0.9, 1.0)) - kamera.location).length
szene.camera = kamera

# ---------------------------------------------------------------------------
# Rendern mit Cycles (Grafikkarte, falls möglich)
# ---------------------------------------------------------------------------
szene.render.engine = "CYCLES"
cycles_prefs = bpy.context.preferences.addons["cycles"].preferences
for art in ("OPTIX", "CUDA"):
    try:
        cycles_prefs.compute_device_type = art
        cycles_prefs.get_devices()
        geraete = [d for d in cycles_prefs.devices if d.type == art]
        if geraete:
            for d in cycles_prefs.devices:
                d.use = d.type == art
            szene.cycles.device = "GPU"
            print(f"RENDER mit {art}: {[d.name for d in geraete]}")
            break
    except TypeError:
        continue
szene.cycles.samples = SAMPLES
setze(szene.cycles, use_denoising=True, denoiser="OPENIMAGEDENOISE", use_adaptive_sampling=True)
szene.render.resolution_x = 1600
szene.render.resolution_y = 1000
szene.render.film_transparent = False
setze(szene.view_settings, view_transform="AgX", look="AgX - Punchy")
szene.render.filepath = AUSGABE
bpy.ops.render.render(write_still=True)
print(f"GERENDERT {AUSGABE}")
