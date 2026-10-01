#!/usr/bin/env python3
"""One pdflatex run (the oracle's cost per pass) on a settled directory, N times: wall and CPU."""
import os, resource, subprocess, sys, time, json
d = sys.argv[1]
n = int(sys.argv[2]) if len(sys.argv) > 2 else 3
env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1",
           PATH="/Users/dqi26/flashtex-wt/d1-bin:" + os.environ["PATH"])
out = []
for _ in range(n):
    r0 = resource.getrusage(resource.RUSAGE_CHILDREN)
    l0 = os.getloadavg()[0]
    t0 = time.time()
    subprocess.run(["pdflatex", "-interaction=nonstopmode", "main.tex"], cwd=d, env=env,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    w = time.time() - t0
    r1 = resource.getrusage(resource.RUSAGE_CHILDREN)
    out.append({"wall": round(w, 3), "cpu": round(r1.ru_utime + r1.ru_stime - r0.ru_utime - r0.ru_stime, 3),
                "load": round(l0, 2)})
print(json.dumps(out))
