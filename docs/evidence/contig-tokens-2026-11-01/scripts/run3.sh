#!/usr/bin/env bash
# run3.sh: the rest of run2.sh after two host sessions hit the 900 s limit at load 22: host
# rounds 2 (main, lazy) and 3 with a 2400 s limit, then the cold A/B (engines and documents as run2.sh set them).
set -e
export PATH=/home/kubar/texlive/2026/bin/x86_64-linux:$PATH
C=~/.cache/flashtex-agents/contig
export INCR_BENCH_DIR=$C/ib S=$C/src/tools/incr-bench
PERF=$(ls -d /nix/store/*-perf-linux-7.3/bin/perf | head -1); export PERF
export PATH=$(dirname $PERF):$PATH
R=$C/res
cd $C/src
echo "== host rounds 2-3 $(date -u +%T)"
export TO=2400 KEYS=20 OUT=$R/host EDIT=x2/book/number-theory/modular-arithmetic.tex
START=2 bash $C/hostab2.sh infdesc-x2 900 300 2 main lazy 2>&1 | tee -a $R/host.txt
START=3 bash $C/hostab2.sh infdesc-x2 900 300 3 main mainB lazy 2>&1 | tee -a $R/host.txt
echo "== cold A/B $(date -u +%T)"
python3 $C/bench2.py $R/cold --reps 5 --engine main=$C/bin-main --engine mainB=$C/bin-mainB \
  --engine lazy=$C/bin-lazy $(cat $R/docs.txt) 2> $R/cold.err
rm -rf $R/cold/work $R/host/work $R/host/s0
echo "== done $(date -u +%T)"
