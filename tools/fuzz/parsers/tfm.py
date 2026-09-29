#!/usr/bin/env python3
"""TFM font-metric file fuzzer for the FlashTeX candidate engine. MIT.

Each iteration mutates a real .tfm seed (found at run time with kpsewhich,
never copied into the repo), writes it as fuzz.tfm next to a job file
containing ``\\font\\x=fuzz \\x a \\bye``, and runs the candidate on it.
Classes: ok / graceful-error / crash / hang. Crash and hang inputs are
stored under OUT/<class>/ with a .json sidecar, deduped by panic location
or signal. Stdlib only. Deterministic given --seed (all draws go through
one random.Random).
"""
import argparse
import hashlib
import json
import os
import random
import re
import shutil
import struct
import subprocess
import sys
import tempfile

JOB_TEX = "\\font\\x=fuzz \\x a \\bye\n"
SEED_NAMES = ("cmr10.tfm", "cmmi10.tfm", "cmsy10.tfm", "ptmr8t.tfm")
# TFM header: 12 big-endian u16 words (lf, lh, bc, ec, nw, nh, nd, ni,
# nl, nk, ne, np) in the first 24 bytes.
HEADER_OFFSETS = tuple(2 * i for i in range(12))
FIELD_VALUES = (0, 1, 0xFFFF, 0x7FFFFFFF, 0xFFFFFFFF)
CLASSES = ("ok", "graceful-error", "crash", "hang")
STORE = ("crash", "hang")


def load_seeds():
    """Read seed TFM bytes from the TeX Live tree at run time."""
    seeds = []
    for name in SEED_NAMES:
        try:
            cap = subprocess.run(["kpsewhich", name], capture_output=True,
                                 text=True, timeout=30)
        except (OSError, subprocess.TimeoutExpired):
            continue
        path = (cap.stdout or "").strip().splitlines()
        if not path:
            continue
        try:
            with open(path[0], "rb") as fh:
                seeds.append((name, fh.read()))
        except OSError:
            continue
    if not seeds:
        fields = [34, 2, 0, 1, 1, 1, 0, 0, 0, 0, 0, 1]
        blob = struct.pack(">" + "H" * 12, *fields) + bytes(range(256)) * 2
        seeds.append(("synthetic.tfm", blob))
    return [(n, d) for n, d in seeds if d]


def _flip(data, rng):
    if not data:
        return None
    buf = bytearray(data)
    for _ in range(1 + rng.randrange(8)):
        buf[rng.randrange(len(buf))] = rng.randrange(256)
    return bytes(buf), "flip"


def _trunc(data, rng):
    if len(data) < 2:
        return None
    off = rng.randrange(len(data))
    return data[:off], "trunc@%d/%d" % (off, len(data))


def _delete(data, rng):
    if len(data) < 4:
        return None
    i = rng.randrange(len(data))
    j = min(len(data), i + 1 + rng.randrange(min(64, len(data) - i)))
    return data[:i] + data[j:], "del@%d+%d" % (i, j - i)


def _dup(data, rng):
    if len(data) < 2:
        return None
    i = rng.randrange(len(data))
    j = min(len(data), i + 1 + rng.randrange(min(64, len(data) - i)))
    chunk = data[i:j]
    return data[:j] + chunk + data[j:], "dup@%d+%d" % (i, j - i)


def _field(data, rng):
    if not data:
        return None
    off = rng.choice(HEADER_OFFSETS)
    val = rng.choice(FIELD_VALUES)
    raw = struct.pack(">H", val) if val <= 0xFFFF else struct.pack(">I", val)
    buf = bytearray(data)
    if off + len(raw) > len(buf):
        buf.extend(b"\x00" * (off + len(raw) - len(buf)))
    buf[off:off + len(raw)] = raw
    return bytes(buf), "field@%d=0x%X" % (off, val)


MUTATIONS = (_flip, _trunc, _delete, _dup, _field)


def mutate_with_info(data, rng):
    """Apply one mutation; return (new_bytes, description). Deterministic."""
    order = list(MUTATIONS)
    rng.shuffle(order)
    for fn in order:
        got = fn(data, rng)
        if got is not None and (got[0] != data or len(data) == 0):
            return got
    buf = bytearray(data or b"\x00")
    buf[0] ^= 0xFF
    return bytes(buf), "flip-fallback"


def panic_location(log):
    """Text after 'panicked at' up to the first :digits span, else None."""
    if not log or "panicked at" not in log:
        return None
    after = log.split("panicked at", 1)[1]
    m = re.search(r":\d+", after)
    return after[:m.start()].strip() if m else None


