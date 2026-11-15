#!/usr/bin/env python3
"""A parity hunt's units (lane PARITY-HUNT): corpus documents the gated tiers don't measure this way.

Oracle tooling only (standard library; runs the repository's own harnesses). One unit is one run of this
script over a slice of a corpus manifest (`tools/parity/corpus/*.json`), fetched by `corpus.py` first:

    hunt.py parity  SELECT [-- parity.py arguments]     P-T1/P-T2/L0-L4 against pdfTeX (tools/parity/parity.py)
    hunt.py sound   SELECT [-- soundness.py arguments]  the incremental path against cold runs: random edits of each
                                                        document's entry file (tools/incr-bench/soundness.py)
    hunt.py xelatex SELECT [-- latex.py arguments]      the documents under xelatex: box dumps at every \\shipout,
                                                        the log and the XDV of the XeTeX port against TeX Live's
                                                        (tools/xetex-lockstep/latex.py, trace `boxes`)
    hunt.py suite --pins REPO:TAG:SHA[,REPO:TAG:SHA] --suite base|required|l3kernel|all
                                                        the LaTeX team's suites at other pins than PINS.txt's (a newer
                                                        LaTeX release): tools/latex-suites/run.py through pdfTeX and
                                                        through the engine, then compare_failures.py (a test only the
                                                        engine fails is a finding; the reference run is the baseline)

    SELECT:  --manifest FILE.json [--start I --end J] [--ids ID,ID] [--grep REGEX] [--max-kb N]

`--grep` keeps the documents whose TeX sources match REGEX (e.g. `tikz|pgfplots|beamer|biblatex`), so a unit can
be drawn from one kind of document. Expected data comes only from the oracles (TeX Live's pdfTeX and XeTeX).

Environment: HUNT_HOME (the work directory: cache, output; default ~/.cache/flashtex-agents/parity-hunt),
HUNT_BIN (the engine binaries: flashtex-initex, flashtex-host, the dl3 tools, flashtex-xetex, pdftex.pool;
default HUNT_HOME/bin), TeX Live 2026 first on PATH. It is a report, never a gate.
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "../../.."))
HOME = os.environ.get("HUNT_HOME") or os.path.expanduser("~/.cache/flashtex-agents/parity-hunt")
BIN = os.environ.get("HUNT_BIN") or os.path.join(HOME, "bin")
CACHE = os.path.join(HOME, "cache")


def log(*a):
    print(*a, flush=True)


def docs_key(m):
    return next(k for k, v in m.items() if isinstance(v, list) and v and isinstance(v[0], dict) and "id" in v[0])


def job_name(doc_id):
    """A job name both engines and every shell take: `d` and the id's letters and digits."""
    return "d" + re.sub(r"[^0-9a-zA-Z]", "", doc_id)


