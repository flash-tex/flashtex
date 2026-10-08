"""Tests for tools/parity/scoreboard.py (the P5 scoreboard aggregator).

    python3 -m unittest discover -s tools/parity -p 'test_*.py' -v
"""

import contextlib
import copy
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
    {"id": "S5", "name": "flip", "status": "not started", "scoreboard_gate": "all",
     "other_preconditions": "T1 (lockstep) has 0 new differences"},
    {"id": "S6", "name": "old route out", "status": "not started", "scoreboard_gate": "all"},
]
# manifest sizes for the synthetic runs: every run below is complete unless a test says otherwise
SIZES = {"arxiv": 10, "templates": 10, "packages": 10, "nightly-5k": 10}
SHA = "0123456789abcdef0123456789abcdef01234567"
SHAS = {"new": SHA[:9], "old": SHA[:9]}


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
                 died=None, engine_sha256=None, oracle="pdfTeX 1.40.29"):
    d = os.path.join(root, name)
    os.makedirs(d)
    meta = {"engine_kind": kind, "engine_version": kind + " 1", "host": host,
            "oracle_pdftex_version": oracle, "command": command, "date": "2026-09-30",
            "flashtex_sha256": engine_sha256}
    with open(os.path.join(d, "scoreboard.json"), "w") as f:
        json.dump({"schema": "flashtex-parity/1", "meta": meta,
                   "tiers": {t: {"summary": s} for t, s in tiers.items()}}, f)
    docs = {t: [{"id": "%s/%d" % (t, i), "worker_died": bool(died and died.get(t, 0) > i)}
                for i in range(s.get("documents", 10))] for t, s in tiers.items()}
    with open(os.path.join(d, "documents.json"), "w") as f:
        json.dump(docs, f)
    return d


def nightly(kind="tex", sha=SHA, engine_sha256="E-new", n=10, tiers=None, **over):
    """A well-formed nightly.py summary.json with one complete nightly-5k tier."""
    t = {"documents": n, "measured": n, "excluded": {}, "unmeasured": 0,
         "P-T1": [n, n] if kind == "tex" else None, "P-T1_not_evaluated": 0 if kind == "tex" else n,
         "P-T1_over_cap": 0, "P-T2": [n, n] if kind == "tex" else [0, n],
         "L0": [n, n], "L1": [n, n], "L2": [n, n], "L3": [n, n], "L4": [0, n], "crashes": 0}
    d = {"schema": "flashtex-nightly/1", "git_sha": sha, "host": {"node": "h1", "label": "linux-x"},
         "engine": {"kind": kind, "version": kind, "sha256": engine_sha256},
         "fingerprint": {"oracle_pdftex_version": "pdfTeX 1.40.29"},
         "shards": {"total": 50, "done": 50, "missing": []},
         "expected": ["nightly-5k/%d" % i for i in range(n)], "not_returned": [], "mixed_fingerprint": [],
         "tiers": tiers if tiers is not None else {"nightly-5k": t}}
    d.update(over)
    return d


def write_json(tmp, name, obj):
    p = os.path.join(tmp, name)
    with open(p, "w") as f:
        json.dump(obj, f)
    return p


def board_for(tmp, new_tiers, old_tiers, extra=None, sizes=SIZES, **kw):
    new = write_parity(tmp, "new", new_tiers, **kw.pop("new_kw", {}))
    old = write_parity(tmp, "old", old_tiers, kind="flashtex-cli", **kw.pop("old_kw", {}))
    srcs = {"new": [("parity",) + sb.load_parity(new, sizes)], "old": [("parity",) + sb.load_parity(old, sizes)]}
    for lab, items in (extra or {}).items():
        srcs[lab].extend(items)
    kw.setdefault("shas", SHAS)
    return sb.build(srcs, stages=STAGES, **kw)


def row(board, tier, metric):
    for r in board["rows"]:
        if r["tier"] == tier and r["metric"] == metric:
            return r
    raise KeyError((tier, metric))


def census(tmp, all_=None, per_family=6, selection=None, kinds=None):
    all_ = all_ if all_ is not None else {"tested": 5, "identical": 4, "both-fail": 1, "failing": []}
    by_kind = {k: {"tested": 1, "identical": 1, "both-fail": 0, "failing": []} for k in (kinds or sb.CENSUS_KINDS)}
    by_kind["all"] = all_
    s = {"per_family": per_family, "by_kind": by_kind}
    if selection is not False:
        s["selection"] = selection or {"only": [], "kind": [], "families_available": all_.get("tested")}
    return write_json(tmp, "census-%d.json" % len(os.listdir(tmp)), {"summary": s})


LIST = ("latex2e/base (-e pdftex): 9 tests\nlatex3/l3kernel (-e pdftex): 1 tests\ntotal: 10 tests\n")
T2_OK = ("latex2e/base: PASS 8 / FAIL 1 / SKIP 0\nlatex2e/base: FAILED x\nlatex3/l3kernel: PASS 1 / FAIL 0 / SKIP 0\n"
         "failing tests:\n  x [expected]\nOK: 10 ran, 1 failed (all expected), 0 skipped\n")


def complete_board(tmp, **kw):
    """Every tier, complete and sound: the board is green. Tests break one thing at a time."""
    full = {t: summary(10, (10, 10), (10, 10), (10, 10, 10, 10)) for t in
            ("fixtures", "arxiv", "templates", "packages")}
    old = {t: summary(10, pt1_na=CLI_NA, pt2=(0, 10), levels=(1, 1, 0, 0)) for t in full}
    full.update(kw.pop("new_tiers", {}))
    old.update(kw.pop("old_tiers", {}))
    listing = sb.parse_latex_list(LIST)
    suites = sb.parse_latex_suites(kw.pop("t2", T2_OK), None, listing)
    smoke = sb.parse_package_smoke(kw.pop("smoke", "a equal\nb equal\n2 documents, 0 differ\n"),
                                   expected=kw.pop("smoke_expected", 2))
    fonts = sb.load_fonts(kw.pop("census", None) or census(tmp))
    nn = write_json(tmp, "n-new.json", kw.pop("nightly_new", nightly()))
    old_t4 = kw.pop("nightly_old", nightly(kind="flashtex-cli", engine_sha256="E-old"))
    extra = {"new": [("latex-suites", {"latex-suites": suites}, {"host": "h1"}),
                     ("package-smoke", {"package-smoke": smoke}, {"host": "h1"}),
                     ("fonts", {"fonts": fonts}, {"host": "h1"}),
                     ("nightly",) + sb.load_nightly(nn, SIZES)],
             "old": []}
    if old_t4 is not None:  # None: no v1 T4 run (decision 1: the one-off baseline stands in)
        extra["old"].append(("nightly",) + sb.load_nightly(write_json(tmp, "n-old.json", old_t4), SIZES))
    return board_for(tmp, full, old, extra=extra, **kw)


def red(board):
    return [(r["tier"], r["metric"], r["verdict"]) for r in board["rows"] if r["verdict"] not in sb.GREEN]


