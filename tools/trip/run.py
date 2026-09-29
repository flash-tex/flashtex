#!/usr/bin/env python3
"""Knuth's trip test and the e-TeX etrip test (port of TeX Live triptrap).

Oracle tooling only: Python 3 standard library, plus the engine under test
and (optionally) pltotf/tftopl/dvitype run as external programs. Upstream
inputs and expected outputs are fetched at run time by tools/trip/fetch.sh
into a cache outside the repository; nothing upstream is committed here.

Trip procedure (mirrors texk/web2c/triptest.test):
  TEXMFCNF=<cache>/triptrap, LC_ALL=C; pltotf trip.pl -> trip.tfm, tftopl
  back -> trip.pl (exact diff, gates); <engine> --progname=initex --ini
  <trip1.in >tripin.fot (trip.log -> tripin.log, trip.fmt must exist);
  <engine> --progname=tex <trip2.in >trip.fot; exact diff of tripos.tex
  (gates); dvitype trip.dvi -> trip.typ; filtered diffs of tripin.log,
  trip.fot, trip.log, trip.typ against the expected files (gate).

Etrip procedure (mirrors texk/web2c/etexdir/etriptest.test, TEXMFCNF=
<cache>/etrip): compat phase (trip files, ctrip* outputs), extended phase
(etrip1.in/trip2.in, xtrip* outputs, extra filter1 on the x side), and the
e-TeX specific phase (etrip files). Raw diffs that upstream leaves ungated
(banners, memory statistics, etrip.out) are reported as INFO; exactly the
comparisons upstream gates with is_OK=false gate here.

  python3 tools/trip/run.py --engine /Library/TeX/texbin/tex --kind trip
  python3 tools/trip/run.py --engine /Library/TeX/texbin/etex --kind etrip

Exit 0 iff no comparison FAILs and nothing SKIPped: a missing helper
tool (pltotf/tftopl/dvitype) is FAIL by default, matching upstream's hard
failure (triptest.test and etriptest.test do `|| exit 1` on those steps).
Pass --allow-missing-tools to restore the old SKIP behaviour for missing
helpers; any SKIP then reports INCOMPLETE (exit 1) with a loud warning that
a pass with skips is NOT a trip pass. A timed-out or signal-killed engine,
or a missing required artifact, is FAIL.

Upstream never checks the engine's exit status: a trip run legitimately
ends with a nonzero status on some engines (INITEX exits 1), so this tool
likewise ignores engine exit codes and gates only on timeouts, signal
death, missing artifacts, and the comparisons below.
"""

import argparse
import difflib
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile

TIMEOUT_DEFAULT = 120
DVITYPE_ARGS = ["-output-level=2", "-dpi=72.27",
                "-page-start=*.*.*.*.*.*.*.*.*.*"]

# (pattern, literal replacement) applied in order, first match per line,
# mirroring the sed filters in triptest.test and etexdir/etriptest.test.
# The etrip filter is NOT the trip filter plus extras: it replaces trip's
# first rule with two line deletions and two normalising rules, and it uses
# (preloaded format=.*tex). \n in a replacement is a real newline.
RULE_STARSTAR = (r"^\*\*\(./trip\.tex ##", "** &trip  trip \n(trip.tex ##")
RULE_TEXLIVE = (r" \(TeX Live 20[^)]*\)", "")
RULE_WEB2C = (r" \(Web2C 202[3-9][^)]*\)", "")
RULE_INITEX_TEX = (r"\(preloaded format=tex\)", "(INITEX)")
RULE_INITEX_ANY = (r"\(preloaded format=.*tex\)", "(INITEX)")
RULE_COMMA1 = (r"^\(trip\.tex ##", "**(./trip.tex ##")
RULE_COMMA2 = (r"^## \(./trip\.tex", "**(./trip.tex ##")
FILTER_TAIL = [
    (r"format=trip [^)][^)]*\)", "format=trip)"),
    (r"\)  [0-9A-Z: ]*$", ")"),
    (r"^\(./", "("),
    (r"[1-9][0-9]* strings out of [1-9].*", "XX strings out of YYY"),
    (r"[1-9][0-9]* string characters out of [1-9].*",
     "XXX string characters out of YYYY"),
    (r"sequences out of [1-9].*", "sequences out of YYYY"),
    (r"[1-9] hyphenation exceptions* out of [1-9].*",
     "X hyphenation exceptions out of YYY"),
    (r"[1-9][0-9]* strings of total length [1-9].*",
     "XXXX strings of total length YYYYY"),
    (r"9 ops out of [1-9][0-9]*", "9 ops out of YYY"),
    (r"TeX output ....\...\...:....", "TeX output YYYY.MM.DD:hhmm"),
    (r" 16341\.999.*fil", " 16342.0fil"),
    (r" 16238\.999.*fil", " 16239.0fil"),
    (r" 16317\.999.*fil", " 16318.0fil"),
    (r" 16330\.999.*fil", " 16331.0fil"),
    (r" 16331\.999.*fil", " 16332.0fil"),
    (r" 16343\.999.*fil", " 16344.0fil"),
    (r" 9737\.587..fil", " 9737.58789fil"),
    (r"down4 639342...", "down4 639342208"),
    (r"y4 2039217..", "y4 203921760"),
    (r"y0 2039217..", "y0 203921760"),
]
FILTER_TRIP = ([RULE_STARSTAR, RULE_TEXLIVE, RULE_WEB2C, RULE_INITEX_TEX]
               + FILTER_TAIL)
