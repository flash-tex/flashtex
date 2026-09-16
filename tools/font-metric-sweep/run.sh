#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
export CARGO_TARGET_DIR=/Users/dqi26/flashtex/target-metricsweep
export CARGO_BUILD_JOBS=4

exec cargo run --manifest-path "$ROOT/crates/render-pipeline/Cargo.toml" \
  --bin font-metric-sweep -- "$@"
