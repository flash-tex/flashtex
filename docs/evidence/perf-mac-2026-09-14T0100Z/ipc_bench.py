#!/usr/bin/env python3
"""Drives flashtex-render over its stdin/stdout JSON Lines like the Mac app:
one compile request per keystroke (typed before \\end{document}), waits for
the compile_result and the display_list sibling, records wall time per
request (send -> last reply byte) and the reply sizes.
Usage: ipc_bench.py <binary> <file.tex> [steps] [--no-v2] [--delta] [--only] [--compact] [--md5 out] [--dump-first path]
  --delta    request display-list-v2-delta with the installed-base acknowledgement
             (a full display_list line is digested with the proposal's Appendix A
             reference; a delta's own list_digest is the next base)
  --only     request display-list-v2-only (v1 pages elided)
  --compact  request display-list-v2-compact (protocol/proposals/display-list-v2-compact.md);
             a compact full line is expanded by the reference decoder below before
             it is digested, so the acknowledgement is the consumer's
  --dump-first path   write the first request's sibling line to `path`
"""
import hashlib, json, math, os, struct, subprocess, sys, time

# ---- display-list-v2-compact reference decoder (proposal §4, third implementation) ----
def scalar_len(text_bytes, at):
    if at < 0 or at >= len(text_bytes): return 0
    b = text_bytes[at]
    if b < 0x80: return 1
    if b & 0xE0 == 0xC0: return 2
    if b & 0xF0 == 0xE0: return 3
    if b & 0xF8 == 0xF0: return 4
    return 0
def expand_run(run):
    """A compact glyph_run → the full form (in place)."""
    glyphs = run['glyphs']
    if glyphs and isinstance(glyphs[0], list):
        run['glyphs'] = glyphs = [dict(gid=g[0], origin_x=g[1], baseline_y=g[2], advance_x=g[3], advance_y=g[4], cluster=g[5]) for g in glyphs]
    text = run['text'].encode('utf-8')
    span = run.pop('sources', None)
    if span is not None:
        assert len(span) == 1, 'run sources: exactly one span'
        span = span[0]
    hit_top, hit_height = run.pop('hit_top', None), run.pop('hit_height', None)
    end = run.pop('end_caret', None)
    first_x, widths = {}, {}
    for g in glyphs:
        first_x.setdefault(g['cluster'], g['origin_x']); widths[g['cluster']] = widths.get(g['cluster'], 0) + g['advance_x']
    out, prev_text_end = [], 0
    prev_src_end = span['start_byte'] if span else 0
    n = len(run['clusters'])
    for i, c in enumerate(run['clusters']):
        ts = c.get('ts', prev_text_end)
        l = c.get('l', scalar_len(text, ts))
        te = ts + l; prev_text_end = te
        if 'h' in c:
            x, top, w, h = c['h']
        else:
            top, h = c['hv'] if 'hv' in c else (hit_top, hit_height)
            x, w = first_x.get(i, 0), widths.get(i, 0)
        rect = dict(x=x, top=top, width=w, height=h)
        if 'c' in c:
            carets = [dict(text_byte=k[0], x=k[1], top=k[2], height=k[3]) for k in c['c']]
        else:
            carets = [dict(text_byte=ts, x=x, top=top, height=h)]
            if i + 1 == n and end is not None:
                carets.append(dict(text_byte=end['text_byte'], x=end['x'], top=top, height=h))
        full = dict(text_start_byte=ts, text_end_byte=te, hit_rects=[rect], carets=carets)
        if 'synthetic_reason' in c: full['synthetic_reason'] = c['synthetic_reason']
        elif 'sources' in c: full['sources'] = c['sources']
        else:
            assert span is not None, 'implicit cluster without a run span'
            start = prev_src_end + c.get('s', 0); e = start + c.get('e', l); prev_src_end = e
            full['sources'] = [dict(path=span['path'], start_byte=start, end_byte=e)]
        out.append(full)
    run['clusters'] = out
def expand_payload(pl):
    """Expands every compact glyph run of a display_list (or delta changed_pages) payload."""
    if pl.get('cluster_encoding') is None: return pl
    assert pl['cluster_encoding'] == 'compact-1', pl['cluster_encoding']
    for p in pl.get('pages', []) + pl.get('changed_pages', []):
        for it in p['items']:
            if it.get('kind') == 'glyph_run': expand_run(it)
    return pl

