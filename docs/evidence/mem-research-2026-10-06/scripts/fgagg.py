#!/usr/bin/env python3
"""fgagg.py FG: heaptrack flamegraph (peak cost) summed by owner: the innermost frame matching a rule."""
import sys, collections
RULES = [  # (label, substring of a frame), checked innermost first
    ('kpathsea db, via host main thread (server::main)', ('kpathsea_init_db', 'server::main')),
    ('kpathsea db, via engine thread (system::configure)', ('kpathsea_init_db', 'system::configure')),
    ('kpathsea db, other', ('kpathsea_init_db',)),
    ('kpathsea other', ('kpathsea',)),
    ('font map (fm_read_info / avl_do_entry / read_field)', ('fm_read_info',)),
    ('font map other', ('fm_',)),
    ('undo logs: seal', ('::seal',)),
    ('undo logs: retain/merge', ('::retain',)),
    ('undo logs: thin_pending', ('thin_pending',)),
    ('convergence diff (diff_branch)', ('diff_branch',)),
    ('restore: rewind/prepare', ('rewind',)),
    ('restore: prepare_restore', ('prepare_restore',)),
    ('display list encode (dl_shipout_end)', ('dl_shipout_end',)),
    ('display list other', ('displaylist',)),
    ('page cache (resident put)', ('resident::',)),
    ('checkpoint records (capture_ext)', ('capture_ext',)),
    ('checkpoint other', ('checkpoint',)),
    ('zlib deflate', ('deflateInit',)),
    ('xpdf / pdf inclusion', ('xpdf',)),
    ('vf packets', ('vfpacket',)),
    ('read output tail', ('read_tail',)),
    ('free_cells', ('free_cells',)),
    ('incr other', ('incr::',)),
    ('server/connection', ('server::',)),
    ('format load / undump', ('undump',)),
    ('engine main_control', ('main_control',)),
]
agg = collections.Counter()
for line in open(sys.argv[1]):
    stack, _, n = line.rstrip().rpartition(' ')
    frames = stack.split(';')
    lab = None
    for f in reversed(frames):
        for name, pats in RULES:
            if pats[0] in f and all(any(q in g for g in frames) for q in pats[1:]):
                lab = name
                break
        if lab:
            break
    agg[lab or 'other: ' + ';'.join(frames[-3:])[:100]] += int(n)
tot = sum(agg.values())
print(f'total at peak {tot/2**20:.1f} MB')
for k, v in agg.most_common(30):
    print(f'{v/2**20:8.1f} MB  {k}')
