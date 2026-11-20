"""Pure-logic tests for tools/unicode-m3 (no TeX Live, network, GPU, display, or engine).
Run: python3 -m unittest discover -s tools/unicode-m3 -p 'test_*.py' -v"""

import contextlib
import io
import json
import os
import socket
import struct
import sys
import tarfile
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import corpus  # noqa: E402
import fontconfig  # noqa: E402
import host_startup  # noqa: E402
import host_sweep  # noqa: E402
import identical  # noqa: E402
import latency  # noqa: E402
import notex_corpus  # noqa: E402
import profile_startup  # noqa: E402
import summarize  # noqa: E402

ABC_SHA256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
DOC = "\\documentclass{article}\n\\begin{document}\nBody.\n\\end{document}\n"


def _w(path, data, mode="w"):
    with open(path, mode) as f:
        f.write(data)


def _r(path, mode="r"):
    with open(path, mode) as f:
        return f.read()


def write_tar(path, members):
    with tarfile.open(path, "w:xz") as t:
        for name, data in members.items():
            ti = tarfile.TarInfo(name)
            ti.size = len(data)
            t.addfile(ti, io.BytesIO(data))


class CorpusExtract(unittest.TestCase):

    def test_keeps_layout_below_root_and_skips_outside(self):
        with tempfile.TemporaryDirectory() as tmp:
            tar = os.path.join(tmp, "a.tar.xz")
            write_tar(tar, {"doc/x/main.tex": b"hi", "doc/x/sub/a.sty": b"s",
                            "other/y.tex": b"skip"})
            dest = os.path.join(tmp, "d")
            corpus.extract(tar, "doc/x", dest)
            self.assertEqual(_r(os.path.join(dest, "main.tex"), "rb"), b"hi")
            self.assertEqual(_r(os.path.join(dest, "sub", "a.sty"), "rb"), b"s")
            self.assertFalse(os.path.exists(os.path.join(dest, "other")))

    def test_dotdot_member_aborts(self):
        with tempfile.TemporaryDirectory() as tmp:
            tar = os.path.join(tmp, "a.tar.xz")
            write_tar(tar, {"doc/x/../../evil.tex": b"evil"})
            with self.assertRaises(SystemExit):
                corpus.extract(tar, "doc/x", os.path.join(tmp, "d"))


