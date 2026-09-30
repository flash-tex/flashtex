"""Tests for the parity scoreboard's scoring (pure python, no binaries).

    python3 -m unittest discover -s tools/parity -p 'test_*.py' -v
"""

import gzip
import io
import json
import os
import sys
import tarfile
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import corpus  # noqa: E402
import definers  # noqa: E402
import glyphkeys  # noqa: E402
import parity  # noqa: E402


def rg(name, x, y, font="CMR10", size=10.0, text=None):
    """A reference glyph record as pdftext.page_glyphs makes it."""
    return {"name": name, "x": x, "y_top": y, "font": font, "size": size, "text": text or "?", "advance": 5.0,
            "bt": 1, "code": 0}


def cg(text, x, y, cluster=None, font="LMRoman10-Regular", size=10.0, sources=None):
    """A candidate glyph record as rank.v2_page_glyphs makes it."""
    return {"text": text, "x": x, "y_top": y, "font": font, "size": size, "advance": 5.0, "bt": 0,
            "math": False, "sources": sources or [], "gid": 1, "cluster": cluster}


class GlyphKeys(unittest.TestCase):
    def test_names_map_to_unicode(self):
        self.assertEqual(glyphkeys.name_text("alpha"), "α")
        self.assertEqual(glyphkeys.name_text("ffi"), "ffi")
        self.assertEqual(glyphkeys.name_text("uni2202"), "∂")
        self.assertEqual(glyphkeys.name_text("parenleftBig"), "(")
        self.assertEqual(glyphkeys.name_text("summationdisplay"), "∑")
        self.assertIsNone(glyphkeys.name_text("nosuchglyphname"))

    def test_accented_names_decompose_like_nfkd(self):
        # pdfTeX T1 `eacute` and a candidate precomposed é are one character sequence
        self.assertEqual(glyphkeys.normalize(glyphkeys.name_text("eacute")), glyphkeys.normalize("é"))
        # OT1 composes: `e` + `acute` (spacing accent -> combining)
        self.assertEqual(glyphkeys.normalize("e" + glyphkeys.name_text("acute")), glyphkeys.normalize("é"))

    def test_folds(self):
        self.assertEqual(glyphkeys.normalize("x − y"), "x-y")          # U+2212 minus
        self.assertEqual(glyphkeys.normalize("“q”"), '"q"')
        self.assertEqual(glyphkeys.normalize("𝑥ﬁ"), "xfi")             # math italic, ligature
        self.assertEqual(glyphkeys.normalize("ˆ"), "\u0302")          # modifier circumflex

    def test_candidate_composites_expand_to_pdftex_pieces(self):
        self.assertEqual(glyphkeys.candidate_key({"text": "⟶"}), "-→")
        self.assertEqual(glyphkeys.candidate_key({"text": "↦"}), "↦→")
        self.assertEqual(glyphkeys.candidate_key({"text": "⋯"}), "···")

    def test_unmapped_is_one_token(self):
        k = glyphkeys.reference_key({"name": "weirdglyph", "text": "?"})
        self.assertEqual(glyphkeys.chars(k), ["⟪weirdglyph⟫"])

    def test_nameless_glyph_falls_back_to_text(self):
        self.assertEqual(glyphkeys.reference_key({"name": None, "text": "a"}), "a")


class Atoms(unittest.TestCase):
    def test_ligature_is_two_characters_at_one_origin(self):
        a = parity.reference_atoms([rg("fi", 10, 20)])
        self.assertEqual([x[0] for x in a], ["f", "i"])
        self.assertTrue(all(x[1] == 10 for x in a))

    def test_piece_stack_is_one_atom_with_its_height_allowed(self):
        stack = [rg("parenlefttp", 50, 10, font="CMEX10"), rg("parenleftex", 50, 20, font="CMEX10"),
                 rg("parenleftbt", 50, 30, font="CMEX10"), rg("x", 60, 25, font="CMMI10")]
        a = parity.reference_atoms(stack)
        self.assertEqual([x[0] for x in a], ["(", "x"])
        self.assertGreaterEqual(a[0][4], 20.0)   # stack height + allowance
        self.assertEqual(a[1][4], 0.0)

    def test_candidate_cluster_counted_once(self):
        a = parity.candidate_atoms([cg("(", 50, 10, cluster=(0, 0)), cg("(", 50, 20, cluster=(0, 0)),
                                    cg("x", 60, 25, cluster=(0, 1))])
        self.assertEqual([x[0] for x in a], ["(", "x"])


class Matching(unittest.TestCase):
    def test_exact_and_within_tolerance(self):
        ra = parity.reference_atoms([rg("a", 10, 10), rg("b", 20, 10)])
        ca = parity.candidate_atoms([cg("a", 10.3, 10.2), cg("b", 20, 10)])
        self.assertEqual(parity.match_within(ra, ca), ([], []))

    def test_beyond_tolerance_is_unmatched(self):
        ra = parity.reference_atoms([rg("a", 10, 10)])
        ca = parity.candidate_atoms([cg("a", 10.6, 10)])
        self.assertEqual(parity.match_within(ra, ca), ([0], [0]))

    def test_same_char_nearest_wins(self):
        ra = parity.reference_atoms([rg("e", 10, 10), rg("e", 10.8, 10)])
        ca = parity.candidate_atoms([cg("e", 10.75, 10), cg("e", 10.1, 10)])
        self.assertEqual(parity.match_within(ra, ca), ([], []))

    def test_extension_glyph_is_held_on_x_only(self):
        ra = parity.reference_atoms([rg("summationdisplay", 100, 80, font="CMEX10")])
        ok = parity.candidate_atoms([cg("∑", 100.2, 95, font="LatinModernMath-Regular")])
        off = parity.candidate_atoms([cg("∑", 101, 80, font="LatinModernMath-Regular")])
        self.assertEqual(parity.match_within(ra, ok), ([], []))
        self.assertEqual(parity.match_within(ra, off), ([0], [0]))

    def test_multiset(self):
        ra = parity.reference_atoms([rg("a", 0, 0), rg("b", 5, 0)])
        ca = parity.candidate_atoms([cg("a", 0, 0), cg("c", 5, 0)])
        missing, extra = parity.multiset_diff(ra, ca)
        self.assertEqual(dict(missing), {"b": 1})
        self.assertEqual(dict(extra), {"c": 1})


