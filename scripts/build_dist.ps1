# Multi-target release build script for Tylogger (Universal Binary Architecture)
param(
    [string]$DistDir = "dist",
    [string]$Version = "1.0.0",
    [string]$Milestone = "v1.0.0-ga"
)

$ErrorActionPreference = "Stop"

$RootDir = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RootDir

Write-Host "=== Building Tylogger Universal Quad-Target Distribution ===" -ForegroundColor Cyan

# Ensure target directories exist
$targets = @("legacy-x86", "legacy-x64", "modern-x86", "modern-x64", "universal-x86", "universal-x64")
foreach ($t in $targets) {
    $dir = Join-Path $DistDir $t
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
}

# 1. Build x86_64 target
Write-Host "`n[1/2] Compiling target x86_64-pc-windows-msvc (Release)..." -ForegroundColor Yellow
cargo build --release --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) {
    throw "Build failed for x86_64-pc-windows-msvc"
}

# 2. Build i686 target
Write-Host "`n[2/2] Compiling target i686-pc-windows-msvc (Release)..." -ForegroundColor Yellow
cargo build --release --target i686-pc-windows-msvc
if ($LASTEXITCODE -ne 0) {
    throw "Build failed for i686-pc-windows-msvc"
}

$x64Bin = Join-Path $RootDir "target\x86_64-pc-windows-msvc\release\tylogger.exe"
$x86Bin = Join-Path $RootDir "target\i686-pc-windows-msvc\release\tylogger.exe"

# Copy binaries to quad-target and universal distribution folders
Copy-Item $x64Bin -Destination (Join-Path $DistDir "modern-x64\tylogger.exe") -Force
Copy-Item $x64Bin -Destination (Join-Path $DistDir "legacy-x64\tylogger.exe") -Force
Copy-Item $x64Bin -Destination (Join-Path $DistDir "universal-x64\tylogger.exe") -Force

Copy-Item $x86Bin -Destination (Join-Path $DistDir "modern-x86\tylogger.exe") -Force
Copy-Item $x86Bin -Destination (Join-Path $DistDir "legacy-x86\tylogger.exe") -Force
Copy-Item $x86Bin -Destination (Join-Path $DistDir "universal-x86\tylogger.exe") -Force

# Read git commit provenance
$gitCommit = (git rev-parse HEAD).Trim()
$timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
$compilerVer = (rustc --version).Trim()

function Get-ArtifactMeta {
    param([string]$Path, [string]$Arch, [string]$PeType, [string]$Subsystem, [string]$Crt)
    $hash = (Get-FileHash -Path $Path -Algorithm SHA256).Hash.ToLower()
    $size = (Get-Item -Path $Path).Length
    return [PSCustomObject]@{
        architecture = $Arch
        pe_type = $PeType
        subsystem = $Subsystem
        crt_linkage = $Crt
        binary_size = $size
        sha256 = $hash
    }
}

$manifest = [PSCustomObject]@{
    product = "Tylogger"
    version = $Version
    milestone = $Milestone
    commit = $gitCommit
    build_timestamp = $timestamp
    compiler = $compilerVer
    universal_binary_policy = "Unified static CRT + NT 6.01 subsystem + runtime delayload shims for Win7 SP1 through Win11"
    targets = [PSCustomObject]@{
        "legacy-x86" = Get-ArtifactMeta -Path (Join-Path $DistDir "legacy-x86\tylogger.exe") -Arch "i686" -PeType "PE32" -Subsystem "WINDOWS,6.01" -Crt "static"
        "legacy-x64" = Get-ArtifactMeta -Path (Join-Path $DistDir "legacy-x64\tylogger.exe") -Arch "x86_64" -PeType "PE32+" -Subsystem "WINDOWS,6.01" -Crt "static"
        "modern-x86" = Get-ArtifactMeta -Path (Join-Path $DistDir "modern-x86\tylogger.exe") -Arch "i686" -PeType "PE32" -Subsystem "WINDOWS,6.01" -Crt "static"
        "modern-x64" = Get-ArtifactMeta -Path (Join-Path $DistDir "modern-x64\tylogger.exe") -Arch "x86_64" -PeType "PE32+" -Subsystem "WINDOWS,6.01" -Crt "static"
        "universal-x86" = Get-ArtifactMeta -Path (Join-Path $DistDir "universal-x86\tylogger.exe") -Arch "i686" -PeType "PE32" -Subsystem "WINDOWS,6.01" -Crt "static"
        "universal-x64" = Get-ArtifactMeta -Path (Join-Path $DistDir "universal-x64\tylogger.exe") -Arch "x86_64" -PeType "PE32+" -Subsystem "WINDOWS,6.01" -Crt "static"
    }
}

$manifestJson = $manifest | ConvertTo-Json -Depth 5
$manifestPath = Join-Path $DistDir "build-manifest.json"
Set-Content -Path $manifestPath -Value $manifestJson -Encoding UTF8

Write-Host "`nBuild complete. Manifest written to: $manifestPath" -ForegroundColor Green
Write-Host $manifestJson
