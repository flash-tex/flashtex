#!/usr/bin/env python3
"""collect.py EVDIR TABLENAME "TAGS" [KEYDOC "KEYTAGS"]: copy runs and tables into the evidence dir."""
import gzip, os, shutil, subprocess, sys
E = sys.argv[1]
os.makedirs(f'{E}/raw/runs', exist_ok=True)
os.makedirs(f'{E}/scripts', exist_ok=True)
for f in os.listdir('/tmp/mfp/ev/scripts'):
    shutil.copy(f'/tmp/mfp/ev/scripts/{f}', f'{E}/scripts/{f}')
for f in os.listdir('/tmp/mfp/ev/raw'):
    if f.endswith('.txt'):
        shutil.copy(f'/tmp/mfp/ev/raw/{f}', f'{E}/raw/{f}')
tags = sys.argv[3].split()
out = subprocess.run(['python3', '/tmp/mfp/table.py'] + tags, capture_output=True, text=True).stdout
open(f'{E}/raw/table-{sys.argv[2]}.txt', 'w').write(out)
if len(sys.argv) > 5:
    out = subprocess.run(['python3', '/tmp/mfp/keys.py', sys.argv[4]] + sys.argv[5].split(), capture_output=True, text=True).stdout
    open(f'{E}/raw/keys-{sys.argv[2]}-{sys.argv[4]}.txt', 'w').write(out)
for t in tags:
    for d in ('full-100', 'long-deck', 'infdesc'):
        src = f'/tmp/mfp/out/{t}-{d}'
        if os.path.exists(src + '.jsonl'):
            with open(src + '.jsonl', 'rb') as i, gzip.open(f'{E}/raw/runs/{t}-{d}.jsonl.gz', 'wb') as o:
                o.write(i.read())
            if os.path.exists(src + '.vmmap'):
                shutil.copy(src + '.vmmap', f'{E}/raw/runs/{t}-{d}.vmmap')
print(out)
