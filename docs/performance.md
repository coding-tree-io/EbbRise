# Performance

One measured run of the EbbRise 0.1.0 release exe next to Stretchly 1.22.1 on the same PC. Repeat it with `scripts/measure_performance.ps1`; it prints the JSON this page is made from.

## Environment

| Item | Value |
| --- | --- |
| Date | 2026-11-09 |
| EbbRise | 0.1.0, commit `11a5db2`, `target/release/ebbrise.exe` (release profile: fat LTO, 1 codegen unit, stripped) |
| Toolchain | rustc 1.99.1 (2026-10-13), Slint 1.18.1 |
| Stretchly | 1.22.1, installed in `C:\Program Files\Stretchly`, with its user's own config |
| OS | Windows 11 Pro N, build 26300 |
| CPU | Intel Core i5-8600K, 3.6 GHz, 6 cores, 6 threads |
| Memory | 31.9 GB |
| GPU | NVIDIA GeForce RTX 3070 Ti |
| Load | Normal desktop session with other apps open; no other benchmark running |

## Method

- **EbbRise** runs with a throwaway `%APPDATA%` and this config: Mini breaks of 20 s, window mode, no update check, natural breaks off. With natural breaks on, an idle PC holds breaks, so `mini` opens nothing during an unattended run.
- **Start time:** from `Start-Process` to the moment the signal window exists, which is when the app takes commands. Five runs.
- **Idle:** 10 s after start, then CPU over 60 s.
- **Settings:** from running `ebbrise preferences` (a second process that forwards the command) until the window exists. Memory 3 s later, then CPU over 10 s. The window is then closed, and memory is read 5 s later.
- **Break:** from running `ebbrise mini` until a new visible window exists. Memory 2 s later, then CPU over 10 s while the countdown ring animates. Memory again after the break ends.
- **Stretchly:** started, measured idle after 20 s, then sent `Stretchly.exe mini`. Its numbers are the sum over all its Electron processes.
- **Memory:** working set (RAM in use) and private bytes (memory committed to the process alone), from `Get-Process`.
- **CPU:** process CPU time per process over the sample, in percent of one core. Windows counts CPU time in 15.6 ms steps, so 0.0 % means less than one step in the sample.

## Results

| Measure | EbbRise | Stretchly | EbbRise vs Stretchly |
| --- | --- | --- | --- |
| Download | 7.3 MB zip, 14.7 MB exe | — | — |
| Start to ready | 20–36 ms (median 31.5 ms) | not measured | — |
| Processes / threads, idle | 1 / 4 | 4 / 135 | — |
| Working set, idle | 15.5 MB | 339.8 MB | 22× less |
| Private bytes, idle | 2.8 MB | 219.1 MB | 78× less |
| CPU, idle (60 s) | 0.0 % | 0.23 % | — |
| Settings window opens | 332 ms | not measured | — |
| Working set / private, settings open | 75.4 / 77.7 MB | — | — |
| CPU, settings open (10 s) | 0.16 % | — | — |
| Break window opens | 255 ms | 625 ms | 2.5× faster |
| Working set, during a break | 90.8 MB | 448.5 MB | 4.9× less |
| Private bytes, during a break | 116.5 MB | 337.9 MB | 2.9× less |
| CPU, during a break (10 s) | 0.63 % | 1.88 % | 3× less |

Raw run (start times in ms): 31.5, 35.8, 21.8, 35.1, 19.9.

## Findings

- At rest, before any window opens, EbbRise uses 15.5 MB of RAM, 4 threads and no measurable CPU.
- The first window costs about 60 MB, and that memory stays after the window closes: 73.9 MB after Settings closed, 91.2 MB after the break ended. It is the Slint renderer and its fonts staying loaded. The second window (the break, after Settings) added only 17 MB more. Releasing the renderer when no window is open is possible future work; a soak test would show whether memory grows over many breaks.
- Every number above is lower than Stretchly's. The open times include the second process that forwards the command, so they are what a user of the command line sees.

## Limits

- One run on one PC; start time is the only measure repeated (5 runs). Numbers from a busy desktop can vary by some milliseconds and megabytes.
- Stretchly ran with its user's config, not the EbbRise test config; its start time and settings window were not measured.
- Full-screen breaks, several monitors, high DPI and a laptop on battery were not measured.
- No memory-over-time (soak) test: one break only.
