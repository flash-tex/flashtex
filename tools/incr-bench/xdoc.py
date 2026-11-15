#!/usr/bin/env python3
"""xdoc.py ENGINE [--docs D,..] [--rounds R] [--edits N]: one resident host serves several
documents in turn, and every compile it answers is compared byte for byte with from-scratch runs
(sweeps.py's sound-xdoc gate; lane REVIEW-BOXMEMO, docs/design/engine-v2/BOX-MEMO.md).

One engine thread, one BOX-MEMO store: the socket host (`$INCR_BENCH_DIR/ENGINE/flashtex-host
--socket`, with an S0 cache) keeps the entries every document recorded while it serves the next,
and a replay may take an entry another document recorded (the host's `stages.boxmemo` counts them:
xdoc_hits). A key part that held for one document and not for another (a font by its number, the
job name, a control sequence's name) gives a wrong page in the second document only; the sweeps
that open one document per host never see it.

For each round, each document in turn: an open (round 1: no .aux; later rounds: the files the last
visit left, as the app reopens a document), then N edits, each compiled and then reverted and
compiled again, sent as the app sends them (`incremental` COMPILEs with `edits`). An edit types a
letter onto the end of a prose word, or a few words after it (the paragraph reflows); its place is
drawn per document, round and edit from the document's prose lines (a beamer deck's frame bodies
too: the place need not be on one page, as dl3-keys' `--at` needs). Each document is compiled in
its own directory with the output directory the root, so the host runs `-fmt=pdflatex
-interaction=nonstopmode -file-line-error -jobname=main main.tex` there (host/server.rs
`Job::argv`).

The reference for a compile: the document's directory as it was before the compile, with the
edit, copied (modification times kept) and run with the CLI engine (`pdftex`, the same argv,
FLASHTEX_BOXMEMO=off: the oracle) until a run changes no file other than the PDF and the log
(at most 5 runs, as incr_bench.py). Every file the two directories hold is compared (but
.synctex, .synctex.gz and .fls); a log that differs only in DESIGN.md §1.1 P-T1 ruling N2's
accounting is reported, not counted.

Documents: a name under $INCR_BENCH_DIR/docs (mkdocs.py: plain-10, full-100, beamer-5, ...), or
rw:NAME, fixtures/real-world/NAME of --fixtures (the tree under test), or xdoc:NAME, one SYNTH writes. BOX-MEMO runs in the mode
FLASHTEX_BOXMEMO says (the host's default when unset); in verify mode with
FLASHTEX_BOXMEMO_VERIFY_FAIL=1 a difference ends the host, which is an error here.

Writes --out (one JSON line per compile) and OUT.host-stderr; prints a totals line
{"compiles", "ok", "bad", "err", "hits", "xdoc_hits", ...} (soundness.py's keys, which sweeps.py
reads). Exit 0 for an exercised passing gate or a labelled BOX-MEMO-off run, 1 for a failed
gate, 2 when the host failed or ended.

  python3 tools/incr-bench/xdoc.py gates --docs plain-10,beamer-5,rw:beamer-madrid --rounds 2
"""
import argparse
import json
import os
import random
import re
import shutil
import signal
import socket
import struct
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
BASE = os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')
HELLO, COMPILE, BYE = 0x01, 0x02, 0x04
H_DONE, H_ERROR = 0x49, 0x4A
MAX_PASSES = 5
STATE_SKIP = ('.pdf', '.log', '.synctex', '.synctex.gz', '.fls')
CMP_SKIP = ('.synctex', '.synctex.gz', '.fls')
WORDS = 'lorem ipsum dolor sit amet consectetur adipiscing elit sed eiusmod tempor'.split()
# lines that hold more than prose (soak.py's list): an edit there could change a label, a file
# name, verbatim text or an overlay specification
SKIP_LINE = re.compile(r'\\(label|ref|eqref|cref|Cref|cite|index|begin|end|input|include|verb|lstinline|'
                       r'url|href|item|section|subsection|chapter|part|caption|def|newcommand|renewcommand|'
                       r'usepackage|includegraphics|draw|node|path|foreach|hypersetup|bookmark|frametitle|'
                       r'only|uncover|pause|onslide|visible|alt|temporal)\b|[#&@~<>]|\\\\')


MEASURE = '\\begingroup\\setbox0\\hbox{%s}\\vrule width\\wd0 height 2pt\\endgroup\\hfil'


