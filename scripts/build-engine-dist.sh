#!/usr/bin/env bash
# build-engine-dist.sh: the profile-guided (PGO) release build of the engine
# binaries (crates/flashtex-engine: flashtex-initex, flashtex-host).
#
# Measured in docs/evidence/l6-optimizations-2026-09-29/ (Apple M5 Pro): PGO
# takes 10-12% of the CPU time off the 300/1,000-page benchmark documents,
# which it is not trained on, and changes no output byte. The other build
# options measured there (fat/thin LTO, codegen-units=1, panic=abort,
# target-cpu=apple-m1) gave nothing outside the noise, so this script uses
# the workspace's `release` profile unchanged.
#
#   scripts/build-engine-dist.sh [--profile FILE] [--out DIR]
#
# Steps: (1) an instrumented build (-Cprofile-generate) in target/pgo-gen;
# (2) its pdflatex format, then every parity fixture (fixtures/real-world,
# fixtures/divergence-probes) and the 10- and 100-page benchmark documents
# (docs/evidence/p4-l2-l3-2026-09-29/scripts/gen.py), each compiled twice;
# (3) llvm-profdata merge; (4) the final build with -Cprofile-use in
# target/pgo-use, copied to --out (default target/dist).
#
# --profile FILE skips steps 1-3 and builds with a merged profile made
# before. Training runs are not bit-for-bit repeatable (the counts depend on
# hash seeds and file-system state), so a reproducible release pins the
# .profdata it was built with (publish it with its sha256) and rebuilds with
# --profile. A profile made for older sources still works: functions whose
# code changed simply lose their profile.
#
# Needs: a TeX Live that the engine's resolver finds (as every engine run
# does; FLASHTEX_TEXLIVE_BIN overrides), and llvm-profdata of rustc's LLVM
# major version (`rustup component add llvm-tools`, or LLVM_PROFDATA=path;
# Xcode's `xcrun llvm-profdata` reads rustc's raw profiles too).
set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"
OUT=target/dist
PROFILE=
while [ $# -gt 0 ]; do
  case $1 in
    --profile) PROFILE=$(cd "$(dirname "$2")" && pwd)/$(basename "$2"); shift 2 ;;
    --out) OUT=$2; shift 2 ;;
    -h|--help) sed -n '2,/^set -euo/p' "$0" | sed 's/^# \{0,1\}//;$d'; exit 0 ;;
    *) echo "build-engine-dist.sh: unknown argument $1" >&2; exit 2 ;;
  esac
done

find_profdata() {
  if [ -n "${LLVM_PROFDATA:-}" ]; then echo "$LLVM_PROFDATA"; return; fi
  local sysroot host
  sysroot=$(rustc --print sysroot)
  host=$(rustc -vV | sed -n 's/^host: //p')
  if [ -x "$sysroot/lib/rustlib/$host/bin/llvm-profdata" ]; then
    echo "$sysroot/lib/rustlib/$host/bin/llvm-profdata"; return
  fi
  if command -v xcrun >/dev/null && xcrun -f llvm-profdata >/dev/null 2>&1; then
    xcrun -f llvm-profdata; return
  fi
  command -v llvm-profdata || { echo "build-engine-dist.sh: no llvm-profdata (rustup component add llvm-tools)" >&2; exit 1; }
}

if [ -z "$PROFILE" ]; then
  WORK=$ROOT/target/pgo-train
  RAW=$WORK/raw
  rm -rf "$WORK"; mkdir -p "$RAW" "$WORK/runs" "$WORK/fmt"
  echo "== 1. instrumented build"
  RUSTFLAGS="-Cprofile-generate=$RAW" CARGO_TARGET_DIR=target/pgo-gen \
    cargo build --release --locked -p flashtex-engine
  BIN=$ROOT/target/pgo-gen/release/flashtex-initex
  POOL=$ROOT/crates/flashtex-engine/pdftex.pool
  echo "== 2. training"
  (cd "$WORK/fmt" && SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 FLASHTEX_POOL=$POOL "$BIN" -ini \
    -jobname=pdflatex -progname=pdflatex -etex -translate-file=cp227.tcx pdflatex.ini </dev/null >/dev/null) \
    || { echo "format build failed; see $WORK/fmt/pdflatex.log" >&2; exit 1; }
  rm -rf "$RAW"; mkdir -p "$RAW"   # train on documents, not on the format build
  export SOURCE_DATE_EPOCH=1700000000 FORCE_SOURCE_DATE=1 TZ=UTC FLASHTEX_POOL=$POOL FLASHTEX_FORMATS=$WORK/fmt
  n=0
  train() { # dir file
    (cd "$1" && for _ in 1 2; do "$BIN" -fmt=pdflatex -interaction=batchmode "$2" >/dev/null 2>&1 || true; done)
    n=$((n + 1))
  }
  for f in fixtures/real-world/*/main.tex fixtures/divergence-probes/*/main.tex; do
    d=$WORK/runs/$(basename "$(dirname "$f")")
    mkdir -p "$d"; cp -R "$(dirname "$f")"/. "$d"/
    train "$d" main.tex
  done
  python3 docs/evidence/p4-l2-l3-2026-09-29/scripts/gen.py "$WORK/docs" >/dev/null
  for doc in plain-10 plain-100 full-10 full-100; do
    d=$WORK/runs/$doc; mkdir -p "$d"; cp "$WORK/docs/$doc.tex" "$d"/
    train "$d" "$doc.tex"
  done
  echo "trained on $n documents"
  echo "== 3. merge"
  PROFILE=$WORK/flashtex-engine.profdata
  "$(find_profdata)" merge -o "$PROFILE" "$RAW"
fi

echo "== 4. optimised build with $PROFILE"
# Always from the same path: the path is part of RUSTFLAGS, which cargo
# hashes into symbol names, so another path gives other bytes.
mkdir -p target/pgo-use
[ "$PROFILE" -ef target/pgo-use/flashtex-engine.profdata ] || cp "$PROFILE" target/pgo-use/flashtex-engine.profdata
RUSTFLAGS="-Cprofile-use=$ROOT/target/pgo-use/flashtex-engine.profdata" CARGO_TARGET_DIR=target/pgo-use \
  cargo build --release --locked -p flashtex-engine
mkdir -p "$OUT"
cp target/pgo-use/release/flashtex-initex target/pgo-use/release/flashtex-host "$OUT"/
cp target/pgo-use/flashtex-engine.profdata "$OUT"/
(cd "$OUT" && shasum -a 256 flashtex-initex flashtex-host flashtex-engine.profdata)
