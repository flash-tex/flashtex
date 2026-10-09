import re, sys, collections


def defs(path):
    d = {}
    for line in open(path, encoding='latin-1'):
        m = re.match(r'\\newlabel\{(.*?)\}(.*)', line)
        if m:
            d['\\r@' + m.group(1)] = m.group(2)
            continue
        m = re.match(r'\\bibcite\{(.*?)\}(.*)', line)
        if m:
            d['\\b@' + m.group(1)] = m.group(2)
            continue
        m = re.match(r'\\x?gdef\s*(\\[A-Za-z@]+)(.*)', line)
        if m:
            d[m.group(1)] = m.group(2)
            continue
        m = re.match(r'\\def\s*(\\[A-Za-z@]+)(.*)', line)
        if m:
            d[m.group(1)] = m.group(2)
    return d


if __name__ == '__main__':
    a, b = defs(sys.argv[1]), defs(sys.argv[2])
    ch = {n for n in set(a) | set(b) if a.get(n) != b.get(n)}
    pages = collections.defaultdict(set)
    for line in open(sys.argv[3]):
        pg, name = line.split(None, 1)
        name = name.strip()
        if name in ch:
            pages[int(pg)].add(name)
    n = int(sys.argv[4])
    print(f"changed names {len(ch)}; page intervals reading a changed name: {len(pages)} of {n}")
    cnt = collections.Counter(x for s in pages.values() for x in s)
    print(cnt.most_common(8))
    only = [p for p, s in pages.items() if s <= {'\\@abspage@last'}]
    print("intervals whose only changed read is abspage@last:", len(only))
