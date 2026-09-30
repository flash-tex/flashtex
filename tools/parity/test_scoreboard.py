"""Tests for tools/parity/scoreboard.py (the P5 scoreboard aggregator).

    python3 -m unittest discover -s tools/parity -p 'test_*.py' -v
"""

import contextlib
import io
import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import scoreboard as sb  # noqa: E402

STAGES = [
    {"id": "S0", "name": "vendor", "status": "done", "scoreboard_gate": None},
    {"id": "S3", "name": "flag", "status": "not started", "scoreboard_gate": ["fixtures:P-T2"]},
    {"id": "S5", "name": "flip", "status": "not started", "scoreboard_gate": "all"},
    {"id": "S6", "name": "old route out", "status": "not started", "scoreboard_gate": "all"},
]


def summary(measured, pt1=None, pt2=(0, 0), levels=(0, 0, 0, 0), excluded=None, documents=None,
            pt1_na=None, skipped=0):
    at = {("L%d" % i): {"documents": n, "percent": None} for i, n in enumerate(levels)}
    at["L4"] = {"documents": None, "percent": None}
    p1 = ({"evaluated": 0, "passed": 0, "percent": None, "not_evaluated": pt1_na, "skipped": 0}
          if pt1_na else {"evaluated": pt1[1], "passed": pt1[0], "percent": None,
                          "not_evaluated": None, "skipped": skipped})
    return {"documents": documents if documents is not None else measured + sum((excluded or {}).values()),
            "measured": measured, "excluded": excluded or {},
            "pt": {"excluded": {}, "P-T1": p1,
                   "P-T2": {"evaluated": pt2[1], "passed": pt2[0], "percent": None,
                            "not_evaluated": None, "skipped": 0}},
            "at_least": at}


CLI_NA = "n/a: the flashtex CLI is not a TeX engine"


def write_parity(root, name, tiers, kind="tex", host="h1", command="parity.py --tier fixtures",
                 died=None):
    d = os.path.join(root, name)
    os.makedirs(d)
    meta = {"engine_kind": kind, "engine_version": kind + " 1", "host": host,
            "oracle_pdftex_version": "pdfTeX 1.40.29", "command": command, "date": "2026-09-30"}
    with open(os.path.join(d, "scoreboard.json"), "w") as f:
        json.dump({"schema": "flashtex-parity/1", "meta": meta,
                   "tiers": {t: {"summary": s} for t, s in tiers.items()}}, f)
    docs = {t: [{"id": "%s/%d" % (t, i), "worker_died": bool(died and died.get(t, 0) > i)}
                for i in range(s["documents"])] for t, s in tiers.items()}
    with open(os.path.join(d, "documents.json"), "w") as f:
        json.dump(docs, f)
    return d


def board_for(tmp, new_tiers, old_tiers, extra=None, **kw):
    new = write_parity(tmp, "new", new_tiers, **kw.pop("new_kw", {}))
    old = write_parity(tmp, "old", old_tiers, kind="flashtex-cli", **kw.pop("old_kw", {}))
    srcs = {"new": [("parity",) + sb.load_parity(new, {})], "old": [("parity",) + sb.load_parity(old, {})]}
    for lab, items in (extra or {}).items():
        srcs[lab].extend(items)
    return sb.build(srcs, stages=STAGES, **kw)


def row(board, tier, metric):
    for r in board["rows"]:
        if r["tier"] == tier and r["metric"] == metric:
            return r
    raise KeyError((tier, metric))


