"""Schaukasten: alle Bäume und Pflanzen der Insel nebeneinander, mit Namensschild (nur Vorschau, kein Export).

Öffnen: dieses Skript auf „Blender live.bat“ ziehen. Ändert sich der Baum-Generator
(art/lib/baeume.py, art/lib/pflanzen.py), baut sich der ganze Schaukasten neu.
"""

import runpy
import pathlib

import bpy

MODELLE = pathlib.Path(__file__).resolve().parents[1] / "modelle" / "natur"
# Eine Reihe je Art: (Name, Anzahl Varianten, Abstand)
REIHEN = [("eiche", 3, 10.0), ("birke", 3, 7.0), ("tanne", 3, 8.0), ("palme", 2, 8.0), ("zauberbaum", 2, 9.0), ("busch", 3, 3.0), ("gras", 3, 2.0)]
ABSTAND_Y = 11.0

schild_mat = bpy.data.materials.get("Schild") or bpy.data.materials.new("Schild")
schild_mat.diffuse_color = (0.9, 0.85, 0.7, 1)

for zeile, (art, anzahl, abstand) in enumerate(REIHEN):
    for spalte in range(anzahl):
        vorher = set(bpy.data.objects)
        runpy.run_path(str(MODELLE / f"{art}_{spalte + 1}.py"), run_name="__main__")
        for obj in set(bpy.data.objects) - vorher:
            obj.location = (spalte * abstand, zeile * ABSTAND_Y, 0)
        text = bpy.data.curves.new(f"Schild {art} {spalte + 1}", "FONT")
        text.body = f"{art.capitalize()} {spalte + 1}"
        text.size = 0.8
        text.align_x = "CENTER"
        schild = bpy.data.objects.new(text.name, text)
        schild.location = (spalte * abstand, zeile * ABSTAND_Y - 3.2, 0.02)
        schild.data.materials.append(schild_mat)
        bpy.context.scene.collection.objects.link(schild)
