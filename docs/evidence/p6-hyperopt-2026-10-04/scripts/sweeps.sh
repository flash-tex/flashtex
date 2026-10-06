#!/bin/bash
# soundness sweeps A, C, D (subsets) for engine $1 under INCR_BENCH_DIR, results in $INCR_BENCH_DIR/gates-$1
E=$1
W=$(cd "$(dirname "$0")/../../../.." && pwd)
S=$W/tools/incr-bench
B=${INCR_BENCH_DIR:-/tmp/p6h}
R=$B/gates-$E
mkdir -p $R
export PYTHONHASHSEED=0
J=${J:-3}
nice python3 $S/soundness.py $E -j $J --trials 12 --dir $B/sound-a-$E --out $R/soundness-a.jsonl \
  --extra $B/src-plain-120:plain-120 --extra $B/src-full-100:full-100 > $R/soundness-a.txt 2>&1
echo "soundness A exit $?" >> $R/soundness-a.txt
nice python3 $S/soundness.py $E -j $J --trials 6 --dir $B/sound-c-$E --out $R/soundness-c.jsonl \
  --kinds sentence,section,label,ref,cite,footnote,unlabel,unsection \
  --extra $B/src-refs-30:refs-30 --extra $B/src-refs-120:refs-120 --extra $B/src-full-100:full-100 > $R/soundness-c.txt 2>&1
echo "soundness C exit $?" >> $R/soundness-c.txt
nice python3 $S/soundness.py $E -j $J --trials 6 --interleave --dir $B/sound-d-$E --out $R/soundness-d.jsonl \
  --kinds replace,insert,sentence,section,label,ref,unlabel \
  --extra $B/src-refs-30:refs-30 --extra $B/src-refs-120:refs-120 --extra $B/src-full-100:full-100 > $R/soundness-d.txt 2>&1
echo "soundness D exit $?" >> $R/soundness-d.txt
