#!/usr/bin/env python3
"""Real-world corpus acceptance harness: FlashTeX producers vs pdfLaTeX.

Owner: lane mac-realworld-corpus (parent mac-claude-a). Python 3 standard
library only. Nothing here is in the product path: pdfLaTeX is an ORACLE that
produces the reference PDFs, the producers under test are the project's own
Rust workers driven over runtime-v1 JSON Lines.

For every fixture directory under fixtures/real-world/ (one `main.tex`, or a
single `*.tex`, plus any `\\input` files) the harness

  1. finds the reference PDF (`reference.pdf`, else `*-reference.pdf`); when
     none exists and pdflatex is available it generates `reference.pdf` with
     `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1` (two passes) and records SHA-256,
     page count, pdflatex version and argv in `reference.json`. User-provided
     references are never overwritten; `--regenerate` writes a separate
     `reference-mactex2026.pdf` next to them and records its SHA too;
  2. runs each producer (`compiler` = flashtex-compiler, `render` =
     flashtex-render) as a fresh process with one `compile` request, capturing
     status, page count, every diagnostic (with source spans), wall time;
  3. turns the producer output into a PDF where a route exists: `render`
     writes the rendering-v2 display list (`--v2`) which `flashtex-pdf-exact
     from-v2` turns into a PDF; `compiler` output goes through `flashtex-pdf`
     (runtime-v1 text items, base-14 fonts) when `--pdf-v1` is given;
  4. rasterises reference and producer PDFs at 144 dpi to 8-bit grey PGM with
     `pdftoppm` or Ghostscript (whichever exists; reported) and counts
     differing pixels per page (exact byte inequality plus the max delta);
  5. derives a construct inventory from the .tex sources with a small
     tokenizer (control sequences, environments, packages and their options,
     class and class options, math delimiters) and classifies every construct
     per producer from the diagnostics that name it or cover it;
  6. writes `report.md` and `report.json` including a ranked list of the most
     frequent unsupported/recovered diagnostics with owner crate and spans.

Pixel counts on `recovered` documents are large by construction; the report
prints them but never ranks by them and never claims parity.
"""

import argparse
import datetime as _dt
import hashlib
import json
import operator
import os
import platform
import re
import shutil
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(REPO, "tools", "visual-oracle"))
import fontenv  # noqa: E402
DEFAULT_FIXTURES = os.path.join(REPO, "fixtures", "real-world")
DEFAULT_TEXBIN = "/usr/local/texlive/2026/bin/universal-darwin"
DPI = 144

# Commander ruling (issue #2 5646504285): this compiler gap is listed first
# regardless of frequency — starred sectioning inside the two-argument
# `\problem` macro of HW1.tex.
PINNED_FIRST = [r"\subsection requires a braced argument"]

UNSUPPORTED_RE = re.compile(
    r"not supported|not implemented|unsupported|unknown|requires a braced|no math glyph|cannot|is not a",
    re.I,
)


# ----------------------------------------------------------------------------
# small utilities


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def run(cmd, stdin_bytes=None, env=None, timeout=300, cwd=None):
    t0 = time.monotonic()
    try:
        p = subprocess.run(cmd, input=stdin_bytes, capture_output=True, env=env, timeout=timeout, cwd=cwd)
        return p.returncode, p.stdout, p.stderr, time.monotonic() - t0, False
    except subprocess.TimeoutExpired as e:
        return None, e.stdout or b"", e.stderr or b"", time.monotonic() - t0, True


def uptime():
    try:
        return subprocess.run(["uptime"], capture_output=True, text=True).stdout.strip()
    except OSError:
        return "uptime unavailable"


def load1():
    try:
        return os.getloadavg()[0]
    except (OSError, AttributeError):
        return None


def utc_now():
    return _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H%M%SZ")


def rel(path):
    try:
        return os.path.relpath(path, REPO)
    except ValueError:
        return path


# ----------------------------------------------------------------------------
# fixtures and references


def discover_fixtures(root):
    out = []
    for name in sorted(os.listdir(root)):
        d = os.path.join(root, name)
        if not os.path.isdir(d):
            continue
        texs = sorted(f for f in os.listdir(d) if f.endswith(".tex"))
        if not texs:
            continue
        entry = "main.tex" if "main.tex" in texs else (texs[0] if len(texs) == 1 else None)
        if entry is None:
            print(f"skip {name}: several top-level .tex files and no main.tex", file=sys.stderr)
            continue
        docs = []
        for dirpath, _, files in os.walk(d):
            for f in sorted(files):
                if f.endswith(".tex"):
                    p = os.path.join(dirpath, f)
                    docs.append(os.path.relpath(p, d))
        out.append({"id": name, "dir": d, "entry": entry, "documents": sorted(docs)})
    return out


def find_reference(fx):
    d = fx["dir"]
    cands = [os.path.join(d, "reference.pdf")] + sorted(
        os.path.join(d, f) for f in os.listdir(d) if f.endswith("-reference.pdf")
    )
    for c in cands:
        if os.path.isfile(c):
            return c
    return None


def pdflatex_version(texbin):
    exe = os.path.join(texbin, "pdflatex")
    if not os.path.isfile(exe):
        return None, None
    code, out, _, _, _ = run([exe, "--version"], timeout=30)
    first = out.decode("utf-8", "replace").splitlines()[0] if out else ""
    return exe, first


def pdf_page_count(path):
    """Count pages from the trailer-independent /Type /Page objects (good enough
    for pdfTeX and this project's writers; ghostscript/pdftoppm re-derive it)."""
    with open(path, "rb") as f:
        data = f.read()
    n = len(re.findall(rb"/Type\s*/Page(?![s/a-zA-Z])", data))
    return n or None


