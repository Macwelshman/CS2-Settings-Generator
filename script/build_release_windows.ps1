param(
    [switch]$CollectOnly
)

$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
$BuildsDir = Join-Path $RootDir "Builds"
$BundleDir = Join-Path $RootDir "target\release\bundle"

New-Item -ItemType Directory -Force -Path $BuildsDir | Out-Null

if (-not $CollectOnly) {
    Push-Location $RootDir
    try {
        & cargo tauri build
        if ($LASTEXITCODE -ne 0) {
            throw "The Windows release build failed with exit code $LASTEXITCODE."
        }
    }
    finally {
        Pop-Location
    }
}

$InstallerFolders = @(
    (Join-Path $BundleDir "nsis"),
    (Join-Path $BundleDir "msi")
)
$Installers = @(
    foreach ($Folder in $InstallerFolders) {
        if (Test-Path -LiteralPath $Folder) {
            Get-ChildItem -LiteralPath $Folder -File |
                Where-Object { $_.Extension -in ".exe", ".msi" }
        }
    }
)

if ($Installers.Count -eq 0) {
    throw "The release build completed without producing a Windows .exe or .msi installer."
}

foreach ($Installer in $Installers) {
    $Destination = Join-Path $BuildsDir $Installer.Name
    Copy-Item -LiteralPath $Installer.FullName -Destination $Destination -Force
    Write-Host "Windows release build ready: $Destination"
}
