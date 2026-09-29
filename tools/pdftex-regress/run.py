#!/usr/bin/env python3
"""T0 pdfTeX regression harness: pdfTeX's own test scripts vs any engine binary.

Runs the TESTS from texk/web2c/pdftexdir/am/pdftex.am (+ ttf2afm, pdftosrc)
against ``--engine <bin>``. Each test runs in a fresh temp dir with the
environment its upstream ``.test`` script sets (TEXINPUTS/TEXFORMATS as
needed, LC_ALL=C), stdin from /dev/null, a per-test timeout with
process-group kill, and the comparison (log / afm / xref) after upstream's
own normalisation.

Upstream test files are NEVER copied into this repo: fetch.sh materialises
the pinned texlive-source checkout into .cache/ and this script reads the
inputs from there. Stdlib only.
"""

import argparse
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_CACHE = os.path.join(HERE, ".cache", "texlive")
EXPECTED_FAILURES = os.path.join(HERE, "EXPECTED-FAILURES.txt")
REF_VERSION = "1.40.29"

PASS, FAIL, SKIP = "PASS", "FAIL", "SKIP"


# ---------------------------------------------------------------------------
# process + file helpers


def run_cmd(argv, cwd, env, timeout):
    """Run argv; stdin is /dev/null. Returns (rc|None, stdout, stderr,
    timed_out, seconds). A timeout SIGKILLs the whole process group."""
    t0 = time.monotonic()
    proc = subprocess.Popen(argv, stdin=subprocess.DEVNULL,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                            env=env, cwd=cwd, start_new_session=True)
    try:
        out, err = proc.communicate(timeout=timeout)
        return proc.returncode, out, err, False, time.monotonic() - t0
    except subprocess.TimeoutExpired:
        try:
            os.killpg(proc.pid, signal.SIGKILL)
        except (ProcessLookupError, PermissionError):
            pass
        out, err = proc.communicate()
        return None, out or b"", err or b"", True, time.monotonic() - t0


def base_env(overrides):
    env = dict(os.environ)
    env["LC_ALL"] = "C"
    env["LANGUAGE"] = "C"
    env.update(overrides)
    return env


def copy_inputs(files, dest):
    for src in files:
        shutil.copy(src, os.path.join(dest, os.path.basename(src)))


def read_bytes(path):
    with open(path, "rb") as f:
        return f.read()


def first_diff_line(a, b):
    for i, (x, y) in enumerate(zip(a.splitlines(), b.splitlines())):
        if x != y:
            return i + 1, x[:100], y[:100]
    if len(a.splitlines()) != len(b.splitlines()):
        return min(len(a.splitlines()), len(b.splitlines())) + 1, b"<eof>", b"<eof>"
    return None


# ---------------------------------------------------------------------------
# pure checks (unit-tested in test_run.py)


def version_ok(first_line):
    return REF_VERSION in first_line


def expanded_normalise(log):
    """Mirror `sed -n -e 's/[\\\\]pdf/\\\\/g' -e '/START/,/END/p'` (bytes)."""
    out, inside = [], False
    for line in log.splitlines(keepends=True):
        if not inside and b"START" not in line:
            continue
        inside = True
        out.append(line.replace(b"\\pdf", b"\\"))
        if b"END" in line:
            inside = False
    return b"".join(out)


WPROB_RE = re.compile(
    rb"^\./pwprob\.tex:12: Could not open file NoSuchFile\.eps\.$", re.M)


def wprob_ok(log):
    return WPROB_RE.search(log) is not None


def parse_expected_failures(path):
    expected = {}
    if not os.path.exists(path):
        return expected
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            name, _, reason = line.partition(":")
            expected[name.strip()] = reason.strip()
    return expected


def gate_exit(failed, expected):
    unexpected = [n for n in failed if n not in expected]
    return (1 if unexpected else 0), unexpected


# ---------------------------------------------------------------------------
# test context + individual tests (each mirrors one upstream .test script)