FILTER_ETRIP_DEL = [r"^\*\* &trip  trip", r"^\*\*entering extended mode"]
FILTER_ETRIP = ([RULE_COMMA1, RULE_COMMA2, RULE_TEXLIVE, RULE_WEB2C,
                 RULE_INITEX_ANY] + FILTER_TAIL + [
    (r"This is .*TeX,", "This is *TeX,"),
    (r" Version 3\.141592653[^(]*\(", " Version 3.141592653* ("),
    (r" before: [1-9][0-9][0-9][0-9]*&[1-9][0-9][0-9][0-9]*; ",
     " before: XXX&YYY; "),
    (r" after: [1-9][0-9][0-9][0-9]*&[1-9][0-9][0-9][0-9]*; ",
     " after: XXX&YYY; "),
    (r" still untouched: [1-9][0-9][0-9][0-9]*", " still untouched: XXX"),
])
# filter1 (extended phase, x side only): whole-text rule joining the
# group-trace lines, mirroring the :l/N/$!b l sed script.
FILTER1 = [(r" inside a group at level 1\).*bottom level",
            r" inside a group at level 1)", True)]

for _pat, _rep in FILTER_TRIP + FILTER_ETRIP:
    re.compile(_pat)
re.compile(FILTER1[0][0])
del _pat, _rep


def _split_sed_lines(data):
    """Split bytes into (body, terminated) exactly as sed sees lines:
    lines end only at b'\\n' (CR, NEL and other bytes are ordinary data);
    a trailing partial line without b'\\n' is still a line and keeps its
    missing terminator (verified with local sed; GNU sed behaves the same);
    the empty file has no lines."""
    if not data:
        return []
    parts = data.split(b"\n")
    if data.endswith(b"\n"):
        return [(p, True) for p in parts[:-1]]
    return [(p, True) for p in parts[:-1]] + [(parts[-1], False)]


def apply_filter(data, rules, extra_dels=()):
    """Filter bytes line by line like `sed -f filter`. Lines are bytes
    split only on b'\\n'; each line is filtered as latin-1 (a 1:1 byte
    mapping, so the ASCII patterns behave byte-wise and never split on
    Unicode boundaries the way str.splitlines() does)."""
    if isinstance(data, str):
        data = data.encode("latin-1")
    out = []
    for body, terminated in _split_sed_lines(data):
        text = body.decode("latin-1")
        if any(re.search(p, text) for p in extra_dels):
            continue
        for pat, rep in rules:
            text = re.sub(pat, rep, text, count=1)
        out.append(text.encode("latin-1") + (b"\n" if terminated else b""))
    return b"".join(out)


def apply_filter1(data):
    if isinstance(data, str):
        data = data.encode("latin-1")
    text = data.decode("latin-1")
    for pat, rep, _dotall in FILTER1:
        text = re.sub(pat, rep, text, count=0, flags=re.DOTALL)
    return text.encode("latin-1")


