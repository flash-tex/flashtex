#!/usr/bin/env python3
"""Unit + fake-engine tests for tools/trip/run.py (stdlib only).

A 'perfect engine' fake (dispatched on --ini/stdin) must PASS; the same fake
serving one changed line must FAIL with a readable first-difference report.
Run:  python3 tools/trip/test_trip.py
"""
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest

RUN_PY = os.path.join(os.path.dirname(os.path.abspath(__file__)), "run.py")

# Slice 5 probes. ATTACK_FOO/BAR: compat-vs-extended outputs that differ
# only in a line between the filter1 anchors; the old DOTALL filter1
# swallowed the middle on both sides and reported PASS. REAL_XSPAN is the
# exact byte shape of the e-TeX group-trace block in real xtrip.fot
# (observed from a real etex run: `(end occurred ...)` / blank /
# `### <group> entered at line N (...)` / `### bottom level`).
ATTACK_FOO = (b"header\nA inside a group at level 1)\n"
              b"FOO middle\nbottom level B\nfooter\n")
ATTACK_BAR = ATTACK_FOO.replace(b"FOO", b"BAR")
REAL_XFOT_SPAN = (b"(end occurred inside a group at level 1)\n"
                  b"\n"
                  b"### semi simple group (level 1) entered at line 429 "
                  b"(begingroup)\n"
                  b"### bottom level\n")
REAL_XFOT_AFTER = b"(end occurred when if on line 442 was incomplete)\n"
SINGLE_LINE = b"header\nA inside a group at level 1) FOO bottom level B\nfooter\n"
SINGLE_LINE_COLLAPSED = (b"header\nA inside a group at level 1) B\nfooter\n")

LEAK_ENGINE = """#!/bin/sh
# Cheat shim: does no TeX work, just serves the expected files straight
# out of $TEXMFCNF. Passes iff TEXMFCNF exposes the expected outputs.
if printf '%s' "$*" | grep -q -- --ini; then
    cp "$TEXMFCNF/tripin.log" trip.log 2>/dev/null
    echo fmt > trip.fmt
else
    cat "$TEXMFCNF/trip.fot" 2>/dev/null
    cp "$TEXMFCNF/trip.log" trip.log 2>/dev/null
    cp "$TEXMFCNF/tripos.tex" tripos.tex 2>/dev/null
    : > trip.dvi
fi
"""


def parse_upstream_sed_scripts(path):
    """Parse the real `filter`/`filter1` sed scripts out of a pinned
    upstream test script (the `cat >name <<-\\_EOF` heredocs; `<<-`
    strips leading tabs). Returns {name: bytes}."""
    out = {}
    cur = None
    with open(path, "rb") as f:
        for raw in f.read().split(b"\n"):
            line = raw.decode("utf-8", "replace")
            if cur is None:
                m = re.match(r"^cat (>>?)(\S+) <<-\\_EOF$", line)
                if m:
                    cur = m.group(2)
                    out.setdefault(cur, b"")
            else:
                # `<<-` also allows the delimiter itself to be tab-indented.
                if line.lstrip("\t") == "_EOF":
                    cur = None
                else:
                    out[cur] += line.lstrip("\t").encode() + b"\n"
    return out


def upstream_paths():
    """Locate the pinned upstream test scripts + cached expected files."""
    bases = []
    env_cache = os.environ.get("FLASHTEX_TRIP_CACHE")
    if env_cache:
        bases.append(env_cache)
    bases.append(os.path.join(os.path.expanduser("~"), ".cache",
                              "flashtex-trip"))
    for base in bases:
        etrip_test = os.path.join(base, "src", "texlive-source", "texk",
                                  "web2c", "etexdir", "etriptest.test")
        trip_test = os.path.join(base, "src", "texlive-source", "texk",
                                 "web2c", "triptest.test")
        tdir = os.path.join(base, "triptrap")
        edir = os.path.join(base, "etrip")
        if (os.path.isfile(etrip_test) and os.path.isfile(trip_test)
                and os.path.isfile(os.path.join(tdir, "trip.fot"))):
            return etrip_test, trip_test, tdir, edir
    return None, None, None, None


