#!/usr/bin/env bash
# soak_gate.sh [--build]: the resident host's memory must plateau over a long editing session
# (DESIGN.md §5.2, §1.2; lane MEMORY-SAFETY, docs/evidence/mem-soak-2026-10-04/). One host on
# plain-120 takes SOAK_EDITS keystrokes (default 600) through the socket, as the app sends them
# (soak.py: letters typed and backspaced, phrases pasted and deleted, at 16 places across the
# document, everything taken back every 100 keystrokes and then 5 s idle), and fails if its
# footprint (macOS phys_footprint, Linux VmRSS) at each DONE grows:
#
#   end (median of the last 5 %)  <= warm-up peak (first 10 %) * 1.15 + SOAK_CONST_MB (default 40)
#   least-squares slope over the second half  <= SOAK_MAX_SLOPE MB per 100 edits (default 20)
#
# Measured on the Mac (evidence README): before the fixes plain-120 grew 23-25 MB per 100 edits
# with no plateau (warm-up peak 262-288 MB, 380-386 MB at 600 edits: over the 370 MB limit);
# after, the footprint stayed at 135-272 MB over 1,200 edits (second-half slope -9). The undo logs
# alone swing by +-40 MB as retention and convergence drop and add checkpoints, which is why the
# limits leave that much room. A host above 4 GB is killed at once.
# Needs TeX Live 2026 (the format build) and a release build of flashtex-engine and
# flashtex-display-list (--build makes one); about 2-3 minutes on an idle machine.
set -euo pipefail
S=$(cd "$(dirname "$0")" && pwd)
W=$(cd "$S/../.." && pwd)
export INCR_BENCH_DIR=${INCR_BENCH_DIR:-${RUNNER_TEMP:-/tmp}/incr-bench-soak}
cd "$W"
if [ "${1:-}" = "--build" ]; then
  cargo build --release --locked -p flashtex-engine -p flashtex-display-list
fi
bash "$S/mkeng.sh" soakgate
python3 "$S/mkdocs.py"
DOC=${SOAK_DOC:-plain-120}
rc=0
python3 "$S/soak.py" soakgate "$DOC" --edits "${SOAK_EDITS:-600}" --gap-ms 50 --idle-s 5 \
  --limit-gb 4 --max-slope "${SOAK_MAX_SLOPE:-20}" --max-ratio 1.15 --const-mb "${SOAK_CONST_MB:-40}" \
  --timeout 2400 --out "$INCR_BENCH_DIR/soak/$DOC-gate" || rc=$?
python3 "$S/soak_curve.py" --every 50 "$INCR_BENCH_DIR/soak/$DOC-gate.jsonl" || true
if [ $rc = 0 ]; then
  echo "soak gate: $DOC plateaus"
else
  echo "soak gate: $DOC FAILED (exit $rc; see $INCR_BENCH_DIR/soak/$DOC-gate.jsonl and .csv)"
  exit 1
fi
