#!/bin/sh
# Regenerate the XeTeX port's translation (docs/design/xetex/PLAN.md):
#
#   scripts/xetex-regenerate.sh
#
# writes crates/flashtex-xetex/src/generated/ and crates/flashtex-xetex/xetex.pool
# from third_party/xetex/xetex.web and the change files that
# crates/flashtex-xetex/web2rust-default.args lists. Never edit the generated
# files: change a change file or the translator and run this. The drift test
# (`cargo test --release -p web2rust --test drift`) fails when the committed
# translation and a fresh one differ.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
cargo build --release --locked -p web2rust
"${CARGO_TARGET_DIR:-$root/target}/release/web2rust" third_party/xetex/xetex.web \
    @crates/flashtex-xetex/web2rust-default.args \
    --out-dir crates/flashtex-xetex/src/generated \
    --pool crates/flashtex-xetex/xetex.pool
