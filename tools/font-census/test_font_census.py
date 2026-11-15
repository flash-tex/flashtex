"""Tests for tools/font-census/census.py (pure logic only: no TeX Live,
no engine, no network; inputs are inline or in a temp dir).

    python3 -m unittest discover -s tools/font-census -p 'test_*.py' -v
"""

import os
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import census  # noqa: E402


def write_tmp(tmp, name, text):
    p = os.path.join(tmp, name)
    with open(p, "w", encoding="latin-1") as f:
        f.write(text)
    return p


class ReadMapTests(unittest.TestCase):

    def kinds(self, text):
        with tempfile.TemporaryDirectory() as tmp:
            return census.read_map(write_tmp(tmp, "t.map", text))

    def test_type1_from_pfb(self):
        self.assertEqual(self.kinds("cmr10 CMR10 <cmr10.pfb\n"),
                         {"cmr10": "type1"})

    def test_enc_filtered_and_last_font_file_wins(self):
        # .enc is an encoding vector, not a font file; the last font file decides
        kinds = self.kinds("a A <a.enc <a.pfb\nb B <b.pfb <b.ttf\n")
        self.assertEqual(kinds, {"a": "type1", "b": "truetype"})

    def test_truetype_and_opentype_suffixes(self):
        text = "a A <a.ttf\nb B <b.ttc\nc C <c.otf\nd D <d.OTF\n"
        self.assertEqual(self.kinds(text),
                         {"a": "truetype", "b": "truetype",
                          "c": "opentype", "d": "opentype"})

    def test_builtin_without_font_file_is_type1(self):
        self.assertEqual(self.kinds("cmr10 CMR10\n"), {"cmr10": "type1"})

    def test_bare_tfm_only_line_is_pk(self):
        self.assertEqual(self.kinds("cmr10\n"), {"cmr10": "pk"})

    def test_first_entry_wins(self):
        text = "a A <a.pfb\na A <a.ttf\n"
        self.assertEqual(self.kinds(text), {"a": "type1"})


class LsRTests(unittest.TestCase):

    def tree(self, text):
        with tempfile.TemporaryDirectory() as tmp:
            write_tmp(tmp, "ls-R", text)
            return census.ls_r(tmp)

    def test_dirs_and_files(self):
        tree = self.tree("fonts/tfm/public/cm:\ncmr10.tfm\ncmbx10.tfm\n\nfonts/vf:\nfoo.vf\n")
        self.assertEqual(tree, {"fonts/tfm/public/cm": ["cmr10.tfm", "cmbx10.tfm"],
                                "fonts/vf": ["foo.vf"]})

    def test_dot_slash_prefix_stripped(self):
        self.assertEqual(self.tree("./fonts/tfm:\na.tfm\n"), {"fonts/tfm": ["a.tfm"]})

    def test_blank_lines_and_pre_header_lines_ignored(self):
        tree = self.tree("total 3\n\nfonts/tfm:\n\ncmr10.tfm\n")
        self.assertEqual(tree, {"fonts/tfm": ["cmr10.tfm"]})


