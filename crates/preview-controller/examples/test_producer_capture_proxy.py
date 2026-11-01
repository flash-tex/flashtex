import errno
import io
import os
import json
import subprocess
import sys
import time
from pathlib import Path
import tempfile
import unittest
from producer_capture_proxy import forward_output, error_record, write_error_stderr


class FailingWriter(io.BytesIO):
    def write(self, data):
        super().write(data[:7])
        raise OSError(errno.ENOSPC, 'private source/path text must not be recorded')


class ProxyTests(unittest.TestCase):
    def test_original_bytes_including_partial_eof_are_preserved(self):
        raw = b'{"x":1}\npartial-eof'
        with tempfile.TemporaryDirectory() as directory:
            output, log = io.BytesIO(), io.BytesIO()
            forward_output(io.BytesIO(raw), output, log, Path(directory), {})
            self.assertEqual(log.getvalue(), raw)
            self.assertEqual(output.getvalue(), raw)

    def test_capture_write_failure_never_forwards_uncaptured_frame(self):
        with tempfile.TemporaryDirectory() as directory:
            state = {}; output = io.BytesIO(); log = FailingWriter()
            with self.assertRaises(OSError) as failure:
                forward_output(io.BytesIO(b'complete original line\n'), output, log, Path(directory), state)
            self.assertEqual(output.getvalue(), b'')
            self.assertEqual(log.getvalue(), b'complet')
            self.assertEqual(error_record(failure.exception, state), dict(
                phase='capture_proxy_failure',stage='capture_write',error_type='OSError',errno=errno.ENOSPC))

    def test_read_failure_exits_proxy_and_terminates_producer(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            producer=root/'producer'
            producer.write_text('#!'+sys.executable+'\nimport time\ntime.sleep(60)\n')
            producer.chmod(0o700)
            script = """import errno
import producer_capture_proxy as proxy
def fail(source, destination, log, root, state):
    state['stage']='producer_read'
    raise OSError(errno.EIO, 'private data')
proxy.forward_output=fail
proxy.main()
"""
            env=dict(os.environ,FLASHTEX_CAPTURE_DIRECTORY=str(root),FLASHTEX_CAPTURE_PRODUCER=str(producer))
            process=subprocess.Popen([sys.executable,'-c',script],stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env,
                cwd=Path(__file__).resolve().parent)
            try:
                process.wait(timeout=3)
                self.assertEqual(process.returncode,126)
                record=json.loads(next(root.glob('*.error.json')).read_text())
                self.assertEqual(record,dict(phase='capture_proxy_failure',stage='producer_read',error_type='OSError',errno=errno.EIO))
                child_pid=int(next(root.glob('*.pid')).read_text())
                deadline=time.monotonic()+2
                while True:
                    status=Path('/proc')/str(child_pid)/'stat'
                    if not status.exists():break
                    if status.read_text().split(') ',1)[1].startswith('Z'):break
                    if time.monotonic()>deadline:self.fail('producer still running after proxy failure')
                    time.sleep(.01)
            finally:
                if process.poll() is None:process.kill()
                process.wait(timeout=2)
                process.stdin.close();process.stdout.close();process.stderr.close()

    def test_full_diagnostic_pipe_does_not_block_or_change_flags(self):
        read_fd,write_fd=os.pipe()
        try:
            os.set_blocking(write_fd,False)
            while True:
                try:os.write(write_fd,b'x'*4096)
                except BlockingIOError:break
            os.set_blocking(write_fd,True)
            write_error_stderr(b'failure record\n',write_fd)
            self.assertTrue(os.get_blocking(write_fd))
        finally:
            os.close(read_fd);os.close(write_fd)

    def test_forward_failure_retains_original_capture(self):
        with tempfile.TemporaryDirectory() as directory:
            raw=b'original\n';log=io.BytesIO();state={}
            with self.assertRaises(OSError):
                forward_output(io.BytesIO(raw), FailingWriter(), log, Path(directory), state)
            self.assertEqual(log.getvalue(),raw)
            self.assertEqual(state['stage'],'forward_write')


if __name__=='__main__':unittest.main()
