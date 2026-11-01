"""Mutation tests: the comparator finds a one-number change in xelatex's PDF.

A small document (hyperref outline and links, coloured text, a rule) is made
by TeX Live's `xelatex` (the oracle; skipped without it), rewritten by qpdf
as uncompressed QDF, and then edited in place, keeping every byte count (so
the cross-reference table stays right):

* one `Td` x moved by 0.02 bp: every glyph of that text object is reported
  moved (tolerance 0.01 bp); moved by 0.004 bp: within tolerance, and the
  deviation is measured; moved by 0.5 bp: Core Graphics' pixels differ too;
* the text colour changed (`1 0 0 rg` -> `0 0 1 rg`);
* a link removed from the page's /Annots;
* an outline title changed;
* the document title changed; a named destination's coordinate changed.

    python3 -m unittest discover -s tools/xetex-pdfparity -p 'test_*.py'
"""
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import compare as C  # noqa: E402
from pdfdoc import QPDF, Doc  # noqa: E402

DOC = r"""\documentclass{article}
\usepackage{fontspec}
\usepackage{xcolor}
\usepackage[pdftitle={Mutation}]{hyperref}
\begin{document}
\section{Alpha}\label{a}
Plain text, \textcolor{red}{red text}, and a link to \ref{b}.
\rule{3cm}{1pt}
\newpage
\section{Beta}\label{b}
Back to \ref{a}.
\end{document}
"""


def have_xelatex():
    return shutil.which("xelatex") is not None and os.path.exists(QPDF)


