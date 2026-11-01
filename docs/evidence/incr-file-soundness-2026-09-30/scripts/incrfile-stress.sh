#!/usr/bin/env bash
# INCR-FILE-SOUNDNESS: R rounds of N parallel opens; TAG prefix; extra env args passed through
P=$1; shift; R=${R:-3}; N=${N:-8}
for r in $(seq 1 $R); do
  for i in $(seq 1 $N); do ~/incrfile-open.sh $P$r-$i "$@" & done; wait
done > /tmp/ifs-stress-$P.txt 2>&1
echo done >> /tmp/ifs-stress-$P.txt
