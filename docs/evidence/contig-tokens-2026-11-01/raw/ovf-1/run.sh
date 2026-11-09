#!/usr/bin/env bash
# run.sh ENGINE...: each test with TeX Live pdfTeX (pdftex) or an engine dir of ib/ (main, lazy)
export PATH=/home/kubar/texlive/2026/bin/x86_64-linux:$PATH SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1
O=~/.cache/flashtex-agents/contig/ovf; IB=~/.cache/flashtex-agents/contig/ib
for e in "$@"; do
  for t in ovf1 ovf2; do
    d=$O/$e; mkdir -p $d; cp $O/$t.tex $d/
    if [ $e = pdftex ]; then (cd $d && pdftex -fmt=pdflatex -interaction=batchmode $t.tex >/dev/null 2>&1; echo "$e $t rc=$?")
    else (cd $d && FLASHTEX_POOL=$IB/$e/pdftex.pool FLASHTEX_FORMATS=$IB/fmt-$e $IB/$e/pdftex -fmt=pdflatex -interaction=batchmode $t.tex >/dev/null 2>&1; echo "$e $t rc=$?"); fi
  done
done
