#!/usr/bin/env python3
"""Unit test for tools/lockstep/run.py:capture(). Stdlib unittest only.

Run as `python3 -m unittest tools.lockstep.test_capture` from the repo
root, or directly as `python3 tools/lockstep/test_capture.py`.
"""
import contextlib
import inspect
import io
import os
import shutil
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import run as lockstep_run


class CaptureTest(unittest.TestCase):
    def test_capture_case001_reference(self):
        if shutil.which("pdftex") is None:
            self.skipTest("reference engine pdftex not on PATH")
        workdir = tempfile.mkdtemp(prefix="lockstep-test-")
        try:
            shutil.copy(lockstep_run.PRELUDE,
                        os.path.join(workdir, "prelude.tex"))
            shutil.copy(os.path.join(lockstep_run.CASES_DIR,
                                     "001-edef-basic.tex"),
                        os.path.join(workdir, "001-edef-basic.tex"))
            sentinel = os.path.join(workdir, "sentinel.txt")
            with open(sentinel, "w") as fh:
                fh.write("capture must not wipe the workdir\n")
            cap = lockstep_run.capture(
                os.path.join(workdir, "001-edef-basic.tex"),
                "pdftex", workdir, extra_env={"LOCKSTEP_TEST": "1"})
            self.assertIs(type(cap.log), str)
            self.assertTrue(cap.log)
            self.assertEqual(cap.returncode, 0)
            self.assertIn("entering extended mode", cap.log)
            self.assertEqual(len(cap.boxes), 1)
            self.assertIs(type(cap.boxes[0]), str)
            self.assertIn("Completed box being shipped out", cap.boxes[0])
            self.assertIsNotNone(cap.pdf_path)
            self.assertTrue(os.path.isfile(cap.pdf_path))
            self.assertTrue(os.path.isfile(sentinel))
        finally:
            shutil.rmtree(workdir, ignore_errors=True)

    def test_capture_stale_workdir_reuse(self):
        if shutil.which("pdftex") is None:
            self.skipTest("reference engine pdftex not on PATH")
        workdir = tempfile.mkdtemp(prefix="lockstep-test-stale-")
        try:
            shutil.copy(lockstep_run.PRELUDE,
                        os.path.join(workdir, "prelude.tex"))
            tex = os.path.join(workdir, "001-edef-basic.tex")
            shutil.copy(os.path.join(lockstep_run.CASES_DIR,
                                     "001-edef-basic.tex"), tex)
            first = lockstep_run.capture(tex, "pdftex", workdir)
            self.assertEqual(first.returncode, 0)
            self.assertTrue(first.log.strip())
            self.assertIsNotNone(first.pdf_path)
            keep_aux = os.path.join(workdir, "001-edef-basic.aux")
            with open(keep_aux, "w") as fh:
                fh.write("caller-owned aux must survive\n")
            fake = os.path.join(workdir, "fake139.sh")
            with open(fake, "w") as fh:
                fh.write("#!/bin/sh\nexit 139\n")
            os.chmod(fake, 0o755)
            cap = lockstep_run.capture(tex, fake, workdir)
            self.assertNotEqual(cap.returncode, 0)
            self.assertEqual(cap.returncode, 139)
            self.assertEqual(cap.log, "")
            self.assertIsNone(cap.pdf_path)
            self.assertTrue(os.path.isfile(keep_aux))
        finally:
            shutil.rmtree(workdir, ignore_errors=True)

    def test_capture_037_lastbox_single_shipout(self):
        if shutil.which("pdftex") is None:
            self.skipTest("reference engine pdftex not on PATH")
        workdir = tempfile.mkdtemp(prefix="lockstep-test-037-")
        try:
            shutil.copy(lockstep_run.PRELUDE,
                        os.path.join(workdir, "prelude.tex"))
            tex = os.path.join(workdir, "037-lastbox.tex")
            shutil.copy(os.path.join(lockstep_run.CASES_DIR,
                                     "037-lastbox.tex"), tex)
            cap = lockstep_run.capture(tex, "pdftex", workdir)
            self.assertEqual(cap.returncode, 0)
            self.assertEqual(cap.log.count(
                "Completed box being shipped out"), 1)
            self.assertEqual(len(cap.boxes), 1)
            self.assertIn("Completed box being shipped out", cap.boxes[0])
        finally:
            shutil.rmtree(workdir, ignore_errors=True)

    def test_capture_twopage_pdflatex_two_boxes(self):
        if shutil.which("pdftex") is None:
            self.skipTest("reference engine pdftex not on PATH")
        fmt_check = shutil.which("kpsewhich") is None or os.system(
            "kpsewhich -engine=pdftex pdflatex.fmt >/dev/null 2>&1") != 0
        if fmt_check:
            self.skipTest("pdflatex format not available")
        workdir = tempfile.mkdtemp(prefix="lockstep-test-2page-")
        try:
            tex = os.path.join(workdir, "twopage.tex")
            with open(tex, "w") as fh:
                fh.write("\\documentclass{article}\n"
                         "\\tracingoutput=1\n"
                         "\\begin{document}\n"
                         "Page one.\n"
                         "\\newpage\n"
                         "Page two.\n"
                         "\\end{document}\n")
            cap = lockstep_run.capture(tex, "pdftex", workdir,
                                       fmt="pdflatex")
            self.assertEqual(cap.returncode, 0)
            self.assertEqual(cap.log.count(
                "Completed box being shipped out"), 2)
            self.assertEqual(len(cap.boxes), 2)
        finally:
            shutil.rmtree(workdir, ignore_errors=True)

    def test_capture_signature(self):
        sig = inspect.signature(lockstep_run.capture)
        params = list(sig.parameters.values())
        self.assertEqual([p.name for p in params[:3]],
                         ["tex_path", "engine_bin", "workdir"])
        for name in ("fmt", "extra_env"):
            self.assertEqual(sig.parameters[name].kind,
                             inspect.Parameter.KEYWORD_ONLY)
            self.assertIsNone(sig.parameters[name].default)


