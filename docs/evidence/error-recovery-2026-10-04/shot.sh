#!/bin/bash
# One window screenshot of the app on an ERROR-RECOVERY case (evidence only).
# usage: shot.sh APP WINID_TOOL CASE_ID OUT.png [v3=1|0]
# Opens the case's good document, types the error (FLASHTEX_V3_CAPTURE_EDIT under
# v3), waits, captures the window by id and terminates only the pid it launched.
# Env hooks only: the user's defaults are never written by this script.
set -euo pipefail
APP="$1"; WINID="$2"; CASE="$3"; OUT="$4"; V3="${5:-1}"
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
WORK="$(mktemp -d)"
python3 - "$HERE/corpus.json" "$CASE" "$WORK" <<'EOF'
import json, sys
c = json.load(open(sys.argv[1])); case = next(x for x in c["cases"] if x["id"] == sys.argv[2])
base = c["bases"][case["base"]]
open(sys.argv[3] + "/main.tex", "w").write(base)
e = case["edits"][0]
# the capture hook inserts after a needle: the edit's replacement minus its find
open(sys.argv[3] + "/edit", "w").write(e["find"] + "|" + e["replace"][len(e["find"]):].replace("\n", "\\n"))
EOF
export FLASHTEX_ENGINE_V3="$V3" FLASHTEX_NO_ACTIVATE=1 FLASHTEX_OPEN="$WORK/main.tex"
export FLASHTEX_HOST="$ROOT/target/release/flashtex-host" FLASHTEX_POOL="$ROOT/crates/flashtex-engine/pdftex.pool"
if [[ "$V3" == 1 ]]; then
  export FLASHTEX_V3_CAPTURE_OUT="$WORK/cap" FLASHTEX_V3_CAPTURE_PAGES=1 FLASHTEX_V3_CAPTURE_HOLD=30 FLASHTEX_V3_CAPTURE_SETTLE=2
  export FLASHTEX_V3_CAPTURE_EDIT="$(cat "$WORK/edit")"
fi
"$APP" > "$WORK/app.log" 2>&1 &
PID=$!
trap 'kill $PID 2>/dev/null || true; rm -rf "$WORK"' EXIT
for _ in $(seq 1 120); do
  sleep 1
  if [[ "$V3" == 1 ]] && grep -q "v3capture: page" "$WORK/app.log"; then break; fi
done
sleep 3
ID="$("$WINID" "$PID")"
screencapture -x -o -l "$ID" "$OUT"
echo "$OUT (window $ID, pid $PID)"