class Levels(unittest.TestCase):
    def test_cumulative(self):
        self.assertEqual(parity.cumulative_level({"L0": False, "L1": True}), -1)
        self.assertEqual(parity.cumulative_level({"L0": True, "L1": True, "L2": False, "L3": True}), 1)
        self.assertEqual(parity.cumulative_level({"L0": True, "L1": True, "L2": True, "L3": True}), 3)
        self.assertEqual(parity.cumulative_level(dict.fromkeys(parity.LEVELS, True)), 4)

    def test_percentile_nearest_rank(self):
        v = sorted([0.1, 0.2, 0.3, 0.4, 10.0])
        self.assertEqual(parity.percentile(v, 50), 0.3)
        self.assertEqual(parity.percentile(v, 95), 10.0)
        self.assertEqual(parity.dist([]), {"n": 0, "p50": None, "p95": None, "max": None})

    def _r(self, did, level, blockers=(), tier="t", excluded=None, errors=()):
        checks = {name: level >= k for k, name in enumerate(parity.LEVELS)}
        r = {"id": did, "tier": tier, "level": level, "checks": checks, "blockers": list(blockers),
             "_errors": list(errors)}
        if excluded:
            r = {"id": did, "tier": tier, "level": None, "excluded": excluded, "_errors": []}
        return r

    def test_summary_counts_and_excludes(self):
        rs = [self._r("a", 4), self._r("b", 3), self._r("c", 1), self._r("d", -1),
              self._r("e", 0, excluded="oracle: pdflatex exit 1")]
        s = parity.summarize(rs)
        self.assertEqual(s["measured"], 4)
        self.assertEqual(s["excluded"], {"oracle": 1})
        self.assertEqual(s["headline_L3_percent"], 50.0)
        self.assertEqual(s["at_least"]["L0"]["documents"], 3)
        self.assertEqual(s["at_least"]["L4"]["documents"], 1)

    def test_cause_ranking(self):
        rs = [self._r("a", -1, ["compiler: cs \\foo"]),
              self._r("b", -1, ["compiler: cs \\foo", "unknown_command: cs \\bar"]),
              self._r("c", 2, ["placement: first divergence in display math"]),
              self._r("d", 4, [])]
        top = parity.rank_causes(rs)
        self.assertEqual(top[0]["cause"], "compiler: cs \\foo")
        self.assertEqual((top[0]["documents"], top[0]["sole"], top[0]["share"]), (2, 1, 1.5))
        self.assertEqual(top[0]["next_level"], {"L0": 2})
        self.assertEqual({c["cause"] for c in top}, {"compiler: cs \\foo", "unknown_command: cs \\bar",
                                                     "placement: first divergence in display math"})

    def test_blockers_for_each_stage(self):
        diag_err = {"severity": "error", "code": "unknown_command", "message": "\\foo is not supported"}
        warn = {"severity": "warning", "code": "unsupported_feature",
                "message": "packages tikz are recognised but not implemented [help: remove that \\usepackage]"}
        font = {"severity": "warning", "code": "math_resource_profile", "message": "lmmi10: glyphs drawn from ..."}
        r0 = {"level": -1, "candidate": {"pdf": True, "v2": True, "errors": 1, "diagnostics": [diag_err, warn]}}
        self.assertEqual(parity.causes_for(r0, {}), ["unknown_command: cs \\foo"])
        r2 = {"level": 2, "divergence": "environment align (math glyph)",
              "candidate": {"diagnostics": [warn, font]}}
        self.assertEqual(parity.causes_for(r2, {}), ["placement: first divergence in environment align (math glyph)",
                                                     "unsupported_feature: package tikz"])
        r3 = {"level": 3, "candidate": {"diagnostics": [font]}}
        self.assertEqual(parity.causes_for(r3, {}), ["fonts: math_resource_profile"])
        crash = {"level": -1, "candidate": {"pdf": False, "v2": False, "errors": None, "exit": 101,
                                            "stderr_tail": "thread 'main' panicked at src/x.rs:1:2:\nboom"}}
        self.assertTrue(parity.causes_for(crash, {})[0].startswith("engine: crash"))

    def test_baseline_gate(self):
        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, "b.json")
            with open(p, "w") as f:
                json.dump({"levels": {"a": 3, "b": 1, "gone": 0, "excl": None}}, f)
            rs = [self._r("a", 2), self._r("b", 4)]
            self.assertEqual(parity.check_baseline(rs, p), [("a", 3, 2), ("gone", 0, None)])


class Localise(unittest.TestCase):
    SRC = ("\\documentclass{article}\n\\usepackage{amsmath}\n\\begin{document}\nHello \\[ \\left( x \\right) \\]\n"
           "and $a+b$ ok \\textbf{bold}\n\\begin{align} y \\end{align}\n")

    def at(self, frag):
        i = self.SRC.index(frag)
        return parity.localise({"m.tex": self.SRC}, {"path": "m.tex", "start_byte": i, "end_byte": i + len(frag)})

    def test_labels(self):
        self.assertEqual(self.at("\\left("), "display math")
        self.assertEqual(self.at("a+b"), "inline math")
        self.assertEqual(self.at("bold"), "command \\textbf")
        self.assertEqual(self.at(" y "), "environment align")
        self.assertEqual(parity.localise({}, None), "unlocalised")

    def test_diag_causes(self):
        d = {"code": "unsupported_feature",
             "message": "packages listings, foo are recognised but not implemented [help: remove that \\usepackage]"}
        self.assertEqual(parity.diag_causes(d), ["unsupported_feature: package foo",
                                                 "unsupported_feature: package listings"])
        self.assertEqual(parity.diag_causes({"code": "compiler", "message": "LaTeX Error: Command \\x undefined."}),
                         ["compiler: cs \\x"])
        self.assertEqual(parity.diag_causes({"code": "unsupported_feature", "message":
                         "\\setlength requires a recognised dimension, got '2\\p@' [note: neurips_2024.sty is loaded here]"}),
                         ["unsupported_feature: cs \\setlength (dimension argument) <- neurips_<N>.sty"])
        self.assertEqual(parity.diag_causes({"code": "unknown_command", "message":
                         "\\hspace is not supported in math mode [note: this command] [help: \\hspace is a text command]"}),
                         ["unknown_command: cs \\hspace (in math mode)"])

    def test_meta_and_export_diagnostics(self):
        self.assertEqual(parity.diag_causes({"code": "unknown_command",
                                             "message": "further 12 similar diagnostics suppressed"}), [])
        self.assertEqual(parity.diag_causes({"code": "output", "message":
                         "pdf: payload.pages[0].items[0]: font Times-Roman (e0272e1c) is format core14-afm, which carries"}),
                         ["output: base-14 font Times-Roman has no program to embed (exact PDF route refuses it)"])

    def test_context_and_loaded_file_grouping(self):
        facts = {"class": "article", "packages": [], "local_files": ["neurips_2024.sty"]}
        idx = {"cs": {"hspace": ["latex.ltx"]}, "env": {}, "requires": {}}
        g = lambda k: definers.group_of(k, facts, idx)  # noqa: E731
        self.assertEqual(g("unsupported_feature: cs \\setlength (dimension argument) <- neurips_<N>.sty"),
                         "project .sty/.cls (read, incompletely)")
        self.assertEqual(g("unknown_command: cs \\x <- lipics-v2021.cls"), "class lipics-v2021")
        self.assertEqual(g("unknown_command: cs \\hspace (in math mode)"), "LaTeX kernel: \\hspace (in math mode)")


