# Tyr Installer for Windows
# Usage: irm https://raw.githubusercontent.com/nurysso/tyr/main/install.ps1 | iex
# script by claude
param(
    [string]$Version = "latest"
)

$ErrorActionPreference = "Stop"

# Configuration
$REPO = "nurysso/tyr"
$BINARY_NAME = "tyr"
$INSTALL_DIR = "$env:LOCALAPPDATA\Programs\tyr"
$CONFIG_DIR = "$env:APPDATA\tyr"

Write-Host ""
Write-Host "Tyr Installer for Windows" -ForegroundColor Cyan
Write-Host "===========================" -ForegroundColor Cyan
Write-Host ""

# Detect architecture
$arch = if ([Environment]::Is64BitOperatingSystem) {
    if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") {
        "aarch64"
    } else {
        "x86_64"
    }
} else {
    Write-Host "Error: 32-bit Windows is not supported" -ForegroundColor Red
    exit 1
}

$ASSET_NAME = "$BINARY_NAME-windows-$arch.exe"
Write-Host "Detected architecture: $arch" -ForegroundColor Green

try {
    # Get download URL
    Write-Host "Fetching latest release..." -ForegroundColor Yellow

    if ($Version -eq "latest") {
        $releaseUrl = "https://api.github.com/repos/$REPO/releases/latest"
    } else {
        $releaseUrl = "https://api.github.com/repos/$REPO/releases/tags/$Version"
    }

    $release = Invoke-RestMethod -Uri $releaseUrl -Headers @{
        "User-Agent" = "Tyr-Installer"
    }

    $asset = $release.assets | Where-Object { $_.name -eq $ASSET_NAME }

    if (-not $asset) {
        Write-Host "Error: Could not find release asset: $ASSET_NAME" -ForegroundColor Red
        Write-Host "Available assets:" -ForegroundColor Yellow
        $release.assets | ForEach-Object { Write-Host "  - $($_.name)" }
        exit 1
    }

    $downloadUrl = $asset.browser_download_url
    Write-Host "Found: $($release.tag_name)" -ForegroundColor Green

    # Create directories
    Write-Host "Creating directories..." -ForegroundColor Yellow
    New-Item -ItemType Directory -Path $INSTALL_DIR -Force | Out-Null
    New-Item -ItemType Directory -Path $CONFIG_DIR -Force | Out-Null

    # Download binary
    $tempFile = Join-Path $env:TEMP "$BINARY_NAME.exe"
    Write-Host "Downloading $BINARY_NAME..." -ForegroundColor Yellow

    Invoke-WebRequest -Uri $downloadUrl -OutFile $tempFile -UseBasicParsing

    # Install binary
    $binaryPath = Join-Path $INSTALL_DIR "$BINARY_NAME.exe"
    Move-Item -Path $tempFile -Destination $binaryPath -Force

    Write-Host "✓ Installed to: $binaryPath" -ForegroundColor Green
    Write-Host "✓ Config directory: $CONFIG_DIR" -ForegroundColor Green
    Write-Host ""

    # Update PATH
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")

    if ($userPath -notlike "*$INSTALL_DIR*") {
        Write-Host "Adding to PATH..." -ForegroundColor Yellow
        $newPath = "$INSTALL_DIR;$userPath"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")

        # Update current session PATH
        $env:Path = "$INSTALL_DIR;$env:Path"

        Write-Host "✓ Added to PATH" -ForegroundColor Green
        Write-Host ""
        Write-Host "Note: New terminals will automatically have tyr in PATH" -ForegroundColor Cyan
        Write-Host "      Current terminal is already updated!" -ForegroundColor Cyan
    } else {
        Write-Host "✓ Already in PATH" -ForegroundColor Green
    }

    Write-Host ""
    Write-Host "Installation complete! 🎉" -ForegroundColor Green
    Write-Host ""
    Write-Host "Run 'tyr --help' to get started" -ForegroundColor Cyan
    Write-Host ""

} catch {
    Write-Host ""
    Write-Host "Installation failed: $_" -ForegroundColor Red
    Write-Host ""
    Write-Host "Please check:" -ForegroundColor Yellow
    Write-Host "  1. Your internet connection" -ForegroundColor Yellow
    Write-Host "  2. The repository exists: https://github.com/$REPO" -ForegroundColor Yellow
    Write-Host "  3. There is a release with the asset: $ASSET_NAME" -ForegroundColor Yellow
    exit 1
}
