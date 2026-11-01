#!/usr/bin/env bash
# soak_gate.sh [--build]: the resident host's memory must plateau over a long editing session
# (DESIGN.md §5.2, §1.2; lane MEMORY-SAFETY, docs/evidence/mem-soak-2026-10-04/). One host on
# plain-120 takes SOAK_EDITS keystrokes (default 600) through the socket, as the app sends them
# (soak.py: letters typed and backspaced, phrases pasted and deleted, at 16 places across the
# document, everything taken back every 100 keystrokes and then 5 s idle), and fails if what it
# allocates grows. The measure is malloc's bytes in use less the undo logs at each DONE (`heap`:
# DONE.mem's malloc_in_use + mapped_bytes - sealed_bytes, FLASHTEX_MEMSTAT=1; the logs are bounded by the budget
# and swing with retention): Rust's and the C libraries' allocations alike, resident or not.
#
#   heap slope over the second half         <= SOAK_MAX_HEAP_SLOPE MB per 100 edits (default 8)
#   heap end (median of the last 5 %)       <= heap at warm-up (median, edits 30-60) + SOAK_HEAP_MB (30)
#   footprint end (a loose check of what malloc does not see: mappings, the word space)
#                                           <= warm-up footprint peak * 1.5 + 100 MB
#
# Measured on the Mac, plain-120 (docs/evidence/mem-soak-2026-10-04/): origin/main grows 22-32 MB
# per 100 edits in every 300-edit window (heap 96 -> 241 MB at 600 edits); with the fixes the
# windows are -1.2..+3.3 and the heap stays 65-79 MB over 1,200 edits (+6 MB at 600). A leak of
# 2.5-6 MB per 100 edits (the position corrections alone, fixed in the same lane) passes this gate;
# the engine test `a_long_session_keeps_its_bookkeeping_bounded` holds that one. The footprint is
# only loosely checked: on a machine short of memory the system compresses and swaps a leak out of
# it (full-1000 showed a 236 MB footprint while holding 697 MB of undo logs). A host above 4 GB
# is killed at once. Needs TeX Live 2026 (the format build) and a release build of
# flashtex-engine and flashtex-display-list (--build makes one); about 2-3 minutes when idle.
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
  --limit-gb 4 --max-heap-slope "${SOAK_MAX_HEAP_SLOPE:-8}" --heap-const-mb "${SOAK_HEAP_MB:-30}" \
  --max-slope 1000 --max-ratio 1.5 --const-mb 100 --warmup 60 \
  --timeout 2400 --out "$INCR_BENCH_DIR/soak/$DOC-gate" || rc=$?
python3 "$S/soak_curve.py" --every 50 "$INCR_BENCH_DIR/soak/$DOC-gate.jsonl" || true
if [ $rc = 0 ]; then
  echo "soak gate: $DOC plateaus"
else
  echo "soak gate: $DOC FAILED (exit $rc; see $INCR_BENCH_DIR/soak/$DOC-gate.jsonl and .csv)"
  exit 1
fi
