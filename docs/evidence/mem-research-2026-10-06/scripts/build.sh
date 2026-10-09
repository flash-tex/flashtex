#!/usr/bin/env bash
# memr/build.sh NAME [FEATURES]: release build of ~/code/flashtex-memr into ~/ib-memr/NAME
set -u
export INCR_BENCH_DIR=$HOME/ib-memr
R=$HOME/.cache/flashtex-actions-runner/.flashtex-actions-runner-2
export RUSTUP_HOME=$R/rustup PATH=$R/cargo/bin:$HOME/texlive/2026/bin/x86_64-linux:$HOME/.nix-profile/bin:$PATH
export RUSTUP_TOOLCHAIN=stable CARGO_BUILD_JOBS=${J:-6}
N=$1; F=${2:-}; WT=${WT:-$HOME/code/flashtex-memr}
export CARGO_TARGET_DIR=$HOME/ib-memr/target-$N
cd $WT || exit 1
if [ -n "$F" ]; then
  cargo build --release --locked -p flashtex-engine -p flashtex-display-list --features "$F" || exit 1
else
  cargo build --release --locked -p flashtex-engine -p flashtex-display-list || exit 1
fi
bash tools/incr-bench/mkeng.sh $N || exit 1
[ -d $INCR_BENCH_DIR/docs/full-1000 ] || python3 tools/incr-bench/mkdocs.py
echo BUILD-DONE $N
