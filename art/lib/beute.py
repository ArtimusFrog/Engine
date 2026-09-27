"""Beute am Boden (siehe game/src/beute.rs): kleine Modelle für Gold, Runenfragment, Fleisch, Fell und
Wolle sowie die zehn Waffen, liegend. Das Spiel lässt sie sanft über dem Boden schweben und sich
drehen; Waffen und Fragmente bekommen dort zusätzlich eine Lichtsäule.

Aufruf über die Modell-Skripte in art/modelle/gegenstaende/ (je eine Zeile: modell("<name>")).
"""

import math

import bpy
from mathutils import Matrix

from lager import einfarbig, holzfarbe, stammfarbe
from tuerme import Werk
from vorkommen import _steinfarbe, farbe

GOLD = "#E8B53A"


def _gold(w):
    """Ein Häufchen Goldmünzen."""
    for i in range(9):
        winkel = i * 2.4
        r = 0.02 + 0.045 * math.sqrt(i)
        z = 0.012 + (0.02 if i < 3 else 0.0) + (0.018 if i == 0 else 0.0)
        w.dreh("Münze", einfarbig(GOLD if i % 3 else "#F2CC5A", 0.05, w.z), [(0.045, -0.007), (0.045, 0.007), (0.035, 0.009)],
               (math.cos(winkel) * r, math.sin(winkel) * r, z), ecken=12, drehung=(w.z.uniform(-15, 15), w.z.uniform(-15, 15), 0))
    w.dreh("Münze", einfarbig(GOLD, 0.05, w.z), [(0.045, -0.007), (0.045, 0.007), (0.035, 0.009)], (0.05, -0.02, 0.06), ecken=12, drehung=(70, 0, 20))


def _runenfragment(w):
    """Zwei blaugraue Splitter, auf dem größeren leuchtet eine Rune."""
    stein = _steinfarbe(w.z, farbe("#4E5A7E"), farbe("#6E7BA0"), farbe("#8E9CC0"), farbe("#B8C6E6"))
    w.stein("Splitter", stein, 0.14, (0.0, 0.0, 0.24), flach=1.6, drehung_z=20)
    w.stein("Splitter", stein, 0.08, (0.14, 0.06, 0.1), flach=1.2, drehung_z=70)
    rune = einfarbig("#8FD8FF", 0.03, w.z)
    w.brett("Rune", rune, (0.02, 0.012, 0.16), (0.0, -0.12, 0.26), (0, 0, 20), fase=0.0, glut=True)
    w.brett("Rune", rune, (0.08, 0.012, 0.02), (0.0, -0.12, 0.29), (0, 30, 20), fase=0.0, glut=True)
    w.dreh("Funke", rune, [(0.0, 0.0), (0.025, 0.04), (0.0, 0.1)], (-0.1, 0.08, 0.02), ecken=5, glut=True)


def _fleisch(w):
    """Eine Keule: Fleisch um einen Knochen."""
    fleisch = einfarbig("#A8513A", 0.06, w.z)
    knochen = einfarbig("#EDE3CB", 0.03, w.z)
    w.dreh("Keule", fleisch, [(0.0, -0.13), (0.07, -0.1), (0.1, -0.02), (0.085, 0.06), (0.04, 0.1), (0.0, 0.11)], (0, 0, 0.09), ecken=12, drehung=(0, 90, 0))
    w.saeule("Knochen", knochen, 0.018, (0.1, 0, 0.09), (0.22, 0, 0.1), ecken=6)
    for dy in (-0.022, 0.022):
        w.dreh("Knochenende", knochen, [(0.0, -0.02), (0.022, 0.0), (0.0, 0.02)], (0.23, dy, 0.1), ecken=6)


def _fell(w):
    """Ein zusammengerolltes Fell mit Schnur."""
    fell = stammfarbe(w.z, "#8A6A48", "#6A4E34", "#B89870", "#9C7E58")
    w.stamm("Fellrolle", fell, 0.08, 0.34, (0, 0, 0.08), (0, 0, 0), ecken=10)
    schnur = einfarbig("#5A3A20", 0.04, w.z)
    for x in (-0.09, 0.09):
        w.dreh("Schnur", schnur, [(0.084, -0.012), (0.084, 0.012)], (x, 0, 0.08), ecken=10, drehung=(0, 90, 0))


def _wolle(w):
    """Ein Bündel weißer Wolle."""
    wolle = einfarbig("#F2EEE4", 0.05, w.z)
    for i, (x, y, r) in enumerate(((0.0, 0.0, 0.1), (0.1, 0.03, 0.075), (-0.08, 0.05, 0.07), (0.03, -0.08, 0.07))):
        w.stein("Wolle", wolle, r, (x, y, r * 0.7), flach=0.8, drehung_z=i * 50)


GEGENSTAENDE = {"beute_gold": _gold, "beute_runenfragment": _runenfragment, "beute_fleisch": _fleisch, "beute_fell": _fell, "beute_wolle": _wolle}


def _waffe_liegend(art):
    """Eine Waffe aus waffen.py, auf den Boden gelegt (mit den Farben der Figuren)."""
    import waffen
    from figuren import Figur
    f = Figur(art, 5)
    ohne = lambda co: {}
    if art in waffen.STAEBE:
        waffen.stab(f, art, 0.0, 0.0, ohne)
    else:
        waffen.hammer(f, art, 0.0, 0.0, ohne, 0.6)
    bpy.ops.object.select_all(action="DESELECT")
    for teil in f.teile:
        teil.select_set(True)
    bpy.context.view_layer.objects.active = f.teile[0]
    bpy.ops.object.join()
    obj = f.teile[0]
    obj.name = art
    obj.vertex_groups.clear()
    mat = bpy.data.materials.new(art)
    try:
        mat.use_nodes = True
    except (AttributeError, TypeError):
        pass
    knoten, links = mat.node_tree.nodes, mat.node_tree.links
    bsdf = next(k for k in knoten if k.type == "BSDF_PRINCIPLED")
    vc = knoten.new("ShaderNodeVertexColor")
    vc.layer_name = "Farbe"
    links.new(vc.outputs["Color"], bsdf.inputs["Base Color"])
    bsdf.inputs["Roughness"].default_value = 0.7
    obj.data.materials.clear()
    obj.data.materials.append(mat)
    # Hinlegen: die Längsachse (Z) wird zur X-Achse, dann mittig auf den Boden
    mesh = obj.data
    skala = 0.8 if art in waffen.STAEBE else 1.0
    mesh.transform(Matrix.Scale(skala, 4) @ Matrix.Rotation(math.radians(90), 4, "Y"))
    xs = [v.co.x for v in mesh.vertices]
    ys = [v.co.y for v in mesh.vertices]
    zs = [v.co.z for v in mesh.vertices]
    mesh.transform(Matrix.Translation((-(min(xs) + max(xs)) / 2, -(min(ys) + max(ys)) / 2, -min(zs))))
    mesh.update()
    dreiecke = sum(len(p.vertices) - 2 for p in mesh.polygons)
    print(f"MODELL {art}: {dreiecke} Dreiecke")
    return obj


def modell(name):
    if name in GEGENSTAENDE:
        w = Werk(len(name))
        GEGENSTAENDE[name](w)
        return w.fertig(name)
    return _waffe_liegend(name)
