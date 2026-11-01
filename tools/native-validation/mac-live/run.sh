#!/usr/bin/env bash
# Independent, repeatable native acceptance runner for the FlashTeX Mac shell.
#
#   1. builds the Rust helpers (flashtex-bridge, flashtex-edit-ledger) from the
#      CURRENT integrated main (default origin/main) — a shared clone pinned
#      (detached) at that exact commit — and records SHAs + sha256;
#   2. builds the engine host (flashtex-host, crates/flashtex-engine) and
#      flashtex-project-files from the branch under test, each in its own
#      pinned clone (the host is the app's engine: it is packaged with the
#      branch's string pool), and the Mac app (release) from a pinned clone of
#      the branch under test (default origin/agent/mac-claude-a/mac-shell);
#   3. packages FlashTeX.app with apps/mac/scripts/make-app.sh
#      (--require-engine-host) and runs apps/mac/scripts/launch-check.sh (engine
#      host + bridge child kill, app survival) with FLASHTEX_NO_ACTIVATE=1
#      through the `open` shim in lib/;
#   4. drives the packaged app's bundled helpers through the capture cycle
#      (submit -> convert refused as provider_disabled -> offline proposal
#      review -> durable ledger apply); the Accessibility Help and Nearby
#      windows opened headlessly and captured by window id; and the branch's
#      own XCTests for the save routing / reviewed reload / conflict refusal
#      paths that need menus or sheets, run against the real project-files
#      helper;
#   5. writes reports/<UTC>.md with every number, hash, SHA, machine/OS/Xcode
#      version and exact command, asserted against thresholds.json.
#
# The old engine's routes (its typing bench, render attach, exact export,
# worker relaunch and multi-file helper-route cycles) left with that engine in
# retirement stage S6 (#1236); their evidence stays under reports/.
#
# Usage: tools/native-validation/mac-live/run.sh [--branch <ref>] [--main-ref <ref>]
#          [--no-fetch] [--skip-launch] [--skip-cycle]
#          [--skip-tests] [--skip-features] [--rebuild] [--force]
#          [--work <dir>] [--session <url-or-id>] [--agent <id>]
# Env:   FLASHTEX_MAC_LIVE_SESSION  provenance: the driving agent session (URL/id)
#        FLASHTEX_MAC_LIVE_AGENT    provenance: the driving agent id
# Exit:  0 when every gate passed, 1 otherwise (the report is written either way).
# Never activates the app window; refuses launch-check while another
# FlashTeX.app is running (launch-check.sh would pkill it) unless --force.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
LIB="$SCRIPT_DIR/lib"
BRANCH="origin/agent/mac-claude-a/mac-shell"
MAIN_REF="origin/main"
DO_FETCH=1
SKIP_LAUNCH=0
SKIP_CYCLE=0
SKIP_TESTS=0
SKIP_FEATURES=0
REBUILD=0
FORCE=0
WORK="$SCRIPT_DIR/build"
SESSION="${FLASHTEX_MAC_LIVE_SESSION:-unknown}"
AGENT="${FLASHTEX_MAC_LIVE_AGENT:-unknown}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --branch) BRANCH="$2"; shift 2 ;;
    --main-ref) MAIN_REF="$2"; shift 2 ;;
    --no-fetch) DO_FETCH=0; shift ;;
    --skip-launch) SKIP_LAUNCH=1; shift ;;
    --skip-cycle) SKIP_CYCLE=1; shift ;;
    --skip-tests) SKIP_TESTS=1; shift ;;
    --skip-features) SKIP_FEATURES=1; shift ;;
    --rebuild) REBUILD=1; shift ;;
    --force) FORCE=1; shift ;;
    --work) WORK="$2"; shift 2 ;;
    --session) SESSION="$2"; shift 2 ;;
    --agent) AGENT="$2"; shift 2 ;;
    -h|--help) sed -n '2,39p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "run.sh: unknown argument $1" >&2; exit 2 ;;
  esac