class Result:
    def __init__(self):
        self.rows = []  # (name, status, detail); status in PASS FAIL SKIP INFO

    def add(self, name, status, detail=""):
        self.rows.append((name, status, detail))

    def failed(self):
        return any(s == "FAIL" for _, s, _ in self.rows)

    def report(self):
        lines = []
        for name, status, detail in self.rows:
            lines.append("%s %s" % (status.ljust(4), name))
            if detail and status in ("FAIL", "INFO", "SKIP"):
                for dline in detail.splitlines()[:25]:
                    lines.append("      | " + dline)
        return "\n".join(lines)


def run_cmd(argv, cwd, env, stdin_path=None, stdout_path=None, timeout=TIMEOUT_DEFAULT):
    """Run argv; return (rc, note). stdin from a file (or /dev/null when
    stdin_path is None); stdout to a file when stdout_path is given, else
    /dev/null. Timeout kills the whole process group."""
    stdin = open(stdin_path, "rb") if stdin_path else open(os.devnull, "rb")
    stdout = open(stdout_path, "wb") if stdout_path else open(os.devnull, "wb")
    stderr = tempfile.TemporaryFile()
    timed_out = False
    try:
        proc = subprocess.Popen(argv, cwd=cwd, env=env, stdin=stdin,
                                stdout=stdout, stderr=stderr,
                                start_new_session=True)
        try:
            proc.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except (ProcessLookupError, PermissionError):
                pass
            proc.communicate()
        rc = proc.returncode
    finally:
        stdin.close()
        stdout.close()
    stderr.seek(0)
    err = stderr.read().decode("utf-8", "replace")[-2000:]
    stderr.close()
    if timed_out:
        return rc, "timed out after %ds, process group killed" % timeout
    if rc is not None and rc < 0:
        return rc, "killed by signal %d%s" % (-rc, (": " + err.strip()) if err.strip() else "")
    return rc, ("stderr: " + err.strip()) if rc and err.strip() else ""


def read(path):
    """Read a file as bytes: no newline translation, no decoding, so CR,
    CRLF and non-UTF-8 bytes compare exactly as upstream's diff/sed see
    them. 'exact' files compare byte for byte."""
    with open(path, "rb") as f:
        return f.read()


def first_diff(expected, actual, exp_label="expected", act_label="actual", n=12):
    if isinstance(expected, bytes):
        expected = expected.decode("latin-1")
    if isinstance(actual, bytes):
        actual = actual.decode("latin-1")
    diff = list(difflib.unified_diff(expected.split("\n"), actual.split("\n"),
                                     exp_label, act_label, lineterm=""))
    if not diff:
        return ""
    head = diff[:n]
    if len(diff) > n:
        head.append("... (%d more diff lines)" % (len(diff) - n))
    return "\n".join(head)


def compare(res, name, exp_path, act_path, filt=None, gate=True, need=()):
    """Compare two files after an optional filter. need lists artifacts that
    must exist for the comparison to be meaningful."""
    for needed in need:
        if not os.path.exists(needed):
            res.add(name, "FAIL", "missing required artifact: %s" % needed)
            return
    if not os.path.exists(exp_path):
        res.add(name, "FAIL", "missing expected file: %s" % exp_path)
        return
    if not os.path.exists(act_path):
        res.add(name, "FAIL", "missing output file: %s" % act_path)
        return
    exp, act = read(exp_path), read(act_path)
    if filt:
        exp, act = filt(exp), filt(act)
    if exp == act:
        res.add(name, "PASS")
    else:
        res.add(name, "FAIL" if gate else "INFO",
                "first difference (%s vs %s):\n%s"
                % (exp_path, act_path, first_diff(exp, act)))


def trip_filter(text):
    return apply_filter(text, FILTER_TRIP)


def etrip_filter(text):
    return apply_filter(text, FILTER_ETRIP, FILTER_ETRIP_DEL)


def engine_env(cnfdir, kind):
    env = dict(os.environ)
    env["TEXMFCNF"] = cnfdir
    env["LC_ALL"] = "C"
    env["LANGUAGE"] = "C"
    if kind == "etrip":
        # Prebuilt e-TeX binaries (e.g. TeX Live 2026 etex, a pdfTeX binary
        # in etex mode) carry more startup pool strings than the 32000 the
        # etrip texmf.cnf allows; 40000 still aborts with "increase
        # POOLSIZE". kpathsea lets the environment override texmf.cnf, and
        # every pool-capacity number is absorbed by the accepted-difference
        # filter, so raise the floor without touching the cnf.
        env["pool_size"] = "60000"
    return env


