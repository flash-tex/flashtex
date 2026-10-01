#!/bin/bash
# gates.sh ENGINE: the lane's gates, output in raw/ (README, "Gates"): P-T1/
# P-T2 on the parity fixtures and lockstep for ENGINE ($ROOT/eng/ENGINE),
# then trip, etrip, web2rust drift and the engine's tests on the worktree.
# Derived from docs/evidence/l6-optimizations-2026-09-29/scripts/gates.sh.
set -u
W=$(cd "$(dirname "$0")/../../../.." && pwd)
R=$W/docs/evidence/p6-throughput-2026-09-30/raw
ROOT=${ROOT:-$HOME/flashtex-wt/d2}
J=${J:-2}
cd "$W"
mkdir -p "$R"
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-3} CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$HOME/flashtex-wt/target-d2}
E=$1
D=$ROOT/eng/$E
rm -rf "$ROOT/parity-work-$E"; mkdir -p "$R/parity-fixtures-$E"
echo "== parity fixtures $E $(date -u +%T) load $(sysctl -n vm.loadavg)"
python3 tools/parity/parity.py --tier fixtures --engine "$D/flashtex-initex" \
  --engine-env FLASHTEX_POOL="$D/pdftex.pool" --engine-env FLASHTEX_FORMATS="$D/fmt" \
  --oracle-pdftex "$(command -v pdftex)" --pt on --raster none -j "$J" \
  --out "$R/parity-fixtures-$E" --work "$ROOT/parity-work-$E" --require-pt \
  --check-baseline tools/parity/baseline-fixtures.json \
  > "$R/parity-fixtures-$E/console.txt" 2>&1; echo "parity exit $?"
tail -4 "$R/parity-fixtures-$E/console.txt"
echo "== lockstep $E $(date -u +%T)"
FLASHTEX_POOL=$D/pdftex.pool python3 tools/lockstep/run.py --engine "$D/flashtex-initex" \
  > "$R/lockstep-$E.txt" 2>&1; echo "lockstep exit $?"
tail -3 "$R/lockstep-$E.txt"
echo "== trip $(date -u +%T)"
sh scripts/flashtex-trip.sh > "$R/trip.txt" 2>&1; echo "trip exit $?"; tail -2 "$R/trip.txt"
echo "== etrip $(date -u +%T)"
sh scripts/flashtex-etrip.sh > "$R/etrip.txt" 2>&1; echo "etrip exit $?"; tail -2 "$R/etrip.txt"
echo "== drift $(date -u +%T)"
cargo test --release -p web2rust --test drift > "$R/drift.txt" 2>&1; echo "drift exit $?"; tail -2 "$R/drift.txt"
echo "== engine tests $(date -u +%T)"
FLASHTEX_REQUIRE_TEXLIVE=1 cargo test --release --locked -p flashtex-engine --no-fail-fast \
  > "$R/engine-tests.txt" 2>&1; echo "engine tests exit $?"
grep -E '^test result|FAILED|panicked' "$R/engine-tests.txt" | sort | uniq -c | tail -8
echo "== done $(date -u +%T)"
