param(
    [Parameter(Mandatory=$true)][string]$FixedWebView2Directory,
    [string]$ExecutablePath='target/release/llama-server-configurator.exe',
    [string]$VCRuntimeDirectory,
    [string]$MakeNsis,
    [switch]$SkipBuild
)
$ErrorActionPreference='Stop'
Push-Location $PSScriptRoot
try {
    if (-not $MakeNsis) {
        $candidate=Get-Command makensis.exe -ErrorAction SilentlyContinue
        if ($candidate) { $MakeNsis=$candidate.Source }
        else { $MakeNsis=Join-Path ${env:ProgramFiles(x86)} 'NSIS/makensis.exe' }
    }
    if (-not (Test-Path -LiteralPath $MakeNsis)) { throw 'Install NSIS 3 (makensis.exe) before packaging.' }
    $stageOutput=& ./package-portable.ps1 -FixedWebView2Directory $FixedWebView2Directory -ExecutablePath $ExecutablePath -VCRuntimeDirectory $VCRuntimeDirectory -SkipBuild:$SkipBuild -StageOnly
    $stageLine=@($stageOutput | Where-Object { $_ -is [string] -and $_.StartsWith('PACKAGE_STAGE=') })
    if ($stageLine.Count -ne 1) { throw 'Expected exactly one fresh package stage' }
    $stage=$stageLine[0].Substring('PACKAGE_STAGE='.Length)
    if ((Test-Path -LiteralPath (Join-Path $stage 'data')) -or (Test-Path -LiteralPath (Join-Path $stage 'test-legacy-context'))) { throw 'Refusing to package user/test data' }
    $version=(Get-Content package.json -Raw | ConvertFrom-Json).version
    if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Invalid release version' }
    $outputFolder=Join-Path $PSScriptRoot 'artifacts/release'
    New-Item -ItemType Directory -Path $outputFolder -Force | Out-Null
    $output=Join-Path $outputFolder "LLamaLauncher-$version-Windows-x86_64.exe"
    $icon=(Resolve-Path icons/icon.ico).Path
    & $MakeNsis /V2 /WX "/DSTAGE=$stage" "/DOUTPUT=$output" "/DVERSION=$version" "/DICON=$icon" installer/windows.nsi
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $output)) { throw 'Portable executable build failed' }
    "$((Get-FileHash -LiteralPath $output -Algorithm SHA256).Hash.ToLower())  $(Split-Path $output -Leaf)" | Set-Content -LiteralPath "$output.sha256" -Encoding utf8
    Write-Output "Portable Windows executable: $output"
} finally { Pop-Location }
