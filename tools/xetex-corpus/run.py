#!/usr/bin/env python3
"""Unicode-mode corpus: typical xelatex documents, built with no TeX Live
(lane UNICODE-INDEPENDENT; docs/design/xetex/PLAN.md §3.4, DESIGN.md D12).

For each document (`docs/<id>/`, plus TeX Live's own examples named in
TEXLIVE_DOCS, copied from the oracle's tree at run time):

* **ref**: TeX Live's `latexmk -xelatex` (the oracle, TeX Live present):
  xelatex, then biber, bibtex or makeindex as latexmk decides.
* **notex**: `flashtex-v3 build --mode unicode` (which starts
  `flashtex-host-unicode` and speaks display-list-v3 to it) with TeX Live
  hidden as `scripts/notex-host-check.sh` hides it: an empty environment, a
  HOME and TMPDIR of our own, the system PATH only, `sandbox-exec` denying
  every TeX Live location, and the bundle from `flashtex-bundle.lock` with
  the user's consent (`FLASHTEX_BUNDLE_ALLOW_FETCH`). Then `flashtex-v3 check
  --json` in the same environment for TeX's diagnostics and the tools' skips.
* **tl** (`--with-texlive`): the same `flashtex-v3 build` with TeX Live
  visible, which separates a bundle gap (tl works, notex does not) from an
  engine or host bug (neither works).

Each PDF is compared with the reference by `tools/xetex-pdfparity/compare.py`
(structural; `--visual` adds Core Graphics rasters). The oracle is used only
as the reference and never in the product path (DESIGN.md).

    python3 tools/xetex-corpus/run.py --v3 target/release/flashtex-v3 \
        --host-unicode crates/flashtex-xetex/target/release/flashtex-host-unicode \
        --out /tmp/xc [--only 'p0*'] [--with-texlive] [--jobs 4] [--json OUT]
"""

import argparse
import concurrent.futures
import fnmatch
import json
import os
import re
import shutil
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
LOCK = os.path.join(REPO, "tools", "bundle", "tl2026", "flashtex-bundle.lock")
TEXBIN = "/Library/TeX/texbin"
TLDOC = "/usr/local/texlive/2026/texmf-dist/doc"
# TeX Live's own Unicode-mode examples, compiled in place of a copy in the repo.
TEXLIVE_DOCS = {
    "x01-tl-fontspec-example": "latex/fontspec/fontspec-example.tex",
    "x02-tl-biblatex-01-introduction": "latex/biblatex/examples/01-introduction.tex",
}
# texbundle.py TEXLIVE_PATHS, plus the user trees and MacTeX's /etc/paths.d/TeX.
HIDE = ["/usr/local/texlive", "/Library/TeX", "/opt/homebrew/opt/texlive",
        "/opt/homebrew/Cellar/texlive", "/usr/local/opt/texlive", "/usr/local/Cellar/texlive",
        "/opt/texlive", "/usr/share/texlive", "/usr/share/texmf",
        os.path.expanduser("~/Library/texmf"), os.path.expanduser("~/texmf")]
DATE = {"SOURCE_DATE_EPOCH": "0", "FORCE_SOURCE_DATE": "1", "TZ": "UTC"}
TIMEOUT = 600


def lock():
    t = open(LOCK).read()
    return (re.search(r'^url *= *"(.*)"', t, re.M).group(1),
            re.search(r'^digest *= *"(.*)"', t, re.M).group(1))


def docs(only):
    out = []
    for d in sorted(os.listdir(os.path.join(HERE, "docs"))):
        out.append((d, os.path.join(HERE, "docs", d), None))
    for d, rel in TEXLIVE_DOCS.items():
        out.append((d, None, os.path.join(TLDOC, rel)))
    out.sort()
    if only:
        out = [x for x in out if any(fnmatch.fnmatch(x[0], o) for o in only.split(","))]
    return out


def stage(doc, dest):
    name, src_dir, tl_file = doc
    shutil.rmtree(dest, ignore_errors=True)
    if src_dir:
        shutil.copytree(src_dir, dest)
    else:
        os.makedirs(dest)
        shutil.copy(tl_file, os.path.join(dest, "main.tex"))


