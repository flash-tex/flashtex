#!/usr/bin/env bash
# APP-PERF-AUDIT: interleaved A/B of one binary with an env toggle.
# usage: ab.sh <label> <doc> <rounds> <envA> <envB> [focus...]
#   envA/envB: VAR=value (use NONE=1 for "no toggle")
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LABEL="$1"; DOC="$2"; ROUNDS="$3"; EA="$4"; EB="$5"; shift 5
AT="${AT:-mid-paragraph}"
for r in $(seq 1 "$ROUNDS"); do
  for side in A B; do
    if [[ $side == A ]]; then E="$EA"; else E="$EB"; fi
    C="/tmp/appperf/runs/$LABEL-$DOC-$side$r"
    rm -rf "$C"
    "$HERE/bench.sh" "/tmp/appperf/docs/$DOC.tex" "$C" --at "$AT" --trace "Time Profiler" --env "$E" --env FLASHTEX_TYPING_BENCH_START_UNPAINTED=1 > /dev/null 2>&1
    "$HERE/analyze.sh" "$C" > /dev/null 2>&1
    echo "$side ($E) $(sed -n 2p "$C/load.txt") :: $(sed -n 4p "$C/summary.txt" | cut -c1-120)"
    python3 "$HERE/mainthread.py" "$C" "$@"
  done
done
