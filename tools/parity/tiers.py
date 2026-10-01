"""DESIGN §1.1 gating tiers for the parity scoreboard.

  P-T1  at every `\\shipout`, the box dump and the whole `\\tracingall` log
        are identical to the pinned pdfTeX's, in PDF mode (the capture is
        `capture.py`, which becomes tools/lockstep's).
  P-T2  identical embedded font subsets, and identical page content streams
        after `qpdf --qdf --normalize-content=y --object-streams=disable`,
        with object numbers resolved away (every indirect reference is
        compared by what it points to, never by its number).

Expected data is only ever produced by the oracle pdfTeX (DESIGN §8); it is
cached outside the repository by the SHA-256 of the source tree, the pdfTeX
version and the capture settings, and never edited.

qpdf runs as an external program (Apache-2.0). The PDF object parser is
tools/visual-oracle/pdftext.py, which the scoreboard already uses.
"""

import collections
import gzip
import hashlib
import json
import os
import re
import shutil
import subprocess
import time
import zlib

import capture as pcapture
import pdftext
import run as rwc  # tools/real-world-corpus/run.py

PINNED_PDFTEX = "1.40.29"
DEFAULT_ORACLE = "/Library/TeX/texbin/pdftex"
FMT = "pdflatex"
PASSES = 6
PASS_TIMEOUT = 300
QPDF_ARGS = ["--qdf", "--normalize-content=y", "--object-streams=disable"]
TAG = re.compile(r"^[A-Z]{6}\+")
SNIP = 200
# What a run converts with \write18 (epstopdf.sty's `<name>-eps-converted-to.pdf`,
# and the same suffix for its other rules). Ghostscript stamps each conversion
# with the time and a fresh id, and pdfTeX copies those into the including PDF
# (/PTEX.InfoDict) and prints the file's date in the log, so two engines that
# each converted a figure differ in P-T1 and P-T2 for no typesetting reason.
# The oracle's conversions are therefore cached and handed to the candidate
# before its first pass (`seed`), with their times, so both runs see the same
# files and neither converts again.
GENERATED = re.compile(r"-converted-to\.pdf$")
# `<name>-<ext>-converted-to.pdf` was converted from `<name>.<ext>`
CONVERTED_FROM = re.compile(r"-([A-Za-z0-9]+)-converted-to\.pdf$")
# Why a traced pass has no complete log: the capture's time limit stopped it
# (a harness limit), or the engine ended early by itself (a crash).
TRACE_TIMEOUT = "the traced pass did not finish in the capture's {} s limit"
TRACE_CRASH = "the traced pass crashed: its log stops before the end of the run ({} s)"


def sha(b):
    return hashlib.sha256(b if isinstance(b, bytes) else b.encode("utf-8")).hexdigest()


def engine_version(exe):
    code, out, _, _, _ = rwc.run([exe, "--version"], timeout=30)
    lines = out.decode("utf-8", "replace").splitlines() if out else []
    return lines[0].strip() if lines else ""


def engine_kind(exe):
    """`flashtex-cli` for the shipped `flashtex` command line, else `tex` (a
    pdfTeX-compatible command line: pdfTeX itself, later the new engine)."""
    return "flashtex-cli" if engine_version(exe).lower().startswith("flashtex") else "tex"


def qpdf_version():
    q = shutil.which("qpdf")
    return engine_version(q) if q else None


# ----------------------------------------------------------------------------
# running a TeX engine to convergence, then one traced pass


