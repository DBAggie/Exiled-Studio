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
    $log = Join-Path ([System.IO.Path]::GetTempPath()) 'loom-msi-install.log'
    Run 'msiexec.exe' @('/i', "`"$Installer`"", '/qn', '/l*v', "`"$log`"")
    # The MSI installs per machine into Program Files, unless it finds an
    # earlier install folder in the registry: the NSIS setup (run before it
    # in the release workflow, as on a user's machine that had it) records
    # its per-user folder under the product's key, which the MSI's
    # INSTALLDIR search reads. Either is a working install.
    $candidates = @((Join-Path $env:ProgramFiles $product), (Join-Path $env:LOCALAPPDATA $product))
    $dir = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if ($null -eq $dir) {
        Get-Content $log | Select-String -Pattern 'INSTALLDIR|Installation success or error status|Return value 3' |
            Select-Object -Last 40 | ForEach-Object { Write-Output $_.Line }
        throw "the MSI installed no folder at $($candidates -join ' or ')"
    }
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
