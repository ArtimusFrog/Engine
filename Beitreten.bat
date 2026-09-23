@echo off
rem Mit einem Host oder Server verbinden
set /p ADRESSE=Adresse des Servers (IP oder Name, z. B. 100.64.1.2):
"%~dp0target\release\game.exe" --join %ADRESSE%
if errorlevel 1 pause
