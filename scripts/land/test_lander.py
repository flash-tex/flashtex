"""Decision-logic tests for scripts/land/lander.py, with fake GitHub responses.

Run: python3 -m unittest discover -s scripts/land -p 'test_lander.py' -v
"""
import datetime as dt
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lander  # noqa: E402

PIN = "a" * 40
NOW = dt.datetime(2026, 11, 10, 3, 0, tzinfo=dt.timezone.utc)
ME = "GoKubar"


def snap(**kw):
    s = {"id": "PR_x", "merged": False, "state": "OPEN", "head": PIN, "base": "main",
         "auto_merge": False, "in_queue": False,
         "checks": [{"name": "CI required", "status": "COMPLETED", "conclusion": "SUCCESS",
                     "workflow": "CI"}],
         "comments": []}
    s.update(kw)
    return s


def ci(conclusion, status="COMPLETED"):
    return [{"name": "CI required", "status": status, "conclusion": conclusion, "workflow": "CI"}]


class Decide(unittest.TestCase):
    def d(self, row, s, snaps=None):
        return lander.decide(row, s, snaps or {row.pr: s}, ME, NOW)

    def test_ready_enables(self):
        self.assertEqual(self.d(lander.Row(1, PIN), snap()).kind, lander.ENABLE)

    def test_no_data_is_retry_not_decision(self):
        self.assertEqual(self.d(lander.Row(1, PIN), None).kind, lander.WAIT)

    def test_moved_head_stops(self):
        d = self.d(lander.Row(1, PIN), snap(head="b" * 40))
        self.assertEqual(d.kind, lander.STOP)
        self.assertIn("moved", d.reason)

    def test_red_ci_required_stops(self):
        for c in ("FAILURE", "TIMED_OUT", "STARTUP_FAILURE"):
            self.assertEqual(self.d(lander.Row(1, PIN), snap(checks=ci(c))).kind, lander.STOP, c)

    def test_cancelled_or_pending_ci_waits(self):
        self.assertEqual(self.d(lander.Row(1, PIN), snap(checks=ci("CANCELLED"))).kind, lander.WAIT)
        self.assertEqual(self.d(lander.Row(1, PIN), snap(checks=ci(None, "IN_PROGRESS"))).kind,
                         lander.WAIT)
        self.assertEqual(self.d(lander.Row(1, PIN), snap(checks=[])).kind, lander.WAIT)

    def test_duplicate_ci_runs_need_all_green(self):
        s = snap(checks=ci("SUCCESS") + ci(None, "QUEUED"))
        self.assertEqual(self.d(lander.Row(1, PIN), s).kind, lander.WAIT)

    def test_stop_lander_comment_by_commander_stops(self):
        s = snap(comments=[{"login": ME, "body": "  STOP-LANDER: hold for #2041"}])
        self.assertEqual(self.d(lander.Row(1, PIN), s).kind, lander.STOP)

    def test_status_comments_do_not_stop(self):
        # Tonight's landers stopped on their own status comments (same account).
        bodies = ["parked; correction pending", "Lander: STOP-LANDER not posted",
                  "MERGE-AFTER-FIXES noted, CORRECTION applied"]
        s = snap(comments=[{"login": ME, "body": b} for b in bodies]
                 + [{"login": "someone-else", "body": "STOP-LANDER"}])
        self.assertEqual(self.d(lander.Row(1, PIN), s).kind, lander.ENABLE)

    def test_after_waits_for_predecessor_merge(self):
        row = lander.Row(2, PIN, after=1)
        me = snap()
        self.assertEqual(self.d(row, me, {1: snap(), 2: me}).kind, lander.WAIT)
        self.assertEqual(self.d(row, me, {1: None, 2: me}).kind, lander.WAIT)
        self.assertEqual(self.d(row, me, {1: snap(merged=True), 2: me}).kind, lander.ENABLE)

    def test_after_still_stops_on_moved_head_while_waiting(self):
        row = lander.Row(2, PIN, after=1)
        me = snap(head="c" * 40)
        self.assertEqual(self.d(row, me, {1: snap(), 2: me}).kind, lander.STOP)

    def test_feature_base_waits(self):
        self.assertEqual(self.d(lander.Row(1, PIN), snap(base="agent/x")).kind, lander.WAIT)

    def test_sweeps(self):
        row = lander.Row(1, PIN, sweeps=("sweeps",))
        self.assertEqual(self.d(row, snap()).kind, lander.WAIT)  # not run
        green = ci("SUCCESS") + [{"name": "verify", "status": "COMPLETED",
                                  "conclusion": "SUCCESS", "workflow": "sweeps"}]
        self.assertEqual(self.d(row, snap(checks=green)).kind, lander.ENABLE)
        red = ci("SUCCESS") + [{"name": "verify", "status": "COMPLETED",
                                "conclusion": "FAILURE", "workflow": "sweeps"}]
        d = self.d(row, snap(checks=red))
        self.assertEqual(d.kind, lander.WAIT)  # held, never enabled, not a stop
        self.assertIn("red", d.reason)
        cancelled = ci("SUCCESS") + [{"name": "verify", "status": "COMPLETED",
                                      "conclusion": "CANCELLED", "workflow": "sweeps"}]
        self.assertEqual(self.d(row, snap(checks=cancelled)).kind, lander.WAIT)

    def test_already_enabled_or_queued_or_merged_is_done(self):
        self.assertEqual(self.d(lander.Row(1, PIN), snap(auto_merge=True)).kind, lander.DONE)
        self.assertEqual(self.d(lander.Row(1, PIN), snap(in_queue=True)).kind, lander.DONE)
        self.assertEqual(self.d(lander.Row(1, PIN), snap(merged=True, head="d" * 40)).kind,
                         lander.DONE)

    def test_deadline_expires(self):
        row = lander.Row(1, PIN, deadline=NOW - dt.timedelta(minutes=1))
        self.assertEqual(self.d(row, snap(checks=ci(None, "QUEUED"))).kind, lander.EXPIRED)


