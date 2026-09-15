#!/usr/bin/env bash
# Measures keystroke -> paint latency of the FlashTeX Mac shell by typing a
# fixed script into the real editor (TypingBench.swift, FLASHTEX_TYPING_BENCH)
# for three seed documents, two typing intervals, and every requested producer
# route:
#   compiler    direct runtime-v1 worker: main's flashtex-compiler
#   render      direct runtime-v1 worker: flashtex-render (render-pipeline branch)
#   controller  durable helper route: flashtex-preview-controller (origin/main
#               crates/preview-controller) owning the ledger + main's compiler
#   v2          the display-list-v2 pane (FLASHTEX_PREVIEW_V2=1) — measured only
#               once PreviewV2View paints live results through TypingBench;
#               until then the route is recorded as not measurable (stub)
# The 1-minute load average is recorded before/after every cell; cells above
# --load-limit are marked "load-affected" in the evidence (never a gate).
# Writes docs/evidence/typing-bench-<UTC>.md plus raw JSON next to it.
#
# Usage: tools/typing-bench/run.sh [--intervals "30 0"] [--seeds "demo body60k fixture"]
#                                  [--producers "compiler render controller v2"]
#                                  [--quiet-load 8] [--quiet-wait 900] [--load-limit 10]
#                                  [--no-render] [--out <evidence.md>] [--at <needle>]
#   --at first-paragraph   type at the end of the first paragraph after
#                          \begin{document} (a visible page) instead of before
#                          \end{document}; any other value is a literal needle
#                          typed after its first occurrence (FLASHTEX_TYPING_BENCH_AT)
#   seeds also accept hw1 (fixtures/real-world/hw1/HW1.tex)
# Env:   FLASHTEX_RENDER=<path>            use an already built flashtex-render
#        FLASHTEX_RENDER_REF=<git ref>     (default origin/agent/mac-render-pipeline/unified)
#        FLASHTEX_PREVIEW_CONTROLLER=<path> use an already built helper
#        FLASHTEX_CONTROLLER_REF=<git ref> (default origin/main; crates/ archived to a scratch dir)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MAC="$ROOT/apps/mac"
INTERVALS="30 0"
SEEDS="demo body60k fixture"
PRODUCERS="compiler render controller v2"
OUT=""
WANT_RENDER=1
QUIET_LOAD=8
QUIET_WAIT=900
LOAD_LIMIT=10
AT=""
UTC="$(date -u +%Y-%m-%dT%H%M%SZ)"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --intervals) INTERVALS="$2"; shift 2 ;;
    --seeds) SEEDS="$2"; shift 2 ;;
    --producers) PRODUCERS="$2"; shift 2 ;;
    --out) OUT="$2"; shift 2 ;;
    --quiet-load) QUIET_LOAD="$2"; shift 2 ;;
    --quiet-wait) QUIET_WAIT="$2"; shift 2 ;;
    --load-limit) LOAD_LIMIT="$2"; shift 2 ;;
    --no-render) WANT_RENDER=0; shift ;;
    --at) AT="$2"; shift 2 ;;
    -h|--help) sed -n '2,26p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "run.sh: unknown argument $1" >&2; exit 1 ;;
  esac
done

WORK="$MAC/build/typing-bench/$UTC"
mkdir -p "$WORK"
[[ -n "$OUT" ]] || OUT="$ROOT/docs/evidence/typing-bench-$UTC.md"
RAW_DIR="$(dirname "$OUT")/typing-bench-$UTC"
mkdir -p "$RAW_DIR"
NOTES_FILE="$WORK/producers.txt"
: > "$NOTES_FILE"
[[ -z "$AT" ]] || echo "typing position: after '$AT' (FLASHTEX_TYPING_BENCH_AT)" >> "$NOTES_FILE"

step() { echo "==> $*"; }
note() { echo "$*" >> "$NOTES_FILE"; }
load1() { sysctl -n vm.loadavg | awk '{print $2}'; }   # "{ 1.23 4.56 7.89 }" -> 1-minute average
wants() { [[ " $PRODUCERS " == *" $1 "* ]]; }

