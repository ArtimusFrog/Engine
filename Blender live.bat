@echo off
setlocal
rem Blender live: ein Modell-Skript aus art\modelle auf diese Datei ziehen.
rem Blender oeffnet sich und baut das Modell bei jeder Aenderung am Skript sofort neu
rem (und exportiert es ins Spiel). Einzelheiten in art\lib\live.py.
cd /d "%~dp0"
if "%~1"=="" (
  echo Ziehe ein Modell-Skript aus art\modelle ^(z. B. art\modelle\tiere\baer.py^) auf diese Datei.
  pause
  exit /b 1
)
set "BLENDER=%BLENDER%"
if not defined BLENDER (
  for /d %%d in ("%ProgramFiles%\Blender Foundation\Blender*") do if exist "%%d\blender.exe" set "BLENDER=%%d\blender.exe"
)
if not defined BLENDER (
  echo Blender wurde nicht gefunden. Bitte installieren: winget install BlenderFoundation.Blender
  pause
  exit /b 1
)
rem Eine leere Blender-Datei oeffnen - dann zeigt Blender keinen Begruessungsbildschirm.
set "LEER=%TEMP%\engine_live_leer.blend"
if not exist "%LEER%" "%BLENDER%" --background --factory-startup --python-expr "import bpy; bpy.ops.wm.read_factory_settings(use_empty=True); bpy.ops.wm.save_as_mainfile(filepath=r'%LEER%')" >nul 2>&1
start "" "%BLENDER%" "%LEER%" --python art\lib\live.py -- "%~f1"
