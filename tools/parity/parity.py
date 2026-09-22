#!/usr/bin/env python3
"""Parity scoreboard: how many documents does FlashTeX typeset exactly like
pdfLaTeX, and what stops the rest?

Oracle tooling only (Python 3 standard library). pdflatex (MacTeX) is the
oracle and never runs in the product path; the producer under test is the
shipped command line, `flashtex build` (the same exact-route PDF the Mac app
exports), run on the untouched source tree.

Parity levels, per document, strictest last. Each is a **cumulative** gate:
a document is "at L3" only when it also passes L0, L1 and L2.

  L0  compiles: pdflatex compiles it with `-halt-on-error` (the document is in
      the denominator only then) and FlashTeX renders it with **zero
      error-severity diagnostics** (`flashtex build --json`: exit 0,
      `summary.errors == 0`, a PDF and a display list written).
  L1  same page count.
  L2  same glyphs on every page: the multiset of glyph characters per page is
      equal (identity via `glyphkeys`: reference glyph names and candidate
      cluster text both reduced to NFKD Unicode). Order and position are L3's.
  L3  every glyph origin within 0.5 bp: a one-to-one matching of same-
      character glyphs with |dx| <= 0.5 and |dy| <= 0.5 bp covers every glyph
      of every page.
  L4  pixel-identical within tolerance: both PDFs rasterised at 144 dpi
      (8-bit grey); a pixel differs when |delta| > RASTER_DELTA grey levels;
      every page must have at most RASTER_FRACTION of its pixels differing.

The headline is the share of documents at L3. Also reported per tier: the
share at each level, each check's independent pass rate, the glyph-position
error distribution (p50/p95/max of max(|dx|,|dy|) over glyphs the word
alignment of `tools/visual-oracle/rank.py` pairs), the first diverging page,
and the causes that block each document's next level, ranked by how many
documents they block (see `causes_for`).

Tiers:
  fixtures  the committed documents under fixtures/real-world and
            fixtures/divergence-probes against their committed reference
            PDFs (no TeX installation needed; deterministic; CI).
  arxiv, templates
            public sources pinned by tools/parity/corpus/*.json and fetched
            by tools/parity/corpus.py into a cache outside the repository;
            the reference is made now by the local pdflatex and cached by the
            SHA-256 of the source tree (+ pdflatex version and argv).

    python3 tools/parity/parity.py --tier fixtures
    python3 tools/parity/parity.py --tier arxiv --tier templates -j 8
    python3 tools/parity/parity.py --tier fixtures --check-baseline tools/parity/baseline-fixtures.json
"""

import argparse
import collections
import concurrent.futures
import datetime as _dt
import hashlib
import json
import math
import os
import platform
import re
import shutil
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(REPO, "tools", "visual-oracle"))
sys.path.insert(0, os.path.join(REPO, "tools", "real-world-corpus"))

import corpus as pcorpus  # noqa: E402  (tools/parity/corpus.py)
import definers  # noqa: E402
import glyphkeys  # noqa: E402
import fontenv  # noqa: E402
import pdftext  # noqa: E402
import rank  # noqa: E402
import run as rwc  # noqa: E402  (tools/real-world-corpus/run.py)
from cumulative import ENV_RE  # noqa: E402

LEVELS = ("L0", "L1", "L2", "L3", "L4")
POS_TOL = 0.5          # bp, per axis (L3)
RASTER_DELTA = 64      # grey levels (L4)
RASTER_FRACTION = 2e-4  # of a page's pixels (L4): 388 px on a 144 dpi US-letter page
ORACLE_PASSES = 6
ORACLE_TIMEOUT = 300
CANDIDATE_TIMEOUT = 180
FIXTURE_ROOTS = ("fixtures/real-world", "fixtures/divergence-probes")
DEFAULT_FLASHTEX = os.path.join(REPO, "crates", "flashtex-cli", "target", "release", "flashtex")
# Diagnostic codes that describe fonts/outline provenance, not typesetting:
# they cannot block L0-L3 and are L4 causes only.
FONT_NOTE_CODES = {"math_resource_profile", "math_metrics_opentype", "font_substitution", "font_face_substituted"}
# Math-extension fonts: pdfTeX's cmex glyphs hang from their origin (TFM
# height ~0, all depth), an OpenType MATH variant sits on the axis, so the
# same ink has a different *vertical* origin in the two PDFs. Without the
# outlines that offset cannot be checked, so these glyphs are held to
# |dx| <= POS_TOL and |dy| <= EXT_DY_EM x font size (reported as
# `x_only_glyphs`).
EXT_FONT = re.compile(r"^(CMEX|LMEX|LMMathExtension|EUEX|TXEX|PXEX|ESINT|MNSYMBOLE|STIXSIZE|XITSMATH)", re.I)
EXT_DY_EM = 2.0
UNSUPPORTED_WARNING = re.compile(
    r"not supported|not implemented|unsupported|unknown|ignored|recognised but|not yet|cannot|no glyph|missing|undefined",
    re.I)


# ----------------------------------------------------------------------------
# small utilities


def percentile(values, q):
    """Nearest-rank percentile (q in [0, 100]) of a non-empty sorted list."""
    if not values:
        return None
    k = max(0, min(len(values) - 1, int(math.ceil(q / 100.0 * len(values))) - 1))
    return values[k]


def dist(values):
    v = sorted(values)
    if not v:
        return {"n": 0, "p50": None, "p95": None, "max": None}
    return {"n": len(v), "p50": round(percentile(v, 50), 4), "p95": round(percentile(v, 95), 4),
            "max": round(v[-1], 4)}


def tree_hash(root):
    """SHA-256 over every file below `root` (relative path + content hash)."""
    h = hashlib.sha256()
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames.sort()
        for f in sorted(filenames):
            if f == ".parity-unpacked":
                continue
            p = os.path.join(dirpath, f)
            h.update(os.path.relpath(p, root).encode("utf-8") + b"\0")
            h.update(rwc.sha256_file(p).encode("ascii") + b"\n")
    return h.hexdigest()


# ----------------------------------------------------------------------------
# glyph atoms


def reference_atoms(glyphs):
    """(char, x, y, glyph_index) for every character of every reference glyph.

    A vertical run of extensible-delimiter pieces (`parenlefttp`,
    `parenleftex`..., `braceex`, `vextendsingle`, `radicalvertex`: pdfTeX
    stacks them, consecutive in the content stream, at one x) is one
    character at its first piece, as the candidate's one cluster is."""
    out = []
    run = None  # index in `out` of the atom a piece run was merged into
    for gi, g in enumerate(glyphs):
        piece = glyphkeys.is_piece(g.get("name"))
        if piece and run is not None and abs(g["x"] - out[run][1]) <= 1.0:
            # the stack's height widens the vertical allowance of its one atom
            ch, x, y, gj, ext = out[run]
            out[run] = (ch, x, y, gj, max(ext, abs(g["y_top"] - y) + EXT_DY_EM * max(g.get("size") or 10.0, 1.0)))
            continue
        ext = EXT_DY_EM * max(g.get("size") or 10.0, 1.0) if EXT_FONT.match(g.get("font") or "") else 0.0
        run = None
        for ch in glyphkeys.chars(glyphkeys.reference_key(g)):
            out.append((ch, g["x"], g["y_top"], gi, ext))
            if piece:
                run = len(out) - 1
    return out


