#!/bin/bash
# Open -> first pixels, cold (no snapshot) then reopen (from the snapshot), per document. Private cache root.
W=/Users/kubar/code/flashtex/.claude/worktrees/agent-a61b967844fa7b489
APP=$W/apps/mac/.build/release/FlashTeXMac
export FLASHTEX_V3_CACHE=$W/.scratch/v3cache-open
TAG=${TAG:-o1}
for n in ${DOCS:-10 120 1000}; do
  rm -rf $FLASHTEX_V3_CACHE/snapshots
  for run in cold reopen; do
    FLASHTEX_LOG=$W/.scratch/bench/open-$TAG-$n-$run.log FLASHTEX_NO_ACTIVATE=1 FLASHTEX_WINDOW_FRAME=40,40,1440,900 \
      FLASHTEX_ENGINE_V3=1 FLASHTEX_OPEN=$W/.scratch/bench/plain-$n/main.tex \
      FLASHTEX_V3_OPEN_BENCH=$W/.scratch/bench/open-$TAG-$n-$run.json FLASHTEX_V3_OPEN_BENCH_SWITCH=$W/.scratch/bench/other/main.tex $APP > /dev/null 2>&1
    echo "plain-$n $run: $(tr -d '\n' < $W/.scratch/bench/open-$TAG-$n-$run.json | cut -c1-200)"
  done
done
