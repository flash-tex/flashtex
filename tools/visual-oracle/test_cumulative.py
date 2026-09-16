#!/usr/bin/env python3
"""Pure-python tests for cumulative.py (no renderer, no pdflatex)."""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import cumulative as cu  # noqa: E402
import pdftext  # noqa: E402
import rank  # noqa: E402


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


def glyph(text, x, y, advance, font="CMMI10", size=10.909, bt=0):
    return {"text": text, "x": x, "y_top": y, "advance": advance,
            "size": size, "font": font, "bt": bt}


class AnchoredMeasurement(unittest.TestCase):
    r"""`lines_from_pairs` measures where `rank.norm` decided identity.

    The geometry is `fixtures/real-world/hw2` page 1: pdfTeX sets `\bigl(`
    from cmex10 on a baseline 8.836 bp above the line, so the reference reads
    `(` and `A...` as two words while the candidate's LatinModernMath variant
    sits on the math baseline and reads `(A...` as one. Both sides put the
    delimiter's origin at x = 243.037 and `A` at x = 248.037, so the *word
    origins* differ by the delimiter's 4.996 bp advance and nothing else.
    """

    X0, XA, Y = 243.037, 248.037, 435.440

    def _page(self, ref_delim_dy=8.836, advance=4.996, cand_shift=0.0):
        ref = pdftext.words_from_glyphs([
            glyph("(", self.X0, self.Y - ref_delim_dy, advance, font="CMEX10"),
            glyph("A", self.XA, self.Y, 8.182),
            glyph("B", self.XA + 40.0, self.Y, 8.182),
        ])
        cand = pdftext.words_from_glyphs([
            glyph("(", self.X0 + cand_shift, self.Y, advance, font="LatinModernMath-Regular"),
            glyph("A", self.XA + cand_shift, self.Y, 8.182, font="LatinModernMath-Regular"),
            glyph("B", self.XA + 40.0, self.Y, 8.182, font="LatinModernMath-Regular"),
        ])
        return ref, cand

    def test_the_two_sides_still_segment_the_delimiter_differently(self):
        ref, cand = self._page()
        self.assertEqual([w["text"] for w in ref], ["(", "A", "B"])
        self.assertEqual([w["text"] for w in cand], ["(A", "B"])

    def test_the_delimiter_is_no_longer_a_five_bp_horizontal_outlier(self):
        ref, cand = self._page()
        pairs, _, _ = rank.align_words(ref, cand)
        lines = cu.lines_from_pairs(pairs, ref, cand)
        self.assertEqual(len(lines), 1)
        # Measured at the word origins this pair read dx = -4.996 and became
        # F7 of the 2026-09-16 sweep; measured at `A` it is exactly zero.
        self.assertEqual([round(w["dx"], 4) for w in lines[0]["_words"]], [0.0, 0.0])
        self.assertEqual(round(lines[0]["dx_max"], 4), 0.0)
        _, outliers = cu.horizontal_findings(lines, cu.GLYPH_GATE)
        self.assertEqual(outliers, [])

    def test_a_genuine_shift_beside_the_delimiter_is_still_reported(self):
        # The anchor must not swallow a real defect: move the whole candidate
        # group right by 3 bp and it has to come back as 3 bp.
        ref, cand = self._page(cand_shift=3.0)
        pairs, _, _ = rank.align_words(ref, cand)
        lines = cu.lines_from_pairs(pairs, ref, cand)
        self.assertEqual(round(lines[0]["_words"][0]["dx"], 4), 3.0)

    def test_the_vertical_profile_cannot_move_with_the_anchor(self):
        r"""`dy`, and so every vertical finding, is untouched by this change.

        `pdftext.words_from_glyphs` starts a new word whenever a glyph's
        baseline differs by 0.05 bp or more, so every glyph of a word shares
        one baseline and `y_alnum` is that baseline too. Anchoring can
        therefore move a word's measured `x` but never its `y` — measured over
        the whole 22-fixture corpus, the anchor moves `x` on 546 of 16496
        reference words and `y` on 0 of them.
        """
        for ref_delim_dy in (0.0, 8.836, 11.952):
            ref, cand = self._page(ref_delim_dy=ref_delim_dy)
            for w in ref + cand:
                if w["y_alnum"] is not None:
                    self.assertLess(abs(w["y_alnum"] - w["y_top"]), 0.05)
            pairs, _, _ = rank.align_words(ref, cand)
            lines = cu.lines_from_pairs(pairs, ref, cand)
            self.assertEqual([round(ln["dy"], 6) for ln in lines], [0.0])
            self.assertEqual([round(ln["ref_y"], 3) for ln in lines], [round(self.Y, 3)])

    def test_reflow_detection_uses_the_same_anchored_points(self):
        r"""A `\Biggl\langle`-sized delimiter must not read as a reflowed word.

        `reflowed` is what excludes a page's whole `dy` profile from the step
        table, so measuring it at the word origins would let one wide
        delimiter suppress every vertical finding on the page. The geometry
        here is the same disagreement, scaled past `REFLOW_DX`: a 60 bp
        delimiter the candidate groups with the `A` that follows it.
        """
        adv, x0 = rank.REFLOW_DX + 10.0, 100.0
        ref = pdftext.words_from_glyphs([
            glyph("(", x0, self.Y - 8.836, adv, font="CMEX10"),
            glyph("A", x0 + adv, self.Y, 8.182),
        ])
        cand = pdftext.words_from_glyphs([
            glyph("(", x0, self.Y, adv, font="LatinModernMath-Regular"),
            glyph("A", x0 + adv, self.Y, 8.182, font="LatinModernMath-Regular"),
        ])
        self.assertEqual([w["text"] for w in ref], ["(", "A"])
        self.assertEqual([w["text"] for w in cand], ["(A"])
        pairs, _, _ = rank.align_words(ref, cand)
        measured = cu.measured_pairs(pairs, ref, cand)
        self.assertEqual(len(measured), 1)
        # At the word origins this pair is 60 bp apart and the page's whole
        # step table would have been thrown away; anchored on `A` it is 0.
        self.assertEqual(round(measured[0][3][0] - measured[0][2][0], 4), 0.0)
        self.assertEqual(sum(1 for _i, _j, rp, cp in measured
                             if abs(cp[0] - rp[0]) > rank.REFLOW_DX), 0)


