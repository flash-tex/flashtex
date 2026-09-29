#!/usr/bin/env python3
"""JPEG-image crash fuzzer for the FlashTeX engine. MIT licensed.

Mutates real JPEG seeds (TeX Live mwe images found via kpsewhich at run
time, or a generated 1x1 baseline fallback) and runs each through the
candidate pdfTeX with the job::

    \\pdfximage{fuzz.jpg}\\setbox0\\hbox{\\pdfrefximage\\pdflastximage}
    \\shipout\\box0 \\bye

Crash contract: any panic (exit 101 / "panicked at"), death by signal,
or hang is a bug; a graceful non-zero exit is fine. Stdlib only.
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

TEX_JOB = ("\\pdfximage{fuzz.jpg}\n\\setbox0\\hbox"
           "{\\pdfrefximage\\pdflastximage}\n\\shipout\\box0\n\\bye\n")
CLASSES = ("crash", "hang", "ok", "graceful-error")
STORE = ("crash", "hang")
SEED_NAMES = ("example-image.jpg", "example-image-a.jpg",
              "example-image-b.jpg", "example-image-c.jpg")
BOUNDARIES = (0, 1, 0xFFFF, 0x7FFFFFFF, 0xFFFFFFFF)
SOF = {0xC0, 0xC1, 0xC2, 0xC3, 0xC5, 0xC6, 0xC7,
       0xC9, 0xCA, 0xCB, 0xCD, 0xCE, 0xCF}
TABLES = {0xC4: "DHT", 0xDB: "DQT"}  # + SOF droppable/duplicable below
NOLEN = {0xD8, 0xD9, 0x01} | set(range(0xD0, 0xD8))



def parse_segments(data):
    """Split JPEG headers into (marker, off, lenoff, length) entries.

    lenoff/length are None for standalone markers (SOI/EOI/RSTn) and for
    the "scan" pseudo-entry covering entropy-coded data after SOS.
    Stops at the first inconsistency; never raises.
    """
    segs = []
    if len(data) < 2 or data[0] != 0xFF or data[1] != 0xD8:
        return segs
    i, n = 2, len(data)
    while i + 1 < n:
        if data[i] != 0xFF:
            segs.append(("scan", i, None, None))
            break
        j = i + 1
        while j < n and data[j] == 0xFF:
            j += 1
        if j >= n:
            break
        m = data[j]
        if m == 0x00:  # stuffed byte inside scan data
            segs.append(("scan", i, None, None))
            break
        if m in NOLEN:
            segs.append((m, i, None, None))
            i = j + 1
            if m == 0xD9:
                break
            continue
        if j + 2 >= n:
            break
        ln = (data[j + 1] << 8) + data[j + 2]
        segs.append((m, i, j + 1, ln))
        if m == 0xDA:  # SOS: everything after is scan data
            segs.append(("scan", j + 1 + ln, None, None))
            break
        if ln < 2:
            break
        i = j + 1 + ln
    return segs


def _seg_span(seg, total):
    m, off, lenoff, ln = seg
    if lenoff is None:
        return (off, min(off + 2, total))
    return (off, min(lenoff + max(ln, 2), total))


def _put16(data, off, val):
    return data[:off] + (val & 0xFFFF).to_bytes(2, "big") + data[off + 2:]


def mutate(data, rng):
    """Apply ONE mutation to JPEG bytes; return (new_bytes, description).

    Deterministic: every choice goes through rng. Ops mix raw edits
    (byte flips, truncation, segment delete/duplicate) with marker-aware
    edits (SOF precision/dims/component-count/kind, segment lengths,
    huge APP, EOI truncation, SOF/DHT/DQT drop/duplicate).
    """
    if len(data) < 4:
        return bytes(data), "noop-too-small"
    segs = parse_segments(data)
    op = rng.randrange(10)
    lengthwise = [s for n, s in enumerate(segs)
                  if n > 0 and s[0] != "scan" and s[2] is not None
                  and s[3] is not None and s[3] >= 2]
    if op == 0:  # byte flips
        out = bytearray(data)
        for _ in range(rng.randint(1, 8)):
            p = rng.randrange(len(out))
            if rng.random() < 0.5:
                out[p] ^= 1 << rng.randrange(8)
            else:
                out[p] = rng.randrange(256)
        return bytes(out), "flip-bytes"
    if op == 1:  # truncation at a random offset
        p = rng.randrange(len(data))
        return data[:p], "trunc@%d/%d" % (p, len(data))
    if op in (2, 3):  # chunk deletion / duplication
        if not lengthwise:
            return mutate_flip_only(data, rng)
        seg = lengthwise[rng.randrange(len(lengthwise))]
        m, off = seg[0], seg[1]
        a, b = _seg_span(seg, len(data))
        if op == 2:
            return data[:a] + data[b:], "del-seg:FF%02X@%d" % (m, off)
        return data[:b] + data[a:b] + data[b:], "dup-seg:FF%02X@%d" % (m, off)
    if op == 4:  # SOF field edits (baseline<->progressive kind included)
        sofs = [s for s in lengthwise if s[0] in SOF and s[3] >= 8]
        if not sofs:
            return mutate_flip_only(data, rng)
        m, off, lenoff, _ = sofs[rng.randrange(len(sofs))]
        p = lenoff + 2  # precision, height(2), width(2), ncomp
        field = rng.randrange(5)
        if field == 0:
            v = rng.choice((0, 1, 8, 16, 0xFF))
            return (data[:p] + bytes([v]) + data[p + 1:],
                    "sof-prec=%d" % v)
        if field in (1, 2):
            v = rng.choice(BOUNDARIES)
            q = p + 1 if field == 1 else p + 3
            return _put16(data, q, v), "sof-%s=0x%X" % (
                "height" if field == 1 else "width", v)
        if field == 3:
            v = rng.choice((0, 1, 2, 3, 4, 5, 0xFF))
            return (data[:p + 5] + bytes([v]) + data[p + 6:],
                    "sof-ncomp=%d" % v)
        v = 0xC2 if m == 0xC0 else 0xC0
        return (data[:off + 1] + bytes([v]) + data[off + 2:],
                "sof-kind:FF%02X->FF%02X" % (m, v))
    if op == 5:  # segment length -> boundary value
        if not lengthwise:
            return mutate_flip_only(data, rng)
        m, _, lenoff, _ = lengthwise[rng.randrange(len(lengthwise))]
        v = rng.choice(BOUNDARIES)
        return _put16(data, lenoff, v), "seglen:FF%02X=0x%X" % (m, v)
    if op == 6:  # APP marker with a huge declared length, tiny payload
        pay = bytes(rng.randrange(256) for _ in range(rng.randint(0, 16)))
        ins = b"\xff\xe0\xff\xff" + pay
        return data[:2] + ins + data[2:], "app-huge-len:%d" % len(pay)
    if op == 7:  # truncate before EOI (drop EOI plus up to 64 bytes)
        cut = rng.randint(2, 66)
        p = max(0, len(data) - cut)
        if data[-2:] == b"\xff\xd9":
            return data[:p], "trunc-before-eoi:%d" % (len(data) - p)
        q = rng.randrange(len(data))
        return data[:q], "trunc@%d/%d" % (q, len(data))
    # op 8/9: drop or duplicate a SOF/DHT/DQT segment specifically
    targets = [s for s in lengthwise
               if s[0] in SOF or s[0] in TABLES]
    if not targets:
        return mutate_flip_only(data, rng)
    m, off, lenoff, ln = targets[rng.randrange(len(targets))]
    a, b = _seg_span((m, off, lenoff, ln), len(data))
    if op == 8:
        label = TABLES.get(m, "SOF")
        return data[:a] + data[b:], "drop-%s@%d" % (label, off)
    label = TABLES.get(m, "SOF")
    return data[:b] + data[a:b] + data[b:], "dup-%s@%d" % (label, off)


def mutate_flip_only(data, rng):
    out = bytearray(data)
    for _ in range(rng.randint(1, 8)):
        p = rng.randrange(len(out))
        out[p] ^= 1 << rng.randrange(8)
    return bytes(out), "flip-bytes-fallback"


def generate_minimal_jpeg():
    """1x1 baseline JPEG (SOI/APP0/DQT/SOF0/DHT/SOS/EOI); rc=0 on pdfTeX."""
    dqt = bytes([16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58,
                 60, 55, 14, 13, 16, 24, 40, 57, 69, 56, 14, 17, 22, 29,
                 51, 87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35,
                 55, 64, 81, 104, 113, 92, 49, 64, 78, 87, 103, 121, 120,
                 101, 72, 92, 95, 98, 112, 100, 103, 99])
    dht = bytes([0, 1, 5, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0,
                 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11])
    return (b"\xff\xd8"
            b"\xff\xe0\x00\x10JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00"
            b"\xff\xdb\x00\x43\x00" + dqt +
            b"\xff\xc0\x00\x0b\x08\x00\x01\x00\x01\x01\x01\x11\x00"
            b"\xff\xc4\x00\x1f\x00" + dht +
            b"\xff\xda\x00\x08\x01\x01\x00\x00\x3f\x00\x00\xff\xd9")


def load_seeds():
    """Read seed JPEGs from the TeX Live tree (kpsewhich) at run time."""
    out = []
    kpse = shutil.which("kpsewhich")
    for name in SEED_NAMES:
        path = None
        if kpse:
            try:
                run = subprocess.run([kpse, name], capture_output=True,
                                     text=True, timeout=10)
                first = run.stdout.strip().splitlines()
                path = first[0] if first and first[0] else None
            except (OSError, subprocess.TimeoutExpired):
                path = None
        if path:
            try:
                with open(path, "rb") as fh:
                    out.append((name, fh.read()))
            except OSError:
                continue
    if not out:
        out.append(("generated-minimal.jpg", generate_minimal_jpeg()))
    return out


def is_crash(returncode, output):
    return (returncode is not None
            and (returncode < 0 or returncode == 101
                 or "panicked at" in (output or "")))


def panic_location(output):
    if not output or "panicked at" not in output:
        return None
    after = output.split("panicked at", 1)[1]
    m = re.search(r":\d+", after)
    if not m:
        return None
    return after[:m.start()].strip()


def classify(returncode, output, timed_out):
    if timed_out:
        return "hang"
    if is_crash(returncode, output):
        return "crash"
    if returncode == 0:
        return "ok"
    return "graceful-error"


def signature(cls, returncode, output):
    if cls == "crash":
        loc = panic_location(output)
        if loc is not None:
            return "panic:" + loc
        if returncode is not None and returncode < 0:
            return "signal:%d" % (-returncode,)
        return "exit:%s" % (returncode,)
    if cls == "hang":
        return "timeout"
    return None


def run_one(data, candidate, timeout):
    """Write fuzz.jpg + job tex, run the candidate once.

    Returns (class, returncode, output, last_stderr_line, panic_location).
    """
    work = tempfile.mkdtemp(prefix="jpeg-fuzz-")
    try:
        with open(os.path.join(work, "fuzz.jpg"), "wb") as fh:
            fh.write(data)
        with open(os.path.join(work, "job.tex"), "w") as fh:
            fh.write(TEX_JOB)
        env = dict(os.environ)
        # FLASHTEX_POOL / FLASHTEX_FORMATS come from the caller.
        env["SOURCE_DATE_EPOCH"] = "0"
        try:
            proc = subprocess.run(
                [candidate, "-fmt=pdftex", "-interaction=nonstopmode",
                 "job.tex"], cwd=work, env=env, stdout=subprocess.PIPE,
                stderr=subprocess.PIPE, text=True, timeout=timeout)
            rc, timed = proc.returncode, False
            err_lines = (proc.stderr or "").strip().splitlines()
            out = (proc.stdout or "") + (proc.stderr or "")
        except subprocess.TimeoutExpired as exc:
            rc, timed = None, True
            err = (exc.stderr or b"")
            if isinstance(err, bytes):
                err = err.decode("utf-8", "replace")
            err_lines = err.strip().splitlines()
            so = exc.stdout or b""
            out = (so.decode("utf-8", "replace")
                   if isinstance(so, bytes) else (so or "")) + err
        last = err_lines[-1] if err_lines else ""
        return classify(rc, out, timed), rc, out, last, panic_location(out)
    finally:
        shutil.rmtree(work, ignore_errors=True)


def load_known_signatures(out_dir):
    known = set()
    try:
        with open(os.path.join(out_dir, "signatures.json")) as fh:
            data = json.load(fh)
        if isinstance(data, dict):
            known.update(data.keys())
    except (OSError, ValueError):
        pass
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


def run_fuzz(candidate, out_dir, iterations, seed, timeout):
    rng = random.Random(seed)
    seeds = load_seeds()
    known = load_known_signatures(out_dir)
    seen, sig_counts = set(), {}
    counts = {c: 0 for c in CLASSES}
    for i in range(iterations):
        name, base = seeds[rng.randrange(len(seeds))]
        data, mutation = mutate(base, rng)
        cls, rc, out, last, _ploc = run_one(data, candidate, timeout)
        counts[cls] += 1
        if cls in STORE:
            sig = signature(cls, rc, out)
            if sig is not None:
                sig_counts[sig] = sig_counts.get(sig, 0) + 1
            if sig is None or (sig not in seen and sig not in known):
                if sig is not None:
                    seen.add(sig)
                digest = hashlib.sha256(data).hexdigest()[:16]
                cls_dir = os.path.join(out_dir, cls)
                os.makedirs(cls_dir, exist_ok=True)
                with open(os.path.join(cls_dir, digest + ".jpg"), "wb") as fh:
                    fh.write(data)
                with open(os.path.join(cls_dir, digest + ".json"), "w") as fh:
                    json.dump({"iteration": i, "seed": name,
                               "mutation": mutation, "returncode": rc,
                               "last_stderr_line": last,
                               "panic_location": panic_location(out),
                               "signature": sig}, fh, indent=2)
        if (i + 1) % 100 == 0:
            print("jpeg-fuzz %d/%d: %s"
                  % (i + 1, iterations,
                     " ".join("%s=%d" % (c, counts[c]) for c in CLASSES)))
    if sig_counts:
        try:
            path = os.path.join(out_dir, "signatures.json")
            merged = dict(sig_counts)
            try:
                with open(path) as fh:
                    old = json.load(fh)
                if isinstance(old, dict):
                    for key, val in old.items():
                        merged[key] = merged.get(key, 0) + val
            except (OSError, ValueError):
                pass
            os.makedirs(out_dir, exist_ok=True)
            with open(path, "w") as fh:
                json.dump(merged, fh, indent=2, sort_keys=True)
        except OSError:
            pass
    return counts


def main(argv=None):
    ap = argparse.ArgumentParser(description="JPEG crash fuzzer")
    ap.add_argument("--candidate", required=True)
    ap.add_argument("--iterations", type=int, default=200)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--out", default="jpeg-fuzz-out")
    ap.add_argument("--timeout", type=float, default=15.0,
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
    print("done: %d iterations: %s"
          % (args.iterations,
             " ".join("%s=%d" % (c, counts[c]) for c in CLASSES)))
    return counts


if __name__ == "__main__":
    result = main()
    sys.exit(0 if isinstance(result, dict) else result)
