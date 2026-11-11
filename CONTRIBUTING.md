# Contributing to EbbRise

EbbRise has one maintainer and, for now, **does not take code pull requests from others**. A pull request that nobody asked for will most likely be closed unread. This is about the maintainer's time, not about you.

What helps:

- **Bug reports and ideas.** Use the [issue forms](https://github.com/coding-tree-io/EbbRise/issues/new/choose), or [Discussions](https://github.com/coding-tree-io/EbbRise/discussions) for questions.
- **Translations, wording and study suggestions.** Open an issue first. A break idea must name a real study.
- **Forks.** The code is [0BSD](LICENSE): fork it and change it. If your fork ships binaries, see the Slint badge rule in the [README](README.md#license).

Everyone takes part under the [Code of Conduct](CODE_OF_CONDUCT.md). Security problems go through [SECURITY.md](SECURITY.md), never a public issue.

If this policy changes, this file will say so.

## Working on the code

For the maintainer and for forks.

Requirements: Rust at least as new as `rust-version` in [Cargo.toml](Cargo.toml) and, for the exe icon and version details, the Windows 10/11 SDK (`rc.exe`). The app and the `platform` crate are Windows-only.

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
```

CI runs the first three on every push.

### Rules the checks enforce

- `crates/app/tests/architecture.rs`: every source file stays under the code-line limit set in that test, and crates depend on each other only along the allowed edges ([ADR 0002](docs/decisions/0002-modules-and-libraries.md)).
- `clippy.toml`: limits on complexity, function length, arguments and nesting. Fix the code; do not raise a limit.
- `missing_docs` and the `missing_*_doc` lints are on: document public items.
- The `platform` tests call real Win32 APIs (hidden windows, timers, `SendInput`, the notification history) and need an interactive desktop.

### Habits

- **Commits** follow [Conventional Commits](https://www.conventionalcommits.org): `feat(break): ...`, `fix(statistics): ...`, `chore(release): EbbRise X.Y.Z`.
- **After a dependency change**, run `python -I scripts/third_party_notices.py` and commit `THIRD-PARTY-NOTICES.txt`. CI fails if it is out of date. Dependencies are pinned with `=x.y.z`.
- **Workrave features** were written from a clean-room spec ([docs/workrave-features.md](docs/workrave-features.md)). Workrave is GPL-3.0. Do not copy or translate its code.
- **New texts** are English only until translations catch up. Stretchly's translations are used for the rest.
- **Changelog:** add user-visible changes under `Unreleased` in [CHANGELOG.md](CHANGELOG.md).

### Map of the docs

[Backend inventory](docs/backend-inventory.md), [ADR 0001](docs/decisions/0001-technology-stack.md), [ADR 0002](docs/decisions/0002-modules-and-libraries.md), [project language](docs/language.md), [test plan](docs/test-port-plan.md), [performance](docs/performance.md) and [maintaining](docs/maintaining.md) (releases and repository settings).
