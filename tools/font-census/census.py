#!/usr/bin/env python3
"""Font-family census for the PDF backend (lane P3-FONTS-2, DESIGN.md §1.1
P-T2): does the engine write every font family of the TeX installation the
way pdfTeX does?

A *family* is a directory of TFM files under `fonts/tfm` of TeX Live's
texmf-dist (e.g. `public/amsfonts/cm`). How pdfTeX writes each font follows
from the font map (`pdftex.map`, as `kpsewhich` finds it) and the files:

  type1     a map entry with a Type 1 font file (`.pfb`/`.pfa`), or a
            built-in (non-embedded) font
  truetype  a map entry with a `.ttf`/`.ttc` file
  opentype  a map entry with an `.otf` file
  vf        no map entry, but a virtual font (its characters are other
            fonts' glyphs)
  pk        no map entry and no virtual font, but a METAFONT source: pdfTeX
            writes a Type 3 font from the PK file mktexpk makes

For each family, up to `--per-family` of its fonts (spread over the sorted
list) are typeset testfont-style, every character 0-255 of each at its
design size, by plain TeX in PDF mode with `\\pdfsuppressptexinfo=-1` under
`SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, once by the pinned pdfTeX (the
oracle, never in the product path) and once by the engine, each with a
plain.fmt it built itself. A family is **identical** when the two PDFs are
byte-identical (so their embedded font subsets, P-T2's measure, are too)
and the backend's log lines (`{map}`, `{enc}`, `<font file>`, `Output
written`) agree; **both-fail** when neither writes a PDF and they stop with
the same error; else it **fails**.

    python3 tools/font-census/census.py --engine target/release/flashtex-initex \\
        --texbin ~/texlive/2026/bin/x86_64-linux -j 12 --out /tmp/census

writes `census.json` and `census.md` into `--out`.
"""

import argparse
import concurrent.futures
import json
import os
import re
import shutil
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))

SETUP = (r"\batchmode \pdfoutput=1 \pdfsuppressptexinfo=-1 \pdfminorversion=7 "
         r"\pdfobjcompresslevel=2 \pdfcompresslevel=9 \pdfdecimaldigits=3 "
         r"\pdfpagewidth=8.5truein \pdfpageheight=11truein "
         r"\pdfhorigin=1truein \pdfvorigin=1truein \pdfpkresolution=600 "
         r"\hsize=6.5truein \vsize=9truein \parindent=0pt \tolerance=10000 "
         r"\hbadness=10000 \vbadness=10000 \hfuzz=\maxdimen \vfuzz=\maxdimen "
         r"\overfullrule=0pt \tracinglostchars=0 \pdfgentounicode=0 "
         "\n"
         r"\def\allchars#1{\font\x=#1 \x \count255=0 \loop \char\count255 "
         r"\hskip 0pt plus 1fil\advance\count255 1 \ifnum\count255<256 \repeat\par}"
         "\n")


def kpsewhich(texbin, *args):
    r = subprocess.run([os.path.join(texbin, "kpsewhich"), *args], capture_output=True, text=True)
    return r.stdout.strip()


def read_map(path):
    """tfm name -> kind (type1/truetype/opentype/pk-entry)."""
    kinds = {}
    with open(path, encoding="latin-1") as f:
        for line in f:
            line = line.strip()
            if not line or line[0] in "%#;*":
                continue
            tfm = line.split()[0]
            files = re.findall(r"<[<\[]?([^\s<\[]+)", line)
            ff = [x for x in files if not x.endswith(".enc")]
            if ff:
                low = ff[-1].lower()
                kind = ("truetype" if low.endswith((".ttf", ".ttc"))
                        else "opentype" if low.endswith(".otf") else "type1")
            else:
                toks = line.split()
                ps = [t for t in toks[1:] if not t.startswith("<") and not t.startswith('"')
                      and re.match(r"^[A-Za-z]", t)]
                kind = "type1" if ps else "pk"
            kinds.setdefault(tfm, kind)
    return kinds


def ls_r(texmf):
    """{directory (relative to texmf): [file names]} from ls-R."""
    out, cur = {}, None
    with open(os.path.join(texmf, "ls-R"), encoding="latin-1") as f:
        for line in f:
            line = line.rstrip("\n")
            if line.endswith(":"):
                cur = line[:-1]
                if cur.startswith("./"):
                    cur = cur[2:]
                out.setdefault(cur, [])
            elif line and cur is not None:
                out[cur].append(line)
    return out


