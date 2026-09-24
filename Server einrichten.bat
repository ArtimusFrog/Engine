@echo off
setlocal
rem Einmalig: richtet den Linux-Server ein (Pakete, Firewall, Dienst) und startet das Spiel dort.
rem Voraussetzung: deploy\server.txt enthaelt die Adresse, z. B. root@85.215.123.45
rem und der SSH-Schluessel ist hinterlegt (siehe deploy\README.md).
cd /d "%~dp0"
if not exist "deploy\server.txt" (
  echo Bitte zuerst deploy\server.txt anlegen, Inhalt z. B.: root@85.215.123.45
  pause
  exit /b 1
)
set /p SERVER=<deploy\server.txt
echo Lade den aktuellen Stand hoch nach %SERVER% ...
git archive --format=tar -o "%TEMP%\spiel_quelle.tar" HEAD || goto fehler
scp "%TEMP%\spiel_quelle.tar" %SERVER%:/tmp/spiel_quelle.tar || goto fehler
ssh %SERVER% "rm -rf /opt/spiel/src && mkdir -p /opt/spiel/src && tar -xf /tmp/spiel_quelle.tar -C /opt/spiel/src && bash /opt/spiel/src/deploy/linux/einrichten.sh" || goto fehler
echo.
echo Fertig!
pause
exit /b 0
:fehler
echo.
echo Es ist ein Fehler aufgetreten, siehe oben.
pause
exit /b 1
