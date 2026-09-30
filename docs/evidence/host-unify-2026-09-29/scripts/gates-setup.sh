#!/bin/bash
# Copy this worktree's release binaries to /tmp/hu-gates/eng and build its pdflatex format.
set -e
W=/Users/kubar/code/flashtex/.claude/worktrees/agent-a2411aceaee51c6fc
D=/tmp/hu-gates/eng
F=/tmp/hu-gates/fmt
rm -rf $D $F; mkdir -p $D $F
cp $W/target/release/flashtex-initex $W/target/release/flashtex-host $D/
cp $W/crates/flashtex-engine/pdftex.pool $D/
ln -sf $D/flashtex-initex $D/pdftex
cd $F
SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=$D/pdftex.pool $D/flashtex-initex -ini -jobname=pdflatex -progname=pdflatex -etex -translate-file=cp227.tcx pdflatex.ini </dev/null >/dev/null 2>&1
ls -la $F/pdflatex.fmt
cd $W && git rev-parse --short HEAD > $D/HEAD
cat $D/HEAD
