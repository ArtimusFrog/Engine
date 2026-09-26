"""Vorschaubilder für das Baumenü: jedes Gebäude aus art/lib/bauten.py schräg von vorne oben,
warmes Hauptlicht, kühles Kantenlicht, durchsichtiger Hintergrund.

Aufruf (Ergebnis: game/assets/icons/bau_<name>.png, 384 × 256):
  blender --background --factory-startup --python art/icons/bauten.py [-- holzfaeller ...]
"""

import pathlib
import sys

import bpy
from mathutils import Vector

REPO = pathlib.Path(__file__).resolve().parents[2]
ZIEL = REPO / "game" / "assets" / "icons"
sys.path.insert(0, str(REPO / "art" / "lib"))

import bauten  # noqa: E402
import tuerme  # noqa: E402
import werkstatt  # noqa: E402

GEBAEUDE = {"holzfaeller": bauten.holzfaeller, "steinbruch": bauten.steinbruch, "erzmine": bauten.erzmine}


def _turm(art):
    """Turm in Stufe 2 mit aufgesetztem Kopf (wie im Spiel, Kopf 1,3-fach)."""
    def bauen():
        tuerme.turm(art, 2)
        k = tuerme.kopf(art)
        k.location = (0.0, 0.0, tuerme.KOPF_Z[2])
        k.scale = (1.3, 1.3, 1.3)
    return bauen


for _art in tuerme.TUERME:
    GEBAEUDE["turm_" + _art] = _turm(_art)
for _name in tuerme.FALLEN:
    GEBAEUDE[_name] = (lambda n: lambda: tuerme.falle(n))(_name)


def setze(obj, **werte):
    for name, wert in werte.items():
        if hasattr(obj, name):
            try:
                setattr(obj, name, wert)
            except (TypeError, ValueError):
                pass


def buehne(blick=(-0.75, -1.0, 0.72)):
    szene = bpy.context.scene
    try:
        szene.render.engine = "CYCLES"
        setze(szene.cycles, samples=48, use_denoising=True, device="CPU")
    except TypeError:
        szene.render.engine = "BLENDER_EEVEE"
    szene.render.resolution_x, szene.render.resolution_y = 384, 256
    szene.render.film_transparent = True
    szene.render.image_settings.file_format = "PNG"
    szene.render.image_settings.color_mode = "RGBA"
    setze(szene.view_settings, view_transform="Standard", look="None")
    welt = bpy.data.worlds.new("Welt") if not szene.world else szene.world
    szene.world = welt
    setze(welt, use_nodes=True)
    hintergrund = next((n for n in welt.node_tree.nodes if n.type == "BACKGROUND"), None)
    if hintergrund:
        hintergrund.inputs["Color"].default_value = (0.55, 0.62, 0.72, 1)
        hintergrund.inputs["Strength"].default_value = 0.9

    objekte = [o for o in szene.objects if o.type == "MESH"]
    bpy.context.view_layer.update()
    # Nur was über dem Boden liegt (der Sockel reicht darunter)
    ecken = [o.matrix_world @ v.co for o in objekte for v in o.data.vertices]
    ecken = [e for e in ecken if e.z > -0.1]
    tief = Vector((min(e.x for e in ecken), min(e.y for e in ecken), min(e.z for e in ecken)))
    hoch = Vector((max(e.x for e in ecken), max(e.y for e in ecken), max(e.z for e in ecken)))
    mitte = (tief + hoch) / 2
    kamera_daten = bpy.data.cameras.new("Kamera")
    kamera_daten.type = "ORTHO"
    kamera = bpy.data.objects.new("Kamera", kamera_daten)
    szene.collection.objects.link(kamera)
    richtung = Vector(blick).normalized()
    kamera.location = mitte + richtung * 40
    kamera.rotation_euler = (-richtung).to_track_quat("-Z", "Y").to_euler()
    kamera_daten.clip_end = 200
    szene.camera = kamera
    bpy.context.view_layer.update()
    inverse = kamera.matrix_world.inverted()
    projiziert = [inverse @ e for e in ecken]
    breite = max(abs(p.x) for p in projiziert) * 2
    hoehe = max(abs(p.y) for p in projiziert) * 2
    kamera_daten.ortho_scale = max(breite, hoehe * 1.5) * 1.12

    def licht(name, art, energie, farbe, ort, groesse=2.0):
        daten = bpy.data.lights.new(name, art)
        daten.energy = energie
        daten.color = farbe
        setze(daten, size=groesse, angle=0.1)
        obj = bpy.data.objects.new(name, daten)
        szene.collection.objects.link(obj)
        obj.location = mitte + Vector(ort)
        obj.rotation_euler = (-Vector(ort)).to_track_quat("-Z", "Y").to_euler()

    licht("Sonne", "SUN", 4.0, (1.0, 0.93, 0.82), (-4.0, -6.0, 8.0))
    licht("Kante", "SUN", 1.2, (0.6, 0.75, 1.0), (6.0, 5.0, 3.0))


def main():
    namen = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else list(GEBAEUDE)
    ZIEL.mkdir(parents=True, exist_ok=True)
    for name in namen:
        werkstatt.neu()
        GEBAEUDE[name]()
        if name.startswith("turm_"):
            buehne((-0.8, -1.0, 0.55))
        elif name in tuerme.FALLEN:
            buehne((-0.7, -1.0, 1.1))
        else:
            buehne()
        pfad = ZIEL / f"bau_{name}.png"
        bpy.context.scene.render.filepath = str(pfad)
        bpy.ops.render.render(write_still=True)
        print(f"FERTIG {name} -> {pfad.relative_to(REPO)}")


main()
