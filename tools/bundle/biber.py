#!/usr/bin/env python3
"""TeX Live's biber, mirrored for no-TeX-Live documents (#1782; the engine's
crates/flashtex-engine/src/bundle/biber.rs fetches it on demand).

biblatex's default backend is biber, a Perl program TeX Live ships packed
into one executable per platform. This takes TeX Live 2026's own containers
(`biber.<arch>.r<rev>.tar.xz` and the source container), each checked
against the SHA-512 pinned in tl2026/biber.txt (TeX Live's tlpdb
`containerchecksum`), and writes the release assets: per platform the
executable, unmodified, gzipped (`biber-<arch>.gz`, what the engine
fetches), the source container as it is, and a notice with the licence.
It checks that the biber pinned is the one the bundle's tree has (the tlpdb
of the pinned image), so biber and the bundle's biblatex agree.

    biber.py fetch --tlpdb TREE/tlpkg/texlive.tlpdb --out DIR [--mirror URL]

Standard library only. Prints the lock lines (`biber_url_<arch>`,
`biber_sha256_<arch>`, `biber_size_<arch>`) for flashtex-bundle.lock.
"""

import argparse
import gzip
import struct
import zlib
import hashlib
import io
import json
import os
import re
import sys
import tarfile
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
PINS = os.path.join(HERE, "tl2026", "biber.txt")
MIRROR = "https://mirrors.ctan.org/systems/texlive/tlnet"
REPO = "flash-tex/flashtex"


def die(m):
    print(f"biber.py: {m}", file=sys.stderr)
    sys.exit(1)


def read_pins(path):
    """{'version': ..., 'tag': ..., 'arch': {arch: (rev, sha512)}, 'source': (rev, sha512)}"""
    pins = {"arch": {}}
    for l in open(path, encoding="utf-8"):
        w = l.split("#", 1)[0].split()
        if not w:
            continue
        if w[0] in ("version", "tag"):
            pins[w[0]] = w[1]
        elif w[0] == "source":
            pins["source"] = (w[1], w[2])
        else:
            pins["arch"][w[0]] = (w[1], w[2])
    for k in ("version", "tag", "source"):
        if k not in pins:
            die(f"{path}: no {k}")
    return pins


def tlpdb_entries(text):
    out, cur = {}, None
    for l in text.splitlines():
        if l.startswith("name "):
            cur = out.setdefault(l[5:].strip(), {})
        elif cur is not None and " " in l and not l.startswith(" "):
            k, v = l.split(" ", 1)
            cur.setdefault(k, v.strip())
    return out


def gzip_member(data):
    """One gzip member, the same bytes on every host: level 9, no name, mtime
    0, OS byte 3 (Unix), as the bundle's members (bundle/gz.rs). Python's
    gzip.compress lets zlib write the OS byte of the host it runs on."""
    c = zlib.compressobj(9, zlib.DEFLATED, -15)
    body = c.compress(data) + c.flush()
    head = b"\x1f\x8b\x08\x00" + b"\x00\x00\x00\x00" + b"\x02\x03"
    tail = struct.pack("<II", zlib.crc32(data) & 0xFFFFFFFF, len(data) & 0xFFFFFFFF)
    return head + body + tail


def get(url):
    """The bytes at url, through curl; up to 5 tries, as CTAN's redirector
    sends each try to another mirror (some serve an incomplete certificate
    chain). What comes back is checked against its SHA-512 pin anyway."""
    err = ""
    for _ in range(5):
        r = subprocess.run(["curl", "-fsSL", "--retry", "2", "--max-time", "900", url], capture_output=True)
        if r.returncode == 0:
            return r.stdout
        err = r.stderr.decode(errors="replace").strip()
    die(f"{url}: {err}")


def checked(url, sha512):
    data = get(url)
    got = hashlib.sha512(data).hexdigest()
    if got != sha512:
        die(f"{url}: SHA-512 {got}, not the pinned {sha512} (TeX Live updated biber: pin the new revision)")
    return data


def cmd_fetch(a):
    pins = read_pins(a.pins)
    db = tlpdb_entries(open(a.tlpdb, encoding="utf-8").read())
    have = db.get("biber", {}).get("catalogue-version")
    if have != pins["version"]:
        die(f"the bundle's tree has biber {have}, the pins {pins['version']}: biber must match its biblatex")
    for arch, (rev, sha) in pins["arch"].items():
        e = db.get(f"biber.{arch}")
        if e and (e.get("revision") != rev or e.get("containerchecksum") != sha):
            die(f"biber.{arch}: the tree has r{e.get('revision')}, the pins r{rev}")
    os.makedirs(a.out, exist_ok=True)
    assets, lock = {}, []
    for arch, (rev, sha) in sorted(pins["arch"].items()):
        data = checked(f"{a.mirror}/archive/biber.{arch}.r{rev}.tar.xz", sha)
        exe = f"bin/{arch}/biber" + (".exe" if arch == "windows" else "")
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:xz") as t:
            m = t.getmember(exe)
            program = t.extractfile(m).read()
        gz = gzip_member(program)
        name = f"biber-{arch}.gz"
        with open(os.path.join(a.out, name), "wb") as f:
            f.write(gz)
        h = hashlib.sha256(gz).hexdigest()
        assets[name] = {"arch": arch, "revision": rev, "container_sha512": sha, "sha256": h, "size": len(gz),
                        "program_sha256": hashlib.sha256(program).hexdigest()}
        url = f"https://github.com/{REPO}/releases/download/{pins['tag']}/{name}"
        k = arch.replace("-", "_")  # lock keys are words (texbundle.py read_lock)
        lock += [f'biber_url_{k} = "{url}"', f'biber_sha256_{k} = "{h}"', f'biber_size_{k} = "{len(gz)}"']
    rev, sha = pins["source"]
    src = checked(f"{a.mirror}/archive/biber.source.r{rev}.tar.xz", sha)
    src_name = f"biber-{pins['version']}-source.tar.xz"
    with open(os.path.join(a.out, src_name), "wb") as f:
        f.write(src)
    with open(os.path.join(a.out, "biber-NOTICE.txt"), "w") as f:
        f.write(f"biber {pins['version']} as TeX Live 2026 ships it (tlnet containers, SHA-512 checked), "
                f"for FlashTeX's no-TeX-Live mode.\nEach biber-<arch>.gz is TeX Live's executable "
                f"bin/<arch>/biber, unmodified, gzipped. Its source is {src_name} (TeX Live's "
                f"source container) and https://github.com/plk/biber.\nbiber is under the Artistic "
                f"License 2.0; the executable carries Perl (Artistic or GPL) and modules under their own "
                f"free licences, listed in the source.\n")
    with open(os.path.join(a.out, "biber.json"), "w") as f:
        json.dump({"version": pins["version"], "tag": pins["tag"], "assets": assets}, f, indent=1, sort_keys=True)
    print("\n".join(lock))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("fetch")
    s.add_argument("--pins", default=PINS)
    s.add_argument("--tlpdb", required=True)
    s.add_argument("--out", required=True)
    s.add_argument("--mirror", default=MIRROR)
    a = ap.parse_args()
    {"fetch": cmd_fetch}[a.cmd](a)


if __name__ == "__main__":
    main()
