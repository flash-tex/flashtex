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
         "L0": [n, n], "L1": [n, n], "L2": [n, n], "L3": [n, n], "L4": [0, n]}
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
    no = write_json(tmp, "n-old.json", kw.pop("nightly_old", nightly(kind="flashtex-cli", engine_sha256="E-old")))
    extra = {"new": [("latex-suites", {"latex-suites": suites}, {"host": "h1"}),
                     ("package-smoke", {"package-smoke": smoke}, {"host": "h1"}),
                     ("fonts", {"fonts": fonts}, {"host": "h1"}),
                     ("nightly",) + sb.load_nightly(nn, SIZES)],
             "old": [("nightly",) + sb.load_nightly(no, SIZES)]}
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
        self.assertEqual(sb.fmt_cell(r["old"]), "missing (no v1 leg in nightly: decision 1)")
        self.assertIn("missing (no v1 leg in nightly: decision 1)", sb.render_table(b))
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


if __name__ == "__main__":
    unittest.main()