def candidate_atoms(glyphs):
    """Same for the candidate; a cluster several glyphs share contributes
    its text once, at its first glyph."""
    out, seen = [], set()
    for gi, g in enumerate(glyphs):
        cl = g.get("cluster")
        if cl is not None:
            if cl in seen:
                continue
            seen.add(cl)
        for ch in glyphkeys.chars(glyphkeys.candidate_key(g)):
            out.append((ch, g["x"], g["y_top"], gi, 0.0))
    return out


def multiset_diff(ref_atoms, cand_atoms):
    r = collections.Counter(a[0] for a in ref_atoms)
    c = collections.Counter(a[0] for a in cand_atoms)
    return r - c, c - r


def match_within(ref_atoms, cand_atoms, tol=POS_TOL):
    """Greedy one-to-one matching of same-character atoms within `tol` bp on
    each axis. Returns (unmatched reference indices, unmatched candidate
    indices). Two same-character glyphs never sit within 0.5 bp of each other
    in a real page, so greedy nearest is the matching."""
    cell = max(tol * 2, 1e-6)
    grid = collections.defaultdict(list)
    by_char = collections.defaultdict(list)
    for j, a in enumerate(cand_atoms):
        ch, x, y = a[0], a[1], a[2]
        grid[(ch, int(math.floor(x / cell)), int(math.floor(y / cell)))].append(j)
        by_char[ch].append(j)
    used = set()
    unmatched = []
    # extension-font glyphs last, so a text glyph never loses its partner to one
    order = sorted(range(len(ref_atoms)), key=lambda i: ref_atoms[i][4] > 0)
    for i in order:
        ch, x, y, _, ext = ref_atoms[i]
        best, bestd = None, None
        if ext:
            cands = by_char.get(ch, ())
        else:
            cx, cy = int(math.floor(x / cell)), int(math.floor(y / cell))
            cands = [j for gx in (cx - 1, cx, cx + 1) for gy in (cy - 1, cy, cy + 1) for j in grid.get((ch, gx, gy), ())]
        for j in cands:
            if j in used:
                continue
            x2, y2 = cand_atoms[j][1], cand_atoms[j][2]
            if abs(x2 - x) > tol or abs(y2 - y) > (ext or tol):
                continue
            d = max(abs(x2 - x), abs(y2 - y) if not ext else 0.0) + (abs(y2 - y) * 1e-3 if ext else 0.0)
            if bestd is None or d < bestd:
                best, bestd = j, d
        if best is None:
            unmatched.append(i)
        else:
            used.add(best)
    return sorted(unmatched), [j for j in range(len(cand_atoms)) if j not in used]


def word_view(glyphs):
    return [{k: g[k] for k in ("text", "x", "y_top", "advance", "size", "font", "bt")} for g in glyphs]


def aligned_errors(ref_glyphs, ref_atoms, cand_glyphs, cand_atoms):
    """Glyph-position errors over the word pairs `rank.aligned_pairs` finds.

    Within an aligned word pair whose character sequences are equal, the k-th
    character of one is paired with the k-th of the other; the error is
    max(|dx|, |dy|) in bp, counted once per reference glyph. Returns
    (errors, pairs, ref_words, cand_words)."""
    ref_words = pdftext.words_from_glyphs(word_view(ref_glyphs))
    cand_words = pdftext.words_from_glyphs(word_view(cand_glyphs))
    pairs, _, _ = rank.aligned_pairs(ref_words, cand_words)
    r_by_glyph = collections.defaultdict(list)
    for a in ref_atoms:
        r_by_glyph[a[3]].append(a)
    c_by_glyph = collections.defaultdict(list)
    for a in cand_atoms:
        c_by_glyph[a[3]].append(a)
    errors = []
    for i, j in pairs:
        rw, cw = ref_words[i], cand_words[j]
        ra = [a for gi in range(rw["glyph_index"], rw["glyph_index"] + rw["glyphs"]) for a in r_by_glyph.get(gi, ())]
        ca = [a for gi in range(cw["glyph_index"], cw["glyph_index"] + cw["glyphs"]) for a in c_by_glyph.get(gi, ())]
        if not ra or [a[0] for a in ra] != [a[0] for a in ca]:
            continue
        seen = set()
        for a, b in zip(ra, ca):
            if a[3] in seen:
                continue
            seen.add(a[3])
            # extension-font glyphs: horizontal only (see EXT_FONT)
            errors.append(abs(b[1] - a[1]) if a[4] else max(abs(b[1] - a[1]), abs(b[2] - a[2])))
    return errors, pairs, ref_words, cand_words


# ----------------------------------------------------------------------------
# documents


def fixture_reference(d):
    """Pinned reference of a committed fixture: `reference.pdf`, else the
    harness's MacTeX regeneration `reference-mactex2026.pdf` (hw1: the user's
    own byte-immutable PDF is a different TeX Live build), else
    `*-reference.pdf`."""
    for name in ("reference.pdf", "reference-mactex2026.pdf"):
        p = os.path.join(d, name)
        if os.path.isfile(p):
            return p
    others = sorted(f for f in os.listdir(d) if f.endswith("-reference.pdf"))
    return os.path.join(d, others[0]) if others else None


def reference_engine(d):
    try:
        with open(os.path.join(d, "reference.json"), encoding="utf-8") as f:
            j = json.load(f)
        return j.get("reference_engine") or j.get("pdflatex") or "unknown"
    except (OSError, ValueError):
        return "unknown"


def fixture_documents(roots=FIXTURE_ROOTS, only=()):
    docs = []
    for root in roots:
        base = os.path.join(REPO, root)
        for fx in rwc.discover_fixtures(base):
            did = f"{os.path.basename(root)}/{fx['id']}"
            if only and fx["id"] not in only and did not in only:
                continue
            ref = fixture_reference(fx["dir"])
            docs.append({"id": did, "tier": "fixtures", "dir": fx["dir"], "entry": fx["entry"],
                         "reference": ref, "reference_engine": reference_engine(fx["dir"]), "problem": None})
    return docs


