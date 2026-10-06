#!/usr/bin/env python3
"""Run a command with a wall-clock limit, killing its whole process group.

    run-with-timeout.py SECONDS [--log FILE] -- COMMAND [ARG...]

COMMAND inherits stdin, stdout and stderr, and runs in a session (process
group) of its own. Past SECONDS the group gets SIGTERM, then SIGKILL after 5
seconds, so a child that ignores SIGTERM, or one the command started, cannot
keep running. Exits with COMMAND's status, 128+N if a signal N killed it, or
124 on a timeout; with --log, a timeout also appends one line naming the
command to FILE, so a caller that ignores the status (TeX's trip runs end in
errors on purpose) can still tell afterwards that a run was cut off.

Used by scripts/flashtex-trip.sh and scripts/flashtex-etrip.sh (#1208). A
portable `timeout`: macOS has none. Python 3 standard library only.
"""
import os
import signal
import subprocess
import sys


def main(argv):
    if len(argv) < 3:
        print(__doc__.strip().splitlines()[2].strip(), file=sys.stderr)
        return 2
    try:
        limit = float(argv[0])
    except ValueError:
        print("run-with-timeout: SECONDS must be a number, not %r" % argv[0],
              file=sys.stderr)
        return 2
    if limit <= 0:
        print("run-with-timeout: SECONDS must be positive", file=sys.stderr)
        return 2
    rest = argv[1:]
    log = None
    if rest[0] == "--log":
        if len(rest) < 2:
            print("run-with-timeout: --log needs a file", file=sys.stderr)
            return 2
        log, rest = rest[1], rest[2:]
    if not rest or rest[0] != "--" or len(rest) < 2:
        print("run-with-timeout: expected -- COMMAND", file=sys.stderr)
        return 2
    cmd = rest[1:]
    try:
        proc = subprocess.Popen(cmd, start_new_session=True)
    except OSError as exc:
        print("run-with-timeout: cannot run %s: %s" % (cmd[0], exc),
              file=sys.stderr)
        return 127
    try:
        rc = proc.wait(timeout=limit)
        return 128 - rc if rc < 0 else rc
    except subprocess.TimeoutExpired:
        pass
    # SIGTERM first so the command can clean up, then SIGKILL the group
    # whether or not the command itself has exited: a grandchild it started
    # may have outlived it.
    try:
        os.killpg(proc.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        proc.wait(timeout=5.0)
    except subprocess.TimeoutExpired:
        pass
    try:
        os.killpg(proc.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    proc.wait()
    msg = "timed out after %gs: %s" % (limit, " ".join(cmd))
    print("run-with-timeout: " + msg, file=sys.stderr)
    if log:
        with open(log, "a") as fh:
            fh.write(msg + "\n")
    return 124


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
