#!/usr/bin/env python3
"""File-resolver evaluation for P1-TEX82-CORE (DESIGN.md §4.4, §1 "reuse
before building"). Reproduce with

    python3 docs/evidence/file-resolver-2026-09-29/run.py

on a machine with TeX Live 2026. It

1. builds a deterministic corpus (corpus.tsv) of >= 500 (format, name) lookups
   from the installation's own ls-R, covering every kind of file the engine
   opens plus the awkward cases: names without a suffix, basenames that occur
   in several directories, names only a non-pdflatex path would find, wrong
   case, and names that do not exist;
2. asks `kpsewhich -progname=pdflatex -engine=pdftex -format=F NAME` for each
   (the reference), one process per name, and also once per format in batch;
3. resolves the same corpus through flashtex-engine's KpathseaResolver
   (examples/resolver_corpus.rs) in three environments -- the login shell's,
   a launchd-like one as a GUI app gets (HOME, USER, TMPDIR and
   PATH=/usr/bin:/bin:/usr/sbin:/sbin), and an empty one;
4. copies every file kpsewhich found into one flat directory -- a
   Tectonic-style bundle -- and resolves the corpus against that;
5. writes results.json and prints the summary quoted in README.md.
"""
import json
import os
import random
import shutil
import statistics
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "../../.."))


def sh(*args, **kw):
    return subprocess.run(args, capture_output=True, text=True, **kw)


def build_corpus(dist):
    files = {}  # basename -> list of relative dirs
    cur = ""
    for line in open(os.path.join(dist, "ls-R"), encoding="latin-1"):
        line = line.rstrip("\n")
        if line.endswith(":") and line.startswith("./"):
            cur = line[2:-1]
            continue
        if not line or line.startswith("%"):
            continue
        files.setdefault(line, []).append(cur)

    def pick(pred, k, seed):
        pool = sorted(n for n, dirs in files.items() if any(pred(n, d) for d in dirs))
        random.Random(seed).shuffle(pool)
        return pool[:k]

    tex = lambda ext: (lambda n, d: n.endswith(ext) and d.startswith("tex/"))
    under = lambda top, ext: (lambda n, d: n.endswith(ext) and d.startswith(top))
    out = []
    for ext, k in [(".sty", 150), (".cls", 40), (".tex", 40), (".fd", 30), (".def", 30),
                   (".cfg", 20), (".clo", 15), (".ldf", 15), (".ltx", 5)]:
        out += [("tex", n) for n in pick(tex(ext), k, "tex" + ext)]
    # \input foo: no suffix, kpathsea must add .tex
    out += [("tex", n[:-4]) for n in pick(tex(".tex"), 30, "bare")]
    # Found by name more than once: which copy wins is the ls-R/path order.
    dup = lambda n, d: d.startswith("tex/") and len(files[n]) > 1 and "." in n
    out += [("tex", n) for n in pick(dup, 30, "dup")]
    tfm = pick(under("fonts/tfm/", ".tfm"), 60, "tfm")
    out += [("tfm", n) for n in tfm[:40]] + [("tfm", n[:-4]) for n in tfm[40:]]
    out += [("type1 fonts", n) for n in pick(under("fonts/type1/", ".pfb"), 20, "pfb")]
    out += [("enc files", n) for n in pick(under("fonts/enc/", ".enc"), 15, "enc")]
    out += [("map", n) for n in pick(under("fonts/map/", ".map"), 10, "map")]
    out += [("vf", n) for n in pick(under("fonts/vf/", ".vf"), 10, "vf")]
    out += [("bst", n) for n in pick(under("bibtex/bst/", ".bst"), 10, "bst")]
    out += [("bib", n) for n in pick(under("bibtex/bib/", ".bib"), 5, "bib")]
    out += [("fmt", n) for n in ["pdflatex.fmt", "latex.fmt", "tex.fmt", "etex.fmt",
                                 "pdftex.fmt", "pdflatex"]]
    # Wrong case (TeX Live 2026 sets texmf_casefold_search=1).
    out += [("tex", n.upper()) for n in pick(tex(".sty"), 10, "case")]
    out += [(f, f"flashtex-no-such-{i}{ext}") for i, (f, ext) in enumerate(
        [("tex", ".sty"), ("tex", ".cls"), ("tex", ""), ("tex", ".tex"), ("tfm", ""),
         ("tfm", ".tfm"), ("type1 fonts", ".pfb"), ("enc files", ".enc"), ("map", ".map"),
         ("vf", ".vf"), ("bst", ".bst"), ("fmt", ".fmt")] * 2)]
    out += [("tex", "texmf.cnf"), ("cnf", "texmf.cnf")]
    return out


def kpsewhich(entries, cwd):
    rows, times = [], []
    for f, n in entries:
        t = time.perf_counter()
        r = sh("kpsewhich", "-progname=pdflatex", "-engine=pdftex", f"-format={f}", n, cwd=cwd)
        times.append(time.perf_counter() - t)
        rows.append(r.stdout.strip())
    # One process per format with every name: the library cost with the
    # process start-up spread over many lookups.
    batch = {}
    for f in sorted({f for f, _ in entries}):
        names = [n for g, n in entries if g == f]
        t = time.perf_counter()
        sh("kpsewhich", "-progname=pdflatex", "-engine=pdftex", f"-format={f}", *names, cwd=cwd)
        batch[f] = (time.perf_counter() - t) / len(names)
    return rows, times, batch