class Definers(unittest.TestCase):
    INDEX = {"cs": {"text": ["amstex.sty", "amstext.sty"], "subjclass": ["amsart.cls"],
                    "raisebox": ["hyperref.sty", "latex.ltx"], "myop": ["hyperref.sty"],
                    "author": ["latex.ltx", "amsart.cls"],
                    "autoref": ["hyperref.sty"], "arrow": ["tikzlibrarycd.code.tex"]},
             "env": {"alignat": ["amsmath.sty"]},
             "requires": {"amsart": ["amsmath", "amstex"], "amsmath": ["amstext"], "tikz-cd": ["tikzlibrarycd.code"]},
             "kernel_registers": ["textwidth"]}
    FACTS = {"class": "amsart", "packages": ["amsmath", "tikz-cd"], "cs_tex": ["myop"], "cs_sty": ["styop"],
             "env_tex": [], "env_sty": []}

    def test_scan_finds_definitions_and_loads(self):
        cs, env = {}, {}
        definers._scan(b"\\newcommand{\\foo}[1]{x}\\def\\bar#1{y}\\newenvironment{baz}{}{}\\def\\qux{}\\def\\endqux{}",
                       cs, env, "t.sty")
        self.assertTrue({"foo", "bar", "qux", "endqux"} <= set(cs))
        self.assertTrue({"baz", "qux"} <= set(env))
        cs = {}
        definers._scan(b"\\global\\let\\author\\relax\\let\\copy\\orig\\DeclareTextFontCommand{\\texttt}{x}",
                       cs, {}, "a.cls")
        self.assertEqual(set(cs), {"copy", "texttt"})
        self.assertEqual(definers._requires(b"\\RequirePackage[x]{a,b}\\usetikzlibrary{cd}"),
                         {"a", "b", "tikzlibrarycd.code"})

    def test_grouped_by_what_the_document_loads(self):
        g = lambda k: definers.group_of(k, self.FACTS, self.INDEX)  # noqa: E731
        self.assertEqual(g("unknown_command: cs \\subjclass"), "class amsart")
        self.assertEqual(g("unsupported_feature: cs \\text"), "package amsmath")   # amstext via amsmath, not amstex via the class
        self.assertEqual(g("syntax_error: env alignat"), "package amsmath")
        self.assertEqual(g("compiler: cs \\arrow"), "package tikz-cd")
        self.assertEqual(g("compiler: cs \\raisebox"), "LaTeX kernel: \\raisebox")   # kernel before the patching package
        self.assertEqual(g("compiler: cs \\autoref"), "compiler: cs \\autoref")   # hyperref is not loaded
        self.assertFalse(definers.KERNEL_FILES.match("ltxfront.sty"))
        self.assertTrue(definers.KERNEL_FILES.match("latex-lab-amsmath.ltx"))
        self.assertEqual(g("compiler: cs \\myop"), "project macro (defined in the document)")
        self.assertEqual(g("compiler: cs \\styop"), "project .sty/.cls (read, incompletely)")
        self.assertEqual(g("unsupported_feature: package listings"), "package listings")
        self.assertEqual(g("syntax_error: cs \\author"), "class amsart")               # the class redefines it
        self.assertEqual(g("unsupported_feature: cs \\vskip"), "TeX primitive")
        self.assertEqual(g("unknown_command: cs \\textwidth"), "LaTeX kernel register (length/skip/count)")
        self.assertEqual(g("placement: first divergence in display math"), "placement: first divergence in display math")


class Corpus(unittest.TestCase):
    def _targz(self, files):
        buf = io.BytesIO()
        with tarfile.open(fileobj=buf, mode="w:gz") as tf:
            for name, data in files.items():
                ti = tarfile.TarInfo(name)
                ti.size = len(data)
                tf.addfile(ti, io.BytesIO(data))
        return buf.getvalue()

    def test_unpack_and_entry(self):
        doc = b"\\documentclass{article}\n\\begin{document}\nx\n\\end{document}\n"
        data = self._targz({"paper.tex": doc, "sec/intro.tex": b"intro", "../evil.tex": b"no"})
        with tempfile.TemporaryDirectory() as d:
            dest = os.path.join(d, "src")
            self.assertEqual(corpus.unpack(data, dest), "tar.gz")
            self.assertFalse(os.path.exists(os.path.join(d, "evil.tex")))
            self.assertTrue(os.path.isfile(os.path.join(dest, "sec", "intro.tex")))
            self.assertEqual(corpus.detect_entry(dest), "paper.tex")

    def test_single_gz_and_pdf(self):
        doc = b"\\documentclass{article}\n\\begin{document}\nx\n\\end{document}\n"
        with tempfile.TemporaryDirectory() as d:
            self.assertEqual(corpus.unpack(gzip.compress(doc), os.path.join(d, "a")), "gz")
            self.assertEqual(corpus.detect_entry(os.path.join(d, "a")), "main.tex")
            self.assertEqual(corpus.unpack(b"%PDF-1.5 ...", os.path.join(d, "b")), "pdf")

    def test_entry_ignores_commented_documentclass(self):
        with tempfile.TemporaryDirectory() as d:
            with open(os.path.join(d, "a.tex"), "w") as f:
                f.write("% \\documentclass{article}\n% \\begin{document}\n")
            with open(os.path.join(d, "b.tex"), "w") as f:
                f.write("\\documentclass{amsart}\n\\begin{document}\n\\end{document}\n")
            self.assertEqual(corpus.detect_entry(d), "b.tex")


import capture  # noqa: E402
import shutil  # noqa: E402
import subprocess  # noqa: E402
import tiers  # noqa: E402

BOX = ("Completed box being shipped out [1]\n\\vbox(633.0+0.0)x407.0\n.\\glue 16.0\n.\\hbox(6.94+0.0)x407.0\n"
       "..\\OT1/cmr/m/n/10 H")


class PTOne(unittest.TestCase):
    def test_normalise_log_drops_banner_and_paths(self):
        raw = ("This is pdfTeX, Version 3.141592653-2.6-1.40.29 (TeX Live 2026) (preloaded format=pdflatex)\n"
               " restricted \\write18 enabled.\n**\\tracingall\\input{main.tex}\n(/w/d/main.tex\n"
               "(/w/d/sub/a.tex)\nOutput written on main.pdf (1 page, 12345 bytes).\n")
        out = capture.normalise_log(raw, "/w/d")
        self.assertTrue(out.startswith("**\\tracingall"))
        self.assertIn("(<WORKDIR>/main.tex", out)
        self.assertIn("(<WORKDIR>/sub/a.tex)", out)
        self.assertIn("Output written on main.pdf (1 page, 12345 bytes).", out)  # accounting is split later

    def test_split_boxes_at_every_shipout(self):
        log = f"{{into \\vsize=633.0}}\n\n{BOX}\n\nMemory usage before: 1; after: 1\n\n{BOX.replace('[1]', '[2]')}\n\n"
        boxes = capture.split_boxes(log)
        self.assertEqual(len(boxes), 2)
        self.assertEqual(boxes[0], BOX)
        self.assertTrue(boxes[1].startswith("Completed box being shipped out [2]"))

    def test_compare_equal_and_first_difference(self):
        a = capture.Capture("x\n" + BOX + "\n\ny", [BOX], "a.pdf")
        self.assertTrue(tiers.compare_pt1(a, a)["ok"])
        other = BOX.replace("H", "I")
        b = capture.Capture("x\n" + other + "\n\ny", [other], "b.pdf")
        r = tiers.compare_pt1(a, b)
        self.assertFalse(r["ok"])
        self.assertEqual((r["first_shipout"], r["box_line"]["line"]), (1, 5))
        self.assertEqual(r["log_line"]["line"], 6)
        c = capture.Capture("x\n" + BOX + "\n\nz", [BOX], "c.pdf")  # same boxes, a log line differs
        r = tiers.compare_pt1(a, c)
        self.assertEqual((r["ok"], r["boxes_equal"], r["log_line"]["line"]), (False, True, 8))
        d = capture.Capture(a.log, [BOX, BOX], "d.pdf")  # an extra shipout
        self.assertEqual(tiers.compare_pt1(a, d)["first_shipout"], 2)

    def test_engine_kind_from_version_line(self):
        with tempfile.TemporaryDirectory() as d:
            for name, first, kind in (("f", "flashtex 0.1.0 (abc)", "flashtex-cli"),
                                      ("p", "pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026)", "tex")):
                p = os.path.join(d, name)
                with open(p, "w") as f:
                    f.write(f"#!/bin/sh\necho '{first}'\n")
                os.chmod(p, 0o755)
                self.assertEqual(tiers.engine_kind(p), kind)


