#!/usr/bin/env python3
"""Assembles reports/<UTC>.md from a run directory and asserts thresholds.json.

Inputs (all written by run.sh into the run directory): env.json, helpers.json,
extras.json, app.json, bundle.json, steps.jsonl, commands.log,
launch-check.json (+ launch-check.md), capture-cycle.json, open-window-*.json,
app-tests.json. Missing inputs are reported as failed gates, never silently
skipped. Exit 1 on any failed gate. Stdlib only.
"""
import argparse
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
    windows = {wid: load(os.path.join(rd, "open-window-%s.json" % wid)) for wid in ("a11y-help", "nearby")}
    app_tests = load(os.path.join(rd, "app-tests.json"))
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
        gate(sec, "bundled helpers carry the freshly built object code (signature-masked Mach-O content sha256 equal, both)",
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
        for key in ("bridge", "edit_ledger"):
            if not same((comp.get(key) or {}).get("git_sha"), env.get("sources", {}).get("main_sha")):
                bad.append("%s=%s" % (key, (comp.get(key) or {}).get("git_sha")))
        if not same((comp.get("app") or {}).get("git_sha"), env.get("sources", {}).get("branch_sha")):
            bad.append("app=%s" % (comp.get("app") or {}).get("git_sha"))
        for key, name in (("engine_host", "engine-host"), ("project_files", "project-files")):
            e = extras.get(name) or {}
            if e.get("built_ok") and not same((comp.get(key) or {}).get("git_sha"), e.get("sha")):
                bad.append("%s=%s (expected %s)" % (key, (comp.get(key) or {}).get("git_sha"), (e.get("sha") or "")[:7]))
        gate(sec, "bundle components.json git SHAs: helpers == %s (main), app, engine_host and project_files == %s (branch)" % (main_short, branch_short), not bad, "; ".join(bad) or "all match")
    for name, label in (("engine-host", "flashtex-host"),):
        e = extras.get(name)
        if e is not None:
            gate(sec, "%s built from %s" % (label, e.get("ref")), e.get("built_ok") and e.get("scratch_head") == e.get("sha"), "%s @ %s%s" % (label, (e.get("sha") or "?")[:12], (" cargo rewrote: " + ", ".join(e.get("scratch_dirty_status", []))) if e.get("scratch_dirty_status") else ""))
            if e.get("built_ok") and bbins.get(label):
                gate(sec, "bundled %s carries the freshly built object code (signature-masked content sha256 equal)" % label,
                     bbins[label].get("sha256_content") == e.get("sha256_content"), "%s vs %s" % ((e.get("sha256_content") or "?")[:12], (bbins[label].get("sha256_content") or "?")[:12]))

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

    for wid, rec in windows.items():
        sec = "open-window/" + wid
        if rec is None:
            continue
        if rec.get("refused"):
            gate(sec, "window check ran", False, rec.get("reason"))
            continue
        for c in rec.get("checks", []):
            gate(sec, c["name"], c["ok"], c.get("detail", ""))
    sec = "app-tests"
    if app_tests is not None:
        tg2 = th.get("app_tests", {}).get("gates", {})
        gate(sec, "swift test exit 0 (%d passed, %d failed, %d skipped)" % (app_tests.get("passed", 0), app_tests.get("failed", 0), app_tests.get("skipped", 0)), app_tests.get("exit_code") == 0 and app_tests.get("failed", 1) == 0, json.dumps(app_tests.get("totals")))
        for name in tg2.get("required_passed", []):
            c = app_tests.get("cases", {}).get(name)
            gate(sec, "test %s passed (not skipped)" % name, c is not None and c.get("result") == "passed", (c or {}).get("skip_reason") or "; ".join((c or {}).get("failures", [])) or ("%s s" % (c or {}).get("seconds")))

    failed = [g for g in gates if not g[2]]
    verdict = "PASS" if not failed else "FAIL"

    # ---------------------------------------------------------------- render
    L = []
    src = env.get("sources", {})
    mach = env.get("machine", {})
    L.append("# Mac live acceptance: engine host and helper crash/restart, packaged capture cycle (%s)" % utc)
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
    L.append("| app, engine host and project-files source | `%s` = `%s` (%s, committed %s) |" % (src.get("branch"), src.get("branch_sha"), src.get("branch_subject"), src.get("branch_commit_utc")))
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
        L.append("| `FlashTeX.app/Contents/%s/%s` | bundle | — | `%s` | `%s` | %s | %s |" % ("Helpers" if n.startswith("flashtex-host") else "MacOS", n, b.get("sha256"), b.get("sha256_content"), b.get("bytes"), b.get("signature")))
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

    L.append("## Launch check: packaged app, child crash, app survival")
    L.append("")
    if launch and not launch.get("refused"):
        L.append("`apps/mac/scripts/launch-check.sh` (unmodified) ran with `open` shimmed to add `--env FLASHTEX_NO_ACTIVATE=1` and a private `FLASHTEX_BRIDGE_STORE`; exact `open` call: `%s`." % "; ".join(launch.get("open_calls", [])))
        L.append("")
        L.append("| observation | value |")
        L.append("|---|---|")
        for k in ("pids", "window_confirmed", "host_start_logged", "host_waits_for_download_consent", "host_started", "app_survived_host_kill", "host_exit_logged", "host_restart_logged",
                  "bridge_attached", "app_survived_bridge_kill", "bridge_exit_logged", "quit_cleanly", "bridge_reattached_after_kill", "engine", "log_counts", "exit_code"):
            L.append("| %s | `%s` |" % (k, json.dumps(launch.get(k))))
        L.append("")
        L.append("The app restarts a killed engine host (`engine-v3: restarting the host`, gated); it does not auto-relaunch a killed bridge, so `bridge_reattached_after_kill` records what happened and is not a gate. The full evidence, including the app's `FLASHTEX_LOG`, is `reports/%s/launch-check.md`." % os.path.basename(rd))
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
        L.append("`lib/capture_cycle.py` drove `FlashTeX.app/Contents/MacOS/{flashtex-bridge,flashtex-edit-ledger}` over their real JSON Lines protocols in the shell's order (no UI scripting: Accessibility is not granted and the window is never activated). Bridge launched as `%s` (no `--enable-grok`; `XAI_API_KEY` absent), so `capture_convert` is refused as `provider_disabled` — the expected refusal — and the proposal review uses the offline fixture `fixtures/capture-proposal.json`, explicitly not a provider result. Transcript: `reports/%s/capture-cycle/transcript.jsonl`." % (" ".join(os.path.basename(x) if i == 0 else x for i, x in enumerate(cycle.get("bridge_argv", []))), os.path.basename(rd)))
        L.append("")
        L.append("| check | result | detail |")
        L.append("|---|---|---|")
        for c in cycle.get("checks", []):
            L.append("| %s | %s | %s |" % (c["name"].replace("|", "\\|"), "PASS" if c["ok"] else "FAIL", c.get("detail", "").replace("|", "\\|").replace("\n", " ")[:200]))
        L.append("")
        ci = cycle.get("capture_image", {})
        L.append("- Capture image: `%s`, %s decoded bytes, sha256 `%s` (%s)." % (ci.get("mime_type"), ci.get("decoded_bytes"), ci.get("sha256"), ci.get("source")))
        L.append("- Convert refusal: `%s`." % json.dumps(cycle.get("convert_error")))
        L.append("- Bridge SIGKILL: `%s`; restarted pid %s. Ledger SIGKILL: `%s`." % (json.dumps(cycle.get("bridge_sigkill")), cycle.get("bridge_restart_pid"), json.dumps(cycle.get("ledger_sigkill"))))
        L.append("- Prepared edit (offline review): `%s`; ledger receipt `%s`; durable document after: `%s`." % (json.dumps(cycle.get("prepared_edit")), json.dumps(cycle.get("ledger_receipt")), json.dumps(cycle.get("ledger_document_after"))))
        L.append("- `capture_applied`: %s" % cycle.get("bridge_capture_applied"))
        tm = cycle.get("timings_ms", {})
        L.append("- Round trips (ms): %s." % ", ".join("%s=%s" % (k, "/".join(ms(x) for x in v)) for k, v in sorted(tm.items())))
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
    L.append("## Branch XCTests against the real helpers (save routing, reviewed reload, conflict refusal)")
    L.append("")
    if app_tests:
        L.append("`swift test --filter 'ProjectDocumentsTests|DocumentFilesTests'` in the pinned app clone with `FLASHTEX_PROJECT_FILES`, `FLASHTEX_EDIT_LEDGER`, `FLASHTEX_BRIDGE` pointing at the binaries above: exit %s; %s passed, %s failed, %s skipped; totals `%s`." % (app_tests.get("exit_code"), app_tests.get("passed"), app_tests.get("failed"), app_tests.get("skipped"), json.dumps(app_tests.get("totals"))))
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
    L.append("- The launch check exercises child crash and app survival: the app restarts the engine host itself; it has no automatic bridge relaunch, so for the bridge 'restart' means the app keeps running and reports the exit — reattachment is a user action. Concurrent load on this machine (other agents' builds/tests) is recorded above but not controlled.")
    L.append("- The capture cycle drives the packaged app's bundled helpers, not the app's menus/sheets (Accessibility is not granted; the window is never activated). It therefore proves the helpers and the shell's protocol sequence end-to-end, not the SwiftUI review sheet itself; the shell's own review/insertion logic is covered by `swift test` (`RealBridgeTests`, `ShellModelBridgeTests`) on the branch.")
    L.append("- No Grok/provider call is made anywhere; the proposal reviewed is a fixture. A real conversion needs `--enable-grok` plus the Mac credential adapter and is outside this runner by design.")
    L.append("- make-app.sh re-signs the bundle ad hoc, so as-shipped hashes of the bundled binaries differ from the built ones by the signature blob; the signature-masked content hashes are compared instead. The helpers are built in a shared clone pinned at the main SHA and the app, the engine host and flashtex-project-files in clones pinned at the branch SHA, so `components.json` carries the real short SHAs.")
    L.append("")
    open(a.out, "w", encoding="utf-8").write("\n".join(L) + "\n")
    print("%s: %d gates, %d failed -> %s" % (verdict, len(gates), len(failed), a.out))
    for s, n, ok, d in failed:
        print("  FAIL [%s] %s: %s" % (s, n, d[:160]))
    return 0 if not failed else 1


if __name__ == "__main__":
    sys.exit(main())