# --- 1. Build the app (release) ------------------------------------------
step "building FlashTeXMac (release)"
swift build -c release --package-path "$MAC" 2>&1 | tail -1
APP_BIN="$MAC/.build/release/FlashTeXMac"

# --- 2. Producers ---------------------------------------------------------
# Parallel arrays: name, kind (worker | controller | v2), executable.
declare -a P_NAMES=() P_KINDS=() P_PATHS=()
COMPILER="$ROOT/crates/compiler/target/release/flashtex-compiler"
if wants compiler || wants controller || wants v2; then
  if [[ ! -x "$COMPILER" ]]; then
    step "building flashtex-compiler (release)"
    cargo build --release --manifest-path "$ROOT/crates/compiler/Cargo.toml" 2>&1 | tail -1
  fi
fi
if wants compiler; then
  P_NAMES+=("compiler"); P_KINDS+=("worker"); P_PATHS+=("$COMPILER")
  note "compiler: crates/compiler @ $(git -C "$ROOT" rev-parse --short HEAD) (this checkout)"
fi

# Builds one crate from a git ref into a scratch archive; prints the binary path.
# archive_paths: what to `git archive` (a self-contained crate, or all of crates/).
build_from_ref() { # ref scratch manifest_subdir bin archive_paths...
  local ref="$1" scratch="$2" sub="$3" bin="$4"; shift 4
  git -C "$ROOT" rev-parse --verify -q "$ref" >/dev/null || { echo "    $ref not found (git fetch origin)" >&2; return 1; }
  mkdir -p "$scratch"
  git -C "$ROOT" archive "$ref" "$@" | tar -x -C "$scratch" || return 1
  cargo build --release --manifest-path "$scratch/$sub/Cargo.toml" --bin "$bin" 2>&1 | tail -1 >&2 || return 1
  local out="$scratch/$sub/target/release/$bin"
  [[ -x "$out" ]] && echo "$out"
}

if wants render && [[ $WANT_RENDER == 1 ]]; then
  RENDER="${FLASHTEX_RENDER:-$ROOT/crates/render-pipeline/target/release/flashtex-render}"
  RENDER_NOTE="crates/render-pipeline/target/release (this checkout)"
  if [[ ! -x "$RENDER" ]]; then
    REF="${FLASHTEX_RENDER_REF:-origin/agent/mac-render-pipeline/unified}"
    step "building flashtex-render from $REF"
    if RENDER="$(build_from_ref "$REF" "$MAC/build/typing-bench/render-pipeline" crates/render-pipeline flashtex-render crates/render-pipeline)"; then
      RENDER_NOTE="$REF @ $(git -C "$ROOT" rev-parse --short "$REF") (scratch build, vendored siblings)"
    else
      echo "    flashtex-render build FAILED; skipping that route" >&2
      RENDER=""; RENDER_NOTE="build failed from $REF"
    fi
  fi
  if [[ -n "$RENDER" && -x "$RENDER" ]]; then
    P_NAMES+=("render"); P_KINDS+=("worker"); P_PATHS+=("$RENDER"); note "render: $RENDER_NOTE"
  else
    note "skipped: render ($RENDER_NOTE)"
  fi
fi

if wants controller; then
  CTRL="${FLASHTEX_PREVIEW_CONTROLLER:-$ROOT/crates/preview-controller/target/release/flashtex-preview-controller}"
  CTRL_NOTE="crates/preview-controller/target/release (this checkout)"
  if [[ ! -x "$CTRL" ]]; then
    REF="${FLASHTEX_CONTROLLER_REF:-origin/main}"
    step "building flashtex-preview-controller from $REF (crates/ archived: path dependencies on siblings)"
    if CTRL="$(build_from_ref "$REF" "$MAC/build/typing-bench/main-crates" crates/preview-controller flashtex-preview-controller crates)"; then
      CTRL_NOTE="$REF @ $(git -C "$ROOT" rev-parse --short "$REF") (scratch build of crates/)"
    else
      echo "    flashtex-preview-controller build FAILED; skipping that route" >&2
      CTRL=""; CTRL_NOTE="build failed from $REF"
    fi
  fi
  if [[ -n "$CTRL" && -x "$CTRL" ]]; then
    P_NAMES+=("controller"); P_KINDS+=("controller"); P_PATHS+=("$CTRL")
    note "controller: $CTRL_NOTE, owning compiler $COMPILER"
  else
    note "skipped: controller ($CTRL_NOTE)"
  fi
