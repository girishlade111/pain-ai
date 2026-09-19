<#
.SYNOPSIS
    pain ai (lsc) — Windows Bootstrap & Installation Script
.DESCRIPTION
    Installs pain ai desktop agent, provisions MinGit if needed, configures User PATH,
    provides Antivirus false-positive mitigations, and runs `lsc doctor`.
#>

[CmdletBinding()]
param (
    [string]$Version = "1.0.0",
    [switch]$SkipDoctor = $false
)

$ErrorActionPreference = "Stop"

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host " pain ai (lsc) Windows Installer — v$Version" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

# 1. Architecture Check (x64 only in v1)
$arch = [System.Environment]::GetEnvironmentVariable("PROCESSOR_ARCHITECTURE")
if ($arch -ne "AMD64") {
    Write-Host "[ERROR] pain ai v1.0 only supports Windows x64 (AMD64)." -ForegroundColor Red
    Write-Host "[INFO] Detected architecture: $arch. ARM64 support is scheduled for v2.0." -ForegroundColor Yellow
    exit 1
}

# 2. Setup Local AppData Directory Structure
$painHome = Join-Path $env:LOCALAPPDATA "pain-ai"
$binDir = Join-Path $painHome "bin"
$gitDir = Join-Path $painHome "git"

New-Item -ItemType Directory -Force -Path $painHome | Out-Null
New-Item -ItemType Directory -Force -Path $binDir | Out-Null

Write-Host "[INFO] pain ai Home: $painHome" -ForegroundColor Gray

# 3. MinGit Provisioning (if git is missing)
$gitCmd = Get-Command "git" -ErrorAction SilentlyContinue
if (-not $gitCmd) {
    Write-Host "[INFO] Git not found on system PATH. Provisioning MinGit..." -ForegroundColor Yellow
    $minGitUrl = "https://github.com/git-for-windows/git/releases/download/v2.44.0.windows.1/MinGit-2.44.0-64-bit.zip"
    $minGitZip = Join-Path $painHome "mingit.zip"
    
    try {
        Invoke-WebRequest -Uri $minGitUrl -OutFile $minGitZip -UseBasicParsing
        Expand-Archive -Path $minGitZip -DestinationPath $gitDir -Force
        Remove-Item -Path $minGitZip -Force
        Write-Host "[INFO] MinGit provisioned at $gitDir" -ForegroundColor Green
    } catch {
        Write-Host "[WARN] MinGit download failed (offline mode). Git features may require manual installation." -ForegroundColor Yellow
    }
} else {
    Write-Host "[INFO] Git detected: $($gitCmd.Source)" -ForegroundColor Gray
}

# 4. NSIS Package Installation (/S Silent Mode)
$nsisInstaller = "pain-ai_${Version}_x64-setup.exe"
$localNsis = Join-Path $PSScriptRoot "..\src-tauri\target\release\bundle\nsis\$nsisInstaller"

if (Test-Path $localNsis) {
    Write-Host "[INFO] Installing local NSIS package silently: $localNsis" -ForegroundColor Green
    Start-Process -FilePath $localNsis -ArgumentList "/S" -Wait
} else {
    Write-Host "[INFO] Fetching release artifact from GitHub: $nsisInstaller..." -ForegroundColor Gray
    $releaseUrl = "https://github.com/ladestack/pain-ai/releases/download/v$Version/$nsisInstaller"
    $tempInstaller = Join-Path $env:TEMP $nsisInstaller
    try {
        Invoke-WebRequest -Uri $releaseUrl -OutFile $tempInstaller -UseBasicParsing
        Start-Process -FilePath $tempInstaller -ArgumentList "/S" -Wait
        Remove-Item -Path $tempInstaller -Force
        Write-Host "[INFO] Silent NSIS installation complete." -ForegroundColor Green
    } catch {
        Write-Host "[WARN] GitHub download unavailable. Using existing application binary." -ForegroundColor Yellow
    }
}

# 5. User PATH Registration
$userPath = [System.Environment]::GetEnvironmentVariable("PATH", "User")
if ($userPath -notlike "*$binDir*") {
    $newPath = "$binDir;$userPath"
    [System.Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
    $env:PATH = "$binDir;$env:PATH"
    Write-Host "[INFO] Added $binDir to User PATH." -ForegroundColor Green
}

# 6. Antivirus & Defender Guidance (AV false positive notes)
Write-Host ""
Write-Host "------------------------------------------------------------" -ForegroundColor Cyan
Write-Host " Antivirus & Windows Defender Guidance" -ForegroundColor Cyan
Write-Host "------------------------------------------------------------" -ForegroundColor Cyan
Write-Host "Note: PyInstaller sidecar binaries and local Python engines may occasionally" -ForegroundColor Gray
Write-Host "trigger false-positive heuristic alerts from Windows Defender or third-party AV." -ForegroundColor Gray
Write-Host "If the sidecar or engine fails to start, add an exclusion via PowerShell (Admin):" -ForegroundColor Gray
Write-Host "  Add-MpPreference -ExclusionPath '$painHome'" -ForegroundColor Yellow
Write-Host "------------------------------------------------------------" -ForegroundColor Cyan

# 7. Run Doctor Post-Install
if (-not $SkipDoctor) {
    Write-Host ""
    Write-Host "============================================================" -ForegroundColor Cyan
    Write-Host " Running Diagnostic Health Checks (lsc doctor)" -ForegroundColor Cyan
    Write-Host "============================================================" -ForegroundColor Cyan

    $lscScript = Join-Path $PSScriptRoot "..\bin\lsc.js"
    if (Test-Path $lscScript) {
        node $lscScript doctor
    } else {
        $lscExe = Get-Command "lsc" -ErrorAction SilentlyContinue
        if ($lscExe) {
            & $lscExe doctor
        } else {
            Write-Host "[INFO] Installation complete. Run 'lsc doctor' to verify." -ForegroundColor Green
        }
    }
}

Write-Host ""
Write-Host "[SUCCESS] pain ai installation and configuration complete!" -ForegroundColor Green
