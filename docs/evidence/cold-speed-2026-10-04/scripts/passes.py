#!/usr/bin/env python3
"""Run a document pass by pass from a clean copy, as pdflatex users do, until the log asks for no rerun.

usage: passes.py OUT.jsonl NAME SRC_DIR WORK_DIR EXE [ENV=VAL...]
Per pass: /usr/bin/time -l counters (instructions, cycles, user, sys, wall, rss), load, the log's rerun
lines, and which auxiliary files changed after the pass (sha256 before/after)."""
import hashlib, json, os, re, shutil, subprocess, sys, time

out, name, src, work, exe = sys.argv[1:6]
extra = dict(a.split('=', 1) for a in sys.argv[6:])
env = {k: v for k, v in os.environ.items() if not k.startswith('FLASHTEX_')}
env.update(SOURCE_DATE_EPOCH='0', FORCE_SOURCE_DATE='1', max_print_line='10000', error_line='254',
           half_error_line='238')
env.update(extra)
if os.path.exists(work):
    shutil.rmtree(work)
shutil.copytree(src, work, ignore=shutil.ignore_patterns('*.aux', '*.toc', '*.idx', '*.ind', '*.ilg', '*.out',
                                                         '*.log', 'infdesc.pdf', '*.lof', '*.lot'))
AUX = re.compile(r'\.(aux|toc|idx|ind|ilg|out|lof|lot|nav|snm|vrb)$')


def snap():
    r = {}
    for dp, _, fs in os.walk(work):
        for f in fs:
            if AUX.search(f):
                p = os.path.join(dp, f)
                r[os.path.relpath(p, work)] = hashlib.sha256(open(p, 'rb').read()).hexdigest()[:16]
    return r


cmd = [exe, '-fmt=pdflatex', '-interaction=batchmode', '-jobname=infdesc',
       r'\pdfsetrandomseed 1\relax\input{infdesc.tex}']
o = open(out, 'a')
prev = snap()
for p in range(1, 6):
    l0 = os.getloadavg()[0]
    t0 = time.perf_counter()
    r = subprocess.run(['/usr/bin/time', '-l'] + cmd, cwd=work, env=env, stdout=subprocess.DEVNULL,
                       stderr=subprocess.PIPE, stdin=subprocess.DEVNULL)
    wall = time.perf_counter() - t0
    e = r.stderr.decode(errors='replace')
    g = lambda pat: float(re.search(pat, e).group(1)) if re.search(pat, e) else None
    log = open(os.path.join(work, 'infdesc.log'), 'rb').read().decode('latin-1')
    rerun = sorted(set(m.strip() for m in re.findall(r'^.*(?:Rerun|may have changed|undefined references).*$', log, re.M)))
    pages = re.search(r'Output written on .*\((\d+) pages', log)
    now = snap()
    changed = sorted(k for k in set(now) | set(prev) if now.get(k) != prev.get(k))
    pdf = hashlib.sha256(open(os.path.join(work, 'infdesc.pdf'), 'rb').read()).hexdigest()[:16]
    rec = dict(name=name, pass_=p, rc=r.returncode, wall=round(wall, 2), user=g(r'([\d.]+) user'),
               sys=g(r'([\d.]+) sys'), instr=g(r'(\d+)\s+instructions retired'),
               cycles=g(r'(\d+)\s+cycles elapsed'), rss=g(r'(\d+)\s+maximum resident'), load0=round(l0, 1),
               load1=round(os.getloadavg()[0], 1), pages=int(pages.group(1)) if pages else None,
               rerun=rerun, changed=changed, pdf=pdf)
    print(json.dumps(rec), flush=True)
    o.write(json.dumps(rec) + '\n')
    o.flush()
    prev = now
    if not rerun or p >= 3 and not changed:
        break
