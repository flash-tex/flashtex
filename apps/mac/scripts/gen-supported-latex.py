#!/usr/bin/env python3
"""Generate supported-latex.json from measured package support. MIT.

The completion vocabulary both editors bundle (schema flashtex-supported-latex/1)
comes from the new engine's measured package support and from pdfTeX, never
from hand-written tables or from the old compiler (retirement plan RQ5, ruled
2026-10-11). Three steps, each with its own committed file under
apps/mac/supported-latex/:

  record   package-smoke.json  which package-smoke documents the new engine
                               typesets exactly as pdfTeX does: the stdout of
                               tools/package-smoke/run.py (a P5 scoreboard
                               artifact's smoke-new.txt) plus its oracle.json.
  probe    probe.json          what pdflatex (the oracle) defines: the LaTeX
                               kernel under the article class, and every
                               verified package used by at least
                               VOCABULARY_MIN_DOCUMENTS of the 400 papers of
                               tools/package-smoke/sample-400.json. Needs TeX
                               Live 2026 (pdflatex, kpsewhich).
  render   supported-latex.json  the vocabulary, from the two files above and
                               the package-smoke manifests. Standard library
                               only, no TeX: CI re-renders it and compares.

    gen-supported-latex.py record --smoke-log smoke-new.txt --oracle oracle.json \\
        --engine-commit SHA --source URL
    gen-supported-latex.py probe [--texbin DIR] [--out FILE] [--check]
    gen-supported-latex.py render [--install]
    gen-supported-latex.py check [--installed]

`check` (and `probe --check`) exit 0 when the committed file is what the step
produces now, 1 when it is stale, 2 when it cannot check here (`probe` without
pdflatex). `render --install` also writes the two bundled copies the editors
decode (apps/mac/Sources/FlashTeXMac/Resources/ and
apps/ios/FlashTeXPad/Resources/); until S7 those are still the old compiler's
inventory, kept by sync-supported-latex.sh, so `--install` and
`check --installed` are S7's switch.

How a definition becomes a vocabulary row (`classify`): a \\mathchardef is a
math symbol (math only); a macro whose body starts with a math-only primitive
(\\mathaccent, \\mathop, \\radical, ...) or a math-only primitive itself is a
math structure (math only); every other command is offered in text and in math
(a text row plus a math twin), because pdfTeX accepts a macro in either mode
and the oracle does not say otherwise. Argument shapes come from the macro's
parameter text, LaTeX's optional-argument inner macro, or for a document
command its argument specification (\\GetDocumentCommandArgSpec).
"""
import argparse
import glob
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

MAC_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ROOT = os.path.dirname(os.path.dirname(MAC_DIR))
DATA_DIR = os.path.join(MAC_DIR, "supported-latex")
SMOKE_DIR = os.path.join(ROOT, "tools", "package-smoke")
MEASURED = os.path.join(DATA_DIR, "package-smoke.json")
PROBE = os.path.join(DATA_DIR, "probe.json")
OUTPUT = os.path.join(DATA_DIR, "supported-latex.json")
INSTALLED = [
    os.path.join(MAC_DIR, "Sources", "FlashTeXMac", "Resources", "supported-latex.json"),
    os.path.join(ROOT, "apps", "ios", "FlashTeXPad", "Resources", "supported-latex.json"),
]
SCHEMA = "flashtex-supported-latex/1"
GENERATOR = "apps/mac/scripts/gen-supported-latex.py"
# A verified package's commands are in the vocabulary when at least this many of
# sample-400.json's 400 papers load it (5 %); every verified package is listed.
VOCABULARY_MIN_DOCUMENTS = 20
# The oracle pdfTeX every input must name (tools/lockstep pins the same).
ORACLE_VERSION = "pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026)"
MEANING_CHARS = 160  # a meaning is kept to this many characters (classification reads its start)

