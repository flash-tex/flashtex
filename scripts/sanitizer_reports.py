#!/usr/bin/env python3
"""sanitizer_reports.py: judge AddressSanitizer/LeakSanitizer report files (lane MEMORY-SAFETY,
scripts/sanitizers.sh).

    sanitizer_reports.py [--growth-against DIR] DIR

DIR holds the `log_path` files of one run (ASAN_OPTIONS=log_path=DIR/asan). The verdict:

* any AddressSanitizer error (use after free, overflow, double free, ...) fails;
* a leak fails unless every frame of its allocation stack that is not an allocator is in
  TeX Live's kpathsea (third_party/kpathsea, vendored unmodified). kpathsea's start-up loses
  ~53 KB of strings once per process (texmf.cnf, brace expansion, the ls-R database names):
  upstream says "quite a lot of the freeing is not safe" (kpathsea.c, kpathsea_finish), and
  TeX Live's pdftex loses the same. Those are reported, not failed;
* with --growth-against SHORT, the leaked bytes of DIR (a longer edit session) must not exceed
  SHORT's (a shorter one): a leak per edit, kpathsea's included, fails.

Exit status 0 when the run passes. Prints one summary line per allocation site.
"""
import argparse
import collections
import glob
import os
import re
import sys

ALLOC = re.compile(r'^(malloc|calloc|realloc|free|xmalloc|xcalloc|xrealloc|xstrdup|_Zn[wa]m.*|'
                   r'__rust_alloc.*|__rust_realloc|__rdl_.*|__rg_.*|wrap_.*|.*alloc::alloc.*|'
                   r'_RNv.*5alloc.*)$')
# kpathsea's own functions (third_party/kpathsea/*.c), as they appear in a symbolised stack.
KPATHSEA = re.compile(r'^(kpathsea_\w+|kpse_\w+|brace_expand\w*|str_list_\w+|cstr_list_\w+|'
                      r'str_llist_\w+|concat\w*|fn_\w+|read_line|do_line|hash_\w+|uppercasify|'
                      r'search|absolute_search|dir_list_\w+|expand_elt|dirs?_\w+|db_\w+|'
                      r'find_dpi|init_path|init_maketex|remove_dots|try_size|try_resolution|'
                      r'try_fontmap|try_format|try_fallback_resolutions|maketex|target_asis_name|'
                      r'target_fontmap|target_suffixed_names|flashtex_kpse_new|xdirname|xbasename|'
                      r'xgetcwd|kpathsea|expand|expanding_p|log_search)$')
SYM = re.compile(r'#\d+ 0x[0-9a-f]+ in (\S+)')


def frames(block):
    out = []
    for f in SYM.findall(block):
        f = re.sub(r'\+0x[0-9a-f]+$', '', f)
        if not ALLOC.match(f):
            out.append(f)
    return out


def judge(d):
    """(errors, upstream_bytes, ours) for the reports in d; ours = [(bytes, objs, frames)]."""
    errors, upstream, ours = [], 0, []
    for path in sorted(glob.glob(os.path.join(d, '*'))):
        if not os.path.isfile(path):
            continue
        text = open(path, errors='replace').read()
        for m in re.finditer(r'ERROR: AddressSanitizer: (.*)', text):
            errors.append(f'{os.path.basename(path)}: {m.group(1)}')
        for b in re.split(r'\n(?=(?:Direct|Indirect) leak of)', text):
            m = re.match(r'(Direct|Indirect) leak of (\d+) byte\(s\) in (\d+) object', b)
            if not m:
                continue
            fr = frames(b)
            n, objs = int(m.group(2)), int(m.group(3))
            # The stack up to the first frame outside kpathsea decides: a leak allocated inside
            # kpathsea is upstream's, whoever called kpathsea.
            first = fr[0] if fr else ''
            if fr and KPATHSEA.match(first):
                upstream += n
            else:
                ours.append((n, objs, fr[:6], m.group(1)))
    return errors, upstream, ours


def total(d):
    e, up, ours = judge(d)
    return up + sum(o[0] for o in ours)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('dir')
    ap.add_argument('--growth-against', default='')
    a = ap.parse_args()
    errors, upstream, ours = judge(a.dir)
    ok = True
    for e in errors:
        print(f'FAIL AddressSanitizer: {e}')
        ok = False
    sites = collections.defaultdict(lambda: [0, 0])
    for n, objs, fr, kind in ours:
        k = (kind,) + tuple(fr[:3])
        sites[k][0] += n
        sites[k][1] += objs
    for k, (n, objs) in sorted(sites.items(), key=lambda kv: -kv[1][0]):
        print(f'FAIL leak {n} B in {objs} object(s) ({k[0]}): ' + ' <- '.join(k[1:]))
        ok = False
    print(f'kpathsea (upstream, one-time start-up): {upstream} B leaked')
    if a.growth_against:
        short, long_ = total(a.growth_against), total(a.dir)
        verdict = 'PASS' if long_ <= short else 'FAIL'
        print(f'{verdict} leak growth: {short} B after the short session, {long_} B after the long one')
        ok = ok and long_ <= short
    print('PASS no sanitizer findings' if ok else 'FAIL sanitizer findings above')
    return 0 if ok else 1


if __name__ == '__main__':
    sys.exit(main())
