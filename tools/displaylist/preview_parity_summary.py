#!/usr/bin/env python3
"""Summarise one preview-parity sweep (DESIGN.md §12 P3 exit, §6.2) as Markdown.

Reads PreviewParityTests' per-page report (`FLASHTEX_V3_PARITY_OUT`, the
`.coreGraphics.json` file) and, optionally, check_positions.py's `--json`
results, and prints what the P3 gate reports: page renders identical / total
per scale (tolerance 0), the PDF-fallback rate and the pages that fall back,
positions and P-T2. .github/workflows/preview-parity.yml appends it to the job
summary.

    python3 tools/displaylist/preview_parity_summary.py --sweep parity.coreGraphics.json \
        [--positions positions.json]

Exit status 1 when the sweep has a page render that differs, or covers fewer
fixtures than the positions run produced (a fixture the sweep dropped).
"""

import argparse
import collections
import json
import os
import sys


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--sweep", required=True)
    ap.add_argument("--positions", default=None)
    a = ap.parse_args()
    if not os.path.isfile(a.sweep):
        print(f"**preview parity: no sweep report** (`{os.path.basename(a.sweep)}` missing: the sweep did not run)")
        return 1
    with open(a.sweep, encoding="utf-8") as f:
        rows = json.load(f)
    per = collections.OrderedDict()
    fallback_pages, differing = [], []
    renders = fallback = 0
    for row in rows:
        fb = set()
        for r in row["pages"]:
            renders += 1
            if r["incomplete"]:
                fallback += 1
                fb.add(r["page"] + 1)
                continue
            s = per.setdefault(r["scale"], [0, 0, 0])
            s[0] += 1
            s[2] += r["differingPixels"]
            if r["differingPixels"] == 0:
                s[1] += 1
            else:
                differing.append(f"{row['fixture']} p{r['page'] + 1} @{r['scale']:g}x: "
                                 f"{r['differingPixels']} px (max Δ {r['maxChannelDelta']})")
        fallback_pages += [f"{row['fixture']} p{p}" for p in sorted(fb)]
    refs = collections.Counter(row.get("pdf", "?") for row in rows)
    ok = not differing
    out = [f"### Preview parity at tolerance 0: {'PASS' if ok else 'FAIL'}", "",
           f"{len(rows)} fixtures; reference PDF: "
           + ", ".join(f"`{k}` for {v}" for k, v in sorted(refs.items())), "",
           "| scale | identical / compared | differing px |", "|---|---|---|"]
    for scale, (n, same, px) in sorted(per.items()):
        out.append(f"| {scale:g}x | {same}/{n} | {px} |")
    total = sum(v[0] for v in per.values())
    same = sum(v[1] for v in per.values())
    out.append(f"| all | {same}/{total} | {sum(v[2] for v in per.values())} |")
    out += ["", f"PDF fallback: {fallback}/{renders} page renders "
                f"({100.0 * fallback / renders if renders else 0:.1f} %), {len(fallback_pages)} pages: "
                + (", ".join(fallback_pages) or "none")]
    if differing:
        out += ["", "Differing renders:", ""] + [f"- {d}" for d in differing[:200]]
    if a.positions and os.path.isfile(a.positions):
        with open(a.positions, encoding="utf-8") as f:
            pos = json.load(f)
        s, res = pos["summary"], pos["results"]
        line = f"Positions (0 sp): {s['ok']}/{s['checked']} documents"
        if "pt2" in s:
            line += f"; P-T2 (engine PDF written with the display list): {s['pt2']}/{s['checked']}"
        out += ["", line]
        bad = [r for r in res if not r["ok"]]
        out += [f"- {r['id']}: {r.get('why')}" for r in bad[:50]]
        produced = sum(1 for r in res if r.get("bytes"))
        if len(rows) < produced:
            ok = False
            out += ["", f"**The sweep covered {len(rows)} fixtures, but {produced} display lists were written.**"]
    print("\n".join(out))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