def ours(example, corpus, cwd, env, mode=("texlive",)):
    with tempfile.NamedTemporaryFile("r", suffix=".tsv", delete=False) as o:
        path = o.name
    r = subprocess.run([example, corpus, path, *mode], cwd=cwd, env=env,
                       capture_output=True, text=True)
    if r.returncode != 0:
        raise SystemExit(f"resolver_corpus failed: {r.stderr}")
    lines = open(path).read().splitlines()
    os.unlink(path)
    setup = int(lines[0].split("setup_ns=")[1])
    rows = [l.split("\t") for l in lines[1:]]
    return ([r[2] for r in rows], [int(r[3]) / 1e9 for r in rows],
            [int(r[4]) / 1e9 for r in rows], setup / 1e9, r.stderr)


def main():
    dist = sh("kpsewhich", "-var-value=TEXMFDIST").stdout.strip()
    version = sh("kpsewhich", "--version").stdout.splitlines()[0]
    entries = build_corpus(dist)
    corpus = os.path.join(HERE, "corpus.tsv")
    with open(corpus, "w") as c:
        c.write(f"# {len(entries)} lookups from {dist}/ls-R ({version})\n")
        for f, n in entries:
            c.write(f"{f}\t{n}\n")

    cwd = tempfile.mkdtemp(prefix="resolver-eval-")
    subprocess.run(["cargo", "build", "--release", "--locked", "-p", "flashtex-engine",
                    "--example", "resolver_corpus"], cwd=ROOT, check=True,
                   capture_output=True)
    example = os.path.join(ROOT, "target/release/examples/resolver_corpus")

    want, kt, kbatch = kpsewhich(entries, cwd)
    home = os.environ.get("HOME", "")
    envs = {
        "shell": dict(os.environ),
        "gui": {"HOME": home, "USER": os.environ.get("USER", ""), "TMPDIR": tempfile.gettempdir(),
                "PATH": "/usr/bin:/bin:/usr/sbin:/sbin"},
        "empty": {},
    }
    res = {"corpus": len(entries), "kpathsea": version, "texmfdist": dist,
           "found_by_kpsewhich": sum(1 for w in want if w)}
    for name, env in envs.items():
        got, t1, t2, setup, err = ours(example, corpus, cwd, env)
        diffs = [(f, n, w, g) for (f, n), w, g in zip(entries, want, got) if w != g]
        res[name] = {"identical": len(entries) - len(diffs), "differences": diffs[:20],
                     "setup_s": setup, "first_pass_total_s": sum(t1),
                     "first_pass_median_us": statistics.median(t1) * 1e6,
                     "steady_median_us": statistics.median(t2) * 1e6,
                     "steady_p99_us": sorted(t2)[int(len(t2) * 0.99)] * 1e6,
                     "stderr": err.strip()[:500]}

    bundle = tempfile.mkdtemp(prefix="resolver-bundle-")
    copied = {}
    for w in want:
        if w and os.path.basename(w) not in copied:
            copied[os.path.basename(w)] = w
            shutil.copy(os.path.join(cwd, w) if not os.path.isabs(w) else w, bundle)
    got, t1, t2, setup, err = ours(example, corpus, cwd, dict(os.environ), ("bundle", bundle))
    bdiff = []
    for (f, n), w, g in zip(entries, want, got):
        expect = os.path.basename(w) if w and copied.get(os.path.basename(w)) == w else (
            None if not w else "SHADOWED")
        have = os.path.basename(g) if g else None
        if expect != "SHADOWED" and expect != have:
            bdiff.append((f, n, expect, have))
    res["bundle"] = {"files": len(copied), "identical": len(entries) - len(bdiff),
                     "differences": bdiff[:20], "steady_median_us": statistics.median(t2) * 1e6}

    res["kpsewhich_per_process_median_ms"] = statistics.median(kt) * 1e3
    res["kpsewhich_batch_per_lookup_us"] = {f: v * 1e6 for f, v in kbatch.items()}
    json.dump(res, open(os.path.join(HERE, "results.json"), "w"), indent=2)
    shutil.rmtree(cwd, ignore_errors=True)
    shutil.rmtree(bundle, ignore_errors=True)

    print(f"corpus: {len(entries)} lookups, {res['found_by_kpsewhich']} found by kpsewhich")
    for name in envs:
        r = res[name]
        print(f"{name:6} identical {r['identical']}/{len(entries)}  setup {r['setup_s']*1e3:.2f} ms  "
              f"first pass {r['first_pass_total_s']*1e3:.1f} ms total  "
              f"steady median {r['steady_median_us']:.1f} us p99 {r['steady_p99_us']:.1f} us")
        for d in r["differences"][:5]:
            print("   diff", d)
    b = res["bundle"]
    print(f"bundle identical {b['identical']}/{len(entries)} ({b['files']} files)  "
          f"steady median {b['steady_median_us']:.1f} us")
    for d in b["differences"][:5]:
        print("   diff", d)
    print(f"kpsewhich: {res['kpsewhich_per_process_median_ms']:.1f} ms per process (median); "
          f"batched per lookup: " + ", ".join(f"{f} {v:.0f} us" for f, v in
                                            res["kpsewhich_batch_per_lookup_us"].items()))


if __name__ == "__main__":
    sys.exit(main())