done

UTC="$(date -u +%Y%m%dT%H%M%SZ)"
REPORTS="$SCRIPT_DIR/reports"
RUN_DIR="$REPORTS/$UTC"
REPORT="$REPORTS/$UTC.md"
LOGS="$RUN_DIR/logs"
mkdir -p "$RUN_DIR" "$LOGS" "$WORK"
COMMANDS="$RUN_DIR/commands.log"
: > "$COMMANDS"

step() { echo "==> $*"; }
note() { echo "    $*"; }

# cmd <step-name> <command...>: logs the exact command, runs it with stdout+stderr
# captured to logs/<step>.log (also echoed), records exit code and seconds in
# steps.jsonl. Never aborts the runner; callers inspect $CMD_STATUS.
CMD_STATUS=0
cmd() {
  local name="$1"; shift
  local log="$LOGS/$name.log" start end
  printf '[%s] (cwd %s) %q' "$name" "$PWD" "$1" >> "$COMMANDS"
  local a; for a in "${@:2}"; do printf ' %q' "$a" >> "$COMMANDS"; done
  printf '\n' >> "$COMMANDS"
  start="$(date +%s)"
  "$@" > >(tee -a "$log") 2>&1
  CMD_STATUS=$?
  end="$(date +%s)"
  printf '{"step":"%s","exit":%d,"seconds":%d,"log":"logs/%s.log"}\n' "$name" "$CMD_STATUS" "$((end - start))" "$name" >> "$RUN_DIR/steps.jsonl"
  return 0
}

sha256() { shasum -a 256 "$1" | awk '{print $1}'; }

# ---------------------------------------------------------------- 0. provenance
step "provenance"
if [[ $DO_FETCH == 1 ]]; then cmd fetch git -C "$ROOT" fetch origin --prune --quiet; fi
MAIN_SHA="$(git -C "$ROOT" rev-parse --verify "$MAIN_REF^{commit}" 2>/dev/null)" || { echo "run.sh: cannot resolve $MAIN_REF" >&2; exit 2; }
BRANCH_SHA="$(git -C "$ROOT" rev-parse --verify "$BRANCH^{commit}" 2>/dev/null)" || { echo "run.sh: cannot resolve $BRANCH" >&2; exit 2; }
CHECKOUT_SHA="$(git -C "$ROOT" rev-parse HEAD)"
CHECKOUT_BRANCH="$(git -C "$ROOT" rev-parse --abbrev-ref HEAD)"
CHECKOUT_DIRTY="$(git -C "$ROOT" status --porcelain --untracked-files=no | wc -l | tr -d ' ')"
MAIN_IN_BRANCH="$(git -C "$ROOT" merge-base --is-ancestor "$MAIN_SHA" "$BRANCH_SHA" && echo true || echo false)"
MERGE_BASE="$(git -C "$ROOT" merge-base "$MAIN_SHA" "$BRANCH_SHA")"
note "main   $MAIN_REF = $MAIN_SHA"
note "branch $BRANCH = $BRANCH_SHA (contains main: $MAIN_IN_BRANCH; merge-base $MERGE_BASE)"
python3 - "$RUN_DIR/env.json" "$ROOT" "$SCRIPT_DIR" "$MAIN_REF" "$MAIN_SHA" "$BRANCH" "$BRANCH_SHA" "$CHECKOUT_SHA" "$CHECKOUT_BRANCH" "$CHECKOUT_DIRTY" "$MAIN_IN_BRANCH" "$MERGE_BASE" "$SESSION" "$AGENT" "$UTC" <<'PY'
import json, os, subprocess, sys, hashlib, platform, glob
(out, root, here, main_ref, main_sha, branch, branch_sha, co_sha, co_branch, co_dirty, main_in_branch, merge_base, session, agent, utc) = sys.argv[1:16]
def sh(*a):
    try: return subprocess.run(a, text=True, capture_output=True, timeout=60).stdout.strip()
    except Exception as e: return "unavailable (%s)" % e
