# Veröffentlicht eine neue Version von Engine JN für Windows UND Mac in einem Durchgang:
#   1. auf GitHub pushen und dort den Mac-Build starten
#   2. währenddessen Windows hier bauen
#   3. Mac-Ergebnis herunterladen und HIER signieren (der geheime Schlüssel bleibt auf diesem PC)
#   4. alles auf die Webseite laden, Spielserver aktualisieren
# Spieler bekommen die neue Version beim nächsten Start ihres Launchers automatisch.
#
# Aufruf: Doppelklick auf "Release veröffentlichen.bat" (fragt Version und Neuigkeiten ab)
#     oder: powershell -File deploy\release.ps1 -Version 0.3.1 -Neu "Punkt 1","Punkt 2" [-OhneMac]
param(
    [string]$Version,
    [string[]]$Neu = @(),
    [switch]$OhneMac
)
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"

function Schritt($text) { Write-Host "`n== $text ==" -ForegroundColor Yellow }
function Fehler($text) { Write-Host "`nABGEBROCHEN: $text" -ForegroundColor Red; exit 1 }
function Pruefe($was) { if ($LASTEXITCODE -ne 0) { Fehler "$was ist fehlgeschlagen (siehe oben)." } }

# GitHub-Werkzeug finden
$gh = Get-Command gh -ErrorAction SilentlyContinue
$gh = if ($gh) { $gh.Source } else { Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Packages\GitHub.cli_Microsoft.Winget.Source_8wekyb3d8bbwe\bin\gh.exe' }
if (-not $OhneMac -and -not (Test-Path $gh)) { Fehler "GitHub-Werkzeug 'gh' fehlt (winget install GitHub.cli)." }

if (-not $Version) {
    $Version = Read-Host 'Neue Versionsnummer (z. B. 0.3.1)'
    if (-not $Version) { Fehler 'Keine Version angegeben.' }
    Write-Host 'Neuigkeiten für den Launcher, eine pro Zeile (leer lassen zum Beenden):'
    while ($true) {
        $zeile = Read-Host '  -'
        if (-not $zeile) { break }
        $Neu += $zeile
    }
}

# Alles muss committet sein: Server und Mac werden aus dem Commit gebaut, Windows aus dem Ordner.
if (git status --porcelain) { Fehler 'Es gibt nicht committete Änderungen. Erst committen (z. B. in GitHub Desktop), dann erneut starten.' }
$commit = (git rev-parse HEAD).Trim()

$lauf = $null
if (-not $OhneMac) {
    Schritt 'Auf GitHub hochladen und Mac-Build starten'
    & $gh auth status *> $null
    if ($LASTEXITCODE -ne 0) { Fehler "Nicht bei GitHub angemeldet. Einmalig im Terminal: gh auth login" }
    & $gh auth setup-git | Out-Null
    git push origin HEAD:main; Pruefe 'git push'
    $start = (Get-Date).ToUniversalTime()
    & $gh workflow run mac.yml --ref main; Pruefe 'Mac-Build starten'
    # Warten, bis GitHub den Lauf für genau diesen Commit anzeigt.
    for ($i = 0; $i -lt 30 -and -not $lauf; $i++) {
        Start-Sleep -Seconds 4
        $liste = & $gh run list --workflow mac.yml --commit $commit --limit 5 --json databaseId,createdAt | ConvertFrom-Json
        $lauf = $liste | Where-Object { ([datetime]$_.createdAt).ToUniversalTime() -ge $start.AddSeconds(-30) } | Select-Object -First 1
    }
    if (-not $lauf) { Fehler 'Der Mac-Build ist auf GitHub nicht aufgetaucht.' }
    Write-Host "Mac-Build läuft auf GitHub (Lauf $($lauf.databaseId)) – Windows wird parallel gebaut."
}

Schritt 'Windows bauen'
cargo build --release -p game -p launcher; Pruefe 'cargo build'
$argumente = @('bauen', '--version', $Version)
foreach ($n in $Neu) { $argumente += @('--neu', $n) }
cargo run --release -q -p launcher --bin release -- @argumente; Pruefe 'Release zusammenstellen'

if ($lauf) {
    Schritt 'Auf den Mac-Build warten'
    & $gh run watch $lauf.databaseId --exit-status --interval 20; Pruefe 'Mac-Build'
    $ziel = 'dist\mac'
    if (Test-Path $ziel) { Remove-Item -Recurse -Force $ziel }
    & $gh run download $lauf.databaseId -n engine-jn-macos -D $ziel; Pruefe 'Mac-Ergebnis herunterladen'
    Schritt 'Mac-Version signieren'
    cargo run --release -q -p launcher --bin release -- mac --von $ziel; Pruefe 'Mac-Version hinzufügen'
}

Schritt 'Serverliste für den Serverbrowser'
$serverHost = ((Get-Content deploy\server.txt -TotalCount 1).Trim() -split '@')[-1]
$liste = '[{"name": "Offizieller Server", "adresse": "' + $serverHost + ':7777"}]'
Set-Content -Path dist\web\server.json -Value $liste -Encoding ascii

Schritt 'Auf die Webseite hochladen'
node deploy\web\hochladen.mjs; Pruefe 'Hochladen'

Schritt 'Spielserver aktualisieren'
$server = (Get-Content deploy\server.txt -TotalCount 1).Trim()
$archiv = Join-Path $env:TEMP 'spiel_quelle.tar'
git archive --format=tar -o $archiv HEAD; Pruefe 'git archive'
scp -q -o BatchMode=yes $archiv "${server}:/tmp/spiel_quelle.tar"; Pruefe 'Hochladen zum Server'
ssh -o BatchMode=yes $server 'rm -rf /opt/spiel/src && mkdir -p /opt/spiel/src && tar -xf /tmp/spiel_quelle.tar -C /opt/spiel/src && bash /opt/spiel/src/deploy/linux/bauen_und_starten.sh'
Pruefe 'Server aktualisieren'

$plattformen = if ($lauf) { 'Windows und Mac' } else { 'Windows' }
Write-Host "`nVersion $Version ist veröffentlicht ($plattformen). Spieler bekommen sie beim nächsten Start des Launchers." -ForegroundColor Green
