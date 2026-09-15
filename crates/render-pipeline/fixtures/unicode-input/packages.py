#!/usr/bin/env python3
"""pdfLaTeX reference: which packages, classes and babel languages change the
literal UTF-8 input errors of `reference.json` (test-only; cargo never runs TeX).

For every candidate below, under OT1 (no fontenc) and T1
(`\\usepackage[T1]{fontenc}` first), typesets the same 357 characters as
`oracle.py` with the candidate loaded, and compares each character's LaTeX
errors with the plain `article` baseline. `-recorder` lists the files the
candidate loads beyond the baseline; any of them containing
`\\DeclareUnicodeCharacter` (or a `.dfu` file) means it declares characters
the 357-character probe cannot enumerate.

The effect written to `packages.json`:
  * "neutral": loads, no character's errors change, no declaration;
  * "accepts": loads, no declaration, and some characters lose their error
    under an encoding (and nothing else changes) - listed per encoding;
  * "t1": loads `[T1]{fontenc}` itself; every error equals the T1 baseline;
  * "fail-open": anything else (declares characters, changes the encoding,
    raises new errors, or does not load). The pipeline invents no input
    error for such a document.

    python3 crates/render-pipeline/fixtures/unicode-input/packages.py
"""
import json, os, re, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import oracle  # noqa: E402

PACKAGES = """
amsmath amssymb amsfonts amsthm amsbsy mathtools graphicx graphics xcolor color geometry hyperref
cleveref url xurl bookmark booktabs array longtable multirow tabularx colortbl hhline dcolumn
makecell threeparttable enumitem enumerate multicol siunitx microtype listings natbib cite
fancyhdr titlesec caption subcaption subfig float wrapfig setspace parskip lmodern times mathptmx
helvet courier palatino mathpazo newtxtext newtxmath libertine fourier kpfonts tgtermes tgpagella
charter lipsum xspace ifthen calc etoolbox xparse tikz pgfplots algorithm algorithmic algpseudocode
verbatim fancyvrb comment framed mdframed tcolorbox upgreek textcomp gensymb csquotes
newunicodechar ulem soul bm bbm dsfont mathrsfs eucal cancel physics braket tocloft appendix
footmisc pdfpages epstopdf lastpage nicefrac units marvosym wasysym pifont eurosym cmap
placeins afterpage needspace ragged2e indentfirst titling authblk abstract chngcntr xfrac
bigstrut stmaryrd latexsym exscale relsize fullpage a4wide lscape pdflscape rotating tabu
ltablex arydshln diagbox mhchem chemfig qrcode fontawesome5 academicons orcidlink
babel inputenc fontenc textgreek mlmodern cfr-lm tgheros inconsolata beramono sourcecodepro
""".split()

CLASSES = """
article report book letter amsart amsbook amsproc memoir scrartcl scrreprt scrbook beamer
standalone IEEEtran revtex4-2 elsarticle llncs exam moderncv extarticle
""".split()

BABEL = """
english american british USenglish UKenglish canadian australian newzealand french francais
frenchb acadian canadien german ngerman austrian naustrian swissgerman nswissgerman spanish
italian portuguese portuges brazilian brazil dutch catalan polish czech slovak swedish danish
norsk nynorsk finnish magyar hungarian turkish romanian croatian slovene estonian latvian
lithuanian icelandic irish scottish welsh basque galician latin esperanto indonesian bahasa
malay afrikaans albanian breton friulan interlingua occitan romansh samin serbian sorbian
greek russian ukrainian bulgarian vietnamese hebrew
""".split()

# `\\usepackage[<options>]{fontenc}` alone: which baseline it must equal.
FONTENC = {"OT1": "ot1", "T1": "t1", "T1,OT1": "ot1", "OT1,T1": "t1", "TS1,T1": "t1", "TS1,OT1": "ot1"}


def probe(cls, preamble, cps):
    body = "".join(
        "\\message{[CP=%04X]}\\setbox0\\hbox{%s}\\message{[WD=\\number\\wd0]}\n" % (cp, chr(cp)) for cp in cps
    )
    tex = "\\documentclass{%s}\n%s\\begin{document}\n%s\\end{document}\n" % (cls, preamble, body)
    with tempfile.TemporaryDirectory() as d:
        open(os.path.join(d, "t.tex"), "w", encoding="utf-8").write(tex)
        subprocess.run(
            [oracle.PDFLATEX, "-interaction=nonstopmode", "-recorder", "t.tex"],
            cwd=d, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=300,
            env=dict(os.environ, max_print_line="100000", error_line="254", half_error_line="238"),
        )
        log = open(os.path.join(d, "t.log"), encoding="utf-8", errors="replace").read()
        fls = open(os.path.join(d, "t.fls"), encoding="utf-8", errors="replace").read() if os.path.exists(os.path.join(d, "t.fls")) else ""
    first = log.find("[CP=")
    loads = first >= 0 and not re.search(r"^! ", log[:first], re.M)
    out, cur = {}, None
    for m in re.finditer(r"\[CP=([0-9A-F]+)\]|^! (.*?)\n(?:\s+(.*?)\n)?", log, re.M):
        if m.group(1):
            cur = m.group(1)
            out.setdefault(cur, [])
        elif m.group(2) and cur:
            msg = m.group(2)
            if m.group(3) and not m.group(3).startswith(("See the", "Type", "l.")):
                msg = msg + " " + m.group(3).strip()
            out[cur].append(msg)
    files = {l[6:].strip() for l in fls.splitlines() if l.startswith("INPUT ") and not l[6:].strip().startswith(("t.", "./t."))}
    return loads and len(out) == len(cps), out, files


