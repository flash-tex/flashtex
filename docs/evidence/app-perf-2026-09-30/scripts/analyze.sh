#!/usr/bin/env bash
# APP-PERF-AUDIT: export a cell's trace.trace (time-profile, signposts, hangs)
# and write summary.txt next to it (bench, stage timeline, signposts, profile).
# usage: analyze.sh <celldir> [tp_summary options...]
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../../.." && pwd)"
C="$1"; shift
T="$C/trace.trace"
{
  echo "# $(basename "$C")"
  cat "$C/load.txt" 2>/dev/null
  python3 - "$C/bench.json" <<'PY'
import json,sys
try: d=json.load(open(sys.argv[1]))
except Exception as e: print("no bench.json", e); sys.exit(0)
def f(s,k):
    v=s.get(k); return "-" if v is None else f"{v:.1f}"
k=d["keystroke_to_paint_ms"]; c=d["compile_ms"]; r=d["render_pass_ms"]; rp=d["result_to_paint_ms"]
print(f"bench: typed {d['typed']} painted {d['painted']} coalesced {d['coalesced']} paints {d['paints']} | k2p p50 {f(k,'p50_ms')} p95 {f(k,'p95_ms')} max {f(k,'max_ms')} | compile p50 {f(c,'p50_ms')} p95 {f(c,'p95_ms')} | result->paint p50 {f(rp,'p50_ms')} p95 {f(rp,'p95_ms')}")
PY
  echo; python3 "$ROOT/tools/typing-bench/timeline.py" "$C/app.log" 2>&1 | tail -14
  if [[ -d "$T" ]]; then
    xcrun xctrace export --input "$T" --xpath '/trace-toc/run[@number="1"]/data/table[@schema="time-profile"]' > "$C/tp.xml" 2>/dev/null
    xcrun xctrace export --input "$T" --xpath '/trace-toc/run[@number="1"]/data/table[@schema="os-signpost" and @category="PointsOfInterest"]' > "$C/sp.xml" 2>/dev/null
    xcrun xctrace export --input "$T" --xpath '/trace-toc/run[@number="1"]/data/table[@schema="potential-hangs"]' > "$C/hangs.xml" 2>/dev/null
    echo; echo "## signposts"; python3 "$HERE/sp_summary.py" "$C/sp.xml"
    echo; echo "## hangs (>250 ms): $(grep -c '<row>' "$C/hangs.xml" 2>/dev/null || echo 0)"
    echo; python3 "$HERE/tp_summary.py" "$C/tp.xml" --top 30 "$@"
    echo; echo "## all threads"; python3 "$HERE/tp_summary.py" "$C/tp.xml" --thread all --top 30 "$@" | sed -n '1,33p'
  fi
} > "$C/summary.txt" 2>&1
sed -n '1,40p' "$C/summary.txt"
