#!/bin/bash
# Build the external-tools corpus (TeX Live documents) locally, to copy to the PC.
set -u
C=/tmp/p5x/corpus
rm -rf $C; mkdir -p $C
T=/usr/local/texlive/2026/texmf-dist/doc/latex
for f in $T/biblatex/examples/*.tex; do
  n=$(basename $f .tex)
  mkdir -p $C/biblatex-$n
  cp $f $C/biblatex-$n/
done
for n in sample-idx sample-idx-t1 sample-idx-a4 sample-idx-subset sample-idx-amsmath sample-idx-hyp sample-idx-letter sample-idx-utf8 sample-idx-german sample-idx-babel-german; do
  mkdir -p $C/testidx-$n
  cp $T/testidx/samples/$n.tex $C/testidx-$n/
done
ls $C | wc -l
