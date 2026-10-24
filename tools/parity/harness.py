"""Small helpers the parity scoreboard shares (oracle tooling, standard library only).

They came from tools/real-world-corpus/run.py, the old engine's corpus
harness, when retirement stage S7 deleted it (docs/design/engine-v2/
retirement-plan.md §1.8): running a command with a time limit, file digests,
the fixtures under fixtures/, the local pdflatex's version, the L4
rasteriser and its PGM reader, diagnostic keys for the report, and the
committed reference of a new fixture:

    python3 tools/parity/harness.py reference fixtures/real-world/<id> [--texbin DIR]

writes <id>/reference.pdf (pdflatex, SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1,
run until the PDF stops changing) and its sidecar <id>/reference.json. It
never overwrites a reference: a user-provided one is byte-immutable.
pdflatex is an oracle only, never in the product path.
"""

import argparse
import datetime as _dt
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time

DEFAULT_TEXBIN = "/usr/local/texlive/2026/bin/universal-darwin"


DPI = 144
HERE = os.path.dirname(os.path.abspath(__file__))
# How many pdflatex passes a reference may take before it counts as unsettled.
MAX_ORACLE_PASSES = 6


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def read_text(path):
    with open(path, "rb") as f:
        return f.read().decode("utf-8", "replace")


def run(cmd, stdin_bytes=None, env=None, timeout=300, cwd=None):
    t0 = time.monotonic()
    try:
        p = subprocess.run(cmd, input=stdin_bytes, capture_output=True, env=env, timeout=timeout, cwd=cwd)
        return p.returncode, p.stdout, p.stderr, time.monotonic() - t0, False
    except subprocess.TimeoutExpired as e:
        return None, e.stdout or b"", e.stderr or b"", time.monotonic() - t0, True


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


def pdflatex_version(texbin):
    exe = os.path.join(texbin, "pdflatex")
    if not os.path.isfile(exe):
        return None, None
    code, out, _, _, _ = run([exe, "--version"], timeout=30)
    first = out.decode("utf-8", "replace").splitlines()[0] if out else ""
    return exe, first


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


def md_escape(s):
    return s.replace("|", "\\|").replace("\n", " ")


def pdf_page_count(path):
    """Number of pages in `path` (pdftext's page tree), else a byte scan, else None."""
    try:
        import pdftext  # tools/parity/pdftext.py
        n = len(pdftext.PdfDocument.load(path).pages())
        if n:
            return n
    except Exception:  # noqa: BLE001 - any parse failure falls back to the scan
        pass
    with open(path, "rb") as f:
        data = f.read()
    n = len(re.findall(rb"/Type\s*/Page(?![s/a-zA-Z])", data))
    return n or None


def generate_reference(fx, texbin, out_name, log):
    """Run pdflatex in a scratch copy of the fixture until the PDF it writes
    stops changing (neither the pass count nor "Rerun to get" is the test:
    hyperref-toc needed three passes, lmodern-report three without asking),
    then copy that PDF to <fixture>/<out_name>. Returns the sidecar record,
    or None if it never settled."""
    exe, version = pdflatex_version(texbin)
    if exe is None:
        log.append(f"{fx['id']}: no pdflatex at {texbin}; cannot generate a reference")
        return None
    scratch = os.path.join(HERE, "target", "oracle", fx["id"])
    shutil.rmtree(scratch, ignore_errors=True)
    shutil.copytree(fx["dir"], scratch, ignore=shutil.ignore_patterns("*.pdf", "*.json", "*.md"))
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
    argv = [exe, "-interaction=nonstopmode", "-halt-on-error", fx["entry"]]
    produced = os.path.join(scratch, os.path.splitext(fx["entry"])[0] + ".pdf")
    logpath = os.path.join(scratch, os.path.splitext(fx["entry"])[0] + ".log")
    passes, previous = 0, None
    for _attempt in range(MAX_ORACLE_PASSES):
        code, _, _, _, timed_out = run(argv, env=env, timeout=300, cwd=scratch)
        passes += 1
        if timed_out or code != 0:
            log.append(f"{fx['id']}: pdflatex failed (exit {code}, timeout={timed_out})")
            return None
        if not os.path.isfile(produced):
            log.append(f"{fx['id']}: pdflatex produced no PDF")
            return None
        current = sha256_file(produced)
        if current == previous and "Rerun to get" not in read_text(logpath):
            break
        previous = current
    else:
        log.append(f"{fx['id']}: pdflatex did not converge in {MAX_ORACLE_PASSES} passes; "
                   "the reference is not a settled document and was not written")
        return None
    dest = os.path.join(fx["dir"], out_name)
    shutil.copyfile(produced, dest)
    logtxt = read_text(logpath)
    m = re.search(r"Output written on .*?\((\d+) pages?", logtxt)
    return {
        "file": out_name,
        "sha256": sha256_file(dest),
        "pages": int(m.group(1)) if m else pdf_page_count(dest),
        "pdflatex": version,
        "argv": ["SOURCE_DATE_EPOCH=0", "FORCE_SOURCE_DATE=1", os.path.basename(argv[0])] + argv[1:]
                + [f"(x{passes}, to convergence)"],
        "converged_after_passes": passes,
        "generated_utc": _dt.datetime.now(_dt.timezone.utc).strftime("%Y-%m-%dT%H%M%SZ"),
        "overfull_boxes": len(re.findall(r"^Overfull", logtxt, re.M)),
        "latex_warnings": len(re.findall(r"LaTeX Warning", logtxt)),
    }


def main(argv=None):
    ap = argparse.ArgumentParser(description="the committed pdflatex reference of a new fixture")
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("reference", help="write <fixture>/reference.pdf and reference.json")
    r.add_argument("fixture", help="a fixture directory (fixtures/real-world/<id>, ...)")
    r.add_argument("--texbin", default=DEFAULT_TEXBIN, help="the directory holding pdflatex")
    a = ap.parse_args(argv)
    d = os.path.abspath(a.fixture.rstrip("/"))
    fx = next((f for f in discover_fixtures(os.path.dirname(d)) if f["dir"] == d), None)
    if fx is None:
        print(f"{a.fixture}: not a fixture (one main.tex, or a single .tex file)", file=sys.stderr)
        return 2
    existing = [f for f in os.listdir(d) if f == "reference.pdf" or f.endswith("-reference.pdf")]
    if existing:
        print(f"{a.fixture}: already has {existing[0]}; a reference is never overwritten", file=sys.stderr)
        return 1
    log = []
    gen = generate_reference(fx, a.texbin, "reference.pdf", log)
    for line in log:
        print(line, file=sys.stderr)
    if not gen:
        return 1
    gen["provenance"] = "generated by tools/parity/harness.py with pdflatex (oracle only, never in the product path)"
    sidecar_path = os.path.join(d, "reference.json")
    sidecar = json.load(open(sidecar_path)) if os.path.isfile(sidecar_path) else {}
    sidecar["reference"] = gen
    with open(sidecar_path, "w") as f:
        json.dump(sidecar, f, indent=2)
        f.write("\n")
    print(f"{a.fixture}: reference.pdf, {gen['pages']} pages, {gen['converged_after_passes']} passes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