def generate_reference(fx, texbin, out_name, log):
    """Run pdflatex twice in a scratch copy of the fixture; copy the PDF to
    <fixture>/<out_name>. Returns the sidecar record or None."""
    exe, version = pdflatex_version(texbin)
    if exe is None:
        log.append(f"{fx['id']}: no pdflatex at {texbin}; cannot generate a reference")
        return None
    scratch = os.path.join(HERE, "target", "oracle", fx["id"])
    shutil.rmtree(scratch, ignore_errors=True)
    shutil.copytree(fx["dir"], scratch, ignore=shutil.ignore_patterns("*.pdf", "*.json", "*.md"))
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
    argv = [exe, "-interaction=nonstopmode", "-halt-on-error", fx["entry"]]
    for _pass in range(2):
        code, out, err, secs, timed_out = run(argv, env=env, timeout=300, cwd=scratch)
        if timed_out or code != 0:
            log.append(f"{fx['id']}: pdflatex failed (exit {code}, timeout={timed_out})")
            return None
    produced = os.path.join(scratch, os.path.splitext(fx["entry"])[0] + ".pdf")
    if not os.path.isfile(produced):
        log.append(f"{fx['id']}: pdflatex produced no PDF")
        return None
    dest = os.path.join(fx["dir"], out_name)
    shutil.copyfile(produced, dest)
    logtxt = open(os.path.join(scratch, os.path.splitext(fx["entry"])[0] + ".log"), "rb").read().decode("utf-8", "replace")
    m = re.search(r"Output written on .*?\((\d+) pages?", logtxt)
    return {
        "file": out_name,
        "sha256": sha256_file(dest),
        "pages": int(m.group(1)) if m else pdf_page_count(dest),
        "pdflatex": version,
        "argv": ["SOURCE_DATE_EPOCH=0", "FORCE_SOURCE_DATE=1"] + [os.path.basename(argv[0])] + argv[1:] + ["(x2)"],
        "generated_utc": utc_now(),
        "overfull_boxes": len(re.findall(r"^Overfull", logtxt, re.M)),
        "latex_warnings": len(re.findall(r"LaTeX Warning", logtxt)),
    }


# ----------------------------------------------------------------------------
# producers

# 1970-01-01 UTC -- the civil date of SOURCE_DATE_EPOCH=0, which is what this
# harness already gives pdflatex for the committed references.
EPOCH_DATE = "1970-01-01"


def make_request(fx, rid):
    docs = []
    for name in fx["documents"]:
        with open(os.path.join(fx["dir"], name), encoding="utf-8") as f:
            docs.append({"path": name.replace(os.sep, "/"), "text": f.read()})
    return {
        "protocol_version": 1,
        "id": rid,
        "type": "compile",
        "payload": {
            "project_id": "real-world-" + fx["id"],
            "revision": 1,
            "entry_path": fx["entry"],
            "documents": docs,
            # Pinned, never the wall clock: the committed reference.pdf/.json in
            # every fixture were produced by pdflatex under SOURCE_DATE_EPOCH=0
            # FORCE_SOURCE_DATE=1, so the producer must be given the same civil
            # date or a fixture using \today would drift the moment the date
            # changed. This is an explicit pin rather than a reliance on the
            # compiler's absent-date default, so the reference data survives any
            # later change to what "no date supplied" means.
            # protocol/proposals/runtime-v1-request-date.md
            "date": EPOCH_DATE,
            # The fixture directory, so \includegraphics resolves against the
            # same files pdflatex saw instead of failing with "no project root"
            # (protocol/proposals/display-list-v2-image.md §2).
            "project_root": os.path.abspath(fx["dir"]),
        },
    }


def run_producer(name, exe, extra_args, fx, out_dir, env):
    req = make_request(fx, f"rw-{fx['id']}")
    line = (json.dumps(req, ensure_ascii=False) + "\n").encode("utf-8")
    code, out, err, secs, timed_out = run([exe] + extra_args, stdin_bytes=line, env=env, timeout=180)
    rec = {
        "producer": name,
        "exit": code,
        "timed_out": timed_out,
        "seconds": round(secs, 3),
        "stderr_tail": err.decode("utf-8", "replace")[-2000:],
        "status": None,
        "pages": None,
        "diagnostics": [],
        "reply_type": None,
    }
    with open(os.path.join(out_dir, f"{name}.reply.jsonl"), "wb") as f:
        f.write(out)
    for raw in out.decode("utf-8", "replace").splitlines():
        raw = raw.strip()
        if not raw:
            continue
        try:
            msg = json.loads(raw)
        except json.JSONDecodeError:
            rec["reply_type"] = rec["reply_type"] or "unparseable"
            continue
        if msg.get("type") == "compile_result" and rec["status"] is None:
            p = msg.get("payload", {})
            rec["reply_type"] = "compile_result"
            rec["status"] = p.get("status")
            rec["pages"] = len(p.get("pages", []))
            rec["diagnostics"] = p.get("diagnostics", [])
            rec["layout_capabilities"] = p.get("layout_capabilities")
        elif msg.get("type") == "error" and rec["status"] is None:
            rec["reply_type"] = "error"
            rec["status"] = "error"
            rec["diagnostics"] = [{"severity": "error", "code": "protocol_error", "message": json.dumps(msg.get("payload")), "source": None}]
    if rec["status"] is None:
        rec["status"] = "no_reply"
    return rec


# ----------------------------------------------------------------------------
# rasterisation and comparison


def find_rasterizer(prefer=None):
    """Returns (name, path). PGM output at DPI; pdftoppm first, then gs."""
    cands = []
    if prefer:
        cands.append(prefer)
    cands += ["pdftoppm", "gs"]
    for c in cands:
        p = shutil.which(c) or (c if os.path.isfile(c) else None)
        if p:
            base = os.path.basename(p)
            if base.startswith("pdftoppm"):
                return "pdftoppm", p
            if base in ("gs", "gswin64c"):
                return "gs", p
    if shutil.which("sips"):
        return "sips", shutil.which("sips")
    return None, None


def rasterize(rast, pdf, prefix):
    """Rasterise every page to <prefix>-<n>.pgm. Returns (list of page files,
    argv, note)."""
    name, exe = rast
    os.makedirs(os.path.dirname(prefix), exist_ok=True)
    for f in os.listdir(os.path.dirname(prefix)):
        if f.startswith(os.path.basename(prefix) + "-") and f.endswith(".pgm"):
            os.remove(os.path.join(os.path.dirname(prefix), f))
    if name == "pdftoppm":
        argv = [exe, "-gray", "-r", str(DPI), "-aa", "yes", "-aaVector", "yes", pdf, prefix]
    elif name == "gs":
        argv = [exe, "-q", "-dNOPAUSE", "-dBATCH", "-dSAFER", "-sDEVICE=pgmraw", f"-r{DPI}",
                "-dTextAlphaBits=4", "-dGraphicsAlphaBits=4", f"-sOutputFile={prefix}-%d.pgm", pdf]
    else:
        return [], [], "sips cannot select pages or write PGM; pixel comparison unavailable"
    code, out, err, secs, timed_out = run(argv, timeout=300)
    if timed_out or code != 0:
        return [], argv, f"rasterizer exit {code} timeout={timed_out}: {err.decode('utf-8', 'replace')[-300:]}"
    d = os.path.dirname(prefix)
    files = sorted(
        (os.path.join(d, f) for f in os.listdir(d) if f.startswith(os.path.basename(prefix) + "-") and f.endswith(".pgm")),
        key=lambda p: int(re.search(r"-(\d+)\.pgm$", p).group(1)),
    )
    return files, argv, ""


