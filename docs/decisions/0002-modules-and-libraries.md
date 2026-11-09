# ADR 0002: Modules and libraries

Status: accepted 2026-11-09.
Builds on [ADR 0001](0001-technology-stack.md).

## Decision: one crate per module (Spring Modulith rules, Cargo enforcement)

Rust has no Spring-style application framework, and the port does not need one. A Cargo workspace gives the same rules, and the compiler checks them.

| Spring Modulith | This project |
| --- | --- |
| Application module | One workspace crate. Public API = what `lib.rs` exports. Everything else is private or `pub(crate)`. |
| `ApplicationModules.verify()` | Cargo forbids cycles. One test in `app` reads the crate graph with `cargo_metadata` and fails on any dependency that is not in the allowed list below. |
| Module events / listeners | No event bus. See "How modules talk". |
| Event publication registry | Not needed: no database, no transactions. |
| Dependency injection | Constructors. `app` builds every module and passes in what each needs. No DI container. |
| `@ApplicationModuleTest` | Each crate's own tests: `cargo test -p <crate>`. |
| Documenter | `cargo modules dependencies` when a diagram is wanted. |

### Modules

| Crate | Owns | May depend on |
| --- | --- | --- |
| `settings` | Settings schema, defaults, load/save, Stretchly import | — |
| `breaks` | Breaks planner, break rules, until-morning | `settings` |
| `commands` | CLI grammar, duration parser, pipe message format | — |
| `i18n` (later) | Locale files, time-remaining text | — |
| `ideas` (later) | Break ideas, sanitising | — |
| `platform` (later) | Windows adapters, one internal module per OS signal | no domain crate |
| `ui` (later) | Slint UI host | domain crates, not `platform` |
| `app` | Composition root: message loop, wiring, logging | all |

Rules:
- Domain crates (`settings`, `breaks`, `commands`, `i18n`, `ideas`) never depend on `windows`, `platform`, `ui` or `app`.
- `platform` returns plain values (idle ms, Focus Assist value, process names). `app` turns them into planner calls.
- Adapters stay in one `platform` crate until one grows; then it becomes its own crate.

### How modules talk

Two different things were called "event-driven". They are decided separately.

1. **OS signals: push, not polling.** The OS tells us when something changes (window messages, WNF callback, power and session notifications, one coalescable timer). This is the reason the port is cheaper than Stretchly's three 2-second pollers. It is also the normal Win32 model.
2. **Between modules: direct calls, no bus.** `app` calls planner methods directly (`planner.idle(now, ms)`). The planner returns what must happen (`Vec<Event>`), and `app` does it with a `match` (show toast, open break window, play sound). This is the "sans-IO" or "functional core, imperative shell" pattern, which is common Rust practice (for example `quinn-proto`, `rustls`). It avoids callbacks and shared mutable references that fight the borrow checker, it keeps the planner testable with a fake clock, and the cost is one small `Vec` per call. An event bus or observer traits would add indirection without a speed gain.

## Libraries

Versions are pinned at scaffold time.

| Need | Crate | Why not hand-written |
| --- | --- | --- |
| Win32 / WinRT | `windows` | Official bindings |
| Tray icon + menu | `tray-icon`, `muda` | Icon re-add after Explorer restart, high-DPI icons, native menus |
| Global shortcuts | `global-hotkey` | `RegisterHotKey` and shortcut parsing; convert Stretchly's `CmdOrCtrl+X` format |
| Toasts | `tauri-winrt-notification` | Toast XML; AppUserModelID registration (R2) stays ours |
| Single instance + CLI forwarding | `interprocess` | Named pipe; a failed pipe create means another instance runs |
| Autostart | `auto-launch` | `HKCU\...\Run` |
| Paths, atomic save | `directories`, `tempfile` (`persist`) | `%APPDATA%`; save never leaves a half-written file |
| Logs | `tracing`, `tracing-subscriber`, `tracing-appender` | Rolling log file |
| Errors | `thiserror` (modules), `anyhow` (`app`) | |
| Time | `jiff`, `sunrise` (+ `chrono`, required by `sunrise`) | |
| Localized time-remaining text | ICU4X `DurationFormatter` (`icu_experimental`) | CLDR plural rules for all locales; API may still change |
| Version check | `ureq` on a short-lived thread | Blocking HTTP, no tokio |
| Module graph test | `cargo_metadata` (dev-dependency) | |
| UI | `slint` | ADR 0001 |

Hand-written on purpose: idle (`GetLastInputInfo`), process list (Toolhelp snapshot; `sysinfo` reads far more than names), Focus Assist WNF subscription (no maintained crate, R1), power and lock notifications, monitor list, sound (`PlaySoundW` from memory), CLI parser (102 lines, tested; `clap` mostly adds size).

Rejected: tokio and actix (a runtime and extra wake-ups for no gain), Bevy app/plugins (fits the modular shape, but built around a tick loop), DI containers such as `shaku`, event-bus crates.

## One thread, one loop

Slint, `tray-icon` and `global-hotkey` all need the Win32 message loop on the main thread, and they can share it. `slint::run_event_loop_until_quit` keeps the loop alive with no window open. Backend timers use `SetCoalescableTimer` on a message-only window, pumped by the same loop. Use the crates' `set_event_handler` callbacks, not their channel receivers, which need polling.

Probe 2026-11-09 (`crates/probe`): confirmed. With Slint's loop running and no window open, the tray icon, global hotkeys, a 10 s coalescable timer (ticks within ±10 ms), session and suspend registration, `GetLastInputInfo` and the Focus Assist WNF callback all worked on the one thread. Not yet tested: lock/unlock and sleep/wake delivery.
