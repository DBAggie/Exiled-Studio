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
# The synthetic build as the synthetic tree bundle's Test Brute (passive
# tree plan T1a), so the workflow can allocate on the tree, with a large
# cluster jewel (item 5, the bundle's made-up base and lines; T5b).
$xml = [IO.File]::ReadAllText((Resolve-Path 'tools\parity-cli\testdata\build-round-trip\synthetic-poe1.xml').Path)
$xml = $xml.Replace('className="Scion"', 'className="Test Brute"').Replace('classId="0" ascendClassId="0" nodes=""', 'classId="1" ascendClassId="0" nodes="1000"')
$cluster = "<Item id=`"5`">`nRarity: RARE`nLoom Test Cluster`nLarge Test Cluster Jewel`nImplicits: 0`nAdds +8 Test Cluster Passive Skills`n+2 Test Cluster Jewel Sockets`n+2 Test Cluster Skill Row`n+1 Test Cluster Notable A`n+1 Test Cluster Notable B`n+25% Test Cluster Small Effect`n</Item>`n`t`t<ItemSet id=`"1`""
$xml = $xml.Replace('<ItemSet id="1"', $cluster)
[IO.File]::WriteAllText((Join-Path $work 'import.xml'), $xml)

# A pack of the synthetic tree bundle, signed with the public test key
# (Phase 5, P4: the e2e installs it).
& cargo run -q --locked -p loom-release -- pack crates/loom-data/tests/fixtures/bundle-poe1-tree --test-key --out (Join-Path $work 'bundle.loombundle')
if ($LASTEXITCODE -ne 0) { throw 'loom-release pack failed' }

$env:LOOM_E2E_DIR = $work
$env:LOOM_DATA_DIR = Join-Path $work 'data'
$env:LOOM_BUNDLE_DIR = (Resolve-Path 'crates\loom-data\tests\fixtures\bundle-poe1-tree').Path

# Diagnosis for the hosted runner, where msedgedriver cannot create a
# session ("DevToolsActivePort file doesn't exist") though the workflow
# passes on a Windows desktop: start the app alone with a fixed DevTools
# port and say whether WebView2 answers, then stop it. CI only.
if ($env:CI) {
  $probeArgs = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9333'
  $probe = Start-Process $app -PassThru -RedirectStandardError (Join-Path $work 'probe-err.txt') -RedirectStandardOutput (Join-Path $work 'probe-out.txt')
  $answer = $null
  for ($i = 0; $i -lt 60 -and -not $probe.HasExited; $i++) {
    try { $answer = (Invoke-WebRequest 'http://127.0.0.1:9333/json/version' -UseBasicParsing -TimeoutSec 2).Content; break } catch { Start-Sleep -Seconds 1 }
  }
  $probe.Refresh()
  Write-Output "probe: app exited $($probe.HasExited)$(if ($probe.HasExited) { " (code $($probe.ExitCode))" }), DevTools $(if ($answer) { 'answered' } else { 'did not answer' }) after $i s"
  if ($answer) { Write-Output $answer }
  Get-Content (Join-Path $work 'probe-err.txt') -Tail 30 -ErrorAction SilentlyContinue
  if (-not $probe.HasExited) { Stop-Process $probe -Force }
  # The WebView2 processes outlive the app for a moment and hold its data
  # folder; the driver's session must not start before they are gone.
  for ($i = 0; $i -lt 20 -and (Get-Process msedgewebview2 -ErrorAction SilentlyContinue); $i++) { Start-Sleep -Milliseconds 500 }
  Get-ChildItem (Join-Path $env:LOCALAPPDATA '*\EBWebView') -Directory -ErrorAction SilentlyContinue | ForEach-Object { "probe: WebView2 data folder $($_.FullName)" }
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $probeArgs
}

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
