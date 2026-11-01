#!/usr/bin/env python3
"""Edit replay through flashtex-host-unicode against cold compiles (M3's
"incremental soundness sweeps on Unicode", as far as Unicode mode has an
incremental path today).

    python3 tools/unicode-m3/host_sweep.py --host flashtex-host-unicode --client dl3-client
        --cases-dir DIR [--cases GLOB ...] [--edits N] [--json OUT]

Unicode mode has no incremental engine yet (docs/design/xetex/INCREMENTAL.md
§5: checkpoints, restart and convergence come after the runtime extraction),
so Classic's soundness sweeps (tools/incr-bench, sweeps.yml: incremental equal
to cold, frame by frame) have nothing to sweep. What a Unicode keystroke does
reuse is the host's state between compiles: the hot spare (a run started
ahead and stopped after the format, taken if nothing it read has changed),
the format cache and the `.aux` passes. This sweep checks that reuse: each
document gets a scripted series of edits (a word in the body, a new
section, a cross-reference, a preamble line, a revert), and after each edit
the document is compiled by host A (spares on, the app's configuration) and,
in a copy of the project with the same history, by host B with spares off
(`FLASHTEX_UNICODE_SPARE_CAP=0`), every compile a cold run. The pages, fonts,
forms and images A sends must equal B's byte for byte (the project's path
replaced in both); STARTED, DONE, diagnostics and the sources list are not
compared.
"""
import argparse
import fnmatch
import json
import os
import shutil
import socket
import struct
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "xetex-lockstep"))
import latex as L  # noqa: E402

COMPARED = {0x43: "FONT", 0x44: "IMAGE", 0x45: "PAGE", 0x46: "FORM", 0x4D: "IMAGE_DATA"}


def frames(path, root):
    out = []
    data = open(path, "rb").read() if os.path.exists(path) else b""
    i = 0
    rb = root.encode()
    while i + 5 <= len(data):
        n = struct.unpack("<I", data[i:i + 4])[0]
        kind = data[i + 4]
        body = data[i + 5:i + 4 + n]
        i += 4 + n
        if kind in COMPARED:
            out.append((kind, body.replace(rb, b"<ROOT>")))
    return out


def edits(text, n):
    """The scripted edits of one document: (label, new text)."""
    end = text.rfind("\\end{document}")
    begin = text.find("\\begin{document}")
    if end < 0 or begin < 0:
        return []
    body_add = lambda s: text[:end] + s + "\n" + text[end:]
    steps = [
        ("word", body_add("Sweep word one.")),
        ("word2", body_add("Sweep word one, two.")),
        ("section", body_add("\\section{Swept}\\label{sweep:s} Sweep text, see \\ref{sweep:s}.")),
        ("preamble", text[:begin] + "\\newcommand\\sweepzz{zz}\n" + text[begin:end]
         + "\\section{Swept}\\label{sweep:s} Sweep \\sweepzz, see \\ref{sweep:s}.\n" + text[end:]),
        ("revert", text),
    ]
    return steps[:n]


