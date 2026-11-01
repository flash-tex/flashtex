#!/usr/bin/env python3
"""Expected diagnostic positions for the diag-v1 corpus, from pdflatex.

    python3 tools/diag-oracle/oracle.py [--cases DIR] [--out FILE] [--pdflatex BIN]

Lane P5-DIAGNOSTICS (DESIGN.md §12 P5; docs/protocol/display-list-v3.md
§6.7). For every `.tex` file in the corpus
(crates/flashtex-engine/tests/diagnostics/cases/), this runs TeX Live's
pdflatex (the oracle; never in the product path) as the host runs the
engine (`-interaction=nonstopmode -file-line-error`) in a scratch directory
and reads the positions off pdflatex's own log. Nothing here is written by
hand: the expected file is regenerated from the oracle only (DESIGN.md §8).

For each report in the log, in order:

* **errors** (`file:line: message`, `! message`) and `\\show`s (`> ...`):
  the line of the bottom context line `l.<n> <read>` / `<to read>` and the
  **column** of its split: the byte offset in source line <n> such that the
  line's text before it ends with what the first context line shows (after
  a leading `...`) and the text after it starts with what the second shows
  (before a trailing `...`). A split the display does not determine
  uniquely is recorded as `null` with the candidates.
* **LaTeX, package and class warnings**: the line of "on input line <n>";
  pdflatex shows no context for a warning, so a second run (the *probe*)
  makes `\\GenericWarning` an `\\errmessage` (`\\input` of the case from a
  wrapper, so line numbers are the case's) and each warning's column is
  that error's split, as above: the same input stack as the `\\immediate\\write`
  in the real `\\GenericWarning`.
* **box reports**: the line range TeX prints ("in paragraph at lines a--b",
  "detected at line n").
* **pdfTeX warnings**: their presence.

The result: one JSON object, {"pdflatex": version, "cases": {name: [item]}}.
MIT-licensed; it only runs external programs.
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
CASES = os.path.join(ROOT, 'crates/flashtex-engine/tests/diagnostics/cases')
OUT = os.path.join(ROOT, 'crates/flashtex-engine/tests/diagnostics/expected.json')

ERR_FL = re.compile(r'^(\S*\.\w+):(\d+): (.*)$')
CONTEXT_BOTTOM = re.compile(r'^l\.(\d+) (.*)$')
BOX = re.compile(r'^(Overfull|Underfull|Tight|Loose) \\([hv])box \(.*?\) '
                 r'(?:(?:in paragraph|in alignment) at lines (\d+)--(\d+)|detected at line (\d+)'
                 r'|has occurred while \\output is active)')
WARN = re.compile(r'^(LaTeX Warning|LaTeX Font Warning|Package \S+ Warning|Class \S+ Warning|'
                  r'LaTeX \S+ Warning): ')
PROBE = 'FLASHTEX-PROBE'


def state(d):
    """The files a run wrote that a next run reads (all but the log and PDF)."""
    out = {}
    for f in sorted(os.listdir(d)):
        p = os.path.join(d, f)
        if os.path.isfile(p) and not f.endswith(('.log', '.pdf')):
            with open(p, 'rb') as h:
                out[f] = h.read()
    return out


def run(pdflatex, d, args):
    """Run pdflatex as the host compiles: again while a run changed a file
    the next reads, up to five runs, stopping when the files repeat a
    state (the host's passes, `crate::incr`); the last run's log counts."""
    env = dict(os.environ, SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1')
    seen = [state(d)]
    for _ in range(5):
        subprocess.run([pdflatex, '-interaction=nonstopmode', '-file-line-error'] + args,
                       cwd=d, env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL, timeout=120)
        now = state(d)
        if now in seen:
            break
        seen.append(now)


def printed(b):
    """What TeX prints for a line of the buffer (cp227.tcx: bytes 128-255
    printable; control characters as ^^ notation)."""
    out = []
    for c in b:
        if c < 32:
            out.append('^^' + chr(c + 64))
        elif c == 127:
            out.append('^^?')
        else:
            out.append(chr(c))
    return ''.join(out)


def split_col(src_line, first, second):
    """The byte column of the split, or (None, candidates)."""
    s = src_line.rstrip(b' ')
    p = printed(s)
    # printed index -> byte index (a ^^X is three printed characters)
    byte_at = []
    for i, c in enumerate(s):
        w = 3 if (c < 32 or c == 127) else 1
        byte_at += [i] * w
    byte_at.append(len(s))
    # A line TeX read while \endlinechar was changed shows its end-of-line
    # character (^^M), which the file does not have.
    if second.endswith('^^M') and not p.endswith('^^M'):
        second = second[:-3]
    elif first.endswith('^^M') and not second and not p.endswith('^^M'):
        first = first[:-3]
    trunc_first = first.startswith('...')
    f = first[3:] if trunc_first else first
    trunc_second = second.endswith('...')
    h = second[:-3] if trunc_second else second
    cands = []
    for c in range(len(p) + 1):
        if trunc_first:
            if not p[:c].endswith(f):
                continue
        elif p[:c] != f:
            continue
        rest = p[c:]
        if trunc_second:
            ok = rest.startswith(h)
        else:
            ok = rest == h
        if ok:
            cands.append(byte_at[c])
    cands = sorted(set(cands))
    if len(cands) == 1:
        return cands[0], cands
    return None, cands


def read_source(d, name):
    p = os.path.join(d, name)
    if not os.path.exists(p):
        return None
    with open(p, 'rb') as f:
        return f.read().split(b'\n')


def contexts(lines, i, stop):
    """The bottom context line after log line i, before line `stop`: (n,
    first half, second half) or None."""
    for k in range(i + 1, min(stop, len(lines))):
        m = CONTEXT_BOTTOM.match(lines[k])
        if m:
            nxt = lines[k + 1] if k + 1 < len(lines) else ''
            n = len(lines[k])
            return int(m.group(1)), m.group(2), nxt[n:] if nxt[:n].strip() == '' else nxt.lstrip(' ')
        if lines[k].startswith('<*>'):
            return None
    return None


def is_start(l):
    return bool(ERR_FL.match(l)) or l.startswith('! ') or l.startswith('> ')


MAX_PRINT_LINE = 79


def parse_log(log_text, d, main):
    lines = log_text.split('\n')
    starts = [i for i, l in enumerate(lines) if is_start(l)]
    items = []
    i = 0
    while i < len(lines):
        l = lines[i]
        m = ERR_FL.match(l)
        if m or l.startswith('! ') or l.startswith('> '):
            # TeX broke the message line after max_print_line characters
            # (only a message line is joined: a context line of that length
            # is followed by the next line, which print_nl starts).
            while len(l) == MAX_PRINT_LINE and i + 1 < len(lines):
                l = l + lines[i + 1]
                del lines[i + 1]
                starts = [s - 1 if s > i else s for s in starts]
            m = ERR_FL.match(l)
            nxt = [s for s in starts if s > i]
            stop = nxt[0] if nxt else len(lines)
            if (m and m.group(3).startswith(' ==> Fatal error occurred')) or \
                    l.startswith('!  ==> Fatal error occurred'):
                # the run's summary, not an error report
                i += 1
                continue
            if m:
                file, msg = m.group(1), m.group(3)
                kind = 'error'
            elif l.startswith('! '):
                file, msg = None, l[2:]
                kind = 'error'
            else:
                file, msg = None, l
                kind = 'show'
            ctx = contexts(lines, i, stop)
            item = {'kind': kind, 'message': msg}
            if ctx:
                n, first, second = ctx
                src_name = file or main
                src = read_source(d, src_name)
                item['file'] = src_name
                item['line'] = n
                if src is not None and 0 < n <= len(src):
                    col, cands = split_col(src[n - 1], first, second)
                    item['col'] = col
                    if col is None:
                        item['candidates'] = cands
                    item['context'] = [first, second]
                else:
                    item['col'] = None
            else:
                item['file'] = file
                item['line'] = int(m.group(2)) if m else None
                item['col'] = None
                item['no_context'] = True
            items.append(item)
            i += 1
            continue
        mb = BOX.match(l)
        if mb:
            a = mb.group(3) or mb.group(5)
            b = mb.group(4) or mb.group(5)
            items.append({'kind': 'box', 'message': l.strip(),
                          'lines': [int(a), int(b)] if a else None})
            i += 1
            continue
        mw = WARN.match(l)
        if mw:
            msg = l
            k = i + 1
            while k < len(lines) and lines[k].strip() != '':
                msg += '\n' + lines[k]
                k += 1
            flat = re.sub(r'\n\(\S+\)\s+', ' ', msg).replace('\n', '')
            mm = re.search(r'on input line (\d+)\.', flat)
            items.append({'kind': 'warning', 'message': l,
                          'line': int(mm.group(1)) if mm else None})
            i = k
            continue
        if l.startswith('pdfTeX warning'):
            items.append({'kind': 'pdfwarning', 'message': l})
        i += 1
    return items


def oracle_case(pdflatex, path):
    name = os.path.splitext(os.path.basename(path))[0]
    with tempfile.TemporaryDirectory(prefix='diag-oracle-') as d:
        shutil.copy(path, os.path.join(d, name + '.tex'))
        run(pdflatex, d, [name + '.tex'])
        with open(os.path.join(d, name + '.log'), 'rb') as f:
            log = f.read().decode('latin-1')
        items = parse_log(log, d, './' + name + '.tex')
        # The probe: \GenericWarning as \errmessage, for the warnings'
        # columns.
        warns = [it for it in items if it['kind'] == 'warning']
        if warns:
            pd = os.path.join(d, 'probe')
            os.makedirs(pd)
            shutil.copy(path, os.path.join(pd, name + '.tex'))
            with open(os.path.join(pd, 'flashtex-probe.tex'), 'w') as f:
                # `\GenericWarning` is robust: its body is `\GenericWarning\space`
                # (which amstext's redefinition calls too).
                f.write('\\expandafter\\def\\csname GenericWarning \\endcsname#1#2'
                        '{\\errmessage{%s}}\\input{%s.tex}\n' % (PROBE, name))
            run(pdflatex, pd, ['-jobname=' + name, 'flashtex-probe.tex'])
            with open(os.path.join(pd, name + '.log'), 'rb') as f:
                plog = f.read().decode('latin-1')
            probes = [it for it in parse_log(plog, pd, './' + name + '.tex')
                      if it['kind'] == 'error' and it['message'].startswith(PROBE)]
            if len(probes) == len(warns):
                for w, p in zip(warns, probes):
                    w['col'] = p.get('col')
                    w['probe_line'] = p.get('line')
                    if w.get('line') is None:
                        # no "on input line": where TeX read, from the probe
                        w['line'] = p.get('line')
                    if p.get('file'):
                        w['file'] = p['file']
            else:
                for w in warns:
                    w['col'] = None
                    w['probe_mismatch'] = [len(warns), len(probes)]
    return name, items


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--cases', default=CASES)
    ap.add_argument('--out', default=OUT)
    ap.add_argument('--pdflatex', default=shutil.which('pdflatex') or 'pdflatex')
    a = ap.parse_args()
    ver = subprocess.run([a.pdflatex, '--version'], capture_output=True, text=True).stdout.split('\n')[0]
    cases = {}
    for f in sorted(os.listdir(a.cases)):
        if f.endswith('.tex'):
            name, items = oracle_case(a.pdflatex, os.path.join(a.cases, f))
            cases[name] = items
            print('%-44s %d' % (name, len(items)), file=sys.stderr)
    with open(a.out, 'w') as f:
        json.dump({'pdflatex': ver, 'generator': 'tools/diag-oracle/oracle.py',
                   'cases': cases}, f, indent=1, sort_keys=True)
        f.write('\n')


if __name__ == '__main__':
    main()
