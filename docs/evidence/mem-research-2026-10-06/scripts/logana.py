#!/usr/bin/env python3
"""logana.py DUMP: offline analysis of a FLASHTEX_DUMP_LOGS dump (MEM-RESEARCH prototype).
Prints: bytes now (deltas header / packed words / Vec slack), regions, alternative codecs
(XOR / arithmetic delta against the post-image, compact headers), zstd on cold logs, dedup."""
import hashlib, struct, subprocess, sys, time, zlib
import numpy as np

buf = open(sys.argv[1], 'rb').read()
p = 0
def u64():
    global p
    v = struct.unpack_from('<Q', buf, p)[0]; p += 8; return v
def u32():
    global p
    v = struct.unpack_from('<I', buf, p)[0]; p += 4; return v
assert buf[:8] == b'FTLOGS01'; p = 8
regions = []
for _ in range(u64()):
    n = u64(); name = buf[p:p + n].decode(); p += n
    off = u64(); by = u64(); regions.append((off, by, name))
regions.sort()
nlogs = u64()
logs = []
# per-delta arrays
D_log, D_c, D_m0, D_m1, D_n, chunks = [], [], [], [], [], []
sealed_tot = bytes_tot = 0
for li in range(nlogs):
    cid = u64(); ent = u64(); nd_ = u64(); bl = u64(); sealed = u64()
    sealed_tot += sealed; bytes_tot += bl
    logs.append((cid, ent, nd_, bl, sealed))
    for _ in range(nd_):
        c, m0, m1 = struct.unpack_from('<IQQ', buf, p); p += 20
        n = m0.bit_count() + m1.bit_count()
        chunks.append(buf[p:p + 8 * n]); p += 8 * n
        D_log.append(li); D_c.append(c); D_m0.append(m0); D_m1.append(m1); D_n.append(n)
nseen = u64()
live = {}
for _ in range(nseen):
    c = u32(); live[c] = np.frombuffer(buf, dtype='<u8', count=128, offset=p); p += 1024
assert p == len(buf), (p, len(buf))

nd = len(D_c)
vals = np.frombuffer(b''.join(chunks), dtype='<u8')
M = np.stack([np.array(D_m0, dtype=np.uint64), np.array(D_m1, dtype=np.uint64)], 1)
bitsm = np.unpackbits(M.view(np.uint8).reshape(nd, 16), axis=1, bitorder='little').astype(bool)
di, bi = np.nonzero(bitsm)
c_np = np.array(D_c, dtype=np.int64)
addr = c_np[di] * 128 + bi
l_arr = np.array(D_log)
wlog = l_arr[di]
starts_d = np.r_[0, np.cumsum(D_n)[:-1]]
W_vals = None
nw = len(vals)
MB = 1 / 2**20
print(f'logs {nlogs}  deltas {nd:,}  words {nw:,}  words/delta {nw/max(nd,1):.1f}')
hdr = 24 * nd
print(f'NOW: sealed (capacity) {sealed_tot*MB:.1f} MB = delta headers {hdr*MB:.1f} + packed words {bytes_tot*MB:.1f}'
      f' + Vec slack {(sealed_tot-hdr-bytes_tot)*MB:.1f}; raw words would be {nw*8*MB:.1f} MB')

def halflen(x):
    x = x.astype(np.int64)
    x = np.where(x >= 2**31, x - 2**32, x)
    return np.where(x == 0, 0, np.where((x >= -128) & (x < 128), 1, np.where((x >= -32768) & (x < 32768), 2, 4)))
def packed(v):
    lo = v & np.uint64(0xffffffff); hi = v >> np.uint64(32)
    return halflen(lo) + halflen(hi)
cur = packed(vals)
print(f'codec check: recomputed packed {(cur.sum() + (np.array(D_n)+1)//2 @ np.ones(nd)) * MB:.1f} MB vs dump {bytes_tot*MB:.1f}')

