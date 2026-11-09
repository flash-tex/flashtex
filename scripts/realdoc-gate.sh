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
#             and spends at most pr_gate.to_edited_instr_m_max M engine-thread instructions from
#             the compile's start to the edited page, where the machine counts them: a hosted VM
#             does not, and the budget is then a warning) and typing@150ms@x2-late (whether every
#             key is painted by its own compile: reported, as load moves it); the app test with a
#             100 ms soft budget for a main run-loop turn and for a key into the text storage
#             (EngineV3RealDocTypingTests.Limits: none over 500 ms, at most 5 of 40 over 100 ms,
#             at most 1.5 s in all; #2146)
#   --counted the default, and a machine without instruction counts misses (t7.py --gate counted;
#             ci.yml's real-doc-instr job on a self-hosted Mac)
#   --strict  DESIGN §1.2's wall targets, for a reference Mac: typing@50ms@x2-late <= 20 ms to the
#             edited page, every key its own compile; the app test at one frame (16 ms)
#
#   scripts/realdoc-gate.sh [--strict | --counted] [--t7-only | --app-only] [--no-build] [--keys N]
#
# Needs a TeX Live with infdesc's packages on PATH (kpsewhich pdflatex.ini), Rust and, for the app
# step, Xcode. The book is fetched once at its pinned commit (mkbook.py) into INCR_BENCH_DIR
# (default /tmp/fth-realdoc; /tmp: the host's socket path must fit 104 bytes). Exit: 0 both pass;
# 1 a gate missed; 2 a harness error. On a laptop, measure only when no other measurement runs.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
export INCR_BENCH_DIR="${INCR_BENCH_DIR:-/tmp/fth-realdoc}"

# The app step's verdict from its `swift test` log: `passed`, `failed` or `notrun`. The filter
# selects every test of EngineV3RealDocTypingTests (the stall gate and its limits' unit test), so
# it reads XCTest's last summary line ("Executed N tests, with [K tests skipped and] F failures")
# and passes only when N >= 1, nothing was skipped, nothing failed, every one of the N has a
# "passed" line, and the stall gate itself is among them. Any failure is `failed`; anything else
# (no summary, a skip, a missing gate test) is `notrun`. `--self-test` checks it on sample logs.
GATE_TEST=testTypingOnALatePageOfARealBookNeverStallsTheEditor
app_verdict() {
  local log="$1" summary n skipped failures passed
  summary="$(grep -Eo 'Executed [0-9]+ tests?, with ([0-9]+ tests? skipped and )?[0-9]+ failures?' "$log" | tail -n 1)"
  [[ -n "$summary" ]] || { echo notrun; return; }
  n="$(sed -E 's/^Executed ([0-9]+) .*/\1/' <<<"$summary")"
  failures="$(sed -E 's/.* ([0-9]+) failures?$/\1/' <<<"$summary")"
  skipped=0
  [[ "$summary" == *skipped* ]] && skipped="$(sed -E 's/.*with ([0-9]+) tests? skipped.*/\1/' <<<"$summary")"
  passed="$(grep -Ec "^Test Case '.*' passed" "$log" || true)"
  if (( failures > 0 )); then echo failed
  elif (( n >= 1 && skipped == 0 && passed == n )) && grep -Eq "^Test Case '.*$GATE_TEST\]' passed" "$log"; then echo passed
  else echo notrun
  fi
}
app_verdict_self_test() {
  local d bad=0 want log
  d="$(mktemp -d)"
  tc() { printf "Test Case '-[FlashTeXMacTests.EngineV3RealDocTypingTests %s]' %s (1.0 seconds).\n" "$1" "$2"; }
  write() { local f="$d/$1"; shift; printf '%s\n' "$@" >"$f"; }
  write two-pass "$(tc "$GATE_TEST" passed)" "$(tc testTheHostedLimitsPassTheMeasuredNoiseAndFailTheFreeze passed)" \
    "	 Executed 2 tests, with 0 failures (0 unexpected) in 75.1 (75.2) seconds"
  write one-pass "$(tc "$GATE_TEST" passed)" "	 Executed 1 test, with 0 failures (0 unexpected) in 75.1 (75.2) seconds"
  write gate-failed "$(tc "$GATE_TEST" failed)" "$(tc testTheHostedLimitsPassTheMeasuredNoiseAndFailTheFreeze passed)" \
    "	 Executed 2 tests, with 1 failure (0 unexpected) in 75.1 (75.2) seconds"
  write limits-failed "$(tc "$GATE_TEST" passed)" "$(tc testTheHostedLimitsPassTheMeasuredNoiseAndFailTheFreeze failed)" \
    "	 Executed 2 tests, with 1 failure (0 unexpected) in 75.1 (75.2) seconds"
  write gate-skipped "$(tc "$GATE_TEST" skipped)" "$(tc testTheHostedLimitsPassTheMeasuredNoiseAndFailTheFreeze passed)" \
    "	 Executed 2 tests, with 1 test skipped and 0 failures (0 unexpected) in 0.1 (0.1) seconds"
  write gate-missing "$(tc testTheHostedLimitsPassTheMeasuredNoiseAndFailTheFreeze passed)" \
    "	 Executed 1 test, with 0 failures (0 unexpected) in 0.1 (0.1) seconds"
  write build-error "error: compile command failed due to signal 6"
  write none-run "	 Executed 0 tests, with 0 failures (0 unexpected) in 0.0 (0.0) seconds"
  for want in two-pass:passed one-pass:passed gate-failed:failed limits-failed:failed gate-skipped:notrun \
              gate-missing:notrun build-error:notrun none-run:notrun; do
    log="${want%%:*}"
    if [[ "$(app_verdict "$d/$log")" != "${want#*:}" ]]; then
      echo "realdoc-gate: self-test: $log gave $(app_verdict "$d/$log"), want ${want#*:}" >&2; bad=1
    fi
  done
  rm -rf "$d"
  return $bad
}
# Always first (it takes milliseconds): a parser that reads a pass as "did not run" turns the gate
# red for every PR, and one that reads a failure as a pass turns it off.
app_verdict_self_test || { echo "realdoc-gate: app_verdict self-test failed" >&2; exit 2; }

T7=1; APP=1; BUILD=1; KEYS=16; STRICT=0; GATE=hosted
while [[ $# -gt 0 ]]; do
  case "$1" in
    --self-test) echo "realdoc-gate: app_verdict self-test passed"; exit 0 ;;
    --verdict) app_verdict "$2"; exit 0 ;;
    --strict) STRICT=1; shift ;;
    --counted) GATE=counted; shift ;;
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
    args+=(--gate "$GATE" --phases letter@site,typing@150ms@site --keys 8)
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
  case "$(app_verdict "$INCR_BENCH_DIR/app-test.log")" in
    passed) echo "realdoc-gate: app stall gate passed" ;;
    failed) echo "realdoc-gate: app stall gate FAILED"; [[ $code == 0 ]] && code=1 ;;
    *) echo "realdoc-gate: app stall gate did not run (see $INCR_BENCH_DIR/app-test.log)"; code=2 ;;
  esac
fi
echo "realdoc-gate: exit $code"
exit $code
