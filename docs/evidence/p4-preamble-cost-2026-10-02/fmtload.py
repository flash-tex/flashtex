#!/usr/bin/env python3
"""CPU of loading the pdflatex format: `-fmt=pdflatex \\end` against `-ini \\end` (CLI, median of N)."""
import os, resource, subprocess, sys
E = sys.argv[1] if len(sys.argv) > 1 else 'pr1300'
B = '/tmp/ib-p4pc'
env = dict(os.environ, FLASHTEX_POOL=f'{B}/{E}/pdftex.pool', FLASHTEX_FORMATS=f'{B}/fmt-{E}')
os.makedirs(f'{B}/split', exist_ok=True)


def cpu(args):
    r0 = resource.getrusage(resource.RUSAGE_CHILDREN)
    subprocess.run([f'{B}/{E}/flashtex-initex'] + args, cwd=f'{B}/split', env=env,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, stdin=subprocess.DEVNULL)
    r1 = resource.getrusage(resource.RUSAGE_CHILDREN)
    return 1e3 * ((r1.ru_utime - r0.ru_utime) + (r1.ru_stime - r0.ru_stime))


for name, args in (('ini', ['-ini', '-jobname=x1', '\\end']), ('fmt', ['-fmt=pdflatex', '-jobname=x2', '\\end'])):
    v = sorted(cpu(args) for _ in range(9))
    print(name, 'cpu ms p50', round(v[4], 1), 'min', round(v[0], 1))
