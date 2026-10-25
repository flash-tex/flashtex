#!/usr/bin/env python3
"""mpstat.py HOST-STDERR: summarise the [modpatch] probe lines (see README.md).

Per incremental run, for the strict and the relaxed comparison: pages tested, aligned, equal
modulo the patch, the longest runs of equal pages, the first blocker per page (in the order
input -> effects -> C-part state -> meanings -> uncovered words -> structures), the names,
regions and structure differences behind them; the old run's reads of patched names per page
interval; the test cost.
"""
import collections
import json
import re
import statistics
import sys


def cls_name(n):
    n = n.split('=')[0]
    for pre in ('\\r@', '\\@cref@', '\\bibcite', '\\b@'):
        if n.startswith(pre):
            return pre + '*'
    if n.endswith(' image'):
        return '\\<file> image'
    return n


def iso_class(e):
    e = re.sub(r'\(walking .*', '', e)
    e = re.sub(r'0x[0-9a-f]+', 'X', e)
    e = re.sub(r'-?\d+', 'N', e)
    return e.strip()[:100]


def first_blocker(r, c, relax):
    if not r.get('aligned'):
        return 'no old page at this input position'
    if not r['input']:
        return 'input'
    if not r['effects']:
        return 'external effects'
    bad = [p for p in c['cparts'] if not (relax and p == 'img~')]
    if bad:
        return "C-part state: " + '+'.join(bad)
    if c.get('delta_err'):
        return 'meanings: ' + c['delta_err'][:70]
    if c['unexpl_n']:
        return 'a meaning outside the patch'
    if c['uncov_n']:
        return 'words outside the structures'
    iso = c.get('iso')
    if iso and 'err' in iso:
        return 'structures: ' + iso_class(iso['err'])
    return None


def runs_of(flags):
    out, cur, start, last = [], 0, None, None
    for page, ok in flags:
        if ok:
            if cur == 0:
                start = page
            cur += 1
            last = page
        else:
            if cur:
                out.append((cur, start, last))
            cur = 0
    if cur:
        out.append((cur, start, last))
    return sorted(out, reverse=True)


def summary(rs, key, relax):
    tested = len(rs)
    get = lambda r: r.get(key) if r.get('aligned') else None
    eq = [r for r in rs if r.get('aligned') and get(r) and get(r)['eq']]
    print(f"  [{key}] equal modulo the patch: {len(eq)} of {tested} ({100.0 * len(eq) / max(tested, 1):.1f} %)")
    rl = runs_of([(r['page'], bool(r.get('aligned') and get(r)['eq'])) for r in rs])
    print(f"  [{key}] longest equal runs (count, first page, last page): {rl[:6]}")
    fb = collections.Counter(first_blocker(r, get(r) or {}, relax) for r in rs
                             if not (r.get('aligned') and get(r)['eq']))
    print(f"  [{key}] first blocker per page:")
    for k, v in fb.most_common(12):
        print(f"    {v:5d}  {k}")
    al = [r for r in rs if r.get('aligned') and r['input']]
    cp = collections.Counter(p for r in al for p in get(r)['cparts'])
    if cp:
        print(f"  [{key}] C-part parts differing (pages): {cp.most_common()}")
    un = collections.Counter(cls_name(n) for r in al for n in get(r)['unexpl'])
    if un:
        print(f"  [{key}] meanings outside the patch (pages, by class): {un.most_common(10)}")
    uc = collections.Counter()
    for r in al:
        if get(r)['uncov_n']:
            for part in get(r)['uncov_sum'].split(' '):
                if ':' in part:
                    uc[part.split(':')[0]] += 1
    if uc:
        print(f"  [{key}] uncovered word regions (pages): {uc.most_common(14)}")
    ic = collections.Counter(iso_class(get(r)['iso']['err']) for r in al
                             if get(r).get('iso') and 'err' in get(r)['iso'])
    if ic:
        print(f"  [{key}] structure differences (pages):")
        for k, v in ic.most_common(8):
            print(f"    {v:5d}  {k}")
    if relax:
        st = collections.Counter(re.sub(r'\d+', 'N', get(r)['strs'])[:60] for r in al)
        print(f"  [{key}] string numbering: {st.most_common(5)}")


def main():
    lines = [json.loads(l.split('[modpatch] ', 1)[1]) for l in open(sys.argv[1], errors='replace')
             if l.startswith('[modpatch] ')]
    by = collections.defaultdict(list)
    meta = collections.defaultdict(dict)
    for r in lines:
        if 'page' in r:
            by[r['run']].append(r)
        else:
            meta[r['run']].update(r)
    for run in sorted(by):
        rs = by[run]
        m = meta[run]
        print(f"== run {run}: base {m.get('base')}, old pages {m.get('old_pages')} (kept {m.get('old_kept')}), "
              f"L5 patch {m.get('patch')} names ({m.get('from_l5')}); extended by {m.get('patch_ext_extra')}: "
              f"{m.get('names', [])[:8]}")
        aligned = sum(1 for r in rs if r.get('aligned'))
        offs = collections.Counter(r.get('off') for r in rs if r.get('aligned'))
        print(f"  tested {len(rs)}, aligned by input position {aligned}; offsets {offs.most_common(4)}")
        summary(rs, 'strict', False)
        summary(rs, 'relaxed', True)
        rd = [r for r in rs if r.get('reads_n', -1) >= 0]
        if rd:
            n = sum(1 for r in rd if r['reads_n'] > 0)
            names = collections.Counter(cls_name(x) for r in rd for x in r['reads'])
            print(f"  old run's next page interval reads an (extended) patched name: {n} of {len(rd)} pages; "
                  f"names (pages) {names.most_common(8)}")
        ms = [r['ms'] for r in rs if r.get('aligned')]
        mi = [r['minstr'] for r in rs if r.get('aligned')]
        if len(ms) > 2:
            q = statistics.quantiles(ms, n=10)
            print(f"  test cost (both comparisons): median {statistics.median(ms):.1f} ms, p90 {q[-1]:.1f} ms, "
                  f"max {max(ms):.1f} ms, total {sum(ms) / 1e3:.1f} s; median {statistics.median(mi):.0f} M instructions")


if __name__ == '__main__':
    main()
