#!/usr/bin/env bash
# Check that ci.yml's `old_engine` path filter still names every crate the
# old engine's parity fixtures build.
#
# `plan` in .github/workflows/ci.yml runs the old engine's parity fixtures in
# the merge queue and on pull requests only when the change touches one of
# their paths, and the crate part of that list (`old_engine_crates=`) is
# written out by hand: `plan` runs before any checkout and cannot ask cargo.
# This script asks cargo instead. Every path package in the dependency graph
# of crates/flashtex-cli (dev-dependencies included) that lives under crates/
# must be on the list; a missing one fails, an extra one only warns (the
# filter errs towards running).
#
# The parity-fixtures action runs it. A new path dependency can only be added
# in a Cargo.toml that is already on the list, which makes the action run, so
# the check runs whenever the list can go stale.
#
# Usage: scripts/ci/check-old-engine-paths.sh   (from anywhere in the repo)
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ci="$root/.github/workflows/ci.yml"

listed="$(sed -n "s/^[[:space:]]*old_engine_crates='\([^']*\)'.*/\1/p" "$ci")"
if [[ -z "$listed" ]]; then
  echo "check-old-engine-paths: no old_engine_crates='...' line in $ci" >&2
  exit 1
fi

metadata="$(mktemp)"
trap 'rm -f "$metadata"' EXIT
cargo metadata --locked --format-version 1 \
  --manifest-path "$root/crates/flashtex-cli/Cargo.toml" > "$metadata"

LISTED="$listed" ROOT="$root" python3 - "$metadata" <<'PY'
import json, os, sys

meta = json.load(open(sys.argv[1]))
root = os.path.realpath(os.environ["ROOT"])
listed = set(os.environ["LISTED"].split("|"))
packages = {p["id"]: p for p in meta["packages"]}
nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}

needed, seen, stack = set(), set(), [meta["resolve"]["root"]]
while stack:
    pid = stack.pop()
    if pid in seen:
        continue
    seen.add(pid)
    stack += [d["pkg"] for d in nodes[pid]["deps"]]
    pkg = packages[pid]
    if pkg["source"] is not None:
        continue  # a registry or git crate: Cargo.lock covers it
    rel = os.path.relpath(os.path.dirname(os.path.realpath(pkg["manifest_path"])), root)
    parts = rel.split(os.sep)
    if len(parts) != 2 or parts[0] != "crates":
        sys.exit("check-old-engine-paths: path dependency outside crates/<name>: %s "
                 "(add it to plan's old_engine filter by hand, then to this check)" % rel)
    needed.add(parts[1])

missing = sorted(needed - listed)
extra = sorted(listed - needed)
for name in extra:
    print("warning: old_engine_crates lists %s, which flashtex-cli no longer depends on" % name)
if missing:
    print("old_engine_crates in .github/workflows/ci.yml is missing crates flashtex-cli depends on:")
    for name in missing:
        print("  crates/%s" % name)
    print("add them, or the merge queue will skip the old engine's parity fixtures for changes to them")
    sys.exit(1)
print("old_engine_crates covers all %d path crates flashtex-cli depends on" % len(needed))
PY