MATH_SHEET_P1 = r"""Completed box being shipped out [1]
\vbox(680.43001+0.0)x469.75502
.\glue -37.0
.\vbox(717.43001+0.0)x469.75502
..\vbox(12.0+0.0)x469.75502, glue set 12.0fil
...\glue 0.0 plus 1.0fil
...\hbox(0.0+0.0)x469.75502 []
..\glue 25.0
..\vbox(650.43001+0.0)x469.75502, glue set - 0.74042
...\glue(\topskip) 11.0
Completed box being shipped out [2]
\vbox(680.43001+0.0)x469.75502
.\vbox(717.43001+0.0)x469.75502
..\vbox(12.0+0.0)x469.75502, glue set 12.0fil
..\vbox(650.43001+0.0)x469.75502, glue set 215.83968fil
...\glue(\topskip) 1.06242
"""

# `cv` puts the body one level deeper: `geometry` wraps the whole page.
CV_P1 = r"""Completed box being shipped out [1]
\vbox(694.88379+0.0)x484.2088
.\vbox(0.0+0.0)x0.0, glue set 52.45752fil
.\vbox(694.88379+0.0)x484.2088
..\vbox(746.33757+0.0)x498.66258, shifted -14.45378
...\vbox(12.0+0.0)x498.66258, glue set 12.0fil []
...\vbox(679.33757+0.0)x498.66258, glue set 119.58136fil []
"""