def oracle_reference(doc, texbin, cache, log):
    """The pdflatex reference for a fetched document, cached by source hash.

    Runs `pdflatex -interaction=nonstopmode -halt-on-error` in a scratch copy
    with SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 until the PDF stops changing
    and the log asks for no rerun (at most ORACLE_PASSES). A failure is cached
    too: the document is then outside the denominator (pdflatex does not
    compile it) and reported as such."""
    exe, version = rwc.pdflatex_version(texbin)
    if exe is None:
        return None, {"ok": False, "why": f"no pdflatex in {texbin}"}
    argv = [exe, "-interaction=nonstopmode", "-halt-on-error", doc["entry"]]
    key = hashlib.sha256(json.dumps({"tree": tree_hash(doc["dir"]), "entry": doc["entry"], "pdflatex": version,
                                     "argv": argv[1:], "passes": ORACLE_PASSES, "v": 1}).encode()).hexdigest()
    odir = os.path.join(cache, "oracle", key[:2], key)
    meta_path = os.path.join(odir, "oracle.json")
    if os.path.isfile(meta_path):
        with open(meta_path, encoding="utf-8") as f:
            meta = json.load(f)
        meta["cached"] = True
        pdf = os.path.join(odir, "reference.pdf")
        return (pdf if meta.get("ok") and os.path.isfile(pdf) else None), meta
    work = os.path.join(odir, "work")
    shutil.rmtree(work, ignore_errors=True)
    shutil.copytree(doc["dir"], work)
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
    stem = os.path.splitext(doc["entry"])[0]
    produced = os.path.join(work, stem + ".pdf")
    logpath = os.path.join(work, stem + ".log")
    meta = {"ok": False, "key": key, "pdflatex": version, "argv": ["SOURCE_DATE_EPOCH=0", "FORCE_SOURCE_DATE=1",
            "pdflatex"] + argv[1:], "passes": 0, "seconds": 0.0}
    previous = None
    t0 = time.time()
    for _ in range(ORACLE_PASSES):
        code, _, _, _, timed_out = rwc.run(argv, env=env, timeout=ORACLE_TIMEOUT, cwd=work)
        meta["passes"] += 1
        if timed_out or code != 0 or not os.path.isfile(produced):
            txt = rwc.read_text(logpath) if os.path.isfile(logpath) else ""
            m = re.search(r"^(! .*)$", txt, re.M)
            meta["why"] = (f"pdflatex exit {code}" + (" (timeout)" if timed_out else "")
                           + (f": {m.group(1)[:160]}" if m else ""))
            break
        cur = rwc.sha256_file(produced)
        if cur == previous and "Rerun to get" not in rwc.read_text(logpath):
            meta["ok"] = True
            break
        previous = cur
    else:
        meta["why"] = f"did not converge in {ORACLE_PASSES} passes"
    meta["seconds"] = round(time.time() - t0, 2)
    os.makedirs(odir, exist_ok=True)
    pdf = os.path.join(odir, "reference.pdf")
    if meta["ok"]:
        shutil.copyfile(produced, pdf)
        meta["sha256"] = rwc.sha256_file(pdf)
        m = re.search(r"Output written on .*?\((\d+) pages?", rwc.read_text(logpath))
        meta["pages"] = int(m.group(1)) if m else None
    shutil.rmtree(work, ignore_errors=True)
    with open(meta_path, "w", encoding="utf-8") as f:
        json.dump(meta, f, indent=1)
    meta["cached"] = False
    return (pdf if meta["ok"] else None), meta


def run_candidate(doc, flashtex, font_dirs, env, out_dir):
    """`flashtex build` on the untouched source tree; outputs go to out_dir."""
    os.makedirs(out_dir, exist_ok=True)
    pdf = os.path.join(out_dir, "candidate.pdf")
    v2 = os.path.join(out_dir, "candidate.v2.json")
    for p in (pdf, v2):
        if os.path.exists(p):
            os.remove(p)
    argv = [flashtex, "build", os.path.join(doc["dir"], doc["entry"]), "-o", pdf, "--v2", v2, "--json",
            "--diagnostics", "short", "--color", "never"]
    for d in fontenv.split(font_dirs):
        argv += ["--font-dir", d]
    code, out, err, secs, timed_out = rwc.run(argv, env=env, timeout=CANDIDATE_TIMEOUT, cwd=doc["dir"])
    rec = {"exit": code, "timed_out": timed_out, "seconds": round(secs, 3), "status": None, "errors": None,
           "warnings": None, "pages": None, "diagnostics": [], "pdf": None, "v2": None,
           "stderr_tail": err.decode("utf-8", "replace")[-800:]}
    try:
        rep = json.loads(out.decode("utf-8", "replace"))
    except ValueError:
        rep = None
    if isinstance(rep, dict):
        rec.update({"status": rep.get("status"), "pages": rep.get("pages"),
                    "errors": (rep.get("summary") or {}).get("errors"),
                    "warnings": (rep.get("summary") or {}).get("warnings"),
                    "diagnostics": rep.get("diagnostics") or [], "version": rep.get("version")})
    if os.path.isfile(pdf):
        rec["pdf"] = pdf
    if os.path.isfile(v2):
        rec["v2"] = v2
    return rec


# ----------------------------------------------------------------------------
# rasters (L4)


def raster_page_diff(ref_pgm, cand_pgm):
    rw, rh, rp = rwc.read_pgm(ref_pgm)
    cw, ch, cp = rwc.read_pgm(cand_pgm)
    if (rw, rh) != (cw, ch):
        return {"size_mismatch": [[rw, rh], [cw, ch]], "ok": False}
    if rp == cp:
        return {"differing": 0, "fraction": 0.0, "ok": True}
    t = RASTER_DELTA
    n = sum(1 for a, b in zip(rp, cp) if a - b > t or b - a > t)
    frac = n / float(rw * rh)
    return {"differing": n, "fraction": round(frac, 6), "ok": frac <= RASTER_FRACTION}


def raster_compare(ref_pdf, cand_pdf, work):
    rast = rwc.find_rasterizer()
    if not rast[0] or rast[0] == "sips":
        return {"available": False, "why": "no pdftoppm/gs"}
    rf, _, n1 = rwc.rasterize(rast, ref_pdf, os.path.join(work, "ref"))
    cf, _, n2 = rwc.rasterize(rast, cand_pdf, os.path.join(work, "cand"))
    if n1 or n2 or len(rf) != len(cf):
        return {"available": False, "why": (n1 or n2 or f"page files {len(rf)} vs {len(cf)}")[:200]}
    pages = [raster_page_diff(a, b) for a, b in zip(rf, cf)]
    for p in rf + cf:
        os.remove(p)
    worst = max((p.get("fraction", 1.0) for p in pages), default=0.0)
    return {"available": True, "rasterizer": rast[0], "dpi": rwc.DPI, "pages": pages,
            "ok": all(p["ok"] for p in pages), "worst_fraction": worst}


# ----------------------------------------------------------------------------
# causes


STRUCTURAL_CS = {"\\usepackage", "\\documentclass", "\\begin", "\\end", "\\RequirePackage"}


def diag_causes(d):
    """Groupable cause keys for one diagnostic: `<code>: <kind> <name>` for
    each construct its message names (`rwc.named_constructs`, after the
    `[help: ...]`/`[note: ...]` tails are dropped; `\\usepackage` and friends
    only when nothing more specific is named), else `<code>: <normalised
    message>`."""
    code = d.get("code") or "diagnostic"
    msg = re.sub(r"\s*\[(help|note|hint)[^\]]*\]", "", d.get("message") or "")
    names = set(rwc.named_constructs(msg))
    m = re.search(r"(?:document class|class)\s+[`'\"]?([A-Za-z0-9@_.\-]+)", msg)
    if m:
        names.add(("class", m.group(1)))
    m = re.search(r"package\s+[`'\"]?([A-Za-z0-9@_.\-]+)[`'\"]?", msg, re.I)
    if m and not any(k == "package" for k, _ in names) and m.group(1) not in ("is", "are", "file"):
        names.add(("package", m.group(1)))
    specific = {n for n in names if not (n[0] == "cs" and n[1] in STRUCTURAL_CS)}
    if specific:
        names = specific
    if code == "missing_glyph":
        m = re.match(r"(U\+[0-9A-F]+)", msg)
        return [f"{code}: {m.group(1) if m else rwc.diag_key(msg)[:60]}"]
    if names:
        return [f"{code}: {kind} {name}" for kind, name in sorted(names)]
    return [f"{code}: {rwc.diag_key(msg)[:90]}"]


def source_texts(doc):
    out = {}
    for dirpath, _, files in os.walk(doc["dir"]):
        for f in files:
            if f.endswith((".tex", ".sty", ".cls")):
                p = os.path.join(dirpath, f)
                try:
                    out[os.path.relpath(p, doc["dir"]).replace(os.sep, "/")] = pcorpus.slurp(p, encoding="utf-8", errors="replace")
                except OSError:
                    pass
    return out


