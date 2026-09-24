@echo off
rem Asset im Betrachter oeffnen: Datei (.gltf, .glb, oder Blender-Quelle .py/.blend aus art\modelle)
rem auf diese Datei ziehen. Blender-Quellen werden bei jeder Aenderung automatisch neu gebaut.
if "%~1"=="" (
  echo Ziehe ein Modell oder ein Skript aus art\modelle auf diese Datei.
  pause
  exit /b 1
)
start "" "%~dp0target\release\game.exe" --ansehen "%~f1"
