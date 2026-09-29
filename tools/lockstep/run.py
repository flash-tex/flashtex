#!/usr/bin/env python3
"""Lockstep differential harness: reference pdfTeX vs a candidate engine.

Runs each case in cases/<name>.tex through the reference and the candidate
with identical args and environment (one temp dir per run) and compares the
normalised transcript logs (tracing + box dumps). See README.md. Stdlib only.

The single-run core is capture(), importable by other tools (see README).
"""
import argparse
import dataclasses
import fnmatch
import hashlib
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
import typing

HERE = os.path.dirname(os.path.abspath(__file__))
CASES_DIR = os.path.join(HERE, "cases")
EXPECTED_DIR = os.path.join(HERE, "expected")
PRELUDE = os.path.join(HERE, "prelude.tex")

# Fixed invocation. -cnf-line raises max_print_line/error_line, which are
# texmf.cnf values in web2c pdfTeX, not settable primitives. -etex switches
# both engines to extended mode (log shows "entering extended mode"), which
# is what the prelude's e-TeX tracing switches require.
ENGINE_ARGS = ["-cnf-line=max_print_line = 1000", "-cnf-line=error_line = 254",
               "-ini", "-etex", "-interaction=nonstopmode", "-halt-on-error"]
# DESIGN §4.5 keeps shell escape OFF by default. This is the one place to
# change it: capture() appends these flags for every run (reference and
# candidate, -ini and -fmt modes), so the CLI — which only runs engines
# through capture() — inherits it. Without the flag the log carries
# " restricted \write18 enabled." and \pdfshellescape traces as 2;
# with it the status line is gone and \pdfshellescape is 0 (both verified
# against pdfTeX 1.40.29).
ENGINE_SHELL_FLAGS = ["-no-shell-escape"]
RUN_TIMEOUT = 300
# Seconds between SIGTERM and the unconditional SIGKILL of a run's
# process group, and how long to wait for the stdout reader thread
# before returning with the output collected so far. Same bounded
# handling as tools/latex-suites (its _kill_tree /
# _join_reader_before_close, adapted here so run.py stays stdlib-only
# and self-contained).
KILL_GRACE = 5.0
READER_GRACE = 10.0
# Marker the prelude writes via \message before every \shipout; kept for
# debugging, but capture() no longer uses it for boxes (see split_boxes).
BOX_MARKER_RE = re.compile(r"LOCKSTEP-BOX \d+")
DATE_RE = re.compile(r"\b\d{1,2} (JAN|FEB|MAR|APR|MAY|JUN|JUL|AUG|SEP|OCT|NOV|DEC)"
                     r" \d{4}( \d{2}:\d{2})?\b")
# Real shipout header written by \tracingoutput (same for \shipout,
# \output-driven ships and direct ships). boxes splits on this line,
# independent of any prelude marker.
SHIPOUT_LINE = "Completed box being shipped out"
TRAILER_RE = re.compile(r"^(Here is how much of TeX's memory|Output written on"
                        r"|PDF statistics:)")

# P-T1 accounting normalisation (DESIGN §1.1 ruling 2026-09-29): capacity
# and output-size accounting is normalised in the compared log (matched
# lines are REPLACED by fixed placeholders, so presence, position and
# count stay compared and only the numbers are normalised) and reported
# separately as a non-gating accounting check. Everything else in the
# log stays strict. tools/parity uses this same placeholder rule.
ACCOUNTING_BYTE_TOKEN = "<BYTES>"
MEMORY_USAGE_TOKEN = "Memory usage <ACCOUNTING>"
MEMORY_BLOCK_TOKEN = "<ACCOUNTING memory block>"
PDF_STATS_TOKEN = "<ACCOUNTING pdf statistics>"
# Same "Output written on" shape tools/parity uses: one line, in the
# trailer, ending with "bytes)." — the page count stays compared, the byte
# count becomes <BYTES>. The head is lazy and the byte count is anchored
# to the end of the line, so a file name containing parentheses (e.g.
# 'Output written on "doc (draft).pdf" (1 page, 100 bytes).') keeps its
# name intact and only the trailing byte count is replaced.
OUTPUT_BYTES_RE = re.compile(
    r"^(Output written on .*?\(\d+ pages?), \d+ bytes\)\.$")
MEMORY_USAGE_PREFIX = "Memory usage before:"
MEMORY_BLOCK_HEADER = "Here is how much of TeX's memory you used:"
PDF_STATS_HEADER = "PDF statistics:"
OUTPUT_WRITTEN_PREFIX = "Output written on"
# Exact line shapes (pdfTeX 1.40.29). These are the same shapes
# tools/parity uses (its ACCOUNTING_BLOCKS table, checked against tex.web
# and pdftex.web: plurals follow print_char("s")), adapted here so run.py
# stays stdlib-only and self-contained. Verified against real logs: -ini
# \tracingstats=2 runs (incl. singular "1 font" / "1 hyphenation
# exception"), a pdflatex article (incl. "7 compressed objects within 1
# object stream" and the "<...pfb>" font-list line after the memory
# block), and a rich -ini document (3 fonts, 2 exceptions). Every line of
# every real block is matched; no real line is left over. A line that does
# not match its shape stays in the compared log, so the engines differ.
MEMORY_USAGE_RE = re.compile(
    r"^Memory usage before: \d+&\d+; after: \d+&\d+; "
    r"still untouched: \d+$")
