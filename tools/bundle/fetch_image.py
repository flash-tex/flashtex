#!/usr/bin/env python3
"""Lay out the TeX Live tree of a texlive/texlive image, pinned by digest,
without a container runtime (lane BUNDLE-PUBLISH).

    fetch_image.py texlive/texlive@sha256:<index digest> DEST [--arch amd64] [--keep-layers DIR]
                   [--mirror ghcr.io/flash-tex/texlive]

Fetches from the registry API (Docker Hub for a bare `owner/name`, or the
host a reference names, e.g. `ghcr.io/...`): the image index, whose SHA-256 must
be the pinned digest; the platform's manifest, whose SHA-256 must be the one
the index names; and every layer, whose SHA-256 must be the one the
manifest names. So every byte is verified against the pinned digest, as
`docker pull` verifies it. The layers are applied in order, whiteouts
included, and only `usr/local/texlive/` is kept: DEST/2026/... is the tree
`docker cp <container>:/usr/local/texlive/. DEST/` gives (as
.github/actions/texlive-2026 does), with the same file contents. It runs the
same on a Mac and on a Linux runner, so a bundle packed from it has the same
digest on both.

`--mirror REPO` (any number) names copies of the same image to fall back to,
in order, when a source does not answer: the digests are the image's own, so
a mirror can only serve the very same bytes. bundle-publish.yml keeps one at
ghcr.io/flash-tex/texlive, because Docker Hub may stop serving a digest no
tag points at any more. A private GHCR package is read with GHCR_TOKEN.
"""

import argparse
import hashlib
import json
import os
import shutil
import sys
import tarfile
import base64
import urllib.error
import urllib.request

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


class Source:
    """One registry repository holding the image: its API base, the
    repository's name there, and a pull token."""

    def __init__(self, ref):
        host, _, rest = ref.partition("/")
        if "." in host or ":" in host:
            self.base, self.repo = f"https://{host}", rest
            if host == "ghcr.io":
                auth = f"https://ghcr.io/token?scope=repository:{rest}:pull"
            else:
                auth = None
        else:
            self.base, self.repo = "https://registry-1.docker.io", ref
            auth = f"https://auth.docker.io/token?service=registry.docker.io&scope=repository:{ref}:pull"
        self.name = ref
        self.token = None
        if auth:
            req = urllib.request.Request(auth)
            gh = os.environ.get("GHCR_TOKEN")
            if gh and self.base == "https://ghcr.io":
                req.add_header("Authorization", "Basic " + base64.b64encode(f"x:{gh}".encode()).decode())
            with urllib.request.urlopen(req, timeout=60) as r:
                self.token = json.load(r)["token"]

    def get(self, path, accept=None):
        h = {"Accept": accept} if accept else {}
        if self.token:
            h["Authorization"] = f"Bearer {self.token}"
        return urllib.request.urlopen(urllib.request.Request(f"{self.base}/v2/{self.repo}/{path}", headers=h),
                                      timeout=120)


def from_any(sources, what, fn):
    """fn(source) from the first source that answers."""
    errs = []
    for src in sources:
        try:
            return fn(src)
        except (urllib.error.URLError, OSError) as e:
            errs.append(f"{src.name}: {e}")
            print(f"fetch_image: {what} from {src.name} failed ({e}); trying the next source", file=sys.stderr)
    die(f"{what}: no source answered: {'; '.join(errs)}")


def fetch_verified(sources, digest, accept=None):
    def one(src):
        with src.get(f"manifests/{digest}", accept) as r:
            return r.read()
    body = from_any(sources, digest, one)
    if "sha256:" + hashlib.sha256(body).hexdigest() != digest:
        die(f"{digest}: the registry sent other bytes")
    return json.loads(body)


def fetch_layer(sources, digest, path):
    if os.path.isfile(path) and sha256_file(path) == digest[7:]:
        return
    tmp = path + ".part"

    def one(src):
        h = hashlib.sha256()
        with src.get(f"blobs/{digest}") as r, open(tmp, "wb") as f:
            while True:
                b = r.read(1 << 20)
                if not b:
                    break
                h.update(b)
                f.write(b)
        return h.hexdigest()
    got = from_any(sources, f"layer {digest}", one)
    if "sha256:" + got != digest:
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


def inside(root, p):
    """Whether the path p is root or below it (both already resolved)."""
    return p == root or p.startswith(root + os.sep)


def checked_dir(dest_real, dest, d, name):
    """dest/d, refused unless it resolves inside dest: a symlink laid down
    by an earlier entry must not carry a later one out of the tree."""
    target = os.path.join(dest, d)
    real = os.path.realpath(target)
    if not inside(dest_real, real):
        die(f"{name}: its directory resolves to {real}, outside {dest_real}")
    return target


