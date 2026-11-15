# Writes ui/src/data/pob-config-options.json: PoB's configuration options
# (labels, types, choices, defaults, tooltips) for the configuration page,
# read from the pinned PoB in local/oracle by adapter/config_options.lua.
# Needs the oracle acquired (acquire.ps1); verifies it first (offline).
#
#   powershell -ExecutionPolicy Bypass -File tools/oracle/config-options.ps1

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Resolve-Path (Join-Path $here '..\..')
$oracle = Join-Path $root 'local\oracle'
$manifest = Get-Content (Join-Path $here 'manifest.json') -Raw | ConvertFrom-Json

Push-Location $root
try {
    cargo run --release --quiet -p loom-parity-cli -- oracle-verify tools/oracle/manifest.json local/oracle | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "oracle-verify failed ($LASTEXITCODE): run tools/oracle/acquire.ps1" }
} finally { Pop-Location }

$out = Join-Path $root 'ui\src\data\pob-config-options.json'
New-Item -ItemType Directory -Force (Split-Path $out) | Out-Null
$env:LUA_PATH = './?.lua;../runtime/lua/?.lua;../runtime/lua/?/init.lua'
$env:LUA_CPATH = (Join-Path $oracle 'bin\?.dll')
Push-Location (Join-Path $oracle 'pob\src')
try {
    $script = Join-Path $here 'adapter\config_options.lua'
    # PoB's startup progress goes to stderr; Windows PowerShell would turn
    # it into errors under 'Stop'.
    $ErrorActionPreference = 'Continue'
    $json = & (Join-Path $oracle 'bin\luajit.exe') $script $manifest.pob.tag $manifest.pob.commit 2>$null
    $ErrorActionPreference = 'Stop'
    if ($LASTEXITCODE -ne 0) { throw "config_options.lua failed ($LASTEXITCODE)" }
    # One line of JSON, UTF-8 without a byte order mark.
    [System.IO.File]::WriteAllText($out, ($json -join "`n") + "`n", (New-Object System.Text.UTF8Encoding $false))
} finally { Pop-Location }
Write-Output "wrote $out"
