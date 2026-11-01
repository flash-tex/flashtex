#!/usr/bin/env python3
"""A guard for the from-scratch reference run in incr_bench.py (unittest).

pdfTeX warnings print the program name (`pdfTeX warning: pdftex (file x.pdf): PDF
inclusion: ...`). The host runs the engine under the bare name `pdftex`; the
reference run used the full path, so any document with such a warning mismatched
on every compile. The reference run must use the bare name as argv[0] and the
real binary as `executable`."""
import os
import re
import unittest


class ReferenceRunArgv0(unittest.TestCase):
    def test_reference_run_uses_the_bare_program_name(self):
        here = os.path.dirname(os.path.abspath(__file__))
        with open(os.path.join(here, "incr_bench.py")) as f:
            src = f.read()
        m = re.search(r"def run_cli\(.*?\n(?=def |\Z)", src, re.S)
        self.assertIsNotNone(m)
        body = m.group(0)
        self.assertIn("['pdftex'] + cmdline", body)
        self.assertIn("executable=f'{E}/pdftex'", body)
        self.assertNotIn("[f'{E}/pdftex'] + cmdline", body)


if __name__ == "__main__":
    unittest.main()
