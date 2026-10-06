import sys, struct, array, collections
data = open(sys.argv[1], 'rb').read()
off = 0
logs = []
while off < len(data):
    nd, nw = struct.unpack_from('<QQ', data, off); off += 16
    d = array.array('Q'); d.frombytes(data[off:off + nd * 24]); off += nd * 24
    off += nw * 8
    logs.append([(d[i], d[i + 1] | (d[i + 2] << 64)) for i in range(0, len(d), 3)])
for k in [1, 2, 3, 5, 10, 20, 50]:
    words = deltas = 0
    for g in range(0, len(logs), k):
        m = collections.defaultdict(int)
        for l in logs[g:g + k]:
            for c, mask in l:
                m[c] |= mask
        deltas += len(m)
        words += sum(bin(v).count('1') for v in m.values())
    print(f'merge every {k:3d}: deltas {deltas*24/1e6:7.1f} MB words {words*8/1e6:7.1f} MB total {(deltas*24+words*8)/1e6:7.1f} MB, checkpoints {len(range(0, len(logs), k))}')
