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
    $output=Join-Path $outputFolder "LLamaLauncher-$version-Windows-x86_64-Setup.exe"
    $owned=Join-Path (Split-Path $stage -Parent) 'owned-files.nsh'
    function Escape-Nsis([string]$value) { return $value.Replace('$','$$').Replace('"','$\"') }
    $lines=@(Get-ChildItem -LiteralPath $stage -File -Recurse | ForEach-Object {
        $relative=$_.FullName.Substring($stage.Length+1)
        if ($relative -match '^(data|test-legacy-context)(\\|$)' -or $relative.Contains('..')) { throw 'Unexpected package path' }
        'Delete "$INSTDIR\'+(Escape-Nsis $relative)+'"'
    })
    $lines+=@(Get-ChildItem -LiteralPath $stage -Directory -Recurse | Sort-Object { $_.FullName.Length } -Descending | ForEach-Object {
        'RMDir "$INSTDIR\'+(Escape-Nsis $_.FullName.Substring($stage.Length+1))+'"'
    })
    $lines | Set-Content -LiteralPath $owned -Encoding utf8
    $icon=(Resolve-Path icons/icon.ico).Path
    & $MakeNsis /V2 "/DSTAGE=$stage" "/DOUTPUT=$output" "/DVERSION=$version" "/DOWNED_FILES=$owned" "/DICON=$icon" installer/windows.nsi
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $output)) { throw 'NSIS setup build failed' }
    "$((Get-FileHash -LiteralPath $output -Algorithm SHA256).Hash.ToLower())  $(Split-Path $output -Leaf)" | Set-Content -LiteralPath "$output.sha256" -Encoding utf8
    Write-Output "Windows setup: $output"
} finally { Pop-Location }
