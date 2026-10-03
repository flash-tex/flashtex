#!/bin/bash
# regen.sh: regenerate crates/flashtex-engine/src/generated from pdftex.web and
# the change files (tools/web2rust/README.md), then build the release engine.
set -e
W=$(cd "$(dirname "$0")/../../../.." && pwd)
cd "$W"
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-4}
cargo run -q --release -p web2rust -- third_party/pdftex/pdftex.web \
  @crates/flashtex-engine/web2rust-default.args \
  --out-dir crates/flashtex-engine/src/generated \
  --pool crates/flashtex-engine/pdftex.pool 2>&1 | tail -1
cargo build -q --release -p flashtex-engine 2>&1 | grep -E "^(error|warning)" -A9 | head -60 || true
