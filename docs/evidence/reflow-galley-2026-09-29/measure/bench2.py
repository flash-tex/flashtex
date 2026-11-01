#!/usr/bin/env python3
"""Second batch: hyperref variants and more repetitions.

bench2.py BODY [REPS] [ONLY]
BODY in a|b|c (generated bodies, see gen.py) or e (1000 near-empty pages:
"x\\par\\newpage", measures fixed per-page output-routine+shipout cost).
Driver is mainh.tex (= main.tex, plus \\NOHYPER / \\HYLIGHT switches).
Appends to results2.jsonl.
"""
import json, os, re, resource, shutil, statistics, subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
os.chdir(HERE)
BODY = sys.argv[1]
REPS = int(sys.argv[2]) if len(sys.argv) > 2 else 9
ONLY = set(sys.argv[3].split(",")) if len(sys.argv) > 3 else None
DRIVER = os.environ.get("DRIVER", "mainh")  # mainh | mainh-nosi (no siunitx)
CAL = {"a": (6410, 320, 6, 64), "b": (4500, 225, 5, 45), "c": (4255, 213, 4, 43)}


def gen(chunk):
    if BODY == "e":
        name = "body-e-0.inc"
        with open(name, "w") as f:
            f.write("\\section{E}\n" + "x\\par\\newpage\n" * 1000)
        return name
    NPAR, PERSEC, C1, C10 = CAL[BODY]
    return subprocess.check_output(["python3", "gen.py", BODY, str(NPAR), str(PERSEC), str(chunk)], text=True).strip()


def parse_log(job):
    log = open(job + ".log", encoding="latin-1").read()
    out = {}
    m = re.search(r"GALLEY-SPLIT G=(\d+) P=(\d+) chunks=(\d+)", log)
    if m:
        out["G"] = int(m.group(1)) / 65536.0
        out["P"] = int(m.group(2)) / 65536.0
        out["chunks"] = int(m.group(3))
    for m in re.finditer(r"GALLEY-TIME (\S+) (\d+)", log):
        out["t_" + m.group(1)] = int(m.group(2)) / 65536.0
    m = re.search(r"Output written on \S+ \((\d+) pages?", log)
    out["pages"] = int(m.group(1)) if m else 0
    out["errors"] = len(re.findall(r"^! ", log, re.M))
    return out


def run(name, engine, body, mode, extra, reps=REPS, aux_from=None):
    if ONLY and name not in ONLY:
        return
    job = "h%s-%s-%s" % (BODY, DRIVER, name)
    argv = [engine, "-interaction=batchmode", "-jobname=" + job,
            r"%s\def\BODY{%s}\def\MODE{%s}\input{%s}" % (extra, body, mode, DRIVER)]
    res = []
    for i in range(reps + 2):
        if aux_from and os.path.exists(aux_from + ".aux"):
            shutil.copyfile(aux_from + ".aux", job + ".aux")
        r0 = resource.getrusage(resource.RUSAGE_CHILDREN)
        t0 = time.perf_counter()
        subprocess.run(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        wall = time.perf_counter() - t0
        r1 = resource.getrusage(resource.RUSAGE_CHILDREN)
        d = parse_log(job)
        d.update(wall=wall, cpu=(r1.ru_utime - r0.ru_utime) + (r1.ru_stime - r0.ru_stime), load=os.getloadavg()[0])
        if i >= 2:
            res.append(d)
    with open("results2.jsonl", "a") as f:
        f.write(json.dumps({"body": BODY, "driver": DRIVER, "config": name, "argv": argv, "runs": res}) + "\n")
    w = [x["wall"] for x in res]
    gp = ""
    if "G" in res[0]:
        gp = "G min/med %.3f/%.3f P min/med %.3f/%.3f" % (
            min(x["G"] for x in res), statistics.median(x["G"] for x in res),
            min(x["P"] for x in res), statistics.median(x["P"] for x in res))
    print("%s %-22s pages=%d wall min/med/max %.3f/%.3f/%.3f %s" % (
        BODY, name, res[0]["pages"], min(w), statistics.median(w), max(w), gp), flush=True)


P, L = "pdflatex", "latex"
b0 = gen(0)
if BODY == "e":
    for hy, ex in (("full", ""), ("light", r"\def\HYLIGHT{}"), ("none", r"\def\NOHYPER{}")):
        run("empty-pdf-" + hy, P, "empty.inc", "plain", ex)
        run("empty-dvi-" + hy, L, "empty.inc", "plain", ex)
        run("plain-pdf-" + hy, P, b0, "plain", ex)
        run("plain-discard-" + hy, P, b0, "plain", ex + r"\def\DISCARD{}")
        run("plain-dvi-" + hy, L, b0, "plain", ex)
else:
    b1 = gen(CAL[BODY][2])
    for hy, ex in (("full", ""), ("light", r"\def\HYLIGHT{}"), ("none", r"\def\NOHYPER{}")):
        run("plain-pdf-" + hy, P, b0, "plain", ex)
        run("split1-pdf-" + hy, P, b1, "split", ex)
        run("split1-discard-" + hy, P, b1, "split", ex + r"\def\DISCARD{}",
            aux_from="h%s-%s-split1-pdf-%s" % (BODY, DRIVER, hy))
        run("gonly1-" + hy, P, b1, "gonly", ex, aux_from="h%s-%s-split1-pdf-%s" % (BODY, DRIVER, hy))
        run("split1-dvi-" + hy, L, b1, "split", ex)
