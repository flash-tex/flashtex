#!/usr/bin/env python3
"""Regenerates tests/oracle/hyphenation_corpus.txt from pdflatex (oracle only).

FlashTeX never runs TeX in its product path. This script is a test-data
generator: it typesets every paragraph of CORPUS with pdflatex (TeX Live 2026,
12pt article, T1 + `times` = ptmr8t, geometry margin=1in so \\hsize =
469.75499pt, \\parindent 0pt, justified, LaTeX's default hyphenation language
`english` = hyphen.tex, \\hyphenpenalty=\\exhyphenpenalty=50), puts each one in
a \\vbox, and reads the resulting lines back from \\showbox output in the log.

Output format (UTF-8 text, one record per line, tab-separated):
    P<TAB><paragraph source>
    L<TAB><line text as typeset, a trailing '-' when broken at a hyphen>
    O<TAB><n>   (only when pdflatex reported n Overfull \\hbox warnings)
Glue becomes one space; kerns and penalties are dropped; ligatures are expanded.

Usage: python3 gen_hyphenation_oracle.py [pdflatex]
"""
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))

CORPUS = [
    # 1-2: the HW1/HW2 sentences whose words overflowed without hyphenation.
    "Problems marked with a star require a proof. A counterexample must satisfy all of the hypotheses and must be checked explicitly. Prove each statement that is always true, and give a fully specified counterexample to each statement that is not always true.",
    "For each expression that need not be irrational, give an explicit counterexample and verify that all numbers in the example satisfy the hypotheses. Justify every step of the argument carefully and state clearly which definitions and previously established results are being used at each point.",
    "Hyphenation is the process of dividing words at line ends so that justified paragraphs keep an even texture. Liang's algorithm compresses a dictionary of hyphenated words into a compact set of patterns, and those patterns generalize remarkably well to vocabulary that never appeared in the training dictionary.",
    "The committee considered the environmental consequences of the proposed construction, including groundwater contamination, noise pollution, habitat fragmentation and the displacement of neighbouring communities, before recommending an independent assessment.",
    "Mathematicians frequently distinguish between constructive demonstrations, which exhibit a particular object with the required properties, and nonconstructive arguments, which merely establish that such an object must exist without identifying it explicitly.",
    "Interdisciplinary collaboration between computational linguists, cognitive psychologists and neuroscientists has produced increasingly sophisticated models of language comprehension, although many fundamental questions about representation remain unresolved.",
    "Typesetting systems measure every character, space and punctuation mark with extraordinary precision; consequently, a single additional letter can completely rearrange the line breaks of an otherwise unchanged paragraph.",
    "Photosynthesis converts electromagnetic radiation into chemical energy, storing it in carbohydrate molecules that plants subsequently metabolize. Respiration reverses the transformation, releasing the accumulated energy for growth, reproduction and maintenance.",
    "The international telecommunications infrastructure depends upon submarine cables, satellite constellations and terrestrial microwave relays, each of which introduces characteristic latencies, bandwidth limitations and maintenance requirements.",
    "Archaeological excavations near the river revealed extensive fortifications, ceremonial architecture and residential quarters, suggesting that the settlement was considerably larger and more politically centralized than historians had previously assumed.",
    "Well-known results in combinatorics, such as the pigeonhole principle and the inclusion-exclusion formula, reappear throughout probability theory, number theory and theoretical computer science in surprisingly diverse and elegant forms.",
    "A self-adjusting binary search tree reorganizes itself after every access, moving recently requested elements toward the root, so that frequently repeated queries become progressively cheaper without any explicit bookkeeping of access frequencies.",
    "Authors may also insert discretionary hyphens by hand, so that words such as un\\-believ\\-able, extra\\-ordinary and counter\\-intuitive are split only where indicated, regardless of what the patterns would otherwise suggest in running text.",
    "Constitutional scholars continue to debate the appropriate interpretation of ambiguous provisions, weighing historical understanding, textual analysis, institutional competence and the practical consequences of competing readings for contemporary governance.",
    "Thermodynamics establishes that no heat engine operating between two reservoirs can exceed the efficiency of a reversible Carnot cycle, a conclusion with profound implications for power generation, refrigeration and the ultimate fate of the universe.",
    "The orchestra rehearsed the symphony relentlessly, refining articulation, balancing sonorities and negotiating tempos until the conductor was satisfied that the performance communicated the composer's intentions with conviction and transparency.",
    "Epidemiological investigations require meticulous documentation of exposures, outcomes and potential confounding variables; otherwise, apparent associations between environmental factors and disease may reflect methodological artefacts rather than genuine causation.",
    "Contemporary cryptographic protocols rely on computational assumptions whose hardness has never been proven, yet decades of unsuccessful cryptanalysis provide substantial practical confidence in their security against realistic adversaries.",
    "Geological stratification records millions of years of sedimentation, volcanic activity and tectonic deformation, allowing specialists to reconstruct ancient climates, coastlines and ecosystems with remarkable chronological resolution.",
    "Programming language designers balance expressiveness against predictability: powerful abstractions accelerate development, but unrestricted metaprogramming can make large codebases extraordinarily difficult to understand, maintain and verify.",
    "Migration patterns of shorebirds span entire hemispheres, connecting Arctic breeding grounds with wintering sites along southern coastlines and depending critically on a chain of intermediate wetlands where the birds replenish their reserves.",
    "Linear algebra provides the foundational vocabulary of numerical computation: eigenvalues characterize stability, orthogonal decompositions enable least squares approximation, and sparse factorizations make enormous engineering simulations tractable.",
    "Philosophers of science question whether empirical adequacy suffices for theoretical acceptance or whether explanatory virtues such as simplicity, unification and fertility provide independent grounds for believing that a theory is approximately true.",
    "The municipal transportation authority announced comprehensive improvements to accessibility, including level boarding platforms, audible announcements, tactile paving and redesigned timetables that coordinate connections between bus and rail services.",
    "Statistical mechanics explains macroscopic regularities such as temperature, pressure and entropy as consequences of the collective behaviour of enormous numbers of microscopic constituents whose individual trajectories are practically unpredictable.",
    "Manuscripts submitted for publication undergo anonymous evaluation by independent referees, who assess originality, methodological rigour, clarity of presentation and the significance of the contribution relative to existing literature.",
    "Semiconductor fabrication demands extraordinary cleanliness: microscopic particles can destroy transistors whose dimensions approach atomic scales, so manufacturing facilities filter their atmosphere continuously and monitor contamination obsessively.",
    "Biodiversity conservation increasingly emphasizes landscape connectivity, recognizing that isolated reserves cannot sustain viable populations of wide-ranging species whose survival depends on movement between geographically separated habitats.",
    "Translation between natural languages involves far more than substituting vocabulary; idioms, grammatical structures, cultural references and stylistic conventions must all be reinterpreted so that the translated text remains natural and faithful.",
    "Distributed consensus algorithms tolerate the failure of individual machines by replicating state across several participants and requiring agreement from a majority before any modification becomes permanent, guaranteeing consistency despite unreliable networks.",
]