def is_crash(returncode, log):
    return (returncode is not None
            and (returncode < 0 or returncode == 101
                 or "panicked at" in (log or "")))


def signature(cls, returncode, log):
    if cls == "hang":
        return "hang"
    loc = panic_location(log)
    if loc is not None:
        return "crash:panic:" + loc
    if returncode is not None and returncode < 0:
        return "crash:signal:%d" % (-returncode,)
    return "crash:exit:%s" % (returncode,)


def candidate_env():
    env = dict(os.environ)  # FLASHTEX_POOL / FLASHTEX_FORMATS come from the caller
    env["SOURCE_DATE_EPOCH"] = "0"
    return env


def run_one(tfm_bytes, candidate, timeout):
    """Run one mutated TFM through the candidate; return (class, rc, log)."""
    workdir = tempfile.mkdtemp(prefix="tfm-fuzz-")
    try:
        with open(os.path.join(workdir, "fuzz.tfm"), "wb") as fh:
            fh.write(tfm_bytes)
        with open(os.path.join(workdir, "job.tex"), "w") as fh:
            fh.write(JOB_TEX)
        try:
            cap = subprocess.run(
                [candidate, "-fmt=pdftex", "-interaction=nonstopmode",
                 "job.tex"],
                cwd=workdir, env=candidate_env(), capture_output=True,
                timeout=timeout)
            log = (cap.stdout or b"").decode("utf-8", "replace")
            log += (cap.stderr or b"").decode("utf-8", "replace")
            rc = cap.returncode
        except subprocess.TimeoutExpired as exc:
            out = (exc.stdout or b"") + (exc.stderr or b"")
            return "hang", None, out.decode("utf-8", "replace")
        if is_crash(rc, log):
            return "crash", rc, log
        return ("ok" if rc == 0 else "graceful-error"), rc, log
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


def load_known_signatures(out_dir):
    known = set()
    try:
        for root, _dirs, files in os.walk(out_dir):
            for name in files:
                if not name.endswith(".json") or name == "signatures.json":
                    continue
                try:
                    with open(os.path.join(root, name)) as fh:
                        info = json.load(fh)
                    if isinstance(info, dict) and info.get("signature"):
                        known.add(info["signature"])
                except (OSError, ValueError):
                    continue
    except OSError:
        pass
    return known


def last_line(log):
    lines = [ln for ln in (log or "").splitlines() if ln.strip()]
    return lines[-1][:200] if lines else ""


def run_fuzz(candidate, seeds, out_dir, iterations, seed, timeout):
    rng = random.Random(seed)
    known = load_known_signatures(out_dir)
    seen = set()
    counts = {c: 0 for c in CLASSES}
    for i in range(iterations):
        name, base = seeds[rng.randrange(len(seeds))]
        mutated, mutation = mutate_with_info(base, rng)
        cls, rc, log = run_one(mutated, candidate, timeout)
        counts[cls] += 1
        if cls in STORE:
            sig = signature(cls, rc, log)
            if sig not in seen and sig not in known:
                seen.add(sig)
                digest = hashlib.sha256(mutated).hexdigest()[:16]
                cls_dir = os.path.join(out_dir, cls)
                os.makedirs(cls_dir, exist_ok=True)
                with open(os.path.join(cls_dir, digest + ".tfm"), "wb") as fh:
                    fh.write(mutated)
                with open(os.path.join(cls_dir, digest + ".json"), "w") as fh:
                    json.dump({"seed": seed, "origin": name,
                               "mutation": mutation, "returncode": rc,
                               "last_line": last_line(log),
                               "panic_location": panic_location(log),
                               "signature": sig}, fh, indent=2)
    return counts


def main(argv=None):
    ap = argparse.ArgumentParser(description="TFM font-metric fuzzer")
    ap.add_argument("--candidate", required=True)
    ap.add_argument("--iterations", type=int, default=100)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--out", default="tfm-out")
    ap.add_argument("--timeout", type=float, default=10.0)
    args = ap.parse_args(argv)
    if args.iterations < 0:
        print("error: --iterations must be >= 0", file=sys.stderr)
        return 2
    seeds = load_seeds()
    counts = run_fuzz(args.candidate, seeds, args.out, args.iterations,
                      args.seed, args.timeout)
    print("done: %d iterations: %s"
          % (args.iterations,
             " ".join("%s=%d" % (c, counts[c]) for c in CLASSES)))
    return counts


if __name__ == "__main__":
    result = main()
    sys.exit(0 if isinstance(result, dict) else result)
