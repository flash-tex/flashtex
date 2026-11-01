#!/usr/bin/env python3
"""What the Unicode corpus reads that the no-TeX-Live bundle lacks (lane
M3-BUNDLE; M3's "a no-TeX-Live run of the corpus from the bundle").

    python3 tools/unicode-m3/bundle_gaps.py --root TEXMFROOT --cases-dir DIR
        [--xelatex BIN] [--cases GLOB ...] [--passes N] [-j N] [--json OUT]

Each case runs through TeX Live's own `xelatex -recorder -no-pdf` (the
oracle, in the pinned tree at --root), as many passes as its `.aux` asks
(at most --passes). Every file a pass reads is taken from its `.fls`
(kpathsea's lookups), and every font file from the XDV's
`define_native_font` records (xetex finds fonts by name through fontconfig,
which `-recorder` does not see). Each file under --root is mapped to the
TeX Live package owning it (the tree's tlpdb, as `texbundle.py derive`
maps it), and compared with the packages the bundle packs: packages.txt,
extra.txt and the owners of core.txt's files (`texbundle.py pack`).

The report lists, per case, the packages it reads that the bundle lacks
(and the files it reads from them), then each missing package once, with
the licence the tlpdb records and whether the bundle's packer would accept
it (`texbundle.py licence_problems`). Files read from outside the tree
(system fonts) are listed as not TeX Live's.
"""
import argparse
import fnmatch
import json
import os
import re
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, os.path.join(REPO, "tools", "xetex-lockstep"))
sys.path.insert(0, os.path.join(REPO, "tools", "bundle"))
import latex as L  # noqa: E402
import texbundle as T  # noqa: E402

FONT_PATH = re.compile(rb"/[^\x00-\x1f\"<>|]*?\.(?:otf|ttf|ttc|otc|pfb|OTF|TTF|TTC)")
RERUN = re.compile(r"Rerun|re-?run|Label\(s\) may have changed|undefined references", re.I)


