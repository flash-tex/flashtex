"""Tests for tools/web2rust/tools/pascal_tokens.py: the oracle tokenizer that
normalises a TANGLE-produced Pascal file into the one-token-per-line form
`web2rust --emit-pascal` writes, so the two can be diffed. Pure logic only:
no network, TeX Live, GPU, display, or built engine. Stdlib unittest.

    python3 -m unittest discover -s tools/web2rust -p 'test_*.py' -v
"""

import contextlib
import io
import os
import sys
import tempfile
import unittest
import warnings
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "tools"))

import pascal_tokens  # noqa: E402


def toks(src):
    return list(pascal_tokens.tokens(src))


class Identifiers(unittest.TestCase):

    def test_lowercase_and_drop_underscores(self):
        self.assertEqual(toks("Begin"), ["i begin"])
        self.assertEqual(toks("TeX_Init"), ["i texinit"])
        self.assertEqual(toks("x1_y2"), ["i x1y2"])
        self.assertEqual(toks("already"), ["i already"])


class Numbers(unittest.TestCase):

    def test_integers_drop_leading_zeros(self):
        self.assertEqual(toks("007"), ["n 7"])
        self.assertEqual(toks("0"), ["n 0"])
        self.assertEqual(toks("42"), ["n 42"])

    def test_fraction_makes_a_verbatim_real(self):
        self.assertEqual(toks("3.14"), ["r 3.14"])
        self.assertEqual(toks("0.5"), ["r 0.5"])
        # unlike integers, reals keep their exact spelling
        self.assertEqual(toks("3.50"), ["r 3.50"])

    def test_exponent_makes_a_verbatim_real(self):
        self.assertEqual(toks("1e10"), ["r 1e10"])
        self.assertEqual(toks("10E5"), ["r 10E5"])
        self.assertEqual(toks("1E+5"), ["r 1E+5"])
        self.assertEqual(toks("2.5e-2"), ["r 2.5e-2"])

    def test_range_operator_beats_fraction(self):
        self.assertEqual(toks("1..10"), ["n 1", "o ..", "n 10"])


class Strings(unittest.TestCase):

    def test_contents_with_doubled_quotes_folded(self):
        self.assertEqual(toks("'hi'"), ["s hi"])
        self.assertEqual(toks("'don''t'"), ["s don't"])
        self.assertEqual(toks("''"), ["s "])


class Comments(unittest.TestCase):

    def test_comments_vanish_without_needing_separators(self):
        self.assertEqual(toks("{ hello }"), [])
        self.assertEqual(toks("a{b}c"), ["i a", "i c"])

    def test_brace_in_a_comment_string_does_not_close_it(self):
        self.assertEqual(toks("{ '}' } x"), ["i x"])
        self.assertEqual(toks("{ 'it''s' } y"), ["i y"])


class Operators(unittest.TestCase):

    def test_two_character_operators_lex_as_one(self):
        for op in (":=", "<=", ">=", "<>", ".."):
            with self.subTest(op=op):
                self.assertEqual(toks(f"a{op}b"), ["i a", f"o {op}", "i b"])

    def test_single_character_fallbacks(self):
        self.assertEqual(toks("a:b"), ["i a", "o :", "i b"])
        self.assertEqual(toks("a<b"), ["i a", "o <", "i b"])
        self.assertEqual(toks("a.b"), ["i a", "o .", "i b"])
        self.assertEqual(toks("(a+b);"),
                         ["o (", "i a", "o +", "i b", "o )", "o ;"])


class EndToEnd(unittest.TestCase):

    def test_main_writes_one_token_per_line(self):
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, "in.p")
            with open(p, "w", encoding="latin-1", newline="") as f:
                f.write("Begin x:=1 End.\n")
            out = io.StringIO()
            with mock.patch.object(sys, "argv", ["pascal_tokens.py", p]), \
                    contextlib.redirect_stdout(out):
                # the tool's own open() is never closed; that is its code,
                # which this slice must not touch, so silence the warning here
                with warnings.catch_warnings():
                    warnings.simplefilter("ignore", ResourceWarning)
                    pascal_tokens.main()
            self.assertEqual(
                out.getvalue().splitlines(),
                ["i begin", "i x", "o :=", "n 1", "i end", "o ."])
            self.assertTrue(out.getvalue().endswith("\n"))


if __name__ == "__main__":
    unittest.main()
