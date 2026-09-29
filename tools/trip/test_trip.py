#!/usr/bin/env python3
"""Unit + fake-engine tests for tools/trip/run.py (stdlib only).

A 'perfect engine' fake (dispatched on --ini/stdin) must PASS; the same fake
serving one changed line must FAIL with a readable first-difference report.
Run:  python3 tools/trip/test_trip.py
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest

RUN_PY = os.path.join(os.path.dirname(os.path.abspath(__file__)), "run.py")

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
            "run:*etrip\n": {
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
