param([Parameter(Mandatory=$true)][int]$RootProcessId,[Parameter(Mandatory=$true)][string]$RuntimeFolder)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class LauncherSmokeWindow {
  public delegate bool Callback(IntPtr window, IntPtr state);
  [DllImport("user32.dll")] static extern bool EnumWindows(Callback callback, IntPtr state);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr window, StringBuilder text, int count);
  [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr window, int command);
  public static IntPtr Find(int expectedPid, string title) {
    IntPtr result=IntPtr.Zero;
    EnumWindows((window, state) => {
      uint pid; GetWindowThreadProcessId(window,out pid);
      if(pid != expectedPid) return true;
      var text=new StringBuilder(512); GetWindowText(window,text,text.Capacity);
      if(text.ToString()==title) {result=window;return false;} return true;
    },IntPtr.Zero);
    return result;
  }
}
'@
$title=(Get-Content -LiteralPath (Join-Path $PSScriptRoot '../tauri.conf.json') -Raw | ConvertFrom-Json).app.windows[0].title
$expectedBrowser=Join-Path $RuntimeFolder 'msedgewebview2.exe'
$deadline=[DateTime]::UtcNow.AddSeconds(45)
$lastNames=@();$lastError='';$shown=$false
while([DateTime]::UtcNow -lt $deadline) {
  $owned=Get-Process -Id $RootProcessId -ErrorAction SilentlyContinue
  if(-not $owned){throw 'Owned launcher exited before native window readiness'}
  try {
    $handle=[LauncherSmokeWindow]::Find($RootProcessId,$title)
    if($handle -ne [IntPtr]::Zero) {
      # Only the isolated app window on the hosted virtual desktop is shown.
      if($env:GITHUB_ACTIONS -eq 'true' -and -not $shown){[void][LauncherSmokeWindow]::ShowWindowAsync($handle,5);$shown=$true}
      $window=[System.Windows.Automation.AutomationElement]::FromHandle($handle)
      $elements=$window.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)
      $lastNames=@($elements | ForEach-Object {$_.Current.Name} | Where-Object {$_})
      $ready=@($lastNames | Where-Object {$_ -like '*Ready. Choose a GGUF in Models*'}).Count -gt 0
      $start=$lastNames -contains 'Start server'
      if($ready -and $start){
        $browsers=@(Get-CimInstance Win32_Process -Filter "ParentProcessId = $RootProcessId" | Where-Object {$_.Name -eq 'msedgewebview2.exe'})
        $private=@($browsers | Where-Object {[string]::Equals($_.ExecutablePath,$expectedBrowser,[StringComparison]::OrdinalIgnoreCase)}).Count -gt 0
        if(-not $private){throw 'Native browser is not using the packaged fixed runtime'}
        @{ready=$true;startButton=$true;packagedRuntime=$true;title=$title;method='Windows UI Automation'} | ConvertTo-Json -Compress
        exit 0
      }
    }
  }catch{$lastError=$_.Exception.Message}
  Start-Sleep -Milliseconds 250
}
throw "Native window readiness failed: $lastError; accessible text: $($lastNames | Select-Object -First 100 | ConvertTo-Json -Compress)"
