import copy
import importlib.util
import json
from pathlib import Path
import sys
import unittest

SPEC = importlib.util.spec_from_file_location('corpus_incremental', Path(__file__).with_name('incremental.py'))
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


class FakeWorker:
    mode = 'good'
    closed = 0
    def __init__(self, command, timeout):
        self.count = 0
    def request(self, request):
        self.count += 1
        reply = {'protocol_version': 1, 'type': 'compile_result', 'id': request['id'],
                 'payload': {'project_id': request['payload']['project_id'], 'revision': request['payload']['revision'],
                             'status': 'ok', 'pages': [], 'diagnostics': [], 'pdf_path': None}}
        if self.mode == 'stale' and self.count > 1:
            reply['payload']['revision'] -= 1
        if self.mode == 'different' and self.count > 1:
            reply['payload']['status'] = 'failed'
        return json.dumps(reply), reply
    def close(self):
        type(self).closed += 1
        return ''


class IncrementalTests(unittest.TestCase):
    def setUp(self):
        FakeWorker.mode = 'good'; FakeWorker.closed = 0
        self.scenario, self.case, self.requests = next(module.scenarios())

    def run_case(self):
        return module.run_scenario(self.scenario, self.case, self.requests, ['fake'], factory=FakeWorker)

    def test_equivalent_fake_processes_are_not_semantic_evidence(self):
        result = self.run_case()
        self.assertEqual(result['status'], 'pass')
        self.assertEqual([s['phase'] for s in result['steps']], ['cold', 'warm_unchanged', 'edit'])
        self.assertEqual(FakeWorker.closed, 4)
        self.assertTrue(all('persistent_raw' in s and 'clean_raw' in s for s in result['steps']))

    def test_stale_response_rejected(self):
        FakeWorker.mode = 'stale'
        result = self.run_case()
        self.assertEqual(result['status'], 'fail')
        self.assertIn('stale/mismatched', result['error'])
        self.assertNotIn('equivalent', result['steps'][-1])

    def test_persistent_divergence_fails(self):
        FakeWorker.mode = 'different'
        result = self.run_case()
        self.assertEqual(result['status'], 'fail')
        self.assertFalse(result['steps'][1]['equivalent'])

    def test_repair_scenario_preserves_project_identity(self):
        scenarios = list(module.scenarios())
        self.assertEqual(len(scenarios), 5)
        _, _, requests = scenarios[-1]
        self.assertEqual(len(requests), 4)
        self.assertEqual(requests[0][1]['payload']['documents'], requests[-1][1]['payload']['documents'])
        self.assertEqual([r['payload']['revision'] for _, r in requests], [1, 2, 3, 4])

    def test_real_pipe_worker_and_timeout(self):
        echo = [sys.executable, '-u', '-c', 'import sys; [print(line.strip(), flush=True) for line in sys.stdin]']
        worker = module.Worker(echo, 1)
        try:
            _, reply = worker.request({'id': 'one'})
            self.assertEqual(reply, {'id': 'one'})
            _, reply = worker.request({'id': 'two'})
            self.assertEqual(reply, {'id': 'two'})
        finally:
            worker.close()
        worker = module.Worker([sys.executable, '-c', 'import time; time.sleep(5)'], 0.02)
        try:
            with self.assertRaises(TimeoutError):
                worker.request({'id': 'timeout'})
        finally:
            worker.close()


if __name__ == '__main__':
    unittest.main()
