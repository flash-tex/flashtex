#!/usr/bin/env python3
"""TFM font-metric file fuzzer for the FlashTeX candidate engine. MIT.

Each iteration mutates a real .tfm seed (found at run time with kpsewhich,
never copied into the repo), writes it as fuzz.tfm next to a job file
containing ``\\pdfmapline{+fuzz fuzz <cmr10.pfb}\\font\\x=fuzz \\x a\\bye``
(plus a copy of cmr10.pfb resolved with kpsewhich), and runs the
candidate on it. An unmutated cmr10 copy compiles and embeds cleanly, so
a font that loads is distinguishable from one the engine rejects.
Classes: ok / tfm-rejected / graceful-error / crash / hang. Crash and
hang inputs are stored under OUT/<class>/ with a .json sidecar, deduped
by panic location or signal. Stdlib only. Deterministic given --seed
(all draws go through one random.Random).
"""
import argparse
import hashlib
import json
import os
import random
import shutil
import struct
import subprocess
import sys
import tempfile

sys.path.insert(0,
                os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import run as fuzz_run

JOB_TEX = "\\pdfmapline{+fuzz fuzz <cmr10.pfb}\\font\\x=fuzz \\x a\\bye\n"
SEED_NAMES = ("cmr10.tfm", "cmmi10.tfm", "cmsy10.tfm", "cmex10.tfm",
              "ptmr8t.tfm")
# TFM header: 12 big-endian u16 words (lf, lh, bc, ec, nw, nh, nd, ni,
# nl, nk, ne, np) in the first 24 bytes.
FIELD_NAMES = ("lf", "lh", "bc", "ec", "nw", "nh", "nd", "ni", "nl",
               "nk", "ne", "np")
HEADER_OFFSETS = tuple(2 * i for i in range(12))
FIELD_VALUES = (0, 1, 0xFFFF, 0x7FFFFFFF, 0xFFFFFFFF)
# Log text pdfTeX prints for a rejected metric, e.g.
# "! Font \\x=fuzz not loadable: Bad metric (TFM) file."
TFM_REJECT_HINTS = ("Bad metric", "not loadable")
CLASSES = ("ok", "tfm-rejected", "graceful-error", "crash", "hang",
           "output-flood")
STORE = ("crash", "hang", "output-flood")
_PFB_CACHE = {}


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


def parse_tfm(data):
    """Parse the TFM header; return a dict or None when too short.

    The dict holds lf, lh, bc, ec, nw, nh, nd, ni, nl, nk, ne, np plus
    nchars (ec-bc+1, or 0 when ec < bc), spans {table: (start, end)} in
    file order, and total (byte after the param table). Lengths are in
    4-byte words for lf and match the TFM spec: 24 bytes of half-words,
    then lh header words, then one 4-byte char_info per char, then the
    width/height/depth/italic/lig-kern/kern/exten/param tables of 4
    bytes per entry. Tables may extend past the end of a truncated
    input; callers check spans against len(data).
    """
    if len(data) < 24:
        return None
    info = dict(zip(FIELD_NAMES, struct.unpack(">12H", data[:24])))
    nchars = info["ec"] - info["bc"] + 1 if info["ec"] >= info["bc"] else 0
    info["nchars"] = nchars
    spans = {}
    pos = 24
    spans["header"] = (pos, pos + 4 * info["lh"])
    pos = spans["header"][1]
    spans["char_info"] = (pos, pos + 4 * nchars)
    pos = spans["char_info"][1]
    for key, name in (("nw", "width"), ("nh", "height"),
                      ("nd", "depth"), ("ni", "italic"),
                      ("nl", "ligkern"), ("nk", "kern"),
                      ("ne", "exten"), ("np", "param")):
        spans[name] = (pos, pos + 4 * info[key])
        pos += 4 * info[key]
    info["spans"] = spans
    info["total"] = pos
    return info


def _span_in(start, end, data):
    return 0 <= start < end <= len(data)


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


def _charinfo(data, rng):
    """Edit one byte of one char_info word (width/height/italic/tag)."""
    info = parse_tfm(data)
    if info is None or info["nchars"] <= 0:
        return None
    start, end = info["spans"]["char_info"]
    if not _span_in(start, end, data):
        return None
    idx = rng.randrange(info["nchars"])
    sub = rng.randrange(4)
    off = start + 4 * idx + sub
    val = rng.randrange(256)
    if data[off] == val:
        val ^= 0xFF
    buf = bytearray(data)
    buf[off] = val
    return bytes(buf), "charinfo@%d+%d=0x%02X" % (idx, sub, val)


def _index(data, rng):
    """Set one char's width/height/depth/italic index out of range."""
    info = parse_tfm(data)
    if info is None or info["nchars"] <= 0:
        return None
    start, end = info["spans"]["char_info"]
    if not _span_in(start, end, data):
        return None
    idx = rng.randrange(info["nchars"])
    field = rng.choice(("w", "h", "d", "i"))
    off = start + 4 * idx
    buf = bytearray(data)
    if field == "w":
        cands = [info["nw"], info["nw"] + 1, 255, rng.randrange(256)]
        val = rng.choice(cands) & 0xFF
        if buf[off] == val:
            val ^= 0xFF
        buf[off] = val
        desc = "index@%d:w=%d" % (idx, val)
    elif field == "i":
        cands = [info["ni"], info["ni"] + 1, 255, rng.randrange(256)]
        val = rng.choice(cands) & 0xFF
        if buf[off + 2] == val:
            val ^= 0xFF
        buf[off + 2] = val
        desc = "index@%d:i=%d" % (idx, val)
    else:
        hi = field == "h"
        limit = info["nh"] if hi else info["nd"]
        cands = [limit, limit + 1, 15, rng.randrange(16)]
        val = rng.choice(cands) & 0x0F
        old = buf[off + 1]
        if hi:
            new = (old & 0x0F) | (val << 4)
        else:
            new = (old & 0xF0) | val
        if new == old:
            new ^= 0xF0 if hi else 0x0F
        buf[off + 1] = new
        desc = "index@%d:%s=%d" % (idx, field, val)
    return bytes(buf), desc


def _ligkern(data, rng):
    """Edit one lig_kern instruction field (skip/next/op/remainder).

    Each program step is 4 bytes: skip byte, next char, op byte,
    remainder. Skip choices include 0/128/255 (tight jumps, stop flag,
    long jumps that can loop); next-char and remainder choices include
    labels past the end of the program (>= nl) and kern indices past
    nk, plus out-of-range char codes.
    """
    info = parse_tfm(data)
    if info is None or info["nl"] <= 0:
        return None
    start, end = info["spans"]["ligkern"]
    if not _span_in(start, end, data):
        return None
    idx = rng.randrange(info["nl"])
    base = start + 4 * idx
    field = rng.choice(("skip", "next", "op", "rem"))
    buf = bytearray(data)
    if field == "skip":
        val = rng.choice((0, 1, 127, 128, 129, 255,
                          rng.randrange(256)))
        if buf[base] == val:
            val ^= 0xFF
        buf[base] = val
    elif field == "next":
        cands = [info["ec"] + 1, info["ec"] + 2, 255, 0,
                 rng.randrange(256)]
        val = rng.choice(cands) & 0xFF
        if buf[base + 1] == val:
            val ^= 0xFF
        buf[base + 1] = val
    elif field == "op":
        val = rng.choice((0, 1, 2, 3, 7, 11, 128, 129, 255,
                          rng.randrange(256)))
        if buf[base + 2] == val:
            val ^= 0xFF
        buf[base + 2] = val
    else:
        if buf[base + 2] >= 128:
            cands = [info["nk"], info["nk"] + 1, 255,
                     rng.randrange(256)]
        else:
            cands = [info["nl"], info["nl"] + 1, info["ec"] + 1, 255,
                     rng.randrange(256)]
        val = rng.choice(cands) & 0xFF
        if buf[base + 3] == val:
            val ^= 0xFF
        buf[base + 3] = val
    return bytes(buf), "ligkern@%d:%s=%d" % (idx, field, val)


def _exten(data, rng):
    """Edit one byte of one exten recipe (top/mid/bot/rep)."""
    info = parse_tfm(data)
    if info is None or info["ne"] <= 0:
        return None
    start, end = info["spans"]["exten"]
    if not _span_in(start, end, data):
        return None
    idx = rng.randrange(info["ne"])
    sub = rng.randrange(4)
    off = start + 4 * idx + sub
    cands = [info["ec"] + 1, info["ec"] + 2, 255, 0, rng.randrange(256)]
    val = rng.choice(cands) & 0xFF
    if data[off] == val:
        val ^= 0xFF
    buf = bytearray(data)
    buf[off] = val
    return bytes(buf), "exten@%d+%d=%d" % (idx, sub, val)


def _tablen(data, rng):
    """Set a header table length inconsistent with lf (or vice versa).

    Picks a length field (lh/bc/ec/nw/nh/nd/ni/nl/nk/ne/np, or lf
    itself) and changes it without adjusting the rest, so the declared
    tables no longer add up to 4*lf bytes. Tries a few candidates and
    keeps the first that is actually inconsistent.
    """
    if len(data) < 24:
        return None
    info = parse_tfm(data)
    fields = ["lf", "lh", "bc", "ec", "nw", "nh", "nd", "ni", "nl",
              "nk", "ne", "np"]
    order = fields[:]
    rng.shuffle(order)
    fallback = None
    for name in order:
        pos = 2 * FIELD_NAMES.index(name)
        old = info[name]
        for val in rng.sample((0, 1, old + 1, old + 2,
                               max(0, old - 1), max(0, old - 2),
                               0xFFFF, rng.randrange(0x10000)), 4):
            if not 0 <= val <= 0xFFFF or val == old:
                continue
            buf = bytearray(data)
            buf[pos:pos + 2] = struct.pack(">H", val)
            cand = parse_tfm(bytes(buf))
            if fallback is None:
                fallback = (bytes(buf), "tablen@%s=%d" % (name, val))
            if cand["total"] != 4 * cand["lf"]:
                return bytes(buf), "tablen@%s=%d" % (name, val)
    return fallback


MUTATIONS = (_flip, _trunc, _delete, _dup, _field, _charinfo, _index,
             _ligkern, _exten, _tablen)


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
    """Panic site as "<file>:<line>"; shared spelling, see run.py."""
    return fuzz_run.panic_location(log)


def is_crash(returncode, log=None):
    # Return code only: log text never decides a crash (an input merely
    # containing the words "panicked at" is not a crash).
    return (returncode is not None
            and (returncode < 0 or returncode == 101))


def signature(cls, returncode, log):
    if cls == "hang":
        return "hang"
    if cls == "output-flood":
        return "output-flood"
    return fuzz_run.crash_signature(returncode, log)


def candidate_env():
    env = dict(os.environ)  # FLASHTEX_POOL / FLASHTEX_FORMATS come from the caller
    env["SOURCE_DATE_EPOCH"] = "0"
    return env


def pfb_bytes():
    """Bytes of cmr10.pfb from the TeX Live tree (cached; None if lost)."""
    if "cmr10.pfb" not in _PFB_CACHE:
        blob = None
        try:
            cap = subprocess.run(["kpsewhich", "cmr10.pfb"],
                                 capture_output=True, text=True,
                                 timeout=30)
            path = (cap.stdout or "").strip().splitlines()
            if path:
                with open(path[0], "rb") as fh:
                    blob = fh.read()
        except (OSError, subprocess.TimeoutExpired):
            blob = None
        _PFB_CACHE["cmr10.pfb"] = blob
    return _PFB_CACHE["cmr10.pfb"]


def classify(returncode, log):
    """ok / tfm-rejected / graceful-error / crash for a finished run."""
    if fuzz_run.is_output_flood(returncode):
        return "output-flood"
    if is_crash(returncode, log):
        return "crash"
    if any(hint in (log or "") for hint in TFM_REJECT_HINTS):
        return "tfm-rejected"
    return "ok" if returncode == 0 else "graceful-error"


def run_one(tfm_bytes, candidate, timeout):
    """Run one mutated TFM through the candidate; return (class, rc, log)."""
    workdir = tempfile.mkdtemp(prefix="tfm-fuzz-")
    try:
        with open(os.path.join(workdir, "fuzz.tfm"), "wb") as fh:
            fh.write(tfm_bytes)
        with open(os.path.join(workdir, "job.tex"), "w") as fh:
            fh.write(JOB_TEX)
        pfb = pfb_bytes()
        if pfb:
            try:
                with open(os.path.join(workdir, "cmr10.pfb"), "wb") as fh:
                    fh.write(pfb)
            except OSError:
                pass
        try:
            rc, out = fuzz_run.run_capped(
                [candidate] + fuzz_run.FUZZ_SHELL_ESCAPE_FLAGS
                + ["-fmt=pdftex", "-interaction=nonstopmode", "job.tex"],
                cwd=workdir, env=candidate_env(), timeout=timeout)
            log = out.decode("utf-8", "replace")
        except subprocess.TimeoutExpired as exc:
            out = (exc.stdout or b"") + (exc.stderr or b"")
            return "hang", None, out.decode("utf-8", "replace")
        return classify(rc, log), rc, log
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
    fuzz_run.apply_fsize_limit()
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
