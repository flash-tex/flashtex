#!/bin/bash
# measure5.sh: book.tex, interleaved (the machine's idle clock moves between runs): base, fin2,
# fin3, fin3 --keep-warm 400 on pages 4 and 129 (a letter mid-line), 2 rounds.
S=$(cd "$(dirname "$0")/../../../../tools/incr-bench" && pwd)
export INCR_BENCH_DIR=/tmp/p4f
F=~/Documents/FlashTeX-1000-page-test/book.tex
for r in 1 2; do
  for page in 4 129; do
    for cfg in base: fin2: fin3: "fin3:--keep-warm 400"; do
      e=${cfg%%:*}; a=${cfg#*:}; t=ab$r-$e${a:+-warm}
      echo "== $t $page $(date +%T) $(uptime | sed 's/.*load averages: //')"
      HOSTARGS="$a" KEYS=6 bash $S/keys_at.sh $e $t $F $page middle
    done
  done
done
echo "== end $(date +%T)"
