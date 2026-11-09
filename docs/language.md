# Project language

Terms follow Stretchly's own names so that parity checks map one to one.

| Term | Meaning |
| --- | --- |
| Mini break | Short break. Default: 20 s every 10 min. |
| Long break | Long break. Default: 5 min after every `longBreakInterval + 1` breaks. |
| Break number | Counter that decides whether the next break is Mini or Long. |
| Break notification | Warning shown before a break (`*NotificationInterval` ahead). |
| Postpone | Move the current break later by `*PostponeTime`. Limited by `*PostponesLimit`. |
| Skip to | Start a Mini or Long break now (after a short delay). |
| Pause | Stop breaks for a duration, until morning, or indefinitely. |
| Natural break | User idle time. Idle > reset time resets the break cycle. |
| DND | Do Not Disturb / Focus Assist. Breaks stop while it is on. |
| App exclusion | Rule that pauses (`pause`) or only allows (`resume`) breaks while a named process runs. |
| Strict mode | Break that the user cannot skip or postpone. |
| Manual finish | Break that waits for the user to end it after its time passes. |
| Danger (break health) | 0–10 score that rises with postpones/skips and tints the break window. |
| Scheduler reference | Name of the one pending timer: `startMiniBreak`, `finishLongBreak`, `resumeBreaks` and so on. |
| Core | Platform-free state machine. No OS calls, injected clock. |
| Platform adapter | Windows code that turns OS signals into core events. |
| UI host | The future redesigned front end. Talks to the core only through commands and events. |