def sha(p):
    h = hashlib.sha256(); h.update(open(p, "rb").read()); return h.hexdigest()
tool_files = sorted(glob.glob(os.path.join(here, "run.sh")) + glob.glob(os.path.join(here, "thresholds.json")) + glob.glob(os.path.join(here, "lib", "*.py")) + glob.glob(os.path.join(here, "lib", "open-shim", "open")) + glob.glob(os.path.join(here, "fixtures", "*")))
env = {
  "utc": utc, "session": session, "agent": agent, "user": sh("id", "-un"), "host": sh("scutil", "--get", "LocalHostName"),
  "machine": {"hardware": sh("sysctl", "-n", "machdep.cpu.brand_string"), "arch": platform.machine(),
               "memory_bytes": sh("sysctl", "-n", "hw.memsize"), "cores": sh("sysctl", "-n", "hw.ncpu"),
               "os": sh("sw_vers", "-productVersion"), "os_build": sh("sw_vers", "-buildVersion"), "kernel": platform.release(),
               "xcode": " ".join(sh("xcodebuild", "-version").split()), "swift": sh("swift", "--version").splitlines()[0] if sh("swift", "--version") else "",
               "cargo": sh("cargo", "--version"), "rustc": sh("rustc", "--version"), "python3": sys.version.split()[0],
               "load_average_at_start": sh("sysctl", "-n", "vm.loadavg"), "uptime": sh("uptime"),
               "other_flashtex_processes_at_start": sh("pgrep", "-l", "-x", "FlashTeX|FlashTeXMac").replace("\n", "; ")},
  "sources": {"main_ref": main_ref, "main_sha": main_sha, "branch": branch, "branch_sha": branch_sha,
               "branch_contains_main": main_in_branch == "true", "merge_base": merge_base,
               "main_subject": sh("git", "-C", root, "log", "-1", "--format=%s", main_sha),
               "branch_subject": sh("git", "-C", root, "log", "-1", "--format=%s", branch_sha),
               "main_commit_utc": sh("git", "-C", root, "log", "-1", "--format=%cI", main_sha),
               "branch_commit_utc": sh("git", "-C", root, "log", "-1", "--format=%cI", branch_sha)},
  "runner": {"checkout": root, "checkout_sha": co_sha, "checkout_branch": co_branch, "checkout_dirty_tracked_files": int(co_dirty),
              "files": {os.path.relpath(f, here): sha(f) for f in tool_files}},
}
json.dump(env, open(out, "w"), indent=1, sort_keys=True)
PY

# --------------------------------------------------------------- 1. helpers
step "helpers from $MAIN_REF ($MAIN_SHA)"
HELPERS="$WORK/helpers/$MAIN_SHA"
HELPER_SRC="$HELPERS/src"
CRATES=(bridge edit-ledger)
# The built helpers live where cargo would put them inside the pinned scratch
# clone, so make-app.sh's git lookup next to each binary resolves the real SHA.
helper_path() { echo "$HELPER_SRC/crates/$1/target/release/flashtex-$1"; }
HELPERS_OK=1
if [[ $REBUILD == 1 || ! -f "$HELPERS/.complete" ]]; then
  rm -rf "$HELPERS"; mkdir -p "$HELPERS"
  # A shared, detached clone pinned at the exact commit: a clean tree with no
  # local edits, no build products, and a truthful `git rev-parse HEAD`.
  cmd helpers-clone git clone -q --shared --no-checkout "$ROOT" "$HELPER_SRC"
  [[ $CMD_STATUS == 0 ]] && cmd helpers-checkout git -C "$HELPER_SRC" checkout -q --detach "$MAIN_SHA"
  [[ $CMD_STATUS == 0 ]] || HELPERS_OK=0
  export CARGO_TARGET_DIR="$WORK/cargo-target"
  for c in "${CRATES[@]}"; do
    [[ $HELPERS_OK == 1 ]] || break
    manifest="$HELPER_SRC/crates/$c/Cargo.toml"
    cmd "helpers-build-$c" cargo build --release --offline --manifest-path "$manifest" --bin "flashtex-$c"
    if [[ $CMD_STATUS != 0 ]]; then
      note "offline build of crates/$c failed; retrying with the registry (crates.io only, no paid service)"
      cmd "helpers-build-$c-online" cargo build --release --manifest-path "$manifest" --bin "flashtex-$c"
    fi
    [[ $CMD_STATUS == 0 ]] || { HELPERS_OK=0; break; }
    mkdir -p "$(dirname "$(helper_path "$c")")"
    cp "$CARGO_TARGET_DIR/release/flashtex-$c" "$(helper_path "$c")"
  done
  unset CARGO_TARGET_DIR
  [[ $HELPERS_OK == 1 ]] && touch "$HELPERS/.complete"
