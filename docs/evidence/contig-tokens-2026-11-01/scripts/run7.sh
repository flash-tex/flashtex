#!/usr/bin/env bash
# run7.sh: after the papers (run6.sh), only the book with \tracingstats=2, one rep per engine:
# its instructions (which do not move with load) show what the recomputed dyn_used costs.
# The book's own cycles are not measured again: on the contended PC the papers' A/A intervals
# were wider than the effect.
set -e
export PATH=/home/kubar/texlive/2026/bin/x86_64-linux:$PATH
C=~/.cache/flashtex-agents/contig
PERF=$(ls -d /nix/store/*-perf-linux-7.3/bin/perf | head -1)
export PATH=$(dirname $PERF):$PATH
R=$C/res
cd $C/src
E="--engine main=$C/bin-main --engine mainB=$C/bin-mainB --engine lazy=$C/bin-lazy"
echo "== cold ts2 $(date -u +%T)"
python3 $C/bench2.py $R/cold --reps 1 $E $(grep ts2 $R/docs.txt) 2>> $R/cold.err
rm -rf $R/cold/work
echo "== done7 $(date -u +%T)"
