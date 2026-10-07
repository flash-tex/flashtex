import json, re, sys
# phases.py log...: per-phase table from EditorInstantPhase lines.
def stat(p, k):
    v = p.get(k, {})
    return "%6.1f %6.1f %6.1f" % (v.get("p50", -1), v.get("p95", -1), v.get("max", -1)) if v else "     -      -      -"
for path in sys.argv[1:]:
    print("==", path)
    for line in open(path, errors="replace"):
        m = re.match(r"EditorInstantPhase (\w+): (.*)$", line.rstrip("\n"))
        if not m: continue
        try:
            p = json.loads(m.group(2))
        except Exception as e:
            print("  ", m.group(1), "unparsable", len(m.group(2))); continue
        secs = p.get("main_sections_ms", {})
        keys = p["keys"]
        def per(k):
            v = secs.get(k)
            return "%.1f" % (v[0] / max(1, keys)) if v else "-"
        print(f"  {m.group(1):7s} keys {keys:2d} drawn {p['keys_drawn']:2d} to {p.get('timed_out')}  key->drawn p50/p95/max {stat(p,'key_to_drawn_ms')} | queue {stat(p,'queue_ms')} | handler {stat(p,'handler_ms')} | ping {stat(p,'ping_ms')}")
        tot = {k: round(v[0], 1) for k, v in secs.items()}
        print("           per-key ms: textDidChange", per("editor.textDidChange"), "bufferCopy", per("sp.bufferCopy"), "modelUpdate", per("sp.modelUpdate"),
              "| model.v3", per("model.v3TextChanged"), "compare", per("model.compare"), "store", per("model.store"), "revision", per("model.revision"), "rest", per("model.rest"))
        print("           totals ms:", {k: tot[k] for k in sorted(tot) if k.startswith(("chrome", "v3."))})
