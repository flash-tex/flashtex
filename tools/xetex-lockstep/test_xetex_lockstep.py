"""Tests for the XeTeX lockstep harness's pure logic (no TeX, no engine).

    python3 -m unittest discover -s tools/xetex-lockstep -p 'test_*.py' -v
"""

import json
import os
import struct
import sys
import tempfile
import unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import fontmatch  # noqa: E402
import latex  # noqa: E402
import run  # noqa: E402


def pre(comment=b"abc"):
    return bytes([247, 7]) + bytes(12) + bytes([len(comment)]) + comment


POST = bytes([249]) + bytes(5) + bytes([223] * 4)


def natrec(path, flags=0):
    rec = (bytes([252]) + (1).to_bytes(4, "big") + (655360).to_bytes(4, "big")
           + flags.to_bytes(2, "big") + bytes([len(path)]) + path
           + (0).to_bytes(4, "big"))
    for bit in (0x0200, 0x1000, 0x2000, 0x4000):
        if flags & bit:
            rec += bit.to_bytes(4, "big")
    return rec


def paged(path):
    """Two pages after a native font record; the second bop points at the first."""
    head = pre() + natrec(path)
    bop = lambda prev: bytes([139]) + bytes(40) + prev.to_bytes(4, "big", signed=True)
    return head + bop(-1) + bytes([140]) + bop(len(head)) + bytes([140]) + POST


def w(path, text="x"):
    with open(path, "w") as fh:
        fh.write(text)


def r(path):
    with open(path) as fh:
        return fh.read()


class NormaliseXdv(unittest.TestCase):

    def test_non_xdv_returned_unchanged(self):
        for data in (b"", b"short", b"\0" * 20, bytes([246]) + bytes(20)):
            self.assertEqual(run.normalise_xdv(data), data)

    def test_unwalkable_stream_keeps_rest(self):
        data = pre() + bytes([255, 1, 2, 3])  # 255 is no opcode
        self.assertEqual(run.normalise_xdv(data), pre(b"\0\0\0") + bytes([255, 1, 2, 3]))

    def test_252_inside_special_params_is_not_a_record(self):
        special = bytes([239, 3]) + b"a\xfcb"  # xxx1 holding a 252 byte
        data = pre() + special + POST
        self.assertEqual(run.normalise_xdv(data), pre(b"\0\0\0") + special + POST)
        self.assertEqual(run.native_font_files(data), [])

    def test_walks_set_glyphs_to_the_next_record(self):
        glyphs = bytes([253]) + bytes(4) + (1).to_bytes(2, "big") + bytes(10)
        got = run.normalise_xdv(pre() + glyphs + natrec(b"/d/F.otf") + POST)
        self.assertIn(bytes([5]) + b"F.otf", got)
        self.assertNotIn(b"/d/F.otf", got)

    def test_dvi_param_lengths(self):
        self.assertEqual(run._dvi_param_len(0, b"", 0), 0)
        self.assertEqual(run._dvi_param_len(138, b"", 0), 0)
        self.assertEqual(run._dvi_param_len(131, b"", 0), 4)
        self.assertEqual(run._dvi_param_len(139, b"", 0), 44)
        self.assertEqual(run._dvi_param_len(248, b"", 0), 28)
        self.assertEqual(run._dvi_param_len(239, bytes([5]) + b"hello", 0), 6)
        fnt = bytes([7]) + bytes(12) + bytes([2, 3]) + b"abcde"
        self.assertEqual(run._dvi_param_len(243, fnt, 0), 20)
        set_g = bytes(4) + (3).to_bytes(2, "big") + bytes(30)
        self.assertEqual(run._dvi_param_len(253, set_g, 0), 36)
        set_tg = (2).to_bytes(2, "big") + bytes(8) + (1).to_bytes(2, "big") + bytes(10)
        self.assertEqual(run._dvi_param_len(254, set_tg, 0), 22)
        for op in (247, 250, 251, 255):
            self.assertIsNone(run._dvi_param_len(op, b"", 0))


class FontFiles(unittest.TestCase):

    def test_native_font_files(self):
        data = pre() + natrec(b"/r/A.ttf") + natrec(b"/r/B.ttf") + POST
        self.assertEqual(run.native_font_files(data), [b"/r/A.ttf", b"/r/B.ttf"])
        self.assertEqual(run.native_font_files(b"junk"), [])
        self.assertEqual(run.native_font_files(pre() + bytes([252])), [])


class XetexTestSteps(unittest.TestCase):

    def test_known_tests(self):
        self.assertEqual(run.xetex_test_steps("bug73"),
                         [("bug73", ["-ini", "bug73"]), ("bug73", ["-fmt=bug73", "bug73"])])
        self.assertEqual(run.xetex_test_steps("ctrlsym"),
                         [("xe-basic", ["-ini", "-etex", "xe-basic"]),
                          ("xe-ctrlsym", ["-etex", "-fmt=xe-basic", "xe-ctrlsym"])])
        self.assertEqual(run.xetex_test_steps("filedump"), [("filedump", ["-ini", "filedump"])])


