import os
from pathlib import Path
import tempfile
import threading
import time
import unittest
from scheduled_sender import ScheduledSender


class SenderTests(unittest.TestCase):
    def test_exact_order_and_parent_writer_survives(self):
        read_fd, write_fd = os.pipe()
        try:
            with tempfile.TemporaryDirectory() as directory:
                sender = ScheduledSender(write_fd, [b'first\n', 'α\n'.encode()], directory,
                                         time.monotonic(), interval=0)
                rows = sender.finish()
                self.assertEqual([r['index'] for r in rows], [0, 1])
                self.assertLessEqual(rows[0]['flush_ms'], rows[1]['sent_ms'])
                os.write(write_fd, b'parent\n')
                self.assertEqual(os.read(read_fd, 100), 'first\nα\nparent\n'.encode())
                self.assertIsNotNone(sender.proc.returncode)
        finally:
            os.close(read_fd); os.close(write_fd)

    def test_fragmented_large_frames_preserve_all_bytes(self):
        read_fd, write_fd = os.pipe()
        frames = [('α' * 50000 + '\n').encode(), b'z' * 75000 + b'\n']
        received = bytearray()
        errors = []
        def drain():
            try:
                while len(received) < sum(map(len, frames)):
                    chunk = os.read(read_fd, 317)
                    if not chunk:
                        break
                    received.extend(chunk)
            except Exception as error:
                errors.append(error)
        reader = threading.Thread(target=drain, daemon=True)
        sender = None
        try:
            reader.start()
            with tempfile.TemporaryDirectory() as directory:
                sender = ScheduledSender(write_fd, frames, directory,
                                         time.monotonic(), interval=0)
                records = sender.finish()
                reader.join(timeout=2)
                self.assertFalse(reader.is_alive())
                self.assertFalse(errors)
                self.assertEqual(bytes(received), b''.join(frames))
                self.assertEqual(len(records), 2)
                self.assertLessEqual(records[0]['flush_ms'], records[1]['sent_ms'])
        finally:
            if sender: sender.stop()
            os.close(write_fd)
            reader.join(timeout=2)
            os.close(read_fd)

    def test_closed_reader_is_reported_and_reaped(self):
        read_fd, write_fd = os.pipe(); os.close(read_fd)
        try:
            with tempfile.TemporaryDirectory() as directory:
                sender = ScheduledSender(write_fd, [b'x'], directory, time.monotonic())
                sender.proc.wait(timeout=2)
                with self.assertRaisesRegex(RuntimeError, 'BrokenPipeError'):
                    sender.check_failure()
                self.assertIsNotNone(sender.proc.returncode)
        finally:
            os.close(write_fd)

    def test_backpressure_has_deadline(self):
        read_fd, write_fd = os.pipe()
        try:
            with tempfile.TemporaryDirectory() as directory:
                sender = ScheduledSender(write_fd, [b'x' * 1000000], directory,
                                         time.monotonic(), duration=.2)
                with self.assertRaisesRegex(RuntimeError, 'deadline'):
                    sender.finish()
                self.assertIsNotNone(sender.proc.returncode)
        finally:
            os.close(read_fd); os.close(write_fd)

    def test_cancel_waiting_sender_reaps_without_writing(self):
        read_fd, write_fd = os.pipe()
        try:
            with tempfile.TemporaryDirectory() as directory:
                sender = ScheduledSender(write_fd, [b'x'], directory, time.monotonic()+60)
                sender.stop(); sender.stop()
                os.set_blocking(read_fd, False)
                with self.assertRaises(BlockingIOError):
                    os.read(read_fd, 1)
                self.assertIsNotNone(sender.proc.returncode)
        finally:
            os.close(read_fd); os.close(write_fd)

    def test_parent_timeout_kills_and_reaps(self):
        read_fd, write_fd = os.pipe()
        try:
            with tempfile.TemporaryDirectory() as directory:
                sender = ScheduledSender(write_fd, [b'x'], directory, time.monotonic()+60)
                with self.assertRaisesRegex(TimeoutError, 'completion deadline'):
                    sender.finish(timeout=.01)
                self.assertIsNotNone(sender.proc.returncode)
        finally:
            os.close(read_fd); os.close(write_fd)


if __name__ == '__main__':
    unittest.main()
