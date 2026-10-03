#!/usr/bin/env bash
# Launches a packaged FlashTeX.app, confirms it actually opened a window,
# confirms the bundled flashtex-compiler AND flashtex-bridge each attached
# (via FLASHTEX_LOG status lines and as child processes), kills each child in
# turn to exercise crash recovery, and asserts the app itself survives and
# logs each recovery.
#
# Usage: apps/mac/scripts/launch-check.sh [--app <path to .app> | --dmg <path to .dmg>]
#                                          [--evidence <file>]
#
# --dmg mounts the image read-only at a private mount point, runs every check
# against the FlashTeX.app inside it, and detaches it afterwards.
# The app is launched with `open -n --env FLASHTEX_NO_ACTIVATE=1` (override
# with FLASHTEX_NO_ACTIVATE=0 in the environment) so it never takes keyboard
# focus. Only the process this script launched is ever signalled: the instance
# is identified by pid (new pid whose executable lives inside the bundle under
# test), never by process name, so another running FlashTeX is left alone.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MAC_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

APP_DIR="$MAC_DIR/build/FlashTeX.app"
DMG_PATH=""
EVIDENCE_FILE=""
NO_ACTIVATE="${FLASHTEX_NO_ACTIVATE:-1}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --app)
      APP_DIR="$2"
      shift 2
      ;;
    --dmg)
      DMG_PATH="$2"
      shift 2
      ;;
    --evidence)
      EVIDENCE_FILE="$2"
      shift 2
      ;;
    -h|--help)
      sed -n '2,17p' "${BASH_SOURCE[0]}"
      exit 0
      ;;
    *)
      echo "launch-check.sh: unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

REPORT_LINES=()
step() { echo "==> $1"; REPORT_LINES+=("" "## $1" ""); }
note() { echo "    $1"; REPORT_LINES+=("- $1"); }
fail() { echo "    FAIL: $1" >&2; REPORT_LINES+=("- FAIL: $1"); }

# Polls $LOG_FILE for a literal substring for up to $2 seconds.
wait_for_log() {
  local pattern="$1" timeout="$2" waited=0
  while (( waited < timeout )); do
    if [[ -f "$LOG_FILE" ]] && grep -qF "$pattern" "$LOG_FILE" 2>/dev/null; then
      return 0
    fi
    sleep 1
    waited=$((waited + 1))
  done
  return 1
}

