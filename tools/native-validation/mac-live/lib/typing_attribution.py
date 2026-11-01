#!/usr/bin/env python3
"""Typing latency on the PACKAGED app, attributed per stage.

Runs the shell's own typing bench (TypingBench.swift, FLASHTEX_TYPING_BENCH)
by executing `FlashTeX.app/Contents/MacOS/FlashTeX` directly — the packaged
binary with its bundled helpers, never `open`, always FLASHTEX_NO_ACTIVATE=1 —
for a set of seed documents on three producer routes:

  v1          direct worker route, bundled flashtex-compiler (PreviewView)
  v1-render   direct worker route, bundled flashtex-render painting v1
  v2          direct worker route, bundled flashtex-render with
              FLASHTEX_PREVIEW_V2=1 (PreviewV2View paints the display-list-v2
              sibling line)
  controller  durable helper route: bundled flashtex-preview-controller owning
              the ledger and launching the bundled flashtex-compiler

and then attributes every painted revision's keystroke -> paint time to the
stages the bench timeline in FLASHTEX_LOG makes visible (the log lines are
gated by TypingBench.isBenchActive, so they exist only during a bench run):

  key->send        keystroke stamp -> "compile: sending revision r"
                   (editor apply + waiting behind the in-flight compile)
  producer         "sending" -> "worker: line … decoded" of that request's
                   last line (child process wall: compile + serialise + pipe)
                   minus the reader's decode time, reported separately
  decode           the reader thread's JSON line decode ("decoded in X ms";
                   summed over the request's lines: v2 sends two)
  ->main           last decoded -> "worker: event on main" (main-queue hop)
  apply            "event on main" -> "compile: applied revision r"
                   (result validation + model apply on the main thread)
  helper roundtrip helper route only: "sending" -> "applied" (the
                   PreviewControllerClient logs no decode/main-hop lines, so
                   the helper's compiler, transport, decode, hop and apply
                   are one stage; "durable: rN" -> applied when logged)
  v2 prepare       "preview-v2: preparing" -> "prepared" (off-main fast
                   reader + validate + prepare), "prerastered" and
                   "delivered" (queue -> main) from the same line
  paint            applied (or v2 published) -> "paint: revision r"
                   (SwiftUI render pass, Canvas/bitmap draw, extra run-loop
                   turns)

Producer CPU is not in the logs; the child processes' cumulative CPU time
(`ps -o cputime`) is sampled twice a second while the cell runs and the last
sample before the app exits is divided by the number of results decoded.
Every cell records `uptime` before and after and waits for a quiet machine
(1-minute load < --quiet-load, up to --quiet-wait s); cells whose load before
or after exceeds --load-limit are marked load-affected, never hidden.

Usage: typing_attribution.py --app <FlashTeX.app> --repo <root> --out <dir>
                             [--seeds "fixture demo hw1 render27 body60k"]
                             [--routes "v1 v1-render v2 controller"]
                             [--interval 30] [--quiet-load 8] [--quiet-wait 600]
                             [--load-limit 10] [--settle-ms 60000]
"""
import argparse
import json
import os
import re
import shutil
import statistics
import subprocess
import sys
import time

NS = 1e6  # ns per ms


def load1():
    out = subprocess.run(["sysctl", "-n", "vm.loadavg"], capture_output=True, text=True).stdout
    return float(out.split()[1])


def uptime():
    return subprocess.run(["uptime"], capture_output=True, text=True).stdout.strip()


def sha256(path):
    import hashlib
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


# ------------------------------------------------------------------ seeds
RENDER_PARA = ("The quick brown fox jumps over the lazy dog while the patient owl watches from an old oak "
               "branch and counts every leaf that falls into the quiet river below. A \\textbf{bold} word, an \\emph{emphasised} "
               "one, the UTF-8 word caf\\'e, ``quotes'' --- and inline math $x_{i}^{2} + \\frac{a}{b} = \\sqrt{z}$ inside the "
               "sentence; then more text so that the paragraph wraps onto several lines of the page.")


