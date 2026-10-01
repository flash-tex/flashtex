#!/usr/bin/env bash
# APP-PERF-AUDIT: editor-only typing cell (no producer, no preview frames), so
# the main-thread cost per keystroke of the editor + SwiftUI shell is isolated.
# usage: editor-only.sh <label> <doc> [at] [extra bench.sh args...]
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LABEL="$1"; DOC="$2"; AT="${3:-mid-paragraph}"; shift 3 || shift $#
C="/tmp/appperf/runs/$LABEL-$DOC-editor-$AT"
rm -rf "$C"
"$HERE/bench.sh" "/tmp/appperf/docs/$DOC.tex" "$C" --at "$AT" --trace "Time Profiler" \
  --env FLASHTEX_AUTOATTACH=0 --env FLASHTEX_TYPING_BENCH_NO_WORKER=1 --env FLASHTEX_TYPING_BENCH_SETTLE_MS=1000 "$@" > /dev/null 2>&1
"$HERE/analyze.sh" "$C" > /dev/null 2>&1
echo "$(basename "$C") $(sed -n 2p "$C/load.txt")"
sed -n '/## signposts/,/## hangs/p' "$C/summary.txt" | grep -E 'editorChange|modelUpdate'
python3 "$HERE/mainthread.py" "$C" textDidChange 'updateNSView' GraphHost.flushTransactions 'CA::Transaction::commit' 'CompletingTextView.draw' 'NSHostingView.layout()'
