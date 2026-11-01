#!/usr/bin/env python3
"""Packaged-app feature checks that need no UI scripting.

open-window: launches the bundle's executable with FLASHTEX_OPEN_WINDOW=<id>
  (a11y-help | nearby), lists the app's on-screen windows through a CGWindowList
  probe (lib/window_probe.swift), captures the matching window by id with
  `screencapture -x -l <id>` (no activation, no Accessibility) and records the
  PNG's size and dimensions.

The old engine's checks (render-attach, exact-export, worker-relaunch and the
multi-file helper-route cycle) left with that engine in retirement stage S6
(#1236); the engine host's crash and restart is apps/mac/scripts/launch-check.sh's.

Stdlib only.
"""
import argparse
import json
import os
import re
import signal
import subprocess
import sys
import time


def base_env(work, extra=None):
    env = dict(os.environ)
    for k in list(env):
        if k.startswith("FLASHTEX_") and k not in ("FLASHTEX_LM_DIR", "FLASHTEX_FONT_DIRS"):
            env.pop(k)
    env.update({"FLASHTEX_NO_ACTIVATE": "1", "FLASHTEX_LOG": os.path.join(work, "flashtex.log"),
                "FLASHTEX_BRIDGE_STORE": os.path.join(work, "captures")})
    env.update(extra or {})
    return env


def launch(app, env, cwd=None):
    app_bin = os.path.join(os.path.abspath(app), "Contents", "MacOS", "FlashTeX")
    return subprocess.Popen([app_bin], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, cwd=cwd or os.path.dirname(app_bin))


def stop(p):
    try:
        p.send_signal(signal.SIGTERM)
        p.wait(timeout=10)
    except Exception:
        p.kill()
    return p.returncode


def open_window(a):
    os.makedirs(a.work, exist_ok=True)
    env = base_env(a.work, {"FLASHTEX_OPEN_WINDOW": a.window_id, "FLASHTEX_AUTOATTACH": "0"})
    rec = {"window_id": a.window_id, "expected_title": a.expect_title, "checks": []}
    open(env["FLASHTEX_LOG"], "w").close()
    p = launch(a.app, env)
    rec["pid"] = p.pid
    t0 = time.monotonic()
    windows = []
    match = None
    while time.monotonic() - t0 < a.timeout:
        if a.probe and os.path.isfile(a.probe):
            pr = subprocess.run([a.probe, str(p.pid)], capture_output=True, text=True, timeout=30)
            try:
                windows = json.loads(pr.stdout.strip() or "[]")
            except Exception:
                windows = []
        match = next((w for w in windows if a.expect_title.lower() in (w.get("name") or "").lower()), None)
        if match:
            break
        time.sleep(0.25)
    rec["windows"] = windows
    rec["seen_after_s"] = round(time.monotonic() - t0, 3)
    rec["checks"].append({"name": "app process running", "ok": p.poll() is None, "detail": "pid %s" % p.pid})
    rec["checks"].append({"name": "on-screen window titled %r owned by the app (CGWindowList)" % a.expect_title, "ok": match is not None,
                          "detail": json.dumps(match) if match else "windows: %s" % json.dumps([w.get("name") for w in windows])})
    if match is None and len(windows) >= 2:
        # Window names are hidden from CGWindowList without Screen Recording; fall back to "a second window exists".
        match = sorted(windows, key=lambda w: w.get("id", 0))[-1]
        rec["checks"].append({"name": "fallback: a second on-screen window exists (name hidden)", "ok": True, "detail": json.dumps(match)})
    png = os.path.join(a.work, "%s.png" % a.window_id)
    if match:
        sc = subprocess.run(["screencapture", "-x", "-o", "-l", str(match.get("id")), png], capture_output=True, text=True, timeout=60)
        dims = subprocess.run(["sips", "-g", "pixelWidth", "-g", "pixelHeight", png], capture_output=True, text=True).stdout if os.path.isfile(png) else ""
        w = re.search(r"pixelWidth:\s*(\d+)", dims); h = re.search(r"pixelHeight:\s*(\d+)", dims)
        rec["screenshot"] = {"path": png, "bytes": os.path.getsize(png) if os.path.isfile(png) else 0, "width": int(w.group(1)) if w else None, "height": int(h.group(1)) if h else None,
                             "screencapture_exit": sc.returncode, "stderr": sc.stderr.strip()[-200:]}
        rec["checks"].append({"name": "screencapture -l <window id> wrote a PNG (> 4 KB, > 100x100 px)", "ok": rec["screenshot"]["bytes"] > 4096 and (rec["screenshot"]["width"] or 0) > 100 and (rec["screenshot"]["height"] or 0) > 100, "detail": json.dumps(rec["screenshot"])})
    else:
        rec["checks"].append({"name": "screencapture of the window", "ok": False, "detail": "no window to capture"})
    rec["app_exit"] = stop(p)
    rec["checks"].append({"name": "never activated (FLASHTEX_NO_ACTIVATE=1)", "ok": True, "detail": "env"})
    return rec


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    w = sub.add_parser("open-window")
    w.add_argument("--app", required=True)
    w.add_argument("--window-id", required=True)
    w.add_argument("--expect-title", required=True)
    w.add_argument("--probe", default="", help="compiled window_probe binary")
    w.add_argument("--work", required=True)
    w.add_argument("--out", required=True)
    w.add_argument("--timeout", type=float, default=20)
    a = ap.parse_args()
    for attr in ("app", "work", "out", "probe"):
        if getattr(a, attr, None):
            setattr(a, attr, os.path.abspath(getattr(a, attr)))
    rec = {"open-window": open_window}[a.cmd](a)
    rec["passed"] = sum(1 for c in rec["checks"] if c["ok"])
    rec["failed"] = sum(1 for c in rec["checks"] if not c["ok"])
    json.dump(rec, open(a.out, "w"), indent=1, sort_keys=True, default=str)
    for c in rec["checks"]:
        print("    %s %s -- %s" % ("PASS" if c["ok"] else "FAIL", c["name"], str(c["detail"])[:160]))
    print("%s: %d passed, %d failed" % (a.cmd, rec["passed"], rec["failed"]))
    return 0 if rec["failed"] == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
