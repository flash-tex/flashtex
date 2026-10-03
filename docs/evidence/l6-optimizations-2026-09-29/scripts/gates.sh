#!/bin/bash
# gates.sh ENGINE...: the lane's gates, output in raw/ (README, "Gates"):
# P-T1/P-T2 on the parity fixtures and lockstep for each ENGINE in
# /tmp/l6o/eng (the default build and the PGO build), then trip, etrip,
# web2rust drift and `scripts/gate.sh pr` once on the worktree.
set -u
W=$(cd "$(dirname "$0")/../../../.." && pwd)
R=$W/docs/evidence/l6-optimizations-2026-09-29/raw
cd "$W"
mkdir -p "$R"
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-6}
for E in "$@"; do
  D=/tmp/l6o/eng/$E
  mkdir -p "$R/parity-fixtures-$E"
  echo "== parity fixtures $E $(date +%T) load $(sysctl -n vm.loadavg 2>/dev/null)"
  python3 tools/parity/parity.py --tier fixtures --engine "$D/flashtex-initex" \
    --engine-env FLASHTEX_POOL=$D/pdftex.pool --engine-env FLASHTEX_FORMATS=$D/fmt \
    -j 6 --out "$R/parity-fixtures-$E" --check-baseline tools/parity/baseline-fixtures.json \
    > "$R/parity-fixtures-$E/console.txt" 2>&1 || echo "parity exit $?"
  tail -3 "$R/parity-fixtures-$E/console.txt"
  echo "== lockstep $E $(date +%T)"
  FLASHTEX_POOL=$D/pdftex.pool python3 tools/lockstep/run.py --engine "$D/flashtex-initex" \
    > "$R/lockstep-$E.txt" 2>&1 || echo "lockstep exit $?"
  tail -3 "$R/lockstep-$E.txt"
done
echo "== trip $(date +%T)"
sh scripts/flashtex-trip.sh > "$R/trip.txt" 2>&1; echo "trip exit $?"; tail -2 "$R/trip.txt"
echo "== etrip $(date +%T)"
sh scripts/flashtex-etrip.sh > "$R/etrip.txt" 2>&1; echo "etrip exit $?"; tail -2 "$R/etrip.txt"
echo "== drift $(date +%T)"
cargo test --release -p web2rust --test drift > "$R/drift.txt" 2>&1; echo "drift exit $?"; tail -2 "$R/drift.txt"
echo "== gate.sh pr $(date +%T)"
FLASHTEX_GATE_BASE=${FLASHTEX_GATE_BASE:-origin/agent/kabir-claude/l6-hyperref-intrinsics} \
  bash scripts/gate.sh pr > "$R/gate-pr-$(git rev-parse --short HEAD).txt" 2>&1; echo "gate exit $?"
tail -25 "$R/gate-pr-$(git rev-parse --short HEAD).txt"
echo "== done $(date +%T)"
