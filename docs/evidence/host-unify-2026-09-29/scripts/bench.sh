#!/bin/bash
# Keystroke -> first page through the socket (dl3-keys), and reopen from S0,
# on generated 10- and 120-page documents (P4-L2-L3's gen.py, plain and full).
W=/Users/kubar/code/flashtex/.claude/worktrees/agent-a2411aceaee51c6fc
B=/tmp/hu-bench
KEYS=${KEYS:-40}
export FLASHTEX_POOL=$W/crates/flashtex-engine/pdftex.pool
export FLASHTEX_FORMATS=/tmp/hu-smoke/fmt
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
mkdir -p $B
python3 - <<'PY'
import sys, os
sys.path.insert(0, "/Users/kubar/code/flashtex/.claude/worktrees/agent-a2411aceaee51c6fc/docs/evidence/p4-l2-l3-2026-09-29/scripts")
import gen
for pages in (10, 120):
    for full in (False, True):
        d = f"/tmp/hu-bench/{'full' if full else 'plain'}{pages}"
        os.makedirs(d, exist_ok=True)
        open(os.path.join(d, "main.tex"), "w").write(gen.doc(pages, full))
PY
uptime
for doc in ${DOCS:-plain10 plain120 full10 full120}; do
  rm -rf $B/$doc/out $B/s0-$doc; mkdir -p $B/$doc/out
  rm -f $B/h.sock
  $W/target/release/flashtex-host --socket $B/h.sock --s0-cache $B/s0-$doc > $B/h.out 2> $B/h.err &
  HP=$!
  for i in $(seq 1 400); do grep -q listening $B/h.out 2>/dev/null && break; sleep 0.05; done
  echo "== $doc: $(head -1 $B/h.out | cut -c1-200)"
  $W/target/release/dl3-keys --socket $B/h.sock --root $B/$doc --main main.tex --output-dir $B/$doc/out --keys $KEYS --at 0.5 > $B/$doc.keys.jsonl
  tail -1 $B/$doc.keys.jsonl
  kill $HP; wait $HP 2>/dev/null
  # Reopen: a new host (warmed), the first compile of the document from S0.
  for r in 1 2 3; do
    rm -f $B/h.sock
    $W/target/release/flashtex-host --socket $B/h.sock --s0-cache $B/s0-$doc > $B/h.out 2> $B/h.err &
    HP=$!
    for i in $(seq 1 400); do grep -q listening $B/h.out 2>/dev/null && break; sleep 0.05; done
    $W/target/release/dl3-client --socket $B/h.sock --root $B/$doc --main main.tex --output-dir $B/$doc/out --quiet 2>/dev/null | python3 -c '
import json,sys
for l in sys.stdin:
    j=json.loads(l); h=j["host"]
    print("reopen", "'$doc'", "first_page_ms", j["first_page_ms"], "mode", h.get("mode"), "s0_error", h.get("s0_error"), "done_ms", j["done_ms"])'
    kill $HP; wait $HP 2>/dev/null
  done
done
uptime
