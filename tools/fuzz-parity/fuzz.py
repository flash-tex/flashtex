#!/usr/bin/env python3
"""Differential parity fuzzer: generated plain-TeX documents, FlashTeX vs pdfTeX. MIT.

    python3 tools/fuzz-parity/fuzz.py --candidate BIN [--oracle pdftex]
        --seeds 0:200 [--workers 2] [--out DIR] [--timeout 60]

Each seed is one document from gen.py. Both engines run it through the
lockstep `capture()` (`-ini -etex`, nonstopmode, no -halt-on-error, same
argv[0], SOURCE_DATE_EPOCH=0) in separate directories, and three things
are compared, per P-T1/P-T2 (DESIGN §1.1):
  * the exit code;
  * the log, normalised by tools/lockstep (`compared_lines`: banner, paths,
    dates and the §1.1 accounting lines replaced), which carries every
    \\showbox / \\showlists / tracing line and every shipped box;
  * the output file: byte-identical, else for a PDF tools/parity's
    `compare_pt2` (qpdf-normalised content streams, fonts, resources).
Every non-equal seed is written to OUT/<class>/<seed>/ with both logs and
a first-difference report. The run prints one summary line and writes
OUT/summary.json. Exit 0 when every seed was equal, 1 otherwise.
"""
import argparse
import concurrent.futures
import importlib.util
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "..", "parity"))
import gen  # noqa: E402

_spec = importlib.util.spec_from_file_location(
    "lockstep_run", os.path.join(HERE, "..", "lockstep", "run.py"))
lockstep = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(lockstep)

# Shell escape fully off for both engines (a generated \write18 never runs).
SHELL_OFF = "-cnf-line=shell_escape=f"
if SHELL_OFF not in lockstep.ENGINE_SHELL_FLAGS:
    lockstep.ENGINE_SHELL_FLAGS.append(SHELL_OFF)

FSIZE_LIMIT = 64 * 1024 * 1024
DIGITS = re.compile(r"\d+")


def cap_output():
    try:
        import resource
        resource.setrlimit(resource.RLIMIT_FSIZE, (FSIZE_LIMIT, FSIZE_LIMIT))
    except (ImportError, ValueError, OSError):
        pass


def run_one(engine, text, workdir, timeout, env=None, fmt=None):
    os.makedirs(workdir, exist_ok=True)
    tex = os.path.join(workdir, "doc.tex")
    with open(tex, "w", encoding="latin-1") as fh:
        fh.write(text)
    try:
        cap = lockstep.capture("doc.tex", engine, workdir, fmt=fmt, extra_env=env,
                               timeout=timeout, halt_on_error=False)
    except subprocess.TimeoutExpired:
        return None
    out = None
    for ext in ("pdf", "dvi"):
        p = os.path.join(workdir, "doc." + ext)
        if os.path.exists(p):
            out = p
    return cap, out


def first_diff(a, b):
    for i, (x, y) in enumerate(zip(a, b)):
        if x != y:
            return i, x, y
    if len(a) != len(b):
        i = min(len(a), len(b))
        return i, (a[i] if i < len(a) else "<EOF>"), (b[i] if i < len(b) else "<EOF>")
    return None


def signature(kind, detail):
    return kind + ":" + DIGITS.sub("#", detail)[:160]


def doc_fmt(text):
    """The format a document runs under: pdflatex for a LaTeX document."""
    return "pdflatex" if "\\documentclass" in text else None


