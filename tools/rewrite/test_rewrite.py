"""Tests for type1_sweep's pure logic (no TeX Live, engine or network).

    python3 -m unittest discover -s tools/rewrite -p 'test_*.py' -v
"""

import contextlib
import hashlib
import io
import json
import os
import struct
import sys
import tempfile
import unittest
import zlib
from argparse import Namespace
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import type1_sweep  # noqa: E402


def _seg(kind, body):
    return b"\x80" + bytes([kind]) + struct.pack("<I", len(body)) + body


def _good_pfb():
    return _seg(1, b"hello") + _seg(2, b"\x01\x02") + b"\x80\x03"


def _rec(job, pdf="p1", log="l1", rc=0, fonts=("f1",), fatal=False, error=None):
    return {"job": job, "pdf": pdf, "log": log, "rc": rc,
            "fonts": list(fonts), "fatal": fatal, "error": error, "warnings": []}


def _write(path, recs):
    with open(path, "w") as fh:
        for r in recs:
            fh.write(json.dumps(r) + "\n")


class PfbToPfa(unittest.TestCase):

    def test_converts_ascii_and_binary_segments(self):
        pfb = _seg(1, b"abc\rdef") + _seg(2, b"\x00\xff\x10") + b"\x80\x03"
        self.assertEqual(type1_sweep.pfb_to_pfa(pfb), b"abc\ndef00ff10\n")

    def test_wraps_binary_hex_at_64_digits_per_line(self):
        pfb = _seg(2, bytes(range(40))) + b"\x80\x03"
        lines = type1_sweep.pfb_to_pfa(pfb).split(b"\n")
        self.assertEqual((len(lines[0]), len(lines[1])), (64, 16))
        self.assertEqual(bytes.fromhex((lines[0] + lines[1]).decode()), bytes(range(40)))

    def test_malformed_inputs_return_none_but_bare_eof_is_empty(self):
        self.assertIsNone(type1_sweep.pfb_to_pfa(b""))
        self.assertIsNone(type1_sweep.pfb_to_pfa(b"\x80\x01\x05\x00"))  # truncated header
        self.assertIsNone(type1_sweep.pfb_to_pfa(b"\x80\x01\x05\x00\x00\x00abc"))  # truncated body
        self.assertIsNone(type1_sweep.pfb_to_pfa(_seg(4, b"xx") + b"\x80\x03"))  # bad kind
        self.assertIsNone(type1_sweep.pfb_to_pfa(_seg(1, b"hi")))  # missing EOF
        self.assertEqual(type1_sweep.pfb_to_pfa(b"\x80\x03"), b"")


class FontStreams(unittest.TestCase):

    def test_hashes_each_stream_in_order(self):
        pdf = (b"/Length1 10 /Length2 20 /Length3 30 >>\nstream\nFONTDATA1endstream"
               b"/Length1 1 /Length2 2 /Length3 3 /Length 99 >>\nstream\nFONTDATA2endstream")
        self.assertEqual(type1_sweep.font_streams(pdf),
                         [hashlib.sha256(b"FONTDATA1").hexdigest()[:16],
                          hashlib.sha256(b"FONTDATA2").hexdigest()[:16]])

    def test_decompresses_deflated_body_and_ignores_other_streams(self):
        pdf = (b"/Length 5 >>\nstream\nhelloendstream"  # not a font stream
               b"/Length1 1 /Length2 2 /Length3 3 >>\nstream\n" + zlib.compress(b"FONTDATA")
               + b"endstream")
        self.assertEqual(type1_sweep.font_streams(pdf), [hashlib.sha256(b"FONTDATA").hexdigest()[:16]])
        self.assertEqual(type1_sweep.font_streams(b"no streams here"), [])


class Messages(unittest.TestCase):

    def test_error_and_warnings_join_wrapped_lines(self):
        log = (b"!pdfTeX error: bad thing\nsplit ==> Fatal error\n\n"
               b"pdfTeX warning: first warn\nwrapped\n\n"
               b"pdfTeX warning: last")
        error, warnings = type1_sweep.messages(log)
        self.assertEqual(error, "bad thingsplit")
        self.assertEqual(warnings, ["first warnwrapped", "last"])


