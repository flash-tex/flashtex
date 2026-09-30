"""Constant-memory P-T1: a traced log read once, as a stream, into its fingerprint.

    fingerprint = Stream(workdir).feed(bytes)... .close()
    fingerprint_file(path, workdir), fingerprint_gz(path), fingerprint_text(log)
    with LogPipe(path, workdir, budget) as pipe: <run the engine>   # a named pipe

`capture.py`'s in-memory P-T1 holds a whole traced log. A `\\tracingall` log
of an arXiv e-print runs to 25 GB, so over the traced-log budget
(`capture.MAX_LOG_BYTES`) the log is streamed instead, and `tiers.compare_pt1`'s
verdict is computed from two fingerprints (`tiers.compare_pt1_streamed`).

**Same semantics, not a second normaliser.** Every rule is `capture.py`'s own
code: `workdir_subs` and `banner_end` (normalise_log), `Accounting.step` (the
N2 ruling in `split_accounting`) and `BoxSplitter` (`split_boxes`). What
differs is only the driver. The in-memory driver knows where the log's last
shipout is before it starts; a stream doesn't, so two lookaheads become
hypotheses that the stream resolves:
  * banner: lines before the first `**` line are dropped, unless there is
    none, when nothing is. Until a `**` line comes, the lines feed a machine
    that keeps them; at the `**` line a fresh machine starts there. At the
    end, whichever machine is current is the answer.
  * trailer: a line is in the trailer when no shipout follows it. The
    machine `N` routes every line as not-trailer, which is right for every
    line up to the latest shipout. The trailer hypothesis `T` is `N` itself
    (no copy) until a line arrives that `Accounting.step` routes differently
    in the trailer (`capture.trailer_sensitive`: a block header or an
    `Output written` line); then `T` becomes a copy of `N` and both run on.
    The next shipout discards `T` (everything before it was not trailer, as
    `N` assumed). At the end `T`, if any, is the answer: the lines after
    the last shipout are the trailer.
The streamed verdict is exactly the in-memory one: the strict log and every
box dump are compared by SHA-256 of the same text `split_accounting` and
`split_boxes` produce, and the accounting lines (at most one per shipout plus
the trailer's) are kept and compared whole, so the non-gating accounting
report is unchanged.

**The fast path.** A run of whole lines in which `Accounting.step` would only
append each line to the strict log, and `BoxSplitter` only extend the open
box, is fed as one text: the strict hash and the box hash take it at once.
`_plain` says when that holds; it looks for each literal that any rule
starts from (the shipout line, `Memory usage before: ` while a shipout owes
one, each block header, `Output written on `, and a blank line while a box
is open) and finds none, and no block is open. Every line is still part of
the text the hashes see; only the per-line Python is skipped.

**Localisation.** The strict log's hash is also taken per SEGMENT bytes, with
the line each segment starts on, so a failure says which lines hold the first
difference. Box dumps are hashed one per shipout, so the first differing
shipout is exact.

Oracle tooling only (Python 3 standard library).
"""

import gzip
import hashlib
import os
import stat
import threading

import capture

SHIPOUT = capture.SHIPOUT
READ = 1 << 20        # bytes per read
CHUNK = 1 << 18       # bytes of whole lines processed at a time
SEGMENT = 1 << 22     # strict-log bytes per localisation digest
TAIL = 65536          # raw bytes kept to tell whether the run reached its end
MAX_LINE = 64 << 20   # a longer line is a harness error, not an unbounded buffer
JOIN_S = 60           # how long the reader may take to drain after the engine exits
_PLAIN_BLOCKERS = tuple(capture.ACCOUNTING_BLOCKS) + ("Output written on ",)
_MEMORY_PREFIX = "Memory usage before: "
V = 1


class StreamError(RuntimeError):
    """A log the stream cannot judge (a line over MAX_LINE, a pipe that stays open)."""


