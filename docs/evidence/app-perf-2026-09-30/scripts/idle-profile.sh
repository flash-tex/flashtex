#!/usr/bin/env bash
# APP-PERF-AUDIT: Time Profiler over an idle, settled app (all threads).
# usage: idle-profile.sh <doc> <outdir> [seconds]
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOC="$1"; OUT="$2"; SECS="${3:-40}"
rm -rf "$OUT"
PID=$("$HERE/launch.sh" "/tmp/appperf/docs/$DOC.tex" "$OUT" FLASHTEX_NO_ACTIVATE=1)
until grep -q 'status: revision .*: \(ok\|recovered\|failed\)' "$OUT/app.log" 2>/dev/null; do sleep 1; done
sleep 15
echo "load $(sysctl -n vm.loadavg)" > "$OUT/load.txt"
xcrun xctrace record --template 'Time Profiler' --attach "$PID" --time-limit "${SECS}s" --output "$OUT/trace.trace" --no-prompt > "$OUT/xctrace.txt" 2>&1
kill "$PID"
xcrun xctrace export --input "$OUT/trace.trace" --xpath '/trace-toc/run[@number="1"]/data/table[@schema="time-profile"]' > "$OUT/tp.xml"
cat "$OUT/load.txt"
python3 "$HERE/tp_summary.py" "$OUT/tp.xml" --thread all --top 15 | head -40