def _math_context(head):
    """'display math' / 'inline math' when `head` (source before the span)
    ends inside \\[..\\], $$..$$, \\(..\\) or $..$; else None."""
    h = re.sub(r"(?<!\\\\)%[^\n]*", "", head)
    h = h.replace("\\$", "")
    if h.count("\\[") > h.count("\\]") or h.count("$$") % 2:
        return "display math"
    if h.count("\\(") > h.count("\\)") or h.replace("$$", "").count("$") % 2:
        return "inline math"
    return None


def localise(texts, span):
    """Label a source span: the innermost open environment, else display /
    inline math, else the first control word inside the span, else the
    nearest one before it (within 80 bytes), else 'paragraph text'."""
    if not span:
        return "unlocalised"
    text = texts.get(span.get("path"))
    if text is None:
        return "unlocalised"
    raw = text.encode("utf-8")
    a, b = span.get("start_byte", 0), span.get("end_byte", 0)
    head = raw[:a].decode("utf-8", "replace")
    body = head[head.find("\\begin{document}"):] if "\\begin{document}" in head else head
    stack = []
    for m in ENV_RE.finditer(body):
        if m.group(1) == "begin":
            stack.append(m.group(2))
        elif stack and stack[-1] == m.group(2):
            stack.pop()
        elif m.group(2) in stack:
            while stack and stack.pop() != m.group(2):
                pass
    stack = [e for e in stack if e != "document"]
    if stack:
        return f"environment {stack[-1]}"
    mathctx = _math_context(body[-4000:])
    if mathctx:
        return mathctx
    inner = [c for c in re.findall(r"\\([A-Za-z@]+\*?)", raw[a:b].decode("utf-8", "replace"))
             if c not in ("begin", "end", "par")]
    if inner:
        return f"command \\{inner[0]}"
    before = [c for c in re.findall(r"\\([A-Za-z@]+\*?)", raw[max(0, a - 80):a].decode("utf-8", "replace"))
              if c not in ("begin", "end", "par", "usepackage", "documentclass")]
    return f"command \\{before[-1]}" if before else "paragraph text"


def word_source(glyphs, word):
    """Source span covering a candidate word's glyphs (as `rank.v2_words`)."""
    srcs = [g["sources"][0] for g in glyphs[word["glyph_index"]:word["glyph_index"] + word["glyphs"]] if g.get("sources")]
    if not srcs:
        return None
    return {"path": srcs[0]["path"], "start_byte": min(x["start_byte"] for x in srcs),
            "end_byte": max(x["end_byte"] for x in srcs)}


MATH_FONT = re.compile(r"^(CM(MI|SY|BSY|MIB)|LMMath|MSAM|MSBM|EUFM|EUSM|RSFS|STMARY|WASY|TXSY|TXMI|PXSY|PXMI)", re.I)


def glyph_kind(g):
    font = (g or {}).get("font") or ""
    if EXT_FONT.match(font):
        return "math-extension glyph"
    if MATH_FONT.match(font):
        return "math glyph"
    return None


def first_divergence(page_recs, ref_pages, cand_pages, texts):
    """The first page that is not at L3, and a source label for its first
    divergence: the first reference glyph (content order) that has no
    same-character partner within tolerance -- or, on an L2 failure, the
    first reference word the alignment leaves unpaired. It is located through
    the aligned candidate word containing it, else the nearest aligned word
    before it (else after it), whose glyphs carry source spans."""
    for pr in page_recs:
        if pr.get("l3"):
            continue
        if "_pairs" not in pr:
            return pr["page"], "page missing on one side"
        pairs, rw, cw, bad_glyph = pr["_pairs"], pr["_ref_words"], pr["_cand_words"], pr["_first_bad_glyph"]
        glyphs = cand_pages[pr["page"] - 1]
        by_ref = dict(pairs)
        target = None
        for ri, w in enumerate(rw):
            if ri not in by_ref or (bad_glyph is not None and w["glyph_index"] <= bad_glyph < w["glyph_index"] + w["glyphs"]):
                target = ri
                break
        if target is None:
            return pr["page"], "unlocalised"
        kind = glyph_kind(ref_pages[pr["page"] - 1][bad_glyph]) if bad_glyph is not None else None
        suffix = f" ({kind})" if kind else ""
        bad = pr.get("_first_bad_atom")
        if bad is not None:
            # the same character nearest to it on the candidate page: its own
            # cluster's source span is the most direct pointer there is
            near = min((a for a in pr["_cand_atoms"] if a[0] == bad[0]),
                       key=lambda a: abs(a[1] - bad[1]) + 0.25 * abs(a[2] - bad[2]), default=None)
            if near is not None and abs(near[1] - bad[1]) < 50 and abs(near[2] - bad[2]) < 50:
                srcs = glyphs[near[3]].get("sources") or []
                if srcs:
                    return pr["page"], localise(texts, srcs[0]) + suffix
        order = [target] + [target - k for k in range(1, len(rw)) if target - k >= 0] + \
            [target + k for k in range(1, len(rw)) if target + k < len(rw)]
        for ri in order:
            if ri in by_ref:
                src = word_source(glyphs, cw[by_ref[ri]])
                if src:
                    return pr["page"], localise(texts, src) + suffix
        return pr["page"], "unlocalised" + suffix
    return None, None


def causes_for(result, texts):
    """The causes blocking a document's *next* level (a set of keys)."""
    lvl = result["level"]
    cand = result.get("candidate") or {}
    diags = cand.get("diagnostics") or []
    if lvl < 0:
        if cand.get("timed_out"):
            return ["engine: timeout"]
        if not cand.get("pdf") or not cand.get("v2"):
            if cand.get("errors"):
                keys = set()
                for d in diags:
                    if d.get("severity") == "error":
                        keys.update(diag_causes(d))
                return sorted(keys) or ["engine: failed without output"]
            m = re.search(r"panicked at [^\n]*\n?([^\n]*)", cand.get("stderr_tail") or "")
            return [f"engine: crash ({m.group(0).splitlines()[0][:100]})" if m else
                    f"engine: no output (exit {cand.get('exit')})"]
        keys = set()
        for d in diags:
            if d.get("severity") == "error":
                keys.update(diag_causes(d))
        return sorted(keys) or ["engine: error count without error diagnostics"]
    if lvl >= 4:
        return []
    keys = set()
    if lvl == 3:
        for d in diags:
            if d.get("code") in FONT_NOTE_CODES:
                keys.add(f"fonts: {d.get('code')}")
        if not keys:
            keys.add("raster: ink differs with every glyph origin in place (rules, images, outlines)")
        return sorted(keys)
    for d in diags:
        if d.get("severity") == "error" or d.get("code") in FONT_NOTE_CODES:
            continue
        if UNSUPPORTED_WARNING.search(d.get("message") or ""):
            keys.update(diag_causes(d))
    stage = {0: "pagination", 1: "content", 2: "placement"}[lvl]
    if result.get("divergence"):
        keys.add(f"{stage}: first divergence in {result['divergence']}")
    return sorted(keys) or [f"{stage}: unlocalised"]


# ----------------------------------------------------------------------------
# scoring one document


def cumulative_level(checks):
    """Highest k such that L0..Lk all pass; -1 when L0 fails. A check that
    was not evaluated (L4 without a raster) counts as not passed."""
    level = -1
    for k, name in enumerate(LEVELS):
        if not checks.get(name):
            break
        level = k
    return level


