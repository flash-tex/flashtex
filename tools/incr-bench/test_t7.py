#!/usr/bin/env python3
"""Tests for t7.py's evaluation (unittest), on a trimmed summary of a real run
(testdata/t7-summary-trim.json: run 2 of docs/evidence/t7-latency-2026-10-02, plain-10 and full-10,
three phases, 13 keystrokes each). Run `python3 -m unittest discover -s tools/incr-bench -p 'test_*.py'`."""
import contextlib
import copy
import io
import json
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import t7  # noqa: E402

FIXTURE = json.load(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), 'testdata', 't7-summary-trim.json')))


def rows_of(summ=FIXTURE, baseline=None):
    rows = t7.summarise(summ['raw'])
    fails = t7.verdict(rows, baseline)
    return {f"{r['doc']} {r['edit']}": r for r in rows}, fails


class PctTests(unittest.TestCase):
    def test_nearest_rank(self):
        self.assertEqual(t7.pct([3, 1, 2], .5), 2)
        self.assertEqual(t7.pct(list(range(1, 21)), .95), 19)  # round(19 * .95) = 18 -> the 19th value
        self.assertEqual(t7.pct([5], .95), 5)
        self.assertIsNone(t7.pct([], .5))


class SummariseTests(unittest.TestCase):
    def test_warmup_dropped(self):
        rows, _ = rows_of()
        self.assertEqual(rows['plain-10 letter@middle']['n'], 12)  # 13 keystrokes, the first a warm-up
        self.assertEqual(rows['plain-10 preamble']['n'], 5)
        self.assertEqual(rows['plain-10 ' + t7.REOPEN_COLD]['n'], 5)  # 6 reopens

    def test_targets_and_gating(self):
        rows, _ = rows_of()
        self.assertEqual(rows['plain-10 letter@middle']['target'], 11.0)  # host share, decision 1
        self.assertEqual(rows['plain-10 preamble']['target'], 400.0)
        self.assertFalse(rows['plain-10 ' + t7.REOPEN_COLD]['gated'])  # decision 8: report-only
        self.assertFalse(rows['plain-10 ' + t7.REOPEN_WARM]['gated'])

    def test_missing_complete_counts_as_incomplete(self):
        summ = copy.deepcopy(FIXTURE)
        summ['raw'][0]['phases']['letter@middle']['keys'][3]['complete'] = None
        rows, fails = rows_of(summ)
        self.assertEqual(rows['plain-10 letter@middle']['incomplete'], 1)
        self.assertTrue(any(f.startswith('plain-10 letter@middle') and 'without all pages current' in f for f in fails))


class VerdictTests(unittest.TestCase):
    def test_fixture_verdict(self):
        rows, fails = rows_of()
        self.assertTrue(rows['plain-10 letter@middle']['pass'])  # p95 10.6 <= 11
        self.assertFalse(rows['full-10 letter@middle']['pass'])  # p95 22.3 > 11
        self.assertEqual(rows['full-10 preamble']['stat'], 'max')  # n = 5 < 12: gated on the maximum
        self.assertFalse(rows['full-10 preamble']['pass'])  # max 428 > 400
        self.assertFalse(any('reopen' in f for f in fails))  # report-only rows never fail the gate
        self.assertEqual(len(fails), 3)  # full-10: letter@middle, split@middle, preamble; nothing on plain-10

    def test_no_noise_margin(self):
        summ = copy.deepcopy(FIXTURE)
        ks = summ['raw'][0]['phases']['letter@middle']['keys']
        for k in ks:
            k['edited_page_ms'] = 11.0
        self.assertTrue(rows_of(summ)[0]['plain-10 letter@middle']['pass'])  # at the target: pass
        for k in ks:
            k['edited_page_ms'] = 11.5
        self.assertFalse(rows_of(summ)[0]['plain-10 letter@middle']['pass'])  # 4.5 % over: a miss

    def test_baseline_hold(self):
        rows, _ = rows_of()
        base = t7.baseline_of(rows.values())
        self.assertEqual(base['plain-10 letter@middle'], dict(converged_rate=1.0, typeset_pages_p50=6))
        self.assertNotIn('plain-10 ' + t7.REOPEN_COLD, base)
        self.assertEqual(rows_of(baseline=base)[1], rows_of()[1])  # its own baseline adds nothing
        worse = copy.deepcopy(base)
        worse['plain-10 split@middle']['converged_rate'] = 1.0
        worse['plain-10 letter@middle']['typeset_pages_p50'] = 1
        _, fails = rows_of(baseline=worse)
        self.assertTrue(any(f.startswith('plain-10 split@middle') and 'convergence 0.00 < baseline 1.00' in f for f in fails))
        self.assertTrue(any(f.startswith('plain-10 letter@middle') and 're-typeset pages p50 6 > baseline 1' in f
                            for f in fails))
        del worse['plain-10 split@middle']
        rows, _ = rows_of(baseline=worse)
        self.assertEqual(rows['plain-10 split@middle'].get('warning'), 'no baseline row')


