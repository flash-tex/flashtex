#!/bin/bash
# keys.sh ENGINE: keystroke -> edited page through the socket (dl3-keys),
# waiting for each DONE (the P3P4-HOST-UNIFY measurement) and with --overlap
# (the next key as soon as the edited page is there: preemption), on the
# generated 10- and 120-page documents. ENGINE is a /tmp/p4l5/NAME made by
# mkeng.sh (its format); the binaries come from the worktree's target/.
W=/Users/kubar/code/flashtex/.claude/worktrees/agent-adbdd56f16d07becc
E=$1
B=/tmp/p4l5/keys
KEYS=${KEYS:-40}
export FLASHTEX_POOL=$W/crates/flashtex-engine/pdftex.pool
export FLASHTEX_FORMATS=/tmp/p4l5/fmt-$E
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
mkdir -p $B
python3 - <<'PY'
import sys, os
sys.path.insert(0, "/Users/kubar/code/flashtex/.claude/worktrees/agent-adbdd56f16d07becc/docs/evidence/p4-l5-2026-09-29/scripts")
import gen
for pages in (10, 120):
    for full in (False, True):
        d = f"/tmp/p4l5/keys/{'full' if full else 'plain'}{pages}"
        os.makedirs(d, exist_ok=True)
        open(os.path.join(d, "main.tex"), "w").write(gen.doc(pages, full))
PY
uptime
for doc in ${DOCS:-plain10 plain120 full10 full120}; do
  for mode in wait overlap; do
    rm -rf $B/$doc/out; mkdir -p $B/$doc/out
    cp $B/$doc/main.tex $B/$doc/main.tex.orig
    rm -f $B/h.sock
    $W/target/release/flashtex-host --socket $B/h.sock > $B/h.out 2> $B/h.err &
    HP=$!
    for i in $(seq 1 400); do grep -q listening $B/h.out 2>/dev/null && break; sleep 0.05; done
    extra=""; [ $mode = overlap ] && extra="--overlap"
    $W/target/release/dl3-keys --socket $B/h.sock --root $B/$doc --main main.tex --output-dir $B/$doc/out --keys $KEYS --at 0.5 $extra > $B/$doc.$mode.jsonl
    echo "== $doc $mode: $(grep summary $B/$doc.$mode.jsonl | cut -c1-400) $(grep cancelled $B/$doc.$mode.jsonl)"
    kill $HP; wait $HP 2>/dev/null
    cp $B/$doc/main.tex.orig $B/$doc/main.tex
  done
done
uptime