def declares(files, base_files, probed):
    """Files the candidate loads that declare characters the probe does not
    cover: a `.dfu` table, or a `\\DeclareUnicodeCharacter` whose code point
    is not a literal among the probed characters."""
    hits = []
    for f in sorted(files - base_files):
        if f.endswith(".dfu"):
            hits.append(os.path.basename(f))
            continue
        try:
            text = open(f, encoding="latin-1").read()
        except OSError:
            continue
        for m in re.finditer(r"\\DeclareUnicodeCharacter\s*(\{[^}]*\}|.)", text):
            arg = m.group(1).strip("{} ").upper()
            if not re.fullmatch(r"[0-9A-F]{4,6}", arg) or "%04X" % int(arg, 16) not in probed:
                hits.append(os.path.basename(f))
                break
    return hits


def main():
    cps = sorted(set(oracle.declared()) | set(oracle.UNDECLARED))
    undeclared = {"%04X" % cp for cp in oracle.UNDECLARED}
    t1 = "\\usepackage[T1]{fontenc}\n"
    base = {"ot1": probe("article", "", cps), "t1": probe("article", t1, cps)}
    assert all(b[0] for b in base.values())

    jobs = []
    for p in PACKAGES:
        jobs.append((p, "article", "\\usepackage{%s}\n" % p))
    for c in CLASSES:
        jobs.append(("class:" + c, c, ""))
    for lang in BABEL:
        jobs.append(("babel:" + lang, "article", "\\usepackage[%s]{babel}\n" % lang))

    def one(job):
        key, cls, pre = job
        res = {}
        for enc, fe in (("ot1", ""), ("t1", t1)):
            res[enc] = probe(cls, fe + pre, cps)
        return key, res

    entries = {}
    with ThreadPoolExecutor(max_workers=int(os.environ.get("JOBS", "4"))) as ex:
        for key, res in ex.map(one, jobs):
            e = {}
            why = []
            for enc, (loads, errs, files) in res.items():
                if not loads:
                    why.append(enc + ": does not load")
                    continue
                accepts, changed = [], []
                for hx, want in base[enc][1].items():
                    got = errs.get(hx, [])
                    if got == want:
                        continue
                    if want and not got:
                        accepts.append(hx)
                    else:
                        changed.append(hx)
                decl = declares(files, base[enc][2], base[enc][1])
                e[enc] = {"accepts": accepts, "changed": changed, "declaring_files": decl}
                if changed:
                    why.append("%s: %d characters' errors change" % (enc, len(changed)))
                if decl:
                    why.append("%s: loads %s" % (enc, ", ".join(decl)))
                if undeclared & set(accepts):
                    why.append(enc + ": accepts an undeclared character")
            ot1 = res["ot1"]
            loads_fontenc = ot1[0] and any(f.endswith("/fontenc.sty") for f in ot1[2] - base["ot1"][2])
            if loads_fontenc and ot1[1] == base["t1"][1] and not any(e.get(enc, {}).get("declaring_files") for enc in e):
                # Loads `[T1]{fontenc}` itself (the kernel preloads t1enc.def,
                # so fontenc.sty is the file that shows it): exactly the T1
                # baseline.
                e["effect"] = "t1"
            elif why:
                e["effect"] = "fail-open"
                e["why"] = why
            elif any(e[enc]["accepts"] for enc in ("ot1", "t1")):
                e["effect"] = "accepts"
            else:
                e["effect"] = "neutral"
            entries[key] = e
            print(key, e["effect"], "; ".join(why), file=sys.stderr)

    fontenc = {}
    for opts, want in FONTENC.items():
        loads, errs, _ = probe("article", "\\usepackage[%s]{fontenc}\n" % opts, cps)
        fontenc[opts] = {"equals": want if loads and errs == base[want][1] else None}

    ref = {
        "engine": subprocess.run([oracle.PDFLATEX, "--version"], capture_output=True, text=True).stdout.splitlines()[0],
        "entries": entries,
        "fontenc": fontenc,
    }
    with open(os.path.join(HERE, "packages.json"), "w", encoding="utf-8") as f:
        json.dump(ref, f, ensure_ascii=False, indent=1, sort_keys=True)
        f.write("\n")


if __name__ == "__main__":
    main()
