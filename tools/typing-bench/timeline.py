#!/usr/bin/env python3
"""Per-painted-revision stage attribution from a FlashTeXMac bench log.

Stages (all in ms, keyed by the editor revision the v2 paint carried):
  key->send      last keystroke of the revision -> compile request sent (debounce + sibling hold)
  send->v1       request sent -> v1 preview applied (helper round trip incl. producer + transport)
  v1->recv       v1 applied -> display_candidate frame decoded on the reader thread (sibling transport + helper idle window)
  recv->val      frame decoded -> validation started on V2Loader.queue (main-thread admission)
  validate       decode + validate + fonts + prepare
  preraster      GlyphRunRenderer.rasterize of every page
  deliver        off-main finish -> main-thread delivery
  pub->paint     displayListV2 published -> bench paint point
  key->paint     the bench's own latency for the LAST keystroke covered by the paint (not coalesced)
Usage: timeline.py <app.log> [--csv]
"""
import json, re, sys, statistics as st

def parse(path):
    ev = {}
    def at(rev): return ev.setdefault(rev, {})
    keys = {}       # revision -> keystroke ns
    sent = {}       # revision -> ns
    applied = {}    # revision -> ns (v1)
    reqrev = {}     # request id -> editor revision (from validating line)
    recv = {}       # request id -> ns
    dl_recv = {}    # request id -> (ns, bytes) from "received"
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.rstrip("\n")
        m = re.search(r"keystroke: revision (\d+) at (\d+)", line)
        if m: keys[int(m.group(1))] = int(m.group(2)); continue
        m = re.search(r"compile: sending revision (\d+) at (\d+)", line)
        if m: sent[int(m.group(1))] = int(m.group(2)); continue
        m = re.search(r"compile: applied revision (\d+) at (\d+)", line)
        if m: applied[int(m.group(1))] = int(m.group(2)); continue
        m = re.search(r"display-candidate: received (\S+) generation \d+ \((\d+) B\) at (\d+)", line)
        if m: dl_recv[m.group(1)] = (int(m.group(3)), int(m.group(2))); continue
        m = re.search(r"display-candidate: validating (\S+) as revision (\d+) ticket \d+ at (\d+)", line)
        if m:
            rid, rev, ns = m.group(1), int(m.group(2)), int(m.group(3))
            reqrev[rid] = rev
            e = at(rev); e["rid"] = rid; e["val_start"] = ns
            if rid in dl_recv: e["recv"], e["bytes"] = dl_recv[rid]
            continue
        m = re.search(r"display-candidate: validated (\S+) in ([\d.]+) ms(?: \(reused pages (\d+)/(\d+)\))?, prerastered (\d+) page\(s\) in ([\d.]+) ms, delivered ([\d.]+) ms later", line)
        if m and m.group(1) in reqrev:
            e = at(reqrev[m.group(1)]); e["validate"] = float(m.group(2)); e["preraster"] = float(m.group(6)); e["deliver"] = float(m.group(7))
            e["pages"] = int(m.group(4)) if m.group(4) else int(m.group(5)); e["rastered"] = int(m.group(5))
            if m.group(3): e["reused"] = int(m.group(3))
            continue
        # direct v2 route lines
        m = re.search(r"preview-v2: preparing live (\S+) \(revision (\d+)\) ticket \d+ at (\d+)", line)
        if m:
            rid, rev, ns = m.group(1), int(m.group(2)), int(m.group(3))
            reqrev[rid] = rev; e = at(rev); e["rid"] = rid; e["val_start"] = ns
            if rid in dl_recv: e["recv"], e["bytes"] = dl_recv[rid]
            continue
        m = re.search(r"preview-v2: prepared live (\S+) \(revision (\d+)\) in ([\d.]+) ms(?: \(reused pages (\d+)/(\d+)\))?, prerastered (\d+) page\(s\) in ([\d.]+) ms, delivered ([\d.]+) ms later", line)
        if m:
            e = at(int(m.group(2))); e["validate"] = float(m.group(3)); e["preraster"] = float(m.group(7)); e["deliver"] = float(m.group(8))
            e["pages"] = int(m.group(5)) if m.group(5) else int(m.group(6)); e["rastered"] = int(m.group(6))
            if m.group(4): e["reused"] = int(m.group(4))
            continue
        m = re.search(r"(?:display-candidate|preview-v2): published (?:live )?(\S+) (?:\(revision (\d+)\) )?(?:revision (\d+) )?at (\d+)", line)
        if m:
            rev = int(m.group(2) or m.group(3)); at(rev)["pub"] = int(m.group(4)); continue
        m = re.search(r"worker: display_list (\S+) line (\d+) B received at (\d+)", line)
        if m: dl_recv[m.group(1)] = (int(m.group(3)), int(m.group(2))); continue
        m = re.search(r"paint: revision (\d+) at (\d+) \(covers (\d+) keystrokes", line)
        if m:
            e = at(int(m.group(1))); e["paint"] = int(m.group(2)); e["covers"] = int(m.group(3)); continue
    rows = []
    for rev in sorted(ev):
        e = ev[rev]
        if "paint" not in e or "pub" not in e: continue
        r = {"rev": rev, "pages": e.get("pages"), "bytes": e.get("bytes"), "covers": e.get("covers")}
        if "reused" in e: r["reused"] = e["reused"]
        if "rastered" in e: r["rastered"] = e["rastered"]
        ms = lambda a, b: (b - a) / 1e6
        if rev in keys and rev in sent: r["key->send"] = ms(keys[rev], sent[rev])
        if rev in sent and rev in applied: r["send->v1"] = ms(sent[rev], applied[rev])
        if rev in applied and "recv" in e: r["v1->recv"] = ms(applied[rev], e["recv"])
        if "recv" in e and "val_start" in e: r["recv->val"] = ms(e["recv"], e["val_start"])
        for k in ("validate", "preraster", "deliver"):
            if k in e: r[k] = e[k]
        r["pub->paint"] = ms(e["pub"], e["paint"])
        if rev in keys: r["key->paint"] = ms(keys[rev], e["paint"])
        if "val_start" in e: r["val->paint"] = ms(e["val_start"], e["paint"])
        rows.append(r)
    return rows

