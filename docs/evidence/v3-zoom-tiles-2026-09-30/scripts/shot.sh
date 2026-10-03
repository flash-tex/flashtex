#!/bin/bash
# usage: shot.sh <label> <ppp|none> [extra env...]  : launch, wait, screenshot the window by id, kill own pid
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
W=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa
L=$1; PPP=$2; shift 2
OUT=$S/shots/$L; mkdir -p $OUT; rm -f $OUT/*
mkdir -p $S/benchdoc; cp $S/dense.tex $S/benchdoc/dense.tex
PPPENV=""; [ "$PPP" != none ] && PPPENV="FLASHTEX_V3_PPP=$PPP"
env FLASHTEX_NO_ACTIVATE=1 FLASHTEX_ENGINE_V3=1 \
FLASHTEX_HOST=$W/target/release/flashtex-host FLASHTEX_POOL=$W/crates/flashtex-engine/pdftex.pool \
FLASHTEX_KEYCHAIN_OFF=1 FLASHTEX_REPO=$W \
FLASHTEX_WINDOW_FRAME=${FRAME:-40,40,1440,900} FLASHTEX_V3_LOG_DONE=1 FLASHTEX_V3_CACHE=$S/v3cache \
FLASHTEX_OPEN=$S/benchdoc/dense.tex FLASHTEX_LOG=$OUT/app.log $PPPENV "$@" \
$W/apps/mac/.build/release/FlashTeXMac > $OUT/stdout.txt 2>&1 &
PID=$!
sleep ${WAIT:-25}
[ -n "$SAMPLE" ] && sample $PID 3 -file $OUT/sample.txt >/dev/null 2>&1
$S/winid $PID > $OUT/windows.txt
WID=$(awk '$2=="FlashTeX" {print $1; exit}' $OUT/windows.txt)
[ -z "$WID" ] && WID=$(awk '$4>600 {print $1; exit}' $OUT/windows.txt)
[ -n "$WID" ] && screencapture -x -o -l $WID $OUT/window.png
kill $PID; sleep 1; kill -9 $PID 2>/dev/null
cat $OUT/windows.txt; ls $OUT