class BackendLinesTests(unittest.TestCase):

    def test_extracts_map_enc_fontfile_and_output(self):
        log = ("This is pdfTeX\n"
               "{/texmf/fonts/map/pdftex/pdftex.map}\n"
               "{/texmf/fonts/enc/dvips/cm/cm.enc}\n"
               "</texmf/fonts/type1/cm/cmr10.pfb>\n"
               "Output written on doc.pdf (1 page, 42 bytes).\n")
        self.assertEqual(census.backend_lines(log, "/bin/pdftex"),
                         ["{/texmf/fonts/map/pdftex/pdftex.map}",
                          "{/texmf/fonts/enc/dvips/cm/cm.enc}",
                          "</texmf/fonts/type1/cm/cmr10.pfb>",
                          "Output written on doc.pdf (1 page, 42 bytes)"])

    def test_lines_broken_at_79_columns_rejoined(self):
        log = "{/texmf/fonts/map/pdftex/pdfte\nx.map}\n"
        self.assertEqual(census.backend_lines(log, "/bin/pdftex"),
                         ["{/texmf/fonts/map/pdftex/pdftex.map}"])

    def test_program_path_normalized_to_prog(self):
        log = "</opt/tex/bin/pdftex/cmr10.pfb>\n"
        self.assertEqual(census.backend_lines(log, "/opt/tex/bin/pdftex"),
                         ["<PROG/cmr10.pfb>"])

    def test_pdftex_error_captured(self):
        log = "Some text\n!pdfTeX error: font expansion broken\n"
        self.assertEqual(census.backend_lines(log, "/bin/pdftex"),
                         ["!pdfTeX error: font expansion broken"])

    def test_irrelevant_log_yields_nothing(self):
        self.assertEqual(census.backend_lines("This is pdfTeX\n(./doc.tex)\n", "/bin/pdftex"), [])


def make_texmf(tmp):
    """Fake TEXMFDIST with an ls-R and a map file; returns (texmf, mappath)."""
    texmf = os.path.join(tmp, "texmf")
    os.makedirs(texmf)
    tfms = "\n".join("f%02d.tfm" % n for n in range(1, 11))
    lsr = ("fonts/tfm/public/foo:\n" + tfms + "\n\n"
           "fonts/tfm/public/mixed:\nm1.tfm\nm2.tfm\nm3.tfm\nm4.tfm\n\n"
           "fonts/tfm/public/nosrc:\nzzz.tfm\n\n"
           "fonts/vf/public/foo:\nm2.vf\n\n"
           "fonts/source/public/foo:\nm3.mf\n")
    write_tmp(texmf, "ls-R", lsr)
    mappath = write_tmp(tmp, "pdftex.map",
                        "".join("f%02d F%02d <f%02d.pfb\n" % (n, n, n) for n in range(1, 11))
                        + "m1 M1 <m1.pfb\n")
    return texmf, mappath


def run_families(tmp, per_family):
    texmf, mappath = make_texmf(tmp)

    def fake_kpsewhich(texbin, *args):
        if args == ("-var-value", "TEXMFDIST"):
            return texmf
        if args == ("pdftex.map",):
            return mappath
        raise AssertionError(args)

    with mock.patch.object(census, "kpsewhich", side_effect=fake_kpsewhich):
        return census.families("DUMMY-TEXBIN", per_family)


class FamiliesTests(unittest.TestCase):

    def test_precedence_map_vf_mf_and_unusable_dropped(self):
        with tempfile.TemporaryDirectory() as tmp:
            fams = {f["id"]: f for f in run_families(tmp, 6)}
        mixed = fams["public/mixed"]
        self.assertEqual(mixed["kinds"], {"m1": "type1", "m2": "vf", "m3": "pk"})
        self.assertEqual(mixed["fonts"], ["m1", "m2", "m3"])
        self.assertEqual(mixed["tfm_count"], 4)  # m4 counted but unusable
        self.assertNotIn("public/nosrc", fams)  # no usable font: skipped

    def test_family_within_budget_picks_everything(self):
        with tempfile.TemporaryDirectory() as tmp:
            fams = {f["id"]: f for f in run_families(tmp, 10)}
        self.assertEqual(fams["public/foo"]["fonts"],
                         ["f%02d" % n for n in range(1, 11)])

    # #2056: `--per-family 1` used to divide by (per_family - 1) whenever a
    # family had more than one usable font.
    def test_per_family_one_does_not_crash(self):
        with tempfile.TemporaryDirectory() as tmp:
            fams = {f["id"]: f for f in run_families(tmp, 1)}
        self.assertEqual(fams["public/foo"]["fonts"], ["f01"])


if __name__ == "__main__":
    unittest.main()
