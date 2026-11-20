#!/usr/bin/env python3
"""The fontconfig set-up under which xelatex (the oracle) finds the same
font files by name as the XeTeX port.

    python3 tools/unicode-m3/fontconfig.py --texlive TL --out DIR >> "$GITHUB_ENV"

writes DIR/fonts.conf and prints FONTCONFIG_FILE=, FLASHTEX_FONT_DIRS= and
FLASHTEX_CACHE_DIR= lines. Both engines see TeX Live's OpenType and TrueType
trees and the system's fonts (/etc/fonts/fonts.conf's directories).

Where a font file's name is in both, the port takes TeX Live's copy: it
searches FLASHTEX_FONT_DIRS before the system's directories, and XeTeX's
font manager keeps the first face it is handed for a name
(XeTeXFontMgr::addToMaps). xelatex is handed fontconfig's FcFontList, whose
order is not the directories': FcFontSetList (fclist.c) puts the matches in
a table of 4099 buckets by a hash of their values, the file name among
them, and returns them bucket by bucket. On
ubuntu-latest it lists /usr/share/fonts' DejaVuSansMono-Bold.ttf before
TeX Live's, with TeX Live's directories given first or last (m3-diag,
2026-10-12). The polyglossia manual then loaded DejaVu Sans Mono from
different files in the two engines: every XDV record was equal, but the
longer path moved every later byte, and page 7's pointer to page 6 differed.

So the system's copy of each such file is rejected (<rejectfont>), and a
name has one file in both engines. Only files whose base name TeX Live also
ships are rejected; every other system font stays visible to both.
"""
import argparse
import os
import sys
from xml.sax.saxutils import escape

FONT_EXT = (".otf", ".ttf", ".ttc", ".otc")


def files_under(d):
    out = []
    for root, _dirs, names in os.walk(d):
        for n in names:
            if n.lower().endswith(FONT_EXT):
                out.append(os.path.join(root, n))
    return sorted(out)


def port_dirs():
    """The system directories the port scans by itself on Linux
    (font-discovery's `default_dirs`): fontconfig's defaults on Debian/Ubuntu."""
    home = os.path.expanduser("~")
    return ["/usr/share/fonts", "/usr/local/share/fonts",
            os.path.join(home, ".fonts"), os.path.join(home, ".local/share/fonts")]


def fontconfig_dirs():
    """The directories of the fonts the system's own fontconfig lists (fc-list
    under /etc/fonts/fonts.conf), which xelatex sees through the <include>.
    Elsewhere than Debian/Ubuntu they need not be under port_dirs(): on NixOS
    every one is in /nix/store (#2525)."""
    import subprocess
    env = {k: v for k, v in os.environ.items() if k != "FONTCONFIG_FILE"}
    try:
        out = subprocess.run(["fc-list", "-f", "%{file}\n"], env=env, capture_output=True,
                             text=True, timeout=120).stdout
    except (OSError, subprocess.SubprocessError) as e:
        print("fontconfig.py: warning: fc-list failed (%s); only the port's own system "
              "directories are checked for duplicates" % e, file=sys.stderr)
        return []
    return sorted({os.path.dirname(f) for f in out.splitlines() if os.path.isabs(f)})


def outside(dirs, roots):
    """The directories of `dirs` under none of `roots` (compared both as given
    and with symbolic links resolved)."""
    def under(d, r):
        return d == r or d.startswith(r.rstrip("/") + "/")

    def forms(p):
        return {p, os.path.realpath(p)}
    return [d for d in dirs
            if not any(under(a, b) for r in roots for a in forms(d) for b in forms(r))]


def system_dirs(tl_dirs=()):
    """Every system font directory xelatex can see: the port's own and
    fontconfig's. A system fontconfig may already include TeX Live's own font
    tree (TeX Live's 09-texlive-fonts.conf set-up); those directories are TeX
    Live's, not the system's, and are left out (REVIEW-5 on #2526)."""
    return port_dirs() + outside(outside(fontconfig_dirs(), port_dirs()), list(tl_dirs))


