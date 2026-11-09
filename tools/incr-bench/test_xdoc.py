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
    def run_xdoc(self, boxmemo, rounds=1, docs='xdoc:fonts-ab'):
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
                return {'stages': {'boxmemo': boxmemo}, 'pages': 1}

        with tempfile.TemporaryDirectory() as base:
            out = os.path.join(base, 'result.jsonl')
            args = ['xdoc.py', 'gates', '--docs', docs, '--rounds', str(rounds),
                    '--edits', '0', '--out', out]
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


if __name__ == '__main__':
    unittest.main()