def sed_filter(data, *scripts):
    """Run `sed -f s1 | sed -f s2 | ...` exactly like upstream's test
    scripts do (filter1 is piped after filter, never merged into one
    sed invocation). scripts are (name, bytes) pairs."""
    tmp = tempfile.mkdtemp(prefix="sedcheck-")
    try:
        procs = []
        paths = []
        for name, blob in scripts:
            p = os.path.join(tmp, name)
            with open(p, "wb") as f:
                f.write(blob)
            paths.append(p)
        inp = data
        for p in paths:
            proc = subprocess.run(["sed", "-f", p], input=inp,
                                  capture_output=True, timeout=60)
            if proc.returncode != 0:
                raise RuntimeError("sed -f %s failed: %s" % (p, proc.stderr))
            inp = proc.stdout
        return inp
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

FAKE_ENGINE = """import json, os, sys
key = ("ini" if "--ini" in sys.argv[1:] else "run") + ":" + sys.stdin.buffer.read().decode()
act = json.loads(os.environ["FAKE_TABLE"])[key]
for dst, src in act.get("copy", {}).items():
    with open(src, "rb") as f: data = f.read()
    with open(dst, "wb") as f: f.write(data)
for name, text in act.get("write", {}).items():
    with open(name, "w") as f: f.write(text)
if "stdout" in act:
    with open(act["stdout"], "rb") as f:
        sys.stdout.buffer.write(f.read())
"""

FAKE_TFMTOPL = """import os, shutil, sys
# pltotf records its source .pl path in the dummy tfm; tftopl copies it back.
if os.path.basename(sys.argv[0]).startswith("fake-pltotf"):
    open(sys.argv[2], "w").write(sys.argv[1])
else:
    with open(sys.argv[1]) as f:
        shutil.copy(f.read().strip(), sys.argv[2])
"""

FAKE_DVITYPE = """import json, os, sys
dvi = os.path.basename(sys.argv[-1])
table = json.loads(os.environ["FAKE_DVI"])
sys.stdout.write(open(table[dvi]).read())
"""


def canned(root, name, text):
    with open(os.path.join(root, name), "w") as f:
        f.write(text)


class TripTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="triptest-")
        self.cache = os.path.join(self.tmp, "cache")
        tdir = os.path.join(self.cache, "triptrap")
        edir = os.path.join(self.cache, "etrip")
        os.makedirs(tdir)
        os.makedirs(edir)
        # Synthetic inputs; engine/tools are faked so contents are arbitrary
        # but self-consistent (fakes serve these very files back).
        canned(tdir, "texmf.cnf", "error_line = 64\n")
        canned(tdir, "trip.tex", "\\end\n")
        canned(tdir, "trip.pl", "canned trip pl\n")
        canned(tdir, "trip1.in", "\\input trip\n")
        canned(tdir, "trip2.in", " &trip  trip \n")
        canned(tdir, "tripin.log", "INITEX banner (INITEX)\n1326 strings of total length 23633\n")
        canned(tdir, "trip.fot", "fot line 1\nfot line 2\n")
        canned(tdir, "trip.log", "trip log line 1\ntrip log line 2\n")
        canned(tdir, "trip.typ", "DVItype output\n")
        canned(tdir, "tripos.tex", "\\gdef\\x{1}\n")
        canned(edir, "texmf.cnf", "error_line = 64\n")
        canned(edir, "etrip.tex", "\\end\n")
        canned(edir, "etrip.pl", "canned etrip pl\n")
        canned(edir, "etrip2.in", "*etrip\n")
        canned(edir, "etrip3.in", "&etrip etrip\n")
        canned(edir, "etrip1.in", "\n*\\input trip\n")
        canned(edir, "trip2.in", "&trip \\toksdef\\tokens=0 \\input trip\n")
        canned(edir, "etripin.log", "etrip initex log\n")
        canned(edir, "etrip.fot", "etrip fot\n")
        canned(edir, "etrip.log", "etrip log\n")
        canned(edir, "etrip.typ", "etrip DVItype output\n")
        canned(edir, "etrip.out", "etrip out\n")
        bindir = os.path.join(self.tmp, "bin")
        os.makedirs(bindir)
        self.engine = os.path.join(bindir, "fake-engine")
        canned(bindir, "fake-engine", FAKE_ENGINE)
        canned(bindir, "fake-pltotf", FAKE_TFMTOPL)
        canned(bindir, "fake-tftopl", FAKE_TFMTOPL)
        canned(bindir, "fake-dvitype", FAKE_DVITYPE)
        for f in ("fake-engine", "fake-pltotf", "fake-tftopl", "fake-dvitype"):
            p = os.path.join(bindir, f)
            with open(p) as fh:
                body = fh.read()
            with open(p, "w") as fh:
                # Absolute interpreter: some tests strip PATH.
                fh.write("#!" + sys.executable + "\n" + body)
            os.chmod(p, 0o755)
        self.bindir = bindir
        self.env = dict(os.environ, FAKE_SRC=tdir,
                        PATH=bindir + ":/usr/bin:/bin")

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def table(self):
        tdir = os.path.join(self.cache, "triptrap")
        edir = os.path.join(self.cache, "etrip")
        return json.dumps({
            "ini:\\input trip\n": {
                "write": {"trip.fmt": "fmt", "etrip.fmt": "fmt"},
                "copy": {"trip.log": tdir + "/tripin.log",
                         "etrip.log": edir + "/etripin.log"}},
            "run: &trip  trip \n": {
                "stdout": tdir + "/trip.fot",
                "copy": {"trip.log": tdir + "/trip.log",
                         "tripos.tex": tdir + "/tripos.tex",
                         "trip.dvi": tdir + "/trip.fot"}},
            "ini:\n*\\input trip\n": {
                "write": {"trip.fmt": "fmt"},
                "copy": {"trip.log": tdir + "/tripin.log"}},
            "run:&trip \\toksdef\\tokens=0 \\input trip\n": {
                "stdout": tdir + "/trip.fot",
                "copy": {"trip.log": tdir + "/trip.log",
                         "tripos.tex": tdir + "/tripos.tex",
                         "trip.dvi": tdir + "/trip.fot"}},
            "ini:*etrip\n": {
                "write": {"etrip.fmt": "fmt"},
                "copy": {"etrip.log": edir + "/etripin.log"}},
            "run:&etrip etrip\n": {
                "stdout": edir + "/etrip.fot",
                "copy": {"etrip.log": edir + "/etrip.log",
                         "etrip.dvi": tdir + "/trip.fot",
                         "etrip.out": edir + "/etrip.out"}},
        })

    def dvi_table(self):
        tdir = os.path.join(self.cache, "triptrap")
        edir = os.path.join(self.cache, "etrip")
        return json.dumps({"trip.dvi": tdir + "/trip.typ",
                           "c.dvi": tdir + "/trip.typ",
                           "x.dvi": tdir + "/trip.typ",
                           "etrip.dvi": edir + "/etrip.typ"})

    def run_cli(self, kind, extra_env=None, extra_args=()):
        env = dict(self.env)
        env["FAKE_TABLE"] = self.table()
        env["FAKE_DVI"] = self.dvi_table()
        if extra_env:
            env.update(extra_env)
        p = subprocess.run(
            [sys.executable, RUN_PY, "--engine", self.engine, "--kind", kind,
             "--cache", self.cache, "--timeout", "60",
             "--pltotf", os.path.join(self.bindir, "fake-pltotf"),
             "--tftopl", os.path.join(self.bindir, "fake-tftopl"),
             "--dvitype", os.path.join(self.bindir, "fake-dvitype")]
            + list(extra_args),
            capture_output=True, text=True, env=env, stdin=subprocess.DEVNULL,
            timeout=300)
        return p

    def test_filter_normalises_banner_and_stats(self):
        sys.path.insert(0, os.path.dirname(RUN_PY))
        import run as r
        a = r.trip_filter("This is TeX, Version 3.141592653 (TeX Live 2026) (INITEX)\n"
                          "1372 strings of total length 24645\n")
        b = r.trip_filter("This is TeX, Version 3.141592653 (INITEX)\n"
                          "1326 strings of total length 23633\n")
        self.assertEqual(a, b)

    def test_trip_perfect_engine_passes(self):
        p = self.run_cli("trip")
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        self.assertIn("result: PASS", p.stdout)
        self.assertNotIn("FAIL", p.stdout)

    def test_trip_one_changed_line_fails(self):
        tdir = os.path.join(self.cache, "triptrap")
        # Serve a tampered pass-2 log while the expected file stays intact.
        tampered = os.path.join(self.tmp, "tampered.log")
        shutil.copy(os.path.join(tdir, "trip.log"), tampered)
        with open(tampered, "a") as f:
            f.write("INJECTED LINE\n")
        t = json.loads(self.table())
        t["run: &trip  trip \n"]["copy"]["trip.log"] = tampered
        env = dict(self.env, FAKE_TABLE=json.dumps(t),
                   FAKE_DVI=self.dvi_table())
        p = subprocess.run(
            [sys.executable, RUN_PY, "--engine", self.engine, "--kind", "trip",
             "--cache", self.cache, "--timeout", "60",
             "--pltotf", os.path.join(self.bindir, "fake-pltotf"),
             "--tftopl", os.path.join(self.bindir, "fake-tftopl"),
             "--dvitype", os.path.join(self.bindir, "fake-dvitype")],
            capture_output=True, text=True, env=env, stdin=subprocess.DEVNULL,
            timeout=300)
        self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
        self.assertIn("FAIL trip.log (filtered)", p.stdout)
        self.assertIn("INJECTED LINE", p.stdout)

    def test_trip_crlf_outputs_fail(self):
        # BLOCKER 1: byte-identical outputs with LF turned into CRLF must
        # FAIL every comparison, including tripos.tex (exact). The old
        # text-mode read() translated CRLF back to LF via universal
        # newlines, so this shim passed.
        tdir = os.path.join(self.cache, "triptrap")
        crlf = {}
        for name in ("tripin.log", "trip.fot", "trip.log", "tripos.tex"):
            with open(os.path.join(tdir, name), "rb") as f:
                data = f.read().replace(b"\n", b"\r\n")
            p = os.path.join(self.tmp, "crlf-" + name)
            with open(p, "wb") as f:
                f.write(data)
            crlf[name] = p
        t = json.loads(self.table())
        t["ini:\\input trip\n"]["copy"]["trip.log"] = crlf["tripin.log"]
        t["run: &trip  trip \n"]["stdout"] = crlf["trip.fot"]
        t["run: &trip  trip \n"]["copy"]["trip.log"] = crlf["trip.log"]
        t["run: &trip  trip \n"]["copy"]["tripos.tex"] = crlf["tripos.tex"]
        env = dict(self.env, FAKE_TABLE=json.dumps(t),
                   FAKE_DVI=self.dvi_table())
        p = subprocess.run(
            [sys.executable, RUN_PY, "--engine", self.engine, "--kind", "trip",
             "--cache", self.cache, "--timeout", "60",
             "--pltotf", os.path.join(self.bindir, "fake-pltotf"),
             "--tftopl", os.path.join(self.bindir, "fake-tftopl"),
             "--dvitype", os.path.join(self.bindir, "fake-dvitype")],
            capture_output=True, text=True, env=env, stdin=subprocess.DEVNULL,
            timeout=300)
        self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
        for row in ("FAIL tripin.log (filtered)",
                    "FAIL trip.fot (filtered)",
                    "FAIL trip.log (filtered)",
                    "FAIL tripos.tex (exact)"):
            self.assertIn(row, p.stdout, p.stdout + p.stderr)

    def test_trip_nel_is_not_a_line_break(self):
        # BLOCKER 1: U+0085 NEL (bytes C2 85) is an ordinary byte to
        # upstream sed, which splits lines only on LF, so the
        # s,^\(./,(, rule must not fire after it mid-line. Python's
        # str.splitlines() splits on NEL, which wrongly fired the rule
        # and masked this byte difference.
        tdir = os.path.join(self.cache, "triptrap")
        with open(os.path.join(tdir, "trip.log"), "wb") as f:
            f.write(b"header\nAAA\xc2\x85(BBB tail\nfooter\n")
        tampered = os.path.join(self.tmp, "nel.log")
        with open(tampered, "wb") as f:
            f.write(b"header\nAAA\xc2\x85(./BBB tail\nfooter\n")
        t = json.loads(self.table())
        t["run: &trip  trip \n"]["copy"]["trip.log"] = tampered
        env = dict(self.env, FAKE_TABLE=json.dumps(t),
                   FAKE_DVI=self.dvi_table())
        p = subprocess.run(
            [sys.executable, RUN_PY, "--engine", self.engine, "--kind", "trip",
             "--cache", self.cache, "--timeout", "60",
             "--pltotf", os.path.join(self.bindir, "fake-pltotf"),
             "--tftopl", os.path.join(self.bindir, "fake-tftopl"),
             "--dvitype", os.path.join(self.bindir, "fake-dvitype")],
            capture_output=True, text=True, env=env, stdin=subprocess.DEVNULL,
            timeout=300)
        self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
        self.assertIn("FAIL trip.log (filtered)", p.stdout,
                      p.stdout + p.stderr)

    def test_trip_empty_fmt_fails(self):
        # BLOCKER 2: upstream aborts (`*** trip.fmt not created`, exit 1)
        # when the format is missing (`test ! -s trip.fmt`). A 0-byte
        # trip.fmt with all other artifacts intact must FAIL.
        t = json.loads(self.table())
        t["ini:\\input trip\n"]["write"] = {"trip.fmt": "",
                                            "etrip.fmt": "fmt"}
        env = dict(self.env, FAKE_TABLE=json.dumps(t),
                   FAKE_DVI=self.dvi_table())
        p = subprocess.run(
            [sys.executable, RUN_PY, "--engine", self.engine, "--kind", "trip",
             "--cache", self.cache, "--timeout", "60",
             "--pltotf", os.path.join(self.bindir, "fake-pltotf"),
             "--tftopl", os.path.join(self.bindir, "fake-tftopl"),
             "--dvitype", os.path.join(self.bindir, "fake-dvitype")],
            capture_output=True, text=True, env=env, stdin=subprocess.DEVNULL,
            timeout=300)
        self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
        self.assertIn("FAIL trip pass 1 (initex)", p.stdout,
                      p.stdout + p.stderr)
        self.assertIn("trip.fmt", p.stdout, p.stdout + p.stderr)

    def test_etrip_empty_fmt_fails(self):
        # BLOCKER 2 (etrip): upstream aborts when etrip.fmt is missing
        # (`test ! -s etrip.fmt`). A 0-byte etrip.fmt with all other
        # artifacts intact must FAIL.
        t = json.loads(self.table())
        t["ini:*etrip\n"]["write"] = {"etrip.fmt": ""}
        env = dict(self.env, FAKE_TABLE=json.dumps(t),
                   FAKE_DVI=self.dvi_table())
        p = subprocess.run(
            [sys.executable, RUN_PY, "--engine", self.engine, "--kind", "etrip",
             "--cache", self.cache, "--timeout", "60",
             "--pltotf", os.path.join(self.bindir, "fake-pltotf"),
             "--tftopl", os.path.join(self.bindir, "fake-tftopl"),
             "--dvitype", os.path.join(self.bindir, "fake-dvitype")],
            capture_output=True, text=True, env=env, stdin=subprocess.DEVNULL,
            timeout=300)
        self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
        self.assertIn("FAIL etrip pass 1 (initex)", p.stdout,
                      p.stdout + p.stderr)
        self.assertIn("etrip.fmt", p.stdout, p.stdout + p.stderr)

    def hidden_dvitype_run(self, extra_args=(), extra_env=None):
        # dvitype hidden from PATH; pltotf/tftopl still faked explicitly.
        env = dict(os.environ, FAKE_TABLE=self.table(),
                   PATH="/usr/bin:/bin")
        if extra_env:
            env.update(extra_env)
        return subprocess.run(
            [sys.executable, RUN_PY, "--engine", self.engine, "--kind", "trip",
             "--cache", self.cache, "--timeout", "60",
             "--pltotf", os.path.join(self.bindir, "fake-pltotf"),
             "--tftopl", os.path.join(self.bindir, "fake-tftopl")]
            + list(extra_args),
            capture_output=True, text=True, env=env, stdin=subprocess.DEVNULL,
            timeout=300)

    def test_trip_missing_dvitype_fails_by_default(self):
        # Upstream hard failure (`|| exit 1`): names dvitype, exit 1, and
        # the result line must not say PASS.
        p = self.hidden_dvitype_run()
        self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
        self.assertIn("FAIL", p.stdout)
        self.assertIn("dvitype", p.stdout)
        self.assertNotIn("result: PASS", p.stdout)

    def test_trip_missing_dvitype_allow_flag_incomplete(self):
        p = self.hidden_dvitype_run(extra_args=("--allow-missing-tools",))
        self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
        self.assertIn("SKIP dvitype", p.stdout)
        self.assertIn("INCOMPLETE", p.stdout)
        self.assertIn("PASS with skips is NOT a trip pass", p.stdout)
        self.assertNotIn("result: PASS", p.stdout)

    def test_trip_corrupt_dvi_hidden_dvitype_no_exit_zero(self):
        # A DVI-corrupting engine with dvitype hidden must not exit 0
        # without the flag (missing helper is FAIL); with dvitype present
        # the corruption itself is caught as FAIL trip.typ.
        tdir = os.path.join(self.cache, "triptrap")
        tampered = os.path.join(self.tmp, "tampered.typ")
        shutil.copy(os.path.join(tdir, "trip.typ"), tampered)
        with open(tampered, "a") as f:
            f.write("INJECTED DVI LINE\n")
        dvi = json.loads(self.dvi_table())
        dvi["trip.dvi"] = tampered
        hidden = self.hidden_dvitype_run(
            extra_env={"FAKE_DVI": json.dumps(dvi)})
        self.assertNotEqual(hidden.returncode, 0,
                            hidden.stdout + hidden.stderr)
        env = dict(self.env, FAKE_TABLE=self.table(),
                   FAKE_DVI=json.dumps(dvi))
        p = subprocess.run(
            [sys.executable, RUN_PY, "--engine", self.engine, "--kind", "trip",
             "--cache", self.cache, "--timeout", "60",
             "--pltotf", os.path.join(self.bindir, "fake-pltotf"),
             "--tftopl", os.path.join(self.bindir, "fake-tftopl"),
             "--dvitype", os.path.join(self.bindir, "fake-dvitype")],
            capture_output=True, text=True, env=env, stdin=subprocess.DEVNULL,
            timeout=300)
        self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
        self.assertIn("FAIL trip.typ (filtered)", p.stdout)
        self.assertIn("INJECTED DVI LINE", p.stdout)

    def test_dead_engine_fails(self):
        failer = os.path.join(self.bindir, "failer")
        with open(failer, "w") as f:
            f.write("#!/bin/sh\nexit 3\n")
        os.chmod(failer, 0o755)
        env = dict(self.env, FAKE_TABLE=self.table(),
                   FAKE_DVI=self.dvi_table())
        p = subprocess.run(
            [sys.executable, RUN_PY, "--engine", failer, "--kind", "trip",
             "--cache", self.cache, "--timeout", "60",
             "--pltotf", os.path.join(self.bindir, "fake-pltotf"),
             "--tftopl", os.path.join(self.bindir, "fake-tftopl"),
             "--dvitype", os.path.join(self.bindir, "fake-dvitype")],
            capture_output=True, text=True, env=env, stdin=subprocess.DEVNULL,
            timeout=300)
        self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
        self.assertIn("FAIL", p.stdout)

    def test_etrip_perfect_engine_passes(self):
        p = self.run_cli("etrip")
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        self.assertIn("result: PASS", p.stdout)
        self.assertNotIn("FAIL", p.stdout)

    def hidden_dvitype_etrip_run(self, table_json, extra_args=()):
        # dvitype hidden from PATH; pltotf/tftopl still faked explicitly.
        env = dict(os.environ, FAKE_TABLE=table_json,
                   PATH="/usr/bin:/bin")
        return subprocess.run(
            [sys.executable, RUN_PY, "--engine", self.engine, "--kind", "etrip",
             "--cache", self.cache, "--timeout", "60",
             "--pltotf", os.path.join(self.bindir, "fake-pltotf"),
             "--tftopl", os.path.join(self.bindir, "fake-tftopl")]
            + list(extra_args),
            capture_output=True, text=True, env=env, stdin=subprocess.DEVNULL,
            timeout=300)

    def test_etrip_fot_compared_without_dvitype(self):
        # Terminal output (.fot) is emitted by the engine directly and
        # needs no dvitype: a corrupted compat-phase fot must FAIL even
        # when dvitype is hidden from PATH.
        tdir = os.path.join(self.cache, "triptrap")
        tampered = os.path.join(self.tmp, "tampered-ctrip.fot")
        shutil.copy(os.path.join(tdir, "trip.fot"), tampered)
        with open(tampered, "a") as f:
            f.write("INJECTED FOT LINE\n")
        t = json.loads(self.table())
        t["run: &trip  trip \n"]["stdout"] = tampered
        p = self.hidden_dvitype_etrip_run(
            json.dumps(t), extra_args=("--allow-missing-tools",))
        self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
        self.assertIn("FAIL ctrip.fot (filtered)", p.stdout,
                      p.stdout + p.stderr)
        self.assertIn("INJECTED FOT LINE", p.stdout)

    def test_xflt_attack_probe_preserved(self):
        # BLOCKER 1: content smuggled between the filter1 anchors on
        # separate lines must NOT be swallowed. The old DOTALL rule
        # collapsed both sides to the same bytes (false PASS).
        sys.path.insert(0, os.path.dirname(RUN_PY))
        import run as r
        self.assertNotEqual(r.xflt(ATTACK_FOO), r.xflt(ATTACK_BAR))
        self.assertIn(b"FOO", r.xflt(ATTACK_FOO))
        self.assertIn(b"BAR", r.xflt(ATTACK_BAR))

    def test_xflt_single_line_span_still_collapses(self):
        # The same-line `... inside a group at level 1) FOO bottom level`
        # shape is what upstream's sed handles on every platform; it keeps
        # collapsing (before and after the fix).
        sys.path.insert(0, os.path.dirname(RUN_PY))
        import run as r
        self.assertEqual(r.xflt(SINGLE_LINE), SINGLE_LINE_COLLAPSED)

    def test_xflt_real_shape_collapses(self):
        # Pin: the real e-TeX group-trace block shape MUST collapse, or
        # the real etrip oracle goes red. c side via etrip_filter (filter
        # only, as upstream), x side via xflt.
        sys.path.insert(0, os.path.dirname(RUN_PY))
        import run as r
        xprobe = REAL_XFOT_SPAN + REAL_XFOT_AFTER
        cprobe = (b"(end occurred inside a group at level 1)\n"
                  + REAL_XFOT_AFTER)
        self.assertEqual(r.xflt(xprobe), r.etrip_filter(cprobe))

    def test_xflt_etrip_cli_catches_middle_difference(self):
        # BLOCKER 1 end to end: compat fot carries FOO between the
        # anchors, extended fot carries BAR. Must FAIL xtrip.fot.
        tdir = os.path.join(self.cache, "triptrap")
        with open(os.path.join(tdir, "trip.fot"), "wb") as f:
            f.write(ATTACK_FOO)
        bar = os.path.join(self.tmp, "bar.fot")
        with open(bar, "wb") as f:
            f.write(ATTACK_BAR)
        t = json.loads(self.table())
        t["run:&trip \\toksdef\\tokens=0 \\input trip\n"]["stdout"] = bar
        env = dict(self.env, FAKE_TABLE=json.dumps(t),
                   FAKE_DVI=self.dvi_table())
        p = subprocess.run(
            [sys.executable, RUN_PY, "--engine", self.engine, "--kind",
             "etrip", "--cache", self.cache, "--timeout", "60",
             "--pltotf", os.path.join(self.bindir, "fake-pltotf"),
             "--tftopl", os.path.join(self.bindir, "fake-tftopl"),
             "--dvitype", os.path.join(self.bindir, "fake-dvitype")],
            capture_output=True, text=True, env=env, stdin=subprocess.DEVNULL,
            timeout=300)
        self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
        self.assertIn("FAIL xtrip.fot (filtered)", p.stdout,
                      p.stdout + p.stderr)

    def test_texmfcnf_leak_shim_fails(self):
        # BLOCKER 2: a shim doing no TeX work that just copies the
        # expected files out of $TEXMFCNF must FAIL (it PASSed while
        # TEXMFCNF pointed at the cache holding the expected outputs).
        leak = os.path.join(self.bindir, "leak-engine")
        with open(leak, "w") as f:
            f.write(LEAK_ENGINE)
        os.chmod(leak, 0o755)
        env = dict(self.env, FAKE_TABLE=self.table(),
                   FAKE_DVI=self.dvi_table())
        p = subprocess.run(
            [sys.executable, RUN_PY, "--engine", leak, "--kind", "trip",
             "--cache", self.cache, "--timeout", "60",
             "--pltotf", os.path.join(self.bindir, "fake-pltotf"),
             "--tftopl", os.path.join(self.bindir, "fake-tftopl"),
             "--dvitype", os.path.join(self.bindir, "fake-dvitype")],
            capture_output=True, text=True, env=env, stdin=subprocess.DEVNULL,
            timeout=300)
        self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
        self.assertIn("FAIL", p.stdout, p.stdout + p.stderr)
        self.assertNotIn("result: PASS", p.stdout)

    def test_cnf_staging_holds_only_texmf_cnf(self):
        # BLOCKER 2 pin: the directory TEXMFCNF points at holds ONLY
        # texmf.cnf (byte-identical to the cached one), never the
        # expected outputs.
        sys.path.insert(0, os.path.dirname(RUN_PY))
        import run as r
        tdir = os.path.join(self.cache, "triptrap")
        d = r.stage_cnfdir(os.path.join(tdir, "texmf.cnf"))
        try:
            self.assertEqual(sorted(os.listdir(d)), ["texmf.cnf"])
            with open(os.path.join(d, "texmf.cnf"), "rb") as f:
                staged = f.read()
            with open(os.path.join(tdir, "texmf.cnf"), "rb") as f:
                orig = f.read()
            self.assertEqual(staged, orig)
        finally:
            shutil.rmtree(d, ignore_errors=True)

    def test_accepted_difference_hidden_injection_matches_sed(self):
        # WARNING pin (upstream-faithful, do NOT change): an injected
        # line matching a deleted/normalised pattern is invisible, exactly
        # like upstream's `/pattern/d`. A memory-stat wording difference
        # is normalised away on both sides too.
        sys.path.insert(0, os.path.dirname(RUN_PY))
        import run as r
        base = (b"line one\n1372 strings of total length 24645\nline two\n")
        injected = (b"line one\n** &trip  trip INJECTED\n"
                    b"9999 strings of total length 88888\nline two\n")
        self.assertEqual(r.etrip_filter(injected), r.etrip_filter(base))
        if not shutil.which("sed"):
            self.skipTest("no sed on PATH")
        etrip_test, _, _, _ = upstream_paths()
        if etrip_test is None:
            self.skipTest("pinned upstream checkout not cached")
        ef = parse_upstream_sed_scripts(etrip_test)
        self.assertIn("filter", ef)
        self.assertEqual(r.etrip_filter(injected),
                         sed_filter(injected, ("filter", ef["filter"])))

    def test_tool_filters_byte_identical_to_upstream_sed(self):
        # The tool's filters must agree byte for byte with `sed -f` on
        # the real upstream filter scripts: cached expected files, the
        # real x-side group-trace shape, and the single-line shape.
        if not shutil.which("sed"):
            self.skipTest("no sed on PATH")
        etrip_test, trip_test, tdir, edir = upstream_paths()
        if etrip_test is None:
            self.skipTest("pinned upstream checkout not cached")
        sys.path.insert(0, os.path.dirname(RUN_PY))
        import run as r
        ef = parse_upstream_sed_scripts(etrip_test)
        tf = parse_upstream_sed_scripts(trip_test)
        self.assertIn("filter1", ef)  # proves the real script was read
        for name in ("trip.fot", "trip.typ"):
            with open(os.path.join(tdir, name), "rb") as f:
                data = f.read()
            self.assertEqual(r.trip_filter(data),
                             sed_filter(data, ("filter", tf["filter"])),
                             name)
            self.assertEqual(r.etrip_filter(data),
                             sed_filter(data, ("filter", ef["filter"])),
                             name)
        for name in ("etrip.fot", "etrip.typ"):
            with open(os.path.join(edir, name), "rb") as f:
                data = f.read()
            self.assertEqual(r.etrip_filter(data),
                             sed_filter(data, ("filter", ef["filter"])),
                             name)
        xprobe = REAL_XFOT_SPAN + REAL_XFOT_AFTER
        self.assertEqual(
            r.xflt(xprobe),
            sed_filter(xprobe, ("filter", ef["filter"]),
                       ("filter1", ef["filter1"])))
        self.assertEqual(
            r.xflt(SINGLE_LINE),
            sed_filter(SINGLE_LINE, ("filter", ef["filter"]),
                       ("filter1", ef["filter1"])))

    def test_engine_timeout_fails(self):
        sleeper = os.path.join(self.bindir, "sleeper")
        with open(sleeper, "w") as f:
            f.write("#!/bin/sh\nsleep 30\n")
        os.chmod(sleeper, 0o755)
        p = subprocess.run(
            [sys.executable, RUN_PY, "--engine", sleeper, "--kind", "trip",
             "--cache", self.cache, "--timeout", "1",
             "--pltotf", os.path.join(self.bindir, "fake-pltotf"),
             "--tftopl", os.path.join(self.bindir, "fake-tftopl"),
             "--dvitype", os.path.join(self.bindir, "fake-dvitype")],
            capture_output=True, text=True, env=self.env,
            stdin=subprocess.DEVNULL, timeout=300)
        self.assertEqual(p.returncode, 1, p.stdout + p.stderr)
        self.assertIn("timed out", p.stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)
