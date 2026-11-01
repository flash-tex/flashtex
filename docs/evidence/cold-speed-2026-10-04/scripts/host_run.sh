#!/bin/bash
# usage: host_run.sh NAME HOSTBIN [extra host args...]   -> /tmp/cs/host-NAME.{jsonl,err,time}
# A clean copy of the book, a socket host as the app starts it, one COMPILE through dl3-client.
set -u
NAME=$1; HB=$2; shift 2
W=/tmp/cs/w-host-$NAME; S=/tmp/cs/h-$NAME.sock
rm -rf "$W" "$S" /tmp/cs/s0-$NAME
rsync -a --exclude '*.aux' --exclude '*.toc' --exclude '*.idx' --exclude '*.ind' --exclude '*.ilg' \
  --exclude '*.out' --exclude '*.log' --exclude 'infdesc.pdf' ~/Documents/infdesc/ "$W/"
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=/tmp/cs/ep/eng/pdftex.pool FLASHTEX_FORMATS=/tmp/cs/ep/fmt
/usr/bin/time -l "$HB" --socket "$S" --s0-cache /tmp/cs/s0-$NAME "$@" 2>/tmp/cs/host-$NAME.err &
TP=$!
for i in $(seq 600); do [ -S "$S" ] && break; sleep 0.2; done
L0=$(sysctl -n vm.loadavg)
/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a006882406b05a296/target/release/dl3-client \
  --socket "$S" --root "$W" --main infdesc.tex --quiet > /tmp/cs/host-$NAME.jsonl
echo "load before: $L0 after: $(sysctl -n vm.loadavg)" >> /tmp/cs/host-$NAME.jsonl
# stop the host: its pid is the child of the time process
HP=$(pgrep -P $TP)
[ -n "$HP" ] && kill $HP
wait $TP
