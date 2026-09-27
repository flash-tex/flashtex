#!/usr/bin/env bash
# Prints the Cargo target directory a crate builds into.
#
#   scripts/crate-target-dir.sh <crate-dir>        e.g. crates/compiler
#   -> /path/to/repo/target                (workspace member)
#   -> /path/to/repo/crates/perf-bench/target  (standalone, see Cargo.toml)
#
# Since the root Cargo workspace (Cargo.toml), most crates build into the
# repository's ./target, not crates/<name>/target; a few are still standalone
# until crates/render-pipeline/vendor/ is deleted. Honours CARGO_TARGET_DIR and
# .cargo/config.toml, because it asks Cargo. Scripts must use this instead of
# hard-coding crates/<name>/target: an old binary left there from before the
# workspace would otherwise be picked up silently.
set -euo pipefail
dir="${1:?usage: crate-target-dir.sh <crate-dir>}"
cargo metadata --format-version 1 --no-deps --offline \
    --manifest-path "$dir/Cargo.toml" 2>/dev/null |
  python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])'
