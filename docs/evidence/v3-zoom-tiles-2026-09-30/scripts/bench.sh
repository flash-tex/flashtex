#!/bin/bash
# usage: bench.sh <label> <ppp> [threshold] [seconds]
# Launches the release app (never activated) on dense.tex with engine v3,
# pages at <ppp> px/pt, and runs the v3 scroll bench. Kills only its own PID.
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
W=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa
L=$1; PPP=$2; TH=${3:-3}; SEC=${4:-8}
OUT=$S/bench/$L; mkdir -p $OUT; rm -f $OUT/*.json $OUT/app.log
DOC=$S/benchdoc/dense.tex; mkdir -p $S/benchdoc; cp $S/${DOCSRC:-dense.tex} $DOC
{
  echo "label $L ppp $PPP threshold $TH"; date; uptime
  echo "runners:"; ps aux | grep -E "[R]unner.Worker|[R]unner.Listener" | awk '{print $11, $12, $13}' | sort | uniq -c
  ps aux | grep -E "actions-runner(-[0-9])?/_work" | grep -v grep | wc -l | xargs echo "runner work processes:"
} > $OUT/env.txt
FLASHTEX_NO_ACTIVATE=1 FLASHTEX_ENGINE_V3=1 \
FLASHTEX_HOST=$W/target/release/flashtex-host FLASHTEX_POOL=$W/crates/flashtex-engine/pdftex.pool \
FLASHTEX_KEYCHAIN_OFF=1 FLASHTEX_REPO=$W \
FLASHTEX_WINDOW_FRAME=${FRAME:-40,40,1440,900} FLASHTEX_V3_LOG_DONE=1 FLASHTEX_V3_CACHE=$S/v3cache FLASHTEX_OPEN=$DOC FLASHTEX_LOG=$OUT/app.log \
FLASHTEX_V3_PPP=$PPP FLASHTEX_V3_TILE_THRESHOLD=$TH FLASHTEX_V3_SCROLL_BENCH=$SEC FLASHTEX_V3_SCROLL_BENCH_DELAY=${DELAY:-6} \
FLASHTEX_V3_SCROLL_BENCH_OUT=$OUT/scroll.json \
$W/apps/mac/.build/release/FlashTeXMac > $OUT/stdout.txt 2>&1 &
PID=$!
echo "pid $PID" >> $OUT/env.txt
caffeinate -d -i -w $PID &
SAMPLED=0
for i in $(seq 1 480); do
  [ -f $OUT/scroll.json ] && break
  kill -0 $PID 2>/dev/null || break
  if [ -n "$SAMPLE" ] && [ $SAMPLED = 0 ] && grep -q "scroll bench started" $OUT/app.log 2>/dev/null; then
    SAMPLED=1; sample $PID 4 -file $OUT/sample.txt >/dev/null 2>&1 &
  fi
  sleep 0.5
done
kill $PID 2>/dev/null
sleep 1; kill -9 $PID 2>/dev/null
uptime >> $OUT/env.txt
ls $OUT