def run(cmd, cwd, env, timeout=TIMEOUT):
    t = time.monotonic()
    try:
        p = subprocess.run(cmd, cwd=cwd, env=env, stdout=subprocess.PIPE,
                           stderr=subprocess.STDOUT, timeout=timeout)
        return p.returncode, p.stdout.decode("utf-8", "replace"), time.monotonic() - t
    except subprocess.TimeoutExpired as e:
        return None, (e.stdout or b"").decode("utf-8", "replace"), time.monotonic() - t


def reference(doc, work):
    d = os.path.join(work, "ref")
    stage(doc, d)
    env = {"PATH": f"{TEXBIN}:/usr/bin:/bin:/usr/sbin:/sbin", "HOME": os.environ["HOME"],
           "LANG": "en_US.UTF-8", **pristine_config(work), **DATE}
    code, out, secs = run(["latexmk", "-xelatex", "-interaction=nonstopmode", "-f", "main.tex"], d, env)
    pdf = os.path.join(d, "main.pdf")
    log = open(os.path.join(d, "main.log"), errors="replace").read() if os.path.exists(os.path.join(d, "main.log")) else ""
    return {"exit": code, "pdf": pdf if os.path.exists(pdf) else None, "secs": round(secs, 2),
            "missing_chars": len(re.findall(r"^Missing character", log, re.M)),
            "errors": re.findall(r"^! .*", log, re.M)[:3],
            "tools": sorted(set(re.findall(r"Latexmk: applying rule '(biber|bibtex|makeindex)", out)))}


def pristine_config(work):
    """TeX Live as installed, but with TEXMFCONFIG and TEXMFSYSCONFIG empty:
    `tlmgr paper letter` (MacTeX's installer) writes its dvipdfmx.cfg there,
    while the bundle carries TeX Live's own default, A4, as a fresh TeX Live
    has it."""
    d = os.path.realpath(os.path.join(work, "..", "..", "texmfconfig"))
    os.makedirs(d, exist_ok=True)
    return {"TEXMFCONFIG": d, "TEXMFSYSCONFIG": d}


def notex_env(a, scratch, hide):
    url, digest = lock()
    env = {"HOME": os.path.join(scratch, "home"), "TMPDIR": a.tmp + "/",
           "PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "LANG": "C",
           "FLASHTEX_BUNDLE_LOCK": LOCK, "FLASHTEX_BUNDLE_ALLOW_FETCH": f"{digest}@{url}",
           "FLASHTEX_BUNDLE_CACHE_DIR": os.path.join(scratch, "bundle-cache"),
           "FLASHTEX_FORMAT_CACHE_DIR": os.path.join(scratch, "format-cache" + ("" if hide else "-tl")),
           "FLASHTEX_HOST_UNICODE": a.host_unicode, **DATE}
    # The user's own fonts (~/Library/Fonts) are system fonts the app sees
    # with the real HOME; the oracle's Core Text sees them too.
    user_fonts = os.path.expanduser("~/Library/Fonts")
    if hide and os.path.isdir(user_fonts):
        env["FLASHTEX_FONT_DIRS"] = user_fonts
    if not hide:
        env["PATH"] = f"{TEXBIN}:" + env["PATH"]
        env["HOME"] = os.environ["HOME"]
        env.update(pristine_config(os.path.join(scratch, "work", "x")))
        for k in ("FLASHTEX_BUNDLE_LOCK", "FLASHTEX_BUNDLE_ALLOW_FETCH"):
            env.pop(k)
    os.makedirs(env["HOME"], exist_ok=True)
    os.makedirs(a.tmp, exist_ok=True)
    return env


