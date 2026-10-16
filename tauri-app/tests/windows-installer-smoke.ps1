param([Parameter(Mandatory=$true)][string]$InstallerPath)
$ErrorActionPreference='Stop'
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'Run this installer/shortcut test only on an isolated GitHub Actions Windows runner.' }
$installer=(Resolve-Path -LiteralPath $InstallerPath).Path
$fixture=Join-Path $PSScriptRoot "../artifacts/installer-smoke-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $fixture -Force | Out-Null
$fixture=(Resolve-Path -LiteralPath $fixture).Path
$installed=Join-Path $fixture 'Installed launcher'
function Install-Package {
    $process=Start-Process -FilePath $installer -ArgumentList "/S /D=$installed" -WindowStyle Hidden -PassThru
    if (-not $process.WaitForExit(300000)) { $process.Kill(); throw 'Installer exceeded five minutes' }
    if ($process.ExitCode -ne 0) { throw "Installer failed: $($process.ExitCode)" }
}
Install-Package
$uninstallKey='HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\LLamaLauncher'
$uninstallExe=Join-Path $installed 'Uninstall.exe'
function Check-RegisteredUninstall {
    $registration=Get-ItemProperty -LiteralPath $uninstallKey
    if ($registration.UninstallString -cne ('"'+$uninstallExe+'"') -or $registration.QuietUninstallString -cne ('"'+$uninstallExe+'" /S')) { throw 'Windows Installed apps uninstall command is incorrectly registered' }
    if ($registration.InstallLocation -cne $installed) { throw 'Windows Installed apps points to another installation' }
}
Check-RegisteredUninstall
$env:LLAMA_LAUNCHER_EXE=Join-Path $installed 'llama-server-configurator.exe'
$env:CI='true'
& node (Join-Path $PSScriptRoot 'portable-package-smoke.mjs')
if ($LASTEXITCODE -ne 0) { throw 'Installed GUI did not initialize' }
$settings=Join-Path $installed 'data/working-settings.bin'
$digest=(Get-FileHash -LiteralPath $settings -Algorithm SHA256).Hash
$marker=Join-Path $installed 'data/keep-user-model.gguf'
[IO.File]::WriteAllText($marker,'Synthetic user-owned marker, not model weights.')
$unrelated=Join-Path $installed 'keep-unrelated.txt'
[IO.File]::WriteAllText($unrelated,'Preserve unrecognized files.')
# Allow the smoke test's owned process and WebView children to finish closing.
Start-Sleep -Seconds 2
Install-Package
Check-RegisteredUninstall
if ((Get-FileHash -LiteralPath $settings -Algorithm SHA256).Hash -ne $digest -or -not (Test-Path -LiteralPath $marker)) { throw 'Upgrade changed saved data' }
$registered=(Get-ItemProperty -LiteralPath $uninstallKey).UninstallString
$process=Start-Process -FilePath $registered.Substring(1,$registered.Length-2) -ArgumentList "/S _?=$installed" -WindowStyle Hidden -PassThru
if (-not $process.WaitForExit(120000)) { $process.Kill(); throw 'Uninstaller exceeded two minutes' }
if ($process.ExitCode -ne 0) { throw "Uninstaller failed: $($process.ExitCode)" }
if ((Test-Path -LiteralPath $env:LLAMA_LAUNCHER_EXE) -or (Test-Path -LiteralPath (Join-Path $installed 'WebView2/msedgewebview2.exe'))) { throw 'Uninstall left packaged application files' }
if ((Get-FileHash -LiteralPath $settings -Algorithm SHA256).Hash -ne $digest -or -not (Test-Path -LiteralPath $marker) -or -not (Test-Path -LiteralPath $unrelated)) { throw 'Uninstall removed or changed user files' }
if (Test-Path -LiteralPath $uninstallKey) { throw 'Uninstall registration was not removed' }
Write-Output 'Installer, real GUI startup, Windows Installed apps registration, upgrade preservation, and safe registered uninstall passed.'
