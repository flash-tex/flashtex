#!/bin/bash
# usage: dlsame_run.sh A B OUT: display-list identity of engines A and B (under /tmp/cs/ib) on every fixture with a main.tex
cd /Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a006882406b05a296
export INCR_BENCH_DIR=/tmp/cs/ib SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
python3 docs/evidence/cold-speed-2026-10-04/scripts/dlsame.py $1 $2 $(cat /tmp/cs/fixtures.txt) > $3 2>&1
echo "exit $?" >> $3
