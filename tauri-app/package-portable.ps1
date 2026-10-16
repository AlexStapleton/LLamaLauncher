param(
    [Parameter(Mandatory=$true)][string]$FixedWebView2Directory,
    [string]$VCRuntimeDirectory,
    [string]$ExecutablePath,
    [switch]$SkipBuild,
    [switch]$StageOnly
)
$ErrorActionPreference = 'Stop'
Push-Location $PSScriptRoot
try {
    $webviewSource = (Resolve-Path -LiteralPath $FixedWebView2Directory).Path
    if (-not (Test-Path -LiteralPath (Join-Path $webviewSource 'msedgewebview2.exe'))) { throw 'Select the extracted x64 Fixed Version WebView2 folder from Microsoft.' }
    if (-not $SkipBuild) { & ./build-windows.ps1; if ($LASTEXITCODE -ne 0) { throw 'Build failed.' } }
    & node tests/verify-glib-backport.mjs
    if ($LASTEXITCODE -ne 0) { throw 'Dependency backport verification failed.' }
    & node tests/generate-notices.mjs
    if ($LASTEXITCODE -ne 0) { throw 'Dependency notice generation failed.' }
    & node tests/generate-settings-guide.mjs --check
    if ($LASTEXITCODE -ne 0) { throw 'Settings reference is stale.' }
    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss-fff'
    $stage = Join-Path $PSScriptRoot "artifacts/portable-$stamp/LLamaLauncher"
    New-Item -ItemType Directory -Path $stage -Force | Out-Null
    if (-not $ExecutablePath) { $ExecutablePath='./desktop-app/llama-server-configurator.exe' }
    $executableSource=(Resolve-Path -LiteralPath $ExecutablePath).Path
    Copy-Item -LiteralPath $executableSource -Destination (Join-Path $stage 'llama-server-configurator.exe')
    Copy-Item -LiteralPath $webviewSource -Destination (Join-Path $stage 'WebView2') -Recurse
    if (-not $VCRuntimeDirectory) {
        $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
        if (Test-Path -LiteralPath $vswhere) {
            $visualStudio = & $vswhere -latest -products '*' -property installationPath
            $redist = Join-Path $visualStudio 'VC/Redist/MSVC'
            $VCRuntimeDirectory = Get-ChildItem -LiteralPath $redist -Directory | Where-Object { $_.Name -match '^\d' } | Sort-Object Name -Descending | ForEach-Object { Join-Path $_.FullName 'x64/Microsoft.VC143.CRT' } | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
        }
    }
    if ($VCRuntimeDirectory) {
        $crtSource = (Resolve-Path -LiteralPath $VCRuntimeDirectory).Path
        foreach ($library in Get-ChildItem -LiteralPath $crtSource -Filter '*.dll' -File) {
            Copy-Item -LiteralPath $library.FullName -Destination $stage
        }
    } else { throw 'Supply the licensed x64 Microsoft VC runtime redistributable folder with -VCRuntimeDirectory.' }
    Copy-Item -LiteralPath '../LICENSE','../PORTABLE_RELEASES.md','../LLAMA_SETTINGS_GUIDE.md' -Destination $stage
    Copy-Item -LiteralPath 'artifacts/THIRD_PARTY_NOTICES.txt' -Destination $stage
    @'
Extract the entire folder, then run llama-server-configurator.exe.
The private WebView2 folder must remain next to the executable.
In Launch, browse to an existing llama-server.exe or choose Stable/Beta and press
Download llama.cpp. The platform/hardware build is selected automatically.
Models are not included. Use Models to choose them.
Settings has separate Context & cache and Chat & output tabs. Open Help & resource
impact for explanations and examples, or read LLAMA_SETTINGS_GUIDE.md offline.
Profiles, settings, installed engines, default model downloads, and logs live in
the data folder beside the EXE. Keep data when updating. Windows private records
remain protected by your Windows account.
Choose a GGUF in Models and press Use for single-model launch, then Start server.
'@ | Set-Content -LiteralPath (Join-Path $stage 'START_HERE.txt') -Encoding utf8
    Get-ChildItem -LiteralPath $stage -File -Recurse | ForEach-Object {
        $relative = $_.FullName.Substring($stage.Length + 1)
        "$( (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLower() )  $relative"
    } | Set-Content -LiteralPath (Join-Path $stage 'SHA256SUMS.txt') -Encoding utf8
    if ($StageOnly) { Write-Output "PACKAGE_STAGE=$stage"; return }
    $archive = Join-Path $PSScriptRoot "artifacts/LLamaLauncher-Windows-x64-$stamp.zip"
    Compress-Archive -LiteralPath $stage -DestinationPath $archive -CompressionLevel Optimal
    "$( (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLower() )  $(Split-Path -Leaf $archive)" | Set-Content -LiteralPath "$archive.sha256" -Encoding utf8
    Write-Output "Portable package: $archive"
} finally { Pop-Location }
