#!/bin/bash
# usage: host_prof.sh NAME HOSTBIN -> /tmp/cs/hp-NAME.trace: a CPU Profiler trace of the socket host's cold compile.
set -u
NAME=$1; HB=$2; shift 2
W=/tmp/cs/w-hp-$NAME; S=/tmp/cs/hp-$NAME.sock
rm -rf "$W" "$S" /tmp/cs/hp-$NAME.trace /tmp/cs/s0-hp-$NAME
rsync -a --exclude '*.aux' --exclude '*.toc' --exclude '*.idx' --exclude '*.ind' --exclude '*.ilg' \
  --exclude '*.out' --exclude '*.log' --exclude 'infdesc.pdf' --exclude .flashtex ~/Documents/infdesc/ "$W/"
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=/tmp/cs/ep/eng/pdftex.pool FLASHTEX_FORMATS=/tmp/cs/ep/fmt
xctrace record --template 'CPU Profiler' --output /tmp/cs/hp-$NAME.trace --launch -- "$HB" --socket "$S" --s0-cache /tmp/cs/s0-hp-$NAME "$@" > /tmp/cs/hp-$NAME.xc 2>&1 &
XP=$!
for i in $(seq 600); do [ -S "$S" ] && break; sleep 0.2; done
/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a006882406b05a296/target/release/dl3-client \
  --socket "$S" --root "$W" --main infdesc.tex --quiet > /tmp/cs/hp-$NAME.jsonl
HP=$(pgrep -f "socket $S" | grep -v $XP | head -1)
sleep 3
[ -n "$HP" ] && kill $HP
wait $XP
xctrace export --input /tmp/cs/hp-$NAME.trace --xpath '/trace-toc/run[@number="1"]/data/table[@schema="cpu-profile"]' > /tmp/cs/hp-$NAME.xml