def run_tex(doc, engine, workdir, trace=True, extra_env=None, seed=None):
    """Copy the source tree to `workdir`, run `engine -fmt=pdflatex` until the
    PDF stops changing and the log asks for no rerun (at most PASSES), then,
    with `trace`, one more pass through `capture.capture`. Every pass runs as
    the capture does: through the `pdftex` link, with SHELL_ESCAPE, and with
    `extra_env` (the candidate's alone; the oracle passes None). `seed`
    ({relative path: file}) is copied into the tree first, times kept: the
    oracle's conversions (GENERATED). Returns (meta, Capture or None, pdf
    path or None)."""
    shutil.rmtree(workdir, ignore_errors=True)
    shutil.copytree(doc["dir"], workdir)
    for rel, src in sorted((seed or {}).items()):
        dst = os.path.join(workdir, rel)
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        shutil.copy2(src, dst)
    entry = doc["entry"]
    stem = os.path.splitext(os.path.basename(entry))[0]
    pdf, logp = os.path.join(workdir, stem + ".pdf"), os.path.join(workdir, stem + ".log")
    args = ["-interaction=nonstopmode", "-halt-on-error", f"-jobname={stem}", pcapture.first_line(entry)]
    meta = {"ok": False, "passes": 0, "traced": bool(trace)}
    t0 = time.time()
    previous = None
    for _ in range(PASSES):
        code, timed_out = pcapture.run_engine(engine, FMT, args, workdir, extra_env, timeout=PASS_TIMEOUT)
        meta["passes"] += 1
        if timed_out or code != 0 or not os.path.isfile(pdf):
            txt = rwc.read_text(logp) if os.path.isfile(logp) else ""
            m = re.search(r"^(! .*)$", txt, re.M)
            meta["exit"] = code
            meta["why"] = (f"exit {code}" + (" (timeout)" if timed_out else "") + (f": {m.group(1)[:160]}" if m else ""))
            meta["seconds"] = round(time.time() - t0, 2)
            return meta, None, None
        cur = rwc.sha256_file(pdf)
        if cur == previous and "Rerun to get" not in rwc.read_text(logp):
            meta["ok"] = True
            break
        previous = cur
    else:
        meta["why"] = f"did not converge in {PASSES} passes"
    meta["exit"] = 0
    cap = None
    if trace and meta["ok"]:
        converged = pdf + ".converged"
        shutil.copyfile(pdf, converged)
        t1 = time.time()
        cap = pcapture.capture(os.path.join(workdir, entry), engine, workdir, fmt=FMT, extra_env=extra_env)
        if not (cap.complete if cap.log is None else trace_complete(cap.log)):
            # killed (the capture's timeout) or crashed mid-run: the log is cut
            # short and the PDF may be partial; P-T1 can't be judged, and P-T2
            # uses the converged pass's PDF, which the traced pass only repeats
            took = time.time() - t1
            meta["trace_incomplete"] = (TRACE_TIMEOUT.format(pcapture.TIMEOUT) if took >= pcapture.TIMEOUT - 1
                                        else TRACE_CRASH.format(round(took, 1)))
            cap = None
            os.replace(converged, pdf)
        else:
            os.remove(converged)
            meta["ok"] = cap.pdf_path is not None
            if not meta["ok"]:
                meta["why"] = "the traced pass wrote no PDF"
    meta["seconds"] = round(time.time() - t0, 2)
    return meta, cap, (pdf if meta["ok"] else None)


def trace_complete(log):
    """Whether a traced log ran to its end: pdfTeX's last lines always include
    `Output written on …` or `No pages of output.`."""
    tail = log[-65536:]
    return "\nOutput written on " in tail or "\nNo pages of output." in tail


# Bumped whenever what an oracle cache entry holds changes, so an entry made
# under the old rule is never reused: v5 keeps a conversion's run-written input
# (keep_generated) and runs every pass after capture.SEED.
ORACLE_CACHE_V = 5


def oracle_key(doc, version, trace, tree_hash, v=ORACLE_CACHE_V):
    return sha(json.dumps({"tree": tree_hash, "entry": doc["entry"], "pdftex": version, "fmt": FMT,
                           "trace": pcapture.TRACE if trace else None, "env": pcapture.TRACE_ENV,
                           "passes": PASSES, "shell_escape": pcapture.SHELL_ESCAPE, "argv0": pcapture.PROGRAM,
                           "seed": pcapture.SEED, "v": v}, sort_keys=True))


