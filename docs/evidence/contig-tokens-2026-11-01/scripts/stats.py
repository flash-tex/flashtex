#!/usr/bin/env python3
"""LAZY-FLUSH's oracle check of the printed memory accounting (PR #2016; lane CONTIG-TOKENS).

usage: stats.py OUT_DIR --oracle PDFTEX --engine NAME=BIN_DIR ... DOC...

Every document (DIR:ENTRY) is compiled by TeX Live's pdfTeX (PDFTEX, its own pdflatex.fmt) and by
each engine (BIN_DIR holds flashtex-initex, pdftex.pool and fmt/pdflatex.fmt) with
`\\tracingstats=2` on the first line, so every shipout prints `Memory usage before: A&B; after:
C&D; still untouched: E` (pdftex.web section 639) and the run ends with the memory block (section
1334). Each runs exactly as tools/parity's P-T1 runs it: argv[0] `pdftex` through a link, the
pinned random seed, SOURCE_DATE_EPOCH, FLASHTEX_* only for the engine (tools/parity/capture.py
`run_engine`); four passes, so the .aux files have settled. The last pass's log is normalised as
P-T1 does (`normalise_log`: the banner and the work directory) and split as P-T1 does
(`split_accounting`) into the strict log and the accounting lines, and each part is compared with
the oracle's, the memory-usage lines one by one. OUT_DIR gets report.md, report.json and every log
(gzipped)."""
import argparse, difflib, gzip, json, os, re, shutil, sys

HERE = os.path.dirname(os.path.abspath(__file__))
W = os.path.abspath(os.path.join(HERE, '..', '..', '..', '..'))
sys.path.insert(0, os.path.join(W, 'tools', 'parity'))
import capture  # noqa: E402

ap = argparse.ArgumentParser()
ap.add_argument('out')
ap.add_argument('--oracle', required=True)
ap.add_argument('--engine', action='append', required=True)
ap.add_argument('--passes', type=int, default=4)
ap.add_argument('docs', nargs='+')
a = ap.parse_args()
os.makedirs(a.out, exist_ok=True)

ENGINES = {'pdftex': (os.path.abspath(a.oracle), {})}
for spec in a.engine:
    name, d = spec.split('=', 1)
    d = os.path.abspath(d)
    ENGINES[name] = (os.path.join(d, 'flashtex-initex'),
                     {'FLASHTEX_POOL': os.path.join(d, 'pdftex.pool'), 'FLASHTEX_FORMATS': os.path.join(d, 'fmt')})
MEM = re.compile(r'^Memory usage before: (\d+)&(\d+); after: (\d+)&(\d+); still untouched: (\d+)$')
FIRST = r'\pdfsetrandomseed 1\relax\tracingstats=2 \input{%s}'


def log(*m):
    print(*m, file=sys.stderr, flush=True)


report = []
for spec in a.docs:
    src, entry = spec.rsplit(':', 1)
    name = os.path.basename(src.rstrip('/'))
    stem = entry[:-4] if entry.endswith('.tex') else entry
    parts = {}
    for eng, (binary, env) in ENGINES.items():
        wd = os.path.join(a.out, 'work', eng, name)
        shutil.rmtree(wd, ignore_errors=True)
        shutil.copytree(src, wd, symlinks=True)
        for i in range(a.passes):
            rc, timed_out = capture.run_engine(binary, 'pdflatex', ['-interaction=batchmode', FIRST % entry], wd,
                                               extra_env=env)
            log(f'{name} {eng} pass {i + 1}: rc={rc} timed_out={timed_out}')
        raw = open(os.path.join(wd, stem + '.log'), 'rb').read().decode('latin-1')
        with gzip.open(os.path.join(a.out, f'{name}-{eng}.log.gz'), 'wt', encoding='latin-1') as f:
            f.write(raw)
        norm = capture.normalise_log(raw, wd)
        strict, acc = capture.split_accounting(norm)
        mem = [ln for ln in acc if MEM.match(ln)]
        parts[eng] = dict(rc=rc, norm=norm, strict=strict, acc=acc, mem=mem,
                          shipouts=sum(1 for ln in norm.split('\n') if ln.startswith(capture.SHIPOUT)))
        shutil.rmtree(wd, ignore_errors=True)
    o = parts['pdftex']
    row = dict(doc=name, shipouts={e: p['shipouts'] for e, p in parts.items()},
               memory_lines={e: len(p['mem']) for e, p in parts.items()}, rc={e: p['rc'] for e, p in parts.items()},
               engines={})
    for eng, p in parts.items():
        if eng == 'pdftex':
            continue
        first_mem = next((i for i, (x, y) in enumerate(zip(o['mem'], p['mem'])) if x != y), None)
        if first_mem is None and len(o['mem']) != len(p['mem']):
            first_mem = min(len(o['mem']), len(p['mem']))
        other_o = [ln for ln in o['acc'] if not MEM.match(ln)]
        other_p = [ln for ln in p['acc'] if not MEM.match(ln)]
        d = list(difflib.unified_diff(o['strict'].split('\n'), p['strict'].split('\n'), 'pdftex', eng, lineterm='', n=1))
        if d:
            open(os.path.join(a.out, f'{name}-pdftex-vs-{eng}.strict.diff'), 'w').write('\n'.join(d) + '\n')
        r = dict(memory_lines_equal=o['mem'] == p['mem'], first_memory_difference=first_mem,
                 accounting_block_equal=other_o == other_p, strict_log_equal=not d,
                 strict_diff_lines=sum(1 for ln in d if ln[:1] in '+-' and not ln.startswith(('+++', '---'))),
                 log_equal_to_main=(p['norm'] == parts['main']['norm']) if 'main' in parts else None)
        if first_mem is not None:
            r['memory_line_pdftex'] = o['mem'][first_mem] if first_mem < len(o['mem']) else None
            r['memory_line_engine'] = p['mem'][first_mem] if first_mem < len(p['mem']) else None
        if other_o != other_p:
            r['accounting_pdftex'] = other_o
            r['accounting_engine'] = other_p
        row['engines'][eng] = r
    report.append(row)
    log(json.dumps(row))

json.dump(report, open(os.path.join(a.out, 'report.json'), 'w'), indent=1)
with open(os.path.join(a.out, 'report.md'), 'w') as f:
    f.write('| document | engine | shipouts (pdfTeX/engine) | memory-usage lines | equal to pdfTeX\'s | '
            'memory block equal | strict log equal | log equal to main\'s |\n|---|---|---|---|---|---|---|---|\n')
    for row in report:
        for eng, r in row['engines'].items():
            f.write(f"| {row['doc']} | {eng} | {row['shipouts']['pdftex']}/{row['shipouts'][eng]} | "
                    f"{row['memory_lines'][eng]} | {'yes' if r['memory_lines_equal'] else 'NO, first at ' + str(r['first_memory_difference'])} | "
                    f"{'yes' if r['accounting_block_equal'] else 'NO'} | "
                    f"{'yes' if r['strict_log_equal'] else 'NO (' + str(r['strict_diff_lines']) + ' lines)'} | "
                    f"{r['log_equal_to_main']} |\n")
print(open(os.path.join(a.out, 'report.md')).read())
bad = any(not (r['memory_lines_equal'] and r['accounting_block_equal'])
          for row in report for r in row['engines'].values())
sys.exit(1 if bad else 0)
