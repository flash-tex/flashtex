#!/usr/bin/env python3
"""Keystroke latency of the Unicode mode against the pdfTeX (Classic) mode on
one long document, through the same client (lane UNICODE).

    python3 tools/unicode-m3/mode_gap.py --classic DIR --unicode-host BIN
        [--pages 120] [--keys 20] [--at 0.5,0.98] [--variants a,b,...]
        [--out DIR] [--json OUT]

`--classic DIR` is a tools/incr-bench/mkeng.sh engine directory
(flashtex-host, dl3-keys, pdftex.pool; its pdflatex format in
DIR/../fmt-NAME). `--unicode-host` is flashtex-host-unicode.

The document is T7's `full-N` (tools/incr-bench/gen.py: article, geometry,
amsmath, xcolor, siunitx, hyperref, cleveref, footnotes, labels, inline and
displayed mathematics), the one Classic's T7 gate types into. Variants:

  classic           full-N as it is, through flashtex-host (pdfTeX mode)
  classic-pagella   full-N with \\usepackage{tgpagella} (Type 1 TeX Gyre
                    Pagella), through flashtex-host
  unicode           full-N as it is, through flashtex-host-unicode (the
                    xelatex kernel's Latin Modern OpenType)
  unicode-fontspec  full-N with fontspec's TeX Gyre Pagella and
                    unicode-math's TeX Gyre Pagella Math, by family name,
                    through flashtex-host-unicode

Each (variant, position) gets a new host, started as the app starts it
(`--socket --s0-cache DIR --once`), and `dl3-keys` (Classic's T7 typist)
opens the document, then types `--keys` keystrokes into a prose line about
`--at` into it, each after the previous DONE, as T7's letter phases do.
Both modes are measured by the same client binary on the same machine, one
after the other: wall time from COMPILE written to the edited page's frame
read (DESIGN.md §1.2's host share), to the first page and to DONE.

On a shared or hosted machine these are not the §1.2 reference numbers;
the ratio between the modes on one machine is the result.
"""
import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import time

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, os.path.join(REPO, "tools", "incr-bench"))
import gen  # noqa: E402  (tools/incr-bench/gen.py, T7's generator)

ANCHOR = "\\usepackage{amsmath,amssymb}\n"
EXTRA = {
    "classic": "",
    "classic-pagella": "\\usepackage[T1]{fontenc}\n\\usepackage{tgpagella}\n",
    "unicode": "",
    "unicode-fontspec": ("\\usepackage{fontspec}\n\\setmainfont{TeX Gyre Pagella}\n"
                         "\\usepackage{unicode-math}\n\\setmathfont{TeX Gyre Pagella Math}\n"),
}


def source(variant, pages):
    s = gen.doc(pages, True)
    assert ANCHOR in s
    return s.replace(ANCHOR, ANCHOR + EXTRA[variant], 1)


