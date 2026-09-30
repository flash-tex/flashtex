#!/usr/bin/env python3
"""Reopen benchmark (DESIGN.md §1.2: reopening a recently edited document
<= 100 ms to the first visible page). usage: reopen.py ENGINE SRCDIR DOC [REPS]

1. A host compiles DOC until settled and saves S0.
2. Fresh process: start flashtex-host iserve, `open S0 1` (stop at page 1):
   wall time from process start to the answer, and the host's own times.
3. Pre-warmed host: start flashtex-host iserve, `warm DIR` (kpathsea and the
   font map, on a throwaway one-page document), then `open S0 1`: the
   command's round trip.
"""
import json
import os
import shutil
import statistics
import subprocess
import sys
import tempfile
import time

eng, src, doc = sys.argv[1:4]
reps = int(sys.argv[4]) if len(sys.argv) > 4 else 5
E = f'/tmp/p4l5/{eng}'
env = dict(os.environ, SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1',
           FLASHTEX_POOL=f'{E}/pdftex.pool', FLASHTEX_FORMATS=f'/tmp/p4l5/fmt-{eng}',
           FLASHTEX_PIN_CLOCK='1700000000.250000', TZ='UTC')
w = tempfile.mkdtemp(prefix=f'reopen-{doc}.', dir='/tmp/p4l5')
for n in os.listdir(src):
    if os.path.isfile(os.path.join(src, n)):
        shutil.copy(os.path.join(src, n), w)
cmdline = ['-fmt=pdflatex', '-interaction=batchmode', f'{doc}.tex']


class Host:
    def __init__(self):
        self.t0 = time.time()
        self.p = subprocess.Popen([f'{E}/flashtex-host', 'iserve', '--'] + cmdline, cwd=w, env=env,
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, bufsize=1)

    def cmd(self, c):
        t = time.time()
        self.p.stdin.write(c + '\n')
        self.p.stdin.flush()
        d = json.loads(self.p.stdout.readline())
        d['_rt'] = time.time() - t
        if 'error' in d:
            raise SystemExit(f'{c}: {d["error"]}')
        return d

    def quit(self):
        self.p.stdin.write('quit\n')
        self.p.stdin.flush()
        self.p.wait()


h = Host()
for _ in range(6):
    r = h.cmd('compile')
    if r['mode'] == 'unchanged':
        break
s = h.cmd(f'save {w}/s0.bin')
h.quit()
fresh, fresh_host, warm, warm_host, warm_s = [], [], [], [], []
for _ in range(reps):
    h = Host()
    r = h.cmd(f'open {w}/s0.bin 1')
    fresh.append(time.time() - h.t0)
    fresh_host.append(r['page_s'])
    h.cmd('finish')
    h.quit()
    h = Host()
    ws = h.cmd(f'warm {w}/.warm')
    warm_s.append(ws['warm_s'])
    r = h.cmd(f'open {w}/s0.bin 1')
    warm.append(r['_rt'])
    warm_host.append(r['page_s'])
    print('warm open: read_s0 %.1f ms config %.1f ms page %.1f ms rt %.1f ms' % (1000*r['restore_s'], 1000*r['find_s'], 1000*r['page_s'], 1000*r['_rt']), file=sys.stderr)
    h.cmd('finish')
    h.quit()
med = lambda x: round(statistics.median(x) * 1000, 1)
print(json.dumps(dict(doc=doc, s0_bytes=s['bytes'], s0_on_disk=s['on_disk'],
                      fresh_process_to_page1_ms=med(fresh), fresh_open_to_page1_ms=med(fresh_host),
                      warm_open_roundtrip_ms=med(warm), warm_open_to_page1_ms=med(warm_host),
                      warm_up_ms=med(warm_s), reps=reps,
                      fresh_all=[round(x * 1000, 1) for x in fresh], warm_all=[round(x * 1000, 1) for x in warm])))
shutil.rmtree(w)
