import re, sys, collections
txt = open(sys.argv[1]).read()
sec = txt.split('Sort by top of stack, same collapsed (when >= 5):')[1]
idle = re.compile(r'__accept|__recvfrom|__psynch_cvwait|__ulock_wait|kevent|__select|__semwait|mach_msg2_trap|__workq_kernreturn|__read_nocancel|__wait4')
groups = [
 ('checkpoint (Arena::checkpoint, seal, merge, Core::save, pack)', r'Arena10checkpoint|pack_words|merge_sealed|4Core4save|seal|10checkpoint|retain|enforce_budget|thin'),
 ('incr session (tests, find, journal)', r'4incr|7readset|3iso|statediff|journal|ReadLog'),
 ('display list (note_node, interp, encode, send)', r'displaylist|display_list|dl_note|dl_token|Sha256|sha2'),
 ('PDF writer + zlib', r'6pdftex|pdf_|zlib|deflate|adler|crc32|writet1|7writet'),
 ('diag / boxmemo / intrinsics', r'4diag|dg_def|dg_here|boxmemo|intrinsics|macroprof'),
 ('kpathsea / file lookups / fs', r'resolver|kpse|fswatch|hashcache|lookupproof|stat\b|lstat|getattrlist|__open|__close|__fstat|__stat'),
 ('malloc / memmove', r'malloc|_free|memmove|memcpy|memset|realloc|_platform_|nanov2|xzm|szone'),
 ('format load', r'fmtimage|load_fmt|undump|open_fmt'),
]
tot = 0; g = collections.Counter()
for l in sec.splitlines():
    m = re.match(r'\s+(.*?)\s+\(in ([^)]*)\)\s+(\d+)', l) or re.match(r'\s+(.*?)\s+(\d+)$', l)
    if not m: continue
    sym, n = m.group(1), int(m.groups()[-1])
    if idle.search(sym): continue
    tot += n
    for name, pat in groups:
        if re.search(pat, sym): g[name] += n; break
    else: g['TeX interpreter (generated body, arena reads)'] += n
print(f'engine-thread samples {tot} (1 ms each)')
for k, v in g.most_common(): print(f'{v:8d} {100*v/tot:5.1f}%  {k}')
