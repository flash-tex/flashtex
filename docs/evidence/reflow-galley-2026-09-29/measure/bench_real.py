#!/usr/bin/env python3
"""Real-document timings: full PDF, -draftmode, DVI, and shipout-discarded.

bench_real.py DIR MAINTEX [REPS]
Aux-type files are snapshotted after two normal warm-up runs and restored
before every timed run, so each run reads identical auxiliary input.
"""
import glob, json, os, re, resource, shutil, statistics, subprocess, sys, time

d, main = sys.argv[1], sys.argv[2]
REPS = int(sys.argv[3]) if len(sys.argv) > 3 else 5
os.chdir(d)
job = main[:-4]
AUXEXT = ("aux", "toc", "lof", "lot", "idx", "glo", "out", "ind", "gls", "bbl", "brf")


def snap():
    os.makedirs("_snap", exist_ok=True)
    for e in AUXEXT:
        for f in glob.glob("*." + e):
            shutil.copyfile(f, os.path.join("_snap", f))


def restore():
    for f in os.listdir("_snap"):
        shutil.copyfile(os.path.join("_snap", f), f)


def once(argv):
    restore()
    r0 = resource.getrusage(resource.RUSAGE_CHILDREN)
    t0 = time.perf_counter()
    subprocess.run(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    wall = time.perf_counter() - t0
    r1 = resource.getrusage(resource.RUSAGE_CHILDREN)
    log = open(job + ".log", encoding="latin-1").read()
    m = re.search(r"Output written on \S+ \((\d+) pages?", log)
    return dict(wall=wall, cpu=(r1.ru_utime - r0.ru_utime) + (r1.ru_stime - r0.ru_stime),
                pages=int(m.group(1)) if m else 0, load=os.getloadavg()[0])


for _ in range(2):
    subprocess.run(["pdflatex", "-interaction=batchmode", main], stdout=subprocess.DEVNULL)
snap()
configs = [
    ("pdf", ["pdflatex", "-interaction=batchmode", main]),
    ("pdf-nocomp", ["pdflatex", "-interaction=batchmode", r"\pdfcompresslevel=0 \pdfobjcompresslevel=0 \input{%s}" % main]),
    ("draft", ["pdflatex", "-interaction=batchmode", "-draftmode", main]),
    ("dvi", ["latex", "-interaction=batchmode", main]),
    ("discard", ["pdflatex", "-interaction=batchmode", r"\AddToHook{shipout/before}{\DiscardShipoutBox}\input{%s}" % main]),
    ("discard-dvi", ["latex", "-interaction=batchmode", r"\AddToHook{shipout/before}{\DiscardShipoutBox}\input{%s}" % main]),
]
for name, argv in configs:
    runs = [once(argv) for _ in range(REPS + 1)][1:]
    w = [r["wall"] for r in runs]
    print("%s %-12s pages=%d wall min/med/max %.3f/%.3f/%.3f cpu med %.3f" % (
        job, name, runs[0]["pages"], min(w), statistics.median(w), max(w),
        statistics.median([r["cpu"] for r in runs])), flush=True)
    with open("../../results-real.jsonl", "a") as f:
        f.write(json.dumps({"doc": job, "config": name, "argv": argv, "runs": runs}) + "\n")
restore()
