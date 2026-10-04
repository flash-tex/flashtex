#!/bin/bash
# usage: host_reopen.sh NAME HOSTBIN [carry]: one cold COMPILE through the socket host with an explicit
# output directory, empty (a relaunch today) or holding a converged run's .aux/.toc/.ind/.out ("carry":
# what the app's carried output gives the next copy). -> /tmp/cs/hr-NAME.jsonl
set -u
NAME=$1; HB=$2; CARRY=${3:-}
W=/tmp/cs/w-hr-$NAME; O=/tmp/cs/o-hr-$NAME; S=/tmp/cs/hr-$NAME.sock
rm -rf "$W" "$O" "$S" /tmp/cs/s0-hr-$NAME
rsync -a --exclude '*.aux' --exclude '*.toc' --exclude '*.idx' --exclude '*.ind' --exclude '*.ilg' \
  --exclude '*.out' --exclude '*.log' --exclude 'infdesc.pdf' --exclude .flashtex ~/Documents/infdesc/ "$W/"
mkdir -p "$O"
if [ -n "$CARRY" ]; then
  for f in /tmp/cs/w-pdftex/*.aux /tmp/cs/w-pdftex/*.toc /tmp/cs/w-pdftex/*.ind /tmp/cs/w-pdftex/*.idx /tmp/cs/w-pdftex/*.ilg /tmp/cs/w-pdftex/*.out; do cp "$f" "$O/"; done
fi
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=/tmp/cs/ep/eng/pdftex.pool FLASHTEX_FORMATS=/tmp/cs/ep/fmt
/usr/bin/time -l "$HB" --socket "$S" --s0-cache /tmp/cs/s0-hr-$NAME 2>/tmp/cs/hr-$NAME.err &
TP=$!
for i in $(seq 600); do [ -S "$S" ] && break; sleep 0.2; done
L0=$(sysctl -n vm.loadavg)
/Users/jay3332/Projects/flashtex/.claude/worktrees/agent-a006882406b05a296/target/release/dl3-client \
  --socket "$S" --root "$W" --main infdesc.tex --output-dir "$O" --quiet > /tmp/cs/hr-$NAME.jsonl
echo "load before: $L0 after: $(sysctl -n vm.loadavg)" >> /tmp/cs/hr-$NAME.jsonl
HP=$(pgrep -P $TP)
[ -n "$HP" ] && kill $HP
wait $TP
