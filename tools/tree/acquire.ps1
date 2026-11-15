# Fetches GGG's passive tree export (grindinggear/skilltree-export) at the
# commit pinned in tools/tree/manifest.json into local/tree-export/
# (ADR-005 Amendment 2; maintainer-run, network: git only). Nothing from it
# is committed; `loom-parity-cli tree-layout` reads it and checks the pin.
#
#   powershell -ExecutionPolicy Bypass -File tools/tree/acquire.ps1

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Resolve-Path (Join-Path $here '..\..')
$pin = (Get-Content (Join-Path $here 'manifest.json') -Raw | ConvertFrom-Json).export
$out = Join-Path $root 'local\tree-export'

if (-not (Test-Path (Join-Path $out '.git'))) {
    git clone --quiet --depth 1 --branch $pin.tag $pin.repository $out
    if ($LASTEXITCODE -ne 0) { throw "git clone failed" }
} else {
    git -C $out fetch --quiet --depth 1 origin tag $pin.tag
    if ($LASTEXITCODE -ne 0) { throw "git fetch failed" }
    git -C $out checkout --quiet --detach $pin.tag
    if ($LASTEXITCODE -ne 0) { throw "git checkout failed" }
}
$head = (git -C $out rev-parse HEAD).Trim()
if ($head -ne $pin.commit) { throw "local/tree-export is at $head, the pin is $($pin.commit)" }
Write-Output "local/tree-export at $($pin.tag) ($head)"