def synth(preamble, head, para, files=None, job='main'):
    """A document of xdoc:NAME: `preamble` after the class, a page head (`\\@oddhead`, a call
    BOX-MEMO offers by default) whose body is `head`, and 18 paragraphs of `para` three times;
    `files` (name: bytes) beside main.tex, compiled as job `job`."""
    body = '\n\n'.join(para * 3 for _ in range(18))
    text = (f'\\documentclass{{article}}\n{preamble}\\makeatletter\n\\def\\@oddhead{{{head}}}\n'
            '\\makeatother\n\\begin{document}\n' + body + '\n\\end{document}\n').encode()
    return dict(text=text, files=dict(files or {}), job=job)


def fonts_doc(first, second):
    """xdoc:fonts-*: two fonts loaded in the order given, so that one font number is another face
    in each of the pair, and a page head that measures text in the second and draws a rule that
    wide: a replay of the other document's entry keyed by the font's number alone draws the other
    face's width (#2091's first item)."""
    para = ('The measured head above each page is drawn from the width of text set in the font '
            'loaded second, which this document and its pair load in the other order. ')
    return synth(f'\\font\\fa={first}\\relax\n\\font\\fb={second}\\relax\n',
                 MEASURE % '\\fb Measured head', para)


PARA = ('The measured head above each page is drawn from what this document and its pair hold '
        'apart, and nothing else in the two differs. ')


def pair(preamble_a, preamble_b, head_a, head_b=None, files_a=None, files_b=None, job_a='main',
         job_b='main'):
    """xdoc:NAME-a and xdoc:NAME-b: two documents alike but for one key part each page head reads
    (the audit of 2026-11-08, BOX-MEMO.md section 17): a replay of the other document's entry
    with that part left out of the key draws the other document's head."""
    return (lambda: synth(preamble_a, head_a, PARA, files_a, job_a),
            lambda: synth(preamble_b, head_b or head_a, PARA, files_b, job_b))


# documents xdoc.py writes itself (xdoc:NAME), in pairs; each pair is alike but for one thing:
# - fonts: cmss12 and cmbx12 are not in the LaTeX format, so each document's \font gives them the
#   next numbers, in its own order (K5's font identity, #2091 item 1)
# - job: the job name (`Entry::job`, #2091 item 2)
# - cat: a category code (K2, `eqtb`'s local region)
# - name: two names of one hash location, `\qqXa` and `\qqaO` (they hash alike), which the head
#   prints (`Entry::names`, #2102)
# - input: a file `\input` in the preamble defines the head's text (K3: a macro by its tokens)
# - size: a file the head reads (`\pdffilesize`, refused while recording)
# - stack: a colour stack that one of the two makes (`Entry::stacks`; the other reports "Unknown
#   color stack number 1")
SYNTH = {'fonts-ab': lambda: fonts_doc('cmss12', 'cmbx12'), 'fonts-ba': lambda: fonts_doc('cmbx12', 'cmss12')}
PAIRS = {
    'job': pair('', '', MEASURE % '\\jobname', job_b='xdoc-a-much-longer-job-name'),
    'cat': pair('', '\\catcode`\\|=13\n',
                MEASURE % '\\ifnum\\catcode`\\|=13 The bar is active, a wide head\\else Other\\fi'),
    'name': pair('\\def\\qqXa{}\n', '\\def\\qqaO{}\n', MEASURE % '\\string\\qqXa',
                 MEASURE % '\\string\\qqaO'),
    'input': pair('\\input{hd}\n', '\\input{hd}\n', MEASURE % '\\hdtext',
                  files_a={'hd.tex': b'\\def\\hdtext{Short}\n'},
                  files_b={'hd.tex': b'\\def\\hdtext{A much longer head, from the file}\n'}),
    'size': pair('', '', MEASURE % '\\pdffilesize{data.txt}', files_a={'data.txt': b'x\n'},
                 files_b={'data.txt': b'0123456789\n' * 150}),
    'stack': pair('\\edef\\stk{\\pdfcolorstackinit page direct{0 g}}\n', '\\def\\stk{1}\n',
                  '\\pdfcolorstack1 push{1 0 0 rg}\\vrule width 20pt height 2pt\\pdfcolorstack1 pop\\hfil'),
}
for _k, (_a, _b) in PAIRS.items():
    SYNTH[f'{_k}-a'], SYNTH[f'{_k}-b'] = _a, _b


