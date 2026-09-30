#!/usr/bin/env bash
# APP-PERF-AUDIT: launch the release FlashTeXMac on a seed document with the
# current (old-engine) producer and the signpost flag; prints the PID.
# usage: launch.sh <seed.tex> <outdir> [VAR=value ...]
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
SEED="$1"; OUT="$2"; shift 2
APP_BIN="${APP_BIN:-$ROOT/apps/mac/.build/release/FlashTeXMac}"
RENDER="${RENDER:-$ROOT/target/release/flashtex-render}"
mkdir -p "$OUT"
cp "$SEED" "$OUT/seed.tex"
echo "load_at_launch $(sysctl -n vm.loadavg)" >> "$OUT/load.txt"
env FLASHTEX_REPO="$ROOT" FLASHTEX_AUTOATTACH=1 FLASHTEX_NO_FILE_WATCH=1 \
  FLASHTEX_LM_DIR="$ROOT/apps/mac/Fonts" FLASHTEX_FONT_DIRS="$ROOT/apps/mac/Fonts" \
  FLASHTEX_COMPILER="$RENDER" FLASHTEX_SEED_FILE="$OUT/seed.tex" FLASHTEX_LOG="$OUT/app.log" \
  FLASHTEX_SIGNPOSTS=1 "$@" "$APP_BIN" > "$OUT/stdout.txt" 2>&1 &
echo $!
