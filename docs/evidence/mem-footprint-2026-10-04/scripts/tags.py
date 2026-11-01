import sys, struct, array
data = open(sys.argv[1], 'rb').read()
step = int(sys.argv[2]) if len(sys.argv) > 2 else 1
off = 0
n = 0
halves = 0
whole = 0
li = 0
def sx(x):
    return x - (1 << 32) if x >= 1 << 31 else x
def th(x):
    a = -x - 1 if x < 0 else x
    return 0 if x == 0 else 1 if a < 128 else 2 if a < 32768 else 4
def tw(x):
    return 0 if x == 0 else 1 if x < 256 else 4 if x < (1 << 32) else 8
while off < len(data):
    nd, nw = struct.unpack_from('<QQ', data, off); off += 16
    off += nd * 24
    if li % step == 0:
        w = array.array('Q'); w.frombytes(data[off:off + nw * 8])
        for x in w:
            n += 1
            halves += th(sx(x & 0xffffffff)) + th(sx(x >> 32))
            whole += tw(x)
    off += nw * 8
    li += 1
print(f'words {n}: halves 2+2-bit tags {(halves + n / 2) / (n * 8):.3f}; whole-word 2-bit tag {(whole + n / 4) / (n * 8):.3f}')
