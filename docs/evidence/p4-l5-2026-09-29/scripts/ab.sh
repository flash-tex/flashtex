#!/bin/bash
# ab.sh DOC REGION TRIALS ENGINE...: interleaved sessions, edited-page wall/cpu per engine
doc=$1; region=$2; n=$3; shift 3
for rep in 1 2; do
  for E in "$@"; do
    printf '%s: ' "$E"
    python3 /tmp/p4l5/incr_bench.py $E /tmp/p4l5/src-$doc $doc --region $region --trials $n 2>&1 | grep -o "edited=[0-9.]*ms cpu=[0-9.]*" | tr '\n' ' '
    echo
  done
done