ACC_TAIL = ("{vertical mode: \\end}\n ) \nHere is how much of TeX's memory you used:\n 492 strings out of 467525\n"
            " 39i,8n,41p,191b,208s stack positions out of 10000i,1000n,20000p,200000b,200000s\n"
            "</usr/local/texlive/2026/texmf-dist/fonts/type1/public/amsfonts/cm/cmr10.pfb>\n"
            "Output written on main.pdf (3 pages, 151150 bytes).\nPDF statistics:\n"
            " 75 PDF objects out of 1000 (max. 8388607)\n 45 compressed objects within 1 object stream\n"
            " 0 named destinations out of 1000 (max. 500000)\n"
            " 1 words of extra memory for PDF output out of 10000 (max. 10000000)\n")
ACC_MEM = "Memory usage before: 7222&399359; after: 4346&398095; still untouched: 4559244"


def acc_capture(box=BOX, mem=ACC_MEM, tail=ACC_TAIL):
    return capture.Capture(f"**\\tracingall\n{box}\n\n{mem}\n{tail}", [box], "x.pdf")


class PTAccounting(unittest.TestCase):
    """DESIGN §1.1 N2 ruling: only end-of-run accounting leaves P-T1, and a
    block ends at the first line that does not start with a space."""

    def test_split_removes_exactly_the_accounting(self):
        strict, acc = capture.split_accounting(acc_capture().log)
        self.assertEqual(len(acc), 10)
        self.assertEqual(acc[0], ACC_MEM)
        self.assertIn("Output written on main.pdf (3 pages, <BYTES> bytes).", strict)
        self.assertIn("cmr10.pfb>", strict)  # not accounting: stays compared
        self.assertIn(" ) ", strict)  # an indented line outside a block stays compared
        self.assertNotIn("strings out of", strict)
        self.assertNotIn("PDF objects", strict)

    def test_identical(self):
        r = tiers.compare_pt1(acc_capture(), acc_capture())
        self.assertTrue(r["ok"])
        self.assertEqual(r["accounting"], {"lines": [10, 10], "equal": True})

    def test_byte_count_only_passes_and_is_reported(self):
        r = tiers.compare_pt1(acc_capture(), acc_capture(tail=ACC_TAIL.replace("151150 bytes", "151187 bytes")))
        self.assertTrue(r["ok"])
        self.assertFalse(r["accounting"]["equal"])
        self.assertIn("151187", r["accounting"]["first"]["candidate"])

    def test_memory_accounting_passes_and_is_reported(self):
        r = tiers.compare_pt1(acc_capture(), acc_capture(mem=ACC_MEM.replace("7222", "7000"),
                                                         tail=ACC_TAIL.replace(" 492 strings", " 500 strings")))
        self.assertTrue(r["ok"])
        self.assertFalse(r["accounting"]["equal"])

    def test_junk_line_after_a_block_fails(self):
        for tail in (ACC_TAIL + "{\\glue 3.0}\n",  # after PDF statistics (the #1191 trap)
                     ACC_TAIL.replace(" stack positions out of 10000i,1000n,20000p,200000b,200000s\n",
                                      " stack positions out of 10000i,1000n,20000p,200000b,200000s\n{\\glue 3.0}\n")):
            r = tiers.compare_pt1(acc_capture(), acc_capture(tail=tail))
            self.assertFalse(r["ok"])
            self.assertEqual(r["log_line"]["candidate"], "{\\glue 3.0}")

    def test_indented_junk_inside_or_after_each_block_fails(self):
        mem_last = " 39i,8n,41p,191b,208s stack positions out of 10000i,1000n,20000p,200000b,200000s\n"
        pdf_first = " 75 PDF objects out of 1000 (max. 8388607)\n"
        pdf_last = " 1 words of extra memory for PDF output out of 10000 (max. 10000000)\n"
        for junk in (" junk", " Overfull \\hbox (1.0pt too wide) in paragraph at lines 3--4"):
            for tail in (ACC_TAIL.replace(" 492 strings out of 467525\n", f" 492 strings out of 467525\n{junk}\n"),
                         ACC_TAIL.replace(mem_last, f"{mem_last}{junk}\n"),
                         ACC_TAIL.replace(pdf_first, f"{pdf_first}{junk}\n"),
                         ACC_TAIL.replace(pdf_last, f"{pdf_last}{junk}\n")):
                self.assertNotEqual(tail, ACC_TAIL)
                r = tiers.compare_pt1(acc_capture(), acc_capture(tail=tail))
                self.assertFalse(r["ok"], (junk, tail))
                self.assertEqual(r["log_line"]["candidate"], junk)

    def test_block_lines_out_of_order_or_repeated_fail(self):
        line = " 492 strings out of 467525\n"
        r = tiers.compare_pt1(acc_capture(), acc_capture(tail=ACC_TAIL.replace(line, line + line)))
        self.assertFalse(r["ok"])
        a, b = " 75 PDF objects out of 1000 (max. 8388607)", " 0 named destinations out of 1000 (max. 500000)"
        swapped = ACC_TAIL.replace(a, "\x00").replace(b, a).replace("\x00", b)
        self.assertNotEqual(swapped, ACC_TAIL)
        r = tiers.compare_pt1(acc_capture(), acc_capture(tail=swapped))
        self.assertFalse(r["ok"])  # pdfTeX's order is part of the shape

    def test_header_mid_log_or_twice_fails(self):
        block = "PDF statistics:\n 75 PDF objects out of 1000 (max. 8388607)\n"
        mid = capture.Capture(f"**\\tracingall\n{block}{BOX}\n\n{ACC_MEM}\n{ACC_TAIL}", [BOX], "x.pdf")
        r = tiers.compare_pt1(acc_capture(), mid)
        self.assertFalse(r["ok"])
        self.assertEqual(r["log_line"]["candidate"], "PDF statistics:")
        r = tiers.compare_pt1(acc_capture(), acc_capture(tail=ACC_TAIL + block))
        self.assertFalse(r["ok"])
        r = tiers.compare_pt1(acc_capture(), acc_capture(tail=ACC_TAIL.replace(
            "Output written", "Output written on x.pdf (3 pages, 1 bytes).\nOutput written")))
        self.assertFalse(r["ok"])

    def test_nested_shipouts_each_owe_one_memory_line(self):
        # beamer: a \shipout inside another's prints two headers, then two lines
        nested = capture.Capture(f"**\n{BOX}\n\n{BOX}\n\n{ACC_MEM}\n{ACC_MEM}\n{ACC_TAIL}", [BOX, BOX], "x.pdf")
        strict, acc = capture.split_accounting(nested.log)
        self.assertNotIn("Memory usage", strict)
        self.assertEqual(sum(1 for a in acc if a.startswith("Memory usage")), 2)

    def test_second_memory_usage_line_after_one_shipout_fails(self):
        r = tiers.compare_pt1(acc_capture(), acc_capture(mem=ACC_MEM + "\n" + ACC_MEM))
        self.assertFalse(r["ok"])

    def test_junk_on_a_memory_usage_line_fails(self):
        r = tiers.compare_pt1(acc_capture(), acc_capture(mem=ACC_MEM + " \\glue 3.0"))
        self.assertFalse(r["ok"])

    def test_one_sp_glue_change_fails(self):
        r = tiers.compare_pt1(acc_capture(), acc_capture(box=BOX.replace("\\glue 16.0", "\\glue 16.00002")))
        self.assertFalse(r["ok"])
        self.assertFalse(r["boxes_equal"])
        self.assertEqual(r["box_line"]["candidate"], ".\\glue 16.00002")

    def test_page_count_change_fails(self):
        r = tiers.compare_pt1(acc_capture(), acc_capture(tail=ACC_TAIL.replace("(3 pages", "(4 pages")))
        self.assertFalse(r["ok"])
        self.assertIn("(4 pages, <BYTES> bytes)", r["log_line"]["candidate"])


