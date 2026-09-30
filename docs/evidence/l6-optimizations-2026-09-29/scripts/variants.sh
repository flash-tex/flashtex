#!/bin/bash
# variants.sh NAME...: build each named variant of the release engine, one at a
# time, into /tmp/l6o/tgt-NAME, and install it with mkeng.sh as
# /tmp/l6o/eng/NAME. The variants are the candidates of the L6-OPTIMIZATIONS
# evidence (README.md). base/fix/fix2/u1 are source states (snap.sh), the rest
# build options over one of them. The `unchk`/`unchkr` rows of the README were
# measured with a since-removed measurement feature in arena.rs (reads, or
# reads and writes, without bounds checks); src/ix.rs replaced it.
set -e
W=$(cd "$(dirname "$0")/../../../.." && pwd)
S=$W/docs/evidence/l6-optimizations-2026-09-29/scripts
cd "${SRC:-$W}"
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-6}
build() {
  local name=$1; shift
  local t0=$(date +%s)
  echo "== $name: $*"
  env "$@" CARGO_TARGET_DIR=/tmp/l6o/tgt-$name cargo build --release -p flashtex-engine $FEATURES 2>&1 | tail -1
  echo "== $name built in $(( $(date +%s) - t0 )) s"
  bash "$S/mkeng.sh" "$name" /tmp/l6o/tgt-$name
}
for v in "$@"; do
  FEATURES=
  case $v in
    base)     build base X=1 ;;
    fix)      build fix X=1 ;;
    cgu1)     build cgu1 CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 ;;
    thin)     build thin CARGO_PROFILE_RELEASE_LTO=thin ;;
    fat)      build fat CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 ;;
    fatabort) build fatabort CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 CARGO_PROFILE_RELEASE_PANIC=abort ;;
    fatm1)    build fatm1 CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 "RUSTFLAGS=-C target-cpu=apple-m1" ;;
    m1)       build m1 "RUSTFLAGS=-C target-cpu=apple-m1" ;;
    fix2)     build fix2 X=1 ;;
    # From the ix.rs change on (src/ix.rs): unchecked reads by default,
    # `chk` the same source with the checked-arrays feature.
    u1)       build u1 X=1 ;;
    u2)       build u2 X=1 ;;
    u2chk)    FEATURES="--features checked-arrays"; build u2chk X=1 ;;
    u2pgogen) build u2pgogen "RUSTFLAGS=-Cprofile-generate=/tmp/l6o/pgo-raw" ;;
    u2pgo)    build u2pgo "RUSTFLAGS=-Cprofile-use=/tmp/l6o/pgo.profdata" ;;
    u1chk)    FEATURES="--features checked-arrays"; build u1chk X=1 ;;
    u1m1)     build u1m1 "RUSTFLAGS=-C target-cpu=apple-m1" ;;
    u1cgu1)   build u1cgu1 CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 ;;
    u1thin)   build u1thin CARGO_PROFILE_RELEASE_LTO=thin ;;
    u1fat)    build u1fat CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 ;;
    u1pgogen) build u1pgogen "RUSTFLAGS=-Cprofile-generate=/tmp/l6o/pgo-raw" ;;
    u1pgo)    build u1pgo "RUSTFLAGS=-Cprofile-use=/tmp/l6o/pgo.profdata" ;;
    u1fatpgo) build u1fatpgo CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 "RUSTFLAGS=-Cprofile-use=/tmp/l6o/pgo.profdata" ;;
    pgogen)   build pgogen "RUSTFLAGS=-Cprofile-generate=/tmp/l6o/pgo-raw" ;;
    pgo)      build pgo "RUSTFLAGS=-Cprofile-use=/tmp/l6o/pgo.profdata" ;;
    fatpgo)   build fatpgo CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 "RUSTFLAGS=-Cprofile-use=/tmp/l6o/pgo.profdata" ;;
    *) echo "unknown variant $v"; exit 2 ;;
  esac
done