MEMORY_BODY_RES = tuple(re.compile(p) for p in (
    r"^ \d+ strings? out of \d+$",
    r"^ \d+ string characters? out of \d+$",
    r"^ \d+ words of memory out of \d+$",
    r"^ \d+ multiletter control sequences? out of \d+\+\d+$",
    r"^ \d+ words of font info for \d+ fonts?, out of \d+ for \d+$",
    r"^ \d+ hyphenation exceptions? out of \d+$",
    r"^ \d+i,\d+n,\d+p,\d+b,\d+s stack positions out of "
    r"\d+i,\d+n,\d+p,\d+b,\d+s$",
))
PDF_BODY_RES = tuple(re.compile(p) for p in (
    r"^ \d+ PDF objects? out of \d+ \(max\. \d+\)$",
    r"^ \d+ compressed objects? within \d+ object streams?$",
    r"^ \d+ named destinations? out of \d+ \(max\. \d+\)$",
    r"^ \d+ words of extra memory for PDF output out of \d+ "
    r"\(max\. \d+\)$",
))
ACCOUNTING_KIND_MEMORY = "memory usage"
ACCOUNTING_KIND_PDFSTATS = "pdf stats"
ACCOUNTING_KIND_BYTES = "pdf bytes"
ACCOUNTING_KIND_ORDER = (ACCOUNTING_KIND_MEMORY, ACCOUNTING_KIND_PDFSTATS,
                         ACCOUNTING_KIND_BYTES)


def split_boxes(log):
    """Split normalised log into one string per real shipout box dump.

    Each box starts at a log line beginning with SHIPOUT_LINE
    ("Completed box being shipped out", written by \\tracingoutput at
    column 0, so the match is anchored: a trace line merely mentioning
    the text does not start a box) and runs to the next such line or
    the end of the log. Trace text printed between two shipouts (the
    \\lsshipbox/\\message tracing for the next ship, an intermediate
    "Memory usage before:" line) belongs to the preceding box. The
    closing trailer — a "Memory usage before:" line, the memory-usage
    block, "Output written on", "PDF statistics:" — is excluded from
    the last box.
    """
    lines = log.splitlines()
    starts = [i for i, ln in enumerate(lines)
              if ln.startswith(SHIPOUT_LINE)]
    boxes = []
    for k, start in enumerate(starts):
        end = starts[k + 1] if k + 1 < len(starts) else len(lines)
        block = lines[start:end]
        if k == len(starts) - 1:
            cut = next((j for j, ln in enumerate(block)
                        if TRAILER_RE.match(ln)
                        or MEMORY_USAGE_RE.match(ln)), len(block))
            block = block[:cut]
        boxes.append("\n".join(block).strip())
    return boxes


PINNED_REFERENCE_VERSION = "1.40.29"
PINNED_REFERENCE_VERSION_RE = re.compile(r"(?<![\d.])1\.40\.29(?![\d.])")
_reference_version_cache = {}
_warned_version = set()


def _read_all(pipe, chunks):
    """Reader-thread target: append stdout bytes until EOF."""
    try:
        while True:
            data = pipe.read(65536)
            if not data:
                break
            chunks.append(data)
    except Exception:
        pass


def _kill_tree(proc):
    """SIGTERM the run's process group, then ALWAYS SIGKILL it; reap.

    The SIGKILL is unconditional, not only when proc.wait() times out:
    the group leader usually exits on SIGTERM within KILL_GRACE while a
    SIGTERM-ignoring survivor is still alive — without the SIGKILL it
    holds the stdout pipe open forever and the run never returns. With
    start_new_session=True the group id equals proc.pid, so
    killpg(proc.pid, ...) reaches survivors even after the leader has
    exited and been reaped. ProcessLookupError (group already empty,
    the common case) is ignored. A setsid-detached grandchild (its own
    session) is NOT in our group: it cannot be killed here and is only
    reaped by the OS; _join_reader_before_close bounds the wait for
    its pipe.
    """
    try:
        if hasattr(os, "killpg"):
            os.killpg(proc.pid, signal.SIGTERM)
        else:
            proc.terminate()
    except OSError:
        pass
    try:
        proc.wait(timeout=KILL_GRACE)
    except subprocess.TimeoutExpired:
        pass
    try:
        if hasattr(os, "killpg"):
            os.killpg(proc.pid, signal.SIGKILL)
        else:
            proc.kill()
    except OSError:
        pass
    proc.wait()


def _join_reader_before_close(proc, reader, grace=READER_GRACE):
    """Bounded reader join; close our pipe end only if the reader is done.

    Never close proc.stdout while the _read_all thread can still be
    blocked in read(): the close deadlocks against it for as long as
    any live process holds the pipe open. After _kill_tree, in-group
    survivors are dead so the reader reaches EOF promptly and the
    close is safe; if the reader is still alive after `grace` seconds,
    a setsid-detached grandchild (unkillable by our process-group
    kill, reaped only by the OS) still holds the pipe — leave our end
    open and return with the output collected so far instead of
    hanging.
    """
    reader.join(timeout=grace)
    if not reader.is_alive():
        try:
            proc.stdout.close()
        except OSError:
            pass


def _run_isolated(argv, cwd, env, timeout):
    """Run argv to completion in its own process group.

    Returns (returncode, stdout_bytes); stderr is merged into stdout
    and stdin is DEVNULL. Raises subprocess.TimeoutExpired on timeout
    (after killing the whole group) and FileNotFoundError when the
    binary is missing. After every run — completion or timeout — the
    whole process group is SIGKILLed (ProcessLookupError ignored), so
    a detached same-group child (e.g. a wrapper's leftover `sleep`)
    never outlives the gate. Return is bounded by
    timeout + KILL_GRACE + READER_GRACE even when a survivor holds
    the pipe.
    """
    proc = subprocess.Popen(argv, cwd=cwd, env=env,
                            stdin=subprocess.DEVNULL,
                            stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT,
                            start_new_session=True)
    chunks = []
    reader = threading.Thread(target=_read_all, args=(proc.stdout, chunks),
                              daemon=True)
    reader.start()
    try:
        proc.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        _kill_tree(proc)
        _join_reader_before_close(proc, reader)
        raise
    _kill_tree(proc)
    _join_reader_before_close(proc, reader)
    return proc.returncode, b"".join(chunks)


