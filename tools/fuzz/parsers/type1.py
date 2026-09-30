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

CharStrings-level mutations (cs-operand, cs-operator, cs-subr,
cs-recursion, cs-endchar) decrypt the eexec block (r=55665), parse the
`len RD ... ND|NP` entries, decrypt one charstring (r=4330, lenIV=4),
mutate its code bytes, then re-encrypt and rebuild the PFB segment
lengths so the file stays structurally valid.
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

sys.path.insert(0,
                os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import run as fuzz_run

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
EEXEC_R = 55665
CS_R = 4330
LENIV = 4
C1 = 52845
C2 = 22719
CS_BOUNDARIES = (0, 1, 107, 108, 1131, 1132, -107, -108, -1131, -1132,
                 255, 256, 32767, -32768, 0x7FFFFFFF, -0x80000000)
CS_OPS = {"hsbw": b"\x0d", "rlineto": b"\x05", "rrcurveto": b"\x08",
          "callsubr": b"\x0a", "callothersubr": b"\x0c\x10",
          "div": b"\x0c\x0c", "seac": b"\x0c\x06",
          "closepath": b"\x09", "endchar": b"\x0e"}
HUGE_SUBRS = (100000, 32767, 1000000, -1, -2, -1000,
              0x7FFFFFFF, -0x80000000)


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


def t1_decrypt(data, r=EEXEC_R):
    """Type 1 decrypt (eexec r=55665, charstring r=4330). Exact inverse of
    t1_encrypt, so encrypt(decrypt(x)) == x byte-for-byte."""
    out = bytearray()
    for c in data:
        out.append(c ^ (r >> 8))
        r = ((c + r) * C1 + C2) & 0xFFFF
    return bytes(out)


def t1_encrypt(data, r=EEXEC_R):
    """Type 1 encrypt. Encrypts the bytes as given (callers preserve the
    lenIV random prefix inside the plaintext, so round-trips are exact)."""
    out = bytearray()
    for p in data:
        c = p ^ (r >> 8)
        out.append(c)
        r = ((c + r) * C1 + C2) & 0xFFFF
    return bytes(out)


def encode_num(v):
    """Encode an integer as a Type 1 charstring number (1, 2 or 5 bytes).
    Out-of-int32 values clamp to the int32 extremes."""
    if -107 <= v <= 107:
        return bytes((v + 139,))
    if 108 <= v <= 1131:
        v -= 108
        return bytes(((v >> 8) + 247, v & 0xFF))
    if -1131 <= v <= -108:
        v = -v - 108
        return bytes(((v >> 8) + 251, v & 0xFF))
    v = max(-0x80000000, min(0x7FFFFFFF, v))
    return b"\xff" + v.to_bytes(4, "big", signed=True)


def cs_tokens(code):
    """Split decrypted charstring bytes (sans lenIV) into a list of
    (kind, value, start, end); kind is "num" (value the integer) or "op"
    (value the raw 1-2 operator bytes)."""
    toks = []
    i, n = 0, len(code)
    while i < n:
        b = code[i]
        if b == 255:
            if i + 4 < n:
                toks.append(("num",
                             int.from_bytes(code[i + 1:i + 5], "big",
                                            signed=True), i, i + 5))
                i += 5
            else:
                toks.append(("op", code[i:i + 1], i, i + 1))
                i += 1
        elif 32 <= b <= 246:
            toks.append(("num", b - 139, i, i + 1))
            i += 1
        elif 247 <= b <= 250:
            if i + 1 < n:
                toks.append(("num", (b - 247) * 256 + code[i + 1] + 108,
                             i, i + 2))
                i += 2
            else:
                toks.append(("op", code[i:i + 1], i, i + 1))
                i += 1
        elif 251 <= b <= 254:
            if i + 1 < n:
                toks.append(("num", -((b - 251) * 256 + code[i + 1] + 108),
                             i, i + 2))
                i += 2
            else:
                toks.append(("op", code[i:i + 1], i, i + 1))
                i += 1
        elif b == 12:
            if i + 1 < n:
                toks.append(("op", code[i:i + 2], i, i + 2))
                i += 2
            else:
                toks.append(("op", code[i:i + 1], i, i + 1))
                i += 1
        else:
            toks.append(("op", code[i:i + 1], i, i + 1))
            i += 1
    return toks


_CS_HDR = re.compile(
    rb"(?:/(?P<name>[A-Za-z0-9_.-]+)|dup\s+(?P<subr>\d+))"
    rb"\s+(?P<len>\d+)\s+RD[ \t\r\n]*")
_CS_TAIL = re.compile(rb"[ \t\r\n]*(?P<term>N[DP])")


def find_cs_entries(plain):
    """Find charstring/subr entries in decrypted eexec bytes: every
    `/name len RD <len bytes> ND` or `dup idx len RD <len bytes> NP`
    whose declared length lines up with an ND/NP terminator. Returns a
    list of dicts with hdr start, len span, body span, name/subr and the
    declared length. Bodies are NOT decrypted here."""
    entries = []
    pos, n = 0, len(plain)
    while pos < n:
        m = _CS_HDR.search(plain, pos)
        if m is None:
            break
        try:
            ln = int(m.group("len"))
        except ValueError:
            pos = m.start() + 1
            continue
        bs, be = m.end(), m.end() + ln
        if ln > 65536 or be > n or not _CS_TAIL.match(plain[be:be + 4]):
            pos = m.start() + 1
            continue
        entries.append({"hs": m.start(), "lspan": m.span("len"),
                        "body": (bs, be), "len": ln,
                        "name": m.group("name"), "subr": m.group("subr")})
        pos = be
    return entries


def mutate_cs_code(code, rng, kind):
    """Mutate decrypted charstring bytes (sans lenIV); return
    (new_code, detail) or None when the kind does not apply."""
    toks = cs_tokens(code)
    nums = [t for t in toks if t[0] == "num"]
    ops = [t for t in toks if t[0] == "op"]
    if kind == "operand":
        if not nums:
            return None
        _k, _v, s, e = rng.choice(nums)
        v = rng.choice(CS_BOUNDARIES)
        return code[:s] + encode_num(v) + code[e:], "num->%d" % v
    if kind == "operator":
        name = rng.choice(sorted(CS_OPS))
        if ops:
            _k, _v, s, e = rng.choice(ops)
            return code[:s] + CS_OPS[name] + code[e:], "op->" + name
        at = rng.randrange(len(code) + 1)
        return code[:at] + CS_OPS[name] + code[at:], "op-insert->" + name
    if kind == "subr":
        idx = rng.choice(HUGE_SUBRS)
        seq = encode_num(idx) + CS_OPS["callsubr"]
        calls = [t for t in ops
                 if code[t[2]:t[3]] in (b"\x0a", b"\x0c\x10")]
        if calls:
            _k, _v, cs, _ce = rng.choice(calls)
            prev = [t for t in nums if t[3] <= cs]
            if prev:
                _k, _v, ns, _ne = prev[-1]
                return (code[:ns] + encode_num(idx) + code[cs:],
                        "callsubr->%d" % idx)
            return code[:cs] + seq + code[cs:], "callsubr->%d" % idx
        at = rng.randrange(len(code) + 1)
        return code[:at] + seq + code[at:], "callsubr-insert->%d" % idx
    if kind == "endchar":
        ends = [t for t in ops if code[t[2]:t[3]] == b"\x0e"]
        if ends:
            _k, _v, s, e = ends[-1]
            return code[:s] + code[e:], "dropped-endchar"
        if code:
            return code[:-1], "dropped-last-byte"
        return None
    raise ValueError("unknown charstring mutation kind: %r" % (kind,))


def _m_charstring(data, rng, segs, kind):
    """Decrypt the eexec block, apply one KIND mutation to a single
    charstring (lenIV prefix preserved), then re-encrypt and rebuild the
    PFB segment lengths so the container stays structurally valid."""
    twos = [s for s in segs if s[1] == 2]
    if not twos:
        return None
    off, _typ, ln = rng.choice(twos)
    start = off + 6
    end = min(start + ln, len(data))
    if end <= start:
        return None
    plain = t1_decrypt(data[start:end], EEXEC_R)
    order = find_cs_entries(plain)
    if not order:
        return None
    rng.shuffle(order)
    for ent in order:
        bs, be = ent["body"]
        body = plain[bs:be]
        if len(body) <= LENIV:
            continue
        prefix, code = body[:LENIV], body[LENIV:]
        if kind == "recursion":
            if ent["subr"] is None:
                continue
            idx = int(ent["subr"])
            new_code = encode_num(idx) + CS_OPS["callsubr"]
            detail = "self-call->%d" % idx
        else:
            got = mutate_cs_code(code, rng, kind)
            if got is None:
                continue
            new_code, detail = got
        new_body = t1_encrypt(prefix + new_code, CS_R)
        ls, le = ent["lspan"]
        new_plain = (plain[:ent["hs"]] + plain[ent["hs"]:ls]
                     + str(len(new_body)).encode() + plain[le:bs]
                     + new_body + plain[be:])
        new_seg = t1_encrypt(new_plain, EEXEC_R)
        out = bytearray(data)
        out[off + 2:off + 6] = len(new_seg).to_bytes(4, "little")
        raw = ent["name"] if ent["name"] is not None else \
            b"subr#" + ent["subr"]
        label = raw.decode("ascii", "replace")
        return (bytes(out[:off + 6]) + new_seg + data[end:],
                "cs-%s@%s:%s" % (kind, label, detail))
    return None


def _m_cs_operand(data, rng, segs):
    return _m_charstring(data, rng, segs, "operand")


def _m_cs_operator(data, rng, segs):
    return _m_charstring(data, rng, segs, "operator")


def _m_cs_subr(data, rng, segs):
    return _m_charstring(data, rng, segs, "subr")


def _m_cs_recursion(data, rng, segs):
    return _m_charstring(data, rng, segs, "recursion")


def _m_cs_endchar(data, rng, segs):
    return _m_charstring(data, rng, segs, "endchar")


MUTATIONS = (("flip", _m_flip), ("trunc", _m_trunc),
             ("delete", _m_delete), ("dup", _m_dup),
             ("segtype", _m_segtype), ("seglen", _m_seglen),
             ("eexec-trunc", _m_eexec),
             ("cs-operand", _m_cs_operand),
             ("cs-operator", _m_cs_operator),
             ("cs-subr", _m_cs_subr),
             ("cs-recursion", _m_cs_recursion),
             ("cs-endchar", _m_cs_endchar))


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
    """Panic site as "<file>:<line>"; shared spelling, see run.py."""
    return fuzz_run.panic_location(log)


def signature(cls, returncode, log, stderr=""):
    if cls == "hang":
        return "hang"
    return fuzz_run.crash_signature(returncode, log, stderr)


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
            sig = signature(cls, rc, log, err)
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