def select(a):
    """Fetch the slice and return [(id, entry, source directory)] of what a unit runs on."""
    path = a.manifest if os.path.isfile(a.manifest) else os.path.join(ROOT, "tools/parity/corpus", a.manifest)
    m = json.load(open(path))
    key = docs_key(m)
    docs = m[key][a.start:a.end]
    if a.ids:
        want = set(a.ids.split(","))
        docs = [d for d in m[key] if d["id"] in want]
    sub = dict(m)
    sub[key] = docs
    sub.pop("on_demand", None)
    os.makedirs(os.path.join(HOME, "man"), exist_ok=True)
    subp = os.path.join(HOME, "man", "%s-%d-%d.json" % (m.get("tier", "x"), a.start, a.end or 0))
    json.dump(sub, open(subp, "w"), indent=1)
    env = dict(os.environ, FLASHTEX_PARITY_CACHE=CACHE)
    r = subprocess.run([sys.executable, os.path.join(ROOT, "tools/parity/corpus.py"), "fetch", "--manifest", subp],
                       env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    log("\n".join(r.stdout.splitlines()[-3:]))
    tier = m.get("tier") or a.tier
    out = []
    rx = re.compile(a.grep) if a.grep else None
    for d in docs:
        src = os.path.join(CACHE, "src", tier, d["id"])
        entry = d.get("entry")
        if not entry or "/" in entry or not os.path.isfile(os.path.join(src, entry)):
            continue
        if rx or a.max_kb:
            text, size = "", 0
            for base, _, files in os.walk(src):
                for f in files:
                    p = os.path.join(base, f)
                    size += os.path.getsize(p)
                    if rx and f.endswith((".tex", ".sty", ".cls", ".ltx")) and len(text) < 8 << 20:
                        text += open(p, encoding="latin-1").read()
            if rx and not rx.search(text):
                continue
            if a.max_kb and size > a.max_kb * 1024:
                continue
        out.append((d["id"], entry, src))
    log("%s: %d of %d documents selected" % (tier, len(out), len(docs)))
    return tier, out


def stage(src, entry, dst, job):
    """A copy of a document's tree whose entry file is JOB.tex (with the files TeX finds by job name)."""
    shutil.rmtree(dst, ignore_errors=True)
    shutil.copytree(src, dst, symlinks=True)
    stem = entry[:-4] if entry.endswith(".tex") else entry
    os.rename(os.path.join(dst, entry), os.path.join(dst, job + ".tex"))
    for ext in (".bbl", ".ind", ".gls", ".nls"):
        if os.path.isfile(os.path.join(dst, stem + ext)):
            os.rename(os.path.join(dst, stem + ext), os.path.join(dst, job + ext))


def texbin():
    return os.path.dirname(shutil.which("pdftex") or sys.exit("hunt.py: no pdftex on PATH (TeX Live 2026 first)"))


def pdftex_formats():
    """The new engine's pdflatex.fmt and pdftex.fmt, built as fmtutil does (scripts/engine-parity.sh)."""
    fmt = os.path.join(HOME, "fmt")
    os.makedirs(fmt, exist_ok=True)
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1", FLASHTEX_POOL=os.path.join(BIN, "pdftex.pool"))
    for name, ini in (("pdflatex", "pdflatex.ini"), ("pdftex", "*pdfetex.ini")):
        if os.path.isfile(os.path.join(fmt, name + ".fmt")):
            continue
        r = subprocess.run([os.path.join(BIN, "flashtex-initex"), "-ini", "-jobname=" + name, "-progname=" + name, "-etex",
                            "-translate-file=cp227.tcx", ini], cwd=fmt, env=env, stdin=subprocess.DEVNULL,
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        if not os.path.isfile(os.path.join(fmt, name + ".fmt")):
            sys.exit("hunt.py: the engine did not build %s.fmt:\n%s" % (name, r.stdout[-2000:]))
    return fmt


def cmd_parity(a, rest):
    tier, docs = select(a)
    if not docs:
        return 0
    fmt = pdftex_formats()
    tl = texbin()
    texmf = subprocess.run(["kpsewhich", "-var-value", "TEXMFDIST"], stdout=subprocess.PIPE, text=True).stdout.strip()
    cmd = [sys.executable, os.path.join(ROOT, "tools/parity/parity.py"), "--tier", tier,
           "--engine", os.path.join(BIN, "flashtex-initex"),
           "--engine-env", "FLASHTEX_FORMATS=" + fmt, "--engine-env", "FLASHTEX_POOL=" + os.path.join(BIN, "pdftex.pool"),
           "--texbin", tl, "--oracle-pdftex", os.path.join(tl, "pdftex"), "--texmf", texmf, "--cache", CACHE,
           "--pt1-max-log-mb", "64", "--pt1-timeout", "900",
           "--out", os.path.join(HOME, "out", a.name), "--work", os.path.join(HOME, "work-" + a.name)]
    for i, _, _ in docs:
        cmd += ["--only", i]
    return subprocess.run(cmd + rest, cwd=ROOT, env=dict(os.environ, FLASHTEX_PARITY_CACHE=CACHE)).returncode


def cmd_sound(a, rest):
    tier, docs = select(a)
    ib = os.path.join(HOME, "ib")
    env = dict(os.environ, INCR_BENCH_DIR=ib, CARGO_TARGET_DIR=os.path.join(HOME, "target"))
    if not os.path.isfile(os.path.join(ib, "fmt-main", "pdflatex.fmt")):
        # mkeng.sh copies $CARGO_TARGET_DIR/release's binaries and builds the format
        rel = os.path.join(HOME, "target", "release")
        os.makedirs(rel, exist_ok=True)
        for f in ("flashtex-initex", "flashtex-host", "dl3-keys", "dl3-client", "dl3-dump", "dl3-coldopen"):
            shutil.copy(os.path.join(BIN, f), rel)
        subprocess.run(["bash", os.path.join(ROOT, "tools/incr-bench/mkeng.sh"), "main"], env=env, check=True)
    extras = []
    for i, entry, src in docs:
        job = job_name(i)
        dst = os.path.join(HOME, "sound-src", job)
        stage(src, entry, dst, job)
        extras += ["--extra", "%s:%s" % (dst, job)]
    if not extras:
        return 0
    out = os.path.join(HOME, "out", a.name)
    os.makedirs(out, exist_ok=True)
    cmd = [sys.executable, os.path.join(ROOT, "tools/incr-bench/soundness.py"), "main", "--no-fixtures",
           "--dir", os.path.join(out, "sound"), "--out", os.path.join(out, "soundness.jsonl")] + rest + extras
    return subprocess.run(cmd, env=env).returncode


def cmd_xelatex(a, rest):
    tier, docs = select(a)
    cases = os.path.join(HOME, "xe-cases", a.name)
    shutil.rmtree(cases, ignore_errors=True)
    os.makedirs(cases)
    names = {}
    for i, entry, src in docs:
        job = job_name(i)
        stage(src, entry, os.path.join(cases, job), job)
        json.dump({"trace": "boxes"}, open(os.path.join(cases, job, ".m3case.json"), "w"))
        names[job] = i
    if not names:
        return 0
    out = os.path.join(HOME, "out", a.name)
    os.makedirs(out, exist_ok=True)
    json.dump(names, open(os.path.join(out, "names.json"), "w"), indent=1)
    cmd = [sys.executable, os.path.join(ROOT, "tools/xetex-lockstep/latex.py"), "--engine", os.path.join(BIN, "flashtex-xetex"),
           "--cases-dir", cases, "--json", os.path.join(out, "xelatex.json")] + rest
    with open(os.path.join(out, "xelatex.txt"), "w") as f:
        p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        for line in p.stdout:
            f.write(line)
            if not line.startswith("  ") or len(line) < 400:
                sys.stdout.write(line[:400] if line.endswith("\n") and len(line) <= 400 else line[:400] + "\n")
                sys.stdout.flush()
        return p.wait()


def cmd_xsound(a, rest):
    """Unicode mode's incremental path: tools/incr-bench/unicode.py --verify on each document (keystrokes and
    random edits through flashtex-host-unicode, each compile compared with a from-scratch one)."""
    import concurrent.futures
    tier, docs = select(a)
    cases = os.path.join(HOME, "xs-cases", a.name)
    shutil.rmtree(cases, ignore_errors=True)
    os.makedirs(cases)
    out = os.path.join(HOME, "out", a.name)
    os.makedirs(out, exist_ok=True)
    jobs = []
    for i, entry, src in docs:
        job = job_name(i)
        stage(src, entry, os.path.join(cases, job), job)
        jobs.append((i, job))

    def one(item):
        i, job = item
        cmd = [sys.executable, os.path.join(ROOT, "tools/incr-bench/unicode.py"), os.path.join(BIN, "flashtex-host-unicode"),
               os.path.join(cases, job), "--doc", job, "--verify", "--at", "mid"] + (rest or ["--keys", "2", "--trials", "3"])
        try:
            r = subprocess.run(cmd, capture_output=True, text=True, timeout=int(os.environ.get("HUNT_DOC_TIMEOUT", "1800")))
        except subprocess.TimeoutExpired:
            return i, None, [], "timeout"
        lines = [json.loads(ln) for ln in r.stdout.splitlines() if ln.startswith("{")]
        summary = lines[-1] if lines and "compiles" in lines[-1] else None
        return i, summary, lines, (r.stderr.strip().splitlines() or [""])[-1][:300]

    bad = 0
    with open(os.path.join(out, "xsound.jsonl"), "w") as f, \
            concurrent.futures.ThreadPoolExecutor(int(os.environ.get("HUNT_J", "2"))) as ex:
        for i, summary, lines, err in ex.map(one, jobs):
            for ln in lines:
                ln["doc"] = i
                f.write(json.dumps(ln) + "\n")
            f.flush()
            if summary is None:
                bad += 1
                log("%s: FAILED %s" % (i, err))
                continue
            mism = [ln for ln in lines[:-1] if ln.get("mismatch")]
            bad += len(mism)
            log("%s: %d compiles, %d mismatches (%d accounting only)" % (i, summary["compiles"], summary["mismatches"],
                                                                        summary.get("accounting", 0)))
            for m in mism:
                log("    %s: %s" % (m.get("tag"), str(m["mismatch"])[:300]))
    log("xsound: %d documents, %d mismatches or failures" % (len(jobs), bad))
    return 1 if bad else 0


def cmd_suite(a, rest):
    """tools/latex-suites at the pins of --pins, in a copy of the directory (its .cache holds the checkouts)."""
    tools = os.path.join(HOME, "tools")
    suites = os.path.join(tools, "latex-suites")
    if not os.path.isdir(suites):
        os.makedirs(tools, exist_ok=True)
        shutil.copytree(os.path.join(ROOT, "tools/latex-suites"), suites, ignore=shutil.ignore_patterns(".cache", "__pycache__"))
        os.symlink(os.path.join(ROOT, "tools/parity"), os.path.join(tools, "parity"))
    pins = {}
    for line in open(os.path.join(ROOT, "tools/latex-suites/PINS.txt")):
        if line.strip() and not line.lstrip().startswith("#"):
            repo, tag, sha = line.split()
            pins[repo] = (tag, sha)
    for pin in (a.pins or "").split(","):
        if pin.strip():
            repo, tag, sha = pin.replace(":", " ").split()
            pins[repo] = (tag, sha)
    with open(os.path.join(suites, "PINS.txt"), "w") as f:
        for repo, (tag, sha) in pins.items():
            f.write("%s %s %s\n" % (repo, tag, sha))
    log("pins:", pins)
    subprocess.run(["sh", os.path.join(suites, "fetch.sh")], check=True)
    fmt = pdftex_formats()
    out = os.path.join(HOME, "out", a.name)
    os.makedirs(out, exist_ok=True)
    run = [sys.executable, os.path.join(suites, "run.py"), "--suite", a.suite, "--allow-stale"] + rest
    sides = (("reference", ["--engine", os.path.join(texbin(), "pdftex")]),
             ("new", ["--engine", os.path.join(BIN, "flashtex-initex"), "--allow-any-engine",
                      "--engine-env", "FLASHTEX_FORMATS=" + fmt,
                      "--engine-env", "FLASHTEX_POOL=" + os.path.join(BIN, "pdftex.pool")]))
    for side, eng in sides:
        with open(os.path.join(out, "t2-%s.txt" % side), "w") as f:
            r = subprocess.run(run + eng + ["--keep-diffs", os.path.join(out, "diffs-" + side)], stdout=f, stderr=subprocess.STDOUT)
        tail = open(os.path.join(out, "t2-%s.txt" % side), errors="replace").read().splitlines()[-12:]
        log("%s: run.py exit %d" % (side, r.returncode))
        log("\n".join(line[:300] for line in tail))
    return subprocess.run([sys.executable, os.path.join(suites, "compare_failures.py"),
                           os.path.join(out, "t2-reference.txt"), os.path.join(out, "t2-new.txt")]).returncode


def main():
    argv = sys.argv[1:]
    rest = []
    if "--" in argv:
        k = argv.index("--")
        argv, rest = argv[:k], argv[k + 1:]
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("mode", choices=("parity", "sound", "xelatex", "xsound", "suite"))
    ap.add_argument("--name", help="the unit's name: its output is HUNT_HOME/out/NAME (default: the mode)")
    ap.add_argument("--manifest", help="a file under tools/parity/corpus/, or a path")
    ap.add_argument("--pins", help="suite: pins that replace PINS.txt's, as REPO:TAG:SHA, comma separated")
    ap.add_argument("--suite", default="all", choices=("base", "required", "l3kernel", "all"))
    ap.add_argument("--tier", help="the tier, for a manifest that names none")
    ap.add_argument("--start", type=int, default=0)
    ap.add_argument("--end", type=int)
    ap.add_argument("--ids", help="these documents instead of START..END, comma separated")
    ap.add_argument("--grep", help="only documents whose TeX sources match this regular expression")
    ap.add_argument("--max-kb", type=int, help="only documents whose tree is at most this large")
    a = ap.parse_args(argv)
    a.name = a.name or a.mode
    if a.mode != "suite" and not a.manifest:
        ap.error("--manifest is required")
    os.makedirs(CACHE, exist_ok=True)
    return {"parity": cmd_parity, "sound": cmd_sound, "xelatex": cmd_xelatex, "xsound": cmd_xsound, "suite": cmd_suite}[a.mode](a, rest)


if __name__ == "__main__":
    sys.exit(main())