class Ctx:
    def __init__(self, engine, timeout, web2c, work, ttf2afm, pdftosrc):
        self.engine = engine
        self.timeout = timeout
        self.web2c = web2c
        self.ptests = os.path.join(web2c, "pdftexdir", "tests")
        self.pdir = os.path.join(web2c, "pdftexdir")
        self.wtests = os.path.join(web2c, "tests")
        self.work = work
        self.ttf2afm = ttf2afm
        self.pdftosrc = pdftosrc

    def texinputs(self, *dirs):
        return base_env({"TEXINPUTS": ":".join(dirs) + ":"})


def t_pdftex(c):
    """pdftexdir/pdftex.test: --version and --help must exit 0."""
    for args in (["--version"], ["--help"]):
        rc, _, _, timed_out, _ = run_cmd([c.engine] + args, c.work,
                                         base_env({}), c.timeout)
        if timed_out:
            return FAIL, "%s timed out" % " ".join(args)
        if rc != 0:
            return FAIL, "%s exit %s" % (" ".join(args), rc)
    return PASS, "--version/--help exit 0"


def t_expanded(c):
    """pdftexdir/expanded.test: normalise START..END block, diff vs .txt."""
    copy_inputs([os.path.join(c.ptests, "expanded.tex")], c.work)
    rc, _, _, timed_out, _ = run_cmd(
        [c.engine, "-ini", "-etex", "--interaction", "batchmode",
         "expanded.tex"], c.work, c.texinputs(c.ptests), c.timeout)
    if timed_out:
        return FAIL, "engine timed out"
    # Upstream tolerates the exit code (no pages of output -> exit 1).
    try:
        log = read_bytes(os.path.join(c.work, "expanded.log"))
    except OSError:
        return FAIL, "no expanded.log written (exit %s)" % rc
    want = read_bytes(os.path.join(c.ptests, "expanded.txt"))
    got = expanded_normalise(log)
    if got == want:
        return PASS, "START..END block matches expanded.txt"
    d = first_diff_line(got, want)
    return FAIL, "diff at line %s: got %r want %r" % d


def t_cnfline(c):
    """pdftexdir/tests/cnfline.test: --cnf-line honored, one-line message."""
    copy_inputs([os.path.join(c.ptests, "cnfline.tex")], c.work)
    rc, _, _, timed_out, _ = run_cmd(
        [c.engine, "-ini", "--interaction=nonstopmode",
         "--cnf-line=max_print_line=500", "cnfline.tex"],
        c.work, c.texinputs(c.ptests), c.timeout)
    if timed_out:
        return FAIL, "engine timed out"
    if rc != 0:
        return FAIL, "exit %s" % rc
    log = read_bytes(os.path.join(c.work, "cnfline.log"))
    for line in log.splitlines():
        if b"those hyphens are" in line:
            return PASS, "long message unbroken on one line"
    return FAIL, "'those hyphens are' not found on one log line"


def t_pdfimage(c):
    """pdftexdir/pdfimage.test: build fmt, then run with -fmt (exit 0/0)."""
    copy_inputs([os.path.join(c.ptests, "pdfimage.tex"),
                 os.path.join(c.wtests, "basic.tex"),
                 os.path.join(c.wtests, "1-4.jpg"),
                 os.path.join(c.wtests, "B.pdf"),
                 os.path.join(c.wtests, "lily-ledger-broken.png")], c.work)
    env = base_env({"TEXINPUTS": c.ptests + ":" + c.wtests + ":.",
                    "TEXFORMATS": c.work})
    rc, _, _, timed_out, _ = run_cmd(
        [c.engine, "-ini", "-interaction=batchmode", "pdfimage"],
        c.work, env, c.timeout)
    if timed_out or rc != 0 or not os.path.exists(
            os.path.join(c.work, "pdfimage.fmt")):
        return FAIL, "fmt build exit %s timeout=%s" % (rc, timed_out)
    rc, _, _, timed_out, _ = run_cmd(
        [c.engine, "-fmt=pdfimage", "-interaction=batchmode", "pdfimage"],
        c.work, env, c.timeout)
    if timed_out or rc != 0:
        return FAIL, "fmt run exit %s timeout=%s" % (rc, timed_out)
    return PASS, "fmt built and ran, exit 0/0"


