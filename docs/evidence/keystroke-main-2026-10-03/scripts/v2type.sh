#!/bin/bash
# P5-KEYSTROKE-MAIN: one editor-only typing cell on the default v2 pane (TypingBench, FLASHTEX_TYPING_BENCH_NO_WORKER: no compiles, no preview frames) recorded under
# xctrace from launch, so the whole typing phase is in the trace.
# usage: v2type.sh <outdir> <doc.tex> [template] [keys] [ms]
#   template: "Time Profiler" (default) or "SwiftUI"
# Env: APP_ARGS (extra app arguments, e.g. "-FlashTeX.EditorPreferences.v1.lineWrapping YES"),
#      W (worktree with the release app + flashtex-host; default this checkout),
#      APP_BIN (overrides the app binary), EXTRA_ENV ("K=V K2=V2", passed to the app).
# Release app, never activated (FLASHTEX_NO_ACTIVATE=1); xctrace launches it,
# so its process ends with the bench (the app exits when the bench is done).
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
W="${W:-$(cd "$HERE/../../../.." && pwd)}"
OUT="$1"; DOC="$2"; TEMPLATE="${3:-Time Profiler}"; KEYS="${4:-200}"; MS="${5:-30}"
APP_BIN="${APP_BIN:-$W/apps/mac/.build/release/FlashTeXMac}"
rm -rf "$OUT"; mkdir -p "$OUT/doc" "$OUT/cache"
cp "$DOC" "$OUT/doc/main.tex"
{ echo "doc $DOC app_args ${APP_ARGS:-} extra_env ${EXTRA_ENV:-} keys $KEYS ms $MS template '$TEMPLATE' app $APP_BIN"; date; echo "load_before $(sysctl -n vm.loadavg)"; } > "$OUT/env.txt"
ENVS=(FLASHTEX_NO_ACTIVATE=1 FLASHTEX_KEYCHAIN_OFF=1 FLASHTEX_WINDOW_FRAME=40,40,1440,900 FLASHTEX_ENGINE_V3=0
  FLASHTEX_REPO="$W" FLASHTEX_AUTOATTACH=0 FLASHTEX_NO_FILE_WATCH=1 FLASHTEX_LOG="$OUT/app.log" FLASHTEX_SIGNPOSTS=1
  FLASHTEX_SEED_FILE="$OUT/doc/main.tex" FLASHTEX_TYPING_BENCH="$W/tools/typing-bench/typed-200.txt" FLASHTEX_TYPING_BENCH_MS="$MS"
  FLASHTEX_TYPING_BENCH_OUT="$OUT/bench.json" FLASHTEX_TYPING_BENCH_SETTLE_MS=1000 FLASHTEX_TYPING_BENCH_MAX_MS=120000
  FLASHTEX_TYPING_BENCH_AT=mid-paragraph FLASHTEX_TYPING_BENCH_NO_WORKER=1)
for kv in ${EXTRA_ENV:-}; do ENVS+=("$kv"); done
ARGS=()
for kv in "${ENVS[@]}"; do ARGS+=(--env "$kv"); done
# xctrace occasionally never launches the app: a watchdog ends it after 200 s.
( sleep 200; pkill -9 -f "xctrace record --template $TEMPLATE --output $OUT/trace.trace" ) &
WATCHDOG=$!
xcrun xctrace record --template "$TEMPLATE" --output "$OUT/trace.trace" --no-prompt --time-limit 120s \
  "${ARGS[@]}" --launch -- "$APP_BIN" \
  -"NSSplitView Subview Frames FlashTeX.workspace.split.editorPreview" '("0.000000, 0.000000, 380.000000, 850.000000, NO, NO", "381.000000, 0.000000, 710.000000, 850.000000, NO, NO")' \
  -"NSSplitView Subview Frames FlashTeX.workspace.split.problems" '("0.000000, 0.000000, 1091.000000, 850.000000, NO, NO", "0.000000, 730.000000, 1091.000000, 0.000000, YES, NO")' \
  ${APP_ARGS:-} > "$OUT/xctrace.txt" 2>&1
kill "$WATCHDOG" 2>/dev/null
echo "load_after $(sysctl -n vm.loadavg)" >> "$OUT/env.txt"
if [[ "$TEMPLATE" == "Time Profiler" ]]; then
  xcrun xctrace export --input "$OUT/trace.trace" --xpath '/trace-toc/run[@number="1"]/data/table[@schema="time-profile"]' > "$OUT/tp.xml" 2>/dev/null
  xcrun xctrace export --input "$OUT/trace.trace" --xpath '/trace-toc/run[@number="1"]/data/table[@schema="os-signpost" and @category="PointsOfInterest"]' > "$OUT/sp.xml" 2>/dev/null
fi
python3 - "$OUT/bench.json" <<'PY'
import json,sys
try: d=json.load(open(sys.argv[1]))
except Exception as e: print("no bench.json", e); sys.exit(0)
print(f"typed {d['typed']}")
PY
cat "$OUT/env.txt"
