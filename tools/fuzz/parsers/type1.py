#!/usr/bin/env python3
"""Type 1 font program (.pfb) crash fuzzer. MIT licensed. Stdlib only.

Each iteration mutates a .pfb seed, installs it as fuzz.pfb next to a copy
of cmr10.tfm (as fuzz.tfm), and runs the candidate pdfTeX job::

    \\pdfmapline{+fuzz fuzz <fuzz.pfb}\\font\\x=fuzz \\x abc \\bye

Classes: crash / hang / ok / graceful-error. Every crash or hang input is
saved to OUT/<class>/<sha256-prefix>.pfb with a .json sidecar (seed,
mutation, returncode, last stderr line, panic location if present);
signatures (panic location or signal) dedupe repeat bugs in signatures.json.
Deterministic given --seed: every choice goes through random.Random.
"""
import argparse
import hashlib
import json
import os
import random
import re
import shutil
import subprocess
import sys
import tempfile

def candidate_env():
    """Environment for the candidate: the caller's, with FLASHTEX_POOL and
    FLASHTEX_FORMATS exported by the caller."""
    env = dict(os.environ)
    env["SOURCE_DATE_EPOCH"] = "0"
    return env


JOB = ("\\pdfmapline{+fuzz fuzz <fuzz.pfb}\\font\\x=fuzz \\x abc \\bye\n")
CLASSES = ("crash", "hang", "ok", "graceful-error")
SEED_NAMES = ("cmr10.pfb", "cmti10.pfb", "cmbx10.pfb")
BOUNDARIES = (0, 1, 0xFFFF, 0x7FFFFFFF, 0xFFFFFFFF)
SEGTYPES = (0, 1, 2, 3, 4, 5, 0xFF)


def _kpse(name):
    try:
        out = subprocess.run(["kpsewhich", name], capture_output=True,
                             text=True, timeout=10)
        path = out.stdout.strip()
        return path or None
    except (OSError, subprocess.TimeoutExpired):
        return None


def _synthetic_seed():
    body = b"%!PS-AdobeFont-1.0 synthetic\n"
    return (b"\x80\x01" + len(body).to_bytes(4, "little") + body
            + b"\x80\x03\x00\x00\x00\x00")


def load_seeds():
    """Read .pfb seeds from the TeX Live tree at run time; fall back to a
    synthetic minimal PFB. Returns (seeds, tfm) with seeds a list of
    (name, bytes) and tfm the bytes of cmr10.tfm (b"" when missing)."""
    seeds = []
    for name in SEED_NAMES:
        path = _kpse(name)
        if not path:
            continue
        try:
            with open(path, "rb") as fh:
                seeds.append((name, fh.read()))
        except OSError:
            continue
    if not seeds:
        seeds = [("synthetic.pfb", _synthetic_seed())]
    tfm_path = _kpse("cmr10.tfm")
    tfm = b""
    if tfm_path:
        try:
            with open(tfm_path, "rb") as fh:
                tfm = fh.read()
        except OSError:
            pass
    return seeds, tfm


def parse_segments(data):
    """Split raw PFB bytes into (offset, segtype, length) headers."""
    segs, off = [], 0
    while (off + 6 <= len(data) and data[off] == 0x80
            and data[off + 1] in (1, 2, 3)):
        ln = int.from_bytes(data[off + 2:off + 6], "little")
        segs.append((off, data[off + 1], ln))
        if data[off + 1] == 3:
            break
        off += 6 + ln
    return segs


def _m_flip(data, rng, _segs):
    buf = bytearray(data)
    for _ in range(rng.randint(1, 4)):
        buf[rng.randrange(len(buf))] = rng.randrange(256)
    return bytes(buf), "flip"


def _m_trunc(data, rng, _segs):
    cut = rng.randrange(1, len(data))
    return data[:cut], "trunc@%d/%d" % (cut, len(data))


def _m_delete(data, rng, _segs):
    i = rng.randrange(len(data))
    j = rng.randint(i + 1, len(data))
    return data[:i] + data[j:], "del:%d:%d" % (i, j)


def _m_dup(data, rng, _segs):
    i = rng.randrange(len(data))
    j = min(rng.randint(i + 1, len(data)), i + 4096)
    k = rng.randrange(len(data) + 1)
    out = data[:k] + data[i:j] + data[k:]
    return out[:2000000], "dup:%d:%d@%d" % (i, j, k)


def _m_segtype(data, rng, segs):
    if not segs:
        return None
    off, typ, _ln = rng.choice(segs)
    new = rng.choice([t for t in SEGTYPES if t != typ])
    buf = bytearray(data)
    buf[off + 1] = new
    return bytes(buf), "segtype@%d:%d->%d" % (off, typ, new)


def _m_seglen(data, rng, segs):
    if not segs:
        return None
    off, _typ, ln = rng.choice(segs)
    new = rng.choice(BOUNDARIES)
    buf = bytearray(data)
    buf[off + 2:off + 6] = new.to_bytes(4, "little")
    return bytes(buf), "seglen@%d:%d->%d" % (off, ln, new)


def _m_eexec(data, rng, segs):
    twos = [s for s in segs if s[1] == 2]
    if not twos:
        return None
    start = twos[0][0] + 6
    end = min(start + twos[0][2], len(data))
    if end <= start + 1:
        return None
    cut = rng.randrange(start + 1, end + 1)
    return data[:cut], "eexec-trunc@%d/%d" % (cut, len(data))


MUTATIONS = (("flip", _m_flip), ("trunc", _m_trunc),
             ("delete", _m_delete), ("dup", _m_dup),
             ("segtype", _m_segtype), ("seglen", _m_seglen),
             ("eexec-trunc", _m_eexec))