fi

if wants v2; then
  # Stub until the preview-v2 lane lands display-list-v2 on the typing path: the
  # pane is measurable only when it reports its paints through TypingBench
  # (willRender/didDraw) for live results; today it draws display lists opened
  # from a file (PreviewV2View.swift), so no keystroke can reach it.
  if grep -q "TypingBench.shared" "$MAC/Sources/FlashTeXMac/PreviewV2View.swift" 2>/dev/null; then
    P_NAMES+=("v2"); P_KINDS+=("v2"); P_PATHS+=("$COMPILER")
    note "v2: display-list-v2 pane (FLASHTEX_PREVIEW_V2=1) with compiler $COMPILER"
  else
    echo "    v2 pane does not paint live results through TypingBench yet; recording the route as not measurable" >&2
    note "skipped: v2 (PreviewV2View draws display lists opened from a file and has no TypingBench paint hooks; the preview-v2 lane is still negotiating display-list-v2 — re-run with --producers v2 once it lands)"
  fi
fi

# --- 3. Seeds -------------------------------------------------------------
TYPED="$SCRIPT_DIR/typed-200.txt"
python3 - "$ROOT" "$WORK" <<'PY'
import json, sys, os
root, work = sys.argv[1], sys.argv[2]
demo = open(os.path.join(root, "apps/mac/Samples/demo.tex"), encoding="utf-8").read()
open(os.path.join(work, "demo.tex"), "w", encoding="utf-8").write(demo)
# 60 KB body: the demo's paragraphs repeated inside one document.
head, body = demo.split("\\begin{document}\n", 1)
body = body.rsplit("\\end{document}", 1)[0]
out = head + "\\begin{document}\n"
while len(out.encode("utf-8")) < 60 * 1024:
    out += body
out += "\\end{document}\n"
open(os.path.join(work, "body60k.tex"), "w", encoding="utf-8").write(out)
req = json.load(open(os.path.join(root, "protocol/fixtures/compile-request.json")))
docs = req["payload"]["documents"]
text = next(d["text"] for d in docs if d["path"] == req["payload"]["entry_path"])
open(os.path.join(work, "fixture.tex"), "w", encoding="utf-8").write(text)
hw1 = open(os.path.join(root, "fixtures/real-world/hw1/HW1.tex"), encoding="utf-8").read()
open(os.path.join(work, "hw1.tex"), "w", encoding="utf-8").write(hw1)
for n in ("demo", "body60k", "fixture", "hw1"):
    print("    seed %-8s %6d bytes" % (n, os.path.getsize(os.path.join(work, n + ".tex"))))
PY

# --- 4. Runs --------------------------------------------------------------
wait_quiet() { # blocks until the 1-minute load is below QUIET_LOAD and no other
  local waited=0 l other # FlashTeXMac (another bench/validation run) is alive, or QUIET_WAIT s passed
  while :; do
    l="$(load1)"; other="$( (pgrep -x FlashTeXMac || true) | wc -l | tr -d ' ')"
    if awk -v l="$l" -v q="$QUIET_LOAD" 'BEGIN { exit !(l < q) }' && (( other == 0 )); then return 0; fi
    if (( waited >= QUIET_WAIT )); then echo "    load $l / $other other FlashTeXMac after $QUIET_WAIT s; running anyway" >&2; return 0; fi
    (( waited == 0 )) && echo "    load $l (limit $QUIET_LOAD), $other other FlashTeXMac process(es); waiting for a quiet machine (up to $QUIET_WAIT s)"
    sleep 10; waited=$((waited + 10))
  done
}

