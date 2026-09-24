@echo off
rem Veroeffentlicht eine neue Version fuer Windows und Mac: bauen, signieren, hochladen,
rem Spielserver aktualisieren. Einzelheiten in deploy\release.ps1.
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File deploy\release.ps1 %*
pause
