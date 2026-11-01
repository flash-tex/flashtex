import json
import os
import tempfile
import unittest
from helper_replay import Client


class SnapshotTests(unittest.TestCase):
    def client(self, file):
        client=Client.__new__(Client)
        client.diagnostic_file=file
        return client

    def test_shared_writer_offset_and_appended_bytes_unchanged(self):
        with tempfile.TemporaryFile() as file:
            file.write(b'{"a":1}\n');file.flush()
            writer=os.dup(file.fileno())
            try:
                os.lseek(writer,3,os.SEEK_SET)
                client=self.client(file)
                self.assertEqual(client.diagnostics(),[{'a':1}])
                self.assertEqual(os.lseek(writer,0,os.SEEK_CUR),3)
                os.lseek(writer,0,os.SEEK_END)
                client.diagnostics()
                os.write(writer,b'{"b":2}\n')
                self.assertEqual(client.diagnostics(),[{'a':1},{'b':2}])
                self.assertEqual(os.pread(writer,100,0),b'{"a":1}\n{"b":2}\n')
            finally:os.close(writer)

    def test_partial_tail_retained_then_completed(self):
        with tempfile.TemporaryFile() as file:
            file.write(b'{"a":1}\n{"b":');file.flush()
            client=self.client(file)
            self.assertEqual(client.diagnostics(),[{'a':1}])
            self.assertTrue(client.diagnostics_status['partial_tail'])
            self.assertTrue(client.last_diagnostics_raw.endswith(b'{"b":'))
            file.write(b'2}\n');file.flush()
            self.assertEqual(client.diagnostics(),[{'a':1},{'b':2}])
            self.assertFalse(client.diagnostics_status['partial_tail'])

    def test_non_json_record_is_preserved_and_refused(self):
        with tempfile.TemporaryFile() as file:
            file.write(b'not json\n');file.flush()
            client=self.client(file)
            with self.assertRaises(json.JSONDecodeError):client.diagnostics()
            self.assertTrue(client.diagnostics_status['non_json'])
            self.assertEqual(client.last_diagnostics_raw,b'not json\n')

    def test_capture_limit_remains_bounded(self):
        with tempfile.TemporaryFile() as file:
            file.write(b'x'*(1024*1024+2));file.flush()
            client=self.client(file)
            with self.assertRaisesRegex(RuntimeError,'capture limit'):client.diagnostics()
            self.assertEqual(len(client.last_diagnostics_raw),1024*1024+1)
            self.assertTrue(client.diagnostics_status['truncated'])


if __name__=='__main__':unittest.main()