# TeX primitives that are errors outside math mode (tex.web §1046 insert_dollar_sign
# and the math-only commands of §1138-§1193).
MATH_ONLY_PRIMITIVES = frozenset("""
mathchar mathaccent delimiter radical mathchoice mathop mathbin mathrel mathord mathopen
mathclose mathpunct mathinner over atop above overwithdelims atopwithdelims abovewithdelims
left right middle limits nolimits displaylimits displaystyle textstyle scriptstyle
scriptscriptstyle underline overline vcenter mkern mskip nonscript eqno leqno fam
""".split())
# A macro body that starts with one of these is math-only (the primitives above,
# plus LaTeX's \mathpalette, which is \mathchoice).
MATH_START = MATH_ONLY_PRIMITIVES | {"mathpalette"}


def starts_math(text):
    """Whether a macro body's first token is a math-only command (a whole name:
    \\left is, \\leftarrow is not)."""
    m = re.match(r"\\([A-Za-z]+)", text)
    return bool(m) and m.group(1) in MATH_START


class Usage(Exception):
    pass


class CannotCheck(Exception):
    pass


# ---- inputs ---------------------------------------------------------------------

def smoke_manifests(smoke_dir=SMOKE_DIR):
    """Document stem -> {package, file, pages, passes}, from every smoke-*-manifest.json."""
    docs = {}
    for path in sorted(glob.glob(os.path.join(smoke_dir, "smoke-*-manifest.json"))):
        with open(path, encoding="utf-8") as fh:
            for p in json.load(fh)["packages"]:
                stem = p["file"][:-4] if p["file"].endswith(".tex") else p["file"]
                if stem in docs:
                    raise Usage("%s lists %s twice" % (path, p["file"]))
                docs[stem] = {"package": p["package"], "file": p["file"],
                              "pages": p.get("pages"), "passes": p.get("passes")}
    return docs


def sample_frequency(smoke_dir=SMOKE_DIR):
    with open(os.path.join(smoke_dir, "sample-400.json"), encoding="utf-8") as fh:
        d = json.load(fh)
    return d["documents"], d["document_frequency"]


_VERDICT = re.compile(r"^(\S+)\s+(equal|EXCLUDED|DIFFERENT)(?:\s+\((.*)\))?\s*$")
_SUMMARY = re.compile(r"^(\d+) documents, (\d+) differ")


def parse_smoke_log(text):
    """tools/package-smoke/run.py stdout -> ({stem: (verdict, reason)}, documents)."""
    verdicts, total = {}, None
    for line in text.splitlines():
        m = _VERDICT.match(line)
        if m:
            stem, verdict, reason = m.group(1), m.group(2).lower(), m.group(3) or ""
            if stem in verdicts:
                raise Usage("the smoke log names %s twice" % stem)
            verdicts[stem] = (verdict, reason)
            continue
        m = _SUMMARY.match(line)
        if m:
            total = int(m.group(1))
    if total is None:
        raise Usage("the smoke log has no summary line (an incomplete run.py output?)")
    if total != len(verdicts):
        raise Usage("the smoke log's summary counts %d documents but lists %d" % (total, len(verdicts)))
    return verdicts, total


# ---- record ---------------------------------------------------------------------

def record(args):
    with open(args.smoke_log, encoding="utf-8") as fh:
        verdicts, total = parse_smoke_log(fh.read())
    with open(args.oracle, encoding="utf-8") as fh:
        oracle = json.load(fh)
    if oracle.get("version") != ORACLE_VERSION:
        raise Usage("the smoke run's oracle is %r, not %s" % (oracle.get("version"), ORACLE_VERSION))
    docs = smoke_manifests()
    unknown = sorted(set(verdicts) - set(docs))
    missing = sorted(set(docs) - set(verdicts))
    if unknown:
        raise Usage("the smoke log names documents no manifest lists: %s" % ", ".join(unknown))
    if missing:
        raise Usage("the smoke log is partial; it lacks %s" % ", ".join(missing))
    if not re.fullmatch(r"[0-9a-f]{40}", args.engine_commit):
        raise Usage("--engine-commit must be a full 40-hex commit")
    out = {
        "about": "Which package-smoke documents the new engine typesets as pdfTeX does "
                 "(tools/package-smoke/run.py, both passes: transcripts, box dumps and "
                 "P-T2 PDFs). Written by %s record; MIT." % GENERATOR,
        "engine_commit": args.engine_commit,
        "source": args.source,
        "oracle": {k: oracle[k] for k in ("version", "latex_format", "tlpdb_sha256", "pdftex_sha256", "platform")
                   if k in oracle},
        # "equal", or "excluded: <reason>" / "different: <reason>" as run.py printed it
        "documents": {stem: (v + ": " + r if r else v) for stem, (v, r) in sorted(verdicts.items())},
    }
    write_json(MEASURED, out)
    equal = sum(1 for v, _ in verdicts.values() if v == "equal")
    print("record: %d documents, %d equal -> %s" % (total, equal, os.path.relpath(MEASURED, ROOT)))
    return 0