def read_pgm(path):
    with open(path, "rb") as f:
        data = f.read()
    pos = 0
    fields = []
    while len(fields) < 4:
        while pos < len(data) and data[pos:pos + 1].isspace():
            pos += 1
        if data[pos:pos + 1] == b"#":
            while data[pos:pos + 1] not in (b"\n", b""):
                pos += 1
            continue
        start = pos
        while pos < len(data) and not data[pos:pos + 1].isspace():
            pos += 1
        fields.append(data[start:pos])
    pos += 1  # single whitespace after maxval
    magic, w, h, maxval = fields[0], int(fields[1]), int(fields[2]), int(fields[3])
    if magic != b"P5" or maxval != 255:
        raise ValueError(f"{path}: expected binary 8-bit PGM, got {magic!r} maxval {maxval}")
    pixels = data[pos:pos + w * h]
    if len(pixels) != w * h:
        raise ValueError(f"{path}: truncated PGM")
    return w, h, pixels


def compare_pages(ref_files, cand_files):
    pages = []
    n = max(len(ref_files), len(cand_files))
    for i in range(n):
        rec = {"page": i + 1}
        if i >= len(ref_files) or i >= len(cand_files):
            rec["result"] = "missing_in_" + ("candidate" if i >= len(cand_files) else "reference")
            pages.append(rec)
            continue
        rw, rh, rp = read_pgm(ref_files[i])
        cw, ch, cp = read_pgm(cand_files[i])
        rec["reference_size"] = [rw, rh]
        rec["candidate_size"] = [cw, ch]
        if (rw, rh) != (cw, ch):
            rec["result"] = "size_mismatch"
            pages.append(rec)
            continue
        differing = sum(map(operator.ne, rp, cp))
        max_delta = max(map(abs, map(operator.sub, rp, cp))) if differing else 0
        ink_ref = sum(1 for b in rp if b < 128)
        ink_cand = sum(1 for b in cp if b < 128)
        rec.update({
            "result": "compared",
            "pixels": rw * rh,
            "differing": differing,
            "differing_fraction": round(differing / (rw * rh), 5),
            "max_delta": max_delta,
            "ink_pixels_reference": ink_ref,
            "ink_pixels_candidate": ink_cand,
        })
        pages.append(rec)
    return pages


# ----------------------------------------------------------------------------
# construct inventory (tokenizer, not a parser)

DEFINERS = {"newcommand", "renewcommand", "providecommand", "DeclareMathOperator", "def", "newtheorem",
            "newenvironment", "renewenvironment", "DeclareRobustCommand", "let"}


def strip_comment(line):
    out = []
    i = 0
    while i < len(line):
        c = line[i]
        if c == "\\" and i + 1 < len(line):
            out.append(line[i:i + 2])
            i += 2
            continue
        if c == "%":
            break
        out.append(c)
        i += 1
    return "".join(out)


def brace_group(text, i):
    """text[i] == '{' -> (content, end index after '}') with nesting."""
    depth = 0
    j = i
    while j < len(text):
        c = text[j]
        if c == "\\":
            j += 2
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return text[i + 1:j], j + 1
        j += 1
    return text[i + 1:], len(text)


def optional_group(text, i):
    if i < len(text) and text[i] == "[":
        j = text.find("]", i)
        if j != -1:
            return text[i + 1:j], j + 1
    return None, i


def tokenize(text, path):
    """Yield construct occurrences: dict(kind, name, path, start, end, note).
    Byte offsets are computed on the UTF-8 encoding to match runtime-v1 spans."""
    # Build a char->byte offset table so spans line up with the compiler's.
    byte_at = [0]
    for ch in text:
        byte_at.append(byte_at[-1] + len(ch.encode("utf-8")))
    occ = []
    defined = set()
    # Remove comments but keep offsets: replace comment chars with spaces.
    lines = text.split("\n")
    cleaned = "\n".join(strip_comment(l).ljust(len(l)) for l in lines)
    n = len(cleaned)
    i = 0
    in_math = 0
    while i < n:
        c = cleaned[i]
        if c == "\\":
            j = i + 1
            if j < n and (cleaned[j].isalpha() or cleaned[j] == "@"):
                while j < n and (cleaned[j].isalpha() or cleaned[j] == "@"):
                    j += 1
                name = cleaned[i + 1:j]
                star = j < n and cleaned[j] == "*"
                if star:
                    j += 1
            elif j < n:
                name = cleaned[j]
                star = False
                j += 1
            else:
                break
            cs = "\\" + name + ("*" if star else "")
            kind = "cs"
            k = j
            if name in ("begin", "end"):
                while k < n and cleaned[k].isspace():
                    k += 1
                if k < n and cleaned[k] == "{":
                    env, k2 = brace_group(cleaned, k)
                    env = env.strip()
                    if name == "begin":
                        occ.append({"kind": "env", "name": env, "path": path, "start": byte_at[i], "end": byte_at[k2]})
                        if env in ("equation", "align", "align*", "gather", "gather*", "equation*", "multline", "eqnarray", "displaymath", "math"):
                            in_math += 1
                    else:
                        if env in ("equation", "align", "align*", "gather", "gather*", "equation*", "multline", "eqnarray", "displaymath", "math"):
                            in_math = max(0, in_math - 1)
                    j = k2
                    i = j
                    continue
            if name in ("usepackage", "RequirePackage", "documentclass"):
                while k < n and cleaned[k].isspace():
                    k += 1
                opts, k = optional_group(cleaned, k)
                while k < n and cleaned[k].isspace():
                    k += 1
                if k < n and cleaned[k] == "{":
                    names, k2 = brace_group(cleaned, k)
                    for pk in [s.strip() for s in names.split(",") if s.strip()]:
                        kind2 = "class" if name == "documentclass" else "package"
                        occ.append({"kind": kind2, "name": pk, "path": path, "start": byte_at[i], "end": byte_at[k2]})
                        for o in [s.strip() for s in (opts or "").split(",") if s.strip()]:
                            occ.append({"kind": kind2 + "-option", "name": f"{pk}[{o}]", "path": path, "start": byte_at[i], "end": byte_at[k2]})
                    j = k2
            if name in DEFINERS:
                while k < n and cleaned[k].isspace():
                    k += 1
                if k < n and cleaned[k] == "{":
                    inner, _ = brace_group(cleaned, k)
                    inner = inner.strip()
                    defined.add(inner if inner.startswith("\\") else inner)
                elif k < n and cleaned[k] == "\\":
                    m = re.match(r"\\[A-Za-z@]+", cleaned[k:])
                    if m:
                        defined.add(m.group(0))
            occ.append({"kind": kind, "name": cs, "path": path, "start": byte_at[i], "end": byte_at[j], "math": in_math > 0})
            if cs in ("\\[", "\\("):
                in_math += 1
            elif cs in ("\\]", "\\)"):
                in_math = max(0, in_math - 1)
            i = j
            continue
        if c == "$":
            if i + 1 < n and cleaned[i + 1] == "$":
                occ.append({"kind": "math", "name": "$$", "path": path, "start": byte_at[i], "end": byte_at[i + 2]})
                in_math = 0 if in_math else 1
                i += 2
            else:
                occ.append({"kind": "math", "name": "$", "path": path, "start": byte_at[i], "end": byte_at[i + 1]})
                in_math = 0 if in_math else 1
                i += 1
            continue
        if c in "&~^_":
            occ.append({"kind": "char", "name": c, "path": path, "start": byte_at[i], "end": byte_at[i + 1], "math": in_math > 0})
        elif c == "-" and cleaned[i:i + 3] == "---":
            occ.append({"kind": "char", "name": "---", "path": path, "start": byte_at[i], "end": byte_at[i + 3]})
            i += 3
            continue
        elif c == "-" and cleaned[i:i + 2] == "--":
            occ.append({"kind": "char", "name": "--", "path": path, "start": byte_at[i], "end": byte_at[i + 2]})
            i += 2
            continue
        elif c == "`" and cleaned[i:i + 2] == "``":
            occ.append({"kind": "char", "name": "``", "path": path, "start": byte_at[i], "end": byte_at[i + 2]})
            i += 2
            continue
        elif ord(c) > 127 and not c.isspace():
            occ.append({"kind": "char", "name": "non-ascii", "path": path, "start": byte_at[i], "end": byte_at[i + 1]})
        i += 1
    for o in occ:
        if o["kind"] == "cs" and o["name"].rstrip("*") in defined or o["kind"] == "env" and o["name"] in defined:
            o["user_defined"] = True
    return occ


