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


def run_tex(doc, engine, workdir, trace=True):
    """Copy the source tree to `workdir`, run `engine -fmt=pdflatex` until the
    PDF stops changing and the log asks for no rerun (at most PASSES), then,
    with `trace`, one more pass through `capture.capture`. Returns
    (meta, Capture or None, pdf path or None)."""
    shutil.rmtree(workdir, ignore_errors=True)
    shutil.copytree(doc["dir"], workdir)
    entry = doc["entry"]
    stem = os.path.splitext(os.path.basename(entry))[0]
    pdf, logp = os.path.join(workdir, stem + ".pdf"), os.path.join(workdir, stem + ".log")
    env = dict(os.environ, **pcapture.TRACE_ENV)
    argv = [engine, f"-fmt={FMT}", "-interaction=nonstopmode", "-halt-on-error", f"-jobname={stem}", entry]
    meta = {"ok": False, "passes": 0, "traced": bool(trace)}
    t0 = time.time()
    previous = None
    for _ in range(PASSES):
        code, _, _, _, timed_out = rwc.run(argv, env=env, timeout=PASS_TIMEOUT, cwd=workdir)
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
        cap = pcapture.capture(os.path.join(workdir, entry), engine, workdir, fmt=FMT)
        meta["ok"] = cap.pdf_path is not None
        if not meta["ok"]:
            meta["why"] = "the traced pass wrote no PDF"
    meta["seconds"] = round(time.time() - t0, 2)
    return meta, cap, (pdf if meta["ok"] else None)


def oracle(doc, pdftex, cache, trace, tree_hash):
    """The P-T reference, cached: (meta, Capture or None, reference PDF path or None)."""
    version = engine_version(pdftex)
    key = sha(json.dumps({"tree": tree_hash, "entry": doc["entry"], "pdftex": version, "fmt": FMT,
                          "trace": pcapture.TRACE if trace else None, "env": pcapture.TRACE_ENV,
                          "passes": PASSES, "v": 1}, sort_keys=True))
    odir = os.path.join(cache, "pt-oracle", key[:2], key)
    meta_path = os.path.join(odir, "oracle.json")
    pdf, logz = os.path.join(odir, "reference.pdf"), os.path.join(odir, "log.gz")
    if not os.path.isfile(meta_path):
        work = os.path.join(odir, f"work-{os.getpid()}")  # two identical trees may run at once
        meta, cap, produced = run_tex(doc, pdftex, work, trace=trace)
        meta.update({"pdftex": version, "pinned": PINNED_PDFTEX in version, "key": key})
        tmp = f".{os.getpid()}.tmp"
        if meta["ok"]:
            shutil.copyfile(produced, pdf + tmp)
            os.replace(pdf + tmp, pdf)
            if cap is not None:
                with gzip.open(logz + tmp, "wt", encoding="latin-1", compresslevel=3) as f:
                    f.write(cap.log)
                os.replace(logz + tmp, logz)
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
    if trace:
        with gzip.open(logz, "rt", encoding="latin-1") as f:
            log = f.read()
        cap = pcapture.Capture(log, pcapture.split_boxes(log), pdf)
    return meta, cap, pdf


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
    """P-T1 passes when every shipout's box dump and the whole normalised log are identical."""
    rec = {"shipouts": [len(ref.boxes), len(cand.boxes)]}
    bad_box = next((i for i, (x, y) in enumerate(zip(ref.boxes, cand.boxes)) if x != y), None)
    if bad_box is None and len(ref.boxes) != len(cand.boxes):
        bad_box = min(len(ref.boxes), len(cand.boxes))
    rec["boxes_equal"] = bad_box is None
    if bad_box is not None:
        rec["first_shipout"] = bad_box + 1
        if bad_box < len(ref.boxes) and bad_box < len(cand.boxes):
            d = first_line_diff(ref.boxes[bad_box], cand.boxes[bad_box])
            rec["box_line"] = {"line": d[0], "oracle": d[1], "candidate": d[2]}
    d = first_line_diff(ref.log, cand.log)
    rec["log_equal"] = d is None
    if d:
        rec["log_line"] = {"line": d[0], "oracle": d[1], "candidate": d[2]}
    rec["ok"] = rec["boxes_equal"] and rec["log_equal"]
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
