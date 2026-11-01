#!/usr/bin/env python3
"""Inputs of the Unicode-mode preview-parity sweep (lane UNICODE-PREVIEW-QA).

The Mac preview draws what `flashtex-host-unicode` streams (display-list-v3,
native OpenType/TrueType fonts with glyph-id codes, spec §11.1). Nothing
compared that preview with what xelatex prints until a page of glyph ids
was drawn as character codes (#1759). This script compiles every case of
`unicode-cases/` (fonts, unicode-math, RTL, Devanagari, CJK, colour, links,
images, TikZ, rotations, transparency) three ways and leaves, per case,
what `UnicodePreviewParityTests` (apps/mac) compares:

    <work>/<case>/display.dl3   the frames the host sent, as the app gets them
                                (dl3-client --save, font formats as the app's)
    <work>/<case>/flashtex.pdf  FlashTeX's own PDF of that same compile
    <work>/<case>/oracle.pdf    TeX Live's xelatex's PDF (two runs), when
                                xelatex is installed; absent otherwise
    <work>/<case>/run.json      the client's report (status, pages, timings)

    python3 tools/displaylist/unicode_preview_parity.py \\
        [--host crates/flashtex-xetex/target/release/flashtex-host-unicode] \\
        [--client target/release/dl3-client] [--work target/dl3-unicode] \\
        [--only 'u0*'] [--jobs 3] [--xelatex PATH|none]

Then `cd apps/mac && FLASHTEX_DL3_UNICODE=<work> swift test --filter
UnicodePreviewParityTests`. Oracle tooling only: xelatex (MacTeX) is never
in the product path; without it the oracle comparison is skipped and the
preview is still compared with FlashTeX's PDF. Exit status 1 when a case
did not compile to a display list with pages.
"""

import argparse
import fnmatch
import json
import os
import shutil
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
CASES = os.path.join(HERE, "unicode-cases")
# The app's COMPILE (EngineV3Session.request): these formats' programs travel.
FONT_FORMATS = "type3,truetype,opentype"


def env():
    e = dict(os.environ)
    e.setdefault("SOURCE_DATE_EPOCH", "0")
    e.setdefault("FORCE_SOURCE_DATE", "1")
    return e


def oracle(xelatex, case_tex, work):
    """xelatex twice (cross references), its PDF to <work>/oracle.pdf."""
    d = os.path.join(work, "xelatex")
    shutil.rmtree(d, ignore_errors=True)
    os.makedirs(d)
    shutil.copy(case_tex, os.path.join(d, "main.tex"))
    for _ in range(2):
        r = subprocess.run([xelatex, "-interaction=nonstopmode", "main.tex"], cwd=d, env=env(),
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=600)
    pdf = os.path.join(d, "main.pdf")
    if not os.path.exists(pdf):
        return "xelatex wrote no PDF (exit %d)" % r.returncode
    shutil.copy(pdf, os.path.join(work, "oracle.pdf"))
    return None


