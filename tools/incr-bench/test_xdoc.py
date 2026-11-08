#!/usr/bin/env python3
"""Regression tests for the sound-xdoc gate (unittest)."""
import contextlib
import io
import json
import os
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import sweeps  # noqa: E402
import xdoc  # noqa: E402


class XdocGateTests(unittest.TestCase):
    def run_xdoc(self, boxmemo, rounds=1, docs='xdoc:fonts-ab', extra=()):
        """BOXMEMO: the host's `stages.boxmemo` counts, or a list of them, one per compile (the
        counts are the host's running totals)."""
        seq = boxmemo if boxmemo and isinstance(boxmemo[0], list) else None
        calls = []
        class Host:
            def __init__(self, *args, **kwargs):
                kwargs['stdout'].write('listening\n')
                kwargs['stdout'].flush()

            def poll(self):
                return None

            def wait(self, timeout=None):
                return 0

        class Conn:
            def __init__(self, path):
                pass

            def send(self, kind, obj):
                pass

            def until_done(self, cid):
                calls.append(cid)
                bm = seq[min(len(calls), len(seq)) - 1] if seq else boxmemo
                return {'stages': {'boxmemo': bm}, 'pages': 1}

        with tempfile.TemporaryDirectory() as base:
            out = os.path.join(base, 'result.jsonl')
            args = ['xdoc.py', 'gates', '--docs', docs, '--rounds', str(rounds),
                    '--edits', '0', '--out', out, *extra]
            stdout = io.StringIO()
            with mock.patch.object(xdoc, 'BASE', base), mock.patch.object(sys, 'argv', args), \
                    mock.patch.object(xdoc, 'Conn', Conn), mock.patch.object(xdoc.subprocess, 'Popen', Host), \
                    mock.patch.object(xdoc.subprocess, 'run'), mock.patch.object(xdoc.signal, 'alarm'), \
                    mock.patch.object(xdoc.signal, 'signal'), \
                    mock.patch.dict(os.environ, {'FLASHTEX_BOXMEMO': 'off' if boxmemo is None else 'on'}), \
                    contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(SystemExit) as exit_:
                    xdoc.main()
            return exit_.exception.code, json.loads(stdout.getvalue().splitlines()[-1])

    def test_zero_cross_document_hits_fail(self):
        code, totals = self.run_xdoc([0, 0, 0, 0, 0])
        self.assertEqual(totals['xdoc_hits'], 0)
        self.assertNotEqual(code, 0)

    def test_four_count_host_fails(self):
        code, totals = self.run_xdoc([0, 1, 0, 0])
        self.assertIsNone(totals['xdoc_hits'])
        self.assertNotEqual(code, 0)

    def test_zero_compiles_fail(self):
        for kwargs in ({'rounds': 0}, {'docs': ''}):
            with self.subTest(**kwargs):
                code, totals = self.run_xdoc([0, 1, 0, 0, 1], **kwargs)
                self.assertEqual(totals['compiles'], 0)
                self.assertNotEqual(code, 0)

    def test_cross_document_hit_passes(self):
        code, totals = self.run_xdoc([0, 1, 0, 0, 1])
        self.assertEqual((code, totals['xdoc_hits']), (0, 1))

    def test_off_is_labelled_not_exercised(self):
        code, totals = self.run_xdoc(None)
        self.assertEqual(code, 0)
        self.assertEqual(totals['gate'], 'not exercised (BOX-MEMO off)')

    def test_sweeps_labels_off_as_not_applicable(self):
        unit = {'key': 'sound-xdoc|xdoc|all', 'tool': 'xdoc', 'docs': ['xdoc:fonts-ab'],
                'rounds': 1, 'edits': 0, 'env': {}}
        totals = dict(compiles=1, ok=1, bad=0, err=0, skipped=0, hits=0, xdoc_hits=None,
                      gate='not exercised (BOX-MEMO off)')
        proc = mock.Mock(returncode=0, stdout=json.dumps(totals), stderr='')
        with tempfile.TemporaryDirectory() as out, mock.patch.object(sweeps.subprocess, 'run', return_value=proc), \
                mock.patch.dict(os.environ, {'FLASHTEX_SWEEP_BOXMEMO': 'off'}):
            result = sweeps._run_unit(out, out, out, unit, 10)
        self.assertEqual(result['status'], 'absent')
        self.assertEqual(result['note'], totals['gate'])

    def test_sweeps_rejects_old_driver_without_cross_document_hits(self):
        unit = {'key': 'sound-xdoc|xdoc|all', 'tool': 'xdoc', 'docs': ['xdoc:fonts-ab'],
                'rounds': 1, 'edits': 0, 'env': {}}
        for hits in (None, 0):
            with self.subTest(xdoc_hits=hits), tempfile.TemporaryDirectory() as out:
                totals = dict(compiles=1, ok=1, bad=0, err=0, skipped=0, hits=0, xdoc_hits=hits)
                proc = mock.Mock(returncode=0, stdout=json.dumps(totals), stderr='')
                with mock.patch.object(sweeps.subprocess, 'run', return_value=proc), \
                        mock.patch.dict(os.environ, {'FLASHTEX_SWEEP_BOXMEMO': 'on'}):
                    result = sweeps._run_unit(out, out, out, unit, 10)
                self.assertEqual(result['status'], 'error')

    # #2112's review: the gate counted cross-document replays for the whole run only, so one
    # document's replays stood for every other's
    TWO = 'xdoc:fonts-ab,xdoc:fonts-ba'
    # fonts-ab's compile replays nothing of another document's; fonts-ba's one entry of fonts-ab's
    ONE_SIDED = [[0, 1, 0, 0, 0], [0, 2, 0, 0, 1]]

    def test_cross_document_hits_are_counted_per_document(self):
        code, totals = self.run_xdoc(self.ONE_SIDED, docs=self.TWO)
        self.assertEqual(totals['xdoc_by_doc'], {'xdoc:fonts-ab': 0, 'xdoc:fonts-ba': 1})
        self.assertEqual((code, totals['xdoc_hits']), (0, 1))

    def test_a_named_document_without_its_own_cross_document_hit_fails(self):
        for need in ('xdoc:fonts-ab', self.TWO, 'plain-10'):
            with self.subTest(need=need):
                code, totals = self.run_xdoc(self.ONE_SIDED, docs=self.TWO, extra=('--need-xdoc', need))
                self.assertEqual(totals['xdoc_hits'], 1)
                self.assertEqual(totals['need_xdoc'], need.split(','))
                self.assertEqual(code, 1)

    def test_a_named_document_with_its_own_cross_document_hit_passes(self):
        code, totals = self.run_xdoc(self.ONE_SIDED, docs=self.TWO, extra=('--need-xdoc', 'xdoc:fonts-ba'))
        self.assertEqual((code, totals['need_xdoc']), (0, ['xdoc:fonts-ba']))

    def test_a_four_count_host_measures_no_document(self):
        code, totals = self.run_xdoc([0, 1, 0, 0], docs=self.TWO, extra=('--need-xdoc', 'xdoc:fonts-ba'))
        self.assertEqual(totals['xdoc_by_doc'], {'xdoc:fonts-ab': None, 'xdoc:fonts-ba': None})
        self.assertNotEqual(code, 0)

    def test_sweeps_names_the_documents_and_rejects_one_without_its_own_hit(self):
        unit = {'key': 'sound-xdoc|xdoc|all', 'tool': 'xdoc', 'docs': self.TWO.split(','),
                'rounds': 1, 'edits': 0, 'env': {}, 'need': ['xdoc:fonts-ab']}
        # (a driver that ignored the requirement: the total alone would pass)
        totals = dict(compiles=2, ok=2, bad=0, err=0, skipped=0, hits=2, xdoc_hits=1,
                      xdoc_by_doc={'xdoc:fonts-ab': 0, 'xdoc:fonts-ba': 1})
        proc = mock.Mock(returncode=0, stdout=json.dumps(totals), stderr='')
        with tempfile.TemporaryDirectory() as out, \
                mock.patch.object(sweeps.subprocess, 'run', return_value=proc) as run, \
                mock.patch.dict(os.environ, {'FLASHTEX_SWEEP_BOXMEMO': 'on'}):
            result = sweeps._run_unit(out, out, out, unit, 10)
        cmd = run.call_args[0][0]
        self.assertEqual(cmd[cmd.index('--need-xdoc') + 1], 'xdoc:fonts-ab')
        self.assertEqual(result['status'], 'error')
        self.assertEqual(result['xdoc_by_doc'], totals['xdoc_by_doc'])

    def test_the_sweep_names_documents_of_its_own(self):
        (unit,) = sweeps.GATES['sound-xdoc']
        self.assertTrue(unit['need'])
        self.assertTrue(set(unit['need']) <= set(unit['docs']), unit['need'])


if __name__ == '__main__':
    unittest.main()
