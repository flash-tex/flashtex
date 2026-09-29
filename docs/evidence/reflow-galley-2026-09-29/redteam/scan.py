import os, re, sys, json, glob
import statistics as st

root = sys.argv[1]
mode = sys.argv[2]
out = sys.argv[3]


def strip(s):
    return re.sub(r'(?<!\\)%.*', '', s)


def read(p):
    try:
        return strip(open(p, encoding='utf-8', errors='replace').read())
    except Exception:
        return ''


docs = []
if mode == 'arxiv':
    for d in sorted(os.listdir(root)):
        dd = os.path.join(root, d)
        if not os.path.isdir(dd):
            continue
        texs = glob.glob(dd + '/**/*.tex', recursive=True)
        main = None
        for t in texs:
            s = read(t)
            if '\\documentclass' in s and '\\begin{document}' in s:
                main = t
                break
        if not main:
            continue
        others = ''.join(read(t) for t in texs if t != main)
        m = read(main)
        mi = m.find('\\begin{document}')
        docs.append((d, m[:mi], m[mi:] + others))
else:
    for t in glob.glob(root + '/**/*.tex', recursive=True):
        s = read(t)
        if '\\documentclass' in s and '\\begin{document}' in s and '\\input' not in s and '\\include{' not in s:
            mi = s.find('\\begin{document}')
            docs.append((t, s[:mi], s[mi:]))

feat = {
    'thepage_body': r'\\thepage\b',
    'c@page_body': r'\\c@page\b|\\value\{page\}|\\arabic\{page\}',
    'pageref': r'\\(?:[cCvV]?pageref|autopageref|pagerefrange|vpagerefrange)\*?\{',
    'marginpar': r'\\marginpar\b|\\marginnote\b|\\todo\b',
    'wrapfig': r'\\begin\{wrap(?:figure|table)\}',
    'multicols': r'\\begin\{multicols\*?\}',
    'longtable': r'\\begin\{(?:longtable|xltabular|supertabular)\*?\}',
    'footnote': r'\\footnote\b',
    'floats': r'\\begin\{(?:figure|table|algorithm|sidewaysfigure|sidewaystable)\*?\}',
    'includegraphics': r'\\includegraphics\b',
    'remember_picture': r'remember picture|\\tikzmark\b',
    'needspace_pagetotal': r'\\[Nn]eedspace\b|\\pagetotal\b|\\pagegoal\b|\\enlargethispage\b',
    'clearpage': r'\\clearpage\b|\\newpage\b|\\cleardoublepage\b|\\pagebreak\b',
    'afterpage': r'\\afterpage\b',
    'lettrine': r'\\lettrine\b',
    'savepos': r'\\pdfsavepos|\\zsavepos',
    'oddpage': r'\\checkoddpage|\\ifoddpage|\\ifthispageodd',
    'display': r'\\\[|\\begin\{(?:equation|align|gather|multline|eqnarray|displaymath)\*?\}|\$\$',
    'section': r'\\(?:section|subsection|subsubsection|chapter|paragraph)\*?[\[{]',
    'item': r'\\item\b',
}
pre_feat = {
    'perpage': r'\\MakePerPage|perpage|\\counterwithin\*?\{footnote\}\{page\}|\\@addtoreset\{footnote\}\{page\}',
    'fancyhdr': r'fancyhdr',
    'hyperref': r'hyperref',
    'twocolumn': r'twocolumn',
    'multicolpkg': r'\{multicol\}',
    'wrapfigpkg': r'wrapfig',
    'tikz': r'\{tikz\}',
    'afterpagepkg': r'afterpage',
    'needspacepkg': r'needspace',
    'lineno': r'lineno',
    'biblatex': r'biblatex',
}
HDR = r'\\(?:fancyhead|fancyfoot|lhead|chead|rhead|lfoot|cfoot|rfoot|markboth|markright|fancypagestyle|pagestyle)\b[^\n]*|\\def\\ps@\w+[^\n]*'
PD_ALL = ['thepage_body', 'c@page_body', 'marginpar', 'wrapfig', 'multicols', 'longtable', 'floats',
          'includegraphics', 'needspace_pagetotal', 'clearpage', 'afterpage', 'lettrine', 'oddpage']
PD_CORE = ['thepage_body', 'c@page_body', 'wrapfig', 'multicols', 'longtable', 'needspace_pagetotal',
           'clearpage', 'afterpage', 'lettrine', 'oddpage']
rows = []
for name, pre, body in docs:
    body_nohdr = re.sub(HDR, '', body)
    blocks = re.split(r'\n\s*\n', body)
    paras = [p for p in blocks if re.search(r'[A-Za-z]{3,}\s+[A-Za-z]{3,}\s+[A-Za-z]{3,}', re.sub(r'\\[A-Za-z@]+', '', p))]
    r = {'doc': name, 'paras': len(paras), 'blocks': len(blocks), 'chars': len(body)}
    for k, v in feat.items():
        r[k] = len(re.findall(v, body_nohdr if k in ('thepage_body', 'c@page_body') else body))
    for k, v in pre_feat.items():
        r['pre_' + k] = 1 if re.search(v, pre) else 0
    pd_all = re.compile('|'.join(feat[k] for k in PD_ALL))
    pd_core = re.compile('|'.join(feat[k] for k in PD_CORE))
    r['blocks_pd_all'] = sum(1 for p in blocks if pd_all.search(re.sub(HDR, '', p)))
    r['blocks_pd_core'] = sum(1 for p in blocks if pd_core.search(re.sub(HDR, '', p)))
    r['blocks_float'] = sum(1 for p in blocks if re.search(feat['floats'] + '|' + feat['marginpar'], p))
    r['blocks_img'] = sum(1 for p in blocks if re.search(feat['includegraphics'], p))
    rows.append(r)
json.dump(rows, open(out, 'w'), indent=0)
N = len(rows)
print('docs', N)
for k in [k for k in rows[0] if k != 'doc']:
    vals = [r[k] for r in rows]
    nz = sum(1 for v in vals if v > 0)
    print(f'{k:22s} docs>0={nz:5d} ({100 * nz / N:5.1f}%)  mean={st.mean(vals):9.2f} median={st.median(vals):8.1f}')
tb = sum(r['blocks'] for r in rows)
for k in ['blocks_pd_all', 'blocks_pd_core', 'blocks_float', 'blocks_img']:
    s = sum(r[k] for r in rows)
    fr = [r[k] / r['blocks'] for r in rows if r['blocks']]
    print(f'{k}: pooled {100 * s / tb:.2f}% of blank-line blocks; per-doc median {100 * st.median(fr):.2f}%, p90 {100 * sorted(fr)[int(0.9 * len(fr))]:.2f}%')
