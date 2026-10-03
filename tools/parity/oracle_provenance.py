#!/usr/bin/env python3
"""Which oracle a board was measured against (DESIGN §8: oracle provenance).

The NixOS PC and the self-hosted Macs both run pdfTeX 1.40.29 from "TeX Live
2026", so the version string alone cannot tell their oracles apart, yet their
trees are different snapshots (the templates manifest, pinned on a Mac, has 7
of its 20 files missing or different on the PC: nightly.yml). This prints the
oracle's identity down to the package database, so a board says which
snapshot made its expected data and two boards are never compared silently
across oracles.

Usage: oracle_provenance.py [--texbin DIR] [--markdown FILE] [--json FILE]
Prints one line (for a board's --host-label); --markdown/--json write the
details. Portable: no /proc, no GNU tools.
"""
import argparse
import json
import os
import platform
import re
import shutil
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from nightly import oracle_identity  # noqa: E402  (the T4 job's own identity)


def tlpdb_fields(path):
    """The release and the revisions of the packages that make the expected data."""
    want = {"pdftex", "latex", "l3kernel", "latex-bin"}
    out, name = {}, None
    try:
        with open(path, encoding="utf-8", errors="replace") as f:
            for line in f:
                line = line.rstrip("\n")
                if line.startswith("name "):
                    name = line[5:]
                elif name == "00texlive.config" and line.startswith("depend release/"):
                    out["release"] = line.split("/", 1)[1]
                elif name in want and line.startswith("revision "):
                    out[name] = line.split()[1]
    except OSError:
        pass
    return out


def latex_format_version(texbin):
    kpse = os.path.join(texbin, "kpsewhich")
    try:
        ltx = subprocess.run([kpse, "latex.ltx"], capture_output=True, text=True, timeout=60).stdout.strip()
        with open(ltx, encoding="utf-8", errors="replace") as f:
            m = re.search(r"\\edef\\fmtversion\s*\{([^}]*)\}", f.read())
        return m.group(1).strip() if m else None
    except (OSError, subprocess.SubprocessError):
        return None


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--texbin", help="TeX Live's bin directory (default: the pdftex on PATH)")
    ap.add_argument("--markdown")
    ap.add_argument("--json")
    a = ap.parse_args()
    pdftex = os.path.join(a.texbin, "pdftex") if a.texbin else shutil.which("pdftex")
    if not pdftex:
        sys.exit("oracle_provenance.py: no pdftex on PATH")
    texbin = os.path.dirname(pdftex)
    ident = oracle_identity(texbin, pdftex)
    version = subprocess.run([pdftex, "--version"], capture_output=True, text=True).stdout.split("\n")[0]
    tl = tlpdb_fields(os.path.join(ident["texlive_root"], "tlpkg", "texlive.tlpdb"))
    rec = dict(ident, version=version, latex_format=latex_format_version(texbin), tlpdb=tl,
               platform="%s %s %s" % (platform.system(), platform.release(), platform.machine()))
    short = (rec["tlpdb_sha256"] or "none")[:12]
    line = "oracle %s, LaTeX %s, tlpdb %s (%s)" % (
        version, rec["latex_format"] or "?", short, rec["texlive_root"])
    if a.json:
        with open(a.json, "w") as f:
            json.dump(rec, f, indent=1, sort_keys=True)
    if a.markdown:
        rows = [("pdfTeX", version), ("platform", rec["platform"]),
                ("TeX Live root", rec["texlive_root"]), ("TeX Live release", tl.get("release")),
                ("texlive.tlpdb sha256", rec["tlpdb_sha256"]),
                ("LaTeX format (latex.ltx)", rec["latex_format"]),
                ("tlpdb revisions", ", ".join("%s r%s" % (k, tl[k]) for k in sorted(tl) if k != "release")),
                ("pdftex binary sha256", rec["pdftex_sha256"])]
        with open(a.markdown, "w") as f:
            f.write("### Oracle provenance\n\n"
                    "Expected data comes from this TeX Live only. A board measured against another "
                    "texlive.tlpdb (another host, or a `tlmgr update`) is a different oracle: "
                    "compare boards only when this table matches.\n\n| | |\n|---|---|\n")
            for k, v in rows:
                f.write("| %s | `%s` |\n" % (k, v if v else "unknown"))
    print(line)


if __name__ == "__main__":
    main()
