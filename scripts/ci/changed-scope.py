#!/usr/bin/env python3
"""Decide which CI job groups a pull request needs, from its changed paths.

Reads changed paths (one per line) on stdin and prints GitHub step outputs:

  rust=true|false          the Rust jobs (rust-workspace, rust-standalone)
  mac=true|false           the Mac app job
  ios=true|false           the iPad companion job
  rust_os=<json list>      OSes for the rust-workspace matrix
  standalone_matrix=<json> the rust-standalone matrix ({"include": [...]})

`--all` (or an empty path list) turns everything on. ci.yml passes `--all`
for every event that is not a pull request, so main, pushes and manual runs
always run every job on both OSes; only pull requests are narrowed.

Why (docs/ci-cd.md, "Pull-request scope"): GitHub runs at most 5 macOS jobs
at once for this repository, every full CI run needs 5 macOS jobs, and PR
runs queued for hours behind each other and behind main. The rules below only
skip a job when the paths it reads did not change, with two exceptions that
were measured before they were made:

  * The macOS legs of rust-workspace and render-pipeline do not run on pull
    requests. Over 253 CI runs (2026-09-23..27) a macOS Rust leg never failed
    while its Ubuntu twin passed. They still run on every push to main.
  * flashtex-cli keeps its macOS leg on pull requests: it carries the parity
    scoreboard baseline gate, which is recorded on macOS, and costs ~2 min.

Anything this file does not recognise counts as relevant (fail open).
"""
import json
import sys

# Paths no Rust build or test reads. A crate test that reads a file here
# would make this list wrong: grep crates/ and tools/ for the prefix first.
# (Rust tests do read apps/mac/Fonts, apps/mac/scripts, apps/mac/docs and
# apps/mac/Samples, so those are NOT listed.)
RUST_IRRELEVANT = (
    "apps/ios/",
    "apps/mac/Sources/",
    "apps/mac/Tests/",
    "apps/mac/Resources/",
    "apps/mac/tools/",
    "apps/mac/Package.swift",
    "apps/mac/Package.resolved",
)

# The iPad project is self-contained under apps/ios/ except for three
# symlinked Swift sources (apps/ios/Packages/FlashTeXPadKit/Sources/*).
IOS_RELEVANT = (
    "apps/ios/",
    "apps/mac/Sources/FlashTeXProtocol/",
    "apps/mac/Sources/FlashTeXEditorCore/",
    "apps/mac/tools/nearby-client/",
)

# The Mac app builds every helper from crates/, so any change outside the
# iPad project can reach it.
MAC_IRRELEVANT = ("apps/ios/",)

# Changing how CI selects jobs runs everything, so the change is exercised.
RUN_ALL = (
    ".github/workflows/ci.yml",
    "scripts/ci/changed-scope.py",
    "scripts/ci/build-helpers.sh",
)

UBUNTU, MACOS = "ubuntu-latest", "macos-15"
STANDALONE = ("render-pipeline", "flashtex-cli")


def scope(paths, run_all=False):
    paths = [p.strip() for p in paths if p.strip()]
    if run_all or not paths or any(p in RUN_ALL for p in paths):
        rust = mac = ios = True
        full = True
    else:
        rust = any(not p.startswith(RUST_IRRELEVANT) for p in paths)
        mac = any(not p.startswith(MAC_IRRELEVANT) for p in paths)
        ios = any(p.startswith(IOS_RELEVANT) for p in paths)
        full = False
    rust_os = [UBUNTU, MACOS] if full else [UBUNTU]
    include = [{"os": os, "crate": c} for os in (UBUNTU, MACOS) for c in STANDALONE
               if full or os == UBUNTU or c == "flashtex-cli"]
    return {
        "rust": rust,
        "mac": mac,
        "ios": ios,
        "rust_os": rust_os,
        "standalone_matrix": {"include": include},
    }


def main(argv):
    run_all = "--all" in argv[1:]
    paths = [] if run_all else sys.stdin.read().splitlines()
    s = scope(paths, run_all)
    for k, v in s.items():
        print(f"{k}={json.dumps(v, separators=(',', ':')) if not isinstance(v, bool) else str(v).lower()}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
