#!/usr/bin/env python3
"""The Unicode corpus through flashtex-host-unicode, with TeX Live or from the
no-TeX-Live bundle alone (M3's "a no-TeX-Live run of the corpus from the
bundle"; docs/design/xetex/PLAN.md §3.4, modes U3c).

    python3 tools/unicode-m3/notex_corpus.py --mode texlive|notex --host flashtex-host-unicode
        --client dl3-client --cases-dir DIR --out DIR [--cases GLOB ...]
        [--lock tools/bundle/tl2026/flashtex-bundle.lock] [--url U --digest D]
        [--compare-with DIR] [--json OUT]

Each case is compiled once through the host's socket as the app compiles it
(dl3-client, the `.aux` passes the host decides), and its PDF (`DONE.pdf`,
FlashTeX's own writer) is kept as `OUT/pdf/NAME.<mode>.pdf`.

`--mode notex` runs the host as scripts/notex-host-check.sh does: an empty
environment (HOME and TMPDIR of its own, the system PATH only), the bundle of
the lock with the user's consent (`FLASHTEX_BUNDLE_ALLOW_FETCH=digest@url`),
and on macOS sandbox-exec denying every TeX Live location; it refuses to run
where pdflatex, pdftex or kpsewhich is on the system PATH. `--mode texlive`
runs it with this environment's TeX Live.

`--compare-with DIR` (a texlive run's `pdf/` directory, with the oracle's
`NAME.reference.pdf` from tools/xetex-pdfparity/run.py `--pdf-dir` beside
it) compares each notex PDF with the texlive one byte for byte, and
structurally (tools/xetex-pdfparity/compare.py, no raster) with xelatex's.
"""
import argparse
import fnmatch
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "xetex-lockstep"))
import latex as L  # noqa: E402

TEXLIVE_PATHS = ["/usr/local/texlive", "/Library/TeX", "/opt/homebrew/opt/texlive",
                 "/opt/homebrew/Cellar/texlive", "/usr/local/opt/texlive", "/usr/local/Cellar/texlive",
                 "/opt/texlive", "/usr/share/texlive", "/usr/share/texmf"]


def lock_values(path):
    s = open(path).read()
    url = re.search(r'^url *= *"(.*)"', s, re.M).group(1)
    digest = re.search(r'^digest *= *"(.*)"', s, re.M).group(1)
    return url, digest


def host_env(a, out):
    pinned = {"SOURCE_DATE_EPOCH": "0", "FORCE_SOURCE_DATE": "1", "TZ": "UTC"}
    if a.mode == "texlive":
        env = dict(os.environ)
        env.update(pinned)
        env["FLASHTEX_FORMAT_CACHE_DIR"] = os.path.join(out, "format-cache")
        return env, []
    for x in ("pdflatex", "pdftex", "kpsewhich"):
        if shutil.which(x, path="/usr/bin:/bin:/usr/sbin:/sbin"):
            raise SystemExit("%s is on the system PATH: no no-TeX-Live run here" % x)
    work = os.path.join(out, "work")
    for d in ("home", "tmp"):
        os.makedirs(os.path.join(work, d), exist_ok=True)
    lock = os.path.abspath(a.lock)
    url, digest = lock_values(lock)
    env = {"HOME": os.path.join(work, "home"), "TMPDIR": os.path.join(work, "tmp") + "/",
           "PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "LANG": "C",
           "FLASHTEX_BUNDLE_CACHE_DIR": os.path.join(out, "bundle-cache"),
           "FLASHTEX_FORMAT_CACHE_DIR": os.path.join(out, "format-cache")}
    env.update(pinned)
    if a.url and (a.url, a.digest) != (url, digest):
        env.update(FLASHTEX_BUNDLE_URL=a.url, FLASHTEX_BUNDLE_DIGEST=a.digest)
    else:
        env.update(FLASHTEX_BUNDLE_LOCK=lock, FLASHTEX_BUNDLE_ALLOW_FETCH="%s@%s" % (digest, url))
    wrap = []
    if sys.platform == "darwin" and os.path.exists("/usr/bin/sandbox-exec"):
        home = os.path.expanduser("~")
        deny = "".join(' (subpath "%s")' % p for p in TEXLIVE_PATHS + [home + "/Library/texmf", home + "/texmf"])
        sb = os.path.join(work, "notex.sb")
        with open(sb, "w") as f:
            f.write('(version 1)\n(allow default)\n(deny file-read* file-write*%s (literal "/etc/paths.d/TeX"))\n' % deny)
        wrap = ["/usr/bin/sandbox-exec", "-f", sb]
        if subprocess.run(wrap + ["/bin/test", "-e", "/Library/TeX/texbin/kpsewhich"]).returncode == 0:
            raise SystemExit("the sandbox does not hide /Library/TeX")
    return env, wrap


def start(a, out, env, wrap):
    sock = os.path.join(out, "host.sock")
    if os.path.exists(sock):
        os.remove(sock)
    log = open(os.path.join(out, "host.log"), "w")
    p = subprocess.Popen(wrap + [os.path.abspath(a.host), "--socket", sock], env=env,
                         stdout=log, stderr=subprocess.STDOUT)
    for _ in range(9000):  # the first start builds xelatex.fmt (fetching what it reads)
        if os.path.exists(sock):
            try:
                s = socket.socket(socket.AF_UNIX)
                s.connect(sock)
                s.close()
                return p, sock
            except OSError:
                pass
        if p.poll() is not None:
            break
        time.sleep(0.1)
    raise SystemExit("the host did not start; its log:\n" + open(log.name).read()[-4000:])


