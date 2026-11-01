#!/usr/bin/env python3
"""Byte-transparent stdio proxy in front of the REAL `flashtex-preview-controller`
that makes one reply deterministically slow (SearchReconcileTests, GH39).

Launch exactly like the helper: `holding_preview_controller_proxy.py CONFIG.json`.
Environment:
  FLASHTEX_PREVIEW_CONTROLLER   the real helper to run (required)
  FLASHTEX_HOLD_TYPE            request type whose reply is held (e.g. apply_group)
  FLASHTEX_HOLD_RELEASE         path; the hold ends when this file exists
  FLASHTEX_HOLD_MARK            path; created once the held request was forwarded
  FLASHTEX_HOLD_WIRE            path; every relayed line is appended, prefixed
                                `>` (app→helper) / `<` (helper→app, at forward time)

Every app→helper line is forwarded unchanged (the helper still processes the
held request immediately, so its ledger moves while the app has no reply).
From the moment a request of FLASHTEX_HOLD_TYPE is forwarded, EVERY helper→app
line is queued in order until the release file exists, then flushed in that
order — the pipe is slow, nothing is reordered or rewritten. Only the first
request of that type is held per launch. The helper's exit ends the proxy with
the same status; the proxy's death closes the helper's stdin.
"""
import json
import os
import subprocess
import sys
import threading
import time


def main():
    helper = os.environ["FLASHTEX_PREVIEW_CONTROLLER"]
    hold_type = os.environ.get("FLASHTEX_HOLD_TYPE")
    release = os.environ.get("FLASHTEX_HOLD_RELEASE")
    mark = os.environ.get("FLASHTEX_HOLD_MARK")
    wire = open(os.environ["FLASHTEX_HOLD_WIRE"], "ab", buffering=0) if os.environ.get("FLASHTEX_HOLD_WIRE") else None

    def trace(prefix, line):
        if wire:
            wire.write(prefix + line[:400] + (b"...\n" if len(line) > 400 else b""))
    child = subprocess.Popen([helper] + sys.argv[1:], stdin=subprocess.PIPE, stdout=subprocess.PIPE, bufsize=0)
    lock = threading.Lock()
    state = {"holding": False, "held_once": False, "queue": []}
    out = sys.stdout.buffer
    stdin = sys.stdin.buffer

    def upstream():
        try:
            for line in iter(stdin.readline, b""):
                child.stdin.write(line)
                child.stdin.flush()
                trace(b"> ", line)
                if hold_type and not state["held_once"]:
                    try:
                        frame = json.loads(line)
                    except ValueError:
                        frame = None
                    if isinstance(frame, dict) and frame.get("type") == hold_type:
                        with lock:
                            state["holding"] = True
                            state["held_once"] = True
                        if mark:
                            with open(mark, "w") as f:
                                f.write(str(frame.get("id", "")))
        except (BrokenPipeError, OSError):
            pass
        finally:
            try:
                child.stdin.close()
            except OSError:
                pass

    def flush_locked():
        for queued in state["queue"]:
            out.write(queued)
            trace(b"< ", queued)
        state["queue"] = []
        out.flush()

    def downstream():
        for line in iter(child.stdout.readline, b""):
            with lock:
                if state["holding"]:
                    state["queue"].append(line)
                    continue
                out.write(line)
                out.flush()
                trace(b"< ", line)

    def releaser():
        while child.poll() is None:
            time.sleep(0.005)
            with lock:
                if state["holding"] and release and os.path.exists(release):
                    state["holding"] = False
                    flush_locked()

    threads = [threading.Thread(target=upstream, daemon=True), threading.Thread(target=downstream),
               threading.Thread(target=releaser, daemon=True)]
    for t in threads:
        t.start()
    threads[1].join()
    code = child.wait()
    with lock:
        state["holding"] = False
        flush_locked()
    sys.exit(code)


if __name__ == "__main__":
    main()
