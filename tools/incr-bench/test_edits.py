#!/usr/bin/env python3
"""Tests for edits.py (unittest): run `python3 -m unittest discover -s tools/incr-bench -p 'test_*.py'`."""
import os
import random
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import edits  # noqa: E402

HEAD = b"\\documentclass{article}\n\\begin{document}\n"
TAIL = b"\\end{document}\n"


def doc(body):
    return HEAD + body + TAIL


def at(src, word):
    return src.index(word)


PLAIN = doc(b"First paragraph has some words\nand a second line.\n\n"
            b"Second paragraph, with $x y$ math and a {group of words} here.\n\n"
            b"\\section{Heading}\n\nThird paragraph ends here.\n")


class NewlineTests(unittest.TestCase):
    def test_space_becomes_newline_and_back(self):
        p = at(PLAIN, b"some")
        out = edits.newline(PLAIN, p)
        self.assertIn(b"some\nwords", out)
        self.assertEqual(out.replace(b"\n", b" "), PLAIN.replace(b"\n", b" "))
        # a toggle: the same edit again restores the original bytes
        self.assertEqual(edits.newline(out, p), PLAIN)

    def test_newline_between_prose_lines_becomes_space(self):
        out = edits.newline(PLAIN, at(PLAIN, b"words\nand"))
        self.assertIn(b"words and a second line", out)

    def test_no_edit_in_a_command_line_or_the_preamble(self):
        self.assertIsNone(edits.newline(PLAIN, at(PLAIN, b"Heading")))
        self.assertIsNone(edits.newline(PLAIN, at(PLAIN, b"article")))

    def test_no_edit_in_a_comment_or_before_one(self):
        src = doc(b"Some words % a comment here\nnext line\n\nOther text.\n")
        self.assertIsNone(edits.newline(src, at(src, b"comment")))
        # the line break after a comment ends the comment: it must stay
        self.assertIsNone(edits.newline(src, at(src, b"here")))

    def test_crlf_files_keep_crlf(self):
        src = PLAIN.replace(b"\n", b"\r\n")
        out = edits.newline(src, at(src, b"some"))
        self.assertIn(b"some\r\nwords", out)
        self.assertEqual(out.count(b"\n"), out.count(b"\r\n"))


