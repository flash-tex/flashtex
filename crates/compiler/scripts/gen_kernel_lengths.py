#!/usr/bin/env python3
r"""Generate src/kernel_lengths.rs: the value every LaTeX kernel length, TeX
parameter and float counter has at `\begin{document}` under each built-in
class, size and layout option, measured with pdflatex.

Oracle tooling only; pdflatex never runs in the product path or in cargo.

    python3 crates/compiler/scripts/gen_kernel_lengths.py [--texbin /Library/TeX/texbin] [--check]

The expansion engine (`flashtex-tex-expansion`) declares the kernel lengths as
real registers (`\textwidth`, `\parskip`, `\topsep`, `\hsize`, ...), so
`\setlength`, `\addtolength`, `\the`, `0.5\textwidth` and `\advance` execute
in it exactly as in TeX. The *values* are the class's: `size1x.clo` sets
`\textwidth`, `\normalsize` sets `\baselineskip` and the display skips,
`\@listi` sets `\topsep`, `\begin{document}` copies `\textwidth` into `\hsize`
and `\linewidth`. Rather than re-implementing every class file this asks
pdflatex: for each class, size and combination of the layout options that
move these lengths (`a4paper`/`letterpaper`, `onecolumn`/`twocolumn`,
`oneside`/`twoside`) it typesets an empty document and records `\the` of
every name in `REGISTERS` after `\begin{document}` (`max_print_line` raised so
nothing wraps), plus the float-fraction macros in `MACROS`. The compiler
(`expansion.rs`) runs the matching row as a host prelude before the document.

Rows are stored as one full row per class (its default options and size) plus
the differences of every other combination, which keeps the table small: an
option moves a handful of page lengths, a size moves the font-dependent ones.
"""
import argparse
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "src", "kernel_lengths.rs")

# TeX's dimen, glue and integer parameters (tex.web §236-§248; pdfTeX's page
# and output parameters), then latex.ltx's `\newdimen`/`\newskip`/`\newcount`
# registers (ltdefns, ltspace, ltlists, ltfloat, ltpage, ltoutput, ltboxes,
# ltfntcmd, ltpictur, lttab, ltsect), then its `\newcounter`s (`\c@...`).
# `\skip\footins` is an insertion, `\thinmuskip` & co. are mu glue: neither
# is a register the engine models, so they are not listed.
REGISTERS = [
    # TeX dimen parameters.
    "parindent", "mathsurround", "lineskiplimit", "hsize", "vsize", "maxdepth",
    "splitmaxdepth", "boxmaxdepth", "hfuzz", "vfuzz", "delimitershortfall",
    "nulldelimiterspace", "scriptspace", "predisplaysize", "displaywidth",
    "displayindent", "overfullrule", "hangindent", "hoffset", "voffset",
    "emergencystretch", "pdfpagewidth", "pdfpageheight", "pdfhorigin", "pdfvorigin",
    # TeX glue parameters.
    "baselineskip", "lineskip", "parskip", "abovedisplayskip", "belowdisplayskip",
    "abovedisplayshortskip", "belowdisplayshortskip", "leftskip", "rightskip",
    "topskip", "splittopskip", "tabskip", "spaceskip", "xspaceskip", "parfillskip",
    # TeX integer parameters.
    "pretolerance", "tolerance", "linepenalty", "hyphenpenalty", "exhyphenpenalty",
    "clubpenalty", "widowpenalty", "displaywidowpenalty", "brokenpenalty",
    "binoppenalty", "relpenalty", "predisplaypenalty", "postdisplaypenalty",
    "interlinepenalty", "doublehyphendemerits", "finalhyphendemerits", "adjdemerits",
    "mag", "delimiterfactor", "looseness", "showboxbreadth", "showboxdepth",
    "hbadness", "vbadness", "uchyph", "outputpenalty", "maxdeadcycles", "hangafter",
    "floatingpenalty", "globaldefs", "fam", "defaulthyphenchar", "defaultskewchar",
    "lefthyphenmin", "righthyphenmin", "holdinginserts", "errorcontextlines",
    "pdfoutput", "pdfcompresslevel", "pdfobjcompresslevel", "pdfdecimaldigits",
    "pdfpkresolution", "pdfminorversion", "pdfmajorversion",
    # latex.ltx dimens.
    "paperwidth", "paperheight", "textwidth", "textheight", "oddsidemargin",
    "evensidemargin", "topmargin", "headheight", "headsep", "footskip",
    "marginparwidth", "marginparsep", "marginparpush", "columnwidth", "columnsep",
    "columnseprule", "linewidth", "leftmargin", "rightmargin", "listparindent",
    "itemindent", "labelwidth", "labelsep", "leftmargini", "leftmarginii",
    "leftmarginiii", "leftmarginiv", "leftmarginv", "leftmarginvi", "footnotesep",
    "jot", "arraycolsep", "tabcolsep", "arrayrulewidth", "doublerulesep", "fboxsep",
    "fboxrule", "unitlength", "@maxdepth", "@wholewidth", "@halfwidth",
    "@totalleftmargin", "@colht", "@colroom", "@pageht", "@pagedp", "@textmin",
    "@textfloatsheight",
    # latex.ltx skips.
    "topsep", "partopsep", "itemsep", "parsep", "@topsep", "@topsepadd", "floatsep",
    "textfloatsep", "intextsep", "dblfloatsep", "dbltextfloatsep", "@fptop", "@fpsep",
    "@fpbot", "@dblfptop", "@dblfpsep", "@dblfpbot", "smallskipamount",
    "medskipamount", "bigskipamount", "abovecaptionskip", "belowcaptionskip",
    # latex.ltx counts.
    "@lowpenalty", "@medpenalty", "@highpenalty", "@beginparpenalty",
    "@endparpenalty", "@itempenalty", "@clubpenalty", "@topnum", "@botnum",
    "@dbltopnum", "interfootnotelinepenalty",
    # latex.ltx counters (`\c@<name>`).
    "c@topnumber", "c@bottomnumber", "c@totalnumber", "c@dbltopnumber",
    "c@secnumdepth", "c@tocdepth",
    # Lengths the standard classes allocate themselves (`\newlength` in
    # article.cls & co., absent from latex.ltx and from other classes): the
    # engine's kernel prelude does not declare them, so a row that has one
    # says so (`class_lengths`) and the host declares it before assigning.
    "abovecaptionskip", "belowcaptionskip",
]
CLASS_LENGTHS = ["abovecaptionskip", "belowcaptionskip"]

