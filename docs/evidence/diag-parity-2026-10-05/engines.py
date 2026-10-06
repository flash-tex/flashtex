#!/usr/bin/env python3
"""Run the DIAG-PARITY corpus through the old engine and the new engine's host.

usage: engines.py WORKDIR --host flashtex-host --client dl3-client --old flashtex-compiler [--only id,...]

* new: one `flashtex-host --socket` (external tools on, so BibTeX runs), and
  per case `dl3-client --diag` (diag-v1): every DIAG, one JSON object a line.
* old: `flashtex-compiler` (runtime-v1 JSON on stdin), every diagnostic.

Writes WORKDIR/engines.json: {case: {"new": [diag...], "old": [diag...]}}.
The engine's output is only read; MacTeX is not involved.
"""
import argparse
import json
import os
import shutil
import subprocess
import time

HERE = os.path.dirname(os.path.abspath(__file__))


def old_run(compiler, files):
    docs = [{"path": p, "text": t} for p, t in files.items()]
    req = {"protocol_version": 1, "type": "compile", "id": "dp-1",
           "payload": {"project_id": "dp", "revision": 1, "entry_path": "main.tex", "documents": docs}}
    p = subprocess.run([compiler], input=(json.dumps(req) + "\n").encode(), stdout=subprocess.PIPE,
                       stderr=subprocess.DEVNULL, timeout=300)
    first = p.stdout.split(b"\n", 1)[0]
    try:
        env = json.loads(first)
    except Exception:
        return {"raw": first[:300].decode(errors="replace")}
    r = env.get("payload", env)
    return {"status": r.get("status"), "diags": r.get("diagnostics", [])}


def new_run(client, sock, d):
    out = os.path.join(d, "diag.jsonl")
    subprocess.run([client, "--socket", sock, "--root", d, "--main", "main.tex", "--diag", out, "--quiet"],
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=300)
    if not os.path.exists(out):
        return {"diags": []}
    return {"diags": [json.loads(l) for l in open(out) if l.strip()]}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("work")
    ap.add_argument("--host")
    ap.add_argument("--client")
    ap.add_argument("--old")
    ap.add_argument("--only")
    a = ap.parse_args()
    corpus = json.load(open(os.path.join(HERE, "corpus.json")))
    only = set(a.only.split(",")) if a.only else None
    os.makedirs(a.work, exist_ok=True)
    sock = os.path.join(a.work, "h.sock")
    host = None
    if a.host:
        if os.path.exists(sock):
            os.remove(sock)
        host = subprocess.Popen([a.host, "--socket", sock, "--external-tools", "auto"],
                                stdout=subprocess.DEVNULL, stderr=open(os.path.join(a.work, "host.err"), "w"))
        for _ in range(600):
            if os.path.exists(sock):
                break
            time.sleep(0.1)
    res = {}
    try:
        for c in corpus["cases"]:
            if only and c["id"] not in only:
                continue
            r = {}
            if host:
                d = os.path.join(a.work, c["id"])
                shutil.rmtree(d, ignore_errors=True)
                for p, t in c["files"].items():
                    os.makedirs(os.path.dirname(os.path.join(d, p)), exist_ok=True)
                    open(os.path.join(d, p), "w").write(t)
                r["new"] = new_run(a.client, sock, d)
            if a.old:
                r["old"] = old_run(a.old, c["files"])
            res[c["id"]] = r
            print(c["id"], {k: len(v.get("diags", [])) for k, v in r.items()}, flush=True)
    finally:
        if host:
            host.terminate()
            host.wait()
    json.dump(res, open(os.path.join(a.work, "engines.json"), "w"), indent=1)


if __name__ == "__main__":
    main()
