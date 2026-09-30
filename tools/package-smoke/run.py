#!/usr/bin/env python3
"""Compare the candidate engine with pdfTeX on the package-smoke documents. MIT.

For every tools/package-smoke/*.tex (or the packages named on the command line)
the document is compiled twice, in the same directory, by the candidate and by
the reference, each in its own fresh directory, through the lockstep capture()
with format pdflatex. A document is `equal` only when, on BOTH passes:

  * the exit codes agree;
  * the compared transcripts agree line for line, including the per-shipout box
    dumps (a tracing prelude sets \\tracingoutput and \\showbox limits to the
    maximum, so a changed box, glue or kern shows up) and the trailing accounting
    placeholders of the lockstep comparison;
  * both runs wrote a PDF that passes the lockstep integrity check (a run that
    writes no PDF, or a truncated one, FAILS);
  * the PDFs are equal after `qpdf --qdf --object-streams=disable` with the
    file-path-dependent second half of the trailer /ID removed (P-T2). If qpdf is
    missing or cannot read a file, the raw bytes are compared instead.

The reference must be pdfTeX 1.40.29 (lockstep's pinned check); anything else is a
harness error. A missing or hanging candidate counts as DIFFERENT for that
document, never as a traceback. Standard library only.

  FLASHTEX_POOL=... FLASHTEX_FORMATS=... \\
  python3 tools/package-smoke/run.py --candidate BIN [--reference pdftex] [PKG ...]

Exit status: 0 every document equal; 1 at least one differs; 2 harness or usage
error (unknown package name, empty selection, wrong reference version).
"""
import argparse
import hashlib
import importlib.util
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
_spec = importlib.util.spec_from_file_location(
    "lockstep_run", os.path.join(HERE, "..", "lockstep", "run.py"))
lockstep = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(lockstep)

CANDIDATE_ENV = ("FLASHTEX_POOL", "FLASHTEX_FORMATS")
DEFAULT_REFERENCE = "/Library/TeX/texbin/pdftex"
MAX_LOG_BYTES = 64 * 1024 * 1024
# doc.tex: turn the box-dump tracing to the maximum, then read the real document.
WRAPPER = ("\\showboxbreadth=\\maxdimen \\showboxdepth=\\maxdimen "
           "\\tracingonline=0 \\tracingoutput=1\n\\input{smoke-doc}\n")
ID_RE = re.compile(rb"^(\s*/ID\s*\[\s*<[0-9a-fA-F]*>)\s*<[0-9a-fA-F]*>\s*\]", re.M)


class HarnessError(Exception):
    """Something wrong with the setup, not with a document (exit 2)."""


def pdf_signature(path):
    """qpdf --qdf view of a PDF with the path-dependent /ID half removed."""
    try:
        proc = subprocess.run(
            ["qpdf", "--qdf", "--object-streams=disable", "--normalize-content=n",
             path, "-"], capture_output=True, timeout=120)
        data = proc.stdout if proc.returncode in (0, 3) and proc.stdout else None
    except (OSError, subprocess.SubprocessError):
        data = None
    if data is None:
        with open(path, "rb") as fh:
            data = fh.read()
    data = ID_RE.sub(rb"\1 <ID>]", data)
    return hashlib.sha256(data).hexdigest()