# regions by word address
starts = np.array([r[0] for r in regions]); byaddr = addr * 8
ri = np.searchsorted(starts, byaddr, side='right') - 1
names = [r[2] for r in regions]
tot = {}
for i, b in zip(*np.unique(ri, return_counts=True)):
    tot[names[i]] = b
bytes_by = {}
for i in np.unique(ri):
    sel = ri == i
    bytes_by[names[i]] = cur[sel].sum() + sel.sum() / 2
print('words by region (share of words, packed MB):')
for k, v in sorted(tot.items(), key=lambda kv: -kv[1])[:12]:
    print(f'   {k:24} {v/nw*100:5.1f} %  {bytes_by[k]*MB:7.1f} MB')

# post-image: the next newer log's pre value of the same word, else the live value
o = np.lexsort((wlog, addr))
a_s, v_s = addr[o], vals[o]
post_s = np.empty_like(v_s)
same_next = np.zeros(len(a_s), bool); same_next[:-1] = a_s[1:] == a_s[:-1]
post_s[:-1] = v_s[1:]
lastidx = np.where(~same_next)[0]
for j in lastidx:
    c, w = divmod(int(a_s[j]), 128)
    post_s[j] = live[c][w]
post = np.empty_like(vals); post[o] = post_s
xor = vals ^ post
diff = (vals.astype(np.int64) - post.astype(np.int64)).astype(np.uint64)
# per-half arithmetic difference
dlo = ((vals & np.uint64(0xffffffff)).astype(np.int64) - (post & np.uint64(0xffffffff)).astype(np.int64)) & 0xffffffff
dhi = ((vals >> np.uint64(32)).astype(np.int64) - (post >> np.uint64(32)).astype(np.int64)) & 0xffffffff
hdiff = (dlo.astype(np.uint64)) | (dhi.astype(np.uint64) << np.uint64(32))
nib = nw / 2
for name, v in [('pre (now)', vals), ('xor post', xor), ('pre-post (64-bit)', diff), ('pre-post per half', hdiff)]:
    b = packed(v).sum() + nib
    best = np.minimum(packed(vals), packed(v)).sum() + nib + nw / 8
    print(f'  words as {name:20} {b*MB:7.1f} MB   (per-word best-of with 1 flag bit: {best*MB:7.1f} MB)')
print(f'  pre-image half zero: lo {np.mean((vals & np.uint64(0xffffffff))==0)*100:.0f} % hi {np.mean((vals>>np.uint64(32))==0)*100:.0f} %;'
      f' only one half changed: {np.mean(((xor & np.uint64(0xffffffff))==0) | ((xor>>np.uint64(32))==0))*100:.0f} %')

# compact headers: chunk gap varint + mask as runs (<=16 bytes) + an index every 16 deltas
c_arr = c_np
gap = np.diff(c_arr, prepend=0); gap[np.r_[True, l_arr[1:] != l_arr[:-1]]] = c_arr[np.r_[True, l_arr[1:] != l_arr[:-1]]]
vlen = np.where(gap < 0, 5, np.where(gap < 128, 1, np.where(gap < 16384, 2, np.where(gap < 2**21, 3, 4))))
def runs(m0, m1):
    m = m0 | m1 << 64
    r = 0; prev = 0
    for i in range(128):
        b = m >> i & 1
        if b and not prev: r += 1
        prev = b
    return r
nruns = (bitsm & ~np.c_[np.zeros((nd, 1), bool), bitsm[:, :-1]]).sum(1)
mlen = np.minimum(16, 1 + 2 * nruns)
hdr2 = vlen.sum() + mlen.sum() + nd / 16 * 4
print(f'headers: now {hdr*MB:.1f} MB; varint gap + run-coded mask + an offset every 16: {hdr2*MB:.1f} MB')