class UnsafeContextTests(unittest.TestCase):
    """Positions inside these must never be edited by newline, split or join."""
    CONTEXTS = {
        "verbatim": b"Text before.\n\\begin{verbatim}\nword one two three\nOther line here\n\\end{verbatim}\n\nAfter text.\n",
        "lstlisting": b"Text before.\n\\begin{lstlisting}\nword one two three\n\\end{lstlisting}\n\nAfter text.\n",
        "verb": b"Text \\verb|word one two three| after it.\n\nNext text.\n",
        "equation": b"Text before.\n\\begin{equation}\nword one two\n\\end{equation}\n\nAfter text.\n",
        "bracket": b"Text before.\n\\[\nword one two\n\\]\n\nAfter text.\n",
        "paren": b"Text \\(word one two\\) after it.\n\nNext text.\n",
        "dollar": b"Text $word one two$ after it.\n\nNext text.\n",
        "dollars": b"Text $$word one two$$ after it.\n\nNext text.\n",
        "tabular": b"Text before.\n\\begin{tabular}{ll}\nword one & two three \\\\\n\\end{tabular}\n\nAfter text.\n",
        "align_star": b"Text before.\n\\begin{align*}\nword one &= two \\\\\nthree &= four\n\\end{align*}\n\nAfter text.\n",
        "lstinline_delim": b"Run \\lstinline|word one two three| now.\n\nNext text.\n",
        "lstinline_brace": b"Run \\lstinline{word one two three} now.\n\nNext text.\n",
        "lstinline_opt": b"Run \\lstinline[language=C]|word one two three| now.\n\nNext text.\n",
        "mintinline": b"Run \\mintinline{python}{word one two three} now.\n\nNext text.\n",
        "mintinline_delim": b"Run \\mintinline[linenos]{python}|word one two three| now.\n\nNext text.\n",
        "url_brace": b"See \\url{http://word one/two three} now.\n\nNext text.\n",
        "path_delim": b"See \\path|word one/two three| now.\n\nNext text.\n",
        "verb_star": b"Run \\verb*|word one two three| now.\n\nNext text.\n",
        "optional_arg": b"As shown \\cite[see the word one two three]{key} in prose.\n\nNext text.\n",
        "escaped": b"It costs \\$5 and then $word one two$ here.\n\nNext text.\n",
    }

    def test_no_line_or_paragraph_edit_inside_a_context(self):
        for name, body in self.CONTEXTS.items():
            src = doc(body)
            p = at(src, b"word")
            for kind in ("newline", "split"):
                self.assertIsNone(edits.apply(kind, src, p), (name, kind))

    def test_tikz_path_is_not_verbatim(self):
        # \\path[..] and \\path(..) are TikZ drawing commands; only the path
        # package's \\path|x| (and \\path{x}) is verbatim.  The old scan ran the
        # verbatim region from \\path[ to the next `[` or the end of the file.
        for tikz in (b"\\path[draw] (0,0) -- (1,1);", b"\\path(0,0) -- (1,1);", b"\\path (0,0) -- (1,1);"):
            src = doc(b"Plot " + tikz + b" and then word one two three here.\n\nNext text.\n")
            self.assertEqual([r for r in edits.scan(src) if r[0] == 'verb'], [], tikz)
            self.assertIsNotNone(edits.split(src, at(src, b"word")), tikz)
        src = doc(b"See \\path{word one/two three} now.\n\nNext text.\n")
        self.assertIsNone(edits.split(src, at(src, b"word")))

    def test_escaped_dollar_is_not_a_math_delimiter(self):
        # the text between \$5 and the real math is plain prose and stays editable
        src = doc(b"It costs \\$5 and then more words here, plus $x y$ later.\n\nNext text.\n")
        out = edits.split(src, at(src, b"words"))
        self.assertIsNotNone(out)
        self.assertIn(b"words\n\nhere", out)
        # while the real math span is still protected
        self.assertIsNone(edits.split(src, at(src, b"x y")))

    def test_join_does_not_join_inside_verbatim(self):
        src = doc(b"Intro text here.\n\\begin{verbatim}\nFoo.\n\nBar\n\\end{verbatim}\nOutro.\n")
        self.assertIsNone(edits.join(src, at(src, b"Foo")))

    def test_join_does_not_join_around_a_comment(self):
        src = doc(b"Text one. % a comment\n\nNext paragraph.\n")
        self.assertIsNone(edits.join(src, 0))

    def test_comment_lines_are_not_edited_by_the_context_kinds(self):
        src = doc(b"Some prose here.\n% $a b$ in a comment\n% x & y\n\nMore prose.\n")
        self.assertIsNone(edits.math_par(src, at(src, b"$a b$")))
        self.assertIsNone(edits.cell_blank(src, at(src, b"x & y")))


class SplitTests(unittest.TestCase):
    def test_split_makes_a_blank_line(self):
        out = edits.split(PLAIN, at(PLAIN, b"some"))
        self.assertIn(b"some\n\nwords", out)
        self.assertEqual(out.count(b"\n\n"), PLAIN.count(b"\n\n") + 1)

    def test_no_split_inside_a_group_or_before_punctuation_or_at_the_end(self):
        self.assertIsNone(edits.split(PLAIN, at(PLAIN, b"group")))
        self.assertIsNone(edits.split(PLAIN, at(PLAIN, b"line")))
        self.assertIsNone(edits.split(b"word", 0))

    def test_whitespace_only_lines_are_paragraph_breaks(self):
        # the open group of the first paragraph does not leak into the second:
        # the line between them holds only blanks
        src = doc(b"Para one has a {\n   \nPara two plain words here.\n")
        out = edits.split(src, at(src, b"plain"))
        self.assertIsNotNone(out)
        self.assertIn(b"plain\n\nwords", out)

    def test_crlf_files_keep_crlf(self):
        src = PLAIN.replace(b"\n", b"\r\n")
        out = edits.split(src, at(src, b"some"))
        self.assertIn(b"some\r\n\r\nwords", out)
        self.assertEqual(out.count(b"\n"), out.count(b"\r\n"))


