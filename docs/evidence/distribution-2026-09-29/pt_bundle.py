#!/usr/bin/env python3
"""P-T1/P-T2 of the parity fixtures with the engine reading from a bundle.

The parity scoreboard (tools/parity) with one addition: the candidate's log
names files under the bundle's cache tree (`<cache>/<digest>/tree/texmf-dist/
...`), where the oracle's names them under its TeX Live
(`/usr/local/texlive/2026/texmf-dist/...`). The bundle keeps TeX Live's
relative paths, so replacing the one prefix makes the two comparable; nothing
else is normalised beyond what tools/parity does. Everything else -- the runs
to convergence, the traced pass, the cached oracle, the P-T1 and P-T2
comparisons -- is tools/parity's own code, imported.

    python3 pt_bundle.py --engine target/release/flashtex-initex \
        --bundle fixtures.ttb --work /tmp/pt-bundle --cache <parity cache> \
        --texmfroot /usr/local/texlive/2026 [-j 6] [--out result.json]

Prints one line per document and the totals; with --out, writes per-document
records. Oracle tooling only: the engine under test is FlashTeX.
"""
import argparse
import concurrent.futures
import json
import os
import struct
import sys

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
sys.path.insert(0, os.path.join(REPO, "tools", "parity"))
import parity  # noqa: E402
import tiers as ptiers  # noqa: E402


def bundle_digest(path):
    with open(path, "rb") as f:
        h = f.read(66)
    assert h[:14] == b"tectonicbundle" and struct.unpack_from("<I", h, 14)[0] == 1
    return h[34:66].hex()


def one(doc, cfg):
    env = cfg["env"]
    out_dir = os.path.join(cfg["work"], doc["id"].replace("/", "__"))
    os.makedirs(out_dir, exist_ok=True)
    meta, cap, pdf = ptiers.run_tex(doc, cfg["engine"], os.path.join(out_dir, "src"), trace=True, extra_env=env)
    rec = {"id": doc["id"], "candidate_ok": meta["ok"], "why": meta.get("why")}
    if cap is not None:
        tree = cfg["tree"].rstrip("/") + "/"
        cap = ptiers.pcapture.Capture(cap.log.replace(tree, cfg["root"]),
                                      [b.replace(tree, cfg["root"]) for b in cap.boxes], cap.pdf_path)
    ometa, ref_cap, ref_pdf = ptiers.oracle(doc, cfg["oracle"], cfg["cache"], True, parity.tree_hash(doc["dir"]))
    if not ref_pdf:
        rec["excluded"] = ometa.get("why")
        return rec
    rec["P-T1"] = False
    if cap is not None:
        pt1 = ptiers.compare_pt1(ref_cap, cap)
        rec["P-T1"] = pt1["ok"]
        if not pt1["ok"]:
            rec["pt1"] = {k: pt1.get(k) for k in ("log_line", "box_line", "first_shipout", "shipouts")}
    pt2 = ptiers.compare_pt2(ref_pdf, pdf, os.path.join(out_dir, "pt2")) if pdf else {"ok": False}
    rec["P-T2"] = pt2["ok"]
    return rec


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--engine", required=True)
    ap.add_argument("--bundle", required=True)
    ap.add_argument("--work", required=True)
    ap.add_argument("--cache", default=parity.pcorpus.default_cache())
    ap.add_argument("--oracle-pdftex", default="/Library/TeX/texbin/pdftex")
    ap.add_argument("--texmfroot", default="/usr/local/texlive/2026")
    ap.add_argument("--pool", default=os.path.join(REPO, "crates/flashtex-engine/pdftex.pool"))
    ap.add_argument("--only", action="append", default=[])
    ap.add_argument("-j", type=int, default=6)
    ap.add_argument("--out")
    a = ap.parse_args()
    digest = bundle_digest(a.bundle)
    work = os.path.realpath(a.work)
    os.makedirs(work, exist_ok=True)
    bundles = os.path.join(work, "bundles")
    env = {
        "FLASHTEX_POOL": os.path.abspath(a.pool),
        "FLASHTEX_RESOLVER": "bundle",
        "FLASHTEX_BUNDLE_URL": "file://" + os.path.abspath(a.bundle),
        "FLASHTEX_BUNDLE_DIGEST": digest,
        "FLASHTEX_BUNDLE_CACHE_DIR": bundles,
        "FLASHTEX_FORMAT_CACHE_DIR": os.path.join(work, "formats"),
    }
    cfg = {"engine": os.path.abspath(a.engine), "env": env, "work": work, "cache": a.cache,
           "oracle": a.oracle_pdftex, "tree": os.path.join(bundles, digest, "tree"),
           "root": a.texmfroot.rstrip("/") + "/"}
    docs = parity.fixture_documents(only=tuple(a.only))
    recs = []
    with concurrent.futures.ProcessPoolExecutor(max_workers=a.j) as ex:
        for r in ex.map(one, docs, [cfg] * len(docs)):
            recs.append(r)
            mark = lambda v: {True: "pass", False: "fail", None: "n/a"}[v]  # noqa: E731
            print(f"{r['id']}: P-T1={mark(r.get('P-T1'))} P-T2={mark(r.get('P-T2'))}"
                  + (f"  {r.get('pt1')}" if r.get("P-T1") is False else ""), flush=True)
    n = len([r for r in recs if "excluded" not in r])
    print(f"bundle {digest}: {n} measured, P-T1 {sum(1 for r in recs if r.get('P-T1'))}/{n}, "
          f"P-T2 {sum(1 for r in recs if r.get('P-T2'))}/{n}")
    if a.out:
        with open(a.out, "w", encoding="utf-8") as f:
            json.dump(recs, f, indent=1)


if __name__ == "__main__":
    main()
