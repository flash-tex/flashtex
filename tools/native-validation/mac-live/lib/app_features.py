#!/usr/bin/env python3
"""Packaged-app feature checks that need no UI scripting.

render-attach: launches FlashTeX.app's executable directly (never activated:
  FLASHTEX_NO_ACTIVATE=1) with FLASHTEX_AUTOATTACH=1 and
  FLASHTEX_COMPILER=<bundled flashtex-render>, exactly what File > Attach Render
  Pipeline resolves to, and waits for FLASHTEX_LOG to show
  `launched ... (preview face: latin-modern)` and `revision 1: ok`; also checks
  the flashtex-render child under the app pid, then terminates the app.

exact-export: runs the bundled `flashtex-pdf-exact from-v2 <fixture> --out <pdf>
  --font-dir <fonts>` on the checked-in display-list-v2 fixture and reads the
  result back through PDFKit (a probe compiled from lib/pdfkit_probe.swift):
  page count must equal the fixture's, and the extracted text must contain the
  fixture's words.

open-window: launches the bundle's executable with FLASHTEX_OPEN_WINDOW=<id>
  (a11y-help | nearby), lists the app's on-screen windows through a CGWindowList
  probe (lib/window_probe.swift), captures the matching window by id with
  `screencapture -x -l <id>` (no activation, no Accessibility) and records the
  PNG's size and dimensions.

worker-relaunch: launches the bundle with its compiler, SIGKILLs the compiler
  child repeatedly and asserts the shell's bounded auto-relaunch (relaunch +
  recompile within 5 s for the first three abnormal exits in a minute, then
  "worker relaunch limit reached" and no new child).

multifile: writes a temp project (main.tex with an \\input of chapter.tex),
  launches the bundle on the helper route with FLASHTEX_OPEN_INCLUDES=1 and
  FLASHTEX_ACTIVE_PATH=chapter.tex, types into the active editor with the
  typing bench, and asserts from FLASHTEX_LOG / the helper ledger / the disk
  that chapter.tex was opened through the helper, became the active document,
  received the edit durably, and that neither file on disk changed (no save).

Stdlib only; hashes via lib/hashes.py.
"""
import argparse
import json
import os
import re
import signal
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from hashes import describe  # noqa: E402


def wait_for(path, needles, timeout):
    """Returns {needle: seconds-until-seen or None}."""
    seen = {n: None for n in needles}
    t0 = time.monotonic()
    while time.monotonic() - t0 < timeout and any(v is None for v in seen.values()):
        try:
            text = open(path, encoding="utf-8", errors="replace").read()
        except OSError:
            text = ""
        for n in needles:
            if seen[n] is None and n in text:
                seen[n] = round(time.monotonic() - t0, 3)
        time.sleep(0.1)
    return seen