def families(texbin, per_family):
    texmf = kpsewhich(texbin, "-var-value", "TEXMFDIST")
    tree = ls_r(texmf)
    names = {}
    for d, files in tree.items():
        for f in files:
            names.setdefault(f, d)
    vf = {f[:-3] for d, fs in tree.items() if d.startswith("fonts/vf") for f in fs if f.endswith(".vf")}
    mf = {f[:-3] for d, fs in tree.items() if d.startswith("fonts/source") for f in fs if f.endswith(".mf")}
    fmap = read_map(kpsewhich(texbin, "pdftex.map"))
    fams = []
    for d, files in sorted(tree.items()):
        if not d.startswith("fonts/tfm/"):
            continue
        tfms = sorted(f[:-4] for f in files if f.endswith(".tfm"))
        if not tfms:
            continue
        kinds = {}
        for t in tfms:
            if t in fmap:
                k = fmap[t]
            elif t in vf:
                k = "vf"
            elif t in mf:
                k = "pk"
            else:
                k = None  # pdfTeX would need mktexpk to find a source it has not got
            if k == "pk-entry":
                k = "pk"
            kinds[t] = k
        usable = [t for t in tfms if kinds[t]]
        if not usable:
            continue
        if len(usable) <= per_family:
            pick = usable
        else:
            step = (len(usable) - 1) / (per_family - 1)
            pick = sorted({usable[round(i * step)] for i in range(per_family)})
        counts = {}
        for t in pick:
            counts[kinds[t]] = counts.get(kinds[t], 0) + 1
        primary = max(sorted(counts), key=lambda k: counts[k])
        fams.append({"id": d[len("fonts/tfm/"):], "fonts": pick, "kinds": {t: kinds[t] for t in pick},
                     "kind": primary, "tfm_count": len(tfms)})
    return fams


def run(argv, cwd, env, timeout):
    t0 = time.time()
    try:
        r = subprocess.run(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                           stderr=subprocess.DEVNULL, timeout=timeout)
        code = r.returncode
    except subprocess.TimeoutExpired:
        code = "timeout"
    return code, time.time() - t0


def backend_lines(log, prog):
    text = "".join(log.splitlines()).replace(prog, "PROG")
    out = []
    for m in re.finditer(r"\{[^{}]*\.(?:map|enc|sfd)\}|<<?[^<>]*\.(?:pfb|pfa|ttf|ttc|otf|pgc|\d+pk)>>?"
                         r"|Output written on [^)]*\)|!pdfTeX error:[^.]*\.|! [^.]*\.", text):
        out.append(m.group(0))
    return out


