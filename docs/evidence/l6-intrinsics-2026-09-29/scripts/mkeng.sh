#!/bin/bash
# mkeng.sh NAME [--keep-fmt]: copy this worktree's release binaries to /tmp/l6/NAME
# and build their pdflatex format in /tmp/l6/fmt-NAME (as P4-L2-L3's mkeng.sh).
set -e
W=$(cd "$(dirname "$0")/../../../.." && pwd)
N=$1
D=/tmp/l6/$N
mkdir -p "$D"
rm -f "$D/flashtex-initex" "$D/flashtex-host"
cp "$W/target/release/flashtex-initex" "$W/target/release/flashtex-host" "$D/"
cp "$W/crates/flashtex-engine/pdftex.pool" "$D/"
if [ "$2" != "--keep-fmt" ]; then
  mkdir -p /tmp/l6/fmt-$N
  cd /tmp/l6/fmt-$N
  SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=$D/pdftex.pool "$D/flashtex-initex" -ini -jobname=pdflatex -progname=pdflatex -etex -translate-file=cp227.tcx pdflatex.ini </dev/null >/dev/null 2>&1 || { echo "format build failed"; tail -5 pdflatex.log; exit 1; }
  ls -la /tmp/l6/fmt-$N/pdflatex.fmt
fi
