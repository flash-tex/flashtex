#!/usr/bin/env python3
"""Markdown tables of the M3 harnesses' JSON results found in a directory
(pt1.json, export.json, host-texlive.json, notex.json, sweep.json,
latency-*.json, visual.json, startup-*.json, classic-*.json), by corpus
family where a case has one.

    python3 tools/unicode-m3/summarize.py DIR [DIR ...]
"""
import collections
import glob
import json
import os
import sys


def load(dirs, pattern):
    out = []
    for d in dirs:
        for p in sorted(glob.glob(os.path.join(d, "**", pattern), recursive=True)):
            try:
                with open(p) as f:
                    out.append((p, json.load(f)))
            except (OSError, ValueError):
                pass
    return out


def families(dirs):
    fam = {}
    for _, idx in load(dirs, "corpus-index.json"):
        for k, v in idx.items():
            fam[k] = v.get("family", "?")
    return fam


def by_family(rows, fam, ok):
    t = collections.OrderedDict()
    for r in rows:
        f = fam.get(r["case"], r.get("family") or "?")
        a = t.setdefault(f, [0, 0, []])
        a[0] += 1
        if ok(r):
            a[1] += 1
        else:
            a[2].append(r["case"])
    lines = ["| family | cases | pass | failing |", "|---|---:|---:|---|"]
    for f, (n, p, bad) in t.items():
        lines.append("| %s | %d | %d | %s |" % (f, n, p, ", ".join(bad) or "-"))
    n = sum(v[0] for v in t.values())
    p = sum(v[1] for v in t.values())
    lines.append("| **all** | **%d** | **%d** | |" % (n, p))
    return lines


def main(argv):
    dirs = argv or ["."]
    fam = families(dirs)
    out = []
    for p, j in load(dirs, "pt1.json"):
        out += ["### P-T1 + XDV against xelatex -no-pdf (%s)" % os.path.basename(os.path.dirname(p)), ""]
        out += by_family(j["cases"], fam, lambda r: r["ok"])
        out += ["", "Format logs: %d lines, %d differ." % (j["format_log_lines"], j["format_log_differences"]), ""]
    for p, j in load(dirs, "export.json"):
        out += ["### Export: structural against xelatex's PDF", ""]
        out += by_family([r for r in j["cases"] if not r.get("excluded")], fam, lambda r: bool(r.get("ok")))
        if j.get("excluded"):
            out += ["", "Listed, not counted (R1: the reference wrote no PDF): %s." % ", ".join(j["excluded"])]
        r1 = [r["case"] for r in j["cases"] if r.get("r1")]
        if r1:
            out += ["", "Counted under R1 (xelatex's own error, the same in FlashTeX): %s." % ", ".join(r1)]
        t = j.get("totals", {})
        out += ["", "Structural differences %s; XDV check: %s glyphs over 0.001 bp, max %s bp." % (
            t.get("structural"), t.get("xdv_over"), t.get("xdv_max_bp")), ""]
    for p, j in load(dirs, "visual.json"):
        out += ["### Export: Core Graphics", ""]
        for k, t in j.get("totals", {}).items():
            out.append("- %s: %d cases, %d at 0 px, %d px in all" % (k, t["cases"], t["zero"], t["px"]))
        out.append("")
    for name in ("host-texlive.json", "notex.json"):
        for p, j in load(dirs, name):
            out += ["### Host corpus run: %s (HELLO texlive=%s, resolver=%s)" % (
                j["mode"], (j.get("hello") or {}).get("texlive"), (j.get("hello") or {}).get("resolver")), ""]
            out += by_family(j["cases"], fam, lambda r: r.get("status") == "ok" and r.get("pdf"))
            if any("bytes_equal_texlive" in r for r in j["cases"]):
                eq = sum(1 for r in j["cases"] if r.get("bytes_equal_texlive"))
                st = sum(1 for r in j["cases"] if r.get("structural_vs_xelatex") == 0)
                out += ["", "Byte-identical to the Linux TeX Live run: %d; 0 structural differences against "
                        "xelatex: %d (of %d)." % (eq, st, len(j["cases"]))]
            out.append("")
    for p, j in load(dirs, "sweep.json"):
        t = j["totals"]
        out += ["### Edit sweep through the host (spares against cold)", "",
                "%(documents)d documents, %(compiles)d compiles: %(equal)d equal, %(differ)d differ, "
                "%(failed)d failed or empty." % t, ""]
    for p, j in load(dirs, "latency-*.json"):
        out += ["### Latency (%s, %s, counter %s)" % (os.path.basename(p), j.get("platform"), j.get("counter")), "",
                "| document | candidate full run (M instr) | ms | -no-pdf (M instr) | xelatex -no-pdf (M instr) | xelatex+xdvipdfmx (M instr) |",
                "|---|---:|---:|---:|---:|---:|"]
        mi = lambda v: "-" if not v or v.get("instructions_best") is None else "%.0f" % (v["instructions_best"] / 1e6)
        for doc, v in j["engine"].items():
            if doc == "startup":
                continue
            out.append("| %s | %s | %s | %s | %s | %s |" % (
                doc, mi(v.get("candidate-pdf")), (v.get("candidate-pdf") or {}).get("ms_best"),
                mi(v.get("candidate-no-pdf")), mi(v.get("xelatex-no-pdf")), mi(v.get("xelatex-pdf"))))
        s = j["engine"].get("startup", {})
        out.append("| start-up (format loaded) | %s | %s | | %s | |" % (
            mi(s.get("candidate")), (s.get("candidate") or {}).get("ms_best"), mi(s.get("xelatex"))))
        if "host" in j:
            out += ["", "| document (host) | open first page ms | reopen first page ms | edit first page ms (5) | preamble edit first page ms |",
                    "|---|---:|---:|---|---:|"]
            for doc, h in j["host"].items():
                out.append("| %s | %s | %s | %s | %s |" % (
                    doc, h["open"].get("first_page_ms"), h["reopen"].get("first_page_ms"),
                    ", ".join(str(e.get("first_page_ms")) for e in h["edits"]),
                    h["preamble_edit"].get("first_page_ms")))
        out.append("")
    rows = load(dirs, "classic-*.json") + load(dirs, "startup-*.json")
    if rows:
        out += ["### Host binaries", "", "| file | size | stripped | sha256 | HELLO ms (median) | instructions (median) |",
                "|---|---:|---:|---|---:|---:|"]
        for p, j in rows:
            out.append("| %s | %d | %d | %s | %s | %s |" % (
                os.path.basename(p), j["size"], j["stripped_size"], j["sha256"][:16],
                j.get("hello_ms_median"), j.get("instructions_median")))
        out.append("")
    print("\n".join(out))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
