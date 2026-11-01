"""window.py HTIME AUX1 AUX2: in pass 2 (between the first and second AuxDone), how much engine
work a pass 3 would have to redo if it re-ran, around every read of an .aux entry that changed,
(a) from the checkpoint before the page's first such read to the page's end, or
(b) from the checkpoint before each such read to the checkpoint after it (convergence at any
checkpoint)."""
import re
import sys

sys.path.insert(0, __import__('os').path.dirname(__import__('os').path.abspath(__file__)))
from changed import defs  # noqa: E402

a, b = defs(sys.argv[2]), defs(sys.argv[3])
ch = {n for n in set(a) | set(b) if a.get(n) != b.get(n)}
ev = []
n_auxdone = 0
for line in open(sys.argv[1], encoding='latin-1'):
    if not line.startswith('[watch]'):
        continue
    f = line.split()
    if f[1] == 'ck':
        if f[2] == 'AuxDone':
            n_auxdone += 1
            continue
        if n_auxdone == 1:
            ev.append(('ck', f[2], int(f[4])))
    elif f[1] == 'read' and n_auxdone == 1:
        name = ' '.join(f[4:])
        if name in ch:
            ev.append(('rd', name, int(f[3])))
ships = [i for i, e in enumerate(ev) if e[0] == 'ck' and e[1] == 'Shipout']
total = ev[ships[-1]][2] - ev[0][2]
win_a = 0
win_b = 0
nspan = [0]
pages_read = 0
prev_ship = 0
first_i = ev[0][2]
for s in ships:
    seg = ev[prev_ship:s + 1]
    reads = [k for k, e in enumerate(seg) if e[0] == 'rd']
    if reads:
        pages_read += 1
        k0 = reads[0]
        before = max((k for k in range(k0) if seg[k][0] == 'ck'), default=None)
        start = seg[before][2] if before is not None else (ev[prev_ship - 1][2] if prev_ship else first_i)
        win_a += seg[-1][2] - start
        # (b): union of [ck before read, ck after read]
        spans = []
        for k in reads:
            lo = max((j for j in range(k) if seg[j][0] == 'ck'), default=None)
            hi = min((j for j in range(k + 1, len(seg)) if seg[j][0] == 'ck'))
            lo_i = seg[lo][2] if lo is not None else (ev[prev_ship - 1][2] if prev_ship else first_i)
            spans.append((lo_i, seg[hi][2]))
        spans.sort()
        cur = None
        for lo, hi in spans:
            if cur and lo <= cur[1]:
                cur = (cur[0], max(cur[1], hi))
            else:
                if cur:
                    win_b += cur[1] - cur[0]; nspan[0] += 1
                cur = (lo, hi)
        if cur:
            win_b += cur[1] - cur[0]; nspan[0] += 1
    prev_ship = s + 1
print(f"changed names {len(ch)}; pages {len(ships)}; pages reading one {pages_read}")
print(f"pass-2 body instructions {total / 1e9:.2f} G")
print(f"(a) page windows: {win_a / 1e9:.2f} G = {100 * win_a / total:.1f} %")
print(f"(b) checkpoint windows: {win_b / 1e9:.2f} G = {100 * win_b / total:.1f} %, {nspan[0]} windows")
# per page: window (a) fraction, and whether the only changed read is \@abspage@last
prev_ship = 0
rows = []
for s in ships:
    seg = ev[prev_ship:s + 1]
    start_page = ev[prev_ship - 1][2] if prev_ship else first_i
    reads = [k for k, e in enumerate(seg) if e[0] == 'rd']
    names = {seg[k][1] for k in reads}
    if reads:
        k0 = reads[0]
        before = max((k for k in range(k0) if seg[k][0] == 'ck'), default=None)
        start = seg[before][2] if before is not None else start_page
        rows.append((names <= {'\\@abspage@last'}, (seg[-1][2] - start) / max(1, seg[-1][2] - start_page)))
    prev_ship = s + 1
for only in (True, False):
    fr = sorted(f for o, f in rows if o == only)
    if fr:
        print(f"abspage-only={only}: {len(fr)} pages, window fraction p50 {fr[len(fr)//2]:.2f} mean {sum(fr)/len(fr):.2f}")
