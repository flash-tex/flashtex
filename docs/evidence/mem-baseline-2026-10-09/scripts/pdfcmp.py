"""pdfcmp.py TAG BASE PR...: for every document and mode both ran, whether the final PDF, log and aux
(after the same keystrokes) are byte-identical between BASE and each PR engine."""
import glob
import os
import sys

tag, base, prs = sys.argv[1], sys.argv[2], sys.argv[3:]
W = '/private/tmp/mb/w'


def norm(d, wdir):
    """The work directory's name (in the log, and in the PDF's /ID, an MD5 of the file's
    path and time) taken out."""
    import re
    d = d.replace(wdir.encode(), b'WORKDIR')
    d = d.replace(wdir.replace('/private', '').encode(), b'WORKDIR')
    return re.sub(rb'/ID \[<[0-9A-F]+> <[0-9A-F]+>\]', b'/ID []', d)


same = diff = 0
for bdir in sorted(glob.glob(f'{W}/{tag}-{base}-*')):
    rest = bdir[len(f'{W}/{tag}-{base}-'):]
    for pr in prs:
        pdir = f'{W}/{tag}-{pr}-{rest}'
        if not os.path.isdir(pdir):
            continue
        for f in sorted(glob.glob(f'{bdir}/out/*')):
            name = os.path.basename(f)
            if not name.endswith(tuple(os.environ.get('EXTS', '.pdf,.log,.aux').split(','))):
                continue
            g = f'{pdir}/out/{name}'
            a = norm(open(f, 'rb').read(), bdir)
            b = norm(open(g, 'rb').read(), pdir) if os.path.exists(g) else None
            if a == b:
                same += 1
            else:
                diff += 1
                print('DIFF', rest, pr, name, len(a), None if b is None else len(b))
print(f'{tag}: {same} files identical, {diff} different')
