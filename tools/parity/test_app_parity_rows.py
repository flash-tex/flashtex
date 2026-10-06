#!/usr/bin/env python3
"""Tests for app_parity_rows.py: the row -> test check (retirement plan §4.4, S4(h))."""

import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import app_parity_rows as apr  # noqa: E402

CHECKLIST = """# App parity

| # | Feature | main | Evidence |
|---|---|---|---|
| A1 | Compile | done | x |
| A2 | Engines | **done** (#1) | x |
| A6 | Include | different (intended) | x |
| A9 | Fonts | **missing**; P5 scope | x |
| C25 | Debug | **partial** | x |
"""

SWIFT = """import XCTest
final class FooTests: XCTestCase {
    func testOne() throws {}
    func testTwo() {}
}
"""

SWIFT_EXT = """extension FooTests {
    func testThree() {}
}
@MainActor final class BarTests: XCTestCase { func testHost() async throws {} }
"""

RUST = "#[test]\nfn builds_it() {}\n"


def rows(**over):
    base = [
        {"id": "A1", "feature": "Compile", "status": "done", "s5": "test", "tests": ["FooTests.testOne"]},
        {"id": "A2", "feature": "Engines", "status": "done", "s5": "test",
         "tests": ["FooTests.testThree", "BarTests.testHost"]},
        {"id": "A6", "feature": "Include", "status": "different", "s5": "owner", "owner_retirement": None,
         "tests": []},
        {"id": "A9", "feature": "Fonts", "status": "missing", "s5": "fallback", "tests": ["FooTests.testTwo"]},
        {"id": "C25", "feature": "Debug", "status": "partial", "s5": "s6",
         "tests": ["rust:crates/x/tests/cli.rs::builds_it"]},
    ]
    for r in base:
        r.update(over.get(r["id"], {}))
    return {"schema": apr.SCHEMA, "checklist": "README.md", "host_driven": ["BarTests.testHost"], "rows": base}


class Tree:
    def __enter__(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = self.tmp.name
        for rel, text in (("README.md", CHECKLIST), ("apps/mac/Tests/T/FooTests.swift", SWIFT),
                          ("apps/mac/Tests/T/More.swift", SWIFT_EXT), ("crates/x/tests/cli.rs", RUST)):
            os.makedirs(os.path.dirname(os.path.join(root, rel)), exist_ok=True)
            with open(os.path.join(root, rel), "w") as f:
                f.write(text)
        return root

    def __exit__(self, *a):
        self.tmp.cleanup()


def log(*cases):
    f = tempfile.NamedTemporaryFile("w", suffix=".log", delete=False)
    for t, o in cases:
        s, n = t.split(".")
        f.write("Test Case '-[FlashTeXMacTests.%s %s]' %s (0.001 seconds).\n" % (s, n, o))
    f.close()
    return f.name


class CheckTests(unittest.TestCase):
    def test_a_well_formed_file_passes(self):
        with Tree() as root:
            self.assertEqual(apr.check(rows(), root), [])

    def test_a_renamed_test_is_reported(self):
        with Tree() as root:
            errs = apr.check(rows(A1={"tests": ["FooTests.testGone"]}), root)
            self.assertIn("test not found in the source: A1: FooTests.testGone", errs)

    def test_a_missing_rust_fn_is_reported(self):
        with Tree() as root:
            errs = apr.check(rows(C25={"tests": ["rust:crates/x/tests/cli.rs::nope"]}), root)
            self.assertTrue(any("nope" in e for e in errs), errs)

    def test_rows_must_be_the_checklists(self):
        with Tree() as root:
            doc = rows()
            doc["rows"] = doc["rows"][1:]
            self.assertTrue(any("missing ['A1']" in e for e in apr.check(doc, root)))

    def test_status_must_match_the_checklist(self):
        with Tree() as root:
            errs = apr.check(rows(A1={"status": "partial"}), root)
            self.assertTrue(any("A1: status 'partial'" in e for e in errs), errs)

    def test_an_owner_row_needs_the_field(self):
        with Tree() as root:
            doc = rows()
            del doc["rows"][2]["owner_retirement"]
            self.assertTrue(any("owner_retirement" in e for e in apr.check(doc, root)))

    def test_host_driven_must_be_a_named_test(self):
        with Tree() as root:
            doc = rows()
            doc["host_driven"].append("FooTests.testNobody")
            self.assertTrue(any("FooTests.testNobody" in e for e in apr.check(doc, root)))


class StageTests(unittest.TestCase):
    def test_s5_lists_pending_tests_and_unruled_owner_rows(self):
        state, why = apr.stage_state(rows(A1={"tests": []}), "S5")
        self.assertEqual(state, "not met")
        self.assertIn("no named test: A1", why)
        self.assertIn("owner retirement not in writing: A6", why)

    def test_s5_is_met_once_every_row_is_closed(self):
        state, why = apr.stage_state(rows(A6={"owner_retirement": "https://example.invalid/ruling"}), "S5")
        self.assertEqual((state, why), ("met", []))

    def test_s6_still_waits_for_the_fallback_and_old_route_rows(self):
        state, why = apr.stage_state(rows(A6={"owner_retirement": "x"}), "S6")
        self.assertEqual(state, "not met")
        self.assertIn("still on the previous engine's fallback: A9", why)
        self.assertIn("old-route rows open: C25", why)


class LogTests(unittest.TestCase):
    def test_each_leg_checks_its_own_tests(self):
        doc = rows()
        self.assertEqual(apr.leg_tests(doc, "hosted"), ["FooTests.testOne", "FooTests.testThree", "FooTests.testTwo"])
        self.assertEqual(apr.leg_tests(doc, "host"), ["BarTests.testHost"])

    def test_a_skipped_or_absent_test_is_not_passed(self):
        p = log(("FooTests.testOne", "passed"), ("FooTests.testThree", "skipped"))
        try:
            res = apr.read_log([p])
            self.assertEqual(apr.log_problems(["FooTests.testOne", "FooTests.testThree", "FooTests.testTwo"], res),
                             ["FooTests.testThree: skipped", "FooTests.testTwo: absent"])
        finally:
            os.unlink(p)

    def test_a_suite_entry_needs_every_test_passed(self):
        p = log(("FooTests.testOne", "passed"), ("FooTests.testTwo", "skipped"))
        try:
            self.assertEqual(apr.outcome("FooTests", apr.read_log([p])), "skipped")
            self.assertEqual(apr.outcome("BarTests", apr.read_log([p])), "absent")
        finally:
            os.unlink(p)

    def test_check_log_exit_codes(self):
        with Tree() as root:
            path = os.path.join(root, "rows.json")
            import json
            with open(path, "w") as f:
                json.dump(rows(), f)
            good = log(("BarTests.testHost", "passed"))
            bad = log(("BarTests.testHost", "skipped"))
            try:
                self.assertEqual(apr.main(["--rows", path, "check-log", "--leg", "host", good]), 0)
                self.assertEqual(apr.main(["--rows", path, "check-log", "--leg", "host", bad]), 1)
            finally:
                os.unlink(good)
                os.unlink(bad)


class RepoFileTests(unittest.TestCase):
    """The committed file against the committed checklist and test sources."""

    def test_the_committed_rows_file_checks_clean(self):
        self.assertEqual(apr.check(apr.load()), [])


if __name__ == "__main__":
    unittest.main()