class JoinTests(unittest.TestCase):
    def test_join_two_prose_paragraphs(self):
        out = edits.join(PLAIN, at(PLAIN, b"Second"))
        self.assertIn(b"second line. Second paragraph", out)
        self.assertEqual(out.count(b"\n\n"), PLAIN.count(b"\n\n") - 1)

    def test_join_treats_blank_only_lines_and_crlf_as_one_break(self):
        src = doc(b"First paragraph ends here.\n \t \n\nSecond paragraph starts.\n")
        self.assertIn(b"here. Second", edits.join(src, 0))
        crlf = PLAIN.replace(b"\n", b"\r\n")
        out = edits.join(crlf, at(crlf, b"Second"))
        self.assertIn(b"line. Second paragraph", out)

    def test_no_join_around_a_command_paragraph_or_outside_the_body(self):
        src = doc(b"Words end here.\n\n\\section{A}\n\nMore words.\n")
        self.assertIsNone(edits.join(src, 0))
        pre = b"Preamble text here.\n\nMore preamble.\n\\begin{document}\nx\n\\end{document}\n"
        self.assertIsNone(edits.join(pre, 0))


CTX = doc(b"Some $a b$ words.\n\\begin{verbatim}\nline one\nline two\n\\end{verbatim}\n"
          b"\\begin{tabular}{ll}\nx & y \\\\\nz & w \\\\\n\\end{tabular}\n")


class MeaningChangingTests(unittest.TestCase):
    def test_math_par_puts_a_blank_line_inside_one_span(self):
        out = edits.math_par(CTX, at(CTX, b"Some"))
        self.assertIn(b"$a\n\nb$", out)

    def test_math_par_stays_inside_one_span_not_between_two(self):
        src = doc(b"Words $a$ and $b$ here, and $c d$ later.\n")
        out = edits.math_par(src, at(src, b"$a$"))
        # only the span that has a space inside is eligible: the blank line is inside "$c d$"
        self.assertIn(b"$c\n\nd$", out)
        self.assertIn(b"$a$ and $b$", out)

    def test_math_par_works_in_paren_math_and_needs_a_space(self):
        src = doc(b"Words \\(a b\\) here.\n")
        self.assertIn(b"\\(a\n\nb\\)", edits.math_par(src, 0))
        self.assertIsNone(edits.math_par(doc(b"Only $ab$ here.\n"), 0))

    def test_verbatim_blank_adds_one_line_after_the_first(self):
        out = edits.verbatim_blank(CTX, at(CTX, b"line one"))
        self.assertIn(b"line one\n\nline two", out)
        self.assertEqual(len(out), len(CTX) + 1)

    def test_verbatim_blank_needs_a_second_line_and_an_uncommented_begin(self):
        self.assertIsNone(edits.verbatim_blank(doc(b"\\begin{verbatim}\nonly\\end{verbatim}\n"), 0))
        self.assertIsNone(edits.verbatim_blank(doc(b"%\\begin{verbatim}\nx\ny\n%\\end{verbatim}\n"), 0))

    def test_cell_blank_splits_a_cell_with_a_blank_line(self):
        out = edits.cell_blank(CTX, at(CTX, b"x & y"))
        self.assertIn(b"x &\n\ny", out)

    def test_cell_blank_needs_a_tabular(self):
        self.assertIsNone(edits.cell_blank(doc(b"a & b\n"), 0))