def run_one(a, variant, at, work):
    """One host, one dl3-keys session: (summary, per-key rows, notes)."""
    classic = variant.startswith("classic")
    d = os.path.join(work, "%s-%s" % (variant, at))
    shutil.rmtree(d, ignore_errors=True)
    os.makedirs(os.path.join(d, "out"))
    with open(os.path.join(d, "main.tex"), "w", encoding="utf-8") as f:
        f.write(source(variant, a.pages))
    # (Unix socket paths must fit in 104 bytes)
    sock = os.path.join(tempfile.gettempdir(), "mg-%d-%s-%s.sock" % (os.getpid(), variant, at))
    if os.path.exists(sock):
        os.unlink(sock)
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
    if classic:
        eng = os.path.abspath(a.classic)
        env.update(FLASHTEX_POOL=os.path.join(eng, "pdftex.pool"),
                   FLASHTEX_FORMATS=os.path.join(os.path.dirname(eng), "fmt-" + os.path.basename(eng)))
        exe = os.path.join(eng, "flashtex-host")
    else:
        exe = a.unicode_host
    err = open(os.path.join(d, "host.err"), "w")
    p = subprocess.Popen([exe, "--socket", sock, "--s0-cache", os.path.join(d, "s0"), "--once"],
                         stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=err, text=True, env=env)
    lines = []
    up = threading.Event()

    def read():
        for line in p.stdout:
            lines.append(line.rstrip("\n"))
            if "listening" in line:
                up.set()
        up.set()

    threading.Thread(target=read, daemon=True).start()
    if not up.wait(600) or p.poll() is not None:
        p.kill()
        return None, [], "the host did not listen (%s)" % os.path.join(d, "host.err")
    keys_bin = a.dl3_keys or os.path.join(os.path.abspath(a.classic), "dl3-keys")
    t0 = time.time()
    r = subprocess.run([keys_bin, "--socket", sock, "--root", d, "--main", "main.tex",
                        "--output-dir", os.path.join(d, "out"), "--keys", str(a.keys), "--at", at],
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=3600)
    wall = time.time() - t0
    p.terminate()
    try:
        p.wait(30)
    except subprocess.TimeoutExpired:
        p.kill()
    with open(os.path.join(d, "keys.jsonl"), "w") as f:
        f.write(r.stdout)
    rows, summary = [], None
    for line in r.stdout.splitlines():
        try:
            j = json.loads(line)
        except ValueError:
            continue
        if "summary" in j:
            summary = j
        else:
            rows.append(j)
    note = None if r.returncode == 0 else "dl3-keys exit %d: %s" % (r.returncode, r.stderr.strip()[-400:])
    if summary is not None:
        summary["session_s"] = round(wall, 1)
    return summary, rows, note


def fmt(x):
    return "-" if x is None else ("%.1f" % x)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--classic", required=True, help="mkeng.sh engine directory")
    ap.add_argument("--unicode-host", required=True)
    ap.add_argument("--dl3-keys", help="default: the Classic engine directory's")
    ap.add_argument("--pages", type=int, default=120)
    ap.add_argument("--keys", type=int, default=20)
    ap.add_argument("--at", default="0.5,0.98")
    ap.add_argument("--variants", default="classic,classic-pagella,unicode,unicode-fontspec")
    ap.add_argument("--out", default=os.path.join(tempfile.gettempdir(), "mode-gap"))
    ap.add_argument("--json")
    a = ap.parse_args(argv)
    results = []
    for at in a.at.split(","):
        for v in a.variants.split(","):
            s, rows, note = run_one(a, v, at, a.out)
            res = dict(variant=v, at=at, summary=s, keys=rows, note=note)
            results.append(res)
            e = (s or {}).get("edited_page_ms") or {}
            dn = (s or {}).get("done_ms") or {}
            print("%-17s at %-5s pages %-4s edited page p50 %s p95 %s max %s ms; DONE p50 %s p95 %s ms%s"
                  % (v, at, (s or {}).get("pages"), fmt(e.get("p50")), fmt(e.get("p95")), fmt(e.get("max")),
                     fmt(dn.get("p50")), fmt(dn.get("p95")), "; " + note if note else ""), flush=True)
    by = {(r["variant"], r["at"]): ((r["summary"] or {}).get("edited_page_ms") or {}) for r in results}
    print("\n| at | classic p50/p95 | unicode p50/p95 | unicode-fontspec p50/p95 | fontspec / classic (p95) |")
    print("|---|---|---|---|---|")
    for at in a.at.split(","):
        c, u, f = by.get(("classic", at), {}), by.get(("unicode", at), {}), by.get(("unicode-fontspec", at), {})
        ratio = (f.get("p95") / c.get("p95")) if f.get("p95") and c.get("p95") else None
        print("| %s | %s / %s | %s / %s | %s / %s | %s |" % (
            at, fmt(c.get("p50")), fmt(c.get("p95")), fmt(u.get("p50")), fmt(u.get("p95")),
            fmt(f.get("p50")), fmt(f.get("p95")), "-" if ratio is None else "%.2fx" % ratio))
    if a.json:
        with open(a.json, "w") as f:
            json.dump(dict(pages=a.pages, keys=a.keys, results=results), f, indent=1)
    return 0 if all(r["summary"] and not r["note"] for r in results) else 1


if __name__ == "__main__":
    sys.exit(main())
