#!/bin/bash
# measure4.sh BEFORE AFTER: the owner's book.tex matrix again (keys_at.sh keeping the host's stderr
# apart), then DESIGN §1.2's preamble edit (preamble.py, 5 edits) on plain-100, full-100 and
# full-1000, BEFORE and AFTER.
S=$(cd "$(dirname "$0")/../../../../tools/incr-bench" && pwd)
export INCR_BENCH_DIR=/tmp/p4f
B=$1; A=$2
echo "== book $(date +%T) $(uptime)"
bash $S/keys_matrix.sh $A book-$A ~/Documents/FlashTeX-1000-page-test/book.tex
for doc in plain-100 full-100 full-1000; do
  for e in $B $A; do
    echo "== preamble $doc $e $(date +%T) $(uptime | sed 's/.*load averages: //')"
    $S/to.sh 1800 python3 $S/preamble.py $e $doc 5
  done
done
echo "== end $(date +%T) $(uptime)"