def load_measured(path=MEASURED):
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


def verified_packages(measured, docs):
    """Package -> stem of its document, for every document the engine matched."""
    out = {}
    for stem, v in measured["documents"].items():
        if v == "equal" and stem in docs:
            out[docs[stem]["package"]] = stem
    return out


def vocabulary_packages(measured, docs, frequency):
    """Verified packages in the vocabulary, most-loaded first (name breaks ties)."""
    verified = verified_packages(measured, docs)
    chosen = [p for p in verified if frequency.get(p, 0) >= VOCABULARY_MIN_DOCUMENTS]
    return sorted(chosen, key=lambda p: (-frequency[p], p))


# ---- probe (pdflatex is the oracle) ----------------------------------------------

_CS = re.compile(r"\\([A-Za-z@_:]+)")
_ENV_DECL = re.compile(r"\\(?:(?:re)?newenvironment|(?:New|Renew|Provide|Declare)DocumentEnvironment)"
                       r"\s*\*?\s*\{([A-Za-z]+\*?)\}")
_USEPACKAGE = re.compile(r"\\usepackage\s*(\[[^\]]*\])?\s*\{([^}]*)\}")


def candidates(text):
    """Letter-only control-sequence names and environment names a source file mentions.

    @, _ and : count as letters (package and expl3 code), so \\foo@bar is one name and
    is dropped as internal, never cut to \\foo. pdflatex decides which are defined."""
    names, envs = set(), set()
    for m in _CS.finditer(text):
        n = m.group(1)
        if n.isalpha() and n.isascii():
            names.add(n)
            if n.startswith("end") and len(n) > 3:
                envs.add(n[3:])
    for m in _ENV_DECL.finditer(text):
        envs.add(m.group(1))
    return names, envs


def usepackage_line(package, doc_text):
    """The smoke document's own \\usepackage[...]{package}, so the probe loads the
    package with the options the verified document used."""
    for m in _USEPACKAGE.finditer(doc_text):
        if package in [p.strip() for p in m.group(2).split(",")]:
            return "\\usepackage%s{%s}" % (m.group(1) or "", package)
    return "\\usepackage{%s}" % package


PROBE_MACROS = r"""\makeatletter
\newwrite\ft@out \immediate\openout\ft@out=ftprobe.ftw\relax
\def\ft@w#1{\begingroup\newlinechar=-1 \immediate\write\ft@out{#1}\endgroup}
\def\ft@pre#1{\ifcsname #1\endcsname\ft@w{p #1}\fi}
\def\ft@preenv#1{\ifcsname #1\endcsname\ifcsname end#1\endcsname\ft@w{q #1}\fi\fi}
\def\ft@post#1{%
  \ifcsname #1\endcsname\ft@w{d #1 \expandafter\meaning\csname #1\endcsname}\fi
  \ifcsname #1\space\endcsname\ft@w{s #1 \expandafter\meaning\csname #1\space\endcsname}\fi
  \ifcsname \@backslashchar#1\endcsname\ft@w{b #1 \expandafter\meaning\csname \@backslashchar#1\endcsname}\fi
  \ifcsname \@backslashchar#1\space\endcsname\ft@w{c #1 \expandafter\meaning\csname \@backslashchar#1\space\endcsname}\fi}
\def\ft@postenv#1{\ifcsname #1\endcsname\ifcsname end#1\endcsname
  \ft@w{e #1 \expandafter\meaning\csname end#1\endcsname}\fi\fi}
\def\ft@spec#1{\let\ArgumentSpecification\relax
  \expandafter\GetDocumentCommandArgSpec\csname #1\endcsname
  \ft@w{a #1 \detokenize\expandafter{\ArgumentSpecification}}}
"""


