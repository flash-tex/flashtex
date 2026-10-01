#!/bin/bash
# measure.sh BEFORE AFTER: the lane's macOS measurements, one after another (engines from
# mkeng.sh; the AFTER engine's dl3-keys drives both). Waits for the 1-minute load average to be
# below 8 before each part. Output: /tmp/p4f/measure-AFTER.log and the files the parts name.
#  1. app-like keystrokes through the socket: plain-10/120/1000, page 1, no viewport, 300 ms
#     apart, 30 keys; BEFORE, AFTER, AFTER --keep-warm 400, interleaved, 2 rounds (ab.sh)
#  2. the owner's book.tex: BEFORE on the four pages (middle, a letter), AFTER on the matrix
#  3. the engine matrix (incr_bench.py through iserve): plain/full x 10/100/300/1000 x
#     start/middle/end, BEFORE and AFTER
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
for doc in plain-10 plain-120 plain-1000; do
  wait_load
  echo "== app-like $doc $(date +%T) $(uptime | sed 's/.*load averages: //')"
  KEYS=30 bash $S/ab_engines.sh $doc 0 300 2 "before=$B:" "after=$A:" "afterwarm=$A:--keep-warm 400"
done
wait_load
echo "== book before $(date +%T)"
for page in 4 129 539 999; do KEYS=6 bash $S/keys_at.sh $B before-$B ~/Documents/FlashTeX-1000-page-test/book.tex $page middle; done
wait_load
echo "== book after $(date +%T)"
bash $S/keys_matrix.sh $A after-$A ~/Documents/FlashTeX-1000-page-test/book.tex
wait_load
echo "== engine matrix $(date +%T)"
python3 $S/matrix.py $A /tmp/p4f/matrix-$A
python3 $S/matrix.py $B /tmp/p4f/matrix-$B
echo "== end $(date +%T) $(uptime)"