# ---- dl2-canon-1 (docs/proposals/display-list-v2-delta.md, Appendix A) ----
def i64(n):  return struct.pack('<q', int(n))
def f64(x):
    x = float(x)
    if not math.isfinite(x): raise ValueError('non-finite paint component is not encodable')
    return struct.pack('<d', 0.0 if x == 0.0 else x)
def s(t):
    b = t.encode('utf-8'); return i64(len(b)) + b
def ranges(rs):
    out = i64(len(rs))
    for r in rs: out += s(r['path']) + i64(r['start_byte']) + i64(r['end_byte'])
    return out
def provenance(o):
    if 'sources' in o: return b'\x10' + ranges(o['sources'])
    return b'\x11' + s(o.get('synthetic_reason', ''))
def paint(p): return f64(p['r']) + f64(p['g']) + f64(p['b']) + f64(p['a'])
def item(it):
    if it['kind'] == 'glyph_run':
        out = b'\x01' + s(it['font_id']) + i64(it['font_size']) + s(it['text'])
        out += i64(len(it['glyphs']))
        for g in it['glyphs']:
            out += i64(g['gid']) + i64(g['origin_x']) + i64(g['baseline_y']) + i64(g['advance_x']) + i64(g['advance_y']) + i64(g['cluster'])
        out += i64(len(it['clusters']))
        for c in it['clusters']:
            out += i64(c['text_start_byte']) + i64(c['text_end_byte'])
            out += i64(len(c['hit_rects']))
            for r in c['hit_rects']: out += i64(r['x']) + i64(r['top']) + i64(r['width']) + i64(r['height'])
            out += i64(len(c['carets']))
            for k in c['carets']: out += i64(k['text_byte']) + i64(k['x']) + i64(k['top']) + i64(k['height'])
            out += provenance(c)
        return out + paint(it['paint'])
    if it['kind'] == 'rule':
        return b'\x02' + i64(it['x']) + i64(it['top']) + i64(it['width']) + i64(it['height']) + paint(it['paint']) + provenance(it)
    raise ValueError(it['kind'])
def page_digest(p):
    h = hashlib.sha256(b'flashtex:dl2:page:1\0')
    h.update(i64(p['number']) + i64(p['width']) + i64(p['height']) + i64(len(p['items'])))
    for it in p['items']: h.update(item(it))
    return h.hexdigest()
def header_digest(pl):
    h = hashlib.sha256(b'flashtex:dl2:header:1\0')
    for k in ('render_format', 'coordinate_unit', 'color_space', 'text_extraction', 'project_id'): h.update(s(pl[k]))
    h.update(i64(pl['revision']))
    h.update(i64(len(pl['required_features'])))
    for f in pl['required_features']: h.update(s(f))
    h.update(i64(len(pl['documents'])))
    for d in pl['documents']: h.update(s(d['path']) + i64(d['revision']) + s(d['sha256']) + i64(d['byte_length']))
    h.update(i64(len(pl['fonts'])))
    for f in pl['fonts']:
        h.update(s(f['font_id']) + s(f['sha256']) + i64(f['byte_length']) + s(f['format']) + i64(f['face_index']) + i64(f['units_per_em']) + i64(f['glyph_count']) + s(f['postscript_name']))
    h.update(i64(len(pl['diagnostics'])))
    for d in pl['diagnostics']: h.update(s(d['code']) + s(d['message']) + s(d['severity']) + ranges(d['sources']))
    return h.hexdigest()
def list_digest(pl, page_digests):
    h = hashlib.sha256(b'flashtex:dl2:list:1\0')
    h.update(bytes.fromhex(header_digest(pl)) + i64(len(page_digests)))
    for d in page_digests: h.update(bytes.fromhex(d))
    return h.hexdigest()

binary, path = sys.argv[1], sys.argv[2]
steps = int(sys.argv[3]) if len(sys.argv) > 3 and sys.argv[3].isdigit() else 40
v2 = "--no-v2" not in sys.argv
delta = "--delta" in sys.argv
only = "--only" in sys.argv
compact = "--compact" in sys.argv
dump_first = sys.argv[sys.argv.index("--dump-first") + 1] if "--dump-first" in sys.argv else None
md5_out = None
if "--md5" in sys.argv:
    md5_out = sys.argv[sys.argv.index("--md5") + 1]
