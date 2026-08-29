$env:CARGO_TARGET_DIR = "c:\Users\USA\Documents\Osuocha\grainlify\target-final"
Set-Location "c:\Users\USA\Documents\Osuocha\grainlify\contracts\grainlify-core"
Write-Host "=== Running cargo test governance -- --nocapture ==="
& cargo test governance -- --nocapture
Write-Host "=== DONE. ExitCode=$LASTEXITCODE ==="
