#!/usr/bin/env python3
"""Pure-python tests for cumulative.py (no renderer, no pdflatex)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import cumulative as cu  # noqa: E402


def line(dy, dx=0.0, n=4, y=0.0, math=False, spread=0.0, source=None):
    words = []
    for i in range(n):
        words.append({"text": f"w{i}", "dx": dx, "dy": dy + (spread if i == n - 1 else 0.0),
                      "ref_x": 10.0 * i, "source": source, "math": math,
                      "font": "LMRoman10-Regular", "ref_font": "SFRM1095"})
    dys = [w["dy"] for w in words]
    return {"ref_y": y, "words": n, "dy": dy, "dx": dx, "dx_max": abs(dx),
            "math_fraction": 1.0 if math else 0.0, "dy_spread": max(dys) - min(dys),
            "text": "w0 w1", "_words": words}


FX = {"documents": [{"path": "main.tex", "text":
      "Alpha alpha.\n\n\\begin{itemize}\n  \\item One\n\\end{itemize}\n\n\\begin{enumerate}\n  \\item Two\n"}]}


class CauseKey(unittest.TestCase):
    def test_end_then_begin_is_one_cause(self):
        key, _ = cu.cause_key("\n\\end{itemize}\n\n\\begin{enumerate}\n  ")
        self.assertEqual(key, "\\end{itemize} -> \\begin{enumerate}")

    def test_leaving_an_environment(self):
        self.assertEqual(cu.cause_key(" in full.\n\\end{enumerate}\n\nA closing note")[0],
                         "after \\end{enumerate}")

    def test_entering_an_environment(self):
        self.assertEqual(cu.cause_key("}\n\\begin{align}\n  ")[0], "before \\begin{align}")

    def test_plain_line_break_has_no_cause(self):
        self.assertEqual(cu.cause_key("\n")[0], "line-break-within-paragraph")

    def test_blank_line_is_a_paragraph_break(self):
        self.assertEqual(cu.cause_key("}\n\n\\entry{")[0], "paragraph-break")

    def test_vertical_command_names_itself(self):
        self.assertEqual(cu.cause_key(".\n\n\\section*{")[0], "\\section")

    def test_missing_gap_is_unknown(self):
        self.assertEqual(cu.cause_key(None)[0], "unknown-boundary")


class EnclosingEnv(unittest.TestCase):
    def test_innermost_open_environment(self):
        byte = FX["documents"][0]["text"].index("\\item Two")
        self.assertEqual(cu.enclosing_env(FX, "main.tex", byte), "enumerate")

    def test_document_level_is_not_an_environment(self):
        self.assertIsNone(cu.enclosing_env(FX, "main.tex", 3))


class Steps(unittest.TestCase):
    def test_a_step_that_never_returns_is_cumulative_and_sized_by_the_rest(self):
        lines = [line(0.0), line(0.0), line(-2.0), line(-2.0), line(-2.0)]
        steps = cu.steps_for_page(FX, lines, 0.05)
        self.assertEqual([s["kind"] for s in steps], ["cumulative"])
        self.assertEqual(steps[0]["lines_affected"], 3)
        self.assertTrue(steps[0]["crosses_glyph_gate"])

    def test_a_step_that_returns_is_local_and_worth_one_line(self):
        lines = [line(0.0), line(0.0), line(20.0), line(0.0), line(0.0)]
        steps = cu.steps_for_page(FX, lines, 0.05)
        self.assertEqual([s["kind"] for s in steps], ["local", "local"])
        self.assertEqual(steps[0]["lines_affected"], 1)

    def test_an_alternating_pitch_is_not_counted_as_cumulative(self):
        # A matrix whose rows alternate must not be read as compounding steps:
        # the page's prevailing level is unchanged below it. (Both medians need
        # a few lines to be stable; on a page of four lines an alternation and
        # a staircase are genuinely indistinguishable, and the tool says so in
        # its Honest limits.)
        lines = [line(0.0)] * 3 + [line(2.8), line(0.0)] * 4 + [line(0.0)] * 3
        steps = cu.steps_for_page(FX, lines, 0.05)
        self.assertEqual({s["kind"] for s in steps}, {"local"})

    def test_sub_gate_step_is_still_reported_when_it_accumulates(self):
        lines = [line(0.0)] + [line(-0.12)] * 30
        steps = cu.steps_for_page(FX, lines, 0.05)
        self.assertEqual(steps[0]["kind"], "cumulative")
        self.assertFalse(steps[0]["crosses_glyph_gate"])
        self.assertTrue(steps[0]["crosses_rule_gate"])
        self.assertEqual(steps[0]["lines_affected"], 30)

    def test_first_line_of_a_page_is_page_origin(self):
        steps = cu.steps_for_page(FX, [line(1.0), line(1.0)], 0.05)
        self.assertEqual(steps[0]["kind"], "page-origin")

    def test_a_line_whose_words_disagree_is_low_confidence(self):
        lines = [line(0.0), line(0.0, spread=19.0), line(0.0)]
        steps = cu.steps_for_page(FX, lines, 0.05)
        self.assertTrue(all(s["confidence"] == "low" for s in steps))

    def test_a_single_math_word_line_is_low_confidence(self):
        lines = [line(0.0), line(18.9, n=1, math=True), line(0.0)]
        steps = cu.steps_for_page(FX, lines, 0.05)
        self.assertEqual(steps[0]["confidence"], "low")


class Aggregate(unittest.TestCase):
    def _doc(self, fid, steps):
        return {"id": fid, "pages": [{"page": 1, "steps": steps}]}

    def _step(self, cause, bp, kind="cumulative", affected=10, conf="ok"):
        return {"cause": cause, "step_bp": bp, "kind": kind, "lines_affected": affected,
                "confidence": conf, "ref_y": 100.0, "after_text": "x"}

    def test_one_cause_over_many_fixtures_is_one_row(self):
        docs = [self._doc(f"fx{i}", [self._step("after \\end{itemize}", 1.99)]) for i in range(17)]
        rows = cu.aggregate(docs)
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["fixture_count"], 17)
        self.assertEqual(rows[0]["lines_affected"], 170)

    def test_radius_outranks_magnitude(self):
        docs = [self._doc("a", [self._step("small-but-everywhere", 1.99, affected=40)]),
                self._doc("b", [self._step("big-but-once", 20.0, kind="local", affected=1)])]
        rows = cu.aggregate(docs)
        self.assertEqual(rows[0]["cause"], "small-but-everywhere")

    def test_low_confidence_steps_are_not_ranked(self):
        docs = [self._doc("a", [self._step("artefact", 18.9, conf="low")])]
        self.assertEqual(cu.aggregate(docs), [])


if __name__ == "__main__":
    unittest.main()
