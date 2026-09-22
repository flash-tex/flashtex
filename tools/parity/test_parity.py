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


class Definers(unittest.TestCase):
    INDEX = {"cs": {"text": ["amstex.sty", "amstext.sty"], "subjclass": ["amsart.cls"], "raisebox": ["latex.ltx"],
                    "autoref": ["hyperref.sty"], "arrow": ["tikzlibrarycd.code.tex"]},
             "env": {"alignat": ["amsmath.sty"]},
             "requires": {"amsart": ["amsmath", "amstex"], "amsmath": ["amstext"], "tikz-cd": ["tikzlibrarycd.code"]}}
    FACTS = {"class": "amsart", "packages": ["amsmath", "tikz-cd"], "cs_tex": ["myop"], "cs_sty": ["styop"],
             "env_tex": [], "env_sty": []}

    def test_scan_finds_definitions_and_loads(self):
        cs, env = {}, {}
        definers._scan(b"\\newcommand{\\foo}[1]{x}\\def\\bar#1{y}\\newenvironment{baz}{}{}\\def\\qux{}\\def\\endqux{}",
                       cs, env, "t.sty")
        self.assertTrue({"foo", "bar", "qux", "endqux"} <= set(cs))
        self.assertTrue({"baz", "qux"} <= set(env))
        self.assertEqual(definers._requires(b"\\RequirePackage[x]{a,b}\\usetikzlibrary{cd}"),
                         {"a", "b", "tikzlibrarycd.code"})

    def test_grouped_by_what_the_document_loads(self):
        g = lambda k: definers.group_of(k, self.FACTS, self.INDEX)  # noqa: E731
        self.assertEqual(g("unknown_command: cs \\subjclass"), "class amsart")
        self.assertEqual(g("unsupported_feature: cs \\text"), "package amsmath")   # amstext via amsmath, not amstex via the class
        self.assertEqual(g("syntax_error: env alignat"), "package amsmath")
        self.assertEqual(g("compiler: cs \\arrow"), "package tikz-cd")
        self.assertEqual(g("compiler: cs \\raisebox"), "LaTeX kernel")
        self.assertEqual(g("compiler: cs \\autoref"), "compiler: cs \\autoref")   # hyperref is not loaded
        self.assertEqual(g("compiler: cs \\myop"), "project macro (defined in the document)")
        self.assertEqual(g("compiler: cs \\styop"), "project .sty/.cls (not read)")
        self.assertEqual(g("unsupported_feature: package listings"), "package listings")
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


if __name__ == "__main__":
    unittest.main()
