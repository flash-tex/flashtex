#!/usr/bin/env python3
"""Run every tests/tex-corpus case through a compiler binary and flashtex-pdf.

Records, per case: compiler status and diagnostics, pages and text items, PDF
writer warnings (default fonts and, optionally, with an embedded font), the
writer's structural self-check, and whether macOS `sips` opens the file. Prints
a Markdown table plus per-case detail to stdout. Nothing here invokes a TeX
engine; the only inputs are the corpus sources and the two Rust binaries.

Usage:
  corpus_report.py --corpus tests/tex-corpus --compiler path/to/flashtex-compiler \
      --pdf path/to/flashtex-pdf [--embed-font auto|PATH] [--out-dir DIR]
"""

import argparse
import json
import os
import platform
import subprocess
import sys
import tempfile


def run(cmd, stdin_bytes=None, env=None):
    proc = subprocess.run(cmd, input=stdin_bytes, capture_output=True, env=env)
    return proc.returncode, proc.stdout, proc.stderr


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--corpus", required=True)
    ap.add_argument("--compiler", required=True)
    ap.add_argument("--pdf", required=True)
    ap.add_argument("--embed-font", default=None)
    ap.add_argument("--out-dir", default=None)
    ap.add_argument("--label", default="", help="free text recorded in the header")
    args = ap.parse_args()

    manifest = json.load(open(os.path.join(args.corpus, "manifest.json"), encoding="utf-8"))
    out_dir = args.out_dir or tempfile.mkdtemp(prefix="flashtex-corpus-")
    os.makedirs(out_dir, exist_ok=True)
    validate = os.path.join(args.corpus, "validate.py")
    is_mac = platform.system() == "Darwin"

    rows = []
    details = []
    for case in manifest["cases"]:
        cid = case["id"]
        code, req, err = run([sys.executable, validate, "--emit-request", cid])
        if code != 0:
            rows.append((cid, "request failed", "", "", "", "", ""))
            details.append(f"### {cid}\n\nvalidate.py failed: {err.decode(errors='replace')}\n")
            continue
        code, result_line, cerr = run([args.compiler], stdin_bytes=req)
        if code != 0 or not result_line.strip():
            rows.append((cid, f"compiler exit {code}", "", "", "", "", ""))
            details.append(f"### {cid}\n\ncompiler failed: {cerr.decode(errors='replace')}\n")
            continue
        result = json.loads(result_line.decode("utf-8").splitlines()[0])
        result_path = os.path.join(out_dir, f"{cid}.compile-result.json")
        with open(result_path, "wb") as f:
            f.write(result_line.splitlines()[0] + b"\n")
        if result.get("type") != "compile_result":
            rows.append((cid, f"envelope {result.get('type')}", "", "", "", "", ""))
            details.append(f"### {cid}\n\n{json.dumps(result.get('payload'), ensure_ascii=False)}\n")
            continue
        payload = result["payload"]
        status = payload.get("status")
        diags = payload.get("diagnostics", [])
        pages = payload.get("pages", [])
        items = sum(len(p.get("items", [])) for p in pages)
        texts = [i.get("text", "") for p in pages for i in p.get("items", []) if i.get("kind") == "text"]

        def render(tag, extra):
            pdf_path = os.path.join(out_dir, f"{cid}{tag}.pdf")
            code, _, perr = run([args.pdf, result_path, "--out", pdf_path, "--verify"] + extra)
            lines = perr.decode("utf-8", errors="replace").splitlines()
            warnings = [l[len("warning: "):] for l in lines if l.startswith("warning: ")]
            errors = [l for l in lines if l.startswith("error: ")]
            notes = [l for l in lines if l.startswith("note: embedding")]
            sips = ""
            if code == 0 and is_mac:
                scode, sout, serr = run(["/usr/bin/sips", "-g", "pixelWidth", "-g", "pixelHeight", pdf_path])
                s = sout.decode(errors="replace")
                sips = "opens" if scode == 0 and "pixelWidth" in s and b"Error" not in serr else f"sips failed: {serr.decode(errors='replace').strip()}"
            return code, warnings, errors, notes, sips

        pcode, pwarn, perr_lines, _, psips = render("", [])
        embed_summary = ""
        embed_detail = []
        if args.embed_font:
            ecode, ewarn, eerr, enotes, esips = render(".embedded", ["--embed-font", args.embed_font])
            embed_summary = f"exit {ecode}, {len(ewarn)} warning(s), {esips}" if not eerr else "; ".join(eerr)
            embed_detail = enotes + [f"warning: {w}" for w in ewarn] + eerr

        pdf_summary = f"exit {pcode}, {len(pwarn)} warning(s), {psips}" if not perr_lines else "; ".join(perr_lines)
        rows.append((cid, status, len(diags), len(pages), items, pdf_summary, embed_summary))
        detail = [f"### {cid}", "", f"Compiler: status `{status}`, {len(diags)} diagnostic(s), {len(pages)} page(s), {items} item(s)."]
        for d in diags:
            detail.append(f"- {d.get('severity')}: {d.get('message')}" + (f" (recovery: {d.get('recovery')})" if d.get("recovery") else ""))
        detail.append("")
        detail.append("Text items: " + " ".join(f"`{t}`" for t in texts[:40]) + (" …" if len(texts) > 40 else ""))
        detail.append("")
        detail.append("PDF (default fonts): " + pdf_summary)
        for w in pwarn:
            detail.append(f"- warning: {w}")
        if args.embed_font:
            detail.append("")
            detail.append(f"PDF (`--embed-font {args.embed_font}`): " + embed_summary)
            for l in embed_detail:
                detail.append(f"- {l}")
        detail.append("")
        details.append("\n".join(detail))

    print(f"Runner: `{os.path.basename(sys.argv[0])}` on {platform.system()} {platform.machine()}. {args.label}".strip())
    print(f"Artifacts: `{out_dir}` (local, not committed).")
    print()
    head = ["case", "compiler status", "diags", "pages", "items", "PDF default fonts"]
    if args.embed_font:
        head.append(f"PDF --embed-font {args.embed_font}")
    print("| " + " | ".join(head) + " |")
    print("|" + "---|" * len(head))
    for r in rows:
        cells = list(r[: len(head)])
        print("| " + " | ".join(str(c) for c in cells) + " |")
    print()
    print("\n".join(details))


if __name__ == "__main__":
    main()