class GlueSet(unittest.TestCase):
    def test_reads_the_body_box_not_the_folio_box(self):
        """The 12 pt box beside the body carries `12.0fil` for the page number.

        Picking the first or the shallowest set box reads that one and reports
        every page as stretched to infinity.
        """
        pages = cu.glue_sets_from_log(MATH_SHEET_P1)
        self.assertEqual(len(pages), 2)
        self.assertEqual(pages[0]["glue_set"], -0.74042)
        self.assertEqual(pages[0]["glue_order"], "fin")
        self.assertEqual(pages[0]["set_box_height"], 650.43001)

    def test_a_fil_set_does_not_displace_and_is_not_flagged(self):
        r"""Page 2's `215.83968fil` is the `\vfil` of a short last page.

        An infinite-order set leaves every finite glue at its natural size, so
        the baselines are where the material put them and a step there does
        localise. Flagging it would throw away good measurements — `cv` page 1,
        the corpus's largest cumulative defect, is such a page.
        """
        p2 = cu.glue_sets_from_log(MATH_SHEET_P1)[1]
        self.assertEqual(p2["glue_set"], 0.0)
        self.assertEqual(p2["glue_order"], "fil")
        self.assertFalse(cu.localisation(p2)[0])
        cv = cu.glue_sets_from_log(CV_P1)[0]
        self.assertEqual(cv["glue_set"], 0.0)
        self.assertFalse(cu.localisation(cv)[0])

    def test_a_finite_set_flags_every_step_on_the_page(self):
        lines = [line(0.0, y=0.0), line(0.0, y=12.0), line(-0.5, y=24.0),
                 line(-0.5, y=36.0), line(-0.5, y=48.0)]
        glue = cu.glue_sets_from_log(MATH_SHEET_P1)[0]
        steps = cu.steps_for_page({"documents": []}, lines, cu.STEP_GATE, glue)
        self.assertEqual(len(steps), 1)
        self.assertTrue(steps[0]["cause_not_localised"])
        self.assertIn("0.74042", steps[0]["cause_not_localised_why"])
        # Same profile on a page whose glue was not set: attributable.
        clean = cu.steps_for_page({"documents": []}, lines, cu.STEP_GATE,
                                  cu.glue_sets_from_log(MATH_SHEET_P1)[1])
        self.assertFalse(clean[0]["cause_not_localised"])

    def test_a_truncated_box_line_still_reports_its_set(self):
        r"""`\showboxdepth` appends ` []` to the deepest boxes it prints.

        That is exactly the page body box under a class that nests one level
        deeper (`listings-manual`, `cv`), so a parser that requires the ratio to
        end the line reports those pages as unset — silently, and in the one
        direction that lets an unattributable step back into the ranking.
        """
        log = ("Completed box being shipped out [1]\n"
               r"\vbox(680.43001+0.0)x469.75502" "\n"
               r".\vbox(0.0+0.0)x0.0, glue set 38.00374fil" "\n"
               r"..\vbox(717.43001+0.0)x469.75502" "\n"
               r"...\vbox(12.0+0.0)x469.75502, glue set 12.0fil []" "\n"
               r"...\vbox(650.43001+0.0)x469.75502, glue set - 0.65112 []" "\n")
        page = cu.glue_sets_from_log(log)[0]
        self.assertEqual(page["glue_set"], -0.65112)
        self.assertEqual(page["glue_order"], "fin")
        self.assertTrue(cu.localisation(page)[0])

    def test_an_unmeasured_glue_set_is_not_a_zero_one(self):
        """`None` means "not known", and must flag, or a missing pdflatex
        silently turns every step back into an attributed finding."""
        flagged, why = cu.localisation(None)
        self.assertTrue(flagged)
        self.assertIn("could not be measured", why)

    def test_flagged_lines_leave_the_ranking_but_are_still_reported(self):
        def doc(fid, flagged):
            return {"id": fid, "pages": [{"page": 1, "steps": [
                {"cause": r"\section", "kind": "cumulative", "lines_affected": 40,
                 "step_bp": -0.07, "ref_y": 100.0, "after_text": "x",
                 "cause_not_localised": flagged}]}]}
        rows = cu.aggregate([doc("shrunk", True), doc("clean", False)])
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["lines_affected"], 40)
        self.assertEqual(rows[0]["lines_not_localised"], 40)
        self.assertFalse(rows[0]["only_on_set_glue_pages"])
        self.assertEqual(rows[0]["attributable_fixture_count"], 1)
        only = cu.aggregate([doc("shrunk", True)])
        self.assertEqual(only[0]["lines_affected"], 0)
        self.assertEqual(only[0]["lines_not_localised"], 40)
        self.assertTrue(only[0]["only_on_set_glue_pages"])
        self.assertIsNone(only[0]["max_attributable_step_bp"])

    def test_a_cause_with_no_attributable_witness_ranks_below_one_that_has(self):
        """The ordering that would have kept `\\maketitle` out of fifth place."""
        rows = cu.aggregate([
            {"id": "a", "pages": [{"page": 1, "steps": [
                {"cause": "big-but-unlocalised", "kind": "cumulative", "lines_affected": 900,
                 "step_bp": -0.5, "ref_y": 1.0, "after_text": "x", "cause_not_localised": True}]}]},
            {"id": "b", "pages": [{"page": 1, "steps": [
                {"cause": "small-but-real", "kind": "cumulative", "lines_affected": 3,
                 "step_bp": -0.5, "ref_y": 1.0, "after_text": "x", "cause_not_localised": False}]}]},
        ])
        self.assertEqual([r["cause"] for r in rows], ["small-but-real", "big-but-unlocalised"])

    def test_candidate_overfull_pages_are_read_from_the_engines_diagnostics(self):
        diags = [{"code": "overfull_vbox", "message": "page 3: a line extends 4.20pt past the text area"},
                 {"code": "overfull_hbox", "message": "page 3: something else"},
                 {"code": "overfull_vbox", "message": "page 3: a line extends 1.00pt past the text area"}]
        self.assertEqual(sorted(cu.candidate_overfull_pages(diags)), [3])
        self.assertEqual(len(cu.candidate_overfull_pages(diags)[3]), 2)


if __name__ == "__main__":
    unittest.main()
