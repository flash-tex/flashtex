#!/usr/bin/env python3
"""Summarises an apps/mac/scripts/launch-check.sh evidence file as JSON.

Reads the Markdown evidence (notes, FAIL lines and the embedded FLASHTEX_LOG),
plus the `open` shim's call log, and records what the packaged app did:
attach/kill/survive for the compiler and bridge children, whether the launch
carried FLASHTEX_NO_ACTIVATE=1, and whether any child was re-attached after
its kill (the shell does not auto-relaunch; recorded, not gated). Stdlib only.
"""
import argparse
import json
import re


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--evidence", required=True)
    ap.add_argument("--open-log", required=True)
    ap.add_argument("--exit", type=int, required=True)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()

    text = open(a.evidence, encoding="utf-8", errors="replace").read() if a.evidence else ""
    notes = [l[2:].strip() for l in text.splitlines() if l.startswith("- ") and not l.startswith("- FAIL:")]
    fails = [l[len("- FAIL:"):].strip() for l in text.splitlines() if l.startswith("- FAIL:")]
    log = ""
    m = re.search(r"## FLASHTEX_LOG \((.*?)\)\n\n```\n(.*?)```", text, re.S)
    if m:
        log = m.group(2)
    log_lines = log.splitlines()
    pids = {}
    for n in notes:
        mm = re.search(r"(flashtex-[a-z]+) attached, pid=(\d+)", n)
        if mm:
            pids[mm.group(1)] = int(mm.group(2))
        mm = re.search(r"FlashTeX running, pid=(\d+)", n)
        if mm:
            pids["FlashTeX"] = int(mm.group(1))

    def count(sub):
        return sum(1 for l in log_lines if sub in l)

    # the engine the shell said typesets the document (EngineChoice.logLine,
    # retirement plan #1236 S3r); None for an app that logged no engine
    engines = [m.group(1) for m in (re.search(r"engine: (new|previous) \(", l) for l in log_lines) if m]

    open_calls = open(a.open_log, encoding="utf-8").read().strip().splitlines() if a.open_log else []
    no_activate = any("FLASHTEX_NO_ACTIVATE=1" in c for c in open_calls)
    # Relaunch evidence: a second compiler/bridge attach after the kill lines.
    worker_exit_idx = next((i for i, l in enumerate(log_lines) if "worker exited" in l), None)
    bridge_exit_idx = next((i for i, l in enumerate(log_lines) if "bridge exited" in l), None)
    compiler_reattached = worker_exit_idx is not None and any("status: attached:" in l for l in log_lines[worker_exit_idx + 1:])
    bridge_reattached = bridge_exit_idx is not None and any("bridge: attached:" in l for l in log_lines[bridge_exit_idx + 1:])
    summary = {
        "exit_code": a.exit,
        "notes": notes,
        "fail_lines": fails,
        "pids": pids,
        "open_calls": open_calls,
        "launched_with_no_activate": no_activate,
        "window_confirmed": any(n.startswith("CGWindowListCopyWindowInfo confirms") for n in notes),
        "compiler_attached": any("flashtex-compiler attached" in n for n in notes),
        "compiler_first_result": any("revision 1: ok" in n for n in notes),
        "app_survived_compiler_kill": any("still running after its compiler child was killed" in n for n in notes),
        "worker_exit_logged": any("'worker exited (' status line" in n for n in notes),
        "bridge_attached": any("flashtex-bridge attached" in n for n in notes),
        "app_survived_bridge_kill": any("still running after its bridge child was killed" in n for n in notes),
        "bridge_exit_logged": any("'bridge exited (' status line" in n for n in notes),
        "quit_cleanly": any(n == "FlashTeX quit cleanly" for n in notes),
        "compiler_reattached_after_kill": compiler_reattached,
        "bridge_reattached_after_kill": bridge_reattached,
        "engine": engines[-1] if engines else None,
        "log_line_count": len(log_lines),
        "log_counts": {
            "status: attached:": count("status: attached:"),
            "revision 1: ok": count("revision 1: ok"),
            "worker exited": count("worker exited"),
            "bridge: attached:": count("bridge: attached:"),
            "bridge exited": count("bridge exited"),
            "engine:": count("engine: "),
        },
        "log_excerpt": [l for l in log_lines if any(k in l for k in ("attached", "exited", "revision 1", "bridge", "ledger"))][:40],
    }
    json.dump(summary, open(a.out, "w"), indent=1, sort_keys=True)
    print("launch-check: fails=%d compiler=%s/%s bridge=%s/%s no_activate=%s quit=%s" % (
        len(fails), summary["compiler_attached"], summary["app_survived_compiler_kill"],
        summary["bridge_attached"], summary["app_survived_bridge_kill"], no_activate, summary["quit_cleanly"]))


if __name__ == "__main__":
    main()
