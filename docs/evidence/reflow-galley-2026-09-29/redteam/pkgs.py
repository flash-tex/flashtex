import subprocess,re
pk='needspace wrapfig multicol footmisc perpage afterpage changepage lettrine longtable fancyhdr zref-savepos tikzmark'.split()
pat=re.compile(r'\\pagetotal|\\pagegoal|\\c@page|\\thepage|\\pdfsavepos|\\pagedepth|\\lastpenalty|\\aftergroup|\\output *=|\\output\{|\\pagediscards')
for p in pk:
    f=subprocess.run(['/Library/TeX/texbin/kpsewhich',p+'.sty'],capture_output=True,text=True).stdout.strip()
    print('==',p,f)
    if not f: continue
    for i,l in enumerate(open(f,encoding='latin-1')):
        if l.lstrip().startswith('%'): continue
        if pat.search(l): print(f'  {i+1}: {l.rstrip()[:130]}')
