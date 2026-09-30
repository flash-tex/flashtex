#!/usr/bin/env python3
"""Document-level differential fuzzer: candidate engine vs oracle. MIT.

Builds small complete LaTeX (article class) documents from real TeX Live
packages and runs each on both engines through the lockstep capture()
with fmt='pdflatex'. Besides the usual run.py classes, the raw log line
'N words of font info for M fonts' (normalised away by the lockstep
comparison) is compared directly: a font-count difference on two
otherwise equal runs is class 'fontcount-diff'. Stdlib only.
"""
import argparse
import hashlib
import json
import os
import random
import re
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import run as fuzz_run

CLASSES = ("equal", "diverge", "candidate-crash", "oracle-crash",
           "both-crash", "both-fail", "both-hang", "timeout",
           "fontcount-diff", "output-flood", "both-flood")
STORE = ("diverge", "candidate-crash", "oracle-crash", "both-crash",
         "both-hang", "timeout", "fontcount-diff",
         "output-flood", "both-flood")

# Menu of real packages. Presence is checked with kpsewhich at run time;
# a missing package is simply never selected.
FONT_PKGS = ("lmodern", "mathpazo", "newtxtext", "libertine")
# Microtype option sets. Half the microtype draws use a spacing set (the
# spacing + lone-space-header combination panics the candidate with
# 'index out of bounds' in adjust_interword_glue); the other half covers
# the remaining named options (expansion, protrusion, tracking,
# letterspace, final).
MICROTYPE_SPACING_SETS = ("spacing=true", "spacing",
                          "spacing=true,expansion=true",
                          "spacing=true,protrusion=true")
MICROTYPE_OTHER_SETS = ("expansion", "protrusion", "tracking",
                        "letterspace=120", "final",
                        "expansion,protrusion", "tracking,final")
# Lone-space header boxes: each holds a single space and nothing else.
LONE_SPACE_BOXES = (("hbox-space", "\\hbox{ }"),
                    ("hbox-0pt", "\\hbox to 0pt{ }"),
                    ("hbox-bs", "\\hbox{\\ }"))
HYPERREF_OPT_SETS = ("", "draft", "colorlinks", "pdftex,unicode",
                     "breaklinks,hyperfootnotes=false", "pagebackref")
GEOMETRY_OPT_SETS = ("", "margin=1in", "a4paper,margin=2cm",
                     "textwidth=12cm,textheight=20cm")
PAGESTYLES = ("plain", "empty", "headings")
TEXT = ("ffi ffl fi fl -- --- ``quotes'' VA To AV Wo Ta "
        "extraordinary packaging office efficient waffle.")

_kpse_cache = {}


def package_present(sty):
    """True when kpsewhich finds <sty>.sty in the TeX Live tree."""
    if sty not in _kpse_cache:
        try:
            proc = subprocess.run(
                ["kpsewhich", sty + ".sty"], capture_output=True,
                text=True, timeout=30)
            _kpse_cache[sty] = (proc.returncode == 0
                                and bool(proc.stdout.strip()))
        except (OSError, subprocess.SubprocessError):
            _kpse_cache[sty] = False
    return _kpse_cache[sty]


def available_font_pkgs():
    return [p for p in FONT_PKGS if package_present(p)]


def header_block(rng):
    """One preamble page-style block; return (snippet, desc).

    With probability ~0.55 the block defines a page style whose header
    or footer box holds a lone space (\\ps@headings oddhead/oddfoot/both,
    a custom \\ps@mine, or fancyhdr); otherwise a plain \\pagestyle.
    Deterministic for a given random.Random state.
    """
    if rng.random() < 0.55:
        boxname, box = rng.choice(LONE_SPACE_BOXES)
        kind = rng.choice(("headings-oddhead", "headings-oddfoot",
                           "headings-both", "mine", "fancy"))
        if kind == "fancy" and not package_present("fancyhdr"):
            kind = "headings-oddhead"
        if kind == "headings-oddhead":
            snippet = ("\\makeatletter\\def\\ps@headings{\\def\\@oddhead"
                       "{%s}}\\makeatother\n\\pagestyle{headings}\n" % box)
            return snippet, "headings-oddhead:%s/headings" % boxname
        if kind == "headings-oddfoot":
            snippet = ("\\makeatletter\\def\\ps@headings{\\def\\@oddfoot"
                       "{%s}}\\makeatother\n\\pagestyle{headings}\n" % box)
            return snippet, "headings-oddfoot:%s/headings" % boxname
        if kind == "headings-both":
            snippet = ("\\makeatletter\\def\\ps@headings{\\def\\@oddhead"
                       "{%s}\\def\\@oddfoot{%s}}\\makeatother\n"
                       "\\pagestyle{headings}\n" % (box, box))
            return snippet, "headings-both:%s/headings" % boxname
        if kind == "mine":
            snippet = ("\\makeatletter\\def\\ps@mine{\\def\\@oddhead"
                       "{\\slshape Mine \\thepage}\\def\\@oddfoot{%s}}"
                       "\\makeatother\n\\pagestyle{mine}\n" % box)
            return snippet, "mine:%s/mine" % boxname
        snippet = ("\\usepackage{fancyhdr}\n\\pagestyle{fancy}\n"
                   "\\fancyhead[L]{%s}\n\\fancyfoot[C]{%s}\n"
                   % (box, box))
        return snippet, "fancy:%s/fancy" % boxname
    style = rng.choice(PAGESTYLES)
    if rng.random() < 0.3:
        snippet = ("\\makeatletter\\def\\ps@mine{\\def\\@oddhead"
                   "{\\slshape Mine \\thepage}}\\makeatother\n"
                   "\\pagestyle{mine}\n")
        return snippet, "mine:text/mine"
    return "\\pagestyle{%s}\n" % style, "none/%s" % style


