"""Baut ein Asset: führt ein Modell-Skript (oder eine .blend-Datei) in Blender aus und
exportiert das Ergebnis nach game/assets/.

Aufruf (macht „Assets bauen.bat“ für dich):
    blender --background --factory-startup --python-exit-code 1 --python art/lib/bauen.py -- art/modelle/natur/fels.py

art/modelle/natur/fels.py   → game/assets/natur/fels.gltf (+ fels.bin)
art/modelle/natur/eiche.blend → game/assets/natur/eiche.gltf
"""

import pathlib
import runpy
import sys

import bpy

REPO = pathlib.Path(__file__).resolve().parents[2]
MODELLE = REPO / "art" / "modelle"
ZIEL = REPO / "game" / "assets"

sys.path.insert(0, str(REPO / "art" / "lib"))
import werkstatt  # noqa: E402


def main():
    argumente = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    if not argumente:
        raise SystemExit("Welches Modell? Pfad nach -- angeben.")
    quelle = pathlib.Path(argumente[0]).resolve()
    ziel = (ZIEL / quelle.relative_to(MODELLE)).with_suffix(".gltf")

    if quelle.suffix == ".blend":
        bpy.ops.wm.open_mainfile(filepath=str(quelle))
    else:
        werkstatt.neu()
        runpy.run_path(str(quelle), run_name="__main__")

    werkstatt.exportieren(ziel)
    print(f"FERTIG {quelle.relative_to(REPO)} -> {ziel.relative_to(REPO)}")


main()
