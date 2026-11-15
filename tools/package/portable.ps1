# Makes the portable zip (Phase 5, P1; docs/phase-5/packaging.md) from the
# plain release executable: "Exiled Studio.exe", the `portable` marker that
# keeps the data in `data\` beside it, the license with GGG's notice, and
# the shipped game data packs in `bundles\` when the release workflow made
# them (crates/loom-tauri/shipped/, P4).
#
#   portable.ps1 <loom-tauri.exe> <out dir> [<version>]
#
# Give the executable before `tauri build` runs: the bundler patches its
# installer type into the executable in place, and a patched one would
# report itself as an installer (and try to update itself).
param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [Parameter(Mandatory = $true)][string]$OutDir,
    [string]$Version = 'dev'
)
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$stage = Join-Path ([System.IO.Path]::GetTempPath()) ("loom-portable-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force $stage | Out-Null
try {
    Copy-Item $Exe (Join-Path $stage 'Exiled Studio.exe')
    Set-Content -Path (Join-Path $stage 'portable') -Value 'Exiled Studio keeps its data in the data folder beside it while this file is here.' -Encoding ascii
    Copy-Item (Join-Path $root 'crates/loom-tauri/installer/LICENSE.txt') (Join-Path $stage 'LICENSE.txt')
    $shipped = Get-ChildItem -ErrorAction SilentlyContinue (Join-Path $root 'crates/loom-tauri/shipped/*.loombundle')
    if ($shipped) {
        New-Item -ItemType Directory -Force (Join-Path $stage 'bundles') | Out-Null
        $shipped | Copy-Item -Destination (Join-Path $stage 'bundles')
    }
    New-Item -ItemType Directory -Force $OutDir | Out-Null
    $zip = Join-Path $OutDir "Exiled-Studio_${Version}_x64-portable.zip"
    if (Test-Path $zip) { Remove-Item -Force $zip }
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zip
    Write-Output $zip
} finally {
    Remove-Item -Recurse -Force $stage
}
