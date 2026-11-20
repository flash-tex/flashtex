#!/usr/bin/env python3
"""JBIG2 image fuzzer for the FlashTeX candidate pdfTeX engine. MIT, stdlib only.

Each iteration builds a JBIG2 file from a segment model (ITU-T T.88 Annex
D: file header, segment headers, sequential or random-access
organisation), applies ONE structure-aware mutation, and includes it with
``\\pdfximage page N {fuzz.jbig2}`` in a minimal job. pdfTeX's writejbig2.c
reads only the file header, the segment headers and the page information
segments (it copies segment data unread), so the mutations target exactly
those: segment numbers (which set the width of referred-to segment
numbers: 1, 2 or 4 bytes), types, page associations and their 1/4-byte
size flag, data lengths (0xFFFFFFFF is "unknown"), referred-to segment
lists in the short and the long form, segment order, the file header's
organisation and page-count flags, and page information (size,
resolution, striping with an unknown height, end-of-stripe rows).

Without ``--oracle`` it is a crash fuzzer, like png.py: returncode < 0 or
101 is ``crash``, a timeout ``hang``, 0 ``ok``, other ``graceful-error``.
With ``--oracle BIN`` it is differential: both engines run the same job
(``SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1``) and must agree on the exit
status, the PDF bytes and the log from the ``**`` line on (lines joined,
the program name after ``pdfTeX warning:``/``!pdfTeX error:`` and the
end-of-run accounting removed). Classes then follow run.py: ``equal``,
``diverge``, ``candidate-crash``, ``oracle-crash``, ``both-crash``,
``timeout``, ``both-hang``, ``output-flood``, ``both-flood``.

Deterministic given --seed: every choice goes through one random.Random.
"""
import argparse
import hashlib
import json
import os
import random
import re
import shutil
import struct
import subprocess
import sys
import tempfile

