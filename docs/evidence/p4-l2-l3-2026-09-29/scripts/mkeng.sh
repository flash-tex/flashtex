#!/bin/bash
# mkeng.sh NAME: copy the worktree's release binaries to /tmp/p4l2/NAME and build its pdflatex format.
set -e
W=/Users/kubar/code/flashtex/.claude/worktrees/agent-a139454d4ebf282eb
N=$1
D=/tmp/p4l2/$N
mkdir -p $D; [ "$2" = "--keep-fmt" ] || mkdir -p /tmp/p4l2/fmt-$N
cp $W/target/release/flashtex-initex $W/target/release/flashtex-host $D/
cp $W/crates/flashtex-engine/pdftex.pool $D/
ln -sf $D/flashtex-initex $D/pdftex
if [ "$2" != "--keep-fmt" ]; then
  cd /tmp/p4l2/fmt-$N
  SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=$D/pdftex.pool $D/flashtex-initex -ini -jobname=pdflatex -progname=pdflatex -etex -translate-file=cp227.tcx pdflatex.ini </dev/null >/dev/null 2>&1 || { echo "format build failed"; tail -5 pdflatex.log; exit 1; }
  ls -la /tmp/p4l2/fmt-$N/pdflatex.fmt
fi
