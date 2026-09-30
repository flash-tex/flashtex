#!/usr/bin/env python3
"""book_sum.py TAG [DIR]: summarise book_matrix.sh's sessions (/tmp/p4f/book/TAG-*.jsonl).

Per session (page, where, letter or sentence): the byte the keystrokes edit and the line it is
in, then per keystroke: client ms to the edited page, the host's first page and its thread CPU,
apply/find/restore, the restart point (pages shipped before it, a segment or a page checkpoint,
bytes before the edit), where the run converged and how many pages it typeset, and DONE.
The restart check: the checkpoint after the one restarted from must read the edited file past
the edit (restart_next_gap > 0; the host reports it), so the restart point is the newest
checkpoint before the edit. Checkpoints between pages are taken after `build_page` at least
0.5 ms of engine time apart, so the newest one can lie a few paragraphs before the edited line;
"in line" says whether it is in the edited line or the blank line before it."""
import glob
import json
import os
import re
import sys

tag = sys.argv[1]
d = sys.argv[2] if len(sys.argv) > 2 else '/tmp/p4f/book'
src = os.path.expanduser('~/Documents/FlashTeX-1000-page-test/book.tex')
text = open(src, 'rb').read()


def pct(v, p):
    v = sorted(x for x in v if isinstance(x, (int, float)))
    return v[round((len(v) - 1) * p)] if v else float('nan')


def key(f):
    m = re.match(rf'{re.escape(tag)}-(\d+)-(\w+?)(-sentence)?\.jsonl$', os.path.basename(f))
    return (int(m.group(1)), ['start', 'middle', 'end'].index(m.group(2)), bool(m.group(3)))


bad = 0
far = 0
rows = []
for f in sorted(glob.glob(f'{d}/{tag}-*.jsonl'), key=key):
    page, w, sent = key(f)
    err = open(f[:-6] + '.err').read()
    m = re.search(r'typing on line (\d+) \(page (\d+)\), byte (\d+)', err)
    if not m:
        print(f'{f}: no keystrokes ({err.strip()[-200:]})')
        continue
    line, at = int(m.group(1)), int(m.group(3))
    col = at - (text.rfind(b'\n', 0, at) + 1)
    keys = [json.loads(l) for l in open(f) if l.startswith('{"key"')]
    print(f"== page {page} {['start', 'middle', 'end'][w]} {'sentence' if sent else 'letter'}: "
          f"line {line}, byte {at} (column {col})")
    for k in keys:
        h = k['host']
        st = h.get('stages', {})
        gap = h.get('restart_gap')
        nxt = h.get('restart_next_gap', 'n/a')
        ok = nxt is None or (isinstance(nxt, int) and nxt > 0)
        inline = isinstance(gap, int) and gap <= col + 2
        bad += 0 if ok else 1
        far += 0 if inline else 1
        print(f"   key {k['key']}: edited p{k.get('edited_page')} {k.get('edited_page_ms') or float('nan'):7.1f} ms"
              f" | host first p{k.get('first_index')} {st.get('first_page', float('nan')):6.1f}"
              f" cpu {st.get('first_page_cpu', float('nan')):6.1f}"
              f" | apply {st.get('apply', 0):4.1f} find {st.get('find', 0):4.1f} restore {st.get('restore', 0):5.1f}"
              f" | restart: {h.get('restart_page')} pages, {'segment' if h.get('restart_mid_page') else 'page'},"
              f" {gap} B before, next {nxt}{'' if ok else '  <-- A LATER CHECKPOINT WAS USABLE'}{'' if inline else ' (not in the line)'}"
              f" | converged {h.get('converged_at')} rerun {h.get('rerun_pages')} tests {st.get('tests')}"
              f" | DONE {k.get('done_ms', float('nan')):7.0f} ms")
        rows.append((page, w, sent, k))
print()
print('restart checks: %d keystrokes; %d restarted before a later usable checkpoint; %d from a checkpoint before the edited line' % (len(rows), bad, far))
for sent in (False, True):
    for page in sorted({r[0] for r in rows}):
        v = [r[3].get('edited_page_ms') for r in rows if r[0] == page and r[2] == sent]
        c = [r[3]['host'].get('stages', {}).get('first_page_cpu') for r in rows if r[0] == page and r[2] == sent]
        dn = [r[3].get('done_ms') for r in rows if r[0] == page and r[2] == sent]
        print(f"{'sentence' if sent else 'letter  '} page {page:4d}: edited page p50 {pct(v, .5):6.1f} p95 {pct(v, .95):6.1f} ms;"
              f" host first-page CPU p50 {pct(c, .5):6.1f}; DONE p50 {pct(dn, .5):7.0f} ms (n {len(v)})")
