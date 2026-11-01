#!/usr/bin/env python3
"""The export gate's visual half (Q2: pages against xelatex's PDF at 2x and
3x): every `NAME.reference.pdf` / `NAME.candidate.pdf` pair that
tools/xetex-pdfparity/run.py `--pdf-dir` kept, rasterised by Core Graphics
(tools/xetex-pdfparity/raster.py; macOS only) at each scale.

    python3 tools/unicode-m3/visual.py --pdf-dir DIR [--scales 2 3] [--json OUT]

Per case and scale: differing pixels and the largest channel difference, and
the structural differences compare.py finds (the same as the Linux run's,
unless a platform difference shows). Under the Commander's ruling of
2026-10-09 (docs/design/xetex/PLAN.md §4B) 0 px is not the gate; the floor
is measured and reported.
"""
import argparse
import glob
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
PP = os.path.join(os.path.dirname(HERE), "xetex-pdfparity")
sys.path.insert(0, PP)
import compare as C  # noqa: E402


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--pdf-dir", required=True)
    ap.add_argument("--scales", nargs="*", type=float, default=[2.0, 3.0])
    ap.add_argument("--json")
    a = ap.parse_args(argv)
    rows = []
    totals = {}
    for ref in sorted(glob.glob(os.path.join(a.pdf_dir, "*.reference.pdf"))):
        name = os.path.basename(ref)[:-len(".reference.pdf")]
        cand = os.path.join(a.pdf_dir, name + ".candidate.pdf")
        if not os.path.exists(cand):
            rows.append({"case": name, "error": "no candidate PDF"})
            print("%-34s no candidate PDF" % name)
            continue
        row = {"case": name}
        for s in a.scales:
            p = argparse.ArgumentParser()
            C.add_options(p)
            opt = C.options_from(p.parse_args(["--scale", str(s)]))
            try:
                rep = C.compare(ref, cand, opt, tag=name).as_json()
            except Exception as e:  # reported, not hidden
                row["%gx" % s] = {"error": "%s: %s" % (type(e).__name__, e)}
                continue
            row["%gx" % s] = {"px": rep.get("visual_pixels"), "structural": rep.get("structural_differences"),
                              "max_delta": rep.get("visual_max_delta")}
            t = totals.setdefault("%gx" % s, {"px": 0, "cases": 0, "zero": 0})
            t["px"] += rep.get("visual_pixels") or 0
            t["cases"] += 1
            t["zero"] += (rep.get("visual_pixels") or 0) == 0
        rows.append(row)
        print("%-34s %s" % (name, "  ".join("%s: %s px, %s structural" % (
            k, v.get("px", v.get("error")), v.get("structural")) for k, v in row.items() if k != "case")),
            flush=True)
    for k, t in totals.items():
        print("%s: %d cases, %d at 0 px, %d px in all" % (k, t["cases"], t["zero"], t["px"]))
    if a.json:
        with open(a.json, "w") as f:
            json.dump({"cases": rows, "totals": totals}, f, indent=1)
    return 0


if __name__ == "__main__":
    sys.exit(main())