sys.path.insert(0,
                os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import run as fuzz_run

SIG = b"\x97JB2\r\n\x1a\n"
JOB = ("\\pdfoutput=1 \\pdfsuppressptexinfo=-1 \\pdfcompresslevel=0 "
       "\\pdfobjcompresslevel=0 \\pdfminorversion=%d\n%s\\bye\n")
CRASH_CLASSES = ("crash", "hang", "ok", "graceful-error", "output-flood")
CRASH_STORE = ("crash", "hang", "output-flood")
# equal-pdf: same exit status, log and PDF bytes; equal-error: no PDF from
# either, same exit status and the same non-empty log (matching "!pdfTeX
# error" text is real parity); inconclusive: same exit status but neither
# engine wrote a PDF or a log, so nothing was compared (e.g. both failed to
# load the format). inconclusive is counted, not stored.
DIFF_CLASSES = ("equal-pdf", "equal-error", "inconclusive", "diverge", "candidate-crash", "oracle-crash",
                "both-crash", "timeout", "both-hang", "output-flood",
                "both-flood")
DIFF_STORE = DIFF_CLASSES[3:]
U32 = (0, 1, 2, 0xFF, 0x100, 0xFFFF, 0x10000, 0x7FFFFFFF, 0xFFFFFFFF)
# T.88 7.3: segment types (0 symbol dictionary ... 62 extension) and some
# that are reserved
TYPES = (0, 4, 6, 7, 16, 20, 22, 23, 36, 38, 39, 40, 42, 43, 48, 49, 50,
         51, 52, 53, 62, 1, 2, 63)
PAGE_INFO, END_OF_PAGE, END_OF_STRIPE, END_OF_FILE = 48, 49, 50, 51


class Seg:
    """One segment: header fields plus its data bytes. ``length`` None is
    the data's own length; ``long_page`` forces the 4-byte page
    association; ``long_refs`` forces the long referred-to form."""

    def __init__(self, number, typ, page, data=b"", refs=(), retain=0):
        self.number, self.type, self.page = number, typ, page
        self.data, self.refs, self.retain = data, list(refs), retain
        self.length = None
        self.long_page = False
        self.long_refs = False
        self.deferred = False

    def clone(self):
        s = Seg(self.number, self.type, self.page, self.data, self.refs,
                self.retain)
        s.length, s.long_page = self.length, self.long_page
        s.long_refs, s.deferred = self.long_refs, self.deferred
        return s


class Doc:
    """A JBIG2 file: organisation, page count (None: unknown), segments."""

    def __init__(self, segs, sequential=True, pages=1, extra_flags=0):
        self.segs, self.sequential, self.pages = segs, sequential, pages
        self.extra_flags = extra_flags

    def clone(self):
        return Doc([s.clone() for s in self.segs], self.sequential,
                   self.pages, self.extra_flags)


def ref_width(number):
    """T.88 7.2.5: referred-to segment numbers are 1 byte for a segment
    numbered up to 256, 2 bytes up to 65536, else 4."""
    if number <= 256:
        return 1
    return 2 if number <= 65536 else 4


def seg_header(s):
    long_page = s.long_page or s.page > 0xFF
    flags = (s.type & 0x3F) | (0x40 if long_page else 0) | \
        (0x80 if s.deferred else 0)
    h = struct.pack(">I", s.number & 0xFFFFFFFF) + bytes([flags])
    n = len(s.refs)
    if n <= 4 and not s.long_refs:
        h += bytes([(n << 5) | (s.retain & 0x1F)])
    else:
        # long form: 3 bits of 7, a 29-bit count, then one retention bit
        # for the segment and one per referred-to segment
        h += struct.pack(">I", (7 << 29) | (n & 0x1FFFFFFF))
        h += bytes([s.retain & 0xFF]) + bytes((n + 1 + 7) // 8 - 1)
    w = ref_width(s.number)
    for r in s.refs:
        h += (r & ((1 << (8 * w)) - 1)).to_bytes(w, "big")
    h += struct.pack(">I", s.page & 0xFFFFFFFF) if long_page \
        else bytes([s.page & 0xFF])
    length = len(s.data) if s.length is None else s.length
    return h + struct.pack(">I", length & 0xFFFFFFFF)


def build(doc):
    """Serialise: the file header, then each segment's header and data
    (sequential) or all headers and then all data (random access)."""
    flags = (1 if doc.sequential else 0) | \
        (2 if doc.pages is None else 0) | (doc.extra_flags & 0xFC)
    out = SIG + bytes([flags & 0xFF])
    if doc.pages is not None:
        out += struct.pack(">I", doc.pages & 0xFFFFFFFF)
    if doc.sequential:
        for s in doc.segs:
            out += seg_header(s) + s.data
    else:
        out += b"".join(seg_header(s) for s in doc.segs)
        out += b"".join(s.data for s in doc.segs)
    return out


def page_info(w, h, xres=11811, yres=11811, flags=0, striping=0):
    """T.88 7.4.8: width, height, resolutions (pixels per metre), flags,
    striping information."""
    return struct.pack(">IIIIBH", w, h, xres, yres, flags, striping)


def region(w, h, payload=b"\x00" * 8):
    """A generic region's region segment information field (7.4.1) and a
    flags byte, then bytes the engine copies without decoding."""
    return struct.pack(">IIIIB", w, h, 0, 0, 0) + b"\x01" + payload


def seed_docs():
    """[(name, Doc)]: sequential one page; random access, two pages that
    refer to a page-0 symbol dictionary; a striped page of unknown
    height with end-of-stripe segments."""
    seq = Doc([Seg(0, PAGE_INFO, 1, page_info(64, 32)),
               Seg(1, 38, 1, region(64, 32)),
               Seg(2, END_OF_PAGE, 1),
               Seg(3, END_OF_FILE, 0)], sequential=True, pages=1)
    ra = Doc([Seg(0, 0, 0, b"\x00\x01" + b"\x55" * 16),
              Seg(1, PAGE_INFO, 1, page_info(48, 48, 3937, 3937)),
              Seg(2, 6, 1, region(48, 48), refs=[0]),
              Seg(3, END_OF_PAGE, 1),
              Seg(4, PAGE_INFO, 2, page_info(20, 10)),
              Seg(5, 6, 2, region(20, 10), refs=[0]),
              Seg(6, END_OF_PAGE, 2),
              Seg(7, END_OF_FILE, 0)], sequential=False, pages=2)
    striped = Doc([Seg(0, PAGE_INFO, 1,
                       page_info(32, 0xFFFFFFFF, striping=0x8000 | 16)),
                   Seg(1, 38, 1, region(32, 16)),
                   Seg(2, END_OF_STRIPE, 1, struct.pack(">I", 15)),
                   Seg(3, 38, 1, region(32, 16)),
                   Seg(4, END_OF_STRIPE, 1, struct.pack(">I", 31)),
                   Seg(5, END_OF_PAGE, 1)], sequential=True, pages=None)
    return [("seq-1page", seq), ("ra-2pages-globals", ra),
            ("striped", striped)]


def mutate_doc(doc, rng):
    """Apply ONE structural mutation to a copy of ``doc``; return
    (Doc, description)."""
    d = doc.clone()
    ops = ["number", "type", "page", "length", "refs", "file-flags",
           "file-pages", "page-info"]
    if d.segs:
        ops += ["del", "dup", "swap"]
    op = rng.choice(ops)
    if not d.segs and op not in ("file-flags", "file-pages"):
        op = "file-flags"
    i = rng.randrange(len(d.segs)) if d.segs else 0
    s = d.segs[i] if d.segs else None
    if op == "number":
        others = [x.number for x in d.segs if x is not s]
        s.number = rng.choice(list(U32[:7]) + others + [257, 65537])
        return d, "number[%d]=%d" % (i, s.number)
    if op == "type":
        s.type = rng.choice(TYPES)
        return d, "type[%d]=%d" % (i, s.type)
    if op == "page":
        s.page = rng.choice((0, 1, 2, 3, 0xFF, 0x100, 0xFFFFFFFF))
        s.long_page = rng.random() < 0.3
        return d, "page[%d]=%d%s" % (i, s.page, "L" if s.long_page else "")
    if op == "length":
        n = len(s.data)
        s.length = rng.choice((0, 1, max(n - 1, 0), n + 1, 0xFFFF,
                               0x7FFFFFFF, 0xFFFFFFFF))
        return d, "length[%d]=%d" % (i, s.length)
    if op == "refs":
        kind = rng.choice(("add", "self", "missing", "forward", "many",
                           "long", "clear"))
        if kind == "add" and len(d.segs) > 1:
            s.refs.append(d.segs[rng.randrange(len(d.segs))].number)
        elif kind == "self":
            s.refs.append(s.number)
        elif kind == "missing":
            s.refs.append(rng.choice((200, 0xFFFF, 0xFFFFFFFF)))
        elif kind == "forward":
            s.refs.append(s.number + 1)
        elif kind == "many":
            s.refs += [x.number for x in d.segs][:5]
        elif kind == "long":
            s.long_refs = True
        else:
            s.refs = []
        s.retain = rng.randrange(32)
        return d, "refs[%d]-%s" % (i, kind)
    if op == "file-flags":
        kind = rng.choice(("org", "pages-unknown", "bit2", "byte"))
        if kind == "org":
            d.sequential = not d.sequential
        elif kind == "pages-unknown":
            d.pages = None if d.pages is not None else 1
        elif kind == "bit2":
            d.extra_flags |= 0x04
        else:
            d.extra_flags = rng.randrange(256)
        return d, "file-flags-%s" % kind
    if op == "file-pages":
        d.pages = rng.choice((0, 1, 2, 3, 0xFFFFFFFF))
        return d, "file-pages=%d" % d.pages
    if op == "page-info":
        infos = [x for x in d.segs if x.type == PAGE_INFO]
        p = rng.choice(infos) if infos else s
        field = rng.choice(("w", "h", "xres", "yres", "flags", "striping",
                            "short"))
        if field == "short":
            p.data = p.data[:rng.randrange(len(p.data) + 1)]
            return d, "page-info-short=%d" % len(p.data)
        vals = list(struct.unpack(">IIIIBH", p.data[:19])) \
            if len(p.data) >= 19 else [1, 1, 0, 0, 0, 0]
        k = ("w", "h", "xres", "yres", "flags", "striping").index(field)
        if k == 4:  # flags byte
            vals[k] = rng.choice((0, 1, 0x20, 0xFF))
        elif k == 5:  # striping: bit 15 striped, 15 bits of stripe size
            vals[k] = rng.choice((0, 0x8000, 0x8001, 0xFFFF))
        else:
            vals[k] = rng.choice(U32)
        p.data = struct.pack(">IIIIBH", *vals) + p.data[19:]
        return d, "page-info-%s=%d" % (field, vals[k])
    if op == "del":
        del d.segs[i]
        return d, "del[%d]" % i
    if op == "dup":
        d.segs.insert(i, s.clone())
        return d, "dup[%d]" % i
    j = rng.randrange(len(d.segs))
    d.segs[i], d.segs[j] = d.segs[j], d.segs[i]
    return d, "swap[%d,%d]" % (i, j)


def mutate(doc, rng):
    """One structural mutation, serialised; every ~6th input also gets raw
    byte flips or a truncation. Returns (bytes, description)."""
    d, desc = mutate_doc(doc, rng)
    data = build(d)
    r = rng.random()
    if r < 0.1 and len(data) > 1:
        i = rng.randrange(1, len(data))
        return data[:i], desc + "+trunc@%d" % i
    if r < 0.17:
        buf = bytearray(data)
        for _ in range(rng.randint(1, 4)):
            k = rng.randrange(len(buf))
            buf[k] = (buf[k] + rng.randint(1, 255)) % 256
        return bytes(buf), desc + "+flip"
    return data, desc


def job_body(rng):
    """(\\pdfminorversion, body): one page of the file, two pages (which
    share the page-0 segments), or page 0."""
    minor = 5 if rng.random() < 0.95 else 3
    pick = rng.random()
    if pick < 0.6:
        pages = [1]
    elif pick < 0.85:
        pages = [1, 2]
    elif pick < 0.95:
        pages = [rng.choice((2, 3))]
    else:
        pages = [0]
    body = "".join("\\pdfximage page %d {fuzz.jbig2}\\setbox0\\hbox"
                   "{\\pdfrefximage\\pdflastximage}\\shipout\\box0\n" % p
                   for p in pages)
    return minor, body


def normalised_log(text):
    """The log from the ``**`` line on, lines joined, the program name
    after "pdfTeX warning: " and "!pdfTeX error: " replaced, and the
    end-of-run accounting (its numbers are each engine's own) cut."""
    i = text.find("\n**")
    s = "".join((text[i + 1:] if i >= 0 else text).splitlines())
    for key in ("Here is how much of TeX's memory you used:",
                "PDF statistics:"):
        k = s.find(key)
        if k >= 0:
            s = s[:k]
    return re.sub(r"(pdfTeX warning: |!pdfTeX error: )[^ :]+", r"\1PROG", s)


def run_engine(binary, data, job, timeout, env):
    """Run one engine on ``job`` with ``data`` as fuzz.jbig2 in a fresh
    directory: (rc or None on timeout, output, log text, PDF bytes)."""
    tmp = tempfile.mkdtemp(prefix="jbig2fuzz-")
    try:
        with open(os.path.join(tmp, "fuzz.jbig2"), "wb") as fh:
            fh.write(data)
        with open(os.path.join(tmp, "job.tex"), "w") as fh:
            fh.write(job)
        try:
            rc, out = fuzz_run.run_capped(
                [binary] + fuzz_run.FUZZ_SHELL_ESCAPE_FLAGS
                + ["-fmt=pdftex", "-interaction=nonstopmode", "job.tex"],
                cwd=tmp, timeout=timeout, env=env)
        except subprocess.TimeoutExpired:
            return None, b"", "", None
        log = ""
        try:
            with open(os.path.join(tmp, "job.log"), "rb") as fh:
                log = fh.read().decode("latin-1")
        except OSError:
            pass
        pdf = None
        try:
            with open(os.path.join(tmp, "job.pdf"), "rb") as fh:
                pdf = fh.read()
        except OSError:
            pass
        return rc, out, log, pdf
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def engine_env(candidate):
    """The pinned environment; FLASHTEX_POOL / FLASHTEX_FORMATS reach the
    candidate only (run.py's rule)."""
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
    if not candidate:
        for k in fuzz_run.CANDIDATE_ENV_VARS:
            env.pop(k, None)
    return env


def is_crash(rc):
    # Return code only: output text never decides a crash.
    return rc is not None and (rc < 0 or rc == 101)


def classify_crash(rc):
    """Crash-only mode."""
    if rc is None:
        return "hang"
    if fuzz_run.is_output_flood(rc):
        return "output-flood"
    if is_crash(rc):
        return "crash"
    return "ok" if rc == 0 else "graceful-error"


def classify_diff(cand, orc):
    """Differential mode: ``cand`` and ``orc`` are run_engine results."""
    crc, orc_rc = cand[0], orc[0]
    if crc is None or orc_rc is None:
        return "both-hang" if crc is None and orc_rc is None else "timeout"
    cf, of = fuzz_run.is_output_flood(crc), fuzz_run.is_output_flood(orc_rc)
    if cf or of:
        return "both-flood" if cf and of else "output-flood"
    cc, oc = is_crash(crc), orc_rc < 0
    if cc and oc:
        return "both-crash"
    if cc:
        return "candidate-crash"
    if oc:
        return "oracle-crash"
    if crc != orc_rc or cand[3] != orc[3] or \
            normalised_log(cand[2]) != normalised_log(orc[2]):
        return "diverge"
    if cand[3] is not None:
        return "equal-pdf"
    if normalised_log(orc[2]).strip():
        return "equal-error"
    return "inconclusive"


def first_diff(a, b):
    """The first differing stretch of two normalised logs, or None."""
    for i, (x, y) in enumerate(zip(a, b)):
        if x != y:
            return a[max(0, i - 40):i + 80], b[max(0, i - 40):i + 80]
    if len(a) != len(b):
        n = min(len(a), len(b))
        return a[max(0, n - 40):n + 80], b[max(0, n - 40):n + 80]
    return None


def signature(cls, cand, orc=None):
    """Dedupe key for a stored result."""
    if cls in ("hang", "both-hang", "output-flood", "both-flood"):
        return cls
    if cls == "timeout":
        return "timeout:" + ("candidate" if cand[0] is None else "oracle")
    if cls in ("crash", "candidate-crash"):
        return fuzz_run.crash_signature(cand[0], _text(cand[1]))
    if cls == "oracle-crash":
        return "oracle-crash:" + fuzz_run.crash_signature(orc[0],
                                                          _text(orc[1]))
    if cls == "both-crash":
        return "both-crash:" + fuzz_run.crash_signature(cand[0],
                                                        _text(cand[1]))
    # diverge: the first differing log stretch, else exit status, else PDF
    d = first_diff(normalised_log(cand[2]), normalised_log(orc[2]))
    if d is not None:
        what = "log %r vs %r" % d
    elif cand[0] != orc[0]:
        what = "rc %s vs %s" % (cand[0], orc[0])
    else:
        what = "pdf"
    return "diverge:" + hashlib.sha256(
        re.sub(r"\d", "N", what).encode("utf-8")).hexdigest()[:16]


def _text(out):
    return (out or b"").decode("utf-8", "replace")


def last_line(out):
    lines = _text(out).strip().splitlines()
    return lines[-1][-200:] if lines else ""


def load_known(out_dir):
    """Signatures already stored under OUT (per-case .json sidecars)."""
    known = set()
    for root, _dirs, files in os.walk(out_dir):
        for name in files:
            if not name.endswith(".json"):
                continue
            try:
                with open(os.path.join(root, name)) as fh:
                    info = json.load(fh)
            except (OSError, ValueError):
                continue
            if isinstance(info, dict) and info.get("signature"):
                known.add(info["signature"])
    return known


def run_fuzz(candidate, out_dir, iterations, seed, timeout, oracle=None,
             seeds=None):
    rng = random.Random(seed)
    seeds = seeds if seeds is not None else seed_docs()
    classes, store = (DIFF_CLASSES, DIFF_STORE) if oracle \
        else (CRASH_CLASSES, CRASH_STORE)
    known = load_known(out_dir)
    seen = set()
    counts = {c: 0 for c in classes}
    oracle_output = 0  # iterations where the oracle wrote a PDF or a log
    for i in range(iterations):
        name, base = seeds[rng.randrange(len(seeds))]
        data, desc = mutate(base, rng)
        minor, body = job_body(rng)
        job = JOB % (minor, body)
        cand = run_engine(candidate, data, job, timeout, engine_env(True))
        orc = None
        if oracle:
            orc = run_engine(oracle, data, job, timeout, engine_env(False))
            cls = classify_diff(cand, orc)
            if orc[3] is not None or (orc[2] or "").strip():
                oracle_output += 1
        else:
            cls = classify_crash(cand[0])
        counts[cls] += 1
        if cls in store:
            sig = signature(cls, cand, orc)
            if sig not in seen and sig not in known:
                seen.add(sig)
                digest = hashlib.sha256(data + job.encode()).hexdigest()[:16]
                cls_dir = os.path.join(out_dir, cls)
                os.makedirs(cls_dir, exist_ok=True)
                with open(os.path.join(cls_dir, digest + ".jbig2"),
                          "wb") as fh:
                    fh.write(data)
                with open(os.path.join(cls_dir, digest + ".json"),
                          "w") as fh:
                    json.dump({"seed": name, "mutation": desc, "job": job,
                               "returncode": cand[0],
                               "oracle_returncode":
                               orc[0] if orc else None,
                               "last_output_line": last_line(cand[1]),
                               "panic_location":
                               fuzz_run.panic_location(_text(cand[1])),
                               "signature": sig}, fh, indent=2)
        if (i + 1) % 100 == 0:
            print("jbig2fuzz %d/%d: %s"
                  % (i + 1, iterations,
                     " ".join("%s=%d" % (c, counts[c]) for c in classes)))
    print("done: %d iterations: %s"
          % (iterations, " ".join("%s=%d" % (c, counts[c]) for c in classes)))
    if oracle and iterations > 0 and oracle_output == 0:
        print("error: harness failure: the oracle wrote no PDF and no log in "
              "%d iterations; nothing was compared" % iterations,
              file=sys.stderr)
        return 2
    return counts


def main(argv=None):
    ap = argparse.ArgumentParser(description="JBIG2 image fuzzer")
    ap.add_argument("--candidate", required=True,
                    help="candidate engine binary")
    ap.add_argument("--oracle", default=None,
                    help="reference pdfTeX: compare with it (differential)")
    ap.add_argument("--iterations", type=int, required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--out", required=True, help="output directory")
    ap.add_argument("--timeout", type=float, default=10,
                    help="per-run timeout in seconds")
    args = ap.parse_args(argv)
    fuzz_run.apply_fsize_limit()
    for b in filter(None, (args.candidate, args.oracle)):
        if not (os.path.isfile(b) or shutil.which(b)):
            print("error: engine not found: %s" % b, file=sys.stderr)
            return 2
    if args.iterations < 0:
        print("error: --iterations must be >= 0", file=sys.stderr)
        return 2
    return run_fuzz(args.candidate, args.out, args.iterations, args.seed,
                    args.timeout, oracle=args.oracle)


if __name__ == "__main__":
    result = main()
    sys.exit(0 if isinstance(result, dict) else result)
