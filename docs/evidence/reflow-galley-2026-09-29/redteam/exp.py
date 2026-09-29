"""Galley-diff and output-routine-share experiment (read-only on sources; all writes in scratchpad).

For each selected arXiv paper:
  run A: timing run (pdfelapsedtime accumulated inside \\output) on the unmodified doc
  run D: \\tracingoutput dump of the unmodified doc
  run E: same, with ~12 lines of plain text inserted before the first \\section of the main file
Then compare the multiset of "line" hboxes (hbox subtrees whose parent is a vbox) between D and E.
"""
import os, re, sys, glob, json, shutil, subprocess, time, collections

SRC = '/Users/kubar/.cache/flashtex-parity/src/arxiv'
WORK = '/private/tmp/claude-501/-Users-kubar-code-flashtex/80e4852c-78aa-4764-b1d3-86bb9edb4d82/scratchpad/galley-redteam/work'
PDFLATEX = '/Library/TeX/texbin/pdflatex'

TIMING = r"""
\newcount\ORacc \newcount\ORt \newcount\ORn \newcount\BodyStart
\AddToHook{begindocument/end}{%
  \global\BodyStart=\pdfelapsedtime\relax
  \global\output\expandafter{\expandafter\ORbegin\the\output\ORfinish}}
\protected\def\ORbegin{\global\ORt=\pdfelapsedtime\relax}
\protected\def\ORfinish{\global\advance\ORacc by \numexpr\pdfelapsedtime-\ORt\relax \global\advance\ORn by 1\relax}
\AddToHook{enddocument/afterlastpage}{\immediate\write-1{ORSTATS body=\the\numexpr\pdfelapsedtime-\BodyStart\relax\space or=\the\ORacc\space n=\the\ORn\space page=\the\c@page}}
"""
TRACE = r"""
\AddToHook{begindocument/end}{\showboxdepth=\maxdimen \showboxbreadth=\maxdimen \tracingoutput=1 \tracingonline=0 }
"""
FILLER = ("\n\nInserted filler paragraph for the galley experiment. " + "The quick brown fox jumps over the lazy dog again and again. " * 14 + "\n\n")


def strip(s):
    return re.sub(r'(?<!\\)%.*', '', s)


def find_main(d):
    for t in sorted(glob.glob(d + '/**/*.tex', recursive=True)):
        try:
            s = strip(open(t, encoding='utf-8', errors='replace').read())
        except Exception:
            continue
        if '\\documentclass' in s and '\\begin{document}' in s:
            return t
    return None


def run(cwd, main, pre, jobname, outdir):
    os.makedirs(outdir, exist_ok=True)
    open(os.path.join(cwd, 'zzprelude.tex'), 'w').write(pre)
    rel = os.path.relpath(main, cwd)
    cmd = [PDFLATEX, '-interaction=nonstopmode', '-output-directory=' + outdir, '-jobname=' + jobname,
           r'\input{zzprelude}\input{' + rel + '}']
    t0 = time.time()
    try:
        subprocess.run(cmd, cwd=os.path.dirname(main), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=240)
    except subprocess.TimeoutExpired:
        return None, 0
    return os.path.join(outdir, jobname + '.log'), time.time() - t0


BOXLINE = re.compile(r'^(\.*)\\(hbox|vbox)\(')


def lines_from_log(path, any_parent=True):
    """Return list of hbox subtree strings whose parent is a vbox (i.e. lines, display rows, etc.)."""
    out = []
    txt = open(path, encoding='latin-1').read().split('\n')
    # find Completed box being shipped out blocks
    i = 0
    n = len(txt)
    while i < n:
        if txt[i].startswith('Completed box being shipped out'):
            j = i + 1
            block = []
            while j < n and (txt[j] != '' or (j + 1 < n and txt[j + 1].startswith('.'))):
                block.append(txt[j]); j += 1
            # join continuation lines: TeX wraps at max_print_line; lines not starting with '.' or '\\' are continuations
            joined = []
            prevlen = 0
            for l in block:
                if joined and prevlen == 79:
                    joined[-1] += l
                else:
                    joined.append(l)
                prevlen = len(l)
            stack = []  # (depth, kind, start_index)
            depth_of = []
            for k, l in enumerate(joined):
                d = len(l) - len(l.lstrip('.'))
                depth_of.append(d)
            for k, l in enumerate(joined):
                d = depth_of[k]
                m = BOXLINE.match(l)
                if m and m.group(2) == 'hbox':
                    # parent: nearest previous line with depth d-1
                    p = k - 1
                    while p >= 0 and depth_of[p] >= d:
                        p -= 1
                    if p >= 0 and (any_parent or '\\vbox(' in joined[p][:depth_of[p] + 6]):
                        e = k + 1
                        while e < len(joined) and depth_of[e] > d:
                            e += 1
                        sub = '\n'.join(x[d:] for x in joined[k:e])
                        if re.search(r'\\(?:OT1|T1|OML|OMS|OMX|U|TS1|LY1|T5|LGR)/', sub):
                            out.append(sub)
            i = j
        else:
            i += 1
    return out


def main():
    sel = sys.argv[1:]
    res = []
    for name in sel:
        src = os.path.join(SRC, name)
        dst = os.path.join(WORK, name)
        if not os.path.exists(dst):
            shutil.copytree(src, dst)
        m = find_main(dst)
        if not m:
            continue
        orig = open(m, encoding='utf-8', errors='replace').read()
        r = {'doc': name}
        la, ta = run(dst, m, TIMING, 'timing', os.path.join(dst, 'out'))
        if la and os.path.exists(la):
            s = open(la, encoding='latin-1').read()
            mm = re.search(r'ORSTATS body=(-?\d+) or=(-?\d+) n=(\d+) page=(\d+)', s)
            if mm:
                r.update(body_ms=int(mm.group(1)) / 65.536, or_ms=int(mm.group(2)) / 65.536, or_n=int(mm.group(3)), pages=int(mm.group(4)) - 1)
            r['wall_s'] = ta
        ld, _ = run(dst, m, TRACE, 'traceD', os.path.join(dst, 'out'))
        # edited copy
        k = re.search(r'\\section\*?[\[{]', orig[orig.find('\\begin{document}'):])
        if not k:
            res.append(r); continue
        pos = orig.find('\\begin{document}') + k.start()
        m2 = m[:-4] + '_zzedit.tex'
        open(m2, 'w').write(orig[:pos] + FILLER + orig[pos:])
        le, _ = run(dst, m2, TRACE, 'traceE', os.path.join(dst, 'out'))
        if ld and le and os.path.exists(ld) and os.path.exists(le):
            A = lines_from_log(ld)
            B = lines_from_log(le)
            ca, cb = collections.Counter(A), collections.Counter(B)
            common = sum((ca & cb).values())
            r.update(lines_D=len(A), lines_E=len(B), matched=common,
                     pagesD=open(ld, encoding='latin-1').read().count('Completed box being shipped out'),
                     pagesE=open(le, encoding='latin-1').read().count('Completed box being shipped out'))
            onlyA = list((ca - cb).elements())
            r['unmatched_D'] = len(onlyA)
            r['unmatched_samples'] = [x[:300] for x in onlyA[:6]]
        res.append(r)
        print(json.dumps({k: v for k, v in r.items() if k != 'unmatched_samples'}), flush=True)
    json.dump(res, open(os.path.join(WORK, 'results-%d.json' % os.getpid()), 'w'), indent=1)


if __name__ == '__main__':
    main()
