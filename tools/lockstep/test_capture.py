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

    def test_shell_escape_off_by_default(self):
        # DESIGN §4.5: ENGINE_SHELL_FLAGS is the one place to change it;
        # with it the " restricted \write18 enabled." status line is gone.
        self.assertEqual(lockstep_run.ENGINE_SHELL_FLAGS,
                         ["-no-shell-escape"])
        if shutil.which("pdftex") is None:
            self.skipTest("reference engine pdftex not on PATH")
        workdir = tempfile.mkdtemp(prefix="lockstep-test-shell-")
        try:
            shutil.copy(lockstep_run.PRELUDE,
                        os.path.join(workdir, "prelude.tex"))
            tex = os.path.join(workdir, "001-edef-basic.tex")
            shutil.copy(os.path.join(lockstep_run.CASES_DIR,
                                     "001-edef-basic.tex"), tex)
            cap = lockstep_run.capture(tex, "pdftex", workdir)
            self.assertEqual(cap.returncode, 0)
            self.assertNotIn("restricted \\write18 enabled.", cap.log)
        finally:
            shutil.rmtree(workdir, ignore_errors=True)

    def test_engine_runs_through_pdftex_symlink(self):
        # Both engines are executed as a symlink literally named
        # "pdftex" (argv[0] prints identically); kpathsea resolves the
        # link, so the reference still finds its configuration.
        pdftex = shutil.which("pdftex")
        if pdftex is None:
            self.skipTest("reference engine pdftex not on PATH")
        workdir = tempfile.mkdtemp(prefix="lockstep-test-link-")
        try:
            link = lockstep_run.engine_link(pdftex, workdir)
            self.assertEqual(os.path.basename(link), "pdftex")
            self.assertTrue(os.path.islink(link))
            self.assertEqual(os.path.realpath(link),
                             os.path.realpath(pdftex))
            other = os.path.join(workdir, "other-engine")
            with open(other, "w") as fh:
                fh.write("#!/bin/sh\nexit 0\n")
            link2 = lockstep_run.engine_link(other, workdir)
            self.assertEqual(os.path.basename(link2), "pdftex")
            self.assertNotEqual(os.path.dirname(link),
                                os.path.dirname(link2))
            # A repeated call reuses the same link without touching
            # caller files.
            sentinel = os.path.join(workdir, "sentinel.txt")
            with open(sentinel, "w") as fh:
                fh.write("caller-owned\n")
            self.assertEqual(lockstep_run.engine_link(pdftex, workdir),
                             link)
            self.assertTrue(os.path.isfile(sentinel))
        finally:
            shutil.rmtree(workdir, ignore_errors=True)

    def test_font_list_line_after_memory_block_stays_compared(self):
        # Real log with a font-list "<...pfb>" line after the memory
        # block: every memory/PDF shape is matched, the font line is
        # NOT in the accounting list.
        if shutil.which("pdftex") is None:
            self.skipTest("reference engine pdftex not on PATH")
        workdir = tempfile.mkdtemp(prefix="lockstep-test-fontlist-")
        try:
            shutil.copy(lockstep_run.PRELUDE,
                        os.path.join(workdir, "prelude.tex"))
            tex = os.path.join(workdir, "fontlist.tex")
            with open(tex, "w") as fh:
                fh.write("\\input prelude\n\\tracingstats=2\n"
                         "\\font\\a=cmr10\n\\hyphenation{hy-phen-ation}\n"
                         "\\a Hi\n"
                         "\\setbox0=\\hbox{\\a x}\\lsshipbox0\n\\end\n")
            cap = lockstep_run.capture(tex, "pdftex", workdir)
            self.assertEqual(cap.returncode, 0)
            lines = cap.log.splitlines()
            font_lines = [ln for ln in lines if ".pfb>" in ln]
            self.assertTrue(font_lines)
            kept, acc = lockstep_run.split_accounting(lines)
            for ln in font_lines:
                self.assertIn(ln, kept)
                self.assertNotIn(ln, acc)
            self.assertIn("Here is how much of TeX's memory you used:",
                          acc)
            self.assertIn("PDF statistics:", acc)
            hits = [sum(1 for ln in acc if r.match(ln))
                    for r in lockstep_run.MEMORY_BODY_RES]
            self.assertEqual(hits, [1, 1, 1, 1, 1, 1, 1])
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
    # SAMPLE mirrors a real pdfTeX 1.40.29 trailer: the Memory usage line
    # comes after its shipout (one is owed per shipout), then the memory
    # block, the Output line, then the PDF-statistics block.
    SAMPLE = [
        "BANNER",
        "entering extended mode",
        "{\\tracingassigns}",
        ".\\glue 10.0",
        "Completed box being shipped out [0]",
        "\\hbox(0.0+0.0)x0.0",
        "Memory usage before: 29&45; after: 20&45; still untouched: 4998918",
        "Here is how much of TeX's memory you used:",
        " 12 strings out of 497895",
        " 2i,0n,1p,153b,9s stack positions out of "
        "10000i,1000n,20000p,200000b,200000s",
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
            "Completed box being shipped out [0]",
            "\\hbox(0.0+0.0)x0.0",
            "",
            "Output written on foo.pdf (2 pages, <BYTES> bytes).",
            "",
            "tail line",
        ])
        self.assertEqual(acc, [
            "Memory usage before: 29&45; after: 20&45; still untouched: 4998918",
            "Here is how much of TeX's memory you used:",
            " 12 strings out of 497895",
            " 2i,0n,1p,153b,9s stack positions out of "
            "10000i,1000n,20000p,200000b,200000s",
            "Output written on foo.pdf (2 pages, 1500 bytes).",
            "PDF statistics:",
            " 6 PDF objects out of 1000 (max. 8388607)",
        ])

    def test_space_junk_after_block_stays_compared(self):
        for header in ("Here is how much of TeX's memory you used:",
                       "PDF statistics:"):
            lines = list(self.SAMPLE)
            i = next(idx for idx, ln in enumerate(lines)
                     if ln == header)
            j = i + 1
            while j < len(lines) and any(
                    r.match(lines[j]) for r in (
                        lockstep_run.MEMORY_BODY_RES
                        if header.startswith("Here") else
                        lockstep_run.PDF_BODY_RES)):
                j += 1
            for payload in (" junk",
                            " Overfull \\hbox (1.0pt too wide) in "
                            "paragraph at lines 6--6"):
                cand = lines[:j] + [payload] + lines[j:]
                self.assertNotEqual(
                    lockstep_run.compared_lines("\n".join(lines) + "\n"),
                    lockstep_run.compared_lines("\n".join(cand) + "\n"),
                    msg=(header, payload))

    def test_midlog_header_stays_compared(self):
        lines = list(self.SAMPLE)
        cand = lines[:2] + ["PDF statistics:", " hidden"] + lines[2:]
        self.assertNotEqual(
            lockstep_run.compared_lines("\n".join(lines) + "\n"),
            lockstep_run.compared_lines("\n".join(cand) + "\n"))

    def test_dropped_header_body_stays_compared(self):
        lines = [ln for ln in self.SAMPLE
                 if ln != "Here is how much of TeX's memory you used:"]
        self.assertNotEqual(
            lockstep_run.compared_lines("\n".join(self.SAMPLE) + "\n"),
            lockstep_run.compared_lines("\n".join(lines) + "\n"))

    def test_malformed_usage_stays_compared(self):
        lines = list(self.SAMPLE)
        usage = next(ln for ln in lines
                     if ln.startswith("Memory usage before:"))
        bad = usage + "0 junk"
        cand = [bad if ln == usage else ln for ln in lines]
        self.assertNotEqual(
            lockstep_run.compared_lines("\n".join(lines) + "\n"),
            lockstep_run.compared_lines("\n".join(cand) + "\n"))

    def test_usage_before_shipout_stays_compared(self):
        # A Memory usage line counts only when an earlier shipout owes
        # it; moved before the shipout it stays compared (same rule as
        # tools/parity).
        lines = list(self.SAMPLE)
        usage = next(ln for ln in lines
                     if ln.startswith("Memory usage before:"))
        rest = [ln for ln in lines if ln != usage]
        ship = next(i for i, ln in enumerate(rest)
                    if "Completed box being shipped out" in ln)
        cand = rest[:ship] + [usage] + rest[ship:]
        self.assertNotEqual(
            lockstep_run.compared_lines("\n".join(lines) + "\n"),
            lockstep_run.compared_lines("\n".join(cand) + "\n"))
        kept, acc = lockstep_run.split_accounting(cand)
        self.assertIn(usage, kept)
        self.assertNotIn(usage, acc)

    def test_out_of_order_body_stays_compared(self):
        # Block shapes must come in pdfTeX's order: swapping two body
        # lines leaves the second one compared (same as tools/parity).
        lines = list(self.SAMPLE)
        i = next(idx for idx, ln in enumerate(lines)
                 if ln == "Here is how much of TeX's memory you used:")
        cand = list(lines)
        cand[i + 1], cand[i + 2] = cand[i + 2], cand[i + 1]
        self.assertNotEqual(
            lockstep_run.compared_lines("\n".join(lines) + "\n"),
            lockstep_run.compared_lines("\n".join(cand) + "\n"))

    def test_repeated_body_line_stays_compared(self):
        # Each shape matches at most once: repeating a body line leaves
        # the copy compared (same as tools/parity).
        lines = list(self.SAMPLE)
        i = next(idx for idx, ln in enumerate(lines)
                 if ln == "Here is how much of TeX's memory you used:")
        cand = lines[:i + 2] + [lines[i + 1]] + lines[i + 2:]
        self.assertNotEqual(
            lockstep_run.compared_lines("\n".join(lines) + "\n"),
            lockstep_run.compared_lines("\n".join(cand) + "\n"))

    def test_singular_page_byte_count_replaced(self):
        kept, acc = lockstep_run.split_accounting(
            ["Output written on f.pdf (1 page, 950 bytes)."])
        self.assertEqual(
            kept, ["Output written on f.pdf (1 page, <BYTES> bytes)."])
        self.assertEqual(acc, ["Output written on f.pdf (1 page, 950 bytes)."])

    # Real trailer samples, copied verbatim from pdfTeX 1.40.29 runs:
    # SING is an -ini -etex \tracingstats=2 document loading one font
    # (singular "1 font" / "1 hyphenation exception", no object stream);
    # ART is a -fmt=pdflatex article ("40 fonts", "1141 hyphenation
    # exceptions", "7 compressed objects within 1 object stream"). The
    # split asserted here is the same split tools/parity's
    # split_accounting produces for these logs (cross-checked on the
    # full logs plus junk/Overfull/mid-header/dropped-header tweaks:
    # 43 inputs, 0 disagreements).
    SING_TRAILER = [
        "BANNER",
        "Completed box being shipped out [0]",
        "\\hbox(0.0+0.0)x0.0",
        "Memory usage before: 88&50; after: 32&48; still untouched: 4998912",
        "Here is how much of TeX's memory you used:",
        " 15 strings out of 497895",
        " 215 string characters out of 6213314",
        " 1088 words of memory out of 5000000",
        " 554 multiletter control sequences out of 15000+600000",
        " 307 words of font info for 1 font, out of 8000000 for 9000",
        " 1 hyphenation exception out of 8191",
        " 2i,1n,1p,153b,9s stack positions out of "
        "10000i,1000n,20000p,200000b,200000s",
        "</usr/local/texlive/2026/texmf-dist/fonts/type1/public/"
        "amsfonts/cm/cmr10.pfb>",
        "Output written on sing.pdf (2 pages, 11660 bytes).",
        "PDF statistics:",
        " 13 PDF objects out of 1000 (max. 8388607)",
        " 0 named destinations out of 1000 (max. 500000)",
        " 1 words of extra memory for PDF output out of 10000 "
        "(max. 10000000)",
        "",
    ]
    ART_TRAILER = [
        "BANNER",
        "Completed box being shipped out [1]",
        "\\hbox(6.94444+0.0)x345.0",
        "Here is how much of TeX's memory you used:",
        " 422 strings out of 467525",
        " 7858 string characters out of 5418982",
        " 433756 words of memory out of 5000000",
        " 29416 multiletter control sequences out of 15000+600000",
        " 627721 words of font info for 40 fonts, out of 8000000 for 9000",
        " 1141 hyphenation exceptions out of 8191",
        " 35i,5n,38p,144b,126s stack positions out of "
        "10000i,1000n,20000p,200000b,200000s",
        "</usr/local/texlive/2026/texmf-dist/fonts/type1/public/"
        "amsfonts/cm/cmr10.pfb>",
        "Output written on art.pdf (1 page, 12680 bytes).",
        "PDF statistics:",
        " 13 PDF objects out of 1000 (max. 8388607)",
        " 7 compressed objects within 1 object stream",
        " 0 named destinations out of 1000 (max. 500000)",
        " 1 words of extra memory for PDF output out of 10000 "
        "(max. 10000000)",
        "",
    ]

    def test_real_trailer_samples_split(self):
        for sample in (self.SING_TRAILER, self.ART_TRAILER):
            kept, acc = lockstep_run.split_accounting(list(sample))
            font_line = next(ln for ln in sample if ".pfb>" in ln)
            out_line = next(ln for ln in sample
                            if ln.startswith("Output written on"))
            byte_count = out_line[out_line.index(", ") + 2:
                                  out_line.index(" bytes")]
            replaced = out_line.replace(byte_count, "<BYTES>")
            ship = next(ln for ln in sample
                        if "Completed box being shipped out" in ln)
            box = next(ln for ln in sample if ln.startswith("\\hbox"))
            # Exact partition: banner/shipout/box dump, the font-list
            # line and the blank stay compared (Output with bytes
            # replaced); everything else is accounting, in log order.
            self.assertEqual(
                kept, ["BANNER", ship, box, font_line, replaced, ""])
            self.assertEqual(
                acc, [ln for ln in sample if ln not in
                      ("BANNER", ship, box, font_line, "")])
            self.assertIn(out_line, acc)
            # Every shape fires at most once and every real body line
            # matches exactly one shape.
            for header, shapes in (
                    ("Here is how much of TeX's memory you used:",
                     lockstep_run.MEMORY_BODY_RES),
                    ("PDF statistics:", lockstep_run.PDF_BODY_RES)):
                start = acc.index(header) + 1
                end = (acc.index("PDF statistics:")
                       if header.startswith("Here") else len(acc))
                body = [ln for ln in acc[start:end]
                        if not ln.startswith("Output written on")]
                hits = [sum(1 for ln in body if r.match(ln))
                        for r in shapes]
                self.assertTrue(all(h <= 1 for h in hits))
                self.assertEqual(sum(hits), len(body))

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
MEM_HDR = "Here is how much of TeX's memory you used:"
MEM_BLOCK = [MEM_HDR,
             " 12 strings out of 497895",
             " 188 string characters out of 6213314",
             " 1082 words of memory out of 5000000",
             " 554 multiletter control sequences out of 15000+600000",
             " 7 words of font info for 0 fonts, out of 8000000 for 9000",
             " 0 hyphenation exceptions out of 8191",
             " 2i,0n,1p,153b,9s stack positions out of "
             "10000i,1000n,20000p,200000b,200000s"]
