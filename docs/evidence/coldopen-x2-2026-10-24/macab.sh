#!/bin/bash
# take the Mac lock, run cold opens of infdesc-x2 for each engine name given (alternating, 1 rep each round), release
until mkdir /tmp/fth-mac.lock 2>/dev/null; do sleep 10; done
while pgrep -f dl3-keys >/dev/null; do sleep 5; done
echo "lock taken $(date +%T)"
cd /Users/kubar/code/flashtex/.claude/worktrees/agent-a52691e1ef8eb6d65
D=/private/tmp/claude-501/-Users-kubar-code-flashtex/440fe00e-c203-4f15-b706-f22dec3a333e/scratchpad/clive/docs/infdesc-x2
tag=$1; shift
for r in 1 2; do for n in "$@"; do
  INCR_BENCH_DIR=/tmp/fth-co/ib nice -n 5 python3 tools/incr-bench/coldopen.py /tmp/fth-co/ib/$n $D infdesc-x2.tex /tmp/fth-co/r/$tag-$n.jsonl --external-tools auto --host-arg=--profile --host-arg=balanced --tag $tag-$n 2>&1 | tail -1 | sed "s/^/$n: /"
done; done
rmdir /tmp/fth-mac.lock; echo "lock released $(date +%T)"
