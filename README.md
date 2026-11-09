# stretch-core (working name)

A Windows-first, native port of the Stretchly break-reminder backend. A new UI will sit on top of it.

Upstream reference: [hovancik/stretchly](https://github.com/hovancik/stretchly) at `64450ca` (v1.22.1, 5 Nov 2026), cloned into `../upstream/stretchly`.

## Status

Stage: platform-free core done and tested (`settings`, `breaks`, `commands`). Windows adapters (`platform`: signal window, timer, clocks, Focus Assist, running processes, toasts, sounds, autostart, single instance) and the app (`app`: tray plus a Slint break window) run, with settings saved and app exclusions checked. CLI commands reach the running app. The tray, tooltip and notifications use Stretchly's translations, and breaks show our own evidence-based ideas (action, finding, source), English only. The tray menu follows the app state as upstream's does. The backend is complete except global hotkeys, which are not ported. Breaks open a Slint window (design B1: ring countdown, dark green palette) with Postpone, Skip and Finish as upstream offers them. Preferences (tray or `stretch preferences`) opens a Slint settings window (design S2) that saves each change at once; its labels are English only. Long breaks run on their own clock (`longBreakEvery`, minutes in the settings window), so they work without Mini breaks; a Long break due near a Mini break takes its place. A Stretchly config import converts its Mini-break count to that interval. The app is named Ebb (icon: the tide in a ring, `crates/app/ui/icons`); the exe and the `%APPDATA%\stretch-core` folder keep their old names. Both windows use a sand-tinted palette in light and dark mode, following the Windows app theme or Stretchly's `themeSource`.

- [Backend inventory](docs/backend-inventory.md): every backend behaviour and OS integration to port, with the Windows mechanism for each.
- [ADR 0001: technology stack](docs/decisions/0001-technology-stack.md): Rust backend, Slint UI, Windows only.
- [ADR 0002: modules and libraries](docs/decisions/0002-modules-and-libraries.md): one crate per module, allowed dependencies, libraries.
- [Test plan](docs/test-port-plan.md).
- [Project language](docs/language.md): domain terms.

## Licence

Stretchly is BSD-2-Clause. This port is a derivative work, so it keeps Stretchly's copyright notice in [LICENSE-STRETCHLY](LICENSE-STRETCHLY). Our own licence is not chosen yet.
