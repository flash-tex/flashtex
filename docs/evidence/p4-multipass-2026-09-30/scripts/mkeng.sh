#!/bin/bash
# mkeng.sh NAME: copy release binaries to ~/flashtex-wt/d1/NAME and build pdflatex.fmt there
set -e
N=$1; W=/Users/dqi26/flashtex/.claude/worktrees/agent-a9b0d190d583b912c
D=/Users/dqi26/flashtex-wt/d1/$N; rm -rf $D; mkdir -p $D/fmt
cp /Users/dqi26/flashtex-wt/target-d1/release/flashtex-initex /Users/dqi26/flashtex-wt/target-d1/release/flashtex-host $D/
cp $W/crates/flashtex-engine/pdftex.pool $D/
ln -sf $D/flashtex-initex $D/pdftex
(cd $D/fmt && SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=$D/pdftex.pool $D/flashtex-initex -ini \
   -jobname=pdflatex -progname=pdflatex -etex -translate-file=cp227.tcx pdflatex.ini </dev/null >/dev/null 2>&1)
ls -la $D/fmt/pdflatex.fmt