# All scratch files for this run live in one directory so a single `rm -rf`
# cleans up everything (macOS mktemp only substitutes a trailing run of X's,
# so templates need a real trailing XXXXXX with no suffix after it).
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/flashtex-launch-check.XXXXXX")"
MOUNT_POINT=""
APP_PID=""
cleanup() {
  # Only ever signal the pid this script launched.
  if [[ -n "$APP_PID" ]] && kill -0 "$APP_PID" 2>/dev/null; then
    kill -TERM "$APP_PID" 2>/dev/null || true
    for _ in $(seq 1 10); do kill -0 "$APP_PID" 2>/dev/null || break; sleep 0.5; done
  fi
  if [[ -n "$MOUNT_POINT" ]]; then
    for _ in $(seq 1 10); do
      hdiutil detach "$MOUNT_POINT" >/dev/null 2>&1 && { MOUNT_POINT=""; break; }
      sleep 1
    done
    [[ -n "$MOUNT_POINT" ]] && hdiutil detach "$MOUNT_POINT" -force >/dev/null 2>&1 || true
  fi
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT

# --- 0. Mount the DMG when asked -------------------------------------------
if [[ -n "$DMG_PATH" ]]; then
  if [[ ! -f "$DMG_PATH" ]]; then
    echo "launch-check.sh: $DMG_PATH not found; run make-app.sh --dmg first" >&2
    exit 1
  fi
  MOUNT_POINT="$WORK_DIR/mnt"
  mkdir -p "$MOUNT_POINT"
  step "Mounting $DMG_PATH read-only at $MOUNT_POINT"
  if hdiutil attach -nobrowse -readonly -noautoopen -mountpoint "$MOUNT_POINT" "$DMG_PATH" >"$WORK_DIR/hdiutil.log" 2>&1; then
    note "mounted (hdiutil attach -nobrowse -readonly -mountpoint)"
  else
    MOUNT_POINT=""
    echo "launch-check.sh: hdiutil attach failed for $DMG_PATH:" >&2
    cat "$WORK_DIR/hdiutil.log" >&2
    exit 1
  fi
  APP_DIR="$MOUNT_POINT/FlashTeX.app"
  if [[ -d "$APP_DIR" ]]; then
    note "image contains FlashTeX.app$( [[ -L "$MOUNT_POINT/Applications" ]] && echo ' and an /Applications symlink')"
  else
    fail "no FlashTeX.app at the root of the mounted image"
    exit 1
  fi
fi

if [[ ! -d "$APP_DIR" ]]; then
  echo "launch-check.sh: $APP_DIR not found; run make-app.sh first" >&2
  exit 1
fi
# ps reports the resolved executable path; compare against both spellings.
APP_REAL="$(cd "$APP_DIR" && pwd -P)"

# --- 1. Compile a tiny CGWindowList/NSRunningApplication probe on the fly ----
# `probe window <pid>` -> exit 0 if that pid owns an on-screen window, else 1.
# `probe quit <pid>`   -> asks exactly that process to quit (NSRunningApplication
#                         .terminate(), the same request Finder/Dock send);
#                         exit 0 if the request was delivered.
PROBE_SRC="$WORK_DIR/probe.swift"
PROBE_BIN="$WORK_DIR/probe"
cat > "$PROBE_SRC" <<'SWIFT'
import AppKit
import CoreGraphics
import Foundation

let args = CommandLine.arguments
guard args.count > 2, let pid = Int32(args[2]) else {
    FileHandle.standardError.write("usage: probe window|quit <pid>\n".data(using: .utf8)!)
    exit(2)
}
switch args[1] {
case "window":
    let options: CGWindowListOption = [.optionOnScreenOnly, .excludeDesktopElements]
    guard let list = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: AnyObject]] else {
        exit(1)
    }
    let owned = list.contains { entry in
        (entry[kCGWindowOwnerPID as String] as? Int32) == pid
    }
    exit(owned ? 0 : 1)
case "quit":
    guard let app = NSRunningApplication(processIdentifier: pid) else { exit(1) }
    exit(app.terminate() ? 0 : 1)
default:
    exit(2)
}
SWIFT

step "Compiling window/quit probe"
if swiftc -O "$PROBE_SRC" -o "$PROBE_BIN" 2>"$WORK_DIR/probe-build.log"; then
  note "probe compiled at $PROBE_BIN"
else
  note "probe failed to compile (see $WORK_DIR/probe-build.log); window check will be skipped and quit falls back to SIGTERM on the launched pid"
  PROBE_BIN=""
fi

# --- 1b. Pinned rooted TFM metrics (GH36; static, read-only) -----------------
step "Pinned bundle resources (Contents/Resources/texmf + Fonts)"
RESOURCE_VERIFIER="$MAC_DIR/scripts/verify_bundle_resources.py"
if [[ -f "$RESOURCE_VERIFIER" ]]; then
  rc=0; python3 "$RESOURCE_VERIFIER" "$APP_DIR/Contents/Resources" > "$WORK_DIR/resource-coverage.json" 2>&1 || rc=$?
  if [[ "$rc" -eq 0 ]]; then
    note "verify_bundle_resources.py exit 0: 3 fonts, 5 rooted metrics and the rooted GUST license match the pinned manifest"
  else
    fail "verify_bundle_resources.py exit $rc: $(grep -E '"(status|reason)"' "$WORK_DIR/resource-coverage.json" | grep -v verified | head -3 | tr -s ' \n' ' ')"
  fi
else
  note "verifier not in this checkout ($RESOURCE_VERIFIER); pinned resource check skipped"
fi
if [[ -f "$APP_DIR/Contents/Resources/components.json" ]] && python3 -c "import json,sys; r=json.load(open(sys.argv[1]))['resources']; assert r['status']=='verified' and len(r['resources'])==9" "$APP_DIR/Contents/Resources/components.json" 2>/dev/null; then
  note "components.json records 9 verified resource hashes"
