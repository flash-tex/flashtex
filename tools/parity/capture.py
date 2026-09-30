"""P-T1 capture adapter: one traced run of a TeX engine, in PDF mode.

    capture(tex_path, engine_bin, workdir, *, fmt=None, extra_env=None) -> Capture(log, boxes, pdf_path)

(tools/parity adds `stream=` and `timeout=`, and a Capture of a log over
MAX_LOG_BYTES has `log` None and its `fingerprint`: see pt1stream.py.)

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

The rules are `workdir_subs`, `banner_end`, `Accounting.step` and
`BoxSplitter`. `normalise_log`, `split_accounting` and `split_boxes` drive
them over a whole log; pt1stream.py drives the same code over a stream, for a
log too big to hold (one implementation, two drivers).
"""

import collections
import hashlib
import os
import re
import subprocess
import tempfile

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
# DESIGN §4.5 pins the random seed. pdfTeX seeds \pdfuniformdeviate (l3kernel's
# \int_rand, pgf's random numbers) from the clock, so a document that draws one
# differs from itself between runs. Every pass of every TeX engine the harness
# runs (the oracles and a TeX --engine) starts its first line with this.
SEED = r"\pdfsetrandomseed 1\relax"
TRACE_ENV = {"SOURCE_DATE_EPOCH": "0", "FORCE_SOURCE_DATE": "1",
             "max_print_line": "10000", "error_line": "254", "half_error_line": "238"}
# The traced pass's time limit in seconds (parity.py --pt1-timeout). A pass it
# stops is a harness error (tiers.TRACE_TIMEOUT), never a pass.
TIMEOUT = 1800

# The traced-log budget in bytes (parity.py --pt1-max-log-mb; 0: none). A log
# over it is never read whole: a \tracingall log runs to 25 GB (arXiv
# 2501.08663v2), and reading one whole costs several times that in memory.
# Such a log is read once as a stream into its P-T1 fingerprint
# (pt1stream.py, constant memory), and a Capture of it has `log` and `boxes`
# None, its `size`, `complete` (from its tail) and `fingerprint`.
MAX_LOG_BYTES = 1024 << 20

Capture = collections.namedtuple("Capture", "log boxes pdf_path size complete fingerprint timed_out",
                                 defaults=(None, None, None, False))


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


def workdir_subs(workdir):
    """The (text, replacement) pairs `normalise_log` applies, in order: each
    spelling of `workdir` (real and absolute path), the longest first so a
    path that contains the other (`/private/tmp/w` and `/tmp/w`) is replaced
    whole, and for each its `dir/` form before the bare one. No pair holds a
    newline, so applying them to each line, or to any run of whole lines, is
    the same as applying them to the whole text (pt1stream relies on this)."""
    subs = []
    for p in sorted({os.path.realpath(workdir), os.path.abspath(workdir)}, key=lambda p: (-len(p), p)):
        subs += [(p.rstrip("/") + "/", "<WORKDIR>/"), (p, "<WORKDIR>")]
    assert not any("\n" in a for a, _ in subs), workdir
    return subs


def banner_end(line):
    """Whether `line` is the `**` first-line echo, where the banner ends."""
    return line.startswith("**")


def normalise_log(text, workdir):
    """The legitimate-difference normalisation described in the module doc."""
    lines = text.split("\n")
    start = next((i for i, ln in enumerate(lines) if banner_end(ln)), 0)
    out = "\n".join(lines[start:])
    for a, b in workdir_subs(workdir):
        out = out.replace(a, b)
    return out