def render_attach(a):
    macos = os.path.join(a.app, "Contents", "MacOS")
    app_bin = os.path.join(macos, "FlashTeX")
    render = os.path.join(macos, "flashtex-render")
    os.makedirs(a.work, exist_ok=True)
    log = os.path.join(a.work, "flashtex.log")
    open(log, "w").close()
    env = dict(os.environ)
    env.update({"FLASHTEX_AUTOATTACH": "1", "FLASHTEX_NO_ACTIVATE": "1", "FLASHTEX_COMPILER": render,
                "FLASHTEX_LOG": log, "FLASHTEX_BRIDGE_STORE": os.path.join(a.work, "captures")})
    env.pop("FLASHTEX_PREVIEW_FACE", None)
    env.pop("FLASHTEX_PREVIEW_CONTROLLER", None)
    rec = {"app": app_bin, "render": describe(render), "env": {k: env[k] for k in ("FLASHTEX_AUTOATTACH", "FLASHTEX_NO_ACTIVATE", "FLASHTEX_COMPILER", "FLASHTEX_LOG")},
           "checks": []}
    if not os.path.isfile(render):
        rec["checks"].append({"name": "bundled flashtex-render present", "ok": False, "detail": render})
        return rec
    rec["checks"].append({"name": "bundled flashtex-render present", "ok": True, "detail": render})
    t0 = time.monotonic()
    p = subprocess.Popen([app_bin], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, cwd=macos)
    rec["pid"] = p.pid
    needles = ["(preview face: latin-modern)", "revision 1: ok", "status: attached:"]
    seen = wait_for(log, needles, a.timeout)
    rec["log_seen_after_s"] = seen
    child = subprocess.run(["pgrep", "-P", str(p.pid), "-x", "flashtex-render"], capture_output=True, text=True).stdout.split()
    rec["render_child_pids"] = child
    rec["app_alive_after_wait"] = p.poll() is None
    launched = [l for l in open(log, encoding="utf-8", errors="replace").read().splitlines() if "launched" in l or "revision 1" in l or "attached" in l][:12]
    rec["log_excerpt"] = launched
    rec["checks"].append({"name": "app launched flashtex-render with preview face latin-modern (FLASHTEX_LOG 'preview face: latin-modern')", "ok": seen["(preview face: latin-modern)"] is not None, "detail": "seen after %s s" % seen["(preview face: latin-modern)"]})
    rec["checks"].append({"name": "flashtex-render attached (status line)", "ok": seen["status: attached:"] is not None, "detail": "seen after %s s" % seen["status: attached:"]})
    rec["checks"].append({"name": "first compile through flashtex-render painted (revision 1: ok)", "ok": seen["revision 1: ok"] is not None, "detail": "seen after %s s" % seen["revision 1: ok"]})
    rec["checks"].append({"name": "flashtex-render child process under the app pid", "ok": bool(child), "detail": "pids %s" % child})
    rec["checks"].append({"name": "app still running after the wait (not activated)", "ok": p.poll() is None, "detail": "exit %s" % p.poll()})
    try:
        p.send_signal(signal.SIGTERM)
        p.wait(timeout=10)
    except Exception:
        p.kill()
    rec["app_exit"] = p.returncode
    rec["elapsed_s"] = round(time.monotonic() - t0, 3)
    return rec


def exact_export(a):
    macos = os.path.join(a.app, "Contents", "MacOS")
    tool = os.path.join(macos, "flashtex-pdf-exact")
    os.makedirs(a.work, exist_ok=True)
    out = os.path.join(a.work, "exact-export.pdf")
    rec = {"tool": describe(tool), "fixture": describe(a.fixture), "font_dir": a.font_dir, "checks": []}
    if not os.path.isfile(tool):
        rec["checks"].append({"name": "bundled flashtex-pdf-exact present", "ok": False, "detail": tool})
        return rec
    rec["checks"].append({"name": "bundled flashtex-pdf-exact present", "ok": True, "detail": tool})
    fixture = json.load(open(a.fixture))
    payload = fixture.get("payload", fixture)
    expected_pages = len(payload.get("pages", []))
    argv = [tool, "from-v2", a.fixture, "--out", out, "--font-dir", a.font_dir]
    rec["argv"] = argv
    t0 = time.monotonic()
    p = subprocess.run(argv, capture_output=True, text=True, timeout=120)
    rec["exit"] = p.returncode
    rec["stdout_tail"] = p.stdout[-800:]
    rec["stderr_tail"] = p.stderr[-800:]
    rec["ms"] = round((time.monotonic() - t0) * 1000, 3)
    rec["checks"].append({"name": "flashtex-pdf-exact from-v2 exit 0", "ok": p.returncode == 0, "detail": (p.stderr or p.stdout)[-200:]})
    rec["pdf"] = describe(out) if os.path.isfile(out) else None
    rec["checks"].append({"name": "PDF written", "ok": bool(rec["pdf"] and rec["pdf"]["bytes"]), "detail": out})
    probe = None
    if a.probe and os.path.isfile(out):
        pr = subprocess.run([a.probe, out], capture_output=True, text=True, timeout=60)
        try:
            probe = json.loads(pr.stdout.strip().splitlines()[-1])
        except Exception:
            probe = {"error": (pr.stderr or pr.stdout)[-300:]}
    rec["pdfkit"] = probe
    text = (probe or {}).get("text", "") or ""
    norm = re.sub(r"\s+", " ", text)
    rec["checks"].append({"name": "PDFKit page count == fixture pages (%d)" % expected_pages, "ok": bool(probe) and probe.get("pages") == expected_pages, "detail": (probe or {}).get("pages")})
    for word in a.expect_text:
        rec["checks"].append({"name": "PDFKit text contains %r" % word, "ok": word in norm, "detail": norm[:160]})
    rec["pdfkit_text"] = norm[:1000]
    return rec


