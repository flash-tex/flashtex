#!/bin/bash
# prof.sh ENGINE DOC PHASE OUT: run a t7 phase without keep-warm and sample the host for 8 s
ENG=$1; DOC=$2; PH=$3; OUT=$4
cd /Users/jay3332/Projects/flashtex/.claude/worktrees/agent-ae991ac8b1c6223cd
INCR_BENCH_DIR=/tmp/ib-el python3 tools/incr-bench/t7.py --engine "$ENG" --docs "$DOC" --phases "$PH" --keys 40 --host-args "--keep-warm 0" --out "$OUT" > "$OUT.log" 2>&1 &
TP=$!
P=""
for i in $(seq 1 120); do
  P=$(pgrep -f "^/tmp/ib-el/$ENG/flashtex-host" | head -1)
  [ -n "$P" ] && break
  sleep 0.5
done
echo "host pid $P"
# let the open and the warm-up keystroke pass
sleep ${WAIT:-8}
P=$(pgrep -f "^/tmp/ib-el/$ENG/flashtex-host" | tail -1)
ps -o pid,etime,command -p "$P" | tail -1
/usr/bin/sample "$P" 8 1 -file "$OUT.sample.txt" > "$OUT.sample.log" 2>&1
echo "sampled"
wait $TP
echo "t7 done"
