"""Run TeX Live's makeindex (the oracle) and FlashTeX's port on the same
inputs and compare everything they produce, byte for byte: the exit status,
standard output, standard error, and every file in the working directory
afterwards (the .ind, the .ilg, and anything else).

Each run gets its own fresh directory holding exactly the case's input
files, and runs with that directory as its working directory, so names in
messages are identical. Python 3 standard library only.
"""

import os
import shutil
import subprocess
import tempfile

ORACLE = "/Library/TeX/texbin/makeindex"


def run_one(binary, files, args, stdin=None, env=None, timeout=120):
    d = tempfile.mkdtemp(prefix="mki-")
    try:
        for name, data in files.items():
            p = os.path.join(d, name)
            os.makedirs(os.path.dirname(p), exist_ok=True)
            with open(p, "wb") as f:
                f.write(data)
        argv0 = binary
        if binary == ORACLE:
            # As restricted \write18 runs it (/bin/sh -c "makeindex ..."):
            # argv[0] is `makeindex`, found along PATH, and kpathsea names
            # the program by it in its messages ("makeindex: Not writing
            # to ...").
            argv0 = "makeindex"
            env = dict(env if env is not None else os.environ)
            env["PATH"] = os.path.dirname(ORACLE) + os.pathsep + env.get("PATH", "")
        try:
            p = subprocess.run(
                [argv0] + list(args),
                executable=binary,
                cwd=d,
                input=stdin if stdin is not None else b"",
                capture_output=True,
                env=env,
                timeout=timeout,
            )
            status, so, se = p.returncode, p.stdout, p.stderr
        except subprocess.TimeoutExpired as e:
            # A run that never ends (the C program's find_pageno on some
            # logs): what it wrote before it was killed is compared.
            status, so, se = "never ends", e.stdout or b"", e.stderr or b""
        # The port's command line exits with 125 where it proved that the C
        # program would never end (flashtex_makeindex::LOOPS_FOREVER).
        if status == 125 and binary != ORACLE:
            status = "never ends"
        out = {}
        for root, _, names in os.walk(d):
            for n in names:
                full = os.path.join(root, n)
                rel = os.path.relpath(full, d)
                with open(full, "rb") as f:
                    out[rel] = f.read()
        return {
            "status": status,
            "stdout": so,
            "stderr": se,
            "files": out,
        }
    finally:
        shutil.rmtree(d, ignore_errors=True)


LAST_STATUS = {}


def compare(port, files, args, stdin=None, env=None, timeout=60):
    """None if identical, else a short description of the first difference
    (and both results). The oracle's status is left in LAST_STATUS."""
    a = run_one(ORACLE, files, args, stdin, env, timeout)
    b = run_one(port, files, args, stdin, env, timeout)
    LAST_STATUS["oracle"] = a["status"]
    for k in ("status", "stdout", "stderr"):
        if a[k] != b[k]:
            return ("%s differs" % k, a, b)
    if sorted(a["files"]) != sorted(b["files"]):
        return ("file set differs: %s vs %s" % (sorted(a["files"]), sorted(b["files"])), a, b)
    for n in sorted(a["files"]):
        if a["files"][n] != b["files"][n]:
            return ("%s differs" % n, a, b)
    return None


def first_diff(x, y):
    n = min(len(x), len(y))
    for i in range(n):
        if x[i] != y[i]:
            return i
    return n if len(x) != len(y) else -1


def describe(diff):
    what, a, b = diff
    lines = [what]
    key = what.split(" ")[0]
    if key in ("status",):
        lines.append("oracle %r port %r" % (a["status"], b["status"]))
        lines.append("oracle stderr %r" % a["stderr"][-400:])
        lines.append("port stderr %r" % b["stderr"][-400:])
    elif key in ("stdout", "stderr"):
        x, y = a[key], b[key]
        i = first_diff(x, y)
        lines.append("at byte %d: oracle %r | port %r" % (i, x[max(0, i - 80):i + 120], y[max(0, i - 80):i + 120]))
    elif key in a["files"]:
        x, y = a["files"][key], b["files"][key]
        i = first_diff(x, y)
        lines.append("at byte %d: oracle %r | port %r" % (i, x[max(0, i - 120):i + 160], y[max(0, i - 120):i + 160]))
    return "\n".join(lines)