class PTSummary(unittest.TestCase):
    def test_counts_and_not_applicable(self):
        rs = [{"id": "a", "pt": {"P-T1": None, "P-T2": True, "why": {"P-T1": parity.NOT_TEX}}},
              {"id": "b", "pt": {"P-T1": None, "P-T2": False, "why": {"P-T1": parity.NOT_TEX}}},
              {"id": "c", "pt": {"P-T1": None, "P-T2": None, "why": {}, "excluded": "oracle: exit 1"}}]
        s = parity.summarize_pt(rs)
        self.assertEqual((s["P-T2"]["evaluated"], s["P-T2"]["passed"], s["P-T2"]["percent"]), (2, 1, 50.0))
        self.assertEqual(s["P-T1"]["evaluated"], 0)
        self.assertTrue(s["P-T1"]["not_evaluated"].startswith("n/a: the flashtex CLI"))
        self.assertEqual(s["excluded"], {"oracle": 1})
        self.assertEqual(parity.pt_cell({"pt": s}, "P-T1"), "n/a")
        self.assertEqual(parity.pt_cell({"pt": s}, "P-T2"), "1/2 (50.0%)")

    def test_where(self):
        pt = {"P-T1": True, "P-T2": False, "pt2": {"fonts_equal": True, "first_page": {
            "page": 2, "differs": ["content"], "content_line": {"line": 7}}}}
        self.assertEqual(parity.pt_where(pt), "P-T2: page 2 content line 7")
        pt = {"P-T1": False, "pt1": {"boxes_equal": False, "first_shipout": 3, "shipouts": [4, 4],
                                     "box_line": {"line": 9}}}
        self.assertEqual(parity.pt_where(pt), "P-T1: shipout 3 of [4, 4], box line 9")


PDFTEX = tiers.DEFAULT_ORACLE if os.path.isfile(tiers.DEFAULT_ORACLE) else None
DOC = r"""\documentclass{article}
\begin{document}
%s
\newpage
Second page, $x^2 + y_1$.
\end{document}
"""


