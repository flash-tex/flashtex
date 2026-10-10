#!/usr/bin/env python3
"""The no-TeX-Live bundle: what goes in it, packing it, its release notes,
its oracle references and the no-TeX-Live gate (DESIGN.md 4.4, D12; lane
BUNDLE-PUBLISH).

Oracle and release tooling only (standard library). The bundle carries
whole, unmodified TeX Live 2026 packages (LPPL and the other free licences
of each package; DESIGN.md 3), from one pinned TeX Live tree: the
texlive/texlive image (scheme-full) pinned in bundle-publish.yml, laid out
by fetch_image.py.

    texbundle.py record  --tier T [--sample F] (--pdflatex P | --engine BIN --texlive-root R) --out DIR
    texbundle.py derive  --fls DIR... --root R [--bundle-root B] --out packages.txt
    texbundle.py tlpdb   --root R --out merged.tlpdb
    texbundle.py core    --root R --engine BIN --out core.txt
    texbundle.py pack    --root R --dist BIN --out F.ttb [--json F.json]
    texbundle.py notes   --root R --pack F.json --tag T --repo O/N > notes.md
    texbundle.py refs    --pdflatex P --out DIR [-j N]
    texbundle.py gate    --engine BIN --url U --digest D --refs DIR --out DIR [-j N]

`record` runs the oracle's pdflatex with -recorder over a corpus tier
(fixtures, arxiv, beamer, packages) and keeps each run's .fls; `derive`
maps every TeX Live file those runs read to its tlpdb package, and writes
the package list the bundle packs whole (`tl2026/packages.txt`). `core`
records, with the engine itself, what building pdflatex.fmt and compiling a
hello-world read (`tl2026/core.txt`): those files go first in the bundle,
as one range, so a first compile needs one request for them.

`pack` packs every runfile of every listed package, plus the core, from the
tree at R (TEXMFROOT) with `flashtex-dist bundle-pack`. A package the tree
lacks, or a core file it lacks, is fatal: the tree is not the pinned one.
The bundle's digest is the SHA-256 of its file list (paths and contents),
so the same tree packs to the same digest anywhere.

`refs` and `gate` are the no-TeX-Live gate (notex-gate.yml): `refs` runs
the oracle's pdflatex over the gate's documents (tl2026/gate.json) in the
pinned tree and keeps each PDF's SHA-256; `gate` compiles the same
documents with the engine, TeX Live hidden, from the bundle alone, and
compares. A difference fails unless tl2026/gaps.json lists the document
with the reason (a known gap whose lane has not landed).
"""

import argparse
import concurrent.futures
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
DATA = os.path.join(HERE, "tl2026")
# The pinned bundle: the app ships it (apps/mac/scripts/make-app.sh), the
# publish workflow refuses to publish a bundle with another digest, and the
# gate tests it.
LOCK = os.path.join(DATA, "flashtex-bundle.lock")
MANIFESTS = {
    "arxiv": "arxiv-2025-01.json",
    "beamer": "beamer.json",
    "packages": "packages-texlive-2026.json",
}
# Reproducible runs, as tools/parity and scripts/engine-parity.sh.
DATE_ENV = {"SOURCE_DATE_EPOCH": "0", "FORCE_SOURCE_DATE": "1"}
PASSES = 3
PASS_TIMEOUT = 300
HELLO = "\\documentclass{article}\n\\begin{document}\nHello, world.\n\\end{document}\n"


def die(msg):
    print(f"texbundle: {msg}", file=sys.stderr)
    sys.exit(1)


def read_list(path):
    """Non-comment, non-blank lines."""
    with open(path, encoding="utf-8") as f:
        return [l.strip() for l in f if l.strip() and not l.lstrip().startswith("#")]


def safe_name(doc_id):
    return re.sub(r"[^A-Za-z0-9._-]", "_", doc_id)


# --- documents ---------------------------------------------------------------

def documents(tier, only=None, cache=None):
    """[{id, dir, entry}] of a tier: the parity fixtures, or a corpus
    manifest's documents fetched (and verified) into the parity cache."""
    sys.path.insert(0, os.path.join(REPO, "tools", "parity"))
    if tier == "fixtures":
        import parity  # noqa: E402

        docs = parity.fixture_documents()
    elif tier in MANIFESTS:
        import corpus  # noqa: E402

        cache = cache or os.environ.get("FLASHTEX_PARITY_CACHE") or os.path.expanduser("~/.cache/flashtex-parity")
        ids = None if only is None else {corpus.safe_id(i) for i in only}
        docs = corpus.fetch_manifest(os.path.join(REPO, "tools", "parity", "corpus", MANIFESTS[tier]), cache,
                                     log=lambda *a, **k: None, only=ids)
    else:
        die(f"unknown tier {tier}")
    out = []
    for d in docs:
        if only is not None and d["id"] not in only and d["id"].split("/")[-1] not in only:
            continue
        if d.get("problem") or not d.get("entry"):
            print(f"skip {tier}/{d['id']}: {d.get('problem') or 'no entry'}", file=sys.stderr)
            continue
        out.append({"id": f"{tier}/{d['id']}" if tier != "fixtures" else d["id"], "dir": d["dir"],
                    "entry": d["entry"]})
    return out


def run_passes(argv, cwd, env, job, passes=PASSES):
    """Run until the PDF stops changing and the log asks for no rerun.
    Returns (ok, passes run, why)."""
    pdf = os.path.join(cwd, job + ".pdf")
    log = os.path.join(cwd, job + ".log")
    prev = None
    for n in range(1, passes + 1):
        try:
            r = subprocess.run(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                               stderr=subprocess.DEVNULL, timeout=PASS_TIMEOUT)
            code = r.returncode
        except subprocess.TimeoutExpired:
            return False, n, "timeout"
        if code != 0 or not os.path.isfile(pdf):
            text = open(log, errors="replace").read() if os.path.isfile(log) else ""
            m = re.search(r"^(! .*)$", text, re.M)
            return False, n, f"exit {code}" + (f": {m.group(1)[:160]}" if m else "")
        cur = sha256_file(pdf)
        text = open(log, errors="replace").read()
        if cur == prev and "Rerun to get" not in text and "Rerun LaTeX" not in text:
            return True, n, ""
        prev = cur
    return True, passes, ""


