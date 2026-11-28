# Build the Microsoft Store MSIX package for T1-Customizer.
#   pwsh windows/build-msix.ps1              (build the exes, then pack)
#   pwsh windows/build-msix.ps1 -SkipBuild   (pack the exes already in dist-win)
# Output: dist-win\t1-customizer-<ver>.msix  (UNSIGNED - the Store signs it)
#
# The package identity in packaging\msix\Package.appxmanifest must match
# Partner Center (Product management > Product identity) before the
# package can be uploaded. See docs/microsoft-store.md.
param(
    [switch]$SkipBuild,
    [string]$MakeAppxPath
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$dist = Join-Path $root 'dist-win'
$packDir = Join-Path $root 'packaging\msix'
$stage = Join-Path $packDir 'stage'
$manifestSrc = Join-Path $packDir 'Package.appxmanifest'

if (-not (Test-Path $manifestSrc)) { throw "manifest not found: $manifestSrc" }
$manifestText = Get-Content $manifestSrc -Raw
if ($manifestText -match 'REPLACE-WITH-PARTNER-CENTER') {
    Write-Warning 'Package.appxmanifest still contains the Partner Center placeholders.'
    Write-Warning 'The package will build, but the Store will REJECT it on upload until'
    Write-Warning 'Identity Name / Publisher match Partner Center > Product identity.'
    Write-Warning 'See docs/microsoft-store.md.'
}

if (-not $SkipBuild) {
    # Same build as windows/build-release.ps1: RUSTFLAGS replaces
    # rust/.cargo/config.toml, so repeat +crt-static.
    Push-Location (Join-Path $root 'rust')
    try {
        $env:RUSTFLAGS = "--remap-path-prefix=$root=/src -C target-feature=+crt-static"
        cargo build --release --locked
        if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
        $target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path (Get-Location) 'target' }
    } finally {
        Pop-Location
        Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
    }
    New-Item -ItemType Directory -Force $dist | Out-Null
    foreach ($name in 't1-keyboard-config.exe', 't1-keyboard-config-gui.exe') {
        Copy-Item (Join-Path $target "release\$name") (Join-Path $dist $name) -Force
    }
}

foreach ($name in 't1-keyboard-config.exe', 't1-keyboard-config-gui.exe') {
    if (-not (Test-Path (Join-Path $dist $name))) {
        throw "$name not found in $dist (build first, or drop -SkipBuild)"
    }
}

# Version: read x.y.z from the console executable, MSIX uses x.y.z.0.
$versionOut = & (Join-Path $dist 't1-keyboard-config.exe') --version
if ($versionOut -notmatch '(\d+)\.(\d+)\.(\d+)') { throw "cannot parse version from: $versionOut" }
$ver = "$($Matches[1]).$($Matches[2]).$($Matches[3])"
$msixVer = "$ver.0"

# Stage the package layout: exes at the root, manifest + Assets next to them.
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force $stage | Out-Null
foreach ($name in 't1-keyboard-config.exe', 't1-keyboard-config-gui.exe') {
    Copy-Item (Join-Path $dist $name) (Join-Path $stage $name) -Force
}
Copy-Item (Join-Path $packDir 'Assets') (Join-Path $stage 'Assets') -Recurse -Force
# Rewrite the Version attribute in the STAGED manifest only; the checked-in
# manifest keeps its placeholders/version untouched.
$stagedManifest = $manifestText -replace 'Version="\d+\.\d+\.\d+\.\d+"', "Version=`"$msixVer`""
Set-Content (Join-Path $stage 'AppxManifest.xml') $stagedManifest -Encoding UTF8

# Locate makeappx.exe from the Windows SDK if not given.
if (-not $MakeAppxPath) {
    $kitsBin = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    $candidate = Get-ChildItem $kitsBin -Directory -ErrorAction SilentlyContinue |
        Sort-Object Name -Descending |
        ForEach-Object { Join-Path $_.FullName 'x64\makeappx.exe' } |
        Where-Object { Test-Path $_ } | Select-Object -First 1
    if ($candidate) { $MakeAppxPath = $candidate }
}
if (-not $MakeAppxPath -or -not (Test-Path $MakeAppxPath)) {
    throw 'makeappx.exe not found. Install the Windows 10/11 SDK or pass -MakeAppxPath.'
}

$msix = Join-Path $dist "t1-customizer-$ver.msix"
& $MakeAppxPath pack /d $stage /p $msix /o
if ($LASTEXITCODE -ne 0) { throw 'makeappx pack failed' }

Write-Host ''
Write-Host "Built (unsigned, ready for Store upload): $msix"
Write-Host 'Do NOT sign this file. The Microsoft Store re-signs it after certification.'
Get-Item $msix | Format-Table Name, Length
