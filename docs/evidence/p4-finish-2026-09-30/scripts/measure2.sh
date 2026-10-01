#!/bin/bash
# measure2.sh BEFORE AFTER: measure.sh's parts for the final engine AFTER (BEFORE's book and
# app-like runs are measure.sh's): the owner's book.tex matrix, app-like keystrokes (BEFORE and
# AFTER interleaved again), then the engine matrix for AFTER and BEFORE.
S=$(cd "$(dirname "$0")/../../../../tools/incr-bench" && pwd)
export INCR_BENCH_DIR=/tmp/p4f
B=$1; A=$2
cp /tmp/p4f/$A/dl3-keys /tmp/p4f/$B/dl3-keys
wait_load() {
  for i in $(seq 1 120); do
    l=$(sysctl -n vm.loadavg | awk '{print int($2)}')
    [ "$l" -lt 8 ] && return
    sleep 10
  done
}
echo "== start $(date +%T) $(uptime)"
wait_load
echo "== book after $(date +%T)"
bash $S/keys_matrix.sh $A after-$A ~/Documents/FlashTeX-1000-page-test/book.tex
for doc in plain-10 plain-120 plain-1000; do
  wait_load
  echo "== app-like $doc $(date +%T) $(uptime | sed 's/.*load averages: //')"
  KEYS=30 bash $S/ab_engines.sh $doc 0 300 2 "before2=$B:" "final=$A:" "finalwarm=$A:--keep-warm 400"
done
wait_load
echo "== engine matrix $(date +%T)"
python3 $S/matrix.py $A /tmp/p4f/matrix-$A
wait_load
python3 $S/matrix.py $B /tmp/p4f/matrix-$B
echo "== end $(date +%T) $(uptime)"
