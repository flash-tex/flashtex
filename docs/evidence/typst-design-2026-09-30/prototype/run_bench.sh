#!/bin/bash
# usage: run_bench.sh OUT N EVICT doc...
HERE="$(cd "$(dirname "$0")" && pwd)"
B="${TBENCH:-$HERE/proto/target/release/tbench}"
D="${DOCS:-$HERE/docs}"
OUT=$1; N=$2; EV=$3; shift 3
for d in "$@"; do
  echo "{\"load_before\":\"$(uptime | sed 's/.*load averages: //')\",\"doc\":\"$d\"}"
  "$B" bench "$D/$d" "$N" "$EV" "$d"
  echo "{\"load_after\":\"$(uptime | sed 's/.*load averages: //')\",\"doc\":\"$d\"}"
done | tee -a "$OUT"
