$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    npm test
    if ($LASTEXITCODE -ne 0) { throw 'Frontend tests failed.' }
    npm run build
    if ($LASTEXITCODE -ne 0) { throw 'Frontend build failed.' }
    cargo +stable-x86_64-pc-windows-msvc check --locked
    if ($LASTEXITCODE -ne 0) { throw 'Rust check failed.' }
    cargo +stable-x86_64-pc-windows-msvc build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Windows GUI build failed. Close the application before rebuilding.' }
    $output = Join-Path $PSScriptRoot 'desktop-app'
    New-Item -ItemType Directory -Path $output -Force | Out-Null
    $buildTarget = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $PSScriptRoot 'target' }
    Copy-Item -LiteralPath (Join-Path $buildTarget 'release/llama-server-configurator.exe') -Destination $output -Force
    Write-Output "Windows GUI executable: $output\llama-server-configurator.exe"
} finally {
    Pop-Location
}