def run_passes(tex, binary, extra, passes, timeout, *, is_reference):
    """Run one engine `passes` times in one fresh dir; list of per-pass dicts."""
    work = tempfile.mkdtemp(prefix="pkgsmoke-")
    results = []
    try:
        shutil.copy(tex, os.path.join(work, "smoke-doc.tex"))
        with open(os.path.join(work, "doc.tex"), "w") as fh:
            fh.write(WRAPPER)
        for _ in range(passes):
            try:
                cap = lockstep.capture(
                    os.path.join(work, "doc.tex"), binary, work, fmt="pdflatex",
                    extra_env=extra, timeout=timeout,
                    require_reference_version=is_reference)
            except FileNotFoundError:
                return [{"error": "engine binary not found: %s" % binary}]
            except subprocess.TimeoutExpired:
                return [{"error": "timed out after %ss" % timeout}]
            except RuntimeError as exc:
                if is_reference:
                    raise HarnessError(str(exc)) from exc
                return [{"error": "engine refused: %s" % exc}]
            except OSError as exc:
                return [{"error": "could not run engine: %s" % exc}]
            if len(cap.log) > MAX_LOG_BYTES:
                return [{"error": "transcript larger than %d MiB" % (MAX_LOG_BYTES >> 20)}]
            entry = {"returncode": cap.returncode,
                     "lines": lockstep.compared_lines(cap.log),
                     "boxes": cap.boxes, "pdf": None, "error": None}
            problem = lockstep.output_integrity_error(cap.log, work)
            if cap.returncode == 0 and (cap.pdf_path is None or problem):
                entry["error"] = problem or "no PDF written"
            elif cap.pdf_path is not None:
                entry["pdf"] = pdf_signature(cap.pdf_path)
            results.append(entry)
        return results
    finally:
        shutil.rmtree(work, ignore_errors=True)


def first_difference(a, b):
    for i, (x, y) in enumerate(zip(a, b)):
        if x != y:
            return i + 1
    return min(len(a), len(b)) + 1 if len(a) != len(b) else None


def compare(cand, ref):
    """None when equal, else a one-line reason."""
    for who, res in (("candidate", cand), ("reference", ref)):
        if len(res) == 1 and res[0].get("error") and "returncode" not in res[0]:
            return "%s: %s" % (who, res[0]["error"])
    for n, (c, r) in enumerate(zip(cand, ref), 1):
        if c["error"] or r["error"]:
            return "pass %d: %s" % (n, "candidate: " + c["error"] if c["error"]
                                    else "reference: " + r["error"])
        if r["returncode"] != 0:
            return "pass %d: the reference itself does not compile the document (exit %s)" % (
                n, r["returncode"])
        if c["returncode"] != r["returncode"]:
            return "pass %d: exit candidate=%s reference=%s" % (
                n, c["returncode"], r["returncode"])
        line = first_difference(c["lines"], r["lines"])
        if line is not None:
            return "pass %d: transcript differs at compared line %d" % (n, line)
        box = first_difference(c["boxes"], r["boxes"])
        if box is not None:
            return "pass %d: shipped box %d differs" % (n, box)
        if c["pdf"] != r["pdf"]:
            return "pass %d: PDF differs after qpdf normalisation (P-T2)" % n
    return None


def select(names):
    available = sorted(f[:-4] for f in os.listdir(HERE) if f.endswith(".tex"))
    if not names:
        chosen = available
    else:
        unknown = [n for n in names if n not in available]
        if unknown:
            raise HarnessError("unknown package document(s): %s" % ", ".join(unknown))
        chosen = [n for n in available if n in names]
    if not chosen:
        raise HarnessError("no documents selected")
    return chosen


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--candidate", required=True, help="candidate engine binary")
    ap.add_argument("--reference", default=DEFAULT_REFERENCE,
                    help="reference pdfTeX (must be 1.40.29)")
    ap.add_argument("--passes", type=int, default=2, help="passes per document")
    ap.add_argument("--timeout", type=float, default=300.0, help="seconds per run")
    ap.add_argument("packages", nargs="*", help="document names (default: all)")
    args = ap.parse_args(argv)
    try:
        if args.passes < 1:
            raise HarnessError("--passes must be at least 1")
        names = select(args.packages)
        cand_env = {k: os.environ[k] for k in CANDIDATE_ENV if k in os.environ} or None
        bad = 0
        for name in names:
            tex = os.path.join(HERE, name + ".tex")
            cand = run_passes(tex, args.candidate, cand_env, args.passes,
                              args.timeout, is_reference=False)
            ref = run_passes(tex, args.reference, None, args.passes,
                             args.timeout, is_reference=True)
            reason = compare(cand, ref)
            if reason is None:
                print("%-16s equal" % name, flush=True)
            else:
                bad += 1
                print("%-16s DIFFERENT (%s)" % (name, reason), flush=True)
    except HarnessError as exc:
        print("error: %s" % exc, file=sys.stderr)
        return 2
    print("%d documents, %d differ" % (len(names), bad))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
