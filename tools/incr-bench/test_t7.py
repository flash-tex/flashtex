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


class EditLocationTests(unittest.TestCase):
    """Beamer decks give their edit lines (genbeamer.py's t7.json): dl3-keys gets --line/--page."""

    def test_located_replaces_at(self):
        lines = {0.02: (20, 2), 0.5: (21, 2), 0.98: (27, 3)}
        self.assertEqual(t7.located(['--kind', 'letter', '--at', '0.5', '--interval-ms', '50'], lines),
                         ['--kind', 'letter', '--line', '21', '--page', '2', '--interval-ms', '50'])
        self.assertEqual(t7.located(['--kind', 'sentence', '--at', '0.98'], lines)[2:], ['--line', '27', '--page', '3'])
        self.assertEqual(t7.located(['--kind', 'letter', '--at', '0.5'], None), ['--kind', 'letter', '--at', '0.5'])

    def test_generated_decks(self):
        import tempfile
        import genbeamer
        with tempfile.TemporaryDirectory() as d:
            sys_argv, sys.argv = sys.argv, ['genbeamer.py', d]
            try:
                genbeamer.main()
            finally:
                sys.argv = sys_argv
            for doc in ('beamer-5', 'beamer-30'):
                lines = t7.edit_lines(f'{d}/docs/{doc}')
                text = open(f'{d}/docs/{doc}/main.tex').read()
                src = text.split('\n')
                self.assertEqual(sorted(lines), [0.02, 0.5, 0.98])
                for line, _ in lines.values():
                    words = src[line - 1].split()
                    self.assertTrue(sum(1 for w in words if len(w) >= 4 and w.isalpha() and w.islower()) >= 2, src[line - 1])
                at = t7.body_line(text, 0.5, lines[0.5][0])
                self.assertTrue(text[at - 1].isalpha() and text[at].isalpha())  # inside the line's second word
            self.assertEqual(text.count('\\begin{frame}'), 30)


class HarnessErrorTests(unittest.TestCase):
    """A dl3-keys failure ends its document only (the M5 Pro run of 2026-10-14 lost infdesc's eleven
    measured phases to its preamble phase's failure): the phases that ran are kept and summarised."""

    class FakeHost:
        startup, ready_ms, killed_at_limit = None, 1.0, False

        def __init__(self, *args):
            pass

        def settle_saves(self, **kw):
            pass

        def stop(self):
            return 100 << 20

    def test_failure_keeps_earlier_phases(self):
        import tempfile
        import types
        key = dict(edited_page_ms=5.0, first_page_ms=5.0, done_ms=9.0, complete=True,
                   host=dict(converged_at=3, typeset_pages=2, mode='incremental'))
        calls = []

        def keys(eng, sock, work, args, out, timeout):
            calls.append(args)
            if len(calls) > 1:
                raise RuntimeError('dl3-keys --kind letter: exit 1: dl3-keys: no prose line on one page')
            return [dict(key=k, **key) for k in range(4)], 'dl3-keys: 10 pages'

        saved = (t7.Host, t7.keys, t7.IB)
        with tempfile.TemporaryDirectory() as d:
            os.makedirs(f'{d}/docs/doc-a')
            open(f'{d}/docs/doc-a/main.tex', 'w').write('x\n')
            os.makedirs(f'{d}/out/raw')
            t7.Host, t7.keys, t7.IB = self.FakeHost, keys, d
            try:
                a = types.SimpleNamespace(order='fixed', seed=7, phases=t7.PHASE_NAMES, preamble=2,
                                          typing_keys=4, keys=4, gap_ms=0, timeout=10, host_args='', reopen=3)
                res = t7.run_doc(a, f'{d}/eng', 'doc-a', f'{d}/out', 0)
            finally:
                t7.Host, t7.keys, t7.IB = saved
        self.assertIn('no prose line', res['error'])
        self.assertEqual(list(res['phases']), ['letter@start'])
        self.assertEqual(res['reopen'], [])
        self.assertEqual(res['rss_peak'], 100 << 20)
        rows = t7.summarise([res])
        self.assertEqual([(r['doc'], r['edit'], r['n']) for r in rows], [('doc-a', 'letter@start', 3)])
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(t7.report(dict(raw=[res], error=res['error']), None, None, False)[0], 2)