def score(doc, cfg):
    """Run the oracle (fetched tiers), FlashTeX, and every check. Returns the
    per-document record plus `_errors` (all aligned glyph errors) for the
    tier distribution."""
    t0 = time.time()
    res = {"id": doc["id"], "tier": doc["tier"], "entry": doc.get("entry"), "source": doc.get("source"),
           "category": doc.get("category"), "level": None, "checks": {}, "oracle": None}
    if doc.get("problem"):
        res["excluded"] = f"fetch: {doc['problem']}"
        return res
    if doc["tier"] == "fixtures":
        ref = doc.get("reference")
        res["oracle"] = {"ok": bool(ref), "reference": os.path.relpath(ref, REPO) if ref else None,
                         "engine": doc.get("reference_engine"), "sha256": rwc.sha256_file(ref) if ref else None}
        if cfg["regenerate"] and cfg["texbin"]:
            ref, meta = oracle_reference(doc, cfg["texbin"], cfg["cache"], None)
            res["oracle"] = meta
    else:
        ref, meta = oracle_reference(doc, cfg["texbin"], cfg["cache"], None)
        res["oracle"] = meta
    if not ref:
        res["excluded"] = "oracle: " + ((res["oracle"] or {}).get("why") or "no reference PDF")
        return res
    out_dir = os.path.join(cfg["work"], doc["tier"], doc["id"].replace("/", "__"))
    cand = run_candidate(doc, cfg["flashtex"], cfg["font_dirs"], cfg["env"], out_dir)
    res["candidate"] = {k: v for k, v in cand.items() if k not in ("pdf", "v2")}
    res["candidate"]["pdf"] = bool(cand["pdf"])
    res["candidate"]["v2"] = bool(cand["v2"])
    font_bad = fontenv.font_diagnostics(cand["diagnostics"])
    if font_bad:
        res["font_env_failure"] = sorted({d.get("code") for d in font_bad})
    checks = res["checks"]
    checks["L0"] = (cand["exit"] == 0 and not cand["timed_out"] and cand["errors"] == 0
                    and bool(cand["pdf"]) and bool(cand["v2"]))
    ref_doc = pdftext.PdfDocument.load(ref)
    ref_pages_raw = ref_doc.pages()
    ref_pages = []
    notes = set()
    for p in ref_pages_raw:
        try:
            g, n = pdftext.page_glyphs(ref_doc, p)
        except pdftext.PdfError as e:
            g, n = [], [f"unsupported: {e}"]
        ref_pages.append(g)
        notes.update(n)
    res["reference_pages"] = len(ref_pages)
    res["reference_notes"] = sorted(notes)[:8]
    errors = []
    page_recs = []
    cand_pages = []
    if cand["v2"]:
        try:
            with open(cand["v2"], encoding="utf-8") as f:
                cand_pages = [p["glyphs"] for p in rank.v2_page_glyphs(json.load(f))]
        except (ValueError, KeyError) as e:
            res["candidate"]["v2_error"] = str(e)[:200]
            cand_pages = []
    res["candidate_pages"] = len(cand_pages) if cand["v2"] else None
    checks["L1"] = bool(cand["v2"]) and len(cand_pages) == len(ref_pages)
    unmapped = collections.Counter()
    missing_all, extra_all = collections.Counter(), collections.Counter()
    for i in range(max(len(ref_pages), len(cand_pages))):
        pr = {"page": i + 1}
        if i >= len(ref_pages) or i >= len(cand_pages):
            pr.update({"l2": False, "l3": False, "missing_page": "candidate" if i >= len(cand_pages) else "reference"})
            page_recs.append(pr)
            continue
        ra, ca = reference_atoms(ref_pages[i]), candidate_atoms(cand_pages[i])
        for a in ra:
            if a[0].startswith(glyphkeys.UNMAPPED_OPEN):
                unmapped[a[0]] += 1
        missing, extra = multiset_diff(ra, ca)
        missing_all.update(missing)
        extra_all.update(extra)
        pr["l2"] = not missing and not extra
        un_r, un_c = match_within(ra, ca)
        pr["l3"] = pr["l2"] and not un_r and not un_c
        pr["glyphs"] = len(ra)
        pr["x_only_glyphs"] = sum(1 for a in ra if a[4])
        pr["unmatched"] = [len(un_r), len(un_c)]
        errs, pairs, rw, cw = aligned_errors(ref_pages[i], ra, cand_pages[i], ca)
        errors.extend(errs)
        pr["_pairs"], pr["_ref_words"], pr["_cand_words"] = pairs, rw, cw
        first_bad = min(un_r, key=lambda k: ra[k][3]) if (pr["l2"] and un_r) else None
        pr["_first_bad_glyph"] = ra[first_bad][3] if first_bad is not None else None
        pr["_first_bad_atom"] = ra[first_bad] if first_bad is not None else None
        pr["_cand_atoms"] = ca
        page_recs.append(pr)
    checks["L2"] = checks["L1"] and all(p["l2"] for p in page_recs)
    checks["L3"] = checks["L2"] and all(p["l3"] for p in page_recs)
    res["glyphs"] = sum(p.get("glyphs", 0) for p in page_recs)
    res["x_only_glyphs"] = sum(p.get("x_only_glyphs", 0) for p in page_recs)
    res["glyphs_placed"] = sum(p.get("glyphs", 0) - p.get("unmatched", [0])[0] for p in page_recs if "glyphs" in p)
    res["missing_chars"] = [[k, v] for k, v in missing_all.most_common(8)]
    res["extra_chars"] = [[k, v] for k, v in extra_all.most_common(8)]
    if unmapped:
        res["unmapped_reference_glyphs"] = [[k, v] for k, v in unmapped.most_common(8)]
    res["glyph_error"] = dist(errors)
    # L4: rasters, only where they can decide something
    want_raster = cfg["raster"] == "l1" and checks["L1"] or cfg["raster"] == "l3" and checks["L3"]
    if want_raster and cand["pdf"]:
        rc = raster_compare(ref, cand["pdf"], os.path.join(out_dir, "raster"))
        res["raster"] = {k: v for k, v in rc.items() if k != "pages"}
        if rc.get("available"):
            res["raster"]["failing_pages"] = [i + 1 for i, p in enumerate(rc["pages"]) if not p["ok"]][:20]
            checks["L4"] = checks["L3"] and rc["ok"]
    level = cumulative_level(checks)
    res["level"] = level
    texts = source_texts(doc) if level < 3 else {}
    fd_page, fd_where = first_divergence(page_recs, ref_pages, cand_pages, texts) if checks["L0"] or cand["v2"] else (None, None)
    res["first_diverging_page"] = fd_page
    res["divergence"] = fd_where
    res["blockers"] = causes_for(res, texts)
    res["facts"] = definers.project_facts(doc["dir"])
    for p in page_recs:
        for k in ("_pairs", "_ref_words", "_cand_words", "_first_bad_glyph", "_first_bad_atom", "_cand_atoms"):
            p.pop(k, None)
    res["pages"] = [p for p in page_recs if not p.get("l3")][:12]
    res["seconds"] = round(time.time() - t0, 2)
    if not cfg["keep_work"]:
        shutil.rmtree(out_dir, ignore_errors=True)
    res["_errors"] = errors
    return res


def score_safe(doc, cfg):
    try:
        return score(doc, cfg)
    except Exception as e:  # noqa: BLE001 - one broken document must not sink the run
        import traceback
        return {"id": doc["id"], "tier": doc["tier"], "level": None, "excluded": f"harness error: {e!r}",
                "traceback": traceback.format_exc()[-1500:], "_errors": []}


# ----------------------------------------------------------------------------
# aggregation