def compare(text, candidate, oracle, work, timeout, formats=None):
    """Run both engines on text; return a result dict (cls, sig, detail).

    A LaTeX document runs as -fmt=pdflatex: the oracle with TeX Live's
    format, the candidate with the one in `formats` (FLASHTEX_FORMATS,
    passed to the candidate only)."""
    fmt = doc_fmt(text)
    cand_env = {"FLASHTEX_FORMATS": formats} if fmt and formats else None
    od, cd = os.path.join(work, "oracle"), os.path.join(work, "candidate")
    ro = run_one(oracle, text, od, timeout, fmt=fmt)
    rc = run_one(candidate, text, cd, timeout, cand_env, fmt=fmt)
    if ro is None and rc is None:
        return {"cls": "both-hang"}
    if ro is None or rc is None:
        side = "oracle" if ro is None else "candidate"
        return {"cls": "timeout", "sig": "timeout:" + side}
    (co, po), (cc, pc) = ro, rc
    res = {"rc": [co.returncode, cc.returncode], "logs": (co.log, cc.log)}
    if cc.returncode < 0 or cc.returncode == 101:
        res.update(cls="candidate-crash", sig="crash:%d" % cc.returncode)
        return res
    lo, lc = lockstep.compared_lines(co.log), lockstep.compared_lines(cc.log)
    d = first_diff(lo, lc)
    if d is not None:
        i, x, y = d
        res.update(cls="log", sig=signature("log", x + " || " + y),
                   detail={"line": i + 1, "oracle": x, "candidate": y,
                           "context": lo[max(0, i - 8):i]})
        return res
    if co.returncode != cc.returncode:
        res.update(cls="rc", sig="rc:%d/%d" % (co.returncode, cc.returncode))
        return res
    if (po is None) != (pc is None):
        res.update(cls="output", sig="output-missing:" + ("oracle" if po is None else "candidate"))
        return res
    if po is not None:
        with open(po, "rb") as f1, open(pc, "rb") as f2:
            same = f1.read() == f2.read()
        if not same:
            if po.endswith(".pdf") and pc.endswith(".pdf"):
                import tiers  # tools/parity; imported lazily (qpdf is only needed here)
                rec = tiers.compare_pt2(po, pc, os.path.join(work, "pt2"))
                if not rec.get("ok"):
                    res.update(cls="pdf", sig=signature("pdf", json.dumps(rec.get("first_page", rec.get("why", "")))),
                               detail=rec)
                    return res
                res["pdf_bytes_differ"] = True
            else:
                res.update(cls="dvi", sig="dvi-bytes")
                return res
    to, tc = co.terminal.split("\n"), cc.terminal.split("\n")
    d = first_diff(to, tc)
    if d is not None:
        i, x, y = d
        res.update(cls="terminal", sig=signature("terminal", x + " || " + y),
                   detail={"line": i + 1, "oracle": x, "candidate": y})
        return res
    res["cls"] = "equal"
    return res


def fuzz_seed(seed, args):
    doc = gen.generate(seed, latex=None if args.formats else False)
    text = doc.render()
    work = tempfile.mkdtemp(prefix="fzp-%d-" % seed, dir=args.tmp)
    try:
        res = compare(text, args.candidate, args.oracle, work, args.timeout, args.formats)
    finally:
        if not args.keep:
            shutil.rmtree(work, ignore_errors=True)
    logs = res.pop("logs", None)
    if res["cls"] != "equal":
        d = os.path.join(args.out, res["cls"], str(seed))
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, "doc.tex"), "w", encoding="latin-1") as fh:
            fh.write(text)
        if logs:
            for name, log in zip(("oracle.log", "candidate.log"), logs):
                with open(os.path.join(d, name), "w", encoding="utf-8") as fh:
                    fh.write(log)
        with open(os.path.join(d, "result.json"), "w") as fh:
            json.dump(res, fh, indent=1, default=str)
    res["seed"] = seed
    return res


def parse_seeds(spec):
    a, _, n = spec.partition(":")
    return range(int(a), int(a) + int(n or 1))


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--candidate", required=True)
    ap.add_argument("--oracle", default="pdftex")
    ap.add_argument("--seeds", default="0:50", help="START:COUNT")
    ap.add_argument("--workers", type=int, default=2)
    ap.add_argument("--out", default="fuzz-parity-out")
    ap.add_argument("--tmp", default=None)
    ap.add_argument("--timeout", type=float, default=60)
    ap.add_argument("--keep", action="store_true")
    ap.add_argument("--formats", default=None,
                    help="dir holding the candidate's own pdflatex.fmt; without it no LaTeX documents")
    args = ap.parse_args(argv)
    if args.formats:
        args.formats = os.path.abspath(args.formats)
    args.oracle = shutil.which(args.oracle) or args.oracle
    args.candidate = os.path.abspath(args.candidate)
    cap_output()
    os.makedirs(args.out, exist_ok=True)
    results = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=max(1, min(args.workers, 2))) as ex:
        for r in ex.map(lambda s: fuzz_seed(s, args), parse_seeds(args.seeds)):
            results.append(r)
            if r["cls"] != "equal":
                print("seed %d: %s %s" % (r["seed"], r["cls"], r.get("sig", "")), flush=True)
    counts = {}
    sigs = {}
    for r in results:
        counts[r["cls"]] = counts.get(r["cls"], 0) + 1
        if r["cls"] != "equal":
            sigs.setdefault(r.get("sig", r["cls"]), []).append(r["seed"])
    with open(os.path.join(args.out, "summary.json"), "w") as fh:
        json.dump({"counts": counts, "signatures": sigs}, fh, indent=1)
    print("%d seeds: %s; %d distinct signatures" % (
        len(results), ", ".join("%s %d" % kv for kv in sorted(counts.items())), len(sigs)))
    return 0 if counts.get("equal", 0) == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
