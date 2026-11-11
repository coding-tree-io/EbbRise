"""Fails when a dependency needs a newer Rust than the workspace's `rust-version`.

Run from the workspace root: python -I scripts/check_rust_version.py
The workspace value in Cargo.toml is the single place that docs and the README badge read.
"""

import json
import subprocess
import sys


def parse(version: str) -> tuple[int, ...]:
    return tuple(int(part) for part in version.split("."))


def main() -> int:
    run = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked",
         "--filter-platform", "x86_64-pc-windows-msvc"],
        capture_output=True, text=True, encoding="utf-8",  # Windows would guess cp1252
    )
    if run.returncode:
        print(f"error: cargo metadata failed (is Cargo.lock out of date?):\n{run.stderr}")
        return 1
    out = run.stdout
    packages = json.loads(out)["packages"]
    ours = {p["name"]: p["rust_version"] for p in packages if p["source"] is None}
    declared = {parse(v) for v in ours.values() if v}
    if len(declared) != 1:
        print(f"error: workspace crates must share one rust-version, found {sorted(ours.items())}")
        return 1
    (workspace,) = declared
    needs = max(
        ((parse(p["rust_version"]), p["name"]) for p in packages if p["source"] and p["rust_version"]),
        default=((0,), "none"),
    )
    shown = ".".join(map(str, workspace))
    if needs[0] > workspace:
        print(f"error: {needs[1]} needs Rust {'.'.join(map(str, needs[0]))}, "
              f"but rust-version in Cargo.toml is {shown}. Raise it.")
        return 1
    print(f"ok: rust-version {shown} covers every dependency (highest: {needs[1]})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
