#!/usr/bin/env python3
"""Diagnostic precision against pdflatex: the engine's diag-v1 and the old
engine's (v1) diagnostics on the corpus, position by position.

    python3 tools/diag-oracle/compare.py --host FLASHTEX_HOST --formats DIR \\
        [--pool FILE] [--v1 FLASHTEX_CLI] [--json OUT] [--cases NAME...]

Lane P5-DIAGNOSTICS. The expected positions are `expected.json`, made from
pdflatex's own log by `oracle.py` (never by hand). For each case this runs

* the engine host, `flashtex-host iserve` (`compile`, then `diagnostics`:
  the `DIAG` messages a `diag-v1` client gets), in a scratch copy of the
  case, as the socket host runs it (nonstopmode, file:line:error, display
  list on);
* optionally the old engine, `flashtex check --json` (runtime-v1's
  diagnostics: line, column, byte span);

and matches reports to pdflatex's in order, per kind (error, `\\show`,
warning, box, pdfTeX warning). Per report it records whether each side has
it at all, at the same line, and at the same column (TeX's context split;
for a warning, the split of the probe run's context; for a box report, the
same line range). pdflatex columns are bytes; v1's are 1-based characters,
converted to bytes on the source line before comparing.

Exit status 0 when every pdflatex position is matched by the engine.
MIT-licensed; it only runs external programs.
"""
import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
CASES = os.path.join(ROOT, 'crates/flashtex-engine/tests/diagnostics/cases')
EXPECTED = os.path.join(ROOT, 'crates/flashtex-engine/tests/diagnostics/expected.json')


def kind_of(d):
    code = d.get('code', '')
    if code == 'tex/show':
        return 'show'
    if code.startswith('tex/') and (code.endswith('-hbox') or code.endswith('-vbox')):
        return 'box'
    if d.get('origin') == 'pdftex' and d.get('severity') == 'warning':
        return 'pdfwarning'
    if d.get('severity') == 'error':
        return 'error'
    if d.get('severity') == 'warning':
        return 'warning'
    return 'other'


def ours(host, fmt, pool, case_path, name):
    with tempfile.TemporaryDirectory(prefix='diag-cmp-') as d:
        shutil.copy(case_path, os.path.join(d, name + '.tex'))
        env = dict(os.environ, FLASHTEX_FORMATS=fmt, SOURCE_DATE_EPOCH='1700000000',
                   FORCE_SOURCE_DATE='1', FLASHTEX_PIN_CLOCK='1700000000.123456',
                   FLASHTEX_DISPLAY_LIST='/dev/null')
        if pool:
            env['FLASHTEX_POOL'] = pool
        p = subprocess.run([host, 'iserve', '--', '-fmt=pdflatex', '-interaction=nonstopmode',
                            '-file-line-error', name + '.tex'],
                           input='compile\ndiagnostics\nquit\n', capture_output=True, text=True,
                           cwd=d, env=env, timeout=300)
        for line in p.stdout.splitlines():
            if line.startswith('{"diagnostics"'):
                return json.loads(line)['diagnostics']
        raise RuntimeError('%s: no diagnostics from the host: %s %s' % (name, p.stdout[-400:], p.stderr[-400:]))


def v1(cli, case_path, name):
    with tempfile.TemporaryDirectory(prefix='diag-v1-') as d:
        shutil.copy(case_path, os.path.join(d, name + '.tex'))
        p = subprocess.run([cli, 'check', '--json', name + '.tex'], capture_output=True,
                           text=True, cwd=d, timeout=300)
        try:
            j = json.loads(p.stdout)
        except ValueError:
            return None
        return j.get('diagnostics', [])


