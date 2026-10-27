<#
.SYNOPSIS
    Installs or updates InTime for the current user in %LOCALAPPDATA%\InTime.

.DESCRIPTION
    Downloads InTime from the GitHub Releases, copies it to %LOCALAPPDATA%\InTime, creates
    Start menu shortcuts and registers InTime under "Installed apps". No admin rights needed.
    An existing installation is updated; your data is kept.

    Install the latest version:
        irm https://github.com/admirschwab/InTime/releases/latest/download/install.ps1 | iex

    With options:
        & ([scriptblock]::Create((irm https://github.com/admirschwab/InTime/releases/latest/download/install.ps1))) -Silent -Autostart

    NOTE: This file must stay pure ASCII (no umlauts, no BOM). Windows PowerShell 5.1 does
    not decode downloaded scripts as UTF-8, so any non-ASCII character would break "irm | iex".

.PARAMETER Version
    Install a specific version (e.g. 0.1.1) instead of the latest.

.PARAMETER ZipPath
    Install from a local InTime-windows.zip instead of downloading it.

.PARAMETER Autostart
    Start InTime with Windows (no prompt).

.PARAMETER DesktopShortcut
    Create a desktop shortcut (no prompt).

.PARAMETER Silent
    No prompts; only the options given as switches are applied.
#>
[CmdletBinding()]
param(
    [string]$Version,
    [string]$ZipPath,
    [switch]$Autostart,
    [switch]$DesktopShortcut,
    [switch]$Silent
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'  # the progress bar makes downloads very slow in PowerShell 5.1

$Repo         = 'admirschwab/InTime'
$InstallDir   = Join-Path $env:LOCALAPPDATA 'InTime'
$TargetExe    = Join-Path $InstallDir 'InTime.exe'
$StartMenuDir = Join-Path ([Environment]::GetFolderPath('Programs')) 'InTime'
$DesktopLink  = Join-Path ([Environment]::GetFolderPath('Desktop')) 'InTime.lnk'
$StartupLink  = Join-Path ([Environment]::GetFolderPath('Startup')) 'InTime.lnk'
$UninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\InTime'
$WebView2Id   = '{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'

function Confirm-Choice([string]$Question, [bool]$Default) {
    if ($Silent) { return $Default }
    $hint = if ($Default) { 'J/n' } else { 'j/N' }
    $answer = Read-Host "  $Question [$hint]"
    if ([string]::IsNullOrWhiteSpace($answer)) { return $Default }
    return $answer.Trim() -match '^(j|ja|y|yes)$'
}

function New-Shortcut([string]$Path, [string]$Target, [string]$Arguments = '', [string]$Description = 'InTime - Zeiterfassung') {
    $shortcut = (New-Object -ComObject WScript.Shell).CreateShortcut($Path)
    $shortcut.TargetPath = $Target
    $shortcut.Arguments = $Arguments
    $shortcut.WorkingDirectory = $InstallDir
    $shortcut.IconLocation = "$TargetExe,0"
    $shortcut.Description = $Description
    $shortcut.Save()
}

function Stop-InTime {
    $processes = Get-Process -Name 'InTime' -ErrorAction SilentlyContinue
    if (-not $processes) { return $true }
    if (-not (Confirm-Choice 'InTime laeuft gerade und wird fuer die Installation beendet. Fortfahren?' $true)) {
        return $false
    }
    # Close normally first so the running entry is saved, then force.
    $processes | ForEach-Object { [void]$_.CloseMainWindow() }
    $processes | Wait-Process -Timeout 5 -ErrorAction SilentlyContinue
    Get-Process -Name 'InTime' -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Milliseconds 500
    return $true
}

# Everything runs inside this function so that "return" ends the installation without
# closing the user's PowerShell window (which "exit" would do under "irm | iex").
function Install-InTime {
    Write-Host ''
    Write-Host '  InTime - Installation' -ForegroundColor Cyan
    Write-Host "  Ziel: $InstallDir"
    Write-Host ''

    $hasWebView2 = @(
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\$WebView2Id",
        "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\$WebView2Id",
        "HKCU:\Software\Microsoft\EdgeUpdate\Clients\$WebView2Id"
    ) | Where-Object { Test-Path $_ }
    if (-not $hasWebView2) {
        Write-Warning 'Microsoft Edge WebView2 wurde nicht gefunden. InTime braucht es fuer die Oberflaeche.'
        Write-Warning 'Download: https://go.microsoft.com/fwlink/p/?LinkId=2124703'
    }

    # ------------------------------------------------------------ download

    $work = Join-Path ([IO.Path]::GetTempPath()) "InTime-install-$([guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Force -Path $work | Out-Null
    try {
        if ($ZipPath) {
            $zip = (Resolve-Path $ZipPath).Path
        } else {
            $zip = Join-Path $work 'InTime-windows.zip'
            $url = if ($Version) {
                "https://github.com/$Repo/releases/download/v$($Version.TrimStart('v'))/InTime-windows.zip"
            } else {
                "https://github.com/$Repo/releases/latest/download/InTime-windows.zip"
            }
            Write-Host "  Lade herunter: $url"
            [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
            try {
                Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
            } catch {
                throw "Download fehlgeschlagen ($url). Gibt es schon ein Release auf GitHub? Details: $($_.Exception.Message)"
            }
        }

        $package = Join-Path $work 'package'
        Expand-Archive -Path $zip -DestinationPath $package -Force
        if (-not (Test-Path (Join-Path $package 'InTime.exe'))) {
            throw 'Das Paket enthaelt keine InTime.exe.'
        }
        $newVersion = (Get-Content (Join-Path $package 'version.txt') -Raw).Trim()
        Write-Host "  [OK] InTime $newVersion heruntergeladen"
        Write-Host ''

        # -------------------------------------------------------- options

        $isUpdate = Test-Path $TargetExe
        # With -Silent only the options given as switches are applied.
        $wantDesktop = $DesktopShortcut -or (-not $Silent -and (Confirm-Choice 'Verknuepfung auf dem Desktop anlegen?' $false))
        $wantAutostart = $Autostart -or (-not $Silent -and (Confirm-Choice 'InTime beim Windows-Start automatisch oeffnen?' $true))
        Write-Host ''

        if (-not (Stop-InTime)) {
            Write-Host '  Installation abgebrochen.'
            return
        }

        # -------------------------------------------------------- install

        New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
        Copy-Item (Join-Path $package 'InTime.exe') $TargetExe -Force
        Copy-Item (Join-Path $package 'uninstall.ps1') (Join-Path $InstallDir 'uninstall.ps1') -Force
        # Downloaded files carry the "from the internet" mark; the installed copies don't need it.
        Get-ChildItem $InstallDir -File | Unblock-File
        Write-Host '  [OK] Programm installiert'

        $uninstallCommand = 'powershell.exe'
        $uninstallArgs = "-NoProfile -ExecutionPolicy Bypass -File `"$InstallDir\uninstall.ps1`""

        New-Item -ItemType Directory -Force -Path $StartMenuDir | Out-Null
        New-Shortcut (Join-Path $StartMenuDir 'InTime.lnk') $TargetExe
        New-Shortcut (Join-Path $StartMenuDir 'InTime deinstallieren.lnk') $uninstallCommand $uninstallArgs 'InTime deinstallieren'
        Write-Host '  [OK] Startmenue-Eintraege angelegt'

        if ($wantDesktop) {
            New-Shortcut $DesktopLink $TargetExe
            Write-Host '  [OK] Desktop-Verknuepfung angelegt'
        } elseif (Test-Path $DesktopLink) {
            Remove-Item $DesktopLink -Force
        }

        if ($wantAutostart) {
            New-Shortcut $StartupLink $TargetExe
            Write-Host '  [OK] Autostart eingerichtet'
        } elseif (Test-Path $StartupLink) {
            Remove-Item $StartupLink -Force
        }

        # Entry under Settings > Apps > Installed apps.
        $sizeKb = [int]((Get-ChildItem $InstallDir -File | Measure-Object Length -Sum).Sum / 1KB)
        New-Item -Path $UninstallKey -Force | Out-Null
        $entry = @{
            DisplayName     = 'InTime'
            DisplayVersion  = $newVersion
            Publisher       = 'Admir Schwab'
            DisplayIcon     = "$TargetExe,0"
            InstallLocation = $InstallDir
            InstallDate     = (Get-Date -Format 'yyyyMMdd')
            UninstallString = "$uninstallCommand $uninstallArgs"
            URLInfoAbout    = "https://github.com/$Repo"
        }
        foreach ($name in $entry.Keys) { Set-ItemProperty -Path $UninstallKey -Name $name -Value $entry[$name] }
        Set-ItemProperty -Path $UninstallKey -Name 'NoModify' -Value 1 -Type DWord
        Set-ItemProperty -Path $UninstallKey -Name 'NoRepair' -Value 1 -Type DWord
        Set-ItemProperty -Path $UninstallKey -Name 'EstimatedSize' -Value $sizeKb -Type DWord
        Write-Host '  [OK] Unter "Installierte Apps" eingetragen'

        # Installation details, read by the uninstaller and handy for support.
        [ordered]@{
            name        = 'InTime'
            version     = $newVersion
            installedAt = (Get-Date -Format 'o')
            source      = if ($ZipPath) { $zip } else { $url }
            installDir  = $InstallDir
            dataDir     = Join-Path $InstallDir 'data'
            logsDir     = Join-Path $InstallDir 'logs'
            shortcuts   = [ordered]@{
                startMenu = $StartMenuDir
                desktop   = if ($wantDesktop) { $DesktopLink } else { $null }
                autostart = if ($wantAutostart) { $StartupLink } else { $null }
            }
        } | ConvertTo-Json | Set-Content -Path (Join-Path $InstallDir 'install.json') -Encoding UTF8

        Write-Host ''
        if ($isUpdate) {
            Write-Host "  InTime wurde auf Version $newVersion aktualisiert. Deine Daten sind erhalten geblieben." -ForegroundColor Green
        } else {
            Write-Host "  InTime $newVersion wurde installiert. Du findest es im Startmenue." -ForegroundColor Green
        }
        Write-Host ''

        if (-not $Silent -and (Confirm-Choice 'InTime jetzt starten?' $true)) {
            Start-Process -FilePath $TargetExe -WorkingDirectory $InstallDir
        }
    } finally {
        Remove-Item $work -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Install-InTime
