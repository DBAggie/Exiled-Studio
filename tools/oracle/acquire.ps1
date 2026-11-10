#Requires -Version 5.1
<#
.SYNOPSIS
Acquires the pinned PoB oracle (Spike C) on Windows: fetches PoB, LuaJIT and
luautf8 at the commits in manifest.json and builds LuaJIT and the lua-utf8
module with MSVC into <OracleDir>\bin, then writes <OracleDir>\build.json.

.DESCRIPTION
Maintainer and CI script. It is the only oracle step that uses the network
(git fetch); loom-parity-cli never fetches. Nothing is installed outside
<OracleDir>. Needs git and Visual Studio with the C++ workload (found with
vswhere). Check the result with `loom-parity-cli oracle-verify`.
#>
[CmdletBinding()]
param(
    [string]$Manifest,
    [string]$OracleDir
)
$ErrorActionPreference = 'Stop'
# Defaults here, not in param(): Windows PowerShell 5.1 run with -File leaves
# $PSScriptRoot empty while it binds parameters.
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $Manifest) { $Manifest = Join-Path $here 'manifest.json' }
if (-not $OracleDir) { $OracleDir = Join-Path $here '..\..\local\oracle' }

function Invoke-Native {
    param([string]$File, [string[]]$Arguments)
    # Windows PowerShell 5.1 turns native stderr into errors when output is
    # redirected; the exit code decides here.
    $ErrorActionPreference = 'Continue'
    & $File @Arguments 2>&1 | ForEach-Object { "$_" }
    if ($LASTEXITCODE -ne 0) { throw "$File $($Arguments -join ' ') exited with $LASTEXITCODE" }
}

# Fetches $Commit of $Url into $Dir (shallow, by SHA) and checks it out.
# A checkout already at $Commit is left as it is (oracle-verify reports a dirty tree).
function Get-Pinned {
    param([string]$Url, [string]$Commit, [string]$Dir)
    if (-not (Test-Path (Join-Path $Dir '.git'))) {
        New-Item -ItemType Directory -Force $Dir | Out-Null
        Invoke-Native git @('-C', $Dir, 'init', '-q')
        Invoke-Native git @('-C', $Dir, 'config', 'core.autocrlf', 'false')
        Invoke-Native git @('-C', $Dir, 'remote', 'add', 'origin', $Url)
    }
    Invoke-Native git @('-C', $Dir, 'remote', 'set-url', 'origin', $Url)
    $head = & git -C $Dir rev-parse --verify -q 'HEAD^{commit}'
    if ($head -ne $Commit) {
        Invoke-Native git @('-C', $Dir, 'fetch', '-q', '--depth', '1', 'origin', $Commit)
        Invoke-Native git @('-C', $Dir, '-c', 'advice.detachedHead=false', 'checkout', '-q', '--force', $Commit)
    }
    $head = & git -C $Dir rev-parse 'HEAD^{commit}'
    if ($head -ne $Commit) { throw "$Dir is at $head, expected $Commit" }
}

function Get-Sha256 {
    param([string]$Path)
    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

$m = Get-Content -Raw -LiteralPath $Manifest | ConvertFrom-Json
if ($m.format -ne 1) { throw "unsupported manifest format $($m.format)" }
New-Item -ItemType Directory -Force $OracleDir | Out-Null
$OracleDir = (Resolve-Path $OracleDir).Path
$luajitDir = Join-Path $OracleDir 'src\luajit'
$utf8Dir = Join-Path $OracleDir 'src\luautf8'
$bin = Join-Path $OracleDir 'bin'

Get-Pinned $m.pob.repository $m.pob.commit (Join-Path $OracleDir 'pob')
Get-Pinned $m.luajit.repository $m.luajit.commit $luajitDir
Get-Pinned $m.luautf8.repository $m.luautf8.commit $utf8Dir

# Fresh build trees at exactly the pinned sources (both are ours, under
# local/): an edited tracked file would otherwise be built and recorded as
# the pinned commit.
foreach ($pair in @(@($luajitDir, $m.luajit.commit), @($utf8Dir, $m.luautf8.commit))) {
    Invoke-Native git @('-C', $pair[0], '-c', 'advice.detachedHead=false', 'checkout', '-q', '--force', $pair[1])
    Invoke-Native git @('-C', $pair[0], 'clean', '-q', '-fdx')
}
if (Test-Path $bin) { Remove-Item -Recurse -Force $bin }
New-Item -ItemType Directory $bin | Out-Null

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vcvars = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -find 'VC\Auxiliary\Build\vcvars64.bat'
if (-not $vcvars) { throw 'vcvars64.bat not found (Visual Studio C++ workload needed)' }

# cl and link read extra options from the CL and LINK variables. /Brepro makes
# the output depend on the inputs only (no time stamps).
$defines = ($m.luajit.defines | ForEach-Object { "/D$_" }) -join ' '
$script = @"
@echo off
rem The file is UTF-8 (paths may hold any character).
chcp 65001 >nul
rem msvcbuild.bat runs its tools from the current directory, which cmd skips
rem when this variable is set (some agent and CI shells set it).
set NoDefaultCurrentDirectoryInExePath=
call "$vcvars" >nul || exit /b 1
set "CL=/Brepro $defines"
set "LINK=/Brepro"
cd /d "$luajitDir\src" || exit /b 1
call msvcbuild.bat || exit /b 1
cd /d "$utf8Dir" || exit /b 1
cl /nologo /MD /O2 /W3 /I"$luajitDir\src" /LD lutf8lib.c /Fe:lua-utf8.dll /link /EXPORT:luaopen_utf8 "$luajitDir\src\lua51.lib" || exit /b 1
cl 2> "$OracleDir\toolchain.txt"
exit /b 0
"@
$cmdFile = Join-Path $OracleDir 'build.cmd'
[System.IO.File]::WriteAllText($cmdFile, $script, (New-Object System.Text.UTF8Encoding $false))
Invoke-Native cmd.exe @('/d', '/c', $cmdFile)

Copy-Item (Join-Path $luajitDir 'src\luajit.exe') $bin
Copy-Item (Join-Path $luajitDir 'src\lua51.dll') $bin
Copy-Item (Join-Path $utf8Dir 'lua-utf8.dll') $bin

$files = [ordered]@{}
foreach ($name in 'luajit.exe', 'lua51.dll', 'lua-utf8.dll') {
    $files["bin/$name"] = Get-Sha256 (Join-Path $bin $name)
}
$toolchain = (Get-Content -LiteralPath (Join-Path $OracleDir 'toolchain.txt') | Select-Object -First 1).Trim()
$record = [ordered]@{
    format         = 1
    platform       = 'windows-x86_64'
    luajit_commit  = $m.luajit.commit
    luautf8_commit = $m.luautf8.commit
    defines        = @($m.luajit.defines)
    toolchain      = $toolchain
    files          = $files
}
$json = $record | ConvertTo-Json -Depth 4
[System.IO.File]::WriteAllText((Join-Path $OracleDir 'build.json'), $json + "`n")
Write-Output $json
