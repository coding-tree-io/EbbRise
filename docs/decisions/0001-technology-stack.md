# ADR 0001: Technology stack for the backend

Status: accepted 2026-11-09. Rust backend, Slint UI host, Windows only. Owner: skip performance measurement for now (targets below stay as design goals, not gates). Work is test-driven, starting from the upstream tests.
Date: 2026-11-09.

## Goal, stated as numbers

"Extremely performant" means these targets. Each one is measured, not assumed.

| Metric (idle, breaks scheduled, no window open) | Target | How it is measured |
| --- | --- | --- |
| Processes | 1 | Task Manager / `Get-Process` |
| Private memory | < 5 MB | `Get-Process` PrivateMemorySize64, 10 min sample |
| Wake-ups (context switches) | < 1 per 10 s on average | WPR CPU trace or `\Thread(*)\Context Switches/sec` |
| CPU time | < 1 s per 24 h | `Get-Process` CPU delta |
| Timer resolution change | never | `powercfg /energy` report |
| Cold start to tray ready | < 50 ms | WPR trace, process start to tray icon registered |
| Installed size (backend) | < 3 MB | file size of release exe |
| Break start latency after deadline | < 16 ms | log timestamps against scheduled deadline |

Baseline: run the same script against Stretchly 1.22.1 (installed at `C:\Program Files\Stretchly`, 446 MB on disk) before we write code, so every number has an "upstream" column.

## Decision: Rust, single process, Win32 message loop, no async runtime

### Why Rust

- Native code, no runtime, no GC pauses, small static exe.
- Microsoft's official `windows` crate covers every API in the inventory, including WinRT toasts.
- Strong test tooling for a pure state machine.

Alternatives considered: C++ (same speed, weaker safety and tooling), C# NativeAOT (good Windows APIs, but larger idle memory and a runtime), Zig (immature Windows bindings).

### Shape

```
crates/
  core/              platform-free state machine, injected clock, no I/O
  platform-windows/  Win32 adapters: idle, DND, processes, power, session, hotkeys, tray, toast, sound, autostart, pipe
  app/               exe: message loop, wires adapters to core, settings file, logging, CLI
```

- One thread runs a Win32 message loop on a message-only window. All OS signals arrive as window messages or waitable-timer events. The core runs on that same thread, so there are no locks.
- The core exposes `Command` in and `Event` out (serde types). Any UI host binds to this API, in process or over the named pipe. The UI choice does not change the core.
- No tokio and no polling loops. Time is one deadline at a time, armed with a coalescable waitable timer so Windows can batch the wake-up.

### Removing the polling

Upstream wakes three 2-second pollers for ever. The port replaces each:

| Upstream poller | Replacement |
| --- | --- |
| Idle every 2 s | `GetLastInputInfo` once, then arm the next check for when the idle threshold could first be crossed. While the user types, about one check per 20 s. While idle, one check at the reset time. |
| DND every 2 s | Subscribe to the WNF Focus Assist state change (push). Fallback when unsupported: poll `SHQueryUserNotificationState` at 2 s with coalescing. |
| ps-list every 2 s (spawns `fastlist.exe`) | `CreateToolhelp32Snapshot` in process, only while an exclusion rule is active, on a coalescable timer. |

### Libraries

Versions are pinned at scaffold time through Context7 / crates.io, not here.

| Need | Choice | Reason |
| --- | --- | --- |
| Win32 + WinRT | `windows` | Official, complete, zero-cost |
| Settings JSON | `serde`, `serde_json` | Same file format as Stretchly; enables import |
| Local time, DST, "until morning" | `jiff` | Time-zone-correct local arithmetic on Windows |
| Sunrise | `sunrise` | Replaces `meeussunmoon` |
| Logging | `tracing` + file writer | Structured, cheap when disabled |
| Sound | No crate: parse PCM WAV, scale samples by volume, `PlaySoundW(SND_MEMORY \| SND_ASYNC)` | No audio thread or device kept open. Limit: PCM WAV only, which is already Stretchly's rule. |
| Toasts | WinRT `ToastNotificationManager` through `windows` | Native Action Center notification |
| Tests | `cargo test` only | Risk-ranked scenarios; see [test plan](../test-port-plan.md) |
| Size/profile | `cargo bloat`, release profile `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = true` | |

## What proves it right

1. Break-timing scenarios with a fake clock, taken from `breaksPlanner.js` behaviour and real CHANGELOG regressions ([test plan](../test-port-plan.md)).
2. Trimmed upstream tests for the pure rules that decide user actions.
3. Platform probes: small exes that print the raw OS signal (idle ms, DND state, session lock, power events) on this machine's Windows build. Run before adapter code is trusted.

## Risks

| ID | Risk | Mitigation |
| --- | --- | --- |
| R1 | Focus Assist/DND state uses an undocumented WNF name; Windows 11 changed DND UI. | Probe 2026-11-09 (Windows 11 build 26300): the WNF subscription pushed `2` when DND turned on and `0` when it turned off. Keep the documented `SHQueryUserNotificationState` fallback. |
| R2 | Toasts from an unpackaged exe need a registered AppUserModelID (Start-menu shortcut or registry). | Probe: an `HKCU\Software\Classes\AppUserModelId\<id>` entry with `DisplayName` was enough; toasts appeared. The first toast after registration did not show. One `show()` call blocked the thread for 4 s, so toasts must be sent off the loop thread. |
| R3 | Sleep and timers: upstream re-arms with remaining time on resume. Rust `Instant` and waitable timers may count or skip sleep time differently. | Probe: tick count includes sleep, `QueryUnbiasedInterruptTime` excludes it (they differed by the machine's total sleep time). Timer behaviour across a real sleep is not tested yet. |
| R4 | Bundled WAV and image licences are not stated upstream. | Confirm with upstream or replace assets. |
| R5 | Contributor sync depends on Stretchly's server. | Out of scope unless chosen. |

## Owner decisions (2026-11-09)

1. Platform scope: **Windows only.** No macOS/Linux adapters.
2. UI host: **the fastest and lightest of the three.** Owner assumed WinUI 3 (C#) wins and would then write everything in C#. Assessment below; final choice waits for the probe or owner confirmation.
3. Optional features: no preference. Defaults: keep Stretchly config import (cheap, helps migration), keep version check pointed at our own releases, drop contributor sync (needs Stretchly's server).

### UI host: expected ranking on speed and resources

Expected, from how each framework works. Numbers are typical figures to confirm with the probe on this machine.

| Rank | Host | Idle with window open | Why |
| --- | --- | --- | --- |
| 1 | Slint (Rust) | Single process, low tens of MB at most | Own renderer, no browser engine, no runtime. Same language as the backend. |
| 2 | WinUI 3 (C#, NativeAOT) | One process, several tens of MB | XAML framework and composition layers load into the process; .NET runtime and GC stay resident even with NativeAOT. |
| 3 | Tauri 2 (WebView2) | 3+ extra processes, about 100 MB while a window is open | Full Chromium-based browser engine. Can be torn down when no window is open. |

WinUI 3 is not expected to be the lightest. It gives the most native Fluent/Mica look, not the smallest footprint. Moving the backend to C# as well would also raise the idle backend cost (runtime + GC resident at all times) against the targets above.

Probe to settle it: one full-screen break window with a countdown, built in Slint and in WinUI 3 (NativeAOT). Measure process count, private memory, open latency and idle wake-ups with the same script as the baseline.
