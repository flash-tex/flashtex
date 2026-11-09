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
the oracle's, the memory-usage lines one by one and field by field: B and D are dyn_used (what
LAZY-FLUSH recomputes), A and C var_used and E hi_mem_min-lo_mem_max-1 (which SyncTeX's extra node
words in TeX Live's pdfTeX move; etrip normalises the lines for that reason). The PR's engine is
also compared with its base (`main`, the running count): every line and the whole log. Exit 1 if
the PR's engine prints anything else than its base, or another dyn_used than pdfTeX where the base
prints pdfTeX's. OUT_DIR gets report.md, report.json and every log (gzipped). With --logs DIR the
logs of an earlier run are compared again, without compiling (run 41349282689's first report
took the memory-usage lines from split_accounting, which keeps them in the strict log when the
log has no `Completed box` lines, and so compared none)."""
import argparse, difflib, gzip, json, os, re, shutil, sys

HERE = os.path.dirname(os.path.abspath(__file__))
W = os.path.abspath(os.path.join(HERE, '..', '..', '..', '..'))
sys.path.insert(0, os.path.join(W, 'tools', 'parity'))
import capture  # noqa: E402

ap = argparse.ArgumentParser()
ap.add_argument('out')
ap.add_argument('--oracle', default=None)
ap.add_argument('--engine', action='append', default=[], help='NAME=BIN_DIR, or NAME alone with --logs')
ap.add_argument('--passes', type=int, default=4)
ap.add_argument('--logs', default=None, metavar='DIR',
                help="compare the logs an earlier run left in DIR (DOC-ENGINE.log.gz) instead of compiling")
ap.add_argument('docs', nargs='+', help='DIR:ENTRY, or the document names with --logs')
a = ap.parse_args()
os.makedirs(a.out, exist_ok=True)

ENGINES = {'pdftex': (os.path.abspath(a.oracle) if a.oracle else None, {})}
for spec in a.engine:
    name, _, d = spec.partition('=')
    d = os.path.abspath(d) if d else None
    ENGINES[name] = (d and os.path.join(d, 'flashtex-initex'),
                     d and {'FLASHTEX_POOL': os.path.join(d, 'pdftex.pool'), 'FLASHTEX_FORMATS': os.path.join(d, 'fmt')})
MEM = re.compile(r'^Memory usage before: (\d+)&(\d+); after: (\d+)&(\d+); still untouched: (\d+)$')
FIRST = r'\pdfsetrandomseed 1\relax\tracingstats=2 \input{%s}'


def log(*m):
    print(*m, file=sys.stderr, flush=True)


def compile_log(eng, src, entry, name):
    """The last pass's raw log, and its work directory (removed), and the exit code."""
    binary, env = ENGINES[eng]
    stem = entry[:-4] if entry.endswith('.tex') else entry
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
    shutil.rmtree(wd, ignore_errors=True)
    return raw, wd, rc


report = []
for spec in a.docs:
    if a.logs:
        name, src, entry = spec, None, None
    else:
        src, entry = spec.rsplit(':', 1)
        name = os.path.basename(src.rstrip('/'))
    parts = {}
    for eng in ENGINES:
        if a.logs:
            raw = gzip.open(os.path.join(a.logs, f'{name}-{eng}.log.gz'), 'rt', encoding='latin-1').read()
            wd, rc = os.path.join(a.logs, 'work', eng, name), None
        else:
            raw, wd, rc = compile_log(eng, src, entry, name)
        norm = capture.normalise_log(raw, wd)
        # (without \tracingoutput no line says where a shipout ended, so split_accounting leaves
        # the memory-usage lines in the strict log: they are taken from the whole log, and the
        # rest of the strict log is compared without them)
        strict, acc = capture.split_accounting(norm)
        mem = [ln for ln in norm.split('\n') if MEM.match(ln)]
        strict = '\n'.join(ln for ln in strict.split('\n') if not MEM.match(ln))
        parts[eng] = dict(rc=rc, norm=norm, strict=strict, acc=acc, mem=mem,
                          shipouts=sum(1 for ln in norm.split('\n') if ln.startswith(capture.SHIPOUT)))
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
        fields = lambda m: [tuple(int(x) for x in MEM.match(ln).groups()) for ln in m]  # noqa: E731
        fo, fp = fields(o['mem']), fields(p['mem'])
        r = dict(memory_lines_equal=o['mem'] == p['mem'], first_memory_difference=first_mem,
                 # B and D, dyn_used before and after the shipout's flush, and A, C and E (var_used,
                 # hi_mem_min-lo_mem_max-1), which SyncTeX's node words move in TeX Live's pdfTeX
                 dyn_used_equal=[(f[1], f[3]) for f in fo] == [(f[1], f[3]) for f in fp],
                 var_fields_equal=[(f[0], f[2], f[4]) for f in fo] == [(f[0], f[2], f[4]) for f in fp],
                 memory_lines_equal_to_main=(p['mem'] == parts['main']['mem']) if 'main' in parts else None,
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
    yn = lambda b: 'yes' if b else ('NO' if b is not None else '-')  # noqa: E731
    f.write('| document | engine | memory-usage lines (pdfTeX/engine) | all equal to pdfTeX\'s | '
            'dyn_used (B, D) equal to pdfTeX\'s | var fields (A, C, E) equal | lines equal to main\'s | '
            'memory block equal to pdfTeX\'s | rest of the log equal to pdfTeX\'s (P-T1 strict) | '
            'whole log equal to main\'s |\n'
            '|---|---|---|---|---|---|---|---|---|---|\n')
    for row in report:
        for eng, r in row['engines'].items():
            f.write(f"| {row['doc']} | {eng} | {row['memory_lines']['pdftex']}/{row['memory_lines'][eng]} | "
                    f"{'yes' if r['memory_lines_equal'] else 'NO, first at ' + str(r['first_memory_difference'])} | "
                    f"{yn(r['dyn_used_equal'])} | {yn(r['var_fields_equal'])} | {yn(r['memory_lines_equal_to_main'])} | "
                    f"{yn(r['accounting_block_equal'])} | "
                    f"{'yes' if r['strict_log_equal'] else 'NO (' + str(r['strict_diff_lines']) + ' lines)'} | "
                    f"{yn(r['log_equal_to_main'])} |\n")
print(open(os.path.join(a.out, 'report.md')).read())
# red if the PR's engine prints anything else than its base, or another dyn_used than pdfTeX where
# its base prints pdfTeX's (a difference the base has too is the base's, reported above)
bad = any(r['memory_lines_equal_to_main'] is False or r['log_equal_to_main'] is False
          or (not r['dyn_used_equal'] and row['engines'].get('main', {}).get('dyn_used_equal', False))
          for row in report for e, r in row['engines'].items() if e != 'main')
sys.exit(1 if bad else 0)
