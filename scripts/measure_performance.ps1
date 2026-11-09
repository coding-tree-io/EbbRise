param(
    [string]$Exe = "$PSScriptRoot\..\target\release\ebbrise.exe",
    [string]$Stretchly = "C:\Program Files\Stretchly\Stretchly.exe",
    [int]$Runs = 5,
    [int]$IdleSeconds = 60,
    [string]$Out = "$env:TEMP\ebbrise-perf.json"
)
# Measures EbbRise (and, when installed, Stretchly) on this PC: start time, idle memory and
# CPU, the settings window, and a Mini break. EbbRise runs with a throwaway %APPDATA%, so the
# user's data is not touched. Stretchly runs with its own config and is closed at the end.
# Results go to $Out as JSON; docs/performance.md records one run.
$ErrorActionPreference = "Stop"
Add-Type @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class P {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string c, string t);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  public static string[] Visible(uint[] pids) {
    var list = new System.Collections.Generic.List<string>();
    EnumWindows((h, l) => {
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (IsWindowVisible(h) && Array.IndexOf(pids, pid) >= 0) {
        var s = new StringBuilder(256); GetWindowText(h, s, 256);
        if (s.Length > 0) list.Add(s.ToString());
      }
      return true;
    }, IntPtr.Zero);
    return list.ToArray();
  }
}
"@

function Wait-Until([scriptblock]$Test, [int]$TimeoutMs = 15000) {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while ($watch.ElapsedMilliseconds -lt $TimeoutMs) {
        if (& $Test) { return $watch.Elapsed.TotalMilliseconds }
        Start-Sleep -Milliseconds 5
    }
    return $null
}

# Memory in MB and CPU seconds, summed over every process of the app.
function Get-Usage([string]$Name) {
    $procs = @(Get-Process -Name $Name -ErrorAction SilentlyContinue)
    [pscustomobject]@{
        processes  = $procs.Count
        working_mb = [math]::Round(($procs | Measure-Object WorkingSet64 -Sum).Sum / 1MB, 1)
        private_mb = [math]::Round(($procs | Measure-Object PrivateMemorySize64 -Sum).Sum / 1MB, 1)
        cpu_s      = ($procs | ForEach-Object { $_.TotalProcessorTime.TotalSeconds } | Measure-Object -Sum).Sum
        handles    = ($procs | Measure-Object HandleCount -Sum).Sum
        threads    = ($procs | ForEach-Object { $_.Threads.Count } | Measure-Object -Sum).Sum
    }
}

# Average CPU in percent of one logical core over $Seconds, per process, so a process that
# starts or ends during the sample does not skew it (an ended one's last slice is lost).
function Measure-Cpu([string]$Name, [int]$Seconds) {
    $before = @{}
    Get-Process -Name $Name -ErrorAction SilentlyContinue | ForEach-Object { $before[$_.Id] = $_.TotalProcessorTime.TotalSeconds }
    Start-Sleep -Seconds $Seconds
    $used = 0.0
    Get-Process -Name $Name -ErrorAction SilentlyContinue | ForEach-Object {
        $start = if ($before.ContainsKey($_.Id)) { $before[$_.Id] } else { 0.0 }
        $used += $_.TotalProcessorTime.TotalSeconds - $start
    }
    [math]::Round($used / $Seconds * 100, 3)
}

function Get-Pids([string]$Name) { [uint32[]]@(Get-Process -Name $Name -ErrorAction SilentlyContinue | ForEach-Object Id) }

