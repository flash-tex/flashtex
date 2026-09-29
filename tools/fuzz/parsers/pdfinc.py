#!/usr/bin/env python3
"""Crash fuzzer for PDF files included as images (``\\pdfximage``). MIT.

Seeds are 4 small PDFs compiled by the reference pdfTeX from tiny
documents at run time (nothing third-party is copied into the repo).
Each iteration mutates one seed (byte-level or PDF-structure-aware) and
runs the job ``\\pdfximage{fuzz.pdf}\\setbox0\\hbox{\\pdfrefximage
\\pdflastximage}\\shipout\\box0 \\bye`` (sometimes ``\\pdfximage page 2``)
through the candidate. Crash contract: the candidate must never panic,
die by signal, or hang; a graceful nonzero exit is fine. Stdlib only.
Deterministic given --seed (every choice via the caller's random.Random).
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

REFERENCE = "/Library/TeX/texbin/pdftex"
CLASSES = ("crash", "hang", "ok", "graceful-error")
BOUNDARIES = (b"0", b"1", b"65535", b"2147483647", b"4294967295")
XREF_OFFSETS = (b"0000000000", b"0000000001", b"9999999999", b"4294967295")

SEED_TEX = {
    "one": "\\shipout\\hbox{hello seed one}\\bye\n",
    "two": "\\shipout\\hbox{page one}\\shipout\\hbox{page two}\\bye\n",
    "halign": "\\hsize=200pt\\shipout\\vbox{\\halign{#\\hfil\\cr a\\cr b\\cr}}\\bye\n",
    "font": "\\font\\f=cmr10 \\f seed four with text\\bye\n",
}

JOB = ("\\pdfximage%s{fuzz.pdf}\\setbox0\\hbox{\\pdfrefximage"
       "\\pdflastximage}\\shipout\\box0 \\bye\n")


def build_seeds(texbin=REFERENCE):
    """Compile SEED_TEX with the reference pdfTeX; return [(name, bytes)]."""
    tmp = tempfile.mkdtemp(prefix="pdfinc-seeds-")
    try:
        env = dict(os.environ, SOURCE_DATE_EPOCH="0")
        seeds = []
        for name, tex in sorted(SEED_TEX.items()):
            with open(os.path.join(tmp, name + ".tex"), "w") as fh:
                fh.write(tex)
            subprocess.run([texbin, "-interaction=nonstopmode", name + ".tex"],
                           cwd=tmp, env=env, capture_output=True, timeout=60)
            try:
                with open(os.path.join(tmp, name + ".pdf"), "rb") as fh:
                    seeds.append((name, fh.read()))
            except OSError:
                continue
        if not seeds:
            raise RuntimeError("reference pdfTeX built no seed PDFs")
        return seeds
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def _one(pattern, data, rng):
    hits = list(pattern.finditer(data))
    return rng.choice(hits) if hits else None


def _m_byteflip(data, rng):
    if not data:
        return None
    buf = bytearray(data)
    for _ in range(1 + rng.randrange(4)):
        i = rng.randrange(len(buf))
        buf[i] ^= 1 << rng.randrange(8)
    return bytes(buf), "byteflip"


def _m_truncate(data, rng):
    if not data:
        return None
    i = rng.randrange(len(data))
    return data[:i], "truncate@%d/%d" % (i, len(data))


def _m_chunkdel(data, rng):
    if len(data) < 2:
        return None
    s = rng.randrange(len(data))
    e = min(len(data), s + 1 + rng.randrange(max(1, len(data) // 4)))
    return data[:s] + data[e:], "chunkdel@%d:%d" % (s, e)


def _m_chunkdup(data, rng):
    if len(data) < 2 or len(data) > 65536:
        return None
    s = rng.randrange(len(data))
    e = min(len(data), s + 1 + rng.randrange(max(1, len(data) // 4)))
    at = rng.randrange(len(data))
    return data[:at] + data[s:e] + data[at:], "chunkdup@%d:%d->%d" % (s, e, at)


def _m_xref(data, rng):
    m = _one(re.compile(rb"(?<=\n)\d{10}(?= \d{5} [nf])"), data, rng)
    if m is None:
        return None
    val = rng.choice(XREF_OFFSETS)
    return data[:m.start()] + val + data[m.end():], "xref->%s" % val.decode()


def _field(pattern, data, rng, values, label):
    m = _one(pattern, data, rng)
    if m is None:
        return None
    val = rng.choice(values)
    return (data[:m.start(1)] + val + data[m.end(1):],
            "%s->%s" % (label, val.decode()))


def _m_startxref(data, rng):
    return _field(re.compile(rb"startxref\s*\r?\n?(\d+)"), data, rng,
                  BOUNDARIES, "startxref")


def _m_length(data, rng):
    return _field(re.compile(rb"/Length\s+(\d+)"), data, rng,
                  BOUNDARIES, "length")


def _m_count(data, rng):
    return _field(re.compile(rb"/Count\s+(\d+)"), data, rng,
                  BOUNDARIES, "count")


def _m_objstm(data, rng):
    return _field(re.compile(rb"/N\s+(\d+)"), data, rng,
                  BOUNDARIES[3:], "objstm-N")


def _m_trailer(data, rng):
    i = data.find(b"trailer")
    if i >= 0:
        j = data.find(b"startxref", i)
        tail = b"" if j < 0 else data[j:]
        return data[:i] + tail, "drop-trailer"
    if b"%%EOF" in data:
        return data.replace(b"%%EOF", b""), "drop-eof"
    return None


def _m_kids(data, rng):
    m = _one(re.compile(rb"/Kids\s*\[[^\]]*\]"), data, rng)
    if m is None:
        return None
    return data[:m.start()] + b"/Kids[0 0 R 0 0 R]" + data[m.end():], \
        "kids-cycle"


def _m_cycle(data, rng):
    objs = list(re.compile(rb"(\d+) 0 obj").finditer(data))
    if not objs:
        return None
    a = rng.choice(objs).group(1)
    bs = [m.group(1) for m in objs if m.group(1) != a] or [a]
    b = rng.choice(bs)
    out = data
    for num, ref in ((a, b), (b, a)):
        tag = num + b" 0 obj"
        i = out.find(tag)
        j = out.find(b"endobj", i)
        if i < 0 or j < 0:
            return None
        ins = b"<< /FzCycle " + ref + b" 0 R >> "
        out = out[:j] + ins + out[j:]
    return out, "cycle-%s-%s" % (a.decode(), b.decode())


def _m_streamcut(data, rng):
    m = _one(re.compile(rb"(?:^|\n)stream\r?\n"), data, rng)
    if m is None:
        return None
    j = data.find(b"endstream", m.end())
    if j <= m.end() + 1:
        return None
    cut = (m.end() + j) // 2
    return data[:cut] + data[j:], "stream-truncate"


MUTATIONS = (_m_byteflip, _m_truncate, _m_chunkdel, _m_chunkdup, _m_xref,
             _m_startxref, _m_trailer, _m_length, _m_count, _m_objstm,
             _m_kids, _m_cycle, _m_streamcut)


def mutate_pdf(data, rng):
    """Apply one mutation; return (new_bytes, description). Deterministic."""
    if not data:
        return data, "noop"
    start = rng.randrange(len(MUTATIONS))
    for k in range(len(MUTATIONS)):
        got = MUTATIONS[(start + k) % len(MUTATIONS)](data, rng)
        if got is not None:
            return got
    return data, "noop"


def panic_location(log):
    if not log or "panicked at" not in log:
        return None
    after = log.split("panicked at", 1)[1]
    m = re.search(r":\d+", after)
    return after[:m.start()].strip() if m else None


def is_crash(returncode, log):
    return (returncode is not None
            and (returncode < 0 or returncode == 101
                 or "panicked at" in (log or "")))


def signature(returncode, log):
    loc = panic_location(log)
    if loc is not None:
        return "panic:" + loc
    if returncode is not None and returncode < 0:
        return "signal:%d" % (-returncode,)
    return "exit:%s" % (returncode,)


def run_case(pdf_bytes, candidate, timeout, page2=False):
    """Run one mutated PDF through the candidate; return (class, rc, log)."""
    tmp = tempfile.mkdtemp(prefix="pdfinc-")
    try:
        with open(os.path.join(tmp, "fuzz.pdf"), "wb") as fh:
            fh.write(pdf_bytes)
        with open(os.path.join(tmp, "job.tex"), "w") as fh:
            fh.write(JOB % (" page 2" if page2 else ""))
        # FLASHTEX_POOL / FLASHTEX_FORMATS come from the caller.
        env = dict(os.environ, SOURCE_DATE_EPOCH="0")
        try:
            proc = subprocess.run(
                [candidate, "-fmt=pdftex", "-interaction=nonstopmode",
                 "job.tex"], cwd=tmp, env=env, capture_output=True,
                timeout=timeout)
            log = (proc.stdout or b"").decode("latin-1") + \
                (proc.stderr or b"").decode("latin-1")
            rc = proc.returncode
        except subprocess.TimeoutExpired as exc:
            out = (exc.stdout or b"") + (exc.stderr or b"")
            return "hang", None, out.decode("latin-1", "replace")
        if is_crash(rc, log):
            return "crash", rc, log
        return ("ok" if rc == 0 else "graceful-error"), rc, log
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def known_signatures(out_dir):
    known = set()
    try:
        for root, _dirs, files in os.walk(out_dir):
            for name in files:
                if not name.endswith(".json"):
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


def run_fuzz(candidate, seeds, out_dir, iterations, seed, timeout):
    """Mutate seeds through the candidate; store crash/hang inputs."""
    rng = random.Random(seed)
    seen = set()
    counts = {c: 0 for c in CLASSES}
    for i in range(iterations):
        name, base = seeds[rng.randrange(len(seeds))]
        pdf, mutation = mutate_pdf(base, rng)
        page2 = rng.randrange(4) == 0
        if page2:
            mutation += "|page2"
        cls, rc, log = run_case(pdf, candidate, timeout, page2)
        counts[cls] += 1
        if cls in ("crash", "hang"):
            sig = "hang" if cls == "hang" else signature(rc, log)
            if sig not in seen and sig not in known_signatures(out_dir):
                seen.add(sig)
                digest = hashlib.sha256(pdf).hexdigest()[:16]
                cls_dir = os.path.join(out_dir, cls)
                os.makedirs(cls_dir, exist_ok=True)
                with open(os.path.join(cls_dir, digest + ".pdf"), "wb") as fh:
                    fh.write(pdf)
                lines = [ln for ln in log.splitlines() if ln.strip()]
                loc = panic_location(log)
                with open(os.path.join(cls_dir, digest + ".json"), "w") as fh:
                    json.dump({"seed": name, "mutation": mutation,
                               "returncode": rc,
                               "last_stderr_line": lines[-1] if lines else "",
                               "panic_location": loc,
                               "signature": sig}, fh, indent=2)
        if (i + 1) % 100 == 0:
            print("pdfinc %d/%d: %s" % (
                i + 1, iterations,
                " ".join("%s=%d" % (c, counts[c]) for c in CLASSES)))
    return counts


def main(argv=None):
    ap = argparse.ArgumentParser(description="pdfTeX image-PDF crash fuzzer")
    ap.add_argument("--candidate", required=True)
    ap.add_argument("--iterations", type=int, default=300)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--out", required=True)
    ap.add_argument("--timeout", type=float, default=10)
    args = ap.parse_args(argv)
    if not (os.path.isfile(args.candidate) or shutil.which(args.candidate)):
        print("error: candidate not found: %s" % args.candidate,
              file=sys.stderr)
        return 2
    seeds = build_seeds()
    # Sanity: the unmutated first seed must work through the reference job.
    cls, rc, _log = run_case(seeds[0][1], REFERENCE, args.timeout)
    print("reference unmutated seed: %s rc=%s" % (cls, rc))
    counts = run_fuzz(args.candidate, seeds, args.out, args.iterations,
                      args.seed, args.timeout)
    print("done: %d iterations: %s" % (
        args.iterations, " ".join("%s=%d" % (c, counts[c]) for c in CLASSES)))
    saved = []
    for cls in ("crash", "hang"):
        d = os.path.join(args.out, cls)
        try:
            saved += [os.path.join(d, f) for f in sorted(os.listdir(d))
                      if f.endswith(".pdf")]
        except OSError:
            pass
    for path in saved:
        print("saved: %s" % path)
    return counts


if __name__ == "__main__":
    result = main()
    sys.exit(0 if isinstance(result, dict) else result)
