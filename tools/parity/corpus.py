#!/usr/bin/env python3
"""Tier (b) of the parity scoreboard: public sources, fetched reproducibly.

Oracle tooling only (standard library). Third-party sources are **never
committed**: a manifest under `tools/parity/corpus/` pins what to fetch (an
arXiv identifier *with its version*, or a file of the local TeX Live
distribution) and the SHA-256 of the exact bytes, and this module fetches,
verifies and unpacks them into a cache outside the repository.

    python3 tools/parity/corpus.py fetch                 # every manifest
    python3 tools/parity/corpus.py fetch --manifest tools/parity/corpus/arxiv-2025-01.json
    python3 tools/parity/corpus.py select-arxiv --out tools/parity/corpus/arxiv-2025-01.json \\
        --from 202501130000 --to 202501172359 --per-category 10 math.AG math.PR cs.LG hep-th ...

`select-arxiv` is how the committed arXiv manifest was drawn (recorded in the
manifest's `selection`): for each category, the export API's papers with that
*primary* category submitted in the window, oldest first; each is fetched
(one request every `--delay` seconds, arXiv's published limit for automated
access is one every three) and kept when its e-print is TeX source with a
top-level file that has both `\\documentclass` and `\\begin{document}`, until
`--per-category` are kept. PDF-only submissions are skipped and counted.
Re-running it on another day may pick different papers (new versions,
withdrawals); `fetch` on the committed manifest never does: a version-pinned
e-print is immutable, and a changed hash is reported, not accepted.

Cache layout (`--cache`, default `$FLASHTEX_PARITY_CACHE` or
`~/.cache/flashtex-parity`):

    eprints/<id>            the downloaded bytes (verified against the manifest)
    src/<tier>/<doc-id>/    the unpacked tree the oracle and FlashTeX both read
"""

import argparse
import gzip
import hashlib
import io
import json
import os
import re
import shutil
import sys
import tarfile
import time
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
MANIFEST_DIR = os.path.join(HERE, "corpus")
DEFAULT_TEXMF = "/usr/local/texlive/2026/texmf-dist"
USER_AGENT = "flashtex-parity-scoreboard/1 (oracle corpus fetch; https://github.com/flash-tex/flashtex)"
ATOM = {"a": "http://www.w3.org/2005/Atom", "arxiv": "http://arxiv.org/schemas/atom"}


def default_cache():
    return os.environ.get("FLASHTEX_PARITY_CACHE") or os.path.join(
        os.path.expanduser("~"), ".cache", "flashtex-parity")


def slurp(path, mode="r", **kw):
    """The whole file, closed again (text unless mode is "rb")."""
    with open(path, mode, **kw) as f:
        return f.read()


def sha256_bytes(data):
    return hashlib.sha256(data).hexdigest()


def safe_id(ident):
    return re.sub(r"[^A-Za-z0-9._-]", "_", ident)


# ----------------------------------------------------------------------------
# unpacking and entry detection


def unpack(data, dest):
    """Unpack an arXiv e-print (tar.gz, tar, or a gzipped single file) into
    `dest`. Returns the format name, or "pdf" / "unknown" when there is no TeX
    source. Members that would escape `dest` are skipped."""
    shutil.rmtree(dest, ignore_errors=True)
    os.makedirs(dest)
    if data[:4] == b"%PDF":
        return "pdf"
    raw = data
    fmt = "tar"
    if data[:2] == b"\x1f\x8b":
        raw = gzip.decompress(data)
        fmt = "tar.gz"
    try:
        with tarfile.open(fileobj=io.BytesIO(raw)) as tf:
            root = os.path.realpath(dest)
            for m in tf.getmembers():
                target = os.path.realpath(os.path.join(dest, m.name))
                if not (target == root or target.startswith(root + os.sep)):
                    continue
                if m.isdir():
                    os.makedirs(target, exist_ok=True)
                elif m.isfile():
                    os.makedirs(os.path.dirname(target), exist_ok=True)
                    with tf.extractfile(m) as src, open(target, "wb") as out:
                        shutil.copyfileobj(src, out)
            return fmt
    except tarfile.TarError:
        pass
    if raw[:4] == b"%PDF":
        return "pdf"
    if data[:2] == b"\x1f\x8b" and re.search(rb"\\(documentclass|documentstyle|begin|input)", raw[:200000]):
        with open(os.path.join(dest, "main.tex"), "wb") as f:
            f.write(raw)
        return "gz"
    return "unknown"


