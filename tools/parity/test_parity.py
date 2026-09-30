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


def meta_path_of(root, meta):
    """The oracle.json of a P-T oracle entry cached under `<root>/cache`."""
    return os.path.join(root, "cache", "pt-oracle", meta["key"][:2], meta["key"], "oracle.json")


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

    def test_pt_gate(self):
        ok = {"id": "ok", "pt": {"P-T1": True, "P-T2": True, "why": {}}}
        t2 = {"id": "t2", "pt": {"P-T1": True, "P-T2": False, "why": {"P-T2": "page 1 stream"}}}
        na = {"id": "na", "pt": {"P-T1": None, "P-T2": None, "why": {}, "excluded": "oracle: does not compile"}}
        off = {"id": "off", "pt": None}
        skipped = {"id": "skip", "excluded": "no entry"}
        self.assertEqual(parity.require_pt([ok, skipped]), [])
        self.assertEqual([m[0] for m in parity.require_pt([ok, t2, na, off])], ["na", "off", "t2"])
        self.assertIn("P-T2 fail (page 1 stream)", parity.require_pt([t2])[0][1])
        self.assertEqual(parity.require_pt([skipped])[0][0], "(none)")


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

    def test_template_files_survive_skip_pdfs(self):
        doc = b"\\documentclass{article}\n\\begin{document}\nx\n\\end{document}\n"
        with tempfile.TemporaryDirectory() as d:
            texmf = os.path.join(d, "texmf")
            os.makedirs(os.path.join(texmf, "doc", "t"))
            for name, data in (("s.tex", doc), ("s.pdf", b"%PDF prebuilt"), ("fig.pdf", b"%PDF fig"), ("x.bib", b"")):
                with open(os.path.join(texmf, "doc", "t", name), "wb") as f:
                    f.write(data)
            man = os.path.join(d, "m.json")
            with open(man, "w") as f:
                json.dump({"tier": "templates", "entries": [
                    {"id": "t", "path": "doc/t/s.tex", "copy_dir": True, "skip_pdfs": True, "files": ["fig.pdf"],
                     "sha256": corpus.sha256_bytes(doc)}]}, f)
            (rec,) = corpus.fetch_manifest(man, os.path.join(d, "cache"), texmf, log=lambda *_: None)
            self.assertIsNone(rec["problem"])
            self.assertEqual(sorted(os.listdir(rec["dir"])), [".parity-copied", "fig.pdf", "s.tex", "x.bib"])
            h = parity.tree_hash(rec["dir"])  # the marker is not part of the source tree
            os.remove(os.path.join(rec["dir"], ".parity-copied"))
            self.assertEqual(parity.tree_hash(rec["dir"]), h)
            corpus.fetch_manifest(man, os.path.join(d, "cache"), texmf, log=lambda *_: None)
            ino = os.stat(os.path.join(rec["dir"], "s.tex")).st_ino
            corpus.fetch_manifest(man, os.path.join(d, "cache"), texmf, log=lambda *_: None)
            self.assertEqual(os.stat(os.path.join(rec["dir"], "s.tex")).st_ino, ino)  # left alone, not rebuilt

    def test_packages_tier_copies_from_texlive(self):
        doc = b"\\documentclass{article}\n\\usepackage{p}\n\\begin{document}\nx\n\\end{document}\n"
        with tempfile.TemporaryDirectory() as d:
            texmf = os.path.join(d, "texmf")
            os.makedirs(os.path.join(texmf, "doc", "p", "figures"))
            for name, data in (("p.tex", doc), ("other.tex", b"%"), ("figures/fig.png", b"png")):
                with open(os.path.join(texmf, "doc", "p", name), "wb") as f:
                    f.write(data)
            man = os.path.join(d, "m.json")
            with open(man, "w") as f:
                json.dump({"tier": "packages", "entries": [
                    {"id": "p-single", "path": "doc/p/p.tex", "files": ["figures/fig.png"],
                     "sha256": corpus.sha256_bytes(doc)},
                    {"id": "p-bad", "path": "doc/p/other.tex", "sha256": "0" * 64}]}, f)
            ok, bad = corpus.fetch_manifest(man, os.path.join(d, "cache"), texmf, log=lambda *_: None)
            self.assertIsNone(ok["problem"])
            self.assertEqual((ok["tier"], ok["entry"]), ("packages", "p.tex"))
            self.assertEqual(ok["dir"], os.path.join(d, "cache", "src", "packages", "p-single"))
            self.assertEqual(sorted(os.listdir(ok["dir"])), [".parity-copied", "fig.png", "p.tex"])
            self.assertIn("sha256 mismatch", bad["problem"])

    def test_packages_manifest_is_well_formed(self):
        with open(os.path.join(corpus.MANIFEST_DIR, "packages-texlive-2026.json"), encoding="utf-8") as f:
            man = json.load(f)
        with open(os.path.join(corpus.MANIFEST_DIR, "templates-texlive-2026.json"), encoding="utf-8") as f:
            templates = {e["path"] for e in json.load(f)["entries"]}
        self.assertEqual((man["schema"], man["tier"]), ("flashtex-parity-corpus/1", "packages"))
        entries, skipped = man["entries"], man["skipped"]
        self.assertEqual((len(entries), len(skipped)), (92, 6))
        self.assertEqual([e["id"] for e in entries if e.get("pt1_skip")], ["tabu-europasscv"])
        ids, paths = [e["id"] for e in entries], [e["path"] for e in entries]
        self.assertEqual(len(set(ids)), len(ids))
        self.assertEqual(len(set(paths)), len(paths))  # no file pinned under two ids
        self.assertFalse(set(paths) & templates)  # nor repeated from the templates tier
        for e in entries:
            self.assertTrue(e["path"].startswith("doc/"), e["id"])
            self.assertRegex(e["sha256"], r"^[0-9a-f]{64}$")
            self.assertLessEqual(e["pages"], 100, e["id"])
            self.assertLessEqual(e["passes"], 3, e["id"])
        self.assertTrue(all(s["package"] and s["reason"] for s in skipped))
        self.assertEqual(len({s["package"] for s in skipped}), len(skipped))
        self.assertIn("packages", corpus.TEXLIVE_TIERS)

    def test_run_written_conversion_input_is_kept(self):
        import tiers
        with tempfile.TemporaryDirectory() as d:
            src, work = os.path.join(d, "src"), os.path.join(d, "work")
            os.makedirs(src)
            os.makedirs(work)
            for name in ("fig.eps",):  # shipped with the source
                open(os.path.join(src, name), "w").close()
            for name in ("fig.eps", "fig-eps-converted-to.pdf", "a.eps", "a-eps-converted-to.pdf", "b.eps", "x.aux"):
                open(os.path.join(work, name), "w").close()
            kept = tiers.keep_generated(src, work, os.path.join(d, "kept"))
            # a.eps was written by the run (filecontents) and converted; b.eps was not converted
            self.assertEqual(kept, ["a-eps-converted-to.pdf", "a.eps", "fig-eps-converted-to.pdf"])

    def test_old_oracle_cache_entry_is_not_reused(self):
        import capture
        import hashlib
        import tiers
        doc = {"id": "grfguide", "entry": "grfguide.tex", "dir": "/nonexistent"}
        version = "pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026)"
        # the v4 key exactly as tiers.oracle built it before keep_generated kept run-written inputs
        v4 = hashlib.sha256(json.dumps({
            "tree": "t", "entry": doc["entry"], "pdftex": version, "fmt": tiers.FMT, "trace": capture.TRACE,
            "env": capture.TRACE_ENV, "passes": tiers.PASSES, "shell_escape": capture.SHELL_ESCAPE,
            "argv0": capture.PROGRAM, "v": 4}, sort_keys=True).encode()).hexdigest()
        self.assertNotEqual(tiers.oracle_key(doc, version, True, "t"), v4)
        saved = tiers.engine_version, tiers.run_tex
        with tempfile.TemporaryDirectory() as cache:
            old = os.path.join(cache, "pt-oracle", v4[:2], v4)
            os.makedirs(old)
            with open(os.path.join(old, "oracle.json"), "w") as f:
                json.dump({"ok": True, "generated": ["a-eps-converted-to.pdf"], "key": v4}, f)
            def fresh(_doc, _exe, work, **_kw):  # the real run_tex makes the work dir under the entry
                os.makedirs(work)
                return {"ok": False, "why": "made afresh"}, None, None
            try:
                tiers.engine_version = lambda _exe: version
                tiers.run_tex = fresh
                meta, cap, pdf = tiers.oracle(doc, "/stub/pdftex", cache, True, "t")
            finally:
                tiers.engine_version, tiers.run_tex = saved
            self.assertEqual((meta["cached"], meta["why"], cap, pdf), (False, "made afresh", None, None))
            self.assertNotEqual(meta["key"], v4)

    def test_traced_log_over_budget_is_never_read(self):
        import capture
        import tiers
        saved = capture.run_engine, capture.MAX_LOG_BYTES, tiers.engine_version, tiers.run_tex
        with tempfile.TemporaryDirectory() as d:
            def engine(_bin, _fmt, _args, workdir, _env=None):
                with open(os.path.join(workdir, "main.log"), "w") as f:
                    f.write("**x\n" + "y" * 5000 + "\nOutput written on main.pdf (1 page, 9 bytes).\n")
                return 0, False
            try:
                capture.run_engine, capture.MAX_LOG_BYTES = engine, 1000
                cap = capture.capture(os.path.join(d, "main.tex"), "/stub", d)
                self.assertEqual((cap.log, cap.boxes, cap.complete), (None, None, True))
                self.assertGreater(cap.size, 5000)
                # the oracle keeps only the size: no log.gz, and it is never loaded
                pdf = os.path.join(d, "ref.pdf")
                open(pdf, "wb").close()

                def run(_doc, _exe, work, **_kw):
                    os.makedirs(work)
                    return {"ok": True, "passes": 1}, cap, pdf
                tiers.engine_version, tiers.run_tex = (lambda _exe: "pdfTeX stub"), run
                doc = {"id": "big", "entry": "main.tex", "dir": d}
                meta, ref_cap, _ = tiers.oracle(doc, "/stub", os.path.join(d, "cache"), True, "t")
                self.assertEqual((meta["log_chars"], meta["log_unread"], ref_cap), (cap.size, True, None))
                self.assertFalse(os.path.exists(os.path.join(os.path.dirname(meta_path_of(d, meta)), "log.gz")))
                meta, ref_cap, _ = tiers.oracle(doc, "/stub", os.path.join(d, "cache"), True, "t")
                self.assertEqual((meta["cached"], ref_cap), (True, None))
            finally:
                capture.run_engine, capture.MAX_LOG_BYTES, tiers.engine_version, tiers.run_tex = saved

    def test_every_pass_starts_with_the_pinned_seed(self):
        import capture
        self.assertEqual(capture.first_line("a/main.tex"), r"\pdfsetrandomseed 1\relax\input{a/main.tex}")
        self.assertTrue(capture.first_line("main.tex", trace=True).startswith(capture.SEED + capture.TRACE))

    def test_manifest_pt1_skip_is_not_evaluated(self):
        doc = {"id": "d", "tier": "packages", "problem": None, "pt1_skip": "pdfTeX seeds \\pdfuniformdeviate from the clock"}
        cfg = {"pt": "on", "oracle_pdftex": "/bin/true"}
        self.assertEqual(parity.pt1_skip_reason(doc, cfg), {
            "why": "not evaluated: pdfTeX seeds \\pdfuniformdeviate from the clock", "traced_oracle": False})
        self.assertIsNone(parity.pt1_skip_reason(doc, dict(cfg, pt="pt2")))


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

    def test_trace_complete_needs_the_end_of_run_line(self):
        self.assertTrue(tiers.trace_complete("x\n" * 10 + "Output written on a.pdf (1 page, <BYTES> bytes).\n"))
        self.assertTrue(tiers.trace_complete("x\nNo pages of output.\n"))
        self.assertFalse(tiers.trace_complete("x\n" + "~.....\\" * 3))  # cut off mid-trace

    def test_oversized_traced_log_skips_p_t1_and_counts_it(self):
        with tempfile.TemporaryDirectory() as d:
            logz = os.path.join(d, "log.gz")
            with gzip.open(logz, "wt", encoding="latin-1") as f:
                f.write("x" * 5000)
            self.assertEqual(tiers.log_chars(logz, chunk=1024), 5000)
        cfg = {"pt": "on", "oracle_pdftex": "/bin/pdftex", "cache": "/nonexistent", "pt1_max_log": 4096}
        doc = {"id": "d", "tier": "arxiv", "dir": HERE, "entry": "x.tex"}
        real = tiers.oracle
        try:
            tiers.oracle = lambda *a, **k: ({"ok": True, "log_chars": 5000}, None, "ref.pdf")
            skip = parity.pt1_skip_reason(doc, cfg)
            self.assertIn("above --pt1-max-log-mb", skip["why"])
            self.assertTrue(skip["traced_oracle"])  # the traced oracle exists: its key is kept, its log not loaded
            skip = parity.pt1_skip_reason(doc, dict(cfg, pt1_skip=["arxiv/d"]))
            self.assertEqual(skip, {"why": "not evaluated: listed in --pt1-skip", "traced_oracle": False})
            self.assertIsNone(parity.pt1_skip_reason(doc, dict(cfg, pt1_max_log=0)))
            self.assertIsNone(parity.pt1_skip_reason(doc, dict(cfg, pt="pt2")))
            tiers.oracle = lambda *a, **k: ({"ok": True, "log_chars": 100}, None, "ref.pdf")
            self.assertIsNone(parity.pt1_skip_reason(doc, cfg))
            tiers.oracle = lambda *a, **k: ({"ok": True, "trace_incomplete": "the traced pass did not finish"},
                                            None, "ref.pdf")
            self.assertIn("did not finish", parity.pt1_skip_reason(doc, dict(cfg, pt1_max_log=0))["why"])
        finally:
            tiers.oracle = real
        rs = [{"id": "a", "pt": {"P-T1": True, "P-T2": True, "why": {}}},
              {"id": "b", "pt": {"P-T1": None, "P-T2": True, "why": {"P-T1": "not evaluated: ..."}}}]
        s = parity.summarize_pt(rs)
        self.assertEqual((s["P-T1"]["evaluated"], s["P-T1"]["passed"], s["P-T1"]["skipped"]), (1, 1, 1))

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

    @unittest.skipUnless(shutil.which("gs"), "needs Ghostscript for epstopdf")
    def test_converted_figures_are_seeded_from_the_oracle(self):
        """epstopdf converts an EPS figure under restricted \\write18; two runs
        converting it themselves differ (date in the log, Ghostscript's stamps
        in the PDF). Seeded with the oracle's conversion, the candidate matches."""
        src = os.path.join(self.d, "src")
        os.makedirs(src)
        with open(os.path.join(src, "fig.eps"), "w") as f:
            f.write("%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 20 20\nnewpath 0 0 moveto 20 20 lineto stroke\n")
        with open(os.path.join(src, "main.tex"), "w") as f:
            f.write("\\documentclass{article}\\usepackage{graphicx}\\begin{document}"
                    "\\includegraphics{fig.eps}\\end{document}\n")
        doc = {"dir": src, "entry": "main.tex"}
        old = capture.SHELL_ESCAPE
        capture.SHELL_ESCAPE = "-shell-restricted"  # TeX Live's default, which allows repstopdf
        try:
            work = os.path.join(self.d, "oracle")
            mo, co, po = tiers.run_tex(doc, PDFTEX, work)
            self.assertTrue(mo["ok"], mo.get("why"))
            kept = tiers.keep_generated(src, work, os.path.join(self.d, "kept"))
            self.assertEqual(kept, ["fig-eps-converted-to.pdf"])
            seed = {k: os.path.join(self.d, "kept", k) for k in kept}
            self.assertEqual(os.stat(seed[kept[0]]).st_mtime, os.stat(os.path.join(work, kept[0])).st_mtime)
            import time
            time.sleep(1.1)  # a conversion of its own would carry another second
            mc, cc, pc = tiers.run_tex(doc, PDFTEX, os.path.join(self.d, "cand"), seed=seed)
            self.assertTrue(tiers.compare_pt1(co, cc)["ok"], tiers.compare_pt1(co, cc).get("log_line"))
            self.assertTrue(tiers.compare_pt2(po, pc, self.d)["ok"])
        finally:
            capture.SHELL_ESCAPE = old

    def test_object_renumbering_is_invisible(self):
        _, _, p1 = self.build("a", "Hello world.")
        lin = os.path.join(self.d, "renumbered.pdf")
        subprocess.run(["qpdf", "--linearize", "--object-streams=generate", p1, lin], check=True)
        self.assertTrue(tiers.compare_pt2(p1, lin, self.d)["ok"])


