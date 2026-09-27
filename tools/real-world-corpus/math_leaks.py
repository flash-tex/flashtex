#!/usr/bin/env python3
"""#846 re-measurement: leaked `\\name` tokens in extracted text (FlashTeX minus
pdflatex) and the unresolved-command diagnostics per name, per document.

usage: measure.py --repo <worktree> --corpus <dir> --out <dir> [--only id,...]
pdflatex is an oracle only; its outputs are cached under <out>/<id>/oracle.
"""
import argparse, collections, json, os, re, shutil, subprocess, sys

TOKEN = re.compile(r"\\[A-Za-z]{2,}")
UNSUPPORTED = re.compile(r"^\\([A-Za-z@]+) is not supported in math mode")


def sh(argv, stdin=None, env=None, cwd=None, timeout=300):
    try:
        p = subprocess.run(argv, input=stdin, capture_output=True, env=env, cwd=cwd, timeout=timeout)
        return p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired as e:
        return -999, e.stdout or b"", e.stderr or b""


def fixtures(corpus, only):
    out = []
    for name in sorted(os.listdir(corpus)):
        d = os.path.join(corpus, name)
        if not os.path.isdir(d):
            continue
        if only and name not in only:
            continue
        texs = []
        for root, _, files in os.walk(d):
            for f in files:
                if f.endswith((".tex", ".sty", ".cls", ".bbl", ".bib", ".clo", ".def")):
                    texs.append(os.path.relpath(os.path.join(root, f), d))
        entry = None
        for cand in ["main.tex"] + sorted(t for t in texs if t.endswith(".tex")):
            p = os.path.join(d, cand)
            if os.path.isfile(p) and "\\documentclass" in open(p, encoding="utf-8", errors="replace").read():
                entry = cand
                break
        if entry:
            out.append({"id": name, "dir": d, "entry": entry, "documents": sorted(texs)})
    return out


def pdf_text(pdf):
    if not os.path.isfile(pdf):
        return None
    code, out, _ = sh(["pdftotext", "-layout", pdf, "-"])
    return out.decode("utf-8", "replace") if code == 0 else None


def token_counts(text):
    return collections.Counter(TOKEN.findall(text or ""))


def oracle(fx, work):
    od = os.path.join(work, "oracle")
    pdf = os.path.join(od, os.path.splitext(fx["entry"])[0] + ".pdf")
    if os.path.isfile(pdf):
        return pdf
    if os.path.isdir(od):
        shutil.rmtree(od)
    shutil.copytree(fx["dir"], od)
    env = dict(os.environ, SOURCE_DATE_EPOCH="0", FORCE_SOURCE_DATE="1")
    for _ in range(3):
        sh(["/Library/TeX/texbin/pdflatex", "-interaction=nonstopmode", fx["entry"]], env=env, cwd=od, timeout=300)
        base = os.path.splitext(fx["entry"])[0]
        if os.path.isfile(os.path.join(od, base + ".aux")) and any(
            f.endswith(".bib") for f in fx["documents"]) or any(f.endswith(".bbl") for f in fx["documents"]):
            sh(["/Library/TeX/texbin/bibtex", base], cwd=od, timeout=120)
    return pdf if os.path.isfile(pdf) else None


