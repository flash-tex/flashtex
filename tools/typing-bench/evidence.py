#!/usr/bin/env python3
"""Writes docs/evidence/typing-bench-<UTC>.md from the raw JSON summaries of one
tools/typing-bench/run.sh invocation (one table per producer route, a cross-route
comparison, methodology, limitations). Usage:

  evidence.py <out.md> <raw dir> <repo root> <UTC stamp> <typed script> <producer notes file>

Raw file names are `<route>-<seed>-<interval>ms.json`; run.sh adds `route`,
`load_avg_before`, `load_avg_after`, `load_limit` and `load_affected` to each
summary the app wrote. Cells with `load_affected` are marked, never dropped.
"""
import glob
import json
import os
import subprocess
import sys

out, raw, root, utc, typed_path, notes_path = sys.argv[1:7]


def sh(*a):
    try:
        return subprocess.check_output(a, text=True).strip()
    except Exception as e:  # noqa: BLE001
        return "unavailable (%s)" % e


def ms(v):
    if v is None:
        return "—"
    return "%.0f" % v if v >= 10 else "%.1f" % v


sha = sh("git", "-C", root, "rev-parse", "--short", "HEAD")
branch = sh("git", "-C", root, "rev-parse", "--abbrev-ref", "HEAD")
hw = sh("sysctl", "-n", "machdep.cpu.brand_string")
osv = sh("sw_vers", "-productVersion")
notes = [n.strip() for n in open(notes_path, encoding="utf-8").read().splitlines() if n.strip()]

runs = []
for f in sorted(glob.glob(os.path.join(raw, "*.json"))):
    d = json.load(open(f, encoding="utf-8"))
    d["_file"] = os.path.basename(f)
    route, seed, interval = d["_file"][:-5].split("-", 2)
    d.setdefault("route", route)
    d["_seed"], d["_interval"] = seed, interval
    runs.append(d)

ROUTE_ORDER = ["compiler", "render", "controller", "v2"]
ROUTE_TEXT = {
    "compiler": "`compiler`: direct worker route — `FLASHTEX_COMPILER` = main's `flashtex-compiler`, runtime-v1 JSON Lines on stdin/stdout, one request in flight.",
    "render": "`render`: direct worker route with `flashtex-render` (render-pipeline branch) as the runtime-v1 worker.",
    "controller": "`controller`: durable helper route — `FLASHTEX_PREVIEW_CONTROLLER` = `flashtex-preview-controller` (crates/preview-controller on origin/main), which owns the edit ledger (fsync per edit under a temporary `FLASHTEX_CONTROLLER_LEDGER_ROOT`), the lexical index and main's compiler; previews arrive as asynchronous `update` frames.",
    "v2": "`v2`: the experimental display-list-v2 pane (`FLASHTEX_PREVIEW_V2=1`) painting live results.",
}
routes = [r for r in ROUTE_ORDER if any(d["route"] == r for d in runs)]
routes += sorted({d["route"] for d in runs} - set(routes))


def engine_of(d):
    """The run's engine (the summary's `engine`, from retirement plan #1236
    S3r on); a summary without one was the previous engine, the only one the
    shell's typing bench drove before."""
    return d.get("engine") or "previous (unlabelled)"


def load_mark(d):
    if not d.get("load_affected"):
        return ""
    return " ⚠ load %.1f" % max(d.get("load_avg_before") or 0, d.get("load_avg_after") or 0)


lines = []
lines.append("# Typing bench: keystroke → paint latency (%s)" % utc)
lines.append("")
lines.append("Branch `%s` @ `%s`; %s; macOS %s; release build of `FlashTeXMac` (`swift build -c release`)." % (branch, sha, hw, osv))
lines.append("Script: `tools/typing-bench/run.sh`; raw JSON summaries in `%s/`." % os.path.basename(raw))
lines.append("")
lines.append("Producers:")
for n in notes:
    lines.append("- " + n)
