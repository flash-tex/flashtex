"""P-T1 capture adapter: one traced run of a TeX engine, in PDF mode.

    capture(tex_path, engine_bin, workdir, *, fmt=None, extra_env=None) -> Capture(log, boxes, pdf_path)

`extra_env` is added to the run's environment (the candidate engine's
`FLASHTEX_FORMATS`, never the oracle's). Every run uses the one shell-escape
setting (SHELL_ESCAPE; by default no flag) and a `pdftex` symlink as argv[0]
(engine_link).

This is the shape `tools/lockstep/run.py` exposes (agreed on #2, comments
5885107001 and 5885140240), so there is one capture implementation in the
repository, not two. `tools/parity` only calls `capture` through this module.

TODO(lockstep): when `tools/lockstep/run.py` with `capture` lands on main,
delete the stand-in below and import lockstep's `capture` at the bottom of
this file. The stand-in is deliberately minimal and must not grow features that
lockstep lacks.

Settings, as DESIGN §1.1 / the P0-LOCKSTEP-HARNESS assignment define them:
LaTeX's `\\tracingall` (which includes `\\tracingoutput=1`, so every
`\\shipout` dumps its box into the log), then `\\tracingonline=1`,
`\\showboxdepth=\\showboxbreadth=2147483647`, and `\\nonstopmode` again,
because LaTeX's `\\loggingoutput` sets `\\errorstopmode` and TeX would block
on stdin at the first error. `max_print_line`, `error_line` and
`half_error_line` are raised through the environment, and
`SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1` pin every date.

`normalise_log` covers what differs between two runs of the same engine in
two places:
  * banner: every line before the `**` first-line echo (engine name and
    version, format date, `\\write18` and `%&-line` notes);
  * paths: the absolute work directory becomes `<WORKDIR>`.

`split_accounting` then applies the DESIGN §1.1 P-T1 ruling (2026-09-29,
"N2"). It removes only end-of-run capacity and output-size accounting, and
hands those lines back so the harness can report them as a separate,
non-gating accounting check:
  * `\\tracingstats` lines `Memory usage before: a&b; after: c&d; still
    untouched: e`, matched as the whole line;
  * the `Here is how much of TeX's memory you used:` block;
  * the `PDF statistics:` block;
  * the byte count in `Output written on X (N pages, B bytes).`. The page
    count stays compared.
Every removed line must have one of the exact shapes pdfTeX 1.40.29 prints
(`ACCOUNTING_BLOCKS`, taken from the 82 fixture logs and checked against
tex.web and pdftex.web; plurals follow `print_char("s")`). Anything else
stays compared (#1191 review 5888078003; #1196 review 5888464701):
  * a block's lines must come in pdfTeX's order, each shape at most once. The
    first line that doesn't fit ends the block and is compared, even if it
    starts with a space (` junk`, ` Overfull \\hbox`);
  * each block header, and `Output written on`, counts only once, and only
    in the end-of-run trailer after the last `\\shipout`. A header earlier in
    the log is compared;
  * a `Memory usage` line counts only when an earlier shipout still owes
    its one line. Nested shipouts, as in beamer, owe theirs too.
"""

import collections
import hashlib
import os
import re
import subprocess
import tempfile
import time

# The one \write18 setting, for BOTH engines. Owner decision #1209 (DESIGN
# §4.5): restricted by default, as in TeX Live's pdflatex. So the default is
# no flag at all, and each engine runs in its own default mode (texmf.cnf
# `shell_escape = p`: ` restricted \write18 enabled.`, \pdfshellescape=2,
# which l3kernel's \sys_if_shell reads). `parity.py --shell-escape-flag`
# overrides it with -shell-restricted, -no-shell-escape or -shell-escape.
SHELL_ESCAPE = None
PROGRAM = "pdftex"  # the name every engine runs under (see engine_link)
BIN_ROOT = os.path.join(tempfile.gettempdir(), "flashtex-parity-bin")

TRACE =(r"\tracingall\tracingonline=1\showboxdepth=2147483647\showboxbreadth=2147483647"
         r"\nonstopmode")
TRACE_ENV = {"SOURCE_DATE_EPOCH": "0", "FORCE_SOURCE_DATE": "1",
             "max_print_line": "10000", "error_line": "254", "half_error_line": "238"}
TIMEOUT = 600

# `oversize`: the traced log passed `max_log_bytes` (its size when the run
# was stopped); `log` and `boxes` are then None, because the log was not read
Capture = collections.namedtuple("Capture", "log boxes pdf_path oversize", defaults=(None,))

