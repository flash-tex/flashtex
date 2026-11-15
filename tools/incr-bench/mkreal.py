#!/usr/bin/env python3
"""mkreal.py [DOC ...]: T7's real documents (realdocs.json), under INCR_BENCH_DIR (default
/tmp/incr-bench)/docs/DOC, for t7.py. The default is every real document.

A real document is a document someone wrote, not a generated one (lane REAL-DOC-GATE, 2026-10-24:
the owner found typing on page 883 of Infinite Descent x2 taking 32 s, which T7's generated
documents never showed). Each comes from a pinned source and nothing of it is committed here:

  {"book": NAME}                          mkbook.py's book NAME (infdesc-x2: the owner's 1,142 pages)
  {"manifest": FILE, "id": ID}            an entry of tools/parity/corpus/FILE: an arXiv e-print
                                          (fetched, checked against its SHA-256) or a TeX Live
                                          document (copied from this TeX Live, its SHA-256 checked),
                                          through tools/parity/corpus.py's fetch_manifest

The tree goes into docs/DOC with the entry file copied to main.tex (t7.py's dl3-keys opens
main.tex), and docs/DOC/doc.json says how t7.py edits it (docspec.py): `edit`, the file a site
without an `edit` of its own types into (main.tex). A changed source or entry makes it again.
"""
import json
import os
import shutil
import subprocess
import sys

S = os.path.dirname(os.path.abspath(__file__))
W = os.path.dirname(os.path.dirname(S))
IB = os.environ.get('INCR_BENCH_DIR', '/tmp/incr-bench')
CORPUS = os.path.join(W, 'tools', 'parity', 'corpus')
SPEC = os.path.join(S, 'realdocs.json')
sys.path.insert(0, os.path.join(W, 'tools', 'parity'))


def spec():
    """realdocs.json's documents: {DOC: {source, pages, sites}}."""
    return json.load(open(SPEC))['docs']


def texmf_dist():
    """This TeX Live's texmf-dist (TeX Live documents are copied from it)."""
    try:
        out = subprocess.run(['kpsewhich', '-var-value', 'TEXMFDIST'], capture_output=True, text=True).stdout.strip()
    except OSError:
        out = ''
    return out or '/usr/local/texlive/2026/texmf-dist'


def make(doc):
    s = spec()[doc]
    src = s['source']
    if 'book' in src:
        import mkbook
        return mkbook.make(src['book'])
    import corpus
    cache = os.path.join(IB, 'cache', 'parity')
    rec = corpus.fetch_manifest(os.path.join(CORPUS, src['manifest']), cache, texmf=texmf_dist(), log=lambda m: None,
                                only={corpus.safe_id(src['id'])})
    if not rec or rec[0]['problem']:
        sys.exit(f"mkreal: {doc}: {src['id']}: {rec[0]['problem'] if rec else 'not in ' + src['manifest']}")
    rec = rec[0]
    d = os.path.join(IB, 'docs', doc)
    stamp = os.path.join(d, 'doc.json')
    want = dict(source=src, entry=rec['entry'], edit='main.tex',
                marker=open(os.path.join(rec['dir'], next(m for m in ('.parity-unpacked', '.parity-copied')
                                                         if os.path.exists(os.path.join(rec['dir'], m))))).read())
    if os.path.exists(stamp) and json.load(open(stamp)) == want:
        return d
    shutil.rmtree(d, ignore_errors=True)
    shutil.copytree(rec['dir'], d, ignore=shutil.ignore_patterns('.parity-*'))
    if rec['entry'] != 'main.tex':
        shutil.copy(os.path.join(d, rec['entry']), os.path.join(d, 'main.tex'))
    json.dump(want, open(stamp, 'w'), indent=1)
    return d


def main():
    docs = sys.argv[1:] or list(spec())
    bad = [d for d in docs if d not in spec()]
    if bad:
        sys.exit(f"mkreal: unknown {bad}; known: {', '.join(spec())}")
    for d in docs:
        print(make(d))


if __name__ == '__main__':
    main()
