#!/bin/bash
# usage: run1.sh DIR [prefix cmd...]: one engine pass in DIR (env given by caller)
D=$1; shift
cd "$D" || exit 1
export SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=/tmp/cs/ep/eng/pdftex.pool FLASHTEX_FORMATS=/tmp/cs/ep/fmt
exec "$@" ${ENGINE:-/tmp/cs/ep/eng/pdftex} -fmt=pdflatex -interaction=batchmode -jobname=infdesc '\pdfsetrandomseed 1\relax\input{infdesc.tex}'