def generate_doc(rng):
    """Build a small complete article document; return (text, description).

    Deterministic for a given random.Random state. Every menu choice —
    microtype option set, font package, fontenc, header block, title
    block, null page, pagestyle switch, floats, lists, footnotes, shape
    switches, hyperref/geometry sets — goes through rng.
    """
    parts = ["\\documentclass{article}\n"]
    desc = []
    enc = rng.choice(["T1", "OT1"])
    parts.append("\\usepackage[%s]{fontenc}\n" % enc)
    desc.append("enc=%s" % enc)
    fonts = available_font_pkgs()
    if fonts and rng.random() < 0.8:
        font = rng.choice(fonts)
        parts.append("\\usepackage{%s}\n" % font)
        desc.append("font=%s" % font)
    else:
        desc.append("font=none")
    if package_present("microtype") and rng.random() < 0.7:
        if rng.random() < 0.5:
            opts = rng.choice(MICROTYPE_SPACING_SETS)
        else:
            opts = rng.choice(MICROTYPE_OTHER_SETS)
        parts.append("\\usepackage[%s]{microtype}\n" % opts)
        desc.append("microtype=%s" % opts)
    else:
        desc.append("microtype=none")
    if package_present("hyperref") and rng.random() < 0.5:
        opts = rng.choice(HYPERREF_OPT_SETS)
        parts.append("\\usepackage[%s]{hyperref}\n" % opts if opts
                     else "\\usepackage{hyperref}\n")
        desc.append("hyperref=%s" % (opts or "none"))
    else:
        desc.append("hyperref=none")
    if package_present("geometry") and rng.random() < 0.5:
        opts = rng.choice(GEOMETRY_OPT_SETS)
        if opts:
            parts.append("\\usepackage[%s]{geometry}\n" % opts)
        desc.append("geometry=%s" % (opts or "none"))
    else:
        desc.append("geometry=none")
    snippet, hdesc = header_block(rng)
    parts.append(snippet)
    desc.append("header=%s" % hdesc)
    parts.append("\\begin{document}\n")
    if rng.random() < 0.4:
        if rng.random() < 0.5:
            parts.append("\\title{Probe Title}\n\\author{An Author}\n"
                         "\\maketitle\n")
            desc.append("title=maketitle")
        else:
            parts.append("{\\centering\\Large Probe Title\\par\n"
                         "\\large An Author\\par}\\vspace{1em}\n")
            desc.append("title=hand")
    else:
        desc.append("title=none")
    if rng.random() < 0.5:
        parts.append("\\null\n\\newpage\n")
        desc.append("nullpage=yes")
    else:
        desc.append("nullpage=no")
    parts.append("\\section{Probe %d}\n" % rng.randint(0, 999))
    paras = rng.randint(1, 3)
    for _ in range(paras):
        parts.append(TEXT + "\n\n")
    if rng.random() < 0.3:
        switch = rng.choice(PAGESTYLES)
        parts.append("\\pagestyle{%s}\n" % switch)
        desc.append("pageswitch=%s" % switch)
    else:
        desc.append("pageswitch=none")
    for switch in rng.sample(
            ["\\textsl{%s}", "\\textit{%s}", "\\textsc{%s}",
             "\\emph{%s}"], rng.randint(0, 2)):
        parts.append(switch % "shaped words here" + "\n\n")
    if rng.random() < 0.5:
        if rng.random() < 0.4:
            parts.append("\\begin{figure}\n\\centering\\hbox{}\n"
                         "\\caption{An empty figure.}\n\\end{figure}\n")
            desc.append("figure=emptybox")
        else:
            parts.append("\\begin{figure}\n\\centering\\rule{3cm}{2cm}\n"
                         "\\caption{A figure.}\n\\end{figure}\n")
            desc.append("figure=yes")
    if rng.random() < 0.5:
        if rng.random() < 0.4:
            parts.append("\\begin{table}\n\\centering\n"
                         "\\begin{tabular}{ll}\n\\end{tabular}\n"
                         "\\caption{An empty table.}\n\\end{table}\n")
            desc.append("table=empty")
        else:
            parts.append("\\begin{table}\n\\centering\n"
                         "\\begin{tabular}{ll}\na & b \\\\ c & d \\\\\n"
                         "\\end{tabular}\n\\caption{A table.}\n\\end{table}\n")
            desc.append("table=yes")
    if rng.random() < 0.5:
        env = rng.choice(["itemize", "enumerate"])
        parts.append("\\begin{%s}\n\\item first\n\\item second\n"
                     "\\end{%s}\n" % (env, env))
        desc.append("list=%s" % env)
    if rng.random() < 0.5:
        parts.append("A sentence.\\footnote{A footnote with \\emph{"
                     "emphasis}.}\n")
        desc.append("footnote=yes")
    parts.append("\\end{document}\n")
    return "".join(parts), ";".join(desc)


