#!/usr/bin/env bash
# APP-PERF-AUDIT: typing matrix with Time Profiler attached.
# usage: matrix.sh <label> [docs...]   (docs default: plain-10 plain-120 plain-1000)
# Cells land in /tmp/appperf/runs/<label>-<doc>-<at>; each gets summary.txt.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LABEL="$1"; shift
DOCS=("$@"); [[ ${#DOCS[@]} -gt 0 ]] || DOCS=(plain-10 plain-120 plain-1000)
AT="${AT:-mid-paragraph}"
for d in "${DOCS[@]}"; do
  C="/tmp/appperf/runs/$LABEL-$d-$AT"
  rm -rf "$C"
  "$HERE/bench.sh" "/tmp/appperf/docs/$d.tex" "$C" --at "$AT" --trace "Time Profiler" --env FLASHTEX_TYPING_BENCH_START_UNPAINTED=1 > /dev/null 2>&1
  "$HERE/analyze.sh" "$C" > /dev/null 2>&1
  echo "== $C"; sed -n '1,4p' "$C/summary.txt"
done
