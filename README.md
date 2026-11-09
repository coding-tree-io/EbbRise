# EbbRise

Break reminders for Windows. EbbRise asks you to take short Mini breaks for your eyes and longer Long breaks to stand up and move. Every break idea names the study behind it.

EbbRise is a native Rust and [Slint](https://slint.dev) port of [Stretchly](https://github.com/hovancik/stretchly) (v1.22.1), for Windows 10 and 11 only.

## Install

1. Download `EbbRise-<version>-windows-x64.zip` from the [latest release](https://github.com/coding-tree-io/EbbRise/releases/latest).
2. Unzip it to a folder of your choice, for example `%LOCALAPPDATA%\Programs\EbbRise`.
3. Run `ebbrise.exe`. It lives in the tray; open Settings from the tray menu.

The exe is not code-signed, so Windows SmartScreen can warn on the first start. Select **More info**, then **Run anyway**.

On the first start, EbbRise imports your Stretchly settings if Stretchly is installed. It keeps its own settings, sounds and logs in `%APPDATA%\EbbRise`. It checks GitHub for a new release every two days; you can turn this off in the config file (`checkNewVersion`).

## Features

- Mini and Long breaks on their own clocks, with notifications before them, strict mode, postpone and skip.
- Breaks in a window or full screen, with a countdown ring and an evidence-based idea.
- Natural breaks: time away from the PC counts as a break. Breaks wait during Do Not Disturb.
- Settings window in light, dark or the Windows theme. The tray, the notifications and most of Settings use Stretchly's translations (54 languages).
- Three built-in sounds; put your own WAV files in `%APPDATA%\EbbRise\audio`.
- Stretchly's command line, for example `ebbrise pause -d 1h` or `ebbrise mini`. Run `ebbrise help` for the full list.

## Build

Requires Rust (edition 2024) and, for the exe icon and version details, the Windows 10/11 SDK (`rc.exe`).

```bash
cargo build --release
```

The exe is `target/release/ebbrise.exe`. Checks: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.

- [Backend inventory](docs/backend-inventory.md): every backend behaviour and OS integration, with the Windows mechanism for each.
- [ADR 0001: technology stack](docs/decisions/0001-technology-stack.md) and [ADR 0002: modules and libraries](docs/decisions/0002-modules-and-libraries.md).
- [Test plan](docs/test-port-plan.md) and [project language](docs/language.md).

Not ported: global hotkeys.

## Licence

EbbRise's own code is [0BSD](LICENSE): use it for anything, with no conditions. EbbRise is a port of [Stretchly](https://github.com/hovancik/stretchly), which is BSD-2-Clause, so the ported parts keep Stretchly's notice in [LICENSE-STRETCHLY](LICENSE-STRETCHLY) (SPDX for each crate: `0BSD AND BSD-2-Clause`).

[THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt) holds Stretchly's notice, the credits of the built-in sounds and the licence of every crate in the exe. Every release ships `LICENSE`, `LICENSE-STRETCHLY` and `THIRD-PARTY-NOTICES.txt` next to the exe. After a dependency change, run `python -I scripts/third_party_notices.py` again.

The UI uses [Slint](https://slint.dev) under its Royalty-free Desktop Application License 2.0. That licence asks for the badge below on the public page the exe is downloaded from (this README and the releases page). Forks that ship binaries must do the same, or choose one of Slint's other licences.

[![Made with Slint](https://raw.githubusercontent.com/slint-ui/slint/master/logo/MadeWithSlint-logo-whitebg.png)](https://slint.dev)