FONT_INFO_RE = re.compile(
    r"(\d+) words of font info for (\d+) fonts?, out of")


def font_count(log):
    """Number of fonts on the raw 'words of font info' log line, or None."""
    m = FONT_INFO_RE.search(log or "")
    return int(m.group(2)) if m else None


def classify(cand_rc, cand_log, orc_rc, orc_log, timed_out):
    """run.classify plus the fontcount-diff rule (equal runs whose raw
    'words of font info for N fonts' lines name different N)."""
    base = fuzz_run.classify(cand_rc, cand_log, orc_rc, orc_log,
                             timed_out)
    if base == "equal":
        cand_n = font_count(cand_log)
        orc_n = font_count(orc_log)
        if (cand_n is not None and orc_n is not None
                and cand_n != orc_n):
            return "fontcount-diff"
    return base


def signature(cls, cand_rc, cand_log, orc_rc, orc_log, diff,
              cand_stderr="", orc_stderr=""):
    """Dedupe signature; delegates to run.signature except fontcount-diff."""
    if cls == "fontcount-diff":
        return ("fontcount-diff:candidate-%s-oracle-%s"
                % (font_count(cand_log), font_count(orc_log)))
    return fuzz_run.signature(cls, cand_rc, cand_log, orc_rc, orc_log,
                              diff, cand_stderr=cand_stderr,
                              orc_stderr=orc_stderr)


def run_one(text, candidate, oracle, timeout, return_logs=False):
    """Run a complete document on both engines (fmt='pdflatex').

    Mirrors run.run_one but with the pdflatex format and no prelude;
    returns (class, cand_rc, orc_rc, diff[, cand_log, orc_log]).

    Each engine gets its own workdir: a pdflatex run writes doc.aux
    (and .out/.toc) next to the source, and sharing one dir would let
    the second engine read the first one's aux file (a spurious
    '(./doc.aux)' log difference).
    """
    workdirs = [tempfile.mkdtemp(prefix="docgen-") for _ in range(2)]
    try:
        results = []
        timeouts = []
        for workdir, binary, extra in (
                (workdirs[0], candidate, fuzz_run.candidate_env()),
                (workdirs[1], oracle, None)):
            tex_path = os.path.join(workdir, "doc.tex")
            with open(tex_path, "w") as fh:
                fh.write(text)
            try:
                cap = fuzz_run.lockstep_run.capture(
                    tex_path, binary, workdir, fmt="pdflatex",
                    extra_env=extra, timeout=timeout)
                results.append((cap.returncode, cap.log))
                timeouts.append(False)
            except subprocess.TimeoutExpired:
                results.append((None, ""))
                timeouts.append(True)
        (cand_rc, cand_full), (orc_rc, orc_full) = results
        # Compare the FULL logs (see run.run_one): capping first would
        # hide a mid-log difference. A log past the file-size cap never
        # reaches the comparison: its engine died with SIGXFSZ and is
        # classed output-flood/both-flood above any log content.
        timed_out = timeouts[0] or timeouts[1]
        cls = classify(cand_rc, cand_full, orc_rc, orc_full, timeouts)
        if timed_out:
            diff = "timeout: %s" % "/".join(
                s for s, t in (("candidate", timeouts[0]),
                               ("oracle", timeouts[1])) if t)
        elif cls == "fontcount-diff":
            diff = ("font info candidate=%s oracle=%s"
                    % (font_count(cand_full), font_count(orc_full)))
        else:
            diff = fuzz_run.first_diff(cand_rc, cand_full, orc_rc, orc_full)
        # Cap only what leaves this function (artifacts and JSON).
        cand_log = fuzz_run.cap_text(cand_full, fuzz_run.LOG_MAX_BYTES)
        orc_log = fuzz_run.cap_text(orc_full, fuzz_run.LOG_MAX_BYTES)
        if return_logs:
            return cls, cand_rc, orc_rc, diff, cand_log, orc_log
        return cls, cand_rc, orc_rc, diff
    finally:
        for workdir in workdirs:
            shutil.rmtree(workdir, ignore_errors=True)