else
  note "reusing helpers already built from $MAIN_SHA in $HELPER_SRC (--rebuild to force)"
  printf '[helpers] reused %s\n' "$HELPER_SRC" >> "$COMMANDS"
fi
HELPER_HEAD="$(git -C "$HELPER_SRC" rev-parse HEAD 2>/dev/null || echo unknown)"
# Anything cargo rewrote in the pinned tree (a stale committed Cargo.lock, for
# instance) is recorded verbatim: the tree must otherwise stay exactly main.
HELPER_DIRTY="$(git -C "$HELPER_SRC" status --porcelain 2>/dev/null)"
HELPER_CLEAN="$(printf '%s' "$HELPER_DIRTY" | grep -c . || true)"
python3 - "$RUN_DIR/helpers.json" "$HELPER_SRC" "$MAIN_REF" "$MAIN_SHA" "$HELPERS_OK" "$LIB" "$HELPER_HEAD" "$HELPER_CLEAN" "$HELPER_DIRTY" "${CRATES[@]}" <<'PY'
import json, os, sys
out, src, ref, sha, ok, lib, head, dirty, dirty_list = sys.argv[1:10]; crates = sys.argv[10:]
sys.path.insert(0, lib)
from hashes import describe
bins = {}
for c in crates:
    d = describe(os.path.join(src, "crates", c, "target", "release", "flashtex-" + c))
    d.update({"crate": "crates/" + c, "git_sha": sha})
    bins["flashtex-" + c] = d
json.dump({"ref": ref, "sha": sha, "built_ok": ok == "1", "scratch_clone": src, "scratch_head": head,
           "scratch_dirty_entries": int(dirty), "scratch_dirty_status": dirty_list.splitlines(), "binaries": bins}, open(out, "w"), indent=1, sort_keys=True)
PY
for c in "${CRATES[@]}"; do [[ -x "$(helper_path "$c")" ]] && note "flashtex-$c $(sha256 "$(helper_path "$c")")"; done
[[ "$HELPER_HEAD" == "$MAIN_SHA" ]] || { note "helpers scratch clone HEAD $HELPER_HEAD != $MAIN_SHA"; HELPERS_OK=0; }

