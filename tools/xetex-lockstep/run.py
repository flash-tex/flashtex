#!/usr/bin/env python3
"""XeTeX lockstep harness: TeX Live's xetex vs the XeTeX port (P-T1 + XDV).

The XeTeX counterpart of tools/lockstep/run.py (docs/design/xetex/PLAN.md,
phase S0). Each case runs through the reference `xetex -no-pdf` and the
candidate with identical arguments and environment, in its own temp dir,
through a symlink named `xetex` (so argv[0]-derived text prints the same),
and the comparison is

  * P-T1 (DESIGN.md §1.1): the normalised transcript logs, box dumps
    included, line for line, with tools/lockstep's normalisation and
    accounting rule (the same functions, imported from its run.py);
  * the exit status;
  * the XDV file, byte for byte, after normalising the date in the
    preamble's comment. (The spike's second normalisation, the path of a
    native font in a `define_native_font` record, comes with native fonts
    in phase S1; S0 writes no such record.)

Cases:

  * tools/lockstep/cases/<name>.tex listed in suite.txt (selected with
    --select: cases that use no pdfTeX-only primitive and that TeX Live's
    xetex runs as pdfTeX does), with prelude.tex here (tools/lockstep's
    without \\pdfoutput);
  * cases/<name>.tex here: XeTeX-specific cases (Unicode input, ^^^^,
    \\Uchar, \\Umathcode, ...), with the same prelude;
  * third_party/xetex/tests: XeTeX's own tests (xetexdir/xetex-*.test),
    run as the .test scripts run them (a format first where they make
    one) and compared with their committed .log as well as with xetex.

Usage:
  run.py --engine <bin> [--cases GLOB ...] [--jobs N] [--keep]
  run.py --select           # rewrite suite.txt from tools/lockstep/cases
  run.py --self-test        # the reference against itself

Stdlib only; MIT, like the rest of tools/.
"""
import argparse
import concurrent.futures
import fnmatch
import importlib.util
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
LOCKSTEP = os.path.join(ROOT, "tools", "lockstep")
LOCKSTEP_CASES = os.path.join(LOCKSTEP, "cases")
OWN_CASES = os.path.join(HERE, "cases")
PICTURES = os.path.join(HERE, "pictures")
PRELUDE = os.path.join(HERE, "prelude.tex")
SUITE = os.path.join(HERE, "suite.txt")
XETEX_TESTS = os.path.join(ROOT, "third_party", "xetex", "tests")

PINNED_REFERENCE = "XeTeX 3.141592653-2.6-0.999998 (TeX Live 2026)"