def start_host(host, tmp, tag, spares):
    sock = os.path.join(tmp, "%s.sock" % tag)
    log = open(os.path.join(tmp, "%s.log" % tag), "w")
    env = dict(os.environ)
    env.update(SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1", TZ="UTC")
    env.setdefault("FLASHTEX_FORMAT_CACHE_DIR", os.path.join(tmp, "format-cache"))
    if not spares:
        env["FLASHTEX_UNICODE_SPARE_CAP"] = "0"
    p = subprocess.Popen([host, "--socket", sock], stdout=log, stderr=subprocess.STDOUT, env=env)
    for _ in range(6000):
        if os.path.exists(sock):
            try:
                s = socket.socket(socket.AF_UNIX)
                s.connect(sock)
                s.close()
                return p, sock
            except OSError:
                pass
        if p.poll() is not None:
            raise SystemExit("the host exited: see %s" % log.name)
        time.sleep(0.1)
    raise SystemExit("the host did not start: see %s" % log.name)


def compile_once(client, sock, root, main, save):
    r = subprocess.run([client, "--socket", sock, "--root", root, "--main", main, "--save", save,
                        "--quiet"], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                       timeout=1800)
    rows = [json.loads(x) for x in r.stdout.splitlines() if x.startswith("{")]
    return rows[-1] if rows else {"status": "client-failed", "stderr": r.stderr[-500:]}


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--host", required=True)
    ap.add_argument("--client", required=True)
    ap.add_argument("--cases-dir", action="append", required=True)
    ap.add_argument("--cases", nargs="*", default=["*"])
    ap.add_argument("--edits", type=int, default=5)
    ap.add_argument("--json")
    a = ap.parse_args(argv)
    srcs = L.find_cases(a.cases_dir)
    names = [n for n in sorted(srcs) if any(fnmatch.fnmatch(n, p) for p in a.cases)]
    tmp = tempfile.mkdtemp(prefix="unicode-sweep-")
    hosts = {"a": start_host(a.host, tmp, "a", True), "b": start_host(a.host, tmp, "b", False)}
    results = []
    totals = {"documents": 0, "compiles": 0, "equal": 0, "differ": 0, "failed": 0}
    try:
        for name in names:
            src = srcs[name]
            # a [fonts] fixture's flashtex.toml is staged with it: the host
            # makes the first line itself
            roots = {}
            for k in ("a", "b"):
                roots[k] = os.path.join(tmp, k, name)
                L.stage(src, name, roots[k])
            main_tex = name + ".tex"
            text = open(os.path.join(roots["a"], main_tex), encoding="utf-8",
                        errors="surrogateescape").read()
            steps = [("open", text)] + edits(text, a.edits)
            doc = {"case": name, "steps": []}
            for i, (label, new) in enumerate(steps):
                got = {}
                for k in ("a", "b"):
                    with open(os.path.join(roots[k], main_tex), "w", encoding="utf-8",
                              errors="surrogateescape") as f:
                        f.write(new)
                    save = os.path.join(tmp, "%s-%s-%d.dl3" % (k, name, i))
                    if k == "a" and i:
                        time.sleep(0.5)  # the spare's start-up, as between keys
                    row = compile_once(a.client, hosts[k][1], roots[k], main_tex, save)
                    got[k] = (row, frames(save, roots[k]))
                    if os.path.exists(save):
                        os.remove(save)
                (ra, fa), (rb, fb) = got["a"], got["b"]
                same = fa == fb
                first = None
                if not same:
                    first = next((j for j in range(min(len(fa), len(fb))) if fa[j] != fb[j]),
                                 min(len(fa), len(fb)))
                ok_status = ra.get("status") == rb.get("status")
                step = {"edit": label, "status": [ra.get("status"), rb.get("status")],
                        "pages": [ra.get("pages"), rb.get("pages")], "frames": [len(fa), len(fb)],
                        "equal": same and ok_status,
                        "first_difference": None if same else "%s frame %d" % (
                            COMPARED.get((fa[first] if first < len(fa) else fb[first])[0], "?"), first)}
                doc["steps"].append(step)
                totals["compiles"] += 1
                if step["equal"]:
                    totals["equal"] += 1
                else:
                    totals["differ"] += 1
                if ra.get("status") not in ("ok", "errors") or not fa:
                    totals["failed"] += 1
                print("%s %-34s %-8s status %s/%s pages %s/%s frames %d/%d%s" % (
                    "EQUAL" if step["equal"] else "DIFFER", name, label, step["status"][0],
                    step["status"][1], step["pages"][0], step["pages"][1], len(fa), len(fb),
                    "" if same else " (first difference: %s)" % step["first_difference"]), flush=True)
            totals["documents"] += 1
            results.append(doc)
    finally:
        for p, _ in hosts.values():
            p.terminate()
            p.wait()
    print("%(documents)d documents, %(compiles)d compiles: %(equal)d equal, %(differ)d differ, "
          "%(failed)d failed or empty" % totals)
    if a.json:
        with open(a.json, "w") as f:
            json.dump({"totals": totals, "documents": results}, f, indent=1)
    shutil.rmtree(tmp, ignore_errors=True)
    return 0 if totals["differ"] == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
