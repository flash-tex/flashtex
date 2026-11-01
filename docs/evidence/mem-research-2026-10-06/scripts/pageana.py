#!/usr/bin/env python3
"""pageana.py DUMP: the host's page cache (FLASHTEX_DUMP_PAGES): size, per-page compression, dedup."""
import hashlib, os, struct, subprocess, sys, tempfile, time, zlib
b = open(sys.argv[1], 'rb').read()
p = 0
pages, forms = [], []
while p < len(b):
    form = b[p]; idx, n = struct.unpack_from('<IQ', b, p + 1); p += 13
    (forms if form else pages).append(b[p:p + n]); p += n
MB = 1 / 2**20
tp = sum(map(len, pages)); tf = sum(map(len, forms))
print(f'pages {len(pages)} {tp*MB:.1f} MB (p50 {sorted(map(len,pages))[len(pages)//2]/1024:.0f} KB), forms {len(forms)} {tf*MB:.1f} MB')
uniq = len({hashlib.sha256(x).digest() for x in pages})
print(f'distinct page bodies {uniq}')
t = time.perf_counter(); z1 = sum(len(zlib.compress(x, 1)) for x in pages); tz = time.perf_counter() - t
t = time.perf_counter()
for x in pages:
    zlib.decompress(zlib.compress(x, 1))
print(f'zlib-1 per page: {z1/tp:.3f} of the bytes; compress {tz/len(pages)*1e3:.2f} ms/page')
d = tempfile.mkdtemp()
samp = pages[::max(1, len(pages)//200)]
for i, x in enumerate(samp):
    open(f'{d}/p{i}', 'wb').write(x)
raw = sum(map(len, samp))
for lvl in (1, 3):
    z = 0
    for i in range(len(samp)):
        z += len(subprocess.run(['zstd', f'-{lvl}', '-q', '-c', f'{d}/p{i}'], capture_output=True).stdout)
    print(f'zstd-{lvl} per page (200 sampled): {z/raw:.3f}')
subprocess.run(['zstd', '--train', '-q', '-o', f'{d}/dict'] + [f'{d}/p{i}' for i in range(len(samp))], capture_output=True)
if os.path.exists(f'{d}/dict'):
    z = 0
    for i in range(len(samp)):
        z += len(subprocess.run(['zstd', '-3', '-q', '-c', '-D', f'{d}/dict', f'{d}/p{i}'], capture_output=True).stdout)
    print(f'zstd-3 with a trained dictionary ({os.path.getsize(d+"/dict")/1024:.0f} KB): {z/raw:.3f}')
bench = subprocess.run(['zstd', '-b1', '-q'] + [f'{d}/p{i}' for i in range(len(samp))], capture_output=True, text=True)
print('zstd -b1 (all sampled pages as one):', (bench.stdout + bench.stderr).strip().splitlines()[-1:])
