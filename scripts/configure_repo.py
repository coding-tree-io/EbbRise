"""Applies the GitHub repository settings that cannot live in files.

Prints the plan by default; add --apply to change the repository. Needs the GitHub CLI (`gh`),
signed in as an owner of the repository: python -I scripts/configure_repo.py [--apply]

The About description comes from `description` in Cargo.toml, so there is one copy of it.
Safe to run again: every step sets a state rather than adding to it.
"""

import json
import subprocess
import sys
import tomllib
from pathlib import Path

TOPICS = [
    "rust", "slint", "windows", "windows-11", "break-reminder", "stretchly",
    "eye-strain", "ergonomics", "productivity", "system-tray", "desktop-app",
]
LABELS = {
    "bug": ("d73a4a", "Something does not work"),
    "enhancement": ("a2eeef", "New feature or request"),
    "needs-triage": ("fbca04", "Not looked at yet"),
    "dependencies": ("0366d6", "Dependency updates"),
    "rust": ("dea584", "Cargo and Rust updates"),
    "github-actions": ("000000", "Workflow updates"),
}
# Job names in .github/workflows that must pass before anything reaches the default branch.
REQUIRED_CHECKS = ["CI", "cargo-deny"]


def gh(args, apply, stdin=None):
    """Runs `gh` (or prints it). Returns stdout, or None when it failed or was only planned."""
    shown = "gh " + " ".join(args) + (" < " + stdin.replace("\n", " ") if stdin else "")
    print(("RUN  " if apply else "PLAN ") + shown)
    if not apply:
        return None
    done = subprocess.run(["gh", *args], input=stdin, text=True, capture_output=True, encoding="utf-8")
    if done.returncode:
        print("  failed:", done.stderr.strip() or done.stdout.strip())
        return None
    return done.stdout


def main() -> int:
    apply = "--apply" in sys.argv
    cargo = tomllib.loads(Path("Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]
    # `{owner}/{repo}` is filled in by gh from this checkout's git remote. REST calls only.
    repo = subprocess.run(["gh", "api", "repos/{owner}/{repo}", "--jq", ".full_name"],
                          capture_output=True, text=True, check=True).stdout.strip()

    settings = {
        "description": cargo["description"], "homepage": f"{cargo['repository']}/releases/latest",
        "has_discussions": True, "has_wiki": False,
        "allow_auto_merge": True, "delete_branch_on_merge": True,
        "allow_squash_merge": True, "allow_merge_commit": False, "allow_rebase_merge": False,
    }
    gh(["api", "-X", "PATCH", f"repos/{repo}", "--input", "-"], apply, json.dumps(settings))
    gh(["api", "-X", "PUT", f"repos/{repo}/topics", "--input", "-"], apply, json.dumps({"names": TOPICS}))

    for path in ("vulnerability-alerts", "automated-security-fixes", "private-vulnerability-reporting"):
        gh(["api", "-X", "PUT", f"repos/{repo}/{path}"], apply)
    scanning = {"security_and_analysis": {
        "secret_scanning": {"status": "enabled"},
        "secret_scanning_push_protection": {"status": "enabled"}}}
    gh(["api", "-X", "PATCH", f"repos/{repo}", "--input", "-"], apply, json.dumps(scanning))

    for name, (color, text) in LABELS.items():
        label = json.dumps({"name": name, "color": color, "description": text})
        if gh(["api", "-X", "POST", f"repos/{repo}/labels", "--input", "-"], apply, label) is None and apply:
            gh(["api", "-X", "PATCH", f"repos/{repo}/labels/{name}", "--input", "-"], apply, label)

    ruleset = {
        "name": "main", "target": "branch", "enforcement": "active",
        "conditions": {"ref_name": {"include": ["~DEFAULT_BRANCH"], "exclude": []}},
        # The owner can still push directly; everyone else, and Dependabot, needs green checks.
        "bypass_actors": [{"actor_id": 5, "actor_type": "RepositoryRole", "bypass_mode": "always"}],
        "rules": [
            {"type": "deletion"},
            {"type": "non_fast_forward"},
            {"type": "required_status_checks", "parameters": {
                "strict_required_status_checks_policy": False,
                "required_status_checks": [{"context": c} for c in REQUIRED_CHECKS]}},
        ],
    }
    existing = gh(["api", f"repos/{repo}/rulesets"], True) if apply else None
    found = [r["id"] for r in json.loads(existing or "[]") if r["name"] == "main"]
    method, path = ("PUT", f"rulesets/{found[0]}") if found else ("POST", "rulesets")
    gh(["api", "-X", method, f"repos/{repo}/{path}", "--input", "-"], apply, json.dumps(ruleset))

    print("\nStill manual: upload docs/images/social-preview.png under Settings > Social preview.")
    if not apply:
        print("This was a plan. Run again with --apply to make the changes.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