class Summarise(unittest.TestCase):

    def test_fatal_flags_hashes_and_fonts(self):
        pdf = b"/Length1 1 /Length2 2 /Length3 3 >>\nstream\nHELLOendstream"
        r = type1_sweep.summarise(0, pdf, b"ok !pdfTeX error: x ==> Fatal error")
        self.assertEqual((r["rc"], r["fatal"], r["error"]), (0, True, "x"))
        self.assertEqual(r["pdf"], hashlib.sha256(pdf).hexdigest()[:16])
        self.assertEqual(r["fonts"], [hashlib.sha256(b"HELLO").hexdigest()[:16]])
        self.assertTrue(type1_sweep.summarise(0, b"", b"! Emergency stop")["fatal"])
        clean = type1_sweep.summarise(1, b"", b"plain")
        self.assertEqual((clean["pdf"], clean["fatal"], clean["fonts"]), (None, False, []))
        self.assertEqual(clean["log"], hashlib.sha256(b"plain").hexdigest()[:16])


class Codes(unittest.TestCase):

    def test_some_is_deterministic_and_holds_the_fixed_five(self):
        a, b = type1_sweep.codes("cmr10", "some"), type1_sweep.codes("cmr10", "some")
        self.assertEqual(a, b)
        self.assertLessEqual({65, 66, 97, 101, 48}, set(a))
        self.assertEqual(a, sorted(a))
        self.assertTrue(all(0 <= c <= 255 for c in a))
        self.assertNotEqual(type1_sweep.codes("cmr10", "some"), type1_sweep.codes("cmti10", "some"))


class JobText(unittest.TestCase):

    def test_some_emits_font_without_mapline(self):
        text = type1_sweep.job_text([("cmr10", "cmr10 <cmr10.pfb", "<cmr10.pfb")], "some")
        self.assertNotIn("\\pdfmapline", text)
        self.assertIn("\\font\\x=cmr10", text)
        self.assertTrue(text.endswith("\\end\n") and text.startswith(type1_sweep.HEAD))

    def test_full_rewrites_subset_to_whole(self):
        text = type1_sweep.job_text([("cmr10", "cmr10 <cmr10.pfb", "<cmr10.pfb")], "full")
        self.assertIn("\\pdfmapline{=cmr10 <<cmr10.pfb}", text)
        self.assertIn("\\font\\x=cmr10", text)

    # Known bug, #2054: job_text("full") `continue`s past already-whole (<<)
    # entries, skipping their \font/\shipout line as well as the map rewrite.
    # Remove the decorator in the fix.
    @unittest.expectedFailure
    def test_full_still_typesets_already_whole_fonts(self):
        text = type1_sweep.job_text([("cmr10", "cmr10 <<cmr10.pfb", "<<cmr10.pfb")], "full")
        self.assertIn("\\font\\x=cmr10", text)


class AgainstPdftex(unittest.TestCase):

    def test_reports_first_difference_else_none(self):
        x = {"rc": 0, "fatal": False, "error": None, "fonts": ["a"]}
        self.assertIsNone(type1_sweep.against_pdftex(x, dict(x)))
        for key in ("rc", "fatal", "error", "fonts"):
            y = dict(x)
            y[key] = "DIFF"
            self.assertEqual(type1_sweep.against_pdftex(x, y), key)
        self.assertEqual(type1_sweep.against_pdftex({"rc": 0, "fatal": False}, {"rc": 1, "fatal": True}), "rc")


class Compare(unittest.TestCase):

    def _compare(self, old, new, streams_only):
        with tempfile.TemporaryDirectory() as d:
            a, b = os.path.join(d, "a.jsonl"), os.path.join(d, "b.jsonl")
            _write(a, old)
            _write(b, new)
            buf = io.StringIO()
            with contextlib.redirect_stdout(buf):
                rc = type1_sweep.compare(Namespace(old=a, new=b, streams_only=streams_only))
            return rc, buf.getvalue()

    def test_full_sees_pdf_differences_streams_only_ignores_them(self):
        old, new = [_rec("j1")], [_rec("j1", pdf="DIFF")]
        self.assertEqual(self._compare(old, new, False)[0], 1)
        rc, out = self._compare(old, new, True)
        self.assertEqual((rc, "DIFF j1" in out), (0, False))

    def test_streams_only_catches_font_and_missing_differences(self):
        rc, out = self._compare([_rec("j1")], [_rec("j1", fonts=("DIFF",))], True)
        self.assertEqual(rc, 1)
        self.assertIn("DIFF j1: fonts", out)
        rc, out = self._compare([_rec("j1"), _rec("j2")], [_rec("j1")], False)
        self.assertEqual(rc, 1)
        self.assertIn("DIFF j2: missing", out)