$result = [ordered]@{}
$data = Join-Path $env:TEMP "ebbrise-perf-$PID"
Remove-Item $data -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force "$data\EbbRise" | Out-Null
# Natural breaks off: an idle PC holds breaks, so `mini` would open nothing during an
# unattended run.
'{"miniBreakDuration":20000,"fullscreen":false,"checkNewVersion":false,"naturalBreaks":false}' | Set-Content "$data\EbbRise\config.json"
$realAppData = $env:APPDATA
$env:APPDATA = $data
try {
    if (Get-Process ebbrise -ErrorAction SilentlyContinue) { throw "close the running EbbRise first" }
    $result.exe_mb = [math]::Round((Get-Item $Exe).Length / 1MB, 2)

    # Start: process launch until the signal window exists, which is when commands work.
    $starts = foreach ($i in 1..$Runs) {
        $p = Start-Process $Exe -PassThru
        $ms = Wait-Until { [P]::FindWindow("EbbRiseSignals", [NullString]::Value) -ne [IntPtr]::Zero }
        Start-Sleep -Seconds 1
        Stop-Process -Id $p.Id -Force; $p.WaitForExit()
        Start-Sleep -Milliseconds 500
        [math]::Round($ms, 1)
    }
    $result.ebbrise_start_ms = $starts

    $p = Start-Process $Exe -PassThru
    Wait-Until { [P]::FindWindow("EbbRiseSignals", [NullString]::Value) -ne [IntPtr]::Zero } | Out-Null
    Start-Sleep -Seconds 10
    $result.ebbrise_idle = Get-Usage ebbrise
    $result.ebbrise_idle_cpu_pct = Measure-Cpu ebbrise $IdleSeconds

    $watch = [Diagnostics.Stopwatch]::StartNew()
    Start-Process $Exe -ArgumentList "preferences" -Wait
    $ms = Wait-Until { (Get-Pids ebbrise).Count -gt 0 -and [P]::FindWindow([NullString]::Value, "EbbRise settings") -ne [IntPtr]::Zero }
    $result.ebbrise_settings_open_ms = [math]::Round($watch.Elapsed.TotalMilliseconds, 1)
    Start-Sleep -Seconds 3
    $result.ebbrise_settings = Get-Usage ebbrise
    $result.ebbrise_settings_cpu_pct = Measure-Cpu ebbrise 10
    [P]::PostMessage([P]::FindWindow([NullString]::Value, "EbbRise settings"), 0x10, [IntPtr]0, [IntPtr]0) | Out-Null
    Start-Sleep -Seconds 5
    $result.ebbrise_after_settings = Get-Usage ebbrise

    $shown = @([P]::Visible((Get-Pids ebbrise)))
    $watch = [Diagnostics.Stopwatch]::StartNew()
    Start-Process $Exe -ArgumentList "mini" -Wait
    Wait-Until { @([P]::Visible((Get-Pids ebbrise))).Count -gt $shown.Count } | Out-Null
    $result.ebbrise_break_open_ms = [math]::Round($watch.Elapsed.TotalMilliseconds, 1)
    Start-Sleep -Seconds 2
    $result.ebbrise_break = Get-Usage ebbrise
    $result.ebbrise_break_cpu_pct = Measure-Cpu ebbrise 10
    Start-Sleep -Seconds 12
    $result.ebbrise_after_break = Get-Usage ebbrise
    Stop-Process -Id $p.Id -Force
} finally {
    $env:APPDATA = $realAppData
    Get-Process ebbrise -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq (Resolve-Path $Exe).Path } | Stop-Process -Force
}

if ((Test-Path $Stretchly) -and -not (Get-Process stretchly -ErrorAction SilentlyContinue)) {
    try {
        $watch = [Diagnostics.Stopwatch]::StartNew()
        Start-Process $Stretchly | Out-Null
        Start-Sleep -Seconds 20
        $result.stretchly_idle = Get-Usage stretchly
        $result.stretchly_idle_cpu_pct = Measure-Cpu stretchly $IdleSeconds
        $shown = @([P]::Visible((Get-Pids stretchly)))
        $watch = [Diagnostics.Stopwatch]::StartNew()
        Start-Process $Stretchly -ArgumentList "mini" | Out-Null
        Wait-Until { @([P]::Visible((Get-Pids stretchly))).Count -gt $shown.Count } | Out-Null
        $result.stretchly_break_open_ms = [math]::Round($watch.Elapsed.TotalMilliseconds, 1)
        Start-Sleep -Seconds 2
        $result.stretchly_break = Get-Usage stretchly
        $result.stretchly_break_cpu_pct = Measure-Cpu stretchly 10
    } finally {
        Get-Process stretchly -ErrorAction SilentlyContinue | Stop-Process -Force
    }
}
Remove-Item $data -Recurse -Force -ErrorAction SilentlyContinue
$result | ConvertTo-Json -Depth 4 | Tee-Object -FilePath $Out
