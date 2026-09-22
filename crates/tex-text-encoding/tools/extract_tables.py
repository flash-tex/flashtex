#!/usr/bin/env python3
"""Extract KC-105 data tables from the real TeX Live sources into src/generated.rs.

Sources (located with kpsewhich; versions are recorded in the output header):
  * tex/latex/base/{omsenc,ot1enc,t1enc,ts1enc}.dfu  -- loaded by the pdflatex format in
    that order (verified in texmf-var/web2c/pdftex/pdflatex.log), then utf8.def's own
    \\DeclareUnicodeCharacter lines (utf8.def:360-370).  Last declaration wins.
  * tex/latex/base/{ot1enc,t1enc,ts1enc,omsenc,omlenc}.def -- \\DeclareText* per encoding.
  * tex/latex/base/latex.ltx -- \\DeclareText*Default and \\UndeclareTextCommand, in order.
  * fonts/enc/dvips/{lm,cm-super}/*.enc and the builtin encodings of cm*.pfb -- glyph names.
Oracle/tooling only; cargo never runs this.

    python3 crates/tex-text-encoding/tools/extract_tables.py [--check]
"""
import argparse
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(os.path.dirname(HERE), "src", "generated.rs")


def kpse(name):
    return subprocess.run(["kpsewhich", name], capture_output=True, text=True).stdout.strip()


def read(name):
    with open(kpse(name), encoding="latin-1") as fh:
        return fh.read()


def strip_comments(text):
    out = []
    for line in text.split("\n"):
        # TeX comment: unescaped %.
        m = re.search(r"(?<!\\)%", line)
        out.append(line[: m.start()] if m else line)
    return "\n".join(out)


def version(text, name):
    m = re.search(r"\\ProvidesFile\{" + re.escape(name) + r"\}\s*\[([^\]]*)\]", text)
    return re.sub(r"\s+", " ", m.group(1)).strip() if m else "?"


def braced(text, i):
    """text[i] == '{' -> (content, index after closing brace)."""
    assert text[i] == "{", text[i:i + 40]
    depth = 0
    j = i
    while True:
        c = text[j]
        if c == "\\":
            j += 2
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return text[i + 1:j], j + 1
        j += 1


def skip_ws(text, i):
    while i < len(text) and text[i] in " \t\n":
        i += 1
    return i


def arg(text, i):
    """A macro argument: braced group or single token."""
    i = skip_ws(text, i)
    if text[i] == "{":
        return braced(text, i)
    if text[i] == "\\":
        m = re.match(r"\\([A-Za-z@]+|.)", text[i:])
        return m.group(0), i + m.end()
    return text[i], i + 1