class AccountingSplitTest(unittest.TestCase):
    SAMPLE = [
        "BANNER",
        "entering extended mode",
        "Memory usage before: 29&45; after: 20&45; still untouched: 4998918",
        "{\\tracingassigns}",
        ".\\glue 10.0",
        "Here is how much of TeX's memory you used:",
        " 12 strings out of 497895",
        " 2i,0n,1p,153b,9s stack positions out of 10000i,1000n,20000p,1s",
        "",
        "Output written on foo.pdf (2 pages, 1500 bytes).",
        "PDF statistics:",
        " 6 PDF objects out of 1000 (max. 8388607)",
        "",
        "tail line",
    ]

    def test_compared_removes_accounting_keeps_rest(self):
        kept, acc = lockstep_run.split_accounting(list(self.SAMPLE))
        self.assertEqual(kept, [
            "BANNER",
            "entering extended mode",
            "{\\tracingassigns}",
            ".\\glue 10.0",
            "",
            "Output written on foo.pdf (2 pages, <BYTES> bytes).",
            "",
            "tail line",
        ])
        self.assertEqual(acc, [
            "Memory usage before: 29&45; after: 20&45; still untouched: 4998918",
            "Here is how much of TeX's memory you used:",
            " 12 strings out of 497895",
            " 2i,0n,1p,153b,9s stack positions out of 10000i,1000n,20000p,1s",
            "Output written on foo.pdf (2 pages, 1500 bytes).",
            "PDF statistics:",
            " 6 PDF objects out of 1000 (max. 8388607)",
        ])

    def test_singular_page_byte_count_replaced(self):
        kept, acc = lockstep_run.split_accounting(
            ["Output written on f.pdf (1 page, 950 bytes)."])
        self.assertEqual(
            kept, ["Output written on f.pdf (1 page, <BYTES> bytes)."])
        self.assertEqual(acc, ["Output written on f.pdf (1 page, 950 bytes)."])

    def test_diff_kinds(self):
        _, acc = lockstep_run.split_accounting(list(self.SAMPLE))
        self.assertEqual(lockstep_run.accounting_diff_kinds(acc, acc), [])
        changed = list(acc)
        changed[4] = changed[4].replace("1500", "1501")
        self.assertEqual(lockstep_run.accounting_diff_kinds(acc, changed),
                         ["pdf bytes"])
        changed = list(acc)
        changed[2] = changed[2].replace("497895", "497896")
        self.assertEqual(lockstep_run.accounting_diff_kinds(acc, changed),
                         ["memory usage"])
        changed = list(acc)
        changed[6] = changed[6].replace("8388607", "8388608")
        self.assertEqual(lockstep_run.accounting_diff_kinds(acc, changed),
                         ["pdf stats"])
        changed = list(acc)
        changed[0] = changed[0].replace("4998918", "4998919")
        changed[4] = changed[4].replace("1500", "1501")
        self.assertEqual(lockstep_run.accounting_diff_kinds(acc, changed),
                         ["memory usage", "pdf bytes"])


