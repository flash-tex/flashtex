#!/usr/bin/env python3
"""Tests for trip_fot.py: the two accepted differences pass, anything else
fails, and line endings compare byte for byte (#1208). Stdlib unittest.

    python3 tools/web2rust/tools/test_trip_fot.py
"""
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(HERE, "trip_fot.py")
MASTER = os.path.join(HERE, "..", "..", "..", "third_party", "knuth", "trip",
                      "trip.fot")
TYPED = " &trip  trip "


def ours_from_master():
    """What a correct engine writes: the master without the echoed typed
    line and without its final newline."""
    with open(MASTER, "rb") as fh:
        data = fh.read()
    echo = ("**" + TYPED + "\n").encode("latin-1")
    assert echo in data
    data = data.replace(echo, b"**", 1)
    assert data.endswith(b"\n")
    return data[:-1]


class TripFotTest(unittest.TestCase):
    def setUp(self):
        fd, self.path = tempfile.mkstemp(prefix="trip-fot-", suffix=".fot")
        os.close(fd)

    def tearDown(self):
        os.unlink(self.path)

    def check(self, data, *typed):
        with open(self.path, "wb") as fh:
            fh.write(data)
        p = subprocess.run([sys.executable, SCRIPT, MASTER, self.path]
                           + list(typed or (TYPED,)),
                           capture_output=True, text=True)
        return p.returncode, p.stdout

    def test_correct_transcript_passes(self):
        rc, out = self.check(ours_from_master())
        self.assertEqual(rc, 0, out)
        self.assertIn("final newline dropped", out)

    def test_final_newline_kept_passes(self):
        rc, out = self.check(ours_from_master() + b"\n")
        self.assertEqual(rc, 0, out)

    def test_crlf_fails(self):
        rc, out = self.check(ours_from_master().replace(b"\n", b"\r\n"))
        self.assertEqual(rc, 1, out)
        self.assertIn("first difference at line 1", out)

    def test_one_cr_fails(self):
        data = ours_from_master()
        i = data.index(b"\n", len(data) // 2)
        rc, out = self.check(data[:i] + b"\r" + data[i:])
        self.assertEqual(rc, 1, out)

    def test_changed_character_fails(self):
        data = bytearray(ours_from_master())
        data[len(data) // 2] ^= 1
        rc, out = self.check(bytes(data))
        self.assertEqual(rc, 1, out)
        self.assertIn("first difference", out)

    def test_missing_tail_fails(self):
        rc, out = self.check(ours_from_master()[:-50])
        self.assertEqual(rc, 1, out)

    def test_extra_trailing_space_fails(self):
        rc, out = self.check(ours_from_master() + b" ")
        self.assertEqual(rc, 1, out)

    def test_echo_left_in_fails(self):
        # Ours must not contain the typed line: that is the master's echo.
        with open(MASTER, "rb") as fh:
            data = fh.read()[:-1]
        rc, out = self.check(data)
        self.assertEqual(rc, 1, out)

    def test_unknown_typed_line_fails(self):
        rc, out = self.check(ours_from_master(), "not typed")
        self.assertEqual(rc, 1, out)
        self.assertIn("no `**not typed` prompt echo", out)


if __name__ == "__main__":
    unittest.main()
