#!/bin/bash
# sweeps.sh: gates.sh's sound-a, sound-c, sound-d and sound-lookup (verify flags on), without
# coreutils' timeout (not on this Mac); J=2. Engine "gates" under INCR_BENCH_DIR.
W=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a2fa2131a670119f7
S=$W/tools/incr-bench
B=/private/tmp/mb/gb
R=$B/raw
J=2
export INCR_BENCH_DIR=$B
export FLASHTEX_VERIFY_JUMP=1 FLASHTEX_VERIFY_OLDCACHE=1 FLASHTEX_VERIFY_PREPARED=1 FLASHTEX_VERIFY_RELOC=1
mkdir -p $R
cd $W
PYTHONHASHSEED=0 python3 $S/soundness.py gates -j $J --trials 50 --dir $B/sound-a --out $R/soundness-a.jsonl \
  --extra $B/src-plain-120:plain-120 --extra $B/src-full-100:full-100 > $R/soundness-a.txt 2>&1
echo "soundness A exit $?" >> $R/soundness-a.txt
PYTHONHASHSEED=0 python3 $S/soundness.py gates -j $J --trials 20 --kinds sentence,section,label,ref,cite,footnote,unlabel,unsection \
  --dir $B/sound-c --out $R/soundness-c.jsonl \
  --extra $B/src-refs-30:refs-30 --extra $B/src-refs-120:refs-120 --extra $B/src-full-100:full-100 > $R/soundness-c.txt 2>&1
echo "soundness C exit $?" >> $R/soundness-c.txt
FLASHTEX_SWEEP_STOP_PREPARE=2 PYTHONHASHSEED=0 python3 $S/soundness.py gates -j $J --trials 12 --interleave \
  --kinds replace,insert,sentence,section,label,ref,unlabel --dir $B/sound-d --out $R/soundness-d.jsonl \
  --extra $B/src-refs-30:refs-30 --extra $B/src-refs-120:refs-120 --extra $B/src-full-100:full-100 > $R/soundness-d.txt 2>&1
echo "soundness D exit $?" >> $R/soundness-d.txt
python3 $S/genlookup.py $B > /dev/null
PYTHONHASHSEED=0 python3 $S/soundness.py gates -j $J --trials 30 --no-fixtures \
  --kinds replace,insert,delete,sentence --dir $B/sound-lookup --out $R/soundness-lookup.jsonl \
  --toggle-files "$(python3 $S/genlookup.py --names)" --extra $B/src-lookup:lookup > $R/soundness-lookup.txt 2>&1
echo "soundness lookup exit $?" >> $R/soundness-lookup.txt
PYTHONHASHSEED=0 python3 $S/soundness.py gates -j $J --trials 12 --no-fixtures --interleave \
  --kinds replace,insert,delete,sentence --dir $B/sound-lookup-d --out $R/soundness-lookup-d.jsonl \
  --toggle-files "$(python3 $S/genlookup.py --names)" --extra $B/src-lookup:lookup > $R/soundness-lookup-d.txt 2>&1
echo "soundness lookup-d exit $?" >> $R/soundness-lookup-d.txt
echo SWEEPS-DONE
