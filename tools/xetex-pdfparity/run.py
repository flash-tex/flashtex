#!/usr/bin/env python3
"""PDF parity: a candidate engine's PDF against xelatex's, case by case.

docs/design/xetex/PLAN.md §3.5 (item 4 of §4A's S2 part 2 note). For each
case `.tex` (tools/xetex-lockstep/latex-cases and this directory's
`cases/`), each engine builds its own `xelatex.fmt` and runs the case in its
own directory exactly as tools/xetex-lockstep/latex.py does (its functions
are imported: the format build, the `xelatex` symlink, the pinned
environment `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 TZ=UTC`, the
arguments), but in PDF mode: latex.py's arguments without `-no-pdf`. TeX
Live's xetex then writes the PDF through xdvipdfmx; the FlashTeX binary,
invoked the same way, writes `<job>.pdf` itself. The reference runs until
its auxiliary files (.aux .toc .out .lof .lot .nav .snm .bbl) stop
changing (at most --max-passes); the candidate runs the same number of
passes. Then compare.py compares the two PDFs, and the XDV check
(precision.xdv_check) holds the candidate's glyph origins to TeX's exact
positions, read from the XDV of one more `-no-pdf` reference pass, within
--xdv-tol (0.001 bp). In the self-test that check measures xdvipdfmx and
does not gate.

    run.py --engine <bin> [--cases GLOB ...] [--jobs N] [--keep] [--json OUT]
    run.py --self-test        # xelatex against xelatex, and against qpdf rewrites

TeX Live 2026's xetex is the oracle only (DESIGN.md); it never runs in the
product path.
"""
import argparse
import fnmatch
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
LOCKSTEP_DIR = os.path.join(ROOT, "tools", "xetex-lockstep")
sys.path.insert(0, HERE)
import compare as C  # noqa: E402
import precision as P  # noqa: E402


def _load_latex():
    """tools/xetex-lockstep/latex.py, imported (not forked). It does `import
    run` for tools/xetex-lockstep/run.py, a name this file also has, so that
    module is loaded first and stands as `run` while latex.py loads."""
    import importlib.util

    def load(name, path):
        spec = importlib.util.spec_from_file_location(name, path)
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        return mod

    saved = sys.modules.get("run")
    try:
        sys.modules["run"] = load("xetex_lockstep_run", os.path.join(LOCKSTEP_DIR, "run.py"))
        return load("xetex_lockstep_latex", os.path.join(LOCKSTEP_DIR, "latex.py"))
    finally:
        if saved is None:
            sys.modules.pop("run", None)
        else:
            sys.modules["run"] = saved


L = _load_latex()
from pdfdoc import QPDF  # noqa: E402

X = L.X  # tools/xetex-lockstep/run.py
CASE_DIRS = [L.CASES, os.path.join(HERE, "cases")]
PDF_ARGS = [a for a in L.RUN_ARGS if a != "-no-pdf"]
AUX = (".aux", ".toc", ".out", ".lof", ".lot", ".nav", ".snm", ".bbl")


def all_cases():
    out = {}
    for d in CASE_DIRS:
        for fn in sorted(os.listdir(d)):
            if fn.endswith(".tex"):
                out[fn[:-4]] = os.path.join(d, fn)
    return out


def aux_state(d, name):
    h = hashlib.sha256()
    for ext in AUX:
        p = os.path.join(d, name + ext)
        if os.path.exists(p):
            with open(p, "rb") as f:
                h.update(ext.encode() + f.read())
    return h.hexdigest()


