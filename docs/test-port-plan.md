# Test plan

Rule: test what can silently break the app's main job, which is breaks at the right time. Keep the suite small. One test per behaviour. Use table-driven tests where cases differ only in data. No property tests, no snapshot tests, no tests that restate the implementation.

Sources, in this order of trust:
1. `app/breaksPlanner.js` and its callers in `app/main.js`: what the code does today.
2. `CHANGELOG.md` bug fixes: timing bugs that really happened.
3. Upstream Vitest files: only where they check a real rule.

## Tier A: break timing scenarios (write first)

Scenario tests on the core with a fake clock. Each one is a short timeline: settings, inputs at times, expected events at times. About 20 tests in total.

Behaviour from `breaksPlanner.js` and `main.js`:
- Mini/Long order: with both on, every `longBreakInterval + 1`-th break is Long. Mini only. Long only. Both off: nothing is scheduled and nothing crashes (CHANGELOG: "app not starting when both break types are disabled").
- Pre-break notification fires `*NotificationInterval` before the break, then the break starts on time.
- Postpone: delay is `*PostponeTime`, shortened by the notification interval when a notification is on. The postpone limit is enforced.
- Skip to Mini / Skip to Long: break starts after the short delay, and the Mini/Long order continues correctly afterwards.
- Pause for a duration, until morning, indefinitely; resume; reset.
- Break end (auto and manual finish) plans the next break.
- Time to next break and progress % for the tray.

Regressions from the CHANGELOG:
- Idle longer than `naturalBreaksInactivityResetTime` resets the cycle, and breaks resume on return.
- DND during a break does not end the break; DND between breaks stops scheduling; DND ending resumes. A paused user stays paused when DND ends ("DoNotDisturb being ignored when resuming breaks").
- Focus Assist value "unsupported" (-1) never counts as DND ("DND no longer stays active when Focus Assist is unsupported").
- A `pause` app exclusion that starts during a break finishes that break and stops scheduling ("breaks not being finished when exclusion starts").
- Suspend/lock pauses breaks when `pauseForSuspendOrLock` is on, but not when already paused; resume/unlock resumes them. An indefinite pause stays paused after system resume ("breaks wont resume with indefinite pause after system resume").
- With `pauseForSuspendOrLock` off, resume keeps the remaining time instead of restarting (`correctScheduler`).

Rules the port changes (upstream polls every 2 s; the port reacts to events):
- Idle: the event-driven idle check gives the same reset/resume outcome as the 2 s poll, at most one poll interval later (app simulation test).
- DND and app exclusions: an event calls the same planner method the poll would, only earlier, so the planner tests cover both.
- Sleep: a deadline that passes during sleep fires once on resume, not twice and not never.

## Tier B: pure rules worth keeping (port, trimmed, table-driven)

| Upstream file | Keep | Why |
| --- | --- | --- |
| commands.js | CLI grammar and duration parser `HHhMMm\|HHh\|MMm\|MM`, as one table | User input |
| untilMorning.js | fixed hour before/after, sunrise, one DST change day | Easy to get wrong |
| utils.js | `canPostpone`, `canSkip`, time-remaining text | Decide user actions |
| appExclusionsManager.js | pause rule, resume rule, name match | Covered by Tier A scenarios too; keep only the matching rule |
| naturalBreaksManager.js | covered by Tier A | |
| displayManager.js | target display choice (primary, cursor, index, invalid index fallback), centred placement | Break windows on the wrong screen are a visible bug |
| audioResolver.js, imageResolver.js | allowed file name rule (rejects path traversal) | Trust boundary |
| translations.js | every locale file parses, its nested texts resolve, and the backend keys exist | Broken file breaks the tray |
| shuffled.js | every enabled idea comes once before any repeats | Users notice repeats |
| appIcon.js | one table: tray icon for minutes left, including > 99 minutes | CHANGELOG regression; the other 96 cases are a file-name mapping |

## Dropped

| Upstream file | Reason |
| --- | --- |
| appIcon.js (96 of 97) | File-name mapping; one table row set covers it |
| scheduler.js | Covered by Tier A |
| breakShortcuts.js (all) | Global hotkeys are not ported |
| runOnce.js, ideasLoader.js | Trivial JS helpers |
| versionChecker.js | Network call; one manual check is enough |
| utils.js Linux/X11/Flatpak/Snap/Store helpers, `shouldShowNotificationTitle` | Not Windows, or OS-version cosmetics |
| dndManager.js Linux/macOS cases | Not Windows |
| breakHealthEffect.js, sanitizeIdea.js | UI side; decide when the Slint UI is built |
