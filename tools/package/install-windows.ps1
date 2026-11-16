# Installs a Windows installer silently, starts the installed application
# once (smoke.ps1), and uninstalls it (Phase 5, P1; release workflow).
#
#   install-windows.ps1 <nsis|msi> <installer> [<version>]
#
# NSIS installs per user (`/S`) into %LOCALAPPDATA%\Exiled Studio and is
# removed by its uninstall.exe (`/S`); the MSI installs per machine
# (`msiexec /qn`) and is removed with `msiexec /x`. The installed folder
# must be gone afterwards. The folder is the product's, which follows
# LOOM_CHANNEL (docs/release/channels.md).
param(
    [Parameter(Mandatory = $true)][ValidateSet('nsis', 'msi')][string]$Kind,
    [Parameter(Mandatory = $true)][string]$Installer,
    [string]$Version = ''
)
$ErrorActionPreference = 'Stop'
$product = if ($env:LOOM_CHANNEL -eq 'experimental') { 'Exiled Studio Experimental' } else { 'Exiled Studio' }

function Run([string]$file, [string[]]$arguments) {
    $p = Start-Process -FilePath $file -ArgumentList $arguments -PassThru -Wait
    if ($p.ExitCode -ne 0) { throw "$file $arguments exited $($p.ExitCode)" }
}

if ($Kind -eq 'nsis') {
    $dir = Join-Path $env:LOCALAPPDATA $product
    Run $Installer @('/S')
} else {
    $dir = Join-Path $env:ProgramFiles $product
    $log = Join-Path ([System.IO.Path]::GetTempPath()) 'loom-msi-install.log'
    Run 'msiexec.exe' @('/i', "`"$Installer`"", '/qn', '/l*v', "`"$log`"")
}
$exe = Get-ChildItem -Path $dir -Filter '*.exe' | Where-Object { $_.Name -notlike 'uninstall*' } | Select-Object -First 1
if ($null -eq $exe) { throw "no application in $dir" }
Write-Output "installed: $($exe.FullName)"

& (Join-Path $PSScriptRoot 'smoke.ps1') $exe.FullName $Kind $Version

if ($Kind -eq 'nsis') {
    Run (Join-Path $dir 'uninstall.exe') @('/S')
    # The NSIS uninstaller copies itself and returns before it is done.
    for ($i = 0; $i -lt 60 -and (Test-Path $exe.FullName); $i++) { Start-Sleep -Seconds 1 }
} else {
    Run 'msiexec.exe' @('/x', "`"$Installer`"", '/qn')
}
if (Test-Path $exe.FullName) { throw "still installed after uninstalling: $($exe.FullName)" }
Write-Output "uninstalled: $Kind"
