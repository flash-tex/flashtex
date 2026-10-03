#!/usr/bin/env python3
"""P4-L1 measurements. usage: bench.py BIN FMT DOC EDITFILE|- REPS REOPEN_REPS [--every-shipout]
Runs flashtex-host bench (cold, then REPS compiles from S0 after editing EDITFILE), saves S0,
then REOPEN_REPS fresh-process opens. Prints one JSON summary line."""
import json, os, shutil, statistics, subprocess, sys, tempfile

bin_, fmt, doc, edit, reps, reopen_reps = sys.argv[1:7]
reps, reopen_reps = int(reps), int(reopen_reps)
every = '--every-shipout' in sys.argv
env = dict(os.environ, FLASHTEX_POOL=f'{bin_}/pdftex.pool', FLASHTEX_FORMATS=fmt,
           SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1')
w = tempfile.mkdtemp(prefix=f'bench-{doc}.', dir='/tmp/p4l1')
for n in os.listdir('/tmp/p4l1/docs'):
    shutil.copy(os.path.join('/tmp/p4l1/docs', n), w)
pd = ['-fmt=pdflatex', '-interaction=batchmode', f'{doc}.tex']


def host(args):
    p = subprocess.run([f'{bin_}/flashtex-host'] + args + ['--'] + pd, cwd=w, env=env,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return [json.loads(l) for l in p.stdout.decode().splitlines() if l.startswith('{')]


# converge the .aux first (a cold host run takes S0 only on a stable .aux)
for _ in range(2):
    subprocess.run([f'{bin_}/pdftex'] + pd, cwd=w, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
args = ['bench', '--reps', str(reps), '--save', f'{w}/s0.bin']
if edit != '-':
    args += ['--edit', edit]
if every:
    args += ['--every-shipout']
out = host(args)
cold = out[0]
s0runs = [o for o in out[1:] if o.get('mode') == 's0']
saved = [o for o in out if 'saved' in o][0]
opens, reopen_compiles, oc = [], [], []
for _ in range(reopen_reps):
    o = host(['open', f'{w}/s0.bin'])
    opens.append(o[0])
    reopen_compiles.append(o[1])
    oc.append(o[2]['open_and_compile_s'])


def med(xs):
    return round(statistics.median(xs), 6) if xs else None


res = {
    'doc': doc,
    'every_shipout': every,
    'cold_total_s': cold['total_s'],
    'cold_pages': cold['pages'],
    'cold_first_page_s': cold['first_page_s'],
    's0_at_s': cold['s0_at_s'],
    'checkpoint_s': cold['checkpoint_s'],
    'modes': [o.get('mode') for o in out if 'mode' in o],
    's0_validate_s': med([o['validate_s'] for o in s0runs]),
    's0_restore_s': med([o['restore_s'] for o in s0runs]),
    's0_restore_min_s': min([o['restore_s'] for o in s0runs]) if s0runs else None,
    's0_body_run_s': med([o['run_s'] for o in s0runs]),
    's0_total_s': med([o['total_s'] for o in s0runs]),
    's0_first_page_s': med([o['first_page_s'] for o in s0runs]),
    'undo_log_bytes': s0runs[-1]['log_bytes'] if s0runs else None,
    's0_file_bytes': saved['bytes'],
    's0_on_disk_bytes': saved['on_disk'],
    's0_save_s': saved['seconds'],
    'reopen_config_s': med([o['config_s'] for o in opens]),
    'reopen_open_s': med([o['total_s'] for o in opens]),
    'reopen_open_s_min': min([o['total_s'] for o in opens]) if opens else None,
    'reopen_load_s': med([o['load_s'] for o in opens]),
    'reopen_validate_s': med([o['validate_s'] for o in opens]),
    'reopen_chunks': opens[0]['chunks'] if opens else None,
    'reopen_first_page_s': med([o['first_page_s'] for o in reopen_compiles]),
    'reopen_to_first_page_s': med([a['total_s'] + b['first_page_s'] for a, b in zip(opens, reopen_compiles)]),
    'reopen_modes': [o['mode'] for o in reopen_compiles],
    'reopen_and_compile_s': med(oc),
}
print(json.dumps(res))
shutil.rmtree(w)
