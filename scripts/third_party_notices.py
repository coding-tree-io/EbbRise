"""Writes THIRD-PARTY-NOTICES.txt: Stretchly's notice and the licence files of every crate in the exe.

Run from the workspace root after a dependency change: python -I scripts/third_party_notices.py
It reads `cargo metadata` and the crate sources in the local cargo registry; it downloads nothing.
Crates used only at build time (proc macros and what only they use) are left out.
"""

import hashlib
import json
import subprocess
import sys
from pathlib import Path

TARGET = "x86_64-pc-windows-msvc"
NOTICE_NAMES = ("LICEN", "COPYING", "COPYRIGHT", "NOTICE", "UNLICENSE")
# Slint is offered under three licences; Ebb uses the royalty-free one.
SLINT_LICENCE = "LicenseRef-Slint-Royalty-free-2.0.md"
OUTPUT = Path("THIRD-PARTY-NOTICES.txt")
MIT_TEXT = """Copyright (c) {authors}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE."""
# For crates that ship no licence file: the start of a full text found in another crate's files.
REFERENCE_STARTS = {"Apache-2.0": "Apache License", "BSL-1.0": "Boost Software License - Version 1.0"}


def built_into_app():
    """`name version` of every crate `cargo tree` builds into the app with the features it uses.

    `cargo metadata` resolves features for the whole workspace, dev-dependencies included, so it
    lists crates (such as Skia) that the exe does not contain.
    """
    tree = subprocess.run(
        ["cargo", "tree", "-p", "app", "-e", "normal", "--target", TARGET,
         "--prefix", "none", "--format", "{p}"],
        check=True, capture_output=True, text=True, encoding="utf-8",
    ).stdout
    return {" ".join(line.split()[:2]).replace(" v", " ", 1) for line in tree.splitlines() if line}


def shipped_packages(meta):
    """Workspace dependencies reached through normal (not dev or build) edges, minus proc macros."""
    packages = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    members = set(meta["workspace_members"])
    seen, stack = set(), list(members)
    while stack:
        package_id = stack.pop()
        if package_id in seen:
            continue
        package = packages[package_id]
        if any("proc-macro" in t["kind"] for t in package["targets"]):
            continue
        seen.add(package_id)
        for dep in nodes[package_id]["deps"]:
            if any(kind["kind"] is None for kind in dep["dep_kinds"]):
                stack.append(dep["pkg"])
    built = built_into_app()
    shipped = [packages[i] for i in seen - members if f'{packages[i]["name"]} {packages[i]["version"]}' in built]
    return sorted(shipped, key=lambda p: (p["name"], p["version"]))


def notice_files(package):
    root = Path(package["manifest_path"]).parent
    if package["name"].startswith(("slint", "i-slint")):
        for folder in (root / "LICENSES", root):
            if (folder / SLINT_LICENCE).is_file():
                return [folder / SLINT_LICENCE]
    files = [f for f in root.iterdir() if f.is_file() and f.name.upper().startswith(NOTICE_NAMES)]
    if (root / "LICENSES").is_dir():
        files += [f for f in (root / "LICENSES").iterdir() if f.is_file()]
    return sorted(files)


def main():
    meta = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--filter-platform", TARGET],
            check=True, capture_output=True, text=True, encoding="utf-8",
        ).stdout
    )
    groups, missing = {}, []

    def add(text, label):
        key = hashlib.sha256(text.encode()).hexdigest()
        groups.setdefault(key, (text, []))[1].append(label)

    for package in shipped_packages(meta):
        label = f'{package["name"]} {package["version"]} ({package["license"]})'
        files = notice_files(package)
        if not files:
            missing.append((package, label))
        for file in files:
            add(file.read_text(encoding="utf-8", errors="replace").strip(), label)

    unresolved = []
    for package, label in missing:
        choices = package["license"].replace("(", "").replace(")", "").split(" OR ")
        if "MIT" in choices:
            authors = ", ".join(package["authors"]) or f'the {package["name"]} authors'
            add(MIT_TEXT.format(authors=authors), label + ", MIT chosen; no licence file in the crate")
            continue
        reference = next(
            (text for text, _ in groups.values() for spdx in choices
             if spdx in REFERENCE_STARTS and text.lstrip().startswith(REFERENCE_STARTS[spdx])),
            None,
        )
        if reference:
            add(reference, label + ", no licence file in the crate")
        else:
            unresolved.append(label)
    missing = unresolved

    parts = [
        "Third-party notices for Ebb",
        "",
        "Ebb is 0BSD (LICENSE). It is a port of Stretchly, whose notice follows.",
        "",
        "=" * 78,
        "Stretchly (https://github.com/hovancik/stretchly), BSD-2-Clause",
        "=" * 78,
        Path("LICENSE-STRETCHLY").read_text(encoding="utf-8").strip(),
    ]
    for text, labels in sorted(groups.values(), key=lambda group: group[1][0]):
        parts += ["", "=" * 78, *labels, "=" * 78, text]
    if missing:
        parts += ["", "=" * 78, "Crates without a licence file (licence named in Cargo.toml):", "=" * 78, *missing]
    OUTPUT.write_text("\n".join(parts) + "\n", encoding="utf-8", newline="\n")
    print(f"{sum(len(g[1]) for g in groups.values())} notice files, {len(groups)} distinct texts, "
          f"{len(missing)} crates without a file", file=sys.stderr)
    for label in missing:
        print("  no file:", label, file=sys.stderr)


if __name__ == "__main__":
    main()
