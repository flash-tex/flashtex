"""Unit tests for the harness's pure parts (tokenizer, classification, PGM
compare). No producer, oracle or rasterizer is run here.

    python3 -m unittest discover -s tools/real-world-corpus -p 'test_*.py' -v
"""

import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import run as harness  # noqa: E402


class TokenizerTests(unittest.TestCase):
    def names(self, occ, kind=None):
        return [o["name"] for o in occ if kind is None or o["kind"] == kind]

    def test_control_sequences_environments_packages(self):
        src = "\\documentclass[11pt]{article}\n\\usepackage[margin=1in]{geometry}\n\\usepackage{amsmath,amssymb}\n" \
              "\\begin{document}\n\\section*{A} % \\comment{ignored}\nx $a\\in b$ \\\\\n\\begin{enumerate}\\item y\\end{enumerate}\n\\end{document}\n"
        occ = harness.tokenize(src, "main.tex")
        self.assertIn("article", self.names(occ, "class"))
        self.assertIn("article[11pt]", self.names(occ, "class-option"))
        self.assertIn("geometry", self.names(occ, "package"))
        self.assertIn("geometry[margin=1in]", self.names(occ, "package-option"))
        self.assertEqual(self.names(occ, "package").count("amssymb"), 1)
        self.assertIn("enumerate", self.names(occ, "env"))
        self.assertIn("document", self.names(occ, "env"))
        self.assertIn("\\section*", self.names(occ, "cs"))
        self.assertNotIn("\\comment", self.names(occ, "cs"))
        self.assertIn("\\\\", self.names(occ, "cs"))
        self.assertEqual(self.names(occ, "math").count("$"), 2)
        inmath = [o for o in occ if o["name"] == "\\in"]
        self.assertTrue(inmath and inmath[0]["math"])

    def test_byte_offsets_are_utf8(self):
        src = "é\\alpha"
        occ = harness.tokenize(src, "m.tex")
        alpha = [o for o in occ if o["name"] == "\\alpha"][0]
        self.assertEqual((alpha["start"], alpha["end"]), (2, 8))

    def test_user_defined_macros_are_flagged(self):
        src = "\\newcommand{\\R}{\\mathbb{R}}\n\\DeclareMathOperator{\\tr}{tr}\n$\\R$ $\\tr$ $\\mathbb{Q}$"
        occ = harness.tokenize(src, "m.tex")
        self.assertTrue(all(o.get("user_defined") for o in occ if o["name"] in ("\\R", "\\tr")))
        self.assertFalse(any(o.get("user_defined") for o in occ if o["name"] == "\\mathbb"))


class ClassificationTests(unittest.TestCase):
    def test_named_and_covered(self):
        src = "\\problem{1}{4} $a\\in b$ \\begin{center}x\\end{center}"
        occ = harness.tokenize(src, "m.tex")
        diags = [
            {"severity": "error", "code": "compiler", "message": "\\in is not supported in math mode",
             "source": {"path": "m.tex", "start_byte": src.index("\\in"), "end_byte": src.index("\\in") + 3}},
            {"severity": "error", "code": "compiler", "message": "\\subsection requires a braced argument",
             "source": {"path": "m.tex", "start_byte": 0, "end_byte": 8}},
            {"severity": "warning", "code": "compiler", "message": "environment 'center' is not implemented; its body is typeset as plain text",
             "source": {"path": "m.tex", "start_byte": src.index("\\begin"), "end_byte": src.index("\\end")}},
        ]
        table = harness.classify(occ, diags)
        self.assertEqual(table[("cs", "\\in")]["status"], "unsupported")
        self.assertEqual(table[("env", "center")]["status"], "unsupported")
        # the macro call whose expansion produced the diagnostic is 'recovered'
        self.assertEqual(table[("cs", "\\problem")]["status"], "recovered")
        self.assertIn("\\subsection requires a braced argument", table[("cs", "\\problem")]["diagnostics"])
        self.assertEqual(table[("math", "$")]["status"], "no-diagnostic")

    def test_diag_key_and_owner(self):
        self.assertEqual(harness.diag_key("overfull line: 4.76pt too wide"), "overfull line: <N>pt too wide")
        self.assertIn("crates/compiler", harness.owner_for({"message": "\\foo is not supported in the document preamble"}))
        self.assertIn("math-layout", harness.owner_for({"code": "compiler", "message": "\\in is not supported in math mode"}))
        self.assertTrue(harness.owner_for({"code": "overfull_hbox", "message": "overfull line"}).startswith("crates/render-pipeline"))
        self.assertIn(("package", "amssymb"), harness.named_constructs("packages amsmath, amssymb are recognised but not implemented"))