# dedup across logs
psz = np.add.reduceat(cur, starts_d) if nd else np.array([])
psz = psz + (np.array(D_n) + 1) // 2 + 24
h_full, h_val = set(), set()
dup_full = dup_val = 0
for i in range(nd):
    v = chunks[i]
    k1 = hashlib.blake2b(struct.pack('<IQQ', D_c[i], D_m0[i], D_m1[i]) + v, digest_size=16).digest()
    if k1 in h_full: dup_full += psz[i]
    else: h_full.add(k1)
    k2 = hashlib.blake2b(struct.pack('<QQ', D_m0[i], D_m1[i]) + v, digest_size=16).digest()
    if k2 in h_val: dup_val += psz[i]
    else: h_val.add(k2)
print(f'dedup: identical (chunk, mask, values) in another log: {dup_full*MB:.1f} MB; identical (mask, values) anywhere: {dup_val*MB:.1f} MB')

# zstd / zlib over sealed logs serialised as now (24-byte headers + packed bytes), per log and in groups of 32
def ser(li_from, li_to):
    out = bytearray()
    sel = np.where((l_arr >= li_from) & (l_arr < li_to))[0]
    for i in sel:
        v = np.frombuffer(chunks[i], dtype='<u8')
        out += struct.pack('<IIQQ', D_c[i], 0, D_m0[i], D_m1[i])
        lo = v & np.uint64(0xffffffff); hi = v >> np.uint64(32)
        # the codec's bytes (tags then halves) -- approximated by the halves at their widths
        for x in np.stack([lo, hi], 1).ravel():
            x = int(x); s = x - 2**32 if x >= 2**31 else x
            if s == 0: continue
            out += struct.pack('<b', s) if -128 <= s < 128 else struct.pack('<h', s) if -32768 <= s < 32768 else struct.pack('<I', x)
    return bytes(out)
def zstd(data, lvl):
    t = time.perf_counter()
    z = subprocess.run(['zstd', f'-{lvl}', '-q', '-c'], input=data, capture_output=True).stdout
    return len(z)
sample = list(range(0, nlogs, max(1, nlogs // 40)))
raw = z1 = z3 = zl = 0
for li in sample:
    s = ser(li, li + 1)
    if not s: continue
    raw += len(s); z1 += zstd(s, 1); z3 += zstd(s, 3); zl += len(zlib.compress(s, 1))
print(f'per-log compression (40 sampled logs, {raw*MB:.1f} MB as now): zstd-1 {z1/raw:.2f}, zstd-3 {z3/raw:.2f}, zlib-1 {zl/raw:.2f}')
graw = gz = 0
for g in range(0, nlogs, max(1, nlogs // 8)):
    s = ser(g, g + 32)
    if not s: continue
    graw += len(s); gz += zstd(s, 3)
print(f'groups of 32 logs: zstd-3 {gz/max(graw,1):.2f}')
cnt = np.bincount(l_arr, minlength=nlogs)
print('log sizes (deltas): p50', int(np.median(cnt)), 'p95', int(np.percentile(cnt, 95)), 'max', int(cnt.max()))

# retention: merge every k consecutive logs (keep one checkpoint in k), older value wins
for k in (2, 4, 8, 16):
    g = wlog // k
    o2 = np.lexsort((wlog, addr, g))
    gs, as_ = g[o2], addr[o2]
    first = np.r_[True, (gs[1:] != gs[:-1]) | (as_[1:] != as_[:-1])]
    keep = o2[first]
    nd_k = len(np.unique(g[keep] * (1 << 40) + addr[keep] // 128))
    by = packed(vals[keep]).sum() + len(keep) / 2 + 24 * nd_k
    print(f'keep 1 checkpoint in {k:2}: {nd_k:,} deltas, {len(keep):,} words, {by*MB:.1f} MB (now {(hdr+bytes_tot)*MB:.1f})')
lb = np.bincount(wlog, weights=cur + 0.5, minlength=nlogs) + 24 * np.bincount(l_arr, minlength=nlogs)
print('bytes by decile of the chain (oldest first), MB:', ' '.join(f'{d.sum()*MB:.0f}' for d in np.array_split(lb, 10)))
