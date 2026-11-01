#!/bin/sh
# The WASI spike's build of the batch CLI (README.md §3), as run on
# mac-m1max-a with Homebrew's llvm, wasi-libc and wasi-runtimes.
#
#   WORK=/tmp/wasi EXTRA_CFLAGS="-mllvm -wasm-use-legacy-eh=false" \
#   CARGO_TARGET_WASM32_WASIP1_RUSTFLAGS="-L /opt/homebrew/share/wasi-sysroot/lib/wasm32-wasip1 -l static=setjmp -l static=c++abi" \
#       sh wasm.sh wasm.log --no-default-features
#
# Without --no-default-features the build stops in kpathsea (sys/wait.h).
S=${WORK:?set WORK to a scratch directory}; EV=$(cd "$(dirname "$0")" && pwd)
SYS=/opt/homebrew/share/wasi-sysroot
LLVM=/opt/homebrew/opt/llvm/bin
log=$1; shift
cd "$EV/../../.." || exit 1
export CC_wasm32_wasip1=$LLVM/clang CXX_wasm32_wasip1=$LLVM/clang++ AR_wasm32_wasip1=$LLVM/llvm-ar
export CFLAGS_wasm32_wasip1="--sysroot=$SYS -mllvm -wasm-enable-sjlj -I$EV/wasi-shim $EXTRA_CFLAGS"
export CXXFLAGS_wasm32_wasip1="--sysroot=$SYS -fno-exceptions -mllvm -wasm-enable-sjlj -I$EV/wasi-shim $EXTRA_CFLAGS"
export CARGO_BUILD_JOBS=3 CARGO_TARGET_DIR=$S/target
cargo build --release -p flashtex-engine --bin flashtex-initex --target wasm32-wasip1 "$@" >"$S/$log" 2>&1
echo exit=$?
grep -E "warning: flashtex-engine@.*error|^error|rust-lld: error|undefined symbol" "$S/$log" | sed 's|.*/out/||;s|.*third_party/||' | sort | uniq -c | sort -rn | head -30
