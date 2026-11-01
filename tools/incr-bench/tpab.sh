#!/usr/bin/env bash
# tpab.sh WORK REF: measurement only (lane BEAMER-FRAME, not THROUGHPUT's A/B): BOX-MEMO coverage
# of beamer decks' edited keystrokes on the PC. REF is a measure commit with #1889's T7 harness and
# realdocs.json listing long-deck and three real-world beamer fixtures. For each document and
# site: a plain t7.py letter@SITE run (the host's per-key instructions), then one with the macro
# profile counting instructions, BOX-MEMO's stats and its debug lines. Results in WORK/r.
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
DOCS="long-deck beamer-madrid beamer-visuals beamer-blocks-columns"
python3 tools/incr-bench/mkreal.py $DOCS
KEYS=${COV_KEYS:-10}
for doc in $DOCS; do
  for site in start middle end; do
    t=$doc-$site
    echo "== $t plain"
    python3 tools/incr-bench/t7.py --engine cov --docs "$doc" --phases letter@site --sites "$site" --keys "$KEYS" \
      --preamble 0 --reopen 0 --max-load 999 --out "$WORK/r/plain-$t" >/dev/null 2>&1 || echo "  plain $t: exit $?"
    echo "== $t profiled"
    P="$WORK/r/prof-$t"
    mkdir -p "$P"
    FLASHTEX_MACRO_PROFILE="$P/p" FLASHTEX_MACRO_PROFILE_CLOCK=instr FLASHTEX_BOXMEMO_STATS="$P/bmstats.txt" \
      FLASHTEX_BOXMEMO_DEBUG=1 \
      python3 tools/incr-bench/t7.py --engine cov --docs "$doc" --phases letter@site --sites "$site" --keys "$KEYS" \
      --preamble 0 --reopen 0 --max-load 999 --out "$P/t7" >/dev/null 2>&1 || echo "  profiled $t: exit $?"
  done
done
cd "$WORK"
git -C "$ROOT" worktree remove --force "$src" || true
echo "done" > "$WORK/r/summary.md"
