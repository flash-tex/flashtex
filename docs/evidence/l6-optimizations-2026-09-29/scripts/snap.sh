#!/bin/bash
# snap.sh NAME: copy this worktree's sources (working tree, not a commit) to
# /tmp/l6o/src-NAME, so that variant builds of one source state can run while
# the worktree changes. variants.sh builds from $SRC when it is set.
set -e
W=$(cd "$(dirname "$0")/../../../.." && pwd)
D=/tmp/l6o/src-$1
mkdir -p "$D"
rsync -a --delete --exclude target --exclude '.git' "$W/Cargo.toml" "$W/Cargo.lock" "$W/crates" "$W/third_party" "$W/tools" "$W/fixtures" "$D/"
# The scripts too, so that a copy on another machine is self-contained.
mkdir -p "$D/docs/evidence/l6-optimizations-2026-09-29"
rsync -a --delete "$W/docs/evidence/l6-optimizations-2026-09-29/scripts" "$D/docs/evidence/l6-optimizations-2026-09-29/"
(cd "$W" && git rev-parse HEAD && git status --porcelain --untracked-files=no) > "$D/SNAPSHOT"
echo "$D: $(head -1 "$D/SNAPSHOT") + $(($(wc -l < "$D/SNAPSHOT") - 1)) modified files"
