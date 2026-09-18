"""Unit tests for merge_digests.py: the round-trip and single-case proofs
described in its module docstring, plus the refusal paths.

    python3 -m unittest discover -s crates/perf-bench/scripts -p 'test_*.py' -v
"""

import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import merge_digests as md  # noqa: E402


def _make_report(cases, meta=None, targets=None, unmeasured=None):
    """A minimal report with the same shape as `flashtex-perf-bench --json`,
    serialized the same way the real harness does: one line, sorted keys, no
    extra whitespace (`json.dumps(..., sort_keys=True, separators=(",", ":"))`
    matches the harness's alphabetical-key, minified style closely enough to
    exercise the same code paths as the real files)."""
    doc = {
        "schema_version": 1,
        "meta": meta if meta is not None else {"host_fingerprint": "ref-host", "cpu": "ref-cpu"},
        "cases": cases,
        "targets": targets if targets is not None else [{"id": "t1", "met": True}],
        "unmeasured": unmeasured if unmeasured is not None else [],
    }
    return json.dumps(doc, sort_keys=True, separators=(",", ":"))


def _case(cid, digests, metrics=None):
    return {
        "id": cid,
        "group": "real-world",
        "bytes": 100,
        "digests": digests,
        "metrics": metrics if metrics is not None else {"cold.render_ms": {"median": 1.0}},
        "notes": [],
    }


class MergeDigestsTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)

    def _path(self, name, text):
        p = os.path.join(self.tmp.name, name)
        with open(p, "w", encoding="utf-8") as f:
            f.write(text)
        return p

    def test_empty_case_list_is_byte_identical_round_trip(self):
        baseline_text = _make_report(
            [
                _case("a", {"cold.reply": "aaaa"}),
                _case("b", {"cold.reply": "bbbb"}),
            ]
        )
        baseline = self._path("baseline.json", baseline_text)
        new_report = self._path("new.json", baseline_text)  # content doesn't matter with 0 cases
        out = os.path.join(self.tmp.name, "out.json")

        rc = md.main(["--baseline", baseline, "--new-report", new_report, "--out", out])
        self.assertEqual(rc, 0)
        with open(out, encoding="utf-8") as f:
            out_text = f.read()
        self.assertEqual(out_text, baseline_text)

    def test_one_case_changes_only_that_cases_digests(self):
        baseline_text = _make_report(
            [
                _case("a", {"cold.reply": "aaaa", "export.pdf": "pdf-a"}),
                _case("b", {"cold.reply": "bbbb", "export.pdf": "pdf-b"}),
            ],
            meta={"host_fingerprint": "ref-host", "cpu": "ref-cpu"},
        )
        new_text = _make_report(
            [
                _case("a", {"cold.reply": "AAAA-NEW", "export.pdf": "pdf-a"}),  # 'a' changed
                _case("b", {"cold.reply": "bbbb", "export.pdf": "pdf-b"}),  # 'b' unchanged
            ],
            meta={"host_fingerprint": "some-other-host", "cpu": "different-cpu"},  # must NOT leak in
        )
        baseline = self._path("baseline.json", baseline_text)
        new_report = self._path("new.json", new_text)
        out = os.path.join(self.tmp.name, "out.json")

        rc = md.main(["--baseline", baseline, "--new-report", new_report, "--case", "a", "--out", out])
        self.assertEqual(rc, 0)

        before = json.loads(baseline_text)
        with open(out, encoding="utf-8") as f:
            after = json.loads(f.read())

        # meta/schema_version/targets/unmeasured untouched.
        for key in ("meta", "schema_version", "targets", "unmeasured"):
            self.assertEqual(before[key], after[key], key)

        by_id_before = {c["id"]: c for c in before["cases"]}
        by_id_after = {c["id"]: c for c in after["cases"]}

        # Case 'a': only digests changed, to exactly the new report's values.
        self.assertEqual(by_id_after["a"]["digests"], {"cold.reply": "AAAA-NEW", "export.pdf": "pdf-a"})
        for field in by_id_before["a"]:
            if field == "digests":
                continue
            self.assertEqual(by_id_before["a"][field], by_id_after["a"][field], field)

        # Case 'b' (not named): completely untouched, including its digests.
        self.assertEqual(by_id_before["b"], by_id_after["b"])

    def test_refuses_on_mismatched_case_id_sets(self):
        baseline = self._path("baseline.json", _make_report([_case("a", {"cold.reply": "x"}), _case("b", {"cold.reply": "y"})]))
        new_report = self._path("new.json", _make_report([_case("a", {"cold.reply": "x"})]))  # missing 'b'
        out = os.path.join(self.tmp.name, "out.json")
        rc = md.main(["--baseline", baseline, "--new-report", new_report, "--out", out])
        self.assertNotEqual(rc, 0)
        self.assertFalse(os.path.exists(out))

    def test_refuses_on_case_missing_from_files(self):
        text = _make_report([_case("a", {"cold.reply": "x"})])
        baseline = self._path("baseline.json", text)
        new_report = self._path("new.json", text)
        out = os.path.join(self.tmp.name, "out.json")
        rc = md.main(["--baseline", baseline, "--new-report", new_report, "--case", "nope", "--out", out])
        self.assertNotEqual(rc, 0)
        self.assertFalse(os.path.exists(out))

    def test_refuses_on_duplicate_case_flag(self):
        text = _make_report([_case("a", {"cold.reply": "x"})])
        baseline = self._path("baseline.json", text)
        new_report = self._path("new.json", text)
        out = os.path.join(self.tmp.name, "out.json")
        rc = md.main(["--baseline", baseline, "--new-report", new_report, "--case", "a", "--case", "a", "--out", out])
        self.assertNotEqual(rc, 0)
        self.assertFalse(os.path.exists(out))

    def test_no_op_case_with_identical_digests_is_allowed(self):
        text = _make_report([_case("a", {"cold.reply": "x"}), _case("b", {"cold.reply": "y"})])
        baseline = self._path("baseline.json", text)
        new_report = self._path("new.json", text)  # identical content
        out = os.path.join(self.tmp.name, "out.json")
        rc = md.main(["--baseline", baseline, "--new-report", new_report, "--case", "a", "--out", out])
        self.assertEqual(rc, 0)
        with open(out, encoding="utf-8") as f:
            self.assertEqual(f.read(), text)


if __name__ == "__main__":
    unittest.main()
