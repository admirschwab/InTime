<#
.SYNOPSIS
    Removes InTime from this computer.

.DESCRIPTION
    Closes InTime and removes the program, shortcuts, autostart and the entry under
    "Installed apps". Your data (projects and times) is only deleted if you agree or
    pass -RemoveData.

    Uninstall:
        irm https://github.com/admirschwab/InTime/releases/latest/download/uninstall.ps1 | iex

    With options:
        & ([scriptblock]::Create((irm https://github.com/admirschwab/InTime/releases/latest/download/uninstall.ps1))) -Silent -RemoveData

    The same file is also installed as %LOCALAPPDATA%\InTime\uninstall.ps1 and used by the
    Start menu shortcut and "Installed apps".

    NOTE: This file must stay pure ASCII (no umlauts, no BOM). Windows PowerShell 5.1 does
    not decode downloaded scripts as UTF-8, so any non-ASCII character would break "irm | iex".

.PARAMETER RemoveData
    Also delete all data (database, logs, cache) without asking.

.PARAMETER KeepData
    Keep the data without asking.

.PARAMETER Silent
    No prompts. Without -RemoveData the data is kept.
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
    $answer = Read-Host "  $Question [$hint]"
    if ([string]::IsNullOrWhiteSpace($answer)) { return $Default }
    return $answer.Trim() -match '^(j|ja|y|yes)$'
}

function Remove-IfExists([string]$Path) {
    if (Test-Path $Path) { Remove-Item $Path -Recurse -Force }
}

# Everything runs inside a function so that "return" works when the script is piped into
# "iex" ("exit" would close the user's PowerShell window).
function Uninstall-InTime {
    Write-Host ''
    Write-Host '  InTime - Deinstallation' -ForegroundColor Cyan
    Write-Host ''

    $installed = (Test-Path (Join-Path $InstallDir 'InTime.exe')) -or (Test-Path $UninstallKey)
    if (-not $installed) {
        Write-Host '  InTime ist nicht installiert.'
        Write-Host ''
        return
    }

    if (-not (Confirm-Choice 'InTime wirklich deinstallieren?' $true)) {
        Write-Host '  Abgebrochen.'
        return
    }

    $deleteData = $RemoveData -or (-not $KeepData -and -not $Silent -and
        (Confirm-Choice 'Auch alle Daten (Projekte, Zeiten, Einstellungen) loeschen? Tipp: Vorher in InTime unter Einstellungen > Daten exportieren.' $false))

    # When started from the install folder, don't keep it as working directory.
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
    Write-Host '  [OK] Verknuepfungen und Autostart entfernt'

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
        Write-Host '  [OK] Alle Daten geloescht'
    } else {
        Write-Host "  [i] Deine Daten liegen weiterhin in $InstallDir\data"
        Write-Host '      Bei einer erneuten Installation werden sie automatisch wieder verwendet.'
    }

    Write-Host ''
    Write-Host '  InTime wurde deinstalliert.' -ForegroundColor Green
    Write-Host ''
    # Started from the Start menu the window would close immediately; with "irm | iex" the
    # console stays open anyway ($PSCommandPath is empty then).
    if (-not $Silent -and $PSCommandPath) { Read-Host '  Zum Schliessen Enter druecken' | Out-Null }
}

Uninstall-InTime
