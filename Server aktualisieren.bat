@echo off
setlocal
rem Spielt den aktuellen Stand (letzter Commit) auf den Server und startet ihn neu.
rem Die Welt wird dabei gespeichert und bleibt erhalten.
cd /d "%~dp0"
if not exist "deploy\server.txt" (
  echo Bitte zuerst deploy\server.txt anlegen, Inhalt z. B.: root@85.215.123.45
  pause
  exit /b 1
)
set /p SERVER=<deploy\server.txt
git diff --quiet HEAD || echo Hinweis: Nicht committete Aenderungen werden NICHT hochgeladen.
git archive --format=tar -o "%TEMP%\spiel_quelle.tar" HEAD || goto fehler
scp "%TEMP%\spiel_quelle.tar" %SERVER%:/tmp/spiel_quelle.tar || goto fehler
ssh %SERVER% "rm -rf /opt/spiel/src && mkdir -p /opt/spiel/src && tar -xf /tmp/spiel_quelle.tar -C /opt/spiel/src && bash /opt/spiel/src/deploy/linux/bauen_und_starten.sh" || goto fehler
echo.
echo Server laeuft mit dem neuen Stand.
pause
exit /b 0
:fehler
echo.
echo Es ist ein Fehler aufgetreten, siehe oben.
pause
exit /b 1