def reference_version_first_line(binary):
    """First line of `<binary> --version`, cached per binary path."""
    if binary not in _reference_version_cache:
        _, data = _run_isolated([binary, "--version"], cwd=None, env=None,
                                timeout=RUN_TIMEOUT)
        text = data.decode("utf-8", "replace")
        lines = text.splitlines()
        _reference_version_cache[binary] = lines[0] if lines else ""
    return _reference_version_cache[binary]


def check_reference_version(binary, *, allow_any=False):
    """Require the pinned reference version; refuse otherwise.

    Checks that the first line of `<binary> --version` contains the
    whole version token PINNED_REFERENCE_VERSION (matched with
    PINNED_REFERENCE_VERSION_RE, so "1.40.290" or "11.40.29" do not
    match). Returns the first line when it matches. With
    allow_any=True, prints a warning and returns the line instead of
    raising. Raises RuntimeError on mismatch (or when --version
    itself fails) so the CLI can exit 2 and capture() can refuse.
    """
    try:
        first = reference_version_first_line(binary)
    except (OSError, subprocess.SubprocessError) as exc:
        if allow_any:
            if binary not in _warned_version:
                print("warning: could not get --version from %r (%s); "
                      "proceeding with --allow-any-reference" % (binary, exc),
                      file=sys.stderr)
                _warned_version.add(binary)
            return ""
        raise RuntimeError("reference %r --version failed: %s "
                           "(expected pdfTeX %s)" %
                           (binary, exc, PINNED_REFERENCE_VERSION)) from exc
    if PINNED_REFERENCE_VERSION_RE.search(first):
        return first
    if allow_any:
        if binary not in _warned_version:
            print("warning: reference %r version %r is not the pinned %s; "
                  "proceeding with --allow-any-reference" %
                  (binary, first, PINNED_REFERENCE_VERSION), file=sys.stderr)
            _warned_version.add(binary)
        return first
    raise RuntimeError(
        "reference %r version mismatch: first --version line %r does not "
        "contain pinned %s (use --allow-any-reference to override)" %
        (binary, first, PINNED_REFERENCE_VERSION))


