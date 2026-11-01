#!/bin/bash
set -u
B=~/.cache/flashtex-agents/coldopen
n=$1; at=$2
export TEXMFCNF=$(dirname $(kpsewhich texmf.cnf)) INCR_BENCH_DIR=$B/ib
cd $B/src
~/.local/bin/ftx-run 3600 python3 tools/incr-bench/coldopen.py $B/ib/$n $B/doc infdesc-x2.tex $B/r/edit-$n-$at.jsonl --external-tools auto --host-arg=--profile --host-arg=balanced --edit-at-ms $at --edit-line 47 --edit-file book/number-theory/modular-arithmetic.tex --edit-viewport --tag edit-$n-$at > $B/r/edit-$n-$at.out 2>&1
tail -1 $B/r/edit-$n-$at.out
