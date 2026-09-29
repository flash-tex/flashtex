"""P-T1 capture adapter: one traced run of a TeX engine, in PDF mode.

    capture(tex_path, engine_bin, workdir, *, fmt=None) -> Capture(log, boxes, pdf_path)

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

Normalisation covers only what differs legitimately between two runs of
the same typesetting:
  * banner: every line before the `**` first-line echo (engine name and
    version, format date, `\\write18` and `%&-line` notes);
  * paths: the absolute work directory becomes `<WORKDIR>`;
  * producer: the byte count in `Output written on X (N pages, B bytes).`,
    which follows from the `/Producer` string and the PDF's serialisation
    (P-T2 compares the PDF itself; P-T3 would compare its bytes).
"""

import collections
import os
import re
import subprocess

TRACE = (r"\tracingall\tracingonline=1\showboxdepth=2147483647\showboxbreadth=2147483647"
         r"\nonstopmode")
TRACE_ENV = {"SOURCE_DATE_EPOCH": "0", "FORCE_SOURCE_DATE": "1",
             "max_print_line": "10000", "error_line": "254", "half_error_line": "238"}
TIMEOUT = 600

Capture = collections.namedtuple("Capture", "log boxes pdf_path")

SHIPOUT = "Completed box being shipped out"
_OUTPUT_WRITTEN = re.compile(r"^(Output written on .*\(\d+ pages?), \d+ bytes\)\.$", re.M)


def normalise_log(text, workdir):
    """The legitimate-difference normalisation described in the module doc."""
    lines = text.split("\n")
    start = next((i for i, ln in enumerate(lines) if ln.startswith("**")), 0)
    out = "\n".join(lines[start:])
    for p in {os.path.realpath(workdir), os.path.abspath(workdir)}:
        out = out.replace(p.rstrip("/") + "/", "<WORKDIR>/").replace(p, "<WORKDIR>")
    return _OUTPUT_WRITTEN.sub(r"\1, <BYTES> bytes).", out)


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


def _capture_standin(tex_path, engine_bin, workdir, *, fmt=None):
    tex = os.path.relpath(os.path.abspath(tex_path), os.path.abspath(workdir))
    stem = os.path.splitext(os.path.basename(tex))[0]
    argv = [engine_bin] + ([f"-fmt={fmt}"] if fmt else []) + [
        "-interaction=nonstopmode", "-halt-on-error", f"-jobname={stem}", TRACE + r"\input{" + tex + "}"]
    env = dict(os.environ, **TRACE_ENV)
    try:
        subprocess.run(argv, cwd=workdir, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL, timeout=TIMEOUT, check=False)
    except subprocess.TimeoutExpired:
        pass
    logp, pdf = os.path.join(workdir, stem + ".log"), os.path.join(workdir, stem + ".pdf")
    try:
        with open(logp, "rb") as f:
            raw = f.read().decode("latin-1")  # TeX writes bytes; latin-1 round-trips them
    except OSError:
        raw = ""
    log = normalise_log(raw, workdir)
    return Capture(log, split_boxes(log), pdf if os.path.isfile(pdf) else None)


# TODO(lockstep): replace with `from run import capture` (tools/lockstep on
# sys.path) once tools/lockstep/run.py lands with the `fmt=` keyword; until
# then this stand-in is the implementation.
capture = _capture_standin
SOURCE = "tools/parity/capture.py (stand-in until tools/lockstep lands)"