def render_document(sections):
    """The render-pipeline lane's 27-page incremental fixture
    (crates/render-pipeline/tests/incremental.rs `document(40)` on
    origin/agent/mac-render-pipeline/unified 9aaec57a), reproduced verbatim."""
    s = "\\begin{document}\n"
    for i in range(sections):
        s += "\\section{Part %d}\\label{s%d}\n" % (i, i)
        for j in range(4):
            s += "Paragraph %d.%d of part \\ref{s%d} on page \\pageref{s%d}. %s\n\n" % (i, j, i, i, RENDER_PARA)
        s += "\\[\n\\sum_{k=0}^{n} k^{2} = \\frac{n(n+1)(2n+1)}{6}\n\\]\nAfter the display the paragraph goes on.\n\n"
        if i % 3 == 2:
            s += "\\newpage\n"
    s += "\\end{document}\n"
    return s


def make_seeds(root, work, names):
    demo = open(os.path.join(root, "apps/mac/Samples/demo.tex"), encoding="utf-8").read()
    head, body = demo.split("\\begin{document}\n", 1)
    body = body.rsplit("\\end{document}", 1)[0]
    seeds = {}
    for n in names:
        if n == "demo":
            text = demo
        elif n == "fixture":
            req = json.load(open(os.path.join(root, "protocol/fixtures/compile-request.json")))
            docs = req["payload"]["documents"]
            text = next(d["text"] for d in docs if d["path"] == req["payload"]["entry_path"])
        elif n == "hw1":
            text = open(os.path.join(root, "fixtures/real-world/hw1/HW1.tex"), encoding="utf-8").read()
        elif n == "render27":
            text = render_document(40)
        elif n == "body60k":
            out = head + "\\begin{document}\n"
            while len(out.encode("utf-8")) < 60 * 1024:
                out += body
            text = out + "\\end{document}\n"
        else:
            raise SystemExit("unknown seed %s" % n)
        path = os.path.join(work, n + ".tex")
        open(path, "w", encoding="utf-8").write(text)
        seeds[n] = {"path": path, "bytes": len(text.encode("utf-8")), "sha256": sha256(path)}
    return seeds


# ------------------------------------------------------------ log parsing
RE_TS = re.compile(r"^(?:\[[^\]]*\]\s*)?(?:(\d+(?:\.\d+)?)\s+)?(.*)$")
RE_KEY = re.compile(r"keystroke: revision (\d+) at (\d+)")
RE_SEND = re.compile(r"compile: sending revision (\d+) at (\d+)")
RE_DECODED = re.compile(r"worker: line (\d+) B decoded in ([\d.]+) ms at (\d+)")
RE_MAIN = re.compile(r"worker: event on main at (\d+)")
RE_APPLIED = re.compile(r"compile: applied revision (\d+) at (\d+)")
RE_DURABLE = re.compile(r"durable: r(\d+) for revision (\d+) at (\d+)")
RE_PREPARING = re.compile(r"preview-v2: preparing (.+?) \(revision (\d+)\) ticket (\d+) at (\d+)")
RE_PREPARED = re.compile(r"preview-v2: prepared (.+?) \(revision (\d+)\) in ([\d.]+) ms, prerastered (\d+) page\(s\) in ([\d.]+) ms, delivered ([\d.]+) ms later")
RE_PUBLISHED = re.compile(r"preview-v2: published (.+?) \(revision (\d+)\) revision (\d+) at (\d+)")
RE_COALESCED = re.compile(r"preview-v2: coalesced (.+?) \(revision (\d+)\) behind")
RE_PAINT = re.compile(r"paint: revision (\d+) at (\d+) \(covers (\d+) keystrokes, redrawn (\w+)\)")
RE_STATUS = re.compile(r"status: revision (\d+): ok, (\d+) diagnostics in (\d+) ms")
# The shell's engine label (EngineChoice.logLine; retirement plan #1236, S3r)
RE_ENGINE = re.compile(r"engine: (new|previous) \(([^)]*)\)")


