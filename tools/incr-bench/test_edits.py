#!/usr/bin/env python3
"""Tests for edits.py (unittest): run `python3 -m unittest discover -s tools/incr-bench -p 'test_*.py'`."""
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import edits  # noqa: E402

DOC = (b"\\documentclass{article}\n\\begin{document}\n"
       b"First paragraph has some words\nand a second line.\n\n"
       b"Second paragraph, with $x y$ math and a {group of words} here.\n\n"
       b"\\section{Heading}\n\nThird paragraph ends here.\n\\end{document}\n")


def at(word):
    return DOC.index(word.encode())


class NewlineTests(unittest.TestCase):
    def test_space_becomes_newline(self):
        p = at("some")
        out = edits.newline(DOC, p)
        self.assertEqual(out.replace(b"\n", b" "), DOC.replace(b"\n", b" "))
        self.assertIn(b"some\nwords", out)

    def test_newline_between_prose_lines_becomes_space(self):
        out = edits.newline(DOC, at("words"))
        self.assertIn(b"words and a second line", out)
        self.assertNotIn(b"words\nand", out)

    def test_no_edit_in_a_command_line_or_a_comment(self):
        self.assertIsNone(edits.newline(DOC, at("Heading")))
        src = b"Some words % a comment here\nnext line\n"
        self.assertIsNone(edits.newline(src, src.index(b"comment")))


class SplitTests(unittest.TestCase):
    def test_split_makes_a_blank_line(self):
        out = edits.split(DOC, at("some"))
        self.assertIn(b"some\n\nwords", out)
        self.assertEqual(out.count(b"\n\n"), DOC.count(b"\n\n") + 1)

    def test_no_split_inside_a_group_or_math(self):
        self.assertIsNone(edits.split(DOC, at("group")))
        src = b"A paragraph with $x and y$ math.\n"
        self.assertIsNone(edits.split(src, src.index(b"and")))

    def test_no_split_before_punctuation_or_at_the_end(self):
        self.assertIsNone(edits.split(DOC, at("line")))
        self.assertIsNone(edits.split(b"word", 0))


class JoinTests(unittest.TestCase):
    def test_join_two_prose_paragraphs(self):
        out = edits.join(DOC, at("Second"))
        self.assertIn(b"second line. Second paragraph", out)
        self.assertEqual(out.count(b"\n\n"), DOC.count(b"\n\n") - 1)

    def test_no_join_around_a_command_paragraph(self):
        src = (b"\\begin{document}\nWords end here.\n\n\\section{A}\n\nMore words.\n"
               b"\\end{document}\n")
        self.assertIsNone(edits.join(src, 0))

    def test_no_join_outside_the_body(self):
        src = b"Preamble text here.\n\nMore preamble.\n\\begin{document}\nx\n\\end{document}\n"
        self.assertIsNone(edits.join(src, 0))


if __name__ == "__main__":
    unittest.main()