run_one() { # route kind executable seed interval -> $RAW_DIR/<route>-<seed>-<interval>ms.json
  local route="$1" kind="$2" exe="$3" seed="$4" ms="$5"
  local name="$route-$seed-${ms}ms" log="$WORK/$route-$seed-${ms}ms.log" json="$RAW_DIR/$route-$seed-${ms}ms.json"
  rm -f "$log" "$json"
  wait_quiet
  local before after
  before="$(load1)"
  step "run $name (load $before)"
  # Each cell gets its own seed copy (the durable route writes next to the file)
  # and, for the controller, a fresh temporary ledger root.
  local cell="$WORK/$name"; mkdir -p "$cell"; cp "$WORK/$seed.tex" "$cell/$seed.tex"
  local -a extra=()
  case "$kind" in
    worker) extra=(FLASHTEX_COMPILER="$exe") ;;
    controller) extra=(FLASHTEX_COMPILER="$COMPILER" FLASHTEX_PREVIEW_CONTROLLER="$exe" FLASHTEX_CONTROLLER_LEDGER_ROOT="$cell/ledger") ;;
    v2) extra=(FLASHTEX_COMPILER="$exe" FLASHTEX_PREVIEW_V2=1) ;;
  esac
  (
    cd "$MAC"
    env FLASHTEX_REPO="$ROOT" FLASHTEX_AUTOATTACH=1 FLASHTEX_NO_ACTIVATE=1 \
      FLASHTEX_LM_DIR="$MAC/Fonts" FLASHTEX_FONT_DIRS="$MAC/Fonts" \
      FLASHTEX_SEED_FILE="$cell/$seed.tex" FLASHTEX_LOG="$log" \
      FLASHTEX_TYPING_BENCH="$TYPED" FLASHTEX_TYPING_BENCH_MS="$ms" FLASHTEX_TYPING_BENCH_OUT="$json" \
      FLASHTEX_TYPING_BENCH_SETTLE_MS=60000 FLASHTEX_TYPING_BENCH_MAX_MS=120000 \
      FLASHTEX_TYPING_BENCH_AT="$AT" \
      "${extra[@]}" "$APP_BIN" >/dev/null 2>&1 &
    pid=$!
    for _ in $(seq 1 600); do kill -0 "$pid" 2>/dev/null || break; sleep 0.5; done
    if kill -0 "$pid" 2>/dev/null; then echo "    timed out after 300 s; killing" >&2; kill "$pid" 2>/dev/null || true; fi
    wait "$pid" 2>/dev/null || true
  )
  after="$(load1)"
  if [[ -f "$json" ]]; then
    grep -F "bench: done" "$log" | sed 's/^/    /' || true
    python3 - "$json" "$route" "$before" "$after" "$LOAD_LIMIT" <<'PY'
import json, sys
path, route, before, after, limit = sys.argv[1:6]
d = json.load(open(path, encoding="utf-8"))
d["route"] = route
d["load_avg_before"] = float(before); d["load_avg_after"] = float(after); d["load_limit"] = float(limit)
d["load_affected"] = max(float(before), float(after)) > float(limit)
json.dump(d, open(path, "w", encoding="utf-8"), indent=2, sort_keys=True)
if d["load_affected"]:
    print("    load-affected: %s -> %s (limit %s)" % (before, after, limit))
PY
  else
    echo "    no summary written (see $log)" >&2
    grep -E "status:|bench:|controller" "$log" | tail -3 | sed 's/^/    /' || true
  fi
}

set +u # bash 3.2 treats an empty array as unset
for i in "${!P_NAMES[@]}"; do
  for seed in $SEEDS; do
    for ms in $INTERVALS; do
      run_one "${P_NAMES[$i]}" "${P_KINDS[$i]}" "${P_PATHS[$i]}" "$seed" "$ms"
    done
  done
done
set -u

# --- 5. Evidence document --------------------------------------------------
step "writing $OUT"
python3 "$SCRIPT_DIR/evidence.py" "$OUT" "$RAW_DIR" "$ROOT" "$UTC" "$TYPED" "$NOTES_FILE"