def run_engine(binary, fmt_dir, src, d, name, passes, timeout):
    """Run `passes` passes (or, with passes=None, until the auxiliary files
    are stable). Returns (passes run, exit status of the last, seconds)."""
    os.makedirs(d, exist_ok=True)
    shutil.copy(src, os.path.join(d, name + ".tex"))
    exe = L.link(binary, os.path.join(d, ".bin"), "xelatex")
    env = L.env_for(fmt_dir)
    n = 0
    code = None
    t = time.time()
    state = aux_state(d, name)
    limit = passes or 1
    while n < limit:
        try:
            r = subprocess.run([exe] + PDF_ARGS + [name + ".tex"], cwd=d, env=env,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                               timeout=timeout)
            code = r.returncode
        except subprocess.TimeoutExpired:
            code = "timeout"
            n += 1
            break
        n += 1
        new = aux_state(d, name)
        if passes is None:
            if new != state and n < run_engine.max_passes:
                limit = n + 1
        state = new
    return n, code, time.time() - t


run_engine.max_passes = 4


def reference_xdv(binary, fmt_dir, d, name, timeout):
    """One more reference pass with `-no-pdf` (latex.py's own arguments) in
    the reference's directory, after its PDF passes: the auxiliary files are
    stable, so it typesets the same pages, and its XDV holds TeX's exact
    positions. Returns the XDV's path, or None."""
    exe = L.link(binary, os.path.join(d, ".bin"), "xelatex")
    pdf = os.path.join(d, name + ".pdf")
    keep = pdf + ".keep"
    if os.path.exists(pdf):
        shutil.copy(pdf, keep)
    try:
        subprocess.run([exe] + L.RUN_ARGS + [name + ".tex"], cwd=d, env=L.env_for(fmt_dir),
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=timeout)
    except subprocess.TimeoutExpired:
        return None
    finally:
        if os.path.exists(keep):
            os.replace(keep, pdf)
    xdv = os.path.join(d, name + ".xdv")
    return xdv if os.path.exists(xdv) else None


def run_case(name, src, engines, fmts, tmp, opt, timeout, keep, xdv_check=None, gate_xdv=True):
    res = {"case": name}
    dirs = {w: os.path.join(tmp, name, w) for w in ("reference", "candidate")}
    n, rc, rt = run_engine(engines["reference"], fmts["reference"], src, dirs["reference"], name, None, timeout)
    xdv = reference_xdv(engines["reference"], fmts["reference"], dirs["reference"], name, timeout) \
        if xdv_check is not None else None
    m, cc, ct = run_engine(engines["candidate"], fmts["candidate"], src, dirs["candidate"], name, n, timeout)
    res.update(passes=n, exit=[rc, cc], seconds=[round(rt, 2), round(ct, 2)])
    ra = os.path.join(dirs["reference"], name + ".pdf")
    ca = os.path.join(dirs["candidate"], name + ".pdf")
    msgs = []
    if rc != cc:
        msgs.append("exit status reference=%s candidate=%s" % (rc, cc))
    if rc != 0:
        msgs.append("the reference exits %s: the case is broken (see its .log)" % rc)
    if not os.path.exists(ra):
        res.update(ok=False, error="the reference wrote no PDF", messages=msgs)
        return res
    if not os.path.exists(ca):
        res.update(ok=False, error="the candidate wrote no PDF", messages=msgs)
        return res
    try:
        rep = C.compare(ra, ca, opt, tag=name)
    except Exception as e:  # report, do not hide, a harness failure
        res.update(ok=False, error="compare failed: %s: %s" % (type(e).__name__, e), messages=msgs)
        return res
    with open(ra, "rb") as f, open(ca, "rb") as g:
        res["identical_bytes"] = f.read() == g.read()
    res["report"] = rep.as_json()
    res["summary"] = C.summary_line(rep)
    res["text"] = rep.text("    ")
    if xdv_check is not None:
        if xdv is None:
            msgs.append("no reference XDV for the position check")
        else:
            x = P.xdv_check(rep.models[1], xdv, xdv_check, examples=opt.examples)
            res["xdv"] = x
            if x["over"] and gate_xdv:
                msgs.append("%d glyphs more than %g bp from TeX's position (XDV)" % (x["over"], xdv_check))
            if x["unpaired_pdf"] and gate_xdv:
                msgs.append("%d glyphs without a position in the XDV" % x["unpaired_pdf"])
    res["messages"] = msgs
    res["ok"] = rep.ok() and not msgs
    if keep:
        res["dirs"] = dirs
    return res