class CorpusSubstituteSha(unittest.TestCase):

    def test_substitute_replaces_and_reports_only_hits(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = os.path.join(tmp, "m.tex")
            _w(p, "a SimSun b SimSun c")
            done = corpus.substitute(p, [["SimSun", "Fandol"], ["KaiTi", "FandolKai"]])
            self.assertEqual(done, ["SimSun"])
            self.assertEqual(_r(p), "a Fandol b Fandol c")


class CorpusManifest(unittest.TestCase):

    def test_cases_have_unique_names_and_one_source(self):
        c = corpus.load()
        names = [x["name"] for x in c["cases"]]
        self.assertEqual(len(names), len(set(names)))
        for x in c["cases"]:
            self.assertIn("family", x)
            src = [k for k in ("path", "archive", "fonts") if k in x]
            self.assertEqual(len(src), 1, x["name"])
            if "archive" in x:
                self.assertIn(x["archive"], c["archives"])


class CorpusAssemble(unittest.TestCase):

    def test_path_case_copies_and_writes_meta(self):
        with tempfile.TemporaryDirectory() as tmp:
            meta = corpus.assemble_case(
                {"name": "t1", "family": "fontspec", "path": "tools/unicode-m3/README.md"},
                {}, tmp, os.path.join(tmp, "cache"), None)
            self.assertEqual((meta["family"], meta["trace"]), ("fontspec", "full"))
            self.assertTrue(os.path.exists(os.path.join(tmp, "t1", "t1.tex")))
            idx = json.loads(_r(os.path.join(tmp, "t1", ".m3case.json")))
            self.assertEqual(idx["source_path"], "tools/unicode-m3/README.md")

    def test_path_case_substitutes_and_marks_reference_clean(self):
        with tempfile.TemporaryDirectory() as tmp:
            meta = corpus.assemble_case(
                {"name": "t2", "family": "polyglossia", "reference_clean": True,
                 "path": "tools/xetex-corpus/docs/p04-polyglossia-hebrew/main.tex",
                 "subst": [["{Libertinus Serif}", "{LibertinusSerif}[Extension=.otf]"]]},
                {}, tmp, os.path.join(tmp, "cache"), None)
            self.assertEqual(meta["substituted"], ["{Libertinus Serif}"])
            self.assertIn("{LibertinusSerif}[Extension=.otf]", _r(os.path.join(tmp, "t2", "t2.tex")))
            idx = json.loads(_r(os.path.join(tmp, "t2", ".m3case.json")))
            self.assertTrue(idx["reference_clean"])

    def test_fonts_case_without_fonts_line_is_skipped(self):
        with tempfile.TemporaryDirectory() as tmp:
            meta = corpus.assemble_case(
                {"name": "f", "family": "[fonts]",
                 "fonts": "tools/unicode-m3/fonts-cases/f01-roles"},
                {}, tmp, os.path.join(tmp, "cache"), None)
            self.assertIsNone(meta)
            self.assertFalse(os.path.exists(os.path.join(tmp, "f")))


class FontconfigFiles(unittest.TestCase):

    def test_only_font_extensions_sorted(self):
        with tempfile.TemporaryDirectory() as tmp:
            for n in ("b.TTF", "a.otf", "c.txt", "d.OTC"):
                _w(os.path.join(tmp, n), "")
            got = [os.path.basename(f) for f in fontconfig.files_under(tmp)]
            self.assertEqual(got, ["a.otf", "b.TTF", "d.OTC"])


class FontconfigConfig(unittest.TestCase):

    def _tree(self, tmp, tl_names, sys_names):
        tl, sd = os.path.join(tmp, "tl"), os.path.join(tmp, "sys")
        os.makedirs(tl)
        os.makedirs(sd)
        for d, names in ((tl, tl_names), (sd, sys_names)):
            for n in names:
                _w(os.path.join(d, n), "")
        return tl, sd

    def test_paths_are_xml_escaped(self):
        with tempfile.TemporaryDirectory() as tmp:
            text, _ = fontconfig.config([os.path.join(tmp, "a&b")], os.path.join(tmp, "c&d"), [tmp])
            self.assertIn("a&amp;b", text)
            self.assertIn("c&amp;d", text)


class SummarizeLoad(unittest.TestCase):

    def test_bad_json_is_skipped(self):
        with tempfile.TemporaryDirectory() as tmp:
            _w(os.path.join(tmp, "pt1.json"), "{bad")
            _w(os.path.join(tmp, "ok.json"), '{"x": 1}')
            self.assertEqual([j for _, j in summarize.load([tmp], "*.json")], [{"x": 1}])


class HostStartupFraming(unittest.TestCase):

    def test_frame_layout(self):
        self.assertEqual(host_startup.frame(0x01, b"hi"), b"\x03\x00\x00\x00\x01hi")

    def test_read_frame_round_trip(self):
        a, b = socket.socketpair()
        try:
            a.sendall(host_startup.frame(host_startup.C_HELLO, b'{"x":1}'))
            self.assertEqual(host_startup.read_frame(b), (host_startup.C_HELLO, b'{"x":1}'))
        finally:
            a.close()
            b.close()


class HostSweepEdits(unittest.TestCase):

    def test_five_steps_ending_in_revert(self):
        steps = host_sweep.edits(DOC, 5)
        self.assertEqual([label for label, _ in steps],
                         ["word", "word2", "section", "preamble", "revert"])
        self.assertEqual(steps[-1][1], DOC)
        self.assertIn("Sweep word one.", steps[0][1])

    def test_truncates_and_rejects_markerless_docs(self):
        self.assertEqual(len(host_sweep.edits(DOC, 2)), 2)
        self.assertEqual(host_sweep.edits("no markers here", 5), [])
        self.assertEqual(host_sweep.edits("\\begin{document} only", 5), [])


class HostSweepFrames(unittest.TestCase):

    def test_compared_frames_root_normalised_others_skipped(self):
        def fr(kind, body):
            return struct.pack("<I", len(body) + 1) + bytes([kind]) + body
        with tempfile.TemporaryDirectory() as tmp:
            p = os.path.join(tmp, "s.dl3")
            _w(p, fr(0x45, b"page in " + tmp.encode()) + fr(0x01, b"hello-not-compared")
               + fr(0x43, b"font") + b"\x99\x99", "wb")
            self.assertEqual(host_sweep.frames(p, tmp),
                             [(0x45, b"page in <ROOT>"), (0x43, b"font")])
            self.assertEqual(host_sweep.frames(os.path.join(tmp, "missing.dl3"), tmp), [])


class IdenticalDigest(unittest.TestCase):

    def test_log_digest_ignores_directory_and_banner_lines(self):
        with tempfile.TemporaryDirectory() as tmp:
            a, b = os.path.join(tmp, "a", "x.log"), os.path.join(tmp, "b", "x.log")
            os.makedirs(os.path.dirname(a))
            os.makedirs(os.path.dirname(b))
            _w(a, "This is XeTeX, Version 3.14159\n(dir %s)\n"
                  "(preloaded format=xelatex 2026.10.12)\nok\n" % os.path.join(tmp, "a"))
            _w(b, "This is XeTeX, Version 3.14158-0.99999\n(dir %s)\n"
                  "(preloaded format=xelatex 2026.10.13)\nok\n" % os.path.join(tmp, "b"))
            self.assertEqual(identical.log_digest(a, os.path.join(tmp, "a")),
                             identical.log_digest(b, os.path.join(tmp, "b")))


class LatencySummary(unittest.TestCase):

    def test_best_and_median_with_missing_counts_none(self):
        s = latency.summary([(0, 1.0, 300), (1, 3.0, 100)])
        self.assertEqual(s["exit"], [0, 1])
        self.assertEqual((s["ms_best"], s["ms_median"]), (1000.0, 2000.0))
        self.assertEqual((s["instructions_best"], s["instructions_median"]), (100, 200))
        s = latency.summary([(0, 1.0, None)])
        self.assertIsNone(s["instructions_best"])
        self.assertIsNone(s["instructions_median"])


class NotexParsing(unittest.TestCase):

    def test_lock_values(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = os.path.join(tmp, "flashtex-bundle.lock")
            _w(p, 'url = "https://example.com/b.tar.xz"\ndigest = "deadbeef"\n')
            self.assertEqual(notex_corpus.lock_values(p),
                             ("https://example.com/b.tar.xz", "deadbeef"))

    def test_hello_parses_resolver_line(self):
        with tempfile.TemporaryDirectory() as tmp:
            log = os.path.join(tmp, "host.log")
            _w(log, "noise\nflashtex-host-unicode: {\"resolver\": \"bundle\", \"texlive\": false}\n")
            self.assertEqual(notex_corpus.hello(tmp), {"resolver": "bundle", "texlive": False})
            _w(log, "nothing here\n")
            self.assertIsNone(notex_corpus.hello(tmp))


class ProfileAnnotate(unittest.TestCase):

    def test_rows_costs_commas_and_function_split(self):
        text = ("  1,234 (99.9%)  src/main.rs:main [a.out]\n"
                "  56 ( 0.1%)  some_helper\n"
                "not a row\n")
        self.assertEqual(profile_startup.rows(text), [(1234, "main"), (56, "some_helper")])

    def test_program_totals(self):
        self.assertEqual(profile_startup.total("  5,000 (100.0%)  PROGRAM TOTALS\n"), 5000)
        self.assertIsNone(profile_startup.total("no totals here\n"))

    def test_buckets_best_match_per_pattern_only(self):
        got = profile_startup.buckets([(50, "load_fmt_file"), (200, "load_fmt_file"),
                                       (10, "zzz_quiet")])
        self.assertEqual(got["format: load_fmt_file"]["instructions"], 200)
        self.assertNotIn("init_prim", got)
