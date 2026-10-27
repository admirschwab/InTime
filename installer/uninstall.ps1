<#
.SYNOPSIS
    Entfernt InTime vom Computer.

.DESCRIPTION
    Beendet InTime, entfernt Programm, Verknüpfungen, Autostart und den Eintrag unter
    "Installierte Apps". Deine Daten (Projekte und Zeiten) werden nur gelöscht, wenn du
    zustimmst oder -RemoveData angibst.

.PARAMETER RemoveData
    Auch alle Daten löschen (Datenbank, Logs, Cache) - ohne Nachfrage.

.PARAMETER KeepData
    Daten behalten - ohne Nachfrage.

.PARAMETER Silent
    Keine Rückfragen. Ohne -RemoveData bleiben die Daten erhalten.

.EXAMPLE
    .\uninstall.ps1
    .\uninstall.ps1 -Silent -RemoveData
#>
[CmdletBinding()]
param(
    [switch]$RemoveData,
    [switch]$KeepData,
    [switch]$Silent
)

$ErrorActionPreference = 'Stop'

$InstallDir   = Join-Path $env:LOCALAPPDATA 'InTime'
$StartMenuDir = Join-Path ([Environment]::GetFolderPath('Programs')) 'InTime'
$DesktopLink  = Join-Path ([Environment]::GetFolderPath('Desktop')) 'InTime.lnk'
$StartupLink  = Join-Path ([Environment]::GetFolderPath('Startup')) 'InTime.lnk'
$UninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\InTime'
# Data locations of InTime 0.1.0 (before everything moved to %LOCALAPPDATA%\InTime).
$LegacyDirs   = @(
    (Join-Path $env:APPDATA 'app.intime.desktop'),
    (Join-Path $env:LOCALAPPDATA 'app.intime.desktop')
)

function Confirm-Choice([string]$Question, [bool]$Default) {
    if ($Silent) { return $Default }
    $hint = if ($Default) { 'J/n' } else { 'j/N' }
    $answer = Read-Host "$Question [$hint]"
    if ([string]::IsNullOrWhiteSpace($answer)) { return $Default }
    return $answer.Trim() -match '^(j|ja|y|yes)$'
}

function Remove-IfExists([string]$Path) {
    if (Test-Path $Path) { Remove-Item $Path -Recurse -Force }
}

Write-Host ''
Write-Host '  InTime - Deinstallation' -ForegroundColor Cyan
Write-Host ''

if (-not (Confirm-Choice 'InTime wirklich deinstallieren?' $true)) {
    Write-Host 'Abgebrochen.'
    exit 1
}

$deleteData = $RemoveData -or (-not $KeepData -and -not $Silent -and
    (Confirm-Choice 'Auch alle Daten (Projekte, Zeiten, Einstellungen) löschen? Tipp: Vorher in InTime unter Einstellungen > Daten exportieren.' $false))

# The script itself lives in the install folder; don't hold it as working directory.
Set-Location $env:TEMP

$processes = Get-Process -Name 'InTime' -ErrorAction SilentlyContinue
if ($processes) {
    # Close normally first so the running entry is saved, then force.
    $processes | ForEach-Object { [void]$_.CloseMainWindow() }
    $processes | Wait-Process -Timeout 5 -ErrorAction SilentlyContinue
    Get-Process -Name 'InTime' -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Milliseconds 500
    Write-Host '  [OK] InTime beendet'
}

Remove-IfExists $StartMenuDir
Remove-IfExists $DesktopLink
Remove-IfExists $StartupLink
Write-Host '  [OK] Verknüpfungen und Autostart entfernt'

Remove-IfExists $UninstallKey
Write-Host '  [OK] Eintrag unter "Installierte Apps" entfernt'

foreach ($file in 'InTime.exe', 'uninstall.ps1', 'install.json') {
    Remove-IfExists (Join-Path $InstallDir $file)
}
Remove-IfExists (Join-Path $InstallDir 'webview')
Write-Host '  [OK] Programm entfernt'

if ($deleteData) {
    Remove-IfExists $InstallDir
    $LegacyDirs | ForEach-Object { Remove-IfExists $_ }
    Write-Host '  [OK] Alle Daten gelöscht'
} else {
    Write-Host "  [i] Deine Daten liegen weiterhin in $InstallDir\data"
    Write-Host '    Bei einer erneuten Installation werden sie automatisch wieder verwendet.'
}

Write-Host ''
Write-Host '  InTime wurde deinstalliert.' -ForegroundColor Green
Write-Host ''
if (-not $Silent) { Read-Host 'Zum Schließen Enter drücken' | Out-Null }
