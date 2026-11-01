"""Run TeX Live's bibtex (the oracle) and FlashTeX's port on the same inputs
and compare everything they produce, byte for byte: the exit status,
standard output, standard error, and every file in the working directory
afterwards (the .bbl, the .blg, and anything else).

Each run gets its own fresh directory holding exactly the case's input
files and runs there, as latexmk and the engine host run bibtex. Python 3
standard library only.
"""

import os
import shutil
import subprocess
import tempfile

ORACLE = "/Library/TeX/texbin/bibtex"
TMP = os.environ.get("BTCMP_TMP") or None


def run_one(binary, files, args, env=None, timeout=120):
    d = tempfile.mkdtemp(prefix="bt-", dir=TMP)
    try:
        for name, data in files.items():
            p = os.path.join(d, name)
            os.makedirs(os.path.dirname(p), exist_ok=True)
            with open(p, "wb") as f:
                f.write(data)
        env = dict(env if env is not None else os.environ)
        # argv[0] is `bibtex`, found along PATH, as latexmk and \write18
        # run it (getopt and kpathsea print it).
        env["PATH"] = os.path.dirname(ORACLE) + os.pathsep + env.get("PATH", "")
        try:
            p = subprocess.run(
                ["bibtex"] + list(args),
                executable=binary,
                cwd=d,
                stdin=subprocess.DEVNULL,
                capture_output=True,
                env=env,
                timeout=timeout,
            )
            status, so, se = p.returncode, p.stdout, p.stderr
        except subprocess.TimeoutExpired as e:
            status, so, se = "never ends", e.stdout or b"", e.stderr or b""
        out = {}
        for root, _, names in os.walk(d):
            for n in names:
                full = os.path.join(root, n)
                rel = os.path.relpath(full, d)
                with open(full, "rb") as f:
                    out[rel] = f.read()
        return {"status": status, "stdout": so, "stderr": se, "files": out}
    finally:
        shutil.rmtree(d, ignore_errors=True)


LAST_STATUS = {}


def compare(port, files, args, env=None, timeout=120):
    """None if identical, else (what, oracle result, port result)."""
    a = run_one(ORACLE, files, args, env, timeout)
    b = run_one(port, files, args, env, timeout)
    LAST_STATUS["oracle"] = a["status"]
    LAST_STATUS["blg"] = a["files"].get(next((n for n in a["files"] if n.endswith(".blg")), ""), b"")
    # Two runs that never end (a style's endless loop) are killed at the
    # timeout; what each wrote by then depends on when, so one must be a
    # prefix of the other.
    killed = a["status"] == b["status"] == "never ends"

    def same(x, y):
        if killed:
            n = min(len(x), len(y))
            return x[:n] == y[:n]
        return x == y

    if a["status"] != b["status"]:
        return ("status differs", a, b)
    for k in ("stdout", "stderr"):
        if not same(a[k], b[k]):
            return ("%s differs" % k, a, b)
    if sorted(a["files"]) != sorted(b["files"]):
        return ("file set differs: %s vs %s" % (sorted(a["files"]), sorted(b["files"])), a, b)
    for n in sorted(a["files"]):
        if not same(a["files"][n], b["files"][n]):
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
    if key == "status":
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
