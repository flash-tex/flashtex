#!/bin/bash
# Sample the socket host during 5 cold compiles of plain120 (a new document each time).
W=/Users/kubar/code/flashtex/.claude/worktrees/agent-a2411aceaee51c6fc
B=/tmp/hu-bench
export FLASHTEX_POOL=$W/crates/flashtex-engine/pdftex.pool
export FLASHTEX_FORMATS=/tmp/hu-smoke/fmt
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
doc=${DOC:-plain120}
rm -f $B/p.sock
$W/target/release/flashtex-host --socket $B/p.sock --no-warm > $B/p.out 2>/dev/null &
HP=$!
for i in $(seq 1 400); do grep -q listening $B/p.out 2>/dev/null && break; sleep 0.05; done
sample $HP 6 -file $B/prof.txt > /dev/null 2>&1 &
SP=$!
for r in 1 2 3 4 5 6; do
  rm -rf $B/$doc/p$r $B/$doc/po$r; mkdir -p $B/$doc/p$r $B/$doc/po$r; cp $B/$doc/main.tex $B/$doc/p$r/
  $W/target/release/dl3-client --socket $B/p.sock --root $B/$doc/p$r --main main.tex --output-dir $B/$doc/po$r --quiet > /dev/null 2>&1
done
wait $SP
kill $HP; wait $HP 2>/dev/null
grep -c . $B/prof.txt
