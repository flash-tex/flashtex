#!/bin/bash
# Verify that the chunked (split) documents typeset identically to the plain one.
# DVI: compare dvitype listings minus the preamble comment (timestamp) line.
# PDF: compare pdftotext output (all pages, form-feed separated).
set -u
cd "$(dirname "$0")"
B=$1; NPAR=$2; PERSEC=$3; C1=$4; C10=$5
for c in 0 $C1 $C10; do python3 gen.py $B $NPAR $PERSEC $c >/dev/null; done
for c in 0 $C1 $C10; do
  mode=split; [ $c = 0 ] && mode=plain
  for i in 1 2; do
    latex -interaction=batchmode -jobname=v-$B-$c "\def\BODY{body-$B-$c.inc}\def\MODE{$mode}\input{main}" >/dev/null
    pdflatex -interaction=batchmode -jobname=vp-$B-$c "\def\BODY{body-$B-$c.inc}\def\MODE{$mode}\input{main}" >/dev/null
  done
  grep -E 'Output written' v-$B-$c.log vp-$B-$c.log
  grep -c '^!' v-$B-$c.log
  dvitype v-$B-$c.dvi | grep -v "^'" | grep -v 'magnification=' > v-$B-$c.dvitype
  pdftotext vp-$B-$c.pdf v-$B-$c.txt
done
for c in $C1 $C10; do
  echo "== body $B chunk $c vs plain"
  if cmp -s v-$B-0.dvitype v-$B-$c.dvitype; then echo "DVI listing IDENTICAL"; else echo "DVI listing DIFFERS"; diff v-$B-0.dvitype v-$B-$c.dvitype | head -20; fi
  if cmp -s v-$B-0.txt v-$B-$c.txt; then echo "pdftotext IDENTICAL"; else echo "pdftotext DIFFERS"; fi
  cmp v-$B-0.aux v-$B-$c.aux && echo "aux IDENTICAL"
done
