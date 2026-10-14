param([string]$Executable = (Join-Path $PSScriptRoot '../desktop-app/llama-server-configurator.exe'))
$ErrorActionPreference = 'Stop'
$fixtureRoot = Join-Path $PSScriptRoot '../.test-fixtures/native-benchmark'
New-Item -ItemType Directory -Path $fixtureRoot -Force | Out-Null
$startInfo = [System.Diagnostics.ProcessStartInfo]::new()
$startInfo.FileName = [System.IO.Path]::GetFullPath($Executable)
$startInfo.UseShellExecute = $false
$startInfo.WindowStyle = [System.Diagnostics.ProcessWindowStyle]::Hidden
$startInfo.Environment['LOCALAPPDATA'] = [System.IO.Path]::GetFullPath($fixtureRoot)
$startInfo.Environment['WEBVIEW2_USER_DATA_FOLDER'] = Join-Path ([System.IO.Path]::GetFullPath($fixtureRoot)) 'webview'
$timer = [System.Diagnostics.Stopwatch]::StartNew()
$process = [System.Diagnostics.Process]::Start($startInfo)
try {
    while ($timer.ElapsedMilliseconds -lt 15000) {
        $process.Refresh()
        if ($process.HasExited) { throw 'Benchmark launcher exited before window creation.' }
        if ($process.MainWindowHandle -ne 0) { break }
        Start-Sleep -Milliseconds 50
    }
    if ($process.MainWindowHandle -eq 0) { throw 'Benchmark window creation timed out.' }
    $windowMs = $timer.ElapsedMilliseconds
    Start-Sleep -Seconds 3
    function Get-OwnedSamples {
        $identities = [System.Collections.Generic.HashSet[int]]::new()
        [void]$identities.Add($process.Id)
        $inventory = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId)
        do {
            $added = $false
            foreach ($item in $inventory) {
                if ($identities.Contains([int]$item.ParentProcessId) -and $identities.Add([int]$item.ProcessId)) { $added = $true }
            }
        } while ($added)
        $result = @{}
        foreach ($identity in $identities) {
            $sample = Get-Process -Id $identity -ErrorAction SilentlyContinue
            if ($sample) { $result[$identity] = @{ cpu = $sample.TotalProcessorTime.TotalSeconds; working = $sample.WorkingSet64 } }
        }
        return $result
    }
    $before = Get-OwnedSamples
    $idleTimer = [System.Diagnostics.Stopwatch]::StartNew()
    Start-Sleep -Seconds 5
    $after = Get-OwnedSamples
    $cpuSeconds = 0.0
    $workingBytes = 0L
    foreach ($identity in $after.Keys) {
        $workingBytes += $after[$identity].working
        if ($before.ContainsKey($identity)) { $cpuSeconds += $after[$identity].cpu - $before[$identity].cpu }
    }
    $report = @{
        kind = 'Windows native launcher with isolated profile, no inference'
        windowCreationMs = $windowMs
        idleSampleSeconds = $idleTimer.Elapsed.TotalSeconds
        idleCpuCoreEquivalent = $cpuSeconds / $idleTimer.Elapsed.TotalSeconds
        processCount = $after.Count
        summedWorkingSetBytes = $workingBytes
        executableBytes = (Get-Item -LiteralPath $startInfo.FileName).Length
        caveat = 'Window creation is not DOM readiness; summed working sets can count shared pages more than once. One run, not a distribution.'
    }
    $artifact = Join-Path $PSScriptRoot '../artifacts/native-benchmark.json'
    $report | ConvertTo-Json | Set-Content -LiteralPath $artifact
    $report | ConvertTo-Json -Compress
} finally {
    $process.Refresh()
    if (-not $process.HasExited) {
        [void]$process.CloseMainWindow()
        if (-not $process.WaitForExit(3000)) { $process.Kill($true); $process.WaitForExit() }
    }
    $process.Dispose()
}
