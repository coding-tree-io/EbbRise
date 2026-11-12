# Changelog

All notable changes to EbbRise are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

New texts are in English only until translations catch up.

## [Unreleased]

## [0.7.0] - 2026-11-13

Breaks that wait for real rest, a daily limit that only informs, and About one click from the tray.

### Changed

- **Break countdown pauses while you use the PC:** mouse or keyboard input during a break holds the countdown. It continues from the same time after 2 s without input, and the ring says "paused while active".
- **Daily limit is a notice:** the screen keeps the EbbRise look but offers only "Got it". It comes back after more use, and its text says when. Settings → Activity → "Remind again after" sets that time (5–240 min, default 20). "Postpone" and "Done for today" are gone.
- **About** is in the tray menu. The About page has a **Check for updates** button.

### Fixed

- Break start and end sounds play even when `silentNotifications` is on (often carried over from Stretchly). That setting now mutes notifications only.

## [0.6.0] - 2026-11-11

One clear choice for time away from the PC, and fixes that keep settings, pauses and statistics safe.

### Fixed

- Unreadable settings are backed up and no longer change start at login on startup.
- Changing away settings no longer leaves breaks stopped.
- Pause until morning ends after an overnight sleep.
- Active time is saved during use and when Windows ends the session.
- Statistics roll over at the 04:00 day boundary and before exit.
- Command-line actions work when EbbRise is not already running.
- Sunrise pauses use today's sunrise until it passes.
- A postponed daily-limit reminder stays postponed after restart.

### Changed

- **When you're away** (Settings → Schedule → Break timing): one choice, Keep counting, Pause timer or Count as break, replaces the "Natural breaks" switch on General and "Count only active time" on Activity. Count as break works like Stretchly's natural breaks: time away of 5 minutes or more restarts the timer as a break. Saved settings keep their meaning.
- The tray tooltip says "Timer pauses while you're away" instead of "Counts only active time".

## [0.5.0] - 2026-11-10

Calmer break advice backed by newer studies, and a steadier statistics view.

### Added

- **Advice appears a few seconds in:** the break shows its action, the finding and the study 4 seconds after it starts (or after a quarter of a shorter break), rising gently into view. If you postpone or skip at once, it stays out of your way. The daily-limit reminder still shows its message at once.
- **Newer, tighter break ideas:** every built-in idea now cites a study from 2016 to 2025 and fits inside the break, for example looking at something 6 m away, full blinks, a two-minute walk, chair stands or slow exhale breathing.

### Fixed

- The streak text in Statistics no longer flickers while the shoreline waves move.

## [0.4.0] - 2026-11-10

Smarter break timing, a tide ring, week and month statistics, and fixes.

### Added

- **No breaks in full screen** (Settings → General, on by default): while a game, a full-screen video or a presentation fills the screen, a due break waits. It shows as soon as you leave full screen.
- **Pause media during breaks** (Settings → General, off by default): music and videos that are playing pause when a break starts and resume after it. It works with apps that show in the Windows media controls, such as browsers, Spotify and the Media Player app.
- **Show breaks on** (Settings → General): the main display, the display with the mouse, all displays, or one display. Settings imported from Stretchly ("screen", "allScreens") now apply.
- **Tide ring:** water rises in the break ring as the break goes on.
- **Week and month statistics:** a Day, Week and Month switch. The charts show active time per day, the share of breaks taken and the daily limit.
- **Shoreline streak:** the last 14 days on a shoreline, with your streak of days with no skipped break. A day without breaks does not end the streak.

### Fixed

- After the postpone limit is used, a break now shows Skip, as in Stretchly.
- The break window can no longer be minimized or closed with Alt+F4. Only its buttons end a break early.
- Statistics history is kept when `statistics.json` cannot be read, or a save fails.
- Idle time at the 04:00 day change no longer counts as active time for the old day.
- The Do Not Disturb hint is no longer cut off.

## [0.3.0] - 2026-11-10

Fixes and a manual update check.

### Added

- **Check for updates** (tray menu): checks for a new release at once, even with automatic checks off. A notification tells you the result: a new version, up to date, or the check failed. A new version also adds "Download latest version" to the tray menu.

### Changed

- **Clearer Do Not Disturb switch** (Settings → General): now "No breaks during Do Not Disturb", with a hint that the timer restarts when Do Not Disturb ends.

### Fixed

- **Breaks take focus:** the break window now gets keyboard focus when it opens. Before, it opened on top but keys still went to the app behind it.

## [0.2.0] - 2026-11-09

New features from Workrave, written from a clean-room spec. All but statistics are opt-in.

### Added

- **Count only active time** (Settings → Activity): idle time holds the wait for the next break. With natural breaks on, idle time as long as a break counts as that break.
- **Daily limit** (Settings → Activity): a reminder after 4 h of active time, with Postpone and "Done for today". A day starts at 04:00.
- **Statistics** (tray menu → Statistics, or `ebbrise statistics`): shown, completed, natural, skipped and postponed breaks, active time and daily-limit reminders, one day at a time. Saved in `%APPDATA%\EbbRise\statistics.json` for 365 days.
- **Block keyboard and mouse** (Settings → General): during a break, other apps get no keys or clicks. Ctrl+Alt+Del always works. This is different from strict mode, which removes Skip and Postpone.

## [0.1.0] - 2026-11-09

First release of EbbRise: break reminders for Windows 10 and 11, a native Rust and Slint port of Stretchly. Stretchly settings are imported on the first start.

[0.7.0]: https://github.com/coding-tree-io/EbbRise/releases/tag/v0.7.0
[0.6.0]: https://github.com/coding-tree-io/EbbRise/releases/tag/v0.6.0
[0.5.0]: https://github.com/coding-tree-io/EbbRise/releases/tag/v0.5.0
[0.4.0]: https://github.com/coding-tree-io/EbbRise/releases/tag/v0.4.0
[0.3.0]: https://github.com/coding-tree-io/EbbRise/releases/tag/v0.3.0
[0.2.0]: https://github.com/coding-tree-io/EbbRise/releases/tag/v0.2.0
[0.1.0]: https://github.com/coding-tree-io/EbbRise/releases/tag/v0.1.0
