"""Blender live: ein Modell-Skript in Blender (mit Fenster) bauen und bei jeder Änderung neu bauen.

Start: „Blender live.bat“ – ein Skript aus art/modelle/ daraufziehen – oder
  blender --python art/lib/live.py -- art/modelle/tiere/baer.py

Bei jeder Änderung am Skript oder an art/lib/*.py:
  - Szene leeren, Skript ausführen (die Ansicht bleibt, wie man sie gedreht hat)
  - nach game/assets/ exportieren – Spiel-Betrachter und Asset-Galerie laden automatisch mit
  - Fehler erscheinen oben im 3D-Fenster statt Blender abstürzen zu lassen
Animationen: Leertaste spielt ab (Idle ist eingestellt), andere im Aktions-Editor wählen.
"""

import importlib
import pathlib
import runpy
import sys
import time
import traceback

import bpy

REPO = pathlib.Path(__file__).resolve().parents[2]
LIB = REPO / "art" / "lib"
MODELLE = REPO / "art" / "modelle"
ZIEL = REPO / "game" / "assets"
sys.path.insert(0, str(LIB))

argumente = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
QUELLE = pathlib.Path(argumente[0]).resolve() if argumente else None

_zustand = {"stempel": None, "erstes_mal": True}


def _stempel():
    """Jüngste Änderung an der Quelle oder der Bibliothek."""
    dateien = [QUELLE] + sorted(LIB.glob("*.py"))
    return max(p.stat().st_mtime for p in dateien if p.exists())


def _ansichten():
    """Alle 3D-Ansichten in allen Fenstern: (Fenster, Bereich, Region)."""
    for fenster in bpy.context.window_manager.windows:
        for bereich in fenster.screen.areas:
            if bereich.type == "VIEW_3D":
                region = next((r for r in bereich.regions if r.type == "WINDOW"), None)
                if region:
                    yield fenster, bereich, region


def _hinweis(text):
    """Text oben im 3D-Fenster (None = Hinweis entfernen)."""
    for _, bereich, _ in _ansichten():
        bereich.header_text_set(text)


def _module_neu_laden():
    """Geänderte Werkstatt-Bibliotheken neu einlesen (sonst gelten alte Fassungen)."""
    for name, modul in list(sys.modules.items()):
        datei = getattr(modul, "__file__", None)
        if datei and pathlib.Path(datei).resolve().parent == LIB and name != "live":
            importlib.reload(modul)


def neu_bauen():
    beginn = time.perf_counter()
    _module_neu_laden()
    import werkstatt  # noqa: E402 – nach dem Neuladen

    try:
        if bpy.context.object and bpy.context.object.mode != "OBJECT":
            bpy.ops.object.mode_set(mode="OBJECT")
        werkstatt.neu()
        runpy.run_path(str(QUELLE), run_name="__main__")
        # Nur echte Modelle (art/modelle/) gehen ins Spiel; Vorschau-Skripte (art/vorschau/) nicht.
        if MODELLE in QUELLE.parents:
            werkstatt.exportieren((ZIEL / QUELLE.relative_to(MODELLE)).with_suffix(".gltf"))
    except Exception:
        fehler = traceback.format_exc()
        print(fehler)
        letzte = fehler.strip().splitlines()[-1]
        _hinweis(f"FEHLER in {QUELLE.name}: {letzte}   (Details in der Konsole: Fenster → Systemkonsole)")
        return

    # Animation zum Ansehen: Idle (oder die erste) aufs Skelett legen.
    for obj in bpy.data.objects:
        if obj.type == "ARMATURE" and bpy.data.actions:
            aktion = bpy.data.actions.get("Idle") or bpy.data.actions[0]
            obj.animation_data_create()
            obj.animation_data.action = aktion
            szene = bpy.context.scene
            szene.frame_start, szene.frame_end = 0, int(aktion.frame_range[1])
            break

    for fenster, bereich, region in _ansichten():
        raum = bereich.spaces.active
        raum.shading.type = "MATERIAL"
        if _zustand["erstes_mal"]:
            with bpy.context.temp_override(window=fenster, area=bereich, region=region):
                bpy.ops.view3d.view_all(center=False)
    _zustand["erstes_mal"] = False

    dauer = time.perf_counter() - beginn
    exportiert = ", exportiert" if MODELLE in QUELLE.parents else " (nur Vorschau)"
    _hinweis(f"LIVE: {QUELLE.name} – neu gebaut in {dauer:.1f} s{exportiert}. Leertaste: Animation abspielen.")
    print(f"LIVE neu gebaut: {QUELLE.name} ({dauer:.1f} s)")


def _pruefen():
    """Alle halbe Sekunde: hat sich etwas geändert? Dann neu bauen."""
    try:
        stempel = _stempel()
        if stempel != _zustand["stempel"]:
            _zustand["stempel"] = stempel
            neu_bauen()
    except Exception:
        traceback.print_exc()
    return 0.5


if QUELLE is None or not QUELLE.exists():
    print("Blender live: Welches Modell? Pfad nach -- angeben, z. B. art/modelle/tiere/baer.py")
elif (REPO / "art") not in QUELLE.parents:
    print(f"Blender live: {QUELLE} liegt nicht unter art/")
else:
    # Erst bauen, wenn das Fenster steht.
    bpy.app.timers.register(_pruefen, first_interval=0.5, persistent=True)