class WorkerDeath(unittest.TestCase):
    """A candidate that kills the worker scoring it must not shrink any
    denominator: the document is failed, the others are scored, and the run
    exits non-zero."""

    def test_denominators_hold_and_the_run_fails(self):
        a, b = "real-world/article-twocolumn", "real-world/beamer-default"
        with tempfile.TemporaryDirectory() as d:
            probe = os.path.join(d, "probe")
            with open(probe, "w") as f:  # a "TeX engine" that kills its parent (the worker) on document a
                f.write("#!/bin/sh\ncase \"$1\" in --version) echo 'pdfTeX probe'; exit 0;; esac\n"
                        "case \"$(pwd)\" in *article-twocolumn*) kill -9 $PPID; sleep 5;; esac\nexit 1\n")
            os.chmod(probe, 0o755)
            out = os.path.join(d, "out")
            code = parity.main(["--tier", "fixtures", "--only", a, "--only", b, "--engine", probe,
                                "--pt", "off", "--raster", "none", "-j", "2", "--out", out,
                                "--work", os.path.join(d, "work"), "--cache", os.path.join(d, "cache")])
            self.assertEqual(code, parity.DIED_EXIT)
            with open(os.path.join(out, "documents.json")) as f:
                recs = {r["id"]: r for r in json.load(f)["fixtures"]}
            with open(os.path.join(out, "scoreboard.json")) as f:
                s = json.load(f)["tiers"]["fixtures"]["summary"]
        self.assertEqual(sorted(recs), [a, b])
        self.assertTrue(recs[a]["worker_died"])
        self.assertEqual(recs[a]["level"], -1)
        self.assertNotIn("excluded", recs[a])
        self.assertNotIn("worker_died", recs[b])  # re-queued and scored normally (the probe fails it at L0)
        self.assertEqual((s["documents"], s["measured"], s["at_least"]["L0"]["documents"]), (2, 2, 0))

    def test_worker_died_record_fails_every_tier(self):
        cfg = {"engine_kind": "tex", "pt": "on"}
        r = parity.worker_died_record({"id": "x"}, "arxiv", cfg)
        self.assertEqual((r["level"], r["pt"]["P-T1"], r["pt"]["P-T2"]), (-1, False, False))
        s = parity.summarize([r])
        self.assertEqual((s["measured"], s["pt"]["P-T1"]["evaluated"], s["pt"]["P-T1"]["passed"]), (1, 1, 0))