def rs(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", " ") + '"'


def slot_value(s):
    s = s.strip()
    if s.startswith("`"):
        c = s[1:]
        return ord(c[-1]) if c.startswith("\\") else ord(c)
    if s.startswith("'"):
        return int(s[1:], 8)
    if s.startswith('"'):
        return int(s[1:], 16)
    return int(s)


def unicode_table():
    entries = {}
    sources = []
    for f in ["omsenc.dfu", "ot1enc.dfu", "t1enc.dfu", "ts1enc.dfu"]:
        text = read(f)
        sources.append((f, version(text, f)))
        parse_dfu(strip_comments(text), f, entries)
    utf8 = strip_comments(read("utf8.def"))
    sources.append(("utf8.def", version(read("utf8.def"), "utf8.def")))
    parse_dfu(utf8, "utf8.def", entries)
    return entries, sources


def parse_dfu(text, fname, entries):
    for m in re.finditer(r"\\DeclareUnicodeCharacter\{([0-9A-Fa-f]+)\}", text):
        body, _ = braced(text, skip_ws(text, m.end()))
        cp = int(m.group(1), 16)
        prev = entries.get(cp)
        files = (prev[1] + [fname]) if prev else [fname]
        entries[cp] = (body.strip(), files)


def encoding_decls(fname):
    text = strip_comments(read(fname))
    decls = []
    pat = re.compile(r"\\(DeclareTextSymbol|DeclareTextAccent|DeclareTextCommand|DeclareTextComposite|DeclareTextCompositeCommand)(?![A-Za-z])")
    for m in pat.finditer(text):
        kind = m.group(1)
        i = m.end()
        cmd, i = arg(text, i)
        cmd = cmd.strip()
        enc, i = arg(text, i)
        if kind in ("DeclareTextSymbol", "DeclareTextAccent"):
            slot, i = arg(text, i)
            decls.append((kind, cmd, enc, None, str(slot_value(slot))))
        elif kind == "DeclareTextCommand":
            j = skip_ws(text, i)
            nargs = 0
            if text[j] == "[":
                k = text.index("]", j)
                nargs = int(text[j + 1:k])
                j = k + 1
            body, i = arg(text, j)
            decls.append((kind, cmd, enc, str(nargs), body))
        elif kind == "DeclareTextComposite":
            base, i = arg(text, i)
            slot, i = arg(text, i)
            decls.append((kind, cmd, enc, base.strip(), str(slot_value(slot))))
        else:
            base, i = arg(text, i)
            body, i = arg(text, i)
            decls.append((kind, cmd, enc, base.strip(), body))
    return decls, version(read(fname), fname)


def kernel_defaults():
    text = read("latex.ltx")
    lines = text.split("\n")
    out = []
    skipping = False
    in_legacy = False
    for n, line in enumerate(lines, 1):
        code = strip_comments(line)
        # latex.ltx: \def\UseLegacyTextSymbols{...} is only a user-callable macro body.
        if code.startswith(r"\def\UseLegacyTextSymbols"):
            in_legacy = True
            continue
        if in_legacy:
            if code.strip() == "}":
                in_legacy = False
            continue
        # latex.ltx:14453 \ifx\Umathcode\@undefined ... \else (Unicode engines) ... \fi
        if re.match(r"\\ifx\\Umathcode\\@undefined", code.strip()):
            continue
        if code.strip() == r"\else" and any(r"\capital" in l for l in lines[n - 3:n]):
            skipping = True
            continue
        if skipping:
            if code.strip() == r"\fi":
                skipping = False
            continue
        m = re.match(r"\s*\\(DeclareTextSymbolDefault|DeclareTextAccentDefault)\{(\\[^}]+)\}\{([A-Z0-9]+)\}", code)
        if m:
            out.append((n, m.group(1), m.group(2), m.group(3)))
            continue
        m = re.match(r"\s*\\UndeclareTextCommand\{(\\[^}]+)\}\{([A-Z0-9]+)\}", code)
        if m:
            out.append((n, "UndeclareTextCommand", m.group(1), m.group(2)))
            continue
        m = re.match(r"\s*\\DeclareTextCommandDefault\s*\{?(\\[A-Za-z@]+)\}?", code)
        if m:
            joined = "\n".join(strip_comments(l) for l in lines[n - 1:n + 12])
            k = joined.index(m.group(1)) + len(m.group(1))
            k = skip_ws(joined, k)
            if joined[k] == "}":
                k = skip_ws(joined, k + 1)
            if joined[k] == "[":
                k = joined.index("]", k) + 1
            k = skip_ws(joined, k)
            body, _ = braced(joined, k) if joined[k] == "{" else ("", k)
            out.append((n, "DeclareTextCommandDefault", m.group(1), re.sub(r"\s+", " ", body).strip()))
    return out


def enc_vector(name):
    text = strip_comments(read(name))
    names = re.findall(r"/([^\s\[\]/{}]+)", text)
    return names[1:257]


def builtin_vector(pfb):
    with open(kpse(pfb), "rb") as fh:
        head = fh.read(30000).decode("latin-1")
    vec = [".notdef"] * 256
    for m in re.finditer(r"dup (\d+) /(\S+) put", head):
        vec[int(m.group(1))] = m.group(2)
    return vec


def write_or_check(path, text, check):
    """Write `text` to `path`, or with `--check` compare instead.

    `--check` exit codes (the same for every generator; see
    scripts/check-generated.py): 0 the committed file is what this generator
    produces now, 1 it is stale (a diff excerpt is printed), 2 it cannot be
    checked here (a tool or input is missing; the message says which).
    """
    data = text.encode("utf-8")
    rel = os.path.relpath(path)
    if not check:
        with open(path, "wb") as fh:
            fh.write(data)
        return 0
    try:
        with open(path, "rb") as fh:
            old = fh.read()
    except FileNotFoundError:
        print("STALE: %s does not exist" % rel)
        return 1
    if old == data:
        print("up to date: %s" % rel)
        return 0
    import difflib
    diff = list(difflib.unified_diff(old.decode("utf-8", "replace").splitlines(),
                                     text.splitlines(), "committed", "regenerated",
                                     lineterm="", n=0))
    for line in diff[:40]:
        print(line)
    if len(diff) > 40:
        print("... %d more diff lines" % (len(diff) - 40))
    print("STALE: %s differs from what %s generates now" % (rel, os.path.relpath(sys.argv[0])))
    return 1


def require_tools(*tools):
    """Exit 2 (cannot check here) when a TeX tool is not on PATH."""
    import shutil
    missing = [t for t in tools if not shutil.which(t)]
    if missing:
        print("CANNOT CHECK: %s not found on PATH (needs TeX Live 2026)" % ", ".join(missing))
        sys.exit(2)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true",
                    help="compare with src/generated.rs instead of writing it")
    args = ap.parse_args()
    if args.check:
        require_tools("kpsewhich")
    uni, usources = unicode_table()
    out = []
    w = out.append
    w("// @generated by crates/tex-text-encoding/tools/extract_tables.py -- do not edit.")
    w("// Data copied from TeX Live 2026 sources (LPPL / GUST / public TFM-encoding data);")
    w("// file versions are recorded next to each table.")
    w("#![allow(clippy::all)]")
    w("")
    w("/// UTF-8 input declarations visible in a default pdflatex document, in load order.")
    for f, v in usources:
        w("/// - `%s` [%s]" % (f, v))
    w("/// `(code point, expansion, files that declared it (last wins))`.")
    w("pub static UNICODE_DECLARATIONS: &[(u32, &str, &[&str])] = &[")
    for cp in sorted(uni):
        body, files = uni[cp]
        w("    (0x%04X, %s, &[%s])," % (cp, rs(body), ", ".join(rs(f) for f in files)))
    w("];")
    w("")
    w("/// Kinds of LaTeX text-command declarations (`\\DeclareText*`).")
    w("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    w("pub enum DeclKind { Symbol, Accent, Command, Composite, CompositeCommand }")
    w("")
    w("/// `(kind, command, encoding, extra (argument count or composite base), payload (slot or body))`.")
    w("pub type Decl = (DeclKind, &'static str, &'static str, &'static str, &'static str);")
    kinds = {"DeclareTextSymbol": "Symbol", "DeclareTextAccent": "Accent", "DeclareTextCommand": "Command",
             "DeclareTextComposite": "Composite", "DeclareTextCompositeCommand": "CompositeCommand"}
    for f in ["ot1enc.def", "t1enc.def", "ts1enc.def", "omsenc.def", "omlenc.def"]:
        decls, ver = encoding_decls(f)
        const = f.split(".")[0].upper()
        w("/// `%s` [%s]" % (f, ver))
        w("pub static %s: &[Decl] = &[" % const)
        for kind, cmd, enc, extra, payload in decls:
            w("    (DeclKind::%s, %s, %s, %s, %s)," % (kinds[kind], rs(cmd), rs(enc), rs(extra or ""), rs(payload)))
        w("];")
        w("")
    w("/// `latex.ltx` %s: text-command defaults in file order `(line, directive, command, encoding or body)`." % version(read("latex.ltx"), "latex.ltx"))
    w("pub static KERNEL_TEXT_DEFAULTS: &[(u32, &str, &str, &str)] = &[")
    for n, kind, cmd, extra in kernel_defaults():
        w("    (%d, %s, %s, %s)," % (n, rs(kind), rs(cmd), rs(extra)))
    w("];")
    w("")
    vectors = [
        ("LM_RM", "lm-rm.enc"), ("LM_EC", "lm-ec.enc"), ("LM_TS1", "lm-ts1.enc"),
        ("LM_MATHSY", "lm-mathsy.enc"), ("LM_MATHIT", "lm-mathit.enc"),
        ("CM_SUPER_T1", "cm-super-t1.enc"), ("CM_SUPER_TS1", "cm-super-ts1.enc"),
    ]
    for const, f in vectors:
        vec = enc_vector(f)
        assert len(vec) == 256, (f, len(vec))
        w("/// Glyph names, `%s` (%s)." % (f, kpse(f).replace("/usr/local/texlive/2026/", "TL2026:")))
        w("pub static ENC_%s: [&str; 256] = [%s];" % (const, ", ".join(rs(n) for n in vec)))
        w("")
    for const, f in [("CMR10", "cmr10.pfb"), ("CMSY10", "cmsy10.pfb"), ("CMMI10", "cmmi10.pfb")]:
        vec = builtin_vector(f)
        w("/// Builtin Type 1 encoding of `%s` (no /Differences written by pdfTeX)." % f)
        w("pub static BUILTIN_%s: [&str; 256] = [%s];" % (const, ", ".join(rs(n) for n in vec)))
        w("")
    rc = write_or_check(OUT, "\n".join(out), args.check)
    if not args.check:
        print("wrote", OUT, "unicode", len(uni))
    return rc


if __name__ == "__main__":
    sys.exit(main())