@unittest.skipUnless(PDFTEX and shutil.which("qpdf"), "needs pdfTeX and qpdf")
class PTWithOracle(unittest.TestCase):
    """End to end against the real pdfTeX: the self-test property on a tiny
    document, object renumbering, and that real differences are caught."""

    def setUp(self):
        self.d = tempfile.mkdtemp()

    def tearDown(self):
        shutil.rmtree(self.d, ignore_errors=True)

    def build(self, name, body):
        src = os.path.join(self.d, name, "src")
        os.makedirs(src)
        with open(os.path.join(src, "main.tex"), "w") as f:
            f.write(DOC % body)
        return tiers.run_tex({"dir": src, "entry": "main.tex"}, PDFTEX, os.path.join(self.d, name, "run"))

    def test_self_and_differences(self):
        m1, c1, p1 = self.build("a", "Hello world.")
        m2, c2, p2 = self.build("b", "Hello world.")
        m3, c3, p3 = self.build("c", "Hello wordl.")
        m4, c4, p4 = self.build("d", "Hello world!")  # '!' is a glyph the others' cmr10 subset lacks
        self.assertTrue(m1["ok"] and m2["ok"] and m3["ok"] and m4["ok"])
        self.assertEqual(len(c1.boxes), 2)
        self.assertTrue(tiers.compare_pt1(c1, c2)["ok"])
        self.assertTrue(tiers.compare_pt2(p1, p2, self.d)["ok"])
        r1 = tiers.compare_pt1(c1, c3)
        self.assertFalse(r1["ok"])
        self.assertEqual(r1["first_shipout"], 1)
        r2 = tiers.compare_pt2(p1, p3, self.d)
        self.assertFalse(r2["content_equal"])
        self.assertEqual(r2["first_page"]["page"], 1)
        r4 = tiers.compare_pt2(p1, p4, self.d)
        self.assertFalse(r4["fonts_equal"])
        self.assertIn("CMR10", r4["fonts"]["different_program"])

    def wrapper(self, name, edit):
        """A candidate engine: pdfTeX, then `edit` (a Python expression over
        `t`, the log text) applied to every .log in its working directory."""
        p = os.path.join(self.d, name)
        with open(p, "w") as f:
            f.write(f"#!{sys.executable}\nimport glob, re, subprocess, sys\n"
                    f"rc = subprocess.run([{PDFTEX!r}] + sys.argv[1:]).returncode\n"
                    "for n in glob.glob('*.log'):\n"
                    "    t = open(n, encoding='latin-1').read()\n"
                    f"    open(n, 'w', encoding='latin-1').write({edit})\n"
                    "sys.exit(rc)\n")
        os.chmod(p, 0o755)
        return p

    def test_accounting_ruling_end_to_end(self):
        src = os.path.join(self.d, "src")
        os.makedirs(src)
        with open(os.path.join(src, "main.tex"), "w") as f:
            f.write(DOC % "Hello world.")
        doc = {"dir": src, "entry": "main.tex"}
        _, ref, _ = tiers.run_tex(doc, PDFTEX, os.path.join(self.d, "ref"))
        cases = {
            "bytes": ("re.sub(r'(pages?), \\d+ bytes', r'\\1, 99 bytes', t)", True),
            "junk": ("t + '{\\\\glue 3.0}\\n'", False),
            "pages": ("re.sub(r'\\(\\d+ pages', '(9 pages', t)", False),
        }
        for name, (edit, ok) in cases.items():
            _, cand, _ = tiers.run_tex(doc, self.wrapper(name, edit), os.path.join(self.d, "run-" + name))
            r = tiers.compare_pt1(ref, cand)
            self.assertEqual(r["ok"], ok, (name, r))
            if name == "bytes":
                self.assertFalse(r["accounting"]["equal"])

    def test_shell_escape_is_the_engines_default_mode(self):
        """Owner decision #1209: no shell-escape flag, so pdfTeX runs in TeX
        Live's default mode: restricted, \\pdfshellescape=2 (l3kernel's
        \\sys_if_shell reads it), with the ` restricted \\write18 enabled.`
        status line, including in the fixtures where it changes the trace."""
        self.assertIsNone(capture.SHELL_ESCAPE)
        src = os.path.join(self.d, "se")
        os.makedirs(src)
        with open(os.path.join(src, "main.tex"), "w") as f:
            f.write(DOC % r"\typeout{SHELLESCAPE=\the\pdfshellescape}")
        m, cap, _ = tiers.run_tex({"dir": src, "entry": "main.tex"}, PDFTEX, os.path.join(self.d, "se-run"))
        self.assertTrue(m["ok"])
        self.assertIn("SHELLESCAPE=2", cap.log)
        try:  # an explicit override reaches every run; `default` means no flag again
            parity.set_shell_escape("-no-shell-escape")
            _, cap0, _ = tiers.run_tex({"dir": src, "entry": "main.tex"}, PDFTEX, os.path.join(self.d, "se-off"))
            self.assertIn("SHELLESCAPE=0", cap0.log)
        finally:
            parity.set_shell_escape("default")
        self.assertIsNone(capture.SHELL_ESCAPE)
        for fx in ("real-world/hyperref-toc", "real-world/conf-paper"):
            d = os.path.join(parity.REPO, "fixtures", fx)
            if not os.path.isdir(d):
                continue
            work = os.path.join(self.d, fx.replace("/", "_"))
            m, _, _ = tiers.run_tex({"dir": d, "entry": "main.tex"}, PDFTEX, work, trace=False)
            self.assertTrue(m["ok"], fx)
            with open(os.path.join(work, "main.log"), encoding="latin-1") as f:
                self.assertIn("\n restricted \\write18 enabled.\n", f.read(), fx)

    def recorder(self, name):
        """A candidate engine that records its argv and FLASHTEX_FORMATS, then
        runs pdfTeX (by its full path, so kpathsea finds TeX Live)."""
        p = os.path.join(self.d, name)
        rec = os.path.join(self.d, name + ".rec")
        with open(p, "w") as f:
            f.write(f"#!{sys.executable}\nimport json, os, sys\n"
                    f"open({rec!r}, 'a').write(json.dumps([sys.argv, os.environ.get('FLASHTEX_FORMATS')]) + '\\n')\n"
                    f"os.execv({PDFTEX!r}, [{PDFTEX!r}] + sys.argv[1:])\n")
        os.chmod(p, 0o755)
        return p, rec

    def test_extra_env_reaches_the_candidate_only(self):
        src = os.path.join(self.d, "env")
        os.makedirs(src)
        with open(os.path.join(src, "main.tex"), "w") as f:
            f.write(DOC % "Hello.")
        doc = {"dir": src, "entry": "main.tex"}
        eng, rec = self.recorder("cand-engine")
        old = os.environ.get("FLASHTEX_FORMATS")
        os.environ["FLASHTEX_FORMATS"] = "/inherited/must/not/leak"
        try:
            m, cap, _ = tiers.run_tex(doc, eng, os.path.join(self.d, "cand"), extra_env={"FLASHTEX_FORMATS": "/fmts"})
            self.assertTrue(m["ok"])
            m, _, _ = tiers.run_tex(doc, eng, os.path.join(self.d, "orac"), trace=False)  # as the oracle runs
            self.assertTrue(m["ok"])
        finally:
            if old is None:
                os.environ.pop("FLASHTEX_FORMATS")
            else:
                os.environ["FLASHTEX_FORMATS"] = old
        with open(rec) as f:
            runs = [json.loads(ln) for ln in f]
        cand, orac = runs[:-m["passes"]], runs[-m["passes"]:]
        self.assertTrue(cand and orac)
        self.assertTrue(all(env == "/fmts" for _, env in cand))  # every pass, the traced one included
        self.assertTrue(all(env is None for _, env in orac))
        for argv, _ in runs:  # (a script's sys.argv[0] is its path; argv[0] is tested with real pdfTeX below)
            self.assertEqual(argv[1], "-fmt=pdflatex")  # no shell-escape flag by default
            self.assertFalse([a for a in argv if "shell" in a])

    def test_warnings_print_the_same_program_name(self):
        """pdfTeX prints argv[0] as given in warnings; every engine runs as
        `pdftex`, so an engine binary called something else is invisible."""
        src = os.path.join(self.d, "warn")
        os.makedirs(src)
        _, _, pdf = self.build("fig", "Figure.")
        subprocess.run(["qpdf", "--force-version=2.0", pdf, os.path.join(src, "fig.pdf")], check=True)
        with open(os.path.join(src, "main.tex"), "w") as f:
            f.write("\\documentclass{article}\\usepackage{graphicx}\\begin{document}"
                    "\\includegraphics{fig.pdf}\\end{document}\n")
        other = os.path.join(self.d, "otherengine")
        os.symlink(PDFTEX, other)
        doc = {"dir": src, "entry": "main.tex"}
        _, ref, _ = tiers.run_tex(doc, PDFTEX, os.path.join(self.d, "w-ref"))
        _, cand, _ = tiers.run_tex(doc, other, os.path.join(self.d, "w-cand"))
        self.assertIn("pdfTeX warning: pdftex (file ./fig.pdf)", cand.log)
        self.assertNotIn("otherengine", cand.log)
        self.assertTrue(tiers.compare_pt1(ref, cand)["ok"])

    def test_object_renumbering_is_invisible(self):
        _, _, p1 = self.build("a", "Hello world.")
        lin = os.path.join(self.d, "renumbered.pdf")
        subprocess.run(["qpdf", "--linearize", "--object-streams=generate", p1, lin], check=True)
        self.assertTrue(tiers.compare_pt2(p1, lin, self.d)["ok"])


