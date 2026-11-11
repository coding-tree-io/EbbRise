# Maintaining EbbRise

Notes for the maintainer. Nothing here repeats a version number: the version lives in `crates/app/Cargo.toml`, the minimum Rust version in the workspace `Cargo.toml`, and everything else reads them.

## Where each value comes from

| Shown in | Read from |
| --- | --- |
| Release name, zip name, tag check | `version` in `crates/app/Cargo.toml` (`release.yml`) |
| Release notes | the matching section of `CHANGELOG.md` (`release.yml`) |
| README "minimum Rust" badge, build docs | `rust-version` in the workspace `Cargo.toml` |
| GitHub About description | `description` in the workspace `Cargo.toml` (`scripts/configure_repo.py`) |
| README release badge | the latest GitHub release |
| Licenses of dependencies | `THIRD-PARTY-NOTICES.txt` (`scripts/third_party_notices.py`) and `deny.toml` |

CI fails when `rust-version` is lower than a dependency needs (`scripts/check_rust_version.py`) and when `THIRD-PARTY-NOTICES.txt` is out of date.

## Make a release

1. In `CHANGELOG.md`, rename `Unreleased` to the new version and date, and add a fresh empty `Unreleased` section above it.
2. Raise `version` in `crates/app/Cargo.toml`. Run `cargo build` so `Cargo.lock` follows.
3. Commit as `chore(release): EbbRise X.Y.Z` and push to `main`. Wait for CI.
4. Tag and push the tag:

   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

`release.yml` then checks that the tag matches the app version and that the changelog has a section for it, builds the exe, zips it with the license files, attests the build, and creates the GitHub release with the SHA-256 and the Slint badge in the notes.

To rehearse without publishing, run the **Release** workflow by hand (Actions tab, Run workflow). It builds, checks the zip layout and uploads the zip as a workflow artifact only.

## Repository settings

Settings that are not files (About, topics, security features, labels, branch rules) are applied by one script. It prints a plan first:

```bash
python -I scripts/configure_repo.py
python -I scripts/configure_repo.py --apply
```

It needs the [GitHub CLI](https://cli.github.com), signed in as an owner. It also sets a ruleset on `main` that requires the `CI` and `cargo-deny` checks and blocks force pushes and deletion. Auto-merge for Dependabot depends on that ruleset: without required checks, auto-merge would merge at once.

One step stays manual: upload `docs/images/social-preview.png` under Settings → Social preview.

## Dependabot

`dependabot.yml` opens weekly pull requests for Cargo and for GitHub Actions, grouped by minor and patch updates, with a 7-day cooldown on new releases. `dependabot-automerge.yml` turns on auto-merge for everything except major updates. A pull request merges once the required checks pass. Major updates, and Cargo updates that change `THIRD-PARTY-NOTICES.txt` or need a newer Rust, wait for you:

```bash
python -I scripts/third_party_notices.py
```

## Screenshots

The README has none yet. Add PNGs under `docs/images/` and reference them from the README.
