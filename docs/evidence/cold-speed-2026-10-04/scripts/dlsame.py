#!/usr/bin/env python3
"""Display-list and diagnostics identity of two engines (COLD-SPEED; from P6-HYPEROPT's dlsame.py, plus
`--diag`): for each document directory (main file given), three compiles through each engine's host
(`dl3-client --save --diag`), then `dl3-dump --canonical` of both and their diag-v1 messages, compared.

usage: dlsame.py ENGINE_A ENGINE_B DIR:MAIN ..."""
import hashlib, os, re, shutil, subprocess, sys

sys.path.insert(0, os.path.join(os.path.abspath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', '..')), 'tools', 'incr-bench'))
import json  # noqa: E402
import t7  # noqa: E402


def norm_path(h):
    """A `file` line's path (hex) with the host's own output directory (`flashtex-host-<pid>-<conn>`, the
    default when the client names none) made the same for every host."""
    try:
        p = bytes.fromhex(h.decode())
    except ValueError:
        return h
    return re.sub(rb'/flashtex-host-\d+-', b'/flashtex-host-PID-', p)


def unnumber_spans(d):
    """Span ids and file ids are the host's handles for (file, line) pairs and for paths, numbered as the
    run first meets each (a run that notes fewer allocations meets fewer); name each span by its path and
    line, so that two hosts that number them differently compare by what they say. `file` lines (dropped:
    which ids exist) and the span references of `span` items and `link` records are rewritten."""
    lines = d.split(b'\n')
    files, name = {}, {}
    for l in lines:
        f = l.split(b' ')
        if len(f) == 3 and f[0] == b'file':
            files[f[1]] = norm_path(f[2])
    for l in lines:
        f = l.split(b' ')
        if len(f) == 4 and f[0] == b'span':
            name[f[1]] = b'@' + files.get(f[2], b'?' + f[2]) + b':' + f[3]
    out = []
    for l in lines:
        f = l.split(b' ')
        if f[0] == b'file' and len(f) == 3:
            continue
        if f[0] == b'span' and len(f) == 2:
            f[1] = name.get(f[1], f[1])
        elif f[0] == b'span' and len(f) == 4:
            f = [b'span', name.get(f[1], f[1])]
        elif f[0] == b'link' and len(f) == 9:
            f[5] = name.get(f[5], f[5]) if f[5] != b'0' else f[5]
        out.append(b' '.join(f))
    return b'\n'.join(out)


def span_names(d):
    """The name `unnumber_spans` gives each span id the dump defines."""
    files, r = {}, {}
    for l in d.split(b'\n'):
        f = l.split(b' ')
        if len(f) == 3 and f[0] == b'file':
            files[f[1]] = norm_path(f[2])
        if len(f) == 4 and f[0] == b'span':
            r[int(f[1])] = '@%s:%s' % (files.get(f[2], b'?' + f[2]).decode(), f[3].decode())
    return r


def unnumber_diag(v, names):
    """A diag-v1 message (JSON) with its `span` fields named as `unnumber_spans` names them."""
    if isinstance(v, dict):
        return {k: (names.get(x, x) if k == 'span' and isinstance(x, int) else unnumber_diag(x, names))
                for k, x in v.items()}
    if isinstance(v, list):
        return [unnumber_diag(x, names) for x in v]
    return v



IB = t7.IB
a, b = sys.argv[1], sys.argv[2]
bad = 0
saved_only = os.environ.get('DLSAME_SAVED') == '1'
for spec in sys.argv[3:]:
    src, main = spec.rsplit(':', 1)
    name = os.path.basename(src.rstrip('/'))
    dumps = {}
    diags = {}
    for eng in (a, b):
        # one work directory for both engines: image entries and spans name their files
        work = f'{IB}/dlsame/work/{name}'
        shutil.rmtree(work, ignore_errors=True)
        shutil.copytree(src, work, symlinks=True)
        save = f'{IB}/dlsame/{eng}-{name}.dl3'
        dfile = f'{IB}/dlsame/{eng}-{name}.diag'
        diags[eng] = [{k: v for k, v in json.loads(l).items() if k not in ('id',)}
                      for l in open(dfile)] if os.path.exists(dfile) else []
        if saved_only and os.path.exists(save):
            d = subprocess.run([f'{IB}/{eng}/dl3-dump', '--canonical', save], capture_output=True, timeout=600).stdout
            dumps[eng] = b'\n'.join(l for l in d.split(b'\n')
                                     if not l.startswith((b'json started', b'json done', b'json tool', b'json diag')))
            continue
        sock = f'/tmp/cs-dls-{eng}.sock'
        dfile = f'{IB}/dlsame/{eng}-{name}.diag'
        h = t7.Host(f'{IB}/{eng}', sock, f'{IB}/dlsame/s0-{eng}-{name}', f'{IB}/dlsame/{eng}-{name}.stderr', [])
        try:
            save = f'{IB}/dlsame/{eng}-{name}.dl3'
            p = subprocess.run([f'{IB}/{eng}/dl3-client', '--socket', sock, '--root', work, '--main', main,
                                '--save', save, '--diag', dfile, '--quiet', '--repeat', '3'], capture_output=True, text=True, timeout=3600)
            if p.returncode:
                print(name, eng, 'dl3-client failed', p.stderr[-300:])
        finally:
            h.stop()
        diags[eng] = [{k: v for k, v in json.loads(l).items() if k not in ('id',)}
                      for l in open(dfile)] if os.path.exists(dfile) else []
        d = subprocess.run([f'{IB}/{eng}/dl3-dump', '--canonical', save], capture_output=True, timeout=600).stdout
        # the run's own metadata (timings in STARTED/DONE, the work directory's paths) is not the display list
        dumps[eng] = b'\n'.join(l for l in d.split(b'\n')
                                 if not l.startswith((b'json started', b'json done', b'json tool', b'json diag')))
    if os.environ.get('DLSAME_SPAN_IDS') != '1':
        diags = {k: [unnumber_diag(m, span_names(dumps[k])) for m in v] for k, v in diags.items()}
        dumps = {k: unnumber_spans(v) for k, v in dumps.items()}
    diags = {k: [json.dumps(m, sort_keys=True) for m in v] for k, v in diags.items()}
    same = dumps[a] == dumps[b]
    dsame = diags[a] == diags[b]
    bad += not (same and dsame)
    print(f'{name}: {"same" if same else "DIFFERENT"} ({len(dumps[a])} bytes, sha {hashlib.sha256(dumps[a]).hexdigest()[:12]}); '
          f'diag {"same" if dsame else "DIFFERENT"} ({len(diags[a])} messages)', flush=True)
print('different:', bad)
sys.exit(1 if bad else 0)
