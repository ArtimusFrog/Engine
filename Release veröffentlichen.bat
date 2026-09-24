@echo off
setlocal
rem Veröffentlicht eine neue Version: bauen, signieren, auf die Webseite laden, Spielserver aktualisieren.
rem Alle Spieler bekommen sie beim nächsten Start des Launchers automatisch.
cd /d "%~dp0"
set /p VERSION=Neue Versionsnummer (z. B. 0.3.0): 
if "%VERSION%"=="" goto fehler
echo Neuigkeiten fuer den Launcher (leer lassen zum Beenden):
set NEWS=
:frage
set ZEILE=
set /p ZEILE=  - 
if not "%ZEILE%"=="" (
  set NEWS=%NEWS% --neu "%ZEILE%"
  goto frage
)
echo.
echo == Bauen ==
cargo build --release -p game -p launcher || goto fehler
echo == Release zusammenstellen und signieren ==
cargo run --release -q -p launcher --bin release -- bauen --version %VERSION% %NEWS% || goto fehler
echo == Hochladen ==
node deploy\web\hochladen.mjs || goto fehler
echo == Spielserver aktualisieren ==
call "Server aktualisieren.bat"
echo.
echo Version %VERSION% ist veroeffentlicht.
pause
exit /b 0
:fehler
echo.
echo Abgebrochen - siehe Fehlermeldung oben.
pause
exit /b 1