def sha256_file(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for b in iter(lambda: f.read(1 << 20), b""):
            h.update(b)
    return h.hexdigest()


def parallel(fn, items, jobs):
    with concurrent.futures.ThreadPoolExecutor(max_workers=max(1, jobs)) as ex:
        return list(ex.map(fn, items))


# --- record / derive ---------------------------------------------------------

def read_set_paths(path):
    """Absolute paths of an engine FLASHTEX_READ_SET file (`open\t<path>`,
    `lookup\t<format>\t<must>\t<name>\t<path>`)."""
    out = []
    try:
        f = open(path, errors="replace")
    except OSError:
        return out
    with f:
        for l in f:
            parts = l.rstrip("\n").split("\t")
            p = parts[1] if parts[0] == "open" and len(parts) == 2 else (
                parts[4] if parts[0] == "lookup" and len(parts) == 5 else parts[0])
            if p.startswith("/"):
                out.append(p)
    return out


def cmd_record(a):
    """pdflatex -recorder over a tier (the .fls kept), or with --engine the
    engine itself over the TeX Live at --texlive-root (its read set, written
    as .fls `INPUT` lines): the engine reads what pdfTeX reads, so it can
    record over a tree whose own binaries do not run here (the Linux image's
    tree on a Mac)."""
    only = set(read_list(a.sample)) if a.sample else None
    docs = documents(a.tier, only)
    os.makedirs(a.out, exist_ok=True)
    work = tempfile.mkdtemp(prefix="texbundle-rec-")
    env = dict(os.environ, **DATE_ENV)
    argv0 = [a.pdflatex, "-recorder"]
    if a.engine:
        if not a.texlive_root:
            die("--engine needs --texlive-root")
        os.makedirs(os.path.join(work, "bin"))
        os.symlink(os.path.abspath(a.engine), os.path.join(work, "bin", "pdftex"))
        argv0 = [os.path.join(work, "bin", "pdftex"), "-fmt=pdflatex"]
        for k in ("FLASHTEX_FORMATS", "FLASHTEX_RESOLVER", "FLASHTEX_BUNDLE_URL", "FLASHTEX_BUNDLE_DIGEST"):
            env.pop(k, None)
        env.update(FLASHTEX_TEXLIVE_BIN=texlive_bin(os.path.realpath(a.texlive_root)),
                   FLASHTEX_FORMAT_CACHE_DIR=os.path.join(work, "fmt"))
        if a.pool:
            env["FLASHTEX_POOL"] = os.path.abspath(a.pool)

    def one(d):
        w = os.path.join(work, safe_name(d["id"]))
        shutil.copytree(d["dir"], w, symlinks=True)
        job = os.path.splitext(os.path.basename(d["entry"]))[0]
        cwd = os.path.join(w, os.path.dirname(d["entry"]))
        e = dict(env)
        reads = os.path.join(w, ".texbundle-reads")
        if a.engine:
            e["FLASHTEX_READ_SET"] = reads
        argv = [*argv0, "-interaction=nonstopmode", os.path.basename(d["entry"])]
        ok, n, why = run_passes(argv, cwd, e, job, passes=2)
        fls = os.path.join(cwd, job + ".fls")
        dest = os.path.join(a.out, safe_name(d["id"]) + ".fls")
        if a.engine:
            paths = read_set_paths(reads)
            have = bool(paths)
            if have:
                with open(dest, "w") as f:
                    f.writelines(f"INPUT {p}\n" for p in sorted(set(paths)))
        else:
            have = os.path.isfile(fls)
            if have:
                shutil.copy(fls, dest)
        shutil.rmtree(w, ignore_errors=True)
        line = f"{d['id']}: {'ok' if ok else 'FAILED ' + why} ({n} passes){'' if have else ', nothing recorded'}"
        print(line, flush=True)
        return ok

    if a.engine and docs:
        res = [one(docs[0])] + parallel(one, docs[1:], a.jobs)  # the first alone: it builds the format
    else:
        res = parallel(one, docs, a.jobs)
    shutil.rmtree(work, ignore_errors=True)
    print(f"recorded {len(docs)} documents of {a.tier}: {sum(res)} compiled")


def parse_tlpdb(text):
    """{package: {"runfiles": [...], "fields": {key: [values]}}} of a tlpdb
    or tlpobj text, file paths relative to TEXMFROOT (`RELOC/` is
    texmf-dist/, as a relocatable package installs)."""
    out = {}
    cur = None
    section = None
    for l in text.split("\n"):
        if l.startswith("name "):
            cur = {"runfiles": [], "fields": {}}
            out[l[5:].strip()] = cur
            section = None
        elif cur is None:
            continue
        elif l.startswith(" "):
            if section == "runfiles":
                f = l[1:].split(" ")[0]
                if f.startswith("RELOC/"):
                    f = "texmf-dist/" + f[len("RELOC/"):]
                cur["runfiles"].append(f)
        elif l.strip():
            key, _, val = l.partition(" ")
            section = key
            cur["fields"].setdefault(key, []).append(val)
    return out


def merged_tlpdb_text(root):
    """The installation's texlive.tlpdb, plus every tlpkg/tlpobj/*.tlpobj
    of a package it does not list (the pinned extras, which
    .github/actions/texlive-2026 unpacks without tlmgr), relocated."""
    with open(os.path.join(root, "tlpkg", "texlive.tlpdb"), encoding="utf-8", errors="replace") as f:
        text = f.read()
    names = set(re.findall(r"^name (\S+)", text, re.M))
    extra = []
    d = os.path.join(root, "tlpkg", "tlpobj")
    for fn in sorted(os.listdir(d)) if os.path.isdir(d) else []:
        if not fn.endswith(".tlpobj"):
            continue
        with open(os.path.join(d, fn), encoding="utf-8", errors="replace") as f:
            t = f.read()
        m = re.match(r"name (\S+)", t)
        if not m or m.group(1) in names:
            continue
        t = re.sub(r"^ RELOC/", " texmf-dist/", t, flags=re.M)
        extra.append(t.rstrip("\n") + "\n")
    return text.rstrip("\n") + "\n\n" + "\n".join(extra)


def cmd_tlpdb(a):
    with open(a.out, "w", encoding="utf-8") as f:
        f.write(merged_tlpdb_text(a.root))


def platform_package(name):
    """Binaries of one platform (`pdftex.x86_64-linux`): never bundled."""
    return "." in name and not name.startswith("texlive.") and name.split(".")[-1] not in ("infra",)


def cmd_derive(a):
    roots = [os.path.realpath(r) for r in a.root]
    # Files are named by the trees the runs read (`--root`, any number) and
    # owned by the packages of the tree the bundle is packed from
    # (`--bundle-root`), where a file may since have moved to another package.
    db = parse_tlpdb(merged_tlpdb_text(os.path.realpath(a.bundle_root or a.root[0])))
    owner = {}
    for p, e in db.items():
        for f in e["runfiles"]:
            owner.setdefault(f, p)
    pkgs, unowned = set(), {}
    for d in a.fls:
        for fn in sorted(os.listdir(d)):
            if not fn.endswith(".fls"):
                continue
            with open(os.path.join(d, fn), errors="replace") as f:
                for l in f:
                    if not l.startswith("INPUT "):
                        continue
                    p = os.path.realpath(l[6:].strip())
                    root = next((r for r in roots if p.startswith(r + "/")), None)
                    if root is None:
                        continue
                    rel = p[len(root) + 1:]
                    pk = owner.get(rel)
                    if pk is None:
                        unowned.setdefault(rel, set()).add(fn)
                        continue
                    if platform_package(pk):
                        continue
                    pkgs.add(pk)
    with open(a.out, "w", encoding="utf-8") as f:
        f.write("# TeX Live 2026 packages the no-TeX-Live bundle carries whole (tools/bundle/texbundle.py).\n"
                "# Derived by `texbundle.py derive`: every package (of the bundle's tree) that owns a file\n"
                "# read by the parity fixtures, the arXiv corpus (arxiv-2025-01), the beamer corpus or\n"
                "# packages-2026, recorded by the engine over the bundle's tree and by MacTeX's pdflatex\n"
                "# -recorder. One name per line.\n")
        for p in sorted(pkgs):
            f.write(p + "\n")
    print(f"{len(pkgs)} packages -> {a.out}")
    for rel in sorted(unowned):
        print(f"read, in no package: {rel} ({len(unowned[rel])} runs)")


# --- core / pack ----------------------------------------------------------------

def texlive_bin(root):
    b = os.path.join(root, "bin")
    arches = sorted(os.listdir(b)) if os.path.isdir(b) else []
    if not arches:
        die(f"no {b}/<arch>")
    return os.path.join(b, arches[0])


def cmd_core(a):
    """What building pdflatex.fmt and compiling a hello-world read, by the
    engine over the tree (its FLASHTEX_READ_SET and the format cache's
    manifest), relative to TEXMFROOT."""
    root = os.path.realpath(a.root)
    work = tempfile.mkdtemp(prefix="texbundle-core-")
    try:
        doc = os.path.join(work, "doc")
        os.makedirs(doc)
        with open(os.path.join(doc, "hello.tex"), "w") as f:
            f.write(HELLO)
        bindir = os.path.join(work, "bin")
        os.makedirs(bindir)
        os.symlink(os.path.abspath(a.engine), os.path.join(bindir, "pdftex"))
        reads = os.path.join(work, "reads.txt")
        env = dict(os.environ, **DATE_ENV)
        for k in ("FLASHTEX_FORMATS", "FLASHTEX_RESOLVER", "FLASHTEX_BUNDLE_URL", "FLASHTEX_BUNDLE_DIGEST"):
            env.pop(k, None)
        env.update(FLASHTEX_TEXLIVE_BIN=texlive_bin(root), FLASHTEX_FORMAT_CACHE_DIR=os.path.join(work, "fmt"),
                   FLASHTEX_READ_SET=reads)
        if a.pool:
            env["FLASHTEX_POOL"] = os.path.abspath(a.pool)
        r = subprocess.run([os.path.join(bindir, "pdftex"), "-fmt=pdflatex", "-interaction=nonstopmode", "hello.tex"],
                           cwd=doc, env=env, stdin=subprocess.DEVNULL, capture_output=True, text=True)
        if r.returncode != 0 or not os.path.isfile(os.path.join(doc, "hello.pdf")):
            die(f"the engine did not compile hello.tex over {root}: {r.stdout[-2000:]}{r.stderr[-2000:]}")
        paths = set()
        with open(reads, errors="replace") as f:
            for l in f:
                parts = l.rstrip("\n").split("\t")
                p = parts[1] if parts[0] == "open" and len(parts) == 2 else (
                    parts[4] if parts[0] == "lookup" and len(parts) == 5 else parts[0])
                paths.add(p)
        for dp, _, fns in os.walk(os.path.join(work, "fmt")):
            if "manifest" in fns:
                with open(os.path.join(dp, "manifest"), errors="replace") as f:
                    for l in f:
                        if l.startswith("file\t"):
                            parts = l.rstrip("\n").split("\t", 7)
                            if len(parts) == 8:
                                paths.add(parts[7])
        rels = set()
        for p in paths:
            if not p.startswith("/"):
                continue
            rp = os.path.realpath(p)
            if rp.startswith(root + "/") and os.path.isfile(rp):
                rels.add(rp[len(root) + 1:])
        with open(a.out, "w", encoding="utf-8") as f:
            f.write("# The bundle's core range (tools/bundle/texbundle.py core): what the engine reads\n"
                    "# building pdflatex.fmt and compiling a hello-world, relative to TEXMFROOT.\n")
            for rel in sorted(rels):
                f.write(rel + "\n")
        print(f"{len(rels)} core files -> {a.out}")
    finally:
        shutil.rmtree(work, ignore_errors=True)


# --- licences ---------------------------------------------------------------------

# TeX Live's catalogue-license values that do not allow redistribution (or
# say nothing about it). TeX Live itself ships only free software, so these
# should never occur; the bundle refuses them rather than trusting that.
NONFREE = {"nosell", "nodistrib", "noinfo", "nonfree", "other-nonfree", "nosource", "shareware", "unknown"}
# Non-commercial Creative Commons terms are "nosell" in all but name.
NONFREE_PREFIXES = ("cc-by-nc",)
# Packages TeX Live records no catalogue-license for, allowed by name, each
# with the reason it is free (checked by hand against the files, 2026-10-04).
NO_LICENCE_RECORDED = {
    "hyphen-*": "hyphenation patterns of the hyph-utf8 project, which has no CTAN catalogue entry per "
                "language; every pattern file states its own free licence (LPPL, MIT or similar) in "
                "its header, and TeX Live distributes them as free software",
    "latexconfig": "TeX Live's own configuration files for the LaTeX formats (part of TeX Live's "
                   "infrastructure, under TeX Live's licensing, LICENSE.TL)",
}


def latex_format(root):
    """{fmtversion, patch_level} of the tree's latex.ltx (the LaTeX release
    the engine's format is built from)."""
    with open(os.path.join(root, "texmf-dist/tex/latex/base/latex.ltx"), encoding="latin-1") as f:
        text = f.read()
    v = re.search(r"\\edef\\fmtversion\s*\{([^}]*)\}", text) or re.search(r"\\def\\fmtversion\{([^}]*)\}", text)
    pl = re.search(r"\\def\\patch@level\{([^}]*)\}", text)
    if not v:
        die("no \\fmtversion in latex.ltx")
    return {"fmtversion": v.group(1).strip(), "patch_level": pl.group(1).strip() if pl else "?"}


def licence_of(db, p):
    return (db[p]["fields"].get("catalogue-license") or [""])[0].strip()


def no_licence_reason(p):
    for pat, why in NO_LICENCE_RECORDED.items():
        if p == pat or (pat.endswith("*") and p.startswith(pat[:-1])):
            return why
    return None


def licence_problems(db, packages):
    """Why each package may not go into the bundle (empty: all may)."""
    out = []
    for p in packages:
        lic = licence_of(db, p)
        toks = lic.split()
        if not toks:
            if no_licence_reason(p) is None:
                out.append(f"{p}: no licence recorded in the tlpdb, and not in NO_LICENCE_RECORDED")
            continue
        for t in toks:
            if t in NONFREE or t.startswith(NONFREE_PREFIXES):
                out.append(f"{p}: licence {lic!r} ({t} is not free to redistribute)")
                break
    return out


def bundle_packages(packages_txt, extra_txt):
    """The packages the bundle packs whole: packages.txt (derived from the
    engine's reads) and extra.txt (hand-picked: what the in-process tools
    and documents outside the corpora read), each once, in sorted order."""
    extra = read_list(extra_txt) if extra_txt and os.path.isfile(extra_txt) else []
    return sorted(set(read_list(packages_txt)) | set(extra))


def cmd_pack(a):
    root = os.path.realpath(a.root)
    packages = bundle_packages(a.packages, a.extra)
    core = read_list(a.core)
    work = tempfile.mkdtemp(prefix="texbundle-pack-")
    try:
        tlpdb_path = os.path.join(work, "merged.tlpdb")
        text = merged_tlpdb_text(root)
        with open(tlpdb_path, "w", encoding="utf-8") as f:
            f.write(text)
        db = parse_tlpdb(text)
        owner = {f: p for p, e in db.items() for f in e["runfiles"]}
        missing = [p for p in packages if p not in db]
        if missing:
            die(f"{len(missing)} listed packages are not in the tree's tlpdb (is it the pinned tree?): "
                f"{' '.join(missing)}")
        absent_core = [c for c in core if not os.path.isfile(os.path.join(root, c))]
        if absent_core:
            die(f"core files missing from {root}: {' '.join(absent_core[:20])}")
        bundled = sorted(set(packages) | {owner[c] for c in core if c in owner})
        bad = licence_problems(db, bundled)
        if bad:
            die("refusing to pack packages that are not free to redistribute:\n  " + "\n  ".join(bad))
        reads = os.path.join(work, "packages.lst")
        n = 0
        with open(reads, "w") as f:
            for p in packages:
                for rel in db[p]["runfiles"]:
                    f.write(os.path.join(root, rel) + "\n")
                    n += 1
        core_lst = os.path.join(work, "core.lst")
        with open(core_lst, "w") as f:
            for c in core:
                f.write(os.path.join(root, c) + "\n")
        env = dict(os.environ, FLASHTEX_TEXLIVE_BIN=texlive_bin(root))
        r = subprocess.run([a.dist, "bundle-pack", "--out", a.out, "--read", reads, "--read", core_lst,
                            "--core-read", core_lst, "--tlpdb", tlpdb_path, "--whole-packages"],
                           env=env, capture_output=True, text=True)
        sys.stdout.write(r.stdout)
        sys.stderr.write(r.stderr)
        if r.returncode != 0:
            die("bundle-pack failed")
        digest = re.search(r"^digest ([0-9a-f]{64})$", r.stdout, re.M).group(1)
        m = re.search(r"^(\d+) files .*?(\d+) packages, \d+ core \((\d+) bytes\)", r.stdout, re.M)
        ix = re.search(r"^size (\d+) bytes, index (\d+) bytes gzipped", r.stdout, re.M)
        info = {
            "digest": digest,
            "size": int(ix.group(1)),
            "index_gzip": int(ix.group(2)),
            "files": int(m.group(1)),
            "ranges": int(m.group(2)),
            "core_bytes": int(m.group(3)),
            "packages": packages,
            # Every package the bundle holds whole: the listed ones and the
            # owners of core files (bundle-pack's --whole-packages takes in
            # the package of every file it is given). Files of no package
            # (texmf-var: what TeX Live's installer generates) are not one.
            "bundle_packages": sorted(set(packages) | {owner[c] for c in core if c in owner}),
            "generated": sorted(c for c in core if c not in owner),
            "sha256": sha256_file(a.out),
        }
        if a.json:
            with open(a.json, "w") as f:
                json.dump(info, f, indent=1, sort_keys=True)
        print(f"packed {len(packages)} whole packages ({n} runfiles listed) + {len(core)} core files: "
              f"{info['size']} bytes, digest {digest}, file sha256 {info['sha256']}")
    finally:
        shutil.rmtree(work, ignore_errors=True)


# --- lock --------------------------------------------------------------------------

def read_lock(path):
    """(url, digest) of a flashtex-bundle.lock, the subset this repository
    writes: `key = "value"` lines and comments (the full grammar is
    bundle::parse_lock's, docs/contracts/bundle-lock-vectors.json)."""
    kv = {}
    with open(path, encoding="utf-8") as f:
        for l in f:
            l = l.strip()
            if not l or l.startswith("#"):
                continue
            m = re.match(r'^(\w+)\s*=\s*"([^"]*)"\s*(#.*)?$', l)
            if not m:
                die(f"{path}: cannot read line {l!r}")
            kv[m.group(1)] = m.group(2)
    if "url" not in kv or not re.fullmatch(r"[0-9a-f]{64}", kv.get("digest", "")):
        die(f"{path}: needs url and a 64-hex digest")
    return kv["url"], kv["digest"]


def release_of(url):
    """(owner/repo, tag, asset) of a GitHub release download URL."""
    m = re.fullmatch(r"https://github\.com/([^/]+/[^/]+)/releases/download/([^/]+)/([^/]+)", url)
    if not m:
        die(f"{url} is not a GitHub release asset URL")
    return m.group(1), m.group(2), m.group(3)


def cmd_lock(a):
    """Print what the workflows need from the lock (for $GITHUB_OUTPUT)."""
    url, digest = read_lock(a.lock)
    repo, tag, asset = release_of(url)
    print(f"url={url}\ndigest={digest}\nrepo={repo}\ntag={tag}\nasset={asset}")


# --- release notes ------------------------------------------------------------------

TLNET = "https://mirrors.ctan.org/systems/texlive/tlnet/archive"
HISTORIC = "https://ftp.math.utah.edu/pub/tex/historic/systems/texlive/2026"


def cmd_notes(a):
    root = os.path.realpath(a.root)
    db = parse_tlpdb(merged_tlpdb_text(root))
    with open(a.pack) as f:
        info = json.load(f)
    url, digest = read_lock(a.lock)
    _, tag, asset = release_of(url)
    if info["digest"] != digest:
        die(f"packed digest {info['digest']} is not the lock's {digest}")
    bad = licence_problems(db, info.get("bundle_packages") or info["packages"])
    if bad:
        die("packages not free to redistribute:\n  " + "\n  ".join(bad))
    fmt = latex_format(root)
    rows = []
    by_licence = {}
    packages = info.get("bundle_packages") or info["packages"]
    for p in packages:
        fld = db[p]["fields"]
        lic = licence_of(db, p) or "(none recorded in the tlpdb; see below)"
        rev = (fld.get("revision") or ["?"])[0]
        by_licence.setdefault(lic, []).append(p)
        row = {
            "package": p,
            "revision": rev,
            "licence": lic,
            "catalogue": (fld.get("catalogue") or [p])[0],
            "runfiles_container_sha512": (fld.get("containerchecksum") or [""])[0],
            "doc_container": f"{TLNET}/{p}.doc.tar.xz" if fld.get("doccontainersize") else "",
            "doc_container_sha512": (fld.get("doccontainerchecksum") or [""])[0],
            "source_container": f"{TLNET}/{p}.source.tar.xz" if fld.get("srccontainersize") else "",
            "source_container_sha512": (fld.get("srccontainerchecksum") or [""])[0],
        }
        rows.append(row)
    with open(a.licences, "w", encoding="utf-8") as f:
        cols = list(rows[0])
        f.write("\t".join(cols) + "\n")
        for r in rows:
            f.write("\t".join(r[c] for c in cols) + "\n")
    latest = max((int(e["fields"]["revision"][0]) for e in db.values()
                  if e["fields"].get("revision", [""])[0].isdigit()), default=0)
    mb = info["size"] / 1e6
    out = []
    w = out.append
    w(f"FlashTeX's no-TeX-Live bundle: {len(packages)} whole, unmodified TeX Live 2026 packages "
      f"({info['files']} files, {mb:.1f} MB), for the new engine on a Mac without TeX Live "
      f"(DESIGN.md 4.4, D12). The app fetches files from it by byte range, only after the user agrees.")
    w("")
    w("```")
    w("# flashtex-bundle.lock")
    w(f'url = "{url}"')
    w(f'digest = "{digest}"')
    w("```")
    w("")
    w("## Licensing")
    w("")
    w("Every file in `" + asset + "` is a file of TeX Live 2026, **unmodified**, and is distributed under "
      "the licence of the TeX Live package it belongs to (most are under the LaTeX Project Public "
      "Licence; the table below gives each package's licence as TeX Live's package database records it). "
      "The packages are carried whole: every runfile of each package, so a package travels with its "
      "licence statement. Nothing is patched, so LPPL clause 6 (distributing a modified work) never "
      "applies. FlashTeX's own licences (MIT for the app, GPL-2.0-or-later for the engine) do not cover "
      "these files, and the bundle does not change their terms. TeX Live's own licensing statement is "
      "`LICENSE.TL`, and CTAN's `LICENSE.CTAN`, both attached.")
    w("")
    w("`packages.tsv` (attached) lists, per package: its TeX Live revision, its licence, its CTAN "
      "catalogue entry, the SHA-512 of its runfiles container, and where its documentation and source "
      "containers are, with their SHA-512 checksums.")
    w("")
    w("**Sources.** Each package's source and documentation are in TeX Live's distribution: the "
      f"`<package>.source.tar.xz` and `<package>.doc.tar.xz` containers ({TLNET}/; after TeX Live 2026 "
      f"is frozen, {HISTORIC}/), TeX Live's Subversion repository (https://tug.org/svn/texlive/, "
      "`Master/texmf-dist`, at the revisions listed), and CTAN (https://ctan.org/pkg/<catalogue>).")
    w("")
    w("| Licence | Packages |")
    w("|---|---|")
    for lic in sorted(by_licence, key=lambda l: (-len(by_licence[l]), l)):
        names = by_licence[lic]
        w(f"| `{lic}` ({len(names)}) | {', '.join(sorted(names))} |")
    w("")
    gpl_nosrc = [p for p in packages
                 if any(t.startswith(("gpl", "lgpl", "agpl")) for t in licence_of(db, p).split())
                 and not db[p]["fields"].get("srccontainersize")]
    if gpl_nosrc:
        w("**GPL source: written offer.** For most GPL-licensed packages TeX Live has a source "
          "container (above). For these it has none, because the files in the bundle are themselves "
          "the form TeX Live distributes for modification (TeX macro files, font metrics and Type 1 "
          "fonts): " + ", ".join(f"`{p}`" for p in gpl_nosrc) + ". For at least three years from "
          "this release, and for as long as it is offered, the FlashTeX project will give anyone who "
          "asks the complete corresponding source of any GPL-licensed file in this bundle, at no "
          "charge beyond the cost of providing it. Ask in an issue at "
          "https://github.com/flash-tex/flashtex/issues, naming this release.")
        w("")
    unrecorded = sorted({no_licence_reason(p) for p in packages if not licence_of(db, p)})
    if unrecorded:
        w("TeX Live records no licence for some packages; each is allowed by name "
          "(`NO_LICENCE_RECORDED` in tools/bundle/texbundle.py), because:")
        w("")
        for pat, why in NO_LICENCE_RECORDED.items():
            if why in unrecorded:
                w(f"- `{pat}`: {why}.")
        w("")
        w("The packer refuses any package whose recorded licence is `nosell`, `nodistrib`, `noinfo`, "
          "`nonfree`, `other-nonfree`, `nosource`, `shareware`, `unknown` or non-commercial, and any package with no licence recorded that is "
          "not allowed by name.")
        w("")
    if info.get("generated"):
        w("Besides the packages, the bundle carries the files TeX Live's installer generates from them "
          "(`" + "`, `".join(info["generated"]) + "`), as TeX Live installs them.")
        w("")
    w("## Provenance")
    w("")
    w(f"- TeX Live tree: `{a.image}` (texlive/texlive, scheme-full, linux/amd64), laid out by "
      f"`tools/bundle/fetch_image.py`; its newest package revision is {latest}.")
    w(f"- LaTeX format: `latex.ltx` of {fmt['fmtversion']}, patch level {fmt['patch_level']} "
      "(the format the engine builds from this bundle).")
    w(f"- Packed by `tools/bundle/texbundle.py pack` (commit {a.commit}): the package list "
      "`tools/bundle/tl2026/packages.txt` (every package whose files the parity fixtures, the arXiv "
      "corpus, the beamer corpus and packages-2026 read), the hand-picked "
      "`tools/bundle/tl2026/extra.txt` (bibtex's and makeindex's styles and other files the tools "
      "FlashTeX runs read, common bibliography styles) and the core range "
      "`tools/bundle/tl2026/core.txt`.")
    w(f"- Digest (SHA-256 of the bundle's file list): `{digest}`; file SHA-256: `{info['sha256']}`; "
      f"index {info['index_gzip']} bytes gzipped; core range {info['core_bytes']} bytes.")
    w("- Packed twice in the same job: byte-identical.")
    w("")
    w("`oracle-refs.json` holds the SHA-256 of the PDF TeX Live's own pdflatex makes from each gate "
      "document in the same tree (notex-gate.yml compares the engine against it); no PDF of a "
      "third-party document is published.")
    with open(a.out, "w", encoding="utf-8") as f:
        f.write("\n".join(out) + "\n")
    print(f"notes -> {a.out} ({len(''.join(out))} characters); {len(rows)} packages -> {a.licences}")


# --- oracle references and the gate ------------------------------------------------

def gate_documents(spec_path):
    with open(spec_path) as f:
        spec = json.load(f)
    docs = []
    for tier, ids in spec["tiers"].items():
        docs += documents(tier, None if ids is None else set(ids))
    return docs


def pages_of(log):
    try:
        text = open(log, errors="replace").read()
    except OSError:
        return 0
    m = re.search(r"Output written on .*?\((\d+) pages?", text, re.S)
    return int(m.group(1)) if m else 0


def compile_doc(d, work, argv0, env, wrap=()):
    """Copy the document, run passes; {exit, passes, sha256, pages, why}."""
    w = os.path.join(work, safe_name(d["id"]))
    shutil.rmtree(w, ignore_errors=True)
    shutil.copytree(d["dir"], w, symlinks=True)
    cwd = os.path.join(w, os.path.dirname(d["entry"]))
    job = os.path.splitext(os.path.basename(d["entry"]))[0]
    for stale in (job + ".pdf",):
        try:
            os.remove(os.path.join(cwd, stale))
        except OSError:
            pass
    argv = [*wrap, *argv0, "-interaction=nonstopmode", os.path.basename(d["entry"])]
    pdf = os.path.join(cwd, job + ".pdf")
    log = os.path.join(cwd, job + ".log")
    prev, code, n, why = None, None, 0, ""
    for n in range(1, PASSES + 1):
        try:
            code = subprocess.run(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                  stderr=subprocess.DEVNULL, timeout=PASS_TIMEOUT).returncode
        except subprocess.TimeoutExpired:
            code, why = None, "timeout"
            break
        if not os.path.isfile(pdf):
            break
        cur = sha256_file(pdf)
        text = open(log, errors="replace").read() if os.path.isfile(log) else ""
        if cur == prev and "Rerun to get" not in text and "Rerun LaTeX" not in text:
            break
        prev = cur
    res = {"exit": code, "passes": n, "sha256": sha256_file(pdf) if os.path.isfile(pdf) else None,
           "pages": pages_of(log)}
    if res["sha256"] is None or code != 0:
        text = open(log, errors="replace").read() if os.path.isfile(log) else ""
        m = re.search(r"^(! .*)$", text, re.M)
        why = why or (m.group(1)[:200] if m else "")
    if why:
        res["why"] = why
    shutil.rmtree(w, ignore_errors=True)
    return res


def cmd_refs(a):
    docs = gate_documents(a.spec)
    work = tempfile.mkdtemp(prefix="texbundle-refs-")
    env = dict(os.environ, **DATE_ENV)
    try:
        ver = subprocess.run([a.pdflatex, "--version"], capture_output=True, text=True).stdout.split("\n")[0]
        res = dict(zip([d["id"] for d in docs],
                       parallel(lambda d: compile_doc(d, work, [a.pdflatex], env), docs, a.jobs)))
    finally:
        shutil.rmtree(work, ignore_errors=True)
    out = {"about": "SHA-256 of the PDF TeX Live's pdflatex makes from each notex-gate document "
                    "(tools/bundle/texbundle.py refs; SOURCE_DATE_EPOCH=0, FORCE_SOURCE_DATE=1, "
                    "-interaction=nonstopmode, passes until the PDF is stable)",
           "pdflatex": ver, "image": a.image, "documents": res}
    if a.root:
        out["latex_format"] = latex_format(os.path.realpath(a.root))
    with open(a.out, "w") as f:
        json.dump(out, f, indent=1, sort_keys=True)
    ok = sum(1 for r in res.values() if r["sha256"])
    print(f"{len(res)} oracle references ({ok} with a PDF) -> {a.out}")


SANDBOX = """(version 1)
(allow default)
(deny file-read* file-write* {paths})
"""
TEXLIVE_PATHS = ["/usr/local/texlive", "/Library/TeX", "/opt/homebrew/opt/texlive", "/opt/homebrew/Cellar/texlive",
                 "/usr/local/opt/texlive", "/usr/local/Cellar/texlive", "/opt/texlive", "/usr/share/texlive",
                 "/usr/share/texmf"]


def no_texlive_wrap(work):
    """`sandbox-exec` denying every TeX Live location (macOS), or nothing
    elsewhere; and the gate refuses to run where a TeX Live is visible."""
    for exe in ("pdflatex", "pdftex", "kpsewhich"):
        if shutil.which(exe):
            die(f"{shutil.which(exe)} is on PATH: the no-TeX-Live gate needs a machine without TeX Live")
    if sys.platform != "darwin" or not os.path.exists("/usr/bin/sandbox-exec"):
        return ()
    prof = os.path.join(work, "notex.sb")
    with open(prof, "w") as f:
        f.write(SANDBOX.format(paths=" ".join(f'(subpath "{p}")' for p in TEXLIVE_PATHS)))
    probe = subprocess.run(["/usr/bin/sandbox-exec", "-f", prof, "/usr/bin/true"])
    if probe.returncode != 0:
        die("sandbox-exec does not run here")
    return ("/usr/bin/sandbox-exec", "-f", prof)


def cmd_gate(a):
    with open(a.refs) as f:
        refs = json.load(f)["documents"]
    with open(a.gaps) as f:
        gaps = json.load(f)["gaps"]
    docs = gate_documents(a.spec)
    os.makedirs(a.out, exist_ok=True)
    work = tempfile.mkdtemp(prefix="texbundle-gate-")
    try:
        wrap = no_texlive_wrap(work)
        for d in ("home", "tmp", "bin"):
            os.makedirs(os.path.join(work, d))
        exe = os.path.join(work, "bin", "pdftex")
        os.symlink(os.path.abspath(a.engine), exe)
        env = {"HOME": os.path.join(work, "home"), "PATH": "/usr/bin:/bin:/usr/sbin:/sbin",
               "TMPDIR": os.path.join(work, "tmp") + "/", "LANG": "C", **DATE_ENV,
               "FLASHTEX_RESOLVER": "bundle", "FLASHTEX_BUNDLE_URL": a.url, "FLASHTEX_BUNDLE_DIGEST": a.digest,
               "FLASHTEX_BUNDLE_CACHE_DIR": os.path.join(a.out, "bundle-cache"),
               "FLASHTEX_FORMAT_CACHE_DIR": os.path.join(a.out, "format-cache")}
        # One compile first, alone: it fetches the index and the core and
        # builds the format, which the parallel compiles then share.
        warm = {"id": "warm/hello", "dir": os.path.join(work, "hello"), "entry": "hello.tex"}
        os.makedirs(warm["dir"])
        with open(os.path.join(warm["dir"], "hello.tex"), "w") as f:
            f.write(HELLO)
        r = compile_doc(warm, work, [exe, "-fmt=pdflatex"], env, wrap)
        print(f"warm-up (index, core, format): {r}", flush=True)
        if not r["sha256"]:
            die("the engine compiled nothing from the bundle")
        results = dict(zip([d["id"] for d in docs],
                           parallel(lambda d: compile_doc(d, work, [exe, "-fmt=pdflatex"], env, wrap), docs,
                                    a.jobs)))
    finally:
        shutil.rmtree(work, ignore_errors=True)
    rows, failed, fixed = [], [], []
    for did, r in results.items():
        ref = refs.get(did)
        if ref is None:
            verdict = "no reference"
        elif not ref["sha256"]:
            verdict = "oracle failed"
        elif r["sha256"] == ref["sha256"]:
            verdict = "same"
        elif not r["sha256"]:
            verdict = "no PDF"
        else:
            verdict = "differs"
        gap = gaps.get(did)
        bad = verdict in ("differs", "no PDF", "no reference")
        if bad and not gap:
            failed.append(did)
        if gap and verdict == "same":
            fixed.append(did)
        rows.append((did, verdict, gap["kind"] if gap else "", r, ref or {}))
    with open(os.path.join(a.out, "results.json"), "w") as f:
        json.dump({did: {"verdict": v, "gap": g, "engine": r, "oracle": o} for did, v, g, r, o in rows}, f,
                  indent=1, sort_keys=True)
    same = sum(1 for r in rows if r[1] == "same")
    measured = sum(1 for r in rows if r[1] not in ("oracle failed",))
    lines = [f"# no-TeX-Live gate: bundle {a.digest[:12]}",
             "",
             f"{same}/{measured} documents byte-identical to TeX Live's pdflatex (from the bundle alone, "
             f"TeX Live hidden); {len(failed)} unexpected differences; "
             f"{sum(1 for r in rows if r[2] and r[1] != 'same')} known gaps (tools/bundle/tl2026/gaps.json); "
             f"{sum(1 for r in rows if r[1] == 'oracle failed')} the oracle does not compile.",
             "", "| document | verdict | known gap | pages (engine/oracle) | engine error |", "|---|---|---|---|---|"]
    for did, v, g, r, o in sorted(rows, key=lambda x: (x[1] == "same", x[0])):
        lines.append(f"| {did} | {v} | {g} | {r['pages']}/{o.get('pages', '')} | {r.get('why', '')[:90]} |")
    if fixed:
        lines += ["", "Listed as known gaps but now identical (remove them from gaps.json): " + ", ".join(fixed)]
    report = "\n".join(lines) + "\n"
    with open(os.path.join(a.out, "report.md"), "w") as f:
        f.write(report)
    print(report)
    if failed:
        die(f"{len(failed)} documents differ from TeX Live's pdflatex and are not known gaps: {', '.join(failed)}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("record")
    s.add_argument("--tier", required=True, choices=["fixtures", *MANIFESTS])
    s.add_argument("--sample")
    s.add_argument("--pdflatex", default=shutil.which("pdflatex") or "pdflatex")
    s.add_argument("--engine", help="record with flashtex-initex instead of pdflatex")
    s.add_argument("--texlive-root", help="with --engine: the TEXMFROOT it reads")
    s.add_argument("--pool")
    s.add_argument("--out", required=True)
    s.add_argument("-j", "--jobs", type=int, default=4)
    s = sub.add_parser("derive")
    s.add_argument("--fls", action="append", required=True)
    s.add_argument("--root", action="append", required=True, help="TEXMFROOT of a TeX Live the recorded runs read")
    s.add_argument("--bundle-root", help="TEXMFROOT of the tree the bundle is packed from (default: --root)")
    s.add_argument("--out", required=True)
    s = sub.add_parser("tlpdb")
    s.add_argument("--root", required=True)
    s.add_argument("--out", required=True)
    s = sub.add_parser("core")
    s.add_argument("--root", required=True)
    s.add_argument("--engine", required=True, help="flashtex-initex")
    s.add_argument("--pool")
    s.add_argument("--out", default=os.path.join(DATA, "core.txt"))
    s = sub.add_parser("pack")
    s.add_argument("--root", required=True)
    s.add_argument("--dist", required=True, help="flashtex-dist")
    s.add_argument("--packages", default=os.path.join(DATA, "packages.txt"))
    s.add_argument("--extra", default=os.path.join(DATA, "extra.txt"),
                   help="hand-picked packages packed besides --packages (tl2026/extra.txt)")
    s.add_argument("--core", default=os.path.join(DATA, "core.txt"))
    s.add_argument("--out", required=True)
    s.add_argument("--json")
    s = sub.add_parser("lock")
    s.add_argument("--lock", default=LOCK)
    s = sub.add_parser("notes")
    s.add_argument("--root", required=True)
    s.add_argument("--pack", required=True, help="pack --json output")
    s.add_argument("--lock", default=LOCK)
    s.add_argument("--image", required=True)
    s.add_argument("--commit", default="?")
    s.add_argument("--out", required=True)
    s.add_argument("--licences", required=True, help="packages.tsv to write")
    s = sub.add_parser("refs")
    s.add_argument("--pdflatex", required=True)
    s.add_argument("--root", help="TEXMFROOT of the tree, to record its LaTeX format date")
    s.add_argument("--spec", default=os.path.join(DATA, "gate.json"))
    s.add_argument("--image", default="")
    s.add_argument("--out", required=True)
    s.add_argument("-j", "--jobs", type=int, default=4)
    s = sub.add_parser("gate")
    s.add_argument("--engine", required=True, help="flashtex-initex")
    s.add_argument("--url", required=True)
    s.add_argument("--digest", required=True)
    s.add_argument("--refs", required=True)
    s.add_argument("--spec", default=os.path.join(DATA, "gate.json"))
    s.add_argument("--gaps", default=os.path.join(DATA, "gaps.json"))
    s.add_argument("--out", required=True)
    s.add_argument("-j", "--jobs", type=int, default=3)
    a = ap.parse_args()
    globals()["cmd_" + a.cmd](a)


if __name__ == "__main__":
    main()
