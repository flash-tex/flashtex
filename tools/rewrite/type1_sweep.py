#!/usr/bin/env python3
"""Differential sweep of the Type 1 font writer over TeX Live's whole font map.

The proof obligation of a rewritten module (docs/design/engine-v2/REWRITE.md,
"Proof per step") asks for byte-identical output on every input the module can
see, not only on the parity fixtures. For the Type 1 writer (writet1.c's port,
`crates/flashtex-engine/src/pdftex/writet1/`) that input is every Type 1 font
TeX Live maps, under every map option pdfTeX applies to it, and every broken
font a user may have. This script makes small `-ini` jobs and runs them:

  record   run the map's jobs through ENGINE (or TeX Live's pdftex) and write
           one line per job to OUT: exit status, whether the run ended in a
           fatal error and its message, SHA-256 of the PDF, of the log and of
           each embedded font file stream in order
  compare  compare two recorded files: every job must have the same PDF, log
           and exit status (old engine against new), or with --streams-only
           the same font file streams, exit status, fatal error and its
           message (an engine against TeX Live's pdftex, whose PDF and log
           differ elsewhere: producer, IDs, paths)
  fuzz     differential fuzzing: mutate cmr10/cmti10/cmbx10 as PFB and as
           PFA with tools/fuzz/parsers/type1.py's mutations, embed each
           (subset and whole) through every --engine and, with --pdftex,
           TeX Live's pdftex. Fails on a crash, and where the engines
           differ byte for byte; with --pdftex only where the last engine
           then also differs from pdftex (exit status, error and warning
           messages, font streams): a regression, not a fix; with --strict
           on any difference from pdftex. Disagreeing inputs are saved to
           OUT with a .json sidecar.

Variants per map entry (record):
  some   eight character codes (A B a e 0 and three chosen by the entry's name)
  all    all 256 codes
  full   all 256 codes, the entry rewritten by \\pdfmapline to `<<' (whole font)

Which entries: every font file once (its first map line), every entry that
slants or extends a font, and every tenth remaining entry (re-encodings), in
map order; with --every N, every Nth of those. A batch that ends in a fatal
font error is run again entry by entry, so one broken font hides no other.

Usage:
  type1_sweep.py record (--engine DIR | --pdftex PATH) --out FILE [-j N]
                        [--limit N] [--every N]
  type1_sweep.py compare OLD NEW [--streams-only]
  type1_sweep.py fuzz --engine DIR [--engine DIR ...] [--pdftex PATH]
                      --iterations N --seed S --out DIR [-j N]

DIR is an engine directory as scripts/engine-parity.sh stages it (`pdftex`,
a link to flashtex-initex, and `pdftex.pool`). Every program runs with
argv[0] `pdftex` (warnings name the program as invoked), SOURCE_DATE_EPOCH=0
and FORCE_SOURCE_DATE=1, so a PDF is a function of the engine and the font.
"""

import argparse
import concurrent.futures as cf
import hashlib
import importlib.util
import json
import os
import random
import re
import shutil
import subprocess
import sys
import tempfile
import zlib

BATCH = 20
HERE = os.path.dirname(os.path.abspath(__file__))


def map_file():
    return subprocess.run(["kpsewhich", "pdftex.map"], capture_output=True,
                          text=True, check=True).stdout.strip()


def entries(limit, every=1):
    """(tfm, line, fontfile token) for the selected map lines."""
    seen_ff, out, k, n = set(), [], 0, 0
    with open(map_file(), encoding="latin-1") as fh:
        for line in fh:
            line = line.rstrip("\n")
            if not line.strip() or line[0] in "%#*;":
                continue
            toks = re.findall(r'"[^"]*"|\S+', line)
            ff = [t for t in toks if t.startswith("<") and
                  re.search(r"\.pf[ab]$", t, re.I)]
            if not ff:
                continue
            name = ff[0].lstrip("<[")
            quoted = " ".join(t for t in toks if t.startswith('"'))
            if name not in seen_ff:
                seen_ff.add(name)
                pick = True
            elif "Slant" in quoted or "Extend" in quoted:
                pick = True
            else:
                k += 1
                pick = k % 10 == 0
            if pick:
                n += 1
                if (n - 1) % every == 0:
                    out.append((toks[0], line, ff[0]))
            if limit and len(out) >= limit:
                break
    return out


