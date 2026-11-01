#!/usr/bin/env python3
"""mkbook.py [BOOK ...]: the book benchmark documents, under INCR_BENCH_DIR (default
/tmp/incr-bench)/docs/BOOK, for t7.py, mem.py and mem_gate.sh. The default is every book.

  infdesc      Clive Newstead's *An Infinite Descent into Pure Mathematics* (592 pages), the
               books.json entry infdesc-48825c5 (tools/parity/corpus/books.json)
  infdesc-x2   the same with its main matter and appendices twice (1,142 pages; the owner's heavy
               benchmark of 2026-10-06, tools/parity/corpus/infdesc_x2.py)

The archive is fetched from the author's repository at the pinned commit, checked against the
entry's SHA-256 and kept in INCR_BENCH_DIR/cache; nothing from the book is committed here (LPPL
1.3c source, CC BY-SA 4.0 book: books.json's licence note). The tree is unpacked into docs/BOOK
with the entry file as main.tex, and docs/BOOK/doc.json says how the drivers edit it (docspec.py):
in-body keystrokes go into `edit`, a chapter the main file inputs (the main file's body is only
\\input lines; in infdesc-x2 the first copy's: the second inputs copies of its own, so a keystroke
changes one place). A changed generator or entry makes the document again.
"""
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tarfile
import urllib.request

S = os.path.dirname(os.path.abspath(__file__))
W = os.path.dirname(os.path.dirname(S))
IB = os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')
CORPUS = os.path.join(W, 'tools', 'parity', 'corpus')
# BOOK: (books.json id, the chapter the in-body edits go into, the generator of the doubled book)
BOOKS = {
    # number theory: pages 245-290 (of the first copy), 33 prose lines
    'infdesc': ('infdesc-48825c5', 'book/number-theory/modular-arithmetic.tex', False),
    'infdesc-x2': ('infdesc-48825c5', 'book/number-theory/modular-arithmetic.tex', True),
}
X2 = os.path.join(CORPUS, 'infdesc_x2.py')


def fetch(entry):
    """The entry's archive, fetched once and checked against its SHA-256."""
    cache = os.path.join(IB, 'cache')
    os.makedirs(cache, exist_ok=True)
    path = os.path.join(cache, f"{entry['id']}.{entry['format']}")
    if not os.path.exists(path):
        tmp = path + '.part'
        with urllib.request.urlopen(entry['url'], timeout=300) as r, open(tmp, 'wb') as f:
            shutil.copyfileobj(r, f)
        os.replace(tmp, path)
    h = hashlib.sha256(open(path, 'rb').read()).hexdigest()
    if h != entry['sha256']:
        os.unlink(path)
        sys.exit(f"mkbook: {entry['url']}: SHA-256 {h}, not the pinned {entry['sha256']}")
    return path


def make(book):
    eid, edit, doubled = BOOKS[book]
    entry = next(e for e in json.load(open(os.path.join(CORPUS, 'books.json')))['entries'] if e['id'] == eid)
    archive = fetch(entry)
    d = os.path.join(IB, 'docs', book)
    stamp = os.path.join(d, 'doc.json')
    want = dict(source=entry['id'], sha256=entry['sha256'], edit=edit,
                generator=hashlib.sha256(open(X2, 'rb').read()).hexdigest() if doubled else None)
    if os.path.exists(stamp) and json.load(open(stamp)) == want:
        return d
    shutil.rmtree(d, ignore_errors=True)
    tmp = d + '.unpack'
    shutil.rmtree(tmp, ignore_errors=True)
    os.makedirs(tmp)
    with tarfile.open(archive) as t:
        if hasattr(tarfile, 'data_filter'):
            t.extractall(tmp, filter='data')
        else:  # (Python before 3.12 and its backports: the archive is the pinned one, checked above)
            t.extractall(tmp)
    shutil.move(os.path.join(tmp, entry['root']), d)
    shutil.rmtree(tmp)
    main = entry['entry']
    if doubled:
        subprocess.run([sys.executable, X2, d], check=True, stdout=subprocess.DEVNULL)
        main = 'infdesc-x2.tex'
    shutil.copy(os.path.join(d, main), os.path.join(d, 'main.tex'))
    json.dump(want, open(stamp, 'w'), indent=1)
    return d


def main():
    books = sys.argv[1:] or list(BOOKS)
    bad = [b for b in books if b not in BOOKS]
    if bad:
        sys.exit(f"mkbook: unknown {bad}; known: {', '.join(BOOKS)}")
    for b in books:
        print(make(b))


if __name__ == '__main__':
    main()