def oracle(doc, pdftex, cache, trace, tree_hash, load_log=True):
    """The P-T reference, cached: (meta, Capture or None, reference PDF path or None).
    With `load_log=False` the traced log stays on disk (no Capture); its size
    is `meta["log_chars"]` either way (see `log_chars`)."""
    version = engine_version(pdftex)
    key = oracle_key(doc, version, trace, tree_hash)
    odir = os.path.join(cache, "pt-oracle", key[:2], key)
    meta_path = os.path.join(odir, "oracle.json")
    pdf, logz = os.path.join(odir, "reference.pdf"), os.path.join(odir, "log.gz")
    if not os.path.isfile(meta_path):
        work = os.path.join(odir, f"work-{os.getpid()}")  # two identical trees may run at once
        meta, cap, produced = run_tex(doc, pdftex, work, trace=trace)
        meta.update({"pdftex": version, "pinned": PINNED_PDFTEX in version, "key": key})
        tmp = f".{os.getpid()}.tmp"
        if meta["ok"]:
            meta["generated"] = keep_generated(doc["dir"], work, os.path.join(odir, "generated"))
            shutil.copyfile(produced, pdf + tmp)
            os.replace(pdf + tmp, pdf)
            if cap is not None and cap.log is None:  # over capture.MAX_LOG_BYTES: never read, not kept
                meta["log_chars"], meta["log_unread"] = cap.size, True
            elif cap is not None:
                with gzip.open(logz + tmp, "wt", encoding="latin-1", compresslevel=3) as f:
                    f.write(cap.log)
                os.replace(logz + tmp, logz)
                meta["log_chars"] = len(cap.log)
        shutil.rmtree(work, ignore_errors=True)
        with open(meta_path + tmp, "w", encoding="utf-8") as f:
            json.dump(meta, f, indent=1)
        os.replace(meta_path + tmp, meta_path)  # written last: its presence means the entry is complete
        meta["cached"] = False
    else:
        with open(meta_path, encoding="utf-8") as f:
            meta = json.load(f)
        meta["cached"] = True
    if not meta.get("ok"):
        return meta, None, None
    cap = None
    if trace and not meta.get("trace_incomplete") and "log_chars" not in meta:
        meta["log_chars"] = log_chars(logz)  # an entry cached before the size was recorded
    budget = pcapture.MAX_LOG_BYTES
    over = meta.get("log_unread") or bool(budget and (meta.get("log_chars") or 0) > budget)
    if trace and load_log and not meta.get("trace_incomplete") and not over:  # size known before the read
        with gzip.open(logz, "rt", encoding="latin-1") as f:
            log = f.read()
        cap = pcapture.Capture(log, pcapture.split_boxes(log), pdf)
    return meta, cap, pdf


def keep_generated(src, work, dest):
    """Copy the files a run converted (GENERATED, absent from the source tree
    `src`) from `work` to `dest`, times kept; returns their relative paths.
    A conversion's input that the run wrote itself (`filecontents` writing
    `a.eps`, then `a-eps-converted-to.pdf`: grfguide.tex) is kept with it,
    since epstopdf logs the input's date."""
    out = []
    for root, _, files in os.walk(work):
        for name in files:
            rel = os.path.relpath(os.path.join(root, name), work)
            if GENERATED.search(name) and not os.path.exists(os.path.join(src, rel)):
                keep = [rel]
                m = CONVERTED_FROM.search(rel)
                if m:
                    source = rel[:m.start()] + "." + m.group(1)
                    if os.path.isfile(os.path.join(work, source)) and not os.path.exists(os.path.join(src, source)):
                        keep.append(source)
                for r in keep:
                    os.makedirs(os.path.dirname(os.path.join(dest, r)), exist_ok=True)
                    shutil.copy2(os.path.join(work, r), os.path.join(dest, r))
                    out.append(r)
    return sorted(set(out))


def oracle_seed(meta, cache):
    """{relative path: cached file} of the oracle's conversions, for `run_tex(seed=)`."""
    key = meta.get("key")
    if not key or not meta.get("generated"):
        return {}
    base = os.path.join(cache, "pt-oracle", key[:2], key, "generated")
    return {rel: os.path.join(base, rel) for rel in meta["generated"] if os.path.isfile(os.path.join(base, rel))}


def log_chars(logz, chunk=1 << 24):
    """Length of a gzipped latin-1 log (one byte per character), streamed, so
    a multi-gigabyte log is measured without being held in memory. (The
    gzip trailer's ISIZE is the length mod 2**32, so it can't be used.)"""
    n = 0
    with gzip.open(logz, "rb") as f:
        while True:
            b = f.read(chunk)
            if not b:
                return n
            n += len(b)


# ----------------------------------------------------------------------------
# P-T1


def first_line_diff(a, b):
    """(1-based line number, line of a, line of b) of the first difference, or None."""
    if a == b:
        return None
    la, lb = a.split("\n"), b.split("\n")
    for i, (x, y) in enumerate(zip(la, lb)):
        if x != y:
            return i + 1, x[:SNIP], y[:SNIP]
    n = min(len(la), len(lb))
    return n + 1, (la[n][:SNIP] if n < len(la) else "<end of log>"), (lb[n][:SNIP] if n < len(lb) else "<end of log>")