def apply_layer(path, dest):
    """Extract the layer's usr/local/texlive/ part over dest (what lies
    under usr/local/texlive/ goes to dest/), honouring whiteouts.

    Refused, and fatal: a member path that is absolute or has `..`; one whose
    directory resolves (through symlinks already extracted) outside dest; a
    symlink whose target is absolute or leaves dest; a hard link to anything
    outside the tree. The pinned digest makes these unlikely, not impossible."""
    n = 0
    dest_real = os.path.realpath(dest)
    with tarfile.open(path, "r:gz") as t:
        for m in t:
            if not m.name.startswith(KEEP):
                continue
            rel = m.name[len(KEEP):].rstrip("/")
            if not rel:
                continue
            if rel.startswith("/") or any(part in ("..", ".", "") for part in rel.split("/")):
                die(f"{m.name}: not a plain relative path")
            d, base = os.path.split(rel)
            target_dir = checked_dir(dest_real, dest, d, m.name)
            if base == ".wh..wh..opq":
                for e in os.listdir(target_dir) if os.path.isdir(target_dir) else []:
                    remove(os.path.join(target_dir, e))
                continue
            if base.startswith(".wh."):
                remove(os.path.join(target_dir, base[4:]))
                continue
            out = os.path.join(target_dir, base)
            if m.isdir():
                if os.path.lexists(out) and not os.path.isdir(out):
                    remove(out)
                os.makedirs(out, exist_ok=True)
                checked_dir(dest_real, dest, rel, m.name)
                continue
            if os.path.lexists(out):
                remove(out)
            os.makedirs(target_dir, exist_ok=True)
            if m.issym():
                link = m.linkname
                # Lexically and through the links already laid down.
                lexical = os.path.normpath(os.path.join(dest_real, d, link))
                real = os.path.realpath(os.path.join(dest_real, d, link))
                if os.path.isabs(link) or not inside(dest_real, lexical) or not inside(dest_real, real):
                    die(f"{m.name}: symlink to {link!r} leaves the tree")
                os.symlink(link, out)
            elif m.islnk():
                src = m.linkname
                srel = src[len(KEEP):] if src.startswith(KEEP) else None
                if srel is None or srel.startswith("/") or any(x in ("..", ".", "") for x in srel.split("/")):
                    die(f"{m.name}: hard link to {src!r} outside the tree")
                spath = os.path.join(dest, srel)
                if not inside(dest_real, os.path.realpath(spath)):
                    die(f"{m.name}: hard link to {src!r} resolves outside the tree")
                os.link(spath, out)
            elif m.isfile():
                with t.extractfile(m) as src, open(out, "wb") as f:
                    shutil.copyfileobj(src, f, 1 << 20)
                os.chmod(out, m.mode & 0o777)
            else:
                die(f"{m.name}: not a file, directory or link")
            n += 1
    return n


def check_links(dest):
    """After the last layer: every symlink in the tree must still resolve
    inside it. A link checked when it was laid down can be redirected by a
    later one (`a/b/s -> x/../..`, then `a/b/x -> ../..` makes s point above
    dest), so the per-member checks are not enough on their own. Fatal, and
    the tree is removed."""
    dest_real = os.path.realpath(dest)
    bad = []
    for dp, dns, fns in os.walk(dest):
        for n in dns + fns:
            p = os.path.join(dp, n)
            if os.path.islink(p) and not inside(dest_real, os.path.realpath(p)):
                bad.append(f"{os.path.relpath(p, dest)} -> {os.readlink(p)} (resolves to {os.path.realpath(p)})")
    if bad:
        shutil.rmtree(dest, ignore_errors=True)
        die("symlinks that leave the tree (it was removed):\n  " + "\n  ".join(bad))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("image", help="REPO@sha256:<index digest>")
    ap.add_argument("dest")
    ap.add_argument("--arch", default="amd64")
    ap.add_argument("--keep-layers", help="directory for the downloaded layers (default: removed after)")
    ap.add_argument("--mirror", action="append", default=[], help="another repository holding the same image")
    a = ap.parse_args()
    if "@sha256:" not in a.image:
        die(f"{a.image} is not pinned by digest")
    repo, digest = a.image.split("@", 1)
    sources = []
    for ref in [repo, *a.mirror]:
        try:
            sources.append(Source(ref))
        except (urllib.error.URLError, OSError) as e:
            print(f"fetch_image: {ref}: no pull token ({e})", file=sys.stderr)
    if not sources:
        die("no source of the image answered")
    index = fetch_verified(sources, digest, ACCEPT)
    mdigest = digest
    if "manifests" in index:
        ms = [m for m in index["manifests"]
              if m.get("platform", {}).get("os") == "linux" and m.get("platform", {}).get("architecture") == a.arch]
        if len(ms) != 1:
            die(f"{a.image}: no single linux/{a.arch} manifest")
        mdigest = ms[0]["digest"]
        manifest = fetch_verified(sources, mdigest, ACCEPT)
    else:
        manifest = index
    layers_dir = a.keep_layers or os.path.join(os.path.dirname(os.path.abspath(a.dest)), ".layers")
    os.makedirs(layers_dir, exist_ok=True)
    if os.path.lexists(a.dest):
        die(f"{a.dest} exists")
    os.makedirs(a.dest)
    for layer in manifest["layers"]:
        p = os.path.join(layers_dir, layer["digest"][7:] + ".tgz")
        fetch_layer(sources, layer["digest"], p)
        n = apply_layer(p, a.dest)
        print(f"layer {layer['digest'][7:19]} ({layer['size']} bytes): {n} TeX Live entries", flush=True)
    check_links(a.dest)
    if not a.keep_layers:
        shutil.rmtree(layers_dir, ignore_errors=True)
    print(f"{a.image} (linux/{a.arch} {mdigest}) -> {a.dest}")


if __name__ == "__main__":
    main()
