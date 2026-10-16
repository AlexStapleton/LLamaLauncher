param([Parameter(Mandatory=$true)][string]$ExecutablePath,[string]$MakeNsis)
$ErrorActionPreference='Stop'
$source=(Resolve-Path -LiteralPath $ExecutablePath).Path
$fixture=Join-Path $PSScriptRoot "../artifacts/portable-exe-smoke-$([guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $fixture -Force | Out-Null
$fixture=(Resolve-Path -LiteralPath $fixture).Path
$folder=Join-Path $fixture 'Portable app with spaces'
$temporary=Join-Path $fixture 'Temporary runtime'
$unrelated=Join-Path $fixture 'Unrelated working directory'
New-Item -ItemType Directory -Path $folder,$temporary,$unrelated | Out-Null
$portable=Join-Path $folder 'LLamaLauncher.exe'
Copy-Item -LiteralPath $source -Destination $portable
$registration='HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\LLamaLauncher'
$beforeRegistration=if(Test-Path -LiteralPath $registration){Get-ItemProperty -LiteralPath $registration | ConvertTo-Json -Compress}else{''}
$shortcut=Join-Path ([Environment]::GetFolderPath('Desktop')) 'LLamaLauncher.lnk'
$beforeShortcut=if(Test-Path -LiteralPath $shortcut){(Get-FileHash -LiteralPath $shortcut).Hash}else{''}
$env:CI='true'
function Check-PortableGui {
    $info=[Diagnostics.ProcessStartInfo]::new($portable)
    $info.UseShellExecute=$false
    $info.CreateNoWindow=$true
    $info.WorkingDirectory=$unrelated
    $info.Environment['TEMP']=$temporary
    $info.Environment['TMP']=$temporary
    $info.Environment['LOCALAPPDATA']=(Join-Path $fixture 'Isolated legacy state')
    $info.Environment['WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS']='--disable-gpu --no-sandbox --force-renderer-accessibility'
    foreach($name in @('LLAMA_LAUNCHER_DATA_DIR','LLAMA_LAUNCHER_PORTABLE_EXE','WEBVIEW2_BROWSER_EXECUTABLE_FOLDER','WEBVIEW2_USER_DATA_FOLDER','LLAMA_MCP_PROFILE','HF_TOKEN')){[void]$info.Environment.Remove($name)}
    $wrapper=[Diagnostics.Process]::Start($info)
    $gui=$null
    try {
        $deadline=[DateTime]::UtcNow.AddSeconds(90)
        do {
            if($wrapper.HasExited){throw "Portable container exited before GUI startup: $($wrapper.ExitCode)"}
            $gui=Get-CimInstance Win32_Process -Filter "ParentProcessId = $($wrapper.Id)" | Where-Object {$_.Name -eq 'llama-server-configurator.exe'} | Select-Object -First 1
            if(-not $gui){Start-Sleep -Milliseconds 250}
        }while(-not $gui -and [DateTime]::UtcNow -lt $deadline)
        if(-not $gui){throw 'Portable container did not start its GUI'}
        $runtimeFolder=Split-Path $gui.ExecutablePath -Parent
        if(-not $runtimeFolder.StartsWith($temporary+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Runtime was not extracted into the isolated temporary folder'}
        foreach($name in @('THIRD_PARTY_NOTICES.txt','LLAMA_SETTINGS_GUIDE.md','WINDOWS_PACKAGE_VERSIONS.json','WebView2/msedgewebview2.exe')){
            if(-not(Test-Path -LiteralPath (Join-Path $runtimeFolder $name))){throw "Missing embedded file: $name"}
        }
        $probe=& powershell.exe -NoProfile -STA -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'native-window-ready.ps1') -RootProcessId $gui.ProcessId -RuntimeFolder (Join-Path $runtimeFolder 'WebView2')
        if($LASTEXITCODE -ne 0){throw 'Portable GUI readiness failed'}
        $report=$probe | ConvertFrom-Json
        if(-not $report.ready -or -not $report.packagedRuntime){throw 'GUI did not use its embedded runtime'}
        $data=Join-Path $folder 'data'
        $deadline=[DateTime]::UtcNow.AddSeconds(15)
        while(-not(Test-Path -LiteralPath (Join-Path $data 'working-settings.bin')) -and [DateTime]::UtcNow -lt $deadline){Start-Sleep -Milliseconds 200}
        if(-not(Test-Path -LiteralPath (Join-Path $data 'working-settings.bin')) -or -not(Test-Path -LiteralPath (Join-Path $data 'webview'))){throw 'Settings were not saved beside the original EXE'}
        if((Test-Path -LiteralPath (Join-Path $runtimeFolder 'data')) -or (Test-Path -LiteralPath (Join-Path $unrelated 'data'))){throw 'Settings leaked into temporary or working-directory storage'}
        $ownedGui=Get-Process -Id $gui.ProcessId
        if(-not $ownedGui.CloseMainWindow()){throw 'Could not close the owned GUI normally'}
        if(-not $wrapper.WaitForExit(120000)){throw 'Portable container did not finish after GUI close'}
        if($wrapper.ExitCode -ne 0){throw "Portable launcher returned $($wrapper.ExitCode)"}
        if(Test-Path -LiteralPath $runtimeFolder){
            $remaining=@(Get-ChildItem -LiteralPath $runtimeFolder -File -Recurse | Select-Object -First 12 -ExpandProperty Name)
            throw "Normal exit did not clean its temporary runtime; remaining files: $($remaining -join ', ')"
        }
    } finally {
        # Only the owned fixture tree can be stopped on failure. Normal success
        # uses the app's close handshake and lets the container clean itself.
        if(-not $wrapper.HasExited){& "$env:SystemRoot/System32/taskkill.exe" /PID $wrapper.Id /T /F | Out-Null}
    }
}
Check-PortableGui
$marker=Join-Path $folder 'data/keep-user-model.gguf'
[IO.File]::WriteAllText($marker,'Synthetic user-owned marker, not model weights.')
$preserved=Join-Path $folder 'keep-unrelated.txt'
[IO.File]::WriteAllText($preserved,'Preserve unrecognized files.')
# Rename the single download and start again from a different working directory.
$renamed=Join-Path $folder 'Renamed launcher.exe'
Rename-Item -LiteralPath $portable -NewName 'Renamed launcher.exe'
$portable=$renamed
Check-PortableGui
if([IO.File]::ReadAllText($marker) -ne 'Synthetic user-owned marker, not model weights.' -or -not(Test-Path -LiteralPath $preserved)){throw 'Relaunch modified user-owned files'}
$afterRegistration=if(Test-Path -LiteralPath $registration){Get-ItemProperty -LiteralPath $registration | ConvertTo-Json -Compress}else{''}
$afterShortcut=if(Test-Path -LiteralPath $shortcut){(Get-FileHash -LiteralPath $shortcut).Hash}else{''}
if($afterRegistration -cne $beforeRegistration -or $afterShortcut -cne $beforeShortcut){throw 'Portable launch changed installer registration or desktop shortcuts'}
$unexpected=@(Get-ChildItem -LiteralPath $folder | Where-Object {$_.Name -notin @('Renamed launcher.exe','data','keep-unrelated.txt')})
if($unexpected.Count){throw 'Portable launch left unpacked application files beside the EXE'}
# Exercise the exact container's recovery exit-code loop independently of UI
# automation. This synthetic payload can request one restart and verify that its
# extracted executable still exists in the second process.
if(-not $MakeNsis){
    $candidate=Get-Command makensis.exe -ErrorAction SilentlyContinue
    $MakeNsis=if($candidate){$candidate.Source}else{Join-Path ${env:ProgramFiles(x86)} 'NSIS/makensis.exe'}
}
$restartStage=Join-Path $fixture 'Restart fixture'
New-Item -ItemType Directory -Path $restartStage | Out-Null
$fixtureSource=Join-Path $fixture 'RestartFixture.cs'
@'
using System;
using System.IO;
public static class RestartFixture {
  public static int Main() {
    string original=Environment.GetEnvironmentVariable("LLAMA_LAUNCHER_PORTABLE_EXE");
    if(String.IsNullOrEmpty(original)||!File.Exists(original)) return 10;
    string marker=Path.Combine(Path.GetDirectoryName(original),"restart-proof.txt");
    string current=System.Reflection.Assembly.GetExecutingAssembly().Location;
    if(!File.Exists(current)) return 11;
    if(!File.Exists(marker)){File.WriteAllText(marker,current);return 75;}
    if(File.ReadAllText(marker)!=current) return 12;
    File.AppendAllText(marker,"\nrestarted");
    return 0;
  }
}
'@ | Set-Content -LiteralPath $fixtureSource -Encoding utf8
$compiler=Join-Path $env:SystemRoot 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
& $compiler /nologo /target:winexe "/out:$restartStage/llama-server-configurator.exe" $fixtureSource
if($LASTEXITCODE -ne 0){throw 'Cannot compile synthetic restart fixture'}
$restartExe=Join-Path $fixture 'Restart container.exe'
$recipe=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../installer/windows.nsi')).Path
$icon=(Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../icons/icon.ico')).Path
& $MakeNsis /V2 /WX "/DSTAGE=$restartStage" "/DOUTPUT=$restartExe" /DVERSION=1.0.0 "/DICON=$icon" $recipe
if($LASTEXITCODE -ne 0){throw 'Cannot compile restart container fixture'}
$restartProcess=Start-Process -FilePath $restartExe -WindowStyle Hidden -PassThru
if(-not $restartProcess.WaitForExit(30000)){ $restartProcess.Kill(); throw 'Portable restart loop did not finish' }
if($restartProcess.ExitCode -ne 0){throw "Portable restart loop failed: $($restartProcess.ExitCode)"}
$proof=[IO.File]::ReadAllLines((Join-Path $fixture 'restart-proof.txt'))
if($proof.Count -ne 2 -or $proof[1] -ne 'restarted' -or (Test-Path -LiteralPath $proof[0])){throw 'Container failed to preserve runtime across restart or clean it after final exit'}
Write-Output 'Single portable EXE, real embedded-runtime GUI, adjacent data, rename/relaunch preservation, recovery restart loop, temporary cleanup, and no install registration passed.'