def t_partoken(c):
    """pdftexdir/tests/partoken.test: ok run exits 0, xfail exits nonzero."""
    copy_inputs([os.path.join(c.wtests, "partoken-ok.tex"),
                 os.path.join(c.wtests, "partoken-xfail.tex")], c.work)
    env = base_env({"TEXINPUTS": c.wtests + ":",
                    "TEXMFDOTDIR": c.wtests})
    rc, _, _, timed_out, _ = run_cmd(
        [c.engine, "-ini", "--interaction=nonstopmode", "partoken-ok.tex"],
        c.work, env, c.timeout)
    if timed_out or rc != 0:
        return FAIL, "partoken-ok exit %s timeout=%s" % (rc, timed_out)
    rc, _, _, timed_out, _ = run_cmd(
        [c.engine, "-ini", "--interaction=nonstopmode", "partoken-xfail.tex"],
        c.work, env, c.timeout)
    if timed_out:
        return FAIL, "partoken-xfail timed out (must fail fast)"
    if rc == 0:
        return FAIL, "partoken-xfail exited 0, must fail"
    return PASS, "ok exits 0, xfail exits %s" % rc


def t_wprob(c):
    """pdftexdir/wprob.test: missing-image run must fail with exact message."""
    shutil.copy(os.path.join(c.wtests, "wprob.tex"),
                os.path.join(c.work, "pwprob.tex"))
    rc, _, _, timed_out, _ = run_cmd(
        [c.engine, "--ini", "--etex", "--file-line-error",
         "--interaction=nonstopmode", "pwprob.tex"],
        c.work, base_env({}), c.timeout)
    if timed_out:
        return FAIL, "timed out (must fail fast)"
    if rc == 0:
        return FAIL, "exited 0, must fail"
    try:
        log = read_bytes(os.path.join(c.work, "pwprob.log"))
    except OSError:
        return FAIL, "no pwprob.log written"
    if not wprob_ok(log):
        return FAIL, "expected './pwprob.tex:12: Could not open ...' line"
    return PASS, "fails with exact missing-file message"


def t_ttf2afm(c):
    """pdftexdir/ttf2afm.test: afm output matches after dropping dateline."""
    if not c.ttf2afm:
        return SKIP, "no ttf2afm binary (sibling of --engine or --ttf2afm)"
    bad = []
    for stem in ("postV3", "postV7"):
        rc, out, _, timed_out, _ = run_cmd(
            [c.ttf2afm, os.path.join(c.ptests, stem + ".ttf")],
            c.work, base_env({}), c.timeout)
        if timed_out or rc != 0:
            bad.append("%s exit %s timeout=%s" % (stem, rc, timed_out))
            continue
        got = b"\n".join(l for l in out.splitlines()
                         if b"Converted at" not in l) + b"\n"
        if got != read_bytes(os.path.join(c.ptests, stem + ".afm")):
            bad.append("%s afm differs" % stem)
    if bad:
        return FAIL, "; ".join(bad)
    return PASS, "postV3/postV7 afm match"


def t_pdftosrc(c):
    """pdftexdir/pdftosrc.test: xref output matches (CR-normalised)."""
    if not c.pdftosrc:
        return SKIP, "no pdftosrc binary (sibling of --engine or --pdftosrc)"
    bad = []
    for stem in ("test-13", "test-15"):
        shutil.copy(os.path.join(c.ptests, stem + ".pdf"), c.work)
        rc, _, _, timed_out, _ = run_cmd(
            [c.pdftosrc, stem + ".pdf", "-1"], c.work,
            base_env({}), c.timeout)
        if timed_out or rc != 0:
            bad.append("%s exit %s timeout=%s" % (stem, rc, timed_out))
            continue
        try:
            got = read_bytes(os.path.join(c.work, stem + ".xref"))
        except OSError as e:
            bad.append("%s no xref: %s" % (stem, e))
            continue
        want = read_bytes(os.path.join(c.ptests, stem + ".xref"))
        if got.replace(b"\r", b"") != want.replace(b"\r", b""):
            bad.append("%s xref differs" % stem)
    if bad:
        return FAIL, "; ".join(bad)
    return PASS, "test-13/test-15 xref match"