def _load_lockstep():
    spec = importlib.util.spec_from_file_location(
        "_lockstep_run", os.path.join(LOCKSTEP, "run.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


LS = _load_lockstep()
ENGINE_ARGS = LS.ENGINE_ARGS + ["-no-pdf"]


def pinned_env():
    env = dict(os.environ)
    env.update(SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1", TZ="UTC",
               LC_ALL="C", LANGUAGE="C")
    return env


def link_engine(binary, workdir):
    """A symlink named `xetex` to binary, in its own directory."""
    target = os.path.realpath(shutil.which(binary) or binary)
    d = os.path.join(workdir, ".bin")
    os.makedirs(d, exist_ok=True)
    link = os.path.join(d, "xetex")
    if os.path.lexists(link):
        os.remove(link)
    os.symlink(target, link)
    return link


def normalise(text, tmpdir):
    text = text.replace(os.path.join(tmpdir, ".bin", "xetex"), "<XETEX>")
    return LS.normalise(text, tmpdir)


# ---------------------------------------------------------------------------
# XDV
# ---------------------------------------------------------------------------

XDV_PRE = 247


# XDV's opcodes after the preamble: the number of parameter bytes of each
# fixed-size one (DVI's, plus XeTeX's `set_glyphs` and
# `set_text_and_glyphs`, whose sizes are read from the stream).
def _dvi_param_len(op, b, i):
    if op < 128 or 171 <= op <= 234 or op in (138, 141, 142, 147, 152, 161, 166):
        return 0
    for base in (128, 133, 143, 148, 153, 157, 162, 167, 235):
        if base <= op <= base + 3:
            return op - base + 1
    if op in (132, 137):
        return 8
    if op == 139:
        return 44
    if op == 140:
        return 0
    if 239 <= op <= 242:
        n = op - 238
        return n + int.from_bytes(b[i:i + n], "big")
    if 243 <= op <= 246:
        n = op - 242
        return n + 12 + 2 + b[i + n + 12] + b[i + n + 13]
    if op == 253:
        g = int.from_bytes(b[i + 4:i + 6], "big")
        return 6 + 10 * g
    if op == 254:
        n = int.from_bytes(b[i:i + 2], "big")
        j = 2 + 2 * n
        g = int.from_bytes(b[i + j + 4:i + j + 6], "big")
        return j + 6 + 10 * g
    if op == 248:
        return 28
    return None


def _native_font_def_len(b, i):
    """Bytes of a `define_native_font` record's parameters at `i` (after
    the opcode): k[4] size[4] flags[2] l[1] name[l] index[4], then a colour,
    extend, slant and embolden value of 4 bytes each as the flags say."""
    flags = int.from_bytes(b[i + 8:i + 10], "big")
    n = 4 + 4 + 2 + 1 + b[i + 10] + 4
    for bit in (0x0200, 0x1000, 0x2000, 0x4000):
        if flags & bit:
            n += 4
    return n


def normalise_xdv(data):
    """The XDV bytes with the two normalisations of PLAN.md §3.5: the
    preamble's comment blanked, and in each `define_native_font` record the
    font file's path replaced by its base name (the length byte follows;
    every other byte of the record is kept). The stream is walked opcode by
    opcode, so a 252 inside another command's parameters is never taken
    for a record. A stream that cannot be walked is returned unchanged
    after the first normalisation, so it still compares byte for byte."""
    b = bytearray(data)
    if not (len(b) >= 15 and b[0] == XDV_PRE):
        return bytes(b)
    k = b[14]
    b[15:15 + k] = b"\0" * k
    out = bytearray(b[:15 + k])
    i = 15 + k
    try:
        while i < len(b):
            op = b[i]
            if op == 249:  # post_post: the rest is q[4], i[1] and the 223s
                out += b[i:]
                return bytes(out)
            if op == 252:
                n = _native_font_def_len(b, i + 1)
                rec = b[i + 1:i + 1 + n]
                l = rec[10]
                name = bytes(rec[11:11 + l])
                base = name.rsplit(b"/", 1)[-1]
                out.append(op)
                out += rec[:10] + bytes([len(base)]) + base + rec[11 + l:]
                i += 1 + n
                continue
            n = _dvi_param_len(op, b, i + 1)
            if n is None:
                raise ValueError("opcode %d at %d" % (op, i))
            out += b[i:i + 1 + n]
            i += 1 + n
    except (IndexError, ValueError):
        return bytes(b)
    return bytes(out)


def _self_test_normalise_xdv():
    """A native font record's path is cut to its base name; the rest stays."""
    pre = bytes([247, 7]) + bytes(12) + bytes([3]) + b"abc"
    rec = (bytes([252]) + (1).to_bytes(4, "big") + (655360).to_bytes(4, "big")
           + (0x1000).to_bytes(2, "big") + bytes([12]) + b"/a/b/font.otf"[:12]
           + (0).to_bytes(4, "big") + (65536).to_bytes(4, "big"))
    post = bytes([249]) + bytes(5) + bytes([223] * 4)
    got = normalise_xdv(pre + rec + post)
    want_rec = (bytes([252]) + (1).to_bytes(4, "big") + (655360).to_bytes(4, "big")
                + (0x1000).to_bytes(2, "big") + bytes([7]) + b"font.ot"
                + (0).to_bytes(4, "big") + (65536).to_bytes(4, "big"))
    assert got == pre[:15] + b"\0\0\0" + want_rec + post, got
    other = bytes([247, 7]) + bytes(12) + bytes([3]) + b"xyz"
    rec2 = bytearray(rec)
    rec2[12:24] = b"/c/d/font.ot"
    assert normalise_xdv(other + bytes(rec2) + post) == got


def compare_xdv(ref_path, cand_path):
    """None when equal after normalisation, else a description."""
    try:
        a = open(ref_path, "rb").read()
    except OSError:
        a = None
    try:
        b = open(cand_path, "rb").read()
    except OSError:
        b = None
    if a is None and b is None:
        return None
    if a is None or b is None:
        return "XDV only from %s" % ("candidate" if a is None else "reference")
    na, nb = normalise_xdv(a), normalise_xdv(b)
    if na == nb:
        return None
    i = next((k for k in range(min(len(na), len(nb))) if na[k] != nb[k]),
             min(len(na), len(nb)))
    return "XDV differs at byte %d (sizes %d vs %d)" % (i, len(a), len(b))


# ---------------------------------------------------------------------------
# Runs
# ---------------------------------------------------------------------------

def run_one(binary, workdir, args, job, timeout):
    """Run binary (as `xetex`) in workdir; return a result dict."""
    argv0 = link_engine(binary, workdir)
    try:
        rc, out = LS._run_isolated([argv0] + args, cwd=workdir,
                                   env=pinned_env(), timeout=timeout)
    except subprocess.TimeoutExpired:
        return {"error": "timed out"}
    except FileNotFoundError:
        return {"error": "binary not found"}
    log_path = os.path.join(workdir, job + ".log")
    try:
        with open(log_path, encoding="utf-8", errors="replace",
                  newline="") as fh:
            raw = fh.read()
    except OSError:
        raw = out.decode("utf-8", "replace") if rc != 0 else ""
    return {"returncode": rc,
            "log": normalise(raw, workdir) if raw.strip() else "",
            "terminal": normalise(out.decode("utf-8", "replace"), workdir),
            "xdv": os.path.join(workdir, job + ".xdv")}


def stage_case(src, name, tmp):
    shutil.copy(PRELUDE, os.path.join(tmp, "prelude.tex"))
    shutil.copy(src, os.path.join(tmp, name + ".tex"))
    # The picture files of the p-cases (make_cases.py writes them).
    if os.path.basename(name).startswith("p") and os.path.isdir(PICTURES):
        for f in sorted(os.listdir(PICTURES)):
            shutil.copy(os.path.join(PICTURES, f), os.path.join(tmp, f))


def case_source(name):
    own = os.path.join(OWN_CASES, name + ".tex")
    if os.path.exists(own):
        return own
    return os.path.join(LOCKSTEP_CASES, name + ".tex")


def run_case(name, ref, cand, timeout, keep):
    """Compare one prelude case; returns (name, ok, messages, xdv_state)."""
    src = case_source(name)
    try:
        no_halt = "no-halt" in LS.case_options(src)
    except ValueError as e:
        return name, False, [str(e)], None
    args = [a for a in ENGINE_ARGS if not (no_halt and a == "-halt-on-error")]
    res = {}
    dirs = {}
    for label, binary in (("reference", ref), ("candidate", cand)):
        tmp = tempfile.mkdtemp(prefix="xels-")
        dirs[label] = tmp
        stage_case(src, name, tmp)
        res[label] = run_one(binary, tmp, args + [name + ".tex"], name, timeout)
    msgs = []
    ok = True
    r, c = res["reference"], res["candidate"]
    if "error" in r or "error" in c:
        msgs.append("run failed: reference=%s candidate=%s"
                    % (r.get("error"), c.get("error")))
        ok = False
    else:
        # As tools/lockstep requires: a case runs to its \end (exit 0),
        # unless it is a `% lockstep: no-halt` case, which must end with an
        # error; otherwise the case itself is wrong.
        if no_halt and r["returncode"] == 0:
            msgs.append("case error: `% lockstep: no-halt` but the reference exits 0")
            ok = False
        elif not no_halt and r["returncode"] != 0:
            msgs.append("case error: the reference exits %s" % r["returncode"])
            ok = False
        if r["returncode"] != c["returncode"]:
            msgs.append("returncode reference=%s candidate=%s"
                        % (r["returncode"], c["returncode"]))
            ok = False
        a, b = LS.compared_lines(r["log"]), LS.compared_lines(c["log"])
        if a != b:
            ok = False
            i = next(k for k in range(max(len(a), len(b)))
                     if (a[k] if k < len(a) else "<EOF>")
                     != (b[k] if k < len(b) else "<EOF>"))
            msgs.append("log differs at line %d:" % (i + 1))
            msgs.append("  reference: %s" % (a[i] if i < len(a) else "<EOF>"))
            msgs.append("  candidate: %s" % (b[i] if i < len(b) else "<EOF>"))
    xdv_state = None
    if ok:
        d = compare_xdv(r["xdv"], c["xdv"])
        if os.path.exists(r["xdv"]):
            xdv_state = "identical" if d is None else "differs"
        if d is not None:
            ok = False
            msgs.append(d)
    if not keep:
        for d in dirs.values():
            shutil.rmtree(d, ignore_errors=True)
    else:
        msgs.append("dirs: %s" % dirs)
    return name, ok, msgs, xdv_state


# ---------------------------------------------------------------------------
# XeTeX's own tests (xetexdir/xetex-*.test)
# ---------------------------------------------------------------------------

def xetex_test_steps(name):
    """The engine runs of xetexdir/xetex-NAME.test: (job, args) pairs; the
    last one's log is compared, with its first line dropped (`sed 1d`)."""
    if name == "bug73":
        return [("bug73", ["-ini", "bug73"]),
                ("bug73", ["-fmt=bug73", "bug73"])]
    if name == "ctrlsym":
        return [("xe-basic", ["-ini", "-etex", "xe-basic"]),
                ("xe-ctrlsym", ["-etex", "-fmt=xe-basic", "xe-ctrlsym"])]
    if name == "filedump":
        return [("filedump", ["-ini", "filedump"])]
    raise KeyError(name)


def stage_xetex_test(name, tmp):
    if name == "ctrlsym":
        shutil.copy(os.path.join(XETEX_TESTS, "ctrlsym.tex"),
                    os.path.join(tmp, "xe-ctrlsym.tex"))
        shutil.copy(os.path.join(XETEX_TESTS, "dump-basic.tex"),
                    os.path.join(tmp, "xe-basic.tex"))
    else:
        shutil.copy(os.path.join(XETEX_TESTS, name + ".tex"), tmp)
    shutil.copy(os.path.join(XETEX_TESTS, "basic.tex"), tmp)


def run_xetex_test(name, ref, cand, timeout, keep):
    msgs = []
    ok = True
    logs = {}
    dirs = {}
    for label, binary in (("reference", ref), ("candidate", cand)):
        tmp = tempfile.mkdtemp(prefix="xets-")
        dirs[label] = tmp
        stage_xetex_test(name, tmp)
        env = pinned_env()
        env.update(TEXINPUTS=".:", TEXFORMATS=".")
        last = None
        for job, args in xetex_test_steps(name):
            argv0 = link_engine(binary, tmp)
            try:
                rc, out = LS._run_isolated([argv0, "-no-pdf"] + args, cwd=tmp,
                                           env=env, timeout=timeout)
            except subprocess.TimeoutExpired:
                rc = "timeout"
            last = (job, rc)
        job, rc = last
        try:
            with open(os.path.join(tmp, job + ".log"), encoding="utf-8",
                      errors="replace", newline="") as fh:
                text = fh.read()
        except OSError:
            text = ""
        logs[label] = (rc, text.split("\n", 1)[1] if "\n" in text else "")
    expected = open(os.path.join(XETEX_TESTS, name + ".log"),
                    encoding="utf-8").read()
    if logs["candidate"][0] != logs["reference"][0]:
        ok = False
        msgs.append("returncode reference=%s candidate=%s"
                    % (logs["reference"][0], logs["candidate"][0]))
    if logs["candidate"][1] != logs["reference"][1]:
        ok = False
        msgs.append("log differs from the reference's")
    if logs["candidate"][1] != expected:
        ok = False
        msgs.append("log differs from third_party/xetex/tests/%s.log" % name)
    if not keep:
        for d in dirs.values():
            shutil.rmtree(d, ignore_errors=True)
    else:
        msgs.append("dirs: %s" % dirs)
    return "xetex-test:" + name, ok, msgs, None


# ---------------------------------------------------------------------------
# Selection
# ---------------------------------------------------------------------------

def primitives(web):
    text = open(web, encoding="latin-1").read()
    return set(re.findall(r'primitive\("([A-Za-z]+)"', text))


def pdftex_only_primitives():
    p = primitives(os.path.join(ROOT, "third_party", "pdftex", "pdftex.web"))
    x = primitives(os.path.join(ROOT, "third_party", "xetex", "xetex.web"))
    return p - x


def select(ref, jobs, timeout):
    """Rewrite suite.txt: the lockstep cases that use no pdfTeX-only
    primitive, whose reference xetex run ends as the case expects (exit 0,
    or nonzero for a `% lockstep: no-halt` case) without an undefined
    control sequence."""
    only = pdftex_only_primitives()
    names = sorted(fn[:-4] for fn in os.listdir(LOCKSTEP_CASES)
                   if fn.endswith(".tex"))
    cand = []
    skipped_prim = 0
    for n in names:
        src = open(os.path.join(LOCKSTEP_CASES, n + ".tex"),
                   encoding="utf-8", errors="replace").read()
        used = set(re.findall(r"\\([A-Za-z]+)", src))
        if used & only:
            skipped_prim += 1
            continue
        cand.append(n)

    def check(n):
        src = os.path.join(LOCKSTEP_CASES, n + ".tex")
        no_halt = "no-halt" in LS.case_options(src)
        args = [a for a in ENGINE_ARGS
                if not (no_halt and a == "-halt-on-error")]
        tmp = tempfile.mkdtemp(prefix="xesel-")
        try:
            stage_case(src, n, tmp)
            r = run_one(ref, tmp, args + [n + ".tex"], n, timeout)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
        if "error" in r:
            return n, False
        ends_ok = (r["returncode"] != 0) if no_halt else (r["returncode"] == 0)
        undefined = "! Undefined control sequence." in r["log"]
        return n, ends_ok and not undefined

    kept = []
    with concurrent.futures.ThreadPoolExecutor(jobs) as ex:
        for n, good in ex.map(check, cand):
            if good:
                kept.append(n)
    with open(SUITE, "w") as fh:
        fh.write("# The tools/lockstep cases of the XeTeX lockstep suite, written\n"
                 "# by `run.py --select` (see its docstring): %d of %d cases;\n"
                 "# %d use a pdfTeX-only primitive, %d end differently under\n"
                 "# TeX Live's xetex.\n"
                 % (len(kept), len(names), skipped_prim,
                    len(cand) - len(kept)))
        for n in kept:
            fh.write(n + "\n")
    print("selected %d of %d lockstep cases (%d use pdfTeX-only primitives, "
          "%d end differently under xetex)"
          % (len(kept), len(names), skipped_prim, len(cand) - len(kept)))


def suite_cases():
    names = []
    if os.path.exists(SUITE):
        for line in open(SUITE):
            line = line.split("#")[0].strip()
            if line:
                names.append(line)
    if os.path.isdir(OWN_CASES):
        names += sorted(fn[:-4] for fn in os.listdir(OWN_CASES)
                        if fn.endswith(".tex"))
    return names


def check_reference(ref):
    try:
        out = subprocess.run([ref, "-version"], capture_output=True,
                             text=True, timeout=60).stdout
    except (OSError, subprocess.TimeoutExpired):
        return False
    return out.splitlines()[:1] == [PINNED_REFERENCE]


def main(argv=None):
    ap = argparse.ArgumentParser(description="XeTeX lockstep harness")
    ap.add_argument("--engine", help="candidate engine binary")
    ap.add_argument("--reference", default="xetex")
    ap.add_argument("--cases", nargs="*", default=[])
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--timeout", type=float, default=300)
    ap.add_argument("--keep", action="store_true")
    ap.add_argument("--select", action="store_true")
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--no-xetex-tests", action="store_true")
    ap.add_argument("--test-normalise", action="store_true",
                    help="only test the XDV normalisation (no TeX needed)")
    a = ap.parse_args(argv)
    # The normalisation is checked on every run, before it is relied on.
    _self_test_normalise_xdv()
    if a.test_normalise:
        print("XDV normalisation: ok")
        return 0
    if not check_reference(a.reference):
        print("reference %s is not %s" % (a.reference, PINNED_REFERENCE))
        return 2
    if a.select:
        select(a.reference, a.jobs, a.timeout)
        return 0
    cand = a.reference if a.self_test else a.engine
    if not cand:
        ap.error("--engine is required (or --self-test)")
    names = suite_cases()
    if a.cases:
        names = [n for n in names
                 if any(fnmatch.fnmatch(n, p) for p in a.cases)]
    tests = [] if (a.no_xetex_tests or a.cases) else ["bug73", "ctrlsym",
                                                       "filedump"]
    results = []
    with concurrent.futures.ThreadPoolExecutor(a.jobs) as ex:
        futs = [ex.submit(run_case, n, a.reference, cand, a.timeout, a.keep)
                for n in names]
        futs += [ex.submit(run_xetex_test, t, a.reference, cand, a.timeout,
                           a.keep) for t in tests]
        for f in futs:
            results.append(f.result())
    n_ok = 0
    xdv = {"identical": 0, "differs": 0}
    for name, ok, msgs, xs in results:
        if xs:
            xdv[xs] += 1
        if ok:
            n_ok += 1
            continue
        print("FAIL %s" % name)
        for m in msgs:
            print("  " + m)
    total = len(results)
    print("%d cases, %d equal, %d differ" % (total, n_ok, total - n_ok))
    print("XDV: %d of %d identical after normalisation"
          % (xdv["identical"], xdv["identical"] + xdv["differs"]))
    return 0 if n_ok == total else 1


if __name__ == "__main__":
    sys.exit(main())
