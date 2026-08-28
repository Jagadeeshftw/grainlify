# Build wasm artifacts from a clean target and generate a sha256 manifest (PowerShell wrapper)
param(
    [string]$WorkspaceRoot = ".",
    [string]$Out = "artifacts"
)

Set-StrictMode -Version Latest

# find manifest
if (Test-Path -Path (Join-Path $WorkspaceRoot 'Cargo.toml')) {
    $manifest = Join-Path $WorkspaceRoot 'Cargo.toml'
} elseif (Test-Path -Path (Join-Path $WorkspaceRoot 'grainlify\Cargo.toml')) {
    $manifest = Join-Path $WorkspaceRoot 'grainlify\Cargo.toml'
} else {
    Write-Error "Could not find workspace Cargo.toml in $WorkspaceRoot or $WorkspaceRoot\grainlify"
    exit 2
}

# require git
$gitOk = & git -C $WorkspaceRoot rev-parse --short HEAD 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Error "Workspace is not a git repository or git not available"
    exit 2
}

Write-Host "Cleaning workspace targets..."
Push-Location $WorkspaceRoot
try {
    & cargo clean
} finally { Pop-Location }

Write-Host "Building wasm artifacts (release)..."
Push-Location $WorkspaceRoot
try {
    & cargo build --manifest-path $manifest --workspace --target wasm32-unknown-unknown --release
} finally { Pop-Location }

Write-Host "Generating manifest and copying artifacts to $Out"
python "$WorkspaceRoot\scripts\generate_manifest.py" --workspace-root $WorkspaceRoot --out $Out --manifest-name sha256-manifest.txt
Write-Host "Done. Manifest at $Out\sha256-manifest.txt"
