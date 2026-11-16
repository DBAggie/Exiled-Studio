# Fetches GGG's passive tree export (grindinggear/skilltree-export) at the
# commit pinned in tools/tree/manifest.json into local/tree-export/
# (ADR-005 Amendment 2; maintainer-run, network: git only). With a tree
# version (`3_28`), the commit pinned for that version under `versions`
# into local/tree-export-<version>/ instead (tree packs). Nothing from it
# is committed; `loom-parity-cli tree-layout` and `tree-pack` read it and
# check the pin.
#
#   powershell -ExecutionPolicy Bypass -File tools/tree/acquire.ps1 [-Version 3_28]

param([string]$Version = '')
$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Resolve-Path (Join-Path $here '..\..')
$manifest = Get-Content (Join-Path $here 'manifest.json') -Raw | ConvertFrom-Json
$repository = $manifest.export.repository
if ($Version) {
    $pin = $manifest.versions.$Version
    if (-not $pin) { throw "no pin for tree version $Version in manifest.json" }
    $out = Join-Path $root "local\tree-export-$Version"
} else {
    $pin = $manifest.export
    $out = Join-Path $root 'local\tree-export'
}

if (-not (Test-Path (Join-Path $out '.git'))) {
    # Without the advice off, cloning a tag writes its detached HEAD note to
    # stderr, which Windows PowerShell 5.1 stops on under 'Stop'.
    git -c advice.detachedHead=false clone --quiet --depth 1 --branch $pin.tag $repository $out
    if ($LASTEXITCODE -ne 0) { throw "git clone failed" }
} else {
    git -C $out fetch --quiet --depth 1 origin tag $pin.tag
    if ($LASTEXITCODE -ne 0) { throw "git fetch failed" }
    git -C $out checkout --quiet --detach $pin.tag
    if ($LASTEXITCODE -ne 0) { throw "git checkout failed" }
}
$head = (git -C $out rev-parse HEAD).Trim()
if ($head -ne $pin.commit) { throw "$out is at $head, the pin is $($pin.commit)" }
Write-Output "$out at $($pin.tag) ($head)"
