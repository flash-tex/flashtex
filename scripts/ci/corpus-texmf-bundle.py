#!/usr/bin/env python3
"""The TeX Live tiers' pinned documents as one tarball, for a host whose TeX
Live has no doc files (p5-scoreboard.yml's GitHub-hosted route).

The templates and packages manifests (tools/parity/corpus/*-texlive-2026.json)
pin files of one TeX Live 2026 tree by texmf-dist path and SHA-256. A host
without those files measures them as unmeasured (the tiers read partial); the
NixOS PC keeps a copy of exactly those files in
~/.cache/flashtex-corpus-texmf/texlive-2026 (tools/parity/README.md, "The
board's host"), and parity.py --corpus-texmf reads them from there. This writes
the same selection as a tarball:

  * each entry's file (checked against its sha256: a mismatch is an error, the
    tarball is never written from another snapshot);
  * its `files`, from the entry's directory;
  * for a `copy_dir` entry, the entry's whole directory;

at their texmf-dist paths, with their modification times (parity.py copies with
times kept: tiers.GENERATED), in name order, owner 0:0, so one tree always
gives the same bytes.

    python3 -I scripts/ci/corpus-texmf-bundle.py --texmf /usr/local/texlive/2026/texmf-dist \\
        --out corpus-texmf.tar.xz

Standard library only. Prints the tarball's sha256 last.
"""

import argparse
import hashlib
import json
import lzma
import os
import sys
import tarfile

REPO = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", ".."))
MANIFESTS = ("templates-texlive-2026.json", "packages-texlive-2026.json")


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def selection(texmf):
    """{texmf-relative path} the manifests need, after checking every entry's sha256."""
    paths, bad = set(), []
    for name in MANIFESTS:
        with open(os.path.join(REPO, "tools", "parity", "corpus", name), encoding="utf-8") as f:
            man = json.load(f)
        for e in man["entries"]:
            if e.get("repo"):
                continue  # in this repository, not in TeX Live
            src = os.path.join(texmf, e["path"])
            if not os.path.isfile(src) or sha256(src) != e["sha256"]:
                bad.append(f"{name}: {e['id']}: {e['path']}")
                continue
            d = os.path.dirname(e["path"])
            paths.add(e["path"])
            for extra in e.get("files", []):
                if os.path.isfile(os.path.join(texmf, d, extra)):
                    paths.add(os.path.normpath(os.path.join(d, extra)))
            if e.get("copy_dir"):
                for root, dirs, files in os.walk(os.path.join(texmf, d)):
                    dirs.sort()
                    for fn in files:
                        p = os.path.join(root, fn)
                        if os.path.isfile(p) and not os.path.islink(p):
                            paths.add(os.path.relpath(p, texmf))
    return sorted(paths), bad


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--texmf", required=True, help="the TeX Live tree's texmf-dist")
    ap.add_argument("--out", required=True, help="the .tar.xz to write")
    args = ap.parse_args()
    paths, bad = selection(args.texmf)
    if bad:
        print("not the manifests' files (another TeX Live snapshot?):", *bad, sep="\n  ", file=sys.stderr)
        return 1

    def norm(ti):
        ti.uid = ti.gid = 0
        ti.uname = ti.gname = ""
        ti.mode = 0o644
        return ti

    tmp = args.out + ".tmp"
    with lzma.open(tmp, "wb", preset=9) as xz, tarfile.open(fileobj=xz, mode="w", format=tarfile.PAX_FORMAT) as tar:
        for p in paths:
            tar.add(os.path.join(args.texmf, p), arcname=p, recursive=False, filter=norm)
    os.replace(tmp, args.out)
    print(f"{len(paths)} files, {os.path.getsize(args.out)} bytes")
    print(sha256(args.out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
