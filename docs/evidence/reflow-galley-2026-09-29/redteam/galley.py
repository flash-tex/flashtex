"""Reconstruct the main-galley item stream (children of each text column box after \\topskip) and diff D vs E."""
import os, re, sys, difflib, hashlib, collections

WORK = '/private/tmp/claude-501/-Users-kubar-code-flashtex/80e4852c-78aa-4764-b1d3-86bb9edb4d82/scratchpad/galley-redteam/work'


def blocks(path):
    txt = open(path, encoding='latin-1').read().split('\n')
    i, n = 0, len(txt)
    while i < n:
        if txt[i].startswith('Completed box being shipped out'):
            j = i + 1
            block = []
            while j < n and (txt[j] != '' or (j + 1 < n and txt[j + 1].startswith('.'))):
                block.append(txt[j]); j += 1
            joined, prevlen = [], 0
            for l in block:
                if joined and prevlen == 79:
                    joined[-1] += l
                else:
                    joined.append(l)
                prevlen = len(l)
            yield joined
            i = j
        else:
            i += 1


def depth(l):
    return len(l) - len(l.lstrip('.'))


def galley(path):
    items = []
    for page in blocks(path):
        dep = [depth(l) for l in page]
        for k, l in enumerate(page):
            if l.lstrip('.').startswith('\\glue(\\topskip)'):
                d = dep[k]
                # siblings after topskip at same depth, until parent ends
                e = k + 1
                items.append('<PAGE>')
                while e < len(page) and dep[e] >= d:
                    if dep[e] == d:
                        f = e + 1
                        while f < len(page) and dep[f] > d:
                            f += 1
                        head = page[e][d:]
                        sub = '\n'.join(x[d:] for x in page[e:f])
                        if head.startswith(('\\hbox', '\\vbox')):
                            items.append(head.split(',')[0] + '#' + hashlib.md5(sub.encode()).hexdigest()[:8])
                        else:
                            items.append(head)
                        e = f
                    else:
                        e += 1
    return items


doc = sys.argv[1]
ta, tb = sys.argv[2], sys.argv[3]
A = galley(os.path.join(WORK, doc, 'out', ta + '.log'))
B = galley(os.path.join(WORK, doc, 'out', tb + '.log'))
sm = difflib.SequenceMatcher(a=A, b=B, autojunk=False)
kinds = collections.Counter()
nA = sum(1 for x in A if x != '<PAGE>')
changed = 0
examples = []
for tag, i1, i2, j1, j2 in sm.get_opcodes():
    if tag == 'equal':
        continue
    a = [x for x in A[i1:i2] if x != '<PAGE>']
    b = [x for x in B[j1:j2] if x != '<PAGE>']
    changed += len(a)
    for x in a:
        kinds['D-only ' + re.sub(r'[-0-9.]+', '#', x.split('#')[0])[:50]] += 1
    for x in b:
        kinds['E-only ' + re.sub(r'[-0-9.]+', '#', x.split('#')[0])[:50]] += 1
    if len(examples) < 8 and (a or b) and len(a) + len(b) < 12:
        examples.append((a, b))
print(f'{doc}: galley items D={nA} E={sum(1 for x in B if x != "<PAGE>")} D-items-not-aligned={changed} ({100 * changed / max(nA, 1):.2f}%)')
for k, v in kinds.most_common(14):
    print('   ', v, k)
for a, b in examples:
    print('   EX D:', a[:5], '\n      E:', b[:5])
