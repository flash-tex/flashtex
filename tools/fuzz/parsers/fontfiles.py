#!/usr/bin/env python3
"""Virtual font, font map and encoding file fuzzer for the FlashTeX
candidate engine. MIT.

A project can carry its own .vf, .map and .enc files, and the engine
reads them from the project folder before the TeX tree (lane
TRANSFER-AUDIT, sandbox finding 5). Each iteration writes one mutated file
of the chosen --kind into a fresh job folder and runs the candidate on a
job that loads it:

  vf   fuzz.vf (a mutated ptmr8t.vf / pplr8t.vf, or a synthetic one) with
       fuzz.tfm (the seed's own TFM): ``\\font\\x=fuzz \\x Az\\bye``.
       VF-aware edits: the font a fnt_def names set to ``fuzz`` itself (a
       virtual font that refers to itself), fnt_def fields (font number,
       scale, design size, name lengths) set to edge values, char packet
       lengths and codes set past the file, and DVI opcodes in a packet
       replaced (an undefined font, pushes without pops, pops without
       pushes, specials and rules of edge sizes).
  map  fuzz.map, generated from map-line pieces and mutated:
       ``\\pdfmapfile{=fuzz.map}\\font\\x=fuzz \\x a\\bye`` with fuzz.tfm
       (cmr10.tfm) and cmr10.pfb beside it.
  enc  fuzz.enc (a mutated 8r.enc, or a synthetic one) re-encoding cmr10:
       ``\\pdfmapline{=fuzz CMR10 "FuzzEncoding ReEncodeFont" <fuzz.enc
       <cmr10.pfb}\\font\\x=fuzz \\x abc\\bye``.

Classes: ok / rejected (the log shows a pdfTeX error or warning about the
file) / graceful-error / crash / hang / output-flood. Crash, hang and flood
inputs are stored as OUT/<class>/<sha256-prefix>.<kind> with a .json
sidecar, deduped by panic location or signal. Seeds come from the TeX
tree with kpsewhich at run time (nothing copied into the repo). Stdlib
only; deterministic given --seed (one random.Random).

  python3 tools/fuzz/parsers/fontfiles.py --kind vf --candidate BIN \\
      --iterations N --seed S --out DIR --timeout SEC
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
import run as fuzz_run  # noqa: E402

KINDS = ("vf", "map", "enc")
CLASSES = ("ok", "rejected", "graceful-error", "crash", "hang",
           "output-flood")
STORE = ("crash", "hang", "output-flood")
REJECT_HINTS = ("pdfTeX error", "pdfTeX warning", "not loadable",
                "Bad VF", "bad VF", "invalid", "Invalid", "cannot")
EDGE32 = (0, 1, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFF)
_TREE = {}


def kpse_bytes(name):
    """Bytes of a TeX tree file (cached; None when kpsewhich cannot find it)."""
    if name not in _TREE:
        blob = None
        try:
            cap = subprocess.run(["kpsewhich", name], capture_output=True,
                                 text=True, timeout=30)
            path = (cap.stdout or "").strip().splitlines()
            if path:
                with open(path[0], "rb") as fh:
                    blob = fh.read()
        except (OSError, subprocess.TimeoutExpired):
            blob = None
        _TREE[name] = blob
    return _TREE[name]


# --- generic byte mutations -------------------------------------------------

def _flip(data, rng):
    if not data:
        return None
    buf = bytearray(data)
    for _ in range(rng.randint(1, 8)):
        buf[rng.randrange(len(buf))] ^= 1 << rng.randrange(8)
    return bytes(buf), "flip"


def _trunc(data, rng):
    if len(data) < 2:
        return None
    return data[:rng.randrange(len(data))], "truncate"


def _delete(data, rng):
    if len(data) < 4:
        return None
    a = rng.randrange(len(data))
    b = min(len(data), a + rng.randint(1, 64))
    return data[:a] + data[b:], "delete-chunk"


def _dup(data, rng):
    if not data:
        return None
    a = rng.randrange(len(data))
    b = min(len(data), a + rng.randint(1, 64))
    return data[:b] + data[a:b] * rng.randint(1, 16) + data[b:], "dup-chunk"


GENERIC = (_flip, _trunc, _delete, _dup)


# --- virtual fonts ----------------------------------------------------------

def synthetic_vf():
    """A tiny VF: one fnt_def (cmr10) and one char packet (set_char_65)."""
    pre = bytes([247, 202, 0]) + struct.pack(">II", 0, 10 << 20)
    name = b"cmr10"
    fnt = (bytes([243, 0]) + struct.pack(">III", 0, 10 << 20, 10 << 20)
           + bytes([0, len(name)]) + name)
    pkt = bytes([1, 65]) + (0x80000).to_bytes(3, "big") + bytes([65])
    return pre + fnt + pkt + bytes([248] * 4)


def parse_vf(data):
    """(preamble_end, fnt_defs, packets) or None. fnt_defs: list of
    (start, end, name_start, name_end); packets: list of (start, end,
    dvi_start) in file order."""
    if len(data) < 11 or data[0] != 247 or data[1] != 202:
        return None
    i = 3 + data[2] + 8
    defs, pkts = [], []
    while i < len(data):
        op = data[i]
        if 243 <= op <= 246:
            k = op - 242
            j = i + 1 + k + 12
            if j + 2 > len(data):
                return None
            a, ln = data[j], data[j + 1]
            end = j + 2 + a + ln
            if end > len(data):
                return None
            defs.append((i, end, j + 2, end))
            i = end
        elif op == 242:
            if i + 13 > len(data):
                return None
            pl = struct.unpack(">I", data[i + 1:i + 5])[0]
            end = i + 13 + pl
            if end > len(data):
                return None
            pkts.append((i, end, i + 13))
            i = end
        elif op < 242:
            end = i + 5 + op
            if end > len(data):
                return None
            pkts.append((i, end, i + 5))
            i = end
        else:
            break
    return 3 + data[2] + 8, defs, pkts


def _vf_self_reference(data, rng):
    p = parse_vf(data)
    if not p or not p[1]:
        return None
    start, end, ns, ne = p[1][rng.randrange(len(p[1]))]
    j = ns - 2
    rec = bytearray(data[start:j]) + bytes([0, 4]) + b"fuzz"
    return data[:start] + bytes(rec) + data[end:], "vf-self-reference"


def _vf_fntdef_field(data, rng):
    p = parse_vf(data)
    if not p or not p[1]:
        return None
    start, end, ns, ne = p[1][rng.randrange(len(p[1]))]
    k = data[start] - 242
    buf = bytearray(data)
    which = rng.choice(("number", "checksum", "scale", "design", "a", "l"))
    if which == "number":
        buf[start + 1:start + 1 + k] = bytes([0xFF] * k)
    elif which in ("checksum", "scale", "design"):
        off = start + 1 + k + {"checksum": 0, "scale": 4, "design": 8}[which]
        buf[off:off + 4] = struct.pack(">I", rng.choice(EDGE32))
    else:
        buf[ns - 2 + (0 if which == "a" else 1)] = rng.choice((0, 1, 255))
    return bytes(buf), "vf-fntdef-" + which


def _vf_packet_header(data, rng):
    p = parse_vf(data)
    if not p or not p[2]:
        return None
    start, end, dvi = p[2][rng.randrange(len(p[2]))]
    buf = bytearray(data)
    if data[start] == 242:
        off = start + 1 + 4 * rng.randrange(3)
        buf[off:off + 4] = struct.pack(">I", rng.choice(EDGE32))
        return bytes(buf), "vf-long-packet-field"
    which = rng.randrange(3)
    if which == 0:
        buf[start] = rng.choice((0, 241))
    elif which == 1:
        buf[start + 1] = rng.randrange(256)
    else:
        buf[start + 2:start + 5] = bytes([0xFF, 0xFF, 0xFF])
    return bytes(buf), "vf-short-packet-field"


# DVI commands a packet may hold, as replacement byte strings.
def _dvi_snippets(rng):
    return (
        bytes([235, 0xFF]),                               # fnt1 255: undefined
        bytes([171 + 63]),                                # fnt_num_63
        bytes([141]) * rng.randint(1, 4096),              # push x n
        bytes([142]) * rng.randint(1, 64),                # pop x n
        bytes([239, 0xFF]) + b"x" * rng.randint(0, 255),  # xxx1, short body
        bytes([242]) + struct.pack(">I", 0xFFFFFFFF),     # xxx4 huge length
        bytes([132]) + struct.pack(">ii", 0x7FFFFFFF, -0x80000000),  # set_rule
        bytes([146]) + struct.pack(">i", 0x7FFFFFFF),     # right4 huge
        bytes([160]) + struct.pack(">i", -0x80000000),    # down4 huge
        bytes([128, 0xFF]),                               # set1 255
        bytes([250, 251, 252, 253, 254, 255]),            # undefined opcodes
    )


def _vf_dvi(data, rng):
    p = parse_vf(data)
    if not p or not p[2]:
        return None
    start, end, dvi = p[2][rng.randrange(len(p[2]))]
    snip = rng.choice(_dvi_snippets(rng))
    a = rng.randint(dvi, end)
    body = data[dvi:a] + snip + data[a:end]
    if data[start] == 242:
        head = bytearray(data[start:dvi])
        head[1:5] = struct.pack(">I", len(body))
        return data[:start] + bytes(head) + body + data[end:], "vf-dvi-insert"
    if len(body) < 242:
        head = bytearray(data[start:dvi])
        head[0] = len(body)
        return data[:start] + bytes(head) + body + data[end:], "vf-dvi-insert"
    # The length no longer fits a short packet: keep it, so the packet
    # runs into what follows.
    return data[:dvi] + snip + data[dvi + len(snip):], "vf-dvi-overwrite"


VF_MUTATIONS = (_vf_self_reference, _vf_fntdef_field, _vf_packet_header,
                _vf_dvi) + GENERIC


# --- map files --------------------------------------------------------------

MAP_PIECES = (
    "fuzz", "CMR10", "<cmr10.pfb", "<<cmr10.pfb", "<[fuzz.enc", "<fuzz.enc",
    "<", "<<", "<[", '"', '" "', '".167 SlantFont"', '"1.2 ExtendFont"',
    '"FuzzEnc ReEncodeFont"', '"1e309 SlantFont"', '"-99999999999 ExtendFont"',
    '"0 ExtendFont"', '"SlantFont"', '"ReEncodeFont"', "4", "-4", "99999999999",
    "%", "+fuzz", "=fuzz", "-fuzz", "fuzz.ttf", "<fuzz.ttf", "<cmr10.pfa",
    "\\", "\x00", "\xff\xfe", "x" * 300, "/" * 40, "../cmr10.pfb",
)


def seed_map(rng):
    lines = ["fuzz CMR10 <cmr10.pfb"]
    for _ in range(rng.randint(0, 6)):
        lines.append(" ".join(rng.choice(MAP_PIECES)
                              for _ in range(rng.randint(1, 8))))
    rng.shuffle(lines)
    return ("\n".join(lines) + rng.choice(("\n", "", "\r\n"))).encode(
        "latin-1", "replace")


def _map_long_line(data, rng):
    return data + b"fuzz CMR10 " + b"x" * rng.choice((1023, 1024, 65536)) \
        + b"\n", "map-long-line"


def _map_many_lines(data, rng):
    return data + b"fuzz CMR10 <cmr10.pfb\n" * rng.choice((2, 1000)), \
        "map-many-lines"


MAP_MUTATIONS = (_map_long_line, _map_many_lines) + GENERIC


# --- encodings --------------------------------------------------------------

def synthetic_enc():
    names = " ".join("/a%d" % i for i in range(256))
    return ("/FuzzEncoding [ %s ] def\n" % names).encode()


def _enc_structure(data, rng):
    t = data.decode("latin-1")
    which = rng.randrange(6)
    if which == 0:
        return t.replace("[", "", 1).encode("latin-1"), "enc-no-open"
    if which == 1:
        return t.replace("]", "", 1).encode("latin-1"), "enc-no-close"
    if which == 2:
        extra = " ".join("/x%d" % i for i in range(rng.choice((1, 300, 5000))))
        return t.replace("]", extra + " ]", 1).encode("latin-1"), "enc-too-many"
    if which == 3:
        return t.replace("/", "/" + "n" * rng.choice((255, 256, 70000)), 1) \
            .encode("latin-1"), "enc-long-name"
    if which == 4:
        return t.replace(" /", " ", rng.randint(1, 50)).encode("latin-1"), \
            "enc-bare-names"
    return t.replace("def", "", 1).encode("latin-1"), "enc-no-def"


ENC_MUTATIONS = (_enc_structure,) * 3 + GENERIC


# --- the run ----------------------------------------------------------------

def load_seeds(kind):
    if kind == "vf":
        seeds = []
        for name in ("ptmr8t", "pplr8t"):
            vf, tfm = kpse_bytes(name + ".vf"), kpse_bytes(name + ".tfm")
            if vf and tfm:
                seeds.append((name + ".vf", vf, tfm))
        if not seeds:
            seeds.append(("synthetic.vf", synthetic_vf(),
                          kpse_bytes("cmr10.tfm") or b""))
        return seeds
    if kind == "enc":
        enc = kpse_bytes("8r.enc")
        return [("8r.enc", enc, None)] if enc else [("synthetic.enc",
                                                     synthetic_enc(), None)]
    return [("generated.map", None, None)]


def mutate_with_info(kind, base, rng):
    """(mutated bytes, mutation name) for one draw."""
    if kind == "map":
        base = seed_map(rng)
        muts = MAP_MUTATIONS
    else:
        muts = VF_MUTATIONS if kind == "vf" else ENC_MUTATIONS
    order = list(muts)
    rng.shuffle(order)
    for fn in order[:3]:
        got = fn(base, rng)
        if got is not None and got[0] != base:
            return got
    if kind == "map":
        return base, "map-generated"
    return _flip(base or b"\x00", rng)


def job_files(kind, mutated, tfm):
    """{file name: bytes} of the job folder (job.tex included)."""
    cmr_tfm, pfb = kpse_bytes("cmr10.tfm"), kpse_bytes("cmr10.pfb")
    files = {}
    if kind == "vf":
        files["fuzz.vf"] = mutated
        if tfm:
            files["fuzz.tfm"] = tfm
        files["job.tex"] = b"\\font\\x=fuzz \\x Az\\bye\n"
    elif kind == "map":
        files["fuzz.map"] = mutated
        if cmr_tfm:
            files["fuzz.tfm"] = cmr_tfm
        files["job.tex"] = b"\\pdfmapfile{=fuzz.map}\\font\\x=fuzz \\x a\\bye\n"
    else:
        files["fuzz.enc"] = mutated
        if cmr_tfm:
            files["fuzz.tfm"] = cmr_tfm
        files["job.tex"] = (b"\\pdfmapline{=fuzz CMR10 \"FuzzEncoding ReEncodeFont\" "
                            b"<fuzz.enc <cmr10.pfb}\\font\\x=fuzz \\x abc\\bye\n")
    if pfb and kind != "vf":
        files["cmr10.pfb"] = pfb
    return files


def classify(returncode, log):
    if fuzz_run.is_output_flood(returncode):
        return "output-flood"
    if returncode is not None and (returncode < 0 or returncode == 101):
        return "crash"
    if returncode == 0:
        return "ok"
    if any(h in (log or "") for h in REJECT_HINTS):
        return "rejected"
    return "graceful-error"


def candidate_env():
    env = dict(os.environ)  # FLASHTEX_POOL / FLASHTEX_FORMATS from the caller
    env["SOURCE_DATE_EPOCH"] = "0"
    return env


def run_one(kind, mutated, tfm, candidate, timeout):
    """Run one mutated file through the candidate; (class, rc, log)."""
    workdir = tempfile.mkdtemp(prefix="fontfiles-fuzz-")
    try:
        for name, blob in job_files(kind, mutated, tfm).items():
            with open(os.path.join(workdir, name), "wb") as fh:
                fh.write(blob)
        try:
            rc, out = fuzz_run.run_capped(
                [candidate] + fuzz_run.FUZZ_SHELL_ESCAPE_FLAGS
                + ["-fmt=pdftex", "-interaction=nonstopmode", "job.tex"],
                cwd=workdir, env=candidate_env(), timeout=timeout)
        except subprocess.TimeoutExpired as exc:
            out = (exc.stdout or b"") + (exc.stderr or b"")
            return "hang", None, out.decode("utf-8", "replace")
        log = out.decode("utf-8", "replace")
        return classify(rc, log), rc, log
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


def signature(cls, rc, log):
    if cls in ("hang", "output-flood"):
        return cls
    return fuzz_run.crash_signature(rc, log)


def load_known_signatures(out_dir):
    known = set()
    for root, _dirs, files in os.walk(out_dir):
        for name in files:
            if name.endswith(".json") and name != "signatures.json":
                try:
                    with open(os.path.join(root, name)) as fh:
                        info = json.load(fh)
                    if isinstance(info, dict) and info.get("signature"):
                        known.add(info["signature"])
                except (OSError, ValueError):
                    continue
    return known


def last_line(log):
    lines = [ln for ln in (log or "").splitlines() if ln.strip()]
    return lines[-1][:200] if lines else ""


def run_fuzz(kind, candidate, seeds, out_dir, iterations, seed, timeout):
    rng = random.Random(seed)
    known = load_known_signatures(out_dir)
    seen = set()
    counts = {c: 0 for c in CLASSES}
    for _ in range(iterations):
        name, base, tfm = seeds[rng.randrange(len(seeds))]
        mutated, mutation = mutate_with_info(kind, base, rng)
        cls, rc, log = run_one(kind, mutated, tfm, candidate, timeout)
        counts[cls] += 1
        if cls not in STORE:
            continue
        sig = signature(cls, rc, log)
        if sig in seen or sig in known:
            continue
        seen.add(sig)
        digest = hashlib.sha256(mutated).hexdigest()[:16]
        d = os.path.join(out_dir, cls)
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, digest + "." + kind), "wb") as fh:
            fh.write(mutated)
        with open(os.path.join(d, digest + ".json"), "w") as fh:
            json.dump({"kind": kind, "seed": seed, "origin": name,
                       "mutation": mutation, "returncode": rc,
                       "last_line": last_line(log),
                       "panic_location": fuzz_run.panic_location(log),
                       "signature": sig}, fh, indent=2)
    return counts


def main(argv=None):
    ap = argparse.ArgumentParser(description="VF / map / enc file fuzzer")
    ap.add_argument("--kind", choices=KINDS, required=True)
    ap.add_argument("--candidate", required=True)
    ap.add_argument("--iterations", type=int, default=100)
    ap.add_argument("--seed", type=int, default=0)
    ap.add_argument("--out", default=None)
    ap.add_argument("--timeout", type=float, default=10.0)
    args = ap.parse_args(argv)
    fuzz_run.apply_fsize_limit()
    if args.iterations < 0:
        print("error: --iterations must be >= 0", file=sys.stderr)
        return 2
    counts = run_fuzz(args.kind, args.candidate, load_seeds(args.kind),
                      args.out or (args.kind + "-out"), args.iterations,
                      args.seed, args.timeout)
    print("done: %d iterations: %s"
          % (args.iterations,
             " ".join("%s=%d" % (c, counts[c]) for c in CLASSES)))
    return counts


if __name__ == "__main__":
    result = main()
    sys.exit(0 if isinstance(result, dict) else result)