def engine(host, client, case_tex, work):
    """One `flashtex-host-unicode` (`--once`) and one compile through it, as
    the app's: format xelatex, the app's font formats. The frames go to
    display.dl3, DONE's PDF to flashtex.pdf."""
    src = os.path.join(work, "src")
    out = os.path.join(work, "out")
    for d in (src, out):
        shutil.rmtree(d, ignore_errors=True)
        os.makedirs(d)
    shutil.copy(case_tex, os.path.join(src, "main.tex"))
    # A short socket path (sun_path is 104 bytes on macOS).
    sock = "/tmp/ftx-up-%d-%s.sock" % (os.getpid(), os.path.basename(work)[:12])
    if os.path.exists(sock):
        os.remove(sock)
    with open(os.path.join(work, "host.log"), "wb") as log:
        h = subprocess.Popen([host, "--socket", sock, "--once", "--accept-timeout", "900"],
                             env=env(), stdout=log, stderr=subprocess.STDOUT)
    try:
        t = time.time()
        while not os.path.exists(sock):
            if h.poll() is not None:
                return "the host exited (%d) before listening" % h.returncode, None
            if time.time() - t > 900:
                return "the host did not listen", None
            time.sleep(0.05)
        r = subprocess.run([client, "--socket", sock, "--root", src, "--main", "main.tex",
                            "--format", "xelatex", "--font-formats", FONT_FORMATS,
                            "--output-dir", out, "--save", os.path.join(work, "display.dl3"),
                            "--diag", os.path.join(work, "diag.jsonl"), "--quiet"],
                           env=env(), capture_output=True, text=True, timeout=900)
        line = r.stdout.strip().splitlines()[-1] if r.stdout.strip() else "{}"
        try:
            rep = json.loads(line)
        except ValueError:
            rep = {"raw": line}
        rep["client_exit"] = r.returncode
        rep["client_stderr"] = r.stderr[-2000:]
        pdf = (rep.get("host") or {}).get("pdf")
        if pdf and os.path.exists(pdf):
            shutil.copy(pdf, os.path.join(work, "flashtex.pdf"))
        if r.returncode != 0 or not rep.get("pages"):
            return "the compile sent no pages (client exit %d)" % r.returncode, rep
        if not os.path.exists(os.path.join(work, "flashtex.pdf")):
            return "DONE named no PDF", rep
        return None, rep
    finally:
        try:
            h.wait(timeout=30)
        except subprocess.TimeoutExpired:
            h.kill()
            h.wait()
        if os.path.exists(sock):
            os.remove(sock)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--host", default=os.path.join(REPO, "crates/flashtex-xetex/target/release/flashtex-host-unicode"))
    ap.add_argument("--client", default=os.path.join(REPO, "target/release/dl3-client"))
    ap.add_argument("--work", default=os.path.join(REPO, "target/dl3-unicode"))
    ap.add_argument("--only", default="*")
    ap.add_argument("--jobs", type=int, default=3)
    ap.add_argument("--xelatex", default=None, help="the oracle (default: xelatex on PATH); 'none' skips it")
    a = ap.parse_args()
    for exe in (a.host, a.client):
        if not os.access(exe, os.X_OK):
            sys.exit("not built: %s" % exe)
    xelatex = None if a.xelatex == "none" else (a.xelatex or shutil.which("xelatex"))
    if not xelatex:
        print("no xelatex: the oracle comparison will be skipped", file=sys.stderr)
    cases = sorted(f for f in os.listdir(CASES) if f.endswith(".tex") and fnmatch.fnmatch(f[:-4], a.only))
    os.makedirs(a.work, exist_ok=True)

    def one(name):
        work = os.path.join(a.work, name[:-4])
        os.makedirs(work, exist_ok=True)
        for f in ("display.dl3", "flashtex.pdf", "oracle.pdf"):
            if os.path.exists(os.path.join(work, f)):
                os.remove(os.path.join(work, f))
        tex = os.path.join(CASES, name)
        t = time.time()
        err, rep = engine(a.host, a.client, tex, work)
        t_engine = time.time() - t
        oerr = oracle(xelatex, tex, work) if xelatex else "no xelatex"
        row = {"case": name[:-4], "engine_error": err, "oracle_error": oerr,
               "engine_s": round(t_engine, 2), "report": rep}
        with open(os.path.join(work, "run.json"), "w") as f:
            json.dump(row, f, indent=1)
        return row

    # The first case alone: the host builds and caches xelatex.fmt once.
    rows = [one(cases[0])] if cases else []
    with ThreadPoolExecutor(max(1, a.jobs)) as ex:
        rows += list(ex.map(one, cases[1:]))
    bad = 0
    for r in rows:
        rep = r["report"] or {}
        print("%-24s %s  pages %s  %.1fs  oracle: %s" % (
            r["case"], "ok " if not r["engine_error"] else "ERR", rep.get("pages"), r["engine_s"],
            "ok" if not r["oracle_error"] else r["oracle_error"]))
        if r["engine_error"]:
            bad += 1
            print("    engine: %s" % r["engine_error"])
    with open(os.path.join(a.work, "runs.json"), "w") as f:
        json.dump(rows, f, indent=1)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
