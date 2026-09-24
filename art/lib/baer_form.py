"""Körperform des realistischen Braunbären – gemeinsam für den Cycles-Render
(art/renders/baer_foto.py) und das Spielmodell (art/modelle/tiere/baer_realistisch.py).

Der Körper besteht aus Metaballs: weiche Ellipsoide, die wie Muskeln und Fettpolster
ineinander übergehen – ohne Knickstellen. Koordinaten: Z oben, Bär schaut nach -Y,
Maße wie ein erwachsener Braunbär (~2,1 m lang, Schulter ~1,3 m).
"""

import bpy
from mathutils import Vector

# (Mitte, Halbachsen x/y/z in Metern)
FORMEN = [
    # Rumpf: Becken, Bauch (hängt etwas), Brust, Schulterbuckel – schmaler als hoch
    ((0, 0.62, 0.97), (0.31, 0.38, 0.33)),
    ((0, 0.12, 0.95), (0.35, 0.48, 0.36)),
    ((0, -0.32, 1.0), (0.34, 0.36, 0.38)),
    ((0, -0.45, 1.17), (0.26, 0.28, 0.22)),
    # Hals und Kopf: deutlicher Hals, breiter Schädel, Wangen, Stirnabsatz, längere Schnauze
    ((0, -0.8, 1.02), (0.22, 0.26, 0.23)),
    ((0, -1.07, 0.98), (0.235, 0.21, 0.21)),
    ((0.1, -1.12, 0.92), (0.11, 0.11, 0.1)),
    ((-0.1, -1.12, 0.92), (0.11, 0.11, 0.1)),
    ((0, -1.33, 0.9), (0.095, 0.16, 0.085)),
    ((0, -1.24, 0.98), (0.085, 0.11, 0.075)),
    # Schwanz
    ((0, 1.0, 0.9), (0.07, 0.07, 0.07)),
]
for _x in (-1, 1):
    FORMEN += [
        # Ohren: runde Muscheln oben am Kopf
        ((0.17 * _x, -1.0, 1.2), (0.085, 0.045, 0.085)),
        # Vorderbein: Oberarm, Unterarm, breite Tatze
        ((0.23 * _x, -0.42, 0.72), (0.14, 0.15, 0.28)),
        ((0.24 * _x, -0.47, 0.32), (0.105, 0.11, 0.27)),
        ((0.24 * _x, -0.54, 0.06), (0.11, 0.15, 0.06)),
        # Hinterbein: dicker Oberschenkel, Unterschenkel, lange Sohle
        ((0.22 * _x, 0.6, 0.72), (0.17, 0.23, 0.3)),
        ((0.24 * _x, 0.64, 0.3), (0.1, 0.115, 0.26)),
        ((0.24 * _x, 0.55, 0.06), (0.11, 0.17, 0.06)),
    ]

NASE = Vector((0, -1.475, 0.915))

# Sichtbarer Radius einer Metaball-Kugel ≈ 0,575 × Einflussradius (Schwelle 0,6, Härte 2).
_SICHTBAR = 0.575


def koerper(name="Baer", aufloesung=0.03, beulen=0.025):
    """Baut den Körper als Mesh (weich schattiert) und gibt das Objekt zurück."""
    kugeln = bpy.data.metaballs.new(name + "Form")
    kugeln.resolution = aufloesung
    kugeln.render_resolution = aufloesung
    kugeln.threshold = 0.6
    for mitte, (hx, hy, hz) in FORMEN:
        element = kugeln.elements.new(type="ELLIPSOID")
        element.co = mitte
        element.radius = 1.0
        element.size_x, element.size_y, element.size_z = hx / _SICHTBAR, hy / _SICHTBAR, hz / _SICHTBAR
        element.stiffness = 2.0
    form = bpy.data.objects.new(name + "Form", kugeln)
    bpy.context.scene.collection.objects.link(form)
    bpy.context.view_layer.update()
    bpy.ops.object.select_all(action="DESELECT")
    form.select_set(True)
    bpy.context.view_layer.objects.active = form
    bpy.ops.object.convert(target="MESH")
    obj = bpy.context.active_object
    obj.name = name

    if beulen > 0:
        # Unebenheiten: Muskeln, Fettpolster – großflächiges Rauschen, nur leicht.
        textur = bpy.data.textures.new(name + "Beulen", "CLOUDS")
        textur.noise_scale = 0.3
        modifier = obj.modifiers.new("Beulen", "DISPLACE")
        modifier.texture = textur
        modifier.strength = beulen
        modifier.mid_level = 0.5
        bpy.ops.object.modifier_apply(modifier=modifier.name)
    for polygon in obj.data.polygons:
        polygon.use_smooth = True
    return obj


def augen_orte(obj):
    """Wo die Augen auf der Kopfoberfläche sitzen (Strahl von vorne auf den Kopf)."""
    orte = []
    for x in (-1, 1):
        treffer, ort, normale, _ = obj.ray_cast(Vector((0.11 * x, -2.0, 1.04)), Vector((0, 1, 0)))
        orte.append((ort - normale * 0.008) if treffer else Vector((0.11 * x, -1.2, 1.04)))
    return orte
