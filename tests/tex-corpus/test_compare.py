import copy
from datetime import datetime, timedelta, timezone
import importlib.util
import json
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location('corpus_compare', Path(__file__).with_name('compare.py'))
compare = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(compare)


class BaselineTests(unittest.TestCase):
    def setUp(self):
        self.artifact = json.loads((compare.ROOT / 'evidence/compiler-9f1033b-included-source.json').read_text())
        self.build = self.artifact['build_command']

    def verify(self):
        return compare.verify(self.artifact, compare.BASELINE_SHA, self.build)

    def test_baseline_replayed_not_trusted(self):
        records = self.verify()
        result = compare.compare(records, records)
        self.assertEqual(result['gate'], 'pass')
        self.assertEqual(len(result['groups']['still_failing']), 5)
        self.assertEqual(len(result['groups']['still_unsupported']), 9)

    def test_forged_pass_rejected(self):
        self.artifact['results'][0]['status'] = 'pass'
        with self.assertRaisesRegex(ValueError, 'checks/status'):
            self.verify()

    def test_provenance_rejected(self):
        with self.assertRaisesRegex(ValueError, 'revision mismatch'):
            compare.verify(self.artifact, '0' * 40, self.build)
        with self.assertRaisesRegex(ValueError, 'build provenance'):
            compare.verify(self.artifact, compare.BASELINE_SHA, 'different build')

    def test_stale_candidate_rejected(self):
        now = datetime.fromisoformat(self.artifact['created_utc']) + timedelta(hours=2)
        with self.assertRaisesRegex(ValueError, 'stale'):
            compare.verify(self.artifact, compare.BASELINE_SHA, self.build, 3600, now)

    def test_changed_request_rejected(self):
        self.artifact['results'][0]['request']['payload']['documents'][0]['text'] += 'Changed'
        with self.assertRaisesRegex(ValueError, 'source request'):
            self.verify()

    def test_missing_case_rejected(self):
        self.artifact['results'].pop()
        with self.assertRaisesRegex(ValueError, 'missing corpus'):
            self.verify()

    def test_improvements_cannot_cancel_regressions(self):
        old = {'a': {'status': 'fail', 'checks': []}, 'b': {'status': 'pass', 'checks': []}}
        new = {'a': {'status': 'pass', 'checks': []}, 'b': {'status': 'fail', 'checks': []}}
        result = compare.compare(old, new)
        self.assertEqual(result['groups']['newly_passing'], ['a'])
        self.assertEqual(result['groups']['regressed'], ['b'])
        self.assertEqual(result['gate'], 'fail')

    def test_still_failing_case_cannot_hide_lost_subcheck(self):
        old = {'a': {'status': 'fail', 'checks': [{'check': 'source', 'status': 'pass'}]}}
        new = {'a': {'status': 'fail', 'checks': [{'check': 'source', 'status': 'fail'}]}}
        result = compare.compare(old, new)
        self.assertEqual(result['groups']['still_failing'], ['a'])
        self.assertEqual(result['gate'], 'fail')
        self.assertEqual(len(result['check_regressions']), 1)


if __name__ == '__main__':
    unittest.main()