WRAPPER_SRC = r'''#!/usr/bin/env python3
import os, re, subprocess, sys
PDFTEX = @@PDFTEX@@
MODE = @@MODE@@
def main():
    args = sys.argv[1:]
    job = os.path.splitext(os.path.basename(args[-1]))[0]
    proc = subprocess.run([PDFTEX] + args, stdout=subprocess.PIPE,
                          stderr=subprocess.STDOUT)
    log_path = job + ".log"
    try:
        with open(log_path, encoding="utf-8", errors="replace") as fh:
            log = fh.read()
    except OSError:
        return proc.returncode
    if MODE == "bytes":
        log = re.sub(r"(\(\d+ pages?, )\d+( bytes\))",
                     r"\g<1>42424242\g<2>", log)
    elif MODE == "memory":
        # The prelude pins \tracingstats=0, so case logs have no stats
        # block; simulate an engine that prints one (a different memory
        # layout). Inserted before "Output written on", as pdfTeX does.
        block = ["Here is how much of TeX's memory you used:",
                 " 12 strings out of 497895",
                 " 188 string characters out of 6213314",
                 " 1082 words of memory out of 5000000",
                 " 554 multiletter control sequences out of 15000+600000",
                 " 7 words of font info for 0 fonts, out of 8000000 for 9000",
                 " 0 hyphenation exceptions out of 8191",
                 " 2i,0n,1p,153b,9s stack positions out of "
                 "10000i,1000n,20000p,200000b,200000s"]
        lines = log.split("\n")
        for i, ln in enumerate(lines):
            if ln.startswith("Output written on"):
                lines[i:i] = block
                break
        log = "\n".join(lines)
    elif MODE == "pdfstats":
        lines = log.split("\n")
        for i, ln in enumerate(lines):
            if ln.startswith("PDF statistics:") and i + 1 < len(lines):
                lines[i + 1] = re.sub(r"\d+",
                                      lambda m: m.group(0) + "7",
                                      lines[i + 1], count=1)
                break
        log = "\n".join(lines)
    elif MODE == "pages":
        log = re.sub(r"Output written on (.*)\((\d+) (page)",
                     lambda m: "Output written on %s(%d %s" %
                     (m.group(1), int(m.group(2)) + 1, m.group(3)), log)
    elif MODE == "glue":
        log = re.sub(r"(\\glue )([\d.]+)",
                     lambda m: m.group(1) + m.group(2) + "1", log, count=1)
    elif MODE == "trace":
        log = log.replace("{\\tracingoutput}", "{\\tracingOutput}", 1)
    with open(log_path, "w", encoding="utf-8") as fh:
        fh.write(log)
    return proc.returncode
sys.exit(main())
'''


