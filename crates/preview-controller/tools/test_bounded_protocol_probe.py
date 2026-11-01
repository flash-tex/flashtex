import os
from pathlib import Path
import tempfile
import unittest

from bounded_protocol_probe import run


class BoundedProbeTests(unittest.TestCase):
    def exercise(self, body, request=b'{}\n', expected=b'{}\n', error=None, limit=1024):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            worker = root / 'worker'
            worker.write_text('#!/usr/bin/env python3\nimport os, sys, time\n'
                              'from pathlib import Path\n'
                              'Path(__file__).with_suffix(".pid").write_text(str(os.getpid()))\n' + body)
            worker.chmod(0o700)
            (root / 'request').write_bytes(request)
            (root / 'expected').write_bytes(expected)
            args = (worker, root / 'request', root / 'expected', root / 'output')
            if error:
                with self.assertRaises(error):
                    run(*args, timeout=0.3, max_reply=limit)
                self.assertFalse((root / 'output/result.json').exists())
            else:
                result = run(*args, timeout=2, max_reply=limit)
                self.assertEqual(len(result), 2)
            pid = int(worker.with_suffix('.pid').read_text())
            with self.assertRaises(ProcessLookupError):
                os.kill(pid, 0)

    def test_existing_output_cannot_reuse_a_stale_success(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'result.json').write_text('old success')
            with self.assertRaises(FileExistsError):
                run('/missing-worker', '/missing-input', '/missing-reference', root)
            self.assertEqual((root / 'result.json').read_text(), 'old success')

    def test_nonfinite_timeout_is_rejected_before_startup(self):
        with tempfile.TemporaryDirectory() as directory:
            for value in [float('nan'), float('inf'), -1.0, 0.0]:
                with self.assertRaises(ValueError):
                    run('/missing-worker', '/missing-input', '/missing-reference',
                        Path(directory) / 'output', timeout=value)
                self.assertFalse((Path(directory) / 'output').exists())

    def test_trailing_output_after_eof_is_not_success(self):
        self.exercise('for line in sys.stdin:\n sys.stdout.write(line); sys.stdout.flush()\nsys.stdout.write("{}\\n"); sys.stdout.flush()\n', error=ValueError)

    def test_wrong_request_identity_cannot_match_wrong_reference(self):
        self.exercise('import json; sys.stdin.readline(); print(json.dumps(dict(id="wrong")), flush=True)',
                      request=b'{"id":"right"}\n', expected=b'{"id":"wrong"}\n', error=ValueError)

    def test_partial_response_times_out_and_reaps_worker(self):
        self.exercise('sys.stdin.readline()\nsys.stdout.write("{"); sys.stdout.flush()\ntime.sleep(60)\n',
                      error=TimeoutError)

    def test_nonreading_worker_cannot_block_request_write(self):
        self.exercise('time.sleep(60)\n', request=b'x' * 1048576 + b'\n', error=TimeoutError)

    def test_oversized_partial_frame_is_bounded(self):
        self.exercise('sys.stdin.readline()\nsys.stdout.write("x" * 2000); sys.stdout.flush()\ntime.sleep(60)\n',
                      error=ValueError, limit=256)

    def test_two_complete_replies_match(self):
        self.exercise('for line in sys.stdin:\n sys.stdout.write(line); sys.stdout.flush()\n',
                      request=b'{}\n{"x":1}\n', expected=b'{}\n{"x":1}\n')


if __name__ == '__main__':
    unittest.main()