QPDF_REWRITES = {
    # Different object numbers, object streams, recompressed streams.
    "objstreams": ["--object-streams=generate", "--recompress-flate", "--compression-level=1"],
    # Uncompressed QDF: every stream decoded and rewritten.
    "qdf": ["--qdf", "--object-streams=disable"],
}


def self_test_rewrites(name, pdf, opt, tmp):
    """The reference against qpdf rewrites of itself: the comparison must not
    depend on object layout or compression."""
    out = {}
    for tag, args in QPDF_REWRITES.items():
        dst = os.path.join(tmp, "%s-%s.pdf" % (name, tag))
        r = subprocess.run([QPDF] + args + [pdf, dst], capture_output=True)
        if r.returncode not in (0, 3):
            out[tag] = {"error": r.stderr.decode()[:300]}
            continue
        rep = C.compare(pdf, dst, opt, tag=name + "-" + tag)
        out[tag] = {"structural": rep.structural, "visual": rep.visual, "ok": rep.ok(),
                    "text": rep.text("      ")}
    return out


def judge(r, base):
    """A case under a baseline (--baseline): it passes when the run itself
    is clean (exit statuses, PDFs, the XDV check), every structural
    difference is a ToUnicode difference the baseline lists for the case,
    and the differing pixels are at most the baseline's. Sets r["ok"] and
    r["baseline"]; returns r["ok"]."""
    b = (base.get("cases") or {}).get(r["case"], {})
    allowed_px = b.get("px", 0)
    allowed_tu = [json.dumps(d, sort_keys=True, ensure_ascii=False) for d in b.get("tounicode", [])]
    out = {"px_allowed": allowed_px, "unexpected": []}
    if "report" not in r:
        r["ok"] = False
        r["baseline"] = out
        return False
    rep = r["report"]
    seen = set()
    for kind, v in rep["kinds"].items():
        if kind == "visual":
            continue
        if kind == "tounicode":
            for d in rep["details"].get("tounicode", []):
                key = json.dumps(d, sort_keys=True, ensure_ascii=False)
                if key in allowed_tu:
                    seen.add(key)
                else:
                    out["unexpected"].append("tounicode %s: %r vs %r" % (d["glyph"], d["reference"], d["candidate"]))
            continue
        out["unexpected"].append("%s: %d (%s)" % (kind, v["count"], "; ".join(v["examples"][:2])))
    px = rep["visual_pixels"]
    out["px"] = px
    if px > allowed_px:
        out["unexpected"].append("%d px differ at %gx, baseline %d" % (px, rep["stats"].get("visual_scale", 0), allowed_px))
    out["stale"] = [k for k in allowed_tu if k not in seen]
    r["baseline"] = out
    r["ok"] = not r.get("messages") and not out["unexpected"]
    return r["ok"]