# --------------------------------------- 1b. engine host and project files
# extra_build <name> <ref> <manifest-relative crate dir> <bin>: pinned shared
# clone at <ref>, cargo build --release of one bin, path in EXTRA_<NAME>.
EXTRAS_JSON="$RUN_DIR/extras.json"
echo "{}" > "$EXTRAS_JSON"
extra_build() {
  local name="$1" ref="$2" crate="$3" bin="$4" sha dir src bin_path ok=1 head
  sha="$(git -C "$ROOT" rev-parse --verify "$ref^{commit}" 2>/dev/null)" || { note "$name: cannot resolve $ref"; python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); d[sys.argv[2]]={"ref": sys.argv[3], "built_ok": False, "error": "unresolved ref"}; json.dump(d, open(sys.argv[1], "w"), indent=1)' "$EXTRAS_JSON" "$name" "$ref"; return; }
  dir="$WORK/$name/$sha"; src="$dir/src"; bin_path="$src/$crate/target/release/$bin"
  if [[ $REBUILD == 1 || ! -f "$dir/.complete" ]]; then
    rm -rf "$dir"; mkdir -p "$dir"
    cmd "$name-clone" git clone -q --shared --no-checkout "$ROOT" "$src"
    [[ $CMD_STATUS == 0 ]] && cmd "$name-checkout" git -C "$src" checkout -q --detach "$sha"
    [[ $CMD_STATUS == 0 ]] || ok=0
    if [[ $ok == 1 ]]; then
      export CARGO_TARGET_DIR="$WORK/cargo-target-$name"
      cmd "$name-build" cargo build --release --offline --manifest-path "$src/$crate/Cargo.toml" --bin "$bin"
      [[ $CMD_STATUS == 0 ]] || cmd "$name-build-online" cargo build --release --manifest-path "$src/$crate/Cargo.toml" --bin "$bin"
      [[ $CMD_STATUS == 0 ]] || ok=0
      if [[ $ok == 1 ]]; then mkdir -p "$(dirname "$bin_path")"; cp "$CARGO_TARGET_DIR/release/$bin" "$bin_path"; fi
      unset CARGO_TARGET_DIR
    fi
    [[ $ok == 1 ]] && touch "$dir/.complete"
  else
    note "reusing $bin already built from $sha"
    printf '[%s] reused %s\n' "$name" "$bin_path" >> "$COMMANDS"
  fi
  head="$(git -C "$src" rev-parse HEAD 2>/dev/null || echo unknown)"
  [[ -x "$bin_path" ]] || ok=0
  python3 - "$EXTRAS_JSON" "$name" "$ref" "$sha" "$crate" "$bin_path" "$ok" "$LIB" "$head" "$(git -C "$src" status --porcelain 2>/dev/null)" "$(git -C "$ROOT" log -1 --format=%s "$sha")" <<'PY'
import json, sys
out, name, ref, sha, crate, path, ok, lib, head, dirty, subject = sys.argv[1:12]
sys.path.insert(0, lib)
from hashes import describe
d = json.load(open(out))
e = describe(path); e.update({"ref": ref, "sha": sha, "crate": crate, "built_ok": ok == "1", "scratch_head": head, "scratch_dirty_status": dirty.splitlines(), "subject": subject})
d[name] = e
json.dump(d, open(out, "w"), indent=1, sort_keys=True)
PY
  [[ $ok == 1 ]] && note "$bin $(sha256 "$bin_path") ($ref @ $sha)"
  eval "EXTRA_$(echo "$name" | tr 'a-z-' 'A-Z_')=\"$bin_path\""
}
# The engine host from the branch under test: make-app.sh stages the string
# pool and licence from the app clone (crates/flashtex-engine), so the host is
# built from the same commit.
step "flashtex-host from $BRANCH"
EXTRA_ENGINE_HOST=""
extra_build engine-host "$BRANCH_SHA" crates/flashtex-engine flashtex-host
[[ -x "$EXTRA_ENGINE_HOST" ]] || EXTRA_ENGINE_HOST=""
# flashtex-project-files: the JSON Lines host lives in crates/project-files/src/bin
# on the app branch (not on main yet), so it is built from the branch SHA.
step "flashtex-project-files from $BRANCH"
EXTRA_PROJECT_FILES=""
extra_build project-files "$BRANCH_SHA" crates/project-files flashtex-project-files
[[ -x "$EXTRA_PROJECT_FILES" ]] || EXTRA_PROJECT_FILES=""