def engine_of(lines):
    """The engine the last `engine:` line names, with its reason, or None
    (a log from an app before S3r, which logged no engine)."""
    last = None
    for line in lines:
        m = RE_ENGINE.search(line)
        if m:
            last = {"engine": m.group(1), "why": m.group(2)}
    return last


def attribute(log_path):
    """Per painted revision: stage durations in ms (None when a stage did not
    appear for that revision). Decoded lines carry no revision, so they are
    matched to the consumer event that follows them on the main thread: the
    oldest pending decoded line belongs to the next "compile: applied revision
    r" (v1 result line) or the next "preview-v2: preparing/coalesced (revision
    r)" (v2 sibling line). The shell sends the next request as soon as the v1
    result is applied, before the sibling is decoded, so send order alone
    would misattribute the sibling."""
    keys, sends, applied, durable, paints = {}, {}, {}, {}, {}
    preparing, prepared, published = {}, {}, {}
    lines = {}     # revision -> {"v1": [bytes, decode_ms, decoded_ns, main_ns], "v2": ...}
    pending = []   # decoded lines not yet claimed
    coalesced_v2 = 0

    def claim(r, kind):
        if pending:
            lines.setdefault(r, {})[kind] = pending.pop(0)

    for raw in open(log_path, encoding="utf-8", errors="replace"):
        line = raw.rstrip("\n")
        m = RE_KEY.search(line)
        if m:
            keys[int(m.group(1))] = int(m.group(2)); continue
        m = RE_SEND.search(line)
        if m:
            sends[int(m.group(1))] = int(m.group(2)); continue
        m = RE_DECODED.search(line)
        if m:
            pending.append([int(m.group(1)), float(m.group(2)), int(m.group(3)), None]); continue
        m = RE_MAIN.search(line)
        if m:
            for p in pending:
                if p[3] is None:
                    p[3] = int(m.group(1)); break
            continue
        m = RE_APPLIED.search(line)
        if m:
            r = int(m.group(1)); applied[r] = int(m.group(2)); claim(r, "v1"); continue
        m = RE_DURABLE.search(line)
        if m:
            durable[int(m.group(2))] = int(m.group(3)); continue
        m = RE_PREPARING.search(line)
        if m:
            r = int(m.group(2)); preparing[r] = int(m.group(4)); claim(r, "v2"); continue
        m = RE_PREPARED.search(line)
        if m:
            prepared[int(m.group(2))] = (float(m.group(3)), int(m.group(4)), float(m.group(5)), float(m.group(6))); continue
        m = RE_PUBLISHED.search(line)
        if m:
            published[int(m.group(3))] = int(m.group(4)); continue
        m = RE_COALESCED.search(line)
        if m:
            coalesced_v2 += 1; claim(int(m.group(2)), "v2"); continue
        m = RE_PAINT.search(line)
        if m:
            paints[int(m.group(1))] = (int(m.group(2)), int(m.group(3)), m.group(4) == "true"); continue
    rows = []
    for r, (paint_ns, covered, redrawn) in sorted(paints.items()):
        key_ns = keys.get(r)
        row = {"revision": r, "covers": covered, "redrawn": redrawn}
        if key_ns is None:
            rows.append(row); continue
        row["key_to_paint_ms"] = (paint_ns - key_ns) / NS
        send_ns = sends.get(r)
        if send_ns is not None:
            row["key_to_send_ms"] = (send_ns - key_ns) / NS
        got = lines.get(r, {})
        if send_ns is not None and got:
            last = got.get("v2") or got.get("v1")
            decode_ms = sum(v[1] for v in got.values())
            row["lines"] = len(got); row["line_bytes"] = [v[0] for v in got.values()]
            row["producer_ms"] = (last[2] - send_ns) / NS - decode_ms
            row["decode_ms"] = decode_ms
            if last[3] is not None:
                row["to_main_ms"] = (last[3] - last[2]) / NS
            v1 = got.get("v1")
            if v1 and v1[3] is not None and r in applied:
                row["apply_ms"] = (applied[r] - v1[3]) / NS
        elif send_ns is not None and r in applied:
            # helper route: PreviewControllerClient logs no decode/main-hop
            # lines, so the helper's whole answer (its compiler + transport +
            # decode + main hop + apply) is one stage
            row["helper_roundtrip_ms"] = (applied[r] - send_ns) / NS
            if r in durable:
                row["durable_to_apply_ms"] = (applied[r] - durable[r]) / NS
        end = applied.get(r)
        if r in preparing:
            v2 = got.get("v2")
            base = v2[3] if v2 and v2[3] is not None else applied.get(r)
            if base is not None:
                row["v2_wait_ms"] = (preparing[r] - base) / NS
            if r in prepared:
                p = prepared[r]
                row["v2_prepare_ms"], row["v2_pages"], row["v2_preraster_ms"], row["v2_deliver_ms"] = p
            if r in published:
                row["v2_publish_ms"] = (published[r] - preparing[r]) / NS - sum(prepared.get(r, (0, 0, 0, 0))[i] for i in (0, 2, 3))
                end = published[r]
        if end is not None:
            row["paint_ms"] = (paint_ns - end) / NS
        rows.append(row)
    return rows, {"keystrokes_logged": len(keys), "sends": len(sends), "paints": len(paints), "applied": len(applied),
                  "durable": len(durable), "v2_coalesced": coalesced_v2, "requests_with_lines": len(lines), "unclaimed_lines": len(pending)}


