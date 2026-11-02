#!/usr/bin/env bash
# tpab.sh WORK REF: measurement only (lane BEAMER-FRAME, not THROUGHPUT's A/B): beamer keystrokes'
# instructions with BOX-MEMO on and off on the PC (t7.py letter@SITE, 12 keys). REF is a measure
# commit with #1889's T7 harness and realdocs.json listing long-deck and three real-world fixtures.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK=${1:?WORK}
shift
REF=${1:?REF}
mkdir -p "$WORK/r" "$WORK/tmp"
WORK=$(cd "$WORK" && pwd)
export INCR_BENCH_DIR="$WORK/ib" TMPDIR="$WORK/tmp"
mkdir -p "$INCR_BENCH_DIR"
TBASE=${CARGO_TARGET_DIR_BASE:-$WORK/target}
src="$WORK/src"
git -C "$ROOT" worktree remove --force "$src" 2>/dev/null || true
git -C "$ROOT" worktree add --detach "$src" "$REF" >/dev/null
echo "build $REF = $(git -C "$src" rev-parse --short HEAD)" | tee "$WORK/r/engines.txt"
(cd "$src" && CARGO_TARGET_DIR="$TBASE-0" nice -n 5 cargo build --release --locked -q -p flashtex-engine -p flashtex-display-list)
(cd "$src" && CARGO_TARGET_DIR="$TBASE-0" bash tools/incr-bench/mkeng.sh cov >/dev/null)
cd "$src"
python3 tools/incr-bench/mkreal.py long-deck beamer-madrid beamer-visuals beamer-blocks-columns
for t in long-deck:middle long-deck:start beamer-madrid:middle beamer-visuals:start beamer-blocks-columns:end beamer-madrid:start; do
  doc=${t%%:*}; site=${t#*:}
  for m in on off; do
    o="$WORK/r/$doc-$site-$m"
    echo "== $doc $site $m"
    FLASHTEX_BOXMEMO=$m python3 tools/incr-bench/t7.py --engine cov --docs "$doc" --phases letter@site --sites "$site" --keys 12 \
      --preamble 0 --reopen 0 --max-load 999 --out "$o" >/dev/null 2>&1 || echo "  exit $?"
  done
done
cd "$WORK"
git -C "$ROOT" worktree remove --force "$src" || true
echo "done" > "$WORK/r/summary.md"