class _Hash:
    """SHA-256 of a text written in pieces, with a digest per SEGMENT bytes
    (and the 1-based line each segment starts on) for localisation."""

    __slots__ = ("h", "seg", "segs", "n", "seg_n", "newlines", "seg_line", "buf", "buf_n")

    def __init__(self):
        self.h, self.seg, self.segs = hashlib.sha256(), hashlib.sha256(), []
        self.n = self.seg_n = self.newlines = 0
        self.seg_line = 1
        self.buf, self.buf_n = [], 0

    def write(self, s):
        self.buf.append(s)
        self.buf_n += len(s)
        if self.buf_n >= CHUNK:
            self.flush()

    def flush(self):
        if not self.buf:
            return
        data = "".join(self.buf).encode("latin-1")  # every character came from latin-1 bytes or "<WORKDIR>"
        self.buf, self.buf_n = [], 0
        self.h.update(data)
        self.n += len(data)
        i = 0
        while i < len(data):
            part = data[i:i + SEGMENT - self.seg_n]
            self.seg.update(part)
            self.seg_n += len(part)
            self.newlines += part.count(b"\n")
            i += len(part)
            if self.seg_n == SEGMENT:
                self.segs.append([self.seg.hexdigest()[:32], self.seg_line])
                self.seg, self.seg_n, self.seg_line = hashlib.sha256(), 0, self.newlines + 1

    def copy(self):
        self.flush()
        c = _Hash.__new__(_Hash)
        c.h, c.seg, c.segs = self.h.copy(), self.seg.copy(), [list(s) for s in self.segs]
        c.n, c.seg_n, c.newlines, c.seg_line = self.n, self.seg_n, self.newlines, self.seg_line
        c.buf, c.buf_n = [], 0
        return c

    def result(self):
        self.flush()
        segs = self.segs + ([[self.seg.hexdigest()[:32], self.seg_line]] if self.seg_n else [])
        return {"sha256": self.h.hexdigest(), "bytes": self.n, "segment_bytes": SEGMENT, "segments": segs}


class _Strict:
    """The strict log as `split_accounting` joins it ("\\n" between lines),
    hashed. `append` is what `Accounting.step` calls, as on a list."""

    __slots__ = ("hash", "lines")

    def __init__(self):
        self.hash, self.lines = _Hash(), 0

    def append(self, ln):
        if self.lines:
            self.hash.write("\n")
        self.hash.write(ln)
        self.lines += 1

    def extend_text(self, text, count):
        """`count` whole lines joined by "\\n", as one write."""
        if self.lines:
            self.hash.write("\n")
        self.hash.write(text)
        self.lines += count

    def copy(self):
        c = _Strict.__new__(_Strict)
        c.hash, c.lines = self.hash.copy(), self.lines
        return c

    def result(self):
        return dict(self.hash.result(), lines=self.lines)


class _Hyp:
    """One hypothesis: the N2 state and what it has routed so far."""

    __slots__ = ("acc", "strict", "accounting")

    def __init__(self):
        self.acc, self.strict, self.accounting = capture.Accounting(), _Strict(), []

    def step(self, ln, trailer):
        self.acc.step(ln, trailer, self.strict, self.accounting)

    def copy(self):
        c = _Hyp.__new__(_Hyp)
        c.acc, c.strict, c.accounting = self.acc.copy(), self.strict.copy(), list(self.accounting)
        return c


class _BoxHashes(capture.BoxSplitter):
    """`split_boxes`, with each box hashed instead of kept."""

    def __init__(self):
        super().__init__()
        self.digests, self.cur, self.buf = [], None, []

    def start(self, first):
        self.cur, self.buf = hashlib.sha256(), [first]

    def add(self, ln):
        self.buf.append("\n")
        self.buf.append(ln)
        if len(self.buf) >= 4096:
            self._flush()

    def add_text(self, text):
        self.buf.append("\n")
        self.buf.append(text)
        self._flush()

    def _flush(self):
        self.cur.update("".join(self.buf).encode("latin-1"))
        self.buf = []

    def end(self):
        self._flush()
        self.digests.append(self.cur.hexdigest())
        self.cur = None


