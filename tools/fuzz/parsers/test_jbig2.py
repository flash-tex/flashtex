#!/usr/bin/env python3
"""Unit tests for tools/fuzz/parsers/jbig2.py. Stdlib unittest only.

Run as `python3 -m unittest discover -s tools/fuzz/parsers` from the repo
root. Engines are small shell scripts written to a temp dir; the real
candidate and oracle are never used.
"""
import importlib.util
import json
import os
import random
import shutil
import struct
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location(
    "jbig2fuzz", os.path.join(HERE, "jbig2.py"))
jb = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(jb)

# Fake engines: each writes job.log (and job.pdf when it "succeeds") in
# its working directory, as pdfTeX would.
LOG = "printf 'banner\\n**job.tex\\n(./job.tex [1] )\\n%s\\n' > job.log\n"
OK_BODY = LOG % "Output written on job.pdf" + "printf PDF > job.pdf\nexit 0\n"
OTHER_PDF_BODY = LOG % "Output written on job.pdf" + \
    "printf PDF2 > job.pdf\nexit 0\n"
OTHER_LOG_BODY = LOG % "!pdfTeX error: x (file ./fuzz.jbig2): boom" + \
    "exit 1\n"
CRASH_BODY = ("echo \"thread 'main' panicked at 'jb2 boom', "
              "src/pdftex/writejbig2.rs:42:7\" >&2\nexit 101\n")
SEGV_BODY = "kill -SEGV $$\n"
SLEEP_BODY = "sleep 30\nexit 0\n"
FAIL_BODY = "exit 1\n"


def make_engine(tmpdir, name, body):
    path = os.path.join(tmpdir, name)
    with open(path, "w") as fh:
        fh.write("#!/bin/sh\n" + body)
    os.chmod(path, 0o755)
    return path


def stored(out, ext=".jbig2"):
    found = []
    for root, _dirs, files in os.walk(out):
        found.extend(os.path.join(root, f) for f in files
                     if f.endswith(ext))
    return sorted(found)


class FormatTest(unittest.TestCase):
    def test_seeds_start_with_the_signature(self):
        for name, doc in jb.seed_docs():
            data = jb.build(doc)
            self.assertEqual(data[:8], jb.SIG, name)

    def test_file_header_flags_and_page_count(self):
        seq = jb.build(jb.Doc([], sequential=True, pages=3))
        self.assertEqual(seq[8], 0x01)
        self.assertEqual(struct.unpack(">I", seq[9:13])[0], 3)
        ra = jb.build(jb.Doc([], sequential=False, pages=None))
        self.assertEqual(ra[8], 0x02)
        self.assertEqual(len(ra), 9)

    def test_reference_width_follows_the_segment_number(self):
        self.assertEqual(jb.ref_width(256), 1)
        self.assertEqual(jb.ref_width(257), 2)
        self.assertEqual(jb.ref_width(65536), 2)
        self.assertEqual(jb.ref_width(65537), 4)
        h = jb.seg_header(jb.Seg(300, 6, 1, b"xy", refs=[7]))
        # number(4) flags(1) count(1) ref(2) page(1) length(4)
        self.assertEqual(len(h), 13)
        self.assertEqual(h[6:8], b"\x00\x07")
        self.assertEqual(struct.unpack(">I", h[9:13])[0], 2)

    def test_long_referred_to_form(self):
        s = jb.Seg(1, 6, 1, refs=[0] * 9)
        h = jb.seg_header(s)
        count = struct.unpack(">I", h[5:9])[0]
        self.assertEqual(count >> 29, 7)
        self.assertEqual(count & 0x1FFFFFFF, 9)
        # ceil((9 + 1) / 8) = 2 retention bytes, 9 one-byte references,
        # a page byte and the length
        self.assertEqual(len(h), 9 + 2 + 9 + 1 + 4)

    def test_long_page_association(self):
        h = jb.seg_header(jb.Seg(1, 48, 300, b""))
        self.assertTrue(h[4] & 0x40)
        self.assertEqual(struct.unpack(">I", h[6:10])[0], 300)

    def test_random_access_puts_headers_first(self):
        doc = jb.Doc([jb.Seg(0, 48, 1, b"AAAA"), jb.Seg(1, 49, 1, b"BB")],
                     sequential=False, pages=1)
        data = jb.build(doc)
        self.assertTrue(data.endswith(b"AAAABB"))