class Conn:
    def __init__(self, path):
        self.s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.s.connect(path)
        self.buf = b''

    def send(self, kind, obj):
        body = json.dumps(obj).encode()
        self.s.sendall(struct.pack('<IB', len(body) + 1, kind) + body)

    def _need(self, n):
        while len(self.buf) < n:
            chunk = self.s.recv(1 << 20)
            if not chunk:
                raise EOFError('the host closed the connection')
            self.buf += chunk

    def frame(self):
        self._need(5)
        n, kind = struct.unpack('<IB', self.buf[:5])
        self._need(4 + n)
        body = self.buf[5:4 + n]
        self.buf = self.buf[4 + n:]
        return kind, body

    def until_done(self, cid):
        while True:
            kind, body = self.frame()
            if kind == H_DONE:
                d = json.loads(body)
                if d.get('id') == cid and d.get('cause') is None:
                    return d
            elif kind == H_ERROR:
                raise RuntimeError(f'host ERROR: {body[:300]!r}')


def sites(text):
    """Byte offsets just after a lowercase prose word (4+ letters, followed by a space and a
    lowercase letter) of the body's prose lines, outside inline math."""
    out = []
    off = 0
    body = False
    for line in text.split(b'\n'):
        s = line.decode('latin-1')
        body = body or '\\begin{document}' in s
        st = s.strip()
        if (body and len(st) >= 30 and st[0] not in '\\%{}[]$' and '%' not in s
                and not SKIP_LINE.search(s) and s.count('$') % 2 == 0 and line.isascii()):
            dollars = 0
            for i, ch in enumerate(s):
                dollars += ch == '$'
                m = re.match(r'[a-z]{4,}(?= [a-z])', s[i:])
                if m and dollars % 2 == 0 and (i == 0 or not s[i - 1].isalpha() and s[i - 1] != '\\'):
                    out.append(off + i + m.end())
        off += len(line) + 1
    return out


def snapshot(d):
    return {n: open(os.path.join(d, n), 'rb').read() for n in sorted(os.listdir(d))
            if os.path.isfile(os.path.join(d, n))}