@unittest.skipUnless(have_xelatex(), "needs TeX Live's xelatex and qpdf")
class Mutations(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.mkdtemp(prefix="pdfparity-mut-")
        with open(os.path.join(cls.tmp, "m.tex"), "w") as f:
            f.write(DOC)
        env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1", TZ="UTC")
        for _ in range(2):
            subprocess.run(["xelatex", "-interaction=nonstopmode", "m.tex"], cwd=cls.tmp, env=env,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
        cls.orig = os.path.join(cls.tmp, "m.pdf")
        # One more pass with -no-pdf keeps the XDV (TeX's exact positions).
        shutil.copy(cls.orig, cls.orig + ".keep")
        subprocess.run(["xelatex", "-no-pdf", "-interaction=nonstopmode", "m.tex"], cwd=cls.tmp, env=env,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True)
        os.replace(cls.orig + ".keep", cls.orig)
        cls.xdv = os.path.join(cls.tmp, "m.xdv")
        cls.base = os.path.join(cls.tmp, "base.pdf")
        subprocess.run([QPDF, "--qdf", "--object-streams=disable", cls.orig, cls.base], check=True)
        with open(cls.base, "rb") as f:
            cls.data = f.read()
        cls.doc = Doc.open(cls.base)
        cls.n = 0

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.tmp, ignore_errors=True)

    # -- helpers ---------------------------------------------------------------
    def obj_range(self, ref):
        num, gen = ref.split()[:2]
        m = re.search(rb"(?m)^%s %s obj\n" % (num.encode(), gen.encode()), self.data)
        self.assertIsNotNone(m, ref)
        end = self.data.index(b"endobj", m.end())
        return m.end(), end

    def edit(self, ref, old, new, data=None):
        """`old` -> `new` (same length) in object `ref`'s text."""
        self.assertEqual(len(old), len(new))
        data = bytearray(self.data if data is None else data)
        a, b = self.obj_range(ref)
        k = data.find(old, a, b)
        self.assertGreaterEqual(k, 0, "%r not in %s" % (old, ref))
        data[k:k + len(old)] = new
        return bytes(data)

    def compare(self, data, visual=False, rel_tol=0.005):
        type(self).n += 1
        p = os.path.join(self.tmp, "mut%d.pdf" % self.n)
        with open(p, "wb") as f:
            f.write(data)
        return C.compare(self.base, p, C.Options(visual=visual, rel_tol=rel_tol))

    def page_content_ref(self, i=0):
        page = self.doc.pages[i][1]
        c = page["/Contents"]
        return c[0] if isinstance(c, list) else c

    def move_td(self, delta):
        ref = self.page_content_ref()
        a, b = self.obj_range(ref)
        for m in re.finditer(rb"(-?\d+\.(\d+)) (-?\d+\.\d+) Td", self.data[a:b]):
            x, nd = m.group(1), len(m.group(2))
            y = b"%.*f" % (nd, float(x) + delta)
            if len(y) == len(x):
                return self.edit(ref, m.group(0), y + m.group(0)[len(x):]), float(x)
        self.fail("no Td to move")

    # -- tests -----------------------------------------------------------------
    def test_unchanged_and_rewritten(self):
        rep = C.compare(self.base, self.base, C.Options())
        self.assertTrue(rep.ok(), rep.text())
        rep = C.compare(self.orig, self.base, C.Options())  # QDF vs xdvipdfmx's own layout
        self.assertTrue(rep.ok(), rep.text())
        self.assertGreater(rep.stats["glyphs_matched"], 50)

    def test_moved_glyphs(self):
        data, x = self.move_td(0.02)
        rep = self.compare(data, rel_tol=0)  # --tol only: 0.01 bp
        self.assertIn("glyph-position", rep.kinds, rep.text())
        self.assertIn("moved by (0.02, 0)", rep.kinds["glyph-position"]["examples"][0])
        # Nothing else changed.
        self.assertEqual(set(rep.kinds), {"glyph-position"}, rep.text())
        # The default tolerance (0.01 bp + 0.005 em) holds it, but not a 0.1 bp move.
        self.assertTrue(self.compare(data).ok())
        data, x = self.move_td(0.1)
        self.assertEqual(set(self.compare(data).kinds), {"glyph-position"})

    def test_move_within_tolerance_is_measured(self):
        data, x = self.move_td(0.004)
        rep = self.compare(data)
        self.assertTrue(rep.ok(), rep.text())
        self.assertAlmostEqual(rep.stats["glyph_max_dev"], 0.004, places=6)

    def test_visible_move(self):
        data, x = self.move_td(0.5)
        rep = self.compare(data, visual=True)
        self.assertIn("glyph-position", rep.kinds)
        self.assertGreater(rep.visual, 0)
        self.assertIn("visual", rep.kinds)

    def test_xdv_check(self):
        """xdvipdfmx's PDF against TeX's positions: every glyph paired, off by
        at most xdvipdfmx's measured error; moving a text object shows."""
        import precision as P
        from model import extract
        x = P.xdv_check(extract(self.orig), self.xdv)
        self.assertGreater(x["paired"], 50)
        self.assertEqual((x["unpaired_pdf"], x["unpaired_xdv"]), (0, 0))
        self.assertLess(x["max_bp"], 0.02)
        data, _ = self.move_td(0.05)
        p = os.path.join(self.tmp, "xdvmove.pdf")
        with open(p, "wb") as f:
            f.write(data)
        y = P.xdv_check(extract(p), self.xdv)
        self.assertGreater(y["max_bp"], 0.05 - x["max_bp"] - 1e-9)
        self.assertGreater(y["over"], 0)

    def test_colour(self):
        ref = self.page_content_ref()
        data = self.edit(ref, b"1 0 0 rg", b"0 0 1 rg")
        rep = self.compare(data)
        self.assertEqual(set(rep.kinds), {"glyph-colour"}, rep.text())
        self.assertIn("DeviceRGB[1 0 0] vs DeviceRGB[0 0 1]", rep.kinds["glyph-colour"]["examples"][0])

    def test_missing_link(self):
        page_ref, page = self.doc.pages[0]
        annots = page["/Annots"]
        if isinstance(annots, str):  # an indirect array
            arr = self.doc.get(annots)
            data = self.edit(annots, arr[0].encode(), b" " * len(arr[0]))
        else:
            data = self.edit(page_ref, annots[0].encode(), b" " * len(annots[0]))
        rep = self.compare(data)
        self.assertEqual(set(rep.kinds), {"link-missing"}, rep.text())
        self.assertEqual(rep.kinds["link-missing"]["count"], 1)

    def test_outline_title(self):
        first = self.doc.catalog["/Outlines"]
        item = self.doc.get(first)["/First"]
        title = self.doc.get(item)["/Title"]
        self.assertEqual(title, "u:Alpha")
        a, b = self.obj_range(item)
        if b"(Alpha)" in self.data[a:b]:
            data = self.edit(item, b"(Alpha)", b"(Alphx)")
        else:  # UTF-16 hex
            data = self.edit(item, b"0061>", b"0078>")
        rep = self.compare(data)
        self.assertEqual(set(rep.kinds), {"outline"}, rep.text())
        self.assertIn("title", rep.kinds["outline"]["examples"][0])

    def test_info_title(self):
        info = self.doc.trailer["/Info"]
        a, b = self.obj_range(info)
        if b"(Mutation)" in self.data[a:b]:
            data = self.edit(info, b"(Mutation)", b"(Mutatiox)")
        else:  # xdvipdfmx writes it as UTF-16: <feff004d...006e>
            data = self.edit(info, b"006e>", b"0078>")
        rep = self.compare(data)
        self.assertEqual(set(rep.kinds), {"info"}, rep.text())

    def test_named_destination(self):
        names = self.doc.get(self.doc.catalog["/Names"])
        tree = self.doc.get(names["/Dests"])
        leaf = tree
        while "/Names" not in leaf:
            leaf = self.doc.get(leaf["/Kids"][0])
        dest = leaf["/Names"][1]
        if isinstance(dest, str) and dest.endswith(" R"):
            arr = self.doc.get(dest)
            target = dest
        else:
            arr = dest
            target = None
        self.assertIsInstance(arr, list)
        x = arr[2]
        old = ("%g" % x).encode()
        new = ("%g" % (x + 1)).encode()
        self.assertEqual(len(old), len(new))
        if target is None:  # inline in the name tree's leaf
            target = next(r for r in self.doc._raw if r.startswith("obj:") and self.doc.obj(r[4:]) is leaf)[4:]
        data = self.edit(target, old, new)  # the first number of the array is x
        rep = self.compare(data)
        self.assertIn("dest", rep.kinds, rep.text())


if __name__ == "__main__":
    unittest.main()