# Number-valued macros the standard classes define (`\renewcommand
# \topfraction{.7}` in article.cls; latex.ltx itself leaves them undefined),
# recorded as text and `\def`d by the host for the classes that have them.
MACROS = ["topfraction", "bottomfraction", "textfraction", "floatpagefraction",
          "dbltopfraction", "dblfloatpagefraction"]

# (class, sizes, has twocolumn option). Every class also takes `a4paper`/
# `letterpaper` and `oneside`/`twoside`. beamer has no paper or side
# options; its sizes are its own.
CLASSES = [
    ("article", ["10pt", "11pt", "12pt"], True),
    ("report", ["10pt", "11pt", "12pt"], True),
    ("book", ["10pt", "11pt", "12pt"], True),
    ("letter", ["10pt", "11pt", "12pt"], False),
    ("amsart", ["8pt", "9pt", "10pt", "11pt", "12pt"], False),
    ("amsbook", ["8pt", "9pt", "10pt", "11pt", "12pt"], False),
    ("amsproc", ["8pt", "9pt", "10pt", "11pt", "12pt"], False),
    ("scrartcl", ["10pt", "11pt", "12pt"], True),
    ("scrreprt", ["10pt", "11pt", "12pt"], True),
    ("scrbook", ["10pt", "11pt", "12pt"], True),
    ("beamer", ["10pt", "11pt", "12pt"], None),
]


