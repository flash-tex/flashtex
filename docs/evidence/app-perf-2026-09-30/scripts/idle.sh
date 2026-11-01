#!/usr/bin/env bash
# APP-PERF-AUDIT: idle cost. Launch the app on a document (worker attached,
# file watching on), wait for it to settle, then measure CPU time and idle
# wake-ups of the app and its producer over a quiet window.
# usage: idle.sh <doc> <outdir> [window-seconds]
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOC="$1"; OUT="$2"; WIN="${3:-60}"
rm -rf "$OUT"
PID=$("$HERE/launch.sh" "/tmp/appperf/docs/$DOC.tex" "$OUT" FLASHTEX_NO_ACTIVATE=1 FLASHTEX_NO_FILE_WATCH=0)
until grep -q 'status: revision .*: \(ok\|recovered\|failed\)' "$OUT/app.log" 2>/dev/null; do sleep 1; done
sleep 15
cpu() { ps -o time= -p "$1" | awk -F'[:.]' '{ if (NF==3) print ($1*60+$2)*1000+$3*10; else print (($1*60+$2)*60+$3)*1000+$4*10 }'; }
WPID=$(pgrep -P "$PID" | head -1)
A0=$(cpu "$PID"); W0=$(cpu "$WPID")
top -l 1 -s 0 -pid "$PID" -stats pid,command,idlew,cpu > /dev/null
sleep "$WIN"
A1=$(cpu "$PID"); W1=$(cpu "$WPID")
TOP=$(top -l 2 -s 5 -pid "$PID" -pid "$WPID" -stats pid,command,cpu,idlew,power | tail -3)
kill "$PID"
{
  echo "doc $DOC window ${WIN}s load $(sysctl -n vm.loadavg)"
  echo "app cpu ms over window: $((A1 - A0))  producer cpu ms: $((W1 - W0))"
  echo "top (5 s sample):"; echo "$TOP"
} | tee "$OUT/idle.txt"