def inventory(fx):
    occ = []
    for name in fx["documents"]:
        with open(os.path.join(fx["dir"], name), encoding="utf-8") as f:
            occ.extend(tokenize(f.read(), name.replace(os.sep, "/")))
    return occ


# ----------------------------------------------------------------------------
# diagnostics -> constructs

NUM_RE = re.compile(r"-?\d+(?:\.\d+)?")


def diag_key(msg):
    """Normalise a diagnostic message into a stable key (numbers -> <N>)."""
    return NUM_RE.sub("<N>", msg)


def named_constructs(msg):
    names = set()
    for m in re.finditer(r"\\([A-Za-z@]+\*?|[^A-Za-z\s])", msg):
        names.add(("cs", "\\" + m.group(1)))
    for m in re.finditer(r"environment '([^']+)'", msg):
        names.add(("env", m.group(1)))
    m = re.search(r"packages? ([\w\-, ]+?) (?:are|is) recognised", msg)
    if m:
        for p in [s.strip() for s in m.group(1).split(",") if s.strip()]:
            names.add(("package", p))
    m = re.search(r"no math glyph for '(.)'", msg)
    if m:
        names.add(("char", m.group(1)))
    return names


COMPILER_MSG_RE = re.compile(
    r"is not supported|not implemented|requires a braced|requires math mode|in the document preamble|"
    r"unknown|recognised but|outside math mode|missing|unclosed|unexpected|undefined|cannot|invalid|is not a"
)


def owner_for(diag):
    """Owning crate for a diagnostic. The `compiler` producer on main emits no
    `code` field, so the message family decides first; the render worker's
    codes decide the rest. Ownership per coordination/authority.json reading:
    compiler = Commander/main, math-layout = FT-020, render-pipeline =
    mac-claude-a. No workaround layer is suggested for parser gaps."""
    code = diag.get("code") or ""
    msg = diag.get("message") or ""
    if code == "compiler" or code == "protocol_error" or (not code and COMPILER_MSG_RE.search(msg)):
        if "in math mode" in msg or "requires math mode" in msg or "outside math mode" in msg:
            return "crates/compiler (math parser src/math.rs; Commander/main) — downstream typesetting: crates/math-layout (FT-020)"
        return "crates/compiler (parser/expansion; Commander/main)"
    if code.startswith("pdf"):
        return "crates/pdf (mac-pdf)"
    if code in ("overfull_display", "math_limitation", "math_resource_profile", "math_metrics_opentype") or "math glyph" in msg or "display is" in msg:
        return "crates/render-pipeline (mac-claude-a) — math boxes: crates/math-layout (FT-020)"
    return "crates/render-pipeline (mac-claude-a)"


def classify(occ, diags):
    """Per construct (kind, name): status + supporting diagnostics."""
    by_name = {}
    spans = []
    for d in diags:
        key = diag_key(d.get("message", ""))
        names = named_constructs(d.get("message", ""))
        src = d.get("source") or {}
        unsupported = bool(UNSUPPORTED_RE.search(d.get("message", "")))
        for nm in names:
            by_name.setdefault(nm, []).append((key, unsupported, d))
        if src and isinstance(src.get("start_byte"), int):
            spans.append((src.get("path"), src["start_byte"], src["end_byte"], key, d, names))
    table = {}
    for o in occ:
        k = (o["kind"], o["name"])
        row = table.setdefault(k, {"kind": o["kind"], "name": o["name"], "count": 0, "user_defined": False,
                                   "status": "no-diagnostic", "diagnostics": {}, "in_math": 0})
        row["count"] += 1
        row["in_math"] += 1 if o.get("math") else 0
        row["user_defined"] = row["user_defined"] or bool(o.get("user_defined"))
        hits = list(by_name.get(k, []))
        if o["kind"] == "cs" and o["name"].endswith("*"):
            hits += by_name.get(("cs", o["name"][:-1]), [])
        for key, unsupported, d in hits:
            row["diagnostics"].setdefault(key, {"count": 0, "severity": d.get("severity"), "code": d.get("code"), "example": d.get("message")})
        if hits:
            if any(u for _, u, _ in hits):
                row["status"] = "unsupported"
            elif row["status"] != "unsupported":
                row["status"] = "recovered"
        else:
            # A diagnostic whose span covers this occurrence but names nothing,
            # or one that names something else and starts exactly here (a macro
            # call whose expansion produced the diagnostic).
            for (p, s, e, key, d, names) in spans:
                if p != o["path"]:
                    continue
                covered = s <= o["start"] < e and (e - s) <= 256
                if (not names and covered) or (names and s == o["start"] and o["kind"] == "cs"):
                    row["diagnostics"].setdefault(key, {"count": 0, "severity": d.get("severity"), "code": d.get("code"), "example": d.get("message")})
                    if row["status"] == "no-diagnostic":
                        row["status"] = "recovered"
    # Count how many times each construct's diagnostics fired (not per occurrence).
    counts = {}
    for d in diags:
        counts[diag_key(d.get("message", ""))] = counts.get(diag_key(d.get("message", "")), 0) + 1
    for row in table.values():
        for key, info in row["diagnostics"].items():
            info["count"] = counts.get(key, 0)
    return table


