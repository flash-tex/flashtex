#!/usr/bin/env bash
# mkeng.sh NAME: engine NAME (tools/incr-bench/mkeng.sh) from the lane's checkout on the NixOS PC,
# and NAME-ms, the same with the mem-stats host (target/memstats; the counting allocator).
set -e
cd ~/code/flashtex-p4mem
export PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
export INCR_BENCH_DIR=${INCR_BENCH_DIR:-/tmp/p4mem}
N=$1
bash tools/incr-bench/mkeng.sh "$N"
mkdir -p "$INCR_BENCH_DIR/$N-ms"
cp -a "$INCR_BENCH_DIR/$N/." "$INCR_BENCH_DIR/$N-ms/"
cp target/memstats/release/flashtex-host "$INCR_BENCH_DIR/$N-ms/flashtex-host"
ln -sfn "$INCR_BENCH_DIR/$N-ms/flashtex-initex" "$INCR_BENCH_DIR/$N-ms/pdftex"
ln -sfn "$INCR_BENCH_DIR/fmt-$N" "$INCR_BENCH_DIR/fmt-$N-ms"
[ -d "$INCR_BENCH_DIR/docs" ] || python3 tools/incr-bench/mkdocs.py
git rev-parse --short HEAD