def compare_pt1(ref, cand):
    """P-T1 passes when every shipout's box dump and the whole log are
    identical once the ruled accounting is split off (`capture.split_accounting`).
    The accounting lines are compared too, but only reported (`accounting`),
    never gating; line numbers refer to the log with accounting removed."""
    ref_boxes, cand_boxes = ref.boxes, cand.boxes  # box dumps hold no accounting: compared as they are
    rec = {"shipouts": [len(ref_boxes), len(cand_boxes)]}
    bad_box = next((i for i, (x, y) in enumerate(zip(ref_boxes, cand_boxes)) if x != y), None)
    if bad_box is None and len(ref_boxes) != len(cand_boxes):
        bad_box = min(len(ref_boxes), len(cand_boxes))
    rec["boxes_equal"] = bad_box is None
    if bad_box is not None:
        rec["first_shipout"] = bad_box + 1
        if bad_box < len(ref_boxes) and bad_box < len(cand_boxes):
            d = first_line_diff(ref_boxes[bad_box], cand_boxes[bad_box])
            rec["box_line"] = {"line": d[0], "oracle": d[1], "candidate": d[2]}
    ref_log, ref_acc = pcapture.split_accounting(ref.log)
    cand_log, cand_acc = pcapture.split_accounting(cand.log)
    d = first_line_diff(ref_log, cand_log)
    rec["log_equal"] = d is None
    if d:
        rec["log_line"] = {"line": d[0], "oracle": d[1], "candidate": d[2]}
    rec["ok"] = rec["boxes_equal"] and rec["log_equal"]
    acc = {"lines": [len(ref_acc), len(cand_acc)], "equal": ref_acc == cand_acc}
    if not acc["equal"]:
        k = next((i for i, (x, y) in enumerate(zip(ref_acc, cand_acc)) if x != y), min(len(ref_acc), len(cand_acc)))
        acc["first"] = {"oracle": ref_acc[k][:SNIP] if k < len(ref_acc) else "<none>",
                        "candidate": cand_acc[k][:SNIP] if k < len(cand_acc) else "<none>"}
    rec["accounting"] = acc
    return rec


# ----------------------------------------------------------------------------
# P-T2


class Graph:
    """Canonical, number-free view of a qpdf-normalised PDF. Every indirect
    object is named by a hash of what it contains (its dictionary with each
    reference replaced by the referent's hash, plus its decoded stream), so
    two PDFs that differ only in object numbering produce the same hashes.
    `/Parent` (a back edge) and `/Length` (a serialisation detail) are left
    out; subset tags (`ABCDEF+`) are normalised in names and font programs."""

    SKIP = {"Parent", "Length"}

    def __init__(self, doc):
        self.doc = doc
        self.memo = {}
        self.fonts = []
        self.tags = sorted({TAG.match(str(o[k])).group(0) for o, _ in doc.objects.values() if isinstance(o, dict)
                            for k in ("BaseFont", "FontName") if k in o and TAG.match(str(o[k]))})

    def stream(self, num):
        obj, raw = self.doc.objects.get(num, (None, None))
        if raw is None:
            return b""
        try:
            data = self.doc.decode(obj, raw)
        except (pdftext.PdfError, zlib.error):
            data = raw  # a filter qpdf keeps (DCT, JPX, ...): compare the encoded bytes
        for t in self.tags:
            data = data.replace(t.encode("latin-1"), b"SUBSET+")
        return data

    def node(self, num):
        if num in self.memo:
            return self.memo[num]
        self.memo[num] = "<cycle>"
        obj, raw = self.doc.objects.get(num, (None, None))
        s = self.canon(obj)
        if raw is not None:
            s += " stream " + sha(self.stream(num))
        h = sha(s)[:40]
        self.memo[num] = h
        if isinstance(obj, dict) and obj.get("Type") == "Font":
            self.fonts.append(self.font_key(obj))
        return h

    def canon(self, x):
        if isinstance(x, pdftext.Ref):
            return "@" + self.node(x[0])
        if isinstance(x, dict):
            return "<<" + " ".join(f"/{k} {self.canon(v)}" for k, v in sorted(x.items()) if k not in self.SKIP) + ">>"
        if isinstance(x, list):
            return "[" + " ".join(self.canon(v) for v in x) + "]"
        if isinstance(x, pdftext.Name):
            return "/" + TAG.sub("SUBSET+", str(x))
        if isinstance(x, bytes):
            return "(" + x.hex() + ")"
        if isinstance(x, float) and x.is_integer():
            return str(int(x))
        return repr(x)

    def font_key(self, f):
        """(name without subset tag, subtype, hash of the embedded program)."""
        r = self.doc.resolve
        name = TAG.sub("", str(f.get("BaseFont", f.get("Name", ""))))
        sub = str(f.get("Subtype", ""))
        if sub == "Type3":
            return name or "<Type3>", sub, sha(self.canon(f.get("CharProcs")) + self.canon(f.get("FontMatrix")))[:40]
        desc = r(f.get("FontDescriptor"))
        if desc is None and sub == "Type0":
            kids = r(f.get("DescendantFonts")) or []
            desc = r((r(kids[0]) or {}).get("FontDescriptor")) if kids else None
        for k in ("FontFile", "FontFile2", "FontFile3"):
            if isinstance(desc, dict) and k in desc:
                return name, sub, self.canon(desc[k]).lstrip("@")
        return name, sub, "not embedded"


