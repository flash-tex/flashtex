#!/usr/bin/env python3
"""Read-only pinned bundle coverage check; no discovery/native/paint assertion.

Run: python3 apps/mac/scripts/verify_bundle_resources.py /path/FlashTeX.app/Contents/Resources
Exit 0: pinned resources match; 1: resource refusal; 2: setup/manifest refusal.
No directories are scanned and no resource bytes are changed or printed.

Moved with its pinned manifest (native-assets-manifest.json, next to this
script) from crates/rendering-core/tools/ in old-engine retirement stage S1;
rendering-core itself was deleted in stage S2 (#1236), and this copy is the only
one.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys

MANIFEST = Path(__file__).resolve().parent / "native-assets-manifest.json"
MANIFEST_SHA256 = "3f0286b31cb69c79e10cbbc578d9fe84845cb1f87224390b10d800793531dc93"
MAX_MANIFEST = 65536
MAX_ENTRIES = 64
MAX_FILE = 16 * 1024 * 1024
MAX_TOTAL = 64 * 1024 * 1024


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate manifest key")
        result[key] = value
    return result


def validate_entries(manifest):
    if manifest.get("schema_version") != 1:
        raise ValueError("unsupported manifest version")
    entries = manifest.get("required_for_existing_fixtures")
    if not isinstance(entries, list) or not 1 <= len(entries) <= MAX_ENTRIES:
        raise ValueError("manifest entry count")
    seen = set()
    total = 0
    for entry in entries:
        if not isinstance(entry, dict):
            raise ValueError("invalid resource entry")
        path = entry.get("path")
        if (not isinstance(path, str) or len(path) > 512 or
                not re.fullmatch(r"[A-Za-z0-9_./-]+", path) or
                any(part in ("", ".", "..") for part in path.split("/")) or path in seen):
            raise ValueError("unsafe or duplicate resource path")
        seen.add(path)
        size = entry.get("byte_length")
        if type(size) is not int or not 0 < size <= MAX_FILE:
            raise ValueError("resource byte limit")
        digest = entry.get("sha256")
        if not isinstance(digest, str) or not re.fullmatch(r"[a-f0-9]{64}", digest):
            raise ValueError("invalid resource digest")
        total += size
    if total > MAX_TOTAL:
        raise ValueError("aggregate resource byte limit")
    return entries


def pinned_manifest():
    with MANIFEST.open("rb") as source:
        raw = source.read(MAX_MANIFEST + 1)
    if len(raw) > MAX_MANIFEST or hashlib.sha256(raw).hexdigest() != MANIFEST_SHA256:
        raise ValueError("pinned manifest mismatch")
    value = json.loads(raw, object_pairs_hook=unique_object)
    validate_entries(value)
    return value


def check_resource(root_fd, entry):
    """Descriptor-relative no-follow walk: never follow a resource symlink."""
    directory = os.dup(root_fd)
    resource = None
    path = entry["path"]
    try:
        parts = path.split("/")
        for part in parts[:-1]:
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                            dir_fd=directory)
            os.close(directory)
            directory = child
        resource = os.open(parts[-1], os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK,
                           dir_fd=directory)
        before = os.fstat(resource)
        if not stat.S_ISREG(before.st_mode):
            return {"path": path, "status": "not_regular"}
        if before.st_size != entry["byte_length"]:
            return {"path": path, "status": "length_mismatch", "actual_bytes": before.st_size}
        digest = hashlib.sha256()
        remaining = entry["byte_length"] + 1
        actual = 0
        while remaining:
            block = os.read(resource, min(65536, remaining))
            if not block:
                break
            actual += len(block)
            remaining -= len(block)
            digest.update(block)
        after = os.fstat(resource)
        if ((before.st_size, before.st_mtime_ns, before.st_ctime_ns) !=
                (after.st_size, after.st_mtime_ns, after.st_ctime_ns)):
            return {"path": path, "status": "changed_during_read"}
        actual_hash = digest.hexdigest()
        status = "verified" if actual == entry["byte_length"] and actual_hash == entry["sha256"] else "digest_mismatch"
        return {"path": path, "status": status, "actual_bytes": actual,
                "actual_sha256": actual_hash}
    except FileNotFoundError:
        return {"path": path, "status": "missing"}
    except OSError as error:
        return {"path": path, "status": "unsafe_or_unreadable", "errno": error.errno}
    finally:
        if resource is not None:
            os.close(resource)
        os.close(directory)


def verify(resources):
    manifest = pinned_manifest()
    entries = validate_entries(manifest)
    if not hasattr(os, "O_NOFOLLOW") or not hasattr(os, "O_DIRECTORY"):
        raise ValueError("descriptor-relative no-follow verification unavailable")
    root = os.open(resources, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        results = [check_resource(root, entry) for entry in entries]
    finally:
        os.close(root)
    return {"format": "flashtex-pinned-bundle-coverage-v1", "manifest_sha256": MANIFEST_SHA256,
            "status": "verified" if all(r["status"] == "verified" for r in results) else "refused",
            "resources": results, "native_discovery_executed": False,
            "scope": manifest["scope"], "extra_resources": "not_scanned"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("resources", help="app Contents/Resources directory (read only)")
    args = parser.parse_args()
    try:
        report = verify(args.resources)
    except (ValueError, OSError) as error:
        report = {"status": "setup_refused", "reason": str(error), "native_discovery_executed": False}
        print(json.dumps(report, indent=2))
        return 2
    print(json.dumps(report, indent=2))
    return 0 if report["status"] == "verified" else 1


if __name__ == "__main__":
    sys.exit(main())
