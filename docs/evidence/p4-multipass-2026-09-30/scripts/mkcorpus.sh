#!/bin/bash
# D1: the P5-EXTERNAL-TOOLS corpus on this Mac (arXiv 60 from the parity cache, TL biblatex examples, testidx)
W=/Users/dqi26/flashtex/.claude/worktrees/agent-a9b0d190d583b912c
C=/Users/dqi26/flashtex-wt/d1/corpus
rm -rf $C; mkdir -p $C
L=/Users/dqi26/flashtex-wt/d1/corpus.txt; : > $L
grep arxiv $W/docs/evidence/p5-external-tools-2026-09-30/raw/linux/p5x-corpus.txt | while read p m rest; do
  i=$(basename $p); echo "$HOME/.cache/flashtex-parity/src/arxiv/$i $m" >> $L; done
T=/usr/local/texlive/2026/texmf-dist/doc/latex
for f in $T/biblatex/examples/*.tex; do n=$(basename $f .tex); mkdir -p $C/biblatex-$n; cp $f $C/biblatex-$n/; echo "$C/biblatex-$n $n.tex" >> $L; done
for n in sample-idx sample-idx-t1 sample-idx-a4 sample-idx-subset sample-idx-amsmath sample-idx-hyp sample-idx-letter sample-idx-utf8 sample-idx-german sample-idx-babel-german; do
  mkdir -p $C/testidx-$n; cp $T/testidx/samples/$n.tex $C/testidx-$n/; echo "$C/testidx-$n $n.tex" >> $L; done
wc -l < $L
