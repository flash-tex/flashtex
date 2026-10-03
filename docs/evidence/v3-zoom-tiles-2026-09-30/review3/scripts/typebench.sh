#!/bin/bash
# usage: DOCFROM=<main.tex> AT="<text>" a38-typebench.sh <label> <ppp> <ms>
# Keystroke -> tiles on screen (EngineV3Bench) with the pane at <ppp> px/pt, 40 keys every <ms> ms;
# records disk bytes written, peak footprint, kept rasters. Release app, never activated; own PID only.
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
W=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa
L=$1; PPP=$2; MS=${3:-300}
OUT=$S/a38-typebench/$L; mkdir -p "$OUT"; rm -f "$OUT"/*
DOCDIR=$S/a38-typedoc/$L; rm -rf "$DOCDIR"; mkdir -p "$DOCDIR"; cp "$DOCFROM" "$DOCDIR/main.tex"
{ echo "label $L ppp $PPP ms $MS doc $DOCFROM at '$AT'"; date; uptime; ps aux | grep -E "actions-runner(-[0-9])?/_work" | grep -v grep | wc -l | xargs echo "runner work processes:"; } > "$OUT/env.txt"
FLASHTEX_NO_ACTIVATE=1 FLASHTEX_KEYCHAIN_OFF=1 FLASHTEX_WINDOW_FRAME=40,40,1440,900 \
FLASHTEX_HOST=$W/target/release/flashtex-host FLASHTEX_POOL=$W/crates/flashtex-engine/pdftex.pool FLASHTEX_REPO=$W \
FLASHTEX_V3_CACHE=$S/a38-v3cache FLASHTEX_LOG=$OUT/app.log FLASHTEX_V3_LOG_DONE=1 FLASHTEX_V3_PPP=$PPP \
FLASHTEX_V3_BENCH=$DOCDIR/main.tex FLASHTEX_V3_BENCH_KEYS=${KEYS:-40} FLASHTEX_V3_BENCH_MS=$MS FLASHTEX_V3_BENCH_AT="$AT" \
FLASHTEX_V3_BENCH_OUT=$OUT/typing.json \
$W/apps/mac/.build/release/FlashTeXMac \
  -"NSSplitView Subview Frames FlashTeX.workspace.split.editorPreview" '("0.000000, 0.000000, 380.000000, 850.000000, NO, NO", "381.000000, 0.000000, 710.000000, 850.000000, NO, NO")' \
  -"NSSplitView Subview Frames FlashTeX.workspace.split.problems" '("0.000000, 0.000000, 1091.000000, 850.000000, NO, NO", "0.000000, 730.000000, 1091.000000, 0.000000, YES, NO")' \
  > "$OUT/stdout.txt" 2>&1 &
PID=$!
caffeinate -d -i -w $PID &
for i in $(seq 1 600); do [ -f "$OUT/typing.json" ] && break; kill -0 $PID 2>/dev/null || break; sleep 0.5; done
sleep 1; kill $PID 2>/dev/null; sleep 1; kill -9 $PID 2>/dev/null
uptime >> "$OUT/env.txt"
