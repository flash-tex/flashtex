#!/usr/bin/env python3
"""Self-tests for compare_failures.py on synthetic run.py transcripts.

Stdlib unittest only: `python3 tools/latex-suites/test_compare_failures.py`.
"""

import contextlib
import io
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from compare_failures import main  # noqa: E402

DVIPS = "latex3/l3kernel[config-backend]@etex-dvips"
DVISVGM = "latex3/l3kernel[config-backend]@etex-dvisvgm"


def transcript(failed_in, labels=True):
    """A run.py transcript in which `failed_in` ({label: [tests]}) failed;
    each directory ran two tests. labels=False drops the per-directory
    FAILED lines, as run.py printed before #1299."""
    lines, names = [], set()
    for label in (DVIPS, DVISVGM):
        failed = sorted(failed_in.get(label, ()))
        names |= set(failed)
        lines.append("%s: PASS %d / FAIL %d / SKIP 0"
                     % (label, 2 - len(failed), len(failed)))
        if failed and labels:
            lines.append("%s: FAILED %s" % (label, " ".join(failed)))
    if names:
        lines.append("failing tests:")
        lines += ["  %s [UNEXPECTED]" % n for n in sorted(names)]
        lines.append("UNEXPECTED failures: %s" % ", ".join(sorted(names)))
    return "\n".join(lines) + "\n"


class CompareFailures(unittest.TestCase):
    def run_main(self, ref, cand):
        with tempfile.TemporaryDirectory() as d:
            paths = []
            for name, text in (("ref.txt", ref), ("cand.txt", cand)):
                paths.append(os.path.join(d, name))
                with open(paths[-1], "w", encoding="utf-8") as f:
                    f.write(text)
            out, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                rc = main(["compare_failures.py"] + paths)
        return rc, out.getvalue(), err.getvalue()

    def test_same_name_failing_under_other_backend_is_candidate_only(self):
        ref = transcript({DVISVGM: ["m3backend01"]})  # reference fails it under dvisvgm
        cand = transcript({DVIPS: ["m3backend01"]})   # candidate fails it under dvips
        rc, out, err = self.run_main(ref, cand)
        self.assertEqual(rc, 1)
        self.assertEqual(out, "T2: reference fails 1, candidate fails 1; candidate-only "
                              "failures: m3backend01 (%s)\n" % DVIPS)
        self.assertEqual(err, "")

    def test_equal_failures_pass(self):
        fails = {DVIPS: ["m3backend01"], DVISVGM: ["m3backend01", "m3backend02"]}
        rc, out, err = self.run_main(transcript(fails), transcript(fails))
        self.assertEqual(rc, 0)
        self.assertEqual(out, "T2: reference fails 3, candidate fails 3; "
                              "candidate-only failures: none\n")
        self.assertEqual(err, "")

    def test_no_failures_needs_no_label_lines(self):
        rc, out, err = self.run_main(transcript({}), transcript({}))
        self.assertEqual((rc, err), (0, ""))
        self.assertIn("candidate-only failures: none", out)

    def test_unlabelled_log_falls_back_to_bare_names_with_warning(self):
        ref = transcript({DVISVGM: ["m3backend01"]}, labels=False)
        cand = transcript({DVIPS: ["m3backend01"], DVISVGM: ["m3backend02"]})
        rc, out, err = self.run_main(ref, cand)
        self.assertEqual(rc, 1)
        self.assertEqual(out, "T2: reference fails 1, candidate fails 2; "
                              "candidate-only failures: m3backend02\n")
        self.assertIn("warning:", err)
        self.assertIn("ref.txt", err)
        self.assertNotIn("cand.txt", err)

    def test_label_lines_short_of_fail_counts_is_an_error(self):
        cand = transcript({DVIPS: ["m3backend01"]}).replace(
            "%s: PASS 2 / FAIL 0" % DVISVGM, "%s: PASS 1 / FAIL 1" % DVISVGM)
        rc, out, err = self.run_main(transcript({}), cand)
        self.assertEqual((rc, out), (2, ""))
        self.assertIn("do not match the FAIL counts", err)


if __name__ == "__main__":
    unittest.main()