lines.append("")
lines.append("## Results")
lines.append("")
lines.append("Latency is keystroke → paint per typed character (ms); a coalesced keystroke is measured to the first paint that showed it. `compile` is the shell's send → result time on the main thread; `render` is PreviewView body → last page Canvas draw. A cell marked ⚠ ran while the 1-minute load average exceeded the limit (%s) before or after it; such cells are reported, never used as a gate." % (runs[0].get("load_limit", 10) if runs else 10))
lines.append("")

HEADER = "| route | engine | producer | seed | bytes | interval | keys typed | paints | coalesced | k→p p50 | p95 | p99 | max | compile p50 | compile p95 | render p50 | render p95 | unpainted | load before/after |"
RULE = "|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|"


def row(d):
    k = d["keystroke_to_paint_ms"]
    c = d["compile_ms"]
    r = d.get("render_pass_ms", {})
    typed = "%d" % d["keystrokes"] + (" of %d (budget)" % d["script_keystrokes"] if d.get("typing_budget_exhausted") else "")
    load = "%s / %s" % (ms(d.get("load_avg_before")), ms(d.get("load_avg_after")))
    return "| %s | %s | %s | %s | %d | %s | %s | %d | %d | %s | %s | %s | %s | %s | %s | %s | %s | %d | %s%s |" % (
        d["route"], engine_of(d), d["producer"], d["_seed"], d["document_bytes_after"], d["_interval"], typed, d["paints"], d["coalesced"],
        ms(k.get("p50_ms")), ms(k.get("p95_ms")), ms(k.get("p99_ms")), ms(k.get("max_ms")),
        ms(c.get("p50_ms")), ms(c.get("p95_ms")), ms(r.get("p50_ms")), ms(r.get("p95_ms")), d["unpainted"], load, load_mark(d))


for route in routes:
    lines.append("### Route `%s`" % route)
    lines.append("")
    if route in ROUTE_TEXT:
        lines.append(ROUTE_TEXT[route])
        lines.append("")
    lines.append(HEADER)
    lines.append(RULE)
    for d in runs:
        if d["route"] == route:
            lines.append(row(d))
    lines.append("")
if not runs:
    lines.append("(no runs completed)")
    lines.append("")

# Cross-route comparison: one row per seed × interval, k→p p50 / p95 per route.
if len(routes) > 1:
    lines.append("### Cross-route comparison (keystroke → paint p50 / p95 ms)")
    lines.append("")
    lines.append("| seed | interval | " + " | ".join(routes) + " |")
    lines.append("|---|---:|" + "|".join("---:" for _ in routes) + "|")
    cells = sorted({(d["_seed"], d["_interval"]) for d in runs}, key=lambda x: (x[0], -int(x[1].rstrip("ms"))))
    for seed, interval in cells:
        parts = []
        for route in routes:
            d = next((d for d in runs if d["route"] == route and d["_seed"] == seed and d["_interval"] == interval), None)
            if d is None:
                parts.append("—")
            else:
                k = d["keystroke_to_paint_ms"]
                parts.append("%s / %s%s" % (ms(k.get("p50_ms")), ms(k.get("p95_ms")), " ⚠" if d.get("load_affected") else ""))
        lines.append("| %s | %s | %s |" % (seed, interval, " | ".join(parts)))
    lines.append("")

skipped = [n for n in notes if n.startswith("skipped:")]
if skipped:
    lines.append("Routes not measured: " + "; ".join(n[len("skipped:"):].strip() for n in skipped))
    lines.append("")

