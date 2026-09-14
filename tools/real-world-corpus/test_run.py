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
        self.assertIn("crates/compiler", harness.owner_for({"code": "unknown_command", "message": "\\alpah is not supported"}))
        self.assertIn("math-layout", harness.owner_for({"code": "unsupported_feature", "message": "\\in is not supported in math mode"}))
        self.assertIn("crates/compiler", harness.owner_for({"code": "syntax_error", "message": "unmatched '{'"}))
        self.assertIn("crates/compiler", harness.owner_for({"code": "recovered_input", "message": "included file not found"}))
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
