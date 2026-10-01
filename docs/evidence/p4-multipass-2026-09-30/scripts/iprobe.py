#!/usr/bin/env python3
"""iserve probe: cold compile, biber, compile; then a \\cite edit, compile, biber, compile.
iprobe.py ENGDIR OUT [--pages 120] [--style biblatex] [--env K=V ...]"""
import argparse, json, os, shutil, subprocess, sys, time
sys.path.insert(0, "/Users/dqi26/flashtex/.claude/worktrees/agent-a9b0d190d583b912c/tools/external-tools")
import xtools

TEXBIN = "/Users/dqi26/flashtex-wt/d1-bin"
ap = argparse.ArgumentParser()
ap.add_argument("eng"); ap.add_argument("out")
ap.add_argument("--pages", type=int, default=120)
ap.add_argument("--style", default="biblatex")
ap.add_argument("--env", action="append", default=[])
ap.add_argument("--edits", type=int, default=1)
ap.add_argument("--defer", action="store_true")
a = ap.parse_args()
shutil.rmtree(a.out, ignore_errors=True)
d = os.path.join(a.out, "doc")
n = xtools.gen_doc(d, a.pages, a.style)
env = dict(os.environ)
env.update(xtools.ENV_PIN)
env.update({"FLASHTEX_FORMATS": os.path.join(a.eng, "fmt"), "FLASHTEX_POOL": os.path.join(a.eng, "pdftex.pool"),
            "FLASHTEX_PIN_CLOCK": "0", "FLASHTEX_TEXLIVE_BIN": TEXBIN, "PATH": TEXBIN + ":" + os.environ["PATH"]})
env.update(dict(kv.split("=", 1) for kv in a.env))
err = open(os.path.join(a.out, "iserve.err"), "w")
p = subprocess.Popen([os.path.join(a.eng, "flashtex-host"), "iserve", "--argv0", "pdflatex", "--", "-interaction=nonstopmode", "main.tex"], cwd=d,
                     env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=err, text=True)


def compile_(tag, cmd="compile"):
    t0 = time.time()
    p.stdin.write(cmd + "\n"); p.stdin.flush()
    line = p.stdout.readline()
    r = json.loads(line)
    keep = {k: r.get(k) for k in ("mode", "cold_reason", "restart_pages", "converged_at", "rerun_pages", "pages",
                                  "passes", "pass_modes", "pass_s", "l5", "oscillation", "total_s", "diffs")}
    keep["wall"] = round(time.time() - t0, 3)
    print(tag, json.dumps(keep), flush=True)
    return r


def biber():
    t0 = time.time()
    r = subprocess.run([os.path.join(TEXBIN, "biber"), "main"], cwd=d, env=env, capture_output=True)
    print("biber", r.returncode, round(time.time() - t0, 3), flush=True)


compile_("open1", "compile-defer" if a.defer else "compile")
biber()
compile_("open2")
compile_("open3")
for i in range(a.edits):
    t = open(os.path.join(d, "main.tex"), "rb").read()
    mid = t.find(b". ", len(t) // 2) + 2
    open(os.path.join(d, "main.tex"), "wb").write(t[:mid] + f"See \\cite{{k{n + i}}}. ".encode() + t[mid:])
    compile_(f"edit{i}-1", "compile-defer" if a.defer else "compile")
    biber()
    compile_(f"edit{i}-2")
    compile_(f"edit{i}-3")
p.stdin.write("quit\n"); p.stdin.flush(); p.wait()
