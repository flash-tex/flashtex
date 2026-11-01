#!/usr/bin/env python3
"""Linux-only transparent original-producer capture for local acceptance replay."""
import ctypes
import json
import os
from pathlib import Path
import signal
import stat
import subprocess
import sys
import threading
import time


def forward_output(source, destination, log, root, state):
    """Preserve original bytes; let pump errors reach the process-level handler."""
    while True:
        state['stage'] = 'producer_read'
        frame = source.readline()
        if not frame:
            return
        state['stage'] = 'capture_write'
        log.write(frame); log.flush()
        if (root / 'hold-output').exists():
            state['stage'] = 'capture_hold'
            (root / 'output-held').write_text(str(os.getpid()))
            deadline = time.monotonic() + 10
            while (root / 'hold-output').exists():
                if time.monotonic() > deadline:
                    raise TimeoutError('capture hold deadline')
                time.sleep(.002)
        state['stage'] = 'forward_write'
        destination.write(frame); destination.flush()


def error_record(error, state):
    # Exception messages can include source/path data; retain only scalar classification.
    return dict(phase='capture_proxy_failure', stage=state['stage'],
                error_type=type(error).__name__, errno=getattr(error, 'errno', None))


def write_error_stderr(record, fd=2):
    """Do not wait on a full diagnostic pipe or change inherited descriptor flags."""
    temporary = None
    try:
        if stat.S_ISFIFO(os.fstat(fd).st_mode):
            temporary = os.open('/proc/self/fd/'+str(fd), os.O_WRONLY | os.O_NONBLOCK)
            os.write(temporary, record)
        elif stat.S_ISREG(os.fstat(fd).st_mode) or stat.S_ISCHR(os.fstat(fd).st_mode):
            os.write(fd, record)
    except OSError:
        pass
    finally:
        if temporary is not None:os.close(temporary)


def main():
    root = Path(os.environ['FLASHTEX_CAPTURE_DIRECTORY'])
    prefix = root / ('producer-' + str(os.getpid()))
    proxy_pid = os.getpid()
    def parent_death():
        # This experimental Linux capture must not orphan a producer on proxy kill.
        if ctypes.CDLL(None).prctl(1, signal.SIGKILL, 0, 0, 0) != 0:
            os._exit(125)
        if os.getppid() != proxy_pid:
            os._exit(125)
    child = subprocess.Popen([os.environ['FLASHTEX_CAPTURE_PRODUCER']],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=sys.stderr,
        preexec_fn=parent_death)
    (prefix.with_suffix('.pid')).write_text(str(child.pid))
    def output():
        state = dict(stage='capture_open')
        try:
            with prefix.with_suffix('.output.jsonl').open('wb') as log:
                forward_output(child.stdout, sys.stdout.buffer, log, root, state)
        except Exception as error:
            record = (json.dumps(error_record(error, state)) + '\n').encode()
            # A full capture filesystem may reject the sidecar too; stderr remains
            # a separate best-effort route; full diagnostic pipes are not awaited.
            try:
                prefix.with_suffix('.error.json').write_bytes(record)
            except OSError:
                pass
            write_error_stderr(record)
            os._exit(126)  # Existing PDEATHSIG kills the original producer.
    reader = threading.Thread(target=output)
    reader.start()
    try:
        with prefix.with_suffix('.input.jsonl').open('wb') as log:
            for frame in sys.stdin.buffer:
                log.write(frame); log.flush()
                child.stdin.write(frame); child.stdin.flush()
        child.stdin.close()
        child.wait(timeout=10)
    finally:
        if child.poll() is None:
            child.kill(); child.wait(timeout=5)
        reader.join(timeout=5)
        if reader.is_alive():
            os._exit(124)

if __name__ == '__main__':
    main()
