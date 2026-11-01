#!/usr/bin/env python3
"""TeX Live's styles and databases, oracle vs port, byte for byte.

    corpus.py --port <binary> [--tier styles|bibs|all] [--jobs N] [--out results.jsonl]

* tier `styles`: every .bst of TeX Live 2026 (found by kpathsea, as
  `\\bibstyle{NAME}` finds it) on each database set of SETS, with
  `\\citation{*}` (every entry) and one cited key before it;
* tier `bibs`: every .bib of TeX Live 2026 with each style of STYLES.

Each case is an .aux in a fresh directory; bibtex runs on it there
(btcmp.py). The .bib and .bst files come from TeX Live through kpathsea, for
the oracle and the port alike.
"""

import argparse
import glob
import json
import multiprocessing
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import btcmp  # noqa: E402

TL = "/usr/local/texlive/2026/texmf-dist"
BSTS = sorted({os.path.basename(p)[:-4] for p in glob.glob(TL + "/bibtex/bst/**/*.bst", recursive=True)})
BIBS = sorted({os.path.basename(p)[:-4] for p in glob.glob(TL + "/bibtex/bib/**/*.bib", recursive=True)})

SETS = [
    ("xampl", ["xampl"], "knuth-ab"),
    ("ieee", ["IEEEabrv", "IEEEexample"], "IEEEexample:BSTcontrolhelp"),
    ("biblatex-examples", ["biblatex-examples"], "knuth:ct"),
    ("tugboat", ["tugboat"], "Beebe:1993:BBT"),
]

STYLES = ["plain", "alpha", "abbrv", "unsrt", "ieeetr", "acm", "apalike", "siam",
          "amsplain", "amsalpha", "plainnat", "abbrvnat", "unsrtnat", "IEEEtran",
          "apsrev4-2", "chicago", "named", "elsarticle-num", "agsm", "jss"]


def aux(style, bibs, key):
    return ("\\relax\n\\citation{%s}\n\\citation{*}\n\\bibstyle{%s}\n\\bibdata{%s}\n"
            % (key, style, ",".join(bibs))).encode()


def job(spec):
    label, data, port = spec
    try:
        d = btcmp.compare(port, {"doc.aux": data}, ["doc"], timeout=600)
    except Exception as e:  # noqa: BLE001
        return {"label": label, "ok": False, "what": "harness: %r" % e}
    st = btcmp.LAST_STATUS.get("oracle")
    if d is None:
        return {"label": label, "ok": True, "status": st}
    return {"label": label, "ok": False, "status": st, "what": btcmp.describe(d)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", required=True)
    ap.add_argument("--tier", default="all")
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--out")
    a = ap.parse_args()
    specs = []
    if a.tier in ("styles", "all"):
        for bst in BSTS:
            for name, bibs, key in SETS:
                specs.append(("style %s on %s" % (bst, name), aux(bst, bibs, key), a.port))
    if a.tier in ("bibs", "all"):
        for bib in BIBS:
            for st in STYLES:
                specs.append(("bib %s with %s" % (bib, st), aux(st, [bib], "x"), a.port))
    out = open(a.out, "w") if a.out else None
    bad = 0
    statuses = {}
    with multiprocessing.Pool(a.jobs) as pool:
        for r in pool.imap_unordered(job, specs):
            statuses[str(r.get("status"))] = statuses.get(str(r.get("status")), 0) + 1
            if out:
                out.write(json.dumps(r) + "\n")
                out.flush()
            if not r["ok"]:
                bad += 1
                print("MISMATCH", r["label"], "\n ", r["what"].replace("\n", "\n  "), flush=True)
    print("cases %d mismatches %d; oracle exit statuses %s; styles %d, databases %d"
          % (len(specs), bad, json.dumps(statuses, sort_keys=True), len(BSTS), len(BIBS)))
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
