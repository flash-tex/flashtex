#!/usr/bin/env python3
"""Extract siunitx unit/prefix tables from the real TeX Live siunitx.sty.

Provenance: the `siunitx.sty` found with kpsewhich (version and date are
recorded in the output header). siunitx v3 declares its built-ins with the
internal `\\siunitx_declare_unit` / `\\siunitx_declare_prefix` /
`\\siunitx_declare_power` functions -- those are what the user-level
`\\DeclareSIUnit` / `\\DeclareSIPrefix` / `\\DeclareSIPower` commands call
(siunitx.sty `\\NewDocumentCommand` block), so parsing the internal form
captures every built-in, including the abbreviated (`\\kg`) and binary
(`\\bit`) units. The public `\\DeclareSIUnit` / `\\DeclareSIPrefix` spellings
are parsed too (they appear in the `.cfg` companions, not in the `.sty`).

Only top-level (column-zero) declarations are taken: the `\\AtBeginDocument`
font-fallback redeclarations (`\\degree`, `\\ohm`, `\\micro`, ...) are
engine-dependent and excluded, as are the `\\cs_new` definitions of the
declare functions themselves (their arguments are `#1`-style parameters,
never concrete `\\names`).
Oracle/tooling only; cargo never runs this.
"""
import glob
import os
import re
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(os.path.dirname(HERE), "src", "siunitx_generated.rs")

KPSE_CANDIDATES = (
    ["kpsewhich", "/Library/TeX/texbin/kpsewhich"]
    + sorted(glob.glob("/usr/local/texlive/*/bin/*/kpsewhich"))
)


def kpse(name):
    for kpsewhich in KPSE_CANDIDATES:
        try:
            out = subprocess.run(
                [kpsewhich, name], capture_output=True, text=True
            ).stdout.strip()
        except OSError:
            continue
        if out:
            return out
    raise SystemExit("kpsewhich not found and %s unreachable" % name)


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


def group_arg(text, i):
    i = skip_ws(text, i)
    return braced(text, i)


def cmd_arg(text, i):
    """A control-sequence argument: (name without backslash, next index)."""
    i = skip_ws(text, i)
    m = re.match(r"\\([A-Za-z]+)", text[i:])
    if not m:
        return None, i
    return m.group(1), i + m.end()


def norm(s):
    return re.sub(r"\s+", " ", s).strip()