def probe_document(load_line, pre, post, spec=()):
    """The probe: names defined before the package (pre), each name's \\meaning after
    it (post), then document commands' argument specifications (spec)."""
    lines = ["\\documentclass{article}", PROBE_MACROS]
    lines += ["\\ft@pre{%s}" % n for n in sorted(pre[0])]
    lines += ["\\ft@preenv{%s}" % n for n in sorted(pre[1])]
    if load_line:
        lines.append(load_line)
    lines += ["\\ft@post{%s}" % n for n in sorted(post[0])]
    lines += ["\\ft@postenv{%s}" % n for n in sorted(post[1])]
    lines += ["\\ft@spec{%s}" % n for n in sorted(spec)]
    lines += ["\\immediate\\closeout\\ft@out", "\\makeatother", "\\begin{document}", "\\end{document}", ""]
    return "\n".join(lines)


def parse_probe_output(text):
    """ftprobe.ftw -> (pre names, pre envs, {name: {d,s,b,c,a}}, envs). (Not .out:
    hyperref writes its bookmarks to \\jobname.out.)"""
    pre, pre_env, defs, envs = set(), set(), {}, set()
    for line in text.splitlines():
        if len(line) < 3 or line[1] != " ":
            continue
        tag, rest = line[0], line[2:]
        name, _, value = rest.partition(" ")
        if tag == "p":
            pre.add(name)
        elif tag == "q":
            pre_env.add(name)
        elif tag == "e":
            if value != "\\end" + name:  # \endinput, \endcsname: primitives, not environment ends
                envs.add(name)
        elif tag == "a":
            if value.strip() != "\\relax":  # not a document command after all
                defs.setdefault(name, {})[tag] = value
        elif tag in "dsbc":
            defs.setdefault(name, {})[tag] = value
    return pre, pre_env, defs, envs


def body(meaning):
    """(prefixes, parameter text, body) of a macro meaning, or None."""
    m = re.match(r"^((?:\\(?:long|protected|outer) )*)macro:(.*?)->(.*)$", meaning, re.S)
    return (m.group(1), m.group(2), m.group(3)) if m else None


def prune(defs):
    """Keep what classify reads: each meaning's start, and the inner macros only where
    the outer one is robust (\\protect) or takes an optional argument (testopt)."""
    out = {}
    for name, d in defs.items():
        if "d" not in d:
            continue
        keep = {"d": d["d"][:MEANING_CHARS]}
        outer = body(d["d"])
        robust = outer and outer[2].startswith("\\protect ") and "s" in d
        eff = body(d["s"]) if robust else outer
        if robust:
            keep["s"] = d["s"][:MEANING_CHARS]
        if eff and re.match(r"\\@(?:protected@)?testopt ", eff[2]):
            inner = "c" if robust else "b"
            if inner in d:
                keep[inner] = d[inner][:MEANING_CHARS]
        if "a" in d:
            keep["a"] = d["a"]
        out[name] = keep
    return dict(sorted(out.items()))


def tex_files(fls_text, cwd):
    """INPUT files of a -recorder .fls that hold TeX source, as absolute paths."""
    out = []
    for line in fls_text.splitlines():
        if not line.startswith("INPUT "):
            continue
        p = line[6:]
        p = p if os.path.isabs(p) else os.path.join(cwd, p)
        if re.search(r"\.(sty|cls|clo|def|cfg|fd|tex|ldf|code\.tex)$", p) and "ftprobe" not in p:
            out.append(os.path.normpath(p))
    return sorted(set(out))


def read_source(path):
    with open(path, encoding="latin-1") as fh:
        return fh.read()


