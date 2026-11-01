#!/usr/bin/env python3
"""Real .idx files, oracle vs port, byte for byte, with and without styles.

    corpus.py --port <binary> --idx LABEL=PATH [--idx ...] [--local-ist PATH ...]
              [--jobs N] [--out results.jsonl]

Every .idx runs with no style, with each of TeX Live 2026's 54 .ist files
(found by kpathsea, as in a document's `-s gind.ist`), and with each
`--local-ist` file (copied beside the .idx, given as `-s NAME`), each under
the option sets of OPTIONS. Each run happens in a fresh directory holding
the .idx (and the local style), as mkicmp does.
"""

import argparse
import glob
import json
import multiprocessing
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mkicmp  # noqa: E402

TL_STYLES = sorted(os.path.basename(p) for p in glob.glob(
    "/usr/local/texlive/2026/texmf-dist/makeindex/**/*.ist", recursive=True))

OPTIONS = [[], ["-q"], ["-l"], ["-r", "-c"]]


def job(spec):
    label, idx_name, idx_data, style, local, opts, port = spec
    files = {idx_name: idx_data}
    args = list(opts)
    if style:
        args += ["-s", style]
        if local is not None:
            files[style] = local
    args.append(idx_name)
    try:
        d = mkicmp.compare(port, files, args, timeout=600)
    except Exception as e:
        return {"label": label, "args": args, "ok": False, "what": "harness: %r" % e}
    st = mkicmp.LAST_STATUS.get("oracle")
    if d is None:
        return {"label": label, "args": args, "ok": True, "status": st}
    return {"label": label, "args": args, "ok": False, "status": st, "what": mkicmp.describe(d)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", required=True)
    ap.add_argument("--idx", action="append", default=[])
    ap.add_argument("--local-ist", action="append", default=[])
    ap.add_argument("--jobs", type=int, default=8)
    ap.add_argument("--out")
    a = ap.parse_args()
    port = os.path.abspath(a.port)
    locals_ = [(os.path.basename(p), open(p, "rb").read()) for p in a.local_ist]
    specs = []
    for item in a.idx:
        label, path = item.split("=", 1)
        data = open(path, "rb").read()
        name = os.path.basename(path)
        styles = [(None, None)] + [(s, None) for s in TL_STYLES] + locals_
        for style, local in styles:
            for opts in OPTIONS:
                specs.append((label, name, data, style, local, opts, port))
    out = open(a.out, "w") if a.out else None
    n = bad = 0
    per = {}
    with multiprocessing.Pool(a.jobs) as pool:
        for res in pool.imap_unordered(job, specs, chunksize=2):
            n += 1
            p = per.setdefault(res["label"], [0, 0])
            p[0] += 1
            if out:
                out.write(json.dumps(res) + "\n")
            if not res["ok"]:
                bad += 1
                p[1] += 1
                if bad <= 20:
                    print("%s %s: %s" % (res["label"], res["args"], res["what"]), flush=True)
    for label, (k, b) in sorted(per.items()):
        print("%-28s runs %5d mismatches %d" % (label, k, b))
    print("styles: none + %d TeX Live + %d local; option sets %d" % (len(TL_STYLES), len(locals_), len(OPTIONS)))
    print("total runs %d mismatches %d" % (n, bad))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
