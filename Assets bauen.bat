@echo off
setlocal enabledelayedexpansion
rem Baut Assets mit Blender: art\modelle\...\name.py (oder .blend) wird zu game\assets\...\name.gltf
rem Ohne Angabe werden alle gebaut, sonst nur die angegebenen (auch per Drag and Drop).
cd /d "%~dp0"
set "BLENDER=%BLENDER%"
if not defined BLENDER (
  for /d %%d in ("%ProgramFiles%\Blender Foundation\Blender*") do if exist "%%d\blender.exe" set "BLENDER=%%d\blender.exe"
)
if not defined BLENDER (
  echo Blender wurde nicht gefunden. Bitte installieren: winget install BlenderFoundation.Blender
  pause
  exit /b 1
)
set FEHLER=0
if "%~1"=="" (
  for /r "art\modelle" %%f in (*.py *.blend) do call :bauen "%%f"
) else (
  for %%f in (%*) do call :bauen "%%~ff"
)
echo.
if !FEHLER!==0 (echo Alles gebaut.) else (echo Es gab Fehler, siehe oben.)
pause
exit /b !FEHLER!

:bauen
echo Baue %~nx1 ...
"%BLENDER%" --background --factory-startup --python-exit-code 1 --python art\lib\bauen.py -- "%~f1" > "%TEMP%\asset_bau.log" 2>&1
if errorlevel 1 (
  set FEHLER=1
  echo FEHLER bei %~nx1:
  type "%TEMP%\asset_bau.log"
) else (
  findstr /b "FERTIG" "%TEMP%\asset_bau.log"
)
exit /b 0
