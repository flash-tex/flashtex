#!/bin/bash
set -u
B=~/.cache/flashtex-agents/coldopen
n=$1
export TEXMFCNF=$(dirname $(kpsewhich texmf.cnf)) INCR_BENCH_DIR=$B/ib
cd $B/src
~/.local/bin/ftx-run 3600 nix shell nixpkgs#perf -c perf record -m 8 -e instructions:u -F 500 -o $B/r/perf-$n.data -- python3 tools/incr-bench/coldopen.py $B/ib/$n $B/doc infdesc-x2.tex $B/r/perf-$n.jsonl --external-tools auto --host-arg=--profile --host-arg=balanced --tag perf-$n > $B/r/perf-$n.out 2>&1
nix shell nixpkgs#perf -c perf report -i $B/r/perf-$n.data --no-children --sort symbol --stdio -F overhead,sym 2>/dev/null | grep -v '^#' | grep -v '^$' | head -70 > $B/r/perf-$n-top.txt
tail -1 $B/r/perf-$n.out
