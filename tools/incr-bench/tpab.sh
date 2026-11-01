#!/usr/bin/env bash
# tpab.sh WORK REF: measurement only (lane BEAMER-FRAME, not THROUGHPUT's A/B): where an edited
# beamer keystroke's instructions go, by Rust function, on the PC. REF is a measure commit with
# #1889's T7 harness and realdocs.json listing long-deck and three real-world beamer fixtures. For
# each site: `perf record` (instructions:u, every process t7.py starts) of a t7.py letter@SITE run
# with 2 keys and one with 22, each reported by symbol; the difference is 20 keys. Results in WORK/r.
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
python3 tools/incr-bench/mkreal.py long-deck beamer-madrid beamer-visuals
for t in long-deck:middle beamer-madrid:middle beamer-visuals:start; do
  doc=${t%%:*}; site=${t#*:}
  for k in 2 22; do
    o="$WORK/r/$doc-$site-k$k"
    mkdir -p "$o"
    echo "== $doc $site keys $k"
    perf record -q -e instructions:u -c 200000 -o "$WORK/perf.data" -- \
      python3 tools/incr-bench/t7.py --engine cov --docs "$doc" --phases letter@site --sites "$site" --keys "$k" \
      --preamble 0 --reopen 0 --max-load 999 --out "$o/t7" >/dev/null 2>&1 || echo "  exit $?"
    perf report -i "$WORK/perf.data" --no-children --sort comm,dso,symbol --stdio -n --percent-limit 0.02 \
      2>/dev/null | grep '%' | grep -v '^#' | head -400 > "$o/perf.txt" || true
    rm -f "$WORK/perf.data"
  done
done
cd "$WORK"
git -C "$ROOT" worktree remove --force "$src" || true
echo "done" > "$WORK/r/summary.md"