def candidate(a, doc, work, hide):
    tag = "notex" if hide else "tl"
    d = os.path.join(work, tag)
    stage(doc, d)
    env = notex_env(a, a.out, hide)
    wrap = ["/usr/bin/sandbox-exec", "-f", a.sandbox] if hide else []
    pdf = os.path.join(work, f"{tag}.pdf")
    if os.path.exists(pdf):
        os.remove(pdf)
    code, out, secs = run(wrap + [a.v3, "build", "main.tex", "-o", pdf, "--mode", "unicode"], d, env)
    r = {"exit": code, "pdf": pdf if os.path.exists(pdf) else None, "secs": round(secs, 2),
         "out": out[-4000:]}
    ccode, cout, _ = run(wrap + [a.v3, "check", "main.tex", "--json", "--mode", "unicode"], d, env)
    diags = []
    for line in cout.splitlines():
        if line.startswith("{"):
            try:
                diags.append(json.loads(line))
            except ValueError:
                pass
    r["diags"] = diags
    r["check_tail"] = cout[-3000:]
    r["missing_chars"] = sum(1 for x in diags if "Missing character" in json.dumps(x))
    return r


def compare(a, ref, cand, work, tag):
    if not (ref and cand):
        return None
    j = os.path.join(work, f"cmp-{tag}.json")
    cmd = [sys.executable, os.path.join(REPO, "tools", "xetex-pdfparity", "compare.py"), ref, cand, "--json", j]
    if not a.visual:
        cmd.append("--no-visual")
    code, out, _ = run(cmd, work, dict(os.environ))
    first = out.strip().splitlines()[0] if out.strip() else ""
    rep = {"ok": code == 0, "summary": first, "text": out[-3000:]}
    try:
        rep["report"] = json.load(open(j))
    except (OSError, ValueError):
        pass
    return rep


def one(a, doc):
    work = os.path.join(a.out, "work", doc[0])
    os.makedirs(work, exist_ok=True)
    res = {"doc": doc[0], "source": doc[2] or os.path.relpath(doc[1], REPO)}
    res["ref"] = reference(doc, work)
    res["notex"] = candidate(a, doc, work, True)
    res["cmp_notex"] = compare(a, res["ref"]["pdf"], res["notex"]["pdf"], work, "notex")
    if a.with_texlive:
        res["tl"] = candidate(a, doc, work, False)
        res["cmp_tl"] = compare(a, res["ref"]["pdf"], res["tl"]["pdf"], work, "tl")
    return res


def verdict(c):
    if c is None:
        return "-"
    return "same" if c["ok"] else c["summary"][:90]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--v3", required=True)
    ap.add_argument("--host-unicode", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--tmp", default="/private/tmp/ftxu", help="TMPDIR (short: the host's socket lives there)")
    ap.add_argument("--only")
    ap.add_argument("--jobs", type=int, default=2)
    ap.add_argument("--with-texlive", action="store_true")
    ap.add_argument("--visual", action="store_true")
    ap.add_argument("--json")
    a = ap.parse_args()
    a.v3, a.host_unicode = os.path.abspath(a.v3), os.path.abspath(a.host_unicode)
    os.makedirs(a.out, exist_ok=True)
    a.out = os.path.realpath(a.out)
    a.sandbox = os.path.join(a.out, "notex.sb")
    deny = " ".join(f'(subpath "{p}")' for p in HIDE)
    with open(a.sandbox, "w") as f:
        f.write(f'(version 1)\n(allow default)\n(deny file-read* file-write* {deny} (literal "/etc/paths.d/TeX"))\n')
    todo = docs(a.only)
    # The first notex run fetches the bundle's core and builds xelatex.fmt: alone.
    results = [one(a, todo[0])] if todo else []
    with concurrent.futures.ThreadPoolExecutor(a.jobs) as ex:
        results += list(ex.map(lambda d: one(a, d), todo[1:]))
    if a.json:
        with open(a.json, "w") as f:
            json.dump(results, f, indent=1, default=str)
    print("| doc | ref | notex build | notex vs xelatex | tl vs xelatex | ref tools | missing chars ref/notex |")
    print("|---|---|---|---|---|---|---|")
    for r in results:
        n = r["notex"]
        print(f"| {r['doc']} | {'pdf' if r['ref']['pdf'] else 'FAIL'} ({r['ref']['secs']} s) | "
              f"{'pdf' if n['pdf'] else 'FAIL'} ({n['secs']} s) | {verdict(r['cmp_notex'])} | "
              f"{verdict(r.get('cmp_tl'))} | {','.join(r['ref']['tools']) or '-'} | "
              f"{r['ref']['missing_chars']}/{n['missing_chars']} |")


if __name__ == "__main__":
    main()