SHIPOUT = "Completed box being shipped out"
_OUTPUT_WRITTEN = re.compile(r"^(Output written on .*\(\d+ pages?), \d+ bytes\)\.$")
_MEMORY_USAGE = re.compile(r"^Memory usage before: \d+&\d+; after: \d+&\d+; still untouched: \d+$")
# header -> the continuation lines it may have, in pdfTeX's order (tex.web
# section 1334, pdftex.web's PDF statistics); each matches the whole line
ACCOUNTING_BLOCKS = {
    "Here is how much of TeX's memory you used:": [re.compile(x) for x in (
        r"^ \d+ strings? out of \d+$",
        r"^ \d+ string characters? out of \d+$",
        r"^ \d+ words of memory out of \d+$",
        r"^ \d+ multiletter control sequences? out of \d+\+\d+$",
        r"^ \d+ words of font info for \d+ fonts?, out of \d+ for \d+$",
        r"^ \d+ hyphenation exceptions? out of \d+$",
        r"^ \d+i,\d+n,\d+p,\d+b,\d+s stack positions out of \d+i,\d+n,\d+p,\d+b,\d+s$")],
    "PDF statistics:": [re.compile(x) for x in (
        r"^ \d+ PDF objects? out of \d+ \(max\. \d+\)$",
        r"^ \d+ compressed objects? within \d+ object streams?$",
        r"^ \d+ named destinations? out of \d+ \(max\. \d+\)$",
        r"^ \d+ words of extra memory for PDF output out of \d+ \(max\. \d+\)$")],
}


def normalise_log(text, workdir):
    """The legitimate-difference normalisation described in the module doc."""
    lines = text.split("\n")
    start = next((i for i, ln in enumerate(lines) if ln.startswith("**")), 0)
    out = "\n".join(lines[start:])
    for p in {os.path.realpath(workdir), os.path.abspath(workdir)}:
        out = out.replace(p.rstrip("/") + "/", "<WORKDIR>/").replace(p, "<WORKDIR>")
    return out


def split_accounting(log):
    """(strict log, accounting lines): the ruling's accounting removed from
    `log` under the rules in the module doc, and exactly the removed lines, in
    order. The `Output written` line stays in the strict log with its byte
    count replaced by `<BYTES>`."""
    lines = log.split("\n")
    last_ship = max((i for i, ln in enumerate(lines) if SHIPOUT in ln), default=-1)
    strict, accounting = [], []
    seen = set()          # trailer items already consumed (headers, Output written)
    block, pos = None, 0  # current block's shapes and the next one allowed
    mem_owed = 0          # shipouts not yet followed by their Memory usage line
    for i, ln in enumerate(lines):
        if block is not None:
            j = next((k for k in range(pos, len(block)) if block[k].match(ln)), None)
            if j is not None:
                accounting.append(ln)
                pos = j + 1
                continue
            block = None
        if SHIPOUT in ln:
            mem_owed += 1  # a \shipout nested in another's (beamer) owes its line too
        trailer = i > last_ship
        if trailer and ln in ACCOUNTING_BLOCKS and ln not in seen:
            seen.add(ln)
            accounting.append(ln)
            block, pos = ACCOUNTING_BLOCKS[ln], 0
            continue
        if mem_owed and _MEMORY_USAGE.match(ln):
            mem_owed -= 1
            accounting.append(ln)
            continue
        m = _OUTPUT_WRITTEN.match(ln) if trailer and "output" not in seen else None
        if m:
            seen.add("output")
            accounting.append(ln)
            strict.append(m.group(1) + ", <BYTES> bytes).")
            continue
        strict.append(ln)
    return "\n".join(strict), accounting


def split_boxes(log):
    """Every `\\shipout` box dump in a `\\tracingoutput` log, in order: from
    the `Completed box being shipped out [..]` line up to the blank line that
    `end_diagnostic(true)` prints after the box."""
    boxes, cur = [], None
    for ln in log.split("\n"):
        if cur is None:
            k = ln.find(SHIPOUT)
            if k >= 0:
                cur = [ln[k:]]
        elif ln == "":
            boxes.append("\n".join(cur))
            cur = None
        else:
            cur.append(ln)
    if cur:
        boxes.append("\n".join(cur))
    return boxes


