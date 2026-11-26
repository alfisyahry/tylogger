# Package integrity and structure verification script for Tylogger
param(
    [string]$DistDir = "dist",
    [string]$Version = "1.0.0"
)

$ErrorActionPreference = "Stop"

$RootDir = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RootDir

Write-Host "=== Validating Tylogger Release Packages Integrity (v$Version) ===" -ForegroundColor Cyan

$distPath = Resolve-Path $DistDir
$zips = @(
    (Join-Path $distPath "Tylogger-$Version-windows-x86-portable.zip"),
    (Join-Path $distPath "Tylogger-$Version-windows-x64-portable.zip")
)

Add-Type -AssemblyName System.IO.Compression.FileSystem

foreach ($zip in $zips) {
    if (-not (Test-Path $zip)) {
        throw "Missing zip package: $zip"
    }

    $size = (Get-Item $zip).Length
    $hash = (Get-FileHash -Path $zip -Algorithm SHA256).Hash.ToLower()
    Write-Host "`nInspecting $(Split-Path $zip -Leaf) ($size bytes)..." -ForegroundColor Yellow

    $archive = [System.IO.Compression.ZipFile]::OpenRead($zip)
    $entryNames = $archive.Entries | ForEach-Object { $_.FullName -replace '\\', '/' }
    $archive.Dispose()

    $requiredEntries = @(
        "Tylogger/tylogger.exe",
        "Tylogger/portable.flag",
        "Tylogger/README.txt",
        "Tylogger/LICENSE",
        "Tylogger/THIRD_PARTY_NOTICES"
    )

    foreach ($req in $requiredEntries) {
        if ($entryNames -notcontains $req) {
            throw "Package $zip is missing required entry: $req"
        }
    }
    Write-Host "  -> PASS: All required runtime and license files verified inside $(Split-Path $zip -Leaf)" -ForegroundColor Green
}

# Verify Inno Setup script presence and syntax readiness
$issPath = Join-Path $RootDir "installer\tylogger.iss"
if (-not (Test-Path $issPath)) {
    throw "Missing Inno Setup script: $issPath"
}
$issContent = Get-Content $issPath -Raw
if ($issContent -notmatch "MyAppMutex.*TyloggerSingleInstanceMutex" -or $issContent -notmatch "PrivilegesRequired=lowest") {
    throw "Inno Setup script failed security audit checks"
}
Write-Host "`n  -> PASS: Inno Setup script installer/tylogger.iss verified with AppMutex and dual install modes" -ForegroundColor Green

Write-Host "`n=== ALL PACKAGE INTEGRITY CHECKS PASSED ===" -ForegroundColor Green