STAGES = ["key_to_send_ms", "producer_ms", "decode_ms", "to_main_ms", "apply_ms", "helper_roundtrip_ms", "durable_to_apply_ms",
          "v2_wait_ms", "v2_prepare_ms", "v2_preraster_ms", "v2_deliver_ms", "v2_publish_ms", "paint_ms", "key_to_paint_ms"]


def stats(values):
    v = sorted(x for x in values if x is not None)
    if not v:
        return None
    q = lambda p: v[min(len(v) - 1, int(round(p * (len(v) - 1))))]
    return {"n": len(v), "p50": round(statistics.median(v), 2), "p95": round(q(0.95), 2), "max": round(v[-1], 2), "mean": round(sum(v) / len(v), 2)}


# ------------------------------------------------------------------ cells
def ps_children(pid):
    out = subprocess.run(["ps", "-axo", "pid=,ppid=,cputime=,rss=,comm="], capture_output=True, text=True).stdout
    rows = {}
    for line in out.splitlines():
        parts = line.split(None, 4)
        if len(parts) < 5:
            continue
        p, pp, cpu, rss, comm = parts
        rows[int(p)] = (int(pp), cpu, int(rss), comm)
    # own row plus direct children and grandchildren (the helper launches the compiler)
    fam = {pid}
    changed = True
    while changed:
        changed = False
        for p, (pp, _, _, _) in rows.items():
            if pp in fam and p not in fam:
                fam.add(p); changed = True
    return {p: {"ppid": rows[p][0], "cputime": rows[p][1], "rss_kb": rows[p][2], "comm": os.path.basename(rows[p][3])} for p in fam if p in rows}


def cputime_s(s):
    # "MM:SS.ss" or "HH:MM:SS"
    parts = s.split(":")
    total = 0.0
    for p in parts:
        total = total * 60 + float(p)
    return total