WCFNAME_LOCALES = ["C.UTF-8", "C.utf8", "en_US.UTF-8", "en_US.utf8",
                   "ja_JP.UTF-8", "ja_JP.utf8"]
WCFNAME_DOCS = ["fn-utf8", "fn£¥µÆÇñß-utf8", "fnさざ波-utf8",
                "fnΔДदダ打다𝕯🎉-utf8"]
WCFNAME_VIR = "fn±×÷§¶-utf8.tex"


def split_locales(available):
    """Partition WCFNAME_LOCALES into (present, missing) given a
    `locale -a` set. Pure; unit-tested."""
    present = [loc for loc in WCFNAME_LOCALES if loc in available]
    missing = [loc for loc in WCFNAME_LOCALES if loc not in available]
    return present, missing


def wcfname_env(loc):
    """Mirror upstream: LC_ALL/LANGUAGE=<loc>, TEXINPUTS over a relative
    testdir (relative so ini-\\openout stays writable under openout_any=p)."""
    env = base_env({"LC_ALL": loc, "LANGUAGE": loc,
                    "TEXINPUTS": "pdftests:.:"})
    return env


def t_wcfname(c):
    """pdftexdir/wcfname.test: non-ASCII filenames via kpsewhich + ini runs.

    Ports the upstream steps: perl fn-generate.perl writes the fn-*.tex
    inputs, then for each LC_ALL locale from the upstream matrix that
    `locale -a` reports, each doc is kpsewhich-probed and ini-run with
    -jobname (exit 0, job .txt/.log/.fls produced, -tmp found). Locales
    absent from `locale -a` are reported per-locale, not run."""
    kpsewhich = shutil.which("kpsewhich")
    if not kpsewhich:
        return SKIP, "no kpsewhich on PATH (needed for file-search probes)"
    if not shutil.which("perl"):
        return SKIP, "no perl on PATH (needed for fn-generate.perl)"
    if not shutil.which("locale"):
        return SKIP, "no `locale` program (cannot enumerate LC_ALL matrix)"
    rc, out, _, timed_out, _ = run_cmd(["locale", "-a"], c.work,
                                       base_env({}), c.timeout)
    if timed_out or rc != 0:
        return SKIP, "`locale -a` failed (exit %s timeout=%s)" % (rc,
                                                                  timed_out)
    present, missing = split_locales(
        {l.strip() for l in out.decode("utf-8", "replace").splitlines()
         if l.strip()})
    if not present:
        return SKIP, ("none of the upstream UTF-8 locales installed "
                      "(locale -a has no %s)" % ",".join(WCFNAME_LOCALES))
    testdir = os.path.join(c.work, "pdftests")
    os.mkdir(testdir)
    gen = os.path.join(c.web2c, "tests", "fn-generate.perl")
    # Relative testdir (cwd=c.work): generated \openout paths stay relative.
    rc, _, err, timed_out, _ = run_cmd(
        ["perl", "-s", gen, "-randgen=pdfuniformdeviate", "pdftests"],
        c.work, wcfname_env(present[0]), c.timeout)
    if timed_out or (rc not in (0, 239)):
        # Upstream tolerates 239 (an Encode miss) but exits 77 otherwise.
        return SKIP, "fn-generate.perl exit %s timeout=%s: %s" % (
            rc, timed_out, err.decode("utf-8", "replace")[-200:])
    for f in os.listdir(testdir):
        if f.endswith("-euc.tex") or f.endswith("-sjis.tex"):
            os.unlink(os.path.join(testdir, f))
    bad = []
    for loc in present:
        env = wcfname_env(loc)
        for probe in ("-var-value=TEXMFCNF",
                      "-progname=pdftex -var-value=TEXINPUTS",
                      "-progname=pdftex -var-value=command_line_encoding"):
            run_cmd([kpsewhich] + probe.split(), c.work, env, c.timeout)
        for doc in WCFNAME_DOCS:
            tag = "%s:%s" % (loc, doc)
            for probe in (doc + ".tex", WCFNAME_VIR):
                rc, _, _, timed_out, _ = run_cmd(
                    [kpsewhich, "-progname=pdftex", probe],
                    c.work, env, c.timeout)
                if timed_out or rc != 0:
                    bad.append(tag + " kpse-miss " + probe)
            job = doc + "-pdf"
            for f in (os.path.join(testdir, doc + "-tmp.tex"),
                      os.path.join(testdir, job + ".txt"),
                      os.path.join(testdir, job + ".log"),
                      os.path.join(testdir, job + ".fls"),
                      os.path.join(c.work, job + ".txt"),
                      os.path.join(c.work, job + ".log"),
                      os.path.join(c.work, job + ".fls")):
                if os.path.exists(f):
                    os.unlink(f)
            rc, term, _, timed_out, _ = run_cmd(
                [c.engine, "-ini", "-interaction", "nonstopmode",
                 "-jobname=" + job, "--shell-escape", "-etex", "--recorder",
                 doc + ".tex"], c.work, env, c.timeout)
            with open(os.path.join(testdir, doc + "-term.log"), "wb") as f:
                f.write(term)
            if timed_out or rc != 0:
                bad.append(tag + " tex exit %s timeout=%s" % (rc, timed_out))
                continue
            rc, _, _, timed_out, _ = run_cmd(
                [kpsewhich, "-progname=pdftex", doc + "-tmp.tex"],
                c.work, env, c.timeout)
            if timed_out or rc != 0:
                bad.append(tag + " kpse-miss tmp")
            missing_out = [f for f in (job + ".txt", job + ".log",
                                       job + ".fls")
                           if not os.path.exists(os.path.join(c.work, f))]
            if missing_out:
                bad.append(tag + " missing " + ",".join(missing_out))
                continue
            for f in (job + ".txt", job + ".log", job + ".fls"):
                os.rename(os.path.join(c.work, f),
                          os.path.join(testdir, f))
    if bad:
        return FAIL, "; ".join(bad)
    detail = "pass in %s (%d docs each)" % (",".join(present),
                                            len(WCFNAME_DOCS))
    if missing:
        detail += "; no such locale: %s" % ",".join(missing)
    return PASS, detail


