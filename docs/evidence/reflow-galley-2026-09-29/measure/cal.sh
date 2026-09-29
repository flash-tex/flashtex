#!/bin/bash
# Calibration: pages per 1000 paragraphs for each body.
set -u
cd "$(dirname "$0")"
for b in a b c; do
  python3 gen.py $b 1000 300 0 >/dev/null
  pdflatex -interaction=batchmode -jobname=cal-$b "\def\BODY{body-$b-0.inc}\def\MODE{plain}\input{main}" >/dev/null
  grep -E 'Output written|GALLEY|^!' cal-$b.log | head -5
done
