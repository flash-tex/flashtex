#!/usr/bin/env python3
"""D1 P4-MULTIPASS bench: latexmk (oracle) vs flashtex-host on generated docs.

mpbench.py ENGDIR OUTDIR [--pages 120] [--style biblatex] [--reps 3] [--skip-lmk]
"""
import argparse, json, os, resource, shutil, subprocess, sys, time, hashlib
sys.path.insert(0, "/Users/dqi26/flashtex/.claude/worktrees/agent-a9b0d190d583b912c/tools/external-tools")
import xtools  # noqa

TEXBIN = "/Users/dqi26/flashtex-wt/d1-bin"


def load():
    return os.getloadavg()[0]


def cpu_children():
    r = resource.getrusage(resource.RUSAGE_CHILDREN)
    return r.ru_utime + r.ru_stime


def ps_cpu(pid):
    out = subprocess.run(["ps", "-o", "time=", "-p", str(pid)], capture_output=True, text=True).stdout.strip()
    if not out:
        return None
    parts = out.split(":")
    s = 0.0
    for p in parts:
        s = s * 60 + float(p)
    return s


def edit_bytes(t, key):
    mid = t.find(b". ", len(t) // 2) + 2
    ins = f"See \\cite{{{key}}}. ".encode()
    return mid, ins, t[:mid] + ins + t[mid:]


def lmk(d):
    env = dict(os.environ)
    env.update(xtools.ENV_PIN)
    env["PATH"] = TEXBIN + os.pathsep + env["PATH"]
    c0, l0, t0 = cpu_children(), load(), time.time()
    p = subprocess.run(["latexmk", "-pdf", "-interaction=nonstopmode", "main.tex"], cwd=d, env=env,
                       stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    wall = time.time() - t0
    out = p.stdout.decode("utf-8", "replace")
    rules = [l.split("'")[1] for l in out.splitlines() if l.startswith("Run number")]
    return {"wall": round(wall, 3), "cpu": round(cpu_children() - c0, 3), "load": round(l0, 2), "rc": p.returncode,
            "rules": rules}


def sha(p):
    return hashlib.sha256(open(p, "rb").read()).hexdigest()[:16] if os.path.exists(p) else None


def gen_book(d, pages, nbib=400):
    """The book-sized tier: book class, hyperref, a table of contents,
    chapters and sections with labels, \\ref/\\pageref, footnotes, makeidx
    \\index entries, biblatex + biber with a 400-entry .bib."""
    import random
    xtools.gen_doc(d, 1, "biblatex", nbib)  # the .bib (main.tex is replaced)
    rng = random.Random(1000 + pages)
    words = ("lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore "
             "et dolore magna aliqua enim ad minim veniam quis nostrud exercitation ullamco laboris nisi aliquip "
             "ex ea commodo consequat duis aute irure in reprehenderit voluptate velit esse cillum").split()
    body, n, nsec = [], 0, 0
    for p in range(pages * 5):
        if p % 75 == 0:
            body.append(f"\\chapter{{Chapter {p // 75}}}\\label{{ch:{p // 75}}}\n")
        if p % 15 == 0:
            body.append(f"\\section{{Part {p}}}\\label{{sec:{nsec}}}\n")
            nsec += 1
        s = " ".join(rng.choice(words) for _ in range(90)).capitalize() + "."
        if p % 4 == 1:
            w = rng.choice(words)
            s += f"\\index{{{w}}}\\index{{{w}!{rng.choice(words)}}}"
        if p % 2 == 0 and n < nbib - 40:
            s += f" \\cite{{k{n}}}."
            n += 1
        if p % 7 == 3 and nsec > 1:
            k = rng.randrange(nsec)
            s += f" See section~\\ref{{sec:{k}}} on page~\\pageref{{sec:{k}}}."
        if p % 10 == 5:
            s += f"\\footnote{{A note on {rng.choice(words)}.}}"
        body.append(s + "\n\n")
    pre = ("\\documentclass{book}\n\\usepackage{makeidx}\\makeindex\n"
           "\\usepackage[backend=biber,style=numeric]{biblatex}\n\\addbibresource{refs.bib}\n"
           "\\usepackage{hyperref}\n\\begin{document}\n\\tableofcontents\n")
    post = "\\printbibliography\n\\printindex\n\\end{document}\n"
    with open(os.path.join(d, "main.tex"), "w") as f:
        f.write(pre + "".join(body) + post)
    return n


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("eng")
    ap.add_argument("out")
    ap.add_argument("--pages", type=int, default=120)
    ap.add_argument("--style", default="biblatex")
    ap.add_argument("--reps", type=int, default=3)
    ap.add_argument("--skip-lmk", action="store_true")
    ap.add_argument("--env", action="append", default=[])
    a = ap.parse_args()
    shutil.rmtree(a.out, ignore_errors=True)
    os.makedirs(a.out)
    src = os.path.join(a.out, "src")
    ncited = gen_book(src, a.pages) if a.style == "book" else xtools.gen_doc(src, a.pages, a.style)
    keys = [f"k{ncited + r}" for r in range(a.reps)]
    res = {"doc": f"{a.style}-{a.pages}", "load_start": round(load(), 2)}
    # oracle
    if not a.skip_lmk:
        d = os.path.join(a.out, "lmk")
        shutil.copytree(src, d)
        r = lmk(d)
        r["pdf"] = sha(os.path.join(d, "main.pdf"))
        res["lmk_open"] = r
        shutil.copy(os.path.join(d, "main.pdf"), os.path.join(a.out, "lmk-open.pdf"))
        res["lmk_edits"] = []
        for i, k in enumerate(keys):
            t = open(os.path.join(d, "main.tex"), "rb").read()
            _, _, nt = edit_bytes(t, k)
            open(os.path.join(d, "main.tex"), "wb").write(nt)
            r = lmk(d)
            r["pdf"] = sha(os.path.join(d, "main.pdf"))
            shutil.copy(os.path.join(d, "main.pdf"), os.path.join(a.out, f"lmk-edit{i}.pdf"))
            res["lmk_edits"].append(r)
        print(json.dumps({"lmk": res.get("lmk_open"), "edits": res["lmk_edits"]}), flush=True)
    # host
    d = os.path.join(a.out, "host")
    shutil.copytree(src, d)
    work = os.path.join(a.out, "hwork")
    os.makedirs(work)
    env = dict(kv.split("=", 1) for kv in a.env)
    env["FLASHTEX_TEXLIVE_BIN"] = TEXBIN
    h = xtools.Host(os.path.join(a.eng, "flashtex-host"), os.path.join(a.eng, "fmt"),
                    os.path.join(a.eng, "pdftex.pool"), work, env_extra=env)
    pid = h.p.pid

    def cyc(req):
        c0, l0, t0 = ps_cpu(pid), load(), time.time()
        ev = h.cycle(req)
        wall = time.time() - t0
        c1 = ps_cpu(pid)
        tools = [{k: x.get(k) for k in ("tool", "ms", "changed", "status")} for x in ev["tools"] if x.get("event") == "done"]
        dones = [{k: x.get(k) for k in ("mode", "run_ms", "passes", "pass_modes", "pass_s", "pages", "cause", "rerun_pages",
                                       "converged_at", "restart_pages", "l5", "passes", "deferred", "bytes", "typeset_pages") if k in x} for x in ev["dones"]]
        return {"wall": round(wall, 3), "host_cpu": round(c1 - c0, 3), "load": round(l0, 2),
                "t_first_done": round(ev["t_done"] or 0, 3), "tools": tools, "dones": dones,
                "errors": ev["errors"][:3], "done_keys": sorted(ev["dones"][0].keys()) if ev["dones"] else []}, ev

    def export(i, name):
        ex = h.export({"id": 9000 + i, "root": d, "main": "main.tex", "output_dir": d})
        p = os.path.join(d, "main.pdf")
        shutil.copy(p, os.path.join(a.out, f"host-{name}.pdf"))
        return sha(p)

    try:
        base = {"root": d, "main": "main.tex", "output_dir": d, "external_tools": "auto", "incremental": True}
        r, ev = cyc(dict(base, id=1))
        r["pdf"] = export(0, "open")
        res["host_open"] = r
        print(json.dumps({"host_open": r}), flush=True)
        res["host_edits"] = []
        for i, k in enumerate(keys):
            t = open(os.path.join(d, "main.tex"), "rb").read()
            mid, ins, _ = edit_bytes(t, k)
            e = {"path": "main.tex", "offset": mid, "delete": 0, "insert": ins.decode()}
            r, ev = cyc(dict(base, id=100 + i, edits=[e]))
            r["pdf"] = export(i + 1, f"edit{i}")
            res["host_edits"].append(r)
            print(json.dumps({"host_edit": i, **r}), flush=True)
    finally:
        h.close()
    res["load_end"] = round(load(), 2)
    json.dump(res, open(os.path.join(a.out, "result.json"), "w"), indent=1)


if __name__ == "__main__":
    main()
