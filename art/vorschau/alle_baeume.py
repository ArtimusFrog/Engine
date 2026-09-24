"""Schaukasten: alle Bäume der Insel nebeneinander, mit Namensschild (nur Vorschau, kein Export).

Öffnen: dieses Skript auf „Blender live.bat“ ziehen. Ändert sich der Baum-Generator
(art/lib/baeume.py), baut sich der ganze Schaukasten neu.
"""

import bpy

from baeume import eiche, palme, tanne, zauberbaum

# Eine Reihe je Art: (Name, Funktion, Einstellungen je Variante)
REIHEN = [
    ("Eiche", eiche, [dict(seed=11), dict(seed=22), dict(seed=33)]),
    ("Tanne", tanne, [dict(seed=17), dict(seed=34), dict(seed=51)]),
    ("Palme", palme, [dict(seed=5), dict(seed=10)]),
    ("Zauberbaum", zauberbaum, [dict(seed=7), dict(seed=14)]),
]
ABSTAND_X = 8.0
ABSTAND_Y = 10.0

schild_mat = bpy.data.materials.get("Schild") or bpy.data.materials.new("Schild")
schild_mat.diffuse_color = (0.9, 0.85, 0.7, 1)

for zeile, (art, funktion, varianten) in enumerate(REIHEN):
    for spalte, einstellungen in enumerate(varianten):
        baum = funktion(name=f"{art} {spalte + 1}", **einstellungen)
        baum.location = (spalte * ABSTAND_X, zeile * ABSTAND_Y, 0)

        text = bpy.data.curves.new(f"Schild {art} {spalte + 1}", "FONT")
        text.body = f"{art} {spalte + 1}"
        text.size = 0.8
        text.align_x = "CENTER"
        schild = bpy.data.objects.new(text.name, text)
        schild.location = (spalte * ABSTAND_X, zeile * ABSTAND_Y - 3.2, 0.02)
        schild.data.materials.append(schild_mat)
        bpy.context.scene.collection.objects.link(schild)
