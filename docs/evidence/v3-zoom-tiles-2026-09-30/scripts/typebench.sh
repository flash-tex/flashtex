#!/bin/bash
# usage: typebench.sh <label> <ppp> <threshold>: keystroke -> pixels (EngineV3Bench) with the pane at <ppp>
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
W=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa
L=$1; PPP=$2; TH=$3
OUT=$S/typebench/$L; mkdir -p $OUT; rm -f $OUT/*
mkdir -p $S/typedoc; cp $S/dense.tex $S/typedoc/main.tex
{ date; uptime; } > $OUT/env.txt
FLASHTEX_NO_ACTIVATE=1 FLASHTEX_KEYCHAIN_OFF=1 FLASHTEX_WINDOW_FRAME=40,40,1440,900 \
FLASHTEX_HOST=$W/target/release/flashtex-host FLASHTEX_POOL=$W/crates/flashtex-engine/pdftex.pool FLASHTEX_REPO=$W \
FLASHTEX_V3_CACHE=$S/v3cache FLASHTEX_LOG=$OUT/app.log FLASHTEX_V3_PPP=$PPP FLASHTEX_V3_TILE_THRESHOLD=$TH \
FLASHTEX_V3_BENCH=$S/typedoc/main.tex FLASHTEX_V3_BENCH_KEYS=40 FLASHTEX_V3_BENCH_MS=300 FLASHTEX_V3_BENCH_OUT=$OUT/typing.json \
$W/apps/mac/.build/release/FlashTeXMac > $OUT/stdout.txt 2>&1 &
PID=$!
caffeinate -d -i -w $PID &
for i in $(seq 1 240); do [ -f $OUT/typing.json ] && break; kill -0 $PID 2>/dev/null || break; sleep 0.5; done
sleep 1; kill $PID 2>/dev/null; sleep 1; kill -9 $PID 2>/dev/null
uptime >> $OUT/env.txt
grep "v3bench:" $OUT/app.log | tail -2 | cut -c1-400
