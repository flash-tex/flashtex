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

    python3 tools/parity/corpus.py select-arxiv-grid --out tools/parity/corpus/nightly-5k.json \\
        --years 2016-2025 --per-cell 25 math.AG cs.LG hep-th ...     # the nightly T4 corpus
    python3 tools/parity/corpus.py fetch --tier nightly-5k           # on-demand manifests by tier

`select-arxiv-grid` draws the large nightly corpus the same way, one cell
per (primary category, year), each from a window whose start day is a
SHA-256 of the seed, category and year (`grid_window`); the rule, seed and
every cell's query and counts are recorded in the manifest's `selection`.
Such a manifest is `on_demand`: a bare `fetch` skips it.

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
    archives/<id>           the same, for an archive tier's source archive
    src/<tier>/<doc-id>/    the unpacked tree the oracle and FlashTeX both read

An archive tier (`ARCHIVE_TIERS`: `books`) pins a whole project's source
archive at a fixed commit by URL and SHA-256, as an e-print is pinned. An
entry's `root` names the archive's top directory, which becomes the tree's
root, so the entry's relative `\\input`s resolve as in the project's checkout.
"""

import argparse
import datetime
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
# tiers whose entries are files of the local TeX Live, copied (never committed);
# an entry with `repo` instead names a document committed in this repository
# (the beamer tier's own small decks), copied with its directory, unpinned
# because Git pins it
TEXLIVE_TIERS = ("templates", "packages", "beamer")
# Bumped when `unpack` makes a different tree from the same bytes. 2: files keep the archive's times.
UNPACK_V = 2
USER_AGENT = "flashtex-parity-scoreboard/1 (oracle corpus fetch; https://github.com/flash-tex/flashtex)"
# tiers whose entries are arXiv e-prints, fetched and pinned by SHA-256
ARXIV_TIERS = ("arxiv", "nightly-5k")
# tiers whose entries are a project's source archive at a pinned commit (url + SHA-256),
# with `root` the archive's top directory (a textbook's repository snapshot, say)
ARCHIVE_TIERS = ("books",)
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
                    # the archive's times, not the unpack's: epstopdf compares an EPS's date with
                    # its conversion's, so a re-unpack must not make a cached oracle's seed stale.
                    # A time the OS can't set (a pax mtime of 1e20) keeps the unpack time.
                    try:
                        os.utime(target, (m.mtime, m.mtime))
                    except (OverflowError, OSError, ValueError):
                        pass
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


def unpack_root(data, dest, root):
    """`unpack` an archive whose tree is under its top directory `root`, as
    a forge's commit archive is, and make that directory `dest`, so the
    entry's relative paths resolve as in the project's checkout. Raises
    ValueError when the archive has no such directory."""
    tmp = f"{dest}.tmp-{os.getpid()}"
    unpack(data, tmp)
    top = os.path.realpath(os.path.join(tmp, root))
    if not (top.startswith(os.path.realpath(tmp) + os.sep) and os.path.isdir(top)):
        shutil.rmtree(tmp, ignore_errors=True)
        raise ValueError(f"archive has no top directory {root!r}")
    shutil.rmtree(dest, ignore_errors=True)
    os.rename(top, dest)
    shutil.rmtree(tmp, ignore_errors=True)


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


class Polite:
    """At most one arXiv request every `delay` seconds (arXiv's published
    limit for automated access is one every three), shared by every call."""

    def __init__(self, delay):
        self.delay, self.last = delay, 0.0

    def __call__(self):
        wait = self.delay - (time.time() - self.last)
        if wait > 0:
            time.sleep(wait)
        self.last = time.time()


def with_retries(fn, polite, tries=3, log=None):
    """`fn()` with up to `tries` attempts, each after `polite()`, backing off
    (30 s, 60 s, ...) after an error such as arXiv's 503 "retry later"."""
    for i in range(tries):
        polite()
        try:
            return fn()
        except Exception as e:  # noqa: BLE001 - retried, then raised
            if i == tries - 1 or getattr(e, "code", None) in (403, 404, 410):  # no retry makes these appear
                raise
            wait = 30 * (i + 1)
            if getattr(e, "code", None) in THROTTLED:  # arXiv asks us to slow down: honour Retry-After
                try:
                    wait = max(wait, int((e.headers or {}).get("Retry-After") or 0), 300 * (i + 1))
                except (TypeError, ValueError):
                    wait = max(wait, 300 * (i + 1))
            if log:
                log(f"  retry {i + 1}/{tries - 1} in {wait} s after: {e}")
            time.sleep(wait)
    return None