def char_col_to_byte(src_line, col1):
    """v1's 1-based character column -> 0-based byte column."""
    if col1 is None:
        return None
    s = src_line.decode('utf-8', 'replace')
    return len(s[:max(col1 - 1, 0)].encode('utf-8'))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--host', required=True)
    ap.add_argument('--formats', required=True)
    ap.add_argument('--pool')
    ap.add_argument('--v1')
    ap.add_argument('--json')
    ap.add_argument('--cases', nargs='*')
    a = ap.parse_args()
    exp = json.load(open(EXPECTED))
    rows = []
    extras = []
    totals = {}

    def tally(key, n=1):
        totals[key] = totals.get(key, 0) + n

    for name, items in sorted(exp['cases'].items()):
        if a.cases and name not in a.cases:
            continue
        path = os.path.join(CASES, name + '.tex')
        src = open(path, 'rb').read().split(b'\n')
        got = [d for d in ours(a.host, a.formats, a.pool, path, name)]
        by_kind = {}
        for d in got:
            by_kind.setdefault(kind_of(d), []).append(d)
        v1d = v1(a.v1, path, name) if a.v1 else None
        seen = {}
        for it in items:
            k = it['kind']
            idx = seen.get(k, 0)
            seen[k] = idx + 1
            mine = by_kind.get(k, [])
            o = mine[idx] if idx < len(mine) else None
            row = {'case': name, 'kind': k, 'message': it['message'][:80],
                   'expected': {x: it.get(x) for x in ('file', 'line', 'col', 'lines')}}
            tally('reports')
            tally('reports.' + k)
            # ours
            if o is None:
                row['ours'] = None
                tally('ours.missing')
            else:
                row['ours'] = {x: o.get(x) for x in ('file', 'line', 'col', 'range', 'lines', 'code', 'exact')}
                row['ours']['trace'] = len(o.get('trace', []))
                row['ours']['help'] = len(o.get('help', []))
                tally('ours.found')
                if o.get('exact'):
                    tally('ours.exact')
                if o.get('trace'):
                    tally('ours.with_trace')
                if o.get('help'):
                    tally('ours.with_help')
            if k == 'box':
                if it.get('lines'):
                    tally('box.with_lines')
                    if o and o.get('lines') == it['lines']:
                        tally('ours.lines_match')
                    if o and o.get('col') is not None:
                        tally('ours.box_col')
            elif it.get('line') is not None:
                tally('with_line')
                same_file = (o is not None and (it.get('file') is None or
                             os.path.basename(o.get('file') or '') == os.path.basename(it['file'])))
                if o and not same_file:
                    row['file_mismatch'] = True
                if o and same_file and o.get('line') == it['line']:
                    tally('ours.line_match')
                    if it.get('col') is not None and o.get('range') and \
                            o['range'][0] <= it['col'] <= o['range'][1]:
                        tally('ours.range_covers_split')
                if it.get('col') is not None:
                    tally('with_col')
                    if o and same_file and o.get('line') == it['line'] and o.get('col') == it['col']:
                        tally('ours.col_match')
                    elif o:
                        row['col_mismatch'] = True
            else:
                tally('no_position_in_pdflatex')
                if o and o.get('line') is not None:
                    tally('ours.position_where_pdflatex_has_none')
            # v1
            if v1d is not None:
                sev = 'error' if k == 'error' else ('info' if k == 'show' else 'warning')
                exp_line = it.get('line') if k != 'box' else (it.get('lines') or [None])[0]
                cands = [d for d in v1d if (d.get('severity') == sev or (sev == 'warning' and d.get('severity') in ('warning', 'info')))]
                at_line = [d for d in cands if exp_line is not None and d.get('line') == exp_line]
                row['v1'] = [{x: d.get(x) for x in ('line', 'column', 'start_byte', 'end_byte', 'code', 'message')} for d in at_line[:1]]
                if cands:
                    tally('v1.any_same_severity')
                if at_line:
                    tally('v1.line_match' if k != 'box' else 'v1.box_line_match')
                    if it.get('col') is not None and exp_line and 0 < exp_line <= len(src):
                        if any(char_col_to_byte(src[exp_line - 1], d.get('column')) == it['col'] for d in at_line):
                            tally('v1.col_match')
                        # v1's byte span on the line covers TeX's split
                        line_start = sum(len(x) + 1 for x in src[:exp_line - 1])
                        if any(d.get('start_byte') is not None and d.get('end_byte') is not None and
                               d['start_byte'] - line_start <= it['col'] <= d['end_byte'] - line_start
                               for d in at_line):
                            tally('v1.span_covers_split')
            rows.append(row)
        # engine reports pdflatex's log does not list (e.g. its terminal-only lines)
        for k, v in by_kind.items():
            for d in v[seen.get(k, 0):]:
                tally('ours.extra')
                extras.append({'case': name, 'kind': k, 'message': d.get('message'),
                               'code': d.get('code'), 'exact': d.get('exact'),
                               'file': d.get('file'), 'line': d.get('line'), 'col': d.get('col')})
        if v1d is not None:
            tally('v1.total_reported', len(v1d))
    print(json.dumps(totals, indent=1, sort_keys=True))
    for e in extras:
        print('EXTRA', json.dumps(e), file=sys.stderr)
    bad = [r for r in rows if r.get('col_mismatch') or r.get('file_mismatch') or r['ours'] is None or
           (r['kind'] != 'box' and r['expected']['line'] is not None and r['ours'] and r['ours']['line'] != r['expected']['line']) or
           (r['kind'] == 'box' and r['expected']['lines'] and r['ours'] and r['ours']['lines'] != r['expected']['lines'])]
    for r in bad:
        print('MISMATCH', json.dumps(r), file=sys.stderr)
    if a.json:
        with open(a.json, 'w') as f:
            json.dump({'totals': totals, 'rows': rows, 'extras': extras}, f, indent=1)
    sys.exit(1 if bad else 0)


if __name__ == '__main__':
    main()