class SuiteCases(unittest.TestCase):

    def test_parses_suite_and_own_cases(self):
        with tempfile.TemporaryDirectory() as d:
            suite = os.path.join(d, "suite.txt")
            w(suite, "# comment\n\na\n b # trailing\n")
            own = os.path.join(d, "cases")
            os.mkdir(own)
            w(os.path.join(own, "x001.tex"))
            w(os.path.join(own, "notes.txt"))
            with mock.patch.object(run, "SUITE", suite), mock.patch.object(run, "OWN_CASES", own):
                self.assertEqual(run.suite_cases(), ["a", "b", "x001"])
            with mock.patch.object(run, "SUITE", os.path.join(d, "nope.txt")), \
                    mock.patch.object(run, "OWN_CASES", own):
                self.assertEqual(run.suite_cases(), ["x001"])

    def test_case_source_prefers_own(self):
        with tempfile.TemporaryDirectory() as d:
            own, lock = os.path.join(d, "own"), os.path.join(d, "lock")
            os.mkdir(own)
            os.mkdir(lock)
            w(os.path.join(own, "n.tex"))
            with mock.patch.object(run, "OWN_CASES", own), \
                    mock.patch.object(run, "LOCKSTEP_CASES", lock):
                self.assertEqual(run.case_source("n"), os.path.join(own, "n.tex"))
                self.assertEqual(run.case_source("m"), os.path.join(lock, "m.tex"))


class FontmatchParsing(unittest.TestCase):

    def test_unhat(self):
        self.assertEqual(fontmatch.unhat("caf^^c3^^a9"), "café")
        self.assertEqual(fontmatch.unhat("plain"), "plain")

    def _named(self, text):
        fh = tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False)
        fh.write(text)
        fh.close()
        self.addCleanup(os.remove, fh.name)
        return fh.name

    def test_read_names(self):
        path = self._named("# c\n\nHelvetica\nTimes\t12 \n  # indented\nArial\tscaled 1200\n")
        self.assertEqual(fontmatch.read_names(path),
                         [("Helvetica", ""), ("Times", "12"), ("Arial", "scaled 1200")])

    def test_read_known(self):
        path = self._named("# c\n\nface\tSTIX Two Text Bold\nname\tN\t12\n")
        self.assertEqual(fontmatch.read_known(path),
                         {("face", "STIX Two Text Bold", ""), ("name", "N", "12")})
        self.assertEqual(fontmatch.read_known(os.path.join(HERE, "no-such-file.txt")), set())
        self.assertEqual(fontmatch.read_known(None), set())
        with self.assertRaises(SystemExit):
            fontmatch.read_known(self._named("size\tN\n"))


class PostambleFonts(unittest.TestCase):

    def _xdv(self, defs, q=15):
        body = pre(b"") + bytes([248]) + bytes(28) + b"".join(defs)
        return body + bytes([249]) + struct.pack(">I", q) + bytes([2]) + bytes([223] * 4)

    def test_native_fonts(self):
        path = b"/a/F.otf"
        rec = (bytes([252]) + struct.pack(">IiHB", 7, 655360, 0x1000, len(path))
               + path + struct.pack(">I", 2) + struct.pack(">I", 100))
        fnt = bytes([243, 5]) + bytes(12) + bytes([1, 2]) + b"d" + b"ef"
        self.assertEqual(fontmatch.native_fonts(self._xdv([fnt, rec])),
                         [(7, 655360, 0x1000, "/a/F.otf", 2)])
        with self.assertRaises(ValueError):
            fontmatch.native_fonts(b"nope")
        with self.assertRaises(ValueError):
            fontmatch.native_fonts(self._xdv([], q=16))


class LatexCases(unittest.TestCase):

    def test_doc_arg(self):
        self.assertEqual(latex.doc_arg("m", {}, False), "m.tex")
        self.assertEqual(latex.doc_arg("m", {"firstline": "\\A"}, False), "\\A\\input{m.tex}")
        self.assertEqual(latex.doc_arg("m", {}, True), latex.TRACE_PREFIX["full"] + "\\input{m}")
        self.assertEqual(latex.doc_arg("m", {"firstline": "\\A", "trace": "boxes"}, True),
                         latex.TRACE_PREFIX["boxes"] + "\\A\\input{m}")

    def test_find_cases(self):
        with tempfile.TemporaryDirectory() as d:
            w(os.path.join(d, "a.tex"))
            os.mkdir(os.path.join(d, "b"))
            w(os.path.join(d, "b", "b.tex"))
            w(os.path.join(d, "c.txt"))
            os.mkdir(os.path.join(d, "e"))
            w(os.path.join(d, "e", "other.tex"))
            self.assertEqual(latex.find_cases([d]),
                             {"a": os.path.join(d, "a.tex"), "b": os.path.join(d, "b")})

    def test_stage(self):
        with tempfile.TemporaryDirectory() as d:
            src = os.path.join(d, "n.tex")
            w(src, "tex")
            out = os.path.join(d, "out")
            latex.stage(src, "n", out)
            self.assertEqual(r(os.path.join(out, "n.tex")), "tex")
            c = os.path.join(d, "c")
            os.mkdir(c)
            w(os.path.join(c, "c.tex"), "tex")
            w(os.path.join(c, "extra.sty"), "sty")
            w(os.path.join(c, latex.META), "{}")
            w(os.path.join(c, "expect.json"), "{}")
            latex.stage(c, "c", os.path.join(d, "out2"))
            self.assertEqual(sorted(os.listdir(os.path.join(d, "out2"))), ["c.tex", "extra.sty"])


if __name__ == "__main__":
    unittest.main()