# ------------------------------------------------------------------- 2. app
step "app from $BRANCH ($BRANCH_SHA)"
APP_SRC="$WORK/app/$BRANCH_SHA"
MAC="$APP_SRC/apps/mac"
APP_OK=1
if [[ $REBUILD == 1 || ! -f "$APP_SRC/.extracted" ]]; then
  rm -rf "$APP_SRC"
  cmd app-clone git clone -q --shared --no-checkout "$ROOT" "$APP_SRC"
  [[ $CMD_STATUS == 0 ]] && cmd app-checkout git -C "$APP_SRC" checkout -q --detach "$BRANCH_SHA"
  [[ $CMD_STATUS == 0 ]] && echo "$BRANCH_SHA" > "$APP_SRC/.extracted" || APP_OK=0
else
  note "reusing sources already checked out at $BRANCH_SHA in $APP_SRC"
  printf '[app] reused %s\n' "$APP_SRC" >> "$COMMANDS"
fi
APP_HEAD="$(git -C "$APP_SRC" rev-parse HEAD 2>/dev/null || echo unknown)"
[[ "$APP_HEAD" == "$BRANCH_SHA" ]] || { note "app scratch clone HEAD $APP_HEAD != $BRANCH_SHA"; APP_OK=0; }
if [[ $APP_OK == 1 ]]; then
  cmd app-build swift build -c release --package-path "$MAC"
  [[ $CMD_STATUS == 0 ]] || APP_OK=0
fi
APP_BIN="$MAC/.build/release/FlashTeXMac"
if [[ $APP_OK == 1 && -x "$APP_BIN" ]]; then
  note "FlashTeXMac $(sha256 "$APP_BIN")"
  python3 -c 'import json,sys; sys.path.insert(0, sys.argv[5]); from hashes import describe; d=describe(sys.argv[2]); d.update({"branch":sys.argv[3],"sha":sys.argv[4],"built_ok":True,"binary":sys.argv[2]}); json.dump(d, open(sys.argv[1],"w"), indent=1, sort_keys=True)' "$RUN_DIR/app.json" "$APP_BIN" "$BRANCH" "$BRANCH_SHA" "$LIB"
else
  APP_OK=0
  python3 -c 'import json,sys; json.dump({"branch":sys.argv[2],"sha":sys.argv[3],"built_ok":False,"binary":None,"sha256":None}, open(sys.argv[1],"w"), indent=1)' "$RUN_DIR/app.json" "$BRANCH" "$BRANCH_SHA"
fi

# -------------------------------------------------------------- 3. package
BUNDLE="$MAC/build/FlashTeX.app"
BUNDLE_OK=0
if [[ $APP_OK == 1 && $HELPERS_OK == 1 && -n "$EXTRA_ENGINE_HOST" ]]; then
  step "make-app.sh (bundle with the bridge, the edit ledger${EXTRA_PROJECT_FILES:+, flashtex-project-files} and the engine host)"
  MAKE_APP_ARGS=(--bridge "$(helper_path bridge)" --ledger "$(helper_path edit-ledger)" --engine-host "$EXTRA_ENGINE_HOST" --require-engine-host)
  [[ -n "$EXTRA_PROJECT_FILES" ]] && MAKE_APP_ARGS+=(--project-files "$EXTRA_PROJECT_FILES")
  cmd make-app bash "$MAC/scripts/make-app.sh" "${MAKE_APP_ARGS[@]}"
  [[ $CMD_STATUS == 0 && -x "$BUNDLE/Contents/MacOS/FlashTeX" ]] && BUNDLE_OK=1
  python3 - "$RUN_DIR/bundle.json" "$BUNDLE" "$BUNDLE_OK" "$LIB" <<'PY'
import json, os, sys
out, bundle, ok, lib = sys.argv[1:5]
sys.path.insert(0, lib)
from hashes import describe
bins = {}
# Contents/MacOS (the app and its helpers) and Contents/Helpers (the engine hosts).
for sub in ("MacOS", "Helpers"):
    d = os.path.join(bundle, "Contents", sub)
    if os.path.isdir(d):
        for n in sorted(os.listdir(d)):
            p = os.path.join(d, n)
            if os.path.isfile(p):
                bins[n] = describe(p)
