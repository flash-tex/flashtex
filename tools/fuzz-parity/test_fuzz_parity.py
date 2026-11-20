#!/usr/bin/env python3
"""Fixed-seed smoke test for tools/fuzz-parity (well under 30 s). MIT.

    python3 -m unittest tools/fuzz-parity/test_fuzz_parity.py
    FUZZ_PARITY_CANDIDATE=/path/to/flashtex-initex python3 tools/fuzz-parity/test_fuzz_parity.py

Without pdftex on PATH only the generator and ddmin tests run. With it,
the oracle is compared with itself on fixed seeds (the harness must call
two runs of one engine equal). With FUZZ_PARITY_CANDIDATE set, the
candidate is compared with the oracle on the same fixed seeds.
"""
import hashlib
import os
import shutil
import sys
import tempfile
import time
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import fuzz  # noqa: E402
import gen  # noqa: E402
import interact  # noqa: E402
import minimize  # noqa: E402

SMOKE_SEEDS = (0, 1, 2, 7)
INTERACT_CASES = (0, 3, 14)
# sha256 of gen.generate(0).render(): pins the generator, so a change to the
# grammar is a deliberate update of this value (old seeds then mean new
# documents; findings record the generator revision they came from).
SEED0_SHA = None


class GeneratorTest(unittest.TestCase):
    def test_deterministic(self):
        for s in SMOKE_SEEDS:
            self.assertEqual(gen.generate(s).render(), gen.generate(s).render())

    def test_seeds_differ(self):
        texts = {gen.generate(s).render() for s in range(20)}
        self.assertEqual(len(texts), 20)

    def test_shape(self):
        for s in range(50):
            d = gen.generate(s)
            t = d.render()
            if d.fmt:
                self.assertIn("\\begin{document}", t)
                self.assertTrue(t.rstrip().endswith("\\end{document}"))
            else:
                self.assertIn("\\input plain", t)
                self.assertTrue(t.rstrip().endswith("\\end"))
            self.assertEqual(t.count("%%FZ-CHUNK"), len(d.chunks))
            t.encode("latin-1")

    def test_seed0_pinned(self):
        h = hashlib.sha256(gen.generate(0).render().encode("latin-1")).hexdigest()
        if SEED0_SHA is not None:
            self.assertEqual(h, SEED0_SHA)

    def test_latex_split(self):
        d = gen.generate(5, latex=True)
        head, chunks, tail = minimize.split_doc(d.render())
        self.assertEqual(head, "")
        self.assertEqual(tail, gen.LATEX_TAIL)
        self.assertEqual(len(chunks), len(d.chunks) + 1)
        self.assertIn("\\documentclass", chunks[0])

    def test_split_body_roundtrip(self):
        d = gen.generate(3, latex=False)
        chunks = minimize.split_body(d.render())
        self.assertEqual(len(chunks), len(d.chunks) + 1)  # + preamble remainder


class DdminTest(unittest.TestCase):
    def test_finds_pair(self):
        items = list(range(40))
        out = minimize.ddmin(items, lambda xs: 7 in xs and 31 in xs)
        self.assertEqual(out, [7, 31])

    def test_single(self):
        self.assertEqual(minimize.ddmin(list("abcdef"), lambda xs: "d" in xs), ["d"])


class InteractCaseTest(unittest.TestCase):
    def test_deterministic(self):
        self.assertEqual(interact.case(5), interact.case(5))
        self.assertNotEqual(interact.case(5), interact.case(6))

    def test_shape(self):
        for n in range(50):
            doc, stdin = interact.case(n)
            self.assertTrue(doc.rstrip().endswith("\\end"))
            self.assertIsInstance(stdin, bytes)


ORACLE = shutil.which("pdftex")


@unittest.skipUnless(ORACLE, "pdftex (the oracle) not on PATH")
class EngineSmokeTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="fzp-test-")
        self.t0 = time.time()

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def _run(self, candidate):
        bad = []
        for s in SMOKE_SEEDS:
            work = os.path.join(self.tmp, str(s))
            r = fuzz.compare(gen.generate(s, latex=False).render(), candidate, ORACLE, work, 60)
            if r["cls"] != "equal":
                bad.append((s, r["cls"], r.get("sig")))
        self.assertEqual(bad, [])

    def test_oracle_self(self):
        self._run(ORACLE)
        self.assertLess(time.time() - self.t0, 30)

    @unittest.skipUnless(os.environ.get("FUZZ_PARITY_CANDIDATE"), "FUZZ_PARITY_CANDIDATE not set")
    def test_candidate(self):
        self._run(os.path.abspath(os.environ["FUZZ_PARITY_CANDIDATE"]))

    def _interact(self, candidate):
        oracle = os.path.realpath(ORACLE)
        bad = [(n, interact.compare(n, candidate, oracle, self.tmp)) for n in INTERACT_CASES]
        self.assertEqual([b for b in bad if b[1][0] != "equal"], [])

    def test_interact_oracle_self(self):
        self._interact(os.path.realpath(ORACLE))

    @unittest.skipUnless(os.environ.get("FUZZ_PARITY_CANDIDATE"), "FUZZ_PARITY_CANDIDATE not set")
    def test_interact_candidate(self):
        self._interact(os.path.abspath(os.environ["FUZZ_PARITY_CANDIDATE"]))


if __name__ == "__main__":
    unittest.main()
