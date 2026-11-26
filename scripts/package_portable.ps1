# Packaging script for Tylogger Portable Distributions
param(
    [string]$DistDir = "dist",
    [string]$Version = "1.0.0"
)

$ErrorActionPreference = "Stop"

$RootDir = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RootDir

Write-Host "=== Packaging Tylogger Portable Distributions (v$Version) ===" -ForegroundColor Cyan

$distPath = Resolve-Path $DistDir

$archs = @(
    @{ Name = "x86"; BinaryPath = Join-Path $distPath "legacy-x86\tylogger.exe"; ZipName = "Tylogger-$Version-windows-x86-portable.zip" },
    @{ Name = "x64"; BinaryPath = Join-Path $distPath "legacy-x64\tylogger.exe"; ZipName = "Tylogger-$Version-windows-x64-portable.zip" }
)

foreach ($arch in $archs) {
    Write-Host "`nPackaging portable archive for $($arch.Name)..." -ForegroundColor Yellow
    
    if (-not (Test-Path $arch.BinaryPath)) {
        throw "Binary not found at $($arch.BinaryPath). Please run build_dist.ps1 first."
    }

    $stageDir = Join-Path $env:TEMP ("tylogger_portable_stage_" + $arch.Name + "_" + [System.Guid]::NewGuid().ToString("N"))
    $appDir = Join-Path $stageDir "Tylogger"
    New-Item -ItemType Directory -Path $appDir -Force | Out-Null

    # Copy binary
    Copy-Item $arch.BinaryPath -Destination (Join-Path $appDir "tylogger.exe") -Force

    # Create portable flag marker
    New-Item -ItemType File -Path (Join-Path $appDir "portable.flag") -Force | Out-Null

    # Copy documentation and metadata
    if (Test-Path (Join-Path $RootDir "README.txt")) {
        Copy-Item (Join-Path $RootDir "README.txt") -Destination (Join-Path $appDir "README.txt") -Force
    }
    if (Test-Path (Join-Path $RootDir "LICENSE")) {
        Copy-Item (Join-Path $RootDir "LICENSE") -Destination (Join-Path $appDir "LICENSE") -Force
    }
    if (Test-Path (Join-Path $RootDir "THIRD_PARTY_NOTICES")) {
        Copy-Item (Join-Path $RootDir "THIRD_PARTY_NOTICES") -Destination (Join-Path $appDir "THIRD_PARTY_NOTICES") -Force
    }

    # Create empty directories for portable runtime
    New-Item -ItemType Directory -Path (Join-Path $appDir "config") -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $appDir "plugins") -Force | Out-Null
    
    # Copy examples if available
    $examplesDir = Join-Path $RootDir "examples"
    if (Test-Path $examplesDir) {
        Copy-Item $examplesDir -Destination (Join-Path $appDir "examples") -Recurse -Force
    }

    # Create zip archive in DistDir
    $zipDest = Join-Path $distPath $arch.ZipName
    if (Test-Path $zipDest) {
        Remove-Item $zipDest -Force
    }

    Write-Host "Compressing to $zipDest..."
    Compress-Archive -Path "$appDir\*" -DestinationPath $zipDest -CompressionLevel Optimal

    # Cleanup staging directory
    Remove-Item $stageDir -Recurse -Force

    $zipSize = (Get-Item $zipDest).Length
    $zipHash = (Get-FileHash -Path $zipDest -Algorithm SHA256).Hash.ToLower()
    Write-Host "Generated $($arch.ZipName): $zipSize bytes | SHA256: $zipHash" -ForegroundColor Green
}

Write-Host "`nAll portable distribution packages built successfully." -ForegroundColor Green
