"""docspec.py: how the socket drivers (t7.py, mem.py) set up and edit a benchmark document in
INCR_BENCH_DIR/docs/DOC. A generated document (mkdocs.py) is main.tex alone, edited in place; a
book (mkbook.py) is a tree whose doc.json names the chapter its in-body keystrokes go into."""
import json
import os
import shutil


def spec(ib, doc):
    """docs/DOC/doc.json, or {} for a one-file document."""
    p = os.path.join(ib, 'docs', doc, 'doc.json')
    return json.load(open(p)) if os.path.exists(p) else {}


def copy(ib, doc, work):
    """Put the document into `work` (main.tex, and a book's whole tree)."""
    src = os.path.join(ib, 'docs', doc)
    if spec(ib, doc):
        shutil.copytree(src, work, dirs_exist_ok=True, ignore=shutil.ignore_patterns('doc.json'))
    else:
        shutil.copy(os.path.join(src, 'main.tex'), os.path.join(work, 'main.tex'))


def edit_args(ib, doc):
    """dl3-keys' arguments for an in-body keystroke: the chapter it goes into."""
    s = spec(ib, doc)
    return ['--edit', s['edit']] if s.get('edit') else []


def edit_file(ib, doc):
    """The file in-body edits go into, relative to the document's root."""
    return spec(ib, doc).get('edit') or 'main.tex'