class Accounting:
    """The N2 ruling's per-line state: which lines of a log are accounting.
    `step` is the one implementation of the rules in the module doc.
    `split_accounting` drives it over a whole log, knowing where the last
    shipout is; `pt1stream` drives it over a stream, which does not (it runs
    a copy per hypothesis). `copy` is what makes that possible."""

    __slots__ = ("block", "pos", "mem_owed", "seen")

    def __init__(self):
        self.seen = set()          # trailer items already consumed (headers, Output written)
        self.block, self.pos = None, 0  # current block's shapes and the next one allowed
        self.mem_owed = 0          # shipouts not yet followed by their Memory usage line

    def copy(self):
        c = Accounting()
        c.seen, c.block, c.pos, c.mem_owed = set(self.seen), self.block, self.pos, self.mem_owed
        return c

    def step(self, ln, trailer, strict, accounting):
        """Route one line: to `strict`, to `accounting`, or (`Output written`)
        to both, its byte count replaced in the strict copy. `trailer`: the
        line comes after the log's last shipout."""
        block = self.block
        if block is not None:
            j = next((k for k in range(self.pos, len(block)) if block[k].match(ln)), None)
            if j is not None:
                accounting.append(ln)
                self.pos = j + 1
                return
            self.block = None
        if SHIPOUT in ln:
            self.mem_owed += 1  # a \shipout nested in another's (beamer) owes its line too
        if trailer and ln in ACCOUNTING_BLOCKS and ln not in self.seen:
            self.seen.add(ln)
            accounting.append(ln)
            self.block, self.pos = ACCOUNTING_BLOCKS[ln], 0
            return
        if self.mem_owed and _MEMORY_USAGE.match(ln):
            self.mem_owed -= 1
            accounting.append(ln)
            return
        m = _OUTPUT_WRITTEN.match(ln) if trailer and "output" not in self.seen else None
        if m:
            self.seen.add("output")
            accounting.append(ln)
            strict.append(m.group(1) + ", <BYTES> bytes).")
            return
        strict.append(ln)


def trailer_sensitive(ln):
    """Whether `Accounting.step` can route `ln` differently in the trailer
    than before it, given no block is open and nothing has been consumed:
    a block header or an `Output written` line. Every other line goes the
    same way either side of the last shipout."""
    return ln in ACCOUNTING_BLOCKS or (ln.startswith("Output written on ") and _OUTPUT_WRITTEN.match(ln) is not None)


def split_accounting(log):
    """(strict log, accounting lines): the ruling's accounting removed from
    `log` under the rules in the module doc, and exactly the removed lines, in
    order. The `Output written` line stays in the strict log with its byte
    count replaced by `<BYTES>`."""
    lines = log.split("\n")
    last_ship = max((i for i, ln in enumerate(lines) if SHIPOUT in ln), default=-1)
    strict, accounting = [], []
    st = Accounting()
    for i, ln in enumerate(lines):
        st.step(ln, i > last_ship, strict, accounting)
    return "\n".join(strict), accounting


class BoxSplitter:
    """Where `\\shipout` box dumps start and end in a `\\tracingoutput` log:
    from the `Completed box being shipped out [..]` line up to the blank line
    that `end_diagnostic(true)` prints after the box. `feed` each line, then
    `close`; a subclass says what to do with the pieces (`split_boxes`
    collects them, pt1stream hashes them)."""

    def __init__(self):
        self.open = False

    def feed(self, ln):
        if not self.open:
            k = ln.find(SHIPOUT)
            if k >= 0:
                self.open = True
                self.start(ln[k:])
        elif ln == "":
            self.open = False
            self.end()
        else:
            self.add(ln)

    def close(self):
        if self.open:
            self.open = False
            self.end()


class _BoxList(BoxSplitter):
    def __init__(self):
        super().__init__()
        self.boxes, self.cur = [], None

    def start(self, first):
        self.cur = [first]

    def add(self, ln):
        self.cur.append(ln)

    def end(self):
        self.boxes.append("\n".join(self.cur))


