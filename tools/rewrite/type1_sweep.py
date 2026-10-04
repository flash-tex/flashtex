#!/usr/bin/env python3
"""Differential sweep of the Type 1 font writer over TeX Live's whole font map.

The proof obligation of a rewritten module (docs/design/engine-v2/REWRITE.md,
"Proof per step") asks for byte-identical output on every input the module can
see, not only on the parity fixtures. For the Type 1 writer (writet1.c's port,
`crates/flashtex-engine/src/pdftex/writet1/`) that input is every Type 1 font
TeX Live maps, under every map option pdfTeX applies to it. This script makes
small `-ini` jobs from `pdftex.map`, each embedding a batch of map entries, and
runs them through one engine:

  record   run every job through ENGINE and write one line per job to OUT
           (exit status, SHA-256 of the PDF and of the log, and of each
           embedded font file stream in order)
  compare  compare two recorded files: every job must have the same PDF and
           log bytes (old engine against new), or with --streams-only the
           same font file streams (engine against TeX Live's pdftex, whose
           PDF differs elsewhere: producer, IDs)

Variants per map entry:
  some   eight character codes (A B a e 0 and three chosen by the entry's name)
  all    all 256 codes
  full   all 256 codes, the entry rewritten by \\pdfmapline to `<<' (whole font)

Which entries: every font file once (its first map line), every entry that
slants or extends a font, and every tenth remaining entry (re-encodings), in
map order. A batch that ends in a fatal font error is run again entry by
entry, so one broken font hides no other.

Usage:
  type1_sweep.py record --engine DIR --out FILE [-j N] [--limit N]
  type1_sweep.py record --pdftex PATH --out FILE [-j N] [--limit N]
  type1_sweep.py compare OLD NEW [--streams-only]

DIR is an engine directory as scripts/engine-parity.sh stages it (`pdftex`,
a link to flashtex-initex, and `pdftex.pool`). Runs use SOURCE_DATE_EPOCH=0
and FORCE_SOURCE_DATE=1, so a PDF is a function of the engine and the font.
"""

import argparse
import concurrent.futures as cf
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import zlib

BATCH = 20


def map_file():
    return subprocess.run(["kpsewhich", "pdftex.map"], capture_output=True,
                          text=True, check=True).stdout.strip()


def entries(limit):
    """(tfm, line, fontfile token index) for the selected map lines."""
    seen_ff, out, k = set(), [], 0
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
                out.append((toks[0], line, ff[0]))
            if limit and len(out) >= limit:
                break
    return out


def codes(tfm, variant):
    if variant == "some":
        h = hashlib.sha256(tfm.encode()).digest()
        return sorted(set([65, 66, 97, 101, 48, h[0], h[1], h[2]]))
    return list(range(256))


def job_text(batch, variant):
    out = ["\\catcode`\\{=1 \\catcode`\\}=2 \\pdfoutput=1 "
           "\\pdfcompresslevel=0 \\pdfobjcompresslevel=0"]
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


def run_job(cmd, env, text, key, work):
    d = os.path.join(work, key)
    os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, "job.tex"), "w", encoding="latin-1") as fh:
        fh.write(text)
    try:
        # argv[0] is `pdftex` for every engine, as a run from PATH has it:
        # warnings name the program as it was invoked
        rc = subprocess.run(["pdftex", "-ini", "-interaction=batchmode", "job.tex"],
                            executable=cmd[0], cwd=d,
                            env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL, timeout=600).returncode
    except subprocess.TimeoutExpired:
        rc = "timeout"

    def read(name):
        try:
            with open(os.path.join(d, name), "rb") as fh:
                return fh.read()
        except OSError:
            return b""
    pdf, log = read("job.pdf"), read("job.log")
    shutil.rmtree(d, ignore_errors=True)
    return {
        "rc": rc,
        "pdf": hashlib.sha256(pdf).hexdigest()[:16] if pdf else None,
        "log": hashlib.sha256(log).hexdigest()[:16],
        "fonts": font_streams(pdf),
        "fatal": b"!pdfTeX error" in log or b"! Emergency stop" in log,
    }


def record(args):
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
    if args.engine:
        cmd = [os.path.join(args.engine, "pdftex")]
        env["FLASHTEX_POOL"] = os.path.join(args.engine, "pdftex.pool")
    else:
        cmd = [args.pdftex]
    sel = entries(args.limit)
    jobs = []
    for variant in ("some", "all", "full"):
        for i in range(0, len(sel), BATCH):
            jobs.append(("%s-%05d" % (variant, i // BATCH), variant, sel[i:i + BATCH]))
    work = tempfile.mkdtemp(prefix="t1sweep-")
    results = {}
    with cf.ThreadPoolExecutor(args.jobs) as ex:
        futs = {ex.submit(run_job, cmd, env, job_text(b, v), k, work): (k, v, b)
                for k, v, b in jobs}
        retry = []
        for f in cf.as_completed(futs):
            k, v, b = futs[f]
            r = f.result()
            results[k] = r
            if r["fatal"] and len(b) > 1:
                retry.extend(("%s-%02d" % (k, j), v, [e]) for j, e in enumerate(b))
        futs = {ex.submit(run_job, cmd, env, job_text(b, v), k, work): k
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
            if x["fonts"] != y["fonts"]:
                diff.append((k, "font streams"))
            else:
                same_streams += len(x["fonts"])
        elif (x["pdf"], x["log"], x["rc"]) != (y["pdf"], y["log"], y["rc"]):
            diff.append((k, "pdf" if x["pdf"] != y["pdf"] else "log/rc"))
    n = sum(len(r["fonts"]) for r in a.values())
    print("%d jobs, %d font streams in OLD; %d differ" % (len(keys), n, len(diff)))
    if args.streams_only:
        print("%d font streams identical" % same_streams)
    for k, why in diff[:50]:
        print("  DIFF %s: %s" % (k, why))
    return 1 if diff else 0


def main():
    p = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = p.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("record")
    g = r.add_mutually_exclusive_group(required=True)
    g.add_argument("--engine")
    g.add_argument("--pdftex")
    r.add_argument("--out", required=True)
    r.add_argument("-j", "--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    r.add_argument("--limit", type=int, default=0)
    c = sub.add_parser("compare")
    c.add_argument("old")
    c.add_argument("new")
    c.add_argument("--streams-only", action="store_true")
    a = p.parse_args()
    if a.cmd == "record":
        record(a)
        return 0
    return compare(a)


if __name__ == "__main__":
    sys.exit(main())
