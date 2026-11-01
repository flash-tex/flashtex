#!/usr/bin/env bash
# quiet-window.sh — unattended typing-latency cells on the packaged app, each
# run only inside a quiet window: before EVERY cell the script waits until the
# 1-minute load average is below --quiet-load (default 8) and no other
# FlashTeX/FlashTeXMac process is alive, then runs that single cell through
# lib/typing_attribution.py and appends `uptime` (before and after the cell) to
# <out>/uptime.log. Cells whose load still exceeded the limit (after
# --quiet-wait s) are run anyway and marked load-affected in the JSON.
#
# Usage: tools/native-validation/mac-live/quiet-window.sh --app <FlashTeX.app>
#          [--out <dir>] [--seeds "fixture demo hw1 render27 body60k"]
#          [--routes "v1 v1-render v2 controller"] [--intervals "30"]
#          [--quiet-load 8] [--quiet-wait 900] [--load-limit 10] [--no-report]
# The bundle must carry flashtex-compiler, flashtex-render and
# flashtex-preview-controller (run.sh packages all three). Re-running with the
# same --out re-runs only cells without a summary (resume after an interruption).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
APP=""
OUT=""
SEEDS="fixture demo hw1 render27 body60k"
ROUTES="v1 v1-render v2 controller"
INTERVALS="30"
QUIET_LOAD=8
QUIET_WAIT=900
LOAD_LIMIT=10
REPORT=1
while [[ $# -gt 0 ]]; do
  case "$1" in
    --app) APP="$2"; shift 2 ;;
    --out) OUT="$2"; shift 2 ;;
    --seeds) SEEDS="$2"; shift 2 ;;
    --routes) ROUTES="$2"; shift 2 ;;
    --intervals) INTERVALS="$2"; shift 2 ;;
    --quiet-load) QUIET_LOAD="$2"; shift 2 ;;
    --quiet-wait) QUIET_WAIT="$2"; shift 2 ;;
    --load-limit) LOAD_LIMIT="$2"; shift 2 ;;
    --no-report) REPORT=0; shift ;;
    -h|--help) sed -n '2,16p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "quiet-window.sh: unknown argument $1" >&2; exit 1 ;;
  esac
done
[[ -x "$APP/Contents/MacOS/FlashTeX" ]] || { echo "quiet-window.sh: --app must be a FlashTeX.app bundle" >&2; exit 1; }
UTC="$(date -u +%Y%m%dT%H%M%SZ)"
[[ -n "$OUT" ]] || OUT="$SCRIPT_DIR/reports/attribution-$UTC"
mkdir -p "$OUT"
UPLOG="$OUT/uptime.log"
load1() { sysctl -n vm.loadavg | awk '{print $2}'; }
others() { { pgrep -x FlashTeX; pgrep -x FlashTeXMac; } 2>/dev/null | wc -l | tr -d ' '; }

wait_quiet() { # prints the seconds waited
  local waited=0 l o
  while :; do
    l="$(load1)"; o="$(others)"
    if awk -v l="$l" -v q="$QUIET_LOAD" 'BEGIN { exit !(l < q) }' && (( o == 0 )); then echo "$waited"; return 0; fi
    if (( waited >= QUIET_WAIT )); then echo "    load $l / $o other FlashTeX after $QUIET_WAIT s; running anyway" >&2; echo "$waited"; return 0; fi
    (( waited == 0 )) && echo "    load $l (limit $QUIET_LOAD), $o other FlashTeX process(es); waiting up to $QUIET_WAIT s" >&2
    sleep 10; waited=$((waited + 10))
  done
}

echo "quiet-window $UTC app=$APP out=$OUT quiet-load=$QUIET_LOAD quiet-wait=$QUIET_WAIT" | tee -a "$UPLOG"
echo "start: $(uptime)" | tee -a "$UPLOG"
for ms in $INTERVALS; do
  for route in $ROUTES; do
    for seed in $SEEDS; do
      cell="$route-$seed-${ms}ms"
      if [[ -f "$OUT/$cell.json" ]]; then echo "==> $cell already has a summary; skipping" | tee -a "$UPLOG"; continue; fi
      waited="$(wait_quiet)"
      echo "before $cell (waited ${waited}s): $(uptime)" | tee -a "$UPLOG"
      # one cell per invocation: the analyzer's own quiet gate is disabled
      # (--quiet-wait 0) because this loop already waited
      python3 "$SCRIPT_DIR/lib/typing_attribution.py" --app "$APP" --repo "$ROOT" --out "$OUT" \
        --seeds "$seed" --routes "$route" --interval "$ms" --quiet-load "$QUIET_LOAD" --quiet-wait 0 --load-limit "$LOAD_LIMIT" \
        --merge || echo "    cell $cell failed (exit $?)" | tee -a "$UPLOG"
      echo "after  $cell: $(uptime)" | tee -a "$UPLOG"
    done
  done
done
echo "end: $(uptime)" | tee -a "$UPLOG"
if (( REPORT == 1 )); then
  python3 "$SCRIPT_DIR/lib/typing_attribution.py" --report "$OUT" | tee "$OUT/attribution.md" >/dev/null
  echo "wrote $OUT/attribution.md"
fi
