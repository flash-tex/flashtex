#!/usr/bin/env python3
"""Third-party licence texts and NOTICE files for flashtex-typst-host.

DESIGN.md §15.8 (TY7): the Typst host links the Apache-2.0 `typst` crates
and ~300 other crates. Distributing it requires (Apache-2.0 §4) the licence
texts, the upstream NOTICE files reproduced, and no unknown licence. The
texts are generated from the pinned dependency graph (Cargo.lock, every crate
checksum-verified by cargo), never vendored:

  third_party.py --check              fail on any problem (Typst CI)
  third_party.py --write DIR          also write DIR/THIRD-PARTY-LICENSES.txt
                                      and DIR/THIRD-PARTY-NOTICES.txt (the
                                      files a DMG ships; About > Acknowledgements)
  third_party.py --summary            one line per crate: name version licence

A problem is:
  * a crate whose licence expression has no alternative in ALLOWED;
  * a crate that ships no licence file when its chosen licence needs one
    (the standard text is used only where it carries no copyright line to
    lose: Apache-2.0, and the public-domain-style 0BSD/CC0-1.0/Unlicense);
  * flashtex-engine (GPL, DESIGN.md §3) anywhere in the graph;
  * the pinned upstream typst/typst NOTICE (the published crates do not ship
    it) missing or not matching its SHA-256.

The graph is every package reachable from flashtex-typst-host through normal
and build dependencies on any platform (a superset of what one binary links).
Needs cargo and python3; the typst/typst NOTICE is fetched once over HTTPS
(or read from $TYPST_NOTICE_FILE) and verified.
"""

import hashlib
import json
import os
import re
import subprocess
import sys
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
HOST = os.path.dirname(HERE)

# Permissive licences the MIT app may ship alongside. MPL-2.0 is file-level
# copyleft: allowed for unmodified crates whose source stays available on
# crates.io (the §3 legal review covers it); the summary flags it.
ALLOWED = {
    "MIT", "MIT-0", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "BSD-2-Clause", "BSD-3-Clause",
    "ISC", "Zlib", "0BSD", "Unicode-3.0", "Unlicense", "CC0-1.0", "BSL-1.0", "MPL-2.0",
}
# Preference when an OR offers several (the first allowed one is recorded).
PREFER = ["MIT", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "BSD-3-Clause", "BSD-2-Clause", "ISC", "Zlib",
          "0BSD", "MIT-0", "Unicode-3.0", "BSL-1.0", "Unlicense", "CC0-1.0", "MPL-2.0"]
# Licences whose standard text may stand in when a crate ships none.
STANDARD_TEXT = {
    "Apache-2.0": "texts/Apache-2.0.txt",
    "Apache-2.0 WITH LLVM-exception": "texts/Apache-2.0.txt",
    "0BSD": None, "CC0-1.0": None, "Unlicense": None,  # no notice required
}
LICENSE_FILE = re.compile(r"^(LICEN[CS]E|COPYING|COPYRIGHT|UNLICENSE)", re.I)
NOTICE_FILE = re.compile(r"^NOTICE", re.I)

# typst/typst's NOTICE at tag v0.15.1 (commit 9dfd3a08), 398 lines: the
# published crates do not include it, so it is pinned here.
TYPST_NOTICE_URL = "https://raw.githubusercontent.com/typst/typst/9dfd3a08500b7896045f907433cf7b4b02434fad/NOTICE"
TYPST_NOTICE_SHA256 = "1778244777547c281b6f5fa9fc0c18ab21f8d4491c803f64e09046800f5fcb26"

FORBIDDEN_PACKAGES = {"flashtex-engine"}


def spdx_alternatives(expr):
    """The sets of licences that satisfy `expr` (OR of ANDs). `/` is the
    legacy spelling of OR; `WITH` binds to its licence."""
    expr = expr.replace("/", " OR ")
    toks = re.findall(r"\(|\)|[A-Za-z0-9.+-]+", expr)
    pos = 0

    def atom():
        nonlocal pos
        t = toks[pos]
        pos += 1
        if t == "(":
            r = disj()
            assert toks[pos] == ")", expr
            pos += 1
            return r
        if pos < len(toks) and toks[pos] == "WITH":
            t = "%s WITH %s" % (t, toks[pos + 1])
            pos += 2
        return [[t]]

    def conj():
        nonlocal pos
        r = atom()
        while pos < len(toks) and toks[pos] == "AND":
            pos += 1
            rhs = atom()
            r = [a + b for a in r for b in rhs]
        return r

    def disj():
        nonlocal pos
        r = conj()
        while pos < len(toks) and toks[pos] == "OR":
            pos += 1
            r = r + conj()
        return r

    out = disj()
    assert pos == len(toks), expr
    return out


def choose(expr):
    alts = [a for a in spdx_alternatives(expr) if all(l in ALLOWED for l in a)]
    if not alts:
        return None
    return min(alts, key=lambda a: max(PREFER.index(l) for l in a))


def graph():
    out = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--manifest-path", os.path.join(HOST, "Cargo.toml")],
        check=True, capture_output=True, text=True).stdout
    meta = json.loads(out)
    pkgs = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    root = next(i for i in meta["workspace_members"] if pkgs[i]["name"] == "flashtex-typst-host")
    seen, stack = {root}, [root]
    while stack:
        cur = stack.pop()
        for d in nodes[cur]["deps"]:
            kinds = {k["kind"] for k in d["dep_kinds"]}
            if kinds & {None, "build"} and d["pkg"] not in seen:
                seen.add(d["pkg"])
                stack.append(d["pkg"])
    return [pkgs[i] for i in sorted(seen, key=lambda i: (pkgs[i]["name"], pkgs[i]["version"]))], root


