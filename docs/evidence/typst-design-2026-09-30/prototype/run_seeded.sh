#!/bin/bash
# usage: run_seeded.sh OUT N validate|time doc...
HERE="$(cd "$(dirname "$0")" && pwd)"
B="${TBENCH:-$HERE/proto/target/release/tbench}"
D="${DOCS:-$HERE/docs}"
OUT=$1; N=$2; V=$3; shift 3
for d in "$@"; do
  echo "{\"load_before\":\"$(uptime | sed 's/.*load averages: //')\",\"doc\":\"$d\",\"mode\":\"seeded-$V\"}"
  "$B" seeded "$D/$d" "$N" "$V" "$d"
  echo "{\"load_after\":\"$(uptime | sed 's/.*load averages: //')\",\"doc\":\"$d\"}"
done | tee -a "$OUT"