def summarize(results):
    measured = [r for r in results if not r.get("excluded")]
    n = len(measured)
    at_least = {}
    for k, name in enumerate(LEVELS):
        if not any(name in r.get("checks", {}) for r in measured):
            at_least[name] = {"documents": None, "percent": None}  # not evaluated (e.g. --raster none)
            continue
        c = sum(1 for r in measured if (r.get("level") if r.get("level") is not None else -1) >= k)
        at_least[name] = {"documents": c, "percent": round(100.0 * c / n, 1) if n else None}
    independent = {}
    for name in LEVELS:
        evaluated = [r for r in measured if name in r.get("checks", {})]
        passed = sum(1 for r in evaluated if r["checks"][name])
        independent[name] = {"evaluated": len(evaluated), "passed": passed}
    errs = sorted(e for r in measured for e in r.get("_errors", ()))
    l1_errs = sorted(e for r in measured if r.get("checks", {}).get("L1") for e in r.get("_errors", ()))
    excluded = collections.Counter()
    for r in results:
        if r.get("excluded"):
            excluded[r["excluded"].split(":", 1)[0]] += 1
    fdp = collections.Counter()
    for r in measured:
        if r.get("level", -1) is not None and r.get("level", -1) < 3 and r.get("first_diverging_page"):
            p = r["first_diverging_page"]
            fdp["1" if p == 1 else "2" if p == 2 else "3-5" if p <= 5 else "6+"] += 1
    glyphs = sum(r.get("glyphs", 0) for r in measured)
    placed = sum(r.get("glyphs_placed", 0) for r in measured)
    return {
        "documents": len(results), "measured": n, "excluded": dict(excluded),
        "headline_L3_percent": at_least["L3"]["percent"],
        "at_least": at_least, "independent_checks": independent,
        "glyph_error_bp": dist(errs), "glyph_error_bp_same_page_count": dist(l1_errs),
        "within_0_5bp_percent": round(100.0 * sum(1 for e in errs if e <= POS_TOL) / len(errs), 2) if errs else None,
        "glyphs_placed_percent": round(100.0 * placed / glyphs, 2) if glyphs else None,
        "first_diverging_page": dict(fdp),
        "level_histogram": dict(collections.Counter(
            ("below L0" if r["level"] == -1 else LEVELS[r["level"]]) for r in measured)),
    }


def owner_hint(key):
    code = key.split(":", 1)[0]
    if code in ("pagination", "content", "placement"):
        return "crates/render-pipeline (page/paragraph layout)"
    if code.startswith("raster") or code.startswith("fonts"):
        return "crates/render-pipeline + crates/pdf (font resources / outlines)"
    if code.startswith("engine"):
        return "crates/render-pipeline (worker robustness)"
    return rwc.owner_for({"code": code, "message": key})


def rank_causes(results, top=20, field="blockers"):
    """Rank blocker keys by the number of documents whose next level they
    block. `sole` = documents blocked by this key alone (fixing it moves them
    up a level for certain); `share` = sum over documents of 1/|blockers| (an
    expected-value estimate when a document has several blockers)."""
    stat = {}
    for r in results:
        if r.get("excluded") or r.get("level") is None:
            continue
        b = r.get(field) or []
        nxt = LEVELS[r["level"] + 1] if r["level"] + 1 < len(LEVELS) else None
        members = r.get("blocker_members") or {}
        for key in b:
            s = stat.setdefault(key, {"cause": key, "documents": 0, "sole": 0, "share": 0.0,
                                      "next_level": collections.Counter(), "tiers": collections.Counter(), "examples": [],
                                      "members": collections.Counter()})
            s["members"].update(members.get(key, ()))
            s["documents"] += 1
            s["share"] += 1.0 / len(b)
            s["next_level"][nxt] += 1
            s["tiers"][r["tier"]] += 1
            if len(b) == 1:
                s["sole"] += 1
            if len(s["examples"]) < 4:
                s["examples"].append(r["id"])
    ranked = sorted(stat.values(), key=lambda s: (-s["documents"], -s["sole"], -s["share"], s["cause"]))
    out = []
    for i, s in enumerate(ranked[:top], 1):
        rec = {"rank": i, "cause": s["cause"], "documents": s["documents"], "sole": s["sole"],
               "share": round(s["share"], 2), "next_level": dict(s["next_level"]), "tiers": dict(s["tiers"]),
               "owner": owner_hint(s["cause"]), "examples": s["examples"]}
        if s["members"]:
            rec["constructs"] = [k for k, _ in s["members"].most_common(8)]
        out.append(rec)
    return out


def missing_features(results, top=15):
    """Unknown commands / environments / packages / classes named by any
    diagnostic, counted by documents (for the "top causes" table)."""
    c = collections.Counter()
    for r in results:
        if r.get("excluded"):
            continue
        seen = set()
        for d in (r.get("candidate") or {}).get("diagnostics") or []:
            msg = d.get("message") or ""
            if d.get("code") in FONT_NOTE_CODES:
                continue
            if d.get("severity") != "error" and not UNSUPPORTED_WARNING.search(msg):
                continue
            for k in diag_causes(d):
                seen.add(k)
        c.update(seen)
    return [[k, v] for k, v in c.most_common(top)]


# ----------------------------------------------------------------------------
# report


def pct(x):
    return "—" if x is None else f"{x:.1f}%"


