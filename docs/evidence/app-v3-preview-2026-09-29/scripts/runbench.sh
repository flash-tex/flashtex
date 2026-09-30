#!/bin/bash
# keystroke -> pixels for plain-10/120/1000 with the release app.
W=/Users/kubar/code/flashtex/.claude/worktrees/agent-a61b967844fa7b489
APP=$W/apps/mac/.build/release/FlashTeXMac
TAG=${TAG:-run}
for n in ${DOCS:-10 120 1000}; do
  d=$W/.scratch/bench/plain-$n
  rm -rf ~/Library/Caches/FlashTeX/engine-v3/projects
  echo "== plain-$n $(date -u +%T) $(uptime)"
  FLASHTEX_LOG=$W/.scratch/bench/app-$TAG-$n.log FLASHTEX_NO_ACTIVATE=1 \
    FLASHTEX_V3_BENCH=$d/main.tex FLASHTEX_V3_BENCH_KEYS=${KEYS:-40} FLASHTEX_V3_BENCH_MS=${MS:-300} \
    FLASHTEX_V3_BENCH_OUT=$W/.scratch/bench/$TAG-plain-$n.json $APP > /dev/null 2>&1
  grep "v3bench:" $W/.scratch/bench/app-$TAG-$n.log | tail -2
done
