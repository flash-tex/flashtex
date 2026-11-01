#!/usr/bin/env python3
"""Assembles reports/<UTC>.md from a run directory and asserts thresholds.json.

Inputs (all written by run.sh into the run directory): env.json, helpers.json,
app.json, bundle.json, steps.jsonl, commands.log, typing-bench/typing-bench-*/
*.json, launch-check.json (+ launch-check.md), capture-cycle.json. Missing
inputs are reported as failed gates, never silently skipped. Exit 1 on any
failed gate. Stdlib only.
"""
import argparse
import glob
import json
import os
import sys


def load(path, default=None):
    try:
        return json.load(open(path, encoding="utf-8"))
    except Exception:
        return default


def ms(v):
    if v is None:
        return "—"
    return "%.0f" % v if v >= 10 else "%.1f" % v


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--run-dir", required=True)
    ap.add_argument("--thresholds", required=True)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    rd = a.run_dir
    th = json.load(open(a.thresholds))
    env = load(os.path.join(rd, "env.json"), {})
    helpers = load(os.path.join(rd, "helpers.json"), {})
    app = load(os.path.join(rd, "app.json"), {})
    bundle = load(os.path.join(rd, "bundle.json"), {})
    launch = load(os.path.join(rd, "launch-check.json"))
    cycle = load(os.path.join(rd, "capture-cycle.json"))
    extras = load(os.path.join(rd, "extras.json"), {})
    render_attach = load(os.path.join(rd, "render-attach.json"))
    windows = {wid: load(os.path.join(rd, "open-window-%s.json" % wid)) for wid in ("a11y-help", "nearby")}
    relaunch = load(os.path.join(rd, "worker-relaunch.json"))
    multifile = load(os.path.join(rd, "multifile.json"))
    app_tests = load(os.path.join(rd, "app-tests.json"))
    hist_analysis = ""
    try:
        hist_analysis = open(os.path.join(rd, "historical", "analysis.txt"), encoding="utf-8").read()
    except Exception:
        pass
    exact_export = load(os.path.join(rd, "exact-export.json"))
    steps = []
    try:
        steps = [json.loads(l) for l in open(os.path.join(rd, "steps.jsonl")) if l.strip()]
    except Exception:
        pass
    commands = ""
    try:
        commands = open(os.path.join(rd, "commands.log"), encoding="utf-8").read().strip()
    except Exception:
        pass
    utc = env.get("utc", os.path.basename(rd))
    gates = []  # (section, name, ok, detail)

    def gate(section, name, ok, detail=""):
        gates.append((section, name, bool(ok), str(detail)))
        return bool(ok)

    # ---------------------------------------------------------------- builds
    sec = "build"
    gate(sec, "helpers built from %s (pinned clone HEAD %s; entries cargo rewrote while building: %s)" % (helpers.get("ref"), (helpers.get("scratch_head") or "?")[:7], ", ".join(x.strip() for x in helpers.get("scratch_dirty_status", [])) or "none"), helpers.get("built_ok") and all(b.get("sha256") for b in helpers.get("binaries", {}).values()),
         ", ".join("%s=%s" % (k, (v.get("sha256") or "missing")[:12]) for k, v in sorted(helpers.get("binaries", {}).items())))
    gate(sec, "app built (release) from %s" % app.get("branch"), app.get("built_ok"), (app.get("sha256") or "missing")[:12])
    bth = th.get("bundle", {}).get("gates", {})
    bbins = bundle.get("binaries", {})
    gate(sec, "FlashTeX.app packaged with %s" % ", ".join(bth.get("required_binaries", [])), bundle.get("built_ok") and all(n in bbins for n in bth.get("required_binaries", [])),
         ", ".join(sorted(bbins)))
    if bth.get("bundled_helper_sha256_equals_built_helper_sha256"):
        mism = []
        for n, b in helpers.get("binaries", {}).items():
            if n not in bth.get("required_binaries", []):
                continue
            if not b.get("sha256_content") or bbins.get(n, {}).get("sha256_content") != b.get("sha256_content"):
                mism.append(n)
        gate(sec, "bundled helpers carry the freshly built object code (signature-masked Mach-O content sha256 equal, all four)",
             bundle.get("built_ok") and not mism, "mismatch: %s" % mism if mism else "identical (make-app.sh re-signs the bundle ad hoc, so only the code-signature blob differs)")
        gate(sec, "bundled FlashTeX carries the freshly built FlashTeXMac object code (signature-masked content sha256 equal)",
             bundle.get("built_ok") and app.get("sha256_content") and bbins.get("FlashTeX", {}).get("sha256_content") == app.get("sha256_content"),
             "%s vs %s" % ((app.get("sha256_content") or "?")[:12], (bbins.get("FlashTeX", {}).get("sha256_content") or "?")[:12]))

    comp = bundle.get("components_json") or {}
    if comp:
        main_short = (env.get("sources", {}).get("main_sha") or "")[:7]
        branch_short = (env.get("sources", {}).get("branch_sha") or "")[:7]
        bad = []
        def same(short, full):  # make-app.sh uses git's auto-length short SHA (7+ chars)
            return bool(short) and bool(full) and len(short) >= 7 and full.startswith(short)
        for key in ("compiler", "pdf", "bridge", "edit_ledger"):
            if not same((comp.get(key) or {}).get("git_sha"), env.get("sources", {}).get("main_sha")):
                bad.append("%s=%s" % (key, (comp.get(key) or {}).get("git_sha")))
        if not same((comp.get("app") or {}).get("git_sha"), env.get("sources", {}).get("branch_sha")):
            bad.append("app=%s" % (comp.get("app") or {}).get("git_sha"))
        for key, name in (("render", "render"), ("pdf_exact", "pdf-exact")):
            e = extras.get(name) or {}
            if e.get("built_ok") and not same((comp.get(key) or {}).get("git_sha"), e.get("sha")):
                bad.append("%s=%s (expected %s)" % (key, (comp.get(key) or {}).get("git_sha"), (e.get("sha") or "")[:7]))
        gate(sec, "bundle components.json git SHAs: helpers == %s (main), app == %s (branch), render/pdf_exact == their branch SHAs" % (main_short, branch_short), not bad, "; ".join(bad) or "all match")
    for name, label in (("render", "flashtex-render"), ("pdf-exact", "flashtex-pdf-exact")):
        e = extras.get(name)
        if e is not None:
            gate(sec, "%s built from %s" % (label, e.get("ref")), e.get("built_ok") and e.get("scratch_head") == e.get("sha"), "%s @ %s%s" % (label, (e.get("sha") or "?")[:12], (" cargo rewrote: " + ", ".join(e.get("scratch_dirty_status", []))) if e.get("scratch_dirty_status") else ""))
            if e.get("built_ok") and bbins.get(label):
                gate(sec, "bundled %s carries the freshly built object code (signature-masked content sha256 equal)" % label,
                     bbins[label].get("sha256_content") == e.get("sha256_content"), "%s vs %s" % ((e.get("sha256_content") or "?")[:12], (bbins[label].get("sha256_content") or "?")[:12]))

    # ---------------------------------------------------------- typing bench
    tb = th.get("typing_bench", {})
    tg = tb.get("gates", {})
    # (producer, pass directory, file prefix typing-bench/run.sh gives that producer)
    producers = [("compiler", "typing-bench", "compiler"), ("render", "typing-bench", "render"), ("controller", "typing-bench", "controller"), ("historical", "typing-bench-historical", "controller")]
    runs = {}  # producer -> cell -> summary (first attempt wins; a retry pass fills cells the first attempt lost)
    retried = {}  # producer -> [cells taken from the retry pass]
    load_before = {}  # pass directory -> {"load_average": "{ 1 5 15 }", ...}
    for prod, sub, prefix in producers:
        runs[prod] = {}
        retried[prod] = []
        load_before[sub] = load(os.path.join(rd, sub, "load-before.json"), {})
        for attempt, pattern in (("first", os.path.join(rd, sub, "typing-bench-*", prefix + "-*.json")), ("retry", os.path.join(rd, sub, "retry", "typing-bench-*", prefix + "-*.json"))):
            for f in sorted(glob.glob(pattern)):
                d = load(f)
                if not d:
                    continue
                cell = os.path.basename(f)[len(prefix) + 1:-5]  # compiler-demo-30ms -> demo-30ms
                if cell in runs[prod]:
                    continue
                d["_attempt"] = attempt
                runs[prod][cell] = d
                if attempt == "retry":
                    retried[prod].append(cell)

    def load1(sub):
        try:
            return float(load_before[sub].get("load_average", "").strip("{} ").split()[0])
        except Exception:
            return None
    max_load = tg.get("latency_gate_max_load_average_1min")
    present = {"compiler": True, "render": bool(runs["render"]), "controller": bool(runs["controller"]), "historical": bool(runs["historical"]) or os.path.isdir(os.path.join(rd, "typing-bench-historical"))}

    def cell_gated(d, pass_gated):
        # The bench marks a cell load-affected when the 1-minute load before or after it exceeded --load-limit (10).
        if "load_affected" in d:
            return not d["load_affected"]
        return pass_gated
    for prod, sub, prefix in producers:
        sec = "typing-bench/" + prod
        pg = tg.get("producers", {}).get(prod, {})
        if not present[prod]:
            continue
        l1 = load1(sub)
        latency_gated = not (max_load is not None and l1 is not None and l1 > max_load)
        if pg.get("per_seed_p50_ms_max"):
            affected = [c for c, d in runs[prod].items() if d.get("load_affected")]
            per_cell = any("load_affected" in d for d in runs[prod].values())
            if affected:
                gate(sec, "latency gates NOT applied to load-affected cells (bench --load-limit 10): %s" % ", ".join(sorted(affected)), True, load_before[sub].get("uptime", ""))
            elif not per_cell and not latency_gated:
                gate(sec, "latency gates NOT applied to this pass (no per-cell load data; 1-minute load %s > %s before the pass)" % (l1, max_load), True, load_before[sub].get("uptime", ""))
        for cell in tg.get("required_cells", []):
            d = runs[prod].get(cell)
            if d is None:
                gate(sec, "%s: summary present" % cell, False, "no JSON summary (run failed or timed out)")
                continue
            ev = tg.get("every_cell", {})
            k = d.get("keystroke_to_paint_ms", {})
            gate(sec, "%s: unpainted keystrokes <= %d" % (cell, ev.get("unpainted_max", 0)), d.get("unpainted", 1) <= ev.get("unpainted_max", 0), "unpainted=%s" % d.get("unpainted"))
            if ev.get("typing_budget_exhausted") is False:
                gate(sec, "%s: typing budget not exhausted" % cell, not d.get("typing_budget_exhausted"), "typed %s of %s" % (d.get("typed"), d.get("script_keystrokes")))
            if ev.get("typed_equals_script_keystrokes"):
                gate(sec, "%s: every script keystroke typed" % cell, d.get("typed") == d.get("script_keystrokes") and d.get("keystrokes") == d.get("script_keystrokes"),
                     "typed=%s keystrokes=%s script=%s" % (d.get("typed"), d.get("keystrokes"), d.get("script_keystrokes")))
            gate(sec, "%s: paints >= %d" % (cell, ev.get("paints_min", 1)), d.get("paints", 0) >= ev.get("paints_min", 1), "paints=%s%s" % (d.get("paints"), " (from the retry pass: the first attempt's app died mid-run)" if d.get("_attempt") == "retry" else ""))
            expected_producer = pg.get("expected_producer")
            if expected_producer:
                gate(sec, "%s: producer reported by the app == %s" % (cell, expected_producer), d.get("producer") == expected_producer, d.get("producer"))
            seed = cell.rsplit("-", 1)[0]
            lim = pg.get("per_seed_p50_ms_max", {}).get(seed)
            if lim is not None and cell_gated(d, latency_gated):
                gate(sec, "%s: keystroke->paint p50 <= %s ms" % (cell, lim), k.get("p50_ms") is not None and k["p50_ms"] <= lim, "p50=%s ms; 1-minute load before/after the cell %s/%s" % (ms(k.get("p50_ms")), d.get("load_avg_before"), d.get("load_avg_after")))
    targets = tb.get("targets", {})

    # ---------------------------------------------------------- launch check
    lg = th.get("launch_check", {}).get("gates", {})
    sec = "launch-check"
    if launch is None:
        gate(sec, "launch-check ran", False, "no launch-check.json (skipped or bundle missing)")
    elif launch.get("refused"):
        gate(sec, "launch-check ran", False, launch.get("reason"))
    else:
        gate(sec, "FAIL lines <= %d" % lg.get("fail_lines_max", 0), len(launch.get("fail_lines", [])) <= lg.get("fail_lines_max", 0), "; ".join(launch.get("fail_lines", [])) or "none")
        notes = launch.get("notes", [])
        for req in lg.get("required_notes", []):
            gate(sec, "note: %s" % req, any(req in n for n in notes), next((n for n in notes if req in n), "absent"))
        if lg.get("no_activate_env_required"):
            gate(sec, "launched with FLASHTEX_NO_ACTIVATE=1 (never activates the window)", launch.get("launched_with_no_activate"), "; ".join(launch.get("open_calls", []))[:300])

    # --------------------------------------------------------- capture cycle
    cg = th.get("capture_cycle", {}).get("gates", {})
    sec = "capture-cycle"
    if cycle is None:
        gate(sec, "capture cycle ran", False, "no capture-cycle.json")
    else:
        for c in cycle.get("checks", []):
            gate(sec, c["name"], c["ok"], c.get("detail", "")[:200])
        if cg.get("no_network_provider_call"):
            gate(sec, "no provider enablement (bridge without --enable-grok, no XAI_API_KEY in env)",
                 cycle.get("network", {}).get("bridge_enable_grok") is False and cycle.get("network", {}).get("xai_api_key_in_env") is False, cycle.get("network"))
        conv = (cycle.get("convert_error") or {}).get("code")
        gate(sec, "capture_convert error code == %s" % cg.get("bridge_convert_error_code"), conv == cg.get("bridge_convert_error_code"), conv)
        comp = cycle.get("compile") or {}
        gate(sec, "compiler status in %s" % cg.get("compiler_status_in"), comp.get("status") in cg.get("compiler_status_in", []), comp.get("status"))
        pdf = cycle.get("pdf") or {}
        gate(sec, "pdf page objects == compiled pages and >= %d" % cg.get("compiled_pages_min", 1),
             pdf.get("page_objects") == comp.get("pages") and (comp.get("pages") or 0) >= cg.get("compiled_pages_min", 1), "pdf=%s compiled=%s" % (pdf.get("page_objects"), comp.get("pages")))

    # ------------------------------------------- bundled render + exact export
    ra = th.get("render_attach", {}).get("gates", {})
    sec = "render-attach"
    if extras.get("render", {}).get("built_ok"):
        if render_attach is None:
            gate(sec, "render attach check ran", False, "no render-attach.json")
        elif render_attach.get("refused"):
            gate(sec, "render attach check ran", False, render_attach.get("reason"))
        else:
            for c in render_attach.get("checks", []):
                gate(sec, c["name"], c["ok"], c.get("detail", ""))
            if ra.get("no_activate_env_required"):
                gate(sec, "launched with FLASHTEX_NO_ACTIVATE=1", (render_attach.get("env") or {}).get("FLASHTEX_NO_ACTIVATE") == "1", json.dumps(render_attach.get("env")))
    sec = "exact-export"
    if extras.get("pdf-exact", {}).get("built_ok"):
        if exact_export is None:
            gate(sec, "exact export check ran", False, "no exact-export.json")
        else:
            for c in exact_export.get("checks", []):
                gate(sec, c["name"], c["ok"], c.get("detail", ""))

    for wid, rec in windows.items():
        sec = "open-window/" + wid
        if rec is None:
            continue
        if rec.get("refused"):
            gate(sec, "window check ran", False, rec.get("reason"))
            continue
        for c in rec.get("checks", []):
            gate(sec, c["name"], c["ok"], c.get("detail", ""))
    sec = "worker-relaunch"
    if relaunch is not None:
        if relaunch.get("refused"):
            gate(sec, "worker relaunch check ran", False, relaunch.get("reason"))
        else:
            for c in relaunch.get("checks", []):
                gate(sec, c["name"], c["ok"], c.get("detail", ""))
    sec = "multifile"
    if multifile is not None:
        if multifile.get("refused"):
            gate(sec, "multi-file check ran", False, multifile.get("reason"))
        else:
            for c in multifile.get("checks", []):
                gate(sec, c["name"], c["ok"], c.get("detail", ""))
    sec = "app-tests"
    if app_tests is not None:
        tg2 = th.get("app_tests", {}).get("gates", {})
        gate(sec, "swift test exit 0 (%d passed, %d failed, %d skipped)" % (app_tests.get("passed", 0), app_tests.get("failed", 0), app_tests.get("skipped", 0)), app_tests.get("exit_code") == 0 and app_tests.get("failed", 1) == 0, json.dumps(app_tests.get("totals")))
        for name in tg2.get("required_passed", []):
            c = app_tests.get("cases", {}).get(name)
            gate(sec, "test %s passed (not skipped)" % name, c is not None and c.get("result") == "passed", (c or {}).get("skip_reason") or "; ".join((c or {}).get("failures", [])) or ("%s s" % (c or {}).get("seconds")))
    sec = "historical"
    if present.get("historical"):
        hg = th.get("historical", {}).get("gates", {})
        n_hist = sum(1 for l in hist_analysis.splitlines() if l.startswith("| historical |"))
        gate(sec, "historical classification produced rows for the FLASHTEX_COMPLETED_SNAPSHOTS=1 pass", n_hist >= 1, "%d historical rows, %d baseline rows" % (n_hist, sum(1 for l in hist_analysis.splitlines() if l.startswith("| baseline |"))))
        if hg.get("historical_frames_painted_min") is not None:
            blocks, cur, depth = [], [], 0
            for line in hist_analysis.splitlines():
                if line.startswith("{"):
                    cur, depth = [line], 1
                elif depth:
                    cur.append(line)
                    if line.startswith("}"):
                        try:
                            blocks.append(json.loads("\n".join(cur)))
                        except Exception:
                            pass
                        depth = 0
            hist_blocks = [b for b in blocks if b.get("mode") == "historical"]
            painted = sum(b.get("historical_frames_painted_log", 0) for b in hist_blocks)
            gate(sec, "historical frames painted (log 'historical: painted') across the historical pass >= %d" % hg["historical_frames_painted_min"], painted >= hg["historical_frames_painted_min"], "painted %d, refused %d over %d cells" % (painted, sum(b.get("historical_frames_refused_log", 0) for b in hist_blocks), len(hist_blocks)))

    failed = [g for g in gates if not g[2]]
    verdict = "PASS" if not failed else "FAIL"

    # ---------------------------------------------------------------- render
    L = []
    src = env.get("sources", {})
    mach = env.get("machine", {})
    L.append("# Mac live acceptance: typing-to-paint, helper crash/restart, packaged capture cycle (%s)" % utc)
    L.append("")
    L.append("**Verdict: %s** — %d gates passed, %d failed (thresholds: `tools/native-validation/mac-live/thresholds.json`)." % (verdict, len(gates) - len(failed), len(failed)))
    L.append("")
    L.append("Runner: `tools/native-validation/mac-live/run.sh`; raw artifacts in `reports/%s/` (JSON summaries, launch-check evidence, capture-cycle transcript, logs)." % os.path.basename(rd))
    L.append("")
    L.append("## Provenance")
    L.append("")
    L.append("| item | value |")
    L.append("|---|---|")
    L.append("| driving agent | `%s` |" % env.get("agent"))
    L.append("| agent session | `%s` |" % env.get("session"))
    L.append("| run started (UTC) | %s |" % utc)
    L.append("| user / host | `%s` / `%s` |" % (env.get("user"), env.get("host")))
    L.append("| helpers source | `%s` = `%s` (%s, committed %s) |" % (src.get("main_ref"), src.get("main_sha"), src.get("main_subject"), src.get("main_commit_utc")))
    L.append("| app source | `%s` = `%s` (%s, committed %s) |" % (src.get("branch"), src.get("branch_sha"), src.get("branch_subject"), src.get("branch_commit_utc")))
    L.append("| branch contains main | %s (merge-base `%s`) |" % (src.get("branch_contains_main"), src.get("merge_base")))
    rn = env.get("runner", {})
    L.append("| runner checkout | `%s` @ `%s` (%s dirty tracked files) |" % (rn.get("checkout_branch"), rn.get("checkout_sha"), rn.get("checkout_dirty_tracked_files")))
    for f, h in sorted(rn.get("files", {}).items()):
        L.append("| runner file `%s` | sha256 `%s` |" % (f, h))
    L.append("| machine | %s, %s cores, %s GiB; macOS %s (%s); kernel %s |" % (mach.get("hardware"), mach.get("cores"), (int(mach.get("memory_bytes", 0) or 0) // (1 << 30)), mach.get("os"), mach.get("os_build"), mach.get("kernel")))
    L.append("| toolchain | %s; %s; %s; %s; python %s |" % (mach.get("xcode"), mach.get("swift"), mach.get("cargo"), mach.get("rustc"), mach.get("python3")))
    end = load(os.path.join(rd, "end.json"), {})
    L.append("| load average at start / end | %s / %s (1, 5, 15 min; other agents build and test on this machine concurrently) |" % (mach.get("load_average_at_start"), end.get("load_average_at_end", "—")))
    L.append("| `uptime` at start / end | `%s` / `%s` |" % (mach.get("uptime", "—"), end.get("uptime_at_end", "—")))
    L.append("| run finished (UTC) | %s |" % end.get("utc_end", "—"))
    L.append("| other FlashTeX/FlashTeXMac processes at start | %s |" % (mach.get("other_flashtex_processes_at_start") or "none"))
    L.append("")
    L.append("### Binaries")
    L.append("")
    L.append("| binary | built from | git SHA | sha256 (as shipped) | sha256 (signature-masked content) | bytes | signature |")
    L.append("|---|---|---|---|---|---:|---|")
    for n, b in sorted(helpers.get("binaries", {}).items()):
        L.append("| `%s` (scratch build) | `%s` | `%s` | `%s` | `%s` | %s | %s |" % (n, b.get("crate"), b.get("git_sha"), b.get("sha256"), b.get("sha256_content"), b.get("bytes"), b.get("signature")))
    L.append("| `FlashTeXMac` (swift build -c release) | `apps/mac` | `%s` | `%s` | `%s` | %s | %s |" % (app.get("sha"), app.get("sha256"), app.get("sha256_content"), app.get("bytes"), app.get("signature")))
    for name, e in sorted(extras.items()):
        L.append("| `%s` (scratch build) | `%s` @ `%s` (%s) | `%s` | `%s` | `%s` | %s | %s |" % (os.path.basename(e.get("path") or name), e.get("ref"), e.get("crate"), (e.get("subject") or "")[:60].replace("|", "/"), e.get("sha"), e.get("sha256"), e.get("sha256_content"), e.get("bytes"), e.get("signature")))
    for n, b in sorted(bbins.items()):
        L.append("| `FlashTeX.app/Contents/MacOS/%s` | bundle | — | `%s` | `%s` | %s | %s |" % (n, b.get("sha256"), b.get("sha256_content"), b.get("bytes"), b.get("signature")))
    L.append("")
    L.append("Signature-masked content = sha256 of the thin Mach-O bytes before the `LC_CODE_SIGNATURE` blob with the signature command's offset/size and the `__LINKEDIT` sizes zeroed (`lib/hashes.py`); `codesign --remove-signature` hashes are also in the JSON but are not stable across re-signs. As-shipped sha256 is what a user would hash.")
    L.append("")
    if bundle.get("components_json"):
        L.append("`components.json` written by make-app.sh (its `git_sha` fields come from `git rev-parse` in the pinned scratch clone next to each source binary, so they must equal the short main / branch SHAs above):")
        L.append("")
        L.append("```json")
        L.append(json.dumps(bundle["components_json"], indent=1, sort_keys=True))
        L.append("```")
        L.append("")

    L.append("## Gates")
    L.append("")
    L.append("| section | gate | result | detail |")
    L.append("|---|---|---|---|")
    for s, n, ok, d in gates:
        L.append("| %s | %s | %s | %s |" % (s, n.replace("|", "\\|"), "PASS" if ok else "FAIL", d.replace("|", "\\|").replace("\n", " ")[:220]))
    L.append("")

    L.append("## Typing bench: keystroke -> paint")
    L.append("")
    L.append("Seeds `fixture` / `demo` / `body60k`, 200 typed characters each, at 30 ms (fast typist) and 0 ms (one keystroke per run-loop turn). Producer `flashtex-compiler` = the app's direct worker route with the scratch-built compiler from main; `flashtex-render` = the same direct route with the render-pipeline producer (Latin Modern metrics) from its branch; `flashtex-preview-controller` = the durable helper route (`FLASHTEX_PREVIEW_CONTROLLER`, built from main, owns the ledger and launches the main compiler); the `historical` rows are the same helper route with `FLASHTEX_COMPLETED_SNAPSHOTS=1` (every keystroke its own durable edit, completed older compiles painted labelled). Each cell waited for a quiet machine (bench `--quiet-load`/`--quiet-wait`) and carries the 1-minute load before/after it; a cell above the bench's load limit (10) is reported, not gated. Definitions and limitations: `reports/%s/typing-bench*/typing-bench.md` (written by `tools/typing-bench/run.sh`)." % os.path.basename(rd))
    L.append("")
    for sub in sorted(set(x[1] for x in producers)):
        lb = load_before.get(sub) or {}
        if lb:
            L.append("- `%s` pass: producers `%s`; `uptime` right before: `%s` (each cell then waited for the bench's quiet-load condition; per-cell load before/after is in the table)" % (sub, lb.get("producers"), lb.get("uptime", "").strip()))
    L.append("")
    L.append("| producer | cell | bytes | typed | paints | coalesced | unpainted | k->p p50 | p95 | p99 | max | compile p50 | compile p95 | render p50 | render p95 | gate p50 <= | project target p50 <= %s / p95 <= %s |" % (targets.get("project_typing_to_visible_p50_ms"), targets.get("project_typing_to_visible_p95_ms")))
    L.append("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|")
    for prod, sub, prefix in producers:
        if not present[prod]:
            continue
        pg = tg.get("producers", {}).get(prod, {})
        l1 = load1(sub)
        gated_pass = not (max_load is not None and l1 is not None and l1 > max_load)
        for cell in tg.get("required_cells", []):
            d = runs[prod].get(cell)
            if not d:
                L.append("| %s | %s | — | — | — | — | — | — | — | — | — | — | — | — | — | — | no summary |" % (prod, cell))
                continue
            k = d.get("keystroke_to_paint_ms", {}); c = d.get("compile_ms", {}); r = d.get("render_pass_ms", {})
            seed = cell.rsplit("-", 1)[0]
            lim = pg.get("per_seed_p50_ms_max", {}).get(seed)
            tp50 = targets.get("project_typing_to_visible_p50_ms"); tp95 = targets.get("project_typing_to_visible_p95_ms")
            tmet = (k.get("p50_ms") is not None and tp50 is not None and k["p50_ms"] <= tp50) and (k.get("p95_ms") is not None and tp95 is not None and k["p95_ms"] <= tp95)
            L.append("| %s | %s | %s | %s/%s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |" % (
                (d.get("producer", prod) + (" +historical" if prod == "historical" else "")), cell + (" (retry)" if d.get("_attempt") == "retry" else "") + (" load %s/%s" % (d.get("load_avg_before"), d.get("load_avg_after")) if "load_avg_before" in d else ""), d.get("document_bytes_after"), d.get("typed"), d.get("script_keystrokes"), d.get("paints"), d.get("coalesced"), d.get("unpainted"),
                ms(k.get("p50_ms")), ms(k.get("p95_ms")), ms(k.get("p99_ms")), ms(k.get("max_ms")), ms(c.get("p50_ms")), ms(c.get("p95_ms")), ms(r.get("p50_ms")), ms(r.get("p95_ms")),
                (lim if cell_gated(d, gated_pass) else "%s (not applied: load %s/%s)" % (lim, d.get("load_avg_before", l1), d.get("load_avg_after", "?"))) if lim is not None else "— (not gated)", "met" if tmet else "not met"))
    L.append("")
    for prod, sub, prefix in producers:
        if runs[prod]:
            L.append("Elapsed per cell, %s (ms): %s." % (prod, ", ".join("%s=%s" % (cell, ms(runs[prod][cell].get("elapsed_ms"))) for cell in tg.get("required_cells", []) if cell in runs[prod])))
        if retried[prod]:
            L.append("Cells taken from the retry pass for %s (the first pass lost them when the app process disappeared mid-run — see `logs/%s.log`; other agents run kill/launch checks on this machine): %s." % (prod, sub, ", ".join(retried[prod])))
    L.append("")

    L.append("## Launch check: packaged app, child crash, app survival")
    L.append("")
    if launch and not launch.get("refused"):
        L.append("`apps/mac/scripts/launch-check.sh` (unmodified) ran with `open` shimmed to add `--env FLASHTEX_NO_ACTIVATE=1`, a private `FLASHTEX_BRIDGE_STORE` and `FLASHTEX_TRANSCRIPT`; exact `open` call: `%s`." % "; ".join(launch.get("open_calls", [])))
        L.append("")
        L.append("| observation | value |")
        L.append("|---|---|")
        for k in ("pids", "window_confirmed", "compiler_attached", "compiler_first_result", "app_survived_compiler_kill", "worker_exit_logged",
                  "bridge_attached", "app_survived_bridge_kill", "bridge_exit_logged", "quit_cleanly", "compiler_reattached_after_kill", "bridge_reattached_after_kill", "log_counts", "exit_code"):
            L.append("| %s | `%s` |" % (k, json.dumps(launch.get(k))))
        L.append("")
        L.append("Auto-relaunch of a killed child is not implemented in the shell (`ShellModel` sets `worker = nil` and reports `worker exited`); `*_reattached_after_kill` records what happened and is not a gate. The full evidence, including the app's `FLASHTEX_LOG`, is `reports/%s/launch-check.md`." % os.path.basename(rd))
        L.append("")
        L.append("Launch-check notes:")
        L.append("")
        for n in launch.get("notes", []):
            L.append("- %s" % n)
        for f in launch.get("fail_lines", []):
            L.append("- FAIL: %s" % f)
    elif launch:
        L.append("Refused: %s" % launch.get("reason"))
    else:
        L.append("Not run.")
    L.append("")

    L.append("## Packaged capture cycle through the bundled helpers")
    L.append("")
    if cycle:
        L.append("`lib/capture_cycle.py` drove `FlashTeX.app/Contents/MacOS/{flashtex-bridge,flashtex-edit-ledger,flashtex-compiler,flashtex-pdf}` over their real JSON Lines protocols in the shell's order (no UI scripting: Accessibility is not granted and the window is never activated). Bridge launched as `%s` (no `--enable-grok`; `XAI_API_KEY` absent), so `capture_convert` is refused as `provider_disabled` — the expected refusal — and the proposal review uses the offline fixture `fixtures/capture-proposal.json`, explicitly not a provider result. Transcript: `reports/%s/capture-cycle/transcript.jsonl`." % (" ".join(os.path.basename(x) if i == 0 else x for i, x in enumerate(cycle.get("bridge_argv", []))), os.path.basename(rd)))
        L.append("")
        L.append("| check | result | detail |")
        L.append("|---|---|---|")
        for c in cycle.get("checks", []):
            L.append("| %s | %s | %s |" % (c["name"].replace("|", "\\|"), "PASS" if c["ok"] else "FAIL", c.get("detail", "").replace("|", "\\|").replace("\n", " ")[:200]))
        L.append("")
        ci = cycle.get("capture_image", {}); comp = cycle.get("compile", {}); pdf = cycle.get("pdf", {})
        L.append("- Capture image: `%s`, %s decoded bytes, sha256 `%s` (%s)." % (ci.get("mime_type"), ci.get("decoded_bytes"), ci.get("sha256"), ci.get("source")))
        L.append("- Convert refusal: `%s`." % json.dumps(cycle.get("convert_error")))
        L.append("- Bridge SIGKILL: `%s`; restarted pid %s. Ledger SIGKILL: `%s`." % (json.dumps(cycle.get("bridge_sigkill")), cycle.get("bridge_restart_pid"), json.dumps(cycle.get("ledger_sigkill"))))
        L.append("- Prepared edit (offline review): `%s`; ledger receipt `%s`; durable document after: `%s`." % (json.dumps(cycle.get("prepared_edit")), json.dumps(cycle.get("ledger_receipt")), json.dumps(cycle.get("ledger_document_after"))))
        L.append("- `capture_applied`: %s" % cycle.get("bridge_capture_applied"))
        L.append("- Compile: status `%s`, %s page(s), %s item(s), %s diagnostic(s), round trip %s ms; items excerpt: `%s`." % (comp.get("status"), comp.get("pages"), comp.get("items"), comp.get("diagnostic_count"), ms(comp.get("round_trip_ms")), (comp.get("item_text_excerpt") or "").replace("`", "'")))
        if comp.get("diagnostics"):
            L.append("- Diagnostics: `%s`" % json.dumps(comp.get("diagnostics"))[:600])
        L.append("- PDF: `%s`, %s bytes, sha256 `%s`, header `%s`, %s page object(s), writer %s ms; writer output: `%s`." % (os.path.basename(pdf.get("path") or ""), pdf.get("bytes"), pdf.get("sha256"), pdf.get("header"), pdf.get("page_objects"), ms(pdf.get("export_ms")), (pdf.get("writer_output") or "").strip().replace("\n", " / ").replace("`", "'")[:300]))
        tm = cycle.get("timings_ms", {})
        L.append("- Round trips (ms): %s." % ", ".join("%s=%s" % (k, "/".join(ms(x) for x in v)) for k, v in sorted(tm.items())))
    else:
        L.append("Not run.")
    L.append("")

    L.append("## Packaged render pipeline: File > Attach Render Pipeline, headless")
    L.append("")
    if render_attach and not render_attach.get("refused"):
        L.append("`lib/app_features.py render-attach` executed `FlashTeX.app/Contents/MacOS/FlashTeX` directly with `FLASHTEX_AUTOATTACH=1 FLASHTEX_NO_ACTIVATE=1 FLASHTEX_COMPILER=<bundled flashtex-render>` (what the menu item resolves to) and read `FLASHTEX_LOG`; pid %s, app exit %s after SIGTERM, %s s." % (render_attach.get("pid"), render_attach.get("app_exit"), render_attach.get("elapsed_s")))
        L.append("")
        L.append("| check | result | detail |")
        L.append("|---|---|---|")
        for c in render_attach.get("checks", []):
            L.append("| %s | %s | %s |" % (c["name"].replace("|", "\\|"), "PASS" if c["ok"] else "FAIL", str(c.get("detail", "")).replace("|", "\\|")[:200]))
        L.append("")
        L.append("Log excerpt: `%s`" % " / ".join(render_attach.get("log_excerpt", []))[:900].replace("`", "'"))
    elif render_attach:
        L.append("Refused: %s" % render_attach.get("reason"))
    else:
        L.append("Not run (no bundled flashtex-render).")
    L.append("")
    L.append("## Packaged exact export: flashtex-pdf-exact from-v2 on the checked-in v2 fixture")
    L.append("")
    if exact_export:
        L.append("Command: `%s`; exit %s in %s ms. Fixture `%s` (sha256 `%s`), fonts from `%s`." % (" ".join(os.path.basename(x) if i == 0 else x for i, x in enumerate(exact_export.get("argv", []))), exact_export.get("exit"), ms(exact_export.get("ms")), os.path.basename((exact_export.get("fixture") or {}).get("path") or ""), (exact_export.get("fixture") or {}).get("sha256"), exact_export.get("font_dir")))
        L.append("")
        L.append("| check | result | detail |")
        L.append("|---|---|---|")
        for c in exact_export.get("checks", []):
            L.append("| %s | %s | %s |" % (c["name"].replace("|", "\\|"), "PASS" if c["ok"] else "FAIL", str(c.get("detail", "")).replace("|", "\\|")[:200]))
        L.append("")
        pk = exact_export.get("pdfkit") or {}
        pdf = exact_export.get("pdf") or {}
        L.append("- PDF: %s bytes, sha256 `%s`; PDFKit: %s page(s), page size %s pt; extracted text: `%s`." % (pdf.get("bytes"), pdf.get("sha256"), pk.get("pages"), pk.get("page_size_pt"), (exact_export.get("pdfkit_text") or "").replace("`", "'")[:300]))
        if exact_export.get("stderr_tail"):
            L.append("- Tool stderr: `%s`" % exact_export["stderr_tail"].strip().replace("\n", " / ").replace("`", "'")[:400])
    else:
        L.append("Not run (no bundled flashtex-pdf-exact).")
    L.append("")
    L.append("## Historical previews: helper route, hold-until-preview vs FLASHTEX_COMPLETED_SNAPSHOTS=1")
    L.append("")
    if hist_analysis:
        L.append("Classification by the branch's `docs/evidence/historical-preview-2026-09-12T1010Z/analyze.py` (copied to `reports/%s/historical/`; its inputs were copies of the `controller-*` JSON + `FLASHTEX_LOG` of the two passes, see `historical/README.txt`), from each cell's `FLASHTEX_LOG`: a paint is *historical* when the log shows `compile: applied historical revision N` for the revision the bench recorded as `painted_by_revision`, else *current*. `baseline` = the `controller` rows above; `historical` = the `FLASHTEX_COMPLETED_SNAPSHOTS=1` pass. Full output: `reports/%s/historical/analysis.txt`." % (os.path.basename(rd), os.path.basename(rd)))
        L.append("")
        for line in hist_analysis.splitlines():
            if line.startswith("|"):
                L.append(line)
    else:
        L.append("Not run.")
    L.append("")
    L.append("## Secondary windows opened headlessly and captured by window id")
    L.append("")
    for wid, rec in windows.items():
        if rec is None:
            L.append("- `%s`: not run." % wid)
            continue
        if rec.get("refused"):
            L.append("- `%s`: refused (%s)." % (wid, rec.get("reason")))
            continue
        sc = rec.get("screenshot") or {}
        L.append("- `FLASHTEX_OPEN_WINDOW=%s` (expected title %r): windows owned by pid %s after %s s: %s; screenshot `%s` (%s bytes, %sx%s px, screencapture exit %s)." % (
            wid, rec.get("expected_title"), rec.get("pid"), rec.get("seen_after_s"), json.dumps([(w.get("id"), w.get("name")) for w in rec.get("windows", [])]),
            os.path.relpath(sc.get("path", ""), rd) if sc.get("path") else "none", sc.get("bytes"), sc.get("width"), sc.get("height"), sc.get("screencapture_exit")))
        for c in rec.get("checks", []):
            L.append("  - %s: %s" % ("PASS" if c["ok"] else "FAIL", c["name"]))
    L.append("")
    L.append("## Worker crash auto-relaunch (bundled compiler, SIGKILL x4)")
    L.append("")
    if relaunch and not relaunch.get("refused"):
        L.append("The app (pid %s, `FLASHTEX_AUTOATTACH=1 FLASHTEX_NO_ACTIVATE=1`) had its `flashtex-compiler` child killed with SIGKILL four times within one minute; the shell relaunches at most 3 times per minute with 0.2 / 1.0 / 3.0 s backoff, then leaves the exit visible." % relaunch.get("pid"))
        L.append("")
        L.append("| kill | killed pid | relaunch scheduled after (s) | relaunched after (s) | new child | a later keystroke recompiled after (s) | elapsed (s) | limit line after (s) |")
        L.append("|---:|---:|---:|---:|---|---|---:|---:|")
        for k in relaunch.get("kills", []):
            L.append("| %s | %s | %s | %s | %s | %s | %s | %s |" % (k.get("attempt"), k.get("killed_pid"), k.get("scheduled_after_s", "—"), k.get("relaunched_after_s", "—"), k.get("new_child", k.get("child_after", "—")), k.get("recompiled_after_s", "—"), k.get("elapsed_s", "—"), k.get("limit_after_s", "—")))
        L.append("")
        for c in relaunch.get("checks", []):
            L.append("- %s: %s — %s" % ("PASS" if c["ok"] else "FAIL", c["name"], str(c.get("detail", ""))[:200]))
        L.append("")
        L.append("Log excerpt: `%s`" % " / ".join(relaunch.get("log_excerpt", []))[:1200].replace("`", "'"))
    elif relaunch:
        L.append("Refused: %s" % relaunch.get("reason"))
    else:
        L.append("Not run.")
    L.append("")
    L.append("## Multi-file project through the helper (main.tex + \\input{chapter})")
    L.append("")
    if multifile and not multifile.get("refused"):
        L.append("A temp project (`main.tex` with `\\input{chapter}`, `chapter.tex`) opened in the packaged app on the helper route with `FLASHTEX_OPEN_INCLUDES=1 FLASHTEX_ACTIVE_PATH=chapter.tex`; the typing bench typed `%s` into the active editor (`FLASHTEX_TYPING_BENCH_APPEND=1`), the app exited on its own; pid %s, exit %s. Bench: `%s`. Disk sha256 before/after: `%s`. Helper ledger documents: `%s`." % (
            multifile.get("typed"), multifile.get("pid"), multifile.get("app_exit"), json.dumps(multifile.get("bench")), json.dumps(multifile.get("disk")), json.dumps(multifile.get("ledger_documents"))))
        L.append("")
        for c in multifile.get("checks", []):
            L.append("- %s: %s — %s" % ("PASS" if c["ok"] else "FAIL", c["name"], str(c.get("detail", ""))[:200].replace("|", "/")))
        L.append("")
        L.append("Log excerpt: `%s`" % " / ".join(multifile.get("log_excerpt", []))[:1500].replace("`", "'"))
        L.append("")
        L.append("Save routing (⌘S to the member, entry untouched), quit-save of dirty members, reviewed reload after an external edit and the on-disk conflict refusal need the Save menu / quit alert / reload sheet, which cannot be driven without Accessibility; they are covered by the branch's XCTests below, run against the real helpers.")
    elif multifile:
        L.append("Refused: %s" % multifile.get("reason"))
    else:
        L.append("Not run.")
    L.append("")
    L.append("## Branch XCTests against the real helpers (save routing, quit-save, reviewed reload, conflict refusal, bounded relaunch)")
    L.append("")
    if app_tests:
        L.append("`swift test --filter 'ProjectDocumentsTests|DocumentFilesTests|DocumentFilesControllerTests|HistoricalPreviewTests|ShellModelWorkerTests/testCrashedWorkerIsRelaunchedWithBoundedBackoff'` in the pinned app clone with `FLASHTEX_COMPILER`, `FLASHTEX_PREVIEW_CONTROLLER`, `FLASHTEX_PROJECT_FILES`, `FLASHTEX_EDIT_LEDGER`, `FLASHTEX_BRIDGE`, `FLASHTEX_PDF` pointing at the binaries above: exit %s; %s passed, %s failed, %s skipped; totals `%s`." % (app_tests.get("exit_code"), app_tests.get("passed"), app_tests.get("failed"), app_tests.get("skipped"), json.dumps(app_tests.get("totals"))))
        L.append("")
        L.append("| test | result | seconds | note |")
        L.append("|---|---|---:|---|")
        for name, c in sorted(app_tests.get("cases", {}).items()):
            L.append("| `%s` | %s | %s | %s |" % (name, c.get("result"), c.get("seconds"), (c.get("skip_reason") or "; ".join(c.get("failures", []))).replace("|", "/")[:200]))
    else:
        L.append("Not run.")
    L.append("")
    L.append("## Steps and exact commands")
    L.append("")
    L.append("| step | exit | seconds | log |")
    L.append("|---|---:|---:|---|")
    for s in steps:
        L.append("| %s | %s | %s | `%s` |" % (s.get("step"), s.get("exit"), s.get("seconds"), s.get("log")))
    L.append("")
    L.append("```")
    L.append(commands)
    L.append("```")
    L.append("")
    L.append("## Limitations")
    L.append("")
    L.append("- The typing bench inserts text programmatically (`insertText`), so the OS keyboard/event-queue hop is not measured, and `paint` is the completed CoreAnimation commit rather than display scan-out (see the bench's own methodology and limitations). One run per cell; percentiles are nearest-rank over 200 keystrokes; concurrent load on this machine (other agents' builds/tests) is recorded above but not controlled.")
    L.append("- The launch check exercises child crash and app survival; the shell has no automatic child relaunch, so 'restart' here means the app keeps running and reports the exit — reattachment is a user action.")
    L.append("- The capture cycle drives the packaged app's bundled helpers, not the app's menus/sheets (Accessibility is not granted; the window is never activated). It therefore proves the helpers and the shell's protocol sequence end-to-end, not the SwiftUI review sheet itself; the shell's own review/insertion logic is covered by `swift test` (`RealBridgeTests`, `ShellModelBridgeTests`) on the branch.")
    L.append("- No Grok/provider call is made anywhere; the proposal reviewed is a fixture. A real conversion needs `--enable-grok` plus the Mac credential adapter and is outside this runner by design.")
    L.append("- make-app.sh re-signs the bundle ad hoc, so as-shipped hashes of the bundled binaries differ from the built ones by the signature blob; the signature-masked content hashes are compared instead. The helpers are built in a shared clone pinned at the main SHA and the app in one pinned at the branch SHA, so `components.json` carries the real short SHAs.")
    L.append("")
    open(a.out, "w", encoding="utf-8").write("\n".join(L) + "\n")
    print("%s: %d gates, %d failed -> %s" % (verdict, len(gates), len(failed), a.out))
    for s, n, ok, d in failed:
        print("  FAIL [%s] %s: %s" % (s, n, d[:160]))
    return 0 if not failed else 1


if __name__ == "__main__":
    sys.exit(main())
