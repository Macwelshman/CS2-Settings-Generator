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

# The app uses the already-installed WebView2 runtime; the ZIP replaces only the executable.
$UpdateExe = Join-Path $RootDir "target\release\cs2-settings-generator.exe"
$UpdateVersion = [Diagnostics.FileVersionInfo]::GetVersionInfo($UpdateExe).ProductVersion
if ($UpdateVersion -notmatch '^\d+\.\d+\.\d+$') { throw "Unexpected executable product version: $UpdateVersion" }
$PeBytes = [IO.File]::ReadAllBytes($UpdateExe)
$PeOffset = [BitConverter]::ToInt32($PeBytes, 0x3c)
$Machine = [BitConverter]::ToUInt16($PeBytes, $PeOffset + 4)
$UpdateArch = switch ($Machine) { 0x8664 { "x64" }; 0xaa64 { "arm64" }; default { throw "Unsupported PE architecture." } }
$UpdateZip = Join-Path $BuildsDir "CS2-Settings-Generator-$UpdateVersion-windows-$UpdateArch.zip"
Compress-Archive -LiteralPath $UpdateExe -DestinationPath $UpdateZip -Force
$Hash = (Get-FileHash -LiteralPath $UpdateZip -Algorithm SHA256).Hash.ToLowerInvariant()
Set-Content -LiteralPath "$UpdateZip.sha256" -Value "$Hash  $([IO.Path]::GetFileName($UpdateZip))"
Write-Host "Update package ready: $UpdateZip"