def normalise_pdf(pdf, out):
    """`qpdf --qdf --normalize-content=y --object-streams=disable`; exit 3 is
    qpdf's "succeeded with warnings"."""
    q = shutil.which("qpdf")
    if not q:
        raise pdftext.PdfError("qpdf not found on PATH")
    p = subprocess.run([q] + QPDF_ARGS + [pdf, out], capture_output=True, timeout=300)
    if p.returncode not in (0, 3) or not os.path.isfile(out):
        raise pdftext.PdfError(f"qpdf exit {p.returncode}: {p.stderr.decode('utf-8', 'replace')[:200]}")


def signature(pdf, work, name):
    """Pages (content bytes, resources hash, media box) and the font multiset."""
    os.makedirs(work, exist_ok=True)
    out = os.path.join(work, name + ".qdf.pdf")
    normalise_pdf(pdf, out)
    doc = pdftext.PdfDocument.load(out)
    g = Graph(doc)
    pages = []
    for p in doc.pages():
        c = doc.resolve(p.get("Contents"))
        refs = c if isinstance(c, list) else [p.get("Contents")]
        content = b"\n".join(g.stream(x[0]) for x in refs if isinstance(x, pdftext.Ref))
        pages.append({"content": content, "resources": sha(g.canon(p.get("Resources"))),
                      "mediabox": g.canon(doc.resolve(p.get("MediaBox")))})
    os.remove(out)
    return pages, collections.Counter(g.fonts)


def compare_pt2(ref_pdf, cand_pdf, work):
    """P-T2 on two PDFs; every sub-check is reported, `ok` only when all pass."""
    try:
        rp, rf = signature(ref_pdf, work, "oracle")
    except (pdftext.PdfError, OSError, subprocess.SubprocessError) as e:
        return {"ok": False, "why": f"oracle PDF: {e}"[:300]}
    try:
        cp, cf = signature(cand_pdf, work, "candidate")
    except (pdftext.PdfError, OSError, subprocess.SubprocessError) as e:
        return {"ok": False, "why": f"candidate PDF: {e}"[:300]}
    rec = {"pages": [len(rp), len(cp)]}
    by_name_r = {n: (s, h) for n, s, h in rf}
    by_name_c = {n: (s, h) for n, s, h in cf}
    rec["fonts"] = {"oracle": sum(rf.values()), "candidate": sum(cf.values()),
                    "missing": sorted(set(by_name_r) - set(by_name_c))[:12],
                    "extra": sorted(set(by_name_c) - set(by_name_r))[:12],
                    "different_program": sorted(n for n in set(by_name_r) & set(by_name_c)
                                                if by_name_r[n] != by_name_c[n])[:12]}
    rec["fonts_equal"] = rf == cf
    equal = 0
    first = None
    for i, (a, b) in enumerate(zip(rp, cp)):
        what = [k for k in ("content", "resources", "mediabox") if a[k] != b[k]]
        if not what:
            equal += 1
            continue
        if first is None:
            first = {"page": i + 1, "differs": what}
            if "content" in what:
                d = first_line_diff(a["content"].decode("latin-1"), b["content"].decode("latin-1"))
                first["content_line"] = {"line": d[0], "oracle": d[1], "candidate": d[2]}
    rec["pages_equal"] = equal
    if first is None and len(rp) != len(cp):
        first = {"page": min(len(rp), len(cp)) + 1, "differs": ["page count"]}
    if first:
        rec["first_page"] = first
    rec["content_equal"] = len(rp) == len(cp) and equal == len(rp)
    rec["ok"] = rec["fonts_equal"] and rec["content_equal"]
    return rec