def flashtex(fx, work, args, env):
    docs = []
    for name in fx["documents"]:
        with open(os.path.join(fx["dir"], name), encoding="utf-8", errors="replace") as f:
            docs.append({"path": name.replace(os.sep, "/"), "text": f.read()})
    req = {"protocol_version": 1, "id": "gh846-" + fx["id"], "type": "compile", "payload": {
        "project_id": "gh846-" + fx["id"], "revision": 1, "entry_path": fx["entry"], "documents": docs,
        "project_root": os.path.realpath(fx["dir"]),
        "layout_capabilities": ["display-list-v2", "display-list-v2-images"], "date": "1970-01-01"}}
    v2 = os.path.join(work, "render.v2.json")
    line = (json.dumps(req, ensure_ascii=False) + "\n").encode("utf-8")
    code, out, err = sh([args.render, "--v2", v2, "--images"], stdin=line, env=env, timeout=300)
    open(os.path.join(work, "reply.jsonl"), "wb").write(out)
    rec = {"exit": code, "status": "no_reply", "diagnostics": [], "pages": 0, "stderr_tail": err.decode("utf-8", "replace")[-600:]}
    for raw in out.decode("utf-8", "replace").splitlines():
        try:
            msg = json.loads(raw)
        except json.JSONDecodeError:
            continue
        if msg.get("type") == "compile_result":
            p = msg.get("payload", {})
            rec["status"] = p.get("status")
            rec["pages"] = len(p.get("pages", []))
            rec["diagnostics"] = p.get("diagnostics", [])
            break
        if msg.get("type") == "error":
            rec["status"] = "error"
            rec["error"] = msg.get("payload")
    pdf = os.path.join(work, "flashtex.pdf")
    if os.path.isfile(v2):
        c, o, e = sh([args.pdf_exact, "from-v2", v2, "--out", pdf, "--font-dir", env["FLASHTEX_FONT_DIRS"],
                      "--project-root", os.path.realpath(fx["dir"])], timeout=300)
        rec["pdf_exit"] = c
        if c != 0:
            rec["pdf_err"] = e.decode("utf-8", "replace")[-400:]
    return rec, pdf


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", required=True)
    ap.add_argument("--corpus", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--only", default="")
    ap.add_argument("--render")
    ap.add_argument("--pdf-exact")
    ap.add_argument("--no-oracle", action="store_true")
    args = ap.parse_args()
    sys.path.insert(0, os.path.join(args.repo, "tools", "visual-oracle"))
    import fontenv
    fonts, tfm = fontenv.resolve_dirs(None, None, os.path.join(args.repo, "apps", "mac", "Fonts"))
    env = dict(os.environ, FLASHTEX_FONT_DIRS=fonts, FLASHTEX_TFM_DIRS=tfm)
    args.render = args.render or os.path.join(args.repo, "crates/render-pipeline/target/release/flashtex-render")
    # crates/pdf is a root-workspace member: it builds into the repository's target/.
    args.pdf_exact = args.pdf_exact or os.path.join(args.repo, "target/release/flashtex-pdf-exact")
    only = set(filter(None, args.only.split(",")))
    os.makedirs(args.out, exist_ok=True)
    report = []
    total_unresolved = collections.Counter()
    total_leak = collections.Counter()
    for fx in fixtures(args.corpus, only):
        work = os.path.join(args.out, fx["id"])
        os.makedirs(work, exist_ok=True)
        ref_counts = collections.Counter()
        ref_pages = None
        if not args.no_oracle:
            ref = oracle(fx, work)
            if ref:
                ref_counts = token_counts(pdf_text(ref))
        rec, pdf = flashtex(fx, work, args, env)
        ft_counts = token_counts(pdf_text(pdf))
        leak = collections.Counter()
        for k, v in ft_counts.items():
            extra = v - ref_counts.get(k, 0)
            if extra > 0:
                leak[k] = extra
        unresolved = collections.Counter()
        other_unknown = collections.Counter()
        for d in rec["diagnostics"]:
            m = UNSUPPORTED.match(d.get("message", ""))
            if m:
                unresolved["\\" + m.group(1)] += 1
            elif d.get("code") == "unknown_command":
                other_unknown[d.get("message", "")[:80]] += 1
        total_unresolved.update(unresolved)
        total_leak.update(leak)
        row = {"id": fx["id"], "status": rec["status"], "pages": rec["pages"], "exit": rec["exit"],
               "pdflatex_tokens": sum(ref_counts.values()), "flashtex_tokens": sum(ft_counts.values()),
               "leaked": sum(leak.values()), "leak_top": leak.most_common(8),
               "unresolved_math": sum(unresolved.values()), "unresolved_top": unresolved.most_common(25),
               "other_unknown": other_unknown.most_common(10), "stderr_tail": rec.get("stderr_tail", "")[-300:],
               "pdf_err": rec.get("pdf_err", "")}
        report.append(row)
        print(f"{fx['id']:32} {row['status']!s:10} pages={row['pages']:3} pdflatex={row['pdflatex_tokens']:4} "
              f"flashtex={row['flashtex_tokens']:4} leaked={row['leaked']:4} unresolved_math={row['unresolved_math']:4}")
        print("   leak:", leak.most_common(6))
        print("   unresolved:", unresolved.most_common(12))
        if other_unknown:
            print("   other unknown:", other_unknown.most_common(5))
        if rec["status"] not in ("ok", "recovered"):
            print("   STDERR:", row["stderr_tail"].replace("\n", " | "))
    with open(os.path.join(args.out, "report.json"), "w") as f:
        json.dump({"rows": report, "unresolved_total": total_unresolved.most_common(),
                   "leak_total": total_leak.most_common()}, f, indent=1)
    print("\n== unresolved math commands across corpus ==")
    for name, n in total_unresolved.most_common(60):
        print(f"{n:5}  {name}")
    print("\n== leaked tokens across corpus ==")
    for name, n in total_leak.most_common(20):
        print(f"{n:5}  {name}")


if __name__ == "__main__":
    main()