class Oracle:
    def __init__(self, texbin=None):
        self.pdflatex = os.path.join(texbin, "pdflatex") if texbin else shutil.which("pdflatex")
        self.kpsewhich = os.path.join(texbin, "kpsewhich") if texbin else shutil.which("kpsewhich")
        if not self.pdflatex or not os.access(self.pdflatex, os.X_OK):
            raise CannotCheck("no pdflatex (TeX Live 2026) here; pass --texbin")
        if not self.kpsewhich or not os.access(self.kpsewhich, os.X_OK):
            raise CannotCheck("no kpsewhich (TeX Live 2026) here; pass --texbin")

    def version(self):
        out = subprocess.run([self.pdflatex, "--version"], capture_output=True, text=True).stdout
        return out.splitlines()[0].strip() if out else ""

    def find(self, name):
        return subprocess.run([self.kpsewhich, name], capture_output=True, text=True).stdout.strip()

    def run(self, tex, workdir, halt=True):
        """pdflatex -recorder on tex; (ok, log, fls, probe output)."""
        with open(os.path.join(workdir, "ftprobe.tex"), "w", encoding="utf-8") as fh:
            fh.write(tex)
        for ext in (".ftw", ".out", ".fls", ".log", ".aux"):
            p = os.path.join(workdir, "ftprobe" + ext)
            if os.path.exists(p):
                os.remove(p)
        proc = subprocess.run([self.pdflatex, "-recorder", "-interaction=batchmode"]
                              + (["-halt-on-error"] if halt else []) + ["-draftmode", "ftprobe.tex"], cwd=workdir, capture_output=True,
                              stdin=subprocess.DEVNULL, timeout=600)

        def rd(ext):
            p = os.path.join(workdir, "ftprobe" + ext)
            if not os.path.exists(p):
                return ""
            with open(p, encoding="utf-8", errors="replace") as fh:
                return fh.read()
        return proc.returncode == 0, rd(".log"), rd(".fls"), rd(".ftw")


def probe_one(oracle, workdir, load_line, base_files, kernel_extra=()):
    """One package (load_line) or the kernel (load_line None): its new definitions."""
    ok, log, fls, _ = oracle.run(probe_document(load_line, (set(), set()), (set(), set())), workdir)
    if not ok:
        return {"error": first_error(log)}, set()
    files = tex_files(fls, workdir)
    own = sorted(set(files) - set(base_files)) if load_line else sorted(set(files) | set(kernel_extra))
    names, envs = set(), set()
    for path in own:
        n, e = candidates(read_source(path))
        names |= n
        envs |= e
    pre = (names, envs) if load_line else (set(), set())
    ok, log, _, out = oracle.run(probe_document(load_line, pre, (names, envs)), workdir)
    if not ok:
        return {"error": first_error(log)}, set(files)
    pre_n, pre_e, defs, got_envs = parse_probe_output(out)
    defs = {n: d for n, d in defs.items() if n not in pre_n}
    spec = sorted(n for n, d in defs.items() if (body(d.get("d", "")) or ("", "", ""))[2].startswith("\\__cmd_"))
    if spec:
        # Not halting: a name that is not a document command after all is an error
        # here, and its spec line then reads \relax (parse_probe_output drops it).
        _, log, _, out = oracle.run(probe_document(load_line, pre, (names, envs), spec), workdir, halt=False)
        for n, d in parse_probe_output(out)[2].items():
            if n in defs and "a" in d:
                defs[n]["a"] = d["a"]
    return {"commands": prune(defs), "environments": sorted(got_envs - pre_e)}, set(files)


def first_error(log):
    for line in log.splitlines():
        if line.startswith("! "):
            return line[2:].strip()
    return "pdflatex failed"


def tlpdb_sha256(oracle):
    root = subprocess.run([oracle.kpsewhich, "-var-value=SELFAUTOPARENT"], capture_output=True,
                          text=True).stdout.strip()
    path = os.path.join(root, "tlpkg", "texlive.tlpdb")
    if not os.path.isfile(path):
        return None
    with open(path, "rb") as fh:
        return hashlib.sha256(fh.read()).hexdigest()