def write_report(out_dir, meta, tiers, causes, constructs, per_tier):
    L = []
    w = L.append
    w(f"# Parity scoreboard — {meta['date']}")
    w("")
    w("FlashTeX `flashtex build` against pdfLaTeX (MacTeX, oracle only). Generated by "
      "`tools/parity/parity.py`; the JSON beside this file carries every document's record.")
    w("")
    w("## Headline")
    w("")
    w("| tier | documents measured | **at L3 (headline)** | L0 | L1 | L2 | L3 | L4 | glyph error p50 / p95 / max (bp) |")
    w("|---|---:|---:|---:|---:|---:|---:|---:|---|")
    for name, t in tiers.items():
        s = t["summary"]
        a = s["at_least"]
        ge = s["glyph_error_bp"]
        gtxt = "—" if not ge["n"] else f"{ge['p50']:.3f} / {ge['p95']:.3f} / {ge['max']:.3f} (n={ge['n']})"
        w(f"| {name} | {s['measured']} | **{pct(s['headline_L3_percent'])}** | {pct(a['L0']['percent'])} | "
          f"{pct(a['L1']['percent'])} | {pct(a['L2']['percent'])} | {pct(a['L3']['percent'])} | "
          f"{pct(a['L4']['percent'])} | {gtxt} |")
    w("")
    w("Levels are cumulative (a document at L2 also passes L0 and L1). L0 = pdflatex compiles it with "
      "`-halt-on-error` and FlashTeX renders it with zero error diagnostics; L1 = same page count; L2 = same "
      "glyph characters on every page; L3 = every glyph origin within 0.5 bp on both axes; L4 = rasters at "
      f"{rwc.DPI} dpi differ (|Δ| > {RASTER_DELTA} grey levels) on at most {RASTER_FRACTION * 100:.2f}% of each page. "
      "Documents pdflatex itself cannot compile are excluded from the denominator and counted below.")
    w("")
    for name, t in tiers.items():
        s = t["summary"]
        w(f"## Tier `{name}`")
        w("")
        w(f"- Documents: {s['documents']} listed, {s['measured']} measured; excluded: "
          + (", ".join(f"{k} {v}" for k, v in sorted(s["excluded"].items())) or "none"))
        w(f"- Level histogram (highest level reached): " + ", ".join(
            f"{k} {v}" for k, v in sorted(s["level_histogram"].items(), key=lambda kv: (kv[0] != "below L0", kv[0]))))
        ic = s["independent_checks"]
        w("- Each check on its own (evaluated / passed): " + ", ".join(
            f"{k} {v['passed']}/{v['evaluated']}" for k, v in ic.items()))
        ge, ge1 = s["glyph_error_bp"], s["glyph_error_bp_same_page_count"]
        w(f"- Glyph-position error over aligned glyphs: p50 {ge['p50']}, p95 {ge['p95']}, max {ge['max']} bp "
          f"(n = {ge['n']}); {s['within_0_5bp_percent']}% within 0.5 bp. On documents with the right page count: "
          f"p50 {ge1['p50']}, p95 {ge1['p95']}, max {ge1['max']} (n = {ge1['n']}).")
        w(f"- Glyphs placed (matched within 0.5 bp, all pages): {s['glyphs_placed_percent']}%")
        w(f"- First diverging page (documents below L3): " + (", ".join(
            f"page {k}: {v}" for k, v in sorted(s["first_diverging_page"].items())) or "—"))
        w("")
        w("Top missing features (documents whose diagnostics name it):")
        w("")
        w("| documents | construct |")
        w("|---:|---|")
        for k, v in t["missing"]:
            w(f"| {v} | `{rwc.md_escape(k)}` |")
        w("")
        w("<details><summary>Per-document results</summary>")
        w("")
        w("| document | level | pages ref/cand | errors | first diverging page | where | blockers (next level) |")
        w("|---|---|---|---:|---:|---|---|")
        for r in sorted(t["results"], key=lambda r: r["id"]):
            if r.get("excluded"):
                w(f"| {r['id']} | excluded | — | — | — | — | {rwc.md_escape(r['excluded'][:90])} |")
                continue
            lvl = "below L0" if r["level"] == -1 else LEVELS[r["level"]]
            c = r.get("candidate") or {}
            bl = "; ".join(r.get("blockers") or [])
            w(f"| {r['id']} | {lvl} | {r.get('reference_pages')}/{r.get('candidate_pages')} | {c.get('errors')} | "
              f"{r.get('first_diverging_page') or '—'} | {rwc.md_escape(r.get('divergence') or '—')} | "
              f"{rwc.md_escape(bl[:160])} |")
        w("")
        w("</details>")
        w("")
    def cause_table(rows, members=True):
        w("| # | root cause | documents | sole | share | next level held | tiers | constructs / owner | examples |")
        w("|---:|---|---:|---:|---:|---|---|---|---|")
        for c in rows:
            extra = ", ".join(f"`{rwc.md_escape(k.split(': ', 1)[-1])}`" for k in c.get("constructs", [])[:6]) if members else ""
            w(f"| {c['rank']} | `{rwc.md_escape(c['cause'])}` | {c['documents']} | {c['sole']} | {c['share']} | "
              + ", ".join(f"{k}: {v}" for k, v in sorted(c["next_level"].items())) + " | "
              + ", ".join(f"{k} {v}" for k, v in sorted(c["tiers"].items())) + f" | {extra or rwc.md_escape(c['owner'])} | "
              + ", ".join(c["examples"][:3]) + " |")
        w("")

    w("## Root causes ranked by documents they hold back")
    w("")
    w("Each document below L4 contributes its *blockers*: what stands between it and its next level. L0: every "
      "error diagnostic, keyed by the construct it names; L1-L3: warnings naming an unimplemented construct, plus "
      "the source construct at the first divergence (`pagination:` for L1, `content:` for L2, `placement:` for L3); "
      "L3->L4: font/outline notes. Constructs are then **grouped by who defines them** (`tools/parity/definers.py`: "
      "the project's own files, else the TeX Live package or class that defines the name, preferring one the document "
      "loads), because a package is what a lane implements. `documents` = documents whose next level this blocks; "
      "`sole` = documents it alone blocks (fixing it moves them up for certain); `share` = Σ 1/|blockers| over those "
      "documents (expected documents moved when several blockers share a document).")
    w("")
    w("### All tiers, grouped (the list to pick lanes from)")
    w("")
    cause_table(causes)
    for name, pc in per_tier.items():
        w(f"### Tier `{name}`, grouped (top 10)")
        w("")
        cause_table(pc["grouped"][:10])
    w("### Individual constructs, all tiers (top 20)")
    w("")
    cause_table(constructs, members=False)
    w("## Provenance")
    w("")
    for k in ("flashtex", "flashtex_version", "flashtex_sha256", "pdflatex", "rasterizer", "font_dirs", "tfm_dirs",
              "host", "platform", "started_utc", "wall_seconds", "jobs", "command"):
        w(f"- {k}: `{meta.get(k)}`")
    w("")
    w("## Honest limits")
    w("")
    w("- Glyph identity is Unicode, not glyph id: the reference's glyph names and the candidate's cluster text are "
      "both NFKD-normalised and folded (`tools/parity/glyphkeys.py`). A reference glyph name with no mapping is "
      "reported per document as `unmapped_reference_glyphs` and counts as a mismatch.")
    w("- Text drawn inside Form XObjects (included PDF figures) is read on neither side; Type 3 / CID reference "
      "fonts are unreadable and show up as missing glyphs (`reference_notes`).")
    w("- Blockers for L1-L3 are heuristics: a warning naming an unimplemented construct is presumed to matter, and "
      "the first diverging word is located by the nearest aligned candidate word's source span.")
    w("- The fixtures tier compares against the committed references; twelve real-world fixtures and every "
      "divergence probe were generated with TeX Live 2025, the rest with MacTeX 2026 (`oracle.engine`).")
    w("")
    with open(os.path.join(out_dir, "report.md"), "w", encoding="utf-8") as f:
        f.write("\n".join(L))


# ----------------------------------------------------------------------------
# baseline gate


def baseline_of(results):
    return {r["id"]: (r["level"] if not r.get("excluded") else None) for r in results}


def check_baseline(results, path):
    """Every document must reach at least the level recorded in `path`."""
    with open(path, encoding="utf-8") as f:
        base = json.load(f)["levels"]
    now = baseline_of(results)
    regressions = []
    for did, lvl in sorted(base.items()):
        if lvl is None:
            continue
        cur = now.get(did)
        if cur is None or cur < lvl:
            regressions.append((did, lvl, cur))
    return regressions


