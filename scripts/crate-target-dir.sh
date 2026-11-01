#!/usr/bin/env bash
# Prints the Cargo target directory a crate builds into.
#
#   scripts/crate-target-dir.sh <crate-dir>        e.g. crates/compiler
#   -> /path/to/repo/target                (every crate is a workspace member)
#
# Every crate is a member of the root Cargo workspace (Cargo.toml), so they all
# build into the repository's ./target, not crates/<name>/target. Honours
# CARGO_TARGET_DIR and .cargo/config.toml, because it asks Cargo. Scripts must
# use this instead of hard-coding either: an old binary left in
# crates/<name>/target from before the workspace (or from before
# crates/render-pipeline/vendor/ was retired) would otherwise be picked up
# silently.
set -euo pipefail
dir="${1:?usage: crate-target-dir.sh <crate-dir>}"
cargo metadata --format-version 1 --no-deps --offline \
    --manifest-path "$dir/Cargo.toml" 2>/dev/null |
  python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])'
