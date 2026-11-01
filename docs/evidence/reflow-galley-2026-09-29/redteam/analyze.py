import os, re, sys, glob, collections, json
sys.path.insert(0, os.path.dirname(__file__))
from exp import lines_from_log

WORK = '/private/tmp/claude-501/-Users-kubar-code-flashtex/80e4852c-78aa-4764-b1d3-86bb9edb4d82/scratchpad/galley-redteam/work'
tags = sys.argv[1:] if len(sys.argv) > 1 else ['traceD', 'traceE']


def leaf(lines):
    return [l for l in lines if '\\vbox(' not in l]


def errors(p):
    s = open(p, encoding='latin-1').read()
    return len(re.findall(r'^! ', s, re.M)), s.count('Completed box being shipped out')


def classify(s):
    body = s.split('\n')
    glyphs = [l for l in body if re.match(r'^\.*\\(?:OT1|T1|OML|OMS|OMX|U|TS1|LY1)/', l)]
    if len(glyphs) <= 4 and all(re.search(r' [0-9ivxlc]$', g) for g in glyphs):
        return 'page-number/header-footer line'
    if '\\pdfrefximage' in s or '\\pdfrefxform' in s:
        return 'contains image/xform ref'
    if re.search(r'\\mark|\\write|\\pdfdest|\\pdfstartlink', s):
        return 'text line with whatsit/mark/link'
    return 'text line'


tot = collections.Counter()
out = {}
for d in sorted(os.listdir(WORK)):
    dd = os.path.join(WORK, d, 'out')
    a = os.path.join(dd, tags[0] + '.log')
    b = os.path.join(dd, tags[1] + '.log')
    if not (os.path.exists(a) and os.path.exists(b)):
        continue
    ea, pa = errors(a)
    eb, pb = errors(b)
    A = leaf(lines_from_log(a))
    B = leaf(lines_from_log(b))
    ca, cb = collections.Counter(A), collections.Counter(B)
    only = list((ca - cb).elements())
    cls = collections.Counter(classify(x) for x in only)
    out[d] = dict(errD=ea, errE=eb, pagesD=pa, pagesE=pb, leafD=len(A), leafE=len(B), unmatched=len(only), cls=dict(cls))
    print(d, json.dumps(out[d]))
    samples = [x for x in only if classify(x) != 'page-number/header-footer line'][:3]
    for s in samples:
        print('   SAMPLE:', s[:400].replace('\n', ' | '))