# ----------------------------------------------------------------------------
# main


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--tier", action="append", choices=["fixtures", "arxiv", "templates"], default=[])
    ap.add_argument("--only", action="append", default=[], help="document id (repeatable)")
    ap.add_argument("--limit", type=int, default=0, help="first N documents per tier (smoke runs)")
    ap.add_argument("--flashtex", default=os.environ.get("FLASHTEX_CLI", DEFAULT_FLASHTEX))
    ap.add_argument("--texbin", default=rwc.DEFAULT_TEXBIN)
    ap.add_argument("--cache", default=pcorpus.default_cache())
    ap.add_argument("--texmf", default=pcorpus.DEFAULT_TEXMF)
    ap.add_argument("--font-dirs", default=None)
    ap.add_argument("--tfm-dirs", default=None)
    ap.add_argument("--regenerate", action="store_true",
                    help="fixtures tier: make the reference now with the local pdflatex instead of the committed PDF")
    ap.add_argument("--raster", choices=["none", "l3", "l1"], default="l1",
                    help="rasterise documents that pass L1 (default), only those at L3, or none")
    ap.add_argument("-j", "--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    ap.add_argument("--out", default=None, help="report dir (default docs/evidence/parity-<UTC date>)")
    ap.add_argument("--work", default=os.path.join(HERE, "target", "work"))
    ap.add_argument("--keep-work", action="store_true")
    ap.add_argument("--write-baseline", default=None, help="write the fixtures levels as a baseline JSON")
    ap.add_argument("--check-baseline", default=None, help="exit 1 if a document falls below its baseline level")
    args = ap.parse_args(argv)
    tiers = args.tier or ["fixtures"]
    if not os.path.isfile(args.flashtex):
        print(f"flashtex CLI not found at {args.flashtex}; build it with\n  cargo build --release "
              "--manifest-path crates/flashtex-cli/Cargo.toml --bin flashtex", file=sys.stderr)
        return 2
    font_dirs, tfm_dirs = fontenv.resolve_dirs(args.font_dirs, args.tfm_dirs, os.path.join(REPO, "apps", "mac", "Fonts"))
    env = dict(fontenv.render_env(font_dirs, tfm_dirs), SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
    started = time.time()
    stamp = _dt.datetime.now(_dt.timezone.utc)
    out_dir = args.out or os.path.join(REPO, "docs", "evidence", f"parity-{stamp.strftime('%Y-%m-%d')}")
    os.makedirs(out_dir, exist_ok=True)
    os.makedirs(args.work, exist_ok=True)
    cfg = {"flashtex": os.path.abspath(args.flashtex), "texbin": args.texbin, "cache": args.cache,
           "font_dirs": font_dirs, "env": env, "work": os.path.abspath(args.work), "keep_work": args.keep_work,
           "raster": args.raster, "regenerate": args.regenerate}
    only = set(args.only)

    def log(msg):
        print(msg, file=sys.stderr, flush=True)

    tier_docs = {}
    for t in tiers:
        if t == "fixtures":
            docs = fixture_documents(only=only)
        else:
            docs = []
            for m in pcorpus.manifests():
                with open(m, encoding="utf-8") as f:
                    if json.load(f).get("tier") != t:
                        continue
                docs += pcorpus.fetch_manifest(m, args.cache, args.texmf, log=log)
            if only:
                docs = [d for d in docs if d["id"] in only]
        if args.limit:
            docs = docs[:args.limit]
        tier_docs[t] = docs
    results = {t: [] for t in tiers}
    jobs = [(t, d) for t in tiers for d in tier_docs[t]]
    log(f"scoring {len(jobs)} documents with {args.jobs} workers")
    with concurrent.futures.ProcessPoolExecutor(max_workers=args.jobs) as ex:
        futs = {ex.submit(score_safe, d, cfg): (t, d) for t, d in jobs}
        for n, fut in enumerate(concurrent.futures.as_completed(futs), 1):
            t, d = futs[fut]
            r = fut.result()
            results[t].append(r)
            lvl = r.get("excluded") or ("below L0" if r.get("level") == -1 else LEVELS[r["level"]])
            log(f"[{n}/{len(jobs)}] {t}/{d['id']}: {lvl} ({r.get('seconds', '?')} s)")
    all_results = [r for t in tiers for r in results[t]]
    exe_ver = rwc.run([cfg["flashtex"], "--version"], timeout=30)[1].decode("utf-8", "replace").strip()
    meta = {"date": stamp.strftime("%Y-%m-%d"), "started_utc": stamp.strftime("%Y-%m-%dT%H:%M:%SZ"),
            "flashtex": os.path.relpath(cfg["flashtex"], REPO), "flashtex_version": exe_ver,
            "flashtex_sha256": rwc.sha256_file(cfg["flashtex"]),
            "pdflatex": rwc.pdflatex_version(args.texbin)[1], "rasterizer": rwc.find_rasterizer()[0],
            "font_dirs": font_dirs, "tfm_dirs": tfm_dirs, "host": platform.node(),
            "platform": f"{platform.system()} {platform.release()} {platform.machine()}",
            "wall_seconds": round(time.time() - started, 1), "jobs": args.jobs,
            "command": "python3 tools/parity/parity.py " + " ".join(argv if argv is not None else sys.argv[1:]),
            "levels": {"pos_tol_bp": POS_TOL, "raster_delta": RASTER_DELTA, "raster_fraction": RASTER_FRACTION,
                       "dpi": rwc.DPI}}
    tiers_out = {}
    for t in tiers:
        rs = results[t]
        tiers_out[t] = {"summary": summarize(rs), "missing": missing_features(rs), "results": rs}
    index = None
    if os.path.isdir(os.path.join(args.texmf, "tex", "latex")):
        log(f"indexing definitions under {args.texmf} (cached)")
        index = definers.texlive_index(args.texmf, args.cache)
    for r in all_results:
        if r.get("excluded") or r.get("level") is None:
            continue
        groups = collections.defaultdict(set)
        loaded = definers.closure(r.get("facts") or {}, index)
        for k in r.get("blockers") or []:
            groups[definers.group_of(k, r.get("facts") or {}, index, loaded)].add(k)
        r["blocker_groups"] = sorted(groups)
        r["blocker_members"] = {g: sorted(v) for g, v in groups.items() if v != {g}}
    causes = rank_causes(all_results, field="blocker_groups")
    constructs = rank_causes(all_results)
    per_tier_causes = {t: {"grouped": rank_causes(results[t], field="blocker_groups"), "constructs": rank_causes(results[t])}
                       for t in tiers}
    for t in tiers:
        for r in results[t]:
            r.pop("_errors", None)
    write_report(out_dir, meta, tiers_out, causes, constructs, per_tier_causes)
    with open(os.path.join(out_dir, "scoreboard.json"), "w", encoding="utf-8") as f:
        json.dump({"schema": "flashtex-parity/1", "meta": meta,
                   "tiers": {t: {"summary": v["summary"], "missing": v["missing"], "causes": per_tier_causes[t]["grouped"],
                                 "constructs": per_tier_causes[t]["constructs"]}
                             for t, v in tiers_out.items()},
                   "causes": causes, "constructs": constructs}, f, indent=1, ensure_ascii=False)
    with open(os.path.join(out_dir, "documents.json"), "w", encoding="utf-8") as f:
        json.dump({t: sorted(results[t], key=lambda r: r["id"]) for t in tiers}, f, indent=1, ensure_ascii=False)
    for t in tiers:
        s = tiers_out[t]["summary"]
        log(f"{t}: {s['measured']} measured, L3 {pct(s['headline_L3_percent'])}; at least: "
            + ", ".join(f"{k} {pct(v['percent'])}" for k, v in s["at_least"].items()))
    log(f"report: {os.path.relpath(os.path.join(out_dir, 'report.md'), REPO)}")
    if args.write_baseline:
        with open(args.write_baseline, "w", encoding="utf-8") as f:
            json.dump({"schema": "flashtex-parity-baseline/1", "generated": meta["started_utc"], "raster": args.raster,
                       "flashtex_version": exe_ver, "levels": baseline_of(all_results)}, f, indent=1, sort_keys=True)
            f.write("\n")
    if args.check_baseline:
        regs = check_baseline(all_results, args.check_baseline)
        if regs:
            for did, was, now in regs:
                log(f"REGRESSION {did}: baseline {was}, now {now}")
            return 1
        log("baseline: no document below its recorded level")
    return 0


if __name__ == "__main__":
    sys.exit(main())
