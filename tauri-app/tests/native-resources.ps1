param([Parameter(Mandatory=$true)][int]$RootProcessId)
$ErrorActionPreference='Stop'
$rows=@(Get-CimInstance Win32_Process)
$owned=[System.Collections.Generic.HashSet[int]]::new()
$null=$owned.Add($RootProcessId)
do {
    $added=$false
    foreach($row in $rows){if($owned.Contains([int]$row.ParentProcessId) -and $owned.Add([int]$row.ProcessId)){$added=$true}}
} while($added)
$selected=@($rows | Where-Object {$owned.Contains([int]$_.ProcessId)})
$root=@($selected | Where-Object {$_.ProcessId -eq $RootProcessId})
$rootServer=@($selected | Where-Object {$_.ParentProcessId -eq $RootProcessId -and $_.Name -eq 'llama-server.exe'})
$cpu=0.0;$ram=0.0;$read=0.0;$write=0.0
foreach($row in $selected){$cpu+=([double]$row.KernelModeTime+[double]$row.UserModeTime)/10000000;$ram+=[double]$row.WorkingSetSize;$read+=[double]$row.ReadTransferCount;$write+=[double]$row.WriteTransferCount}
[ordered]@{timestamp=[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds();processes=$selected.Count;cpuSeconds=$cpu;workingSetBytes=$ram;rootWorkingSetBytes=[double]$root.WorkingSetSize;rootServerWorkingSetBytes=[double]$rootServer.WorkingSetSize;readBytes=$read;writeBytes=$write} | ConvertTo-Json -Compress
