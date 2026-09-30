#!/usr/bin/env bash
# run.sh TAG ENGINE DOC... : tools/incr-bench/mem.py on each document in turn (NixOS PC, lane
# P4-MEMORY): typing on pages 0, 30%, 60% and the last, KEYS keystrokes each (default 16, so 64
# edits after the full compile), 300 ms apart. LIMIT_GB (default 14) kills a runaway host;
# MEMARGS adds mem.py options. Summaries go to $INCR_BENCH_DIR/mem/summary-TAG.jsonl.
cd ~/code/flashtex-p4mem || exit 1
export PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
export INCR_BENCH_DIR=${INCR_BENCH_DIR:-/tmp/p4mem}
TAG=$1; ENG=$2; shift 2
mkdir -p "$INCR_BENCH_DIR/mem"
for DOC in "$@"; do
  n=${DOC##*-}
  pages="0,$((n * 3 / 10)),$((n * 6 / 10)),$((n - 1))"
  python3 tools/incr-bench/mem.py "$ENG" "$DOC" --pages "$pages" --keys "${KEYS:-16}" \
    --limit-gb "${LIMIT_GB:-14}" --tag "$TAG" --timeout 3000 $MEMARGS \
    | tee -a "$INCR_BENCH_DIR/mem/summary-$TAG.jsonl"
done
echo RUN-DONE "$TAG"
