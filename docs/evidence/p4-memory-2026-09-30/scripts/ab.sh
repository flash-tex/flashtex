#!/usr/bin/env bash
# ab.sh TAG "ENGINE..." DOC... : interleaved latency and memory rounds (NixOS PC, lane P4-MEMORY).
# For each round (ROUNDS, default 2), each document and each engine in turn: tools/incr-bench/mem.py
# typing on pages 0, 30%, 60% and the last, KEYS keystrokes each (default 8), 300 ms apart, the
# host's defaults (keep-warm on). Summaries: $INCR_BENCH_DIR/mem/ab-TAG.jsonl.
cd ~/code/flashtex-p4mem || exit 1
export PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
export INCR_BENCH_DIR=${INCR_BENCH_DIR:-/tmp/p4mem}
TAG=$1; ENGS=$2; shift 2
for r in $(seq 1 "${ROUNDS:-2}"); do
  for DOC in "$@"; do
    n=${DOC##*-}
    pages="0,$((n * 3 / 10)),$((n * 6 / 10)),$((n - 1))"
    for E in $ENGS; do
      python3 tools/incr-bench/mem.py "$E" "$DOC" --pages "$pages" --keys "${KEYS:-8}" \
        --limit-gb "${LIMIT_GB:-12}" --tag "$TAG-$E-r$r" --timeout 3000 $MEMARGS \
        | tee -a "$INCR_BENCH_DIR/mem/ab-$TAG.jsonl"
    done
  done
done
echo RUN-DONE "$TAG"
