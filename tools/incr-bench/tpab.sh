#!/usr/bin/env bash
# tpab.sh WORK REF0 REF1 DBG0 DBG1 (lane L6, measurement only, not THROUGHPUT's A/B): BOX-MEMO's
# keystrokes, REF0 (#2041's base) against REF1 (#2041), BOX-MEMO on, on the PC: t7.py letter@SITE,
# 12 keys, at four beamer sites, and letter@start/middle/end on full-100, engines interleaved per
# document; then DBG0/DBG1 (the same with BEAMER-FRAME's edit-window debug lines) with
# FLASHTEX_BOXMEMO_DEBUG=1 at beamer-visuals start and long-deck middle: the misses by name.
# l6keys.py tabulates instructions to the edited page and BOX-MEMO's offers, hits and recordings
# per key. REFs are measure commits with #1889's T7 harness and the beamer coverage docs.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK=${1:?WORK}
shift
mkdir -p "$WORK/r" "$WORK/tmp"
WORK=$(cd "$WORK" && pwd)
export INCR_BENCH_DIR="$WORK/ib" TMPDIR="$WORK/tmp"
mkdir -p "$INCR_BENCH_DIR"
# (the lane's own build directories: another lane's run on the same runner cannot swap binaries)
TBASE=$HOME/.cache/flashtex-agents/l6/keys-target
i=0
for REF in "$@"; do
  src="$WORK/src-$i"
  git -C "$ROOT" worktree remove --force "$src" 2>/dev/null || true
  git -C "$ROOT" worktree add --detach "$src" "$REF" >/dev/null
  echo "e$i: $REF = $(git -C "$src" rev-parse --short HEAD)" | tee -a "$WORK/r/engines.txt"
  # (e2, e3 differ from e0, e1 in boxmemo.rs only: their builds reuse e0's and e1's directories)
  (cd "$src" && CARGO_TARGET_DIR="$TBASE-$((i % 2))" nice -n 5 cargo build --release --locked -q -p flashtex-engine -p flashtex-display-list)
  (cd "$src" && CARGO_TARGET_DIR="$TBASE-$((i % 2))" bash tools/incr-bench/mkeng.sh "e$i" >/dev/null)
  i=$((i + 1))
done
cd "$WORK/src-0"
python3 tools/incr-bench/mkreal.py long-deck beamer-madrid beamer-visuals beamer-blocks-columns
python3 tools/incr-bench/mkdocs.py >/dev/null
ENGS="e0 e1"
for t in beamer-visuals:start long-deck:middle beamer-madrid:start beamer-blocks-columns:end; do
  doc=${t%%:*}; site=${t#*:}
  for e in $ENGS; do
    echo "== $doc $site $e"
    FLASHTEX_BOXMEMO=on python3 tools/incr-bench/t7.py --engine $e --docs "$doc" --phases letter@site --sites "$site" --keys 12 \
      --preamble 0 --reopen 0 --max-load 999 --out "$WORK/r/$doc-$site-$e" >/dev/null 2>&1 || echo "  exit $?"
  done
done
for e in $ENGS; do
  echo "== full-100 $e"
  FLASHTEX_BOXMEMO=on python3 tools/incr-bench/t7.py --engine $e --docs full-100 --phases letter@start,letter@middle,letter@end \
    --order fixed --keys 12 --typing-keys 2 --preamble 0 --reopen 0 --max-load 999 --out "$WORK/r/full-100-$e" >/dev/null 2>&1 || echo "  exit $?"
done
# the misses by name (FLASHTEX_BOXMEMO_DEBUG=1), the edit windows only, values folded, counted
for t in beamer-visuals:start long-deck:middle; do
  doc=${t%%:*}; site=${t#*:}
  for e in e2 e3; do
    echo "== $doc $site $e debug"
    d="$WORK/r/debug-$doc-$site-$e"
    FLASHTEX_BOXMEMO=on FLASHTEX_BOXMEMO_DEBUG=1 python3 tools/incr-bench/t7.py --engine $e --docs "$doc" \
      --phases letter@site --sites "$site" --keys 6 --preamble 0 --reopen 0 --max-load 999 \
      --out "$d" >/dev/null 2>&1 || echo "  exit $?"
    cat "$d"/raw/*.host-stderr 2>/dev/null | grep '^boxmemo' \
      | awk '/^boxmemo: window open/{w=1} w{print} /^boxmemo: window closed/{w=0}' \
      | sed -E 's/was \[[-0-9, ]*\] now \[[-0-9, ]*\]/V/g; s/=\[[-0-9, ]*\]/=V/g; s/ in [0-9.]+(ns|µs)//; s/, [0-9]+ bytes//; s/[0-9]+ bytes//; s/[0-9]+ meanings/N meanings/' \
      | sort | uniq -c | sort -rn > "$d.window-counts" || true
    rm -f "$d"/raw/*.host-stderr
  done
done
cd "$WORK"
python3 "$ROOT/tools/incr-bench/l6keys.py" "$WORK/r" > "$WORK/r/summary.md" || echo "l6keys.py failed" > "$WORK/r/summary.md"
cat "$WORK/r/summary.md"
for j in $(seq 0 $((i - 1))); do git -C "$ROOT" worktree remove --force "$WORK/src-$j" || true; done
