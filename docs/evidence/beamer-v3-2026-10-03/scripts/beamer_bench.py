#!/usr/bin/env python3
"""Beamer incremental bench (lane BEAMER-V3): one `flashtex-host --socket` session on a long deck,
dl3-keys phases (t7.py's Host and keys) typing into a chosen frame's prose line (`--line`, `--page`).

usage: beamer_bench.py ENGINE DOC OUT DUMP.jsonl [KEYS]
DUMP.jsonl is `dl3-dump` of one compile of DOC: it maps each page to its frame's `\\end{frame}` line."""
import json, os, re, shutil, sys
W = os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', '..'))
sys.path.insert(0, f'{W}/tools/incr-bench')
import t7  # noqa: E402

IB = t7.IB
eng, doc, out, dump = f'{IB}/{sys.argv[1]}', sys.argv[2], sys.argv[3], sys.argv[4]
nkeys = int(sys.argv[5]) if len(sys.argv) > 5 else 20
os.makedirs(f'{out}/raw', exist_ok=True)
src = open(f'{IB}/docs/{doc}/main.tex').read().split('\n')

# frames: (begin line, end line, prose line), 1-based
frames, beg = [], None
for i, l in enumerate(src, 1):
    if l.startswith(r'\begin{frame}'):
        beg = i
    if l.startswith(r'\end{frame}') or (beg == i and r'\end{frame}' in l):
        prose = next((j for j in range(beg, i + 1) if len(src[j - 1].split(' ')) > 40), None)
        frames.append((beg, i, prose))
spans, files, page_lines = {}, {}, {}
for l in open(dump):
    j = json.loads(l)
    if j['kind'] == 'sources':
        files.update({f: p for f, p in j['files']})
        spans.update({s: (f, ln) for s, f, ln in j['spans']})
    elif j['kind'] == 'page':
        page_lines[j['index']] = {spans[it[1]][1] for it in j.get('items', [])
                                  if it[0] == 'span' and it[1] in spans and files.get(spans[it[1]][0], '').endswith('/main.tex')}
pages_of = {e: sorted(p for p, ls in page_lines.items() if e in ls) for _, e, _ in frames}
cand = [(b, e, pr, pages_of[e]) for b, e, pr in frames if pr and pages_of[e]]
npages = len(page_lines)


def pick(nslides, frac):
    c = [x for x in cand if len(x[3]) == nslides]
    return min(c, key=lambda x: abs(x[3][0] - frac * npages))


PH = []
for name, ns, frac, which, kind in [
        ('letter@start/1-slide', 1, 0.02, 0, 'letter'),
        ('letter@middle/1-slide', 1, 0.5, 0, 'letter'),
        ('letter@end/1-slide', 1, 0.98, 0, 'letter'),
        ('letter@middle/3-slides-first', 3, 0.5, 0, 'letter'),
        ('letter@middle/3-slides-last', 3, 0.5, -1, 'letter'),
        ('sentence@middle/3-slides-last', 3, 0.5, -1, 'sentence'),
        ('newline@middle/2-slides-last', 2, 0.5, -1, 'newline'),
        ('split@middle/2-slides-last', 2, 0.5, -1, 'split'),
        ('letter@end/3-slides-last', 3, 0.98, -1, 'letter')]:
    b, e, pr, pg = pick(ns, frac)
    PH.append((name, ['--kind', kind, '--line', str(pr), '--page', str(pg[which])], dict(frame_lines=[b, e], pages=pg)))
PH.append(('preamble', ['--kind', 'preamble', '--line', PH[0][1][3], '--page', '0'], {}))  # (the line is unused)

work = f'{IB}/t7-work/{doc}'
s0 = f'{IB}/t7-work/s0-{doc}'
sock = f'{IB}/t7-work/{doc}.sock'
shutil.rmtree(work, ignore_errors=True)
shutil.rmtree(s0, ignore_errors=True)
os.makedirs(f'{work}/out')
shutil.copy(f'{IB}/docs/{doc}/main.tex', f'{work}/main.tex')
h = t7.Host(eng, sock, s0, f'{out}/raw/{doc}.host-stderr', [])
res = {'doc': doc, 'pages': npages, 'phases': {}}
st = lambda v: dict(n=len(v), p50=round(t7.pct(v, .5), 1), p95=round(t7.pct(v, .95), 1), max=round(max(v), 1)) if v else None
try:
    for name, args, info in PH:
        n = 6 if name == 'preamble' else nkeys
        l0 = t7.load1()
        recs, where = t7.keys(eng, sock, work, args + ['--keys', str(n), '--gap-ms', '300'],
                              f'{out}/raw/{doc}-{name.replace("/", "_")}.jsonl', 3600)
        opens = [r for r in recs if 'open' in r]
        if opens and 'open' not in res:
            res['open'] = [dict(done_ms=round(r['done_ms'], 1), mode=r['host'].get('mode'), passes=r['host'].get('passes'),
                                typeset_pages=r['host'].get('typeset_pages')) for r in opens]
        ks = [r for r in recs if 'key' in r][1:]  # the first is a warm-up
        ed = [k['edited_page_ms'] for k in ks if k.get('edited_page_ms') is not None]
        fp = [k['first_page_ms'] for k in ks if k.get('first_page_ms') is not None]
        dn = [k['done_ms'] for k in ks]
        conv = [k['host'].get('converged_at') for k in ks]
        tp = sorted(k['host'].get('typeset_pages') or 0 for k in ks)
        res['phases'][name] = dict(info, where=where, edited_page_ms=st(ed), first_page_ms=st(fp), done_ms=st(dn),
                                   samples=len(ks), converged=sum(c is not None for c in conv),
                                   typeset_pages_median=tp[len(tp) // 2] if tp else None,
                                   modes=sorted({k['host'].get('mode') for k in ks}),
                                   complete=sum(bool(k.get('complete')) for k in ks), missing_edited=len(ks) - len(ed),
                                   load=[l0, t7.load1()])
        print(name, json.dumps(res['phases'][name]), flush=True)
    h.settle_saves()
finally:
    res['rss_peak'] = h.stop()
json.dump(res, open(f'{out}/summary-{doc}.json', 'w'), indent=1)
