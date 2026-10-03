#!/bin/bash
# measure8.sh ENGINE: the prepared restore's effect (book.tex pages 5 and 130, a letter mid-line,
# 6 keystrokes): default against FLASHTEX_NO_PREPARE=1, interleaved, 2 rounds; keep-warm default.
S=$(cd "$(dirname "$0")/../../../../tools/incr-bench" && pwd)
export INCR_BENCH_DIR=/tmp/p4f
E=$1
F=~/Documents/FlashTeX-1000-page-test/book.tex
for r in 1 2; do
  for page in 4 129; do
    for np in "" 1; do
      t=p$r-$E${np:+-noprep}
      echo "== $t $page $(date +%T) $(uptime | sed 's/.*load averages: //')"
      FLASHTEX_NO_PREPARE=$np KEYS=6 bash $S/keys_at.sh $E $t $F $page middle
    done
  done
done
