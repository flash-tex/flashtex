#!/usr/bin/env bash
# Build and launch the Mac app for development.
#
#   scripts/run-mac-dev.sh [--no-build] [FILE.tex | FOLDER]
#
# Builds, in release: flashtex-host (the pdfLaTeX-compatible engine, a
# separate process), flashtex-bridge (captures, project files) and the app,
# then launches apps/mac/.build/release/FlashTeXMac with
#   FLASHTEX_ENGINE_V3=1   the engine session on, even if the shell set it to 0
#                          (apps/mac/docs/engine-v3-preview.md)
#   FLASHTEX_HOST, FLASHTEX_BRIDGE, FLASHTEX_REPO pointing at this checkout
#   FLASHTEX_OPEN=FILE     when a file or folder is given: open it at launch
# --no-build skips the builds. CARGO_BUILD_JOBS caps cargo (default 6).
# The first launch builds pdflatex.fmt from your TeX Live (about 5 s; the
# pane says so); later launches reuse it.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD=1
OPEN=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-build) BUILD=0; shift ;;
    -h|--help) sed -n '2,15p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) OPEN="$1"; shift ;;
  esac
done
if [[ -n "$OPEN" ]]; then
  [[ -e "$OPEN" ]] || { echo "run-mac-dev.sh: $OPEN does not exist" >&2; exit 1; }
  OPEN="$(cd "$(dirname "$OPEN")" && pwd)/$(basename "$OPEN")"
fi
if [[ "$BUILD" -eq 1 ]]; then
  echo "==> cargo build --release (flashtex-host, flashtex-bridge)"
  (cd "$ROOT" && CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-6}" cargo build --release -p flashtex-engine --bin flashtex-host \
    && cargo build --release -p flashtex-bridge)
  echo "==> swift build -c release (FlashTeXMac)"
  (cd "$ROOT/apps/mac" && swift build -c release --product FlashTeXMac)
fi
for f in "$ROOT/target/release/flashtex-host" "$ROOT/target/release/flashtex-bridge" "$ROOT/apps/mac/.build/release/FlashTeXMac"; do
  [[ -x "$f" ]] || { echo "run-mac-dev.sh: $f is not built (run without --no-build)" >&2; exit 1; }
done
export FLASHTEX_ENGINE_V3=1
export FLASHTEX_HOST="$ROOT/target/release/flashtex-host"
export FLASHTEX_POOL="$ROOT/crates/flashtex-engine/pdftex.pool"
export FLASHTEX_BRIDGE="$ROOT/target/release/flashtex-bridge"
export FLASHTEX_REPO="$ROOT"
[[ -n "$OPEN" ]] && export FLASHTEX_OPEN="$OPEN"
echo "==> launching FlashTeXMac${OPEN:+ with $OPEN}"
exec "$ROOT/apps/mac/.build/release/FlashTeXMac"
