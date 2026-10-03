#!/usr/bin/env python3
"""Pick arXiv documents that exercise external tools: bibtex (a \\bibliography
with its .bib present), biblatex+biber, makeindex. Prints 'DIR MAIN KIND' lines."""
import json, os, re, sys
repo = sys.argv[1]
cache = os.path.expanduser("~/.cache/flashtex-parity/src/arxiv")
m = json.load(open(os.path.join(repo, "tools/parity/corpus/arxiv-2025-01.json")))
excluded = {"2501.06999v1", "2501.07032v4", "2501.07072v1", "2501.07184v1", "2501.07190v1", "2501.07482v2",
            "2501.08928v2", "2501.09091v1", "2501.07495v1", "2501.07457v1", "2501.08371v3", "2501.08775v2",
            "2501.07077v1", "2501.10230v1"}
rows = []
for e in m["entries"]:
    d = os.path.join(cache, e["id"])
    if not os.path.isdir(d) or e["id"] in excluded:
        continue
    main = e["entry"]
    texs = []
    for dp, _, fs in os.walk(d):
        for f in fs:
            if f.endswith(".tex"):
                try:
                    texs.append(open(os.path.join(dp, f), errors="replace").read())
                except OSError:
                    pass
    t = "\n".join(re.sub(r"(?<!\\)%.*", "", x) for x in texs)
    bibs = [f for dp, _, fs in os.walk(d) for f in fs if f.endswith(".bib")]
    kind = None
    if re.search(r"\\usepackage(\[[^\]]*\])?\{biblatex\}", t) and bibs:
        kind = "biber" if not re.search(r"backend\s*=\s*bibtex", t) else "biblatex-bibtex"
    elif re.search(r"\\bibliography\{", t) and bibs:
        kind = "bibtex"
    idx = bool(re.search(r"\\makeindex", t)) and bool(re.search(r"\\printindex", t))
    if kind or idx:
        rows.append((d, main, kind or "-", "index" if idx else "-"))
for r in rows:
    print(*r)
print(f"# {len(rows)}", file=sys.stderr)
