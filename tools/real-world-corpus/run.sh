#!/bin/bash
# Convenience wrapper: runs run.py with this checkout's own compiler, render
# worker and pdf routes. Every path can be overridden through the
# environment; missing binaries are reported, not built.
#
#   FLASHTEX_COMPILER   target/release/flashtex-compiler (root workspace)
#   FLASHTEX_RENDER     crates/render-pipeline/target/release/flashtex-render
#   FLASHTEX_PDF_EXACT  target/release/flashtex-pdf-exact
#   FLASHTEX_PDF        target/release/flashtex-pdf
#   FLASHTEX_RENDER_NOTE  human provenance sentence for the render binary
#
# The render worker defaults to THIS checkout's own build, not a scratch build
# of some other branch. It used to default to
# tools/real-world-corpus/target/render-pipeline-9aaec57a/... and to label
# every report "origin/agent/mac-render-pipeline/unified @ 9aaec57a" whether
# or not that was the binary that ran -- a provenance claim the harness had
# not checked. Build it with:
#   cargo build --release --manifest-path crates/render-pipeline/Cargo.toml --bin flashtex-render
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HEAD_SHA="$(git -C "$ROOT" rev-parse --short HEAD)"
# compiler and pdf are root-workspace members (Cargo.toml): their binaries are in
# the repository's target/, render-pipeline's still in crates/render-pipeline/target.
WS_TARGET="$("$ROOT/scripts/crate-target-dir.sh" "$ROOT/crates/compiler" 2>/dev/null || echo "$ROOT/target")"
COMPILER="${FLASHTEX_COMPILER:-$WS_TARGET/release/flashtex-compiler}"
RENDER="${FLASHTEX_RENDER:-$ROOT/crates/render-pipeline/target/release/flashtex-render}"
RENDER_NOTE="${FLASHTEX_RENDER_NOTE:-crates/render-pipeline of this checkout @ $HEAD_SHA}"
PDF_EXACT="${FLASHTEX_PDF_EXACT:-$WS_TARGET/release/flashtex-pdf-exact}"
PDF_V1="${FLASHTEX_PDF:-$WS_TARGET/release/flashtex-pdf}"
exec python3 "$ROOT/tools/real-world-corpus/run.py" \
  --compiler "$COMPILER" --compiler-note "crates/compiler of this checkout @ $HEAD_SHA" \
  --render "$RENDER" --render-note "$RENDER_NOTE" \
  --pdf-exact "$PDF_EXACT" --pdf-v1 "$PDF_V1" "$@"
