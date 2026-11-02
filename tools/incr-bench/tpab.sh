#!/usr/bin/env bash
# tpab.sh WORK REF0 REF1: measurement only (lane BEAMER-FRAME, not THROUGHPUT's A/B): beamer
# keystrokes' instructions, REF0 against REF1, with BOX-MEMO on, on the PC (t7.py letter@SITE, 12
# keys), engines interleaved per site; then REF1 with FLASHTEX_BOXMEMO=verify and its stats (each
# replay checked against the normal path). REFs are measure commits with #1889's T7 harness and
# realdocs.json listing long-deck and three real-world beamer fixtures.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK=${1:?WORK}
shift
mkdir -p "$WORK/r" "$WORK/tmp"
WORK=$(cd "$WORK" && pwd)
export INCR_BENCH_DIR="$WORK/ib" TMPDIR="$WORK/tmp"
mkdir -p "$INCR_BENCH_DIR"
TBASE=${CARGO_TARGET_DIR_BASE:-$WORK/target}
i=0
for REF in "$@"; do
  src="$WORK/src-$i"
  git -C "$ROOT" worktree remove --force "$src" 2>/dev/null || true
  git -C "$ROOT" worktree add --detach "$src" "$REF" >/dev/null
  echo "e$i: $REF = $(git -C "$src" rev-parse --short HEAD)" | tee -a "$WORK/r/engines.txt"
  (cd "$src" && CARGO_TARGET_DIR="$TBASE-$i" nice -n 5 cargo build --release --locked -q -p flashtex-engine -p flashtex-display-list)
  (cd "$src" && CARGO_TARGET_DIR="$TBASE-$i" bash tools/incr-bench/mkeng.sh "e$i" >/dev/null)
  i=$((i + 1))
done
cd "$WORK/src-0"
python3 tools/incr-bench/mkreal.py long-deck beamer-madrid beamer-visuals beamer-blocks-columns
SITES=${COV_SITES:-beamer-visuals:start long-deck:middle}
ENGS=$(seq -f "e%g" 0 $((i - 1)) | tr "\n" " ")
for t in $SITES; do
  doc=${t%%:*}; site=${t#*:}
  for e in $ENGS; do
    echo "== $doc $site $e"
    FLASHTEX_BOXMEMO=on python3 tools/incr-bench/t7.py --engine $e --docs "$doc" --phases letter@site --sites "$site" --keys 12 \
      --preamble 0 --reopen 0 --max-load 999 --out "$WORK/r/$doc-$site-$e" >/dev/null 2>&1 || echo "  exit $?"
  done
done
# BOX-MEMO's edit windows by name (FLASHTEX_BOXMEMO_DEBUG=1), every engine, at COV_DEBUG_SITES: the
# host's stderr from each "window open" to its "window closed", values folded
for t in ${COV_DEBUG_SITES:-beamer-visuals:start long-deck:middle}; do
  doc=${t%%:*}; site=${t#*:}
  for e in $ENGS; do
    echo "== $doc $site $e debug"
    o="$WORK/r/dbg-$doc-$site-$e"
    FLASHTEX_BOXMEMO=on FLASHTEX_BOXMEMO_DEBUG=1 python3 tools/incr-bench/t7.py --engine $e --docs "$doc" \
      --phases letter@site --sites "$site" --keys 4 --preamble 0 --reopen 0 --max-load 999 \
      --out "$o" >/dev/null 2>&1 || echo "  exit $?"
    cat "$o"/raw/*.host-stderr 2>/dev/null | grep '^boxmemo' \
      | awk '/^boxmemo: window open/{w=1} w{print} /^boxmemo: window closed/{w=0}' \
      | sed -E 's/was \[[-0-9, ]*\] now \[[-0-9, ]*\]/V/g; s/=\[[-0-9, ]*\]/=V/g; s/ in [0-9.]+(ns|µs)//; s/, [0-9]+ bytes//; s/[0-9]+ meanings/N meanings/' \
      > "$o.window" || true
    rm -f "$o"/raw/*.host-stderr
  done
done
cd "$WORK"
for j in $(seq 0 $((i - 1))); do git -C "$ROOT" worktree remove --force "$WORK/src-$j" || true; done
echo "done" > "$WORK/r/summary.md"