comp = os.path.join(bundle, "Contents", "Resources", "components.json")
components = json.load(open(comp)) if os.path.isfile(comp) else None
plist = os.path.join(bundle, "Contents", "Info.plist")
json.dump({"path": bundle, "built_ok": ok == "1", "binaries": bins, "components_json": components,
           "info_plist_present": os.path.isfile(plist)}, open(out, "w"), indent=1, sort_keys=True)
PY
else
  step "make-app.sh skipped (app_ok=$APP_OK helpers_ok=$HELPERS_OK engine_host=${EXTRA_ENGINE_HOST:-missing})"
fi

# --------------------------------------------------------- 4. launch check
if [[ $SKIP_LAUNCH == 0 && $BUNDLE_OK == 1 ]]; then
  step "launch-check.sh (FLASHTEX_NO_ACTIVATE=1 via lib/open-shim)"
  RUNNING="$(pgrep -x FlashTeX || true)"
  if [[ -n "$RUNNING" && $FORCE == 0 ]]; then
    note "REFUSED: FlashTeX.app already running (pid $RUNNING); launch-check.sh would pkill it. Use --force to override."
    printf '{"refused":true,"reason":"FlashTeX already running (pid %s)"}\n' "$RUNNING" > "$RUN_DIR/launch-check.json"
  else
    STORE="$RUN_DIR/launch-store"
    mkdir -p "$STORE"
    export FLASHTEX_MAC_LIVE_OPEN_ENV=$'FLASHTEX_NO_ACTIVATE=1\n'"FLASHTEX_BRIDGE_STORE=$STORE/captures"
    export FLASHTEX_MAC_LIVE_OPEN_LOG="$RUN_DIR/launch-open.log"
    : > "$FLASHTEX_MAC_LIVE_OPEN_LOG"
    chmod +x "$LIB/open-shim/open"
    printf '[launch-check] PATH=%q:$PATH FLASHTEX_MAC_LIVE_OPEN_ENV=%q\n' "$LIB/open-shim" "$FLASHTEX_MAC_LIVE_OPEN_ENV" >> "$COMMANDS"
    PATH_SAVED="$PATH"; export PATH="$LIB/open-shim:$PATH"
    cmd launch-check bash "$MAC/scripts/launch-check.sh" --app "$BUNDLE" --evidence "$RUN_DIR/launch-check.md"
    LC_EXIT=$CMD_STATUS
    export PATH="$PATH_SAVED"
    unset FLASHTEX_MAC_LIVE_OPEN_ENV FLASHTEX_MAC_LIVE_OPEN_LOG
    python3 "$LIB/launch_summary.py" --evidence "$RUN_DIR/launch-check.md" --open-log "$RUN_DIR/launch-open.log" \
        --exit "$LC_EXIT" --out "$RUN_DIR/launch-check.json"
    rm -rf "$STORE"
  fi
else
  step "launch-check skipped (skip=$SKIP_LAUNCH bundle_ok=$BUNDLE_OK)"
fi

# --------------------------------------------------------- 5. capture cycle
if [[ $SKIP_CYCLE == 0 && $BUNDLE_OK == 1 ]]; then
  step "capture cycle through the bundled helpers"
  cmd capture-cycle python3 "$LIB/capture_cycle.py" --app "$BUNDLE" --fixtures "$APP_SRC/protocol/fixtures" \
      --proposal "$SCRIPT_DIR/fixtures/capture-proposal.json" --work "$RUN_DIR/capture-cycle" --out "$RUN_DIR/capture-cycle.json"
  note "capture-cycle exit $CMD_STATUS"
else
  step "capture cycle skipped (skip=$SKIP_CYCLE bundle_ok=$BUNDLE_OK)"
fi