def write_baseline(path, results, a, opt, engine):
    import datetime
    cases = {}
    for r in results:
        if "report" not in r:
            print("write-baseline: %s has no report (%s); left out" % (r["case"], r.get("error")))
            continue
        rep = r["report"]
        entry = {"px": rep["visual_pixels"], "tounicode": rep["details"].get("tounicode", [])}
        other = {k: v["count"] for k, v in rep["kinds"].items() if k not in ("visual", "tounicode")}
        if other or r.get("messages"):
            # Recorded so the file says so, but never allowed by judge().
            entry["not_allowed"] = {"structural": other, "messages": r.get("messages", [])}
            print("write-baseline: %s has differences a baseline does not allow: %s" % (r["case"], entry["not_allowed"]))
        cases[r["case"]] = entry
    data = {"comment": "tools/xetex-pdfparity/run.py --baseline: per case, the differing pixels allowed at "
                       "`scale` and the ToUnicode differences allowed. Written by --write-baseline.",
            "written": datetime.date.today().isoformat(), "candidate": a.label or engine,
            "scale": opt.scale, "tol": opt.tol, "rel_tol": opt.rel_tol, "xdv_tol": a.xdv_tol,
            "cases": cases}
    with open(path, "w") as f:
        json.dump(data, f, indent=1, sort_keys=True, ensure_ascii=False)
        f.write("\n")
    print("baseline written: %s (%d cases)" % (path, len(cases)))


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--engine", help="candidate engine binary")
    ap.add_argument("--reference", default="xetex")
    ap.add_argument("--self-test", action="store_true",
                    help="candidate = the reference; also compare each reference PDF with qpdf rewrites of it")
    ap.add_argument("--cases", nargs="*", default=["*"])
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--timeout", type=float, default=900)
    ap.add_argument("--max-passes", type=int, default=4)
    ap.add_argument("--keep", action="store_true")
    ap.add_argument("--formats", help="build the formats in DIR/fmt-reference and DIR/fmt-candidate and "
                    "reuse them on later runs (for iterating; rebuild when an engine changes)")
    ap.add_argument("--xdv-tol", type=float, default=0.001,
                    help="the XDV check: the candidate's glyph origins against TeX's exact positions "
                         "in the reference's XDV, tolerance in bp (default 0.001)")
    ap.add_argument("--no-xdv-check", action="store_true", help="skip the XDV check")
    ap.add_argument("--json", help="write every case's report here")
    ap.add_argument("--baseline", help="a baseline (JSON, --write-baseline): a case passes with its listed "
                    "ToUnicode differences and at most its listed pixels (a CI gate)")
    ap.add_argument("--write-baseline", help="write this run's pixels and ToUnicode differences as a baseline")
    ap.add_argument("--label", help="what the candidate is (a commit), recorded by --write-baseline")
    C.add_options(ap)
    a = ap.parse_args(argv)
    if not a.engine and not a.self_test:
        ap.error("--engine or --self-test")
    if not X.check_reference(a.reference):
        print("reference %s is not %s" % (a.reference, X.PINNED_REFERENCE))
        return 2
    run_engine.max_passes = a.max_passes
    engine = a.engine or a.reference
    opt = C.options_from(a)
    base = None
    if a.baseline:
        with open(a.baseline) as f:
            base = json.load(f)
        if base.get("scale") != opt.scale:
            print("baseline %s is at scale %s, this run at %s" % (a.baseline, base.get("scale"), opt.scale))
            return 2
    cases = all_cases()
    names = [n for n in cases if any(fnmatch.fnmatch(n, p) for p in a.cases)]
    tmp = tempfile.mkdtemp(prefix="xetex-pdfparity-")
    if a.diff_dir is None and a.keep:
        opt.diff_dir = os.path.join(tmp, "diff")
    engines = {"reference": a.reference, "candidate": engine}
    fmts = {}

    def fmt(who):
        d = os.path.join(a.formats or tmp, "fmt-" + who)
        if a.formats and os.path.exists(os.path.join(d, "xelatex.fmt")):
            return who, d, (0.0, 0, "")
        os.makedirs(d, exist_ok=True)
        return who, d, L.build_format(engines[who], d)

    with ThreadPoolExecutor(max_workers=2) as ex:
        for who, d, (secs, code, _log) in ex.map(fmt, ("reference", "candidate")):
            print("format (%s): exit %s, %.1f s" % (who, code, secs))
            if code != 0:
                print("the %s could not build xelatex.fmt" % who)
                return 1
            fmts[who] = d
    results = []
    ok = 0
    totals = {"structural": 0, "visual": 0}
    with ThreadPoolExecutor(max_workers=a.jobs) as ex:
        # In the self-test the candidate is xdvipdfmx's PDF: the XDV check
        # then measures xdvipdfmx (README) and does not gate.
        xc = None if a.no_xdv_check else a.xdv_tol
        futs = [ex.submit(run_case, n, cases[n], engines, fmts, tmp, opt, a.timeout, a.keep, xc,
                          not a.self_test) for n in names]
        for f in futs:
            r = f.result()
            if a.self_test and "report" in r:
                ref_pdf = os.path.join(tmp, r["case"], "reference", r["case"] + ".pdf")
                r["rewrites"] = self_test_rewrites(r["case"], ref_pdf, opt, os.path.join(tmp, r["case"]))
                if not all(v.get("ok") for v in r["rewrites"].values()):
                    r["ok"] = False
            if base is not None:
                judge(r, base)
            results.append(r)
            ok += bool(r.get("ok"))
            if "report" in r:
                totals["structural"] += r["report"]["structural_differences"]
                totals["visual"] += r["report"]["visual_pixels"]
            print("%s %s (%d passes, exit %s): %s" % (
                "PASS" if r.get("ok") else "FAIL", r["case"], r.get("passes", 0),
                "/".join(str(x) for x in r.get("exit", [])), r.get("summary") or r.get("error")))
            x = r.get("xdv")
            if x:
                totals["xdv_over"] = totals.get("xdv_over", 0) + x["over"]
                totals["xdv_max_bp"] = max(totals.get("xdv_max_bp", 0.0), x["max_bp"])
                totals["xdv_max_em"] = max(totals.get("xdv_max_em", 0.0), x["max_em"])
                print("    XDV check%s: %d glyphs paired (%d of the PDF's and %d of the XDV's not), "
                      "%d more than %g bp off, max %.6f bp (%.6f em)" % (
                          " (measures xdvipdfmx, not gating)" if a.self_test else "",
                          x["paired"], x["unpaired_pdf"], x["unpaired_xdv"], x["over"], x["tol"], x["max_bp"], x["max_em"]))
                for e in x["examples"][:3 if a.self_test else len(x["examples"])]:
                    print("      " + e)
            for m in r.get("messages", []):
                print("    " + m)
            if r.get("text"):
                print(r["text"])
            bl = r.get("baseline")
            if bl is not None:
                print("    baseline: %s px of %d allowed; %s" % (
                    bl.get("px", "-"), bl["px_allowed"],
                    ("not allowed: " + " | ".join(bl["unexpected"])) if bl["unexpected"] else "nothing else"))
                if bl.get("stale"):
                    print("    baseline lists %d ToUnicode differences that no longer occur" % len(bl["stale"]))
            for tag, v in (r.get("rewrites") or {}).items():
                print("    vs qpdf %s rewrite: %s" % (tag, v.get("error") or "%d structural, %d px%s" % (
                    v["structural"], v["visual"], ("\n" + v["text"]) if v["text"] else "")))
    print("%d cases, %d pass, %d fail; %d structural differences, %d differing pixels at %gx" % (
        len(names), ok, len(names) - ok, totals["structural"], totals["visual"], opt.scale))
    if "xdv_over" in totals:
        print("XDV check: %d glyphs more than %g bp from TeX's positions, max %.6f bp (%.6f em)%s" % (
            totals["xdv_over"], a.xdv_tol, totals["xdv_max_bp"], totals["xdv_max_em"],
            " (xdvipdfmx's error: not gating in the self-test)" if a.self_test else ""))
    if base is not None:
        print("baseline %s: %d of %d cases pass" % (a.baseline, ok, len(names)))
    if a.write_baseline:
        write_baseline(a.write_baseline, results, a, opt, engine)
    if a.json:
        with open(a.json, "w") as f:
            json.dump({"engine": engine, "reference": a.reference, "self_test": a.self_test,
                       "tol": opt.tol, "rel_tol": opt.rel_tol, "scale": opt.scale,
                       "cases": results, "totals": totals}, f, indent=1, default=str)
    if a.keep:
        print("kept:", tmp)
    else:
        shutil.rmtree(tmp, ignore_errors=True)
    return 0 if ok == len(names) else 1


if __name__ == "__main__":
    sys.exit(main())
