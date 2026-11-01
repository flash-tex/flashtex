#!/bin/bash
# Per-page fixed output cost (1000 "x\par\newpage" pages) vs package set, hyperref default.
# Prints OR time (s, over ~1000 pages) for 3 runs each.
set -u
cd "$(dirname "$0")"
for drv in mainh mainh-nosi mainh-bare; do
  for i in 1 2 3; do
    pdflatex -interaction=batchmode -jobname=pk-$drv "\input{ortime}\def\BODY{body-e-0.inc}\def\MODE{plain}\input{$drv}" >/dev/null
    echo "$drv $(grep -a 'OR-TIME at-enddoc' pk-$drv.log)"
  done
done