class ZeroTrialTests(unittest.TestCase):
    def test_a_run_with_no_trials_is_an_error_message(self):
        self.assertIsNone(edits.zero_trials_error(3, "newline"))
        msg = edits.zero_trials_error(0, "newline,split")
        self.assertIn("0 trials", msg)
        self.assertIn("newline,split", msg)
        self.assertIn("not a pass", msg)

    def test_both_tools_use_it(self):
        here = os.path.dirname(os.path.abspath(__file__))
        for script in ("incr_bench.py", "soundness.py"):
            with open(os.path.join(here, script)) as f:
                self.assertIn("edits.zero_trials_error", f.read(), script)

    def test_soundness_seeds_are_stable_across_runs(self):
        here = os.path.dirname(os.path.abspath(__file__))
        with open(os.path.join(here, "soundness.py")) as f:
            src = f.read()
        self.assertNotIn("hash(name)", src)
        self.assertIn("zlib.crc32", src)


class PurityAndKindsTests(unittest.TestCase):
    def test_edits_are_deterministic_and_leave_their_input_alone(self):
        for kind in edits.LINE_KINDS + edits.CONTEXT_KINDS:
            for p in range(0, len(CTX), 7):
                a = edits.apply(kind, CTX, p)
                b = edits.apply(kind, CTX, p)
                self.assertEqual(a, b, (kind, p))

    def test_seeded_edit_sequences_repeat(self):
        def run(seed):
            rng = random.Random(seed)
            out = []
            for _ in range(40):
                kind = rng.choice(edits.LINE_KINDS + edits.CONTEXT_KINDS)
                p = rng.randrange(len(CTX))
                out.append((kind, p, edits.apply(kind, CTX, p)))
            return out
        self.assertEqual(run(11), run(11))
        self.assertNotEqual(run(11), run(12))

    def test_reverting_restores_the_original_bytes(self):
        # the harness writes the saved source back: an edit never aliases it
        saved = bytes(PLAIN)
        for kind in edits.LINE_KINDS:
            edits.apply(kind, PLAIN, at(PLAIN, b"some"))
        self.assertEqual(PLAIN, saved)

    def test_parse_kinds(self):
        self.assertEqual(edits.parse_kinds("replace,newline,split,join"),
                         ["replace", "newline", "split", "join"])
        self.assertEqual(edits.parse_kinds("math_par"), ["math_par"])
        for bad in ("nope", "replace,", "", "replace,newlines"):
            with self.assertRaises(ValueError):
                edits.parse_kinds(bad)


class CommandLineTests(unittest.TestCase):
    """An unknown --kinds used to run zero trials and report 0 mismatches, which looks like a pass."""

    def run_tool(self, script, *args):
        import subprocess
        here = os.path.dirname(os.path.abspath(__file__))
        return subprocess.run([sys.executable, os.path.join(here, script), *args],
                              capture_output=True, text=True, timeout=60)

    def test_incr_bench_rejects_an_unknown_kind(self):
        r = self.run_tool("incr_bench.py", "eng", "dir", "doc", "--kinds", "replace,newlines")
        self.assertEqual(r.returncode, 2)
        self.assertIn("unknown or empty edit kind", r.stderr)

    def test_soundness_rejects_an_unknown_kind(self):
        r = self.run_tool("soundness.py", "eng", "--kinds", "bogus")
        self.assertEqual(r.returncode, 2)
        self.assertIn("unknown or empty edit kind", r.stderr)


class AfterPackageTests(unittest.TestCase):
    SRC = (b"\\documentclass{article}\n\\usepackage{amsmath}\n\\usepackage{hyperref}\n"
           b"\\title{A title}\n\\author{Jo}\n\\begin{document}\nText.\n\\end{document}\n")

    def test_a_letter_in_the_line_after_the_last_usepackage(self):
        for p in range(6):
            out = edits.pre_after_package(self.SRC, p)
            self.assertEqual(out.replace(b"x", b"", 1), self.SRC)
            line = out.split(b"\n")[3]
            self.assertTrue(line.startswith(b"\\title{"), line)
            self.assertEqual(line.count(b"x"), 1)

    def test_none_without_a_line_after_a_package(self):
        src = b"\\documentclass{article}\n\\usepackage{amsmath}\n\\begin{document}\nText.\n\\end{document}\n"
        self.assertIsNone(edits.pre_after_package(src, 0))
        self.assertIn("pre_after_package", edits.PREAMBLE_KINDS)


if __name__ == "__main__":
    unittest.main()
