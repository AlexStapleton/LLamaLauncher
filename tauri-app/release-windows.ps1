$ErrorActionPreference='Stop'
Push-Location $PSScriptRoot
try {
    $releaseVersion=(Get-Content package.json -Raw | ConvertFrom-Json).version
    if($releaseVersion -ne '1.1.2'){throw 'This release recipe expects version 1.1.2'}
    $inputFolder=Join-Path $PSScriptRoot 'artifacts/release-inputs'
    New-Item -ItemType Directory -Path $inputFolder -Force | Out-Null
    $cab=Join-Path $inputFolder 'WebView2-fixed-x64.cab'
    $inputs=Get-Content runtime-inputs.json -Raw | ConvertFrom-Json
    $url=$inputs.webview2.url
    Invoke-WebRequest -Uri $url -OutFile $cab
    if((Get-FileHash -LiteralPath $cab -Algorithm SHA256).Hash -ine $inputs.webview2.sha256){throw 'Pinned WebView2 checksum mismatch'}
    $unpacked=Join-Path $inputFolder 'webview'
    New-Item -ItemType Directory -Path $unpacked -Force | Out-Null
    & "$env:SystemRoot/System32/expand.exe" $cab '-F:*' $unpacked | Out-Null
    if($LASTEXITCODE -ne 0){throw 'WebView2 extraction failed'}
    $webview=Get-ChildItem -LiteralPath $unpacked -Filter msedgewebview2.exe -File -Recurse | Select-Object -First 1
    if(-not $webview -or (Get-AuthenticodeSignature -LiteralPath $webview.FullName).Status -ne 'Valid'){throw 'Fixed WebView2 signature invalid'}
    if($webview.VersionInfo.FileVersion -ne $inputs.webview2.version){throw 'Fixed WebView2 version mismatch'}
    $crtOutput=& ./tests/prepare-vc-runtime.ps1
    $crtLine=@($crtOutput | Where-Object { $_ -is [string] -and $_.StartsWith('VCRUNTIME_STAGE=') })
    if($crtLine.Count -ne 1){throw 'Expected one verified VC runtime stage'}
    $crt=$crtLine[0].Substring('VCRUNTIME_STAGE='.Length)
    & ./package-windows.ps1 -FixedWebView2Directory $webview.DirectoryName -VCRuntimeDirectory $crt -ExecutablePath target/release/llama-server-configurator.exe -SkipBuild
    if($LASTEXITCODE -ne 0){throw 'Windows installer packaging failed'}
    & ./tests/windows-installer-smoke.ps1 -InstallerPath artifacts/release/LLamaLauncher-1.1.2-Windows-x86_64-Setup.exe
} finally { Pop-Location }