class PgmTests(unittest.TestCase):
    def write(self, d, name, w, h, pixels, comment=True):
        p = os.path.join(d, name)
        with open(p, "wb") as f:
            f.write(b"P5\n" + (b"# made by test\n" if comment else b"") + f"{w} {h}\n255\n".encode() + bytes(pixels))
        return p

    def test_compare_pages(self):
        with tempfile.TemporaryDirectory() as d:
            a = self.write(d, "a.pgm", 3, 2, [255, 0, 255, 0, 255, 0])
            b = self.write(d, "b.pgm", 3, 2, [255, 0, 200, 0, 255, 0], comment=False)
            c = self.write(d, "c.pgm", 2, 2, [0, 0, 0, 0])
            pages = harness.compare_pages([a, a, a], [b, c])
            self.assertEqual(pages[0]["result"], "compared")
            self.assertEqual(pages[0]["differing"], 1)
            self.assertEqual(pages[0]["max_delta"], 55)
            self.assertEqual(pages[0]["ink_pixels_reference"], 3)
            self.assertEqual(pages[1]["result"], "size_mismatch")
            self.assertEqual(pages[2]["result"], "missing_in_candidate")


if __name__ == "__main__":
    unittest.main()


class ReferenceConvergenceTests(unittest.TestCase):
    """A reference is only a reference once pdflatex has settled. Two passes
    used to be assumed; `hyperref-toc` needed three, and `lmodern-report`
    needed three while never printing "Rerun to get" -- so the loop is tested
    against both a late-settling document and one that never settles."""

    def _fixture(self, tmp):
        src = os.path.join(tmp, "src")
        os.makedirs(src)
        with open(os.path.join(src, "main.tex"), "w") as f:
            f.write("\\documentclass{article}\\begin{document}x\\end{document}\n")
        return {"id": "fake", "dir": src, "entry": "main.tex"}

    def _patch(self, tmp, bodies, logtext="", ):
        """Fake pdflatex: writes bodies[i] on pass i, then the last body forever."""
        calls = {"n": 0}

        def fake_run(cmd, stdin_bytes=None, env=None, timeout=300, cwd=None):
            i = min(calls["n"], len(bodies) - 1)
            calls["n"] += 1
            with open(os.path.join(cwd, "main.pdf"), "wb") as f:
                f.write(bodies[i])
            with open(os.path.join(cwd, "main.log"), "w") as f:
                f.write("Output written on main.pdf (1 page).\n" + logtext)
            return 0, b"", b"", 0.0, False

        harness.pdflatex_version = lambda texbin: (os.path.join(texbin, "pdflatex"), "fake pdfTeX")
        harness.run = fake_run
        harness.HERE = tmp
        return calls

    def setUp(self):
        self._saved = (harness.run, harness.pdflatex_version, harness.HERE)

    def tearDown(self):
        harness.run, harness.pdflatex_version, harness.HERE = self._saved

    def test_runs_until_the_pdf_stops_changing(self):
        with tempfile.TemporaryDirectory() as tmp:
            fx = self._fixture(tmp)
            calls = self._patch(tmp, [b"pass1", b"pass2", b"settled"])
            log = []
            rec = harness.generate_reference(fx, tmp, "reference.pdf", log)
            self.assertIsNotNone(rec, log)
            # three distinct passes, plus the one that proved pass 3 was stable
            self.assertEqual(calls["n"], 4)
            self.assertEqual(rec["converged_after_passes"], 4)
            self.assertIn("to convergence", rec["argv"][-1])
            with open(os.path.join(fx["dir"], "reference.pdf"), "rb") as f:
                self.assertEqual(f.read(), b"settled")

    def test_a_rerun_request_is_not_convergence(self):
        """Identical bytes still are not settled while pdflatex asks again."""
        with tempfile.TemporaryDirectory() as tmp:
            fx = self._fixture(tmp)
            self._patch(tmp, [b"same"], logtext="LaTeX Warning: Rerun to get cross-references right.\n")
            log = []
            self.assertIsNone(harness.generate_reference(fx, tmp, "reference.pdf", log))
            self.assertTrue(any("did not converge" in m for m in log), log)
            self.assertFalse(os.path.exists(os.path.join(fx["dir"], "reference.pdf")))