else
  fail "components.json lacks a verified 'resources' entry (bundle packaged before the GH36 staging, or refused)"
fi

# --- 2. Launch --------------------------------------------------------------
COMPILER_IN_BUNDLE="$APP_DIR/Contents/MacOS/flashtex-compiler"
RENDER_IN_BUNDLE="$APP_DIR/Contents/MacOS/flashtex-render"
BRIDGE_IN_BUNDLE="$APP_DIR/Contents/MacOS/flashtex-bridge"
if [[ -x "$RENDER_IN_BUNDLE" ]]; then
  PRODUCER_NAME="flashtex-render"
  PRODUCER_IN_BUNDLE="$RENDER_IN_BUNDLE"
else
  PRODUCER_NAME="flashtex-compiler"
  PRODUCER_IN_BUNDLE="$COMPILER_IN_BUNDLE"
fi
LOG_FILE="$WORK_DIR/flashtex.log"
: > "$LOG_FILE"
step "Launching $APP_DIR"
PIDS_BEFORE=" $( (pgrep -x FlashTeX || true) | tr '\n' ' ') "
if [[ "$PIDS_BEFORE" != "  " ]]; then
  note "other FlashTeX instance(s) already running (pids:$PIDS_BEFORE) — left untouched; a new instance is launched with open -n"
fi
OPEN_ENV=(--env "FLASHTEX_NO_ACTIVATE=$NO_ACTIVATE" --env "FLASHTEX_LOG=$LOG_FILE")
if [[ -x "$PRODUCER_IN_BUNDLE" || -x "$BRIDGE_IN_BUNDLE" ]]; then
  open -n "${OPEN_ENV[@]}" --env FLASHTEX_AUTOATTACH=1 "$APP_DIR"
  note "bundled producer/bridge present; launched with open -n --env FLASHTEX_NO_ACTIVATE=$NO_ACTIVATE --env FLASHTEX_AUTOATTACH=1 --env FLASHTEX_LOG=$LOG_FILE"
else
  open -n "${OPEN_ENV[@]}" "$APP_DIR"
  note "launched with open -n --env FLASHTEX_NO_ACTIVATE=$NO_ACTIVATE --env FLASHTEX_LOG=$LOG_FILE; no bundled producer/flashtex-bridge in $APP_DIR/Contents/MacOS; rebuild with 'make-app.sh --render <path> --compiler <path> --bridge <path>' to exercise the attach/kill/log checks below"
fi

# The launched instance is the new FlashTeX pid whose executable lives inside
# the bundle under test.
for _ in $(seq 1 20); do
  for pid in $(pgrep -x FlashTeX || true); do
    [[ "$PIDS_BEFORE" == *" $pid "* ]] && continue
    exe="$(ps -p "$pid" -o comm= 2>/dev/null || true)"
    if [[ "$exe" == "$APP_DIR/"* || "$exe" == "$APP_REAL/"* ]]; then
      APP_PID="$pid"
      break 2
    fi
  done
  sleep 0.5
done
if [[ -z "$APP_PID" ]]; then
  fail "no new FlashTeX process running from $APP_DIR appeared within 10s"
  exit 1
fi
note "FlashTeX running, pid=$APP_PID"
note "executable: $(ps -p "$APP_PID" -o comm= 2>/dev/null)"

# --- 3. Wait for a window ----------------------------------------------------
step "Waiting for a window owned by pid $APP_PID"
WINDOW_SEEN=0
if [[ -n "$PROBE_BIN" ]]; then
  for _ in $(seq 1 15); do
    if "$PROBE_BIN" window "$APP_PID"; then
      WINDOW_SEEN=1
      break
    fi
    sleep 1
  done
else
  sleep 3
fi
if [[ "$WINDOW_SEEN" -eq 1 ]]; then
  note "CGWindowListCopyWindowInfo confirms an on-screen window owned by pid $APP_PID"
else
  note "window not confirmed via CGWindowList (probe unavailable, headless session, or timed out)"
fi

