#!/bin/bash
# usage: relaunch.sh NAME ENGINE_DIR [nos0]: a cold COMPILE of a fresh copy of the book through one host process
# (which saves S0 after it), then a second host process with the same root, output directory and S0 cache
# (an app relaunch whose copy kept its paths): one COMPILE. "nos0" removes the saved S0 first.
# -> /tmp/cs2/rl-NAME-{1,2}.jsonl and .err
set -u
NAME=$1; E=$2; MODE=${3:-}
W=/tmp/cs2/w-rl-$NAME; O=/tmp/cs2/o-rl-$NAME; C=/tmp/cs2/s0-rl-$NAME; S=/tmp/cs2/rl-$NAME.sock
rm -rf "$W" "$O" "$C" "$S"
rsync -a --exclude '*.aux' --exclude '*.toc' --exclude '*.idx' --exclude '*.ind' --exclude '*.ilg' \
  --exclude '*.out' --exclude '*.log' --exclude 'infdesc.pdf' --exclude .flashtex ~/Documents/infdesc/ "$W/"
mkdir -p "$O"
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=$E/pdftex.pool FLASHTEX_FORMATS=/tmp/cs2/ib/fmt-main
for k in 1 2; do
  [ "$k" = 2 ] && [ "$MODE" = nos0 ] && rm -rf "$C"/*
  /usr/bin/time -l $E/flashtex-host --socket "$S" --s0-cache "$C" 2>/tmp/cs2/rl-$NAME-$k.err &
  TP=$!
  for i in $(seq 600); do [ -S "$S" ] && break; sleep 0.2; done
  L0=$(sysctl -n vm.loadavg)
  $E/dl3-client --socket "$S" --root "$W" --main infdesc.tex --output-dir "$O" --quiet > /tmp/cs2/rl-$NAME-$k.jsonl
  echo "load before: $L0 after: $(sysctl -n vm.loadavg)" >> /tmp/cs2/rl-$NAME-$k.jsonl
  # the host saves S0 after the DONE of a cold compile: wait for it
  for i in $(seq 600); do grep -q 'saved_s0\|saving S0' /tmp/cs2/rl-$NAME-$k.err && break; [ "$k" = 2 ] && break; sleep 0.5; done
  HP=$(pgrep -P $TP)
  [ -n "$HP" ] && kill $HP
  wait $TP
  rm -f "$S"
done
