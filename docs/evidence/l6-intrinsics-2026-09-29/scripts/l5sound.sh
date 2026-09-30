#!/bin/bash
# l5sound.sh ENGINE PASS: P4-L5's soundness harness (docs/evidence/p4-l5-2026-09-29/scripts/
# soundness.py -> incr_bench.py --verify: every incremental compile of the resident host compared
# byte for byte with from-scratch runs) on the full-* and refs-* documents, with the guarded
# intrinsics on (the default). ENGINE is /tmp/p4l5/ENGINE. PASS: A (single characters), C
# (structural), D (interleaved), as in P4-L5's README. Output: /tmp/l6/snd-PASS.{txt,jsonl}.
set -e
W=$(cd "$(dirname "$0")/../../../.." && pwd)
E=$1; P=$2
cp "$W/docs/evidence/p4-l5-2026-09-29/scripts/incr_bench.py" /tmp/p4l5/incr_bench.py
X=(--extra /tmp/p4l5/src-full-10:full-10 --extra /tmp/p4l5/src-full-100:full-100
   --extra /tmp/p4l5/src-full-300:full-300 --extra /tmp/p4l5/src-refs-30:refs-30
   --extra /tmp/p4l5/src-refs-120:refs-120)
case $P in
  A) K=(--trials 20) ;;
  C) K=(--trials 12 --kinds sentence,section,label,ref,cite,footnote,unlabel,unsection) ;;
  D) K=(--trials 8 --interleave --kinds replace,insert,sentence,section,label,ref,unlabel) ;;
esac
# HOOKDOC=1: instead, the document whose registered hook reads a label's .aux entry at
# every paragraph (l6hook.tex, FLASHTEX_INTRINSIC_NAMES=lsixhook): the replay must report
# what it reads to L5's read-set.
if [ -n "$HOOKDOC" ]; then
  X=(--extra /tmp/p4l5/src-l6hook:l6hook); export FLASHTEX_INTRINSIC_NAMES=lsixhook; P2=$P-hook
else
  P2=$P
fi
[ -n "$FIXTURES" ] || X+=(--no-fixtures)
unset FLASHTEX_INTRINSICS
PYTHONHASHSEED=0 python3 "$W/docs/evidence/p4-l5-2026-09-29/scripts/soundness.py" "$E" -j 5 "${K[@]}" "${X[@]}" \
  --dir /tmp/l6/snd-$P2 --out /tmp/l6/snd-$P2.jsonl > /tmp/l6/snd-$P2.txt 2>&1 || true
tail -2 /tmp/l6/snd-$P2.txt