class Parse(unittest.TestCase):
    def test_tsv(self):
        rows = lander.parse_tsv(
            "pr\tsha\tneeds\tdeadline\n# comment\n"
            f"10\t{PIN}\tnone\t-\n"
            f"11\t{PIN}\tafter:10+sweeps:sound,unicode\t2026-11-10T03:45:00Z\n")
        self.assertEqual([r.pr for r in rows], [10, 11])
        self.assertEqual(rows[1].after, 10)
        self.assertEqual(rows[1].sweeps, ("sound", "unicode"))
        self.assertEqual(rows[1].deadline, dt.datetime(2026, 11, 10, 3, 45, tzinfo=dt.timezone.utc))

    def test_graphql_snapshot(self):
        node = {"id": "PR_1", "state": "OPEN", "merged": False, "headRefOid": PIN,
                "baseRefName": "main", "isInMergeQueue": False, "mergeQueueEntry": None,
                "autoMergeRequest": None,
                "comments": {"nodes": [{"author": {"login": ME}, "body": "hi"}]},
                "commits": {"nodes": [{"commit": {"oid": PIN, "statusCheckRollup": {"contexts": {
                    "nodes": [{"__typename": "CheckRun", "name": "CI required",
                               "status": "COMPLETED", "conclusion": "SUCCESS",
                               "checkSuite": {"workflowRun": {"workflow": {"name": "CI"}}}}]}}}}]}}
        s = lander.snap_from_graphql(node)
        self.assertEqual(lander.decide(lander.Row(1, PIN), s, {1: s}, ME, NOW).kind, lander.ENABLE)
        self.assertIsNone(lander.snap_from_graphql(None))

    def test_query_covers_all_prs_in_one_request(self):
        q = lander.build_query([1, 2, 3])
        self.assertEqual(q.count("pullRequest(number:"), 3)
        self.assertIn("rateLimit", q)


if __name__ == "__main__":
    unittest.main()
