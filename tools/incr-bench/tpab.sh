#!/usr/bin/env bash
# tpab.sh WORK REF: measurement only (lane BEAMER-FRAME, not THROUGHPUT's A/B): the edit windows of
# beamer keystrokes by Rust function on the PC. REF is a measure commit with #1889's T7 harness and
# realdocs.json listing long-deck and three real-world beamer fixtures. For each site: `perf record
# -k CLOCK_MONOTONIC` (instructions:u) of a t7.py letter@SITE run with FLASHTEX_PERF_MARKS, its
# samples on the engine thread inside the b..e windows aggregated by symbol (winagg.py). Results in WORK/r.
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
for t in long-deck:middle long-deck:start beamer-madrid:middle beamer-visuals:start beamer-blocks-columns:end; do
  doc=${t%%:*}; site=${t#*:}
  o="$WORK/r/$doc-$site"
  mkdir -p "$o"
  echo "== $doc $site"
  rm -f "$o/marks"
  FLASHTEX_PERF_MARKS="$o/marks" perf record -q -k CLOCK_MONOTONIC -e instructions:u -c 20000 -o "$WORK/perf.data" -- \
    python3 tools/incr-bench/t7.py --engine cov --docs "$doc" --phases letter@site --sites "$site" --keys 12 \
    --preamble 0 --reopen 0 --max-load 999 --out "$o/t7" >/dev/null 2>&1 || echo "  exit $?"
  perf script -i "$WORK/perf.data" -F comm,tid,time,sym 2>/dev/null > "$WORK/perf.script" || true
  python3 "$ROOT/tools/incr-bench/winagg.py" "$WORK/perf.script" "$o/marks" "$o/window.txt" || echo "  winagg failed"
  rm -f "$WORK/perf.data" "$WORK/perf.script"
done
cd "$WORK"
git -C "$ROOT" worktree remove --force "$src" || true
echo "done" > "$WORK/r/summary.md"
