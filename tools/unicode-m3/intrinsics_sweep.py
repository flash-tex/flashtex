#!/usr/bin/env python3
"""XeTeX guarded intrinsics over whole documents (crates/flashtex-xetex/src/intrinsics.rs).

    python3 tools/unicode-m3/intrinsics_sweep.py --engine BIN --work DIR
        [--cases-dir DIR ...] [--t7 10,120] [--jobs 4] [--pdf] [--json OUT]

Each document (the cases of every `--cases-dir`, as `corpus.py assemble` lays
them out, and T7's `full-N` as `mode_gap.py` writes it for the `unicode` and
`unicode-fontspec` variants) is compiled twice (the second run reads the
first's aux data) with `FLASHTEX_INTRINSICS` off, on and verify, by the
engine with its own xelatex format (built here as latex.py builds it). The
verify runs set `FLASHTEX_INTRINSICS_VERIFY_FAIL=1`. A document passes when
the three modes' logs and XDV files (and with `--pdf`, FlashTeX's PDFs of a
third run each) are byte-identical, no verify run differs, and every run's
exit status is the off run's. Prints one line per document and the totals
of the verify reports (offers, admitted, replayed, checked, identical,
differing).
"""
import argparse
import json
import os
import shutil
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, os.path.join(REPO, "tools", "xetex-lockstep"))
sys.path.insert(0, HERE)
import latex as L  # noqa: E402
import mode_gap  # noqa: E402

MODES = ("off", "on", "verify")
KEYS = ("offers", "admitted", "replayed", "verify.checked", "verify.identical",
        "verify.differing", "recordings.committed")


def stats_of(path):
    out = {}
    if os.path.exists(path):
        for line in open(path, encoding="utf-8"):
            k, _, v = line.rstrip("\n").partition(": ")
            if k == "verify.detail":
                out.setdefault("details", []).append(v)
                continue
            try:
                out[k] = int(v)
            except ValueError:
                pass
    return out


def run_doc(name, src, a, fmt):
    meta = L.case_meta(src) if os.path.isdir(src) else {}
    res = {"name": name, "modes": {}}
    outs = {}
    for mode in MODES:
        d = os.path.join(a.work, "docs", name, mode)
        shutil.rmtree(d, ignore_errors=True)
        L.stage(src, name, d)
        exe = L.link(a.engine, os.path.join(d, ".bin"), "xelatex")
        env = L.env_for(fmt)
        env["FLASHTEX_INTRINSICS"] = mode
        env.pop("FLASHTEX_INTRINSICS_FAULT", None)
        stats = os.path.join(d, ".intr-stats")
        env["FLASHTEX_INTRINSICS_STATS"] = stats
        if mode == "verify":
            env["FLASHTEX_INTRINSICS_VERIFY_FAIL"] = "1"
        codes = []
        totals = {}
        args = L.RUN_ARGS[:]
        runs = [args, args] + ([[x for x in args if x != "-no-pdf"]] if a.pdf else [])
        files = {}
        for i, run_args in enumerate(runs):
            try:
                r = subprocess.run([exe] + run_args + [L.doc_arg(name, meta, False)], cwd=d, env=env,
                                   stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, timeout=a.timeout)
                codes.append(r.returncode)
                if r.returncode == 3 and mode == "verify":
                    res.setdefault("stderr", []).append(r.stderr.decode("utf-8", "replace")[-400:])
            except subprocess.TimeoutExpired:
                codes.append("timeout")
            s = stats_of(stats)
            for k in KEYS:
                totals[k] = totals.get(k, 0) + s.get(k, 0)
            if s.get("details"):
                res.setdefault("details", []).extend(s["details"][:4])
            if os.path.exists(stats):
                os.remove(stats)
            # what this run wrote: the second run's log and XDV, the third's PDF
            for ext in (["log", "xdv"] if i == 1 else ["pdf"] if i == 2 else []):
                p = os.path.join(d, name + "." + ext)
                files[ext] = open(p, "rb").read() if os.path.exists(p) else None
        outs[mode] = files
        res["modes"][mode] = {"codes": codes, "stats": totals}
    off = outs["off"]
    res["same"] = {m: {ext: outs[m].get(ext) == off.get(ext) for ext in off} for m in ("on", "verify")}
    res["codes_same"] = all(res["modes"][m]["codes"] == res["modes"]["off"]["codes"] for m in MODES)
    v = res["modes"]["verify"]["stats"]
    res["ok"] = (res["codes_same"] and all(all(x.values()) for x in res["same"].values())
                 and v.get("verify.differing", 0) == 0)
    return res


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--engine", required=True)
    ap.add_argument("--work", required=True)
    ap.add_argument("--cases-dir", action="append", default=[])
    ap.add_argument("--t7", default="", help="page counts of T7's full-N, e.g. 10,120")
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--timeout", type=float, default=900)
    ap.add_argument("--pdf", action="store_true")
    ap.add_argument("--json")
    a = ap.parse_args()
    a.engine = os.path.abspath(a.engine)
    a.work = os.path.abspath(a.work)
    os.makedirs(a.work, exist_ok=True)
    fmt = os.path.join(a.work, "fmt")
    if not os.path.exists(os.path.join(fmt, "xelatex.fmt")):
        os.makedirs(fmt, exist_ok=True)
        secs, code, _ = L.build_format(a.engine, fmt)
        print("format: exit %s, %.1f s" % (code, secs), flush=True)
    docs = dict(L.find_cases(a.cases_dir)) if a.cases_dir else {}
    for n in [int(x) for x in a.t7.split(",") if x]:
        for variant in ("unicode", "unicode-fontspec"):
            d = os.path.join(a.work, "t7", "%s-%d" % (variant, n))
            os.makedirs(d, exist_ok=True)
            name = "t7-%s-%d" % (variant, n)
            with open(os.path.join(d, name + ".tex"), "w", encoding="utf-8") as f:
                f.write(mode_gap.source(variant, n))
            docs[name] = d
    results = []
    with ThreadPoolExecutor(a.jobs) as ex:
        for r in ex.map(lambda kv: run_doc(kv[0], kv[1], a, fmt), sorted(docs.items())):
            v = r["modes"]["verify"]["stats"]
            print("%-40s %s codes %s verify %s same %s%s" % (
                r["name"], "ok  " if r["ok"] else "FAIL", r["modes"]["off"]["codes"],
                " ".join("%s=%d" % (k.split(".")[-1], v.get(k, 0)) for k in KEYS),
                {m: [e for e, s in x.items() if not s] or "all" for m, x in r["same"].items()},
                (" " + str(r.get("details"))) if r.get("details") else ""), flush=True)
            results.append(r)
    tot = {m: {k: sum(r["modes"][m]["stats"].get(k, 0) for r in results) for k in KEYS} for m in MODES}
    bad = [r["name"] for r in results if not r["ok"]]
    print("documents %d, failing %d %s" % (len(results), len(bad), bad))
    for m in ("on", "verify"):
        print("%s totals: %s" % (m, " ".join("%s=%d" % (k, tot[m][k]) for k in KEYS)))
    if a.json:
        with open(a.json, "w") as f:
            json.dump({"results": [{k: v for k, v in r.items()} for r in results], "totals": tot}, f,
                      default=lambda o: None, indent=1)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
