#!/usr/bin/env python3
"""Byte shares of one display-list-v2 line: what each per-cluster field costs
(FT-071 item 3, the compaction question). Usage: glyph_bytes.py <binary> <file.tex>...
A body60k seed is cut to a quarter so the frame fits the 16 MiB line cap (its
pages repeat the demo's paragraphs, so per-cluster shares are representative)."""
import json, os, subprocess, sys

B = sys.argv[1]
def wire(o): return json.dumps(o, separators=(",", ":"))
for path in sys.argv[2:]:
    text = open(path).read(); name = os.path.basename(path)
    env = dict(os.environ)
    env.setdefault("FLASHTEX_FONT_DIRS", os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../apps/mac/Fonts"))
    if "body60k" in name:
        head, body = text.split("\\begin{document}\n", 1); body = body.rsplit("\\end{document}", 1)[0]
        text = head + "\\begin{document}\n" + body[:len(body) // 4] + "\\end{document}\n"
    p = subprocess.Popen([B], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, env=env)
    req = {"protocol_version": 1, "id": "m", "type": "compile", "payload": {"project_id": "d", "revision": 1, "entry_path": name,
           "documents": [{"path": name, "text": text}], "layout_capabilities": ["display-list-v2"]}}
    p.stdin.write((wire(req) + "\n").encode()); p.stdin.flush(); r1 = p.stdout.readline(); r2 = p.stdout.readline(); p.stdin.close(); p.wait()
    pl = json.loads(r2)["payload"]
    tot = len(r2); carets = hits = srcs = glyphs = cl_rest = runs = 0; nglyphs = ncl = 0
    for pg in pl["pages"]:
        for it in pg["items"]:
            if it["kind"] != "glyph_run": continue
            runs += 1
            for g in it["glyphs"]: glyphs += len(wire(g)) + 1; nglyphs += 1
            for c in it["clusters"]:
                ncl += 1
                carets += len(wire(c["carets"])) + len('"carets":,')
                hits += len(wire(c["hit_rects"])) + len('"hit_rects":,')
                if "sources" in c: srcs += len(wire(c["sources"])) + len('"sources":,')
                cl_rest += len(wire({"text_start_byte": c["text_start_byte"], "text_end_byte": c["text_end_byte"]})) + 2
    print(f"{name}: pages {len(pl['pages'])} line {tot} B; runs {runs} glyphs {nglyphs} clusters {ncl}")
    for k, v in [("carets", carets), ("hit_rects", hits), ("sources", srcs), ("cluster text bytes", cl_rest), ("glyphs", glyphs)]:
        print(f"  {k:20s} {v:9d} B  {100 * v / tot:5.1f}%  {v / max(ncl, 1):6.1f} B/cluster")