class Verdicts(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()

    def test_complete_board_is_green(self):
        b = complete_board(self.tmp)
        self.assertTrue(b["all_green"], (red(b), b["partial"], b["samples"]))
        self.assertTrue(all(s["gate_state"] in ("met", "none") for s in b["retirement"]))
        self.assertEqual(row(b, "latex-suites", "tests")["old"]["status"], "n/a")
        self.assertEqual(row(b, "fonts", "fonts")["new"]["of"], 4)

    def test_ahead_equal_and_missing(self):
        b = board_for(self.tmp, {"fixtures": summary(10, (10, 10), (10, 10), (10, 10, 9, 9))},
                      {"fixtures": summary(10, pt1_na=CLI_NA, pt2=(0, 10), levels=(10, 10, 5, 4))})
        self.assertEqual(row(b, "fixtures", "P-T1")["verdict"], "ahead (old n/a)")
        self.assertEqual(row(b, "fixtures", "P-T2")["verdict"], "ahead")
        self.assertEqual(row(b, "fixtures", "L1")["verdict"], "equal")
        self.assertEqual(row(b, "fixtures", "P-T2")["gates"], ["S3", "S5", "S6"])
        self.assertEqual(row(b, "nightly-5k", "(any)")["verdict"], "missing")
        self.assertFalse(b["all_green"])
        self.assertEqual(b["retirement"][1]["gate_state"], "met")      # S3: fixtures P-T2 green
        self.assertEqual(b["retirement"][2]["gate_state"], "not met")  # S5: board not all-green

    def test_behind_opens_issue(self):
        b = board_for(self.tmp, {"templates": summary(10, (10, 10), (10, 10), (3, 3, 3, 3))},
                      {"templates": summary(10, pt1_na=CLI_NA, pt2=(0, 10), levels=(4, 4, 0, 0))})
        self.assertEqual(row(b, "templates", "L0")["verdict"], "behind")
        acts = sb.plan_issues(b, [])
        self.assertEqual([(a[0], a[2]) for a in acts], [("create", "templates")])
        self.assertIn(sb.MARKER % "templates", acts[0][3])
        acts = sb.plan_issues(b, [{"number": 7, "title": "x", "body": "a\n" + sb.MARKER % "templates"}])
        self.assertEqual([(a[0], a[1]) for a in acts], [("edit", 7)])

    def test_recovered_tier_closes_only_its_own_marked_issue(self):
        b = board_for(self.tmp, {"arxiv": summary(10, (10, 10), (10, 10), (10, 10, 10, 10))},
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

    def test_manifest_shortfall_and_unknown_manifest_are_partial(self):
        new = write_parity(self.tmp, "n", {"arxiv": summary(4, (4, 4), (4, 4), (4, 4, 4, 4))})
        tiers, _ = sb.load_parity(new, {"arxiv": 149})
        self.assertEqual(tiers["arxiv"]["L1"]["partial"], "4 of 149 manifest entries")
        tiers, _ = sb.load_parity(new, {})
        self.assertIn("no arxiv manifest", tiers["arxiv"]["L1"]["partial"])

    def test_sample_behind_files_no_issue(self):
        b = board_for(self.tmp, {"templates": summary(10, (10, 10), (10, 10), (3, 3, 3, 3))},
                      {"templates": summary(10, pt1_na=CLI_NA, pt2=(0, 10), levels=(4, 4, 0, 0))},
                      sample_note="local sample")
        self.assertEqual(b["behind_tiers"], ["templates"])  # still shown
        self.assertEqual(sb.plan_issues(b, []), [])

    def test_denominators_differ(self):
        b = board_for(self.tmp, {"packages": summary(10, (10, 10), (10, 10), (10, 10, 10, 10))},
                      {"packages": summary(9, pt1_na=CLI_NA, pt2=(0, 9), levels=(1, 1, 0, 0), documents=10,
                                           excluded={"harness": 1})})
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

    def test_sample_note_blocks_green(self):
        self.assertFalse(complete_board(self.tmp, sample_note="sample")["all_green"])


class Review1299(unittest.TestCase):
    """One test per finding of the independent review of #1299 at a16de610c."""

    def setUp(self):
        self.tmp = tempfile.mkdtemp()

    # 1. stale or mismatched T4 ------------------------------------------------------
    def test_1_stale_t4_commit_is_invalid(self):
        b = complete_board(self.tmp, nightly_new=nightly(sha="fedcba9876543210" * 2 + "fedcba98"))
        self.assertEqual(row(b, "nightly-5k", "L1")["verdict"], "invalid")
        self.assertIn("stale or foreign", row(b, "nightly-5k", "L1")["new"]["invalid"])
        self.assertFalse(b["all_green"])

    def test_1_t4_without_commit_or_board_sha_is_invalid(self):
        b = complete_board(self.tmp, nightly_new=nightly(sha=None))
        self.assertEqual(row(b, "nightly-5k", "L1")["verdict"], "invalid")
        b = complete_board(tempfile.mkdtemp(), shas={})
        self.assertEqual(row(b, "nightly-5k", "L1")["verdict"], "invalid")

    def test_1_other_engine_binary_is_invalid(self):
        b = complete_board(self.tmp, new_kw={"engine_sha256": "E-parity"})  # T4 ran "E-new"
        self.assertEqual(row(b, "nightly-5k", "P-T2")["verdict"], "invalid")
        self.assertIn("different new engine binary", row(b, "nightly-5k", "P-T2")["new"]["invalid"])
        self.assertEqual(row(b, "fixtures", "P-T2")["verdict"], "ahead")  # the parity run is the reference

    def test_1_other_oracle_is_invalid(self):
        o = nightly(kind="flashtex-cli", engine_sha256="E-old")
        o["fingerprint"]["oracle_pdftex_version"] = "pdfTeX 1.40.28"
        b = complete_board(self.tmp, nightly_old=o)
        self.assertEqual(row(b, "nightly-5k", "L0")["verdict"], "invalid")


    def test_1_not_returned_and_spread_are_partial(self):
        n = nightly(not_returned=["nightly-5k/9"])
        n["tiers"]["nightly-5k"]["documents"] = 9
        b = complete_board(self.tmp, nightly_new=n)
        self.assertIn("not returned", row(b, "nightly-5k", "L1")["new"]["partial"])
        self.assertFalse(b["all_green"])
        # a --spread run expects fewer documents than the manifest lists
        n = nightly(n=3)
        b = complete_board(tempfile.mkdtemp(), nightly_new=n)
        self.assertIn("3 of 10 manifest entries", row(b, "nightly-5k", "L1")["new"]["partial"])

    def test_1_nightly_format_drift_raises(self):
        for k in ("shards", "expected", "not_returned", "git_sha", "fingerprint"):
            d = nightly()
            d.pop(k)
            p = write_json(self.tmp, "drift-%s.json" % k, d)
            with self.assertRaises(sb.FormatError, msg=k):
                sb.load_nightly(p, SIZES)
        d = nightly()
        d["shards"] = {"expected": 20, "present": 17}
        with self.assertRaises(sb.FormatError):
            sb.load_nightly(write_json(self.tmp, "drift-shards.json", d), SIZES)
        d = nightly()
        d["shards"] = {"total": 50, "done": 47, "missing": [3, 7, 9]}
        tiers, _ = sb.load_nightly(write_json(self.tmp, "short.json", d), SIZES)
        self.assertIn("47 of 50 shard(s) done", tiers["nightly-5k"]["L1"]["partial"])

    # 2. corpus-t4 also runs the T3 tiers ------------------------------------------
    def test_2_nightly_t3_tiers_are_ignored(self):
        n = nightly()
        n["tiers"]["arxiv"] = copy.deepcopy(n["tiers"]["nightly-5k"])
        b = complete_board(self.tmp, nightly_new=n)
        self.assertEqual(row(b, "arxiv", "L1")["verdict"], "ahead")  # parity.py's, not "two results"
        self.assertTrue(b["all_green"], red(b))
        tiers, ident = sb.load_nightly(write_json(self.tmp, "t.json", n), SIZES)
        self.assertEqual(sorted(tiers), ["nightly-5k"])
        self.assertEqual(ident["ignored_tiers"], ["arxiv"])

    # 3. format drift and zero denominators -----------------------------------------
    def test_3_fonts_drift_raises(self):
        for bad in ({}, {"same": 3, "count": 5}, {"tested": 5, "identical": 5}):
            with self.assertRaises(sb.FormatError, msg=bad):
                sb.load_fonts(census(self.tmp, all_=bad))

    def test_3_fonts_selection_and_zero(self):
        c = sb.load_fonts(census(self.tmp, selection={"only": ["cm"], "kind": [], "families_available": 5}))
        self.assertIn("--only", c["fonts"]["sample"])
        c = sb.load_fonts(census(self.tmp, selection={"only": [], "kind": [], "families_available": 9}))
        self.assertIn("5 of 9 available", c["fonts"]["sample"])
        c = sb.load_fonts(census(self.tmp, selection=False))
        self.assertIn("does not record", c["fonts"]["sample"])
        c = sb.load_fonts(census(self.tmp, kinds=["type1"]))
        self.assertIn("no opentype", c["fonts"]["sample"])
        c = sb.load_fonts(census(self.tmp, all_={"tested": 2, "identical": 0, "both-fail": 2}))
        self.assertTrue(c["fonts"]["invalid"])
        b = complete_board(self.tmp, census=census(self.tmp, per_family=2))
        self.assertFalse(b["all_green"])

    def test_3_latex_suites_zero_and_short(self):
        listing = sb.parse_latex_list(LIST)
        r = sb.parse_latex_suites("base: PASS 0 / FAIL 0 / SKIP 0\nOK: 0 ran, 0 failed\n", None, listing)
        self.assertTrue(r["tests"]["invalid"])
        r = sb.parse_latex_suites("latex2e/base: PASS 5 / FAIL 0 / SKIP 0\nOK: 5 ran, 0 failed\n", None, listing)
        self.assertIn("5 of 10 listed tests ran", r["tests"]["partial"])
        r = sb.parse_latex_suites("latex2e/base: PASS 20 / FAIL 0 / SKIP 0\nOK: 20 ran, 0 failed\n")
        self.assertIn("count unknown", r["tests"]["partial"])  # no --list: never complete
        r = sb.parse_latex_suites("latex2e/base: PASS 20 / FAIL 0 / SKIP 0\nOK: 20 ran, 0 failed\n", None, listing)
        self.assertIn("more tests ran", r["tests"]["invalid"])  # base lists 9
        r = sb.parse_latex_suites("other/dir: PASS 1 / FAIL 0 / SKIP 0\nOK: 1 ran, 0 failed\n", None, listing)
        self.assertTrue(r["tests"]["invalid"])
        with self.assertRaises(sb.FormatError):
            sb.parse_latex_list("nothing here\n")

    def test_3_package_smoke_zero_and_default_count(self):
        self.assertTrue(sb.parse_package_smoke("0 documents, 0 differ\n")["documents"]["invalid"])
        r = sb.parse_package_smoke("a equal\n1 documents, 0 differ\n", None)["documents"]
        self.assertIn("of %d package-smoke documents" % sb.smoke_documents(), r["partial"])

    def test_3_parity_zero_denominator_is_invalid(self):
        z = summary(0, pt1=(0, 0), pt2=(0, 0), levels=(0, 0, 0, 0), documents=10, excluded={"oracle": 10})
        zo = summary(0, pt1_na=CLI_NA, pt2=(0, 0), levels=(0, 0, 0, 0), documents=10, excluded={"oracle": 10})
        b = board_for(self.tmp, {"fixtures": z}, {"fixtures": zo})
        self.assertEqual({r["verdict"] for r in b["rows"] if r["tier"] == "fixtures"}, {"invalid"})

    def test_3_parity_missing_fields_raise(self):
        for path in (("measured",), ("pt", "P-T1", "evaluated"), ("pt", "P-T2", "passed"),
                     ("at_least", "L1"), ("documents",)):
            s = summary(10, (10, 10), (10, 10), (10, 10, 10, 10))
            d = s
            for k in path[:-1]:
                d = d[k]
            d.pop(path[-1])
            p = write_parity(tempfile.mkdtemp(), "x", {"fixtures": s})
            with self.assertRaises(sb.FormatError, msg=path):
                sb.load_parity(p, SIZES)
        p = write_parity(self.tmp, "nodocs", {"fixtures": summary(1, (1, 1), (1, 1), (1, 1, 1, 1))})
        os.remove(os.path.join(p, "documents.json"))
        with self.assertRaises(sb.FormatError):
            sb.load_parity(p, SIZES)

    # 4. a bar for rows v1 cannot run -----------------------------------------------
    def test_4_old_na_needs_the_bar(self):
        b = complete_board(self.tmp, smoke="a equal\nb DIFFERENT (log)\n2 documents, 1 differ\n")
        self.assertEqual(row(b, "package-smoke", "documents")["verdict"], "below bar (old n/a)")
        c = sb.load_fonts(census(self.tmp, all_={"tested": 5, "identical": 0, "both-fail": 0}))
        self.assertEqual(sb.verdict("fonts", "fonts", c["fonts"], sb.cell("n/a"), True), "below bar (old n/a)")
        # P-T1 with a skipped document is not 100% of what was measured
        new = sb.cell("measured", 9, 9, skipped=1)
        self.assertEqual(sb.verdict("arxiv", "P-T1", new, sb.cell("n/a"), True), "below bar (old n/a)")
        # a recorded baseline lowers the bar to itself, never below
        part = sb.cell("measured", 9, 10)
        base = {"package-smoke:documents": {"passed": 9, "of": 10}}
        self.assertEqual(sb.verdict("package-smoke", "documents", part, sb.cell("n/a"), True, base),
                         "ahead (old n/a)")
        base = {"package-smoke:documents": {"passed": 10, "of": 10}}
        self.assertEqual(sb.verdict("package-smoke", "documents", part, sb.cell("n/a"), True, base),
                         "below bar (old n/a)")

    # 5. closing issues ------------------------------------------------------------
    def test_5_close_needs_every_row_clean(self):
        existing = [{"number": 7, "title": "x", "body": sb.MARKER % "fonts"},
                    {"number": 8, "title": "x", "body": sb.MARKER % "fixtures"}]
        b = complete_board(self.tmp)
        self.assertEqual(sorted(a[1] for a in sb.plan_issues(b, existing) if a[0] == "close"), [7, 8])
        # a fonts sample (per-family 2), a sample-noted board, a host mismatch, a partial row: no close
        b = complete_board(tempfile.mkdtemp(), census=census(self.tmp, per_family=2))
        self.assertNotIn(7, [a[1] for a in sb.plan_issues(b, existing)])
        b = complete_board(tempfile.mkdtemp(), sample_note="sample")
        self.assertEqual(sb.plan_issues(b, existing), [])
        b = complete_board(tempfile.mkdtemp(), old_kw={"host": "other"})
        self.assertNotIn(8, [a[1] for a in sb.plan_issues(b, existing)])
        b = complete_board(tempfile.mkdtemp(), new_kw={"command": "parity.py --limit 3"},
                           old_kw={"command": "parity.py --limit 3"})
        self.assertNotIn(8, [a[1] for a in sb.plan_issues(b, existing)])
        b = complete_board(tempfile.mkdtemp(), new_kw={"died": {"fixtures": 1}})
        self.assertNotIn(8, [a[1] for a in sb.plan_issues(b, existing)])

    # 6. S5 --------------------------------------------------------------------------
    def test_6_s5_line_names_the_other_preconditions(self):
        b = complete_board(self.tmp)
        self.assertTrue(b["all_green"])
        status = sb.render_status(b)
        self.assertNotIn("may start", status)
        self.assertIn("T1 (lockstep) has 0 new differences", status)
        stages = {s["id"]: s for s in sb.load_stages()}
        self.assertIn("T1 (lockstep) has 0 new differences", stages["S5"]["other_preconditions"])

    def test_6_s5_and_s6_carry_their_app_parity_rows(self):
        stages = {s["id"]: s for s in sb.load_stages()}
        self.assertEqual(stages["S5"].get("app_parity_gate"), "S5")
        self.assertEqual(stages["S6"].get("app_parity_gate"), "S6")
        self.assertIsNone(sb.app_parity_state(None))
        ap = sb.app_parity_state("S5")
        self.assertIn(ap["state"], ("met", "not met"))
        self.assertEqual(ap["state"] == "met", ap["why"] == [])
        board = complete_board(self.tmp)
        for st in board["retirement"]:
            self.assertIn("app_parity", st)


BACKEND = ("latex3/l3kernel[config-backend]@etex-dvips: PASS %d / FAIL %d / SKIP 0\n%s"
           "latex3/l3kernel[config-backend]@etex-dvisvgm: PASS %d / FAIL %d / SKIP 0\n%s"
           "failing tests:\n  m3backend01 [UNEXPECTED]\nUNEXPECTED failures: m3backend01\n")


class Rereview1299(unittest.TestCase):
    """One test per finding of the re-review of #1299 at 2b6306b80."""

    # 1. T2 failures keyed by (directory, test), not by bare name
    def test_1_same_name_failing_in_other_directory_is_unexpected(self):
        dvips = "latex3/l3kernel[config-backend]@etex-dvips: FAILED m3backend01\n"
        dvisvgm = "latex3/l3kernel[config-backend]@etex-dvisvgm: FAILED m3backend01\n"
        ref = BACKEND % (2, 0, "", 1, 1, dvisvgm)   # pdfTeX fails it under dvisvgm
        eng = BACKEND % (1, 1, dvips, 2, 0, "")     # the engine fails it under dvips
        r = sb.parse_latex_suites(eng, reference=ref)["tests"]
        self.assertEqual(r["unexpected"], ["latex3/l3kernel[config-backend]@etex-dvips:m3backend01"])
        self.assertEqual((r["passed"], r["of"]), (3, 4))
        self.assertNotIn(sb.verdict("latex-suites", "tests", r, sb.cell("n/a"), True), sb.GREEN)
        # the same failure in the same directory is not unexpected
        r = sb.parse_latex_suites(ref, reference=ref)["tests"]
        self.assertEqual((r["unexpected"], r["passed"]), ([], 4))

    def test_1_fail_counts_must_match_the_failure_lines(self):
        # FAIL 1 but no per-directory FAILED line (an old transcript, or a lost block)
        r = sb.parse_latex_suites("latex2e/base: PASS 1 / FAIL 1 / SKIP 0\n"
                                  "UNEXPECTED failures: a\n")["tests"]
        self.assertIn("do not match the FAIL counts", r["invalid"])
        # an UNEXPECTED line with no parsable `failing tests:` block
        r = sb.parse_latex_suites("latex2e/base: PASS 1 / FAIL 1 / SKIP 0\nlatex2e/base: FAILED a\n"
                                  "UNEXPECTED failures: a\n")["tests"]
        self.assertIn("failing tests", r["invalid"])
        # UNEXPECTED names a test no directory failed
        r = sb.parse_latex_suites("latex2e/base: PASS 2 / FAIL 0 / SKIP 0\nUNEXPECTED failures: zz\n")["tests"]
        self.assertIn("no directory failed", r["invalid"])
        # the reference transcript is held to the same rule
        ok = "latex2e/base: PASS 2 / FAIL 0 / SKIP 0\nOK: 2 ran, 0 failed (all expected), 0 skipped\n"
        bad_ref = "latex2e/base: PASS 1 / FAIL 1 / SKIP 0\nOK: 2 ran, 1 failed (all expected), 0 skipped\n"
        self.assertIn("pdfTeX reference", sb.parse_latex_suites(ok, reference=bad_ref)["tests"]["invalid"])


class Round3Review1299(unittest.TestCase):
    """Round-3 review of #1299 at 47b580d3c: a stale EXPECTED-FAILURES entry is not a failure."""

    ENG = ("latex2e/base: PASS 5 / FAIL 0 / SKIP 0\n"
           "stale EXPECTED-FAILURES entries now passing: x\n"
           "stale entries fail the gate: remove them or pass --allow-stale\n"
           "UNEXPECTED failures: x\n")
    REF = ("latex2e/base: PASS 4 / FAIL 1 / SKIP 0\nlatex2e/base: FAILED x\n"
           "failing tests:\n  x [expected]\nOK: 5 ran, 1 failed (all expected), 0 skipped\n")

    def test_stale_entry_with_reference_is_measured(self):
        # the engine passes x, pdfTeX fails x, EXPECTED lists x: 5/5, not INVALID at 0/5
        r = sb.parse_latex_suites(self.ENG, reference=self.REF)["tests"]
        self.assertIsNone(r["invalid"])
        self.assertEqual((r["passed"], r["of"], r["unexpected"]), (5, 5, []))

    def test_stale_entry_without_reference_is_not_a_failure(self):
        r = sb.parse_latex_suites(self.ENG)["tests"]
        self.assertIsNone(r["invalid"])
        self.assertEqual((r["passed"], r["unexpected"]), (5, []))
        # a non-stale UNEXPECTED name with no failure behind it still fails closed
        bad = self.ENG.replace("UNEXPECTED failures: x", "UNEXPECTED failures: x, y")
        self.assertIn("no directory failed", sb.parse_latex_suites(bad)["tests"]["invalid"])

    def test_driver_passes_allow_stale_to_both_t2_runs(self):
        with open(os.path.join(REPO, "tools", "parity", "scoreboard-run.sh")) as f:
            runs = [l for l in f.read().replace("\\\n", " ").splitlines()
                    if "python3 tools/latex-suites/run.py" in l and "--list" not in l]
        self.assertEqual(len(runs), 2)
        self.assertTrue(all("--allow-stale" in l for l in runs), runs)


class Rereview1299T4(unittest.TestCase):
    # 3. corpus-t4 uploads no v1 leg: the board says so, and the row is not green
    def test_3_missing_v1_t4_leg_is_explicit(self):
        tmp = tempfile.mkdtemp()
        full = {t: summary(10, (10, 10), (10, 10), (10, 10, 10, 10)) for t in ("fixtures",)}
        old = {t: summary(10, pt1_na=CLI_NA, pt2=(0, 10), levels=(1, 1, 0, 0)) for t in full}
        nn = write_json(tmp, "n.json", nightly())
        b = board_for(tmp, full, old, extra={"new": [("nightly",) + sb.load_nightly(nn, SIZES)]})
        r = row(b, "nightly-5k", "L1")
        self.assertEqual(r["verdict"], "missing")
        self.assertEqual(sb.fmt_cell(r["old"]), "missing (no v1 one-off baseline (decision 1))")
        self.assertIn("missing (no v1 one-off baseline (decision 1))", sb.render_table(b))
        self.assertFalse(b["all_green"])


REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def jobs_of(workflow_text):
    """{job name: its text} for a workflow's top-level jobs (two-space indent)."""
    body = workflow_text.split("\njobs:\n", 1)[1]
    out, name = {}, None
    for line in body.splitlines():
        if line.startswith("  ") and not line.startswith("   ") and line.rstrip().endswith(":"):
            name = line.strip()[:-1]
            out[name] = []
        elif name:
            out[name].append(line)
    return {k: "\n".join(v) for k, v in out.items()}


class Rereview1299Wiring(unittest.TestCase):
    # 2. no write token near third-party sources
    def test_2_scoreboard_job_holds_no_write_token(self):
        with open(os.path.join(REPO, ".github", "workflows", "p5-scoreboard.yml")) as f:
            wf = f.read()
        jobs = jobs_of(wf)
        self.assertEqual(wf.count("actions/checkout@v4"), wf.count("persist-credentials: false"))
        sb_job = jobs["scoreboard"]
        self.assertNotIn(": write", sb_job)
        self.assertNotIn("--issues apply", sb_job)
        self.assertIn("--issues off", sb_job)
        # the token is in exactly one step's env there, and that step runs before any TeX
        self.assertEqual(sb_job.count("GH_TOKEN"), 1)
        self.assertLess(sb_job.index("GH_TOKEN"), sb_job.index("scoreboard-run.sh"))
        self.assertNotIn("GH_TOKEN", wf.split("\njobs:\n", 1)[0])  # no workflow-level token
        pub = jobs["publish"]
        self.assertIn("contents: write", pub)
        self.assertNotIn("scoreboard-run.sh", pub)  # the write job runs no TeX
        self.assertIn("--from-board", pub)

    def test_2_from_board_plans_issues_without_measuring(self):
        tmp = tempfile.mkdtemp()
        b = board_for(tmp, {"templates": summary(10, (10, 10), (10, 10), (3, 3, 3, 3))},
                      {"templates": summary(10, pt1_na=CLI_NA, pt2=(0, 10), levels=(4, 4, 0, 0))})
        path = write_json(tmp, "board.json", b)
        out = io.StringIO()
        orig = sb.existing_issues
        sb.existing_issues = lambda repo, gh=None: []  # no network in tests
        try:
            with contextlib.redirect_stdout(out):
                rc = sb.main(["--from-board", path, "--issues", "dry-run", "--repo", "o/r"])
        finally:
            sb.existing_issues = orig
        self.assertEqual(rc, 0)
        self.assertIn("issue create (new) templates", out.getvalue())
        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(sb.main(["--from-board", write_json(tmp, "x.json", {"schema": "other"})]), 2)

    # 4. the engine is built alone, as nightly.yml's corpus-t4 builds it
    def test_4_engine_built_in_its_own_cargo_invocation(self):
        with open(os.path.join(REPO, "tools", "parity", "scoreboard-run.sh")) as f:
            builds = [l.strip() for l in f if l.strip().startswith("cargo build")]
        self.assertIn("cargo build --release --locked -p flashtex-engine --bin flashtex-initex", builds)
        self.assertFalse([b for b in builds if "flashtex-engine" in b and "flashtex-cli" in b])


class Parsers(unittest.TestCase):
    def test_latex_suites_unexpected(self):
        r = sb.parse_latex_suites("latex2e/base: PASS 18 / FAIL 2 / SKIP 0\nlatex2e/base: FAILED a b\n"
                                  "latex2e/required/tools: PASS 5 / FAIL 0 / SKIP 0\n"
                                  "failing tests:\n  a [UNEXPECTED]\n  b [expected]\n"
                                  "UNEXPECTED failures: a\n")["tests"]
        self.assertEqual((r["passed"], r["of"]), (24, 25))
        self.assertEqual(r["unexpected"], ["latex2e/base:a"])
        r = dict(r, partial=None)
        self.assertEqual(sb.verdict("latex-suites", "tests", r, sb.cell("n/a"), True,
                                    {"latex-suites:tests": {"passed": 1, "of": 2}}), "below target")

    def test_latex_suites_against_reference_run(self):
        eng = ("latex2e/base: PASS 7 / FAIL 3 / SKIP 0\nlatex2e/base: FAILED a b c\nfailing tests:\n  a [UNEXPECTED]\n"
               "  b [UNEXPECTED]\n  c [expected]\nUNEXPECTED failures: a, b\n")
        ref = ("latex2e/base: PASS 8 / FAIL 2 / SKIP 0\nlatex2e/base: FAILED a c\nfailing tests:\n  a [UNEXPECTED]\n"
               "  c [expected]\nUNEXPECTED failures: a\n")
        r = sb.parse_latex_suites(eng, reference=ref)["tests"]
        self.assertEqual(r["unexpected"], ["latex2e/base:b"])  # a fails in pdfTeX on this host too
        self.assertEqual((r["passed"], r["of"]), (9, 10))
        other = "latex2e/base: PASS 4 / FAIL 0 / SKIP 0\nOK: 4 ran, 0 failed\n"
        self.assertTrue(sb.parse_latex_suites(eng, reference=other)["tests"]["invalid"])

    def test_latex_suites_crash_is_invalid(self):
        self.assertTrue(sb.parse_latex_suites("Traceback ...\n")["tests"]["invalid"])
        r = sb.parse_latex_suites("latex2e/base: PASS 1 / FAIL 0 / SKIP 0\n")["tests"]
        self.assertTrue(r["invalid"])  # no OK / UNEXPECTED summary line

    def test_package_smoke(self):
        r = sb.parse_package_smoke("amsfonts         equal\ncolor            DIFFERENT (pass 2 log)\n"
                                   "2 documents, 1 differ\n", expected=2)["documents"]
        self.assertEqual((r["passed"], r["of"]), (1, 2))
        self.assertIn("color", r["note"])
        self.assertTrue(sb.parse_package_smoke("a equal\n")["documents"]["invalid"])
        self.assertTrue(sb.parse_package_smoke("a equal\n3 documents, 0 differ\n")["documents"]["invalid"])

    def test_package_smoke_reference_failures_are_excluded(self):
        why = "the reference does not compile it on this TeX Live; the candidate fails identically"
        r = sb.parse_package_smoke("a equal\nb EXCLUDED (%s: exit 1)\nc equal\n"
                                   "3 documents, 0 differ, 1 excluded (%s)\n" % (why, why), expected=3)["documents"]
        self.assertEqual((r["passed"], r["of"]), (2, 2))
        self.assertEqual(r["excluded"], {why: 1})
        self.assertIsNone(r.get("partial"))
        self.assertIn("excluded: b", r["note"])
        # the summary must count them
        r = sb.parse_package_smoke("a equal\nb EXCLUDED (x: exit 1)\n2 documents, 0 differ\n", expected=2)
        self.assertTrue(r["documents"]["invalid"])

    def test_smoke_subset_is_partial(self):
        r = sb.parse_package_smoke("a equal\n1 documents, 0 differ\n", expected=59)["documents"]
        self.assertEqual(r["partial"], "1 of 59 package-smoke documents")

    def test_nightly_summary(self):
        d = nightly(shards={"total": 50, "done": 49, "missing": [50]})
        tiers, ident = sb.load_nightly(write_json(tempfile.mkdtemp(), "s.json", d), SIZES)
        c = tiers["nightly-5k"]["P-T2"]
        self.assertEqual((c["passed"], c["of"]), (10, 10))
        self.assertIn("49 of 50 shard(s) done", c["partial"])
        self.assertNotIn("L4", tiers["nightly-5k"])  # un-rasterised L4 is [0, n] in nightly.py
        self.assertEqual(ident["host_label"], "linux-x")

    def test_nightly_v1_pt1_is_na(self):
        tiers, _ = sb.load_nightly(write_json(tempfile.mkdtemp(), "s.json", nightly(kind="flashtex-cli")), SIZES)
        self.assertEqual(tiers["nightly-5k"]["P-T1"]["status"], "n/a")

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
        os.remove(os.path.join(new, "documents.json"))
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            rc = sb.main(["--parity", "new=" + new, "--parity", "old=" + old, "--out", out])
        self.assertEqual(rc, 2)  # a format error fails closed


class OneOracle(unittest.TestCase):
    """P5-T4-MAC: the version string is the same on the PC and the Macs, so T4 is tied to the
    board's oracle by its texlive.tlpdb and pdftex sha256 (DESIGN §8)."""

    MAC = {"pdftex": "/usr/local/texlive/2026/bin/universal-darwin/pdftex", "pdftex_sha256": "3ead7b" + "0" * 58,
           "texlive_root": "/usr/local/texlive/2026", "tlpdb_sha256": "ca39e6" + "0" * 58,
           "latex_format": "2025-11-01", "version": "pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026)"}
    PC = {"pdftex": "/opt/texlive/2026/bin/x86_64-linux/pdftex", "pdftex_sha256": "aa11" + "0" * 60,
          "texlive_root": "/opt/texlive/2026", "tlpdb_sha256": "bb22" + "0" * 60}

    def setUp(self):
        self.tmp = tempfile.mkdtemp()

    def t4(self, oracle, **kw):
        n = nightly(**kw)
        if oracle is not None:
            n["fingerprint"]["oracle"] = {k: oracle[k] for k in ("pdftex", "pdftex_sha256", "texlive_root",
                                                                  "tlpdb_sha256")}
        return n

    def test_same_oracle_is_green(self):
        b = complete_board(self.tmp, oracle=self.MAC, nightly_new=self.t4(self.MAC),
                           nightly_old=self.t4(self.MAC, kind="flashtex-cli", engine_sha256="E-old"))
        self.assertTrue(b["all_green"], red(b))
        self.assertEqual(b["oracle"]["tlpdb_sha256"], self.MAC["tlpdb_sha256"])
        self.assertIn("texlive.tlpdb `ca39e6000000`", sb.render_md(b))

    def test_pc_t4_on_a_mac_board_is_invalid(self):
        b = complete_board(self.tmp, oracle=self.MAC, nightly_new=self.t4(self.PC),
                           nightly_old=self.t4(self.MAC, kind="flashtex-cli", engine_sha256="E-old"))
        r = row(b, "nightly-5k", "L1")
        self.assertEqual(r["verdict"], "invalid")
        self.assertIn("another oracle: texlive.tlpdb bb2200000000 (/opt/texlive/2026)", r["new"]["invalid"])
        self.assertEqual(row(b, "arxiv", "L1")["verdict"], "ahead")  # the board's own runs are untouched
        self.assertFalse(b["all_green"])

    def test_same_tlpdb_other_pdftex_is_invalid(self):
        o = dict(self.MAC, pdftex_sha256="ff" * 32)
        b = complete_board(self.tmp, oracle=self.MAC, nightly_new=self.t4(o))
        self.assertIn("pdftex binary", row(b, "nightly-5k", "P-T2")["new"]["invalid"])

    def test_t4_without_oracle_record_is_invalid_on_a_board_with_one(self):
        b = complete_board(self.tmp, oracle=self.MAC, nightly_new=self.t4(None))
        self.assertIn("records no oracle identity", row(b, "nightly-5k", "L0")["new"]["invalid"])
        # without a board oracle there is nothing to tie it to: as before
        b = complete_board(tempfile.mkdtemp(), nightly_new=self.t4(None))
        self.assertIsNone(row(b, "nightly-5k", "L0")["new"].get("invalid"))

    def test_two_runs_disagree_without_a_board_oracle(self):
        b = complete_board(self.tmp, nightly_new=self.t4(self.MAC),
                           nightly_old=self.t4(self.PC, kind="flashtex-cli", engine_sha256="E-old"))
        self.assertIn("another oracle", row(b, "nightly-5k", "L1")["old"]["invalid"])

    def test_todays_nightly_tier_row_loads(self):
        """nightly.tier_row no longer writes P-T1_over_cap (P-T1 streams large logs); found by
        the P5-T4-MAC local proof, where the board refused the first real summary (exit 2)."""
        import nightly
        recs = [{"excluded": None, "P-T1": True, "P-T2": True, "level_index": 3},
                {"excluded": None, "P-T1": None, "P-T2": True, "level_index": 3,
                 "pt1_not_evaluated": "not evaluated: outside the P-T1 sample (0.05 of tier nightly-5k, --pt1-sample)"},
                {"excluded": "oracle: pdflatex exit 1", "P-T1": None, "P-T2": None}]
        n = self.t4(self.MAC, n=2)
        n["tiers"]["nightly-5k"] = nightly.tier_row(recs)
        n["expected"] = ["nightly-5k/%d" % i for i in range(3)]
        self.assertNotIn("P-T1_over_cap", n["tiers"]["nightly-5k"])
        tiers, _ = sb.load_nightly(write_json(self.tmp, "today.json", n), {"nightly-5k": 3})
        c = tiers["nightly-5k"]["P-T1"]
        self.assertEqual((c["passed"], c["of"]), (1, 1))
        self.assertEqual(c["skipped"], 0)  # outside the sample: out of the denominator by rule
        self.assertEqual(c["excluded"]["P-T1 outside the --pt1-sample (by rule)"], 1)
        self.assertEqual(tiers["nightly-5k"]["L3"]["of"], 2)

    def test_cli_oracle_flag(self):
        o = write_json(self.tmp, "oracle.json", self.MAC)
        nn = write_json(self.tmp, "t4.json", self.t4(self.PC))
        out = os.path.join(self.tmp, "board")
        with contextlib.redirect_stdout(io.StringIO()):
            sb.main(["--nightly", "new=" + nn, "--sha", "new=" + SHA, "--oracle", o, "--out", out])
        with open(os.path.join(out, "scoreboard.json")) as f:
            board = json.load(f)
        self.assertIn("another oracle", row(board, "nightly-5k", "L1")["new"]["invalid"])


class Decision1(unittest.TestCase):
    """T4 decision 1 (Commander, 2026-10-02, #1319 comment 5960583653): the old column of T4 is
    a committed one-off v1 measurement, tools/parity/baselines/t4-v1-oneoff.json. Its rate is
    compared on its own slice: a FINAL baseline against the new run restricted to its IDs, a
    PROVISIONAL one against its new_same_slice (else the whole new run)."""

    IDS = ["1601.00001v1", "1601.00002v1", "1601.00003v1", "1601.00004v1"]

    def setUp(self):
        self.tmp = tempfile.mkdtemp()

    def baseline(self, **over):
        b = {"schema": sb.T4_V1_SCHEMA, "status": "FINAL", "tier": "nightly-5k",
             "decision": {"date": "2026-10-02", "url": "https://example.invalid/r", "remeasure_when": "a D13 fix"},
             "measured_date": "2026-10-01", "source": ["https://example.invalid/m"],
             "documents": {"measured": 4}, "ids": list(self.IDS), "id_list_sha256": sb.id_list_sha256(self.IDS),
             "oracle": {"tlpdb_sha256": "cd" * 32, "pdftex_sha256": "ef" * 32},
             "v1": {"P-T2": [0, 4], "L0": [1, 4], "L1": [1, 4], "L2": [0, 4], "L3": [0, 4]}}
        b.update(over)
        return sb.load_t4_v1_baseline(write_json(self.tmp, "b%d.json" % len(os.listdir(self.tmp)), b))

    def provisional(self, **over):
        o = dict(status="PROVISIONAL", ids=None, id_list_sha256=None, oracle=None, provisional="hash pending")
        o.update(over)
        return self.baseline(**o)

    def board(self, t4_v1, docs=None, **kw):
        """docs: {id: level_index} written as the new T4 run's documents.json (None: no file)."""
        d = tempfile.mkdtemp()
        if docs is not None:
            recs = [{"tier": "nightly-5k", "id": i, "level_index": lv, "P-T1": None,
                     "P-T2": lv is not None and lv >= 0} for i, lv in docs.items()]
            write_json(d, "documents.json", {"schema": "flashtex-nightly-documents/1", "documents": recs})
        n = kw.pop("nightly_new", nightly())
        return complete_board(d, nightly_new=n, nightly_old=None, t4_v1=t4_v1, **kw)

    def all_pass(self):
        return {i: 3 for i in self.IDS}

    def test_committed_baseline_loads(self):
        b = sb.load_t4_v1_baseline(sb.T4_V1_BASELINE)
        self.assertIsInstance(b, dict, b)
        self.assertEqual(b["status"], "FINAL")
        self.assertEqual(b["documents"]["measured"], 440)
        self.assertEqual(len(b["ids"]), 440)
        self.assertEqual(b["id_list_sha256"], sb.id_list_sha256(b["ids"]))
        self.assertEqual(b["oracle"]["tlpdb_sha256"][:12], "ca39e6791582")
        self.assertEqual(b["v1"]["L0"], [7, 440])
        self.assertEqual(b["new_same_slice"]["L1"], [440, 440])
        self.assertIn("5960583653", b["decision"]["url"])

    def test_final_compares_on_its_ids_and_goes_green(self):
        b = self.board(self.baseline(), docs=self.all_pass())
        for m in ("P-T2", "L0", "L1", "L2", "L3"):
            r = row(b, "nightly-5k", m)
            self.assertEqual(r["verdict"], "ahead (v1 one-off)", m)
            self.assertIn("v1 one-off (decision 1, 2026-10-01); new 4/4 on the baseline's 4 IDs",
                          sb.fmt_cell(r["old"]))
        self.assertEqual(row(b, "nightly-5k", "P-T1")["verdict"], "ahead (old n/a)")
        self.assertTrue(b["all_green"], red(b))
        self.assertEqual(b["t4_v1_baseline"]["status"], "FINAL")

    def test_final_uses_the_restricted_slice_not_the_whole_run(self):
        # the whole new run passes everything, but on the baseline's IDs new is at 0 on L0
        docs = {i: -1 for i in self.IDS}
        docs["9999.99999v1"] = 3  # outside the baseline's IDs: not counted
        b = self.board(self.baseline(), docs=docs)
        self.assertEqual(row(b, "nightly-5k", "L0")["verdict"], "behind")
        self.assertEqual(row(b, "nightly-5k", "L2")["verdict"], "equal (v1 one-off)")  # 0/4 vs 0/4
        self.assertFalse(b["all_green"])

    def test_final_without_documents_is_missing(self):
        b = self.board(self.baseline(), docs=None)
        r = row(b, "nightly-5k", "L1")
        self.assertEqual(r["verdict"], "missing")
        self.assertIn("documents.json is needed", sb.fmt_cell(r["old"]))
        self.assertFalse(b["all_green"])

    def test_final_with_ids_missing_from_the_run_is_partial(self):
        docs = self.all_pass()
        docs.pop(self.IDS[0])
        b = self.board(self.baseline(), docs=docs)
        r = row(b, "nightly-5k", "L1")
        self.assertEqual(r["verdict"], "ahead (v1 one-off)")  # 3/3 against 1/4
        self.assertIn("3 of the v1 one-off baseline's 4 IDs", r["old"]["partial"])
        self.assertFalse(b["all_green"])

    def test_provisional_compares_its_same_slice(self):
        # new_same_slice L1 0/4 against v1 1/4: behind, though today's whole run passes
        b = self.board(self.provisional(new_same_slice={"L1": [0, 4], "L0": [4, 4]}))
        self.assertEqual(row(b, "nightly-5k", "L1")["verdict"], "behind")
        r = row(b, "nightly-5k", "L0")
        self.assertEqual(r["verdict"], "ahead (v1 one-off)")
        self.assertIn("new 4/4 on the same slice in that measurement", sb.fmt_cell(r["old"]))
        # a metric it lacks is compared with the whole new run
        self.assertNotIn("against", row(b, "nightly-5k", "L2")["old"])
        self.assertEqual(row(b, "nightly-5k", "L2")["verdict"], "ahead (v1 one-off)")

    def test_provisional_row_green_board_not(self):
        b = self.board(self.provisional())
        r = row(b, "nightly-5k", "L1")
        self.assertEqual(r["verdict"], "ahead (v1 one-off)")
        self.assertIn("PROVISIONAL", sb.fmt_cell(r["old"]))
        self.assertFalse(b["all_green"])  # a provisional baseline is partial
        self.assertTrue(any("hash pending" in p for p in b["partial"]))

    def test_rates_not_counts(self):
        # whole-run comparison (PROVISIONAL, no new_same_slice): new 9/10 (90%) against v1 1/4
        n = nightly()
        n["tiers"]["nightly-5k"]["L0"] = [9, 10]
        self.assertEqual(row(self.board(self.provisional(), nightly_new=n), "nightly-5k", "L0")["verdict"],
                         "ahead (v1 one-off)")
        n = nightly()
        n["tiers"]["nightly-5k"]["L2"] = [0, 10]
        self.assertEqual(row(self.board(self.provisional(), nightly_new=n), "nightly-5k", "L2")["verdict"],
                         "equal (v1 one-off)")
        n = nightly()
        n["tiers"]["nightly-5k"]["L1"] = [2, 10]  # 20% < 25%
        b = self.board(self.provisional(), nightly_new=n)
        self.assertEqual(row(b, "nightly-5k", "L1")["verdict"], "behind")
        self.assertFalse(b["all_green"])

    def test_missing_baseline_is_not_green(self):
        b = self.board(None)
        r = row(b, "nightly-5k", "L1")
        self.assertEqual(r["verdict"], "missing")
        self.assertIn("no v1 one-off baseline", sb.fmt_cell(r["old"]))
        self.assertFalse(b["all_green"])
        self.assertIn("unreadable", sb.load_t4_v1_baseline(os.path.join(self.tmp, "nope.json")))

    def test_malformed_baseline_is_not_green(self):
        bad = [dict(schema="other"), dict(status="DRAFT"), dict(tier="arxiv"),
               dict(v1={"L1": [5, 4]}), dict(v1={"L1": [1, 0]}), dict(v1={"L1": [1, 5]}),
               dict(v1={"L9": [1, 2]}), dict(v1={}), dict(oracle={}),
               dict(decision={"date": "2026-10-02"}), dict(measured_date=None),
               # FINAL: the ID list, each once, as many as measured, and its hash
               dict(ids=None), dict(ids=self.IDS[:3]), dict(ids=self.IDS[:3] + self.IDS[:1]),
               dict(id_list_sha256="ab" * 32), dict(id_list_sha256=None),
               dict(new_same_slice={"L1": [5, 4]})]
        for over in bad:
            t = self.baseline(**over)
            self.assertIsInstance(t, str, over)
            self.assertIn("malformed", t)
            b = self.board(t, docs=self.all_pass())
            self.assertEqual(row(b, "nightly-5k", "L1")["verdict"], "missing", over)
            self.assertIn("malformed", sb.fmt_cell(row(b, "nightly-5k", "L1")["old"]))
            self.assertFalse(b["all_green"])
        with open(os.path.join(self.tmp, "garbage.json"), "w") as f:
            f.write("{not json")
        self.assertIn("unreadable", sb.load_t4_v1_baseline(os.path.join(self.tmp, "garbage.json")))

    def test_id_list_hash_definition(self):
        import hashlib
        self.assertEqual(sb.id_list_sha256(["b", "a"]), hashlib.sha256(b"a\nb\n").hexdigest())

    def test_a_v1_run_wins_over_the_baseline(self):
        b = complete_board(self.tmp, t4_v1=self.baseline())  # complete_board's default v1 nightly run
        self.assertEqual(row(b, "nightly-5k", "L1")["old"]["status"], "measured")

    def test_cli_reads_the_committed_baseline_by_default(self):
        nn = write_json(self.tmp, "t4.json", nightly())
        out = os.path.join(self.tmp, "board")
        with contextlib.redirect_stdout(io.StringIO()):
            sb.main(["--nightly", "new=" + nn, "--sha", "new=" + SHA, "--out", out])
        with open(os.path.join(out, "scoreboard.json")) as f:
            board = json.load(f)
        # the committed baseline is FINAL: without the new run's documents.json beside the
        # summary it cannot compare on its IDs, and says so (never green)
        old = row(board, "nightly-5k", "L0")["old"]
        self.assertEqual(old["status"], "missing")
        self.assertIn("FINAL v1 one-off baseline", old["note"])
        with contextlib.redirect_stdout(io.StringIO()):
            sb.main(["--nightly", "new=" + nn, "--sha", "new=" + SHA, "--out", out, "--t4-v1-baseline", "none"])
        with open(os.path.join(out, "scoreboard.json")) as f:
            self.assertEqual(row(json.load(f), "nightly-5k", "L0")["verdict"], "missing")


class OwnerBar(unittest.TestCase):
    """P5-BOARD-T4: the owner's bar (DESIGN §13, 2026-10-05): P-T2 >= 99% and P-T1 >= 98% on
    arXiv and T4, zero crashes on T4; a red tier opens or updates one p5-red issue; the gate
    fails the run unless the board is all green."""

    def setUp(self):
        self.tmp = tempfile.mkdtemp()

    def arxiv(self, pt1, pt2, n=1000):
        return board_for(tempfile.mkdtemp(), {"arxiv": summary(n, pt1, pt2, (n, n, n, n))},
                         {"arxiv": summary(n, pt1_na=CLI_NA, pt2=(0, n), levels=(1, 1, 0, 0))},
                         sizes={"arxiv": n})

    def test_pt_rates_against_the_bar(self):
        b = self.arxiv((980, 1000), (990, 1000))  # exactly at the bar
        self.assertEqual(row(b, "arxiv", "P-T1")["verdict"], "ahead (old n/a)")
        self.assertEqual(row(b, "arxiv", "P-T2")["verdict"], "ahead")
        self.assertEqual(row(b, "arxiv", "P-T2")["target"], "new >= old; >= 99% (owner bar)")
        self.assertEqual(row(b, "arxiv", "P-T1")["target"], ">= 98% (owner bar; old n/a)")
        b = self.arxiv((979, 1000), (989, 1000))  # one document under, though new > old
        self.assertEqual(row(b, "arxiv", "P-T1")["verdict"], "below bar (old n/a)")
        self.assertEqual(row(b, "arxiv", "P-T2")["verdict"], "below bar")
        self.assertEqual(b["red_tiers"], ["arxiv"])
        # the bar is on counts, not the rounded percent (98.95% shows as 99.0%)
        self.assertFalse(sb.bar_met("arxiv", "P-T2", sb.cell("measured", 1979, 2000)))
        # rows the bar does not cover are unchanged
        self.assertTrue(sb.bar_met("templates", "P-T2", sb.cell("measured", 1, 2)))

    def test_below_bar_opens_a_labelled_issue_and_edit_relabels(self):
        b = self.arxiv((979, 1000), (990, 1000))
        acts = sb.plan_issues(b, [])
        self.assertEqual([(a[0], a[2]) for a in acts], [("create", "arxiv")])
        self.assertIn("below bar", acts[0][3])
        calls = []
        sb.apply_issues(acts, "o/r", gh=lambda args, stdin=None: calls.append(args) or "https://x/1\n")
        self.assertEqual(calls[0][:3], ["label", "create", sb.ISSUE_LABEL])
        self.assertIn("--label", calls[1])
        self.assertEqual(calls[1][calls[1].index("--label") + 1], "p5-red")
        calls = []
        acts = sb.plan_issues(b, [{"number": 9, "title": "old title", "body": sb.MARKER % "arxiv"}])
        sb.apply_issues(acts, "o/r", gh=lambda args, stdin=None: calls.append(args) or "")
        self.assertEqual(calls[1][:3], ["issue", "edit", "9"])
        self.assertIn("--add-label", calls[1])
        # recovered: closed, and no label call is needed
        calls = []
        good = self.arxiv((1000, 1000), (1000, 1000))
        acts = sb.plan_issues(good, [{"number": 9, "title": "t", "body": sb.MARKER % "arxiv"}])
        sb.apply_issues(acts, "o/r", gh=lambda args, stdin=None: calls.append(args) or "")
        self.assertEqual([c[:3] for c in calls], [["issue", "close", "9"]])

    def test_t4_crash_row(self):
        t = nightly()["tiers"]["nightly-5k"]
        t.update(crashes=1, crash_examples=["2101.00001v1"])
        tiers, _ = sb.load_nightly(write_json(self.tmp, "s.json", nightly(tiers={"nightly-5k": t})), SIZES)
        c = tiers["nightly-5k"]["crashes"]
        self.assertEqual((c["passed"], c["of"]), (9, 10))
        self.assertIn("2101.00001v1", c["note"])
        na = sb.cell("n/a", note=sb.NO_V1_CRASHES)
        self.assertEqual(sb.verdict("nightly-5k", "crashes", c, na, True), "below bar (old n/a)")
        self.assertEqual(sb.verdict("nightly-5k", "crashes", sb.cell("measured", 10, 10), na, True),
                         "ahead (old n/a)")
        self.assertEqual(sb.target_of("nightly-5k", "crashes"), "0 crashes (owner bar)")
        # the v1 one-off baseline has no crash count: n/a, with the bar named
        base = {"tier": "nightly-5k", "status": "PROVISIONAL", "measured_date": "2026-10-01",
                "source": ["https://example.invalid/m"], "v1": {"L0": [1, 4]}}
        old = sb.t4_v1_cells(base, "nightly-5k", {"crashes": c, "L0": c})
        self.assertEqual(old["crashes"]["status"], "n/a")
        t["crashes"] = 11
        with self.assertRaises(sb.FormatError):
            sb.load_nightly(write_json(self.tmp, "bad.json", nightly(tiers={"nightly-5k": t})), SIZES)

    def test_crash_count_recovered_from_an_older_summary(self):
        d = tempfile.mkdtemp()
        t = nightly()["tiers"]["nightly-5k"]
        del t["crashes"]
        p = write_json(d, "summary.json", nightly(tiers={"nightly-5k": t}))
        c = sb.load_nightly(p, SIZES)[0]["nightly-5k"]["crashes"]
        self.assertEqual(c["status"], "not run")  # no documents.json: not counted, never green
        recs = [{"tier": "nightly-5k", "id": "a", "cause": "L0: exit 101"},
                {"tier": "nightly-5k", "id": "b", "cause": "L0: exit 1: ! Undefined control sequence."},
                {"tier": "nightly-5k", "id": "c", "first_difference": "P-T1: the candidate's traced pass did not "
                 "run: the traced pass crashed: its log stops before the end of the run (3 s)"},
                {"tier": "arxiv", "id": "d", "cause": "L0: exit -11"},
                {"tier": "nightly-5k", "id": "e", "cause": "L0: exit 101", "excluded": "oracle: pdflatex exit 1"},
                {"tier": "nightly-5k", "id": "f", "cause": "L0: exit None (timeout)"}]
        write_json(d, "documents.json", {"documents": recs})
        c = sb.load_nightly(p, SIZES)[0]["nightly-5k"]["crashes"]
        # every way of not finishing, by kind; an oracle-excluded document never ran
        self.assertEqual((c["passed"], c["of"]), (6, 10))
        self.assertIn("recovered from documents.json", c["partial"])
        for k in ("panic 1", "exit 1", "traced pass cut short 1", "timeout 1"):
            self.assertIn(k, c["note"])

    def test_crash_row_counts_every_kind_over_the_documents_run(self):
        t = nightly(n=10)["tiers"]["nightly-5k"]
        t.update(documents=14, measured=10, excluded={"oracle": 4}, crashes=3,
                 crash_kinds={"timeout": 1, "worker died": 1, "exit": 1}, crash_examples=["a", "b", "c"])
        c = sb.load_nightly(write_json(self.tmp, "k.json", nightly(tiers={"nightly-5k": t})), SIZES)[0]
        c = c["nightly-5k"]["crashes"]
        self.assertEqual((c["passed"], c["of"]), (7, 10))  # of: what the engine ran, not 14
        self.assertEqual(c["excluded"], {"oracle": 4})
        self.assertIn("exit 1, timeout 1, worker died 1", c["note"])

    def test_cross_oracle_v1_baseline_is_stated(self):
        base = {"tier": "nightly-5k", "status": "PROVISIONAL", "measured_date": "2026-10-01",
                "source": ["https://example.invalid/m"], "v1": {"L0": [1, 4]},
                "oracle": {"tlpdb_sha256": "ca39e6791582" + "0" * 52, "texlive_root": "/usr/local/texlive/2026"}}
        new = {"L0": sb.cell("measured", 4, 4)}
        pc = {"tlpdb_sha256": "909745461c89" + "0" * 52, "texlive_root": "/home/kubar/texlive/2026"}
        c = sb.t4_v1_cells(base, "nightly-5k", new, board_oracle=pc)["L0"]
        self.assertIn("CROSS-ORACLE", c["note"])
        self.assertIn("ca39e6791582", sb.fmt_cell(c))
        self.assertIn("909745461c89", sb.fmt_cell(c))
        same = sb.t4_v1_cells(base, "nightly-5k", new, board_oracle=base["oracle"])["L0"]
        self.assertNotIn("CROSS-ORACLE", same["note"])
        # the committed baseline names the Mac's oracle
        self.assertEqual(sb.load_t4_v1_baseline(sb.T4_V1_BASELINE)["oracle"]["tlpdb_sha256"][:12], "ca39e6791582")

    def test_unfetched_documents_make_the_tier_partial(self):
        b = board_for(self.tmp, {"templates": summary(8, (8, 8), (8, 8), (8, 8, 8, 8), excluded={"fetch": 2})},
                      {"templates": summary(8, pt1_na=CLI_NA, pt2=(0, 8), levels=(0, 0, 0, 0), excluded={"fetch": 2})})
        r = row(b, "templates", "L1")
        self.assertIn("2 document(s) unmeasured", r["new"]["partial"])
        self.assertFalse(b["all_green"])

    def test_nightly_counts_crashes(self):
        import nightly as nt
        kind = lambda r: (nt.crash_of(r) or (None,))[0]  # noqa: E731
        self.assertEqual(nt.crash_of({"candidate": {"exit": 101}}), ("panic", "exit 101 (panic)"))
        self.assertEqual(nt.crash_of({"candidate": {"exit": -11}}), ("signal", "killed by signal 11"))
        self.assertEqual(kind({"candidate": {"exit": None, "timed_out": True}}), "timeout")
        self.assertEqual(kind({"candidate": {"exit": 1, "stderr_tail": "exit 1: ! Emergency stop."}}), "exit")
        self.assertEqual(kind({"candidate": {"exit": 2}}), "exit")
        self.assertEqual(kind({"worker_died": True, "candidate": {"status": "died"}}), "worker died")
        self.assertEqual(kind({"candidate": {"exit": 0, "stderr_tail":
                                             "the traced pass did not finish in the capture's 600 s limit"}}), "timeout")
        self.assertEqual(kind({"candidate": {"exit": 0, "stderr_tail":
                                             "the traced pass crashed: its log stops before the end (2 s)"}}),
                         "traced pass cut short")
        self.assertIsNone(nt.crash_of({"candidate": {"exit": 0, "stderr_tail": ""}}))
        # never reached the engine: excluded by the oracle, or unmeasured
        self.assertIsNone(nt.crash_of({"excluded": "oracle: pdflatex exit 1", "candidate": {"exit": 101}}))
        recs = [{"id": "x", "crash": "exit 101 (panic)", "crash_kind": "panic"}, {"id": "y"},
                {"id": "z", "excluded": "fetch: gone", "unmeasured": True}]
        row_ = nt.tier_row(recs)
        self.assertEqual((row_["crashes"], row_["crash_kinds"], row_["crash_examples"], row_["measured"]),
                         (1, {"panic": 1}, ["x"], 2))

    def test_workflow_one_official_host_and_the_gate(self):
        with open(os.path.join(REPO, ".github", "workflows", "p5-scoreboard.yml")) as f:
            wf = f.read()
        jobs = jobs_of(wf)
        auto = jobs["pick"].split("auto)", 1)[1].split(";;", 1)[0]
        self.assertIn("route=pc", auto)
        self.assertNotIn("route=mac", auto)  # never a silent Mac stand-in
        pub = jobs["publish"]
        self.assertIn("--require-green", pub)
        self.assertLess(pub.index("--issues \"$ISSUES\""), pub.index("--require-green"))  # issues first
        self.assertIn("OFFICIAL: ${{ needs.pick.outputs.route == 'pc' }}", pub)
        self.assertIn("|| 'off' }}", pub.split("ISSUES:", 1)[1].split("\n", 1)[0])  # a Mac board files none
        self.assertIn("--slice=flashtex.slice", jobs["scoreboard"])
        self.assertIn("exit 1", jobs["scoreboard-unavailable"])

    def test_gate_from_board(self):
        b = self.arxiv((979, 1000), (990, 1000))
        path = write_json(self.tmp, "board.json", b)
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            rc = sb.main(["--from-board", path, "--issues", "off", "--require-green"])
        self.assertEqual(rc, 1)
        self.assertIn("::error::P5 scoreboard gate", out.getvalue())
        self.assertIn("arxiv", out.getvalue())
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(sb.main(["--from-board", path, "--issues", "off"]), 0)  # no gate asked
        green = complete_board(tempfile.mkdtemp())
        self.assertTrue(green["all_green"], red(green))
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(sb.main(["--from-board", write_json(self.tmp, "g.json", green), "--issues", "off",
                                      "--require-green"]), 0)


if __name__ == "__main__":
    unittest.main()
