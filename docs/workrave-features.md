# Workrave features in EbbRise (clean-room spec)

EbbRise adds four features that Workrave has and Stretchly does not. Workrave is GPL-3.0 and EbbRise is 0BSD, so EbbRise copies no Workrave code, text, images or data. This page describes the behaviour in our own words. The EbbRise code is written from this page only.

Source of the behaviour: Workrave 1.11.5 (source at commit `83981eb`, 2026-11-08): its user-visible behaviour, its preference names and its default values. Default values are facts, not creative work.

| # | Feature | Default |
| --- | --- | --- |
| 1 | Count only active time | Off (opt-in) |
| 2 | Daily limit | Off (opt-in) |
| 3 | Statistics | Always on |
| 6 | Block input during breaks | Off (opt-in) |

## Activity

Features 1, 2 and 3 need to know when the user is active.

- The user is **active** while keyboard or mouse input arrives. The user becomes **idle** 5 s after the last input. These 5 s still count as active.
- A single input after an idle period is **noise**, for example a bumped mouse. The return counts only when more input arrives at least 1 s after the first input.
- Windows mechanism: `GetLastInputInfo`, read at each wake-up. EbbRise sets no input hook for this. While the user is active, EbbRise wakes at the moment the 5 s idle threshold can pass. While the user is idle, it checks every 2 s for the return, as the natural-break check does now. EbbRise measures activity only while feature 1 or 2 is on (D2).

## 1. Count only active time

Setting `countActiveTimeOnly`, off by default.

- The wait for a Mini break and for a Long break advances only while the user is active. Idle time holds the wait; it does not reset it.
- Natural breaks (when on): idle time at least as long as a break resets that break's wait (D1). The older single reset time (`naturalBreaksInactivityResetTime`) applies only while this feature is off.
- Idle time also holds the time between the notification and the break.
- Break durations, pauses and postpone times stay clock time.
- The tray tooltip says that only active time counts.
- Natural-break idle time starts 5 s after the last input, as idle time does.

## 2. Daily limit

Settings `dailyLimit` (off by default), `dailyLimitTime` (4 h of active time), `dailyLimitPostponeTime` (20 min).

- EbbRise counts active time per day. A day starts at 04:00 local time, so late-night work counts towards the day before.
- When the active time reaches the limit, EbbRise shows a reminder (decision D3). It has two actions:
  - **Postpone:** the reminder comes again after `dailyLimitPostponeTime` more of active time.
  - **Done for today:** no more reminders until the next day starts.
- The window stays until the user picks an action.
- Pause, Do Not Disturb and app exclusions hold the reminder, as they hold breaks.

## 3. Statistics

Always on. A day starts at 04:00, as for the daily limit.

For each day, EbbRise records:

| Value | Meaning |
| --- | --- |
| Shown | A Mini or Long break window opened |
| Completed | The break ran to its end, or the user pressed Finish |
| Natural | Time away reset the wait, so the break was not needed |
| Skipped | The user pressed Skip, or an app exclusion ended the break |
| Postponed | The user pressed Postpone |
| Active time | Time of use, as defined above; recorded only while feature 1 or 2 is on (D2) |
| Daily limit | Reminders shown and whether the user stopped for the day |

- Storage: `%APPDATA%\EbbRise\statistics.json`, one record per day, written atomically like the settings, kept for 365 days.
- EbbRise writes the file when a count changes, when the day changes and at quit.
- A Statistics window opens from the tray menu (decision D4).

## 6. Block input during breaks

Setting `blockInputDuringBreaks`, off by default.

EbbRise already has strict mode. The two settings do different things:

| | Strict mode | Block input |
| --- | --- | --- |
| Skip and Postpone buttons | Removed | Stay, if strict mode is off |
| Quit from the tray during a break | Refused | Not reachable: the taskbar is another app |
| Keyboard and mouse to other apps | Still work | Blocked; the pointer still moves |
| Keyboard and mouse to EbbRise's windows | Work | Work |

- Both can be on: then the user cannot leave the break before its end.
- With full-screen breaks, block input matches Workrave's "block input and screen". With window breaks, it matches "block input".
- Windows mechanism: low-level keyboard and mouse hooks (`WH_KEYBOARD_LL`, `WH_MOUSE_LL`) on the main thread. They pass pointer moves, and keys and clicks that go to EbbRise's own windows; they drop the rest.
- Safety:
  - Ctrl+Alt+Del always works, because Windows does not let hooks block it.
  - The block ends when the break window closes or when the session locks.
  - The block also ends when the break time ends, even if the window stays (manual finish).

## Decisions (2026-11-09)

| # | Question | Chosen |
| --- | --- | --- |
| D1 | How do natural breaks work while only active time counts? | Per break: idle time at least as long as a break resets that break's wait (20 s for Mini, 5 min for Long by default), as Workrave does. Shorter idle time only holds the wait. With natural breaks off, idle time only holds. |
| D2 | When does EbbRise measure active time? | Only when feature 1 or 2 is on. With both off, EbbRise never wakes while you work, and statistics show break counts only. |
| D3 | How does the daily-limit reminder look? | A break-style window. |
| D4 | How does the Statistics window show the data? | One day at a time, with arrows to move between days. |