def base_env(work, extra=None):
    env = dict(os.environ)
    for k in list(env):
        if k.startswith("FLASHTEX_") and k not in ("FLASHTEX_LM_DIR", "FLASHTEX_FONT_DIRS"):
            env.pop(k)
    # No FLASHTEX_TRANSCRIPT: it records every compile request in full (megabytes per run); FLASHTEX_LOG is the evidence.
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


def log_lines(path):
    try:
        return open(path, encoding="utf-8", errors="replace").read().splitlines()
    except OSError:
        return []


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


def worker_relaunch(a):
    """The shell's compile() is a no-op while the applied result matches the
    editor revision, so a relaunch alone never recompiles: the typing bench
    keeps editing during the kills, and 'recompiled' means a new
    `status: revision N: ok` line appeared after the relaunch."""
    os.makedirs(a.work, exist_ok=True)
    macos = os.path.join(a.app, "Contents", "MacOS")
    seed = os.path.join(a.app, "Contents", "Resources", "Samples", "demo.tex")
    script = os.path.join(a.work, "typed.txt")
    open(script, "w").write(("The quick brown fox jumps over the lazy dog. " * 40)[:1200])
    bench_out = os.path.join(a.work, "bench.json")
    extra = {"FLASHTEX_AUTOATTACH": "1", "FLASHTEX_TYPING_BENCH": script, "FLASHTEX_TYPING_BENCH_MS": "30", "FLASHTEX_TYPING_BENCH_OUT": bench_out,
             "FLASHTEX_TYPING_BENCH_SETTLE_MS": "4000", "FLASHTEX_TYPING_BENCH_MAX_MS": "90000"}
    if os.path.isfile(seed):
        extra["FLASHTEX_SEED_FILE"] = seed
    env = base_env(a.work, extra)
    open(env["FLASHTEX_LOG"], "w").close()
    log = env["FLASHTEX_LOG"]
    rec = {"compiler": describe(os.path.join(macos, "flashtex-compiler")), "seed": seed if os.path.isfile(seed) else None, "kills": [], "checks": []}
    p = launch(a.app, env)
    rec["pid"] = p.pid
    # With a seed file the first compile is revision 2 (the seed replaces the fixture project).
    seen = wait_for(log, [": ok, ", "bench: typing"], a.timeout)
    rec["checks"].append({"name": "bundled compiler attached, first compile ok, typing bench started editing", "ok": seen[": ok, "] is not None and seen["bench: typing"] is not None, "detail": json.dumps(seen)})

    def child():
        return subprocess.run(["pgrep", "-P", str(p.pid), "-x", "flashtex-compiler"], capture_output=True, text=True).stdout.split()

    def counts():
        lines = log_lines(log)
        return {"ok": sum(1 for l in lines if re.search(r"status: revision \d+: ok", l)), "relaunched": sum(1 for l in lines if "relaunched flashtex-compiler" in l),
                "attached": sum(1 for l in lines if "status: attached: flashtex-compiler" in l), "limit": sum(1 for l in lines if "worker relaunch limit reached" in l)}

    time.sleep(0.5)
    for attempt in range(1, 5):
        pids = child()
        k = {"attempt": attempt, "child_before": pids}
        if not pids:
            k["error"] = "no compiler child to kill"
            rec["kills"].append(k)
            rec["checks"].append({"name": "kill %d: a compiler child existed" % attempt, "ok": False, "detail": "none"})
            break
        before = counts()
        t0 = time.monotonic()
        os.kill(int(pids[0]), signal.SIGKILL)
        k["killed_pid"] = int(pids[0])
        if attempt <= 3:
            scheduled = relaunched = recompiled = None
            new_child = []
            while time.monotonic() - t0 < 5.0:
                c = counts()
                lines = log_lines(log)
                if scheduled is None and any("(attempt %d)" % attempt in l for l in lines):
                    scheduled = round(time.monotonic() - t0, 3)
                if relaunched is None and c["relaunched"] > before["relaunched"]:
                    relaunched = round(time.monotonic() - t0, 3)
                if recompiled is None and relaunched is not None and c["ok"] > before["ok"]:
                    recompiled = round(time.monotonic() - t0, 3)
                new_child = child()
                if scheduled is not None and relaunched is not None and recompiled is not None and new_child and new_child[0] != pids[0]:
                    break
                time.sleep(0.05)
            k.update({"scheduled_after_s": scheduled, "relaunched_after_s": relaunched, "recompiled_after_s": recompiled, "new_child": new_child, "elapsed_s": round(time.monotonic() - t0, 3), "app_alive": p.poll() is None})
            rec["checks"].append({"name": "kill %d (SIGKILL compiler child): relaunch scheduled, relaunched (new child pid), recompiled a later keystroke — all within 5 s, app alive" % attempt,
                                  "ok": scheduled is not None and relaunched is not None and recompiled is not None and bool(new_child) and new_child[0] != pids[0] and p.poll() is None,
                                  "detail": json.dumps({x: k[x] for x in ("scheduled_after_s", "relaunched_after_s", "recompiled_after_s", "new_child", "elapsed_s")})})
        else:
            limit = None
            while time.monotonic() - t0 < 5.0:
                if counts()["limit"] > before["limit"]:
                    limit = round(time.monotonic() - t0, 3)
                    break
                time.sleep(0.05)
            time.sleep(1.5)
            new_child = child()
            k.update({"limit_after_s": limit, "child_after": new_child, "app_alive": p.poll() is None})
            rec["checks"].append({"name": "kill 4 within the same minute: 'worker relaunch limit reached' logged, no relaunch, app alive",
                                  "ok": limit is not None and not new_child and p.poll() is None, "detail": json.dumps({x: k[x] for x in ("limit_after_s", "child_after", "app_alive")})})
        rec["kills"].append(k)
    # The bench keeps typing without a worker, settles, writes its summary and exits.
    try:
        p.wait(timeout=90)
        rec["app_exit"] = p.returncode
    except subprocess.TimeoutExpired:
        rec["app_exit"] = stop(p)
    bench = None
    try:
        bench = json.load(open(bench_out))
    except Exception:
        pass
    rec["bench"] = {x: bench.get(x) for x in ("keystrokes", "painted", "unpainted", "paints", "compiles", "producer")} if bench else None
    rec["checks"].append({"name": "app survived all four kills and the bench exited cleanly (unpainted keystrokes after the limit are expected)", "ok": rec.get("app_exit") == 0 and bench is not None, "detail": json.dumps(rec["bench"])})
    rec["log_counts"] = counts()
    rec["log_excerpt"] = [l for l in log_lines(log) if any(x in l for x in ("relaunch", "worker exited", "attached", "limit", "bench:"))][:50]
    return rec


