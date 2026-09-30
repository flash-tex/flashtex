#!/usr/bin/env bash
# mkeng.sh NAME [--keep-fmt]: copy this checkout's release binaries (flashtex-host, flashtex-initex,
# dl3-keys, dl3-client; `cargo build --release -p flashtex-engine -p flashtex-display-list`) to
# $INCR_BENCH_DIR/NAME and build its pdflatex format in $INCR_BENCH_DIR/fmt-NAME.
set -e
IB=${INCR_BENCH_DIR:-/tmp/incr-bench}
S=$(cd "$(dirname "$0")" && pwd)
W=$(cd "$S/../.." && pwd)
N=$1
D=$IB/$N
mkdir -p $D
cp $W/target/release/flashtex-initex $W/target/release/flashtex-host $W/target/release/dl3-keys $W/target/release/dl3-client $D/
cp $W/crates/flashtex-engine/pdftex.pool $D/
ln -sf $D/flashtex-initex $D/pdftex
(cd $W && git rev-parse --short HEAD) > $D/HEAD
if [ "$2" != "--keep-fmt" ]; then
  mkdir -p $IB/fmt-$N; cd $IB/fmt-$N
  SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=$D/pdftex.pool $S/to.sh 300 $D/flashtex-initex -ini -jobname=pdflatex -progname=pdflatex -etex -translate-file=cp227.tcx pdflatex.ini </dev/null >/dev/null 2>&1 || { echo "format build failed"; tail -5 pdflatex.log; exit 1; }
  ls -la $IB/fmt-$N/pdflatex.fmt
fi