def probe(texbin, cls, options):
    # `\makeatletter` after `\begin{document}`: `\document` makes `@` other.
    lines = [r"\documentclass[%s]{%s}" % (",".join(options), cls),
             r"\begin{document}", r"\makeatletter", r"\typeout{FLASHTEX-BEGIN}"]
    # Class-declared names may be absent: `\ifdefined` keeps the probe alive
    # and the row simply lacks them.
    for name in REGISTERS:
        lines.append(r"\typeout{R:%s=\ifdefined\%s\the\%s\else\string\UNDEFINED\fi}" % (name, name, name))
    for name in MACROS:
        lines.append(r"\typeout{M:%s=\ifdefined\%s\%s\else\string\UNDEFINED\fi}" % (name, name, name))
    lines += [r"\typeout{FLASHTEX-END}", r"\end{document}"]
    with tempfile.TemporaryDirectory() as tmp:
        with open(os.path.join(tmp, "probe.tex"), "w") as fh:
            fh.write("\n".join(lines) + "\n")
        env = dict(os.environ, max_print_line="100000", SOURCE_DATE_EPOCH="0")
        r = subprocess.run(
            [os.path.join(texbin, "pdflatex"), "-interaction=nonstopmode", "-halt-on-error", "probe.tex"],
            cwd=tmp, capture_output=True, text=True, check=False, env=env,
        )
        out = r.stdout
    if "FLASHTEX-END" not in out:
        sys.stderr.write("pdflatex failed for %s %s:\n%s\n" % (cls, options, out[-2000:]))
        return None
    values = {}
    for m in re.finditer(r"^([RM]):([@a-zA-Z]+)=(.*)$", out, re.M):
        kind, name, value = m.groups()
        if value.strip() != r"\UNDEFINED":
            values[(kind, name)] = value.strip()
    missing = [n for n in REGISTERS if n not in CLASS_LENGTHS and ("R", n) not in values]
    if missing:
        sys.stderr.write("%s %s: no value for %s\n" % (cls, options, missing))
        return None
    return values


def version(texbin):
    r = subprocess.run([os.path.join(texbin, "pdflatex"), "--version"], capture_output=True, text=True, check=False)
    return r.stdout.splitlines()[0] if r.stdout else "pdflatex (unknown version)"


def write_or_check(path, text, check):
    """Write `text` to `path`, or with `--check` compare instead (exit codes as
    scripts/check-generated.py documents: 0 current, 1 stale, 2 cannot check)."""
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