def _is_nonempty(path):
    """Mirror upstream `test -s`: exists and larger than 0 bytes."""
    return os.path.isfile(path) and os.path.getsize(path) > 0


def engine_run(res, name, engine, args, work, env, stdin, stdout, timeout,
               require=(), require_nonempty=()):
    """Run the engine (exit code ignored, like upstream); FAIL on timeout,
    signal death, a missing required artifact afterwards, or an empty
    artifact listed in require_nonempty (upstream's `test ! -s <fmt>`
    aborts with exit 1, e.g. `*** trip.fmt not created`). Plain require
    mirrors the `mv ... || exit 1` steps (existence only)."""
    rc, note = run_cmd([engine] + args, cwd=work, env=env, stdin_path=stdin,
                       stdout_path=stdout, timeout=timeout)
    missing = [a for a in require if not os.path.exists(os.path.join(work, a))]
    missing += [a for a in require_nonempty
                if not _is_nonempty(os.path.join(work, a))]
    if note.startswith("timed out") or (rc is not None and rc < 0):
        res.add(name, "FAIL", note or ("exit %s" % rc))
        return False
    if missing:
        res.add(name, "FAIL", "missing or empty after run (exit %s): %s%s"
                % (rc, ", ".join(missing), ("; " + note) if note else ""))
        return False
    return True


def font_roundtrip(res, name, srcdir, work, env, pl, timeout, tools,
                   allow_missing=False):
    """pltotf <pl> -> work/<base>.tfm (the engine reads it from the workdir,
    as upstream), tftopl back -> roundtrip.pl, exact diff. FAIL (like
    upstream's `|| exit 1`) when either tool is absent unless allow_missing,
    which restores SKIP."""
    missing = [t for t in ("pltotf", "tftopl") if not tools[t]]
    if missing:
        msg = ("missing helper tool(s) on PATH: %s "
               "(upstream triptest.test exits 1 here; pass --%s or "
               "--allow-missing-tools to skip)"
               % (", ".join(missing), missing[0]))
        res.add(name, "SKIP" if allow_missing else "FAIL", msg)
        return
    base = os.path.splitext(pl)[0]
    tfm = os.path.join(work, base + ".tfm")
    back = os.path.join(work, "roundtrip-%s" % pl)
    rc, note = run_cmd([tools["pltotf"], os.path.join(srcdir, pl), tfm],
                       cwd=work, env=env, timeout=timeout)
    if rc != 0:
        res.add(name, "FAIL", "pltotf exit %s%s" % (rc, ("; " + note) if note else ""))
        return
    rc, note = run_cmd([tools["tftopl"], tfm, back],
                       cwd=work, env=env, timeout=timeout)
    if rc != 0:
        res.add(name, "FAIL", "tftopl exit %s%s" % (rc, ("; " + note) if note else ""))
        return
    compare(res, name, os.path.join(srcdir, pl), back)


def dvitype_run(res, name, work, env, dvi, typ_out, timeout, tools,
                allow_missing=False):
    """dvitype <dvi> -> typ_out. FAIL (like upstream's `|| exit 1`) when
    dvitype is absent unless allow_missing, which restores SKIP; FAIL when
    it errors."""
    if not tools["dvitype"]:
        res.add(name, "SKIP" if allow_missing else "FAIL",
                "missing helper tool on PATH: dvitype "
                "(upstream triptest.test exits 1 here; pass --dvitype or "
                "--allow-missing-tools to skip)")
        return False
    rc, note = run_cmd([tools["dvitype"]] + DVITYPE_ARGS
                       + [os.path.join(work, dvi)],
                       cwd=work, env=env,
                       stdout_path=os.path.join(work, typ_out), timeout=timeout)
    if rc != 0 or not os.path.exists(os.path.join(work, typ_out)):
        res.add(name, "FAIL", "dvitype exit %s%s" % (rc, ("; " + note) if note else ""))
        return False
    return True


