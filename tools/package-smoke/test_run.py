import importlib.util
import os
import shutil
import stat
import tempfile
import unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location("smoke_run", os.path.join(HERE, "run.py"))
smoke = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(smoke)

VERSION_LINE = "pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026)"
# A fake engine. `--version` prints $FAKE_VERSION; a run writes doc.log (with a box dump
# whose text depends on $FAKE_BOX and on the contents of smoke-doc.tex) and, unless
# $FAKE_NOPDF is set, doc.pdf (contents from $FAKE_PDF). $FAKE_RC is the exit code.
FAKE = r"""#!/bin/sh
case "$1" in --version) echo "$FAKE_VERSION"; exit 0;; esac
box="${FAKE_BOX:-box}"
{
  echo "This is a fake engine"
  echo "Completed box being shipped out [1]"
  echo "\\hbox($box)"
  if [ -n "$FAKE_NOPDF" ]; then echo "Output written on doc.pdf (1 page, 10 bytes)."
  else echo "Output written on doc.pdf (1 page, 10 bytes)."; fi
} > doc.log
[ -z "$FAKE_NOPDF" ] && printf '%%PDF-1.5\n%s\n%%%%EOF\n' "${FAKE_PDF:-pdf}" > doc.pdf
exit "${FAKE_RC:-0}"
"""


def make_engine(tmp, name="pdftex"):
    path = os.path.join(tmp, name)
    with open(path, "w") as fh:
        fh.write(FAKE)
    os.chmod(path, os.stat(path).st_mode | stat.S_IXUSR)
    return path


class RunTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="pkgsmoke-test-")
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.eng = make_engine(self.tmp)
        self.tex = os.path.join(self.tmp, "smoke-input.tex")
        with open(self.tex, "w") as fh:
            fh.write("\\documentclass{article}\n")
        self.env = {"FAKE_VERSION": VERSION_LINE}

    def passes(self, env, is_reference, passes=2):
        with mock.patch.dict(os.environ, env, clear=False):
            return smoke.run_passes(self.tex, self.eng, None, passes, 30,
                                    is_reference=is_reference)

    def diff(self, cand_env, ref_env=None):
        base = dict(self.env)
        cand = self.passes({**base, **cand_env}, False)
        ref = self.passes({**base, **(ref_env or {})}, True)
        return smoke.compare(cand, ref)

    def test_identical_engines_are_equal_and_run_two_passes(self):
        self.assertEqual(len(self.passes(self.env, False)), 2)
        self.assertIsNone(self.diff({}))

    def test_different_box_dumps_are_different(self):
        self.assertIn("differs", self.diff({"FAKE_BOX": "other"}))

    def test_missing_pdf_fails(self):
        why = self.diff({"FAKE_NOPDF": "1"})
        self.assertIsNotNone(why)
        self.assertIn("PDF", why)

    def test_different_pdf_content_is_different(self):
        self.assertIn("PDF differs", self.diff({"FAKE_PDF": "other"}))

    def test_exit_code_difference(self):
        self.assertIn("exit", self.diff({"FAKE_RC": "1"}))

    def test_reference_that_does_not_compile_is_not_equal(self):
        why = self.diff({"FAKE_RC": "1"}, {"FAKE_RC": "1"})
        self.assertIn("reference itself", why)

    def test_wrong_reference_version_is_a_harness_error(self):
        env = {**self.env, "FAKE_VERSION": "pdfTeX 3.14-2.6-1.40.28 (TeX Live 2025)"}
        with mock.patch.dict(os.environ, env, clear=False):
            with self.assertRaises(smoke.HarnessError):
                smoke.run_passes(self.tex, self.eng, None, 1, 30, is_reference=True)

    def test_missing_candidate_is_different_not_a_traceback(self):
        with mock.patch.dict(os.environ, self.env, clear=False):
            cand = smoke.run_passes(self.tex, os.path.join(self.tmp, "nonexistent"),
                                    None, 2, 30, is_reference=False)
        ref = self.passes(self.env, True)
        self.assertIn("not found", smoke.compare(cand, ref))

    def test_hanging_candidate_is_different(self):
        hang = os.path.join(self.tmp, "hang")
        with open(hang, "w") as fh:
            fh.write("#!/bin/sh\nsleep 30\n")
        os.chmod(hang, 0o755)
        with mock.patch.dict(os.environ, self.env, clear=False):
            cand = smoke.run_passes(self.tex, hang, None, 1, 1, is_reference=False)
        ref = self.passes(self.env, True)
        self.assertIn("timed out", smoke.compare(cand, ref))


class SelectTest(unittest.TestCase):
    def test_unknown_package_is_an_error(self):
        with self.assertRaises(smoke.HarnessError):
            smoke.select(["no-such-package-xyz"])

    def test_default_selects_every_document(self):
        names = smoke.select([])
        self.assertIn("framed", names)
        self.assertIn("xkeyval-smoke", names)

    def test_main_exits_2_on_unknown_and_on_empty_selection(self):
        self.assertEqual(smoke.main(["--candidate", "/nonexistent", "no-such-xyz"]), 2)
        with mock.patch.object(smoke, "select", side_effect=smoke.HarnessError("none")):
            self.assertEqual(smoke.main(["--candidate", "/nonexistent"]), 2)

    def test_id_second_half_is_removed(self):
        a = b"  /ID [<aa><11>]\n"
        b = b"  /ID [<aa><22>]\n"
        self.assertEqual(smoke.ID_RE.sub(rb"\1 <ID>]", a), smoke.ID_RE.sub(rb"\1 <ID>]", b))


if __name__ == "__main__":
    unittest.main()
