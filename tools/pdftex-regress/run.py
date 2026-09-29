#!/usr/bin/env python3
"""T0 pdfTeX regression harness: pdfTeX's own test scripts vs any engine binary.

Runs the TESTS from texk/web2c/pdftexdir/am/pdftex.am (+ ttf2afm, pdftosrc)
against ``--engine <bin>``. Each test runs in a fresh temp dir with the
environment its upstream ``.test`` script sets (TEXINPUTS/TEXFORMATS as
needed, LC_ALL=C), stdin from /dev/null, a per-test timeout with
process-group kill inside a whole-run --budget, and the comparison
(log / afm / xref) after upstream's own normalisation.

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

# Grace period (seconds) for draining pipes after a timeout kill. After
# SIGKILLing the process group, leftover output flushes immediately --
# unless a detached survivor holds the pipes open, in which case we stop
# waiting here instead of deadlocking (see run_cmd).
DRAIN_TIMEOUT = 5


# ---------------------------------------------------------------------------
# process + file helpers


def run_cmd(argv, cwd, env, timeout):
    """Run argv; stdin is /dev/null. Returns (rc|None, stdout, stderr,
    timed_out, seconds). A timeout SIGKILLs the whole process group, then
    drains the pipes with a bounded wait: a detached survivor holding
    stdout/stderr open can delay us by DRAIN_TIMEOUT at most, never hang
    us (we close our pipe ends and reap our already-dead child)."""
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
        try:
            out, err = proc.communicate(timeout=DRAIN_TIMEOUT)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except (ProcessLookupError, PermissionError):
                pass
            try:
                proc.kill()
            except (ProcessLookupError, PermissionError, OSError):
                pass
            for fh in (proc.stdout, proc.stderr):
                try:
                    if fh is not None:
                        fh.close()
                except (OSError, ValueError):
                    pass
            try:
                proc.wait(timeout=DRAIN_TIMEOUT)
            except (subprocess.TimeoutExpired, OSError):
                pass
            out, err = b"", b""
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


def nonempty(path):
    """True when path exists and holds at least one byte. A stub that
    only touches empty artifacts must not satisfy an existence check."""
    try:
        return os.path.getsize(path) > 0
    except OSError:
        return False


def first_diff_line(a, b):
    for i, (x, y) in enumerate(zip(a.splitlines(), b.splitlines())):
        if x != y:
            return i + 1, x[:100], y[:100]
    if len(a.splitlines()) != len(b.splitlines()):
        return min(len(a.splitlines()), len(b.splitlines())) + 1, b"<eof>", b"<eof>"
    return None


# ---------------------------------------------------------------------------
# pure checks (unit-tested in test_run.py)


VERSION_RE = re.compile(r"(?<![\d.])" + re.escape(REF_VERSION) +
                          r"(?![\d.])")


def version_ok(first_line):
    return "pdfTeX" in first_line and VERSION_RE.search(first_line) is not None


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
    rb"^\./pwprob\.tex:12: Could not open file NoSuchFile\.eps\.\r?$", re.M)


def wprob_ok(log):
    return WPROB_RE.search(log) is not None


WCFNAME_CARET_RE = re.compile(rb"\^\^([0-9a-f]{2})")


def caret_encode(raw):
    """Encode bytes the way pdfTeX terminal/\\write output does: printable
    ASCII passes through, every other byte becomes a lowercase ^^XX
    escape. Pure; unit-tested."""
    out = bytearray()
    for b in raw:
        if 0x20 <= b <= 0x7E:
            out.append(b)
        else:
            out += ("^^%02x" % b).encode("ascii")
    return bytes(out)


def term_has_job(term, job):
    """True when the terminal log carries this document's JOB[<job>]
    marker. Engine terminal lines wrap at 79 columns with a bare
    newline, so lines are joined before searching; the marker uses the
    ^^XX-escaped job-name spelling. Pure; unit-tested."""
    flat = term.replace(b"\r", b"").replace(b"\n", b"")
    return b"JOB[" + caret_encode(job.encode("utf-8")) + b"]" in flat


def wcfname_job_ok(got, want):
    """Compare a produced job.txt against tests/fn-utf8.txt. pdfTeX's
    \\write emits non-ASCII bytes as printable ^^XX escapes, which is
    why upstream's plain `diff` is commented out as not working; decode
    those escapes first, then byte-compare. Pure; unit-tested."""
    def dec(m):
        return bytes((int(m.group(1), 16),))
    return WCFNAME_CARET_RE.sub(dec, got) == want


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


def gate_exit(failed, expected, passed=(), allow_stale=False):
    """Gate verdict. unexpected = failed but unlisted; stale = listed in
    EXPECTED-FAILURES.txt but passed in this run (the list rotted).
    Returns (exit_code, unexpected, stale). A stale entry fails the gate
    unless allow_stale is given. Only tests run here count: entries for
    tests outside --tests are never stale."""
    unexpected = [n for n in failed if n not in expected]
    stale = sorted(set(expected) & set(passed))
    code = 1 if unexpected else 0
    if stale and not allow_stale:
        code = 1
    return code, unexpected, stale


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
    # Isolation: only the work dir is searchable. The expected
    # expanded.txt stays in the checkout for the runner's post-run read;
    # the engine must not reach it via $TEXINPUTS (trailing ':' keeps
    # the engine's own TeX Live tree defaults).
    rc, _, _, timed_out, _ = run_cmd(
        [c.engine, "-ini", "-etex", "--interaction", "batchmode",
         "expanded.tex"], c.work, c.texinputs(c.work), c.timeout)
    if timed_out:
        return FAIL, "engine timed out"
    # Upstream expanded.test never checks the engine exit status; the
    # gate hardens it: the reference engine exits 1 (`No pages of
    # output.' -- INITEX run, no \dump, nothing shipped), so only 0/1
    # are tolerated. A crash (None/negative) or any other code FAILs
    # even when the log matches.
    if rc is None or rc < 0:
        return FAIL, "engine crashed (rc %s), want exit 0 or 1" % (rc,)
    if rc not in (0, 1):
        return FAIL, "engine exit %s (want 0 or 1)" % (rc,)
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
        c.work, c.texinputs(c.work), c.timeout)
    if timed_out:
        return FAIL, "engine timed out"
    if rc is None or rc < 0:
        return FAIL, "engine crashed (rc %s), want exit 0" % (rc,)
    if rc != 0:
        return FAIL, "exit %s" % rc
    try:
        log = read_bytes(os.path.join(c.work, "cnfline.log"))
    except OSError:
        return FAIL, "no cnfline.log written (exit %s)" % rc
    if not log:
        return FAIL, "cnfline.log empty (exit %s)" % rc
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
    # Isolation: inputs are copied above; the checkout itself is not on
    # the search path (trailing ':' keeps the engine's TeX Live defaults).
    env = base_env({"TEXINPUTS": c.work + ":",
                    "TEXFORMATS": c.work})
    rc, _, _, timed_out, _ = run_cmd(
        [c.engine, "-ini", "-interaction=batchmode", "pdfimage"],
        c.work, env, c.timeout)
    if timed_out:
        return FAIL, "fmt build exit %s timeout=%s" % (rc, timed_out)
    if rc is None or rc < 0:
        return FAIL, "fmt build crashed (rc %s), want exit 0" % (rc,)
    if rc != 0 or not nonempty(os.path.join(c.work, "pdfimage.fmt")):
        return FAIL, "fmt build exit %s (want 0 with non-empty fmt)" % (rc,)
    rc, _, _, timed_out, _ = run_cmd(
        [c.engine, "-fmt=pdfimage", "-interaction=batchmode", "pdfimage"],
        c.work, env, c.timeout)
    if timed_out:
        return FAIL, "fmt run exit %s timeout=%s" % (rc, timed_out)
    if rc is None or rc < 0:
        return FAIL, "fmt run crashed (rc %s), want exit 0" % (rc,)
    if rc != 0:
        return FAIL, "fmt run exit %s timeout=%s" % (rc, timed_out)
    # Upstream checks only the two exit codes; a stub that touches an
    # empty fmt and exits 0 twice produces no PDF/log, so require the
    # run's own artifacts to exist and be non-empty as well.
    for artifact in ("pdfimage.pdf", "pdfimage.log"):
        if not nonempty(os.path.join(c.work, artifact)):
            return FAIL, "fmt run produced no non-empty %s" % artifact
    return PASS, "fmt built and ran, exit 0/0"


def t_partoken(c):
    """pdftexdir/tests/partoken.test: ok run exits 0, xfail exits nonzero."""
    copy_inputs([os.path.join(c.wtests, "partoken-ok.tex"),
                 os.path.join(c.wtests, "partoken-xfail.tex")], c.work)
    # Isolation: inputs are copied above; only the work dir is searched
    # (trailing ':' keeps the engine's TeX Live tree defaults).
    env = base_env({"TEXINPUTS": c.work + ":",
                    "TEXMFDOTDIR": c.work + ":"})
    rc, _, _, timed_out, _ = run_cmd(
        [c.engine, "-ini", "--interaction=nonstopmode", "partoken-ok.tex"],
        c.work, env, c.timeout)
    if timed_out:
        return FAIL, "partoken-ok exit %s timeout=%s" % (rc, timed_out)
    if rc is None or rc < 0:
        return FAIL, "partoken-ok crashed (rc %s), want exit 0" % (rc,)
    if rc != 0:
        return FAIL, "partoken-ok exit %s timeout=%s" % (rc, timed_out)
    rc, _, _, timed_out, _ = run_cmd(
        [c.engine, "-ini", "--interaction=nonstopmode", "partoken-xfail.tex"],
        c.work, env, c.timeout)
    if timed_out:
        return FAIL, "partoken-xfail timed out (must fail fast)"
    if rc is None or rc < 0:
        return FAIL, "partoken-xfail crashed (rc %s), must fail" % (rc,)
    if rc == 0:
        return FAIL, "partoken-xfail exited 0, must fail"
    return PASS, "ok exits 0, xfail exits %s" % rc


def t_wprob(c):
    """pdftexdir/wprob.test: missing-image run must fail with exact message."""
    shutil.copy(os.path.join(c.wtests, "wprob.tex"),
                os.path.join(c.work, "pwprob.tex"))
    # No TEXINPUTS here: with an explicit search path kpathsea reports
    # the input as <dir>/pwprob.tex, but upstream's anchored grep needs
    # the ./pwprob.tex form. The checkout is not exposed either way.
    rc, _, _, timed_out, _ = run_cmd(
        [c.engine, "--ini", "--etex", "--file-line-error",
         "--interaction=nonstopmode", "pwprob.tex"],
        c.work, base_env({}), c.timeout)
    if timed_out:
        return FAIL, "timed out (must fail fast)"
    if rc is None or rc < 0:
        return FAIL, "crashed (rc %s), must fail" % (rc,)
    if rc == 0:
        return FAIL, "exited 0, must fail"
    try:
        log = read_bytes(os.path.join(c.work, "pwprob.log"))
    except OSError:
        return FAIL, "no pwprob.log written"
    if not log:
        return FAIL, "pwprob.log empty"
    if not wprob_ok(log):
        return FAIL, "expected './pwprob.tex:12: Could not open ...' line"
    return PASS, "fails with exact missing-file message"


def t_ttf2afm(c):
    """pdftexdir/ttf2afm.test: afm output matches after dropping dateline."""
    if not c.ttf2afm:
        return SKIP, "no ttf2afm binary (sibling of --engine or --ttf2afm)"
    # Isolation: only the .ttf inputs are copied in; the helper is
    # invoked with a relative name so a fake helper cannot derive the
    # expected .afm as a sibling of an absolute argv path. The expected
    # files are read by the runner only, after the run.
    copy_inputs([os.path.join(c.ptests, "postV3.ttf"),
                 os.path.join(c.ptests, "postV7.ttf")], c.work)
    bad = []
    for stem in ("postV3", "postV7"):
        rc, out, _, timed_out, _ = run_cmd(
            [c.ttf2afm, stem + ".ttf"],
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
        # Isolation (already): only the .pdf inputs are copied in and
        # the helper is invoked with a relative name; the expected
        # .xref files are read by the runner only, after the run.
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
    # Isolation: the generator script is an input, so it is copied into
    # the work dir and invoked by relative path -- an absolute checkout
    # path would let a fake perl derive sibling expected files
    # (tests/fn-utf8.txt). That expected file is read by the runner
    # only, after the run.
    shutil.copy(os.path.join(c.web2c, "tests", "fn-generate.perl"), c.work)
    try:
        want_job = read_bytes(os.path.join(c.web2c, "tests", "fn-utf8.txt"))
    except OSError as e:
        return FAIL, "cannot read expected fn-utf8.txt: %s" % e
    # Relative testdir (cwd=c.work): generated \openout paths stay relative.
    rc, _, err, timed_out, _ = run_cmd(
        ["perl", "-s", "fn-generate.perl", "-randgen=pdfuniformdeviate",
         "pdftests"],
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
            # Content checks (finding D): upstream only requires these
            # files to exist (`mv ... || rc=14`) -- its own content diff
            # is commented out -- so a stub touching empty files passes.
            # Every artifact must be non-empty; job.txt must carry the
            # expected content after ^^XX decoding; the term log must
            # carry this document's JOB marker.
            try:
                term = read_bytes(os.path.join(testdir, doc + "-term.log"))
            except OSError:
                term = b""
            # The JOB marker carries the job name in ^^XX-escaped form
            # for non-ASCII documents, exactly as the engine prints it.
            if not term or not term_has_job(term, job):
                bad.append(tag + " term log missing JOB[%s] marker" % job)
            try:
                got_job = read_bytes(os.path.join(testdir, job + ".txt"))
            except OSError:
                got_job = b""
            if not got_job:
                bad.append(tag + " empty " + job + ".txt")
            elif not wcfname_job_ok(got_job, want_job):
                bad.append(tag + " " + job + ".txt differs from fn-utf8.txt")
            for f in (job + ".log", job + ".fls"):
                if not nonempty(os.path.join(testdir, f)):
                    bad.append(tag + " empty " + f)
            if not nonempty(os.path.join(testdir, doc + "-tmp.tex")):
                bad.append(tag + " empty " + doc + "-tmp.tex")
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
    ap.add_argument("--budget", type=float, default=1800,
                    help="whole-run budget in seconds (default 1800): once "
                    "exhausted, remaining tests FAIL as (budget exhausted)")
    ap.add_argument("--allow-any-engine", action="store_true",
                    help="skip the pdfTeX 1.40.29 version gate")
    ap.add_argument("--allow-stale", action="store_true",
                    help="excuse EXPECTED-FAILURES.txt entries for tests "
                    "that now pass (default: stale entries fail the gate)")
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
    # Subprocesses run with cwd=scratch, where relative paths (which
    # subprocess resolves against the child's cwd) would break: absolutise
    # everything up front.
    args.engine = os.path.abspath(args.engine)
    args.cache = os.path.abspath(args.cache)
    if args.ttf2afm:
        args.ttf2afm = os.path.abspath(args.ttf2afm)
    if args.pdftosrc:
        args.pdftosrc = os.path.abspath(args.pdftosrc)
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
    for flag, path in (("--ttf2afm", args.ttf2afm),
                       ("--pdftosrc", args.pdftosrc)):
        if path and not (os.path.isfile(path)
                         and os.access(path, os.X_OK)):
            print("error: %s not executable: %s" % (flag, path),
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
        deadline = time.monotonic() + args.budget
        budget_exhausted = False
        for i, (name, script, desc, fn) in enumerate(selected):
            if time.monotonic() >= deadline:
                budget_exhausted = True
                for name, _, _, _ in selected[i:]:
                    results.append((name, FAIL, "budget exhausted"))
                    print("%-9s %s  (%s)" % (name, FAIL, "budget exhausted"))
                break
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
    passed = [n for n, s, _ in results if s == PASS]
    code, unexpected, stale = gate_exit(failed, expected, passed,
                                        args.allow_stale)
    if budget_exhausted:
        code = 1
    if failed and not unexpected:
        print("all failures are listed in EXPECTED-FAILURES.txt")
    for n in stale:
        print("stale-expected-failure: %s (listed in EXPECTED-FAILURES.txt "
              "but passed%s)" % (
                  n, "; --allow-stale given" if args.allow_stale
                  else "; remove it or pass --allow-stale"))
    if names:
        named_skipped = sorted(set(names) &
                               {n for n, s, _ in results if s == SKIP})
        if named_skipped:
            print("skipped explicitly requested test(s): %s "
                  "(--tests names a test that did not run)" %
                  " ".join(named_skipped))
            code = 1
    if npass == 0 and nfail == 0:
        print("no test produced PASS or FAIL "
              "(%d skipped; nothing was verified)" % nskip)
        code = 1
    return code


if __name__ == "__main__":
    sys.exit(main())
