#!/usr/bin/env bash
# build.sh TAG: release builds in ~/code/flashtex-rt2; Classic binaries to ~/rt/ib/TAG (+format), Unicode host to ~/rt/ub/TAG
set -u
. ~/rt/env.sh
W=~/code/flashtex-rt2; T=$1; L=~/rt/rt2/build-$T.log
cd $W && ~/.local/bin/ftx-run 5400 cargo build --release -p flashtex-engine -p flashtex-display-list > $L 2>&1 || { echo classic build failed; tail -20 $L; exit 1; }
(cd $W/crates/flashtex-xetex && ~/.local/bin/ftx-run 5400 cargo build --release --bin flashtex-host-unicode) >> $L 2>&1 || { echo unicode build failed; tail -20 $L; exit 1; }
INCR_BENCH_DIR=~/rt/ib $W/tools/incr-bench/mkeng.sh $T >> $L 2>&1 || { echo mkeng failed; tail $L; exit 1; }
mkdir -p ~/rt/ub/$T && cp $W/crates/flashtex-xetex/target/release/flashtex-host-unicode ~/rt/ub/$T/
echo "built $T $(cd $W && git rev-parse --short HEAD) dirty=$(cd $W && git status -s | wc -l)"
