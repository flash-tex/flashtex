#!/usr/bin/env python3
"""retirement_refs.py: no tracked file names code a retirement stage deleted, outside a named allowlist.

usage: retirement_refs.py [--root DIR] [--config FILE] [--stage S2,...] [--list]

The old-engine retirement plan (docs/design/engine-v2/retirement-plan.md §4.6) deletes code in stages.
After a stage lands, nothing that stays may still name what it deleted: a path to a deleted crate is
a broken script, a stale doc, or a dependency that would bring it back. This check greps every tracked
text file (`git ls-files`) for each landed stage's names (crate directories, package, library and
binary names, deleted scripts) and fails on a hit unless it is:

- under one of `always_allowed` (append-only evidence, coordination records, the plan and reviews), or
- in the stage's `allow` list: a path, the stage that deletes or rewrites that file, and why.

An `allow` entry whose file no longer names anything of its stage is stale and fails too, so the list
only shrinks. Config: tools/parity/retirement-refs-allow.json. Exit 0 clean, 1 hits or stale entries,
2 a bad config. `--list` prints every hit, allowed or not.
"""
import argparse
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_CONFIG = os.path.join(HERE, 'retirement-refs-allow.json')


def tracked_files(root):
    out = subprocess.run(['git', 'ls-files', '-z'], cwd=root, capture_output=True, check=True).stdout
    return [p for p in out.decode('utf-8', 'surrogateescape').split('\0') if p]


def read_text(path):
    """The file's text, or None for a binary (a NUL in the first 8 KiB) or unreadable file."""
    try:
        with open(path, 'rb') as f:
            data = f.read()
    except OSError:
        return None
    if b'\0' in data[:8192]:
        return None
    return data.decode('utf-8', 'replace')


def scan(files, names, read):
    """{path: sorted names found} for every file naming one of `names`."""
    hits = {}
    for p in files:
        text = read(p)
        if text is None:
            continue
        found = sorted({n for n in names if n in text})
        if found:
            hits[p] = found
    return hits


def under(path, roots):
    return any(path == r.rstrip('/') or path.startswith(r if r.endswith('/') else r + '/') for r in roots)


def check(config, files, read, stages=None):
    """(problems, report): problems are the lines that fail the check."""
    always = config.get('always_allowed', [])
    problems, report = [], []
    for sid, st in config['stages'].items():
        if stages and sid not in stages:
            continue
        if not st.get('landed', False):
            continue
        names = st['names']
        allow = {a['path']: a for a in st.get('allow', [])}
        for a in allow.values():
            for k in ('path', 'until', 'why'):
                if not a.get(k):
                    raise ValueError(f'{sid}: allow entry {a} lacks {k!r}')
        hits = scan([p for p in files if not under(p, always)], names, read)
        for p, found in sorted(hits.items()):
            if p in allow:
                report.append(f'{sid}: allowed until {allow[p]["until"]}: {p}: {", ".join(found)}')
            else:
                problems.append(f'{sid}: {p} names {", ".join(found)} (deleted in {sid}); fix it, '
                                f'or add it to {sid}\'s allow list with the stage that removes it')
        for p in sorted(set(allow) - set(hits)):
            problems.append(f'{sid}: stale allow entry {p}: it no longer names anything {sid} deleted; remove it')
    return problems, report


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--root', default=os.path.dirname(os.path.dirname(HERE)))
    ap.add_argument('--config', default=DEFAULT_CONFIG)
    ap.add_argument('--stage', help='comma list of stage ids (default: every landed stage)')
    ap.add_argument('--list', action='store_true', help='also print the allowed hits')
    a = ap.parse_args(argv)
    try:
        config = json.load(open(a.config))
    except (OSError, ValueError) as e:
        print(f'retirement_refs: {a.config}: {e}', file=sys.stderr)
        return 2
    os.chdir(a.root)
    stages = set(a.stage.split(',')) if a.stage else None
    try:
        problems, report = check(config, tracked_files('.'), read_text, stages)
    except (KeyError, ValueError) as e:
        print(f'retirement_refs: bad config: {e}', file=sys.stderr)
        return 2
    if a.list:
        for r in report:
            print(r)
    for p in problems:
        print(p)
    landed = [s for s, st in config['stages'].items() if st.get('landed') and (not stages or s in stages)]
    print(f'retirement_refs: {"FAIL" if problems else "ok"}: stages {", ".join(landed) or "none"}; '
          f'{len(problems)} problems, {len(report)} allowed hits')
    return 1 if problems else 0


if __name__ == '__main__':
    sys.exit(main())
