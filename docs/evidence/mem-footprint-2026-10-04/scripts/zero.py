import sys, struct, array
data = open(sys.argv[1], 'rb').read()
thr = int(sys.argv[2]) if len(sys.argv) > 2 else 3
off = 0
before = after = nd_all = zd = 0
while off < len(data):
    nd, nw = struct.unpack_from('<QQ', data, off); off += 16
    d = array.array('Q'); d.frombytes(data[off:off + nd * 24]); off += nd * 24
    w = array.array('Q'); w.frombytes(data[off:off + nw * 8]); off += nw * 8
    k = 0
    for i in range(0, len(d), 3):
        n = bin(d[i + 1]).count('1') + bin(d[i + 2]).count('1')
        z = sum(1 for x in w[k:k + n] if x == 0)
        k += n
        before += n * 8
        if z >= thr:
            after += 16 + (n - z) * 8; zd += 1
        else:
            after += n * 8
    nd_all += nd
print(f'words {before/1e6:.1f} MB -> {after/1e6:.1f} MB (deltas {nd_all}, zero-form {zd}); delta headers {nd_all*24/1e6:.1f} MB')
