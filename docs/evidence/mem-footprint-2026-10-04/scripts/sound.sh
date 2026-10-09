#!/bin/bash
# sound.sh ENGINE TAG: soundness A, C, D on the long documents (the retention rule needs > 16 pages).
E=$1; T=$2
S=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-ab46fa803477079b1/tools/incr-bench
B=/tmp/mfp/ib
R=/tmp/mfp/sound-$T
mkdir -p $R
export INCR_BENCH_DIR=$B PYTHONHASHSEED=0
python3 $S/soundness.py $E -j 4 --trials 30 --no-fixtures --dir $B/sa-$T --out $R/soundness-a.jsonl \
  --extra $B/src-plain-120:plain-120 --extra $B/src-full-100:full-100 > $R/soundness-a.txt 2>&1
echo "A exit $?" >> $R/exits.txt
python3 $S/soundness.py $E -j 4 --trials 12 --no-fixtures --dir $B/sc-$T --out $R/soundness-c.jsonl \
  --kinds sentence,section,label,ref,cite,footnote,unlabel,unsection \
  --extra $B/src-refs-120:refs-120 --extra $B/src-full-100:full-100 > $R/soundness-c.txt 2>&1
echo "C exit $?" >> $R/exits.txt
python3 $S/soundness.py $E -j 4 --trials 8 --interleave --no-fixtures --dir $B/sd-$T --out $R/soundness-d.jsonl \
  --kinds replace,insert,sentence,section,label,ref,unlabel \
  --extra $B/src-refs-120:refs-120 --extra $B/src-full-100:full-100 > $R/soundness-d.txt 2>&1
echo "D exit $?" >> $R/exits.txt
echo ALLDONE >> $R/exits.txt
