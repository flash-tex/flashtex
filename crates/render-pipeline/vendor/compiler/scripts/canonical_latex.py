#!/usr/bin/env python3
"""Regenerate crates/compiler/supported/canonical-latex.tsv.

The canonical list is the denominator of the coverage figures printed by
`flashtex-compiler --supported`. It is not hand-written: every name in it is

  1. a *candidate* taken from a published source file on disk, and
  2. *confirmed* by a real pdfLaTeX run to be defined by that source.

Candidates
  kernel   every `@findex \\name` command and `@EnvIndex{name}` environment of
           "LaTeX2e: An unofficial reference manual" (TeX Live package
           latex2e-help-texinfo, doc/latex/latex2e-help-texinfo/latex2e.texi).
  packages every letters-only control sequence token (plus `name*` forms)
           appearing in the package's own source files listed in PACKAGES.

Confirmation (pdfLaTeX, `\\documentclass{article}`)
  kernel   defined (not undefined, not \\relax) in a document with no packages.
  package  defined after `\\usepackage{pkg}` (preamble, document body, or, for
           tikz, inside a tikzpicture) and NOT defined in that package's
           dependency baseline: article plus every other .sty the package
           loads (read from \\@filelist), so keyval's \\setkeys or pgf's
           \\pgfpicture are not counted as geometry or TikZ commands.
           Names containing `@`, `_` or `:` are never candidates. tikz also
           excludes `pgf*` names (PGF basic layer, not the TikZ frontend).
  environment  `name` is an environment when \\endname is newly defined and
           \\name is defined; it is then listed as an environment only.

MacTeX is a development-time oracle only; nothing here is in the product path.
Run from anywhere:  python3 crates/compiler/scripts/canonical_latex.py
"""

import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE.parent / "supported" / "canonical-latex.tsv"

PACKAGES = [
    # (set name, \usepackage name, the package's own source files)
    ("amsmath", "amsmath", ["amsmath.sty", "amstext.sty", "amsbsy.sty", "amsopn.sty"]),
    ("amssymb", "amssymb", ["amssymb.sty", "amsfonts.sty"]),
    ("enumitem", "enumitem", ["enumitem.sty"]),
    ("geometry", "geometry", ["geometry.sty"]),
    ("graphicx", "graphicx", ["graphicx.sty", "graphics.sty"]),
    ("hyperref", "hyperref", ["hyperref.sty", "nameref.sty"]),
    ("tikz", "tikz", ["tikz.sty", "tikz.code.tex"]),
    ("xcolor", "xcolor", ["xcolor.sty"]),
    ("siunitx", "siunitx", ["siunitx.sty"]),
]

TOKEN = re.compile(r"\\([A-Za-z]+)")
STARRED = re.compile(r"(?:newenvironment|namedef|csname|newlist)\s*\{?\s*([A-Za-z]+\*)")


def run(*cmd):
    return subprocess.run(cmd, check=True, capture_output=True, text=True).stdout


def kpse(name):
    path = run("kpsewhich", name).strip()
    if not path:
        sys.exit(f"kpsewhich could not find {name}")
    return Path(path)


def kernel_candidates():
    texmf = Path(run("kpsewhich", "-var-value", "TEXMFDIST").strip())
    texi = texmf / "doc/latex/latex2e-help-texinfo/latex2e.texi"
    text = texi.read_text(encoding="utf-8", errors="replace")
    commands = set(re.findall(r"^@findex \\([A-Za-z]+)(?:\*|\s|@|$)", text, re.M))
    environments = set(re.findall(r"^@EnvIndex\{([A-Za-z]+\*?)\}", text, re.M))
    updated = re.search(r"^@set UPDATED (.+)$", text, re.M).group(1).strip()
    return updated, commands, environments


def package_candidates(name, files):
    names = set()
    for f in files:
        text = kpse(f).read_text(encoding="latin-1")
        names |= set(TOKEN.findall(text))
        names |= set(STARRED.findall(text))
    if name == "tikz":
        names = {n for n in names if not n.startswith("pgf")}
    return names


def probe_lines(tag, names):
    out = [f"\\immediate\\openout\\probeout=probe-{tag}.txt"]
    for n in sorted(names):
        out.append(
            f"\\immediate\\write\\probeout{{{n} \\ifcsname {n}\\endcsname"
            f"\\expandafter\\ifx\\csname {n}\\endcsname\\relax 0\\else 1\\fi\\else 0\\fi}}"
        )
    out.append("\\immediate\\closeout\\probeout")
    return "\n".join(out)


def pdflatex(workdir, stem, lines):
    (workdir / f"{stem}.tex").write_text("\n".join(lines) + "\n", encoding="utf-8")
    subprocess.run(
        ["pdflatex", "-interaction=batchmode", "-halt-on-error", f"{stem}.tex"],
        cwd=workdir, capture_output=True, text=True,
    )
    return (workdir / f"{stem}.log").read_text(encoding="latin-1")


def read_probe(workdir, tag):
    path = workdir / f"probe-{tag}.txt"
    if not path.exists():
        sys.exit(f"pdflatex did not write {path}; see the .log files in {workdir}")
    defined = set()
    for line in path.read_text(encoding="latin-1").splitlines():
        name, _, flag = line.rpartition(" ")
        if flag == "1":
            defined.add(name)
    return defined


def with_env_names(names):
    """Adds `endX` and `X*`/`endX*` probes so environments can be recognised."""
    extra = set()
    for n in names:
        base = n.rstrip("*")
        extra |= {base, "end" + base, base + "*", "end" + base + "*"}
    return names | extra