def split_boxes(log):
    """Every `\\shipout` box dump in a `\\tracingoutput` log, in order (`BoxSplitter`)."""
    b = _BoxList()
    for ln in log.split("\n"):
        b.feed(ln)
    b.close()
    return b.boxes


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
    link = engine_link(engine_bin)
    argv = [PROGRAM] + ([SHELL_ESCAPE] if SHELL_ESCAPE else []) + ([f"-fmt={fmt}"] if fmt else []) + list(args)
    try:
        p = subprocess.run(argv, executable=link, cwd=workdir, env=engine_env(link, extra_env),
                           stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                           timeout=timeout, check=False)
        return p.returncode, False
    except subprocess.TimeoutExpired:
        return None, True


def first_line(tex, trace=False):
    """The command line's TeX input for `tex`: the pinned seed, the tracing
    settings for a traced pass, then `\\input{tex}`."""
    return SEED + (TRACE if trace else "") + r"\input{" + tex + "}"


def _capture_standin(tex_path, engine_bin, workdir, *, fmt=None, extra_env=None, stream=False, timeout=None):
    """One traced pass. With `stream`, its log is a named pipe that
    pt1stream reads while the engine runs, so no byte of it reaches the disk:
    "pipe" keeps it in memory up to the budget and streams it past that (the
    oracle, whose size is not known yet); "fingerprint" streams it from the
    first byte, in constant memory (a candidate whose oracle's log is over
    the budget). Otherwise the log is a file as usual; one over the budget is
    read as a stream and then deleted. `timeout` (default TIMEOUT) bounds the
    pass; `timed_out` says it stopped it."""
    import pt1stream  # the streamed half of this module's normalisation

    tex = os.path.relpath(os.path.abspath(tex_path), os.path.abspath(workdir))
    stem = os.path.splitext(os.path.basename(tex))[0]
    logp, pdf = os.path.join(workdir, stem + ".log"), os.path.join(workdir, stem + ".pdf")
    args = ["-interaction=nonstopmode", "-halt-on-error", f"-jobname={stem}", first_line(tex, trace=True)]
    timeout = timeout or TIMEOUT
    raw = None
    if stream:
        with pt1stream.LogPipe(logp, workdir, None if stream == "fingerprint" else MAX_LOG_BYTES) as pipe:
            _, timed_out = run_engine(engine_bin, fmt, args, workdir, extra_env, timeout=timeout)
        pdf = pdf if os.path.isfile(pdf) else None
        if pipe.replaced:  # the engine put a file where the pipe was: read that instead
            stream = False
        elif pipe.fingerprint is not None:
            fp = pipe.fingerprint
            return Capture(None, None, pdf, fp["bytes"], fp["complete"], fp, timed_out)
        else:
            raw, size = pipe.raw, len(pipe.raw)
    else:
        _, timed_out = run_engine(engine_bin, fmt, args, workdir, extra_env, timeout=timeout)
        pdf = pdf if os.path.isfile(pdf) else None
    if raw is None:
        size = os.path.getsize(logp) if os.path.isfile(logp) else 0
        if MAX_LOG_BYTES and size > MAX_LOG_BYTES:  # checked before a byte of it is read whole
            fp = pt1stream.fingerprint_file(logp, workdir)
            os.remove(logp)  # up to tens of GB: never left behind
            return Capture(None, None, pdf, size, fp["complete"], fp, timed_out)
        try:
            with open(logp, "rb") as f:
                raw = f.read()
        except OSError:
            raw = b""
    log = normalise_log(raw.decode("latin-1"), workdir)  # TeX writes bytes; latin-1 round-trips them
    return Capture(log, split_boxes(log), pdf, size, None, None, timed_out)


# TODO(lockstep): replace with `from run import capture` (tools/lockstep on
# sys.path) once tools/lockstep/run.py lands with the `fmt=` and `extra_env=`
# keywords, the one shell-escape setting and the `pdftex` argv[0]; until
# then this stand-in is the implementation.
capture = _capture_standin
SOURCE = "tools/parity/capture.py (stand-in until tools/lockstep lands)"