def pinned_env():
    env = dict(os.environ)
    env.update(SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1", TZ="UTC")
    return env


def engine_link(engine_bin, workdir):
    """Path to execute for engine_bin: <workdir>/.lockstep-bin-<hash>/pdftex.

    Warnings print argv[0], and the invoked name even selects the format
    outside -ini mode ("preloaded format=<name>", mktexfmt), so reference
    and candidate must be invoked through paths that print identically:
    every run executes a symlink literally named "pdftex" pointing at the
    resolved engine binary. The directory is per-engine (keyed by the
    resolved target), so repeated capture() calls in one workdir with
    different engines do not clash. Only a symlink this function created
    is ever replaced; anything else at that path is left alone and the
    caller falls back to engine_bin. Returns the path to execute.
    """
    resolved = engine_bin
    if (not os.path.isabs(resolved) and os.path.dirname(resolved) == ""):
        # A bare name ("pdftex") resolves via PATH, as it would when
        # executed directly; fall back to the name itself (a dangling
        # link, so exec still raises FileNotFoundError) when missing.
        resolved = shutil.which(resolved) or resolved
    target = os.path.realpath(resolved)
    digest = hashlib.sha1(target.encode("utf-8")).hexdigest()[:12]
    bindir = os.path.join(workdir, ".lockstep-bin-" + digest)
    os.makedirs(bindir, exist_ok=True)
    link = os.path.join(bindir, "pdftex")
    if os.path.islink(link):
        os.remove(link)
    if not os.path.lexists(link):
        os.symlink(target, link)
    if os.path.islink(link):
        return link
    return engine_bin


def normalise(text, tmpdir):
    """Strip only what legitimately differs: temp paths, banner, dates.

    Byte-exact otherwise: the text is split on "\\n" only (never
    splitlines()) and no trailing newline is forced, so CR bytes and
    the presence or absence of the final newline survive into the
    compared log — a candidate that writes CRLF line endings or drops
    the final newline compares different.
    """
    lines = text.replace(tmpdir, "<TMP>").split("\n")
    if lines and lines[0].startswith("This is "):
        lines[0] = "BANNER"
    return "\n".join(DATE_RE.sub("<DATE>", ln) for ln in lines)


def _split_accounting(lines):
    """Core split: (compared lines, [(kind, original line), ...]).

    Replaces exactly the §1.1 accounting lines by fixed placeholders
    under the same rule tools/parity uses (its split_accounting,
    adapted here so run.py stays stdlib-only and self-contained):

    * each matched "Memory usage before:" line becomes
      MEMORY_USAGE_TOKEN ("Memory usage <ACCOUNTING>");
    * the memory block (header plus its matched body lines) becomes one
      MEMORY_BLOCK_TOKEN ("<ACCOUNTING memory block>") placed where the
      header was; the PDF-statistics block likewise becomes one
      PDF_STATS_TOKEN ("<ACCOUNTING pdf statistics>");
    * a block's body lines must come in pdfTeX's order, each shape at most
      once — shapes may be missing, but the first line that fits no
      remaining shape ends the block and is compared, even if it starts
      with a space (" junk", " Overfull \\hbox");
    * each block header, and "Output written on", counts only once, and
      only in the end-of-run trailer after the last SHIPOUT_LINE. A header
      anywhere else (mid-log injection, duplicate) stays compared. (In
      real logs the trailer reads: memory block, font-list "<...pfb>"
      lines, "Output written on", "PDF statistics:"; "Transcript written
      on" goes to stdout, never into the .log capture() reads.)
    * a "Memory usage before:" line counts only when an earlier shipout
      still owes its one line — real logs print one per shipout, right
      after its box dump (a 2-shipout log has 2, each after its shipout).
      With no shipout (a -fmt run without \\tracingoutput) no line is
      owed, so every "Memory usage before:" line stays compared. A
      shipout is a line BEGINNING with SHIPOUT_LINE (the same anchored
      rule split_boxes uses): a trace line merely mentioning the text
      mid-line neither ends the trailer search nor owes a usage line,
      so a forged "Memory usage before:" line placed before any real
      shipout stays compared even when such a mention sits nearby.

    The byte count in "Output written on … (N pages, B bytes)." becomes
    <BYTES>; the page count stays compared. Any line that does not match
    its shape stays in the compared log, so the engines differ. The
    original removed lines are still returned as accounting (non-gating).
    """
    last_ship = max((i for i, ln in enumerate(lines)
                     if ln.startswith(SHIPOUT_LINE)), default=-1)
    kept, accounting = [], []
    seen = set()  # trailer items already consumed (headers, Output written)
    block, pos, kind = None, 0, None  # current block's shapes, next allowed
    mem_owed = 0  # shipouts not yet followed by their Memory usage line
    for i, ln in enumerate(lines):
        if block is not None:
            j = next((k for k in range(pos, len(block))
                      if block[k].match(ln)), None)
            if j is not None:
                accounting.append((kind, ln))
                pos = j + 1
                continue
            block = None
        if ln.startswith(SHIPOUT_LINE):
            mem_owed += 1
        trailer = i > last_ship
        if trailer and ln == MEMORY_BLOCK_HEADER and ln not in seen:
            seen.add(ln)
            accounting.append((ACCOUNTING_KIND_MEMORY, ln))
            kept.append(MEMORY_BLOCK_TOKEN)
            block, pos, kind = MEMORY_BODY_RES, 0, ACCOUNTING_KIND_MEMORY
            continue
        if trailer and ln == PDF_STATS_HEADER and ln not in seen:
            seen.add(ln)
            accounting.append((ACCOUNTING_KIND_PDFSTATS, ln))
            kept.append(PDF_STATS_TOKEN)
            block, pos, kind = PDF_BODY_RES, 0, ACCOUNTING_KIND_PDFSTATS
            continue
        if mem_owed and MEMORY_USAGE_RE.match(ln):
            mem_owed -= 1
            accounting.append((ACCOUNTING_KIND_MEMORY, ln))
            kept.append(MEMORY_USAGE_TOKEN)
            continue
        m = (OUTPUT_BYTES_RE.match(ln)
             if trailer and OUTPUT_WRITTEN_PREFIX not in seen else None)
        if m:
            seen.add(OUTPUT_WRITTEN_PREFIX)
            accounting.append((ACCOUNTING_KIND_BYTES, ln))
            kept.append(m.group(1) + ", %s bytes)." % ACCOUNTING_BYTE_TOKEN)
            continue
        kept.append(ln)
    return kept, accounting


def split_accounting(lines):
    """Split normalised log lines into (compared, accounting) line lists.

    Compared lines are what the PASS/FAIL check uses (matched accounting
    lines replaced by fixed placeholders); accounting holds the removed
    original lines in log order, before replacing (so the "Output
    written on" entry keeps the real byte count).
    """
    kept, kinded = _split_accounting(lines)
    return kept, [ln for _, ln in kinded]


def compared_lines(log):
    """Compared view of a normalised log: accounting replaced by placeholders.

    Split on "\\n" only (never splitlines()) so CR bytes and the final
    newline stay compared, matching normalise().
    """
    kept, _ = split_accounting(log.split("\n"))
    return kept


def _accounting_kind(line, current):
    if (line.startswith(MEMORY_USAGE_PREFIX) or
            line.startswith(MEMORY_BLOCK_HEADER)):
        return ACCOUNTING_KIND_MEMORY
    if line.startswith(PDF_STATS_HEADER):
        return ACCOUNTING_KIND_PDFSTATS
    if line.startswith(OUTPUT_WRITTEN_PREFIX):
        return ACCOUNTING_KIND_BYTES
    return current if current is not None else ACCOUNTING_KIND_MEMORY


def accounting_diff_kinds(ref_accounting, cand_accounting):
    """Kind labels whose accounting lines differ, in stable order."""
    groups = {}
    for tag, acc in (("ref", ref_accounting), ("cand", cand_accounting)):
        current = None
        per_kind = {}
        for ln in acc:
            current = _accounting_kind(ln, current)
            per_kind.setdefault(current, []).append(ln)
        groups[tag] = per_kind
    kinds = [k for k in ACCOUNTING_KIND_ORDER
             if k in groups["ref"] or k in groups["cand"]]
    kinds += [k for k in groups["ref"] if k not in kinds]
    kinds += [k for k in groups["cand"] if k not in kinds]
    return [k for k in kinds
            if groups["ref"].get(k, []) != groups["cand"].get(k, [])]


# Same shape as OUTPUT_BYTES_RE, but capturing the named file: the
# head is lazy so a name with parentheses or spaces (TeX quotes such
# names: 'Output written on "doc (draft).pdf" (1 page, 100 bytes).')
# keeps its name intact and only the trailing byte count is anchored.
OUTPUT_NAME_RE = re.compile(
    r"^Output written on (.*?) \(\d+ pages?, \d+ bytes\)\.$")


def log_output_name(log):
    """File the transcript's "Output written on" line names, or None.

    Only a line starting with OUTPUT_WRITTEN_PREFIX counts (anchored
    like split_boxes: a trace line merely mentioning the text does
    not). Surrounding double quotes TeX adds around names with spaces
    are stripped. None means the log claims no output file, or its
    claim has no parseable name.
    """
    for ln in log.splitlines():
        if not ln.startswith(OUTPUT_WRITTEN_PREFIX):
            continue
        m = OUTPUT_NAME_RE.match(ln)
        if not m:
            return None
        name = m.group(1)
        if len(name) >= 2 and name.startswith('"') and name.endswith('"'):
            name = name[1:-1]
        return name or None
    return None


def log_expects_pdf(log):
    """True when the transcript says an output file was written.

    Any log line starting with OUTPUT_WRITTEN_PREFIX ("Output written
    on", anchored like split_boxes: a trace line merely mentioning the
    text does not count) means the engine claims it produced an output
    file (PDF or DVI) next to the log. Kept for callers that only need
    the claim's presence; output_integrity_error() checks the file.
    """
    return any(ln.startswith(OUTPUT_WRITTEN_PREFIX)
               for ln in log.splitlines())


def pdf_integrity_error(pdf_path):
    """None when pdf_path looks like a real engine-produced PDF, else why.

    Structural only: the file must exist, be non-empty, start with
    "%PDF-" and end with "%%EOF" (trailing whitespace allowed), so a
    deleted, truncated or garbage-appended PDF FAILs the case.
    Byte-level PDF equality is another tool's job (tools/parity P-T2),
    not this harness's.
    """
    try:
        with open(pdf_path, "rb") as fh:
            data = fh.read()
    except OSError:
        return "PDF missing: %s" % pdf_path
    if not data:
        return "PDF empty: %s" % pdf_path
    if not data.startswith(b"%PDF-"):
        return "PDF missing %%PDF- header: %s" % pdf_path
    if not data.rstrip(b" \t\r\n\x0b\x0c").endswith(b"%%EOF"):
        return "PDF missing %%EOF trailer: %s" % pdf_path
    return None


def dvi_integrity_error(dvi_path):
    """None when dvi_path looks like a real engine-produced DVI, else why.

    Structural only, mirroring pdf_integrity_error: the file must
    exist, be non-empty, start with the DVI preamble bytes F7 02 and
    end with at least four DF post-postamble (trailer) bytes, so a
    deleted, truncated or garbage-prefixed DVI FAILs the case.
    """
    try:
        with open(dvi_path, "rb") as fh:
            data = fh.read()
    except OSError:
        return "DVI missing: %s" % dvi_path
    if not data:
        return "DVI empty: %s" % dvi_path
    if not data.startswith(b"\xf7\x02"):
        return "DVI missing preamble: %s" % dvi_path
    if len(data) - len(data.rstrip(b"\xdf")) < 4:
        return "DVI missing trailer: %s" % dvi_path
    return None


def output_integrity_error(log, tmpdir):
    """None when the log's named output file passes its check, else why.

    Follows the file the log's "Output written on <file>" line names:
    .pdf gets the PDF check, .dvi the DVI check. Any other extension,
    or a claim with no parseable name, is an error with a clear
    message. None when the log claims no output file at all. A
    relative name resolves next to the log; an absolute one is used
    as is.
    """
    claimed = [ln for ln in log.splitlines()
               if ln.startswith(OUTPUT_WRITTEN_PREFIX)]
    if not claimed:
        return None
    name = log_output_name(log)
    if not name:
        return "output file name missing in: %s" % claimed[0]
    path = name if os.path.isabs(name) else os.path.join(
        tmpdir, os.path.basename(name))
    ext = os.path.splitext(name)[1].lower()
    if ext == ".pdf":
        return pdf_integrity_error(path)
    if ext == ".dvi":
        return dvi_integrity_error(path)
    return "unsupported output extension %r in: %s" % (ext or name,
                                                      claimed[0])


@dataclasses.dataclass
class Capture:
    """One traced engine run: normalised log, per-shipout box dumps, PDF."""

    log: str  # normalised transcript text (always a plain str)
    boxes: typing.List[str]  # one normalised string per shipout box dump
    pdf_path: typing.Optional[str]  # produced PDF, or None if there is none
    returncode: int
    accounting: typing.List[str] = dataclasses.field(default_factory=list)
    # original lines replaced by placeholders in the compared view


def capture(tex_path, engine_bin, workdir, *, fmt=None, extra_env=None,
            allow_any_reference=False, require_reference_version=False,
            timeout=RUN_TIMEOUT):
    """Run one engine once on tex_path and return a Capture.

    Runs with cwd=workdir and never wipes or cleans files already in it:
    tex_path may be a file inside workdir (a caller may stage a source
    tree, run its own convergence passes, then call capture for the one
    traced pass). The transcript is read from <jobname>.log in workdir;
    when the engine wrote no log because it failed to start (nonzero
    exit), the captured stdout is used instead, so a failed run's log is
    never missing; an exit-0 run without its own job.log is an error
    (empty log). Each entry of boxes starts at one
    "Completed box being shipped out" log line and runs to the next such
    line or the end of the log (trailer excluded), independent of any
    prelude marker, so direct \\shipout and \\output ships count too.
    accounting lists the §1.1 original lines the comparison replaces
    by placeholders; log itself stays the full normalised transcript.

    fmt=None keeps the default: -ini (-etex) plain/primitive mode. When
    fmt is given (e.g. fmt="pdflatex"), the engine runs as -fmt=<fmt>
    instead and -ini mode is not used. Every run appends
    ENGINE_SHELL_FLAGS (shell escape off, DESIGN §4.5) and executes the
    engine through a per-engine ".../pdftex" symlink inside workdir (see
    engine_link), so argv[0]-derived log text prints identically for both
    engines; kpathsea resolves the symlink to the real binary. extra_env
    adds environment variables on top of the pinned ones. stdin is
    DEVNULL so a run that accidentally enters \\errorstopmode (e.g. after
    an injected \\tracingall, see README) fails fast on EOF instead of
    blocking. The engine runs in its own process group
    (start_new_session) under `timeout` seconds; afterwards the whole
    group is SIGKILLed (ProcessLookupError ignored), so a detached
    same-group child never outlives the run, and a hang raises
    subprocess.TimeoutExpired only after the group is killed.

    With require_reference_version=True, the pinned reference check runs
    first via check_reference_version() (cached per binary path):
    allow_any_reference=True warns and proceeds, otherwise a mismatch
    raises RuntimeError. Default leaves the version unchecked so
    candidate engines and stale-reuse probes are unaffected.

    Raises FileNotFoundError when the engine binary is missing and
    subprocess.TimeoutExpired on timeout (after killing the run's
    process group). `timeout` bounds one run in seconds (default
    RUN_TIMEOUT, 300 s, overridable per call and via --timeout).
    """
    if require_reference_version:
        check_reference_version(engine_bin, allow_any=allow_any_reference)
    env = pinned_env()
    if extra_env:
        env.update(extra_env)
    if fmt is None:
        args = ENGINE_ARGS + ENGINE_SHELL_FLAGS
    else:
        args = [a for a in ENGINE_ARGS if a not in ("-ini", "-etex")]
        args = args + ENGINE_SHELL_FLAGS + ["-fmt=" + fmt]
    # Execute through a per-engine ".../pdftex" symlink (see engine_link),
    # so argv[0]-derived log text prints identically for both engines.
    argv0 = engine_link(engine_bin, workdir)
    job = os.path.splitext(os.path.basename(tex_path))[0]
    log_path = os.path.join(workdir, job + ".log")
    pdf_path = os.path.join(workdir, job + ".pdf")

    def _sig(path):
        try:
            st = os.stat(path)
            return (st.st_ino, st.st_mtime_ns)
        except OSError:
            return None

    log_before = _sig(log_path)
    pdf_before = _sig(pdf_path)
    # Own process group (start_new_session) with a per-run timeout;
    # the whole group is SIGKILLed afterwards, so a wrapper's detached
    # same-group child never outlives the gate.
    returncode, raw_out = _run_isolated([argv0] + args + [tex_path],
                                        cwd=workdir, env=env,
                                        timeout=timeout)
    out = raw_out.decode("utf-8", "replace")
    if log_before is not None and _sig(log_path) == log_before:
        # The engine left a pre-existing log untouched (e.g. it failed
        # before opening the transcript and wrote diagnostics only to
        # stdout): keep the captured stdout as the log instead of
        # discarding it, so the failure stays diagnosable — but only
        # for a run that produced no log because the engine failed to
        # start (nonzero exit). An exit-0 run must produce its own
        # job.log; stdout is never its log (this rejects a replayed
        # transcript printed on stdout with no log file).
        if returncode != 0 and out.strip():
            log = normalise(out, workdir)
        else:
            log = ""
    else:
        try:
            # newline="" keeps CR bytes: text mode would otherwise
            # translate CRLF to LF before normalise() ever sees it.
            with open(log_path, encoding="utf-8", errors="replace",
                      newline="") as fh:
                raw = fh.read()
        except OSError:
            # No log file at all: stdout is the log only for a failed
            # run (nonzero exit). An exit-0 run without its own job.log
            # is an error (empty log, reported by run_engine).
            raw = out if returncode != 0 else ""
        if not raw.strip():
            log = ""
        else:
            log = normalise(raw, workdir)
    boxes = split_boxes(log)
    _, accounting = split_accounting(log.split("\n"))
    if pdf_before is not None and _sig(pdf_path) == pdf_before:
        pdf_path = None
    elif not os.path.exists(pdf_path):
        pdf_path = None
    return Capture(log=log, boxes=boxes, pdf_path=pdf_path,
                   returncode=returncode, accounting=accounting)


def run_engine(binary, name, *, allow_any_reference=False,
               require_reference_version=False, timeout=RUN_TIMEOUT):
    """Run one engine on one case in a fresh temp dir. Returns a dict."""
    tmpdir = tempfile.mkdtemp(prefix="lockstep-")
    shutil.copy(PRELUDE, os.path.join(tmpdir, "prelude.tex"))
    shutil.copy(os.path.join(CASES_DIR, name + ".tex"),
                os.path.join(tmpdir, name + ".tex"))
    try:
        cap = capture(os.path.join(tmpdir, name + ".tex"), binary, tmpdir,
                      allow_any_reference=allow_any_reference,
                      require_reference_version=require_reference_version,
                      timeout=timeout)
    except FileNotFoundError:
        return {"ok": False, "tmpdir": tmpdir, "error": "binary not found"}
    except RuntimeError as exc:
        return {"ok": False, "tmpdir": tmpdir, "error": str(exc)}
    except subprocess.TimeoutExpired:
        return {"ok": False, "tmpdir": tmpdir, "error": "timed out"}
    if not cap.log.strip():
        return {"ok": False, "tmpdir": tmpdir,
                "returncode": cap.returncode,
                "error": "exit %d, no log" % cap.returncode}
    if cap.returncode == 0 and log_expects_pdf(cap.log):
        # The log claims an output file was written: the file the
        # "Output written on" line names must exist next to the log
        # and pass its format check (.pdf as before, .dvi for
        # \pdfoutput=0 runs), or the case FAILs. A deleted,
        # truncated or garbage-mangled output no longer passes on the
        # log alone.
        problem = output_integrity_error(cap.log, tmpdir)
        if problem is not None:
            return {"ok": False, "tmpdir": tmpdir,
                    "returncode": cap.returncode,
                    "error": problem}
    if cap.returncode != 0:
        tail = "\n".join(cap.log.splitlines()[-5:])
        return {"ok": True, "tmpdir": tmpdir, "log": cap.log,
                "returncode": cap.returncode,
                "accounting": cap.accounting,
                "error": "exit %d. tail:\n%s" % (cap.returncode, tail)}
    return {"ok": True, "tmpdir": tmpdir, "log": cap.log,
            "returncode": cap.returncode, "accounting": cap.accounting}


def _diff_window(a_line, b_line, context=20, width=61):
    """(1-based column, a snippet, b snippet) around the first difference.

    Shows a window around the first differing column instead of only the
    line start, so long log lines read from the differing region.
    """
    n = min(len(a_line), len(b_line))
    col = next((k for k in range(n) if a_line[k] != b_line[k]), n)
    lo = max(0, col - context)
    def _snip(line):
        seg = line[lo:lo + width]
        prefix = "..." if lo > 0 else ""
        suffix = "..." if len(line) > lo + width else ""
        return "%s%s%s" % (prefix, seg, suffix)
    caret = " " * ((3 if lo > 0 else 0) + min(context, col - lo)) + "^"
    return col + 1, _snip(a_line), _snip(b_line), caret


def check_pair(name, a_label, a_lines, b_label, b_lines):
    """Compare two normalised logs; report the first differing line."""
    n = max(len(a_lines), len(b_lines))
    idx = next((i for i in range(n)
                if (a_lines[i] if i < len(a_lines) else "<EOF>") !=
                   (b_lines[i] if i < len(b_lines) else "<EOF>")), None)
    if idx is None:
        print("PASS %s" % name)
        return True
    print("FAIL %s (%s vs %s differ)" % (name, a_label, b_label))
    print("  first difference at log line %d:" % (idx + 1))
    for j in range(max(0, idx - 3), idx):
        for label, lines in ((a_label, a_lines), (b_label, b_lines)):
            if j < len(lines):
                print("  %4d %s: %s" % (j + 1, label, lines[j]))
    a_line = a_lines[idx] if idx < len(a_lines) else "<EOF>"
    b_line = b_lines[idx] if idx < len(b_lines) else "<EOF>"
    print("  %4d %s: %s" % (idx + 1, a_label, a_line))
    print("  %4d %s: %s" % (idx + 1, b_label, b_line))
    if a_line != "<EOF>" and b_line != "<EOF>":
        col, a_snip, b_snip, caret = _diff_window(a_line, b_line)
        print("  column %d:" % col)
        print("    %s: %s" % (a_label, a_snip))
        print("         %s" % caret)
        print("    %s: %s" % (b_label, b_snip))
    return False


def _cases_differ_word(n):
    return "case differs" if n == 1 else "cases differ"


def report_accounting(name, ref_accounting, other_accounting):
    """Non-gating accounting check: print a line when kinds differ.

    Returns True when the accounting lines differ (for the CLI total).
    Never affects the exit code or the PASS/FAIL count.
    """
    kinds = accounting_diff_kinds(ref_accounting, other_accounting)
    if kinds:
        print("accounting: %s differs (%s)" % (name, " / ".join(kinds)))
        return True
    return False


def write_expected(name, log):
    os.makedirs(EXPECTED_DIR, exist_ok=True)
    # newline="" writes the normalised log byte-exactly (CR bytes and
    # the final newline survive the round-trip through expected/).
    with open(os.path.join(EXPECTED_DIR, name + ".log"), "w",
              newline="") as fh:
        fh.write(log)


def select_cases(patterns):
    names = sorted(fn[:-4] for fn in os.listdir(CASES_DIR) if fn.endswith(".tex"))
    if not patterns:
        return names
    return sorted(nm for nm in names
                  if any(fnmatch.fnmatch(nm, p) or fnmatch.fnmatch(nm + ".tex", p)
                         for p in patterns))


def valid_run(result, name, what):
    if result.get("ok") and "log" in result:
        return True
    print("FAIL %s (%s failed: %s)" % (name, what, result.get("error")))
    return False


def check_returncodes(name, ref, other, other_label):
    """FAIL when either return code is nonzero or the codes differ.

    A missing return code also FAILs: an unknown exit status must never
    compare equal.
    """
    ref_rc = ref.get("returncode")
    other_rc = other.get("returncode")
    if ref_rc is None or other_rc is None:
        print("FAIL %s (returncode missing: reference=%s vs %s=%s)" %
              (name, ref_rc, other_label, other_rc))
        return False
    if ref_rc != 0 or other_rc != 0 or ref_rc != other_rc:
        if ref_rc != other_rc:
            print("FAIL %s (returncode reference=%s vs %s=%s)" %
                  (name, ref_rc, other_label, other_rc))
        elif ref_rc != 0:
            print("FAIL %s (returncode %s, both engines exit %s)" %
                  (name, other_label, ref_rc))
        else:
            print("FAIL %s (returncode reference=%s vs %s=%s)" %
                  (name, ref_rc, other_label, other_rc))
        return False
    return True


def check_shipout(name, result, what):
    """Self-test requires exit 0 and at least one real shipout.

    Anchored like split_boxes: only a line beginning with SHIPOUT_LINE
    counts — a trace line merely mentioning the text is not a shipout.
    """
    if result.get("returncode") != 0:
        print("FAIL %s (%s exit %s, expected 0)" %
              (name, what, result.get("returncode")))
        return False
    if not any(ln.startswith(SHIPOUT_LINE)
               for ln in result.get("log", "").split("\n")):
        print("FAIL %s (%s shipped no box)" % (name, what))
        return False
    return True


def main(argv=None):
    ap = argparse.ArgumentParser(description="lockstep differential harness")
    ap.add_argument("--engine", help="path to candidate engine binary")
    ap.add_argument("--cases", nargs="*", default=[], help="globs or case names")
    ap.add_argument("--reference", default="pdftex", help="reference engine")
    ap.add_argument("--update-expected", action="store_true",
                    help="regenerate expected/*.log from the reference")
    ap.add_argument("--keep", action="store_true", help="keep per-run temp dirs")
    ap.add_argument("--self-test", action="store_true",
                    help="run the reference against itself for every case")
    ap.add_argument("--allow-any-reference", action="store_true",
                    help="skip the pinned pdfTeX 1.40.29 reference check")
    ap.add_argument("--timeout", type=float, default=RUN_TIMEOUT,
                    help="per-run engine timeout in seconds (default %s)" %
                    RUN_TIMEOUT)
    args = ap.parse_args(argv)

    if not os.path.isfile(PRELUDE):
        print("error: missing prelude.tex", file=sys.stderr)
        return 2
    names = select_cases(args.cases)
    if not names:
        print("error: no cases match %r" % (args.cases,), file=sys.stderr)
        return 2
    if shutil.which(args.reference) is None and not os.path.isfile(args.reference):
        print("error: reference not found: %s" % args.reference, file=sys.stderr)
        return 2
    try:
        check_reference_version(args.reference,
                                allow_any=args.allow_any_reference)
    except RuntimeError as exc:
        print("error: %s" % exc, file=sys.stderr)
        return 2
    if not args.self_test and not args.engine and not args.update_expected:
        print("error: --engine is required (or use --self-test)", file=sys.stderr)
        return 2

    kept, equal, differ, accounting_differ = [], 0, 0, 0
    for name in names:
        ref = run_engine(args.reference, name,
                         allow_any_reference=args.allow_any_reference,
                         require_reference_version=True,
                         timeout=args.timeout)
        if not valid_run(ref, name, "reference"):
            differ += 1
            kept.append(ref["tmpdir"])
            continue
        if args.update_expected:
            # Never bless a bad reference run: a reference that exited
            # nonzero or shipped no box is reported as an error and its
            # log is not written to expected/.
            if not check_shipout(name, ref, "reference"):
                differ += 1
                kept.append(ref["tmpdir"])
                continue
            write_expected(name, ref["log"])
        if args.self_test or not args.engine:
            if args.self_test:
                again = run_engine(args.reference, name,
                                   allow_any_reference=args.allow_any_reference,
                                   require_reference_version=True,
                                   timeout=args.timeout)
                if not valid_run(again, name, "reference re-run"):
                    differ += 1
                    kept.append(again["tmpdir"])
                    kept.append(ref["tmpdir"])
                    continue
                if (not check_shipout(name, ref, "reference") or
                        not check_shipout(name, again, "reference re-run")):
                    differ += 1
                    kept.append(again["tmpdir"])
                    kept.append(ref["tmpdir"])
                    continue
                if not check_returncodes(name, ref, again, "ref-run2"):
                    differ += 1
                    if ("accounting" in ref and "accounting" in again
                            and report_accounting(
                                name, ref["accounting"],
                                again["accounting"])):
                        accounting_differ += 1
                    kept.append(again["tmpdir"])
                    kept.append(ref["tmpdir"])
                    continue
                same = check_pair(name, "ref-run1",
                                  compared_lines(ref["log"]),
                                  "ref-run2", compared_lines(again["log"]))
                if report_accounting(name, ref.get("accounting", []),
                                     again.get("accounting", [])):
                    accounting_differ += 1
                kept.append(again["tmpdir"])
                exp = os.path.join(EXPECTED_DIR, name + ".log")
                if same and os.path.exists(exp):
                    with open(exp, newline="") as fh:
                        same = check_pair(name, "reference",
                                          compared_lines(ref["log"]),
                                          "expected",
                                          compared_lines(fh.read()))
                equal, differ = equal + same, differ + (not same)
            else:
                print("wrote expected/%s.log" % name)
                equal += 1
        else:
            cand = run_engine(args.engine, name,
                              timeout=args.timeout)
            if not valid_run(cand, name, "candidate"):
                differ += 1
                kept.append(cand["tmpdir"])
                kept.append(ref["tmpdir"])
                continue
            if not check_returncodes(name, ref, cand, "candidate"):
                differ += 1
                if ("accounting" in ref and "accounting" in cand
                        and report_accounting(
                            name, ref["accounting"],
                            cand["accounting"])):
                    accounting_differ += 1
                kept.append(cand["tmpdir"])
                kept.append(ref["tmpdir"])
                continue
            same = check_pair(name, "reference",
                              compared_lines(ref["log"]),
                              "candidate", compared_lines(cand["log"]))
            if report_accounting(name, ref.get("accounting", []),
                                 cand.get("accounting", [])):
                accounting_differ += 1
            equal, differ = equal + same, differ + (not same)
            kept.append(cand["tmpdir"])
        kept.append(ref["tmpdir"])
    if not args.keep:
        for path in kept:
            shutil.rmtree(path, ignore_errors=True)
    else:
        for path in kept:
            print("kept %s" % path)
    if args.update_expected and not args.engine and not args.self_test:
        print("wrote %d expected file(s)" % equal)
        print("accounting: %d %s" % (accounting_differ, _cases_differ_word(
            accounting_differ)))
        return 0 if differ == 0 else 1
    print("%d cases, %d equal, %d differ" % (len(names), equal, differ))
    print("accounting: %d %s" % (accounting_differ, _cases_differ_word(
        accounting_differ)))
    return 1 if differ else 0


if __name__ == "__main__":
    sys.exit(main())