class _Log:
    """Everything after the banner: the box splitter and the N/T hypotheses."""

    def __init__(self):
        self.boxes = _BoxHashes()
        self.n = _Hyp()   # every line routed as before the last shipout
        self.t = None     # the trailer hypothesis once it differs from n (None: it is n)

    def _plain(self, text):
        """Whether `Accounting.step` would append every line of `text` to the
        strict log unchanged and `BoxSplitter.feed` only extend the open box
        (or do nothing): see the module doc. A literal a rule starts from that
        is not a substring of `text` is on none of its lines."""
        if SHIPOUT in text:
            return False
        for h in (self.n, self.t):
            if h is not None and (h.acc.block is not None or (h.acc.mem_owed and _MEMORY_PREFIX in text)):
                return False
        if any(b in text for b in _PLAIN_BLOCKERS):
            return False
        return not (self.boxes.open and (text == "" or "\n\n" in text or text[0] == "\n" or text[-1] == "\n"))

    def text(self, text):
        """Whole lines joined by "\\n" (the last one has no newline)."""
        if self._plain(text):
            count = text.count("\n") + 1
            self.n.strict.extend_text(text, count)
            if self.t is not None:
                self.t.strict.extend_text(text, count)
            if self.boxes.open:
                self.boxes.add_text(text)
            return
        self.lines(text.split("\n"))

    def lines(self, lines):
        n, boxes = self.n, self.boxes
        for ln in lines:
            boxes.feed(ln)
            if SHIPOUT in ln:
                self.t = None  # this line and all before it are before the last shipout
            elif self.t is None and capture.trailer_sensitive(ln):
                self.t = n.copy()
            if self.t is not None:
                self.t.step(ln, True)
            n.step(ln, False)

    def result(self):
        self.boxes.close()
        h = self.t or self.n
        return {"strict": h.strict.result(), "boxes": self.boxes.digests, "accounting": h.accounting}


class Stream:
    """One traced log, fed as raw bytes in any pieces; `close` returns its
    fingerprint. `workdir`: normalise its paths as `normalise_log` does (None
    for a log that is already normalised, such as a cached oracle log)."""

    def __init__(self, workdir=None):
        self.subs = capture.workdir_subs(workdir) if workdir else []
        self.log = _Log()
        self.banner_seen = False
        self.rest, self.pending, self.pending_n = b"", [], 0
        self.size, self.tail = 0, b""

    def feed(self, data):
        self.size += len(data)
        self.pending.append(data)
        self.pending_n += len(data)
        if self.pending_n >= CHUNK:
            self._run()
        return self

    def _run(self):
        new = b"".join(self.pending)
        self.pending, self.pending_n = [], 0
        self.tail = new[-TAIL:] if len(new) >= TAIL else (self.tail + new)[-TAIL:]
        data = self.rest + new
        cut = data.rfind(b"\n")
        if cut < 0:
            self.rest = data
        else:
            self.rest = data[cut + 1:]
            self._text(data[:cut].decode("latin-1"))
        if len(self.rest) > MAX_LINE:
            raise StreamError(f"a log line is longer than {MAX_LINE >> 20} MiB")

    def _text(self, text):
        for a, b in self.subs:  # line-local (workdir_subs), so a run of whole lines at a time is exact
            text = text.replace(a, b)
        if not self.banner_seen:
            lines = text.split("\n")
            start = next((i for i, ln in enumerate(lines) if capture.banner_end(ln)), None)
            if start is None:
                self.log.lines(lines)  # the hypothesis that no `**` line comes: they are kept
                return
            self.banner_seen = True
            self.log = _Log()  # the banner ends here: nothing before this line counts
            self.log.lines(lines[start:])
            return
        self.log.text(text)

    def close(self):
        """The fingerprint: {"v", "bytes", "complete", "strict", "boxes", "accounting"}."""
        self._run()
        self._text(self.rest.decode("latin-1"))  # the last line ("" after a final newline)
        self.rest = b""
        tail = self.tail.decode("latin-1")
        fp = {"v": V, "bytes": self.size,
              "complete": "\nOutput written on " in tail or "\nNo pages of output." in tail}
        fp.update(self.log.result())
        return fp