text = open(path, encoding="utf-8").read()
name = os.path.basename(path)
at = text.rfind("\\end{document}")
if at < 0:
    at = len(text)
typed = "abcde fghij "
env = dict(os.environ)
env.setdefault("FLASHTEX_FONT_DIRS", os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../apps/mac/Fonts"))
p = subprocess.Popen([binary], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, env=env)
caps = ["rules-v1", "font-hints-v1"] + (["display-list-v2"] if v2 else [])
if v2 and delta:
    caps.append("display-list-v2-delta")
if v2 and only:
    caps.append("display-list-v2-only")
if v2 and compact:
    caps.append("display-list-v2-compact")
lat, sizes_v1, sizes_v2 = [], [], []
deltas = fulls = declined = 0
digest = hashlib.md5()
first = None
base = None  # installed-base acknowledgement (from the producer's own digests)



for step in range(steps + 1):
    body = text[:at] + "".join(typed[i % len(typed)] for i in range(step)) + text[at:]
    payload = {"project_id": "demo", "revision": step + 1, "entry_path": name,
               "documents": [{"path": name, "text": body}], "layout_capabilities": caps}
    if delta and base:
        payload["display_list_base"] = base
    req = {"protocol_version": 1, "id": f"mac-{step}", "type": "compile", "payload": payload}
    line = (json.dumps(req, separators=(",", ":")) + "\n").encode()
    t0 = time.perf_counter()
    p.stdin.write(line); p.stdin.flush()
    r1 = p.stdout.readline()
    t1 = time.perf_counter()
    r2 = b""
    if v2 and b'"display-list-v2"' in r1:
        r2 = p.stdout.readline()
    t2 = time.perf_counter()
    digest.update(r1); digest.update(r2)
    if r2 and step == 0 and dump_first:
        open(dump_first, "wb").write(r2)
    if r2:
        env2 = json.loads(r2)
        if compact:
            assert env2["payload"].get("cluster_encoding") == "compact-1", "compact requested but the sibling is not compact"
            expand_payload(env2["payload"])
        if env2["type"] == "display_list_delta":
            deltas += 1
            pl = env2["payload"]
            base = {"request_id": env2["id"], "project_id": pl["project_id"], "revision": pl["revision"],
                    "page_count": pl["page_count"], "list_digest": pl["list_digest"]}
        else:
            fulls += 1
            if delta:
                # Acknowledge the full frame as the Mac does: dl2-canon-1
                # (proposal Appendix A reference, embedded below).
                pl = env2["payload"]
                pds = [page_digest(pg) for pg in pl["pages"]]
                base = {"request_id": env2["id"], "project_id": pl["project_id"], "revision": pl["revision"],
                        "page_count": len(pl["pages"]), "list_digest": list_digest(pl, pds)}
    else:
        if v2:
            declined += 1
        base = None
    if step == 0:
        first = ((t2 - t0) * 1000, len(r1), len(r2))
        continue
    lat.append((t2 - t0) * 1000); sizes_v1.append(len(r1)); sizes_v2.append(len(r2))
p.stdin.close(); p.wait()
lat.sort()
def pct(q): return lat[min(len(lat) - 1, int(round(q * (len(lat) - 1))))]
mode = ("v2" if v2 else "v1") + ("+delta" if delta and v2 else "") + ("+only" if only and v2 else "") + ("+compact" if compact and v2 else "")
print(f"{name:12s} {mode:14s} steps={steps} first(cold) {first[0]:.1f} ms (v1 {first[1]} B, v2 {first[2]} B) | warm p50 {pct(0.5):.2f} p95 {pct(0.95):.2f} max {lat[-1]:.2f} ms | v1 {sum(sizes_v1)//len(sizes_v1)} B v2 {sum(sizes_v2)//len(sizes_v2)} B | deltas {deltas} full {fulls} declined {declined} | md5 {digest.hexdigest()[:16]}")
if md5_out:
    open(md5_out, "a").write(f"{name} {mode} steps={steps} {digest.hexdigest()}\n")