PREAMBLE = r"""\documentclass[12pt]{article}
\usepackage[T1]{fontenc}
\usepackage{times}
\usepackage[letterpaper,margin=1in]{geometry}
\setlength{\parindent}{0pt}
\showboxdepth=100 \showboxbreadth=1000000
\pagestyle{empty}
\begin{document}
"""


def main():
    tex = sys.argv[1] if len(sys.argv) > 1 else "/Library/TeX/texbin/pdflatex"
    body = [PREAMBLE]
    for i, para in enumerate(CORPUS):
        assert "\t" not in para and "\n" not in para
        body.append("\\setbox0\\vbox{%s\\par}\\showbox0\n" % para)
    body.append("\\end{document}\n")
    with tempfile.TemporaryDirectory() as tmp:
        with open(os.path.join(tmp, "corpus.tex"), "w") as f:
            f.write("".join(body))
        env = dict(os.environ, max_print_line="100000", error_line="254", half_error_line="238")
        subprocess.run([tex, "-interaction=nonstopmode", "corpus.tex"], cwd=tmp, env=env,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        log = open(os.path.join(tmp, "corpus.log"), encoding="latin-1").read()
    banner = log.splitlines()[0]
    chunks = log.split("> \\box0=")
    boxes = chunks[1:]
    # TeX writes a paragraph's Overfull warning before that paragraph's \showbox.
    overfull = [c.count("Overfull \\hbox") for c in chunks[:-1]]
    assert len(boxes) == len(CORPUS), (len(boxes), len(CORPUS))
    char_re = re.compile(r"^\.\.\\T1/ptm/m/n/12 (.*)$")
    out = ["# Generated by gen_hyphenation_oracle.py; oracle: " + banner]
    for para, box, over in zip(CORPUS, boxes, overfull):
        out.append("P\t" + para)
        if over:
            out.append("O\t%d" % over)
        lines, cur = [], None
        for raw in box.splitlines():
            if raw.startswith("! OK") or raw.startswith("l."):
                break
            if raw.startswith(".\\hbox("):
                if cur is not None:
                    lines.append(cur)
                cur = []
                continue
            if cur is None or not raw.startswith("..\\"):
                continue
            m = char_re.match(raw)
            if m:
                tok = m.group(1)
                lig = re.search(r"\(ligature (.*)\)$", tok)
                cur.append(lig.group(1) if lig else tok)
            elif raw.startswith("..\\glue") and not raw.startswith("..\\glue(\\"):
                cur.append(" ")
            elif raw.startswith("..\\discretionary") or raw.startswith("..\\kern") or \
                    raw.startswith("..\\penalty") or raw.startswith("..\\glue(\\") or \
                    raw.startswith("..\\hbox("):
                pass
            else:
                raise SystemExit("unhandled node in log: " + raw)
        if cur is not None:
            lines.append(cur)
        for ln in lines:
            out.append("L\t" + re.sub(" +", " ", "".join(ln)).strip())
    path = os.path.join(HERE, "hyphenation_corpus.txt")
    with open(path, "w") as f:
        f.write("\n".join(out) + "\n")
    print("wrote", path, len(CORPUS), "paragraphs")


if __name__ == "__main__":
    main()