def run_docs(candidate, oracle, out_dir, iterations, seed, timeout):
    rng = random.Random(seed)
    known = fuzz_run.load_known_signatures(out_dir)
    seen = set()
    sig_counts = {}
    counts = {c: 0 for c in CLASSES}
    for i in range(iterations):
        text, options = generate_doc(rng)
        cls, cand_rc, orc_rc, diff, cand_log, orc_log = run_one(
            text, candidate, oracle, timeout, return_logs=True)
        counts[cls] += 1
        if cls in STORE:
            cand_err, orc_err, panic_loc = "", "", None
            if cls in ("candidate-crash", "both-crash"):
                cand_err = fuzz_run.crash_stderr(
                    text, candidate, fuzz_run.candidate_env(), timeout,
                    fmt="pdflatex")
                panic_loc = (fuzz_run.panic_location(cand_err)
                             or fuzz_run.panic_location(cand_log))
            if cls in ("oracle-crash", "both-crash"):
                orc_err = fuzz_run.crash_stderr(text, oracle, None, timeout,
                                               fmt="pdflatex")
                if cls == "oracle-crash":
                    panic_loc = (fuzz_run.panic_location(orc_err)
                                 or fuzz_run.panic_location(orc_log))
            sig = signature(cls, cand_rc, cand_log, orc_rc, orc_log,
                            diff, cand_stderr=cand_err, orc_stderr=orc_err)
            if sig is not None:
                sig_counts[sig] = sig_counts.get(sig, 0) + 1
            if sig is None or (sig not in seen and sig not in known):
                if sig is not None:
                    seen.add(sig)
                digest = hashlib.sha256(
                    text.encode("utf-8")).hexdigest()[:16]
                cls_dir = os.path.join(out_dir, cls)
                os.makedirs(cls_dir, exist_ok=True)
                with open(os.path.join(cls_dir, digest + ".tex"), "w") as fh:
                    fh.write(text)
                with open(os.path.join(cls_dir, digest + ".json"), "w") as fh:
                    json.dump({"iteration": i, "options": options,
                               "candidate_returncode": cand_rc,
                               "oracle_returncode": orc_rc,
                               "first_diff": diff,
                               "panic_location": panic_loc,
                               "signature": sig}, fh, indent=2)
        if (i + 1) % 100 == 0:
            print("docgen %d/%d: %s"
                  % (i + 1, iterations,
                     " ".join("%s=%d" % (c, counts[c]) for c in CLASSES)))
    if sig_counts:
        try:
            path = os.path.join(out_dir, "signatures.json")
            merged = dict(sig_counts)
            try:
                with open(path) as fh:
                    old = json.load(fh)
                if isinstance(old, dict):
                    for key, val in old.items():
                        merged[key] = merged.get(key, 0) + val
            except (OSError, ValueError):
                pass
            os.makedirs(out_dir, exist_ok=True)
            with open(path, "w") as fh:
                json.dump(merged, fh, indent=2, sort_keys=True)
        except OSError:
            pass
    return counts


def main(argv=None):
    ap = argparse.ArgumentParser(description="document-level TeX fuzzer")
    ap.add_argument("--candidate", required=True, help="candidate engine")
    ap.add_argument("--oracle", required=True, help="oracle engine")
    ap.add_argument("--out", required=True, help="output directory")
    ap.add_argument("--iterations", type=int, required=True)
    ap.add_argument("--seed", type=int, required=True)
    ap.add_argument("--timeout", type=float, required=True,
                    help="per-engine timeout in seconds")
    args = ap.parse_args(argv)
    fuzz_run.apply_fsize_limit()
    for label, binary in (("candidate", args.candidate),
                          ("oracle", args.oracle)):
        if not (os.path.isfile(binary) or shutil.which(binary)):
            print("error: %s not found: %s" % (label, binary),
                  file=sys.stderr)
            return 2
    if args.iterations < 0:
        print("error: --iterations must be >= 0", file=sys.stderr)
        return 2
    counts = run_docs(args.candidate, args.oracle, args.out,
                      args.iterations, args.seed, args.timeout)
    print("done: %d iterations: %s"
          % (args.iterations,
             " ".join("%s=%d" % (c, counts[c]) for c in CLASSES)))
    return counts


if __name__ == "__main__":
    result = main()
    sys.exit(0 if isinstance(result, dict) else result)