class NightlySelection(unittest.TestCase):
    """T4: which documents a shard scores, and which are traced for P-T1."""

    def args(self, **kw):
        import argparse
        return argparse.Namespace(**dict({"limit": 0, "spread": 0, "shard": None}, **kw))

    def test_shards_partition_the_tier(self):
        ids = [f"d{i}" for i in range(103)]
        shards = [parity.select_documents(ids, self.args(shard=(k, 5))) for k in range(5)]
        self.assertEqual(sorted(sum(shards, [])), sorted(ids))
        self.assertEqual(shards[1][:3], ["d1", "d6", "d11"])
        self.assertLessEqual(max(map(len, shards)) - min(map(len, shards)), 1)

    def test_spread_then_shard(self):
        ids = [f"d{i}" for i in range(100)]
        spread = parity.select_documents(ids, self.args(spread=10))
        self.assertEqual(spread, [f"d{i}" for i in range(0, 100, 10)])
        both = [parity.select_documents(ids, self.args(spread=10, shard=(k, 3))) for k in range(3)]
        self.assertEqual(sorted(sum(both, [])), sorted(spread))

    def test_parse_shard(self):
        self.assertEqual(parity.parse_shard("2/7"), (2, 7))
        for bad in ("7/7", "x", "1/0", "-1/3"):
            with self.assertRaises(Exception):
                parity.parse_shard(bad)

    def test_pt1_sample_is_fixed_and_about_the_fraction(self):
        docs = [{"tier": "nightly-5k", "id": f"2001.{i:05d}v1"} for i in range(4000)]
        picked = [d for d in docs if parity.in_pt1_sample(d, 0.05)]
        self.assertTrue(150 < len(picked) < 250, len(picked))
        self.assertEqual(picked, [d for d in docs if parity.in_pt1_sample(d, 0.05)])
        self.assertFalse(any(parity.in_pt1_sample(d, 0.0) for d in docs))
        self.assertTrue(all(parity.in_pt1_sample(d, 1.0) for d in docs))

    def test_pt1_wanted(self):
        cfg = {"engine_kind": "tex", "pt": "on", "pt1_sample": {"nightly-5k": 0.0}}
        self.assertEqual(parity.pt1_wanted({"tier": "arxiv", "id": "x"}, cfg), (True, None))
        trace, why = parity.pt1_wanted({"tier": "nightly-5k", "id": "x"}, cfg)
        self.assertFalse(trace)
        self.assertIn("outside the P-T1 sample", why)
        self.assertEqual(parity.pt1_wanted({"tier": "arxiv", "id": "x"}, dict(cfg, pt="pt2")), (False, None))
        self.assertEqual(parity.pt1_wanted({"tier": "arxiv", "id": "x"}, dict(cfg, engine_kind="flashtex-cli")),
                         (False, None))

    def test_parse_pt1_sample(self):
        self.assertEqual(parity.parse_pt1_sample(["nightly-5k=0.05", "arxiv=1"]), {"nightly-5k": 0.05, "arxiv": 1.0})
        for bad in (["x"], ["x=2"], ["=0.1"]):
            with self.assertRaises(Exception):
                parity.parse_pt1_sample(bad)

    def test_grid_window_is_reproducible_and_inside_the_year(self):
        a = corpus.grid_window("s", "math.AG", 2020, 14)
        self.assertEqual(a, corpus.grid_window("s", "math.AG", 2020, 14))
        self.assertNotEqual(a, corpus.grid_window("s", "math.AG", 2021, 14))
        for y in range(2016, 2026):
            lo, hi = corpus.grid_window("seed", "hep-th", y, 14)
            self.assertTrue(lo.startswith(str(y)) and hi.startswith(str(y)), (lo, hi))
            self.assertTrue(lo.endswith("0000") and hi.endswith("2359"))

    def test_on_demand_manifests_are_left_out_of_a_bare_fetch(self):
        with tempfile.TemporaryDirectory() as d:
            for name, extra in (("small.json", {}), ("big.json", {"on_demand": True})):
                with open(os.path.join(d, name), "w") as f:
                    json.dump(dict({"tier": name[:-5], "entries": [{"id": "1/a"}]}, **extra), f)
            old = corpus.MANIFEST_DIR
            corpus.MANIFEST_DIR = d
            try:
                self.assertEqual([os.path.basename(m) for m in corpus.manifests()], ["small.json"])
                self.assertEqual([os.path.basename(m) for m in corpus.manifests(include_on_demand=True)],
                                 ["big.json", "small.json"])
                self.assertEqual(corpus.manifest_tiers(), ["big", "small"])
                self.assertEqual(corpus.manifest_ids(os.path.join(d, "big.json")), ["1_a"])
            finally:
                corpus.MANIFEST_DIR = old


