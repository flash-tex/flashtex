#!/bin/bash
# usage: run_mem.sh OUT doc N EVICT seeded|std
HERE="$(cd "$(dirname "$0")" && pwd)"
B="${TBENCH:-$HERE/proto/target/release/tbench}"
D="${DOCS:-$HERE/docs}"
OUT=$1; d=$2; N=$3; EV=$4; M=$5
{
  echo "{\"load_before\":\"$(uptime | sed 's/.*load averages: //')\",\"doc\":\"$d\",\"mode\":\"mem-$M-evict$EV\"}"
  "$B" mem "$D/$d" "$N" "$EV" "$M"
  echo "{\"load_after\":\"$(uptime | sed 's/.*load averages: //')\",\"doc\":\"$d\"}"
} | tee -a "$OUT"
