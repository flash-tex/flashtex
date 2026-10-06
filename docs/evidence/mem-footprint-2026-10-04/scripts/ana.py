import sys, struct, zlib, collections, array, time
data = open(sys.argv[1], 'rb').read()
step = int(sys.argv[2]) if len(sys.argv) > 2 else 1
off = 0
logs = []
while off < len(data):
    nd, nw = struct.unpack_from('<QQ', data, off); off += 16
    d = array.array('Q'); d.frombytes(data[off:off + nd * 24]); off += nd * 24
    w = array.array('Q'); w.frombytes(data[off:off + nw * 8]); off += nw * 8
    logs.append((d, w))
D = sum(len(d) // 3 for d, _ in logs)
NW = sum(len(w) for _, w in logs)
print(f'logs {len(logs)}, deltas {D} ({D*24/1e6:.1f} MB), words {NW} ({NW*8/1e6:.1f} MB), words/delta {NW/D:.1f}')
s = logs[::step]
cnt = collections.Counter()
n = 0
vb = 0
tb = 0
def sx(x):
    return x - (1 << 32) if x >= 1 << 31 else x
def vl(x):
    z = 2 * x if x >= 0 else -2 * x - 1
    k = 1
    while z >= 128:
        z >>= 7; k += 1
    return k
def tg(x):
    a = -x - 1 if x < 0 else x
    return 0 if x == 0 else 1 if a < 128 else 2 if a < 32768 else 4
for _, w in s:
    for x in w:
        n += 1
        lo, hi = sx(x & 0xffffffff), sx(x >> 32)
        if x == 0: cnt['zero'] += 1
        if hi == 0: cnt['hi0'] += 1
        if lo == 0: cnt['lo0'] += 1
        if -32768 <= hi < 32768: cnt['hi16'] += 1
        if -32768 <= lo < 32768: cnt['lo16'] += 1
        vb += vl(lo) + vl(hi)
        tb += tg(lo) + tg(hi) + 0.5
print({k: round(v / n, 3) for k, v in cnt.items()})
print(f'varint halves ratio {vb/(n*8):.3f}; 2-bit-tag halves ratio {tb/(n*8):.3f}')
t = time.time(); zb = sum(len(zlib.compress(w.tobytes(), 1)) for _, w in s); dt = time.time() - t
sb = sum(len(w) * 8 for _, w in s)
print(f'zlib-1 per log ratio {zb/sb:.3f} at {sb/1e6/dt:.0f} MB/s')
zb = sum(len(zlib.compress(w.tobytes(), 6)) for _, w in s[:100]) / sum(len(w) * 8 for _, w in s[:100])
print(f'zlib-6 ratio {zb:.3f}')
pc = collections.Counter()
for d, _ in s:
    for i in range(0, len(d), 3):
        pc[min(16, bin(d[i + 1]).count('1') + bin(d[i + 2]).count('1'))] += 1
tot = sum(pc.values())
print('words per delta:', [(k, round(v / tot, 3)) for k, v in sorted(pc.items())])
cs = collections.Counter()
for d, _ in logs:
    for i in range(0, len(d), 3):
        cs[d[i]] += 1
print('distinct chunks', len(cs), 'mean logs per chunk', round(sum(cs.values()) / len(cs), 1))