THROTTLED = (429, 503)


class Throttled(Exception):
    """arXiv kept refusing (429/503) after the retries: the selection stops
    rather than skip an e-print, which would change the draw."""


def draw_category(cat, date_from, date_to, per_category, pool, cache, polite, log=print, keep_src=True):
    """The e-prints `select-arxiv` keeps for one primary category and window:
    the export API's papers with that *primary* category submitted in the
    window, oldest first, kept when the e-print is TeX source with a
    top-level file (`detect_entry`), until `per_category`. Returns
    (entries, stats)."""
    def query():
        url, hits = arxiv_query(cat, date_from, date_to, pool)
        if not hits:  # the API sometimes answers an empty feed: retried like an error
            raise RuntimeError(f"empty feed for {cat} {date_from}..{date_to}")
        return url, hits
    try:
        url, hits = with_retries(query, polite, log=log)
    except Exception as e:  # noqa: BLE001 - an empty cell is recorded, not fatal
        if getattr(e, "code", None) in THROTTLED:
            raise Throttled(str(e)) from e
        log(f"{cat} {date_from[:8]}..{date_to[:8]}: no feed: {e}")
        return [], {"query": None, "error": str(e)[:200], "kept": 0, "skipped_pdf_only": 0, "skipped_other": 0}
    entries = []
    kept = skipped_pdf = skipped_other = 0
    for h in hits:
        if kept >= per_category:
            break
        if h["primary_category"] != cat or not re.search(r"v\d+$", h["id"]):
            continue
        path = os.path.join(cache, "eprints", safe_id(h["id"]))
        if os.path.isfile(path):
            data = slurp(path, "rb")
        else:
            try:
                data = with_retries(lambda: http_get(eprint_url(h["id"])), polite, log=log)
            except Exception as e:  # noqa: BLE001 - recorded, not fatal
                if getattr(e, "code", None) in THROTTLED:
                    raise Throttled(str(e)) from e
                log(f"  {h['id']}: fetch failed: {e}")
                skipped_other += 1
                continue
            with open(path + ".tmp", "wb") as f:
                f.write(data)
            os.replace(path + ".tmp", path)
        dest = os.path.join(cache, "src", "arxiv", safe_id(h["id"]))
        fmt = unpack(data, dest)
        entry = detect_entry(dest) if fmt not in ("pdf", "unknown") else None
        hint = uses_other_engine(dest, entry) if entry else False
        if not keep_src:
            shutil.rmtree(dest, ignore_errors=True)  # `fetch` unpacks it again from the verified bytes
        if entry is None:
            if fmt == "pdf":
                skipped_pdf += 1
            else:
                skipped_other += 1
            continue
        kept += 1
        entries.append({"id": h["id"], "category": cat, "title": h["title"], "published": h["published"],
                        "url": eprint_url(h["id"]), "sha256": sha256_bytes(data), "bytes": len(data),
                        "format": fmt, "entry": entry, "other_engine_hint": hint})
    stats = {"query": url, "kept": kept, "skipped_pdf_only": skipped_pdf, "skipped_other": skipped_other}
    log(f"{cat} {date_from[:8]}..{date_to[:8]}: kept {kept}, pdf-only {skipped_pdf}, other {skipped_other}")
    return entries, stats


LICENCE_NOTE = ("arXiv e-prints are the authors' copyright under the licence each chose; they are "
                "fetched into a local cache for measurement and never committed to this repository.")