def config(tl_dirs, cache_dir, sys_dirs):
    tl_names = {os.path.basename(f) for d in tl_dirs for f in files_under(d)}
    # TeX Live's own files are never rejected, even when a system directory
    # given here is TeX Live's tree or contains it.
    rejected = [f for d in sys_dirs for f in files_under(d)
                if os.path.basename(f) in tl_names
                and outside([os.path.dirname(f)], tl_dirs)]
    lines = ['<?xml version="1.0"?>',
             '<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">',
             "<fontconfig>",
             '  <include ignore_missing="yes">/etc/fonts/fonts.conf</include>']
    lines += ["  <dir>%s</dir>" % escape(d) for d in tl_dirs]
    if rejected:
        lines.append("  <!-- TeX Live ships these files too: one file per name -->")
        lines.append("  <selectfont><rejectfont>")
        lines += ["    <glob>%s</glob>" % escape(f) for f in rejected]
        lines.append("  </rejectfont></selectfont>")
    lines += ["  <cachedir>%s</cachedir>" % escape(cache_dir), "</fontconfig>", ""]
    return "\n".join(lines), rejected


def _self_test():
    import tempfile
    with tempfile.TemporaryDirectory() as t:
        tl, sysd = os.path.join(t, "tl", "truetype", "dejavu"), os.path.join(t, "sys", "dejavu")
        os.makedirs(tl)
        os.makedirs(sysd)
        for d, names in ((tl, ["DejaVuSansMono-Bold.ttf"]),
                         (sysd, ["DejaVuSansMono-Bold.ttf", "OnlyHere.ttf", "notes.txt"])):
            for n in names:
                open(os.path.join(d, n), "w").close()
        text, rejected = config([os.path.join(t, "tl")], "/c", [os.path.join(t, "sys")])
        assert rejected == [os.path.join(sysd, "DejaVuSansMono-Bold.ttf")], rejected
        assert "<glob>%s</glob>" % rejected[0] in text
        assert "OnlyHere" not in text
        assert text.index("<include") < text.index("<dir>")
    # NixOS: fontconfig's directories are in /nix/store, not under the port's own
    assert outside(["/nix/store/x-gyre/share/fonts", "/usr/share/fonts/truetype", "/usr/share/fontsx"],
                   ["/usr/share/fonts"]) == ["/nix/store/x-gyre/share/fonts", "/usr/share/fontsx"]


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--texlive", help="TeX Live's root (the one with texmf-dist/)")
    ap.add_argument("--out", help="directory for fonts.conf and the caches")
    ap.add_argument("--self-test", action="store_true")
    a = ap.parse_args(argv)
    _self_test()
    if a.self_test:
        print("fontconfig.py: ok", file=sys.stderr)
        return 0
    if not (a.texlive and a.out):
        ap.error("--texlive and --out are required")
    tl_dirs = [os.path.join(a.texlive, "texmf-dist", "fonts", x) for x in ("opentype", "truetype")]
    os.makedirs(a.out, exist_ok=True)
    sys_dirs = system_dirs(tl_dirs)
    text, rejected = config(tl_dirs, os.path.join(a.out, "fc-cache"), sys_dirs)
    path = os.path.join(a.out, "fonts.conf")
    with open(path, "w") as f:
        f.write(text)
    print("fontconfig.py: %s, %d system files TeX Live also ships rejected" % (path, len(rejected)),
          file=sys.stderr)
    for r in rejected:
        print("  rejected %s" % r, file=sys.stderr)
    print("FONTCONFIG_FILE=%s" % path)
    # The system directories the port does not scan by itself go after TeX
    # Live's, so both engines see the same system fonts (NixOS: #2525).
    print("FLASHTEX_FONT_DIRS=%s" % ":".join(tl_dirs + outside(sys_dirs, port_dirs())))
    print("FLASHTEX_CACHE_DIR=%s" % os.path.join(a.out, "flashtex-cache"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