class TypingTests(unittest.TestCase):
    """typing@ rows: dl3-keys --interval-ms lines (key, edited_page_ms, by) and DONE lines."""

    def phase(self, by, unpainted=0):
        def done(i, status='ok'):
            return dict(id=i, status=status, first_page_ms=9.0,
                        stages=dict(queue=1.0 + i % 3, first_page_cpu=7.0))
        dones = [done(i, 'ok' if i in by else 'cancelled') for i in range(10, 10 + len(by) + unpainted)]
        keys = [dict(key=k, edited_page_ms=10.0 + k, by=b) for k, b in enumerate(by)]
        return dict(keys=keys, dones=dones, summary=dict(unpainted=unpainted, interval_ms=100),
                    load=[1.0, 1.0])

    def test_every_keystroke_its_own_page(self):
        r = t7.typing_row(dict(doc='d', pages=5), 'typing@100ms', self.phase([10, 11, 12, 13]))
        self.assertEqual((r['n'], r['missing'], r['not_own']), (3, 0, 0))  # keystroke 0 is the warm-up
        self.assertEqual(r['ms']['max'], 13.0)
        self.assertEqual((r['off_cpu']['p50'], r['off_cpu']['max']), (0.0, 1.0))  # 9 - queue (3, 1, 2) - 7, never < 0
        self.assertEqual(t7.verdict([r], None), ['d typing@100ms: max 13.0 ms > 11 ms'])

    def test_keystrokes_painted_by_a_later_compile(self):
        # keystrokes 1 and 2 were painted by compile 13 (keystroke 3's), keystroke 4 never
        r = t7.typing_row(dict(doc='d', pages=5), 'typing@100ms', self.phase([10, 13, 13, 13], unpainted=1))
        self.assertEqual((r['n'], r['missing'], r['not_own']), (4, 1, 2))
        r['ms'] = t7.stats([1.0] * 3)
        why = '; '.join(t7.verdict([r], None))
        self.assertIn('1 of 4 without the watched page', why)
        self.assertIn('2 of 4 keystrokes not painted by their own compile', why)


class ReferenceTests(unittest.TestCase):
    def state(self, **kw):
        s = dict(source='AC', battery_pct=100, low_power_mode=False, thermal='', thermal_limited=False,
                 load=[1.0, 1.0, 1.0], ncpu=10)
        s.update(kw)
        return s

    def test_reference_conditions(self):
        rows = [dict(load=[1.0, 2.0])]
        ok = dict(power=dict(before=self.state(), after=self.state()))
        self.assertEqual(t7.reference_issues(ok, rows), [])
        self.assertEqual(t7.reference_issues(dict(power=dict(before=None)), rows), ['power state not recorded'])
        bad = dict(power=dict(before=self.state(source='battery', battery_pct=30, low_power_mode=True),
                              after=self.state(thermal_limited=True, thermal='CPU_Speed_Limit = 80')))
        issues = t7.reference_issues(bad, [dict(load=[1.0, 7.5])])
        self.assertIn('on battery at start (30 %)', issues)
        self.assertIn('Low Power Mode on at start', issues)
        self.assertIn('thermal limit at end: CPU_Speed_Limit = 80', issues)
        self.assertIn('load1 up to 7.5 (> 5.0)', issues)

    def test_cpu_quota_throttling(self):
        before = self.state(cpu_throttle={'user.slice/x.slice': [100, 10, 5_000_000], 'a/b': [5, 0, 0]})
        after = self.state(cpu_throttle={'user.slice/x.slice': [700, 410, 25_000_000], 'a/b': [9, 0, 0]})
        issues = t7.reference_issues(dict(power=dict(before=before, after=after)), [dict(load=[1.0, 1.0])])
        self.assertEqual(issues, ['CPU quota of x.slice throttled 400 of 600 periods (20.0 s stopped)'])
        quiet = self.state(cpu_throttle={'user.slice/x.slice': [100, 10, 5_000_000]})
        self.assertEqual(t7.reference_issues(dict(power=dict(before=quiet, after=quiet)), [dict(load=[1.0, 1.0])]), [])

    def test_exit_codes(self):
        with contextlib.redirect_stdout(io.StringIO()):
            self.exit_codes()

    def exit_codes(self):
        summ = copy.deepcopy(FIXTURE)
        self.assertEqual(t7.report(summ, None, None, False)[0], 1)  # misses
        summ['error'] = 'dl3-keys failed'
        self.assertEqual(t7.report(summ, None, None, False)[0], 2)  # --check honours the error
        plain = copy.deepcopy(FIXTURE)
        plain['raw'] = plain['raw'][:1]
        self.assertEqual(t7.report(plain, None, None, False)[0], 0)  # plain-10 passes
        self.assertEqual(t7.report(plain, None, None, True)[0], 3)  # but its power was not recorded


if __name__ == '__main__':
    unittest.main()