def typst_notice():
    path = os.environ.get("TYPST_NOTICE_FILE")
    if path:
        data = open(path, "rb").read()
    else:
        with urllib.request.urlopen(TYPST_NOTICE_URL, timeout=60) as r:
            data = r.read()
    got = hashlib.sha256(data).hexdigest()
    if got != TYPST_NOTICE_SHA256:
        raise SystemExit("typst/typst NOTICE: sha256 %s, expected %s" % (got, TYPST_NOTICE_SHA256))
    return data.decode("utf-8")


def main(argv):
    write_dir = None
    if "--write" in argv:
        write_dir = argv[argv.index("--write") + 1]
    problems, rows, licences, notices = [], [], [], []
    pkgs, root = graph()
    for p in pkgs:
        name, ver = p["name"], p["version"]
        if name in FORBIDDEN_PACKAGES:
            problems.append("%s %s is in the graph (GPL, DESIGN.md §3)" % (name, ver))
            continue
        if p["id"] == root or p.get("source") is None:
            continue  # our own MIT code (the repository's LICENSE)
        expr = p.get("license") or ""
        chosen = choose(expr) if expr else None
        if chosen is None:
            problems.append("%s %s: licence %r has no allowed alternative" % (name, ver, expr or p.get("license_file")))
            continue
        d = os.path.dirname(p["manifest_path"])
        files = sorted(f for f in os.listdir(d) if os.path.isfile(os.path.join(d, f)))
        lic_files = [f for f in files if LICENSE_FILE.match(f)]
        texts = [(f, open(os.path.join(d, f), encoding="utf-8", errors="replace").read()) for f in lic_files]
        how = "shipped"
        if not texts:
            # Without the crate's own files, take an alternative whose
            # standard text loses nothing (e.g. Apache-2.0 of "MIT OR Apache-2.0").
            covered = [a for a in spdx_alternatives(expr) if all(l in STANDARD_TEXT for l in a)]
            if covered:
                chosen = min(covered, key=lambda a: max(PREFER.index(l) for l in a))
            missing = [l for l in chosen if l not in STANDARD_TEXT]
            if missing:
                problems.append("%s %s: ships no licence file and %s needs its copyright notice" % (name, ver, " AND ".join(missing)))
                continue
            how = "standard text (the crate ships none)"
            for l in chosen:
                if STANDARD_TEXT[l]:
                    texts.append((l, open(os.path.join(HERE, STANDARD_TEXT[l]), encoding="utf-8").read()))
        for f in files:
            if NOTICE_FILE.match(f):
                notices.append(("%s %s (%s)" % (name, ver, f), open(os.path.join(d, f), encoding="utf-8").read()))
        rows.append((name, ver, expr, " AND ".join(chosen), how))
        licences.append((name, ver, expr, texts))

    try:
        notices.insert(0, ("typst/typst v0.15.1 (NOTICE, %s)" % TYPST_NOTICE_URL, typst_notice()))
    except Exception as e:  # network or hash
        problems.append("typst/typst NOTICE: %s" % e)

    if "--summary" in argv:
        for r in rows:
            print("%-28s %-10s %-50s -> %s%s" % (r[0], r[1], r[2], r[3], "" if r[4] == "shipped" else "  [" + r[4] + "]"))
        mpl = [r[0] for r in rows if "MPL-2.0" in r[3]]
        if mpl:
            print("MPL-2.0 (file-level copyleft, unmodified, source on crates.io): %s" % ", ".join(mpl))

    if write_dir:
        os.makedirs(write_dir, exist_ok=True)
        with open(os.path.join(write_dir, "THIRD-PARTY-LICENSES.txt"), "w", encoding="utf-8") as fh:
            fh.write("flashtex-typst-host links the following crates (Cargo.lock, pinned by checksum).\n\n")
            seen_text = {}
            for name, ver, expr, texts in licences:
                fh.write("=" * 78 + "\n%s %s -- %s\n" % (name, ver, expr))
                for f, t in texts:
                    h = hashlib.sha256(t.encode()).hexdigest()
                    if h in seen_text:
                        fh.write("  %s: identical to the text given for %s\n" % (f, seen_text[h]))
                    else:
                        seen_text[h] = "%s %s (%s)" % (name, ver, f)
                        fh.write("--- %s ---\n%s\n" % (f, t.rstrip("\n")))
                fh.write("\n")
        with open(os.path.join(write_dir, "THIRD-PARTY-NOTICES.txt"), "w", encoding="utf-8") as fh:
            fh.write("NOTICE files of the Apache-2.0 components flashtex-typst-host links (Apache-2.0 §4(d)).\n\n")
            for src, t in notices:
                fh.write("=" * 78 + "\n%s\n%s\n\n" % (src, "-" * 78) + t.rstrip("\n") + "\n\n")

    print("third-party: %d crates, %d NOTICE files, %d problem(s)" % (len(rows), len(notices), len(problems)))
    for pr in problems:
        print("PROBLEM  " + pr, file=sys.stderr)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
