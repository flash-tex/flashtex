#!/usr/bin/env python3
"""Differential Unicode fuzzer: generated xetex documents, FlashTeX vs XeTeX. MIT.

    python3 tools/fuzz-unicode/fuzz.py --candidate BIN [--oracle xetex]
        --seeds 0:200 [--workers 2] [--out DIR] [--timeout 60] [--pdf]

Each seed is one document from gen.py. Both engines run it through
tools/xetex-lockstep's `run_one` (a symlink named `xetex`, `-ini -etex
-no-pdf`, nonstopmode WITHOUT -halt-on-error so error recovery is compared
too, max_print_line 1000, error_line 254, SOURCE_DATE_EPOCH=0,
FORCE_SOURCE_DATE=1, TZ=UTC), each in its own directory with the picture
files of tools/xetex-lockstep/pictures. Compared, per P-T1 and the XDV rule
of docs/design/xetex/PLAN.md §3.5:

  * the log after the lockstep normalisation (`compared_lines`);
  * the exit status;
  * the XDV file after `normalise_xdv` (preamble date, font paths);
  * the terminal output.

With `--pdf` both engines also run without `-no-pdf` (xetex through
xdvipdfmx, the candidate's own writer) and the PDFs are compared by
tools/xetex-pdfparity/compare.py (structural, P-T2's Unicode form; needs
fontTools and qpdf, so run it under a Python that has fontTools).

TeX Live 2026's xetex is the oracle only (DESIGN.md §13). Every non-equal
seed is written to OUT/<class>/<seed>/; OUT/summary.json groups seeds by
signature. Exit 0 when every seed was equal or neutral.
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
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
import gen  # noqa: E402


def _load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


XL = _load("_xetex_lockstep_run", os.path.join(ROOT, "tools", "xetex-lockstep", "run.py"))
LS = XL.LS
PICTURES = XL.PICTURES
ARGS_XDV = [a for a in XL.ENGINE_ARGS if a != "-halt-on-error"]
ARGS_PDF = [a for a in ARGS_XDV if a != "-no-pdf"]
FSIZE_LIMIT = 64 * 1024 * 1024
NEUTRAL = ("equal", "both-hang", "both-signal", "oracle-crash")
DIGITS = re.compile(r"\d+")
DOC_MTIME = 1000000000


def cap_output():
    try:
        import resource
        resource.setrlimit(resource.RLIMIT_FSIZE, (FSIZE_LIMIT, FSIZE_LIMIT))
    except (ImportError, ValueError, OSError):
        pass


def stage(text, workdir):
    os.makedirs(workdir, exist_ok=True)
    if os.path.isdir(PICTURES):
        for f in os.listdir(PICTURES):
            shutil.copy(os.path.join(PICTURES, f), os.path.join(workdir, f))
    tex = os.path.join(workdir, "doc.tex")
    with open(tex, "w", encoding="utf-8") as fh:
        fh.write(text)
    os.utime(tex, (DOC_MTIME, DOC_MTIME))


def run(engine, text, workdir, timeout, args=ARGS_XDV, env=None):
    stage(text, workdir)
    old = dict(os.environ)
    try:
        if env:
            os.environ.update(env)
        r = XL.run_one(engine, workdir, args + ["doc.tex"], "doc", timeout)
    finally:
        if env:
            os.environ.clear()
            os.environ.update(old)
    if r.get("error") == "timed out":
        return None
    return r


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


def crashed(rc):
    return rc < 0 or rc >= 128 or rc == 101


def compare_runs(ro, rc):
    """Compare two run_one results; a result dict with cls and sig."""
    if ro is None and rc is None:
        return {"cls": "both-hang"}
    if ro is None or rc is None:
        return {"cls": "timeout", "sig": "timeout:" + ("oracle" if ro is None else "candidate")}
    if "error" in ro or "error" in rc:
        return {"cls": "run-error", "sig": "run-error:%s/%s" % (ro.get("error"), rc.get("error"))}
    res = {"rc": [ro["returncode"], rc["returncode"]], "logs": (ro["log"], rc["log"])}
    if crashed(ro["returncode"]):
        if ro["returncode"] == rc["returncode"]:
            res.update(cls="both-signal", sig="both-signal:%d" % rc["returncode"])
        else:
            # XeTeX itself crashed: undefined behaviour, not comparable.
            res.update(cls="oracle-crash", sig="oracle-crash:%d" % ro["returncode"])
        return res
    if crashed(rc["returncode"]):
        res.update(cls="candidate-crash", sig="crash:%d" % rc["returncode"])
        return res
    lo, lc = LS.compared_lines(ro["log"]), LS.compared_lines(rc["log"])
    d = first_diff(lo, lc)
    if d is not None:
        i, x, y = d
        res.update(cls="log", sig=signature("log", x + " || " + y),
                   detail={"line": i + 1, "oracle": x, "candidate": y,
                           "context": lo[max(0, i - 8):i]})
        return res
    if ro["returncode"] != rc["returncode"]:
        res.update(cls="rc", sig="rc:%d/%d" % (ro["returncode"], rc["returncode"]))
        return res
    x = XL.compare_xdv(ro["xdv"], rc["xdv"])
    if x is not None:
        res.update(cls="xdv", sig=signature("xdv", re.sub(r"at byte \d+ \(sizes \d+ vs \d+\)", "", x)),
                   detail=x)
        return res
    to, tc = ro["terminal"].split("\n"), rc["terminal"].split("\n")
    d = first_diff(to, tc)
    if d is not None:
        i, x, y = d
        res.update(cls="terminal", sig=signature("terminal", x + " || " + y),
                   detail={"line": i + 1, "oracle": x, "candidate": y})
        return res
    res["cls"] = "equal"
    return res


def compare_pdf(text, candidate, oracle, work, timeout):
    od, cd = os.path.join(work, "pdf-oracle"), os.path.join(work, "pdf-candidate")
    ro = run(oracle, text, od, timeout, ARGS_PDF)
    rc = run(candidate, text, cd, timeout, ARGS_PDF)
    if ro is None or rc is None or "error" in ro or "error" in rc:
        return None
    po, pc = os.path.join(od, "doc.pdf"), os.path.join(cd, "doc.pdf")
    if not os.path.exists(po) and not os.path.exists(pc):
        return None
    if os.path.exists(po) != os.path.exists(pc):
        return {"cls": "pdf", "sig": "pdf-missing:" + ("oracle" if not os.path.exists(po) else "candidate")}
    out = os.path.join(work, "pdfcmp.json")
    p = subprocess.run([sys.executable, os.path.join(ROOT, "tools", "xetex-pdfparity", "compare.py"),
                        po, pc, "--json", out, "--no-visual"],
                       capture_output=True, text=True, timeout=timeout)
    if p.returncode == 0:
        return None
    first = ""
    try:
        with open(out) as fh:
            rep = json.load(fh)
        diffs = rep.get("differences") or rep.get("diffs") or []
        first = json.dumps(diffs[0] if diffs else rep)[:300]
    except (OSError, ValueError):
        first = (p.stdout + p.stderr)[-300:]
    return {"cls": "pdf", "sig": signature("pdf", first), "detail": first}


def compare(text, candidate, oracle, work, timeout, pdf=False):
    ro = run(oracle, text, os.path.join(work, "oracle"), timeout)
    rc = run(candidate, text, os.path.join(work, "candidate"), timeout)
    res = compare_runs(ro, rc)
    if res["cls"] == "equal" and pdf:
        p = compare_pdf(text, candidate, oracle, work, timeout)
        if p:
            res.update(p)
    return res


def fuzz_seed(seed, args):
    text = gen.generate(seed).render()
    work = tempfile.mkdtemp(prefix="fzu-%d-" % seed, dir=args.tmp)
    try:
        res = compare(text, args.candidate, args.oracle, work, args.timeout, args.pdf)
    finally:
        if not args.keep:
            shutil.rmtree(work, ignore_errors=True)
    logs = res.pop("logs", None)
    if res["cls"] not in NEUTRAL or res["cls"] == "oracle-crash":
        d = os.path.join(args.out, res["cls"], str(seed))
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, "doc.tex"), "w", encoding="utf-8") as fh:
            fh.write(text)
        if logs:
            for name, log in zip(("oracle.log", "candidate.log"), logs):
                with open(os.path.join(d, name), "w", encoding="utf-8") as fh:
                    fh.write(log)
        with open(os.path.join(d, "result.json"), "w") as fh:
            json.dump(res, fh, indent=1, default=str)
    res["seed"] = seed
    return res


_LATEX = None


def latex_mod():
    global _LATEX
    if _LATEX is None:
        _LATEX = _load("_xetex_lockstep_latex", os.path.join(ROOT, "tools", "xetex-lockstep", "latex.py"))
    return _LATEX


def build_formats(args):
    """Each engine's own xelatex.fmt, built as fmtutil does (latex.py)."""
    L = latex_mod()
    fmts = {}
    for who, binary in (("reference", args.oracle), ("candidate", args.candidate)):
        d = os.path.join(args.formats, who)
        if not os.path.exists(os.path.join(d, "xelatex.fmt")):
            os.makedirs(d, exist_ok=True)
            secs, rc, _ = L.build_format(binary, d)
            print("%s xelatex.fmt: exit %s, %.0f s" % (who, rc, secs), flush=True)
        fmts[who] = d
    return fmts


