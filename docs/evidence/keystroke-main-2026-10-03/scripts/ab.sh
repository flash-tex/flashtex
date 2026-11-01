#!/bin/bash
# P5-KEYSTROKE-MAIN: interleaved before/after cells, same document, keys and
# machine state. Each pair runs BEFORE then AFTER back to back.
# usage: ab.sh <outroot> <doc.tex> <pairs> <before-app> <after-app> [after-label] [after-extra-env]
# Env: CELL=v2type.sh for the editor-only v2-pane cell (default v3type.sh);
#      KEYS and MS (default 200 keys every 30 ms).
# Appends one line per cell to <outroot>/results.txt (main-thread ms/keystroke
# and the per-frame split, from the Time Profiler, plus the load average).
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MT="$HERE/../../app-perf-2026-09-30/scripts/mainthread.py"
ROOT="$1"; DOC="$2"; PAIRS="$3"; BEFORE="$4"; AFTER="$5"; ALABEL="${6:-after}"; AENV="${7:-}"
mkdir -p "$ROOT"
FOCUS=(GraphHost.flushTransactions 'CA::Transaction::commit' 'NSHostingView.layout()' 'CompletingTextView.draw' 'LineNumberGutter.draw' EngineV3 textDidChange)
cell() { # <label> <app> <extra-env>
  local c="$ROOT/$1"
  APP_BIN="$2" EXTRA_ENV="$3" "$HERE/${CELL:-v3type.sh}" "$c" "$DOC" "Time Profiler" "${KEYS:-200}" "${MS:-30}" > "$c.log" 2>&1
  { echo "$(basename "$c") | $(sed -n 3p "$c/env.txt") | $(tail -1 "$c/env.txt")"
    python3 "$MT" "$c" "${FOCUS[@]}"
    [[ -f "$c/typing.json" ]] && python3 - "$c/typing.json" <<'PY'
import json,sys
d=json.load(open(sys.argv[1]))
print(f"  keystroke->first changed page committed: p50 {d.get('p50Ms') or 0:.1f} ms p95 {d.get('p95Ms') or 0:.1f} ms ({d['samples']} samples)")
PY
  } >> "$ROOT/results.txt" 2>&1
  rm -f "$c/tp.xml" # 15 MB; the trace bundle stays for re-export
}
for i in $(seq 1 "$PAIRS"); do
  cell "before$i" "$BEFORE" ""
  cell "$ALABEL$i" "$AFTER" "$AENV"
done
cat "$ROOT/results.txt"
