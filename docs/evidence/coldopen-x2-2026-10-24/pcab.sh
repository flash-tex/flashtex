#!/bin/bash
# on the PC: build engine dirs for the given names (already built binaries per name in $B/bin-NAME) and run coldopen A/B
set -u
B=~/.cache/flashtex-agents/coldopen
export TEXMFCNF=$(dirname $(kpsewhich texmf.cnf)) INCR_BENCH_DIR=$B/ib
cd $B/src
for n in "$@"; do
  ~/.local/bin/ftx-run 3600 python3 tools/incr-bench/coldopen.py $B/ib/$n $B/doc infdesc-x2.tex $B/r/$n.jsonl --external-tools auto --host-arg=--profile --host-arg=balanced --tag $n > $B/r/$n.out 2>&1 &
done
wait
for n in "$@"; do echo "$n: $(tail -1 $B/r/$n.out)"; done
