#!/bin/bash
# Pixel-parity matrix for T0 (2026-10-04): two text pages x scales x origin
# sources x shape sources, subpixel quantisation off (and default for the
# original pxdiff row). Writes ../raw/pixel-parity.jsonl.
cd "$(dirname "$0")/proto-copy"
OUT=../raw/pixel-parity.jsonl
mkdir -p ../raw ../png
: > $OUT
for spec in "d10/p2-pos2.json d10/p2.pdf 2 d10p2" "d300/p150-pos2.json d300/d300p150.pdf 150 d300p150"; do
  set -- $spec; J=px/$1; P=px/$2; PG=$3; N=$4
  for s in 1 2 3; do
    for pos in frame nearest cg; do
      for sh in frame pdf; do
        R=$(POS=$pos SHAPES=$sh SUBQ=0 ../pxdiff2 $J $P $PG $s ../png/$N-$s-$pos-$sh)
        echo "{\"doc\":\"$N\",\"load\":\"$(uptime | sed 's/.*load averages: //')\",\"result\":$R}" | tee -a $OUT
      done
    done
  done
done
# The original prototype harness (px/pxdiff.swift, frame shapes, y-down) on the
# pdfpos.py origins, for comparison with Track A's raw-a/pixel-parity.jsonl.
for spec in "d10/p2.json d10/p2.pdf 2 d10p2" "d300/d300p150.json d300/d300p150.pdf 150 d300p150"; do
  set -- $spec; J=px/$1; P=px/$2; PG=$3; N=$4
  ../venv/bin/python px/pdfpos.py $J $P $PG ${J%.json}-pdfpos.json > /dev/null
  for s in 2 3; do
    R=$(SUBQ=0 MBH=841.8898 ../pxdiff ${J%.json}-pdfpos.json $P $PG $s)
    echo "{\"doc\":\"$N\",\"harness\":\"prototype pxdiff.swift + pdfpos.py\",\"load\":\"$(uptime | sed 's/.*load averages: //')\",\"result\":$R}" | tee -a $OUT
  done
done
