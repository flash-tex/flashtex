#!/usr/bin/env python3
"""Summarises an apps/mac/scripts/launch-check.sh evidence file as JSON.

Reads the Markdown evidence (notes, FAIL lines and the embedded FLASHTEX_LOG),
plus the `open` shim's call log, and records what the packaged app did:
start/kill/survive for the engine host and bridge children, the host's logged
restart, whether the launch carried FLASHTEX_NO_ACTIVATE=1, and whether the
bridge was re-attached after its kill (the shell does not auto-relaunch it;
recorded, not gated). Stdlib only.
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
        mm = re.search(r"(flashtex-[a-z]+) (?:attached|started), pid=(\d+)", n)
        if mm:
            pids[mm.group(1)] = int(mm.group(2))
        mm = re.search(r"FlashTeX running, pid=(\d+)", n)
        if mm:
            pids["FlashTeX"] = int(mm.group(1))

    def count(sub):
        return sum(1 for l in log_lines if sub in l)

    # the engine the shell said typesets the document (EngineChoice.logLine,
    # `engine: new (<classic|unicode>) <file>`); None for an app that logged no
    # engine. The old engine's label (`engine: previous (…)`) left with it in
    # retirement stage S6 (#1236) and is not attributed.
    engines = [m.group(1) for m in (re.search(r"engine: (new) \(", l) for l in log_lines) if m]

    open_calls = open(a.open_log, encoding="utf-8").read().strip().splitlines() if a.open_log else []
    no_activate = any("FLASHTEX_NO_ACTIVATE=1" in c for c in open_calls)
    # Relaunch evidence: a second bridge attach after the kill lines.
    bridge_exit_idx = next((i for i, l in enumerate(log_lines) if "bridge exited" in l), None)
    bridge_reattached = bridge_exit_idx is not None and any("bridge: attached:" in l for l in log_lines[bridge_exit_idx + 1:])
    summary = {
        "exit_code": a.exit,
        "notes": notes,
        "fail_lines": fails,
        "pids": pids,
        "open_calls": open_calls,
        "launched_with_no_activate": no_activate,
        "window_confirmed": any(n.startswith("CGWindowListCopyWindowInfo confirms") for n in notes),
        "host_start_logged": any(n.startswith("log shows the engine host starting") for n in notes),
        "host_waits_for_download_consent": any(n.startswith("no TeX Live: the engine waits") for n in notes),
        "host_started": any("flashtex-host started" in n for n in notes),
        "app_survived_host_kill": any("still running after its engine host was killed" in n for n in notes),
        "host_exit_logged": any(n.startswith("log shows the host's exit") for n in notes),
        "host_restart_logged": any(n.startswith("log shows the host restarting") for n in notes),
        "bridge_attached": any("flashtex-bridge attached" in n for n in notes),
        "app_survived_bridge_kill": any("still running after its bridge child was killed" in n for n in notes),
        "bridge_exit_logged": any("'bridge exited (' status line" in n for n in notes),
        "quit_cleanly": any(n.startswith("FlashTeX quit cleanly") for n in notes),
        "bridge_reattached_after_kill": bridge_reattached,
        "engine": engines[-1] if engines else None,
        "log_line_count": len(log_lines),
        "log_counts": {
            "engine-v3: starting": count("engine-v3: starting"),
            "engine-v3: host exited": sum(1 for l in log_lines if re.search(r"engine-v3: host \d+ exited with status", l)),
            "engine-v3: restarting the host": count("engine-v3: restarting the host"),
            "bridge: attached:": count("bridge: attached:"),
            "bridge exited": count("bridge exited"),
            "engine:": count("engine: "),
        },
        "log_excerpt": [l for l in log_lines if any(k in l for k in ("attached", "exited", "engine-v3: starting", "restarting", "bridge", "ledger"))][:40],
    }
    json.dump(summary, open(a.out, "w"), indent=1, sort_keys=True)
    print("launch-check: fails=%d host=%s/%s bridge=%s/%s no_activate=%s quit=%s" % (
        len(fails), summary["host_started"], summary["app_survived_host_kill"],
        summary["bridge_attached"], summary["app_survived_bridge_kill"], no_activate, summary["quit_cleanly"]))


if __name__ == "__main__":
    main()
