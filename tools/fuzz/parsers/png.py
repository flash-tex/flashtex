#!/usr/bin/env python3
"""PNG image fuzzer for the FlashTeX candidate pdfTeX engine. MIT, stdlib only.

Each iteration mutates a seed PNG, embeds it in a minimal ``\\pdfximage`` job
(``\\pdfximage{fuzz.png}\\setbox0\\hbox{\\pdfrefximage\\pdflastximage}``
``\\shipout\\box0 \\bye``) and runs only the candidate. Crash contract: the
candidate must never panic, die on a signal, or hang, whatever the input;
returncode < 0, returncode 101, or the words ``panicked at`` in stdout/stderr
is a crash, exceeding --timeout is a hang, returncode 0 is ok, any other
nonzero exit is a graceful-error. Deterministic given --seed: every choice
goes through one random.Random instance.

Seeds are read from the TeX Live tree at run time (never copied into the
repo) plus two tiny PNGs generated here with zlib/struct.
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
import zlib

SIG = b"\x89PNG\r\n\x1a\n"
JOB = ("\\pdfximage{fuzz.png}\\setbox0\\hbox{\\pdfrefximage\\pdflastximage}"
       "\\shipout\\box0 \\bye\n")
CLASSES = ("crash", "hang", "ok", "graceful-error")
STORE = ("crash", "hang")
BOUNDARY = (0, 1, 0xFFFF, 0x7FFFFFFF, 0xFFFFFFFF)
# (name, path under TEXMFDIST): colour type 0 (gray) x2, colour type 3
# (palette) x1; the mwe tree ships no other colour types and no interlaced
# files, so interlace and types 2/4/6 are covered by ihdr mutations.
SEED_FILES = (("mwe-gray", "tex/latex/mwe/example-image.png"),
              ("mwe-gray-a", "tex/latex/mwe/example-image-a.png"),
              ("mwe-palette", "tex/latex/mwe/example-grid-100x100pt.png"))


def chunk_raw(typ, payload):
    return (struct.pack(">I", len(payload)) + typ + payload
            + struct.pack(">I", zlib.crc32(typ + payload) & 0xFFFFFFFF))


def build(chunks):
    """Assemble a PNG from [(type, payload)] with fresh CRCs."""
    return SIG + b"".join(chunk_raw(t, p) for t, p in chunks)


def parse(data):
    """Split raw bytes into [(type, payload, len_off, crc_off)]; None if the
    framing is broken (bad signature, short read, or truncated chunk)."""
    if data[:8] != SIG:
        return None
    out, pos = [], 8
    while pos + 8 <= len(data):
        ln = struct.unpack(">I", data[pos:pos + 4])[0]
        typ = data[pos + 4:pos + 8]
        end = pos + 12 + ln
        if end > len(data):
            return None
        out.append((typ, data[pos + 8:pos + 8 + ln], pos, pos + 8 + ln))
        pos = end
        if typ == b"IEND":
            break
    return out


def make_gray(w=4, h=4, val=0x80, interlace=0):
    """Tiny colour-type-0 PNG. Only 1x1 is used with interlace=1 (a single
    Adam7 pass-1 pixel), which real pdfTeX accepts."""
    rows = b"".join(b"\x00" + bytes([val]) * w for _ in range(h))
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 0, 0, 0, interlace)
    return build([(b"IHDR", ihdr), (b"IDAT", zlib.compress(rows)),
                  (b"IEND", b"")])


def make_palette():
    """Tiny 2x2 colour-type-3 PNG with a 3-entry PLTE."""
    plte = bytes([255, 0, 0, 0, 255, 0, 0, 0, 255])
    rows = b"\x00\x00\x01" + b"\x00\x02\x00"
    ihdr = struct.pack(">IIBBBBB", 2, 2, 8, 3, 0, 0, 0)
    return build([(b"IHDR", ihdr), (b"PLTE", plte),
                  (b"IDAT", zlib.compress(rows)), (b"IEND", b"")])


def texmf_dist():
    try:
        out = subprocess.run(["kpsewhich", "-var-value", "TEXMFDIST"],
                             capture_output=True, text=True, timeout=10)
        if out.returncode == 0 and out.stdout.strip():
            return out.stdout.strip()
    except OSError:
        pass
    return None


def load_seeds():
    """[(name, png_bytes)]: real mwe seeds when the TeX tree is present,
    always the two synthetic seeds."""
    seeds = []
    base = texmf_dist()
    if base:
        for name, rel in SEED_FILES:
            try:
                with open(os.path.join(base, rel), "rb") as fh:
                    seeds.append((name, fh.read()))
            except OSError:
                continue
    seeds.append(("synth-gray-interlaced", make_gray(1, 1, 0x80, 1)))
    seeds.append(("synth-palette", make_palette()))
    return [(n, d) for n, d in seeds if d[:8] == SIG]


def _extra_chunk(rng):
    kind = rng.choice(("tEXt", "gAMA", "cHRM", "sRGB", "iCCP"))
    if kind == "tEXt":
        payload = b"Comment\x00" + b"A" * 100000
    elif kind == "gAMA":
        payload = struct.pack(">I", 45455)
    elif kind == "cHRM":
        payload = struct.pack(">8I", 31270, 32900, 64000, 33000,
                              30000, 60000, 15000, 6000)
    elif kind == "sRGB":
        payload = b"\x00"
    else:
        payload = b"prof\x00\x00" + zlib.compress(b"Z" * 100000)
    return kind, chunk_raw(kind.encode("latin1"), payload)[8:]


def mutate_png(data, rng):
    """Apply ONE mutation to raw PNG bytes; return (new_bytes, description).
    Chunk-aware ops need valid framing, otherwise only flip/trunc apply."""
    chunks = parse(data)
    ops = ["flip", "trunc"]
    if chunks:
        ops += ["del-chunk", "dup-chunk", "length", "crc", "add-chunk"]
        types = [t for t, _, _, _ in chunks]
        if chunks[0][0] == b"IHDR" and len(chunks[0][1]) == 13:
            ops.append("ihdr")
        if b"IEND" in types:
            ops.append("drop-iend")
        if b"IDAT" in types:
            ops.append("reorder-idat")
    op = rng.choice(ops)
    if op == "flip":
        buf = bytearray(data)
        for _ in range(rng.randint(1, 8)):
            i = rng.randrange(len(buf))
            buf[i] = (buf[i] + rng.randint(1, 255)) % 256
        return bytes(buf), "flip"
    if op == "trunc":
        i = rng.randrange(1, len(data)) if len(data) > 1 else 0
        return data[:i], "trunc@%d" % i
    if op == "del-chunk":
        t, _, start, crc = chunks[rng.randrange(len(chunks))]
        return data[:start] + data[crc + 4:], \
            "del-%s" % t.decode("latin1")
    if op == "dup-chunk":
        t, _, start, crc = chunks[rng.randrange(len(chunks))]
        raw = data[start:crc + 4]
        return data[:crc + 4] + raw + data[crc + 4:], \
            "dup-%s" % t.decode("latin1")
    if op == "length":
        t, _, start, _ = chunks[rng.randrange(len(chunks))]
        v = rng.choice(BOUNDARY)
        return data[:start] + struct.pack(">I", v) + data[start + 4:], \
            "len-%s=%d" % (t.decode("latin1"), v)
    if op == "crc":
        t, _, _, crc = chunks[rng.randrange(len(chunks))]
        mode = rng.choice(("corrupt", "zero", "drop"))
        if mode == "drop":
            return data[:crc] + data[crc + 4:], \
                "crc-%s-drop" % t.decode("latin1")
        repl = (b"\x00\x00\x00\x00" if mode == "zero"
                else bytes(rng.randrange(256) for _ in range(4)))
        return data[:crc] + repl + data[crc + 4:], \
            "crc-%s-%s" % (t.decode("latin1"), mode)
    if op == "drop-iend":
        for t, _, start, crc in reversed(chunks):
            if t == b"IEND":
                return data[:start] + data[crc + 4:], "drop-iend"
    if op == "reorder-idat":
        spans = [(s, c + 4) for t, _, s, c in chunks if t == b"IDAT"]
        if len(spans) >= 2:
            (s1, e1), (s2, e2) = spans[0], spans[1]
            ra, rb = data[s1:e1], data[s2:e2]
            return data[:s1] + rb + data[e1:s2] + ra + data[e2:], \
                "reorder-idat"
        t, p, _, _ = next(c for c in chunks if c[0] == b"IDAT")
        k = max(1, len(p) // 2)
        lst = [(x, y) for x, y, _, _ in chunks]
        idx = next(i for i, (x, _) in enumerate(lst) if x == b"IDAT")
        lst[idx:idx + 1] = [(b"IDAT", p[:k]), (b"IDAT", p[k:])]
        return build(lst), "split-idat"
    if op == "add-chunk":
        kind, raw = _extra_chunk(rng)
        iends = [c for c in chunks if c[0] == b"IEND"]
        at = iends[0][2] if iends else len(data)
        return data[:at] + raw + data[at:], "add-%s" % kind
    # ihdr: rebuild with a new width/height/depth/type/interlace, valid CRC
    # so the engine reaches semantic checks instead of stopping at CRC.
    w, h, bd, ct, cp, fl, il = struct.unpack(">IIBBBBB", chunks[0][1])
    field = rng.choice(("w", "h", "bd", "ct", "il"))
    if field in ("w", "h"):
        v = rng.choice(BOUNDARY)
    elif field == "bd":
        v = rng.choice((0, 1, 2, 4, 8, 16, 17))
    elif field == "ct":
        v = rng.choice((0, 2, 3, 4, 6, 7))
    else:
        v = rng.choice((0, 1, 2))
    vals = {"w": w, "h": h, "bd": bd, "ct": ct, "il": il}
    vals[field] = v
    ihdr = struct.pack(">IIBBBBB", vals["w"], vals["h"], vals["bd"],
                       vals["ct"], cp, fl, vals["il"])
    lst = [(b"IHDR", ihdr)] + [(x, y) for x, y, _, _ in chunks[1:]]
    return build(lst), "ihdr-%s=%d" % (field, v)


def is_crash(rc, out):
    return (rc is not None
            and (rc < 0 or rc == 101 or b"panicked at" in (out or b"")))


def classify(rc, out, timed_out=False):
    if timed_out:
        return "hang"
    if is_crash(rc, out):
        return "crash"
    return "ok" if rc == 0 else "graceful-error"


def panic_location(out):
    """Text after 'panicked at' up to the first colon-number pair
    (e.g. 'src/png/idat.rs:42'); None when absent."""
    text = (out or b"").decode("utf-8", "replace")
    if "panicked at" not in text:
        return None
    after = text.split("panicked at", 1)[1]
    m = re.search(r":\d+", after)
    return after[:m.start()].strip() if m else None


def signature(cls, rc, out):
    """Dedupe key: panic location or signal/exit for crashes, 'hang'."""
    if cls == "hang":
        return "hang"
    if cls == "crash":
        loc = panic_location(out)
        if loc is not None:
            return "panic:" + loc
        if rc is not None and rc < 0:
            return "signal:%d" % (-rc,)
        return "exit:%s" % (rc,)
    return None


def last_line(out):
    lines = (out or b"").decode("utf-8", "replace").strip().splitlines()
    return lines[-1][-200:] if lines else ""


def run_once(png, candidate, timeout):
    """Write fuzz.png + job.tex to a temp dir, run the candidate once;
    return (class, returncode, combined_output)."""
    tmp = tempfile.mkdtemp(prefix="pngfuzz-")
    try:
        with open(os.path.join(tmp, "fuzz.png"), "wb") as fh:
            fh.write(png)
        with open(os.path.join(tmp, "job.tex"), "w") as fh:
            fh.write(JOB)
        env = dict(os.environ)
        # FLASHTEX_POOL / FLASHTEX_FORMATS must be exported by the caller;
        # without them every run fails and looks like graceful-error.
        env.setdefault("SOURCE_DATE_EPOCH", "0")
        try:
            proc = subprocess.run(
                [candidate, "-fmt=pdftex", "-interaction=nonstopmode",
                 "job.tex"], cwd=tmp, stdout=subprocess.PIPE,
                stderr=subprocess.PIPE, timeout=timeout, env=env)
            out = (proc.stdout or b"") + b"\n" + (proc.stderr or b"")
            return classify(proc.returncode, out), proc.returncode, out
        except subprocess.TimeoutExpired:
            return "hang", None, b""
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def load_known(out_dir):
    """Signatures already stored under OUT (per-case .json sidecars)."""
    known = set()
    for root, _dirs, files in os.walk(out_dir):
        for name in files:
            if not name.endswith(".json"):
                continue
            try:
                with open(os.path.join(root, name)) as fh:
                    info = json.load(fh)
            except (OSError, ValueError):
                continue
            if isinstance(info, dict) and info.get("signature"):
                known.add(info["signature"])
    return known


def run_fuzz(candidate, out_dir, iterations, seed, timeout, seeds=None):
    rng = random.Random(seed)
    if seeds is None:
        seeds = load_seeds()
    if not seeds:
        raise ValueError("no PNG seeds available")
    known = load_known(out_dir)
    seen = set()
    counts = {c: 0 for c in CLASSES}
    for i in range(iterations):
        name, base = seeds[rng.randrange(len(seeds))]
        mutated, desc = mutate_png(base, rng)
        cls, rc, out = run_once(mutated, candidate, timeout)
        counts[cls] += 1
        if cls in STORE:
            sig = signature(cls, rc, out)
            if sig not in seen and sig not in known:
                seen.add(sig)
                digest = hashlib.sha256(mutated).hexdigest()[:16]
                cls_dir = os.path.join(out_dir, cls)
                os.makedirs(cls_dir, exist_ok=True)
                with open(os.path.join(cls_dir, digest + ".png"),
                          "wb") as fh:
                    fh.write(mutated)
                with open(os.path.join(cls_dir, digest + ".json"),
                          "w") as fh:
                    json.dump({"seed": name, "mutation": desc,
                               "returncode": rc,
                               "last_output_line": last_line(out),
                               "panic_location": panic_location(out)
                               if cls == "crash" else None,
                               "signature": sig}, fh, indent=2)
        if (i + 1) % 100 == 0:
            print("pngfuzz %d/%d: %s"
                  % (i + 1, iterations,
                     " ".join("%s=%d" % (c, counts[c]) for c in CLASSES)))
    print("done: %d iterations: %s"
          % (iterations, " ".join("%s=%d" % (c, counts[c]) for c in CLASSES)))
    return counts


def main(argv=None):
    ap = argparse.ArgumentParser(description="PNG image crash fuzzer")
    ap.add_argument("--candidate", required=True,
                    help="candidate engine binary")
    ap.add_argument("--iterations", type=int, required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--out", required=True, help="output directory")
    ap.add_argument("--timeout", type=float, default=10,
                    help="per-run timeout in seconds")
    args = ap.parse_args(argv)
    if not (os.path.isfile(args.candidate)
            or shutil.which(args.candidate)):
        print("error: candidate not found: %s" % args.candidate,
              file=sys.stderr)
        return 2
    if args.iterations < 0:
        print("error: --iterations must be >= 0", file=sys.stderr)
        return 2
    counts = run_fuzz(args.candidate, args.out, args.iterations,
                      args.seed, args.timeout)
    return counts


if __name__ == "__main__":
    result = main()
    sys.exit(0 if isinstance(result, dict) else result)
