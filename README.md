<p align="center">
  <img alt="EbbRise logo: a tide line inside a ring" src="crates/app/ui/icons/ebbrise.svg" width="128" height="128">
</p>

<h1 align="center">EbbRise</h1>

<p align="center">
  <a href="https://github.com/coding-tree-io/EbbRise/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/coding-tree-io/EbbRise/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/coding-tree-io/EbbRise/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/coding-tree-io/EbbRise"></a>
  <a href="LICENSE"><img alt="License: 0BSD" src="https://img.shields.io/badge/license-0BSD-blue"></a>
  <img alt="Windows 10 and 11" src="https://img.shields.io/badge/Windows-10%20%7C%2011-0078D4">
  <img alt="Minimum Rust version" src="https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Fcoding-tree-io%2FEbbRise%2Fmain%2FCargo.toml&query=%24.workspace.package.rust-version&label=rust&prefix=%E2%89%A5%20&logo=rust&color=orange">
</p>

Break reminders for Windows. EbbRise asks you to take short Mini breaks for your eyes and longer Long breaks to stand up and move. Every break idea names the study behind it.

EbbRise is a native Rust and [Slint](https://slint.dev) port of [Stretchly](https://github.com/hovancik/stretchly) (v1.22.1), for Windows 10 and 11 only.

## Install

1. Download `EbbRise-<version>-windows-x64.zip` from the [latest release](https://github.com/coding-tree-io/EbbRise/releases/latest).
2. Unzip it to a folder of your choice, for example `%LOCALAPPDATA%\Programs\EbbRise`.
3. Run `ebbrise.exe`. It lives in the tray; open Settings from the tray menu.

The exe is not code-signed, so Windows SmartScreen can warn on the first start. Select **More info**, then **Run anyway**.

### Verify your download

Each release lists the SHA-256 of its zip. In PowerShell:

```powershell
(Get-FileHash .\EbbRise-<version>-windows-x64.zip -Algorithm SHA256).Hash
```

The zips are built by [GitHub Actions](.github/workflows/release.yml) from the tagged commit, with a signed build attestation. To check it, with the [GitHub CLI](https://cli.github.com):

```powershell
gh attestation verify .\EbbRise-<version>-windows-x64.zip --repo coding-tree-io/EbbRise
```

(Earlier hand-built releases have no attestation; check their SHA-256 only.)

On the first start, EbbRise imports your Stretchly settings if Stretchly is installed.

## Privacy

EbbRise has no telemetry and no account. Its settings (`config.json`), statistics, sounds and logs are plain files in `%APPDATA%\EbbRise`. The only network call is the update check: it asks the GitHub API for the latest release (the schedule is in [`update.rs`](crates/app/src/update.rs)). Turn it off with `"checkNewVersion": false` in `config.json`, or check by hand from the tray menu.

## Features

- Mini and Long breaks on their own clocks, with notifications before them, strict mode, postpone and skip.
- Breaks in a window or full screen, with a countdown ring and an evidence-based idea.
- Choose how time away affects breaks under Settings → Schedule → Break timing: Keep counting, Pause timer or Count as break. Breaks wait during Do Not Disturb.
- Settings window in light, dark or the Windows theme. The tray, the notifications and most of Settings use Stretchly's translations ([all languages](crates/i18n/locales)).
- From [Workrave](https://workrave.org), as opt-in settings on the Activity page: a daily limit on PC use and an input block for other apps during breaks. Strict mode is a separate setting: it removes Skip and Postpone.
- Statistics for each day: breaks shown, completed, taken naturally, skipped and postponed, and active time. Open them from the tray menu or with `ebbrise statistics`.
- Built-in sounds; put your own WAV files in `%APPDATA%\EbbRise\audio`.
- Stretchly's command line (see below).

## Command line

| Command | What it does |
| --- | --- |
| `ebbrise pause -d 1h` | Pause breaks for an hour (`indefinitely`, `until-morning`, `1h30m`, `90m`) |
| `ebbrise resume` / `ebbrise toggle` | Resume, or pause and resume |
| `ebbrise mini` / `ebbrise long` | Start a Mini or Long break now (`-w` waits first, `-T` and `-t` set the title and text) |
| `ebbrise postpone` / `skip` / `finish` | Act on the running break, when allowed |
| `ebbrise preferences` / `statistics` | Open Settings or Statistics |
| `ebbrise logs` / `version` / `help` | Show the log location, the version, or all commands |

## Build

Requires Rust at least as new as `rust-version` in [Cargo.toml](Cargo.toml) and, for the exe icon and version details, the Windows 10/11 SDK (`rc.exe`).

```bash
cargo build --release
```

The exe is `target/release/ebbrise.exe`. Checks: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.

- [Backend inventory](docs/backend-inventory.md): every backend behaviour and OS integration, with the Windows mechanism for each.
- [ADR 0001: technology stack](docs/decisions/0001-technology-stack.md) and [ADR 0002: modules and libraries](docs/decisions/0002-modules-and-libraries.md).
- [Workrave features](docs/workrave-features.md): the clean-room spec of the features taken from Workrave (GPL-3.0; no code copied).
- [Test plan](docs/test-port-plan.md) and [project language](docs/language.md).

Not ported: global hotkeys.

## Contributing, support and security

EbbRise has one maintainer and does not take outside code pull requests for now. Bug reports and ideas are welcome: see [CONTRIBUTING.md](CONTRIBUTING.md). For help, see [SUPPORT.md](SUPPORT.md). Report security problems privately, as [SECURITY.md](SECURITY.md) describes. What changed in each release is in [CHANGELOG.md](CHANGELOG.md). Everyone takes part under the [Code of Conduct](CODE_OF_CONDUCT.md).

## Acknowledgements

EbbRise stands on [Stretchly](https://github.com/hovancik/stretchly) by Jan Hovancik (BSD-2-Clause), whose behaviour, settings and translations it follows; on [Workrave](https://workrave.org) for the ideas behind the opt-in features (clean-room, no code copied); and on [Slint](https://slint.dev) for the UI. The built-in sounds are credited in [crates/app/audio/CREDITS.md](crates/app/audio/CREDITS.md).

## License

EbbRise's own code is [0BSD](LICENSE): use it for anything, with no conditions. EbbRise is a port of [Stretchly](https://github.com/hovancik/stretchly), which is BSD-2-Clause, so the ported parts keep Stretchly's notice in [LICENSE-STRETCHLY](LICENSE-STRETCHLY) (SPDX for each crate: `0BSD AND BSD-2-Clause`).

[THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt) holds Stretchly's notice, the credits of the built-in sounds and the license of every crate in the exe. Every release ships `LICENSE`, `LICENSE-STRETCHLY` and `THIRD-PARTY-NOTICES.txt` next to the exe. After a dependency change, run `python -I scripts/third_party_notices.py` again.

The UI uses [Slint](https://slint.dev) under its Royalty-free Desktop Application License 2.0. That license asks for the badge below on the public page the exe is downloaded from (this README and the releases page). Forks that ship binaries must do the same, or choose one of Slint's other licenses.

[![Made with Slint](https://raw.githubusercontent.com/slint-ui/slint/master/logo/MadeWithSlint-logo-whitebg.png)](https://slint.dev)