def run_case(a, name, src, work):
    meta = L.case_meta(src)
    d = os.path.join(work, name)
    L.stage(src, name, d)
    env = L.X.pinned_env()
    argv = [a.xelatex, "-recorder"] + L.RUN_ARGS + [L.doc_arg(name, meta, False)]
    reads, fonts, passes, code = set(), set(), 0, None
    log = ""
    for _ in range(a.passes):
        passes += 1
        try:
            r = subprocess.run(argv, cwd=d, env=env, stdout=subprocess.DEVNULL,
                               stderr=subprocess.DEVNULL, timeout=a.timeout)
            code = r.returncode
        except subprocess.TimeoutExpired:
            code = "timeout"
        # -recorder names the .fls after the job, and with a first line the
        # job is `texput` until \input; take every .fls the pass wrote
        for fn in os.listdir(d):
            if fn.endswith(".fls"):
                for line in open(os.path.join(d, fn), errors="replace"):
                    if line.startswith("INPUT "):
                        p = line[6:].strip()
                        reads.add(os.path.realpath(p if p.startswith("/") else os.path.join(d, p)))
        xdv = os.path.join(d, name + ".xdv")
        if os.path.exists(xdv):
            for m in FONT_PATH.findall(open(xdv, "rb").read()):
                p = m.decode("utf-8", "replace")
                if os.path.isfile(p):
                    fonts.add(os.path.realpath(p))
        logp = os.path.join(d, name + ".log")
        log = open(logp, errors="replace").read() if os.path.exists(logp) else ""
        if code == "timeout" or not RERUN.search(log):
            break
    first_error = next((ln for ln in log.split("\n") if ln.startswith("! ")), None)
    not_found = sorted(set(re.findall(
        r"! (?:LaTeX|Package) [^\n]*?(?:File|font) [`'\"]?([^'\"\n]+?)['\"]? (?:not found|cannot be found)", log)))
    return {"case": name, "family": meta.get("family"), "exit": code, "passes": passes,
            "first_error": first_error, "not_found": not_found,
            "reads": sorted(reads | fonts), "fonts": sorted(fonts)}


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--root", required=True, help="TEXMFROOT of the pinned tree")
    ap.add_argument("--cases-dir", action="append", required=True)
    ap.add_argument("--cases", nargs="*", default=["*"])
    ap.add_argument("--xelatex", default="xelatex")
    ap.add_argument("--passes", type=int, default=4)
    ap.add_argument("--timeout", type=int, default=1800)
    ap.add_argument("-j", "--jobs", type=int, default=4)
    ap.add_argument("--packages", default=os.path.join(T.DATA, "packages.txt"))
    ap.add_argument("--extra", default=os.path.join(T.DATA, "extra.txt"))
    ap.add_argument("--core", default=os.path.join(T.DATA, "core.txt"))
    ap.add_argument("--json")
    a = ap.parse_args(argv)
    root = os.path.realpath(a.root)
    db = T.parse_tlpdb(T.merged_tlpdb_text(root))
    owner = {}
    for p, e in db.items():
        for f in e["runfiles"]:
            owner.setdefault(f, p)
    core = T.read_list(a.core)
    bundled = set(T.bundle_packages(a.packages, a.extra)) | {owner[c] for c in core if c in owner}
    core_files = set(core)

    srcs = L.find_cases(a.cases_dir)
    names = [n for n in sorted(srcs) if any(fnmatch.fnmatch(n, p) for p in a.cases)]
    work = tempfile.mkdtemp(prefix="bundle-gaps-")
    # the first alone: a first xetex run may build its font caches
    results = [run_case(a, names[0], srcs[names[0]], work)] if names else []
    with ThreadPoolExecutor(a.jobs) as ex:
        results += list(ex.map(lambda n: run_case(a, n, srcs[n], work), names[1:]))

    missing_all = {}
    for r in results:
        miss, outside, unowned = {}, [], []
        for p in r.pop("reads"):
            if p.startswith(work + "/"):
                continue  # the case's own files
            if not p.startswith(root + "/"):
                if not p.startswith(("/dev/", "/proc/", "/etc/")):
                    outside.append(p)
                continue
            rel = p[len(root) + 1:]
            if rel in core_files:
                continue
            pk = owner.get(rel)
            if pk is None:
                if not rel.startswith("texmf-var/"):
                    unowned.append(rel)
                continue
            if T.platform_package(pk) or pk in bundled:
                continue
            miss.setdefault(pk, []).append(rel)
        r["missing_packages"] = {k: sorted(v) for k, v in sorted(miss.items())}
        r["outside_tree"] = sorted(outside)
        r["unowned"] = sorted(unowned)
        for pk in miss:
            missing_all.setdefault(pk, []).append(r["case"])
    packages = []
    for pk in sorted(missing_all):
        lic = T.licence_of(db, pk)
        bad = T.licence_problems(db, [pk])
        packages.append({"package": pk, "licence": lic or None, "redistributable": not bad,
                         "problem": bad[0] if bad else None, "cases": missing_all[pk]})

    for r in results:
        print("%-34s exit %-4s passes %d missing: %s%s" % (
            r["case"], r["exit"], r["passes"], " ".join(r["missing_packages"]) or "-",
            ("; outside TeX Live: %s" % " ".join(os.path.basename(x) for x in r["outside_tree"])) if r["outside_tree"] else ""))
        if r["first_error"]:
            print("    first error: %s" % r["first_error"][:200])
    print("\n%d packages the bundle lacks:" % len(packages))
    for p in packages:
        print("  %-28s %-24s %s  (%s)" % (p["package"], p["licence"] or "(none recorded)",
                                         "free" if p["redistributable"] else "NOT REDISTRIBUTABLE",
                                         ", ".join(p["cases"])))
    if a.json:
        with open(a.json, "w") as f:
            json.dump({"root": root, "cases": results, "packages": packages}, f, indent=1)
    return 0


if __name__ == "__main__":
    sys.exit(main())