def hello(out):
    for line in open(os.path.join(out, "host.log"), errors="replace"):
        if line.startswith("flashtex-host-unicode: {") and '"resolver"' in line:
            return json.loads(line.split(": ", 1)[1])
    return None


def compare(a, results, pdfdir):
    import importlib.util
    spec = importlib.util.spec_from_file_location(
        "pdfparity_compare", os.path.join(REPO, "tools", "xetex-pdfparity", "compare.py"))
    sys.path.insert(0, os.path.join(REPO, "tools", "xetex-pdfparity"))
    C = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(C)
    ap = argparse.ArgumentParser()
    C.add_options(ap)
    opt = C.options_from(ap.parse_args(["--no-visual"]))
    for r in results:
        mine = os.path.join(pdfdir, r["case"] + ".notex.pdf")
        tl = os.path.join(a.compare_with, r["case"] + ".texlive.pdf")
        ref = os.path.join(a.compare_with, r["case"] + ".reference.pdf")
        if not os.path.exists(mine):
            continue
        if os.path.exists(tl):
            r["bytes_equal_texlive"] = open(mine, "rb").read() == open(tl, "rb").read()
        if os.path.exists(ref):
            try:
                rep = C.compare(ref, mine, opt, tag=r["case"])
                r["structural_vs_xelatex"] = rep.as_json()["structural_differences"]
                r["summary_vs_xelatex"] = C.summary_line(rep)
            except Exception as e:  # reported, not hidden
                r["summary_vs_xelatex"] = "compare failed: %s: %s" % (type(e).__name__, e)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--mode", choices=("texlive", "notex"), required=True)
    ap.add_argument("--host", required=True)
    ap.add_argument("--client", required=True)
    ap.add_argument("--cases-dir", action="append", required=True)
    ap.add_argument("--cases", nargs="*", default=["*"])
    ap.add_argument("--out", required=True)
    ap.add_argument("--lock", default=os.path.join(REPO, "tools", "bundle", "tl2026", "flashtex-bundle.lock"))
    ap.add_argument("--url")
    ap.add_argument("--digest")
    ap.add_argument("--compare-with")
    ap.add_argument("--json")
    a = ap.parse_args(argv)
    out = os.path.abspath(a.out)
    os.makedirs(out, exist_ok=True)
    pdfdir = os.path.join(out, "pdf")
    os.makedirs(pdfdir, exist_ok=True)
    srcs = L.find_cases(a.cases_dir)
    names = [n for n in sorted(srcs) if any(fnmatch.fnmatch(n, p) for p in a.cases)]
    env, wrap = host_env(a, out)
    t = time.time()
    p, sock = start(a, out, env, wrap)
    startup_s = round(time.time() - t, 1)
    results = []
    try:
        for name in names:
            root = os.path.join(out, "docs", name)
            if os.path.exists(root):
                shutil.rmtree(root)
            L.stage(srcs[name], name, root)
            od = os.path.join(out, "build", name)
            os.makedirs(od, exist_ok=True)
            r = subprocess.run([os.path.abspath(a.client), "--socket", sock, "--root", root,
                                "--main", name + ".tex", "--output-dir", od, "--quiet"],
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=3600)
            rows = [json.loads(x) for x in r.stdout.splitlines() if x.startswith("{")]
            row = rows[-1] if rows else {}
            pdf = os.path.join(od, name + ".pdf")
            res = {"case": name, "family": L.case_meta(srcs[name]).get("family"),
                   "status": row.get("status"), "pages": row.get("pages"),
                   "done_ms": row.get("done_ms"), "pdf": os.path.exists(pdf)}
            if not rows:
                res["client_error"] = r.stderr[-400:]
            if os.path.exists(pdf):
                shutil.copy(pdf, os.path.join(pdfdir, "%s.%s.pdf" % (name, a.mode)))
            logp = os.path.join(od, name + ".log")
            if os.path.exists(logp):
                log = open(logp, errors="replace").read()
                miss = re.findall(r"! (?:LaTeX|Package) [^\n]*?(?:File|font) [`'\"]?([^'\"\n]+?)['\"]? (?:not found|cannot be found)", log)
                if miss:
                    res["not_found"] = sorted(set(miss))[:10]
                res["errors"] = log.count("\n! ")
            results.append(res)
            print("%s %-34s status %s pages %s pdf %s%s" % (
                "OK  " if res["status"] == "ok" and res["pdf"] else "FAIL", name, res["status"],
                res["pages"], res["pdf"], (" not found: %s" % ", ".join(res["not_found"])) if res.get("not_found") else ""),
                flush=True)
    finally:
        p.terminate()
        p.wait()
    h = hello(out)
    if a.compare_with:
        compare(a, results, pdfdir)
        for r in results:
            print("  %-34s bytes equal to the texlive run: %s; vs xelatex: %s" % (
                r["case"], r.get("bytes_equal_texlive"), r.get("summary_vs_xelatex")))
    ok = sum(1 for r in results if r["status"] == "ok" and r["pdf"])
    print("%s: %d cases, %d compiled with a PDF; host up in %.1f s; HELLO texlive=%s resolver=%s" % (
        a.mode, len(results), ok, startup_s, h and h.get("texlive"), h and h.get("resolver")))
    if a.json:
        with open(a.json, "w") as f:
            json.dump({"mode": a.mode, "hello": h, "startup_s": startup_s, "cases": results}, f, indent=1)
    shutil.rmtree(os.path.join(out, "work"), ignore_errors=True)
    return 0 if ok == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