def split(new, candidates, available):
    """`new`: names this source defines; `available`: everything defined."""
    envs = sorted(
        n for n in candidates
        if not n.startswith("end") and "end" + n in new and n in available | new
    )
    env_set = set(envs)
    commands = sorted(
        n for n in candidates
        if n in new and n not in env_set and not n.endswith("*")
        and not (n.startswith("end") and n[3:] in env_set)
    )
    return commands, envs


def main():
    for tool in ("pdflatex", "kpsewhich", "tex"):
        if not shutil.which(tool):
            sys.exit(f"{tool} is required (MacTeX/TeX Live) to regenerate the canonical list")

    updated, kernel_cmds, kernel_envs = kernel_candidates()
    pkg_cands = {name: package_candidates(name, files) for name, _, files in PACKAGES}
    everything = with_env_names(kernel_cmds | kernel_envs | set().union(*pkg_cands.values()))
    rows, sources = [], []
    engine = run("tex", "--version").splitlines()[0]

    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        log = pdflatex(work, "baseline", [
            "\\documentclass{article}", "\\newwrite\\probeout",
            probe_lines("base-pre", everything),
            "\\begin{document}", probe_lines("base-doc", everything),
            "\\makeatletter\\immediate\\write16{FMT \\fmtversion\\space patch \\patch@level}\\makeatother",
            "x\\end{document}",
        ])
        base = read_probe(work, "base-pre") | read_probe(work, "base-doc")
        cmds, envs = split(base, with_env_names(kernel_cmds | kernel_envs), base)
        rows += [("kernel", "command", c) for c in cmds if c in kernel_cmds]
        rows += [("kernel", "environment", e) for e in envs if e in kernel_envs]
        fmt = re.search(r"FMT (\S+) patch (\S+)", log)
        sources.append(
            f"kernel: doc/latex/latex2e-help-texinfo/latex2e.texi (UPDATED {updated}) "
            f"@findex commands and @EnvIndex environments; confirmed with LaTeX2e "
            f"{fmt.group(1)} patch level {fmt.group(2)}, article class"
        )

        for name, package, files in PACKAGES:
            cands = with_env_names(pkg_cands[name])
            # \write to a file stream is not wrapped at max_print_line like the log.
            pdflatex(work, f"{name}-files", [
                "\\documentclass{article}", f"\\usepackage{{{package}}}", "\\makeatletter",
                "\\newwrite\\filesout", f"\\immediate\\openout\\filesout=files-{name}.txt",
                "\\immediate\\write\\filesout{\\@filelist}", "\\immediate\\closeout\\filesout",
                "\\makeatother", "\\begin{document}x\\end{document}",
            ])
            loaded = (work / f"files-{name}.txt").read_text(encoding="latin-1").strip()
            own = set(files) | {package + ".sty"}
            deps = [f[:-4] for f in loaded.split(",") if f.endswith(".sty") and f not in own]
            pdflatex(work, f"{name}-deps", [
                "\\documentclass{article}", "\\newwrite\\probeout",
                *[f"\\RequirePackage{{{d}}}" for d in deps],
                probe_lines("dpre", cands), "\\begin{document}", probe_lines("ddoc", cands),
                "x\\end{document}",
            ])
            dep_defined = read_probe(work, "dpre") | read_probe(work, "ddoc")

            doc = ["\\documentclass{article}", "\\newwrite\\probeout", f"\\usepackage{{{package}}}",
                   probe_lines("pre", cands), "\\begin{document}", probe_lines("doc", cands)]
            if name == "tikz":
                doc += ["\\begin{tikzpicture}", probe_lines("pic", cands), "\\end{tikzpicture}"]
            doc.append("\\makeatletter")
            doc += [f"\\immediate\\write16{{VER {f} \\csname ver@{f}\\endcsname}}"
                    for f in files if f.endswith(".sty")]
            if name == "tikz":
                doc.append("\\immediate\\write16{VER pgf \\pgfversion}")
            doc += ["\\makeatother", "x\\end{document}"]
            plog = pdflatex(work, name, doc)
            defined = read_probe(work, "pre") | read_probe(work, "doc")
            if name == "tikz":
                defined |= read_probe(work, "pic")
            new = defined - base - dep_defined
            cmds, envs = split(new, cands, base | dep_defined)
            rows += [(name, "command", c) for c in cmds if c in pkg_cands[name]]
            rows += [(name, "environment", e) for e in envs]
            versions = "; ".join(m.strip() for m in re.findall(r"^VER (.+)$", plog, re.M))
            sources.append(
                f"{name}: tokens of {', '.join(files)} ({versions}); "
                f"dependency baseline: {', '.join(deps) or 'none'}"
            )

    header = [
        "# FlashTeX canonical LaTeX list: the coverage denominator for `flashtex-compiler --supported`.",
        "# GENERATED by crates/compiler/scripts/canonical_latex.py; do not edit by hand.",
        "# Method: candidates from the sources below, each confirmed defined by a pdfLaTeX run",
        "# (\\documentclass{article}); a package set excludes names defined without it (article plus",
        "# the package's own dependencies); names with @ _ : are excluded; tikz excludes pgf*.",
        f"# Engine: {engine}",
    ] + [f"# Source {s}" for s in sources] + ["# set\tkind\tname"]
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(header + ["\t".join(r) for r in rows]) + "\n", encoding="utf-8")
    counts = {}
    for s, k, _ in rows:
        counts[(s, k)] = counts.get((s, k), 0) + 1
    for (s, k), n in sorted(counts.items()):
        print(f"{s:10} {k:12} {n}")
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