# ----------------------------------------------------------------------------
# report


def md_escape(s):
    return s.replace("|", "\\|").replace("\n", " ")


def code(s):
    return "`" + s.replace("`", "'") + "`"


def excerpt(fx_dir, path, start, end, width=48):
    try:
        data = open(os.path.join(fx_dir, path), "rb").read()
    except OSError:
        return ""
    lo = max(0, start - 12)
    hi = min(len(data), max(end, start + 1) + 12)
    s = data[lo:hi].decode("utf-8", "replace").replace("\n", "⏎")
    return s[:width]


def build_report(args, meta, fixtures_out, producers, rasterizer_note):
    lines = []
    L = lines.append
    L(f"# Real-world corpus report — {meta['run_utc']}")
    L("")
    L(f"Generated by `tools/real-world-corpus/run.py` on `{meta['host']}` ({meta['platform']}).")
    L(f"`uptime` at start: `{meta['uptime_start']}`; at end: `{meta['uptime_end']}`.")
    L("")
    L("Nothing in this report is evidence of parity. `differing` pixels are printed")
    L("for every compared page but a document whose status is `recovered` (or whose")
    L("producer PDF route lacks the reference's fonts) differs almost everywhere by")
    L("construction; the numbers are a floor to drive down, not a score. Absence of a")
    L("diagnostic for a construct (`no-diagnostic`) means only that the producer did")
    L("not complain — not that it rendered the construct like pdfLaTeX.")
    L("")
    L("## Producers and tools")
    L("")
    for p in producers:
        L(f"- `{p['name']}`: `{p['exe']}` (sha256 `{p['sha256'][:16]}…`, {p['note']})")
    L(f"- PDF route (render): `{meta.get('pdf_exact') or 'none'}`; PDF route (compiler, v1): `{meta.get('pdf_v1') or 'none'}`")
    L(f"- Oracle: `{meta.get('pdflatex_exe') or 'absent'}` — {meta.get('pdflatex_version') or 'not run'} (oracle only, never in the product path)")
    L(f"- Rasterizer: {rasterizer_note}")
    L(f"- Fonts for the producers: `FLASHTEX_FONT_DIRS={meta.get('font_dirs')}`, `FLASHTEX_TFM_DIRS={meta.get('tfm_dirs')}`")
    L("")
    L("## Per-document results")
    L("")
    L("| fixture | ref pages | producer | status | pages | diags (err/warn) | seconds | PDF | pages compared | differing px (per page) | max Δ |")
    L("|---|---|---|---|---|---|---|---|---|---|---|")
    for fx in fixtures_out:
        ref_pages = fx["reference"].get("pages") if fx.get("reference") else "—"
        for pr in fx["producers"]:
            errs = sum(1 for d in pr["diagnostics"] if d.get("severity") == "error")
            warns = len(pr["diagnostics"]) - errs
            pdf = pr.get("pdf", {})
            pdf_cell = "none" if not pdf else (f"{pdf.get('bytes', 0)} B" if pdf.get("ok") else f"failed: {md_escape(pdf.get('note', ''))[:60]}")
            cmp_pages = pr.get("compare", {}).get("pages", [])
            compared = [p for p in cmp_pages if p.get("result") == "compared"]
            diff_cell = ", ".join(f"{p['differing']} ({p['differing_fraction'] * 100:.1f}%)" for p in compared) or "—"
            missing = [p for p in cmp_pages if p.get("result") != "compared"]
            if missing:
                diff_cell += "; " + ", ".join(f"p{p['page']} {p['result']}" for p in missing)
            maxd = max((p["max_delta"] for p in compared), default="—")
            L(f"| {fx['id']} | {ref_pages} | {pr['producer']} | {pr['status']} | {pr['pages']} | {errs}/{warns} | {pr['seconds']} | {pdf_cell} | {len(compared)}/{len(cmp_pages)} | {diff_cell} | {maxd} |")
    L("")
    font_failures = [(fx["id"], pr) for fx in fixtures_out for pr in fx["producers"] if pr.get("font_diagnostics")]
    if font_failures:
        L("## Font environment failures — these runs are not measurements")
        L("")
        L("The renderer substituted metrics, so the geometry is not the reference's.")
        L("A `recovered` status alongside one of these means the document compiled,")
        L("not that it compiled correctly; page comparisons here are void.")
        L("")
        for fid, pr in font_failures:
            for d in pr["font_diagnostics"]:
                L(f"- `{fid}` / `{pr['producer']}`: `{d['code']}` — {md_escape(d['message'])}")
        L("")
    failures = [(fx["id"], pr) for fx in fixtures_out for pr in fx["producers"] if pr["status"] in ("no_reply", "error", "failed") or pr.get("timed_out")]
    if failures:
        L("## Producer failures (exact non-secret reproduction; owners patch, this lane does not)")
        L("")
        for fid, pr in failures:
            tail = " / ".join(l for l in pr["stderr_tail"].strip().splitlines()[-3:] if l and "RUST_BACKTRACE" not in l)
            L(f"- `{fid}` / `{pr['producer']}`: status `{pr['status']}`, exit `{pr['exit']}`, timed out `{pr['timed_out']}` — stderr: {md_escape(tail)}")
        L("")
    L("## Reference PDFs")
    L("")
    L("| fixture | file | sha256 | pages | provenance |")
    L("|---|---|---|---|---|")
    for fx in fixtures_out:
        r = fx.get("reference") or {}
        L(f"| {fx['id']} | {r.get('file', '—')} | `{(r.get('sha256') or '')[:16]}…` | {r.get('pages', '—')} | {md_escape(str(r.get('provenance', '')))} |")
        for extra in fx.get("extra_references", []):
            L(f"| {fx['id']} | {extra['file']} | `{extra['sha256'][:16]}…` | {extra.get('pages')} | {md_escape(extra.get('provenance', ''))}; pixel diff vs committed reference: {extra.get('diff_vs_reference', 'not compared')} |")
    L("")
    # ranked list
    L("## Ranked unsupported / recovered diagnostics (corpus-wide, per producer)")
    L("")
    L("Rank is by number of diagnostic occurrences across the whole corpus for the")
    L("`render` producer (which embeds the same compiler front end); the `compiler`")
    L("column shows this checkout's `flashtex-compiler`. Owner per")
    L("`coordination/authority.json` reading (compiler = Commander/main;")
    L("math-layout = FT-020; render-pipeline = mac-claude-a). Spans are")
    L("`path:start-end` UTF-8 byte offsets from the diagnostic itself.")
    L("")
    ranked = meta["ranked"]
    L("| # | count (render / compiler) | severity | diagnostic (normalised) | owner | fixtures | example spans |")
    L("|---|---|---|---|---|---|---|")
    for i, r in enumerate(ranked, 1):
        spans = "; ".join(f"`{s['fixture']}/{s['path']}:{s['start']}-{s['end']}` {code(s['excerpt'])}" for s in r["spans"][:3])
        pin = " **(Commander-designated first item)**" if r.get("pinned") else ""
        L(f"| {i} | {r['counts'].get('render', 0)} / {r['counts'].get('compiler', 0)} | {r['severity']} | {code(r['key'])}{pin} | {md_escape(r['owner'])} | {', '.join(r['fixtures'])} | {spans} |")
    L("")
    L("### Paste-ready queue items")
    L("")
    for i, r in enumerate(ranked, 1):
        counts = f"render {r['counts'].get('render', 0)} / compiler {r['counts'].get('compiler', 0)} occurrences"
        first = f"; first span `{r['spans'][0]['fixture']}/{r['spans'][0]['path']}:{r['spans'][0]['start']}-{r['spans'][0]['end']}` {code(r['spans'][0]['excerpt'])}" if r["spans"] else ""
        L(f"{i}. [{r['owner'].split(' (')[0]}] {code(r['key'])} — {counts} in {len(r['fixtures'])} fixture(s) ({', '.join(r['fixtures'])}){first}")
    L("")
    # construct table
    L("## Per-construct coverage (tokenizer-derived, corpus-wide)")
    L("")
    L("Status per producer: `unsupported` = a diagnostic names the construct and says it is")
    L("not supported/implemented; `recovered` = a diagnostic names it (other wording) or a")
    L("short diagnostic span covers an occurrence; `no-diagnostic` = the producer emitted")
    L("nothing about it (not proof of correct rendering). `user` = defined in the document.")
    L("")
    for pname in [p["name"] for p in producers]:
        L(f"### Producer `{pname}`")
        L("")
        L("| construct | kind | count | in math | status | diagnostic (count) |")
        L("|---|---|---|---|---|---|")
        rows = meta["constructs"][pname]
        for row in rows:
            diag = "; ".join(f"{code(k)} ({v['count']})" for k, v in sorted(row["diagnostics"].items(), key=lambda kv: -kv[1]["count"])[:3])
            L(f"| {code(row['name'])}{' (user)' if row['user_defined'] else ''} | {row['kind']} | {row['count']} | {row['in_math']} | {row['status']} | {diag} |")
        L("")
    L("## Notes")
    L("")
    for n in meta["notes"]:
        L(f"- {md_escape(n)}")
    return "\n".join(lines) + "\n"