class Verdicts(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()

    def test_ahead_equal_and_old_na(self):
        b = board_for(self.tmp, {"fixtures": summary(10, (10, 10), (10, 10), (10, 10, 9, 9))},
                      {"fixtures": summary(10, pt1_na=CLI_NA, pt2=(0, 10), levels=(10, 10, 5, 4))})
        self.assertEqual(row(b, "fixtures", "P-T1")["verdict"], "ahead (old n/a)")
        self.assertEqual(row(b, "fixtures", "P-T2")["verdict"], "ahead")
        self.assertEqual(row(b, "fixtures", "L1")["verdict"], "equal")
        self.assertEqual(row(b, "fixtures", "P-T2")["gates"], ["S3", "S5", "S6"])
        # every other tier of the list is still a row, and it is missing
        self.assertEqual(row(b, "nightly-5k", "(any)")["verdict"], "missing")
        self.assertFalse(b["all_green"])
        self.assertEqual(b["retirement"][1]["gate_state"], "met")      # S3: fixtures P-T2 green
        self.assertEqual(b["retirement"][2]["gate_state"], "not met")  # S5: board not all-green

    def test_behind_opens_issue(self):
        b = board_for(self.tmp, {"templates": summary(5, (5, 5), (5, 5), (3, 3, 3, 3))},
                      {"templates": summary(5, pt1_na=CLI_NA, pt2=(0, 5), levels=(4, 4, 0, 0))})
        self.assertEqual(row(b, "templates", "L0")["verdict"], "behind")
        self.assertEqual(b["behind_tiers"], ["templates"])
        acts = sb.plan_issues(b, [])
        self.assertEqual([(a[0], a[2]) for a in acts], [("create", "templates")])
        self.assertIn(sb.MARKER % "templates", acts[0][3])
        # an existing marked issue is edited, not duplicated
        acts = sb.plan_issues(b, [{"number": 7, "title": "x", "body": "a\n" + sb.MARKER % "templates"}])
        self.assertEqual([(a[0], a[1]) for a in acts], [("edit", 7)])

    def test_recovered_tier_closes_only_its_own_marked_issue(self):
        b = board_for(self.tmp, {"arxiv": summary(10, (9, 9), (10, 10), (10, 10, 10, 10))},
                      {"arxiv": summary(10, pt1_na=CLI_NA, pt2=(0, 10), levels=(1, 1, 0, 0))})
        existing = [{"number": 3, "title": "t", "body": sb.MARKER % "arxiv"},
                    {"number": 4, "title": "unrelated", "body": "no marker"}]
        acts = sb.plan_issues(b, existing)
        self.assertEqual([(a[0], a[1]) for a in acts], [("close", 3)])
        calls = []
        sb.apply_issues(acts, "o/r", gh=lambda args, stdin=None: calls.append(args) or "")
        self.assertEqual(calls[0][:3], ["issue", "close", "3"])

    def test_partial_run_never_closes_or_greens(self):
        b = board_for(self.tmp, {"arxiv": summary(4, (4, 4), (4, 4), (4, 4, 4, 4))},
                      {"arxiv": summary(4, pt1_na=CLI_NA, pt2=(0, 4), levels=(0, 0, 0, 0))},
                      new_kw={"command": "parity.py --tier arxiv --limit 4"},
                      old_kw={"command": "parity.py --tier arxiv --limit 4"})
        self.assertIn("--limit", row(b, "arxiv", "L1")["new"]["partial"])
        acts = sb.plan_issues(b, [{"number": 3, "title": "t", "body": sb.MARKER % "arxiv"}])
        self.assertEqual(acts, [])

    def test_manifest_shortfall_is_partial(self):
        new = write_parity(self.tmp, "n", {"arxiv": summary(4, (4, 4), (4, 4), (4, 4, 4, 4))})
        tiers, _ = sb.load_parity(new, {"arxiv": 149})
        self.assertEqual(tiers["arxiv"]["L1"]["partial"], "4 of 149 manifest entries")

    def test_sample_behind_files_no_issue(self):
        b = board_for(self.tmp, {"templates": summary(5, (5, 5), (5, 5), (3, 3, 3, 3))},
                      {"templates": summary(5, pt1_na=CLI_NA, pt2=(0, 5), levels=(4, 4, 0, 0))},
                      sample_note="local sample")
        self.assertEqual(b["behind_tiers"], ["templates"])  # still shown
        self.assertEqual(sb.plan_issues(b, []), [])

    def test_denominators_differ(self):
        b = board_for(self.tmp, {"packages": summary(10, (10, 10), (10, 10), (10, 10, 10, 10))},
                      {"packages": summary(9, pt1_na=CLI_NA, pt2=(0, 9), levels=(1, 1, 0, 0))})
        self.assertEqual(row(b, "packages", "L0")["verdict"], "denominators differ")

    def test_host_mismatch(self):
        b = board_for(self.tmp, {"fixtures": summary(3, (3, 3), (3, 3), (3, 3, 3, 3))},
                      {"fixtures": summary(3, pt1_na=CLI_NA, pt2=(0, 3), levels=(3, 3, 0, 0))},
                      old_kw={"host": "other"})
        self.assertEqual(row(b, "fixtures", "L0")["verdict"], "host mismatch")
        self.assertEqual(row(b, "fixtures", "P-T1")["verdict"], "ahead (old n/a)")

    def test_arxiv_l1_target(self):
        b = board_for(self.tmp, {"arxiv": summary(10, (8, 8), (8, 10), (9, 8, 8, 8))},
                      {"arxiv": summary(10, pt1_na=CLI_NA, pt2=(0, 10), levels=(1, 1, 0, 0))})
        self.assertEqual(row(b, "arxiv", "L1")["verdict"], "below target")
        self.assertEqual(row(b, "arxiv", "L0")["verdict"], "ahead")

    def test_worker_died_is_invalid(self):
        b = board_for(self.tmp, {"fixtures": summary(3, (3, 3), (3, 3), (3, 3, 3, 3))},
                      {"fixtures": summary(3, pt1_na=CLI_NA, pt2=(0, 3), levels=(3, 3, 0, 0))},
                      new_kw={"died": {"fixtures": 1}})
        self.assertEqual(row(b, "fixtures", "L0")["verdict"], "invalid")

    def test_excluded_carried_with_reason(self):
        b = board_for(self.tmp, {"arxiv": summary(8, (8, 8), (8, 8), (8, 8, 8, 8), excluded={"oracle": 2})},
                      {"arxiv": summary(8, pt1_na=CLI_NA, pt2=(0, 8), levels=(0, 0, 0, 0), excluded={"oracle": 2})})
        self.assertEqual(row(b, "arxiv", "L1")["new"]["excluded"], {"oracle": 2})
        self.assertIn("excluded oracle 2", sb.render_md(b))

    def test_all_green_when_every_tier_complete(self):
        full = {t: summary(4, (4, 4), (4, 4), (4, 4, 4, 4)) for t in
                ("fixtures", "arxiv", "templates", "packages", "nightly-5k")}
        old = {t: summary(4, pt1_na=CLI_NA, pt2=(0, 4), levels=(1, 1, 0, 0)) for t in full}
        suites = sb.parse_latex_suites("latex2e/base: PASS 9 / FAIL 1 / SKIP 0\n"
                                       "failing tests:\n  x [expected]\nOK: 10 ran, 1 failed (all expected), 0 skipped\n")
        smoke = sb.parse_package_smoke("a equal\nb equal\n2 documents, 0 differ\n")
        fdir = os.path.join(self.tmp, "fonts")
        os.makedirs(fdir)
        with open(os.path.join(fdir, "census.json"), "w") as f:
            json.dump({"summary": {"per_family": 6, "by_kind": {
                "all": {"tested": 5, "identical": 4, "both-fail": 1, "failing": []}}}}, f)
        extra = {"new": [("latex-suites", {"latex-suites": suites}, {"host": "h1"}),
                         ("package-smoke", {"package-smoke": smoke}, {"host": "h1"}),
                         ("fonts", {"fonts": sb.load_fonts(fdir)}, {"host": "h1"})]}
        b = board_for(self.tmp, full, old, extra=extra)
        self.assertEqual(row(b, "latex-suites", "tests")["old"]["status"], "n/a")
        self.assertEqual(row(b, "fonts", "fonts")["new"]["of"], 4)
        self.assertTrue(b["all_green"], [(r["tier"], r["metric"], r["verdict"]) for r in b["rows"]
                                         if r["verdict"] not in sb.GREEN])
        self.assertTrue(all(s["gate_state"] in ("met", "none") for s in b["retirement"]))
        # a sample note alone keeps it from being green
        b2 = board_for(tempfile.mkdtemp(), full, old, extra=extra, sample_note="sample")
        self.assertFalse(b2["all_green"])


class Parsers(unittest.TestCase):
    def test_latex_suites_unexpected(self):
        r = sb.parse_latex_suites("latex2e/base: PASS 18 / FAIL 2 / SKIP 0\n"
                                  "latex2e/required/tools: PASS 5 / FAIL 0 / SKIP 0\n"
                                  "failing tests:\n  a [UNEXPECTED]\n  b [expected]\n"
                                  "UNEXPECTED failures: a\n")["tests"]
        self.assertEqual((r["passed"], r["of"]), (24, 25))
        self.assertEqual(r["unexpected"], ["a"])
        self.assertEqual(sb.verdict("latex-suites", "tests", r, sb.cell("n/a"), True), "below target")

    def test_latex_suites_against_reference_run(self):
        eng = ("latex2e/base: PASS 7 / FAIL 3 / SKIP 0\nfailing tests:\n  a [UNEXPECTED]\n"
               "  b [UNEXPECTED]\n  c [expected]\nUNEXPECTED failures: a, b\n")
        ref = ("latex2e/base: PASS 8 / FAIL 2 / SKIP 0\nfailing tests:\n  a [UNEXPECTED]\n"
               "  c [expected]\nUNEXPECTED failures: a\n")
        r = sb.parse_latex_suites(eng, reference=ref)["tests"]
        self.assertEqual(r["unexpected"], ["b"])  # a fails in pdfTeX on this host too
        self.assertEqual((r["passed"], r["of"]), (9, 10))

    def test_latex_suites_crash_is_invalid(self):
        self.assertTrue(sb.parse_latex_suites("Traceback ...\n")["tests"]["invalid"])
        r = sb.parse_latex_suites("latex2e/base: PASS 1 / FAIL 0 / SKIP 0\n")["tests"]
        self.assertTrue(r["invalid"])  # no OK / UNEXPECTED summary line

    def test_package_smoke(self):
        r = sb.parse_package_smoke("amsfonts         equal\ncolor            DIFFERENT (pass 2 log)\n"
                                   "2 documents, 1 differ\n")["documents"]
        self.assertEqual((r["passed"], r["of"]), (1, 2))
        self.assertIn("color", r["note"])
        self.assertTrue(sb.parse_package_smoke("a equal\n")["documents"]["invalid"])
        self.assertTrue(sb.parse_package_smoke("a equal\n3 documents, 0 differ\n")["documents"]["invalid"])

    def test_fonts_sample_below_default(self):
        d = tempfile.mkdtemp()
        with open(os.path.join(d, "census.json"), "w") as f:
            json.dump({"summary": {"per_family": 2, "by_kind": {
                "all": {"tested": 3, "identical": 3, "both-fail": 0}}}}, f)
        self.assertIn("per family", sb.load_fonts(d)["fonts"]["sample"])

    def test_nightly_summary(self):
        d = tempfile.mkdtemp()
        with open(os.path.join(d, "summary.json"), "w") as f:
            json.dump({"schema": "flashtex-nightly/1", "host": {"label": "linux-abc", "node": "nix"},
                       "shards": {"total": 50, "done": 49, "missing": [50]},
                       "engine": {"kind": "tex", "version": "x"},
                       "tiers": {"nightly-5k": {"documents": 100, "measured": 90, "excluded": {"oracle": 10},
                                                "unmeasured": 0, "P-T1": [5, 5], "P-T2": [80, 90],
                                                "L0": [90, 90], "L1": [89, 90], "L2": [88, 90],
                                                "L3": [88, 90], "L4": None}}}, f)
        tiers, ident = sb.load_nightly(d, {"nightly-5k": 100})
        c = tiers["nightly-5k"]["P-T2"]
        self.assertEqual((c["passed"], c["of"]), (80, 90))
        self.assertIn("1 shard(s) missing", c["partial"])
        self.assertNotIn("L4", tiers["nightly-5k"])  # un-rasterised L4 is [0, n] in nightly.py
        self.assertEqual(tiers["nightly-5k"]["P-T1"]["status"], "measured")
        self.assertEqual(ident["host_label"], "linux-abc")

    def test_nightly_v1_pt1_is_na(self):
        d = tempfile.mkdtemp()
        with open(os.path.join(d, "summary.json"), "w") as f:
            json.dump({"engine": {"kind": "flashtex-cli"}, "shards": {"missing": []},
                       "tiers": {"nightly-5k": {"documents": 2, "P-T1": None, "P-T1_not_evaluated": 2,
                                                "P-T2": [0, 2], "L0": [1, 2]}}}, f)
        tiers, _ = sb.load_nightly(d, {})
        self.assertEqual(tiers["nightly-5k"]["P-T1"]["status"], "n/a")

    def test_smoke_subset_is_partial(self):
        r = sb.parse_package_smoke("a equal\n1 documents, 0 differ\n", expected=59)["documents"]
        self.assertEqual(r["partial"], "1 of 59 package-smoke documents")

    def test_cli_writes_outputs(self):
        tmp = tempfile.mkdtemp()
        new = write_parity(tmp, "new", {"fixtures": summary(2, (2, 2), (2, 2), (2, 2, 2, 2))})
        old = write_parity(tmp, "old", {"fixtures": summary(2, pt1_na=CLI_NA, pt2=(0, 2), levels=(2, 2, 0, 0))},
                           kind="flashtex-cli")
        out = os.path.join(tmp, "out")
        summ = os.path.join(tmp, "summary.md")
        with contextlib.redirect_stdout(io.StringIO()):
            rc = sb.main(["--parity", "new=" + new, "--parity", "old=" + old, "--out", out,
                          "--summary", summ, "--manifests", os.path.join(tmp, "none")])
        self.assertEqual(rc, 0)
        with contextlib.redirect_stdout(io.StringIO()):
            rc = sb.main(["--parity", "new=" + new, "--parity", "old=" + old, "--out", out,
                          "--require-green", "--manifests", os.path.join(tmp, "none")])
        self.assertEqual(rc, 1)  # other tiers are missing
        with open(os.path.join(out, "scoreboard.json")) as f:
            self.assertEqual(json.load(f)["schema"], sb.SCHEMA)
        with open(summ) as f:
            self.assertIn("| T3 fixtures | P-T2 |", f.read())


if __name__ == "__main__":
    unittest.main()
