#!/usr/bin/env python3
"""The Unicode corpus of the M3 gate: assemble its cases as case directories.

    python3 tools/unicode-m3/corpus.py assemble --out DIR [--fonts-line BIN]
        [--cache DIR] [--only GLOB ...] [--platform linux|macos]
    python3 tools/unicode-m3/corpus.py list

`corpus.json` names every case (README.md). `assemble` writes each as
`DIR/NAME/NAME.tex` with the files it reads, plus `.m3case.json` (family,
source, trace mode, and for a `[fonts]` fixture the first-line code and the
fonts its XDV must and must not load), the layout
tools/xetex-lockstep/latex.py `--cases-dir` and tools/xetex-pdfparity/run.py
`--cases-dir` read. TeX Live documentation sources are fetched from tlnet's
archive into `--cache` and used only if their SHA-256 is the pinned one.

`--fonts-line BIN` is crates/flashtex-xetex's `fonts_line` example: the
`[fonts]` code FlashTeX's host generates for the fixture's flashtex.toml, so
both engines run the same effective source (PROPOSAL.md §4.5). Without it the
`[fonts]` fixtures are left out.
"""
import argparse
import fnmatch
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tarfile
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
META = ".m3case.json"


def load():
    with open(os.path.join(HERE, "corpus.json"), encoding="utf-8") as f:
        return json.load(f)


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for b in iter(lambda: f.read(1 << 20), b""):
            h.update(b)
    return h.hexdigest()


def fetch(archive, spec, cache):
    """The archive's path in `cache`, downloaded if needed, verified."""
    os.makedirs(cache, exist_ok=True)
    p = os.path.join(cache, archive + ".tar.xz")
    if not (os.path.exists(p) and sha256(p) == spec["sha256"]):
        tmp = p + ".part"
        if shutil.which("curl"):
            subprocess.run(["curl", "-fsSL", "--retry", "5", "--retry-all-errors", "-o", tmp, spec["url"]],
                           check=True)
        else:
            with urllib.request.urlopen(spec["url"], timeout=120) as r, open(tmp, "wb") as f:
                shutil.copyfileobj(r, f)
        got = sha256(tmp)
        if got != spec["sha256"]:
            os.remove(tmp)
            raise SystemExit("%s: SHA-256 %s, pinned %s: not used (re-pin it deliberately)"
                             % (spec["url"], got, spec["sha256"]))
        os.replace(tmp, p)
    return p


def extract(tar_path, root, dest):
    """Every regular file under `root/` of the archive into `dest`, keeping
    the layout below `root`; no absolute paths, no `..`, no links."""
    prefix = root.rstrip("/") + "/"
    with tarfile.open(tar_path, "r:xz") as t:
        for m in t.getmembers():
            if not m.isfile() or not m.name.startswith(prefix):
                continue
            rel = m.name[len(prefix):]
            parts = rel.split("/")
            if not rel or rel.startswith("/") or ".." in parts:
                raise SystemExit("%s: unsafe member %r" % (tar_path, m.name))
            out = os.path.join(dest, *parts)
            os.makedirs(os.path.dirname(out), exist_ok=True)
            with t.extractfile(m) as src, open(out, "wb") as f:
                shutil.copyfileobj(src, f)


def substitute(path, pairs):
    with open(path, encoding="utf-8", errors="surrogateescape") as f:
        s = f.read()
    done = []
    for a, b in pairs:
        if a in s:
            s = s.replace(a, b)
            done.append(a)
    with open(path, "w", encoding="utf-8", errors="surrogateescape") as f:
        f.write(s)
    return done


def assemble_case(c, corpus, out, cache, fonts_line):
    name = c["name"]
    d = os.path.join(out, name)
    if os.path.exists(d):
        shutil.rmtree(d)
    os.makedirs(d)
    meta = {"family": c["family"], "source": c.get("source", "FlashTeX repository"),
            "trace": c.get("trace", "full")}
    main = os.path.join(d, name + ".tex")
    if "path" in c:
        shutil.copy(os.path.join(REPO, c["path"]), main)
        meta["source_path"] = c["path"]
    elif "archive" in c:
        spec = corpus["archives"][c["archive"]]
        extract(fetch(c["archive"], spec, cache), c["root"], d)
        src = os.path.join(d, c["main"])
        if os.path.abspath(src) != os.path.abspath(main):
            os.replace(src, main)
        pairs = corpus["cjk_subst"] if c.get("subst") == "cjk" else c.get("subst", [])
        meta["substituted"] = substitute(main, pairs)
        meta["source_path"] = "%s (%s, sha256 %s)" % (
            "/".join([c["root"], c["main"]]), spec["url"], spec["sha256"][:12])
    elif "fonts" in c:
        if not fonts_line:
            shutil.rmtree(d)
            return None
        src = os.path.join(REPO, c["fonts"])
        for fn in os.listdir(src):
            if fn != "expect.json":
                shutil.copy(os.path.join(src, fn), os.path.join(d, fn))
        with open(os.path.join(src, "expect.json"), encoding="utf-8") as f:
            meta.update(json.load(f))
        r = subprocess.run([fonts_line, d], stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                           text=True, check=True)
        meta["firstline"] = r.stdout
        meta["fonts_notes"] = r.stderr.splitlines()
        meta["source_path"] = c["fonts"]
    else:
        raise SystemExit("%s: no source" % name)
    with open(os.path.join(d, META), "w", encoding="utf-8") as f:
        json.dump(meta, f, indent=1, ensure_ascii=False)
    return meta


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    a1 = sub.add_parser("assemble")
    a1.add_argument("--out", required=True)
    a1.add_argument("--cache", default=os.path.join(os.path.expanduser("~"), ".cache", "flashtex-unicode-m3"))
    a1.add_argument("--fonts-line")
    a1.add_argument("--only", nargs="*", default=["*"])
    a1.add_argument("--platform", default="macos" if sys.platform == "darwin" else "linux")
    sub.add_parser("list")
    a = ap.parse_args(argv)
    corpus = load()
    if a.cmd == "list":
        for c in corpus["cases"]:
            print("%-36s %-13s %s" % (c["name"], c["family"], c.get("path") or c.get("fonts")
                                      or "%s:%s/%s" % (c["archive"], c["root"], c["main"])))
        return 0
    os.makedirs(a.out, exist_ok=True)
    rows = {}
    for c in corpus["cases"]:
        if not any(fnmatch.fnmatch(c["name"], p) for p in a.only):
            continue
        if a.platform not in c.get("platforms", [a.platform]):
            print("skip %s (needs %s)" % (c["name"], "/".join(c["platforms"])))
            continue
        m = assemble_case(c, corpus, a.out, a.cache, a.fonts_line)
        if m is None:
            print("skip %s (no --fonts-line)" % c["name"])
            continue
        rows[c["name"]] = m
        extra = ""
        if m.get("substituted"):
            extra = " (fonts substituted: %s)" % ", ".join(m["substituted"])
        if "firstline" in m:
            extra = " (first line: %d bytes of [fonts] code)" % len(m["firstline"])
        print("case %s [%s]%s" % (c["name"], c["family"], extra))
    with open(os.path.join(a.out, "corpus-index.json"), "w", encoding="utf-8") as f:
        json.dump(rows, f, indent=1, ensure_ascii=False)
    print("%d cases in %s" % (len(rows), a.out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