# ----------------------------------------------------------------------------
# main


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--fixtures", default=DEFAULT_FIXTURES)
    ap.add_argument("--only", action="append", help="fixture id (repeatable)")
    ap.add_argument("--compiler", default=os.environ.get("FLASHTEX_COMPILER"), help="flashtex-compiler binary")
    ap.add_argument("--render", default=os.environ.get("FLASHTEX_RENDER"), help="flashtex-render binary")
    ap.add_argument("--render-note", default="", help="provenance note for the render binary (branch @ sha)")
    ap.add_argument("--compiler-note", default="", help="provenance note for the compiler binary")
    ap.add_argument("--pdf-exact", default=os.environ.get("FLASHTEX_PDF_EXACT"), help="flashtex-pdf-exact (from-v2 route for render)")
    ap.add_argument("--pdf-v1", default=os.environ.get("FLASHTEX_PDF"), help="flashtex-pdf (runtime-v1 route for compiler)")
    ap.add_argument("--font-dirs", default=os.environ.get("FLASHTEX_FONT_DIRS", os.path.join(REPO, "apps", "mac", "Fonts")))
    ap.add_argument("--tfm-dirs", default=os.environ.get("FLASHTEX_TFM_DIRS", os.path.join(REPO, "apps", "mac", "Fonts", "texmf", "fonts", "tfm", "public", "lm")))
    ap.add_argument("--texbin", default=DEFAULT_TEXBIN)
    ap.add_argument("--no-oracle", action="store_true", help="never run pdflatex")
    ap.add_argument("--regenerate", action="store_true", help="also regenerate references that already exist, as reference-mactex2026.pdf, and compare")
    ap.add_argument("--rasterizer", default=None, help="pdftoppm or gs path (auto)")
    ap.add_argument("--no-pixels", action="store_true")
    ap.add_argument("--out", default=None, help="output directory (default docs/evidence/real-world-corpus-<UTC>)")
    ap.add_argument("--top", type=int, default=30)
    args = ap.parse_args()

    run_utc = utc_now()
    out_dir = args.out or os.path.join(REPO, "docs", "evidence", f"real-world-corpus-{run_utc}")
    os.makedirs(out_dir, exist_ok=True)
    work = os.path.join(HERE, "target", "runs", run_utc)
    os.makedirs(work, exist_ok=True)
    notes = []
    meta = {
        "run_utc": run_utc,
        "host": platform.node(),
        "platform": f"{platform.system()} {platform.release()} {platform.machine()}",
        "uptime_start": uptime(),
        "load1_start": load1(),
        "font_dirs": args.font_dirs,
        "tfm_dirs": args.tfm_dirs,
        "pdf_exact": args.pdf_exact,
        "pdf_v1": args.pdf_v1,
        "notes": notes,
    }
    if not args.no_oracle:
        exe, ver = pdflatex_version(args.texbin)
        meta["pdflatex_exe"], meta["pdflatex_version"] = exe, ver
    producers = []
    for name, exe, note in (("compiler", args.compiler, args.compiler_note), ("render", args.render, args.render_note)):
        if exe and os.path.isfile(exe):
            producers.append({"name": name, "exe": exe, "sha256": sha256_file(exe), "note": note or "no provenance note given"})
        else:
            notes.append(f"producer {name} not run: binary not given or missing ({exe})")
    rast = (None, None) if args.no_pixels else find_rasterizer(args.rasterizer)
    if rast[0] in ("pdftoppm", "gs"):
        vcode, vout, verr, _, _ = run([rast[1], "--version"], timeout=30)
        rver = (vout or verr).decode("utf-8", "replace").strip().splitlines()[0] if (vout or verr) else "?"
        rasterizer_note = f"`{rast[0]}` at `{rast[1]}` ({rver}), 8-bit grey PGM at {DPI} dpi"
    elif rast[0] == "sips":
        rasterizer_note = "only `sips` found: it cannot select pages, so no pixel comparison was made"
    else:
        rasterizer_note = "none (pixel comparison skipped)"
    meta["rasterizer"] = rasterizer_note

    env = dict(os.environ, FLASHTEX_FONT_DIRS=args.font_dirs, FLASHTEX_TFM_DIRS=args.tfm_dirs)
    fixtures = discover_fixtures(args.fixtures)
    if args.only:
        fixtures = [f for f in fixtures if f["id"] in set(args.only)]
    fixtures_out = []
    all_diags = {p["name"]: [] for p in producers}
    all_occ = []
    for fx in fixtures:
        print(f"== {fx['id']} ({fx['entry']}, {len(fx['documents'])} document(s))", file=sys.stderr)
        fx_work = os.path.join(work, fx["id"])
        os.makedirs(fx_work, exist_ok=True)
        rec = {"id": fx["id"], "entry": fx["entry"], "documents": fx["documents"], "producers": [], "extra_references": []}
        # reference
        ref = find_reference(fx)
        sidecar_path = os.path.join(fx["dir"], "reference.json")
        sidecar = json.load(open(sidecar_path)) if os.path.isfile(sidecar_path) else {}
        generated_now = False
        if ref is None and not args.no_oracle:
            gen = generate_reference(fx, args.texbin, "reference.pdf", notes)
            if gen:
                generated_now = True
                gen["provenance"] = "generated by this harness with MacTeX pdflatex (oracle)"
                sidecar["reference"] = gen
                json.dump(sidecar, open(sidecar_path, "w"), indent=2)
                ref = os.path.join(fx["dir"], "reference.pdf")
        if ref is not None:
            r = dict(sidecar.get("reference") or {})
            r.setdefault("file", os.path.basename(ref))
            actual_sha = sha256_file(ref)
            if r.get("sha256") and r["sha256"] != actual_sha:
                notes.append(f"{fx['id']}: reference.json sha256 does not match {os.path.basename(ref)} on disk")
            r["sha256"] = actual_sha
            r.setdefault("pages", pdf_page_count(ref))
            r.setdefault("provenance", "committed reference (no sidecar record)")
            rec["reference"] = r
            prior = sidecar.get("mactex_regeneration")
            if prior and os.path.isfile(os.path.join(fx["dir"], prior.get("file", ""))) and not (args.regenerate and not args.no_oracle):
                prior = dict(prior)
                prior["sha256_on_disk_matches"] = sha256_file(os.path.join(fx["dir"], prior["file"])) == prior.get("sha256")
                rec["extra_references"].append(prior)
            if args.regenerate and not args.no_oracle and not generated_now:
                gen = generate_reference(fx, args.texbin, "reference-mactex2026.pdf", notes)
                if gen:
                    gen["provenance"] = "regenerated by this harness with MacTeX pdflatex; the committed reference is byte-immutable"
                    gen["byte_identical_to_reference"] = gen["sha256"] == actual_sha
                    rec["extra_references"].append(gen)
                    sidecar["mactex_regeneration"] = gen
                    json.dump(sidecar, open(sidecar_path, "w"), indent=2)
        else:
            rec["reference"] = None
            notes.append(f"{fx['id']}: no reference PDF")
        # rasterise reference once
        ref_pages = []
        if ref is not None and rast[0] in ("pdftoppm", "gs"):
            ref_pages, argv, note = rasterize(rast, ref, os.path.join(fx_work, "reference"))
            rec["reference"]["raster_argv"] = argv
            if note:
                notes.append(f"{fx['id']}: reference rasterisation: {note}")
            for extra in rec["extra_references"]:
                epages, _, enote = rasterize(rast, os.path.join(fx["dir"], extra["file"]), os.path.join(fx_work, "reference-mactex2026"))
                if epages and ref_pages:
                    cmp = compare_pages(ref_pages, epages)
                    extra["diff_vs_reference"] = ", ".join(
                        f"p{p['page']} {p.get('differing', p['result'])}" for p in cmp)
        # inventory
        occ = inventory(fx)
        rec["construct_occurrences"] = len(occ)
        all_occ.extend(occ)
        # producers
        for p in producers:
            extra_args = []
            v2_path = os.path.join(fx_work, "render.v2.json")
            if p["name"] == "render":
                extra_args = ["--v2", v2_path]
            pr = run_producer(p["name"], p["exe"], extra_args, fx, fx_work, env)
            pr["fixture"] = fx["id"]
            all_diags[p["name"]].extend(dict(d, fixture=fx["id"]) for d in pr["diagnostics"])
            # PDF route
            pdf_path = os.path.join(fx_work, f"{p['name']}.pdf")
            pdf = {}
            if p["name"] == "render" and args.pdf_exact and os.path.isfile(v2_path) and pr["status"] in ("ok", "recovered"):
                argv = [args.pdf_exact, "from-v2", v2_path, "--out", pdf_path, "--font-dir", args.font_dirs]
                code_, o, e, secs, to = run(argv, timeout=180)
                pdf = {"route": "flashtex-pdf-exact from-v2", "exit": code_, "seconds": round(secs, 3),
                       "ok": code_ == 0 and os.path.isfile(pdf_path), "bytes": os.path.getsize(pdf_path) if os.path.isfile(pdf_path) else 0,
                       "note": e.decode("utf-8", "replace")[-400:] if code_ != 0 else "",
                       "stderr_lines": len(e.decode("utf-8", "replace").splitlines())}
            elif p["name"] == "compiler" and args.pdf_v1 and pr["status"] in ("ok", "recovered"):
                reply = os.path.join(fx_work, "compiler.reply.jsonl")
                first = os.path.join(fx_work, "compiler.result.json")
                with open(reply, "rb") as fin, open(first, "wb") as fout:
                    for raw in fin:
                        if b'"compile_result"' in raw:
                            fout.write(raw)
                            break
                argv = [args.pdf_v1, first, "--out", pdf_path, "--verify"]
                code_, o, e, secs, to = run(argv, timeout=180)
                pdf = {"route": "flashtex-pdf (runtime-v1 text items, base-14 fonts)", "exit": code_, "seconds": round(secs, 3),
                       "ok": code_ == 0 and os.path.isfile(pdf_path), "bytes": os.path.getsize(pdf_path) if os.path.isfile(pdf_path) else 0,
                       "note": e.decode("utf-8", "replace")[-400:] if code_ != 0 else ""}
            pr["pdf"] = pdf
            if pdf.get("ok"):
                pr["pdf"]["pages"] = pdf_page_count(pdf_path)
            # pixels
            if pdf.get("ok") and ref_pages:
                cand_pages, argv, note = rasterize(rast, pdf_path, os.path.join(fx_work, p["name"]))
                if note:
                    notes.append(f"{fx['id']}/{p['name']}: rasterisation: {note}")
                pr["compare"] = {"rasterizer": rast[0], "argv": argv, "pages": compare_pages(ref_pages, cand_pages) if cand_pages else []}
            else:
                pr["compare"] = {"pages": []}
            rec["producers"].append(pr)
            font_bad = fontenv.font_diagnostics(pr["diagnostics"])
            pr["font_diagnostics"] = [{"code": d.get("code"), "message": (d.get("message") or "")[:200]} for d in font_bad]
            print(f"   {p['name']}: {pr['status']} pages={pr['pages']} diags={len(pr['diagnostics'])} {pr['seconds']}s pdf={'ok' if pdf.get('ok') else pdf.get('note', 'none')[:40]}", file=sys.stderr)
            if font_bad:
                # "recovered" here would be a lie: the renderer substituted
                # metrics, so the page geometry is not the reference's and
                # nothing measured against it means anything.
                fontenv.report_font_failure(f"{fx['id']}/{p['name']}", font_bad, env)
        fixtures_out.append(rec)

    # corpus-wide constructs per producer
    meta["constructs"] = {}
    for p in producers:
        table = classify(all_occ, all_diags[p["name"]])
        order = {"unsupported": 0, "recovered": 1, "no-diagnostic": 2}
        rows = sorted(table.values(), key=lambda r: (order[r["status"]], -r["count"], r["name"]))
        meta["constructs"][p["name"]] = rows
    # ranked diagnostics (render primary, compiler secondary)
    agg = {}
    for pname, diags in all_diags.items():
        for d in diags:
            key = diag_key(d.get("message", ""))
            if not UNSUPPORTED_RE.search(d.get("message", "")) and d.get("severity") != "error":
                continue  # warnings like overfull boxes are reported in the construct table, not ranked
            a = agg.setdefault(key, {"key": key, "counts": {}, "severity": d.get("severity"), "code": d.get("code"),
                                     "owner": owner_for(d), "fixtures": set(), "spans": [], "example": d.get("message")})
            a["counts"][pname] = a["counts"].get(pname, 0) + 1
            a["fixtures"].add(d["fixture"])
            if d.get("code") and not a.get("owner_from_code"):
                a["owner"], a["owner_from_code"] = owner_for(d), True
            src = d.get("source") or {}
            if isinstance(src.get("start_byte"), int) and (len(a["spans"]) < 6 or (pname == "render" and a.get("spans_from") != "render")):
                if pname == "render" and a.get("spans_from") != "render":
                    a["spans"], a["spans_from"] = [], "render"
                elif a.get("spans_from") is None:
                    a["spans_from"] = pname
                fx_dir = os.path.join(args.fixtures, d["fixture"])
                a["spans"].append({"fixture": d["fixture"], "path": src.get("path"), "start": src["start_byte"], "end": src["end_byte"],
                                   "excerpt": excerpt(fx_dir, src.get("path", ""), src["start_byte"], src["end_byte"])})
    for a in agg.values():
        a.pop("owner_from_code", None)
        a["spans"] = a["spans"][:6]
    ranked = sorted(agg.values(), key=lambda a: (-max(a["counts"].get("render", 0), a["counts"].get("compiler", 0)), a["key"]))
    for a in ranked:
        a["fixtures"] = sorted(a["fixtures"])
        a["pinned"] = any(a["key"].startswith(p) for p in PINNED_FIRST)
    ranked = [a for a in ranked if a["pinned"]] + [a for a in ranked if not a["pinned"]]
    meta["ranked"] = ranked[: args.top]
    meta["ranked_total_keys"] = len(ranked)
    meta["uptime_end"] = uptime()
    meta["load1_end"] = load1()

    report = {
        "schema_version": 1,
        "meta": {k: v for k, v in meta.items() if k not in ("constructs", "ranked", "notes")},
        "producers": producers,
        "fixtures": fixtures_out,
        "ranked": meta["ranked"],
        "constructs": meta["constructs"],
        "notes": notes,
    }
    with open(os.path.join(out_dir, "report.json"), "w", encoding="utf-8") as f:
        json.dump(report, f, indent=1, ensure_ascii=False)
    with open(os.path.join(out_dir, "report.md"), "w", encoding="utf-8") as f:
        f.write(build_report(args, meta, fixtures_out, producers, rasterizer_note))
    print(f"wrote {rel(out_dir)}/report.md and report.json; scratch in {rel(work)}", file=sys.stderr)

    # A run whose renderer substituted metrics is not a measurement, so it
    # must not exit 0 -- otherwise a caller (or CI) reads "recovered" and
    # records geometry that was never compared against the real fonts.
    n = sum(len(pr.get("font_diagnostics") or []) for fx in fixtures_out for pr in fx["producers"])
    if n:
        print(f"FONT-ENV FAILURE: {n} font diagnostic(s) across the corpus; "
              "these runs used substituted metrics and are not comparable to the "
              "references. See the report's font-environment section.", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main() or 0)