class MutateTest(unittest.TestCase):
    def test_deterministic(self):
        seeds = jb.seed_docs()
        for _name, doc in seeds:
            rng = random.Random(42)
            first = [jb.mutate(doc, rng) for _ in range(60)]
            rng = random.Random(42)
            second = [jb.mutate(doc, rng) for _ in range(60)]
            self.assertEqual(first, second)

    def test_mutation_does_not_touch_the_seed(self):
        _name, doc = jb.seed_docs()[1]
        before = jb.build(doc)
        rng = random.Random(3)
        for _ in range(100):
            jb.mutate(doc, rng)
        self.assertEqual(jb.build(doc), before)

    def test_many_kinds_of_mutation(self):
        rng = random.Random(1)
        kinds = set()
        for i in range(400):
            _name, doc = jb.seed_docs()[i % 3]
            kinds.add(jb.mutate(doc, rng)[1].split("[")[0].split("=")[0]
                      .split("+")[0])
        for k in ("number", "type", "page", "length", "del", "dup", "swap"):
            self.assertIn(k, kinds)

    def test_jobs(self):
        rng = random.Random(9)
        bodies = [jb.job_body(rng) for _ in range(200)]
        self.assertTrue(any("page 2" in b for _m, b in bodies))
        self.assertTrue(any(m == 3 for m, _b in bodies))
        for _m, b in bodies:
            self.assertIn("{fuzz.jbig2}", b)


class LogTest(unittest.TestCase):
    def test_normalised_log(self):
        text = ("This is pdfTeX\n**job.tex\n(./job.tex\n"
                "!pdfTeX error: /x/pdftex (file ./a.jbig2): no\n"
                "Here is how much of TeX's memory you used:\n 5 strings\n")
        self.assertEqual(jb.normalised_log(text),
                         "**job.tex(./job.tex"
                         "!pdfTeX error: PROG (file ./a.jbig2): no")


class ClassifyTest(unittest.TestCase):
    def test_crash_mode(self):
        self.assertEqual(jb.classify_crash(None), "hang")
        self.assertEqual(jb.classify_crash(-11), "crash")
        self.assertEqual(jb.classify_crash(101), "crash")
        self.assertEqual(jb.classify_crash(0), "ok")
        self.assertEqual(jb.classify_crash(1), "graceful-error")
        self.assertEqual(jb.classify_crash(-25), "output-flood")

    def test_differential(self):
        ok = (0, b"", "**job a", b"PDF")
        self.assertEqual(jb.classify_diff(ok, ok), "equal")
        self.assertEqual(jb.classify_diff(ok, (0, b"", "**job a", b"P2")),
                         "diverge")
        self.assertEqual(jb.classify_diff(ok, (1, b"", "**job a", b"PDF")),
                         "diverge")
        self.assertEqual(jb.classify_diff(ok, (0, b"", "**job b", b"PDF")),
                         "diverge")
        self.assertEqual(jb.classify_diff((-11, b"", "", None), ok),
                         "candidate-crash")
        self.assertEqual(jb.classify_diff((101, b"", "", None), ok),
                         "candidate-crash")
        self.assertEqual(jb.classify_diff(ok, (-11, b"", "", None)),
                         "oracle-crash")
        # exit 101 from the oracle is not a crash (run.py's rule)
        self.assertEqual(jb.classify_diff(ok, (101, b"", "", None)),
                         "diverge")
        self.assertEqual(jb.classify_diff((-6, b"", "", None),
                                          (-11, b"", "", None)),
                         "both-crash")
        self.assertEqual(jb.classify_diff((None, b"", "", None), ok),
                         "timeout")
        self.assertEqual(jb.classify_diff((None, b"", "", None),
                                          (None, b"", "", None)),
                         "both-hang")
        self.assertEqual(jb.classify_diff((-25, b"", "", None), ok),
                         "output-flood")

    def test_signatures(self):
        ok = (0, b"", "**job a", b"PDF")
        self.assertEqual(jb.signature("timeout", (None, b"", "", None), ok),
                         "timeout:candidate")
        self.assertEqual(jb.signature("timeout", ok, (None, b"", "", None)),
                         "timeout:oracle")
        self.assertTrue(jb.signature("oracle-crash", ok,
                                     (-11, b"", "", None))
                        .startswith("oracle-crash:signal:"))
        a = jb.signature("diverge", (0, b"", "**job page 1 x", b"P"),
                         (0, b"", "**job page 1 y", b"P"))
        b = jb.signature("diverge", (0, b"", "**job page 2 x", b"P"),
                         (0, b"", "**job page 2 y", b"P"))
        self.assertEqual(a, b)  # digits do not split signatures
        self.assertTrue(a.startswith("diverge:"))


class RunTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="jbig2fuzz-test-")
        self.out = os.path.join(self.tmp, "out")

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_crash_mode_stores_one_crash(self):
        cand = make_engine(self.tmp, "cand", CRASH_BODY)
        counts = jb.run_fuzz(cand, self.out, 5, 1, 10)
        self.assertEqual(counts["crash"], 5)
        files = stored(self.out)
        self.assertEqual(len(files), 1)  # deduped by panic site
        with open(files[0][:-len(".jbig2")] + ".json") as fh:
            info = json.load(fh)
        self.assertEqual(info["signature"],
                         "panic:src/pdftex/writejbig2.rs:42")
        self.assertIn("{fuzz.jbig2}", info["job"])

    def test_crash_mode_graceful_is_not_stored(self):
        cand = make_engine(self.tmp, "cand", FAIL_BODY)
        counts = jb.run_fuzz(cand, self.out, 4, 1, 10)
        self.assertEqual(counts["graceful-error"], 4)
        self.assertEqual(stored(self.out), [])

    def test_differential_equal(self):
        a = make_engine(self.tmp, "a", OK_BODY)
        b = make_engine(self.tmp, "b", OK_BODY)
        counts = jb.run_fuzz(a, self.out, 4, 1, 10, oracle=b)
        self.assertEqual(counts["equal"], 4)
        self.assertEqual(stored(self.out), [])

    def test_differential_diverge_pdf_and_log(self):
        a = make_engine(self.tmp, "a", OK_BODY)
        b = make_engine(self.tmp, "b", OTHER_PDF_BODY)
        counts = jb.run_fuzz(a, self.out, 3, 1, 10, oracle=b)
        self.assertEqual(counts["diverge"], 3)
        self.assertEqual(len(stored(self.out)), 1)
        out2 = os.path.join(self.tmp, "out2")
        c = make_engine(self.tmp, "c", OTHER_LOG_BODY)
        counts = jb.run_fuzz(a, out2, 3, 1, 10, oracle=c)
        self.assertEqual(counts["diverge"], 3)
        self.assertEqual(len(stored(out2)), 1)

    def test_differential_oracle_crash(self):
        a = make_engine(self.tmp, "a", OK_BODY)
        b = make_engine(self.tmp, "b", SEGV_BODY)
        counts = jb.run_fuzz(a, self.out, 2, 1, 10, oracle=b)
        self.assertEqual(counts["oracle-crash"], 2)
        files = stored(self.out)
        self.assertEqual(len(files), 1)
        self.assertIn(os.sep + "oracle-crash" + os.sep, files[0])

    def test_differential_timeout(self):
        a = make_engine(self.tmp, "a", SLEEP_BODY)
        b = make_engine(self.tmp, "b", OK_BODY)
        counts = jb.run_fuzz(a, self.out, 1, 1, 1, oracle=b)
        self.assertEqual(counts["timeout"], 1)

    def test_known_signatures_are_not_stored_again(self):
        cand = make_engine(self.tmp, "cand", CRASH_BODY)
        jb.run_fuzz(cand, self.out, 2, 1, 10)
        jb.run_fuzz(cand, self.out, 2, 2, 10)
        self.assertEqual(len(stored(self.out)), 1)

    def test_main_rejects_missing_engine(self):
        rc = jb.main(["--candidate", os.path.join(self.tmp, "nope"),
                      "--iterations", "1", "--seed", "1",
                      "--out", self.out])
        self.assertEqual(rc, 2)

    def test_candidate_only_env(self):
        old = os.environ.get("FLASHTEX_POOL")
        os.environ["FLASHTEX_POOL"] = "/x/pdftex.pool"
        try:
            self.assertEqual(jb.engine_env(True)["FLASHTEX_POOL"],
                             "/x/pdftex.pool")
            self.assertNotIn("FLASHTEX_POOL", jb.engine_env(False))
            self.assertEqual(jb.engine_env(False)["SOURCE_DATE_EPOCH"], "0")
        finally:
            if old is None:
                os.environ.pop("FLASHTEX_POOL", None)
            else:
                os.environ["FLASHTEX_POOL"] = old


if __name__ == "__main__":
    unittest.main()