def run_trip(engine, tdir, work, env, timeout, tools, res,
             allow_missing=False):
    for f in ("trip.tex", "trip.pl", "trip1.in", "trip2.in"):
        shutil.copy(os.path.join(tdir, f), work)
    font_roundtrip(res, "trip.pl round-trip", tdir, work, env, "trip.pl",
                   timeout, tools, allow_missing)
    ok = engine_run(res, "trip pass 1 (initex)", engine,
                    ["--progname=initex", "--ini"], work, env,
                    os.path.join(work, "trip1.in"),
                    os.path.join(work, "tripin.fot"), timeout,
                    require=("trip.log",),
                    require_nonempty=("trip.fmt",))
    if ok:
        os.rename(os.path.join(work, "trip.log"), os.path.join(work, "tripin.log"))
    else:
        return
    if not engine_run(res, "trip pass 2", engine, ["--progname=tex"], work,
                      env, os.path.join(work, "trip2.in"),
                      os.path.join(work, "trip.fot"), timeout,
                      require=("trip.log", "trip.dvi", "tripos.tex")):
        return
    compare(res, "tripin.log (filtered)", os.path.join(tdir, "tripin.log"),
            os.path.join(work, "tripin.log"), trip_filter)
    compare(res, "trip.fot (filtered)", os.path.join(tdir, "trip.fot"),
            os.path.join(work, "trip.fot"), trip_filter)
    compare(res, "trip.log (filtered)", os.path.join(tdir, "trip.log"),
            os.path.join(work, "trip.log"), trip_filter)
    compare(res, "tripos.tex (exact)", os.path.join(tdir, "tripos.tex"),
            os.path.join(work, "tripos.tex"))
    if dvitype_run(res, "dvitype", work, env, "trip.dvi", "trip.typ",
                   timeout, tools, allow_missing):
        compare(res, "trip.typ (filtered)", os.path.join(tdir, "trip.typ"),
                os.path.join(work, "trip.typ"), trip_filter)


def etrip_phase(res, engine, tdir, work, env, timeout, tag,
                ini_stdin, run_stdin):
    """One etrip trip-file phase, tag 'ctrip' (compat) or 'xtrip' (extended).
    Output names mirror upstream: <tag>in.fot, <tag>in.log, <tag>.fot,
    <tag>.log, <tag>.fmt, <tag[0]>tripos.tex. False when the engine died."""
    for f in ("trip.tex", "trip.pl", "trip1.in", "trip2.in"):
        shutil.copy(os.path.join(tdir, f), work)
    ok = engine_run(res, "%s pass 1 (initex)" % tag, engine,
                    ["--progname=einitex", "--ini"], work, env, ini_stdin,
                    os.path.join(work, tag + "in.fot"), timeout,
                    require=("trip.log",),
                    require_nonempty=("trip.fmt",))
    if not ok:
        return False
    os.rename(os.path.join(work, "trip.log"), os.path.join(work, tag + "in.log"))
    if not engine_run(res, "%s pass 2" % tag, engine,
                      ["--progname=etex"], work, env, run_stdin,
                      os.path.join(work, tag + ".fot"), timeout,
                      require=("trip.log", "trip.dvi", "tripos.tex")):
        return False
    # pass 2 loads work/trip.fmt, so it is archived only now (as upstream).
    if os.path.exists(os.path.join(work, "trip.fmt")):
        os.rename(os.path.join(work, "trip.fmt"),
                  os.path.join(work, tag + ".fmt"))
    os.rename(os.path.join(work, "trip.log"), os.path.join(work, tag + ".log"))
    os.rename(os.path.join(work, "tripos.tex"),
              os.path.join(work, tag[0] + "tripos.tex"))
    os.rename(os.path.join(work, "trip.dvi"),
              os.path.join(work, tag[0] + ".dvi"))
    return True


def xflt(text):
    return apply_filter1(etrip_filter(text))


