#!/usr/bin/env bash
# run5.sh: the cold A/B (random order, A/A control), cut to fit a contended PC: the book and
# the 13 papers with 5 reps, the book with \tracingstats=2 with 3 (its instructions are the point).
set -e
export PATH=/home/kubar/texlive/2026/bin/x86_64-linux:$PATH
C=~/.cache/flashtex-agents/contig
PERF=$(ls -d /nix/store/*-perf-linux-7.3/bin/perf | head -1)
export PATH=$(dirname $PERF):$PATH
R=$C/res
cd $C/src
echo "== cold A/B $(date -u +%T)"
E="--engine main=$C/bin-main --engine mainB=$C/bin-mainB --engine lazy=$C/bin-lazy"
python3 $C/bench2.py $R/cold --reps 5 $E $(grep -v ts2 $R/docs.txt) 2> $R/cold.err
echo "== cold ts2 $(date -u +%T)"
python3 $C/bench2.py $R/cold --reps 3 $E $(grep ts2 $R/docs.txt) 2>> $R/cold.err
rm -rf $R/cold/work $R/host/work $R/host/s0
echo "== done5 $(date -u +%T)"