class TracedLogCap(unittest.TestCase):
    """A traced log over --pt1-max-log-mb stops the run and is never read."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.d = self.tmp.name
        self.engine = os.path.join(self.d, "fake-engine")
        with open(self.engine, "w") as f:
            # writes <jobname>.log without end, like a trace of many GB, and a PDF
            f.write("#!/bin/sh\nfor a in \"$@\"; do case $a in -jobname=*) j=${a#-jobname=};; esac; done\n"
                    "printf '%%PDF-1.4' > \"$j.pdf\"\n"
                    "while :; do echo 'a traced line of about sixty characters, again and again' >> \"$j.log\"; "
                    "done\n")
        os.chmod(self.engine, 0o755)
        with open(os.path.join(self.d, "main.tex"), "w") as f:
            f.write("x\n")

    def tearDown(self):
        self.tmp.cleanup()

    def test_stopped_at_the_cap_and_not_read(self):
        cap = capture.capture(os.path.join(self.d, "main.tex"), self.engine, self.d, max_log_bytes=200_000)
        self.assertIsNone(cap.log)
        self.assertIsNone(cap.boxes)
        self.assertGreater(cap.oversize, 200_000)
        self.assertLess(cap.oversize, 50_000_000)
        self.assertFalse(os.path.exists(os.path.join(self.d, "main.log")))

    def cfg(self):
        return {"pt": "on", "oracle_pdftex": "pdftex", "engine_kind": "tex", "cache": self.d, "qpdf": False,
                "pt1_max_log": 1 << 20}

    def score_pt(self, ref_cap, cand_cap):
        old = tiers.oracle
        tiers.oracle = lambda *a, **k: ({"ok": True}, ref_cap, "ref.pdf")
        try:
            return parity.score_pt({"tier": "arxiv", "id": "x", "dir": self.d, "entry": "main.tex"}, self.cfg(),
                                   {"pdf": None, "capture": cand_cap}, self.d)
        finally:
            tiers.oracle = old

    def test_oracle_over_the_cap_is_not_evaluated(self):
        pt = self.score_pt(capture.Capture(None, None, "ref.pdf", 5 << 20), capture.Capture("log", [], "c.pdf"))
        self.assertIsNone(pt["P-T1"])
        self.assertIn("not evaluated", pt["why"]["P-T1"])

    def test_only_the_candidate_over_the_cap_fails(self):
        pt = self.score_pt(capture.Capture("log", [], "ref.pdf"), capture.Capture(None, None, None, 5 << 20))
        self.assertIs(pt["P-T1"], False)
        self.assertEqual(parity.candidate_log_cap(self.cfg()), (1 << 20) + (1 << 18))

    def test_memory_bound(self):
        import argparse
        import nightly
        a = argparse.Namespace(jobs=8, pt1_jobs=2, pt1_max_log_mb=512)
        self.assertAlmostEqual(nightly.memory_bound_gib(a), 8 * nightly.WORKER_GIB + 2 * 9 * 0.5)
        # no --pt1-jobs: every worker may trace
        self.assertAlmostEqual(nightly.memory_bound_gib(argparse.Namespace(jobs=2, pt1_jobs=0, pt1_max_log_mb=64)),
                               2 * nightly.WORKER_GIB + 2 * 9 * 64 / 1024)


class NightlyRatchet(unittest.TestCase):
    """T4: classification, and the host-labelled ratchet."""

    FP = {k: "x" for k in ("oracle_pdftex_version", "pdflatex", "shell_escape", "argv0", "pt", "levels",
                           "pt1_sample", "pt1_sample_seed")}

    def setUp(self):
        import nightly
        self.n = nightly
        self.tmp = tempfile.TemporaryDirectory()
        self.d = self.tmp.name

    def tearDown(self):
        self.tmp.cleanup()

    def results(self, docs, label="linux-pc", missing=()):
        out = os.path.join(self.d, "out")
        os.makedirs(out, exist_ok=True)
        with open(os.path.join(out, "summary.json"), "w") as f:
            json.dump({"host": {"label": label}, "fingerprint": dict(self.FP), "shards": {"missing": list(missing)},
                       "run_key": "k", "git_sha": "g", "engine": {}}, f)
        with open(os.path.join(out, "documents.json"), "w") as f:
            json.dump({"documents": docs}, f)
        return out

    def ratchet(self, out, *extra, label="linux-pc"):
        return self.n.main(["--state", self.d, "ratchet", "--host-label", label, "--results", out, *extra])

    @staticmethod
    def doc(i, level, pt1=None, pt2=True, **kw):
        return dict({"tier": "nightly-5k", "id": i, "level_index": level, "P-T1": pt1, "P-T2": pt2}, **kw)

    def test_classes(self):
        c = self.n.classify
        self.assertEqual(c({"excluded": "oracle: did not converge in 6 passes"})[0], "e")
        self.assertEqual(c({"excluded": "oracle: pdflatex exit 1: ! Missing $ inserted."})[0], "d")
        self.assertEqual(c({"excluded": "fetch: sha256 mismatch"})[0], "c")
        self.assertEqual(c({"level": -1, "candidate": {"status": "exit 1: ! LaTeX Error: File `foo.sty' not found."}}),
                         ("a", "missing foo.sty"))
        self.assertEqual(c({"level": -1, "candidate": {"status": "exit 1 (timeout)"}})[0], "b")
        r = {"level": 4, "pt": {"P-T1": False, "P-T2": True,
                                "pt1": {"log_line": {"line": 9, "oracle": "a", "candidate": "b"}}}}
        self.assertEqual(c(r), ("b", "P-T1 log line 9: oracle `a` vs `b`"))
        self.assertEqual(c({"level": 2, "pt": {"P-T1": True, "P-T2": True}})[0], "c")
        self.assertTrue(self.n.full_pass({"level": 4, "pt": {"P-T1": None, "P-T2": True}}))
        self.assertEqual(self.n.signature("P-T1 log line 1234: `\\hbox(6.8+2.1)x397.4`"),
                         "P-T1 log line: `\\hbox(#+#)x#`")

    def test_no_baseline_fails_and_record_then_check_passes(self):
        out = self.results([self.doc("a", 4, True), self.doc("b", 1, None, False)])
        self.assertEqual(self.ratchet(out), 1)
        self.assertEqual(self.ratchet(out, "--record"), 0)
        self.assertEqual(self.ratchet(out), 0)
        with open(os.path.join(self.d, "baseline", "linux-pc.json")) as f:
            base = json.load(f)
        self.assertEqual(base["host_label"], "linux-pc")
        self.assertEqual(base["documents"]["nightly-5k/a"], {"level": 4, "P-T1": True, "P-T2": True})

    def test_a_drop_fails_and_an_improvement_does_not(self):
        self.assertEqual(self.ratchet(self.results([self.doc("a", 4, True), self.doc("b", 1)]), "--record"), 0)
        self.assertEqual(self.ratchet(self.results([self.doc("a", 4, True), self.doc("b", 3)])), 0)
        self.assertEqual(self.ratchet(self.results([self.doc("a", 3, True), self.doc("b", 1)])), 1)
        self.assertEqual(self.ratchet(self.results([self.doc("a", 4, False), self.doc("b", 1)])), 1)
        self.assertEqual(self.ratchet(self.results([self.doc("a", 4, True), self.doc("b", 1, pt2=False)])), 1)
        # oracle-excluded now: not held silently
        self.assertEqual(self.ratchet(self.results([self.doc("a", None, excluded="oracle: x"), self.doc("b", 1)])), 1)
        # the source could not be fetched: unmeasured, reported, not a regression
        out = self.results([self.doc("a", None, excluded="fetch: failed"), self.doc("b", 1)])
        self.assertEqual(self.ratchet(out), 0)
        with open(os.path.join(out, "ratchet.json")) as f:
            self.assertEqual(json.load(f)["unmeasured"], ["nightly-5k/a"])
        # P-T1 outside the sample now: not evaluated is not a failure
        self.assertEqual(self.ratchet(self.results([self.doc("a", 4, None), self.doc("b", 1)])), 0)

    def test_other_hosts_are_refused(self):
        self.assertEqual(self.ratchet(self.results([self.doc("a", 4)]), "--record"), 0)
        mac = self.results([self.doc("a", 4)], label="mac-m5pro")
        self.assertEqual(self.ratchet(mac, label="linux-pc"), 2)   # results from elsewhere
        self.assertEqual(self.ratchet(mac, "--record", label="linux-pc"), 2)
        # a baseline file of another host, put in this host's place
        path = os.path.join(self.d, "baseline", "linux-pc.json")
        with open(path) as f:
            base = json.load(f)
        base["host_label"] = "mac-m5pro"
        with open(path, "w") as f:
            json.dump(base, f)
        self.assertEqual(self.ratchet(self.results([self.doc("a", 4)])), 2)

    def test_a_changed_oracle_is_refused_and_a_partial_run_is_not_recorded(self):
        self.assertEqual(self.ratchet(self.results([self.doc("a", 4)]), "--record"), 0)
        out = self.results([self.doc("a", 4)])
        with open(os.path.join(out, "summary.json")) as f:
            s = json.load(f)
        s["fingerprint"]["oracle_pdftex_version"] = "pdfTeX 1.40.30"
        with open(os.path.join(out, "summary.json"), "w") as f:
            json.dump(s, f)
        self.assertEqual(self.ratchet(out), 2)
        self.assertEqual(self.ratchet(self.results([self.doc("a", 4)], missing=[3]), "--record"), 2)


if __name__ == "__main__":
    unittest.main()
