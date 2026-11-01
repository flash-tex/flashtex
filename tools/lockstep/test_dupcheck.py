import os
import tempfile
import unittest
import importlib.util

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location("dupcheck", os.path.join(HERE, "dupcheck.py"))
dc = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(dc)

A = """% 1000-x: something
\\input prelude
\\setbox0=\\hbox{\\vrule width10pt height8pt depth2pt\\hskip5pt\\vrule width20pt}
\\message{1000 w=\\the\\wd0}
\\lsshipbox0
\\end
"""


class DupcheckTest(unittest.TestCase):
    def dirs(self, ref_files, new_files):
        base = tempfile.mkdtemp(prefix="dupcheck-")
        out = []
        for label, files in (("ref", ref_files), ("new", new_files)):
            d = os.path.join(base, label)
            os.makedirs(d)
            for name, text in files.items():
                with open(os.path.join(d, name), "w") as fh:
                    fh.write(text)
            out.append(d)
        return out

    def run_main(self, ref, new, extra=()):
        r, n = self.dirs(ref, new)
        return dc.main(["--ref", r, "--new", n] + list(extra))

    def test_only_a_value_changed_is_flagged(self):
        b = A.replace("10pt", "37pt").replace("1000", "1999")
        self.assertEqual(self.run_main({"1000-x.tex": A}, {"1999-y.tex": b}), 1)

    def test_comments_and_case_numbers_do_not_matter(self):
        b = A.replace("something", "a totally different comment").replace("1000", "1500")
        self.assertEqual(self.run_main({"1000-x.tex": A}, {"1500-y.tex": b}), 1)

    def test_different_primitives_pass(self):
        b = "\\input prelude\n\\count1=5 \\advance\\count1 by 3\n\\message{\\the\\count1 \\romannumeral\\count1}\n\\end\n"
        self.assertEqual(self.run_main({"1000-x.tex": A}, {"1300-z.tex": b}), 0)

    def test_threshold_is_respected(self):
        b = A.replace("\\hskip5pt", "\\kern5pt")
        self.assertEqual(self.run_main({"1000-x.tex": A}, {"1301-w.tex": b}, ["--threshold", "0.99", "--topic", "2"]), 0)

    def test_same_topic_different_code_is_flagged(self):
        a = "% 1000-nullfont-empty-box: verifies a box of nullfont text is empty (tex.web 552)\n\\input prelude\n\\setbox0=\\hbox{\\nullfont ABC}\n\\end\n"
        b = "% 1266-nullfont-empty-box: verifies nullfont text makes an empty box (tex.web 552)\n\\input prelude\n\\count1=4 \\message{\\the\\fontdimen2\\nullfont}\n\\end\n"
        self.assertEqual(self.run_main({"1000-nullfont-empty-box.tex": a}, {"1266-nullfont-empty-box.tex": b}), 1)

    def test_unrelated_topics_are_not_flagged_by_topic(self):
        a = "% 1000-nullfont-empty-box: verifies a box of nullfont text is empty\n\\input prelude\n\\setbox0=\\hbox{\\nullfont ABC}\n\\end\n"
        b = "% 1266-romannumeral-large: verifies romannumeral of a large count\n\\input prelude\n\\message{\\romannumeral 3999}\n\\end\n"
        self.assertEqual(self.run_main({"1000-nullfont-empty-box.tex": a}, {"1266-romannumeral-large.tex": b}), 0)

    def test_two_new_cases_are_compared_with_each_other(self):
        r, n = self.dirs({"1000-x.tex": "\\input prelude\n\\count1=1\n\\end\n"},
                         {"1400-a.tex": A, "1401-b.tex": A.replace("10pt", "11pt")})
        self.assertEqual(dc.main(["--ref", r, "--new", n]), 1)

    def test_usage_errors(self):
        self.assertEqual(dc.main(["--ref", "/nonexistent", "--new", "/nonexistent"]), 2)

    def test_jaccard_bounds(self):
        self.assertEqual(dc.jaccard({"a"}, {"a"}), 1.0)
        self.assertEqual(dc.jaccard({"a"}, {"b"}), 0.0)


if __name__ == "__main__":
    unittest.main()
