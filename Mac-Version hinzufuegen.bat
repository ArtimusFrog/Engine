@echo off
setlocal
rem Fuegt die Mac-Version zum aktuellen Release hinzu und laedt alles hoch.
rem 1. Auf GitHub unter Actions den Workflow "Mac bauen" starten (Run workflow).
rem 2. Wenn er fertig ist, unter "Artifacts" die Datei engine-jn-macos herunterladen.
rem 3. Die heruntergeladene ZIP-Datei auf diese Datei ziehen.
rem Vorher muss "Release veroeffentlichen.bat" fuer dieselbe Version gelaufen sein.
cd /d "%~dp0"
if "%~1"=="" (
  echo Ziehe die heruntergeladene engine-jn-macos.zip auf diese Datei.
  pause
  exit /b 1
)
if exist "dist\mac" rmdir /s /q "dist\mac"
powershell -NoProfile -Command "Expand-Archive -LiteralPath '%~f1' -DestinationPath 'dist\mac' -Force" || goto fehler
cargo run --release -q -p launcher --bin release -- mac --von dist\mac || goto fehler
node deploy\web\hochladen.mjs || goto fehler
echo.
echo Die Mac-Version ist online.
pause
exit /b 0
:fehler
echo.
echo Abgebrochen - siehe Fehlermeldung oben.
pause
exit /b 1