def run_etrip(engine, tdir, edir, work, env, timeout, tools, res,
              allow_missing=False):
    # Compat phase.
    font_roundtrip(res, "trip.pl round-trip", tdir, work, env, "trip.pl",
                   timeout, tools, allow_missing)
    if not etrip_phase(res, engine, tdir, work, env, timeout,
                       "ctrip", os.path.join(tdir, "trip1.in"),
                       os.path.join(tdir, "trip2.in")):
        return
    compare(res, "ctripin.log vs tripin.log (info)",
            os.path.join(tdir, "tripin.log"), os.path.join(work, "ctripin.log"),
            gate=False)
    compare(res, "ctripos.tex vs tripos.tex (info)",
            os.path.join(tdir, "tripos.tex"), os.path.join(work, "ctripos.tex"),
            gate=False)
    # Terminal output (.fot) is emitted by the engine directly and needs
    # no dvitype, so it is compared independently of dvitype availability.
    compare(res, "ctrip.fot (filtered)", os.path.join(tdir, "trip.fot"),
            os.path.join(work, "ctrip.fot"), etrip_filter)
    if dvitype_run(res, "dvitype (compat)", work, env, "c.dvi",
                   "ctrip.typ", timeout, tools, allow_missing):
        compare(res, "ctrip.typ (filtered)", os.path.join(tdir, "trip.typ"),
                os.path.join(work, "ctrip.typ"), etrip_filter)
    # Extended phase.
    if not etrip_phase(res, engine, tdir, work, env, timeout,
                       "xtrip", os.path.join(edir, "etrip1.in"),
                       os.path.join(edir, "trip2.in")):
        return
    compare(res, "xtripin.log vs ctripin.log (info)",
            os.path.join(work, "ctripin.log"), os.path.join(work, "xtripin.log"),
            gate=False)
    compare(res, "xtripos.tex vs tripos.tex (info)",
            os.path.join(tdir, "tripos.tex"),
            os.path.join(work, "xtripos.tex"), gate=False)
    # Terminal output (.fot) needs no dvitype; compare it even when DVI
    # validation below is skipped or fails.
    compare(res, "xtrip.fot (filtered)",
            os.path.join(work, "ctrip.fot"),
            os.path.join(work, "xtrip.fot"), xflt)
    if not tools["dvitype"]:
        res.add("dvitype (extended)", "SKIP" if allow_missing else "FAIL",
                "missing helper tool on PATH: dvitype "
                "(upstream etriptest.test exits 1 here; pass --dvitype or "
                "--allow-missing-tools to skip)")
    elif not os.path.exists(os.path.join(work, "x.dvi")):
        res.add("dvitype (extended)", "FAIL", "missing required artifact: x.dvi")
    elif dvitype_run(res, "dvitype (extended)", work, env, "x.dvi",
                     "xtrip.typ", timeout, tools, allow_missing):
        compare(res, "xtrip.typ (filtered)",
                os.path.join(work, "ctrip.typ"),
                os.path.join(work, "xtrip.typ"), xflt)
    # e-TeX specific phase.
    for f in ("etrip.tex", "etrip.pl", "etrip2.in", "etrip3.in"):
        shutil.copy(os.path.join(edir, f), work)
    font_roundtrip(res, "etrip.pl round-trip", edir, work, env, "etrip.pl",
                   timeout, tools, allow_missing)
    if not engine_run(res, "etrip pass 1 (initex)", engine,
                      ["--progname=einitex", "--ini"], work, env,
                      os.path.join(work, "etrip2.in"),
                      os.path.join(work, "etripin.fot"), timeout,
                      require=("etrip.log",),
                      require_nonempty=("etrip.fmt",)):
        return
    os.rename(os.path.join(work, "etrip.log"), os.path.join(work, "etripin.log"))
    compare(res, "etripin.log (info)", os.path.join(edir, "etripin.log"),
            os.path.join(work, "etripin.log"), gate=False)
    if not engine_run(res, "etrip pass 2", engine, ["--progname=etex"],
                      work, env, os.path.join(work, "etrip3.in"),
                      os.path.join(work, "etrip.fot"), timeout,
                      require=("etrip.log", "etrip.dvi", "etrip.out")):
        return
    compare(res, "etrip.log (info)", os.path.join(edir, "etrip.log"),
            os.path.join(work, "etrip.log"), gate=False)
    compare(res, "etrip.out (info)", os.path.join(edir, "etrip.out"),
            os.path.join(work, "etrip.out"), gate=False)
    # Terminal output (.fot) needs no dvitype; compare it independently.
    compare(res, "etrip.fot (filtered)", os.path.join(edir, "etrip.fot"),
            os.path.join(work, "etrip.fot"), etrip_filter)
    if dvitype_run(res, "dvitype (etrip)", work, env, "etrip.dvi",
                   "etrip.typ", timeout, tools, allow_missing):
        compare(res, "etrip.typ (filtered)", os.path.join(edir, "etrip.typ"),
                os.path.join(work, "etrip.typ"), etrip_filter)


