#!/usr/bin/env python3
"""Unit tests of tools/bundle (standard library): the licence check, the
lock reader, and fetch_image.py's refusals of members that would leave the
tree. `python3 tools/bundle/test_texbundle.py`"""

import io
import os
import subprocess
import sys
import tarfile
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import fetch_image  # noqa: E402
import texbundle  # noqa: E402

TLPDB = """name freepkg
catalogue-license lppl1.3c
runfiles size=1
 RELOC/tex/latex/freepkg/freepkg.sty

name dual
catalogue-license ofl apache2 lppl1.3

name sellnot
catalogue-license nosell

name share
catalogue-license shareware

name otherbad
catalogue-license other-nonfree

name nosrc
catalogue-license nosource

name ncpkg
catalogue-license cc-by-nc-4

name nolic
runfiles size=0

name hyphen-welsh
runfiles size=0

name latexconfig
runfiles size=0
"""


class Licences(unittest.TestCase):
    def setUp(self):
        self.db = texbundle.parse_tlpdb(TLPDB)

    def test_relocated_runfiles(self):
        self.assertEqual(self.db["freepkg"]["runfiles"], ["texmf-dist/tex/latex/freepkg/freepkg.sty"])

    def test_free_and_allowlisted_pass(self):
        self.assertEqual(texbundle.licence_problems(self.db, ["freepkg", "dual", "hyphen-welsh", "latexconfig"]), [])

    def test_nonfree_and_unrecorded_fail(self):
        names = ["sellnot", "share", "otherbad", "nosrc", "ncpkg", "nolic"]
        bad = texbundle.licence_problems(self.db, names)
        self.assertEqual([b.split(":")[0] for b in bad], names)

    def test_every_nonfree_value_fails(self):
        for v in sorted(texbundle.NONFREE):
            db = texbundle.parse_tlpdb(f"name p\ncatalogue-license lppl1.3c {v}\n")
            self.assertEqual(len(texbundle.licence_problems(db, ["p"])), 1, v)


class Lock(unittest.TestCase):
    def test_shipped_lock(self):
        url, digest = texbundle.read_lock(texbundle.LOCK)
        repo, tag, asset = texbundle.release_of(url)
        self.assertEqual(repo, "flash-tex/flashtex")
        self.assertTrue(tag.startswith("texbundle-tl2026-"))
        self.assertTrue(asset.endswith(".ttb"))
        self.assertEqual(len(digest), 64)


def layer(path, members):
    """A gzipped layer of (name, kind, data-or-linkname) members."""
    with tarfile.open(path, "w:gz") as t:
        for name, kind, x in members:
            ti = tarfile.TarInfo(name)
            if kind == "dir":
                ti.type = tarfile.DIRTYPE
                t.addfile(ti)
            elif kind == "file":
                ti.size = len(x)
                t.addfile(ti, io.BytesIO(x))
            elif kind == "sym":
                ti.type = tarfile.SYMTYPE
                ti.linkname = x
                t.addfile(ti)
            elif kind == "hard":
                ti.type = tarfile.LNKTYPE
                ti.linkname = x
                t.addfile(ti)


class Extraction(unittest.TestCase):
    K = "usr/local/texlive/"

    def run_layer(self, members):
        """Apply a layer in a child process; (exit status, dest)."""
        d = tempfile.mkdtemp(prefix="fetch-image-test-")
        lp = os.path.join(d, "l.tgz")
        layer(lp, members)
        dest = os.path.join(d, "dest")
        os.makedirs(dest)
        code = ("import sys; sys.path.insert(0, %r); import fetch_image; fetch_image.apply_layer(%r, %r)"
                % (HERE, lp, dest))
        r = subprocess.run([sys.executable, "-c", code], capture_output=True, text=True)
        return r.returncode, dest, d

    def test_plain_tree_and_relative_links(self):
        rc, dest, _ = self.run_layer([
            (self.K + "2026/bin/x/pdftex", "file", b"elf"),
            (self.K + "2026/bin/x/pdflatex", "sym", "pdftex"),
            (self.K + "2026/bin/x/latexmk", "sym", "../../texmf-dist/scripts/latexmk.pl"),
            (self.K + "2026/bin/x/etex", "hard", self.K + "2026/bin/x/pdftex"),
            ("etc/passwd", "file", b"ignored: outside usr/local/texlive"),
        ])
        self.assertEqual(rc, 0)
        self.assertEqual(os.readlink(os.path.join(dest, "2026/bin/x/pdflatex")), "pdftex")
        self.assertFalse(os.path.exists(os.path.join(dest, "etc")))

    def test_refusals(self):
        cases = {
            "absolute symlink": [(self.K + "2026/x", "sym", "/etc/passwd")],
            "escaping symlink": [(self.K + "2026/x", "sym", "../../../../etc/passwd")],
            "dotdot member": [(self.K + "2026/../../x", "file", b"x")],
            "hard link outside": [(self.K + "2026/x", "hard", "etc/passwd")],
            "through a symlinked dir": [(self.K + "2026/d", "sym", "."),
                                        (self.K + "2026/out", "sym", "d/../.."),
                                        (self.K + "2026/out/evil", "file", b"x")],
        }
        # The reviewer's case: each link is inside when laid down, but the
        # second redirects the first (s = a/b/x/../.. becomes dest/..): the
        # final walk refuses it and removes the tree.
        d = tempfile.mkdtemp(prefix="fetch-image-test-")
        dest = os.path.join(d, "dest")
        os.makedirs(dest)
        lp = os.path.join(d, "l.tgz")
        layer(lp, [(self.K + "a/b/s", "sym", "x/../.."), (self.K + "a/b/x", "sym", "../..")])
        fetch_image.apply_layer(lp, dest)  # each link alone passes
        code = ("import sys; sys.path.insert(0, %r); import fetch_image; fetch_image.check_links(%r)" % (HERE, dest))
        r = subprocess.run([sys.executable, "-c", code], capture_output=True, text=True)
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("a/b/s", r.stderr)
        self.assertFalse(os.path.exists(dest), "the tree is removed")
        # A tree whose links stay inside passes the walk.
        ok = os.path.join(d, "ok")
        os.makedirs(os.path.join(ok, "bin"))
        os.symlink("../bin", os.path.join(ok, "bin", "up"))
        fetch_image.check_links(ok)
        # The symlink itself is refused (it resolves out), not only the file.
        rc, dest, _ = self.run_layer(cases["through a symlinked dir"][:2])
        self.assertNotEqual(rc, 0)
        for what, members in cases.items():
            rc, _, d = self.run_layer(members)
            self.assertNotEqual(rc, 0, what)
            self.assertFalse(os.path.exists(os.path.join(d, "evil")), what)


if __name__ == "__main__":
    unittest.main()