def detect_entry(root):
    """The top-level file pdflatex would be run on: a .tex file directly in
    `root` with `\\documentclass` and `\\begin{document}` outside comments.
    Several: `main.tex`/`ms.tex`, else the one named by an arXiv 00README
    `toplevelfile`, else the largest. None when there is none."""
    cands = []
    readme_top = None
    for name in sorted(os.listdir(root)):
        p = os.path.join(root, name)
        if name.startswith("00README"):
            try:
                txt = slurp(p, encoding="utf-8", errors="replace")
                m = re.search(r"^\s*([^\s]+\.tex)\s+toplevelfile", txt, re.M) or re.search(
                    r"toplevel\S*\s*[:=]?\s*([^\s]+\.tex)", txt)
                if m:
                    readme_top = m.group(1)
            except OSError:
                pass
        if not name.endswith(".tex") or not os.path.isfile(p):
            continue
        txt = slurp(p, encoding="latin-1")
        body = "\n".join(line.split("%", 1)[0] for line in txt.splitlines())
        if re.search(r"\\documentclass", body) and re.search(r"\\begin\s*\{document\}", body):
            cands.append((name, len(txt)))
    if not cands:
        return None
    names = [c[0] for c in cands]
    if readme_top in names:
        return readme_top
    for pref in ("main.tex", "ms.tex", "paper.tex"):
        if pref in names:
            return pref
    return max(cands, key=lambda c: c[1])[0]


def uses_other_engine(root, entry):
    """True when the entry obviously needs XeLaTeX/LuaLaTeX (fontspec etc.).
    Informational only: the oracle is still pdflatex, which then fails."""
    try:
        txt = slurp(os.path.join(root, entry), encoding="latin-1")
    except OSError:
        return False
    return bool(re.search(r"\\usepackage(\[[^\]]*\])?\{[^}]*(fontspec|unicode-math|xeCJK|luatexja)", txt))


# ----------------------------------------------------------------------------
# network


def http_get(url, timeout=120):
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return r.read()


def eprint_url(ident):
    return f"https://export.arxiv.org/e-print/{ident}"


def arxiv_query(category, date_from, date_to, max_results):
    q = f"cat:{category} AND submittedDate:[{date_from} TO {date_to}]"
    url = ("https://export.arxiv.org/api/query?" + urllib.parse.urlencode(
        {"search_query": q, "sortBy": "submittedDate", "sortOrder": "ascending", "start": 0,
         "max_results": max_results}))
    root = ET.fromstring(http_get(url))
    out = []
    for e in root.findall("a:entry", ATOM):
        ident = e.findtext("a:id", default="", namespaces=ATOM).rsplit("/abs/", 1)[-1]
        prim = e.find("arxiv:primary_category", ATOM)
        out.append({"id": ident, "title": " ".join((e.findtext("a:title", default="", namespaces=ATOM)).split()),
                    "primary_category": prim.get("term") if prim is not None else None,
                    "published": e.findtext("a:published", default="", namespaces=ATOM)})
    return url, out


# ----------------------------------------------------------------------------
# commands