def state(d):
    return tuple((n, b) for n, b in snapshot(d).items() if not n.endswith(STATE_SKIP))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('engine')
    ap.add_argument('--docs', default='plain-10,full-100,beamer-5,beamer-30,rw:beamer-madrid')
    ap.add_argument('--rounds', type=int, default=2)
    ap.add_argument('--edits', type=int, default=2, help='edits per visit, each compiled and reverted')
    ap.add_argument('--fixtures', default=os.path.join(os.path.dirname(os.path.dirname(HERE)), 'fixtures'),
                    help='the fixtures directory rw:NAME is under (default: this checkout\'s)')
    ap.add_argument('--out', default='')
    ap.add_argument('--host-args', default='')
    ap.add_argument('--timeout', type=int, default=5400, help='seconds for the whole run')
    ap.add_argument('--keep', action='store_true')
    a = ap.parse_args()

    def timed_out(*_):
        raise TimeoutError(f'{a.timeout} s')

    signal.signal(signal.SIGALRM, timed_out)
    signal.alarm(a.timeout)
    # (tools/parity: this checkout's, or the tree's under test when sweeps.py runs its own copy)
    sys.path[:0] = [os.path.join(HERE, '../parity'), os.path.join(os.path.dirname(a.fixtures), 'tools/parity')]
    from capture import split_accounting  # noqa: E402

    E = f'{BASE}/{a.engine}'
    W = tempfile.mkdtemp(prefix='xdoc.', dir=BASE)
    out = a.out or f'{W}/xdoc.jsonl'
    rec = open(out, 'w')
    env = dict(os.environ, SOURCE_DATE_EPOCH='1700000000', FORCE_SOURCE_DATE='1',
               FLASHTEX_POOL=f'{E}/pdftex.pool', FLASHTEX_FORMATS=f'{BASE}/fmt-{a.engine}',
               FLASHTEX_PIN_CLOCK='1700000000.250000', TZ='UTC')
    # the oracle: the CLI engine with BOX-MEMO off, as the host runs it (FLASHTEX_PREVIEW)
    cli_env = dict(env, FLASHTEX_PREVIEW='1', FLASHTEX_BOXMEMO='off')
    cli_env.pop('FLASHTEX_BOXMEMO_VERIFY_FAIL', None)
    def argv(job):
        return ['-fmt=pdflatex', '-interaction=nonstopmode', '-file-line-error', f'-jobname={job}', 'main.tex']

    docs = []
    for name in [x for x in a.docs.split(',') if x]:
        root = os.path.join(W, re.sub(r'[^A-Za-z0-9_-]', '_', name))
        if name.startswith('xdoc:'):
            if name[5:] not in SYNTH:
                sys.exit(f'xdoc: no document {name} (have {", ".join(SYNTH)})')
            os.makedirs(root)
            sd = SYNTH[name[5:]]()
            for n, b in [('main.tex', sd['text'])] + sorted(sd['files'].items()):
                open(os.path.join(root, n), 'wb').write(b)
            job = sd['job']
        else:
            src = (os.path.join(a.fixtures, 'real-world', name[3:]) if name.startswith('rw:')
                   else os.path.join(BASE, 'docs', name))
            if not os.path.isfile(os.path.join(src, 'main.tex')):
                sys.exit(f'xdoc: no {src}/main.tex')
            shutil.copytree(src, root, ignore=shutil.ignore_patterns('reference.*', 't7.json', 'doc.json', '.git'))
            job = 'main'
        text = open(os.path.join(root, 'main.tex'), 'rb').read()
        cands = sites(text)
        if not cands:
            sys.exit(f'xdoc: {name}: no prose line to edit')
        docs.append(dict(name=name, root=root, text=text, sites=cands, job=job))

    sock = f'{W}/h.sock'
    sockdir = None
    if len(sock) > 100:  # sun_path holds 104 bytes on macOS
        sockdir = tempfile.mkdtemp(prefix='xdoc.', dir='/tmp')
        sock = f'{sockdir}/h.sock'
    herr = open(f'{out}.host-stderr', 'w')
    hout = open(f'{W}/h.out', 'w')
    host = subprocess.Popen([f'{E}/flashtex-host', '--socket', sock, '--s0-cache', f'{W}/s0']
                            + a.host_args.split(), env=env, stdout=hout, stderr=herr)
    for _ in range(1200):
        if 'listening' in open(f'{W}/h.out').read() or host.poll() is not None:
            break
        time.sleep(0.05)

    tot = dict(compiles=0, ok=0, bad=0, err=0, conv=0, interrupted=0, skipped=0, acct=0, hits=0,
               xdoc_hits=0, docs=len(docs), rounds=a.rounds)
    counts = [None]

    def reference(pre, d, tag):
        s = tempfile.mkdtemp(prefix=f'scr-{tag}.', dir=W)
        for n, b in pre.items():
            p = os.path.join(s, n)
            open(p, 'wb').write(b)
            src = os.path.join(d['root'], n)
            # the modification time the compile saw (`\pdffilemoddate`): a fresh copy's differs
            if os.path.isfile(src):
                st = os.stat(src)
                os.utime(p, ns=(st.st_atime_ns, st.st_mtime_ns))
        for sub in os.listdir(d['root']):  # a fixture's directories (figures), as they are
            if os.path.isdir(os.path.join(d['root'], sub)):
                shutil.copytree(os.path.join(d['root'], sub), os.path.join(s, sub))
        open(os.path.join(s, 'main.tex'), 'wb').write(d['text'])
        seen, runs = [], 0
        while True:
            before = state(s)
            seen.append(before)
            subprocess.run(['pdftex'] + argv(d['job']), executable=f'{E}/pdftex', cwd=s, env=cli_env,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=900)
            runs += 1
            if runs >= MAX_PASSES or state(s) in seen:
                break
        res = snapshot(s)
        shutil.rmtree(s)
        return res, runs

    def compile_(c, cid, d, edits, rnd, step, kind):
        pre = snapshot(d['root'])
        req = {'id': cid, 'root': d['root'], 'main': 'main.tex', 'output_dir': d['root'],
               'jobname': d['job'], 'incremental': True, 'edits': edits}
        t = time.time()
        c.send(COMPILE, req)
        r = c.until_done(cid)
        ms = (time.time() - t) * 1e3
        got = snapshot(d['root'])
        tag = f"{os.path.basename(d['root'])}.r{rnd}.{step}"
        bad = []
        if got.get('main.tex') != d['text']:
            bad.append('main.tex (the host\'s copy of the edit differs from the driver\'s)')
        ref, runs = reference(pre, d, tag)
        acct = False
        for n in sorted(set(got) | set(ref)):
            if n.endswith(CMP_SKIP) or got.get(n) == ref.get(n):
                continue
            if n.endswith('.log') and n in got and n in ref:
                # DESIGN §1.1 P-T1 ruling N2: end-of-run accounting is reported, not gated
                if split_accounting(got[n].decode('latin-1'))[0] == split_accounting(ref[n].decode('latin-1'))[0]:
                    acct = True
                    continue
            bad.append(f'{n} ({len(got.get(n) or b"")} vs {len(ref.get(n) or b"")} bytes)')
            os.makedirs(f'{BASE}/xdoc-mm', exist_ok=True)
            for k, v in (('got', got.get(n)), ('ref', ref.get(n))):
                if v is not None:
                    open(f'{BASE}/xdoc-mm/{tag}.{k}.{n}', 'wb').write(v)
        bm = (r.get('stages') or {}).get('boxmemo') or []
        prev = counts[0]
        counts[0] = bm
        delta = [x - (prev[i] if prev and i < len(prev) else 0) for i, x in enumerate(bm)]
        tot['compiles'] += 1
        tot['acct'] += acct
        tot['ok' if not bad else 'bad'] += 1
        tot['hits'] += delta[1] if len(delta) > 1 else 0
        if len(delta) > 4 and tot['xdoc_hits'] is not None:
            tot['xdoc_hits'] += delta[4]
        else:
            tot['xdoc_hits'] = None
        line = dict(doc=d['name'], round=rnd, step=step, kind=kind, mode=r.get('mode'), status=r.get('status'),
                    pages=r.get('pages'), ms=round(ms, 1), ref_runs=runs, boxmemo=delta, ok=not bad, bad=bad)
        rec.write(json.dumps(line) + '\n')
        rec.flush()
        print(f"{'ok ' if not bad else 'BAD'} {d['name']:24} r{rnd} {step:10} {str(r.get('mode')):10} "
              f"{r.get('pages')} pages {ms:7.0f} ms boxmemo {delta}" + (f"  mismatches: {', '.join(bad)}" if bad else ''),
              file=sys.stderr, flush=True)

    c = None
    try:
        c = Conn(sock)
        c.send(HELLO, {'protocol': 'display-list-v3', 'version': [3, 2], 'client': 'xdoc.py'})
        cid = 0
        for rnd in range(1, a.rounds + 1):
            for d in docs:
                cid += 1
                compile_(c, cid, d, [], rnd, 'open', 'open')
                rng = random.Random(f"{d['name']}:{rnd}")
                for j in range(a.edits):
                    pos = rng.choice(d['sites'])
                    kind = ('letter', 'words')[j % 2]
                    ins = rng.choice('etaoinshrdlu') if kind == 'letter' else \
                        ' ' + ' '.join(rng.choice(WORDS) for _ in range(rng.randint(3, 8)))
                    e = {'path': 'main.tex', 'offset': pos, 'insert': ins, 'delete': 0}
                    d['text'] = d['text'][:pos] + ins.encode() + d['text'][pos:]
                    cid += 1
                    compile_(c, cid, d, [e], rnd, f'edit{j}', kind)
                    d['text'] = d['text'][:pos] + d['text'][pos + len(ins):]
                    cid += 1
                    compile_(c, cid, d, [{'path': 'main.tex', 'offset': pos, 'insert': '', 'delete': len(ins)}],
                             rnd, f'revert{j}', kind)
    except (EOFError, OSError, RuntimeError, TimeoutError) as ex:
        tot['err'] += 1
        time.sleep(0.5)
        print(f'ERROR: {ex!r} (host exit {host.poll()}); host stderr {out}.host-stderr', flush=True)
    finally:
        try:
            if c is not None:
                c.send(BYE, {})
        except OSError:
            pass
        try:
            host.wait(timeout=20)
        except subprocess.TimeoutExpired:
            host.kill()
            host.wait()
        if sockdir:
            shutil.rmtree(sockdir, ignore_errors=True)
    if tot['bad']:
        print(f"{tot['bad']} mismatches (records in {out}; the files in {BASE}/xdoc-mm)", flush=True)
    boxmemo_off = env.get('FLASHTEX_BOXMEMO') == 'off'
    if boxmemo_off:
        tot['gate'] = 'not exercised (BOX-MEMO off)'
    elif tot['compiles'] and not tot['xdoc_hits']:
        reason = ('cross-document replay count not measured' if tot['xdoc_hits'] is None
                  else 'no cross-document replays')
        print(f'sound-xdoc: {reason}', flush=True)
    if not tot['compiles']:
        print('sound-xdoc: no compiles', flush=True)
    print(json.dumps(tot), flush=True)
    rec.close()
    if not a.keep:
        shutil.rmtree(W, ignore_errors=True)
    sys.exit(2 if tot['err'] else 1 if tot['bad'] or not tot['compiles']
             or not boxmemo_off and not tot['xdoc_hits'] else 0)


if __name__ == '__main__':
    main()