def mutate_with_info(data, rng):
    """Apply ONE mutation to raw PFB bytes; return (new_bytes, description).
    Deterministic for a given random.Random state."""
    if len(data) < 2:
        return _m_flip(data + b"\x00\x00", rng, [])
    segs = parse_segments(data)
    names = [k for k, _ in MUTATIONS]
    first = rng.choice(names)
    rest = [k for k in names if k != first]
    rng.shuffle(rest)
    table = dict(MUTATIONS)
    for key in [first] + rest:
        got = table[key](data, rng, segs)
        if got is not None:
            return got
    return _m_flip(data, rng, segs)


def is_crash(returncode, log):
    return (returncode is not None
            and (returncode < 0 or returncode == 101
                 or "panicked at" in (log or "")))


def panic_location(log):
    if not log or "panicked at" not in log:
        return None
    after = log.split("panicked at", 1)[1]
    match = re.search(r":\d+", after)
    if not match:
        return None
    return after[:match.start()].strip()


def signature(cls, returncode, log):
    if cls == "hang":
        return "hang"
    loc = panic_location(log)
    if loc is not None:
        return "panic:" + loc
    if returncode is not None and returncode < 0:
        return "signal:%d" % (-returncode,)
    return "exit:%s" % (returncode,)


def run_once(pfb, tfm, candidate, timeout):
    """Install one job and run the candidate; return
    (class, returncode, stdout, stderr)."""
    tmp = tempfile.mkdtemp(prefix="t1fuzz-")
    try:
        with open(os.path.join(tmp, "fuzz.pfb"), "wb") as fh:
            fh.write(pfb)
        with open(os.path.join(tmp, "fuzz.tfm"), "wb") as fh:
            fh.write(tfm)
        with open(os.path.join(tmp, "X.tex"), "w") as fh:
            fh.write(JOB)
        env = candidate_env()
        try:
            proc = subprocess.run(
                [candidate, "-fmt=pdftex", "-interaction=nonstopmode",
                 "X.tex"], cwd=tmp, env=env, stdout=subprocess.PIPE,
                stderr=subprocess.PIPE, timeout=timeout)
            out = proc.stdout.decode("utf-8", "replace")
            err = proc.stderr.decode("utf-8", "replace")
            rc = proc.returncode
        except subprocess.TimeoutExpired as exc:
            out = (exc.stdout or b"").decode("utf-8", "replace")
            err = (exc.stderr or b"").decode("utf-8", "replace")
            return "hang", None, out, err
        if is_crash(rc, out + "\n" + err):
            return "crash", rc, out, err
        return ("ok" if rc == 0 else "graceful-error"), rc, out, err
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def run_fuzz(candidate, iterations, seed, out_dir, timeout, seeds=None,
             tfm=None):
    rng = random.Random(seed)
    if seeds is None or tfm is None:
        seeds, tfm = load_seeds()
    counts = {c: 0 for c in CLASSES}
    sig_counts, seen = {}, set()
    for i in range(iterations):
        name, base = rng.choice(seeds)
        mutated, desc = mutate_with_info(base, rng)
        cls, rc, out, err = run_once(mutated, tfm, candidate, timeout)
        counts[cls] += 1
        if cls in ("crash", "hang"):
            log = out + "\n" + err
            sig = signature(cls, rc, log)
            sig_counts[sig] = sig_counts.get(sig, 0) + 1
            seen.add(sig)
            digest = hashlib.sha256(mutated).hexdigest()[:16]
            cls_dir = os.path.join(out_dir, cls)
            os.makedirs(cls_dir, exist_ok=True)
            blob = os.path.join(cls_dir, digest + ".pfb")
            if not os.path.exists(blob):
                with open(blob, "wb") as fh:
                    fh.write(mutated)
            err_lines = err.strip().splitlines()
            with open(os.path.join(cls_dir, digest + ".json"), "w") as fh:
                json.dump({"iteration": i, "seed": name, "mutation": desc,
                           "returncode": rc,
                           "last_stderr_line": err_lines[-1] if err_lines
                           else "", "panic_location": panic_location(log),
                           "signature": sig}, fh, indent=2)
        if (i + 1) % 100 == 0:
            print("fuzz %d/%d: %s"
                  % (i + 1, iterations,
                     " ".join("%s=%d" % (c, counts[c]) for c in CLASSES)))
    if sig_counts:
        merged = dict(sig_counts)
        path = os.path.join(out_dir, "signatures.json")
        try:
            with open(path) as fh:
                old = json.load(fh)
            if isinstance(old, dict):
                for key, val in old.items():
                    merged[key] = merged.get(key, 0) + val
        except (OSError, ValueError):
            pass
        try:
            os.makedirs(out_dir, exist_ok=True)
            with open(path, "w") as fh:
                json.dump(merged, fh, indent=2, sort_keys=True)
        except OSError:
            pass
    print("done: %d iterations: %s (unique signatures: %d)"
          % (iterations,
             " ".join("%s=%d" % (c, counts[c]) for c in CLASSES), len(seen)))
    return counts


def main(argv=None):
    ap = argparse.ArgumentParser(description="Type 1 PFB crash fuzzer")
    ap.add_argument("--candidate", required=True)
    ap.add_argument("--iterations", type=int, required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--timeout", type=float, required=True)
    args = ap.parse_args(argv)
    if args.iterations < 0:
        print("error: --iterations must be >= 0", file=sys.stderr)
        return 2
    return run_fuzz(args.candidate, args.iterations, args.seed, args.out,
                    args.timeout)


if __name__ == "__main__":
    result = main()
    sys.exit(0 if isinstance(result, dict) else result)