def check(fam, cfg):
    work = os.path.join(cfg["out"], "work", fam["id"].replace("/", "__"))
    res = {"id": fam["id"], "kind": fam["kind"], "fonts": fam["fonts"], "kinds": fam["kinds"]}
    body = SETUP + "".join(r"\allchars{%s}" % t + "\n" for t in fam["fonts"]) + r"\bye" + "\n"
    out = {}
    for side in ("oracle", "engine"):
        d = os.path.join(work, side)
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, "doc.tex"), "w") as f:
            f.write(body)
        for ext in (".pdf", ".log"):
            p = os.path.join(d, "doc" + ext)
            if os.path.exists(p):
                os.remove(p)
        env = dict(cfg["env"])
        if side == "engine":
            env.update(cfg["engine_env"])
            env["FLASHTEX_FORMATS"] = cfg["fmt_engine"]
            argv = [cfg[side], "&plain", "doc"]
        else:
            env["TEXFORMATS"] = cfg["fmt_oracle"] + os.pathsep
            argv = [cfg[side], "-fmt=plain", "doc"]
        code, secs = run(argv, d, env, cfg["timeout"])
        pdf = os.path.join(d, "doc.pdf")
        log = os.path.join(d, "doc.log")
        data = open(pdf, "rb").read() if os.path.isfile(pdf) else None
        text = open(log, encoding="latin-1").read() if os.path.isfile(log) else ""
        out[side] = {"code": code, "seconds": round(secs, 2), "pdf": data,
                     "lines": backend_lines(text, cfg[side])}
        res[side] = {"code": code, "seconds": round(secs, 2), "pdf_bytes": len(data) if data else None}
    o, e = out["oracle"], out["engine"]
    if o["pdf"] is not None and e["pdf"] is not None:
        if o["pdf"] == e["pdf"] and o["lines"] == e["lines"]:
            res["result"] = "identical"
        else:
            res["result"] = "differs"
            if o["pdf"] != e["pdf"]:
                a, b = o["pdf"], e["pdf"]
                at = next((i for i in range(min(len(a), len(b))) if a[i] != b[i]), min(len(a), len(b)))
                res["why"] = f"PDF differs at byte {at} ({len(a)} vs {len(b)} bytes)"
            else:
                res["why"] = f"log lines {o['lines']} vs {e['lines']}"
    elif o["pdf"] is None and e["pdf"] is None:
        same = o["lines"] == e["lines"]
        res["result"] = "both-fail" if same else "differs"
        res["why"] = f"no PDF from either; errors {o['lines'][-2:]} vs {e['lines'][-2:]}"
    else:
        res["result"] = "differs"
        res["why"] = f"PDF only from the {'oracle' if o['pdf'] else 'engine'}; {o['lines'][-2:]} vs {e['lines'][-2:]}"
    if res["result"] in ("identical", "both-fail") and not cfg["keep"]:
        shutil.rmtree(work, ignore_errors=True)
    return res


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--engine", required=True, help="flashtex-initex")
    ap.add_argument("--pool", default=os.path.join(REPO, "crates", "flashtex-engine", "pdftex.pool"))
    ap.add_argument("--texbin", required=True, help="TeX Live's bin directory (the oracle pdftex, kpsewhich)")
    ap.add_argument("--out", required=True)
    ap.add_argument("--per-family", type=int, default=6)
    ap.add_argument("--only", action="append", default=[], help="family id prefix")
    ap.add_argument("--kind", action="append", default=[], help="only families of this kind")
    ap.add_argument("-j", type=int, default=8)
    ap.add_argument("--timeout", type=int, default=300)
    ap.add_argument("--keep", action="store_true", help="keep the work directories of passing families")
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
    env["PATH"] = a.texbin + os.pathsep + env.get("PATH", "")
    cfg = {"oracle": os.path.join(a.texbin, "pdftex"), "engine": os.path.abspath(a.engine), "env": env,
           "engine_env": {"FLASHTEX_POOL": os.path.abspath(a.pool)}, "out": os.path.abspath(a.out),
           "timeout": a.timeout, "keep": a.keep}
    # plain.fmt, each by its own program
    for side in ("oracle", "engine"):
        d = os.path.join(cfg["out"], "fmt-" + side)
        os.makedirs(d, exist_ok=True)
        e = dict(env)
        if side == "engine":
            e.update(cfg["engine_env"])
        run([cfg[side], "-ini", r"\input plain \dump"], d, e, 600)
        cfg["fmt_" + side] = d
        if not os.path.isfile(os.path.join(d, "plain.fmt")):
            sys.exit(f"census: no plain.fmt from the {side}")
    fams = families(a.texbin, a.per_family)
    if a.only:
        fams = [f for f in fams if any(f["id"].startswith(o) for o in a.only)]
    if a.kind:
        fams = [f for f in fams if f["kind"] in a.kind]
    print(f"census: {len(fams)} families, {sum(len(f['fonts']) for f in fams)} fonts", flush=True)
    results = []
    t0 = time.time()
    with concurrent.futures.ThreadPoolExecutor(max_workers=a.j) as ex:
        futs = {ex.submit(check, f, cfg): f for f in fams}
        for n, fut in enumerate(concurrent.futures.as_completed(futs), 1):
            f = futs[fut]
            try:
                r = fut.result()
            except Exception as exc:  # noqa: BLE001
                r = {"id": f["id"], "kind": f["kind"], "result": "error", "why": f"{type(exc).__name__}: {exc}"}
            results.append(r)
            if r["result"] not in ("identical", "both-fail"):
                print(f"{n}/{len(fams)} {r['result']:9} {r['id']} ({r['kind']}): {r.get('why')}", flush=True)
    results.sort(key=lambda r: r["id"])
    kinds = sorted({r["kind"] for r in results})
    table = {}
    for k in kinds + ["all"]:
        rs = [r for r in results if k == "all" or r["kind"] == k]
        table[k] = {"tested": len(rs), "identical": sum(r["result"] == "identical" for r in rs),
                    "both-fail": sum(r["result"] == "both-fail" for r in rs),
                    "failing": [r["id"] for r in rs if r["result"] not in ("identical", "both-fail")]}
    summary = {"seconds": round(time.time() - t0, 1), "per_family": a.per_family, "by_kind": table,
               "oracle": subprocess.run([cfg["oracle"], "--version"], capture_output=True, text=True)
               .stdout.splitlines()[0]}
    with open(os.path.join(a.out, "census.json"), "w") as f:
        json.dump({"summary": summary, "results": results}, f, indent=1)
    lines = ["| kind | families tested | identical | both fail (same error) | failing |",
             "|---|---|---|---|---|"]
    for k in kinds + ["all"]:
        t = table[k]
        lines.append(f"| {k} | {t['tested']} | {t['identical']} | {t['both-fail']} | {len(t['failing'])} |")
    fails = [r for r in results if r["result"] not in ("identical", "both-fail")]
    if fails:
        lines += ["", "Failing families:", ""]
        lines += [f"- `{r['id']}` ({r['kind']}): {r.get('why')}" for r in fails]
    both = [r for r in results if r["result"] == "both-fail"]
    if both:
        lines += ["", "Families neither program writes (same error):", ""]
        lines += [f"- `{r['id']}` ({r['kind']}): {r.get('why')}" for r in both]
    with open(os.path.join(a.out, "census.md"), "w") as f:
        f.write("\n".join(lines) + "\n")
    print("\n".join(lines[: 2 + len(kinds) + 1]))
    return 0 if not fails else 1


if __name__ == "__main__":
    sys.exit(main())
