#!/usr/bin/env python3
"""Timing harness for the galley (G) vs pagination (P) split.

bench.py BODY [REPS]   -> appends JSON lines to results.jsonl
Each config: 2 warm-up runs (aux settles, caches warm) then REPS timed runs.
Records wall time, child user+sys CPU, and the in-run \\pdfelapsedtime marks.
"""
import json, os, re, resource, shutil, statistics, subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
os.chdir(HERE)
BODY = sys.argv[1]
REPS = int(sys.argv[2]) if len(sys.argv) > 2 else 5
# paragraphs per ~1000 pages, per section (~50 pages), per chunk (~1 page, ~10 pages)
CAL = {"a": (6410, 320, 6, 64), "b": (4500, 225, 5, 45), "c": (4255, 213, 4, 43)}
NPAR, PERSEC, C1, C10 = CAL[BODY]
ONLY = set(sys.argv[3].split(",")) if len(sys.argv) > 3 else None


def gen(chunk):
    return subprocess.check_output(["python3", "gen.py", BODY, str(NPAR), str(PERSEC), str(chunk)], text=True).strip()


def cmd(engine, job, body, mode, extra="", flags=()):
    tex = r"%s\def\BODY{%s}\def\MODE{%s}\input{main}" % (extra, body, mode)
    return [engine, "-interaction=batchmode", *flags, "-jobname=" + job, tex]


def parse_log(job):
    log = open(job + ".log", encoding="latin-1").read()
    out = {}
    for m in re.finditer(r"GALLEY-TIME (\S+) (\d+)", log):
        out["t_" + m.group(1)] = int(m.group(2)) / 65536.0
    m = re.search(r"GALLEY-SPLIT G=(\d+) P=(\d+) chunks=(\d+)", log)
    if m:
        out["G"] = int(m.group(1)) / 65536.0
        out["P"] = int(m.group(2)) / 65536.0
        out["chunks"] = int(m.group(3))
    m = re.search(r"Output written on \S+ \((\d+) pages?", log)
    out["pages"] = int(m.group(1)) if m else 0
    out["errors"] = len(re.findall(r"^! ", log, re.M))
    return out


def run(name, argv, reps=REPS, pre=None):
    if ONLY and name not in ONLY:
        return
    res = []
    for i in range(reps + 2):
        if pre:
            pre()
        r0 = resource.getrusage(resource.RUSAGE_CHILDREN)
        t0 = time.perf_counter()
        subprocess.run(argv, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        wall = time.perf_counter() - t0
        r1 = resource.getrusage(resource.RUSAGE_CHILDREN)
        cpu = (r1.ru_utime - r0.ru_utime) + (r1.ru_stime - r0.ru_stime)
        job = [a for a in argv if a.startswith("-jobname=")][0][9:]
        d = parse_log(job)
        d.update(wall=wall, cpu=cpu, load=os.getloadavg()[0])
        if i >= 2:
            res.append(d)
    rec = {"body": BODY, "config": name, "argv": argv, "runs": res}
    with open("results.jsonl", "a") as f:
        f.write(json.dumps(rec) + "\n")
    w = [x["wall"] for x in res]
    print("%s %-18s pages=%d wall min/med/max %.3f/%.3f/%.3f cpu med %.3f %s" % (
        BODY, name, res[0]["pages"], min(w), statistics.median(w), max(w),
        statistics.median([x["cpu"] for x in res]),
        ("G/P med %.3f/%.3f" % (statistics.median([x["G"] for x in res]), statistics.median([x["P"] for x in res]))) if "G" in res[0] else ""),
        flush=True)


def main():
    b0, b1, b10 = gen(0), gen(C1), gen(C10)
    bempty = "empty.inc"
    open(bempty, "w").write("\\section{Empty}\nx\n")
    P, L = "pdflatex", "latex"
    job = lambda s: "%s-%s" % (BODY, s)
    # baselines
    run("empty-pdf", cmd(P, job("empty-pdf"), bempty, "plain"))
    run("empty-dvi", cmd(L, job("empty-dvi"), bempty, "plain"))
    run("plain-pdf", cmd(P, job("plain-pdf"), b0, "plain"))
    run("plain-pdf-nocomp", cmd(P, job("plain-pdf-nocomp"), b0, "plain", r"\def\NOCOMP{}"))
    run("plain-draft", cmd(P, job("plain-draft"), b0, "plain", flags=("-draftmode",)))
    run("plain-dvi", cmd(L, job("plain-dvi"), b0, "plain"))
    # discard: \shipout's box is thrown away. Labels are never written, so
    # restore the normal aux before each run so \eqref expands identically.
    def restore(src, dst):
        return lambda: shutil.copyfile(src + ".aux", dst + ".aux")
    run("plain-discard", cmd(P, job("plain-discard"), b0, "plain", r"\def\DISCARD{}"),
        pre=restore(job("plain-pdf"), job("plain-discard")))
    # split runs
    run("split1-pdf", cmd(P, job("split1-pdf"), b1, "split"))
    run("split10-pdf", cmd(P, job("split10-pdf"), b10, "split"))
    run("split1-dvi", cmd(L, job("split1-dvi"), b1, "split"))
    run("split10-dvi", cmd(L, job("split10-dvi"), b10, "split"))
    run("split1-discard", cmd(P, job("split1-discard"), b1, "split", r"\def\DISCARD{}"),
        pre=restore(job("plain-pdf"), job("split1-discard")))
    run("split10-discard", cmd(P, job("split10-discard"), b10, "split", r"\def\DISCARD{}"),
        pre=restore(job("plain-pdf"), job("split10-discard")))
    run("gonly1", cmd(P, job("gonly1"), b1, "gonly"), pre=restore(job("plain-pdf"), job("gonly1")))
    run("gonly10", cmd(P, job("gonly10"), b10, "gonly"), pre=restore(job("plain-pdf"), job("gonly10")))


main()