def default_cache():
    return os.environ.get("FLASHTEX_TRIP_CACHE",
                          os.path.join(os.path.expanduser("~"),
                                       ".cache", "flashtex-trip"))


def check_inputs(tdir, edir, kind):
    need = {"trip": ["texmf.cnf", "trip.tex", "trip.pl", "trip1.in",
                     "trip2.in", "tripin.log", "trip.fot", "trip.log",
                     "trip.typ", "tripos.tex"],
            "etrip": ["texmf.cnf", "etrip.tex", "etrip.pl", "etrip2.in",
                      "etrip3.in", "etrip1.in", "trip2.in", "etripin.log",
                      "etrip.fot", "etrip.log", "etrip.typ", "etrip.out"]}
    missing = []
    for f in need["trip"]:
        if not os.path.exists(os.path.join(tdir, f)):
            missing.append("triptrap/" + f)
    if kind == "etrip":
        for f in need["etrip"]:
            if not os.path.exists(os.path.join(edir, f)):
                missing.append("etrip/" + f)
    return missing


def main(argv=None):
    ap = argparse.ArgumentParser(description="Run Knuth's trip test or the "
                                 "e-TeX etrip test against an engine binary.")
    ap.add_argument("--engine", required=True, help="engine binary under test")
    ap.add_argument("--kind", required=True, choices=("trip", "etrip"))
    ap.add_argument("--cache", default=default_cache(),
                    help="fetch.sh cache dir (default $FLASHTEX_TRIP_CACHE or "
                    "~/.cache/flashtex-trip)")
    ap.add_argument("--keep", action="store_true",
                    help="keep the staging workdir and print its path")
    ap.add_argument("--timeout", type=int, default=TIMEOUT_DEFAULT,
                    help="per-subprocess timeout in seconds")
    ap.add_argument("--pltotf", default=None)
    ap.add_argument("--tftopl", default=None)
    ap.add_argument("--dvitype", default=None)
    ap.add_argument("--allow-missing-tools", action="store_true",
                    help="downgrade missing pltotf/tftopl/dvitype from FAIL "
                    "to SKIP (result is INCOMPLETE, never PASS)")
    args = ap.parse_args(argv)

    tdir = os.path.join(args.cache, "triptrap")
    edir = os.path.join(args.cache, "etrip")
    missing = check_inputs(tdir, edir, args.kind)
    if missing:
        print("missing inputs in cache %s:\n  %s\nrun tools/trip/fetch.sh first"
              % (args.cache, "\n  ".join(missing)))
        return 2
    if not (os.path.isfile(args.engine) and os.access(args.engine, os.X_OK)):
        print("engine not executable: %s" % args.engine)
        return 2

    tools = {"pltotf": args.pltotf or shutil.which("pltotf"),
             "tftopl": args.tftopl or shutil.which("tftopl"),
             "dvitype": args.dvitype or shutil.which("dvitype")}
    for name, path in tools.items():
        if path is not None and not (os.path.isfile(path)
                                     and os.access(path, os.X_OK)):
            print("%s tool not executable: %s" % (name, path))
            return 2

    work = tempfile.mkdtemp(prefix="flashtrip-")
    res = Result()
    try:
        env = engine_env(tdir if args.kind == "trip" else edir, args.kind)
        if args.kind == "trip":
            run_trip(args.engine, tdir, work, env, args.timeout, tools, res,
                     args.allow_missing_tools)
        else:
            run_etrip(args.engine, tdir, edir, work, env, args.timeout,
                      tools, res, args.allow_missing_tools)
    finally:
        if args.keep:
            print("workdir kept: %s" % work)
        else:
            shutil.rmtree(work, ignore_errors=True)
    print("engine: %s kind: %s" % (args.engine, args.kind))
    print(res.report())
    fails = sum(1 for _, s, _ in res.rows if s == "FAIL")
    skips = sum(1 for _, s, _ in res.rows if s == "SKIP")
    if fails:
        outcome = "FAIL"
    elif skips:
        outcome = "INCOMPLETE"
    else:
        outcome = "PASS"
    line = "result: %s (%d fail, %d skip)" % (outcome, fails, skips)
    if skips and not fails and args.allow_missing_tools:
        line += (" WARNING: PASS with skips is NOT a trip pass "
                 "(--allow-missing-tools)")
    print(line)
    return 0 if outcome == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
