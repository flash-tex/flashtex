#!/usr/bin/env bash
# INCR-FILE-SOUNDNESS: build engine+host into $B
set -u
W=$HOME/code/flashtex-incrfile
B=${B:-/tmp/ifs-dev}
export PATH=$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-8}
cd $W
nice -n 5 timeout 3000 cargo build --release -p flashtex-engine --bin flashtex-host --bin flashtex-initex > $B.build.txt 2>&1 || { tail -30 $B.build.txt; exit 1; }
mkdir -p $B/eng $B/fmt
cp target/release/flashtex-initex target/release/flashtex-host $B/eng/
cp crates/flashtex-engine/pdftex.pool $B/eng/
ln -sf $B/eng/flashtex-initex $B/eng/pdftex
if [ ! -f $B/fmt/pdflatex.fmt ]; then (cd $B/fmt && SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=$B/eng/pdftex.pool timeout 600 $B/eng/flashtex-initex -ini -jobname=pdflatex -progname=pdflatex -etex -translate-file=cp227.tcx pdflatex.ini </dev/null >/dev/null 2>&1); fi
ls -la $B/fmt/pdflatex.fmt $B/eng/flashtex-host
