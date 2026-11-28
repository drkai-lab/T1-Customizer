# Build T1-Customizer (free edition) for Windows on this PC (no GitHub Actions).
#   pwsh windows/build-release.ps1
# Output: dist-win\t1-customizer-<ver>-setup.exe
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$dist = Join-Path $root 'dist-win'
Push-Location (Join-Path $root 'rust')
try {
    # RUSTFLAGS replaces rust/.cargo/config.toml, so repeat +crt-static.
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
$version = & (Join-Path $dist 't1-keyboard-config.exe') --version
if ($version -notmatch '1\.0\.6') { throw "unexpected version: $version" }
$iscc = @("$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe", "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe", "$env:ProgramFiles\Inno Setup 6\ISCC.exe") |
    Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw 'Inno Setup 6 (ISCC.exe) not found' }
& $iscc (Join-Path $PSScriptRoot 'installer.iss')
if ($LASTEXITCODE -ne 0) { throw 'ISCC failed' }
Get-ChildItem $dist | Format-Table Name, Length
