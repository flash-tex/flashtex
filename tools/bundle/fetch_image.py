#!/usr/bin/env python3
"""Lay out the TeX Live tree of a texlive/texlive image, pinned by digest,
without a container runtime (lane BUNDLE-PUBLISH).

    fetch_image.py texlive/texlive@sha256:<index digest> DEST [--arch amd64] [--keep-layers DIR]

Fetches from Docker Hub's registry API: the image index, whose SHA-256 must
be the pinned digest; the platform's manifest, whose SHA-256 must be the one
the index names; and every layer, whose SHA-256 must be the one the
manifest names. So every byte is verified against the pinned digest, as
`docker pull` verifies it. The layers are applied in order, whiteouts
included, and only `usr/local/texlive/` is kept: DEST/2026/... is the tree
`docker cp <container>:/usr/local/texlive/. DEST/` gives (as
.github/actions/texlive-2026 does), with the same file contents. It runs the
same on a Mac and on a Linux runner, so a bundle packed from it has the same
digest on both.
"""

import argparse
import hashlib
import json
import os
import shutil
import sys
import tarfile
import urllib.request

REGISTRY = "https://registry-1.docker.io"
AUTH = "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{repo}:pull"
ACCEPT = ", ".join([
    "application/vnd.oci.image.index.v1+json",
    "application/vnd.docker.distribution.manifest.list.v2+json",
    "application/vnd.oci.image.manifest.v1+json",
    "application/vnd.docker.distribution.manifest.v2+json",
])
KEEP = "usr/local/texlive/"


def die(msg):
    print(f"fetch_image: {msg}", file=sys.stderr)
    sys.exit(1)


def get(url, token, accept=None):
    req = urllib.request.Request(url, headers={"Authorization": f"Bearer {token}", **({"Accept": accept} if accept else {})})
    return urllib.request.urlopen(req, timeout=120)


def fetch_verified(repo, token, digest, accept=None):
    with get(f"{REGISTRY}/v2/{repo}/manifests/{digest}", token, accept) as r:
        body = r.read()
    if "sha256:" + hashlib.sha256(body).hexdigest() != digest:
        die(f"{digest}: the registry sent other bytes")
    return json.loads(body)


def fetch_layer(repo, token, digest, path):
    if os.path.isfile(path) and sha256_file(path) == digest[7:]:
        return
    tmp = path + ".part"
    h = hashlib.sha256()
    with get(f"{REGISTRY}/v2/{repo}/blobs/{digest}", token) as r, open(tmp, "wb") as f:
        while True:
            b = r.read(1 << 20)
            if not b:
                break
            h.update(b)
            f.write(b)
    if "sha256:" + h.hexdigest() != digest:
        os.remove(tmp)
        die(f"layer {digest}: the registry sent other bytes")
    os.replace(tmp, path)


def sha256_file(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for b in iter(lambda: f.read(1 << 20), b""):
            h.update(b)
    return h.hexdigest()


def remove(p):
    if os.path.islink(p) or os.path.isfile(p):
        os.remove(p)
    elif os.path.isdir(p):
        shutil.rmtree(p)


def apply_layer(path, dest):
    """Extract the layer's usr/local/texlive/ part over dest (what lies
    under usr/local/texlive/ goes to dest/), honouring whiteouts."""
    n = 0
    with tarfile.open(path, "r:gz") as t:
        for m in t:
            if not m.name.startswith(KEEP):
                continue
            rel = m.name[len(KEEP):].rstrip("/")
            if not rel:
                continue
            if any(part in ("..", "") for part in rel.split("/")):
                die(f"{m.name}: not a plain path")
            d, base = os.path.split(rel)
            target_dir = os.path.join(dest, d)
            if base == ".wh..wh..opq":
                for e in os.listdir(target_dir) if os.path.isdir(target_dir) else []:
                    remove(os.path.join(target_dir, e))
                continue
            if base.startswith(".wh."):
                remove(os.path.join(target_dir, base[4:]))
                continue
            out = os.path.join(dest, rel)
            if m.isdir():
                if os.path.lexists(out) and not os.path.isdir(out):
                    remove(out)
                os.makedirs(out, exist_ok=True)
                continue
            if os.path.lexists(out):
                remove(out)
            os.makedirs(os.path.dirname(out), exist_ok=True)
            if m.issym():
                os.symlink(m.linkname, out)
            elif m.islnk():
                src = m.linkname
                if not src.startswith(KEEP):
                    die(f"{m.name}: hard link outside the tree")
                os.link(os.path.join(dest, src[len(KEEP):]), out)
            elif m.isfile():
                with t.extractfile(m) as src, open(out, "wb") as f:
                    shutil.copyfileobj(src, f, 1 << 20)
                os.chmod(out, m.mode & 0o777)
            n += 1
    return n


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("image", help="REPO@sha256:<index digest>")
    ap.add_argument("dest")
    ap.add_argument("--arch", default="amd64")
    ap.add_argument("--keep-layers", help="directory for the downloaded layers (default: removed after)")
    a = ap.parse_args()
    if "@sha256:" not in a.image:
        die(f"{a.image} is not pinned by digest")
    repo, digest = a.image.split("@", 1)
    with urllib.request.urlopen(AUTH.format(repo=repo), timeout=60) as r:
        token = json.load(r)["token"]
    index = fetch_verified(repo, token, digest, ACCEPT)
    mdigest = digest
    if "manifests" in index:
        ms = [m for m in index["manifests"]
              if m.get("platform", {}).get("os") == "linux" and m.get("platform", {}).get("architecture") == a.arch]
        if len(ms) != 1:
            die(f"{a.image}: no single linux/{a.arch} manifest")
        mdigest = ms[0]["digest"]
        manifest = fetch_verified(repo, token, mdigest, ACCEPT)
    else:
        manifest = index
    layers_dir = a.keep_layers or os.path.join(os.path.dirname(os.path.abspath(a.dest)), ".layers")
    os.makedirs(layers_dir, exist_ok=True)
    if os.path.lexists(a.dest):
        die(f"{a.dest} exists")
    os.makedirs(a.dest)
    for layer in manifest["layers"]:
        p = os.path.join(layers_dir, layer["digest"][7:] + ".tgz")
        fetch_layer(repo, token, layer["digest"], p)
        n = apply_layer(p, a.dest)
        print(f"layer {layer['digest'][7:19]} ({layer['size']} bytes): {n} TeX Live entries", flush=True)
    if not a.keep_layers:
        shutil.rmtree(layers_dir, ignore_errors=True)
    print(f"{a.image} (linux/{a.arch} {mdigest}) -> {a.dest}")


if __name__ == "__main__":
    main()