def make_probe(oracle):
    measured = load_measured()
    docs = smoke_manifests()
    _, frequency = sample_frequency()
    packages = vocabulary_packages(measured, docs, frequency)
    version = oracle.version()
    if version != ORACLE_VERSION:
        raise CannotCheck("this pdflatex is %r, not %s" % (version, ORACLE_VERSION))
    stems = verified_packages(measured, docs)
    out = {"about": "What pdflatex defines: the LaTeX kernel under the article class, then each "
                    "verified package's new names (%s probe); MIT." % GENERATOR,
           "oracle": {"version": version, "tlpdb_sha256": tlpdb_sha256(oracle)},
           "kernel": None, "packages": {}}
    with tempfile.TemporaryDirectory(prefix="ft-supported-latex-") as work:
        latex_ltx = oracle.find("latex.ltx")
        if not latex_ltx:
            raise CannotCheck("kpsewhich finds no latex.ltx")
        kernel, base = probe_one(oracle, work, None, (), kernel_extra=[latex_ltx])
        if "error" in kernel:
            raise CannotCheck("the kernel probe failed: %s" % kernel["error"])
        out["kernel"] = kernel
        for p in packages:
            with open(os.path.join(SMOKE_DIR, docs[stems[p]]["file"]), encoding="utf-8") as fh:
                line = usepackage_line(p, fh.read())
            res, _ = probe_one(oracle, work, line, base)
            res = dict({"load": line}, **res)
            out["packages"][p] = res
            print("probe: %-16s %s" % (p, res.get("error") or "%d commands, %d environments" % (
                len(res["commands"]), len(res["environments"]))), flush=True)
    return out


def probe(args):
    oracle = Oracle(args.texbin)
    data = make_probe(oracle)
    out = args.out or PROBE
    if out != PROBE:
        write_json(out, data)
        print("probe: wrote %s" % out)
    if args.check:
        return compare_file(PROBE, dumps_json(data))
    if out == PROBE:
        write_json(PROBE, data)
        print("probe: wrote %s" % os.path.relpath(PROBE, ROOT))
    return 0


# ---- classify and render (no TeX) -----------------------------------------------

def spec_arguments(spec):
    """An xparse argument specification -> the editors' argument shape."""
    out, i, n = [], 0, 0

    def skip_group(j):
        depth = 0
        while j < len(spec):
            c = spec[j]
            depth += c == "{"
            depth -= c == "}"
            j += 1
            if depth == 0:
                return j
        return j
    while i < len(spec):
        c = spec[i]
        if c in " +!":
            i += 1
            continue
        if c in ">=":
            i = skip_group(i + 1) if i + 1 < len(spec) and spec[i + 1] == "{" else i + 1
            continue
        n += 1
        if c in "mvb":
            out.append("{#%d}" % n)
            i += 1
        elif c == "o":
            out.append("[#%d]" % n)
            i += 1
        elif c == "O":
            out.append("[#%d]" % n)
            i = skip_group(i + 1)
        elif c == "s":
            out.append("*")
            i += 1
        elif c == "t":
            out.append(spec[i + 1] if i + 1 < len(spec) else "")
            i += 2
        elif c in "dr":
            pair = spec[i + 1:i + 3]
            out.append("[#%d]" % n if pair == "[]" else "")
            i += 3
        elif c in "DR":
            pair = spec[i + 1:i + 3]
            out.append("[#%d]" % n if pair == "[]" else "")
            i = skip_group(i + 3)
        elif c == "e":
            i = skip_group(i + 1)
        elif c == "E":
            i = skip_group(skip_group(i + 1))
        else:
            return ""
    return "".join(out)


def param_arguments(params):
    """A macro's parameter text -> argument shape: #1#2 -> {#1}{#2}, [#1]#2 -> [#1]{#2};
    a delimited parameter text has no braced shape ("")."""
    if params == "":
        return ""
    m = re.fullmatch(r"(\[#1\])?((?:#\d)*)", params)
    if not m:
        return ""
    opt = m.group(1) or ""
    return opt + "".join("{%s}" % a for a in re.findall(r"#\d", m.group(2)))