def helper_phases(path):
    """Reconstructs the helper's diagnostic_timings stderr JSON objects (logged as
    `controller: <chunk>` fragments, split at pipe-chunk boundaries) and returns
    {phase: [object, ...]} for the phases the display route reports."""
    frags = []
    for line in open(path, encoding="utf-8", errors="replace"):
        i = line.find("\tcontroller: ")
        if i >= 0: frags.append(line[i + len("\tcontroller: "):].rstrip("\n"))
    stream = "".join(frags)
    out, depth, start = {}, 0, None
    for i, c in enumerate(stream):
        if c == "{":
            if depth == 0: start = i
            depth += 1
        elif c == "}" and depth > 0:
            depth -= 1
            if depth == 0 and start is not None:
                try: obj = json.loads(stream[start:i + 1])
                except ValueError: continue
                out.setdefault(obj.get("phase", "?"), []).append(obj)
    return out

def main():
    path = sys.argv[1]
    rows = parse(path)
    cols = ["key->send", "send->v1", "v1->recv", "recv->val", "validate", "preraster", "deliver", "pub->paint", "val->paint", "key->paint"]
    print(f"{len(rows)} painted v2 revisions in {path}")
    print("stage            n   p50     p95     max")
    for c in cols:
        v = sorted(r[c] for r in rows if c in r)
        if not v: continue
        print(f"{c:14s} {len(v):3d} {st.median(v):7.1f} {v[min(len(v)-1, int(len(v)*0.95))]:7.1f} {v[-1]:7.1f}")
    pages = [r["pages"] for r in rows if r.get("pages")]
    covers = [r["covers"] for r in rows if r.get("covers")]
    if pages: print("pages/frame:", st.median(pages), " keystrokes covered per paint: median", st.median(covers), "max", max(covers))
    phases = helper_phases(path)
    for phase in ("display_transport", "optional_output"):
        objs = phases.get(phase, [])
        if not objs: continue
        flat = []
        for o in objs:
            p = dict(o.get("profile") or {}); p.update({k: v for k, v in o.items() if k != "profile"}); flat.append(p)
        keys = sorted({k for p in flat for k, v in p.items() if isinstance(v, (int, float)) and not isinstance(v, bool) and (k.endswith("_ms") or k == "framed_bytes" or k == "bytes")})
        print(f"helper stderr phase {phase}: {len(objs)} objects" + (f" (outcomes: {sorted({o.get('outcome') for o in objs if o.get('outcome')})})" if any(o.get("outcome") for o in objs) else ""))
        for k in keys:
            v = sorted(p[k] for p in flat if k in p)
            print(f"  {k:32s} n={len(v):3d} p50 {st.median(v):9.2f} max {v[-1]:9.2f}")
    if "--csv" in sys.argv:
        print(",".join(["rev"] + cols))
        for r in rows: print(",".join([str(r["rev"])] + [f"{r[c]:.1f}" if c in r else "" for c in cols]))

if __name__ == "__main__": main()