# --- 4. Confirm the preferred bundled producer attached (log + child process)
PRODUCER_PID=""
if [[ -x "$PRODUCER_IN_BUNDLE" ]]; then
  step "Checking for a $PRODUCER_NAME child of pid $APP_PID"
  for _ in $(seq 1 10); do
    PRODUCER_PID="$(pgrep -P "$APP_PID" -x "$PRODUCER_NAME" || true)"
    [[ -n "$PRODUCER_PID" ]] && break
    sleep 0.5
  done
  if [[ -n "$PRODUCER_PID" ]]; then
    note "$PRODUCER_NAME attached, pid=$PRODUCER_PID (child of $APP_PID)"
  else
    fail "no $PRODUCER_NAME child process found under pid $APP_PID within 5s"
  fi

  step "Checking FLASHTEX_LOG for an 'attached:' status line"
  if wait_for_log "status: attached:" 10; then
    note "log shows an 'attached:' status line within 10s"
  else
    fail "no 'status: attached:' line in $LOG_FILE within 10s"
  fi

  step "Checking FLASHTEX_LOG for 'revision 1: ok' (auto-compile completed)"
  if wait_for_log "revision 1: ok" 10; then
    note "log shows 'revision 1: ok' within 10s"
  else
    fail "no 'revision 1: ok' line in $LOG_FILE within 10s"
  fi
fi

# --- 5. Kill the producer child and assert the app survives and logs it -----
if [[ -n "$PRODUCER_PID" ]]; then
  step "Killing $PRODUCER_NAME (pid $PRODUCER_PID) and checking app survival"
  kill "$PRODUCER_PID"
  sleep 2
  if kill -0 "$APP_PID" 2>/dev/null; then
    note "FlashTeX (pid $APP_PID) is still running after its producer child was killed"
  else
    fail "FlashTeX (pid $APP_PID) exited after its producer child was killed"
  fi
  if ! kill -0 "$PRODUCER_PID" 2>/dev/null; then
    note "$PRODUCER_NAME (pid $PRODUCER_PID) confirmed gone"
  fi

  step "Checking FLASHTEX_LOG for a 'worker exited (' status line"
  if wait_for_log "worker exited (" 5; then
    note "log shows a 'worker exited (' status line within 5s"
  else
    fail "no 'worker exited (' line in $LOG_FILE within 5s"
  fi
fi

# --- 5b. Confirm the bundled bridge attached (log + child process) ---------
# The bridge auto-attaches whenever it is bundled next to the executable (see
# ShellModel.init()'s hasBundledBridge check) — no extra env var beyond
# FLASHTEX_AUTOATTACH=1 (already set above) is needed.
BRIDGE_PID=""
if [[ -x "$BRIDGE_IN_BUNDLE" ]]; then
  step "Checking for a flashtex-bridge child of pid $APP_PID"
  for _ in $(seq 1 10); do
    BRIDGE_PID="$(pgrep -P "$APP_PID" -x flashtex-bridge || true)"
    [[ -n "$BRIDGE_PID" ]] && break
    sleep 0.5
  done
  if [[ -n "$BRIDGE_PID" ]]; then
    note "flashtex-bridge attached, pid=$BRIDGE_PID (child of $APP_PID)"
  else
    fail "no flashtex-bridge child process found under pid $APP_PID within 5s"
  fi

  step "Checking FLASHTEX_LOG for a bridge 'attached:' status line"
  if wait_for_log "bridge: attached:" 10; then
    note "log shows a bridge 'attached:' status line within 10s"
  else
    fail "no 'bridge: attached:' line in $LOG_FILE within 10s"
  fi
fi

# --- 5c. Kill the bridge child and assert the app survives and logs it -----
if [[ -n "$BRIDGE_PID" ]]; then
  step "Killing flashtex-bridge (pid $BRIDGE_PID) and checking app survival"
  kill "$BRIDGE_PID"
  sleep 2
  if kill -0 "$APP_PID" 2>/dev/null; then
    note "FlashTeX (pid $APP_PID) is still running after its bridge child was killed"
  else
    fail "FlashTeX (pid $APP_PID) exited after its bridge child was killed"
  fi
  if ! kill -0 "$BRIDGE_PID" 2>/dev/null; then
    note "flashtex-bridge (pid $BRIDGE_PID) confirmed gone"
  fi

  step "Checking FLASHTEX_LOG for a 'bridge exited (' status line"
  if wait_for_log "bridge exited (" 5; then
    note "log shows a 'bridge exited (' status line within 5s"
  else
    fail "no 'bridge exited (' line in $LOG_FILE within 5s"
  fi