def wait_quiet(quiet_load, quiet_wait):
    waited = 0
    while True:
        l = load1()
        others = subprocess.run(["pgrep", "-x", "FlashTeXMac"], capture_output=True, text=True).stdout.split()
        if l < quiet_load and not others:
            return l, waited
        if waited >= quiet_wait:
            print("    load %.2f / %d other FlashTeXMac after %d s; running anyway" % (l, len(others), waited), flush=True)
            return l, waited
        if waited == 0:
            print("    load %.2f (limit %s), %d other FlashTeXMac; waiting up to %d s" % (l, quiet_load, len(others), quiet_wait), flush=True)
        time.sleep(10); waited += 10


def run_cell(a, bundle, route, seed, seeds, out_dir, typed):
    name = "%s-%s-%dms" % (route, seed, a.interval)
    cell = os.path.join(a.work, name)
    shutil.rmtree(cell, ignore_errors=True); os.makedirs(cell)
    seed_path = os.path.join(cell, seed + ".tex")
    shutil.copy(seeds[seed]["path"], seed_path)
    log = os.path.join(out_dir, name + ".log")
    summary = os.path.join(out_dir, name + ".json")
    for f in (log, summary):
        if os.path.exists(f):
            os.remove(f)
    macos = os.path.join(bundle, "Contents", "MacOS")
    compiler = os.path.join(macos, "flashtex-compiler")
    render = os.path.join(macos, "flashtex-render")
    controller = os.path.join(macos, "flashtex-preview-controller")
    fonts = os.path.join(a.repo, "apps/mac/Fonts")
    env = dict(os.environ)
    for k in list(env):
        if k.startswith("FLASHTEX_"):
            del env[k]
    env.update({"FLASHTEX_REPO": a.repo, "FLASHTEX_AUTOATTACH": "1", "FLASHTEX_NO_ACTIVATE": "1",
                "FLASHTEX_LM_DIR": fonts, "FLASHTEX_FONT_DIRS": fonts, "FLASHTEX_SEED_FILE": seed_path,
                "FLASHTEX_LOG": log, "FLASHTEX_TYPING_BENCH": typed, "FLASHTEX_TYPING_BENCH_MS": str(a.interval),
                "FLASHTEX_TYPING_BENCH_OUT": summary, "FLASHTEX_TYPING_BENCH_SETTLE_MS": str(a.settle_ms),
                "FLASHTEX_TYPING_BENCH_MAX_MS": "120000"})
    # Every route here is the previous engine's: pin it, so that a document
    # the new engine would typeset (its default from S5 on) is never measured
    # as one of these routes. The new engine's typing bench is EngineV3Bench
    # (FLASHTEX_V3_BENCH).
    env["FLASHTEX_ENGINE_V3"] = "0"
    if route == "v1":
        env["FLASHTEX_COMPILER"] = compiler
    elif route == "v1-render":
        env["FLASHTEX_COMPILER"] = render
    elif route == "v2":
        env["FLASHTEX_COMPILER"] = render; env["FLASHTEX_PREVIEW_V2"] = "1"
    elif route == "controller":
        env["FLASHTEX_COMPILER"] = compiler; env["FLASHTEX_PREVIEW_CONTROLLER"] = controller
        env["FLASHTEX_CONTROLLER_LEDGER_ROOT"] = os.path.join(cell, "ledger")
    else:
        raise SystemExit("unknown route " + route)
    for exe in (env["FLASHTEX_COMPILER"], env.get("FLASHTEX_PREVIEW_CONTROLLER")):
        if exe and not os.access(exe, os.X_OK):
            return {"cell": name, "route": route, "seed": seed, "skipped": "missing bundled helper %s" % exe}
    load_before, waited = wait_quiet(a.quiet_load, a.quiet_wait)
    up_before = uptime()
    print("==> %s (load %.2f, waited %d s)" % (name, load_before, waited), flush=True)
    t0 = time.time()
    proc = subprocess.Popen([os.path.join(macos, "FlashTeX")], env=env, cwd=cell, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    samples = {}
    deadline = t0 + 300
    while proc.poll() is None and time.time() < deadline:
        time.sleep(0.5)
        try:
            fam = ps_children(proc.pid)
        except Exception:
            fam = {}
        for p, row in fam.items():
            samples[p] = row
    timed_out = proc.poll() is None
    if timed_out:
        print("    timed out after 300 s; killing pid %d" % proc.pid, flush=True)
        proc.kill()
    proc.wait()
    elapsed = time.time() - t0
    load_after = load1()
    up_after = uptime()
    res = {"cell": name, "route": route, "seed": seed, "interval_ms": a.interval, "pid": proc.pid, "exit": proc.returncode,
           "timed_out": timed_out, "elapsed_s": round(elapsed, 1), "load_before": load_before, "load_after": load_after,
           "load_affected": max(load_before, load_after) > a.load_limit, "quiet_waited_s": waited,
           "uptime_before": up_before, "uptime_after": up_after, "env": {k: v for k, v in env.items() if k.startswith("FLASHTEX_")},
           "processes": {str(p): dict(r, cputime_s=cputime_s(r["cputime"])) for p, r in samples.items()}}
    if os.path.exists(summary):
        s = json.load(open(summary))
        res["summary"] = s
        label = engine_of(open(log, encoding="utf-8", errors="replace").read().splitlines()) if os.path.exists(log) else None
        res["engine"] = (label or {}).get("engine") or s.get("engine")
        res["engine_why"] = (label or {}).get("why")
        rows, counts = attribute(log)
        res["attribution_counts"] = counts
        res["stages"] = {k: stats([r.get(k) for r in rows]) for k in STAGES}
        res["stages"] = {k: v for k, v in res["stages"].items() if v}
        json.dump(rows, open(os.path.join(out_dir, name + ".attribution.json"), "w"), indent=1)
        n_results = counts.get("requests_with_lines") or counts.get("applied") or 0
        producers = [r for p, r in res["processes"].items() if int(p) != proc.pid]
        res["producer_cpu_s"] = round(sum(r["cputime_s"] for r in producers), 2)
        res["producer_cpu_ms_per_result"] = round(1000 * res["producer_cpu_s"] / n_results, 2) if n_results else None
        app = res["processes"].get(str(proc.pid))
        res["app_cpu_s"] = app["cputime_s"] if app else None
        print("    p50 %s ms p95 %s ms painted %s/%s; producer %s ms/result CPU, stages p50: %s" % (
            s["keystroke_to_paint_ms"].get("p50_ms"), s["keystroke_to_paint_ms"].get("p95_ms"), s.get("painted"), s.get("keystrokes"),
            res["producer_cpu_ms_per_result"], {k: v["p50"] for k, v in res["stages"].items()}), flush=True)
    else:
        res["error"] = "no bench summary written"
        try:
            tail = [l.rstrip() for l in open(log, encoding="utf-8", errors="replace") if "status:" in l or "bench:" in l or "controller" in l][-3:]
        except Exception:
            tail = []
        res["log_tail"] = tail
        print("    no summary: %s" % tail, flush=True)
    return res


def render_report(out_dir):
    """attribution.json -> Markdown: one table per route with the p50 of every
    stage, the dominating stage, and the load/uptime per cell."""
    run = json.load(open(os.path.join(out_dir, "attribution.json")))
    L = ["# Typing latency on the packaged app, attributed per stage (%s)" % os.path.basename(out_dir.rstrip("/")), ""]
    L.append("App: `%s`. Bundled binaries (sha256): %s." % (run["app"], ", ".join("`%s` %s" % (n, b["sha256"][:12]) for n, b in sorted(run["binaries"].items()))))
    comp = run.get("components_json") or {}
    if comp:
        L.append("`components.json`: " + ", ".join("%s=%s" % (k, (v or {}).get("git_sha")) for k, v in sorted(comp.items()) if isinstance(v, dict) and v.get("bundled")))
    L.append("Seeds: " + "; ".join("`%s` %d bytes (sha256 %s)" % (n, s["bytes"], s["sha256"][:12]) for n, s in run["seeds"].items()) + ". Typed script sha256 %s, %s ms interval." % (run["typed_sha256"][:12], run["interval_ms"]))
    L.append("Quiet gate: 1-minute load < %s before each cell (waited up to %s s); load-affected = load before/after > %s. `uptime` at start `%s`, end `%s`. Other FlashTeX at start: %s." % (run["quiet_load"], run["quiet_wait_s"], run["load_limit"], run.get("uptime_start"), run.get("uptime_end"), run.get("other_flashtex_at_start") or "none"))
    L.append("")
    L.append("Stages (ms, p50 over painted revisions whose own keystroke is the last one the paint covers; see the module docstring for the log lines): key->send = keystroke to `compile: sending`; producer = sending to the last decoded line minus decode; decode = reader JSON decode; ->main = decoded to `event on main`; apply = main to `compile: applied`; helper = sending to applied (helper route, one stage); v2 wait/prepare/preraster/deliver/publish from the `preview-v2:` lines; paint = applied (or v2 published) to `paint:`. Producer CPU = child processes' cumulative CPU (`ps -o cputime`, last sample) / results decoded.")
    L.append("")
    cells = [c for c in run["cells"] if "summary" in c]
    routes = []
    for c in cells:
        if c["route"] not in routes:
            routes.append(c["route"])
    cols = ["key_to_send_ms", "producer_ms", "decode_ms", "to_main_ms", "apply_ms", "helper_roundtrip_ms", "v2_wait_ms", "v2_prepare_ms", "v2_preraster_ms", "v2_deliver_ms", "v2_publish_ms", "paint_ms"]
    names = ["key->send", "producer", "decode", "->main", "apply", "helper", "v2 wait", "v2 prepare", "v2 preraster", "v2 deliver", "v2 publish", "paint"]
    for route in routes:
        L.append("## Route `%s`" % route)
        L.append("")
        L.append("| seed | bench p50 / p95 / max ms | painted / keys / coalesced | " + " | ".join(names) + " | dominant stage | producer CPU ms/result | load before -> after | 200 ms target |")
        L.append("|---|---|---|" + "---|" * len(names) + "---|---|---|---|")
        for c in cells:
            if c["route"] != route:
                continue
            s = c["summary"]; st = c.get("stages", {})
            k2p = s["keystroke_to_paint_ms"]
            vals = [st.get(k, {}).get("p50") for k in cols]
            dom = max(((names[i], v) for i, v in enumerate(vals) if v is not None), key=lambda x: x[1], default=("?", 0))
            total = st.get("key_to_paint_ms", {}).get("p50")
            share = ("%s %.0f%%" % (dom[0], 100 * dom[1] / total)) if total else dom[0]
            met = "met" if (k2p.get("p50_ms") or 1e9) <= 200 and (k2p.get("p95_ms") or 1e9) <= 200 else "NOT met"
            L.append("| %s | %s / %s / %s | %s / %s / %s | %s | %s | %s | %.1f -> %.1f%s | %s |" % (
                c["seed"], k2p.get("p50_ms"), k2p.get("p95_ms"), k2p.get("max_ms"), s.get("painted"), s.get("keystrokes"), s.get("coalesced"),
                " | ".join("—" if v is None else "%.1f" % v for v in vals), share, c.get("producer_cpu_ms_per_result"),
                c["load_before"], c["load_after"], " (load-affected)" if c.get("load_affected") else "", met))
        L.append("")
    failed = [c for c in run["cells"] if "summary" not in c]
    if failed:
        L.append("## Cells without a bench summary")
        L.append("")
        for c in failed:
            L.append("- `%s`: %s %s" % (c["cell"], c.get("error") or c.get("skipped"), c.get("log_tail", "")))
        L.append("")
    L.append("## Per-cell uptime")
    L.append("")
    for c in run["cells"]:
        L.append("- `%s`: before `%s`; after `%s`" % (c["cell"], c.get("uptime_before"), c.get("uptime_after")))
    L.append("")
    L.append("Raw: `attribution.json` (cells, processes sampled, env), `<cell>.json` (the shell's bench summary), `<cell>.attribution.json` (per painted revision stages), `<cell>.log` (FLASHTEX_LOG with the timeline).")
    return "\n".join(L) + "\n"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--app")
    ap.add_argument("--repo")
    ap.add_argument("--out")
    ap.add_argument("--work", default=None)
    ap.add_argument("--seeds", default="fixture demo hw1 render27 body60k")
    ap.add_argument("--routes", default="v1 v1-render v2 controller")
    ap.add_argument("--interval", type=int, default=30)
    ap.add_argument("--quiet-load", type=float, default=8)
    ap.add_argument("--quiet-wait", type=int, default=600)
    ap.add_argument("--load-limit", type=float, default=10)
    ap.add_argument("--settle-ms", type=int, default=60000)
    ap.add_argument("--typed", default=None)
    ap.add_argument("--merge", action="store_true", help="add cells to an existing attribution.json (same cell name replaces)")
    ap.add_argument("--report", default=None, help="render <dir>/attribution.json as Markdown to stdout and exit")
    a = ap.parse_args()
    if a.report:
        sys.stdout.write(render_report(a.report)); return
    if not (a.app and a.repo and a.out):
        ap.error("--app, --repo and --out are required")
    a.repo = os.path.abspath(a.repo); a.app = os.path.abspath(a.app); a.out = os.path.abspath(a.out)
    a.work = os.path.abspath(a.work or os.path.join(a.out, "work"))
    os.makedirs(a.out, exist_ok=True); os.makedirs(a.work, exist_ok=True)
    typed = a.typed or os.path.join(a.repo, "tools/typing-bench/typed-200.txt")
    seeds = make_seeds(a.repo, a.work, a.seeds.split())
    for n, s in seeds.items():
        print("    seed %-9s %6d bytes sha256 %s" % (n, s["bytes"], s["sha256"][:12]), flush=True)
    macos = os.path.join(a.app, "Contents", "MacOS")
    bins = {n: {"sha256": sha256(os.path.join(macos, n)), "bytes": os.path.getsize(os.path.join(macos, n))}
            for n in sorted(os.listdir(macos)) if os.path.isfile(os.path.join(macos, n))}
    comp = os.path.join(a.app, "Contents", "Resources", "components.json")
    path = os.path.join(a.out, "attribution.json")
    run = None
    if a.merge and os.path.exists(path):
        run = json.load(open(path))
        run["seeds"].update(seeds)
    if run is None:
        run = {"app": a.app, "binaries": bins, "components_json": json.load(open(comp)) if os.path.exists(comp) else None,
               "seeds": seeds, "typed_sha256": sha256(typed), "interval_ms": a.interval, "quiet_load": a.quiet_load,
               "quiet_wait_s": a.quiet_wait, "load_limit": a.load_limit, "uptime_start": uptime(),
               "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "cells": [],
               "other_flashtex_at_start": subprocess.run(["pgrep", "-lx", "FlashTeX"], capture_output=True, text=True).stdout.strip()}
    for route in a.routes.split():
        for seed in seeds:
            res = run_cell(a, a.app, route, seed, seeds, a.out, typed)
            run["cells"] = [c for c in run["cells"] if c.get("cell") != res["cell"]] + [res]
            run["uptime_end"] = uptime(); run["finished_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
            json.dump(run, open(path, "w"), indent=1, sort_keys=True)
    print("wrote %s" % path)


if __name__ == "__main__":
    main()
