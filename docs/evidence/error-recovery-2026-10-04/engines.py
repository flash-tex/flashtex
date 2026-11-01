#!/usr/bin/env python3
"""Run every ERROR-RECOVERY case through pdflatex (the oracle), the new engine
invoked as pdftex, and the old engine's `flashtex-compiler`, one shot each.

usage: engines.py WORKDIR [--pdflatex P] [--engine flashtex-host] [--old flashtex-compiler] [--only id,...]

Writes WORKDIR/engines.json: per case and engine, the exit status, pages, the
errors (message, line) and whether the run stopped on a fatal error. The oracle
and the new engine both run `-interaction=nonstopmode -file-line-error`, as the
app's host does (`Job::argv`). MacTeX is the oracle only.
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
FATAL = ("Emergency stop", "job aborted", "TeX capacity exceeded", "Fatal error occurred")


def apply(text, edits):
    for e in edits:
        i = text.index(e["find"])
        text = text[:i] + e["replace"] + text[i + len(e["find"]):]
    return text


def tex_run(cmd, d, env=None):
    t = time.time()
    p = subprocess.run(cmd + ["-interaction=nonstopmode", "-file-line-error", "main.tex"], cwd=d,
                       stdout=subprocess.PIPE, stderr=subprocess.STDOUT, env=env, timeout=600)
    el = time.time() - t
    log = open(os.path.join(d, "main.log"), encoding="latin-1").read() if os.path.exists(os.path.join(d, "main.log")) else ""
    m = re.search(r"Output written on \S+ \((\d+) pages?", log)
    errs = []
    lines = log.splitlines()
    for i, l in enumerate(lines):
        fm = re.match(r"^\./main\.tex:(\d+): (.*)", l)
        if fm:
            errs.append({"line": int(fm.group(1)), "message": fm.group(2)})
        elif l.startswith("! "):
            errs.append({"line": None, "message": l[2:]})
    fatal = next((f for f in FATAL if f in log), None)
    return {"exit": p.returncode, "pages": int(m.group(1)) if m else 0, "errors": errs[:8], "nerrors": len(errs),
            "fatal": fatal, "seconds": round(el, 3)}


def old_run(compiler, text):
    req = {"protocol_version": 1, "type": "compile", "id": "er-1", "payload": {"project_id": "er", "revision": 1, "entry_path": "main.tex",
                                                          "documents": [{"path": "main.tex", "text": text}]}}
    t = time.time()
    p = subprocess.run([compiler], input=(json.dumps(req) + "\n").encode(), stdout=subprocess.PIPE,
                       stderr=subprocess.DEVNULL, timeout=600)
    el = time.time() - t
    first = p.stdout.split(b"\n", 1)[0]
    try:
        env = json.loads(first)
    except Exception:
        return {"exit": p.returncode, "raw": first[:300].decode(errors="replace"), "seconds": round(el, 3)}
    r = env.get("payload", env)
    diags = r.get("diagnostics", [])
    return {"status": r.get("status"), "pages": len(r.get("pages", [])),
            "errors": [{"severity": d.get("severity"), "message": d.get("message"), "source": d.get("source"),
                        "fix": bool(d.get("fix") or d.get("fixes") or d.get("quickFix"))} for d in diags][:8],
            "nerrors": sum(1 for d in diags if d.get("severity") == "error"), "ndiags": len(diags),
            "seconds": round(el, 3)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("work")
    ap.add_argument("--pdflatex", default="/usr/local/texlive/2026/bin/universal-darwin/pdflatex")
    ap.add_argument("--engine")
    ap.add_argument("--old")
    ap.add_argument("--only")
    a = ap.parse_args()
    corpus = json.load(open(os.path.join(HERE, "corpus.json")))
    only = set(a.only.split(",")) if a.only else None
    res = {}
    os.makedirs(a.work, exist_ok=True)
    for c in corpus["cases"]:
        if only and c["id"] not in only:
            continue
        text = apply(corpus["bases"][c["base"]], c["edits"])
        r = {"what": c["what"]}
        for name, cmd in (("pdflatex", [a.pdflatex]), ("v3", [a.engine] if a.engine else None)):
            if not cmd:
                continue
            d = os.path.join(a.work, c["id"], name)
            shutil.rmtree(d, ignore_errors=True)
            os.makedirs(d)
            open(os.path.join(d, "main.tex"), "w").write(text)
            env = dict(os.environ)
            if name == "v3":
                # the engine is the host invoked as `pdftex` (src/host/main.rs)
                link = os.path.join(a.work, "pdftex")
                if not os.path.exists(link):
                    os.symlink(os.path.abspath(a.engine), link)
                cmd = [link, "-fmt=pdflatex"]
            r[name] = tex_run(cmd, d, env)
        if a.old:
            r["old"] = old_run(a.old, text)
        res[c["id"]] = r
        print(c["id"], json.dumps({k: (v.get("pages"), v.get("nerrors"), v.get("fatal") or v.get("status")) for k, v in r.items() if isinstance(v, dict)}), flush=True)
    json.dump(res, open(os.path.join(a.work, "engines.json"), "w"), indent=1)


if __name__ == "__main__":
    main()
