#!/bin/bash
# Open -> pixels for named bench folders (cold, then reopen), with walk timing in the log.
W=/Users/kubar/code/flashtex/.claude/worktrees/agent-a46f56f16e2460dff
APP=$W/apps/mac/.build/release/FlashTeXMac
export FLASHTEX_V3_CACHE=$W/.scratch/v3cache-arch
TAG=${TAG:-a1}
for d in ${DOCS:-plain-120 arch-1000}; do
  rm -rf $FLASHTEX_V3_CACHE/snapshots
  for run in cold reopen; do
    FLASHTEX_V3_LOG_DONE=1 FLASHTEX_LOG=$W/.scratch/bench/open-$TAG-$d-$run.log FLASHTEX_NO_ACTIVATE=1 FLASHTEX_WINDOW_FRAME=40,40,1440,900 \
      FLASHTEX_ENGINE_V3=1 FLASHTEX_OPEN=$W/.scratch/bench/$d/main.tex \
      FLASHTEX_V3_OPEN_BENCH=$W/.scratch/bench/open-$TAG-$d-$run.json FLASHTEX_V3_OPEN_BENCH_SWITCH=$W/.scratch/bench/other/main.tex $APP > /dev/null 2>&1
    echo "$d $run done"
  done
done
