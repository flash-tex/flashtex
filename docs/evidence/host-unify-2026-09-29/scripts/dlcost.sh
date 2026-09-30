#!/bin/bash
# What the display list costs a cold compile, in the resident engine: iserve (no display list)
# against the socket host (display list on), 3 cold compiles each, same documents, preview mode.
W=/Users/kubar/code/flashtex/.claude/worktrees/agent-a2411aceaee51c6fc
B=/tmp/hu-bench
export FLASHTEX_POOL=$W/crates/flashtex-engine/pdftex.pool
export FLASHTEX_FORMATS=/tmp/hu-smoke/fmt
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
uptime
for doc in ${DOCS:-plain120 full120}; do
  for r in 1 2 3; do
    rm -rf $B/$doc/cc; mkdir -p $B/$doc/cc; cp $B/$doc/main.tex $B/$doc/cc/
    cd $B/$doc/cc
    printf 'compile\nquit\n' | $W/target/release/flashtex-host iserve -- -fmt=pdflatex -interaction=nonstopmode -file-line-error main.tex 2>/dev/null \
      | python3 -c 'import json,sys; j=json.loads(sys.stdin.readline()); print("iserve '$doc' cold total_ms %.1f pages %d mode %s" % (j["total_s"]*1e3, j["pages"], j["mode"]))'
  done
  for r in 1 2 3; do
    rm -rf $B/$doc/cc $B/$doc/co; mkdir -p $B/$doc/cc $B/$doc/co; cp $B/$doc/main.tex $B/$doc/cc/
    rm -f $B/c.sock
    $W/target/release/flashtex-host --socket $B/c.sock --no-warm > $B/c.out 2>/dev/null &
    HP=$!
    for i in $(seq 1 400); do grep -q listening $B/c.out 2>/dev/null && break; sleep 0.05; done
    $W/target/release/dl3-client --socket $B/c.sock --root $B/$doc/cc --main main.tex --output-dir $B/$doc/co --quiet 2>/dev/null \
      | python3 -c 'import json,sys; j=json.loads(sys.stdin.readline()); h=j["host"]; print("socket '$doc' cold run_ms %.1f pages %d mode %s done_ms %.1f bytes %d" % (h["run_ms"], h["pages"], h["mode"], j["done_ms"], j["bytes"]))'
    kill $HP; wait $HP 2>/dev/null
  done
done
uptime
