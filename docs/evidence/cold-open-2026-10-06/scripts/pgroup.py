import subprocess, sys, re, collections
out = subprocess.run(['nix','shell','nixpkgs#perf','-c','perf','report','-i',sys.argv[1],'--no-children','--sort','dso,symbol','--stdio','-F','period,dso,sym'],capture_output=True,text=True).stdout
groups=[('checkpoint/arena', r'arena::|checkpoint::|Core>::save|quicksort::<\(u32, \*mut u64\)|merge_sealed|seal'),
        ('display list emit', r'displaylist::|flashtex_display_list::|dl_note_node|dl_token|sha256'),
        ('pdf writer + zlib', r'pdftex::|pdf_|zlib|deflate|adler|crc32|write_zip|libz'),
        ('diag/macroprof/intrinsics bookkeeping', r'diag::|dg_def|dg_here|intr_|flashtex_intr'),
        ('malloc/memmove', r'malloc|free|memmove|memcpy|memset|realloc|_int_'),
        ('kernel/other dso', r'\[kernel|\[unknown'),
       ]
tot=0; g=collections.Counter(); top=collections.Counter()
for l in out.splitlines():
    m=re.match(r'\s*(\d+)\s+(\S+)\s+\[\.\]\s+(.*)$', l)
    if not m: continue
    p=int(m.group(1)); sym=m.group(3); tot+=p
    for name,pat in groups:
        if re.search(pat, sym): g[name]+=p; break
    else: g['TeX interpreter + rest']+=p
print(f'total {tot/1e9:.1f} G')
for k,v in g.most_common(): print(f'{v/1e9:7.1f} G {100*v/tot:5.1f}%  {k}')