def ensure_memory_block(lines):
    if not any(ln.startswith(MEM_HDR) for ln in lines):
        for i, ln in enumerate(lines):
            if ln.startswith("Output written on"):
                lines[i:i] = list(MEM_BLOCK)
                break
    return lines
def insert_after_block(lines, header, payload):
    for i, ln in enumerate(lines):
        if ln.startswith(header):
            j = i + 1
            while j < len(lines) and (lines[j].startswith(" ") or
                                      lines[j].startswith("\t")):
                j += 1
            lines[j:j] = [payload]
            break
    return lines
def tweak_first_body_line(lines, header):
    for i, ln in enumerate(lines):
        if ln.startswith(header):
            lines[i + 1] = re.sub(r"\d+",
                                  lambda m: m.group(0) + "0",
                                  lines[i + 1], count=1)
            break
    return lines
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
    elif MODE == "glue-memory":
        lines = ensure_memory_block(log.split("\n"))
        log = "\n".join(insert_after_block(
            lines, MEM_HDR, "{\\\\glue 3.0}"))
    elif MODE == "glue-pdfstats":
        lines = log.split("\n")
        log = "\n".join(insert_after_block(
            lines, "PDF statistics:", "{\\\\glue 3.0}"))
    elif MODE == "emergency-memory":
        lines = ensure_memory_block(log.split("\n"))
        log = "\n".join(insert_after_block(
            lines, MEM_HDR, "! Emergency stop."))
    elif MODE == "emergency-pdfstats":
        lines = log.split("\n")
        log = "\n".join(insert_after_block(
            lines, "PDF statistics:", "! Emergency stop."))
    elif MODE == "indent-memory":
        lines = ensure_memory_block(log.split("\n"))
        log = "\n".join(tweak_first_body_line(lines, MEM_HDR))
    elif MODE == "indent-pdfstats":
        lines = log.split("\n")
        log = "\n".join(tweak_first_body_line(lines, "PDF statistics:"))
    elif MODE == "junk-memory":
        lines = ensure_memory_block(log.split("\n"))
        log = "\n".join(insert_after_block(lines, MEM_HDR, " junk"))
    elif MODE == "overfull-memory":
        lines = ensure_memory_block(log.split("\n"))
        log = "\n".join(insert_after_block(
            lines, MEM_HDR,
            " Overfull \\hbox (1.0pt too wide) in paragraph at lines 6--6"))
    elif MODE == "junk-pdfstats":
        lines = log.split("\n")
        log = "\n".join(insert_after_block(
            lines, "PDF statistics:", " junk"))
    elif MODE == "overfull-pdfstats":
        lines = log.split("\n")
        log = "\n".join(insert_after_block(
            lines, "PDF statistics:",
            " Overfull \\hbox (1.0pt too wide) in paragraph at lines 6--6"))
    elif MODE == "junk-usage":
        lines = log.split("\n")
        if not any(ln.startswith("Memory usage before:") for ln in lines):
            for i, ln in enumerate(lines):
                if ln.startswith("Output written on"):
                    lines[i:i] = ["Memory usage before: 29&45; after: "
                                  "20&45; still untouched: 4998918"]
                    break
        out = []
        for ln in lines:
            out.append(ln)
            if ln.startswith("Memory usage before:"):
                out.append(" junk")
        log = "\n".join(out)
    elif MODE == "mid-pdfstats":
        lines = log.split("\n")
        lines[2:2] = ["PDF statistics:", " hidden"]
        log = "\n".join(lines)
    elif MODE == "drop-mem-header":
        lines = ensure_memory_block(log.split("\n"))
        lines = [ln for ln in lines if not ln.startswith(MEM_HDR)]
        log = "\n".join(lines)
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
                     "trace", "glue-memory", "glue-pdfstats",
                     "emergency-memory", "emergency-pdfstats",
                     "indent-memory", "indent-pdfstats",
                     "junk-memory", "overfull-memory",
                     "junk-pdfstats", "overfull-pdfstats",
                     "junk-usage", "mid-pdfstats", "drop-mem-header"):
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

    def test_appended_glue_after_memory_block_fails(self):
        self.check_fail("glue-memory")

    def test_appended_glue_after_pdfstats_block_fails(self):
        self.check_fail("glue-pdfstats")

    def test_appended_emergency_after_memory_block_fails(self):
        self.check_fail("emergency-memory")

    def test_appended_emergency_after_pdfstats_block_fails(self):
        self.check_fail("emergency-pdfstats")

    def test_space_junk_after_memory_block_fails(self):
        self.check_fail("junk-memory")

    def test_overfull_after_memory_block_fails(self):
        self.check_fail("overfull-memory")

    def test_space_junk_after_pdfstats_block_fails(self):
        self.check_fail("junk-pdfstats")

    def test_overfull_after_pdfstats_block_fails(self):
        self.check_fail("overfull-pdfstats")

    def test_space_junk_after_usage_line_fails(self):
        self.check_fail("junk-usage")

    def test_midlog_pdfstats_header_fails(self):
        self.check_fail("mid-pdfstats")

    def test_dropped_memory_header_fails(self):
        self.check_fail("drop-mem-header")

    def test_indented_memory_change_passes_with_accounting(self):
        self.check_pass_with_accounting("indent-memory", "memory usage")

    def test_indented_pdfstats_change_passes_with_accounting(self):
        self.check_pass_with_accounting("indent-pdfstats", "pdf stats")

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

    def test_real_blocks_exact_shapes_and_appendage_fails(self):
        # Real pdfTeX output (\tracingstats=2, small \pdfoutput=1
        # document via the prelude): every line of the real memory
        # block and the real PDF-statistics block matches its exact
        # shape, so no real line is left in the compared output. A
        # junk/Overfull line appended after either block stays
        # compared (FAIL); changing a number inside either block stays
        # normalised away (PASS with accounting).
        if shutil.which("pdftex") is None:
            self.skipTest("reference engine pdftex not on PATH")
        workdir = tempfile.mkdtemp(prefix="lockstep-blocks-")
        try:
            shutil.copy(lockstep_run.PRELUDE,
                        os.path.join(workdir, "prelude.tex"))
            tex = os.path.join(workdir, "stats.tex")
            with open(tex, "w") as fh:
                fh.write("\\input prelude\n\\tracingstats=2\n"
                         "\\setbox0=\\hbox{a}\\lsshipbox0\n\\end\n")
            cap = lockstep_run.capture(tex, "pdftex", workdir)
            self.assertEqual(cap.returncode, 0)
            lines = cap.log.splitlines()
            ref_compared = lockstep_run.compared_lines(cap.log)
            cases = (
                ("Here is how much of TeX's memory you used:",
                 lockstep_run.MEMORY_BODY_RES, "memory usage"),
                ("PDF statistics:", lockstep_run.PDF_BODY_RES, "pdf stats"),
            )
            for header, shapes, kind in cases:
                self.assertIn(header, lines)
                i = next(idx for idx, ln in enumerate(lines)
                         if ln == header)
                j = i + 1
                while j < len(lines) and any(
                        r.match(lines[j]) for r in shapes):
                    j += 1
                block = lines[i:j]
                self.assertGreater(len(block), 1, msg=header)
                kept, acc = lockstep_run.split_accounting(list(lines))
                for ln in block:
                    self.assertIn(ln, acc, msg=(header, ln))
                    self.assertNotIn(ln, kept, msg=(header, ln))
                if j < len(lines):
                    self.assertIn(lines[j], kept, msg=(header, lines[j]))
                for payload in ("{\\glue 3.0}", "! Emergency stop.",
                                " junk",
                                " Overfull \\hbox (1.0pt too wide) in "
                                "paragraph at lines 6--6"):
                    cand = lines[:j] + [payload] + lines[j:]
                    self.assertNotEqual(
                        lockstep_run.compared_lines(
                            "\n".join(cand) + "\n"),
                        ref_compared, msg=(header, payload))
                cand = list(lines)
                import re as _re
                cand[i + 1] = _re.sub(r"\d+",
                                      lambda m: m.group(0) + "0",
                                      cand[i + 1], count=1)
                self.assertEqual(lockstep_run.compared_lines(
                    "\n".join(cand) + "\n"), ref_compared, msg=header)
                self.assertEqual(lockstep_run.accounting_diff_kinds(
                    cap.accounting,
                    lockstep_run.split_accounting(cand)[1]), [kind])
        finally:
            shutil.rmtree(workdir, ignore_errors=True)


if __name__ == "__main__":
    unittest.main()