def rs(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def collect(text):
    """(units, prefixes, binary_prefixes, powers), each with line numbers."""
    units, prefixes, binaries, powers = [], [], [], []
    seen_u, seen_p, seen_b = {}, {}, {}
    for m in re.finditer(
        r"^\\siunitx_declare_(unit|prefix|power):([A-Za-z]+)", text, re.M
    ):
        kind, variant = m.group(1), m.group(2)
        i = m.end()
        if kind == "unit" and variant in ("Nn", "Ne", "Nx"):
            name, i = cmd_arg(text, i)
            if name is None:
                continue
            body, i = group_arg(text, i)
            seen_u[name] = (m.start(), (name, norm(body), ""))
        elif kind == "unit" and variant in ("Nnn", "Nen"):
            name, i = cmd_arg(text, i)
            if name is None:
                continue
            body, i = group_arg(text, i)
            opts, i = group_arg(text, i)
            seen_u[name] = (m.start(), (name, norm(body), norm(opts)))
        elif kind == "prefix" and variant in ("Nnn", "Nne", "Nnx"):
            name, i = cmd_arg(text, i)
            if name is None:
                continue
            power, i = group_arg(text, i)
            symbol, i = group_arg(text, i)
            seen_p[name] = (m.start(), (name, norm(symbol), int(power)))
        elif kind == "prefix" and variant == "Nn":
            name, i = cmd_arg(text, i)
            if name is None:
                continue
            symbol, i = group_arg(text, i)
            seen_b[name] = (m.start(), (name, norm(symbol)))
        elif kind == "power" and variant == "NNn":
            pre, i = cmd_arg(text, i)
            post, i = cmd_arg(text, i)
            if pre is None or post is None:
                continue
            power, i = group_arg(text, i)
            powers.append((m.start(), (pre, post, norm(power))))
    # Public spellings (used by the .cfg companions, kept for regeneration
    # against them); siunitx.sty itself declares no built-ins this way.
    for m in re.finditer(r"^\\DeclareSIUnit\s*(\[[^\]]*\])?", text, re.M):
        i = m.end()
        i = skip_ws(text, i)
        if i < len(text) and text[i] == "{":
            name, i = group_arg(text, i)
            body, i = group_arg(text, i)
            name = name.strip().lstrip("\\")
            if re.fullmatch(r"[A-Za-z]+", name or ""):
                seen_u[name] = (m.start(), (name, norm(body), norm(m.group(1) or "")))
    for m in re.finditer(r"^\\DeclareSIPrefix\s*\{", text, re.M):
        i = m.end() - 1
        name, i = group_arg(text, i)
        symbol, i = group_arg(text, i)
        power, i = group_arg(text, i)
        name = name.strip().lstrip("\\")
        if re.fullmatch(r"[A-Za-z]+", name or ""):
            seen_p[name] = (m.start(), (name, norm(symbol), int(power)))
    units = [v for _, v in sorted(seen_u.values())]
    prefixes = [v for _, v in sorted(seen_p.values())]
    binaries = [v for _, v in sorted(seen_b.values())]
    powers = [v for _, v in sorted(powers)]
    return units, prefixes, binaries, powers


def main():
    path = kpse("siunitx.sty")
    with open(path, encoding="latin-1") as fh:
        text = fh.read()
    m = re.search(
        r"\\ProvidesExplPackage\s*\{siunitx\}\s*\{([^}]*)\}\s*\{([^}]*)\}", text
    )
    date, version = (m.group(1).strip(), m.group(2).strip()) if m else ("?", "?")
    units, prefixes, binaries, powers = collect(text)
    assert units and prefixes, "no declarations parsed"
    out = []
    w = out.append
    w("// @generated by crates/compiler/tools/extract_siunitx.py -- do not edit.")
    w("// Provenance: %s [`\\ProvidesExplPackage{siunitx} {%s} {%s}`]."
      % (path, date, version))
    w("// Units/prefixes are the top-level `\\siunitx_declare_unit` /")
    w("// `\\siunitx_declare_prefix` / `\\siunitx_declare_power` declarations")
    w("// (the bodies of user-level `\\DeclareSIUnit` / `\\DeclareSIPrefix`).")
    w("#![allow(clippy::all)]")
    w("")
    w("/// siunitx version the tables below were extracted from.")
    w("pub const SIUNITX_VERSION: &str = %s;" % rs(version))
    w("/// Release date of that siunitx version.")
    w("pub const SIUNITX_DATE: &str = %s;" % rs(date))
    w("")
    w("/// Built-in units in `siunitx.sty` order: `(name, definition, options)`.")
    w("/// `options` is only non-empty for `\\kWh` (`inter-unit-product = `).")
    w("pub static GENERATED_UNITS: &[(&str, &str, &str)] = &[")
    for name, body, opts in units:
        w("    (%s, %s, %s)," % (rs(name), rs(body), rs(opts)))
    w("];")
    w("")
    w("/// SI prefixes in `siunitx.sty` order: `(name, symbol, power of ten)`.")
    w("pub static GENERATED_PREFIXES: &[(&str, &str, i32)] = &[")
    for name, symbol, power in prefixes:
        w("    (%s, %s, %d)," % (rs(name), rs(symbol), power))
    w("];")
    w("")
    w("/// Binary prefixes in `siunitx.sty` order: `(name, symbol)`.")
    w("pub static GENERATED_BINARY_PREFIXES: &[(&str, &str)] = &[")
    for name, symbol in binaries:
        w("    (%s, %s)," % (rs(name), rs(symbol)))
    w("];")
    w("")
    w("/// Powers in `siunitx.sty` order: `(pre-command, post-command, power)`.")
    w("pub static GENERATED_POWERS: &[(&str, &str, &str)] = &[")
    for pre, post, power in powers:
        w("    (%s, %s, %s)," % (rs(pre), rs(post), rs(power)))
    w("];")
    w("")
    with open(OUT, "w") as fh:
        fh.write("\n".join(out))
    print("wrote %s units=%d prefixes=%d binary=%d powers=%d %s %s"
          % (OUT, len(units), len(prefixes), len(binaries), len(powers),
             version, date))


if __name__ == "__main__":
    main()
