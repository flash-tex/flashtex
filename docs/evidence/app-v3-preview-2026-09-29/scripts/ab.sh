#!/bin/bash
# A/B: engine-v3 fast edit path on (TAG a) vs off (TAG b), interleaved per document size.
S=/Users/kubar/code/flashtex/.claude/worktrees/agent-a61b967844fa7b489/.scratch
A=${A:-fa}; B=${B:-fb}
for n in ${DOCS:-10 120 1000}; do
  TAG=$A DOCS=$n KEYS=${KEYS:-60} bash $S/runbench.sh 2>&1 | grep -E "done" | cut -c1-130
  FLASHTEX_V3_FAST_EDITS=0 TAG=$B DOCS=$n KEYS=${KEYS:-60} bash $S/runbench.sh 2>&1 | grep -E "done" | cut -c1-130
done
uptime
cd $S && python3 stages.py bench/$A-plain-*.json bench/$B-plain-*.json
