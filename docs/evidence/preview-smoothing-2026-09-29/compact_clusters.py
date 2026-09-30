"""Collapse each glyph run's clusters into one (rendering is unchanged:
positions, glyph ids, fonts and paint are kept byte for byte)."""
import json, sys
src, dst = sys.argv[1], sys.argv[2]
d = json.load(open(src))
for pg in d['payload']['pages']:
    for it in pg['items']:
        if it.get('kind') != 'glyph_run':
            continue
        c = it['clusters']
        first = c[0]
        srcs = [s for cl in c for s in (cl.get('sources') or [])]
        new = {'carets': [], 'hit_rects': [first['hit_rects'][0]], 'text_start_byte': 0,
               'text_end_byte': len(it['text'].encode())}
        if srcs:
            new['sources'] = [{'path': srcs[0]['path'], 'start_byte': min(s['start_byte'] for s in srcs),
                               'end_byte': max(s['end_byte'] for s in srcs)}]
        elif 'synthetic_reason' in first:
            new['synthetic_reason'] = first['synthetic_reason']
        it['clusters'] = [new]
        for g in it['glyphs']:
            g['cluster'] = 0
json.dump(d, open(dst, 'w'), separators=(',', ':'))