def fuzz_latex_seed(seed, args):
    """A gen.py xelatex/fontspec document, through tools/xetex-lockstep's
    latex.py run_case (an untraced run, then a run traced to the box dumps;
    logs, exit statuses and XDV files compared)."""
    L = latex_mod()
    text = gen.generate(seed, latex=True).render()
    work = tempfile.mkdtemp(prefix="fzul-%d-" % seed, dir=args.tmp)
    name = "doc"
    src = os.path.join(work, "src", name)
    os.makedirs(src)
    with open(os.path.join(src, name + ".tex"), "w", encoding="utf-8") as fh:
        fh.write(text)
    with open(os.path.join(src, L.META), "w") as fh:
        json.dump({"trace": "boxes"}, fh)
    try:
        _, ok, msgs, _, info = L.run_case(
            name, {"reference": args.oracle, "candidate": args.candidate}, args.fmts,
            os.path.join(work, "run"), args.timeout, src=src, keep=False)
    finally:
        if not args.keep:
            shutil.rmtree(work, ignore_errors=True)
    res = {"seed": seed}
    if ok:
        res["cls"] = "equal"
        return res
    first = msgs[0] if msgs else "?"
    cls = "latex-" + ("log" if first.startswith("log differs") else
                      "xdv" if first.startswith("XDV") else
                      "crash" if "panicked" in first else "rc")
    res.update(cls=cls, sig=signature(cls, " | ".join(m.strip() for m in msgs[:3])), detail=msgs[:12])
    d = os.path.join(args.out, cls, str(seed))
    os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, "doc.tex"), "w", encoding="utf-8") as fh:
        fh.write(text)
    with open(os.path.join(d, "result.json"), "w") as fh:
        json.dump(res, fh, indent=1, default=str)
    return res


