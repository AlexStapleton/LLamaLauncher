$ErrorActionPreference='Stop'
Push-Location $PSScriptRoot
try {
    $releaseVersion=(Get-Content package.json -Raw | ConvertFrom-Json).version
    if($releaseVersion -ne '1.1.0'){throw 'This release recipe expects version 1.1.0'}
    $inputFolder=Join-Path $PSScriptRoot 'artifacts/release-inputs'
    New-Item -ItemType Directory -Path $inputFolder -Force | Out-Null
    $cab=Join-Path $inputFolder 'WebView2-fixed-x64.cab'
    $url='https://msedge.sf.dl.delivery.mp.microsoft.com/filestreamingservice/files/01b0fbd9-6ea3-4e12-818a-a71f73f3855b/Microsoft.WebView2.FixedVersionRuntime.155.0.4283.45.x64.cab'
    Invoke-WebRequest -Uri $url -OutFile $cab
    if((Get-FileHash -LiteralPath $cab -Algorithm SHA256).Hash -ne 'ec533bf1b7354d288703cf821a2d640d0130108224f93e330bb9c09faf3f8010'){throw 'Pinned WebView2 checksum mismatch'}
    $unpacked=Join-Path $inputFolder 'webview'
    New-Item -ItemType Directory -Path $unpacked -Force | Out-Null
    & "$env:SystemRoot/System32/expand.exe" $cab '-F:*' $unpacked | Out-Null
    if($LASTEXITCODE -ne 0){throw 'WebView2 extraction failed'}
    $webview=Get-ChildItem -LiteralPath $unpacked -Filter msedgewebview2.exe -File -Recurse | Select-Object -First 1
    if(-not $webview -or (Get-AuthenticodeSignature -LiteralPath $webview.FullName).Status -ne 'Valid'){throw 'Fixed WebView2 signature invalid'}
    & ./package-portable.ps1 -FixedWebView2Directory $webview.DirectoryName -ExecutablePath target/release/llama-server-configurator.exe -SkipBuild
    if($LASTEXITCODE -ne 0){throw 'Portable packaging failed'}
    $package=Get-ChildItem artifacts -Filter 'LLamaLauncher-Windows-x64-*.zip' -File | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $stamp=$package.BaseName.Replace('LLamaLauncher-Windows-x64-','')
    $env:LLAMA_LAUNCHER_EXE=(Resolve-Path "artifacts/portable-$stamp/LLamaLauncher/llama-server-configurator.exe").Path
    & node tests/portable-package-smoke.mjs
    if($LASTEXITCODE -ne 0){throw 'Packaged GUI smoke failed'}
    New-Item -ItemType Directory -Path artifacts/release -Force | Out-Null
    $name='LLamaLauncher-1.1-Windows-x86_64.zip'
    Copy-Item -LiteralPath $package.FullName -Destination (Join-Path 'artifacts/release' $name)
    "$((Get-FileHash -LiteralPath $package.FullName -Algorithm SHA256).Hash.ToLower())  $name" | Set-Content -LiteralPath "artifacts/release/$name.sha256" -Encoding utf8
} finally { Pop-Location }