TESTS = [
    ("pdftex", "pdftexdir/pdftex.test", "version/help smoke", t_pdftex),
    ("expanded", "pdftexdir/expanded.test", "\\expanded log compare", t_expanded),
    ("cnfline", "pdftexdir/tests/cnfline.test", "--cnf-line long line", t_cnfline),
    ("pdfimage", "pdftexdir/pdfimage.test", "fmt build + image run", t_pdfimage),
    ("partoken", "pdftexdir/tests/partoken.test", "partoken ok/xfail", t_partoken),
    ("wprob", "pdftexdir/wprob.test", "missing-file error text", t_wprob),
    ("ttf2afm", "pdftexdir/ttf2afm.test", "ttf->afm compare", t_ttf2afm),
    ("pdftosrc", "pdftexdir/pdftosrc.test", "pdf->xref compare", t_pdftosrc),
    ("wcfname", "pdftexdir/wcfname.test", "unicode filenames", t_wcfname),
]


# ---------------------------------------------------------------------------
# driver


def parse_args(argv):
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--engine", required=True, help="engine binary under test")
    ap.add_argument("--tests", default="",
                    help="comma-separated subset of test names (default: all)")
    ap.add_argument("--list", action="store_true",
                    help="list discovered tests and exit")
    ap.add_argument("--timeout", type=float, default=300,
                    help="seconds per test/engine call (default 300)")
    ap.add_argument("--allow-any-engine", action="store_true",
                    help="skip the pdfTeX 1.40.29 version gate")
    ap.add_argument("--cache", default=DEFAULT_CACHE,
                    help="pinned texlive-source checkout (see fetch.sh)")
    ap.add_argument("--ttf2afm", default="",
                    help="ttf2afm binary (default: sibling of --engine)")
    ap.add_argument("--pdftosrc", default="",
                    help="pdftosrc binary (default: sibling of --engine)")
    return ap.parse_args(argv)


