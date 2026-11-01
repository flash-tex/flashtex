#!/usr/bin/env python3
"""Inclusion speed (see README.md beside this file): the engine
(target/release/flashtex-initex) against TeX Live's pdftex (oracle only) on
large inputs made here under target/ (a 4000x3000 PNG copied and one
decoded, a JPEG, every page of beamer's user guide). Each case is typeset 5
times per engine; medians, and medians minus an empty document's (startup
and format load). Needs Pillow for the inputs.

    python3 docs/evidence/pdf-images-2026-09-29/perf.py [--no-gen]
"""
import os, shutil, statistics, subprocess, time, sys
from PIL import Image
import random

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
OURS = os.path.join(ROOT, "target/release/flashtex-initex")
POOL = os.path.join(ROOT, "crates/flashtex-engine/pdftex.pool")
W = os.path.join(ROOT, "target/pdf-images-perf")
TEXMFDIST = subprocess.run(["kpsewhich", "-var-value", "TEXMFDIST"], capture_output=True,
                           text=True).stdout.strip()
GUIDE = os.path.join(TEXMFDIST, "doc/latex/beamer/beameruserguide.pdf")
os.makedirs(W + "/ours", exist_ok=True)
os.makedirs(W + "/tex", exist_ok=True)

def make_inputs():
    rnd = random.Random(1)
    w, h = 4000, 3000
    # smooth gradient with some noise: realistic compressed size
    base = Image.linear_gradient("L").resize((w, h))
    noise = Image.effect_noise((w, h), 40)
    r = Image.blend(base, noise, 0.3)
    g = base.rotate(90, expand=False).resize((w, h))
    b = noise
    rgb = Image.merge("RGB", (r, g, b))
    rgba = Image.merge("RGBA", (r, g, b, base))
    for d in ("ours", "tex"):
        rgb.save(f"{W}/{d}/big-rgb.png", compress_level=6)
        rgba.save(f"{W}/{d}/big-rgba.png", compress_level=6)
        rgb.save(f"{W}/{d}/big.jpg", quality=90)
        shutil.copy(GUIDE, f"{W}/{d}/guide.pdf")

SETUP = ("\\pdfoutput=1 \\pdfsuppressptexinfo=-1 \\pdfminorversion=7 \\pdfobjcompresslevel=2 "
         "\\pdfcompresslevel=9 \\pdfimagehicolor=1 ")
CASES = {
    "empty": SETUP + "x\\bye\n",
    "png-copy": SETUP + "\\pdfximage width 5in {big-rgb.png}\\pdfrefximage\\pdflastximage\\bye\n",
    "png-decode-alpha": SETUP + "\\pdfximage width 5in {big-rgba.png}\\pdfrefximage\\pdflastximage\\bye\n",
    "jpeg": SETUP + "\\pdfximage width 5in {big.jpg}\\pdfrefximage\\pdflastximage\\bye\n",
    "pdf-all-pages": SETUP + "\\pdfximage{guide.pdf}\\count3=\\pdflastximagepages "
        "\\advance\\count3 by1 \\count2=1 \\loop\\pdfximage width 5in page\\count2 {guide.pdf}"
        "\\pdfrefximage\\pdflastximage\\vfill\\eject\\advance\\count2 by1 "
        "\\ifnum\\count2<\\count3 \\repeat\\bye\n",
}

def run(argv, cwd, env):
    t0 = time.perf_counter()
    subprocess.run(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                   stderr=subprocess.DEVNULL)
    return time.perf_counter() - t0

def main():
    if "--no-gen" not in sys.argv:
        make_inputs()
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
    oenv = dict(env, FLASHTEX_POOL=POOL)
    run([OURS, "-ini", "\\input plain \\dump"], W + "/ours", oenv)
    run(["pdftex", "-ini", "\\input plain \\dump"], W + "/tex", env)
    res = {}
    for name, body in CASES.items():
        for d in ("ours", "tex"):
            open(f"{W}/{d}/{name}.tex", "w").write(body)
        to, tt = [], []
        for _ in range(5):
            to.append(run([OURS, "-fmt=plain", "&plain " + name], W + "/ours", oenv))
            tt.append(run(["pdftex", "&plain " + name], W + "/tex", env))
        same = open(f"{W}/ours/{name}.pdf", "rb").read() == open(f"{W}/tex/{name}.pdf", "rb").read()
        res[name] = (statistics.median(to), statistics.median(tt), same, os.path.getsize(f"{W}/tex/{name}.pdf"))
    e_o, e_t = res["empty"][0], res["empty"][1]
    print("| case | ours (s) | pdfTeX (s) | ours - empty | pdfTeX - empty | PDF bytes | identical |")
    print("|---|---|---|---|---|---|---|")
    for name, (o, t, same, size) in res.items():
        print(f"| {name} | {o:.3f} | {t:.3f} | {o - e_o:.3f} | {t - e_t:.3f} | {size} | {same} |")

main()