class Entries(unittest.TestCase):

    def _entries(self, text, limit=0, every=1):
        with tempfile.NamedTemporaryFile("w", delete=False, suffix=".map") as fh:
            fh.write(text)
            path = fh.name
        try:
            with mock.patch.object(type1_sweep, "map_file", return_value=path):
                return type1_sweep.entries(limit, every)
        finally:
            os.unlink(path)

    def test_first_seen_wins_and_noise_is_skipped(self):
        sel = self._entries("% c\n# c\n* c\n; c\n\ncmr10 <cmr10.pfb\ncmr10-dup <cmr10.pfb\n"
                            "bad <foo.ttf\nnoff cmr12\nptmr8r <ptmr8r.pfa\n")
        self.assertEqual([(t, f) for t, _, f in sel],
                         [("cmr10", "<cmr10.pfb"), ("ptmr8r", "<ptmr8r.pfa")])

    def test_slant_and_extend_are_always_picked(self):
        sel = self._entries("a <a.pfb\nb <b.pfb\nb-slant <b.pfb \" Slant 0.5 \"\n"
                            "b-plain <b.pfb\nc-ext <a.pfb \" 0.9 ExtendFont \"\n")
        self.assertEqual([t for t, _, _ in sel], ["a", "b", "b-slant", "c-ext"])

    def test_tenth_reencoding_limit_and_every(self):
        text = "base <base.pfb\n" + "".join("enc%d <base.pfb\n" % i for i in range(11))
        self.assertEqual([t for t, _, _ in self._entries(text)], ["base", "enc9"])
        self.assertEqual(len(self._entries(text, limit=1)), 1)
        text2 = "a <a.pfb\nb <b.pfb\nc <c.pfb\n"
        self.assertEqual([t for t, _, _ in self._entries(text2, every=2)], ["a", "c"])


class FuzzCase(unittest.TestCase):
    class _Mut:
        def __init__(self, ret=None):
            self.ret, self.calls = ret, []

        def mutate_with_info(self, data, rng):
            self.calls.append(data)
            return self.ret if self.ret is not None else (data, "id")

    def test_deterministic_with_pfa_every_third(self):
        seeds = [("cmr10.pfb", _good_pfb()), ("cmti10.pfb", _good_pfb())]
        first = type1_sweep.fuzz_case(0, 42, seeds, b"TFM", self._Mut())
        again = type1_sweep.fuzz_case(0, 42, seeds, b"TFM", self._Mut())
        self.assertEqual(first, again)
        for i in range(6):
            name, data, _, whole = type1_sweep.fuzz_case(i, 42, seeds, b"TFM", self._Mut())
            self.assertEqual(name.endswith(".pfa"), i % 3 == 2)
            self.assertIsInstance(whole, bool)
            self.assertTrue(data)

    def test_unparseable_mutation_falls_back_to_mutating_the_pfa(self):
        seeds = [("cmr10.pfb", _good_pfb())]
        mut = self._Mut(ret=(b"GARBAGE-NOT-PFB", "corrupt"))
        name, _, _, _ = type1_sweep.fuzz_case(2, 42, seeds, b"TFM", mut)
        self.assertTrue(name.endswith(".pfa"))
        self.assertEqual(len(mut.calls), 2)  # PFB mutation, then PFA mutation
        self.assertEqual(mut.calls[1], type1_sweep.pfb_to_pfa(_good_pfb()))


class EngineCmd(unittest.TestCase):

    def test_engine_and_pdftex_forms(self):
        with mock.patch.dict(os.environ, {}, clear=True):
            exe, env = type1_sweep.engine_cmd(engine="/tmp/eng")
            self.assertEqual((exe, env["FLASHTEX_POOL"]), ("/tmp/eng/pdftex", "/tmp/eng/pdftex.pool"))
            self.assertEqual((env["SOURCE_DATE_EPOCH"], env["FORCE_SOURCE_DATE"]), ("0", "1"))
            exe, env = type1_sweep.engine_cmd(pdftex="/usr/bin/pdftex")
            self.assertEqual(exe, "/usr/bin/pdftex")
            self.assertNotIn("FLASHTEX_POOL", env)
            self.assertEqual((env["SOURCE_DATE_EPOCH"], env["FORCE_SOURCE_DATE"]), ("0", "1"))