def cmd_select_arxiv(args):
    cache = args.cache
    os.makedirs(os.path.join(cache, "eprints"), exist_ok=True)
    entries, stats = [], {}
    last = [0.0]

    def polite():
        wait = args.delay - (time.time() - last[0])
        if wait > 0:
            time.sleep(wait)
        last[0] = time.time()

    for cat in args.categories:
        polite()
        url, hits = arxiv_query(cat, args.date_from, args.date_to, args.pool)
        kept = skipped_pdf = skipped_other = 0
        for h in hits:
            if kept >= args.per_category:
                break
            if h["primary_category"] != cat or not re.search(r"v\d+$", h["id"]):
                continue
            path = os.path.join(cache, "eprints", safe_id(h["id"]))
            if os.path.isfile(path):
                data = slurp(path, "rb")
            else:
                polite()
                try:
                    data = http_get(eprint_url(h["id"]))
                except Exception as e:  # noqa: BLE001 - recorded, not fatal
                    print(f"  {h['id']}: fetch failed: {e}", file=sys.stderr)
                    skipped_other += 1
                    continue
                with open(path, "wb") as f:
                    f.write(data)
            dest = os.path.join(cache, "src", "arxiv", safe_id(h["id"]))
            fmt = unpack(data, dest)
            entry = detect_entry(dest) if fmt not in ("pdf", "unknown") else None
            if entry is None:
                if fmt == "pdf":
                    skipped_pdf += 1
                else:
                    skipped_other += 1
                continue
            kept += 1
            entries.append({"id": h["id"], "category": cat, "title": h["title"], "published": h["published"],
                            "url": eprint_url(h["id"]), "sha256": sha256_bytes(data), "bytes": len(data),
                            "format": fmt, "entry": entry,
                            "other_engine_hint": uses_other_engine(dest, entry)})
            print(f"  {cat} {h['id']} {fmt} entry={entry}", file=sys.stderr)
        stats[cat] = {"query": url, "kept": kept, "skipped_pdf_only": skipped_pdf, "skipped_other": skipped_other}
        print(f"{cat}: kept {kept}, pdf-only {skipped_pdf}, other {skipped_other}", file=sys.stderr)
    manifest = {
        "schema": "flashtex-parity-corpus/1",
        "tier": "arxiv",
        "licence_note": ("arXiv e-prints are the authors' copyright under the licence each chose; they are "
                         "fetched into a local cache for measurement and never committed to this repository."),
        "selection": {"method": "tools/parity/corpus.py select-arxiv", "categories": args.categories,
                      "submitted_from": args.date_from, "submitted_to": args.date_to,
                      "per_category": args.per_category, "pool": args.pool,
                      "selected_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "per_category_stats": stats},
        "entries": entries,
    }
    with open(args.out, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=1, ensure_ascii=False)
        f.write("\n")
    print(f"wrote {args.out}: {len(entries)} entries", file=sys.stderr)
    return 0


def fetch_manifest(manifest_path, cache, texmf=DEFAULT_TEXMF, delay=3.0, log=print):
    """Fetch + verify + unpack every entry. Returns list of document records
    {id, tier, dir, entry, source, problem}. Idempotent: cached bytes are
    re-verified, not re-downloaded."""
    with open(manifest_path, encoding="utf-8") as f:
        man = json.load(f)
    tier = man["tier"]
    docs = []
    last = 0.0
    for e in man["entries"]:
        doc_id = safe_id(e["id"])
        dest = os.path.join(cache, "src", tier, doc_id)
        rec = {"id": doc_id, "tier": tier, "dir": dest, "entry": e.get("entry"), "source": e.get("url") or e.get("path"),
               "category": e.get("category"), "problem": None}
        if tier == "arxiv":
            path = os.path.join(cache, "eprints", doc_id)
            if not os.path.isfile(path):
                wait = delay - (time.time() - last)
                if wait > 0:
                    time.sleep(wait)
                last = time.time()
                try:
                    data = http_get(e["url"])
                except Exception as ex:  # noqa: BLE001
                    rec["problem"] = f"fetch failed: {ex}"
                    docs.append(rec)
                    continue
                with open(path, "wb") as f:
                    f.write(data)
            data = slurp(path, "rb")
            if sha256_bytes(data) != e["sha256"]:
                rec["problem"] = f"sha256 mismatch: manifest {e['sha256'][:12]}, fetched {sha256_bytes(data)[:12]}"
                docs.append(rec)
                continue
            marker = os.path.join(dest, ".parity-unpacked")
            if not (os.path.isfile(marker) and slurp(marker) == e["sha256"]):
                unpack(data, dest)
                with open(marker, "w") as f:
                    f.write(e["sha256"])
        elif tier == "templates":
            src = os.path.join(texmf, e["path"])
            if not os.path.isfile(src):
                rec["problem"] = f"missing in TeX Live: {src}"
                docs.append(rec)
                continue
            got = hashlib.sha256(slurp(src, "rb")).hexdigest()
            if got != e["sha256"]:
                rec["problem"] = f"sha256 mismatch for {e['path']}: manifest {e['sha256'][:12]}, local {got[:12]}"
                docs.append(rec)
                continue
            shutil.rmtree(dest, ignore_errors=True)
            srcdir = os.path.dirname(src)
            if e.get("copy_dir"):
                # the template's own directory: its figures, .bib and \input files
                shutil.copytree(srcdir, dest, ignore=shutil.ignore_patterns("*.pdf") if e.get("skip_pdfs") else None)
            else:
                os.makedirs(dest)
                for extra in [os.path.basename(src)] + e.get("files", []):
                    p = os.path.join(srcdir, extra)
                    if os.path.isfile(p):
                        shutil.copyfile(p, os.path.join(dest, os.path.basename(extra)))
            rec["entry"] = os.path.basename(src)
        else:
            rec["problem"] = f"unknown tier {tier}"
        docs.append(rec)
    log(f"{os.path.relpath(manifest_path, REPO)}: {len(docs)} documents "
        f"({sum(1 for d in docs if d['problem'])} with problems)")
    return docs


def manifests(paths=None):
    if paths:
        return paths
    return sorted(os.path.join(MANIFEST_DIR, f) for f in os.listdir(MANIFEST_DIR) if f.endswith(".json"))


def cmd_fetch(args):
    bad = 0
    for m in manifests(args.manifest):
        for d in fetch_manifest(m, args.cache, args.texmf, args.delay):
            if d["problem"]:
                bad += 1
                print(f"  {d['tier']}/{d['id']}: {d['problem']}", file=sys.stderr)
    return 1 if bad else 0


def cmd_hash_templates(args):
    """Fill in `sha256` for a templates manifest from the local TeX Live."""
    with open(args.manifest, encoding="utf-8") as f:
        man = json.load(f)
    for e in man["entries"]:
        p = os.path.join(args.texmf, e["path"])
        e["sha256"] = hashlib.sha256(slurp(p, "rb")).hexdigest()
    with open(args.manifest, "w", encoding="utf-8") as f:
        json.dump(man, f, indent=1, ensure_ascii=False)
        f.write("\n")
    return 0


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--cache", default=default_cache())
    ap.add_argument("--texmf", default=DEFAULT_TEXMF)
    ap.add_argument("--delay", type=float, default=3.0, help="seconds between arXiv requests")
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("select-arxiv")
    s.add_argument("--out", required=True)
    s.add_argument("--from", dest="date_from", required=True, help="YYYYMMDDHHMM")
    s.add_argument("--to", dest="date_to", required=True, help="YYYYMMDDHHMM")
    s.add_argument("--per-category", type=int, default=10)
    s.add_argument("--pool", type=int, default=60, help="API results considered per category")
    s.add_argument("categories", nargs="+")
    f = sub.add_parser("fetch")
    f.add_argument("--manifest", action="append", default=[])
    h = sub.add_parser("hash-templates")
    h.add_argument("--manifest", required=True)
    args = ap.parse_args(argv)
    return {"select-arxiv": cmd_select_arxiv, "fetch": cmd_fetch, "hash-templates": cmd_hash_templates}[args.cmd](args)


if __name__ == "__main__":
    sys.exit(main())