def sibling(engine, name, explicit):
    if explicit:
        return explicit if os.path.isfile(explicit) else ""
    cand = os.path.join(os.path.dirname(os.path.abspath(engine)), name)
    return cand if os.path.isfile(cand) else ""


def main(argv=None):
    args = parse_args(argv or sys.argv[1:])
    web2c = os.path.join(args.cache, "texk", "web2c")
    if args.list:
        for name, script, desc, _ in TESTS:
            print("%-9s %s  (%s)" % (name, script, desc))
        return 0
    if not os.path.isdir(web2c):
        print("error: pinned checkout missing at %s" % web2c, file=sys.stderr)
        print("run: sh tools/pdftex-regress/fetch.sh  (or pass --cache DIR)",
              file=sys.stderr)
        return 2
    if not (os.path.isfile(args.engine) and os.access(args.engine, os.X_OK)):
        print("error: --engine not executable: %s" % args.engine,
              file=sys.stderr)
        return 2
    names = [t.strip() for t in args.tests.split(",") if t.strip()]
    selected = TESTS if not names else [t for t in TESTS if t[0] in names]
    if names and len(selected) != len(names):
        print("error: unknown test(s): %s" %
              sorted(set(names) - {t[0] for t in TESTS}), file=sys.stderr)
        return 2

    with tempfile.TemporaryDirectory(prefix="pdftex-regress-") as scratch:
        if not args.allow_any_engine:
            rc, out, _, timed_out, _ = run_cmd(
                [args.engine, "--version"], scratch, base_env({}),
                args.timeout)
            line = out.decode("utf-8", "replace").splitlines()
            line = line[0] if line else ""
            print("engine: %s" % line)
            if timed_out or rc != 0 or not version_ok(line):
                print("error: engine must report pdfTeX %s "
                      "(--allow-any-engine overrides)" % REF_VERSION,
                      file=sys.stderr)
                return 2
        else:
            print("engine: (version gate skipped)")

        results = []
        for name, script, desc, fn in selected:
            work = tempfile.mkdtemp(prefix=name + "-", dir=scratch)
            ctx = Ctx(args.engine, args.timeout, web2c, work,
                      sibling(args.engine, "ttf2afm", args.ttf2afm),
                      sibling(args.engine, "pdftosrc", args.pdftosrc))
            try:
                status, detail = fn(ctx)
            except Exception as e:  # noqa: BLE001 - a crashing test is FAIL
                status, detail = FAIL, "harness exception: %r" % e
            results.append((name, status, detail))
            print("%-9s %s  (%s)" % (name, status, detail))

    npass = sum(1 for _, s, _ in results if s == PASS)
    nfail = sum(1 for _, s, _ in results if s == FAIL)
    nskip = sum(1 for _, s, _ in results if s == SKIP)
    print("PASS %d / FAIL %d / SKIP %d" % (npass, nfail, nskip))
    failed = [n for n, s, _ in results if s == FAIL]
    if failed:
        print("failing: %s" % " ".join(failed))
    expected = parse_expected_failures(EXPECTED_FAILURES)
    code, unexpected = gate_exit(failed, expected)
    if failed and not unexpected:
        print("all failures are listed in EXPECTED-FAILURES.txt")
    for n in sorted(set(expected) & {n for n, s, _ in results if s == PASS}):
        print("unexpected-pass: %s (listed in EXPECTED-FAILURES.txt)" % n)
    return code


if __name__ == "__main__":
    sys.exit(main())
