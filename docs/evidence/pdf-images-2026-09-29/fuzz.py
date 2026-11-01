#!/usr/bin/env python3
"""Malformed-input sweep (see README.md beside this file): mutate the test
images of crates/flashtex-engine/tests/images (truncations, random bytes,
zeroed runs; seeded, so every run makes the same mutants), include each
mutant with the engine (target/release/flashtex-initex) and with TeX Live's
pdftex (oracle only), and compare exit status, PDF and log. Reports crashes
(signals), hangs and panics.

    python3 docs/evidence/pdf-images-2026-09-29/fuzz.py [mutants per file]
"""
import os, random, subprocess, sys, re, time

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
IMG = os.path.join(ROOT, "crates/flashtex-engine/tests/images")
OURS = os.path.join(ROOT, "target/release/flashtex-initex")
POOL = os.path.join(ROOT, "crates/flashtex-engine/pdftex.pool")
W = os.path.join(ROOT, "target/pdf-images-fuzz")
BASES = ["png-rgb8.png", "png-pal8.png", "png-rgba16.png", "png-rgb8-interlaced.png",
         "jpg-rgb.jpg", "jpg-progressive.jpg", "jpg-exif.jpg", "jbig2-sequential.jb2", "jbig2-random.jbig2",
         "pdf-hand.pdf", "pdf-fonts.pdf", "pdf-objstm.pdf"]
PER = int(sys.argv[1]) if len(sys.argv) > 1 else 12

def mutants(name, data, rnd):
    n = len(data)
    out = []
    for k in range(1, 5):
        out.append(("trunc%d" % k, data[: n * k // 5]))
    for k in range(PER):
        b = bytearray(data)
        for _ in range(rnd.randint(1, 6)):
            i = rnd.randrange(n)
            b[i] = rnd.randrange(256)
        out.append(("flip%d" % k, bytes(b)))
    for k in range(PER // 3):
        b = bytearray(data)
        i = rnd.randrange(n)
        j = min(n, i + rnd.randint(1, 64))
        b[i:j] = bytes(j - i)
        out.append(("zero%d" % k, bytes(b)))
    return out

def run(argv, cwd, env):
    t0 = time.time()
    try:
        p = subprocess.run(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                           stderr=subprocess.PIPE, timeout=20)
        return p.returncode, time.time() - t0, p.stderr.decode("latin-1")
    except subprocess.TimeoutExpired:
        return "timeout", 20.0, ""

def norm_log(path):
    try:
        s = open(path, encoding="latin-1").read()
    except OSError:
        return None
    i = s.find("\n**")
    s = "".join(s[i + 1:].splitlines())
    s = re.sub(r"(pdfTeX warning: |!pdfTeX error: )[^ :]+", r"\1PROG", s)
    return s

def main():
    os.makedirs(W + "/ours", exist_ok=True)
    os.makedirs(W + "/tex", exist_ok=True)
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
    oenv = dict(env, FLASHTEX_POOL=POOL)
    run([OURS, "-ini", "\\input plain \\dump"], W + "/ours", oenv)
    run(["pdftex", "-ini", "\\input plain \\dump"], W + "/tex", env)
    rnd = random.Random(20260929)
    stats = {"n": 0, "crash_ours": 0, "crash_tex": 0, "hang_ours": 0, "exit_same": 0, "pdf_same": 0,
             "log_same": 0, "both_pdf": 0}
    notes = []
    for base in BASES:
        data = open(os.path.join(IMG, base), "rb").read()
        ext = os.path.splitext(base)[1]
        for tag, mdata in mutants(base, data, rnd):
            name = "m-" + base.replace(".", "-") + "-" + tag
            for d in ("ours", "tex"):
                with open(os.path.join(W, d, name + ext), "wb") as f:
                    f.write(mdata)
                page = "page 2 " if base.startswith("jbig2-seq") else ""
                with open(os.path.join(W, d, name + ".tex"), "w") as f:
                    f.write("\\pdfoutput=1 \\pdfsuppressptexinfo=-1 \\pdfminorversion=7 "
                            "\\pdfobjcompresslevel=2 \\pdfcompresslevel=9 \\pdfimagehicolor=1 "
                            "\\pdfximage %s{%s%s}\\pdfrefximage\\pdflastximage\\bye\n" % (page, name, ext))
                for suf in (".pdf", ".log"):
                    try:
                        os.remove(os.path.join(W, d, name + suf))
                    except OSError:
                        pass
            co, to, eo = run([OURS, "-fmt=plain", "&plain " + name], W + "/ours", oenv)
            ct, tt, et = run(["pdftex", "&plain " + name], W + "/tex", env)
            stats["n"] += 1
            crash_o = co == "timeout" or (isinstance(co, int) and co < 0) or "panicked" in eo
            crash_t = ct == "timeout" or (isinstance(ct, int) and ct < 0)
            stats["crash_ours"] += crash_o
            stats["crash_tex"] += crash_t
            stats["hang_ours"] += co == "timeout"
            stats["exit_same"] += co == ct
            po, pt = os.path.join(W, "ours", name + ".pdf"), os.path.join(W, "tex", name + ".pdf")
            if os.path.exists(po) and os.path.exists(pt):
                stats["both_pdf"] += 1
                if open(po, "rb").read() == open(pt, "rb").read():
                    stats["pdf_same"] += 1
                else:
                    notes.append("%s: PDFs differ" % name)
            elif os.path.exists(po) != os.path.exists(pt):
                notes.append("%s: PDF from %s only" % (name, "ours" if os.path.exists(po) else "pdftex"))
            lo, lt = norm_log(os.path.join(W, "ours", name + ".log")), norm_log(os.path.join(W, "tex", name + ".log"))
            if lo == lt:
                stats["log_same"] += 1
            else:
                notes.append("%s: logs differ (exit ours %s, pdftex %s)" % (name, co, ct))
            if crash_o or crash_t:
                notes.append("%s: crash/hang ours=%s pdftex=%s %s" % (name, co, ct, eo[-200:].strip()))
    print(stats)
    for n in notes:
        print(n)

main()