def codes(tfm, variant):
    if variant == "some":
        h = hashlib.sha256(tfm.encode()).digest()
        return sorted(set([65, 66, 97, 101, 48, h[0], h[1], h[2]]))
    return list(range(256))


HEAD = ("\\catcode`\\{=1 \\catcode`\\}=2 \\pdfoutput=1 "
        "\\pdfcompresslevel=0 \\pdfobjcompresslevel=0")


def job_text(batch, variant):
    out = [HEAD]
    for tfm, line, ff in batch:
        if variant == "full":
            if ff.startswith("<<"):
                continue
            whole = line.replace(ff, "<<" + ff.lstrip("<["), 1)
            out.append("\\pdfmapline{=%s}" % whole)
        chars = "".join("\\char%d " % c for c in codes(tfm, variant))
        out.append("\\font\\x=%s \\setbox0\\hbox{\\x %s}\\shipout\\box0" % (tfm, chars))
    out.append("\\end")
    return "\n".join(out) + "\n"


STREAM = re.compile(rb"/Length1 \d+\s*/Length2 \d+\s*/Length3 \d+\s*(?:/Length \d+\s*)?>>\s*stream\r?\n")


def font_streams(pdf):
    """SHA-256 of each Type 1 font file stream, in file order."""
    hashes = []
    for m in STREAM.finditer(pdf):
        end = pdf.find(b"endstream", m.end())
        body = pdf[m.end():end]
        try:
            body = zlib.decompress(body)
        except zlib.error:
            pass
        hashes.append(hashlib.sha256(body).hexdigest()[:16])
    return hashes


def messages(log):
    """The fatal error's message and the warnings, each joined across the
    log's line breaks (TeX breaks lines at max_print_line)."""
    text = log.decode("latin-1")
    error = None
    m = re.search(r"!pdfTeX error: (.*?) ==> Fatal error", text, re.S)
    if m:
        error = m.group(1).replace("\n", "")
    warnings = [w.replace("\n", "") for w in
                re.findall(r"pdfTeX warning: (.*?)\n\n", text + "\n\n", re.S)]
    return error, warnings


def engine_cmd(engine=None, pdftex=None):
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
    if engine:
        env["FLASHTEX_POOL"] = os.path.join(engine, "pdftex.pool")
        return os.path.join(engine, "pdftex"), env
    return pdftex, env


def run_in(exe, env, d, timeout=600):
    """Run job.tex in `d`; return (rc, pdf bytes, log bytes)."""
    try:
        # argv[0] is `pdftex` for every program, as a run from PATH has it
        rc = subprocess.run(["pdftex", "-ini", "-interaction=batchmode", "job.tex"],
                            executable=exe, cwd=d, env=env,
                            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL, timeout=timeout).returncode
    except subprocess.TimeoutExpired:
        rc = "timeout"

    def read(name):
        try:
            with open(os.path.join(d, name), "rb") as fh:
                return fh.read()
        except OSError:
            return b""
    return rc, read("job.pdf"), read("job.log")


def summarise(rc, pdf, log):
    error, warnings = messages(log)
    return {
        "rc": rc,
        "pdf": hashlib.sha256(pdf).hexdigest()[:16] if pdf else None,
        "log": hashlib.sha256(log).hexdigest()[:16],
        "fonts": font_streams(pdf),
        "fatal": b"!pdfTeX error" in log or b"! Emergency stop" in log,
        "error": error,
        "warnings": warnings,
    }


