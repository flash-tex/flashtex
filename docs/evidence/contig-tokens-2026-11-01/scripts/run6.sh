#!/usr/bin/env bash
# run6.sh: the cold A/B (random order, A/A control) on a PC whose agent slice gives this run
# about 6 % of a core: the 13 papers first (5 reps), then the book (5 reps), then the book with
# \tracingstats=2 (3 reps; its instructions are the point). Each part appends to runs.jsonl.
set -e
export PATH=/home/kubar/texlive/2026/bin/x86_64-linux:$PATH
C=~/.cache/flashtex-agents/contig
PERF=$(ls -d /nix/store/*-perf-linux-7.3/bin/perf | head -1)
export PATH=$(dirname $PERF):$PATH
R=$C/res
cd $C/src
E="--engine main=$C/bin-main --engine mainB=$C/bin-mainB --engine lazy=$C/bin-lazy"
echo "== cold papers $(date -u +%T)"
python3 $C/bench2.py $R/cold --reps 5 $E $(grep -v infdesc $R/docs.txt) 2> $R/cold.err
echo "== cold book $(date -u +%T)"
python3 $C/bench2.py $R/cold --reps 5 $E $(grep 'infdesc-x2:' $R/docs.txt) 2>> $R/cold.err
echo "== cold ts2 $(date -u +%T)"
python3 $C/bench2.py $R/cold --reps 3 $E $(grep ts2 $R/docs.txt) 2>> $R/cold.err
rm -rf $R/cold/work
echo "== done6 $(date -u +%T)"
