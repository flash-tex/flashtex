import importlib.util
import json
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('corpus_runner', Path(__file__).with_name('run.py'))
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class RunnerTests(unittest.TestCase):
    def setUp(self):
        self.case = next(c for c in runner.corpus.validate()[0]['cases'] if c['id'] == 'literal-source-map')
        self.request = runner.corpus.compile_request(self.case, 1)
        self.reply = {'protocol_version': 1, 'id': self.request['id'], 'type': 'compile_result',
                      'payload': {'project_id': self.request['payload']['project_id'], 'revision': 1,
                                  'status': 'ok', 'pages': [], 'diagnostics': []}}

    def execute(self, reply=None, returncode=0):
        process = subprocess.CompletedProcess(['fake'], returncode, json.dumps(reply or self.reply) + '\n', '')
        with patch.object(runner.subprocess, 'run', return_value=process) as call:
            result = runner.run_case(self.case, ['fake', '--json'], timeout=2)
        self.assertEqual(call.call_args.kwargs['timeout'], 2)
        self.assertFalse(call.call_args.kwargs['check'])
        self.assertEqual(json.loads(call.call_args.kwargs['input']), self.request)
        return result

    def test_empty_output_cannot_pass_text_requirements(self):
        self.assertEqual(self.execute()['status'], 'fail')

    def test_wrong_revision_fails(self):
        self.reply['payload']['revision'] = 2
        result = self.execute()
        self.assertIn('project_revision', [c['check'] for c in result['checks'] if c['status'] == 'fail'])

    def test_invalid_utf8_boundary_fails(self):
        data = self.request['payload']['documents'][0]['text'].encode()
        start = data.index('é'.encode())
        self.reply['payload']['pages'] = [{'width_pt': 612, 'height_pt': 792, 'items': [
            {'kind': 'text', 'text': 'é', 'x_pt': 0, 'baseline_y_pt': 20, 'font_size_pt': 12,
             'source': {'path': 'main.tex', 'start_byte': start + 1, 'end_byte': start + 2}}]}]
        result = self.execute()
        self.assertIn('display_items_and_sources', [c['check'] for c in result['checks'] if c['status'] == 'fail'])

    def test_crash_and_timeout_have_failed_evidence(self):
        self.assertEqual(self.execute(returncode=3)['status'], 'fail')
        with patch.object(runner.subprocess, 'run', side_effect=subprocess.TimeoutExpired(['fake'], 2)):
            self.assertEqual(runner.run_case(self.case, ['fake'])['status'], 'fail')

    def test_fake_valid_literals_verify_only_narrow_source_gates(self):
        text = self.request['payload']['documents'][0]['text']
        items = []
        for word in ['éé', 'UniqueAnchor', '尾']:
            start = text.encode().index(word.encode())
            items.append({'kind': 'text', 'text': word, 'x_pt': 10, 'baseline_y_pt': 20, 'font_size_pt': 12,
                          'source': {'path': 'main.tex', 'start_byte': start, 'end_byte': start + len(word.encode())}})
        self.reply['payload']['pages'] = [{'width_pt': 612, 'height_pt': 792, 'items': items}]
        self.assertEqual(self.execute()['status'], 'pass')

    def test_manual_math_never_passes_from_envelope_alone(self):
        case = next(c for c in runner.corpus.validate()[0]['cases'] if c['id'] == 'math-inline-display')
        request = runner.corpus.compile_request(case, 1)
        self.reply['id'] = request['id']; self.reply['payload']['project_id'] = request['payload']['project_id']
        checks = runner.evaluate(case, request, self.reply)
        self.assertTrue(any(c['status'] == 'unverified' for c in checks))
        self.reply['payload']['diagnostics'] = [{'severity': 'error', 'message': 'math not supported', 'source': None, 'recovery': 'plain text'}]
        checks = runner.evaluate(case, request, self.reply)
        self.assertTrue(any(c['status'] == 'unsupported' for c in checks))


if __name__ == '__main__':
    unittest.main()
