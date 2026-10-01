#!/usr/bin/env python3
"""DESIGN §1.2 "preamble edit <= 400 ms to first visible page", engine side
(review 2026-09-30, track 1).

usage: preamble_edit.py ENGINE DOC [REPS]
ENGINE is /tmp/p4l5/ENGINE/{flashtex-host,pdftex.pool} with /tmp/p4l5/fmt-ENGINE
(P4-L5's layout); DOC.tex is copied from /tmp/p4l5/src-DOC.

A `flashtex-host iserve` session settles the document, then REPS times: add a
harmless line to the preamble (`\\newcommand\\zzreview{N}` after
`\\documentclass`), `compile 1` (stop once page 1 is shipped: viewport first),
record the wall time of the command and the host's report, then `finish`,
revert, `compile` (settle again). Prints one JSON line per edit.
"""
import json, os, shutil, subprocess, sys, tempfile, time

E, doc = sys.argv[1], sys.argv[2]
reps = int(sys.argv[3]) if len(sys.argv) > 3 else 5
B = f'/tmp/p4l5/{E}'
env = dict(os.environ, SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1', TZ='UTC',
           FLASHTEX_POOL=f'{B}/pdftex.pool', FLASHTEX_FORMATS=f'/tmp/p4l5/fmt-{E}',
           FLASHTEX_PIN_CLOCK='1700000000.250000')
work = tempfile.mkdtemp(prefix=f'pre-{E}-{doc}.', dir='/tmp/t1rev')
shutil.copy(f'/tmp/p4l5/src-{doc}/{doc}.tex', work)
host = subprocess.Popen([f'{B}/flashtex-host', 'iserve', '--', '-fmt=pdflatex', '-interaction=batchmode', f'{doc}.tex'],
                        cwd=work, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=open(os.path.join("/tmp/t1rev", f"pre-{E}-{doc}.stderr"), "w"),
                        text=True, bufsize=1)


def cmd(c):
    t = time.perf_counter()
    host.stdin.write(c + '\n')
    host.stdin.flush()
    line = host.stdout.readline()
    d = json.loads(line)
    d['_wall'] = time.perf_counter() - t
    if 'error' in d:
        raise SystemExit(f'{c}: {d["error"]}')
    return d


def settle():
    for _ in range(6):
        if cmd('compile')['mode'] == 'unchanged':
            return


p = os.path.join(work, f'{doc}.tex')
orig = open(p).read()
settle()
first = orig.index('\n') + 1
for i in range(reps):
    open(p, 'w').write(orig[:first] + f'\\newcommand\\zzreview{{{i}}}\n' + orig[first:])
    r = cmd('compile 1')
    print(json.dumps(dict(engine=E, doc=doc, rep=i, wall_ms=round(r['_wall'] * 1000, 1), mode=r.get('mode'),
                          edited_page_ms=round(1000 * (r.get('edited_page_s') or 0), 1),
                          total_ms=round(1000 * (r.get('total_s') or 0), 1), passes=r.get('passes'),
                          load=os.getloadavg()[0])), flush=True)
    cmd('finish')
    open(p, 'w').write(orig)
    settle()
host.stdin.write('quit\n')
host.stdin.flush()
host.wait(timeout=60)
shutil.rmtree(work, ignore_errors=True)
