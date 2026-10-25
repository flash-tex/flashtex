#!/usr/bin/env bash
# realdoc-gate.sh: the real-document fast cycle (lane REAL-DOC-GATE, 2026-10-24; DESIGN.md §1.2, §9),
# the PR CI's `real-doc gate` job, in one command:
#
#   1. T7's real-document rows at infdesc-x2's late page in the second copy (site x2-late, the
#      owner's page 883; the 1,152-page heavy benchmark);
#   2. the app's stall gate: EngineV3RealDocTypingTests (the whole app on a real host types into
#      that page during and after the first compile).
#
# Two profiles (Commander, 2026-10-24: wall-clock ms on a shared 3-vCPU hosted VM is noise):
#   default   the PR CI's, hardware independent: t7.py --gate hosted (realdocs.json pr_gate) on
#             letter@x2-late (each key re-typesets at most 3 pages, i.e. converges near the edit,
#             and spends at most pr_gate.edited_instr_m_max M engine instructions to the edited
#             page) and typing@150ms@x2-late (every key painted by its own compile); the app test
#             with a 100 ms budget for a main run-loop turn and for a key into the text storage
#   --strict  DESIGN §1.2's wall targets, for a reference Mac: typing@50ms@x2-late <= 20 ms to the
#             edited page, every key its own compile; the app test at one frame (16 ms)
#
#   scripts/realdoc-gate.sh [--strict] [--t7-only | --app-only] [--no-build] [--keys N]
#
# Needs a TeX Live with infdesc's packages on PATH (kpsewhich pdflatex.ini), Rust and, for the app
# step, Xcode. The book is fetched once at its pinned commit (mkbook.py) into INCR_BENCH_DIR
# (default /tmp/fth-realdoc; /tmp: the host's socket path must fit 104 bytes). Exit: 0 both pass;
# 1 a gate missed; 2 a harness error. On a laptop, measure only when no other measurement runs.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
export INCR_BENCH_DIR="${INCR_BENCH_DIR:-/tmp/fth-realdoc}"
T7=1; APP=1; BUILD=1; KEYS=16; STRICT=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --strict) STRICT=1; shift ;;
    --t7-only) APP=0; shift ;;
    --app-only) T7=0; shift ;;
    --no-build) BUILD=0; shift ;;
    --keys) KEYS="$2"; shift 2 ;;
    -h|--help) awk 'NR == 1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "$0"; exit 0 ;;
    *) echo "realdoc-gate: unknown option $1" >&2; exit 2 ;;
  esac
done
code=0
python3 tools/incr-bench/mkbook.py infdesc-x2 >/dev/null
if [[ $T7 == 1 ]]; then
  args=(--engine realdoc --docs infdesc-x2 --sites x2-late --typing-keys "$KEYS" --out "$INCR_BENCH_DIR/t7-out")
  if [[ $STRICT == 1 ]]; then
    args+=(--phases typing@50ms@site)
  else
    args+=(--gate hosted --phases letter@site,typing@150ms@site --keys 8)
  fi
  [[ $BUILD == 1 ]] && args+=(--build)
  rm -rf "$INCR_BENCH_DIR/t7-out"
  echo "+ t7.py ${args[*]}"
  set +e
  python3 tools/incr-bench/t7.py "${args[@]}"
  t7=$?
  set -e
  echo "realdoc-gate: T7 exit $t7"
  case $t7 in 0|3) ;; 1) code=1 ;; *) code=2 ;; esac
fi
if [[ $APP == 1 ]]; then
  if [[ -z "${FLASHTEX_HOST:-}" ]]; then
    [[ $BUILD == 1 ]] && cargo build --release --locked -p flashtex-engine --bin flashtex-host
    export FLASHTEX_HOST="${CARGO_TARGET_DIR:-$ROOT/target}/release/flashtex-host"
  fi
  export FLASHTEX_REALDOC_DIR="$INCR_BENCH_DIR/docs/infdesc-x2" FLASHTEX_REQUIRE_REALDOC=1 FLASHTEX_REQUIRE_HOST=1
  export FLASHTEX_V3_CACHE="${FLASHTEX_V3_CACHE:-$INCR_BENCH_DIR/v3-cache}"
  [[ $STRICT == 1 ]] || export FLASHTEX_REALDOC_STALL_MS="${FLASHTEX_REALDOC_STALL_MS:-100}"
  rm -rf "$FLASHTEX_V3_CACHE"; mkdir -p "$FLASHTEX_V3_CACHE"
  set +e
  (cd apps/mac && swift test --filter 'FlashTeXMacTests\.EngineV3RealDocTypingTests' 2>&1 | tee "$INCR_BENCH_DIR/app-test.log" |
     grep -E 'RealDocTyping|error:|failed|passed|skipped|Executed')
  set -e
  if grep -q "Executed 1 test, with 0 failures" "$INCR_BENCH_DIR/app-test.log" && ! grep -q "skipped" "$INCR_BENCH_DIR/app-test.log"; then
    echo "realdoc-gate: app stall gate passed"
  elif grep -qE "with [1-9][0-9]* failures?" "$INCR_BENCH_DIR/app-test.log"; then
    echo "realdoc-gate: app stall gate FAILED"; [[ $code == 0 ]] && code=1
  else
    echo "realdoc-gate: app stall gate did not run (see $INCR_BENCH_DIR/app-test.log)"; code=2
  fi
fi
echo "realdoc-gate: exit $code"
exit $code