def classify(name, d):
    """(kind, arguments): kind is 'symbol' (math only, a \\mathchardef), 'math' (math
    only), 'primitive', 'text' (offered in text and math) or None (skip)."""
    meaning = d["d"]
    if meaning.startswith('\\mathchar"'):
        return "symbol", ""
    if meaning == "\\relax" and name != "relax":
        return None, ""  # a placeholder (\let to \relax), not a command
    mac = body(meaning)
    if mac is None:
        prim = re.fullmatch(r"\\([A-Za-z]+)", meaning)
        if prim:
            if prim.group(1) in MATH_ONLY_PRIMITIVES:
                return "math", ""
            return ("primitive" if prim.group(1) == name else "text"), ""
        return "text", ""  # \chardef, a register, a font, a \let to a character
    if "a" in d:
        return "text", spec_arguments(d["a"])
    eff = mac
    robust = mac[2].startswith("\\protect ") and "s" in d
    if robust:
        eff = body(d["s"]) or mac
    if starts_math(eff[2]):
        return "math", param_arguments(eff[1])
    if re.match(r"\\@(?:protected@)?testopt ", eff[2]):
        inner = body(d.get("c" if robust else "b", "")) if d.get("c" if robust else "b") else None
        return "text", param_arguments(inner[1]) if inner else ""
    return "text", param_arguments(eff[1])


def rows(source, description, commands):
    out = []
    for name in sorted(commands):
        kind, args = classify(name, commands[name])
        if kind is None:
            continue
        base = {"name": name}
        if kind == "symbol":
            out.append(dict(base, mode="math", origin="math_symbol", arguments="", description=description))
        elif kind == "math":
            out.append(dict(base, mode="math", origin="math_structure", arguments=args, description=description))
        else:
            origin = "expansion" if kind == "primitive" else "text_dispatch"
            desc = "TeX primitive" if kind == "primitive" else description
            out.append(dict(base, mode="text", origin=origin, arguments=args, description=desc))
            out.append(dict(base, mode="math", origin="math_structure", arguments=args, description="accepted"))
    for r in out:
        r["renders"] = True
        r["source"] = source
    return out


def render_data(measured, probe_data, docs, frequency, sample_documents):
    verified = verified_packages(measured, docs)
    commands, environments, seen, seen_env = [], [], set(), set()

    def add(source, description, part):
        for r in rows(source, description, part.get("commands", {})):
            key = (r["name"], r["mode"])
            if key not in seen:
                seen.add(key)
                commands.append(r)
        for e in part.get("environments", []):
            if e not in seen_env:
                seen_env.add(e)
                environments.append({"name": e, "mode": "text", "description": description, "source": source})
    add("latex", "LaTeX kernel", probe_data["kernel"])
    for p in vocabulary_packages(measured, docs, frequency):
        part = probe_data["packages"].get(p)
        if part is None:
            raise Usage("probe.json lacks %s; re-run the probe step" % p)
        add(p, "\\usepackage{%s}" % p, part)
    vocab = set(vocabulary_packages(measured, docs, frequency))
    packages = []
    for stem, doc in sorted(docs.items(), key=lambda kv: kv[1]["package"]):
        verdict, _, reason = measured["documents"].get(stem, "missing").partition(": ")
        row = {"name": doc["package"], "document": "tools/package-smoke/" + doc["file"], "verdict": verdict}
        if reason:
            row["reason"] = reason
        row["papers"] = frequency.get(doc["package"], 0)
        row["vocabulary"] = doc["package"] in vocab and "error" not in probe_data["packages"].get(doc["package"], {})
        if doc["package"] in probe_data["packages"] and "error" in probe_data["packages"][doc["package"]]:
            row["probe_error"] = probe_data["packages"][doc["package"]]["error"]
        packages.append(row)
    return {
        "schema": SCHEMA,
        "generator": GENERATOR + " render",
        "compiler_version": "flashtex-engine " + measured["engine_commit"][:12],
        "license": "MIT",
        "unsupported_behaviour": "a command outside this list still runs as pdfLaTeX runs it; the list is what "
                                 "completion offers",
        "measurement": {
            "package_smoke": {"engine_commit": measured["engine_commit"], "source": measured["source"],
                              "oracle": measured["oracle"],
                              "documents": len(measured["documents"]),
                              "equal": len(verified)},
            "probe_oracle": probe_data["oracle"],
            "vocabulary": "the LaTeX kernel (article class) and every verified package loaded by at least "
                          "%d of the %d papers of tools/package-smoke/sample-400.json"
                          % (VOCABULARY_MIN_DOCUMENTS, sample_documents),
        },
        "commands": commands,
        "environments": environments,
        "packages": packages,
    }