def multifile(a):
    os.makedirs(a.work, exist_ok=True)
    project = os.path.join(a.work, "project")
    os.makedirs(project, exist_ok=True)
    main_tex = "\\documentclass{article}\n\\begin{document}\nMain document before the include.\n\\input{chapter}\nMain document after the include.\n\\end{document}\n"
    chapter_tex = "\\section{Chapter}\nChapter text before typing.\n"
    open(os.path.join(project, "main.tex"), "w").write(main_tex)
    open(os.path.join(project, "chapter.tex"), "w").write(chapter_tex)
    script = os.path.join(a.work, "typed.txt")
    typed = "TYPEDINTOCHAPTER"
    open(script, "w").write(typed)
    before = {n: describe(os.path.join(project, n))["sha256"] for n in ("main.tex", "chapter.tex")}
    ledger_root = os.path.join(a.work, "ledgers")
    bench_out = os.path.join(a.work, "bench.json")
    extra = {"FLASHTEX_AUTOATTACH": "1", "FLASHTEX_SEED_FILE": os.path.join(project, "main.tex"), "FLASHTEX_OPEN_INCLUDES": "1", "FLASHTEX_ACTIVE_PATH": "chapter.tex",
             "FLASHTEX_PREVIEW_CONTROLLER": a.controller, "FLASHTEX_COMPILER": a.compiler, "FLASHTEX_CONTROLLER_LEDGER_ROOT": ledger_root,
             "FLASHTEX_TYPING_BENCH": script, "FLASHTEX_TYPING_BENCH_MS": "30", "FLASHTEX_TYPING_BENCH_OUT": bench_out, "FLASHTEX_TYPING_BENCH_APPEND": "1",
             "FLASHTEX_TYPING_BENCH_SETTLE_MS": "20000", "FLASHTEX_TYPING_BENCH_MAX_MS": "60000"}
    if a.project_files:
        extra["FLASHTEX_PROJECT_FILES"] = a.project_files
    env = base_env(a.work, extra)
    open(env["FLASHTEX_LOG"], "w").close()
    rec = {"project": project, "typed": typed, "env": {k: v for k, v in extra.items() if k.startswith("FLASHTEX_")}, "checks": []}
    p = launch(a.app, env)
    rec["pid"] = p.pid
    try:
        p.wait(timeout=a.timeout)
    except subprocess.TimeoutExpired:
        rec["timeout"] = True
        stop(p)
    rec["app_exit"] = p.returncode
    lines = log_lines(env["FLASHTEX_LOG"])
    rec["log_excerpt"] = [l for l in lines if any(x in l for x in ("project:", "controller", "bench:", "launched", "durable"))][:60]
    bench = None
    try:
        bench = json.load(open(bench_out))
    except Exception:
        pass
    rec["bench"] = {k: bench.get(k) for k in ("keystrokes", "painted", "unpainted", "producer", "document_bytes_before", "document_bytes_after")} if bench else None
    rec["checks"].append({"name": "bench summary written (app typed and exited on its own)", "ok": bench is not None and not rec.get("timeout"), "detail": json.dumps(rec["bench"])})
    rec["checks"].append({"name": "chapter.tex opened through the helper (log 'project: opened chapter.tex')", "ok": any("project: opened chapter.tex" in l for l in lines), "detail": next((l for l in lines if "opened chapter.tex" in l), "absent")})
    rec["checks"].append({"name": "helper route attached (flashtex-preview-controller launched)", "ok": any("launched" in l and "flashtex-preview-controller" in l for l in lines), "detail": next((l for l in lines if "flashtex-preview-controller" in l and "launched" in l), "absent")[:200]})
    rec["checks"].append({"name": "every typed keystroke painted", "ok": bool(bench) and bench.get("unpainted") == 0 and bench.get("keystrokes") == len(typed), "detail": json.dumps(rec["bench"])})
    after = {n: describe(os.path.join(project, n))["sha256"] for n in ("main.tex", "chapter.tex")}
    rec["disk"] = {"before": before, "after": after}
    rec["checks"].append({"name": "main.tex on disk untouched", "ok": before["main.tex"] == after["main.tex"], "detail": after["main.tex"][:12]})
    rec["checks"].append({"name": "chapter.tex on disk untouched (no save was issued; the edit is durable in the helper's ledger only)", "ok": before["chapter.tex"] == after["chapter.tex"], "detail": after["chapter.tex"][:12]})
    # Durable ledger documents written by the helper: the typed text must be in chapter.tex's, not main.tex's.
    docs = {}
    for root, _, files in os.walk(ledger_root):
        for f in files:
            if f == "document.json":
                try:
                    d = json.load(open(os.path.join(root, f)))
                except Exception:
                    continue
                doc = d.get("document") or d
                path = doc.get("path") or os.path.basename(os.path.dirname(root))
                docs[path] = {"revision": doc.get("revision"), "text": doc.get("text", ""), "file": os.path.join(root, f)}
    rec["ledger_documents"] = {k: {"revision": v["revision"], "bytes": len(v["text"].encode("utf-8")), "contains_typed": typed in v["text"], "file": v["file"]} for k, v in docs.items()}
    chapter_doc = next((v for k, v in docs.items() if k.endswith("chapter.tex")), None)
    main_doc = next((v for k, v in docs.items() if k.endswith("main.tex")), None)
    rec["checks"].append({"name": "helper ledger: chapter.tex durable document contains the typed text", "ok": chapter_doc is not None and typed in chapter_doc["text"], "detail": json.dumps(rec["ledger_documents"].get(next((k for k in docs if k.endswith("chapter.tex")), ""), "no chapter.tex ledger document"))})
    rec["checks"].append({"name": "helper ledger: main.tex durable document does not contain the typed text", "ok": main_doc is not None and typed not in main_doc["text"], "detail": json.dumps(rec["ledger_documents"].get(next((k for k in docs if k.endswith("main.tex")), ""), "no main.tex ledger document"))})
    return rec


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("render-attach")
    r.add_argument("--app", required=True)
    r.add_argument("--work", required=True)
    r.add_argument("--out", required=True)
    r.add_argument("--timeout", type=float, default=45)
    e = sub.add_parser("exact-export")
    e.add_argument("--app", required=True)
    e.add_argument("--fixture", required=True)
    e.add_argument("--font-dir", required=True)
    e.add_argument("--probe", default="", help="compiled pdfkit_probe binary")
    e.add_argument("--work", required=True)
    e.add_argument("--out", required=True)
    e.add_argument("--expect-text", action="append", default=[])
    w = sub.add_parser("open-window")
    w.add_argument("--app", required=True)
    w.add_argument("--window-id", required=True)
    w.add_argument("--expect-title", required=True)
    w.add_argument("--probe", default="", help="compiled window_probe binary")
    w.add_argument("--work", required=True)
    w.add_argument("--out", required=True)
    w.add_argument("--timeout", type=float, default=20)
    k = sub.add_parser("worker-relaunch")
    k.add_argument("--app", required=True)
    k.add_argument("--work", required=True)
    k.add_argument("--out", required=True)
    k.add_argument("--timeout", type=float, default=45)
    m = sub.add_parser("multifile")
    m.add_argument("--app", required=True)
    m.add_argument("--controller", required=True)
    m.add_argument("--compiler", required=True)
    m.add_argument("--project-files", default="")
    m.add_argument("--work", required=True)
    m.add_argument("--out", required=True)
    m.add_argument("--timeout", type=float, default=120)
    a = ap.parse_args()
    for attr in ("app", "work", "out", "fixture", "font_dir", "probe", "controller", "compiler", "project_files", "proposal"):
        if getattr(a, attr, None):
            setattr(a, attr, os.path.abspath(getattr(a, attr)))
    rec = {"render-attach": render_attach, "exact-export": exact_export, "open-window": open_window, "worker-relaunch": worker_relaunch, "multifile": multifile}[a.cmd](a)
    rec["passed"] = sum(1 for c in rec["checks"] if c["ok"])
    rec["failed"] = sum(1 for c in rec["checks"] if not c["ok"])
    json.dump(rec, open(a.out, "w"), indent=1, sort_keys=True, default=str)
    for c in rec["checks"]:
        print("    %s %s -- %s" % ("PASS" if c["ok"] else "FAIL", c["name"], str(c["detail"])[:160]))
    print("%s: %d passed, %d failed" % (a.cmd, rec["passed"], rec["failed"]))
    return 0 if rec["failed"] == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
