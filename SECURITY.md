# Security policy

## Supported versions

Only the [latest release](https://github.com/coding-tree-io/EbbRise/releases/latest) gets security fixes. Update to it before you report a problem.

## Report a vulnerability

Please do not open a public issue for a security problem.

Use GitHub's private reporting: go to the [Security tab](https://github.com/coding-tree-io/EbbRise/security) and select **Report a vulnerability**, or open <https://github.com/coding-tree-io/EbbRise/security/advisories/new>.

Say which version you tested (`ebbrise version`), what you did and what happened. EbbRise has one maintainer, so replies can take a few days. A confirmed problem is fixed in a new release and credited in the release notes, unless you ask not to be named.

## What to know about EbbRise

- The exe is not code-signed. Windows SmartScreen warns on the first start. Check the SHA-256 of your download against the release notes, or verify its build attestation (see [Verify your download](README.md#verify-your-download)).
- The only network call is the update check to `api.github.com`. You can turn it off with `checkNewVersion` in `%APPDATA%\EbbRise\config.json`.
- The opt-in setting that blocks keyboard and mouse input during breaks never blocks Ctrl+Alt+Del.
- Settings, statistics and logs are plain files in `%APPDATA%\EbbRise`.

Releases are built by GitHub Actions from a tagged commit, and the workflow files are in [.github/workflows](.github/workflows).
