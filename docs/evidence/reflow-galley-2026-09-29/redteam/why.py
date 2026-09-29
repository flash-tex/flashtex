import os, re, sys, collections, difflib
sys.path.insert(0, os.path.dirname(__file__))
from exp import lines_from_log

WORK = '/private/tmp/claude-501/-Users-kubar-code-flashtex/80e4852c-78aa-4764-b1d3-86bb9edb4d82/scratchpad/galley-redteam/work'
doc = sys.argv[1]
ta, tb = (sys.argv[2], sys.argv[3]) if len(sys.argv) > 3 else ('traceD', 'traceE')
nshow = int(sys.argv[4]) if len(sys.argv) > 4 else 4


def text(s):
    out = []
    for l in s.split('\n'):
        m = re.match(r'^\.*\\[A-Z0-9]+/[^ ]+ (.*)$', l)
        if m:
            out.append(m.group(1))
        elif re.match(r'^\.*\\glue', l):
            out.append(' ')
    return ''.join(out)


A = [l for l in lines_from_log(os.path.join(WORK, doc, 'out', ta + '.log')) if '\\vbox(' not in l]
B = [l for l in lines_from_log(os.path.join(WORK, doc, 'out', tb + '.log')) if '\\vbox(' not in l]
cb = collections.Counter(B)
ca = collections.Counter(A)
only = list((ca - cb).elements())
btext = {}
for x in B:
    btext.setdefault(text(x), []).append(x)
kinds = collections.Counter()
shown = 0
for x in only:
    t = text(x)
    if t in btext:
        # same glyph text, different box: show diff of box dump
        y = btext[t][0]
        d = [l for l in difflib.unified_diff(x.split('\n'), y.split('\n'), lineterm='', n=0) if not l.startswith(('---', '+++', '@@'))]
        key = ' / '.join(d[:4])[:200]
        kinds['same-text:' + re.sub(r'[0-9.]+', '#', key)[:120]] += 1
        if shown < nshow:
            print('SAME TEXT, DIFFERENT BOX:', repr(t[:80]))
            print('   ', d[:8])
            shown += 1
    else:
        kinds['text-differs'] += 1
        if shown < nshow:
            best = difflib.get_close_matches(t, list(btext), n=1, cutoff=0.5)
            print('TEXT DIFFERS:', repr(t[:100]))
            print('   closest in E:', repr(best[0][:100]) if best else None)
            shown += 1
for k, v in kinds.most_common(12):
    print(v, k)