# ------------------------------------------------------------- 5b. windows
if [[ $SKIP_FEATURES == 0 && $BUNDLE_OK == 1 ]]; then
  RUNNING="$(pgrep -x FlashTeX || true)"
  if [[ -n "$RUNNING" && $FORCE == 0 ]]; then
    step "feature cycles REFUSED: FlashTeX.app already running (pid $RUNNING)"
    for f in open-window-a11y-help open-window-nearby; do
      printf '{"refused":true,"reason":"FlashTeX already running (pid %s)","checks":[],"passed":0,"failed":1}\n' "$RUNNING" > "$RUN_DIR/$f.json"
    done
  else
    WPROBE="$WORK/window_probe"
    if [[ ! -x "$WPROBE" || "$LIB/window_probe.swift" -nt "$WPROBE" ]]; then
      cmd window-probe-build swiftc -O "$LIB/window_probe.swift" -o "$WPROBE"
    fi
    [[ -x "$WPROBE" ]] || WPROBE=""
    step "secondary windows opened headlessly (FLASHTEX_OPEN_WINDOW) and captured by window id"
    cmd open-window-a11y-help python3 "$LIB/app_features.py" open-window --app "$BUNDLE" --window-id a11y-help --expect-title "Accessibility Help" \
        --probe "$WPROBE" --work "$RUN_DIR/open-window-a11y-help" --out "$RUN_DIR/open-window-a11y-help.json"
    cmd open-window-nearby python3 "$LIB/app_features.py" open-window --app "$BUNDLE" --window-id nearby --expect-title "Nearby Companion" \
        --probe "$WPROBE" --work "$RUN_DIR/open-window-nearby" --out "$RUN_DIR/open-window-nearby.json"
    for f in open-window-a11y-help open-window-nearby; do rm -rf "$RUN_DIR/$f/captures"; done
  fi
fi

# --------------------------------------------- 5c. branch XCTests, real helpers
# Save routing to a member (⌘S), reviewed reload after an external edit and
# the on-disk conflict refusal need menus/sheets that cannot be driven without
# Accessibility; the branch's own XCTests exercise exactly those
# ShellModel/ProjectDocuments/DocumentFiles paths, here against the real
# project-files helper built above (no fake for the helper cases).
if [[ $SKIP_TESTS == 0 && $APP_OK == 1 && $HELPERS_OK == 1 ]]; then
  step "swift test (ProjectDocumentsTests, DocumentFilesTests) with the real helpers"
  export FLASHTEX_BRIDGE="$(helper_path bridge)" FLASHTEX_EDIT_LEDGER="$(helper_path edit-ledger)"
  [[ -n "$EXTRA_PROJECT_FILES" ]] && export FLASHTEX_PROJECT_FILES="$EXTRA_PROJECT_FILES"
  printf '[app-tests] FLASHTEX_PROJECT_FILES=%q FLASHTEX_EDIT_LEDGER=%q FLASHTEX_BRIDGE=%q\n' \
      "${FLASHTEX_PROJECT_FILES:-}" "$FLASHTEX_EDIT_LEDGER" "$FLASHTEX_BRIDGE" >> "$COMMANDS"
  cmd app-tests swift test --package-path "$MAC" --filter 'ProjectDocumentsTests|DocumentFilesTests'
  python3 "$LIB/xctest_summary.py" --log "$LOGS/app-tests.log" --exit "$CMD_STATUS" --out "$RUN_DIR/app-tests.json"
  unset FLASHTEX_BRIDGE FLASHTEX_EDIT_LEDGER FLASHTEX_PROJECT_FILES
fi

# ---------------------------------------------------------------- 6. report
step "report"
printf '{"load_average_at_end":"%s","uptime_at_end":"%s","utc_end":"%s"}\n' "$(sysctl -n vm.loadavg)" "$(uptime)" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$RUN_DIR/end.json"
python3 "$LIB/report.py" --run-dir "$RUN_DIR" --thresholds "$SCRIPT_DIR/thresholds.json" --out "$REPORT"
STATUS=$?
note "report: $REPORT"
exit $STATUS
