#!/bin/bash
# ab.sh: interleaved before (p1300) / after (fix) T7 runs, same docs and phases
cd /Users/jay3332/Projects/flashtex/.claude/worktrees/agent-ae991ac8b1c6223cd
export INCR_BENCH_DIR=/tmp/ib-el
DOCS=${DOCS:-full-100,full-1000,plain-100}
PH=${PH:-letter@start,letter@middle,letter@end}
for round in 1 2; do
  for eng in p1300 fix; do
    out=/tmp/ib-el/ab-$eng-r$round
    rm -rf $out
    { date; uptime; pmset -g | grep -i lowpower; pmset -g batt | head -2; } > $out.env 2>&1
    python3 tools/incr-bench/t7.py --engine $eng --docs $DOCS --phases $PH --out $out > $out.log 2>&1
    echo "$eng r$round exit $? $(uptime)"
  done
done
echo AB-DONE