def write_manifest(path, manifest):
    with open(path + ".tmp", "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=1, ensure_ascii=False)
        f.write("\n")
    os.replace(path + ".tmp", path)


def cmd_select_arxiv(args):
    os.makedirs(os.path.join(args.cache, "eprints"), exist_ok=True)
    polite = Polite(args.delay)
    entries, stats = [], {}

    def log(m):
        print(m, file=sys.stderr)
    for cat in args.categories:
        got, st = draw_category(cat, args.date_from, args.date_to, args.per_category, args.pool,
                                args.cache, polite, log)
        entries += got
        stats[cat] = st
    manifest = {
        "schema": "flashtex-parity-corpus/1",
        "tier": "arxiv",
        "licence_note": LICENCE_NOTE,
        "selection": {"method": "tools/parity/corpus.py select-arxiv", "categories": args.categories,
                      "submitted_from": args.date_from, "submitted_to": args.date_to,
                      "per_category": args.per_category, "pool": args.pool,
                      "selected_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "per_category_stats": stats},
        "entries": entries,
    }
    write_manifest(args.out, manifest)
    print(f"wrote {args.out}: {len(entries)} entries", file=sys.stderr)
    return 0


def grid_window(seed, cat, year, days):
    """The submission window of one (category, year) cell: `days` days that
    start on the day of `year` numbered sha256("<seed>/<cat>/<year>") mod
    (days in the year - `days`), so the draw is spread over each year and
    anyone can recompute it."""
    span = (datetime.date(year, 12, 31) - datetime.date(year, 1, 1)).days + 1 - days
    off = int(hashlib.sha256(f"{seed}/{cat}/{year}".encode()).hexdigest(), 16) % span
    start = datetime.date(year, 1, 1) + datetime.timedelta(days=off)
    end = start + datetime.timedelta(days=days - 1)
    return start.strftime("%Y%m%d") + "0000", end.strftime("%Y%m%d") + "2359"


GRID_RULE = ("for every (primary category, year) cell: a window of window_days days starting on day "
             "sha256('<seed>/<category>/<year>') mod (days in the year - window_days) of that year "
             "(corpus.grid_window); the export API's papers with that primary category submitted in the "
             "window, oldest first (at most pool); each e-print is kept when it is TeX source with a "
             "top-level .tex file that has \\documentclass and \\begin{document} outside comments "
             "(corpus.detect_entry), until per_cell are kept. PDF-only and unreadable e-prints are "
             "skipped and counted per cell. Every entry is pinned by its arXiv identifier with version "
             "and the SHA-256 of the e-print bytes; `fetch` refuses a changed hash.")


def grid_manifest(args, years, cells, entries):
    return {
        "schema": "flashtex-parity-corpus/1",
        "tier": args.tier,
        "on_demand": True,
        "licence_note": LICENCE_NOTE,
        "selection": {
            "method": "tools/parity/corpus.py select-arxiv-grid",
            "command": ("python3 tools/parity/corpus.py select-arxiv-grid --out <manifest> "
                        f"--tier {args.tier} --seed {args.seed} --years {args.year_from}-{args.year_to} "
                        f"--window-days {args.window_days} --per-cell {args.per_cell} --pool {args.pool} "
                        + " ".join(args.categories)),
            "rule": GRID_RULE,
            "seed": args.seed, "categories": args.categories, "years": years,
            "window_days": args.window_days, "per_cell": args.per_cell, "pool": args.pool,
            "selected_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "cells": cells,
        },
        "entries": entries,
    }


def cmd_select_arxiv_grid(args):
    """A large reproducible corpus: every (primary category, year) cell draws
    `--per-cell` e-prints from its own `grid_window`, exactly as
    `select-arxiv` draws one category. Resumable: the manifest is rewritten
    after every cell, and a finished cell with the same window is kept."""
    os.makedirs(os.path.join(args.cache, "eprints"), exist_ok=True)
    polite = Polite(args.delay)

    def log(m):
        print(m, file=sys.stderr, flush=True)
    years = list(range(args.year_from, args.year_to + 1))
    done, old_entries = {}, {}
    if os.path.isfile(args.out):
        with open(args.out, encoding="utf-8") as f:
            old = json.load(f)
        if old.get("selection", {}).get("seed") == args.seed:
            done = old["selection"].get("cells", {})
            for e in old["entries"]:
                old_entries.setdefault(e["cell"], []).append(e)
    cells, entries = {}, []
    for year in years:
        for cat in args.categories:
            key = f"{cat}/{year}"
            lo, hi = grid_window(args.seed, cat, year, args.window_days)
            prev = done.get(key)
            if prev and prev.get("query") and prev.get("submitted_from") == lo \
                    and prev.get("per_cell") == args.per_cell:
                cells[key] = prev
                entries += old_entries.get(key, [])
                continue
            try:
                got, st = draw_category(cat, lo, hi, args.per_cell, args.pool, args.cache, polite, log,
                                        keep_src=False)
            except Throttled as e:
                write_manifest(args.out, grid_manifest(args, years, cells, entries))
                log(f"arXiv is throttling ({e}); stopped before cell {key}. Run the same command later: "
                    "finished cells are kept and downloaded e-prints are reused.")
                return 3
            for e in got:
                e["cell"] = key
            entries += got
            cells[key] = dict(st, submitted_from=lo, submitted_to=hi, per_cell=args.per_cell)
            write_manifest(args.out, grid_manifest(args, years, cells, entries))
    write_manifest(args.out, grid_manifest(args, years, cells, entries))
    log(f"wrote {args.out}: {len(entries)} entries from {len(cells)} cells")
    return 0


def repo_tree_hash(d):
    """SHA-256 over a committed document directory's files (names and bytes)."""
    h = hashlib.sha256()
    for root, dirs, files in os.walk(d):
        dirs.sort()
        for name in sorted(files):
            p = os.path.join(root, name)
            h.update(os.path.relpath(p, d).encode() + b"\0" + slurp(p, "rb") + b"\0")
    return h.hexdigest()


def fetch_manifest(manifest_path, cache, texmf=DEFAULT_TEXMF, delay=3.0, log=print, only=None):
    """Fetch + verify + unpack every entry, or with `only` (a set of safe
    ids) just those. Returns list of document records
    {id, tier, dir, entry, source, problem}. Idempotent: cached bytes are
    re-verified, not re-downloaded."""
    with open(manifest_path, encoding="utf-8") as f:
        man = json.load(f)
    tier = man["tier"]
    docs = []
    polite = Polite(delay)
    # A fresh cache (a new runner) has no eprints/ yet; only select-arxiv
    # used to create it, so a first `fetch` died writing the first e-print.
    store = "eprints" if tier in ARXIV_TIERS else "archives"
    if tier in ARXIV_TIERS or tier in ARCHIVE_TIERS:
        os.makedirs(os.path.join(cache, store), exist_ok=True)
    for e in man["entries"]:
        doc_id = safe_id(e["id"])
        if only is not None and doc_id not in only:
            continue
        dest = os.path.join(cache, "src", tier, doc_id)
        rec = {"id": doc_id, "tier": tier, "dir": dest, "entry": e.get("entry"), "source": e.get("url") or e.get("path"),
               "category": e.get("category"), "problem": None}
        if e.get("pt1_skip"):  # why pdfTeX's own traced log is not reproducible (parity.pt1_skip_reason)
            rec["pt1_skip"] = e["pt1_skip"]
        if tier in ARXIV_TIERS or tier in ARCHIVE_TIERS:
            path = os.path.join(cache, store, doc_id)
            if not os.path.isfile(path):
                try:  # one request per `delay` s; a 429/503 is retried after arXiv's Retry-After
                    data = with_retries(lambda: http_get(e["url"]), polite)
                except Exception as ex:  # noqa: BLE001
                    rec["problem"] = f"fetch failed: {ex}"
                    docs.append(rec)
                    continue
                with open(path + ".tmp", "wb") as f:
                    f.write(data)
                os.replace(path + ".tmp", path)
            data = slurp(path, "rb")
            if sha256_bytes(data) != e["sha256"]:
                rec["problem"] = f"sha256 mismatch: manifest {e['sha256'][:12]}, fetched {sha256_bytes(data)[:12]}"
                docs.append(rec)
                continue
            # UNPACK_V in the marker: a tree unpacked before unpack kept the archive's times is made again
            root = e.get("root")
            marker = os.path.join(dest, ".parity-unpacked")
            want = f"{e['sha256']} {UNPACK_V}" + (f" root={root}" if root else "")
            if not (os.path.isfile(marker) and slurp(marker) == want):
                if root:
                    try:
                        unpack_root(data, dest, root)
                    except ValueError as ex:
                        rec["problem"] = str(ex)
                        docs.append(rec)
                        continue
                else:
                    unpack(data, dest)
                with open(marker, "w") as f:
                    f.write(want)
        elif tier in TEXLIVE_TIERS:
            in_repo = bool(e.get("repo"))
            src = os.path.join(REPO, e["repo"]) if in_repo else os.path.join(texmf, e["path"])
            if not os.path.isfile(src):
                rec["problem"] = f"missing in {'the repository' if in_repo else 'TeX Live'}: {src}"
                docs.append(rec)
                continue
            if not in_repo:
                got = hashlib.sha256(slurp(src, "rb")).hexdigest()
                if got != e["sha256"]:
                    rec["problem"] = f"sha256 mismatch for {e['path']}: manifest {e['sha256'][:12]}, local {got[:12]}"
                    docs.append(rec)
                    continue
            # A tree already made from this exact entry is left alone, so parity
            # runs sharing the cache never rebuild it under one another. A new one
            # is built beside it and then renamed into place. A repository entry's
            # marker also holds its directory's content hash, so an edited deck is
            # copied again.
            marker = os.path.join(dest, ".parity-copied")
            want = json.dumps(dict(e, tree=repo_tree_hash(os.path.dirname(src))) if in_repo else e, sort_keys=True)
            if not (os.path.isfile(marker) and slurp(marker) == want):
                srcdir = os.path.dirname(src)
                tmp = f"{dest}.tmp-{os.getpid()}"
                shutil.rmtree(tmp, ignore_errors=True)
                if e.get("copy_dir"):
                    # the template's own directory: its figures, .bib and \input files
                    shutil.copytree(srcdir, tmp, ignore=shutil.ignore_patterns("*.pdf") if e.get("skip_pdfs") else None)
                else:
                    os.makedirs(tmp)
                # `files` are copied in either case, so a figure survives `skip_pdfs`
                # (which keeps the directory's prebuilt sample PDFs out)
                for extra in [os.path.basename(src)] + e.get("files", []):
                    p = os.path.join(srcdir, extra)
                    if os.path.isfile(p):
                        shutil.copy2(p, os.path.join(tmp, os.path.basename(extra)))  # times kept: see tiers.GENERATED
                with open(os.path.join(tmp, ".parity-copied"), "w") as f:
                    f.write(want)
                shutil.rmtree(dest, ignore_errors=True)
                try:
                    os.rename(tmp, dest)
                except OSError:  # another run put the same tree in place first
                    shutil.rmtree(tmp, ignore_errors=True)
            rec["entry"] = os.path.basename(src)
        else:
            rec["problem"] = f"unknown tier {tier}"
        docs.append(rec)
    log(f"{os.path.relpath(manifest_path, REPO)}: {len(docs)} documents "
        f"({sum(1 for d in docs if d['problem'])} with problems)")
    return docs


def _head(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def manifest_tier(path):
    return _head(path).get("tier")


def manifest_ids(path):
    """The safe ids of a manifest's entries, in manifest order."""
    return [safe_id(e["id"]) for e in _head(path)["entries"]]


def manifests(paths=None, include_on_demand=False):
    """The committed manifests. One marked `"on_demand": true` (the nightly
    T4 corpus: ~5,000 e-prints, hours of polite fetching) is left out unless
    `include_on_demand`, so a bare `corpus.py fetch` stays the T3 tiers';
    it is fetched when named by `--manifest` or `--tier`, or by parity.py
    for its tier."""
    if paths:
        return paths
    found = sorted(os.path.join(MANIFEST_DIR, f) for f in os.listdir(MANIFEST_DIR) if f.endswith(".json"))
    return [m for m in found if include_on_demand or not _head(m).get("on_demand")]


def manifest_tiers():
    return sorted({manifest_tier(m) for m in manifests(include_on_demand=True)})


def cmd_fetch(args):
    bad = 0
    chosen = manifests(args.manifest, include_on_demand=bool(args.tier))
    if args.tier:
        chosen = [m for m in chosen if manifest_tier(m) in args.tier]
    for m in chosen:
        for d in fetch_manifest(m, args.cache, args.texmf, args.delay):
            if d["problem"]:
                bad += 1
                print(f"  {d['tier']}/{d['id']}: {d['problem']}", file=sys.stderr)
    return 1 if bad else 0


def cmd_hash_templates(args):
    """Fill in `sha256` for a templates or packages manifest from the local TeX Live."""
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
    g = sub.add_parser("select-arxiv-grid")
    g.add_argument("--out", required=True)
    g.add_argument("--tier", default="nightly-5k")
    g.add_argument("--seed", default="flashtex-nightly-5k")
    g.add_argument("--years", required=True, help="YYYY-YYYY, inclusive")
    g.add_argument("--window-days", type=int, default=14)
    g.add_argument("--per-cell", type=int, default=25)
    g.add_argument("--pool", type=int, default=300, help="API results considered per cell")
    g.add_argument("categories", nargs="+")
    f = sub.add_parser("fetch")
    f.add_argument("--manifest", action="append", default=[])
    f.add_argument("--tier", action="append", default=[], help="only the manifests of this tier (repeatable)")
    h = sub.add_parser("hash-templates")
    h.add_argument("--manifest", required=True)
    args = ap.parse_args(argv)
    if args.cmd == "select-arxiv-grid":
        args.year_from, args.year_to = (int(y) for y in args.years.split("-"))
    return {"select-arxiv": cmd_select_arxiv, "select-arxiv-grid": cmd_select_arxiv_grid, "fetch": cmd_fetch,
            "hash-templates": cmd_hash_templates}[args.cmd](args)


if __name__ == "__main__":
    sys.exit(main())