class RealDocTests(unittest.TestCase):
    """Real documents (realdocs.json): site phases, the 20 ms target, the oracle's page count."""

    def test_spec_sites(self):
        self.assertEqual(t7.REAL['target_ms'], 20.0)
        docs = t7.REAL['docs']
        self.assertIn('infdesc-x2', docs)
        self.assertGreaterEqual(len(docs), 4)
        for doc, s in docs.items():
            self.assertTrue(s['sites'], doc)
            self.assertIsInstance(s['pages'], int, doc)
        late = docs['infdesc-x2']['sites']['x2-late']
        self.assertTrue(late['edit'].startswith('x2/') and late['page'] >= 880)  # the owner's page 883

    def test_phases(self):
        ph = t7.real_phases('infdesc-x2', ['x2-late'])
        self.assertEqual([(n, k) for n, _, k in ph], [('letter@x2-late', 'letter@site'),
                                                      ('typing@50ms@x2-late', 'typing@50ms@site'),
                                                      ('typing@150ms@x2-late', 'typing@150ms@site')])
        args = ph[1][1]
        self.assertEqual(args[args.index('--interval-ms') + 1], '50')
        self.assertEqual(args[args.index('--edit') + 1], 'x2/book/real-numbers/inequalities.tex')
        self.assertTrue(t7.is_typing('typing@50ms@x2-late') and not t7.is_typing('letter@x2-late'))
        beamer = dict((n, a) for n, a, _ in t7.real_phases('beamer-metropolis'))['letter@middle']
        self.assertEqual(beamer[beamer.index('--line') + 1:beamer.index('--line') + 4], ['201', '--page', '20'])
        self.assertIn('--at', dict((n, a) for n, a, _ in t7.real_phases('arxiv-eth-primer'))['letter@end'])

    def real(self, ms, by=None, pages=1152):
        keys = [dict(key=k, edited_page_ms=ms, first_page_ms=ms, done_ms=ms, complete=True, by=(by or 10) + k,
                     host=dict(converged_at=1, typeset_pages=2)) for k in range(14)]
        return dict(doc='infdesc-x2', pages=pages, real=dict(target_ms=20.0, oracle_pages=1152), rss_peak=1,
                    phases={'letter@x2-late': dict(keys=keys, load=[1, 1]),
                            'typing@50ms@x2-late': dict(keys=keys, dones=[dict(id=10 + k) for k in range(14)],
                                                         summary={}, load=[1, 1])})

    def test_target_and_verdict(self):
        rows = t7.summarise([self.real(19.0)])
        self.assertEqual([r['target'] for r in rows], [20.0, 20.0])
        self.assertEqual(t7.verdict(rows, {}), [])  # no baseline needed: a latency gate
        rows = t7.summarise([self.real(32000.0)])  # the owner's 32 s keystroke
        self.assertEqual(len(t7.verdict(rows, None)), 2)

    def test_painted_by_a_later_compile_misses(self):
        d = self.real(5.0)
        for k in d['phases']['typing@50ms@x2-late']['keys']:
            k['by'] = 23  # every key shown only by the last compile
        r = [r for r in t7.summarise([d]) if r['edit'].startswith('typing')][0]
        self.assertIn('not painted by their own compile', '; '.join(t7.verdict([r], None)))

    def test_oracle_pages(self):
        rows = t7.summarise([self.real(5.0, pages=1140)])
        self.assertEqual(t7.verdict(rows, None), [])  # reported, not gated (a parity question)
        self.assertEqual({r['warning'] for r in rows}, {'1140 pages, the oracle 1152'})


class HostedGateTests(unittest.TestCase):
    """--gate hosted (the PR CI): no wall clock; re-typeset pages, instructions, own compiles."""

    def doc(self, ms, rerun, instr_m, by_own=True):
        g = dict(rerun_pages_max=3, edited_instr_m_max=200)
        host = dict(converged_at=1, typeset_pages=rerun, rerun_pages=rerun, stages=dict(edited_instr_k=instr_m * 1000))
        keys = [dict(key=k, edited_page_ms=ms, first_page_ms=ms, done_ms=ms, complete=True,
                     by=10 + k if by_own else 30, host=host) for k in range(14)]
        return dict(doc='infdesc-x2', pages=1152, rss_peak=1,
                    real=dict(target_ms=20.0, oracle_pages=1152, gate='hosted', pr_gate=g),
                    phases={'letter@x2-late': dict(keys=keys, load=[1, 1]),
                            'typing@150ms@x2-late': dict(keys=keys, dones=[dict(id=10 + k) for k in range(14)],
                                                          summary={}, load=[1, 1])})

    def test_wall_clock_not_gated(self):
        rows = t7.summarise([self.doc(ms=900.0, rerun=2, instr_m=60)])  # slow VM, little work: a pass
        self.assertEqual(t7.verdict(rows, None), [])

    def test_no_convergence_and_instructions_miss(self):
        why = '; '.join(t7.verdict(t7.summarise([self.doc(ms=5.0, rerun=254, instr_m=2900)]), None))
        self.assertIn('re-typeset up to 254 pages > 3', why)
        self.assertIn(f"up to 2900 M instructions to the edited page > {t7.REAL['pr_gate']['edited_instr_m_max']} M", why)

    def test_own_compile_still_gated(self):
        fails = t7.verdict(t7.summarise([self.doc(ms=5.0, rerun=2, instr_m=50, by_own=False)]), None)
        self.assertEqual(len(fails), 1)
        self.assertIn('not painted by their own compile', fails[0])


if __name__ == '__main__':
    unittest.main()