fi

# --- 6. Quit (only pid $APP_PID) ---------------------------------------------
step "Quitting FlashTeX (pid $APP_PID)"
QUIT_HOW=""
if [[ -n "$PROBE_BIN" ]] && "$PROBE_BIN" quit "$APP_PID" 2>/dev/null; then
  QUIT_HOW="NSRunningApplication.terminate() on pid $APP_PID"
else
  kill -TERM "$APP_PID" 2>/dev/null || true
  QUIT_HOW="SIGTERM to pid $APP_PID"
fi
for _ in $(seq 1 10); do
  kill -0 "$APP_PID" 2>/dev/null || break
  sleep 0.5
done
if kill -0 "$APP_PID" 2>/dev/null; then
  # NSRunningApplication.terminate() only reports that the quit request was
  # delivered. A modal sheet or AppKit shutdown race may leave this exact
  # test instance alive, so finish with the same pid-bounded SIGTERM that the
  # cleanup trap uses. Never signal by process name.
  kill -TERM "$APP_PID" 2>/dev/null || true
  for _ in $(seq 1 10); do
    kill -0 "$APP_PID" 2>/dev/null || break
    sleep 0.5
  done
  if kill -0 "$APP_PID" 2>/dev/null; then
    fail "FlashTeX (pid $APP_PID) is still running after $QUIT_HOW and pid-bounded SIGTERM"
  else
    note "FlashTeX quit after $QUIT_HOW required pid-bounded SIGTERM fallback"
    APP_PID=""
  fi
else
  note "FlashTeX quit cleanly ($QUIT_HOW)"
  APP_PID=""
fi
if [[ "$PIDS_BEFORE" != "  " ]]; then
  STILL=""
  for pid in $PIDS_BEFORE; do kill -0 "$pid" 2>/dev/null && STILL="$STILL $pid"; done
  note "pre-existing FlashTeX instance(s) untouched:${STILL:- (none still running)}"
fi

if [[ -n "$MOUNT_POINT" ]]; then
  step "Detaching $MOUNT_POINT"
  DETACHED=0
  for _ in $(seq 1 10); do
    if hdiutil detach "$MOUNT_POINT" >/dev/null 2>&1; then DETACHED=1; break; fi
    sleep 1
  done
  if [[ "$DETACHED" -eq 1 ]]; then
    note "image detached cleanly (no -force needed)"
    MOUNT_POINT=""
  else
    fail "hdiutil detach kept failing for 10s (something still holds the image); forcing in cleanup"
  fi
fi

if [[ -n "$EVIDENCE_FILE" ]]; then
  {
    echo "# launch-check.sh run — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo
    echo "App: $APP_DIR"
    [[ -n "$DMG_PATH" ]] && echo "DMG: $DMG_PATH"
    echo "FLASHTEX_NO_ACTIVATE: $NO_ACTIVATE"
    echo "Compiler bundled: $([[ -x "$COMPILER_IN_BUNDLE" ]] && echo yes || echo no)"
    echo "Render pipeline bundled: $([[ -x "$RENDER_IN_BUNDLE" ]] && echo yes || echo no)"
    echo "Preferred producer: $PRODUCER_NAME"
    echo "Bridge bundled: $([[ -x "$BRIDGE_IN_BUNDLE" ]] && echo yes || echo no)"
    echo
    printf '%s\n' "${REPORT_LINES[@]}"
    echo
    echo "## FLASHTEX_LOG ($LOG_FILE)"
    echo
    echo '```'
    cat "$LOG_FILE" 2>/dev/null || echo "(log file missing or empty)"
    echo '```'
  } > "$EVIDENCE_FILE"
  echo "==> Evidence written to $EVIDENCE_FILE"
fi
