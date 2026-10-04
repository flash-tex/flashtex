#!/usr/bin/env python3
"""Display-list identity of two engines: for each document directory (main file given), one cold compile
through each engine's host (`dl3-client --save`), then `dl3-dump --canonical` of both, compared.

usage: dlsame.py ENGINE_A ENGINE_B DIR:MAIN ..."""
import hashlib, os, shutil, subprocess, sys

sys.path.insert(0, os.path.join(os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', '..')), 'tools', 'incr-bench'))
import t7  # noqa: E402

IB = t7.IB
a, b = sys.argv[1], sys.argv[2]
bad = 0
saved_only = os.environ.get('DLSAME_SAVED') == '1'
for spec in sys.argv[3:]:
    src, main = spec.rsplit(':', 1)
    name = os.path.basename(src.rstrip('/'))
    dumps = {}
    for eng in (a, b):
        # one work directory for both engines: image entries and spans name their files
        work = f'{IB}/dlsame/work/{name}'
        shutil.rmtree(work, ignore_errors=True)
        shutil.copytree(src, work, symlinks=True)
        save = f'{IB}/dlsame/{eng}-{name}.dl3'
        if saved_only and os.path.exists(save):
            d = subprocess.run([f'{IB}/{eng}/dl3-dump', '--canonical', save], capture_output=True, timeout=600).stdout
            dumps[eng] = b'\n'.join(l for l in d.split(b'\n')
                                     if not l.startswith((b'json started', b'json done', b'file ')))
            continue
        sock = f'/tmp/p6h-dls-{eng}.sock'
        h = t7.Host(f'{IB}/{eng}', sock, f'{IB}/dlsame/s0-{eng}-{name}', f'{IB}/dlsame/{eng}-{name}.stderr', [])
        try:
            save = f'{IB}/dlsame/{eng}-{name}.dl3'
            p = subprocess.run([f'{IB}/{eng}/dl3-client', '--socket', sock, '--root', work, '--main', main,
                                '--save', save, '--quiet', '--repeat', '3'], capture_output=True, text=True, timeout=3600)
            if p.returncode:
                print(name, eng, 'dl3-client failed', p.stderr[-300:])
        finally:
            h.stop()
        d = subprocess.run([f'{IB}/{eng}/dl3-dump', '--canonical', save], capture_output=True, timeout=600).stdout
        # the run's own metadata (timings in STARTED/DONE, the work directory's paths) is not the display list
        dumps[eng] = b'\n'.join(l for l in d.split(b'\n')
                                 if not l.startswith((b'json started', b'json done', b'file ')))
    same = dumps[a] == dumps[b]
    bad += not same
    print(f'{name}: {"same" if same else "DIFFERENT"} ({len(dumps[a])} bytes, sha {hashlib.sha256(dumps[a]).hexdigest()[:12]})', flush=True)
print('different:', bad)
sys.exit(1 if bad else 0)
