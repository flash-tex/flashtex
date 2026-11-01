#!/usr/bin/env python3
"""ckseg.py ENGINE DIR DOC REPS SETTING...: what segment checkpoints (P4-L5,
DESIGN.md 5.2: checkpoints between shipouts, after build_page) cost.

For each setting of FLASHTEX_SEGMENT_S ("off", or the least engine time in
seconds between two checkpoints, e.g. 0 or 0.002), one cold `flashtex-host
iserve` compile of DOC in a fresh copy of DIR (its .aux settled by a first
process), under /usr/bin/time -l, the settings interleaved REPS times. Prints
per setting the CPU time, instructions, cycles, peak RSS, the checkpoints the
run kept and the undo logs' bytes; the cost of a segment checkpoint is the
difference to "off" divided by the extra checkpoints."""
import json
import os
import re
import shutil
import statistics
import subprocess
import sys
import tempfile

E, d, doc, reps = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
settings = sys.argv[5:] or ['off', '0']
B = f'/tmp/p4l5/{E}'
env = dict(os.environ, SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1',
           FLASHTEX_POOL=f'{B}/pdftex.pool', FLASHTEX_FORMATS=f'/tmp/p4l5/fmt-{E}',
           FLASHTEX_PIN_CLOCK='1700000000.250000', TZ='UTC')
work = tempfile.mkdtemp(prefix='ckseg.', dir='/tmp/p4l5')
for n in os.listdir(d):
    if os.path.isfile(os.path.join(d, n)):
        shutil.copy(os.path.join(d, n), work)
args = [f'{B}/flashtex-host', 'iserve', '--', '-fmt=pdflatex', '-interaction=batchmode', f'{doc}.tex']
# settle the .aux once
subprocess.run(args, input='compile\ncompile\nquit\n', cwd=work, env=env, text=True,
               stdout=subprocess.PIPE, stderr=subprocess.PIPE)
snap = {n: open(os.path.join(work, n), 'rb').read() for n in os.listdir(work)}


def run(setting):
    for n, b in snap.items():
        open(os.path.join(work, n), 'wb').write(b)
    e = dict(env, FLASHTEX_SEGMENT_S=setting)
    p = subprocess.run(['/usr/bin/time', '-l'] + args, input='compile\nquit\n', cwd=work, env=e, text=True,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    err = p.stderr
    j = [json.loads(l) for l in p.stdout.splitlines() if l.startswith('{')][-1]
    return dict(cpu=float(re.search(r'([\d.]+) user', err).group(1)) + float(re.search(r'([\d.]+) sys', err).group(1)),
                ins=int(re.search(r'(\d+)\s+instructions retired', err).group(1)),
                cyc=int(re.search(r'(\d+)\s+cycles elapsed', err).group(1)),
                rss=int(re.search(r'(\d+)\s+maximum resident set size', err).group(1)),
                ck=j['checkpoints'], logs=j['log_bytes'], pages=j['pages'], mode=j['mode'])


rows = {s: [] for s in settings}
for r in range(reps):
    for s in settings:
        rows[s].append(run(s))
        print(s, json.dumps(rows[s][-1]), flush=True)
base = {k: statistics.median(x[k] for x in rows[settings[0]]) for k in ('cpu', 'ins', 'cyc', 'ck', 'rss', 'logs')}
for s in settings:
    m = {k: statistics.median(x[k] for x in rows[s]) for k in ('cpu', 'ins', 'cyc', 'ck', 'rss', 'logs')}
    extra = max(1, m['ck'] - base['ck'])
    pages = rows[s][0]['pages']
    print(json.dumps(dict(setting=s, pages=pages, checkpoints=m['ck'], cpu_s=round(m['cpu'], 3),
                          cpu_per_page_ms=round(1000 * (m['cpu'] - base['cpu']) / pages, 3),
                          ins_per_extra_ck_k=round((m['ins'] - base['ins']) / extra / 1000, 1),
                          cyc_per_extra_ck_k=round((m['cyc'] - base['cyc']) / extra / 1000, 1),
                          ins_per_page_k=round((m['ins'] - base['ins']) / pages / 1000, 1),
                          rss_mb=round(m['rss'] / 2**20), log_mb=round(m['logs'] / 2**20))), flush=True)
shutil.rmtree(work)