class PTCandidateCap(unittest.TestCase):
    def test_only_the_candidate_over_the_cap_fails_p_t1_without_loading_the_oracle(self):
        seen = {}

        def fake(*a, **k):
            seen["load_log"] = k.get("load_log")
            return {"ok": True, "log_chars": 100}, None, "ref.pdf"
        real = tiers.oracle
        tiers.oracle = fake
        try:
            cfg = {"pt": "on", "oracle_pdftex": "/bin/pdftex", "cache": "/nonexistent", "pt1_max_log": 4096,
                   "engine_kind": "tex", "qpdf": False}
            cand = {"capture": capture.Capture("x" * 5000, [], None), "pdf": None}
            pt = parity.score_pt({"id": "d", "tier": "arxiv", "dir": HERE, "entry": "x.tex"}, cfg, cand, HERE)
        finally:
            tiers.oracle = real
        self.assertIs(pt["P-T1"], False)
        self.assertIn("candidate's traced log", pt["pt1"]["why"])
        self.assertIs(seen["load_log"], False)


class Engines(unittest.TestCase):
    """engines.py: several parity runs side by side, failures classified."""

    @staticmethod
    def run_of(host, docs):
        rs = list(docs)
        summary = parity.summarize(rs)
        return ({"meta": {"engine_version": "E", "host": host, "platform": "Darwin", "pdflatex": "TL 2026",
                          "shell_escape": "-shell-restricted", "date": "2026-09-29"},
                 "tiers": {"arxiv": {"summary": summary}}}, {"arxiv": rs})

    @staticmethod
    def rec(i, level, pt1=True, pt2=True, **kw):
        checks = {name: level >= k for k, name in enumerate(parity.LEVELS)}
        r = {"id": i, "tier": "arxiv", "level": level, "checks": checks,
             "pt": {"P-T1": pt1, "P-T2": pt2, "why": {}}}
        r.update(kw)
        return r

    def test_classes_and_table(self):
        import engines
        new = [self.rec("ok", 4),
               self.rec("diff", 2, pt1=False, pt2=False,
                        pt={"P-T1": False, "P-T2": False, "pt1": {"log_line": {"line": 7, "oracle": "a", "candidate": "b"}}}),
               self.rec("pkg", -1, pt1=False, pt2=False,
                        candidate={"status": "exit 1: ! LaTeX Error: File `foo.sty' not found."}),
               {"id": "bad", "tier": "arxiv", "level": None, "excluded": "oracle: pdflatex exit 1"},
               self.rec("meas", 3)]
        old = [self.rec("ok", 1, pt1=None), self.rec("diff", 0, pt1=None), self.rec("pkg", 4, pt1=None),
               {"id": "bad", "tier": "arxiv", "level": None, "excluded": "oracle: pdflatex exit 1"},
               self.rec("meas", 4, pt1=None)]
        rep = engines.build({"new": self.run_of("h", new), "v1": self.run_of("h", old)}, "new",
                            shas={"new": "abc"}, notes={"arxiv/diff": {"section": "§1234", "issue": "#9"}})
        self.assertEqual(rep["classes"], {"a": 1, "b": 1, "c": 1, "d": 1, "e": 0})
        rows = {r["id"]: r for r in rep["documents"]["arxiv"]}
        self.assertNotIn("class", rows["ok"])
        self.assertEqual(rows["pkg"]["cause"], "missing foo.sty")
        self.assertIn("log line 7", rows["diff"]["cause"])
        self.assertEqual(rows["diff"]["section"], "§1234")
        self.assertEqual(rows["meas"]["class"], "c")  # P-T1 and P-T2 pass, a level fails: the measurement
        t = rep["tiers"]["arxiv"]
        self.assertEqual(t["new"]["L1"], [3, 4])
        self.assertEqual(t["new"]["P-T1"], [2, 4])
        self.assertIsNone(t["v1"]["P-T1"])  # the CLI has no P-T1
        self.assertEqual(rep["engines"]["new"]["git_sha"], "abc")
        md = engines.markdown(rep, "T")
        self.assertIn("| new | 5 | oracle 1 | 2/4 (50.0%) |", md)
        self.assertIn("n/a", md)

    def test_p_t1_not_evaluated_is_listed_but_the_cli_is_not(self):
        import engines
        big = self.rec("big", 4, pt1=None)
        big["pt"]["why"] = {"P-T1": "not evaluated: listed in --pt1-skip"}
        cli = self.rec("cli", 4, pt1=None)
        cli["pt"]["why"] = {"P-T1": parity.NOT_TEX}
        rep = engines.build({"new": self.run_of("h", [big]), "v1": self.run_of("h", [cli])}, "new", host_label="m")
        self.assertEqual(rep["documents"]["arxiv"][0]["pt1_not_evaluated"], "not evaluated: listed in --pt1-skip")
        self.assertIn("P-T1 not evaluated (new): 1 documents", engines.markdown(rep, "T"))
        self.assertIn("Measured on **m**", engines.markdown(rep, "T"))
        rep = engines.build({"v1": self.run_of("h", [cli])}, "v1")
        self.assertNotIn("pt1_not_evaluated", rep["documents"]["arxiv"][0])

    def test_timeout_is_harness_crash_is_engine_rerun_rule_is_its_own(self):
        import engines

        def with_pt1_why(i, why):
            r = self.rec(i, 4, pt1=False)
            r["pt"]["pt1"] = {"ok": False, "why": "the candidate's traced pass did not run: " + why}
            return r
        rs = [with_pt1_why("slow", tiers.TRACE_TIMEOUT.format(600)), with_pt1_why("crash", tiers.TRACE_CRASH.format(0.3)),
              {"id": "rerun", "tier": "arxiv", "level": None, "excluded": "oracle: did not converge in 6 passes"},
              parity.worker_died_record({"id": "dead"}, "arxiv", {"engine_kind": "tex", "pt": "on"})]
        rows = {r["id"]: r for r in engines.build({"new": self.run_of("h", rs)}, "new")["documents"]["arxiv"]}
        self.assertEqual({k: v["class"] for k, v in rows.items()}, {"slow": "c", "crash": "b", "rerun": "e", "dead": "c"})

    def test_note_overrides_class_and_keeps_the_automatic_one(self):
        import engines
        new = [self.rec("x", 2, pt1=False, pt2=False)]
        rep = engines.build({"new": self.run_of("h", new)}, "new", notes={"x": {"class": "a", "note": "bundle"}})
        row = rep["documents"]["arxiv"][0]
        self.assertEqual((row["class"], row["auto_class"]), ("a", "b"))


if __name__ == "__main__":
    unittest.main()