def rust_str(s):
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--texbin", default="/Library/TeX/texbin")
    ap.add_argument("--check", action="store_true", help="compare with the committed file instead of writing")
    args = ap.parse_args()
    if not os.path.exists(os.path.join(args.texbin, "pdflatex")):
        print("CANNOT CHECK: %s/pdflatex not found (needs TeX Live 2026)" % args.texbin)
        sys.exit(2)

    classes = []  # (class, default_size, default_a4, default_twocolumn, default_twoside, base_values, [rows])
    for cls, sizes, has_twocolumn in CLASSES:
        beamer = has_twocolumn is None
        default = probe(args.texbin, cls, [])
        if default is None:
            sys.exit(1)
        rows = []
        for size in sizes:
            papers = [None] if beamer else ["letterpaper", "a4paper"]
            columns = [None] if not has_twocolumn else ["onecolumn", "twocolumn"]
            sides = [None] if beamer else ["oneside", "twoside"]
            for paper in papers:
                for column in columns:
                    for side in sides:
                        options = [o for o in (size, paper, column, side) if o]
                        values = probe(args.texbin, cls, options)
                        if values is None:
                            sys.exit(1)
                        rows.append((size, paper, column, side, values))
        # The class defaults: the explicit combination whose values equal the
        # option-less run's.
        match = [r for r in rows if r[4] == default]
        if not match:
            sys.stderr.write("%s: no explicit option row equals the default run\n" % cls)
            sys.exit(1)
        size, paper, column, side, _ = match[0]
        classes.append((cls, size, paper, column, side, default, rows))

    out = []
    out.append("//! Generated by `crates/compiler/scripts/gen_kernel_lengths.py` from")
    out.append("//! `%s`; do not edit." % version(args.texbin))
    out.append("//!")
    out.append("//! Every LaTeX kernel length, TeX parameter, float counter and float-fraction")
    out.append("//! macro as it stands at `\\begin{document}` under each built-in class,")
    out.append("//! size and layout option (see the generator's docstring). The expansion")
    out.append("//! engine's registers are set from the matching row before the document")
    out.append("//! is read (`expansion::class_prelude`).")
    out.append("")
    out.append("/// One measured combination: the class options given and the values that")
    out.append("/// differ from the class's [`ClassDefaults::base`] row.")
    out.append("pub struct Row {")
    out.append("    pub size: &'static str,")
    out.append("    /// `Some(true)` for `a4paper`, `Some(false)` for `letterpaper`, `None`")
    out.append("    /// when the class has no paper option (beamer).")
    out.append("    pub a4paper: Option<bool>,")
    out.append("    pub twocolumn: Option<bool>,")
    out.append("    pub twoside: Option<bool>,")
    out.append("    pub delta: &'static [(&'static str, &'static str)],")
    out.append("}")
    out.append("")
    out.append("/// A class: its default options, the full value row for those defaults")
    out.append("/// (`registers` are `(name, \\the text)`, `macros` are `(name, text)`), and")
    out.append("/// every other measured combination as a difference from that row.")
    out.append("pub struct ClassDefaults {")
    out.append("    pub class: &'static str,")
    out.append("    pub size: &'static str,")
    out.append("    pub a4paper: Option<bool>,")
    out.append("    pub twocolumn: Option<bool>,")
    out.append("    pub twoside: Option<bool>,")
    out.append("    pub registers: &'static [(&'static str, &'static str)],")
    out.append("    /// The `registers` this class allocates itself (`\\newlength` in the")
    out.append("    /// class file, not in latex.ltx): the host declares each before")
    out.append("    /// assigning it.")
    out.append("    pub class_lengths: &'static [&'static str],")
    out.append("    pub macros: &'static [(&'static str, &'static str)],")
    out.append("    pub rows: &'static [Row],")
    out.append("}")
    out.append("")
    out.append("pub const CLASSES: &[ClassDefaults] = &[")
    for cls, size, paper, column, side, default, rows in classes:
        out.append("    ClassDefaults {")
        out.append("        class: %s," % rust_str(cls))
        out.append("        size: %s," % rust_str(size))
        out.append("        a4paper: %s," % ("None" if paper is None else ("Some(true)" if paper == "a4paper" else "Some(false)")))
        out.append("        twocolumn: %s," % ("None" if column is None else ("Some(true)" if column == "twocolumn" else "Some(false)")))
        out.append("        twoside: %s," % ("None" if side is None else ("Some(true)" if side == "twoside" else "Some(false)")))
        out.append("        registers: &[")
        for name in REGISTERS:
            if ("R", name) in default:
                out.append("            (%s, %s)," % (rust_str(name), rust_str(default[("R", name)])))
        out.append("        ],")
        out.append("        class_lengths: &[%s]," % ", ".join(rust_str(n) for n in CLASS_LENGTHS if ("R", n) in default))
        out.append("        macros: &[")
        for name in MACROS:
            if ("M", name) in default:
                out.append("            (%s, %s)," % (rust_str(name), rust_str(default[("M", name)])))
        out.append("        ],")
        out.append("        rows: &[")
        for rsize, rpaper, rcolumn, rside, values in rows:
            delta = [(k, v) for k, v in values.items() if default.get(k) != v]
            out.append("            Row {")
            out.append("                size: %s," % rust_str(rsize))
            out.append("                a4paper: %s," % ("None" if rpaper is None else ("Some(true)" if rpaper == "a4paper" else "Some(false)")))
            out.append("                twocolumn: %s," % ("None" if rcolumn is None else ("Some(true)" if rcolumn == "twocolumn" else "Some(false)")))
            out.append("                twoside: %s," % ("None" if rside is None else ("Some(true)" if rside == "twoside" else "Some(false)")))
            if delta:
                out.append("                delta: &[")
                line = "                   "
                for (kind, name), value in delta:
                    key = name if kind == "R" else "\\" + name
                    entry = " (%s, %s)," % (rust_str(key), rust_str(value))
                    if len(line) + len(entry) > 100 and line.strip():
                        out.append(line)
                        line = "                   "
                    line += entry
                if line.strip():
                    out.append(line)
                out.append("                ],")
            else:
                out.append("                delta: &[],")
            out.append("            },")
        out.append("        ],")
        out.append("    },")
    out.append("];")
    rc = write_or_check(OUT, "\n".join(out) + "\n", args.check)
    if not args.check:
        print("wrote %s (%d classes)" % (os.path.relpath(OUT), len(classes)))
    sys.exit(rc)


if __name__ == "__main__":
    main()
