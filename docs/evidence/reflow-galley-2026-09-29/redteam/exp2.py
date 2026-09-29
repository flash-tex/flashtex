import os, re, sys, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import exp

WORK = exp.WORK
TIMING2 = r"""
\newcount\ORacc \newcount\ORt \newcount\ORn \newcount\BodyStart \newcount\SHacc \newcount\SHt
\AddToHook{begindocument/end}{%
  \global\BodyStart=\pdfelapsedtime\relax
  \global\output\expandafter{\expandafter\ORbegin\the\output\ORfinish}}
\protected\def\ORbegin{\global\ORt=\pdfelapsedtime\relax}
\protected\def\ORfinish{\global\advance\ORacc by \numexpr\pdfelapsedtime-\ORt\relax \global\advance\ORn by 1\relax}
\AddToHook{shipout/before}{\global\SHt=\pdfelapsedtime\relax}
\AddToHook{shipout/after}{\global\advance\SHacc by \numexpr\pdfelapsedtime-\SHt\relax}
\AddToHook{enddocument/afterlastpage}{\immediate\write-1{ORSTATS body=\the\numexpr\pdfelapsedtime-\BodyStart\relax\space or=\the\ORacc\space n=\the\ORn\space ship=\the\SHacc}}
"""
BIG = "\n\n" + ("Inserted filler paragraph for the galley experiment. " + "The quick brown fox jumps over the lazy dog again and again. " * 14 + "\n\n") * 7

res = []
for name in sys.argv[1:]:
    dst = os.path.join(WORK, name)
    m = exp.find_main(dst)
    if not m or m.endswith('_zzedit.tex'):
        cands = [t for t in [m] if t]
    orig = open(m, encoding='utf-8', errors='replace').read()
    r = {'doc': name}
    for rep in range(3):
        la, ta = exp.run(dst, m, TIMING2, 'timing2', os.path.join(dst, 'out'))
        s = open(la, encoding='latin-1').read() if la and os.path.exists(la) else ''
        mm = re.search(r'ORSTATS body=(-?\d+) or=(-?\d+) n=(\d+) ship=(-?\d+)', s)
        if mm:
            vals = dict(body_ms=int(mm.group(1)) / 65.536, or_ms=int(mm.group(2)) / 65.536, or_n=int(mm.group(3)), ship_ms=int(mm.group(4)) / 65.536)
            if 'body_ms' not in r or vals['body_ms'] < r['body_ms']:
                r.update(vals)
    k = re.search(r'\\section\*?[\[{]', orig[orig.find('\\begin{document}'):])
    if k:
        pos = orig.find('\\begin{document}') + k.start()
        m3 = m[:-4] + '_zzbig.tex'
        open(m3, 'w').write(orig[:pos] + BIG + orig[pos:])
        exp.run(dst, m3, exp.TRACE, 'traceE2', os.path.join(dst, 'out'))
    res.append(r)
    print(json.dumps(r), flush=True)
