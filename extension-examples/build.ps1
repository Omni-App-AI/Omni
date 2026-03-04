# ───────────────────────────────────────────────────────────────
# build.ps1 — Build and package an Omni extension (Windows)
#
# Usage:
#   .\build.ps1 word-tools              Build a specific extension
#   .\build.ps1 -All                    Build all extensions
#   .\build.ps1 word-tools -Debug       Build in debug mode
#
# Prerequisites:
#   rustup target add wasm32-wasip1
# ───────────────────────────────────────────────────────────────
param(
    [Parameter(Position=0)]
    [string]$ExtensionDir,

    [switch]$All,
    [switch]$Debug
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Profile = if ($Debug) { "debug" } else { "release" }

# Collect directories to build
$Dirs = @()
if ($All) {
    Get-ChildItem -Path $ScriptDir -Directory | ForEach-Object {
        if (Test-Path (Join-Path $_.FullName "Cargo.toml")) {
            $Dirs += $_.Name
        }
    }
} elseif ($ExtensionDir) {
    $Dirs += $ExtensionDir
} else {
    Write-Host "Usage: .\build.ps1 <extension-dir|-All> [-Debug]"
    Write-Host ""
    Write-Host "Available extensions:"
    Get-ChildItem -Path $ScriptDir -Directory | ForEach-Object {
        if (Test-Path (Join-Path $_.FullName "Cargo.toml")) {
            Write-Host "  $($_.Name)"
        }
    }
    exit 1
}

foreach ($DirName in $Dirs) {
    $ExtDir = Join-Path $ScriptDir $DirName

    if (-not (Test-Path (Join-Path $ExtDir "Cargo.toml"))) {
        Write-Error "Error: $ExtDir\Cargo.toml not found"
        exit 1
    }
    if (-not (Test-Path (Join-Path $ExtDir "omni-extension.toml"))) {
        Write-Error "Error: $ExtDir\omni-extension.toml not found"
        exit 1
    }

    Write-Host ""
    Write-Host "===== Building: $DirName ($Profile) =====" -ForegroundColor Cyan

    # Build the WASM binary
    $CargoArgs = @("build", "--manifest-path=$ExtDir\Cargo.toml", "--target=wasm32-wasip1")
    if ($Profile -eq "release") {
        $CargoArgs += "--release"
    }

    & cargo @CargoArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Cargo build failed"
        exit 1
    }

    # Determine output file name
    $PkgName = (Select-String -Path (Join-Path $ExtDir "Cargo.toml") -Pattern '^name\s*=' | Select-Object -First 1).Line
    $PkgName = ($PkgName -replace '.*=\s*"', '' -replace '".*', '').Replace('-', '_')

    $WasmSrc = Join-Path $ExtDir "target\wasm32-wasip1\$Profile\$PkgName.wasm"
    if (-not (Test-Path $WasmSrc)) {
        Write-Error "Expected WASM output not found at $WasmSrc"
        exit 1
    }

    # Read entrypoint from manifest
    $EntrypointLine = Select-String -Path (Join-Path $ExtDir "omni-extension.toml") -Pattern 'entrypoint' | Select-Object -First 1
    $Entrypoint = ($EntrypointLine.Line -replace '.*=\s*"', '' -replace '".*', '')
    $WasmDst = Join-Path $ExtDir $Entrypoint

    # Copy compiled WASM
    Copy-Item $WasmSrc $WasmDst -Force
    $WasmSize = (Get-Item $WasmDst).Length
    Write-Host "  [OK] Compiled: $Entrypoint ($WasmSize bytes)" -ForegroundColor Green

    # Create dist directory
    $DistDir = Join-Path $ExtDir "dist"
    if (-not (Test-Path $DistDir)) {
        New-Item -ItemType Directory -Path $DistDir | Out-Null
    }

    Write-Host "  [OK] Ready for installation" -ForegroundColor Green
    Write-Host ""
}

Write-Host "Done! To install an extension:" -ForegroundColor Yellow
Write-Host "  1. Copy the extension directory to ~/.omni/extensions/user/"
Write-Host "  2. Or use: omni extension install <path-to-dir>"
