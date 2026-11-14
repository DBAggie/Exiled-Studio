# Runs the real-app workflow (ui/e2e/workflow.mjs) on Windows: starts
# tauri-driver with msedgedriver, then the script. From the repository
# root, in CI's e2e job (Windows) or by hand:
#
#   pwsh ui/e2e/run.ps1 <app binary> <msedgedriver.exe> [<tauri-driver.exe>]
#
# PowerShell, not Git Bash: the native drivers and WebView2 must see
# Windows paths and environment as they are (under Git Bash the session
# could not be created: DevToolsActivePort file doesn't exist).
param(
  [Parameter(Mandatory = $true)][string]$App,
  [Parameter(Mandatory = $true)][string]$EdgeDriver,
  [string]$TauriDriver = 'tauri-driver'
)
$ErrorActionPreference = 'Stop'

if (-not (Test-Path $EdgeDriver)) { throw "no native driver at $EdgeDriver" }
$app = (Resolve-Path $App).Path
$work = Join-Path ([IO.Path]::GetTempPath()) "loom-e2e-$PID"
New-Item -ItemType Directory -Force $work | Out-Null
Copy-Item tools\parity-cli\testdata\build-round-trip\synthetic-poe1.xml (Join-Path $work 'import.xml')

$env:LOOM_E2E_DIR = $work
$env:LOOM_DATA_DIR = Join-Path $work 'data'
$env:LOOM_BUNDLE_DIR = (Resolve-Path 'crates\loom-data\tests\fixtures\bundle-poe1').Path

$driver = Start-Process $TauriDriver -ArgumentList '--native-driver', $EdgeDriver -PassThru -NoNewWindow
try {
  # Wait for the driver to listen.
  for ($i = 0; $i -lt 50; $i++) {
    try { Invoke-WebRequest 'http://127.0.0.1:4444/status' -UseBasicParsing | Out-Null; break } catch { Start-Sleep -Milliseconds 200 }
  }
  node ui\e2e\workflow.mjs $app $work
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
  Stop-Process $driver -ErrorAction SilentlyContinue
}
