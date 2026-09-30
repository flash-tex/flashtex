#!/bin/bash
# measure7.sh BEFORE AFTER: the follow-up's measurements (keep-warm by default, prepared restores,
# the glyph-usage union): measure6.sh's keep-warm cost/latency for AFTER; book.tex interleaved
# (BEFORE, AFTER --keep-warm 0, AFTER) on pages 5 and 130; the engine matrix for AFTER.
S=$(cd "$(dirname "$0")/../../../../tools/incr-bench" && pwd)
H=$(cd "$(dirname "$0")" && pwd)
export INCR_BENCH_DIR=/tmp/p4f
B=$1; A=$2
cp /tmp/p4f/$A/dl3-keys /tmp/p4f/$B/dl3-keys
echo "== warm $(date +%T) $(uptime)"
bash $H/measure6.sh $A
F=~/Documents/FlashTeX-1000-page-test/book.tex
for r in 1 2; do
  for page in 4 129; do
    for cfg in "$B:" "$A:--keep-warm 0" "$A:"; do
      e=${cfg%%:*}; a=${cfg#*:}; t=f$r-$e${a:+-cold}
      echo "== book $t $page $(date +%T) $(uptime | sed 's/.*load averages: //')"
      HOSTARGS="$a" KEYS=6 bash $S/keys_at.sh $e $t $F $page middle
    done
  done
done
echo "== matrix $(date +%T) $(uptime)"
python3 $S/matrix.py $A /tmp/p4f/matrix-$A
echo "== end $(date +%T) $(uptime)"
