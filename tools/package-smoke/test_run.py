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
[ -n "$FAKE_BADPDF" ] && { printf '%%PDF-1.5\ngarbage\n%%%%EOF\n' > doc.pdf; exit "${FAKE_RC:-0}"; }
[ -z "$FAKE_NOPDF" ] && printf '%%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R >>\nendobj\n4 0 obj\n<< /Length 1 >>\nstream\n%s\nendstream\nendobj\ntrailer\n<< /Root 1 0 R /Size 5 >>\n%%%%EOF\n' "${FAKE_PDF:-pdf}" > doc.pdf
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

    def test_unreadable_candidate_pdf_fails_that_document(self):
        cand = self.passes({**self.env, "FAKE_BADPDF": "1"}, False)
        ref = self.passes(self.env, True)
        self.assertIn("unreadable", smoke.compare(cand, ref))

    def test_unreadable_reference_pdf_aborts_the_run(self):
        with self.assertRaises(smoke.HarnessError):
            self.passes({**self.env, "FAKE_BADPDF": "1"}, True)

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


def _pdf(stream):
    """A minimal uncompressed one-page PDF whose content stream is `stream` (bytes)."""
    objs = [b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] /Contents 4 0 R >>",
            b"<< /Length %d >>\nstream\n" % len(stream) + stream + b"\nendstream"]
    out, offs = b"%PDF-1.4\n", []
    for n, body in enumerate(objs, 1):
        offs.append(len(out))
        out += b"%d 0 obj\n" % n + body + b"\nendobj\n"
    xref = len(out)
    out += b"xref\n0 5\n0000000000 65535 f \n" + b"".join(b"%010d 00000 n \n" % o for o in offs)
    return out + b"trailer\n<< /Size 5 /Root 1 0 R /ID [<aa><bb>] >>\nstartxref\n%d\n%%%%EOF\n" % xref


@unittest.skipUnless(shutil.which("qpdf"), "qpdf not installed")
class PdfSignatureTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()

    def tearDown(self):
        self.tmp.cleanup()

    def write(self, name, data):
        p = os.path.join(self.tmp.name, name)
        with open(p, "wb") as fh:
            fh.write(data)
        return p

    def test_same_bytes_at_another_path_are_equal(self):
        data = _pdf(b"0 0 m 5 5 l S")
        self.assertEqual(smoke.pdf_signature(self.write("a.pdf", data)),
                         smoke.pdf_signature(self.write("b.pdf", data)))

    def test_id_lookalike_in_a_content_stream_is_not_hidden(self):
        a = self.write("a.pdf", _pdf(b"/ID [<00> <11>] 0 0 m"))
        b = self.write("b.pdf", _pdf(b"/ID [<00> <22>] 0 0 m"))
        self.assertNotEqual(smoke.pdf_signature(a), smoke.pdf_signature(b))

    def test_unreadable_pdf_is_a_harness_error(self):
        with self.assertRaises(smoke.HarnessError):
            smoke.pdf_signature(self.write("bad.pdf", b"not a pdf"))

    def test_missing_qpdf_is_a_harness_error(self):
        with mock.patch.object(smoke.subprocess, "run", side_effect=FileNotFoundError("qpdf")):
            with self.assertRaises(smoke.HarnessError):
                smoke.pdf_signature("x.pdf")


if __name__ == "__main__":
    unittest.main()
