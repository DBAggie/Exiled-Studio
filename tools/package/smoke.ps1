# Starts a package once and checks its build info (Phase 5, P1;
# docs/phase-5/packaging.md). The application, started with
# `--smoke <file>`, writes the file when its first screen has come up and
# quits (exit 0), or after 90 seconds without it (exit 2).
#
#   smoke.ps1 <portable .zip or installed .exe> <artifact> [<version>]
#
# <artifact> is what the build must report: plain (portable zip), nsis, msi.
# A zip is unpacked into a temporary folder first, and must report itself
# portable. <version>, when given, must equal the reported version. The
# build must report LOOM_CHANNEL's channel (stable when unset;
# docs/release/channels.md).
param(
    [Parameter(Mandatory = $true)][string]$Package,
    [Parameter(Mandatory = $true)][string]$Artifact,
    [string]$Version = ''
)
$ErrorActionPreference = 'Stop'
$channel = if ($env:LOOM_CHANNEL) { $env:LOOM_CHANNEL } else { 'stable' }
$product = if ($channel -eq 'experimental') { 'Exiled Studio Experimental' } else { 'Exiled Studio' }

$work = Join-Path ([System.IO.Path]::GetTempPath()) ("loom-smoke-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force $work | Out-Null
$portable = $Package.EndsWith('.zip')
if ($portable) {
    Expand-Archive -Path $Package -DestinationPath (Join-Path $work 'app')
    $exe = Join-Path $work "app/$product.exe"
} else {
    $exe = $Package
    # An installed build keeps its data in a throwaway folder, never the
    # user's application data (a portable one must use its own folder).
    $env:LOOM_DATA_DIR = Join-Path $work 'data'
}
$out = Join-Path $work 'smoke.json'
$p = Start-Process -FilePath $exe -ArgumentList '--smoke', "`"$out`"" -PassThru
if (-not $p.WaitForExit(150000)) {
    $p.Kill()
    throw "the application did not quit within 150 seconds"
}
if (-not (Test-Path $out)) { throw "no smoke file (exit $($p.ExitCode))" }
$info = Get-Content -Raw $out | ConvertFrom-Json
Write-Output (Get-Content -Raw $out)
$errors = @()
if ($p.ExitCode -ne 0) { $errors += "exit code $($p.ExitCode)" }
if (-not $info.ready) { $errors += 'the first screen did not come up' }
if ($info.e2e) { $errors += 'the e2e feature is in the build' }
if ($info.debug) { $errors += 'a debug build' }
if ($info.artifact -ne $Artifact) { $errors += "artifact $($info.artifact), expected $Artifact" }
if ($info.portable -ne $portable) { $errors += "portable $($info.portable), expected $portable" }
if ($info.channel -ne $channel) { $errors += "channel $($info.channel), expected $channel" }
if ($Version -ne '' -and $info.version -ne $Version) { $errors += "version $($info.version), expected $Version" }
Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
if ($errors.Count -gt 0) { throw ($errors -join '; ') }
Write-Output "smoke ok: $Artifact"
