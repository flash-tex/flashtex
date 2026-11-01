#!/bin/bash
# The P4-L2-L3 soundness harness (docs/evidence/p4-l2-l3-2026-09-29/scripts), pointed at this
# worktree and at /tmp/hu-sound instead of /tmp/p4l2 (which other lanes use).
set -e
W=/Users/kubar/code/flashtex/.claude/worktrees/agent-a2411aceaee51c6fc
S=$W/docs/evidence/p4-l2-l3-2026-09-29/scripts
H=/tmp/hu-sound
mkdir -p $H
for f in incr_bench.py soundness.py gen.py; do
  sed -e "s#/tmp/p4l2#$H#g" -e "s#/Users/kubar/code/flashtex/.claude/worktrees/agent-a139454d4ebf282eb#$W#g" $S/$f > $H/$f
done
ln -sfn /tmp/hu-gates/eng $H/u
ln -sfn /tmp/hu-gates/fmt $H/fmt-u
python3 $H/gen.py $H/docs
mkdir -p $H/src-plain-100 $H/src-full-100
cp $H/docs/plain-100.tex $H/src-plain-100/plain-100.tex
cp $H/docs/full-100.tex $H/src-full-100/full-100.tex
grep -n "/tmp/\|worktrees" $H/incr_bench.py $H/soundness.py | head