def engine_link(engine_bin):
    """The per-engine bin directory's `pdftex` symlink to `engine_bin`.

    Every engine, the oracle included, runs as argv[0] `pdftex` through such
    a link (`run_engine`), because pdfTeX's warnings print argv[0], as in
    `pdfTeX warning: pdftex (file ./x.pdf): ...`. Setting the name before
    the run means nothing in the log needs normalising, so nothing can hide
    behind a token normaliser. kpathsea follows the link to the real binary's
    directory, so SELFAUTOLOC and texmf.cnf are unchanged."""
    real = os.path.realpath(engine_bin)
    d = os.path.join(BIN_ROOT, hashlib.sha256(real.encode("utf-8")).hexdigest()[:16])
    link = os.path.join(d, PROGRAM)
    if os.path.realpath(link) != real:
        os.makedirs(d, exist_ok=True)
        tmp = f"{link}.{os.getpid()}"
        if os.path.lexists(tmp):
            os.remove(tmp)
        os.symlink(real, tmp)
        os.replace(tmp, link)  # atomic: parallel workers may race here
    return link


def engine_env(link, extra_env=None):
    """The run environment. The link's directory comes first on PATH, so
    kpathsea, finding argv[0] `pdftex` there, follows the link to the real
    binary. `FLASHTEX_*` variables are never inherited. Only `extra_env`,
    which callers give the candidate engine alone (e.g. its
    `FLASHTEX_FORMATS`), puts them there, so the oracle never sees them."""
    env = {k: v for k, v in os.environ.items() if not k.startswith("FLASHTEX_")}
    env.update(TRACE_ENV)
    env.update(extra_env or {})
    env["PATH"] = os.path.dirname(link) + os.pathsep + env.get("PATH", "")
    return env


def run_engine(engine_bin, fmt, args, workdir, extra_env=None, timeout=TIMEOUT):
    """One run: argv[0] is exactly `pdftex` (pdfTeX prints argv[0] as given,
    e.g. `pdfTeX warning: ./x (file ...)`, so a path would differ per engine),
    executed through the engine's link, with the one shell-escape setting and
    the format. Returns (exit code or None, timed out)."""
    code, timed_out, _ = _run(engine_bin, fmt, args, workdir, extra_env, timeout)
    return code, timed_out


def _run(engine_bin, fmt, args, workdir, extra_env=None, timeout=TIMEOUT, watch=None, max_bytes=None):
    """`run_engine`, plus: with `max_bytes`, the file `watch` (the traced
    log) is measured every 0.2 s while the engine runs, and the engine is
    killed as soon as it is larger. Returns (exit code or None, timed out,
    the size it was stopped at or None). A log is never allowed to grow to
    gigabytes on disk, and is never read when over the cap."""
    link = engine_link(engine_bin)
    argv = [PROGRAM] + ([SHELL_ESCAPE] if SHELL_ESCAPE else []) + ([f"-fmt={fmt}"] if fmt else []) + list(args)
    p = subprocess.Popen(argv, executable=link, cwd=workdir, env=engine_env(link, extra_env),
                         stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    deadline = time.monotonic() + timeout
    while True:
        try:
            code = p.wait(timeout=0.2 if max_bytes else max(0.0, deadline - time.monotonic()))
            return code, False, None
        except subprocess.TimeoutExpired:
            pass
        if max_bytes:
            try:
                size = os.path.getsize(watch)
            except OSError:
                size = 0
            if size > max_bytes:
                p.kill()
                p.wait()
                return None, False, size
        if time.monotonic() >= deadline:
            p.kill()
            p.wait()
            return None, True, None


def _capture_standin(tex_path, engine_bin, workdir, *, fmt=None, extra_env=None, max_log_bytes=None):
    tex = os.path.relpath(os.path.abspath(tex_path), os.path.abspath(workdir))
    stem = os.path.splitext(os.path.basename(tex))[0]
    logp, pdf = os.path.join(workdir, stem + ".log"), os.path.join(workdir, stem + ".pdf")
    _, _, oversize = _run(engine_bin, fmt, ["-interaction=nonstopmode", "-halt-on-error", f"-jobname={stem}",
                                            TRACE + r"\input{" + tex + "}"], workdir, extra_env,
                          watch=logp, max_bytes=max_log_bytes)
    if oversize is not None:
        try:
            os.remove(logp)  # the partial trace is not compared, and can be large
        except OSError:
            pass
        return Capture(None, None, None, oversize)
    try:
        with open(logp, "rb") as f:
            raw = f.read().decode("latin-1")  # TeX writes bytes; latin-1 round-trips them
    except OSError:
        raw = ""
    log = normalise_log(raw, workdir)
    return Capture(log, split_boxes(log), pdf if os.path.isfile(pdf) else None)


# TODO(lockstep): replace with `from run import capture` (tools/lockstep on
# sys.path) once tools/lockstep/run.py lands with the `fmt=` and `extra_env=`
# keywords, the one shell-escape setting and the `pdftex` argv[0]; until
# then this stand-in is the implementation.
capture = _capture_standin
SOURCE = "tools/parity/capture.py (stand-in until tools/lockstep lands)"
