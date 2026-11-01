#!/usr/bin/env python3
"""Show that the both-paths verifier catches a broken replay.

usage: faults.py ENGINE TEXFILE
For each fault FLASHTEX_INTRINSICS_FAULT can inject into replays (see
crates/flashtex-engine/src/intrinsics.rs, `Fault`), compile TEXFILE with the engine in
/tmp/l6/ENGINE and FLASHTEX_INTRINSICS=verify, and report how many verified calls the
verifier flagged (it prints each difference to stderr as it finds it, so a run that a
fault later breaks -- a freed token list reused -- still reports). The control run
(no fault) must show none."""
import os
import shutil
import subprocess
import sys
import tempfile

E, F = sys.argv[1], sys.argv[2]
FAULTS = ['', 'drop-last', 'drop-first-let', 'no-ref', 'local']
env0 = dict(os.environ, SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1', TZ='UTC',
            FLASHTEX_POOL=f'/tmp/l6/{E}/pdftex.pool', FLASHTEX_FORMATS=f'/tmp/l6/fmt-{E}',
            FLASHTEX_INTRINSICS='verify')
for fault in FAULTS:
    d = tempfile.mkdtemp(dir='/tmp/l6')
    shutil.copy(F, d)
    name = os.path.basename(F)
    env = dict(env0, FLASHTEX_INTRINSICS_FAULT=fault)
    stats = os.path.join(d, 'stats.txt')
    env['FLASHTEX_INTRINSICS_STATS'] = stats
    cmd = [f'/tmp/l6/{E}/flashtex-initex', '-fmt=pdflatex', '-interaction=batchmode', name]
    status = 'finished'
    try:
        r = subprocess.run(cmd, cwd=d, env=env, capture_output=True, timeout=120)
        err = r.stderr.decode('latin1')
    except subprocess.TimeoutExpired as t:
        status = 'killed after 120 s (the fault broke the run afterwards)'
        err = (t.stderr or b'').decode('latin1')
    flagged = [l for l in err.splitlines() if l.startswith('intrinsics verify:')]
    verified = '?'
    if os.path.exists(stats):
        for l in open(stats):
            if l.strip().startswith('verified:'):
                verified = l.split(':')[1].strip(' ,\n')
    print(f"fault={fault or 'none'}: calls flagged {len(flagged)}, verified {verified}, run {status}")
    if flagged:
        print('   first:', flagged[0][:300])
    shutil.rmtree(d, ignore_errors=True)