def dumps_rows(key, items, last=False):
    body_ = ",\n".join("    " + json.dumps(i, ensure_ascii=False) for i in items)
    return '  "%s": [\n%s\n  ]%s\n' % (key, body_, "" if last else ",")


def dumps_supported(data):
    """One row per line, as the old inventory was, so a diff shows changed rows."""
    head = {k: v for k, v in data.items() if k not in ("commands", "environments", "packages")}
    text = "{\n"
    for k, v in head.items():
        text += '  %s: %s,\n' % (json.dumps(k), json.dumps(v, ensure_ascii=False, sort_keys=True))
    text += dumps_rows("commands", data["commands"])
    text += dumps_rows("environments", data["environments"])
    text += dumps_rows("packages", data["packages"], last=True)
    return text + "}\n"


def render_text():
    with open(PROBE, encoding="utf-8") as fh:
        probe_data = json.load(fh)
    documents, frequency = sample_frequency()
    return dumps_supported(render_data(load_measured(), probe_data, smoke_manifests(), frequency, documents))


def render(args):
    text = render_text()
    targets = [OUTPUT] + (INSTALLED if args.install else [])
    for t in targets:
        with open(t, "w", encoding="utf-8") as fh:
            fh.write(text)
        print("render: wrote %s (%s)" % (os.path.relpath(t, ROOT), hashlib.sha256(text.encode()).hexdigest()[:12]))
    return 0


def check(args):
    text = render_text()
    targets = [OUTPUT] + (INSTALLED if args.installed else [])
    status = 0
    for t in targets:
        status = max(status, compare_file(t, text))
    return status


# ---- helpers --------------------------------------------------------------------

def dumps_json(data):
    return json.dumps(data, indent=1, sort_keys=True, ensure_ascii=False) + "\n"


def write_json(path, data):
    with open(path, "w", encoding="utf-8") as fh:
        fh.write(dumps_json(data))


def compare_file(path, text):
    rel = os.path.relpath(path, ROOT)
    try:
        with open(path, encoding="utf-8") as fh:
            have = fh.read()
    except FileNotFoundError:
        print("STALE %s is missing; run %s" % (rel, GENERATOR))
        return 1
    if have == text:
        print("ok    %s is current" % rel)
        return 0
    a, b = have.splitlines(), text.splitlines()
    for i, (x, y) in enumerate(zip(a, b)):
        if x != y:
            print("STALE %s line %d:\n  committed: %s\n  generated: %s" % (rel, i + 1, x[:200], y[:200]))
            break
    else:
        print("STALE %s: %d lines committed, %d generated" % (rel, len(a), len(b)))
    return 1


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="step", required=True)
    r = sub.add_parser("record", help="package-smoke.json from a run.py stdout and its oracle.json")
    r.add_argument("--smoke-log", required=True)
    r.add_argument("--oracle", required=True)
    r.add_argument("--engine-commit", required=True)
    r.add_argument("--source", required=True, help="where the run is (its Actions run URL)")
    p = sub.add_parser("probe", help="probe.json from pdflatex (needs TeX Live 2026)")
    p.add_argument("--texbin")
    p.add_argument("--out", help="write the probe here (default: the committed probe.json)")
    p.add_argument("--check", action="store_true", help="exit 1 when the committed probe.json differs")
    g = sub.add_parser("render", help="supported-latex.json from the committed inputs")
    g.add_argument("--install", action="store_true", help="also write the two bundled copies (S7)")
    c = sub.add_parser("check", help="exit 1 when supported-latex.json is stale")
    c.add_argument("--installed", action="store_true", help="also compare the two bundled copies (S7)")
    args = ap.parse_args(argv)
    try:
        return {"record": record, "probe": probe, "render": render, "check": check}[args.step](args)
    except CannotCheck as exc:
        print("cannot check here: %s" % exc, file=sys.stderr)
        return 2
    except Usage as exc:
        print("error: %s" % exc, file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