def parse_seeds(spec):
    a, _, n = spec.partition(":")
    return range(int(a), int(a) + int(n or 1))


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--candidate", required=True)
    ap.add_argument("--oracle", default="xetex")
    ap.add_argument("--seeds", default="0:50", help="START:COUNT")
    ap.add_argument("--workers", type=int, default=2)
    ap.add_argument("--out", default="fuzz-unicode-out")
    ap.add_argument("--tmp", default=None)
    ap.add_argument("--timeout", type=float, default=60)
    ap.add_argument("--keep", action="store_true")
    ap.add_argument("--pdf", action="store_true", help="also compare the PDFs (xdvipdfmx vs the candidate's writer)")
    ap.add_argument("--latex", action="store_true",
                    help="xelatex/fontspec documents instead of xetex -ini ones")
    ap.add_argument("--formats", default=None,
                    help="with --latex: dir for the two xelatex.fmt files (default OUT/fmt)")
    args = ap.parse_args(argv)
    args.oracle = shutil.which(args.oracle) or args.oracle
    args.candidate = os.path.abspath(args.candidate)
    cap_output()
    os.makedirs(args.out, exist_ok=True)
    one = fuzz_seed
    if args.latex:
        args.formats = os.path.abspath(args.formats or os.path.join(args.out, "fmt"))
        args.fmts = build_formats(args)
        one = fuzz_latex_seed
    results = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=max(1, min(args.workers, 2))) as ex:
        for r in ex.map(lambda s: one(s, args), parse_seeds(args.seeds)):
            results.append(r)
            if r["cls"] not in NEUTRAL:
                print("seed %d: %s %s" % (r["seed"], r["cls"], r.get("sig", "")), flush=True)
    counts, sigs = {}, {}
    for r in results:
        counts[r["cls"]] = counts.get(r["cls"], 0) + 1
        if r["cls"] not in NEUTRAL:
            sigs.setdefault(r.get("sig", r["cls"]), []).append(r["seed"])
    with open(os.path.join(args.out, "summary.json"), "w") as fh:
        json.dump({"counts": counts, "signatures": sigs}, fh, indent=1)
    print("%d seeds: %s; %d distinct signatures" % (
        len(results), ", ".join("%s %d" % kv for kv in sorted(counts.items())), len(sigs)))
    return 0 if sum(counts.get(c, 0) for c in NEUTRAL) == len(results) else 1


if __name__ == "__main__":
    sys.exit(main())