def _read_all(f, s):
    while True:
        b = f.read(READ)
        if not b:
            return s.close()
        s.feed(b)


def fingerprint_file(path, workdir):
    """A raw traced log on disk, read once in pieces."""
    with open(path, "rb") as f:
        return _read_all(f, Stream(workdir))


def fingerprint_gz(path):
    """A cached, already normalised log (tiers.oracle's log.gz)."""
    with gzip.open(path, "rb") as f:
        return _read_all(f, Stream(None))


def fingerprint_text(log):
    """An already normalised log held in memory (a Capture's `log`)."""
    s = Stream(None)
    for i in range(0, len(log), READ):
        s.feed(log[i:i + READ].encode("latin-1"))
    return s.close()


class LogPipe:
    """The traced pass's log as a named pipe at `path`, read by a thread
    while the engine writes it, so the log never reaches the disk. Up to
    `budget` bytes (0: no limit) are kept (`raw`) for the in-memory P-T1;
    past that they are dropped and the log is streamed (`fingerprint`).

    The harness holds a write end of its own until the engine has exited
    (`__exit__`), so the reader sees the end of the log only then, whether
    the engine opened the log once, never, or was killed. `replaced` says the
    engine put a file where the pipe was; the caller reads that instead."""

    def __init__(self, path, workdir, budget):
        self.path, self.workdir, self.budget = path, workdir, budget
        self.raw = self.fingerprint = self.error = None
        self.replaced = False
        self._buf, self._stream = bytearray(), None

    def __enter__(self):
        if os.path.lexists(self.path):
            os.remove(self.path)
        os.mkfifo(self.path, 0o600)
        self._rfd = os.open(self.path, os.O_RDONLY | os.O_NONBLOCK)  # a reader first, so...
        self._wfd = os.open(self.path, os.O_WRONLY | os.O_NONBLOCK)  # ...our writer's open succeeds
        os.set_blocking(self._rfd, True)
        self._thread = threading.Thread(target=self._read, name="pt1-log-pipe", daemon=True)
        self._thread.start()
        return self

    def _read(self):
        try:
            with os.fdopen(self._rfd, "rb", buffering=0) as f:
                while True:
                    b = f.read(READ)
                    if not b:
                        break
                    self._take(b)
            if self._stream is not None:
                self.fingerprint = self._stream.close()
            else:
                self.raw = self._buf  # a bytearray: decoded in place, never copied
        except BaseException as e:  # noqa: BLE001 - closing the pipe stops the engine (EPIPE); reported below
            self.error = f"reading the traced log: {e!r}"

    def _take(self, b):
        if self._stream is not None:
            self._stream.feed(b)
            return
        self._buf += b
        if self.budget and len(self._buf) > self.budget:  # over the budget: stream it from here on
            self._stream = Stream(self.workdir)
            view = memoryview(self._buf)
            for i in range(0, len(view), READ):  # in pieces, so the stream's working set stays small
                self._stream.feed(view[i:i + READ].tobytes())
            view.release()
            self._buf = None

    def __exit__(self, *exc):
        os.close(self._wfd)  # the engine has exited: the log ends when its writers are gone
        self._thread.join(JOIN_S)
        if self._thread.is_alive():
            self.error = self.error or f"the traced log was still open {JOIN_S} s after the engine exited"
        try:
            self.replaced = not stat.S_ISFIFO(os.lstat(self.path).st_mode)
        except FileNotFoundError:
            pass
        if not self.replaced and os.path.lexists(self.path):
            os.remove(self.path)
        if self.error and not self.replaced and exc[0] is None:
            raise StreamError(self.error)
        return False
