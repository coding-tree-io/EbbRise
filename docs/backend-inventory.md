# Backend inventory (Stretchly 1.22.1 → native Windows)

Source read: `app/main.js` (1,942 lines), `app/breaksPlanner.js`, `app/utils/*.js` (about 4,600 lines in total). Upstream has 325 Vitest tests in `test/`.

"Upstream" says how Stretchly does it today. "Windows native" says what the port uses. "Cost now" marks the waste we remove.

## 1. Break scheduling core (platform-free)

| Behaviour | Upstream | Port | Notes |
| --- | --- | --- | --- |
| One pending timer with a name (`reference`) | `Scheduler` wraps `setTimeout` | Single deadline in core state | Upstream replaces the timer object each time. |
| Mini/Long choice | `breakNumber % (longBreakInterval + 1)` | Same rule | Three cases: mini only, long only, both. |
| Pre-break notification | timer = interval − notification interval | Same | Then `nextBreakAfterNotification`. |
| Postpone | `*PostponeTime`, minus notification time if that is shorter | Same | `postponesNumber` limit enforced in main.js. |
| Skip to Mini / Long | 100 ms delay, adjusts `breakNumber` | Same | |
| Pause (ms, `1` = indefinitely) | stops all pollers | Same | |
| Until morning | Luxon + `meeussunmoon` sunrise from lat/long | Same rule, sunrise crate | `morningHour` is a number or `"sunrise"`. |
| Time to next break, progress % | derived from timer | Same | Feeds tray tooltip and UI. |
| Danger score (break health) | ±amount, clamp 0–10 | Same | Tint itself is UI. |
| Manual finish phase | waits for user after time ends | Same | |
| Resume after sleep | `correctScheduler()` re-arms with remaining time | Same contract, explicit wall/monotonic choice | See ADR risk R3. |

Event guards repeat in breaksPlanner: idle/DND/exclusion signals are ignored while paused or during a running break. The port keeps these guards as one table, not copied `if` chains.

## 2. Inputs from the OS

| Signal | Upstream | Cost now | Windows native |
| --- | --- | --- | --- |
| Idle time | `powerMonitor.getSystemIdleTime()` / `node-desktop-idle-v2`, polled every 2 s | 1 wake-up / 2 s for ever | `GetLastInputInfo`. Next check is scheduled at the threshold, not on a fixed tick. |
| Do Not Disturb | `windows-focus-assist` (native addon, undocumented WNF query), polled every 2 s | 1 wake-up / 2 s | WNF state change subscription (push, no polling). Fallback: `SHQueryUserNotificationState`. |
| App exclusions | `ps-list` every 2 s | On Windows ps-list runs a bundled `fastlist.exe` child process every 2 s | `CreateToolhelp32Snapshot` in process (about 1 ms). Name match. |
| Suspend / resume | `powerMonitor` | none | `WM_POWERBROADCAST` / `PowerRegisterSuspendResumeNotification` |
| Lock / unlock | `powerMonitor` | none | `WTSRegisterSessionNotification` |
| Theme (dark/light) | `nativeTheme` | none | `AppsUseLightTheme` / `SystemUsesLightTheme` registry change notification |
| Displays, cursor display | `screen` | none | `EnumDisplayMonitors`, `MonitorFromPoint`, `WM_DISPLAYCHANGE` |

Parity note: ps-list on Windows returns process names, not command lines. Name-only matching on Windows is therefore exact parity.

## 3. Outputs to the OS

| Effect | Upstream | Windows native |
| --- | --- | --- |
| Tray icon, menu, tooltip | Electron `Tray` + `Menu` | `Shell_NotifyIconW` + popup menu (or UI host) |
| Notifications | HTML5 `Notification` in a hidden renderer window | Windows toast (AppUserModelID `net.hovancik.stretchly` upstream; ours must differ) |
| Sounds (WAV, volume 0–1) | `new Audio()` in a hidden renderer window | PCM WAV scaled by volume in memory, then `PlaySoundW(SND_MEMORY \| SND_ASYNC)`; no audio crate (ADR 0001) |
| Global shortcuts (8 configurable, plus end-break) | `globalShortcut` | `RegisterHotKey` |
| Autostart | `app.setLoginItemSettings`; Store build uses StartupTask | `HKCU\...\Run` value; MSIX `StartupTask` if we ship MSIX |
| Single instance + CLI commands | `requestSingleInstanceLock`, `second-instance` argv | Named mutex + named pipe that carries the command |
| Break windows on all/primary/cursor display, top-most, not focusable, kiosk/fullscreen | `BrowserWindow` per display | Window placement and z-order rules belong to the core/UI contract; drawing belongs to the UI host |
| Logs | `electron-log` | `tracing` to a rolling file |
| Open URL / folder | `shell.openExternal` | `ShellExecuteW` |

## 4. Data

| Item | Upstream | Port |
| --- | --- | --- |
| Settings | `electron-store` JSON, 80+ keys, defaults in `defaultSettings.js` | Same JSON schema and key names; atomic write (temp file + rename); import from `%APPDATA%\Stretchly\config.json` |
| Break ideas | 146 + 206 lines of defaults, user ideas, sanitised | Same data, same sanitising rules |
| Translations | i18next JSON, 52 locales | Reuse JSON files; backend needs only tray/notification strings |
| Custom sounds/images | user data `audio/`, `images/` folders, filename rules | Same folder rules |

## 5. CLI (`commands.js`)

Commands: `help`, `version`, `logs`, `reset`, `pause [-d indefinitely|until-morning|HHhMMm]`, `resume`, `toggle`, `postpone`, `mini [-T title] [-n] [-w wait]`, `long [-T] [-t text] [-n] [-w]`, `preferences`, and others. Duration parser `HHhMMm|HHh|MMm|MM`. Port: same grammar, sent to the running instance over the named pipe.

## 6. Out of scope until decided

- Contributor features: "My Stretchly" sign-in, preference sync (`restore-remote-settings`). They depend on Stretchly's own server.
- Version check against `api.github.com/repos/hovancik/stretchly` (must point to our releases instead).
- Linux (D-Bus, gsettings, Flatpak portal) and macOS (`defaults read`) DND and autostart paths.
- Bundled WAV files and images: licence of these assets is not stated in the README. Check before reuse.

## Baseline to beat

Installed Stretchly 1.22.1 on this machine: 446 MB on disk. It was not running, so memory, threads and wake-ups are not measured yet. Electron apps of this kind normally run 4+ processes. The measurement method is in ADR 0001.