def run_job(exe, env, text, key, work):
    d = os.path.join(work, key)
    os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, "job.tex"), "w", encoding="latin-1") as fh:
        fh.write(text)
    r = summarise(*run_in(exe, env, d))
    shutil.rmtree(d, ignore_errors=True)
    return r


def record(args):
    exe, env = engine_cmd(args.engine, args.pdftex)
    sel = entries(args.limit, args.every)
    jobs = []
    for variant in ("some", "all", "full"):
        for i in range(0, len(sel), BATCH):
            jobs.append(("%s-%05d" % (variant, i // BATCH), variant, sel[i:i + BATCH]))
    work = tempfile.mkdtemp(prefix="t1sweep-")
    results = {}
    with cf.ThreadPoolExecutor(args.jobs) as ex:
        futs = {ex.submit(run_job, exe, env, job_text(b, v), k, work): (k, v, b)
                for k, v, b in jobs}
        retry = []
        for f in cf.as_completed(futs):
            k, v, b = futs[f]
            r = f.result()
            results[k] = r
            if r["fatal"] and len(b) > 1:
                retry.extend(("%s-%02d" % (k, j), v, [e]) for j, e in enumerate(b))
        futs = {ex.submit(run_job, exe, env, job_text(b, v), k, work): k
                for k, v, b in retry}
        for f in cf.as_completed(futs):
            results[futs[f]] = f.result()
    shutil.rmtree(work, ignore_errors=True)
    with open(args.out, "w") as fh:
        for k in sorted(results):
            fh.write(json.dumps({"job": k, **results[k]}) + "\n")
    fatal = sum(1 for r in results.values() if r["fatal"])
    streams = sum(len(r["fonts"]) for r in results.values())
    print("%d entries, %d jobs (%d fatal), %d font streams -> %s"
          % (len(sel), len(results), fatal, streams, args.out))


def load(path):
    with open(path) as fh:
        return {r["job"]: r for r in map(json.loads, fh)}


def against_pdftex(x, y):
    """What differs between an engine's run and pdftex's, as compare
    --streams-only sees them; None if nothing."""
    for key in ("rc", "fatal", "error", "fonts"):
        if x.get(key) != y.get(key):
            return key
    return None


def compare(args):
    a, b = load(args.old), load(args.new)
    keys = sorted(set(a) | set(b))
    diff = []
    same_streams = 0
    for k in keys:
        x, y = a.get(k), b.get(k)
        if x is None or y is None:
            diff.append((k, "missing"))
            continue
        if args.streams_only:
            why = against_pdftex(x, y)
            if why:
                diff.append((k, why))
            else:
                same_streams += len(x["fonts"])
        elif (x["pdf"], x["log"], x["rc"]) != (y["pdf"], y["log"], y["rc"]):
            diff.append((k, "pdf" if x["pdf"] != y["pdf"] else "log/rc"))
    n = sum(len(r["fonts"]) for r in a.values())
    print("%d jobs, %d font streams in OLD; %d differ" % (len(keys), n, len(diff)))
    if args.streams_only:
        print("%d font streams identical (with exit status and fatal errors)" % same_streams)
    for k, why in diff[:50]:
        print("  DIFF %s: %s" % (k, why))
    return 1 if diff else 0


# --- fuzz ----------------------------------------------------------------------


def fuzz_mutator():
    """tools/fuzz/parsers/type1.py, the T6 fuzzer's Type 1 mutations."""
    path = os.path.join(HERE, "..", "fuzz", "parsers", "type1.py")
    spec = importlib.util.spec_from_file_location("t1fuzz", path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def pfb_to_pfa(pfb):
    """The PFA form of a PFB (t1ascii): clear segments as they are, binary
    ones in hexadecimal, 64 digits a line; None if the segments do not parse."""
    out, i = bytearray(), 0
    while i + 2 <= len(pfb) and pfb[i] == 0x80:
        kind = pfb[i + 1]
        if kind == 3:
            return bytes(out)
        if i + 6 > len(pfb):
            return None
        n = int.from_bytes(pfb[i + 2:i + 6], "little")
        body = pfb[i + 6:i + 6 + n]
        if len(body) != n or kind not in (1, 2):
            return None
        if kind == 1:
            out += body.replace(b"\r", b"\n")
        else:
            h = body.hex().encode()
            for j in range(0, len(h), 64):
                out += h[j:j + 64] + b"\n"
        i += 6 + n
    return None


def fuzz_case(i, rng_seed, seeds, tfm, mut):
    """The i-th input: (name, bytes, mutation, whole)."""
    rng = random.Random("%d-%d" % (rng_seed, i))
    name, pfb = rng.choice(seeds)
    whole = rng.random() < 0.25
    if i % 3 == 2:
        # a PFA: the PFB mutated then converted, or the PFA mutated as bytes
        mutated, desc = mut.mutate_with_info(pfb, rng)
        pfa = pfb_to_pfa(mutated)
        if pfa is None or rng.random() < 0.5:
            pfa, desc = mut.mutate_with_info(pfb_to_pfa(pfb), rng)
        return name.replace(".pfb", ".pfa"), pfa, desc, whole
    mutated, desc = mut.mutate_with_info(pfb, rng)
    return name, mutated, desc, whole


FUZZ_CHARS = "".join("\\char%d " % c for c in list(range(0, 128, 3)) + [65, 97, 98, 99])


def fuzz_one(i, case, programs, tfm, work, timeout):
    name, font, desc, whole = case
    ext = name[-3:]
    results = []
    for exe, env in programs:
        d = os.path.join(work, "%06d-%d" % (i, len(results)))
        os.makedirs(d)
        with open(os.path.join(d, "fuzz." + ext), "wb") as fh:
            fh.write(font)
        with open(os.path.join(d, "fuzz.tfm"), "wb") as fh:
            fh.write(tfm)
        mapline = "+fuzz fuzz %sfuzz.%s" % ("<<" if whole else "<", ext)
        with open(os.path.join(d, "job.tex"), "w") as fh:
            fh.write("%s\n\\pdfmapline{%s}\\font\\x=fuzz "
                     "\\setbox0\\hbox{\\x %s}\\shipout\\box0\n\\end\n"
                     % (HEAD, mapline, FUZZ_CHARS))
        rc, pdf, log = run_in(exe, env, d, timeout)
        r = summarise(rc, pdf, log)
        if isinstance(rc, int) and (rc < 0 or rc == 101):
            r["crash"] = True
        results.append(r)
        shutil.rmtree(d, ignore_errors=True)
    return results


def fuzz(args):
    mut = fuzz_mutator()
    found = [(n, subprocess.run(["kpsewhich", n], capture_output=True, text=True).stdout.strip())
             for n in ("cmr10.pfb", "cmti10.pfb", "cmbx10.pfb")]
    seeds = [(n, open(p, "rb").read()) for n, p in found if p]
    tfm_path = subprocess.run(["kpsewhich", "cmr10.tfm"], capture_output=True, text=True).stdout.strip()
    if not seeds or not tfm_path:
        print("fuzz: no TeX Live seeds (cmr10.pfb, cmr10.tfm)", file=sys.stderr)
        return 2
    tfm = open(tfm_path, "rb").read()
    programs = [engine_cmd(engine=e) for e in args.engine]
    if args.pdftex:
        programs.append(engine_cmd(pdftex=args.pdftex))
    os.makedirs(args.out, exist_ok=True)
    work = tempfile.mkdtemp(prefix="t1fuzz-")
    counts = {"cases": 0, "pfb": 0, "pfa": 0, "whole": 0, "fatal": 0,
              "engines-differ": 0, "pdftex-differs": 0, "regressions": 0,
              "fixed": 0, "crash": 0}
    with cf.ThreadPoolExecutor(args.jobs) as ex:
        cases = {i: fuzz_case(i, args.seed, seeds, tfm, mut) for i in range(args.iterations)}
        futs = {ex.submit(fuzz_one, i, c, programs, tfm, work, args.timeout): i
                for i, c in cases.items()}
        for f in cf.as_completed(futs):
            i = futs[f]
            name, font, desc, whole = cases[i]
            res = f.result()
            counts["cases"] += 1
            counts[name[-3:]] += 1
            counts["whole"] += whole
            counts["fatal"] += res[0]["fatal"]
            why = []
            engines = res[:len(args.engine)]
            if any(r.get("crash") for r in engines):
                counts["crash"] += 1
                why.append("crash")
            if any((r["pdf"], r["log"], r["rc"]) != (engines[0]["pdf"], engines[0]["log"], engines[0]["rc"])
                   for r in engines[1:]):
                counts["engines-differ"] += 1
                why.append("engines differ")
            if args.pdftex:
                cand, ref = engines[-1], res[-1]
                what = against_pdftex(cand, ref) or (
                    "warnings" if cand["warnings"] != ref["warnings"] else None)
                if what:
                    counts["pdftex-differs"] += 1
                    why.append("pdftex differs: " + what)
                    if "engines differ" in why:
                        # the change moved the last engine, and not to pdfTeX
                        counts["regressions"] += 1
                elif "engines differ" in why:
                    # the last engine now agrees with pdfTeX where the first did not
                    counts["fixed"] += 1
            if why:
                digest = hashlib.sha256(font).hexdigest()[:16]
                base = os.path.join(args.out, "%s.%s" % (digest, name[-3:]))
                with open(base, "wb") as fh:
                    fh.write(font)
                with open(base + ".json", "w") as fh:
                    json.dump({"iteration": i, "seed": name, "mutation": desc,
                               "whole": whole, "why": why, "results": res}, fh, indent=1)
    shutil.rmtree(work, ignore_errors=True)
    print("fuzz: " + " ".join("%s=%d" % kv for kv in counts.items()))
    with open(os.path.join(args.out, "summary.json"), "w") as fh:
        json.dump(counts, fh, indent=1)
    # Without pdftex every difference between the engines fails; with it,
    # only a difference that does not bring the last engine to pdfTeX
    # (a regression), and with --strict any difference from pdfTeX.
    moved = counts["regressions"] if args.pdftex else counts["engines-differ"]
    bad = moved + counts["crash"] + (counts["pdftex-differs"] if args.strict else 0)
    return 1 if bad else 0


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = p.add_subparsers(dest="cmd", required=True)
    jobs = max(1, (os.cpu_count() or 2) // 2)
    r = sub.add_parser("record")
    g = r.add_mutually_exclusive_group(required=True)
    g.add_argument("--engine")
    g.add_argument("--pdftex")
    r.add_argument("--out", required=True)
    r.add_argument("-j", "--jobs", type=int, default=jobs)
    r.add_argument("--limit", type=int, default=0)
    r.add_argument("--every", type=int, default=1, help="every Nth selected entry")
    c = sub.add_parser("compare")
    c.add_argument("old")
    c.add_argument("new")
    c.add_argument("--streams-only", action="store_true")
    z = sub.add_parser("fuzz")
    z.add_argument("--engine", action="append", required=True,
                   help="an engine directory; the first is the reference for the others")
    z.add_argument("--pdftex", help="TeX Live's pdftex, compared with the last --engine")
    z.add_argument("--iterations", type=int, required=True)
    z.add_argument("--seed", type=int, required=True)
    z.add_argument("--out", required=True)
    z.add_argument("--timeout", type=float, default=60)
    z.add_argument("--strict", action="store_true",
                   help="also fail when the last engine and pdftex differ")
    z.add_argument("-j", "--jobs", type=int, default=jobs)
    a = p.parse_args()
    if a.cmd == "record":
        record(a)
        return 0
    if a.cmd == "fuzz":
        return fuzz(a)
    return compare(a)


if __name__ == "__main__":
    sys.exit(main())