lines.append("Typed script: `tools/typing-bench/typed-200.txt` (%d characters, inserted before `\\end{document}` when present, else at the end)." % len(open(typed_path, encoding="utf-8").read()))
lines.append("Seeds: `demo` = `apps/mac/Samples/demo.tex`; `body60k` = the demo's paragraphs repeated to ≥ 60 KB in one document; `fixture` = the entry document of `protocol/fixtures/compile-request.json`.")
lines.append("")
paint_point = runs[0].get("paint_point") if runs else None
lines.append("## Methodology")
lines.append("")
lines.append("- The app is launched with `FLASHTEX_AUTOATTACH=1 FLASHTEX_NO_ACTIVATE=1 FLASHTEX_SEED_FILE=<seed> FLASHTEX_TYPING_BENCH=<script>` plus the route's environment (`FLASHTEX_COMPILER=<worker>` for the direct routes; `FLASHTEX_PREVIEW_CONTROLLER=<helper>` with a fresh temporary `FLASHTEX_CONTROLLER_LEDGER_ROOT` per cell for the durable route; `FLASHTEX_PREVIEW_V2=1` for the v2 pane). After the producer attached and its first result was painted, `TypingBenchDriver` (`apps/mac/Sources/FlashTeXMac/TypingBench.swift`) inserts the script one extended grapheme cluster at a time into the real editor `NSTextView` through `insertText(_:replacementRange:)` from a main-run-loop `Timer` at the configured interval (30 ms ≈ a fast typist; 0 ms = one keystroke per run-loop turn, a burst). Each insertion takes the production path: `NSTextViewDelegate.textDidChange` → SwiftUI binding → `ShellModel.updateActiveText` (revision bump, `keystroke:` log line) → auto-compile / `edit` submission (one in flight, newest buffer coalesced) → result → `PreviewView` render.")
lines.append("- Keystroke time is stamped immediately before `insertText` on the monotonic clock (`clock_gettime_nsec_np(CLOCK_UPTIME_RAW)`, i.e. `mach_absolute_time` in ns). For a person typing, the same recorder uses the `NSEvent.timestamp` of the `keyDown` seen by an in-process local event monitor (HID time on the same clock) — no Accessibility permission or event tap is involved, and a key that changes no text is discarded at the end of its dispatch.")
lines.append("- Paint time (`paint:` log line): %s. A revision whose result changed nothing visible (SwiftUI skipped the canvas redraw) is still recorded as painted, flagged `redrawn: false`." % (paint_point or "see `TypingBench.paintPointDescription`"))
lines.append("- A paint of revision N makes every unpainted keystroke with revision ≤ N visible; those with revision < N are `coalesced`. p50/p95/p99 are nearest-rank percentiles over the per-keystroke latencies. The run ends when every keystroke is painted (or after the settle timeout), and the app writes the JSON summary and exits.")
lines.append("- Machine load: `sysctl vm.loadavg` (1-minute average) is recorded before and after every cell; run.sh waits for the load to drop below `--quiet-load` before each cell (bounded by `--quiet-wait`) and flags cells whose before/after load exceeded `--load-limit`.")
lines.append("")
lines.append("## Limitations")
lines.append("")
lines.append("- The bench inserts text programmatically: there is no OS keyboard event, no event-queue wait, no key repeat and no input-method composition; real typing adds the HID → WindowServer → `NSApplication.sendEvent` hop, which the local-monitor path measures but this bench cannot.")
lines.append("- `paint` is the completed CoreAnimation commit, not the display scan-out: the pixels reach the panel at the next vsync (up to one frame, 8–17 ms at 60–120 Hz) after the stamp, and later still if the render server is behind. No IOSurface presentation callback is observed. The window is ordered back (`FLASHTEX_NO_ACTIVATE=1`) and may be occluded during the run; commits still happen, on-screen visibility is not verified.")
lines.append("- Compile time is measured on the main thread from send to result application, so it includes any time the reply waited behind main-thread work; for the durable route it is edit submission → preview `update` applied (the helper's fsync and compile are inside it).")
lines.append("- Typing stops after `FLASHTEX_TYPING_BENCH_MAX_MS` (120 s here); a cell marked 'of 200 (budget)' typed fewer characters because each keystroke waited for main-thread work.")
lines.append("- One machine, one run per cell, no warm-up discard beyond the first compile; numbers are indicative, not a regression gate. Load-affected cells are marked, not excluded.")
open(out, "w", encoding="utf-8").write("\n".join(lines) + "\n")
print("\n".join(lines[:4]))
for l in lines:
    if l.startswith("| ") and not l.startswith("| route") and not l.startswith("| seed"):
        print(l)
