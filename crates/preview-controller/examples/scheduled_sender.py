#!/usr/bin/env python3
"""Linux benchmark sender isolated from receiver JSON decoding; no provider calls."""
import base64
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import time


def transmit(fd, frames, start, interval, deadline):
    records = []
    for index, frame in enumerate(frames):
        due = start + index * interval
        remaining = min(due, deadline) - time.monotonic()
        if remaining > 0:
            time.sleep(remaining)
        sent = time.monotonic()
        offset = 0
        while offset < len(frame):
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not select.select([], [fd], [], remaining)[1]:
                raise TimeoutError('sender write deadline')
            # This is the only writer during the burst. Writes no larger than
            # PIPE_BUF are safe after pipe writability without changing shared flags.
            count = os.write(fd, frame[offset:offset + select.PIPE_BUF])
            if count <= 0:
                raise BrokenPipeError('sender made no write progress')
            offset += count
        records.append(dict(index=index, target_ms=index * interval * 1000,
                            sent_ms=(sent-start)*1000,
                            flush_ms=(time.monotonic()-start)*1000))
    return records


class ScheduledSender:
    """Parent owns termination/reaping and must finish before writing stdin again."""
    def __init__(self, fd, frames, directory, start, interval=.030, duration=30):
        self.result = Path(directory) / 'sender-result.json'
        config = Path(directory) / 'sender-config.json'
        config.write_text(json.dumps(dict(frames=[base64.b64encode(f).decode() for f in frames],
                                         start=start, interval=interval, deadline=start+duration)))
        self.proc = subprocess.Popen([sys.executable, str(Path(__file__).resolve()),
                                      str(fd), str(config), str(self.result)],
                                     pass_fds=(fd,), stdin=subprocess.DEVNULL,
                                     stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    def check_failure(self):
        status = self.proc.poll()
        if status is not None and status != 0:
            self.finish()

    def finish(self, timeout=2):
        try:
            self.proc.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            self.stop()
            raise TimeoutError('sender completion deadline') from None
        if not self.result.exists():
            raise RuntimeError(f'sender exited {self.proc.returncode} without result')
        result = json.loads(self.result.read_text())
        if self.proc.returncode != 0 or 'error' in result:
            raise RuntimeError(result.get('error', 'sender failed'))
        return result['sends']

    def stop(self):
        if self.proc.poll() is None:
            self.proc.kill()
        self.proc.wait(timeout=2)


if __name__ == '__main__':
    fd, config, result = int(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
    try:
        data = json.loads(config.read_text())
        frames = [base64.b64decode(f, validate=True) for f in data['frames']]
        sends = transmit(fd, frames, data['start'], data['interval'], data['deadline'])
        result.write_text(json.dumps(dict(sends=sends)))
    except Exception as error:
        result.write_text(json.dumps(dict(error=f'{type(error).__name__}: {error}')))
        sys.exit(1)
    finally:
        os.close(fd)
