#!/bin/bash
# mkeng.sh NAME [--keep-fmt]: copy this worktree's release binaries to /tmp/p4f/NAME and
# build its pdflatex format in /tmp/p4f/fmt-NAME (the P4-L5 recipe). Every engine run
# has a time limit (to.sh).
set -e
S=$(cd "$(dirname "$0")" && pwd)
W=$(cd "$S/../../../.." && pwd)
N=$1
D=/tmp/p4f/$N
mkdir -p $D
cp $W/target/release/flashtex-initex $W/target/release/flashtex-host $W/target/release/dl3-keys $W/target/release/dl3-client $D/
cp $W/crates/flashtex-engine/pdftex.pool $D/
ln -sf $D/flashtex-initex $D/pdftex
(cd $W && git rev-parse --short HEAD) > $D/HEAD
if [ "$2" != "--keep-fmt" ]; then
  mkdir -p /tmp/p4f/fmt-$N; cd /tmp/p4f/fmt-$N
  SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=$D/pdftex.pool $S/to.sh 300 $D/flashtex-initex -ini -jobname=pdflatex -progname=pdflatex -etex -translate-file=cp227.tcx pdflatex.ini </dev/null >/dev/null 2>&1 || { echo "format build failed"; tail -5 pdflatex.log; exit 1; }
  ls -la /tmp/p4f/fmt-$N/pdflatex.fmt
fi