class WrapperEngineTest(unittest.TestCase):
    CASE = "001-edef-basic"

    @classmethod
    def setUpClass(cls):
        pdftex = shutil.which("pdftex")
        if pdftex is None:
            raise unittest.SkipTest("reference engine pdftex not on PATH")
        cls.workdir = tempfile.mkdtemp(prefix="lockstep-wrap-")
        cls.wrappers = {}
        for mode in ("bytes", "memory", "pdfstats", "pages", "glue",
                     "trace"):
            path = os.path.join(cls.workdir, "wrap-%s.py" % mode)
            with open(path, "w") as fh:
                fh.write(WRAPPER_SRC.replace("@@PDFTEX@@", repr(pdftex))
                         .replace("@@MODE@@", repr(mode)))
            os.chmod(path, 0o755)
            cls.wrappers[mode] = path

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.workdir, ignore_errors=True)

    def run_cli(self, *argv):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            rc = lockstep_run.main(list(argv))
        return rc, out.getvalue()

    def check_pass_with_accounting(self, mode, kind):
        rc, out = self.run_cli("--engine", self.wrappers[mode], "--cases",
                               self.CASE, "--allow-any-reference")
        self.assertEqual(rc, 0, msg=out)
        self.assertIn("PASS %s" % self.CASE, out)
        self.assertIn("accounting: %s differs (%s)" % (self.CASE, kind),
                      out)
        self.assertIn("accounting: 1 case differs", out)

    def test_bytes_only_pass_with_accounting(self):
        self.check_pass_with_accounting("bytes", "pdf bytes")

    def test_memory_only_pass_with_accounting(self):
        self.check_pass_with_accounting("memory", "memory usage")

    def test_pdfstats_only_pass_with_accounting(self):
        self.check_pass_with_accounting("pdfstats", "pdf stats")

    def check_fail(self, mode):
        rc, out = self.run_cli("--engine", self.wrappers[mode], "--cases",
                               self.CASE, "--allow-any-reference")
        self.assertEqual(rc, 1, msg=out)
        self.assertIn("FAIL %s" % self.CASE, out)

    def test_page_count_fails(self):
        self.check_fail("pages")

    def test_glue_fails(self):
        self.check_fail("glue")

    def test_trace_fails(self):
        self.check_fail("trace")

    def test_self_test_subset_equal(self):
        rc, out = self.run_cli("--self-test", "--cases", self.CASE,
                               "--allow-any-reference")
        self.assertEqual(rc, 0, msg=out)
        self.assertIn("1 cases, 1 equal, 0 differ", out)
        self.assertIn("accounting: 0 cases differ", out)

    def test_capture_accounting_fields(self):
        workdir = tempfile.mkdtemp(prefix="lockstep-acc-")
        try:
            shutil.copy(lockstep_run.PRELUDE,
                        os.path.join(workdir, "prelude.tex"))
            tex = os.path.join(workdir, self.CASE + ".tex")
            shutil.copy(os.path.join(lockstep_run.CASES_DIR,
                                     self.CASE + ".tex"), tex)
            cap = lockstep_run.capture(tex, "pdftex", workdir)
            self.assertEqual(cap.returncode, 0)
            # The prelude pins \tracingstats=0, so no memory block here:
            # accounting is the Output line plus the PDF statistics block.
            self.assertFalse(any(ln.startswith("Here is how much of TeX's "
                                               "memory you used:")
                                 for ln in cap.accounting))
            self.assertIn("PDF statistics:", cap.accounting)
            out_lines = [ln for ln in cap.accounting
                         if ln.startswith("Output written on")]
            self.assertEqual(len(out_lines), 1)
            self.assertRegex(out_lines[0], r"\(1 page, \d+ bytes\)\.$")
            self.assertIn(out_lines[0], cap.log.splitlines())
            kept, _ = lockstep_run.split_accounting(cap.log.splitlines())
            self.assertIn("Output written on %s.pdf (1 page, <BYTES> bytes)."
                          % self.CASE, kept)
        finally:
            shutil.rmtree(workdir, ignore_errors=True)

    def test_capture_real_stats_block(self):
        if shutil.which("pdftex") is None:
            self.skipTest("reference engine pdftex not on PATH")
        workdir = tempfile.mkdtemp(prefix="lockstep-stats-")
        try:
            shutil.copy(lockstep_run.PRELUDE,
                        os.path.join(workdir, "prelude.tex"))
            tex = os.path.join(workdir, "stats.tex")
            with open(tex, "w") as fh:
                fh.write("\\input prelude\n\\tracingstats=2\n"
                         "\\setbox0=\\hbox{a}\\lsshipbox0\n\\end\n")
            cap = lockstep_run.capture(tex, "pdftex", workdir)
            self.assertEqual(cap.returncode, 0)
            self.assertTrue(any(ln.startswith("Memory usage before:")
                                for ln in cap.accounting))
            self.assertTrue(any(ln.startswith("Here is how much of TeX's "
                                              "memory you used:")
                                for ln in cap.accounting))

            def mutate(old, new):
                lines = list(cap.log.splitlines())
                self.assertIn(old, lines)
                return [new if ln == old else ln for ln in lines]

            lines = cap.log.splitlines()
            usage = next(ln for ln in lines
                         if ln.startswith("Memory usage before:"))
            mem_body = next(ln for ln in lines
                            if "strings out of" in ln)
            cand_usage = mutate(usage, usage + "0")
            self.assertEqual(lockstep_run.compared_lines(cap.log),
                             lockstep_run.compared_lines(
                                 "\n".join(cand_usage) + "\n"))
            self.assertEqual(lockstep_run.accounting_diff_kinds(
                cap.accounting, lockstep_run.split_accounting(
                    cand_usage)[1]), ["memory usage"])
            cand_mem = mutate(mem_body, mem_body.replace(" out of ",
                                                         " out of 1", 1))
            self.assertEqual(lockstep_run.compared_lines(cap.log),
                             lockstep_run.compared_lines(
                                 "\n".join(cand_mem) + "\n"))
            self.assertEqual(lockstep_run.accounting_diff_kinds(
                cap.accounting, lockstep_run.split_accounting(
                    cand_mem)[1]), ["memory usage"])
        finally:
            shutil.rmtree(workdir, ignore_errors=True)


if __name__ == "__main__":
    unittest.main()
