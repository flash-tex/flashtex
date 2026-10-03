#!/usr/bin/env bash
# APP-PERF-AUDIT: one typing-bench cell against the release FlashTeXMac, with an
# optional xctrace recording attached to the app for the whole run.
# usage: bench.sh <seed.tex> <outdir> [--ms 30] [--at first-paragraph] [--trace "Time Profiler"] [--script typed.txt]
# Env: APP_BIN (default apps/mac/.build/release/FlashTeXMac), RENDER (flashtex-render).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
SEED="$1"; OUT="$2"; shift 2
MS=30; AT=first-paragraph; TRACE=""; SCRIPT="$ROOT/tools/typing-bench/typed-200.txt"; EXTRA=()
while [[ $# -gt 0 ]]; do case "$1" in
  --ms) MS="$2"; shift 2;; --at) AT="$2"; shift 2;; --trace) TRACE="$2"; shift 2;;
  --script) SCRIPT="$2"; shift 2;; --env) EXTRA+=("$2"); shift 2;;
  *) echo "unknown $1" >&2; exit 1;; esac; done
APP_BIN="${APP_BIN:-$ROOT/apps/mac/.build/release/FlashTeXMac}"
RENDER="${RENDER:-$ROOT/target/release/flashtex-render}"
mkdir -p "$OUT"
cp "$SEED" "$OUT/seed.tex"
echo "load_before $(sysctl -n vm.loadavg)" > "$OUT/load.txt"
env FLASHTEX_REPO="$ROOT" FLASHTEX_AUTOATTACH=1 FLASHTEX_NO_ACTIVATE=1 FLASHTEX_NO_FILE_WATCH=1 \
  FLASHTEX_LM_DIR="$ROOT/apps/mac/Fonts" FLASHTEX_FONT_DIRS="$ROOT/apps/mac/Fonts" \
  FLASHTEX_COMPILER="$RENDER" FLASHTEX_SEED_FILE="$OUT/seed.tex" FLASHTEX_LOG="$OUT/app.log" \
  FLASHTEX_TYPING_BENCH="$SCRIPT" FLASHTEX_TYPING_BENCH_MS="$MS" FLASHTEX_TYPING_BENCH_OUT="$OUT/bench.json" \
  FLASHTEX_TYPING_BENCH_SETTLE_MS=60000 FLASHTEX_TYPING_BENCH_MAX_MS=120000 FLASHTEX_TYPING_BENCH_AT="$AT" \
  FLASHTEX_SIGNPOSTS=1 "${EXTRA[@]}" "$APP_BIN" > "$OUT/stdout.txt" 2>&1 &
PID=$!
if [[ -n "$TRACE" ]]; then
  xcrun xctrace record --template "$TRACE" --attach "$PID" --output "$OUT/trace.trace" --no-prompt > "$OUT/xctrace.txt" 2>&1 &
  XPID=$!
fi
wait "$PID" || true
if [[ -n "$TRACE" ]]; then kill -INT "$XPID" 2>/dev/null || true; wait "$XPID" || true; fi
echo "load_after $(sysctl -n vm.loadavg)" >> "$OUT/load.txt"
python3 - "$OUT/bench.json" <<'PY'
import json,sys
try: d=json.load(open(sys.argv[1]))
except Exception as e: print("no bench.json", e); sys.exit(0)
k=d["keystroke_to_paint_ms"]; c=d["compile_ms"]; r=d["render_pass_ms"]; rp=d["result_to_paint_ms"]
print(f"typed {d['typed']} painted {d['painted']} coalesced {d['coalesced']} k2p p50 {k['p50_ms']:.1f} p95 {k['p95_ms']:.1f} max {k['max_ms']:.1f} | compile p50 {c['p50_ms']} p95 {c['p95_ms']} | render p50 {r['p50_ms']} | result->paint p50 {rp['p50_ms']}")
PY
cat "$OUT/load.txt"
