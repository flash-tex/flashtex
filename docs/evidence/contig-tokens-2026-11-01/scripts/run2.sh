#!/usr/bin/env bash
# run2.sh: lane CONTIG-TOKENS follow-up measurement on the NixOS PC (run inside ftx-run).
# Engines: main, mainB (main's build again: the A/A control) and lazy (#2016's merged head).
set -e
export PATH=/home/kubar/texlive/2026/bin/x86_64-linux:$PATH
C=~/.cache/flashtex-agents/contig
export INCR_BENCH_DIR=$C/ib S=$C/src/tools/incr-bench
PERF=$(ls -d /nix/store/*-perf-linux-7.3/bin/perf | head -1); export PERF
export PATH=$(dirname $PERF):$PATH
R=$C/res; mkdir -p $R
cd $C/src
echo "== start $(date -u +%FT%TZ)"
(uname -a; lscpu | grep -E 'Model name|^CPU\(s\)'; cat /proc/loadavg) > $R/machine.txt
for e in main mainB lazy; do
  mkdir -p $C/bin-$e/fmt
  cp $INCR_BENCH_DIR/$e/* $C/bin-$e/
  ln -sf $C/bin-$e/flashtex-initex $C/bin-$e/pdftex
  cp $INCR_BENCH_DIR/fmt-$e/pdflatex.fmt $C/bin-$e/fmt/
  echo "$e $(cat $INCR_BENCH_DIR/$e/HEAD)"
  sha256sum $C/bin-$e/flashtex-initex $C/bin-$e/flashtex-host $C/bin-$e/fmt/pdflatex.fmt
done > $R/engines.txt
python3 tools/incr-bench/mkbook.py infdesc-x2 > /dev/null
echo "$INCR_BENCH_DIR/docs/infdesc-x2:main.tex" > $R/docs.txt
python3 $C/fetchdocs.py $C/corpus >> $R/docs.txt
# review of #2008, item 2: what the recomputed dyn_used costs when \tracingstats>1 (as under
# \tracingall in lockstep and P-T1): the book with \tracingstats=2 from its first line.
T=$INCR_BENCH_DIR/docs/infdesc-x2-ts2; rm -rf $T; cp -r $INCR_BENCH_DIR/docs/infdesc-x2 $T
printf '\\tracingstats=2 \\input{main.tex}\n' > $T/ts2.tex
echo "$T:ts2.tex" >> $R/docs.txt
echo "== host A/B $(date -u +%T)"
rm -rf $R/host; KEYS=20 OUT=$R/host EDIT=x2/book/number-theory/modular-arithmetic.tex bash $C/hostab.sh infdesc-x2 900 300 3 main mainB lazy 2>&1 | tee $R/host.txt
echo "== cold A/B $(date -u +%T)"
python3 $C/bench2.py $R/cold --reps 5 --engine main=$C/bin-main --engine mainB=$C/bin-mainB \
  --engine lazy=$C/bin-lazy $(cat $R/docs.txt) 2> $R/cold.err
rm -rf $R/cold/work $R/host/work $R/host/s0
echo "== done $(date -u +%T)"
