#!/usr/bin/env python3
"""Output-routine timing (ortime.tex) for synthetic bodies and real documents.

bench_or.py [REPS]  -> results-or.jsonl + stdout table
For each target: normal PDF run and shipout-discarded run, each with the
output routine wrapped by \\ORbegin/\\ORend. Reports (seconds):
  body  = \\pdfelapsedtime from \\begin{document} end to \\enddocument start
  OR    = time inside the output routine (incl. \\shipout) up to \\enddocument
"""
import glob, json, os, re, shutil, statistics, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPS = int(sys.argv[1]) if len(sys.argv) > 1 else 7
AUXEXT = ("aux", "toc", "lof", "lot", "idx", "glo", "out", "ind", "gls", "bbl", "brf")
ORT = os.path.join(HERE, "ortime")


def snap(d, job):
    s = os.path.join(d, "_orsnap")
    os.makedirs(s, exist_ok=True)
    for e in AUXEXT:
        f = os.path.join(d, job + "." + e)
        if os.path.exists(f):
            shutil.copyfile(f, os.path.join(s, job + "." + e))


def restore(d):
    s = os.path.join(d, "_orsnap")
    for f in os.listdir(s):
        shutil.copyfile(os.path.join(s, f), os.path.join(d, f))


def one(d, job, tex):
    restore(d)
    subprocess.run(["pdflatex", "-interaction=batchmode", "-jobname=" + job, tex], cwd=d,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    log = open(os.path.join(d, job + ".log"), encoding="latin-1").read()
    m = re.search(r"OR-TIME at-enddoc acc=(\d+) n=(\d+) now=(\d+)", log)
    m2 = re.search(r"OR-TIME final acc=(\d+) n=(\d+) now=(\d+)", log)
    return dict(OR=int(m.group(1)) / 65536, n=int(m.group(2)), body=int(m.group(3)) / 65536,
                OR_final=int(m2.group(1)) / 65536, total_final=int(m2.group(3)) / 65536)


targets = []
for b in "abc":
    for hy, ex in (("full", ""), ("light", r"\def\HYLIGHT{}")):
        body = r"%s\def\BODY{body-%s-0.inc}\def\MODE{plain}\input{mainh}" % (ex, b)
        targets.append(("%s-%s" % (b, hy), HERE, "or-%s-%s" % (b, hy), body))
targets.append(("TeXbyTopic", os.path.join(HERE, "real/texbytopic"), "TeXbyTopic", r"\input{TeXbyTopic}"))
targets.append(("memman", os.path.join(HERE, "real/memoir"), "memman", r"\input{memman}"))

for name, d, job, tail in targets:
    norm = r"\input{%s}%s" % (ORT, tail)
    disc = r"\input{%s}\AddToHook{shipout/before}{\DiscardShipoutBox}%s" % (ORT, tail)
    for _ in range(2):
        subprocess.run(["pdflatex", "-interaction=batchmode", "-jobname=" + job, norm], cwd=d,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    snap(d, job)
    out = {}
    for kind, tex in (("pdf", norm), ("discard", disc)):
        runs = [one(d, job, tex) for _ in range(REPS)]
        out[kind] = runs
        med = lambda k: statistics.median(r[k] for r in runs)
        mn = lambda k: min(r[k] for r in runs)
        print("%-12s %-8s n=%d body min/med %.3f/%.3f  OR min/med %.3f/%.3f  nonOR med %.3f" % (
            name, kind, runs[0]["n"], mn("body"), med("body"), mn("OR"), med("OR"),
            statistics.median(r["body"] - r["OR"] for r in runs)), flush=True)
    restore(d)
    with open(os.path.join(HERE, "results-or.jsonl"), "a") as f:
        f.write(json.dumps({"target": name, "runs": out}) + "\n")
