#!/bin/bash
# usage: DOCFROM=<file.tex> a38-bench.sh <label> <ppp> [threshold] [seconds]
# Launches the release app (never activated, keychain off) on a copy of
# DOCFROM's directory with engine v3, pages at <ppp> px/pt, and runs the v3
# scroll bench (JSON: footprint, bitmap bytes, cut-raster residency, frames).
# Kills only its own PID.
S=/private/tmp/claude-501/-Users-jay3332-Projects-flashtex/211342f1-98df-4cd5-b564-1eec24da3de0/scratchpad
W=/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a38de97ab0a6ef6aa
L=$1; PPP=$2; TH=${3:-3}; SEC=${4:-8}
OUT=$S/a38-bench/$L; mkdir -p "$OUT"; rm -f "$OUT"/*
SRC=${DOCFROM:-$S/dense.tex}
DOCDIR=$S/a38-benchdoc/$L; rm -rf "$DOCDIR"; mkdir -p "$DOCDIR"
cp "$(dirname "$SRC")"/*.tex "$DOCDIR"/ 2>/dev/null; cp "$SRC" "$DOCDIR"/
DOC=$DOCDIR/$(basename "$SRC")
{
  echo "label $L ppp $PPP threshold $TH doc $SRC"; date; uptime
  echo "runners:"; ps aux | grep -E "[R]unner.Worker" | awk '{print $11}' | sort | uniq -c
  ps aux | grep -E "actions-runner(-[0-9])?/_work" | grep -v grep | wc -l | xargs echo "runner work processes:"
} > "$OUT/env.txt"
FLASHTEX_NO_ACTIVATE=1 FLASHTEX_ENGINE_V3=1 FLASHTEX_KEYCHAIN_OFF=1 \
FLASHTEX_HOST=$W/target/release/flashtex-host FLASHTEX_POOL=$W/crates/flashtex-engine/pdftex.pool FLASHTEX_REPO=$W \
FLASHTEX_WINDOW_FRAME=${FRAME:-40,40,1440,900} FLASHTEX_V3_CACHE=$S/a38-v3cache FLASHTEX_OPEN=$DOC FLASHTEX_LOG=$OUT/app.log \
FLASHTEX_V3_PPP=$PPP FLASHTEX_V3_TILE_THRESHOLD=$TH FLASHTEX_V3_SCROLL_BENCH=$SEC FLASHTEX_V3_SCROLL_BENCH_DELAY=${DELAY:-8} \
FLASHTEX_V3_SCROLL_BENCH_OUT=$OUT/scroll.json \
$W/apps/mac/.build/release/FlashTeXMac \
  -"NSSplitView Subview Frames FlashTeX.workspace.split.editorPreview" '("0.000000, 0.000000, 380.000000, 850.000000, NO, NO", "381.000000, 0.000000, 710.000000, 850.000000, NO, NO")' \
  -"NSSplitView Subview Frames FlashTeX.workspace.split.problems" '("0.000000, 0.000000, 1091.000000, 850.000000, NO, NO", "0.000000, 730.000000, 1091.000000, 0.000000, YES, NO")' \
  > "$OUT/stdout.txt" 2>&1 &
PID=$!
echo "pid $PID" >> "$OUT/env.txt"
caffeinate -d -i -w $PID &
for i in $(seq 1 480); do
  [ -f "$OUT/scroll.json" ] && break
  kill -0 $PID 2>/dev/null || break
  sleep 0.5
done
kill $PID 2>/dev/null; sleep 1; kill -9 $PID 2>/dev/null
uptime >> "$OUT/env.txt"
