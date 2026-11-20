#!/usr/bin/env python3
"""Grammar-based generator of plain-TeX documents for pdfTeX parity fuzzing. MIT.

`generate(seed)` returns a `Doc`: a fixed preamble (INITEX + plain.tex, the
tracing switches) and a list of independent body chunks. Every choice goes
through one `random.Random(seed)`, so a seed is the whole test case. The
chunks are what the minimiser deletes; each chunk is closed on its own
(groups and modes balanced), so dropping one keeps the rest meaningful.

The grammar aims at rarely used corners of tex.web / pdftex.web / etex.ch:
paragraph shape and line breaking, hyphenation and discretionaries, math
lists, vertical splitting, inserts, marks, leaders, alignments, accents,
ligature/kern programs and boundary chars, font dimensions, the e-TeX
expressions and pdfTeX string primitives, the \\show commands and tracing,
error recovery, and arithmetic overflow. Stdlib only.
"""
import random

WORDS = [
    "office", "affluent", "baffle", "shuffling", "fjord", "efficient",
    "AVATAR", "Toward", "WAVY", "Yacht", "VAT", "LT", "flight", "fifth",
    "differentiation", "hyphenation", "incomprehensibilities",
    "representation", "characteristically", "nevertheless", "table",
    "quick", "brown", "fox", "jumps", "over", "lazy", "dog", "a", "I",
    "Ta", "Wo", "ff", "ffi", "ffl", "``quote''", "--", "---", "!`", "?`",
    "x-ray", "well-known", "co-operation", "Mississippi", "TeX",
]

FONTS = ["\\tenrm", "\\tenbf", "\\tenit", "\\tensl", "\\tentt", "\\sevenrm",
         "\\fiverm", "\\tenrm"]

DIMS = ["0pt", "1pt", "-1pt", "3pt", "10pt", "2em", "-2em", "1sp", "-1sp",
        "16383.99998pt", "-16383.99998pt", "0.5\\hsize", "1in", "12.5mm",
        "100bp", "3dd", "2cc", "1nd", "1nc", "7.3pt", "1ex", "-0.1pt"]
SMALLDIMS = ["0pt", "1pt", "2pt", "5pt", "10pt", "20pt", "-3pt", "1sp",
             "0.5em", "1.5em"]
GLUES = ["0pt", "0pt plus 1fil", "0pt plus 1fill", "3pt plus 1pt minus 1pt",
         "0pt minus 1fil", "1pt plus -1fil", "2pt plus 2filll",
         "0pt plus 100pt", "-2pt plus 1fil minus 1fil", "5pt minus 5pt",
         "0pt plus 2em", "1em plus 0.5em minus 0.3em"]
INTS = ["0", "1", "-1", "2", "3", "7", "10", "50", "100", "150", "200",
        "500", "1000", "5000", "9999", "10000", "10001", "-10000",
        "-10001", "2147483647", "-2147483647", "1073741823", "255", "256",
        "32767", "32768", "65535"]


PLAIN_TAIL = "\\par\\vfill\\penalty-20000\\end\n"
LATEX_TAIL = "\\par\\end{document}\n"


class Doc:
    """fmt None: INITEX + plain.tex; fmt "pdflatex": a LaTeX document."""

    def __init__(self, seed, preamble, chunks, pdf=True, fmt=None):
        self.seed = seed
        self.preamble = preamble
        self.chunks = chunks
        self.pdf = pdf
        self.fmt = fmt

    @property
    def tail(self):
        return LATEX_TAIL if self.fmt else PLAIN_TAIL

    def render(self, chunks=None):
        body = self.chunks if chunks is None else chunks
        out = [self.preamble]
        for c in body:
            out.append("%%FZ-CHUNK\n" + c.rstrip("\n") + "\n")
        out.append("%%FZ-END\n" + self.tail)
        return "".join(out)


class G:
    """One generator state: rng plus small helpers."""

    def __init__(self, rng):
        self.r = rng
        self.depth = 0
        self.latex = False

    def pick(self, seq):
        return self.r.choice(seq)

    def chance(self, p):
        return self.r.random() < p

    def n(self, lo, hi):
        return self.r.randint(lo, hi)

    def dim(self):
        return self.pick(DIMS)

    def sdim(self):
        return self.pick(SMALLDIMS)

    def glue(self):
        return self.pick(GLUES)

    def int_(self):
        return self.pick(INTS)

    def words(self, lo=3, hi=25):
        k = self.n(lo, hi)
        ws = []
        for _ in range(k):
            w = self.pick(WORDS)
            if self.chance(0.05):
                w = self.pick(["\\-", "\\/", "\\ ", "~", "\\noboundary ",
                               "\\discretionary{-}{}{}", "\\break ",
                               "\\nobreak ", "\\allowbreak ",
                               "\\kern1pt ", "\\penalty50 "]) + w
            ws.append(w)
        return " ".join(ws)

    # ---- inline material ------------------------------------------------
    def inline(self, budget=3):
        parts = []
        for _ in range(self.n(1, budget)):
            parts.append(self.inline_item())
        return " ".join(parts)

    def inline_item(self):
        self.depth += 1
        try:
            if self.depth > 4:
                return self.words(1, 4)
            k = self.n(0, 22)
            if k <= 5:
                return self.words()
            if k == 6:
                return "{%s %s}" % (self.pick(FONTS), self.words(1, 6))
            if k == 7:
                return self.disc()
            if k == 8:
                return self.inline_math()
            if k == 9:
                return self.accent()
            if k == 10:
                return self.leaders_h()
            if k == 11:
                return "\\vadjust{%s}" % self.pick([
                    "\\kern%s" % self.sdim(), "\\penalty-10000",
                    "\\hrule height%s" % self.sdim(),
                    "\\mark{va}", "\\marks2{vadj}", "\\vskip%s" % self.glue()])
            if k == 12:
                return self.mark()
            if k == 13:
                return self.insert()
            if k == 14:
                return "\\hbox%s{%s}" % (self.box_spec(), self.inline(2))
            if k == 15:
                return self.charstuff()
            if k == 16:
                return "\\hskip%s " % self.glue()
            if k == 17:
                return "\\penalty%s " % self.int_()
            if k == 18:
                return "\\vrule width%s height%s depth%s " % (
                    self.sdim(), self.sdim(), self.sdim())
            if k == 19:
                return "\\lower%s\\hbox{%s}" % (self.sdim(), self.words(1, 3))
            if k == 20:
                return "\\unhcopy%d " % self.n(10, 12)
            if k == 21:
                return "\\vtop{\\hsize3cm %s}" % self.words(2, 8)
            return "\\kern%s " % self.dim()
        finally:
            self.depth -= 1

    def box_spec(self):
        return self.pick(["", "", " to%s" % self.dim(), " spread%s" % self.sdim(),
                          " to0pt", " spread-5pt"])

    def disc(self):
        return self.pick([
            "\\discretionary{%s}{%s}{%s}" % (
                self.pick(["-", "f-", "", "\\kern1pt", "x"]),
                self.pick(["", "f", "l", "\\hbox{y}"]),
                self.pick(["", "ff", "\\kern2pt", "z"])),
            "o\\-f\\-f\\-i\\-c\\-e", "dif\\-fer\\-ent",
            "\\discretionary{}{}{}", "a\\discretionary{-}{}{} b",
            "{\\hyphenchar\\font=%s %s}" % (
                self.pick(["-1", "`-", "`=", "127", "256"]), self.words(3, 8)),
            "{\\exhyphenpenalty=%s x-y-z}" % self.int_(),
            "{\\exhyphenchar=%s ab-cd}" % self.pick(["-1", "`-", "`*"]),
            "{\\uchyph=%s Representation Hyphenation}" % self.pick(["0", "1", "-1"]),
            "{\\lefthyphenmin=%d \\righthyphenmin=%d characteristically}" % (
                self.n(0, 6), self.n(0, 6)),
            "{\\language=%d \\setlanguage%d differentiation}" % (
                self.n(0, 3), self.n(0, 3)),
        ])

    def accent(self):
        return self.pick([
            "\\accent%d %s" % (self.pick([18, 19, 20, 21, 22, 23, 94, 126, 127]),
                               self.pick(["a", "A", "i", "\\i", "o", "{}", "\\char`x"])),
            "\\'e\\`a\\^o\\\"u\\~n\\=o\\.z\\u g\\v c\\H o\\t oo\\c c\\d e\\b o",
            "\\accent127{\\sevenrm X}", "{\\tenit\\accent18 \\tenbf b}",
            "\\accent95\\kern1pt a", "\\accent22\\char`A",
            "\\accent127\\hbox{x}", "\\accent18 \\accent19 e",
        ])

    def charstuff(self):
        return self.pick([
            "\\char%s " % self.pick(["0", "`A", "\"7F", "255", "\"FF", "127", "11", "12"]),
            "f\\noboundary fi", "\\noboundary ffl\\noboundary",
            "ff\\/i", "f{}f{}i", "V\\kern0ptA", "A\\relax V", "{\\tentt ---}",
            "\\setbox0\\hbox{\\char`f\\char`f\\char`i}\\unhbox0",
            "\\chardef\\cX=%s\\cX" % self.pick(["66", "255", "0"]),
            "\\mathchar\"7141 ", "\\noboundary\\char`f\\char`l",
            "{\\tenrm\\char200}", "{\\lccode`A=`b \\lowercase{AAA}}",
            "\\uppercase{office}", "\\romannumeral%s " % self.int_(),
            "\\number\\catcode`\\{ ", "\\fontname\\font", "\\the\\fontdimen%d\\font"
            % self.n(1, 7), "\\the\\skewchar\\teni", "\\the\\hyphenchar\\font",
        ])

    def leaders_h(self):
        return "%s%s\\hskip%s " % (
            self.pick(["\\leaders", "\\cleaders", "\\xleaders"]),
            self.pick(["\\hrule", "\\hbox{.}", "\\hbox to 7pt{\\hss.\\hss}",
                       "\\hrule height1pt", "\\vrule", "\\hbox{}",
                       "\\copy10", "\\hbox{ab}", "\\vbox{\\hrule width3pt}"]),
            self.pick(["3cm", "0pt plus 1fil", "1in plus 1fill", "-2cm",
                       "2pt", "5cm minus 3cm", "0pt"]))

    def mark(self):
        return self.pick([
            "\\mark{m%d}" % self.n(0, 99), "\\marks%d{e%d}" % (self.n(0, 300), self.n(0, 9)),
            "\\mark{}", "\\marks0{zero}", "\\marks32767{big}", "\\marks32768{over}",
        ])

    def insert(self):
        return "\\insert%s{%s}" % (
            self.pick(["100", "100", "254", "200", "255", "0"]),
            self.pick(["\\hbox{ins}", "\\kern%s" % self.sdim(),
                       "\\noindent %s\\par" % self.words(2, 6),
                       "\\vskip%s" % self.glue(), "\\penalty-100",
                       "\\splittopskip=%s\\hrule height 30pt" % self.glue(),
                       "\\floatingpenalty=%s\\hbox{x}" % self.int_()]))

    # ---- math -----------------------------------------------------------
    def atom(self):
        k = self.n(0, 14)
        if k == 0:
            return "{%s\\over %s}" % (self.atom(), self.atom()) if self.depth < 4 else "x"
        if k == 1:
            return "{%s\\atop %s}" % ("a", "b")
        if k == 2:
            return "{%s\\above%s %s}" % ("a", self.sdim(), "b")
        if k == 3:
            return "{a\\%s%s%s b}" % (
                self.pick(["overwithdelims", "atopwithdelims"]),
                self.pick(["(", "[", "\\{", "|", ".", "\\langle"]),
                self.pick([")", "]", "\\}", "\\|", ".", "\\rangle"]))
        if k == 4:
            return "{a\\abovewithdelims()%s b}" % self.sdim()
        if k == 5:
            return "\\sqrt{%s}" % self.pick(["x", "x^2", "{a\\over b}", "\\sum_i", "\\big|"])
        if k == 6:
            return "\\radical\"%s{%s}" % (self.pick(["270370", "161", "0", "8000000", "FFFFFF"]),
                                         self.pick(["x", "\\displaystyle X", ""]))
        if k == 7:
            return "\\left%s %s \\middle%s %s \\right%s" % (
                self.pick(["(", "[", "\\lbrace", ".", "\\lfloor", "/", "\\uparrow", "<"]),
                self.pick(["x", "{a\\over b}", "\\sum", "\\vcenter{\\hrule height 30pt}"]),
                self.pick(["|", "\\|", "/", "\\backslash", ".", "\\Updownarrow"]),
                self.pick(["y", "\\int_0^1", "{1\\over2}"]),
                self.pick([")", "]", "\\rbrace", ".", "\\rceil", "\\backslash", ">"]))
        if k == 8:
            return "\\mathchoice{D}{T}{S}{SS}"
        if k == 9:
            return "%s_{%s}^{%s}" % (self.pick(["x", "\\sum", "\\int", "\\mathop{\\rm lim}",
                                                 "\\mathop{X}\\limits", "\\int\\nolimits",
                                                 "\\sum\\displaylimits"]),
                                      self.pick(["i", "i=1", "{n\\atop k}"]),
                                      self.pick(["n", "\\infty", "2^{2^2}"]))
        if k == 10:
            return self.pick(["\\overline{x}", "\\underline{y}", "\\vcenter{\\hbox{v}}",
                              "\\mathaccent\"7016 a", "\\hat{\\hat x}", "\\widehat{xyz}",
                              "\\mathpunct{,}", "\\mathinner{\\ldots}", "\\mathrel{=}\\mathrel{=}",
                              "\\mathbin{+}\\mathbin{+}", "\\mathopen(\\mathclose)", "\\mathord{-}"])
        if k == 11:
            return self.pick(["\\nonscript\\mskip%s" % self.pick(["5mu", "-3mu plus 2mu", "18mu"]),
                              "\\mkern%s" % self.pick(["3mu", "-1mu", "36mu"]),
                              "\\mskip\\thickmuskip", "\\nonscript\\hskip 3pt",
                              "\\nonscript\\kern2pt"])
        if k == 12:
            return "{%s %s}" % (self.pick(["\\displaystyle", "\\textstyle", "\\scriptstyle",
                                           "\\scriptscriptstyle"]), self.atom())
        if k == 13:
            return self.pick(["\\big(", "\\Bigg]", "\\bigl\\{", "\\delimiter\"4162362 ",
                              "\\mathchar\"8000 ", "\\fam1 x", "\\fam3 \\char0 ",
                              "\\mathchar\"7FFF "])
        return self.pick(["x", "y+z", "a=b", "\\alpha", "f(x)", "-1", "\\cdot", "\\to"])

    def mathlist(self, k=None):
        self.depth += 1
        try:
            return " ".join(self.atom() for _ in range(k or self.n(1, 5)))
        finally:
            self.depth -= 1

    def math_params(self):
        out = []
        for _ in range(self.n(0, 3)):
            out.append(self.pick([
                "\\mathsurround=%s" % self.sdim(), "\\scriptspace=%s" % self.sdim(),
                "\\nulldelimiterspace=%s" % self.sdim(),
                "\\delimiterfactor=%s" % self.pick(["901", "1000", "0", "-1"]),
                "\\delimitershortfall=%s" % self.sdim(),
                "\\thinmuskip=%s" % self.pick(["3mu", "0mu", "-2mu plus 1fil"]),
                "\\medmuskip=4mu plus 2mu minus 4mu",
                "\\binoppenalty=%s" % self.int_(), "\\relpenalty=%s" % self.int_(),
                "\\predisplaysize=%s" % self.dim(),
                "\\displaywidth=%s" % self.dim(), "\\displayindent=%s" % self.sdim(),
                "\\abovedisplayskip=%s" % self.glue(),
                "\\belowdisplayshortskip=%s" % self.glue(),
                "\\predisplaypenalty=%s" % self.int_(),
                "\\postdisplaypenalty=%s" % self.int_(),
                "\\fontdimen%d\\textfont2=%s" % (self.n(8, 22), self.sdim()),
                "\\fontdimen%d\\textfont3=%s" % (self.n(8, 13), self.sdim()),
                "\\mathcode`x=\"8000", "\\delcode`(=\"-1", "\\mathcode`+=\"2B2B",
            ]))
        return " ".join(out)

    def inline_math(self):
        return "{%s$%s$}" % (self.math_params(), self.mathlist())

    def display(self):
        return "%s %s$$%s%s$$ %s" % (
            self.words(2, 10), self.math_params(), self.mathlist(),
            self.pick(["", "", "\\eqno(1)", "\\leqno(2)", "\\eqno\\hbox to 3cm{x}",
                       "\\halign{#\\cr a\\cr}", "\\eqno{\\displaystyle X}"]),
            self.words(2, 10))

    # ---- paragraph shape ------------------------------------------------
    def par_params(self):
        out = []
        for _ in range(self.n(0, 5)):
            k = self.n(0, 23)
            if k == 0:
                n = self.n(1, 4)
                out.append("\\parshape %d %s" % (n, " ".join(
                    "%s %s" % (self.pick(["0pt", "1cm", "-1cm", "2in", "5pt"]),
                               self.pick(["3in", "2in", "1cm", "10cm", "0pt", "-1pt", "4in"]))
                    for _ in range(n))))
            elif k == 1:
                out.append("\\parshape %s" % self.pick(["0", "-1", "1 0pt 0pt"]))
            elif k == 2:
                out.append("\\hangindent=%s \\hangafter=%s" % (
                    self.pick(["1cm", "-1cm", "0pt", "3in", "-5in"]),
                    self.pick(["0", "1", "-1", "-3", "2", "10000", "-10000"])))
            elif k == 3:
                out.append("\\looseness=%s" % self.pick(["1", "-1", "2", "-2", "10", "-10"]))
            elif k == 4:
                out.append("\\emergencystretch=%s" % self.pick(["0pt", "1em", "3em", "100pt", "-1pt"]))
            elif k == 5:
                out.append("\\lastlinefit=%s" % self.pick(["0", "500", "1000", "1001", "-1", "200"]))
            elif k == 6:
                out.append("\\tolerance=%s" % self.pick(["100", "200", "1000", "9999", "10000", "-1"]))
            elif k == 7:
                out.append("\\pretolerance=%s" % self.pick(["-1", "100", "10000", "0"]))
            elif k == 8:
                out.append("\\hsize=%s" % self.pick(["2in", "3in", "1in", "0.5in", "4.5in", "50pt", "0pt"]))
            elif k == 9:
                out.append("\\rightskip=%s" % self.glue())
            elif k == 10:
                out.append("\\leftskip=%s" % self.glue())
            elif k == 11:
                out.append("\\parfillskip=%s" % self.glue())
            elif k == 12:
                out.append("\\%s=%s" % (self.pick(["adjdemerits", "linepenalty",
                                                    "doublehyphendemerits", "finalhyphendemerits",
                                                    "hyphenpenalty", "exhyphenpenalty",
                                                    "brokenpenalty", "interlinepenalty",
                                                    "clubpenalty", "widowpenalty"]),
                                         self.int_()))
            elif k == 13:
                out.append("\\parindent=%s" % self.dim())
            elif k == 14:
                out.append("\\spaceskip=%s \\xspaceskip=%s" % (self.glue(), self.glue()))
            elif k == 15:
                out.append("\\baselineskip=%s \\lineskiplimit=%s \\lineskip=%s" % (
                    self.glue(), self.sdim(), self.glue()))
            elif k == 16:
                out.append("\\pdfadjustspacing=%s" % self.pick(["0", "1", "2", "3"]))
            elif k == 17:
                out.append("\\pdfprotrudechars=%s" % self.pick(["0", "1", "2", "3"]))
            elif k == 18:
                out.append("\\sfcode`.=%s \\spacefactor=%s" % (self.pick(["1000", "3000", "0", "999", "32767"]),
                                                               self.pick(["1000", "1", "999"])))
            elif k == 19:
                out.append("\\frenchspacing" if self.chance(0.5) else "\\nonfrenchspacing")
            elif k == 20:
                out.append("\\interlinepenalties=%d %s" % (2, "100 -50") if self.chance(0.5)
                           else "\\clubpenalties=3 10000 5000 0")
            elif k == 21:
                out.append("\\pdfprotrudechars=2 \\rpcode\\tenrm`.=%s \\lpcode\\tenrm``=%s"
                           " \\rpcode\\tenrm`-=%s" % (self.pick(["500", "1000", "-500", "2000"]),
                                                     self.pick(["700", "1000", "-1000"]),
                                                     self.pick(["300", "1000"])))
            elif k == 22:
                out.append("\\pdfadjustspacing=2 \\pdfprotrudechars=2")
            else:
                out.append("\\pdfeachlinedepth=%s \\pdffirstlineheight=%s" % (
                    self.pick(["-1073741823sp", "2pt"]), self.pick(["-1073741823sp", "8pt"])))
        return " ".join(out)

    def paragraph(self):
        body = self.inline(4)
        if self.chance(0.2):
            body += " " + self.inline_math()
        end = self.pick(["\\par", "\\par", "\\par", "\\endgraf", "\\vadjust{}\\par",
                         "\\unskip\\par", "\\hfil\\break\\par", "\\penalty10000\\par",
                         "\\hskip0pt plus 1fil\\par", "$$\\displaystyle x$$\\par"])
        head = self.pick(["", "", "\\noindent ", "\\indent ", "\\noindent\\indent "])
        return "{%s\n%s%s%s}" % (self.par_params(), head, body, end)

    # ---- vertical -------------------------------------------------------
    def vlist(self):
        items = []
        for _ in range(self.n(1, 4)):
            items.append(self.pick([
                "\\hbox{%s}" % self.words(1, 5), "\\kern%s" % self.sdim(),
                "\\vskip%s" % self.glue(), "\\penalty%s" % self.int_(),
                "\\hrule height%s depth%s" % (self.sdim(), self.sdim()),
                "\\noindent %s\\par" % self.words(5, 20), "\\mark{v}",
                "\\marks1{vv}", "\\vbox to%s{}" % self.sdim(), "\\moveright%s\\hbox{r}" % self.sdim(),
                "\\leaders\\vrule\\vskip%s" % self.pick(["20pt", "0pt plus 1fil", "-3pt"]),
                "\\cleaders\\vbox{\\hrule\\vskip2pt}\\vskip 15pt",
                "\\xleaders\\vbox to 3pt{\\vss\\hrule}\\vskip 1cm",
                self.insert(), "\\vbox{\\hrule\\hbox{%s}}" % self.words(1, 2),
            ]))
        return "\n".join(items)

    def vsplit(self):
        return ("{\\setbox%d=\\vbox{\\hsize=%s \\splittopskip=%s \\splitmaxdepth=%s\n%s}"
                "\\setbox%d=\\vsplit%d to %s %s\n\\showbox%d \\showbox%d"
                " \\message{[\\splitfirstmark|\\splitbotmark|\\splitfirstmarks1|\\splitbotmarks1]}"
                "%s}") % (
            20, self.pick(["2in", "1in", "3in"]), self.glue(), self.pick(["0pt", "2pt", "16383.99998pt", "-1pt"]),
            self.vlist() + "\n" + self.words(10, 40) + "\\par" + self.vlist(),
            21, 20, self.pick(["0pt", "10pt", "20pt", "1in", "-5pt", "100pt", "16383pt"]),
            self.pick(["", "\\vbadness=0", "\\vfuzz=100pt", "\\savingvdiscards=1"]),
            20, 21,
            self.pick(["", "\\box21 \\box20", "\\setbox22\\vbox{\\unvbox20}\\showbox22",
                       "\\showbox\\pagediscards \\showbox\\splitdiscards" if False else
                       "\\setbox22\\vbox{\\pagediscards\\splitdiscards}\\showbox22"]))

    def align(self):
        cols = self.n(1, 4)
        tab = lambda: self.pick(["", "\\tabskip=%s" % self.glue()])
        pre = "".join("%s#%s%s&" % (self.pick(["", "\\hfil", "\\hfill", "$", "\\bf "]),
                                     self.pick(["", "\\hfil", "$", "\\/"]) if True else "",
                                     tab()) for _ in range(cols - 1))
        pre += "%s#%s" % (self.pick(["", "\\hfil"]), self.pick(["", "\\hfil"]))
        if pre.count("$") % 2:
            pre = pre.replace("$", "")
        if self.chance(0.3):
            pre = "&" + pre if self.chance(0.5) else pre + "&&\\hfil#"
        rows = []
        for _ in range(self.n(1, 5)):
            cells = []
            for _ in range(self.n(1, cols + 1)):
                cells.append(self.pick(["a", "bb", "\\omit c", "\\omit\\hfil d", "%s" % self.words(1, 3),
                                        "\\span x", "\\omit\\span\\omit wide", "\\hfill e",
                                        "\\vrule", "{f}", "\\hbox to 1cm{}"]))
            row = "&".join(cells) + "\\cr"
            if self.chance(0.3):
                row += "\\noalign{%s}" % self.pick(["\\hrule", "\\kern3pt", "\\vskip 2pt plus 1fil",
                                                    "\\penalty-100", "\\hrule height 2pt\\kern1pt",
                                                    "\\mark{al}"])
            rows.append(row)
        to = self.pick(["", "", " to%s" % self.pick(["3in", "1cm", "0pt", "10in"]),
                        " spread%s" % self.pick(["10pt", "-10pt", "1in"])])
        if self.chance(0.3):
            # \valign: columns are vertical lists
            vrows = []
            for _ in range(self.n(1, 3)):
                vrows.append("&".join(self.pick(["\\hbox{v}", "\\vskip3pt", "\\hrule", "\\omit\\hbox{o}"])
                                      for _ in range(self.n(1, 3))) + "\\cr")
            return "\\hbox{\\valign%s{%s#\\vfil%s&\\hbox{#}\\cr %s}}%s" % (
                self.pick(["", " to 1in", " spread 5pt"]), self.pick(["\\tabskip=3pt", ""]),
                self.pick(["", "\\tabskip=1pt plus 1fil"]), "\n".join(vrows),
                self.pick(["", "\\showlists"]))
        return "{%s\\halign%s{%s\\cr\n%s}}%s" % (
            self.pick(["", "\\tabskip=%s " % self.glue(), "\\everycr{\\noalign{\\kern1pt}}"]),
            to, pre, "\n".join(rows), self.pick(["", "\\showlists", "\\par"]))

    # ---- arithmetic -----------------------------------------------------
    def expr(self):
        k = self.n(0, 9)
        if k == 0:
            return "\\the\\numexpr %s\\relax" % self.pick([
                "7/2", "-7/2", "7/-2", "(7*11)/3", "2147483647+1", "2147483647*2/3",
                "-2147483647-1", "1/0", "65536*32768", "(1+2", "3*(4+5)*-1",
                "2147483647/2*2", "-(-5)", "\\maxdimen", "0*1/0", "--3", "(((((1)))))"])
        if k == 1:
            return "\\the\\dimexpr %s\\relax" % self.pick([
                "1pt*3/2", "16383.99999pt+1sp", "16383.99999pt*2", "-1pt/3", "1pt/0",
                "(1pt+2pt)*(3)", "1sp*2147483647", "1pt*2/4", "\\hsize/7*7", "7sp/2",
                "-7sp/2", "1pt*1.5", "1em-1ex", "1pt/-3"])
        if k == 2:
            return "\\the\\glueexpr %s\\relax" % self.pick([
                "1pt plus 2fil*3", "1pt plus 1fil + 2pt plus 1fill", "3pt minus 1pt/2",
                "\\baselineskip*2", "1pt plus 1fil - 1pt plus 1fil", "0pt plus 16383.99999fil*2",
                "1pt plus 2pt minus 3pt/0"])
        if k == 3:
            return "\\the\\muexpr %s\\relax" % self.pick(["3mu*2", "1mu plus 1fill/3", "\\thinmuskip+\\medmuskip"])
        if k == 4:
            return "{\\count1=%s \\%s\\count1 by %s \\message{\\the\\count1}}" % (
                self.int_(), self.pick(["multiply", "divide", "advance"]),
                self.pick(["0", "2", "-1", "2147483647", "-2147483647", "65536", "3"]))
        if k == 5:
            return "{\\dimen1=%s \\%s\\dimen1 by %s \\message{\\the\\dimen1}}" % (
                self.dim(), self.pick(["multiply", "divide"]),
                self.pick(["0", "2", "-1", "3", "32768", "100000"]))
        if k == 6:
            return "{\\skip1=%s \\%s\\skip1 by %s \\message{\\the\\skip1}}" % (
                self.glue(), self.pick(["multiply", "divide", "advance"]),
                self.pick(["0", "2", "-1", "3", "32768"]) if self.chance(0.6) else "1pt plus -1fil")
        if k == 7:
            return "\\the\\gluestretch\\glueexpr%s\\relax\\the\\gluestretchorder\\skip3 \\the\\glueshrink\\baselineskip" % self.glue()
        if k == 8:
            return "\\the\\gluetomu\\skip2 \\the\\mutoglue\\thinmuskip\\the\\numexpr\\dimexpr 1pt\\relax\\relax"
        return "\\dimen3=%s\\the\\dimen3 \\count3=%s\\the\\count3 " % (
            self.pick(["16384pt", "16383.999999pt", "-16384pt", "1.000000001pt", "0.00001pt",
                       "99999999sp", "1073741823sp", "1073741824sp", "18in", "1000in", "8191.99999cc"]),
            self.pick(["2147483647", "2147483648", "-2147483648", "\"7FFFFFFF", "\"80000000",
                       "'17777777777", "`\\^^@", "999999999999"]))

    def strprim(self):
        return self.pick([
            "\\message{[\\pdfstrcmp{%s}{%s}]}" % (self.pick(["a", "b", "", "ab", "\\relax"]),
                                                  self.pick(["a", "b", "", "aa", "\\relax "])),
            "\\message{[\\pdfescapestring{%s}]}" % self.pick(["a(b)c", "\\\\x", "^^M^^J", "%%", "\\%", "~#"]),
            "\\message{[\\pdfescapename{%s}]}" % self.pick(["a b#c", "/x", "^^80^^ff"]),
            "\\message{[\\pdfescapehex{%s}]}" % self.pick(["AZ", "^^00", ""]),
            "\\message{[\\pdfunescapehex{%s}]}" % self.pick(["414", "4A4b", "zz41", ""]),
            "\\message{[\\pdfmdfivesum{abc}]}", "\\message{[\\pdfstrcmp{\\noexpand\\foo}{\\string\\foo}]}",
            "\\message{[\\detokenize{a#b \\x}]}", "\\message{[\\unexpanded{\\x#}]}",
            "\\edef\\a{\\pdfstrcmp{x}{y}}\\show\\a", "\\showtokens{a#b\\par}",
            "\\message{\\pdffontname\\tenrm\\pdffontobjnum\\tenrm\\pdffontsize\\tenrm}",
            "\\message{\\the\\pdflastxpos\\the\\pdftexversion\\pdftexrevision\\the\\eTeXversion\\eTeXrevision}",
        ])

    # ---- fonts ----------------------------------------------------------
    def fontstuff(self):
        return self.pick([
            "\\font\\fza=cmr10 at %s \\fza %s" % (self.pick(["13.7pt", "5pt", "0.5pt", "2048pt", "-1pt"]), self.words(2, 8)),
            "\\font\\fzb=cmr10 scaled %s \\fzb %s" % (self.pick(["1440", "500", "0", "32768", "33000"]), self.words(2, 5)),
            "{\\fontdimen%d\\tenrm=%s %s}" % (self.n(1, 7), self.sdim(), self.words(4, 12)),
            "\\font\\fzc=cmr9 \\fontdimen%d\\fzc=1pt \\message{\\the\\fontdimen%d\\fzc}" % (
                self.n(8, 12), self.n(1, 12)),
            "\\font\\fzd=cmtt10 \\hyphenchar\\fzd=`- \\skewchar\\fzd=`* {\\fzd %s}" % self.words(2, 6),
            "\\font\\fze=cmr10 at 2048pt \\fze x",
            "\\pdffontexpand\\tenrm %s %s %s autoexpand \\pdfadjustspacing=2 {%s\\par}" % (
                self.pick(["20", "30", "50", "1000", "0"]), self.pick(["20", "10", "0"]),
                self.pick(["5", "1", "10"]), self.words(15, 40)),
            "{\\letterspacefont\\fzl\\tenrm %s \\fzl %s}" % (self.pick(["100", "-50", "1000", "0"]), self.words(2, 5)),
            "{\\pdfcopyfont\\fzk\\tenrm \\fzk %s}" % self.words(2, 4),
            "{\\pdfnoligatures\\tenrm %s}" % self.words(3, 6),
            "{\\tagcode\\tenrm`a=%s \\pdfprependkern=1 \\pdfappendkern=1 \\knbscode\\tenrm`a=50 %s}" % (
                self.int_(), self.words(2, 6)),
            "{\\pdfinterwordspaceon %s\\pdfinterwordspaceoff}" if False else "{\\efcode\\tenrm`o=%s %s}" % (
                self.pick(["500", "1000", "-1000", "0"]), self.words(3, 8)),
        ])

    # ---- show / tracing / errors ---------------------------------------
    def showstuff(self):
        return self.pick([
            "\\setbox%d\\hbox{%s}\\showbox%d" % (30, self.inline(2), 30),
            "\\setbox%d\\vbox{\\hsize=%s %s}\\showbox%d" % (31, self.pick(["2in", "1in", "100pt"]), self.paragraph(), 31),
            "\\showlists", "{\\showboxdepth=%s \\showboxbreadth=%s \\setbox32\\vbox{%s}\\showbox32}" % (
                self.pick(["-1", "0", "1", "2"]), self.pick(["-1", "0", "1", "3"]), self.vlist()),
            "\\showthe\\%s" % self.pick(["hsize", "parshape", "baselineskip", "lastpenalty", "lastskip",
                                         "lastkern", "badness", "inputlineno", "spacefactor",
                                         "prevdepth", "prevgraf", "pagegoal", "pagetotal",
                                         "lastnodetype", "currentgrouplevel", "interactionmode",
                                         "pdflastxform", "fontcharwd\\tenrm`A", "fontcharic\\tenit`f",
                                         "parshapelength1", "parshapeindent2", "parshapedimen3",
                                         "numexpr1+2", "everypar", "lastlinefit"]),
            "\\showgroups", "\\showifs", "\\show\\par", "\\meaning\\undefinedzz",
            "{\\tracingonline=%s \\tracingcommands=%s \\tracingparagraphs=1 \\hsize=2in %s\\par}" % (
                self.pick(["0", "1", "-1", "2"]), self.pick(["0", "1", "2", "3"]), self.words(20, 50)),
            "\\begingroup\\tracingassigns=1 \\tracingrestores=1 \\count1=5 \\global\\count1=6 \\endgroup",
            "\\setbox33\\hbox{%s}\\message{\\the\\wd33,\\the\\ht33,\\the\\dp33,\\the\\badness}" % self.inline(2),
        ])

    def errorstuff(self):
        return self.pick([
            "\\undefinedcontrolsequence", "\\hbox{$x}", "x}", "{\\catcode`\\^^M=13 }",
            "\\hbox to 1pt{\\vskip 2pt}", "\\vbox{\\moveleft2pt}", "\\count1=\\dimen1 ",
            "\\dimen1=3 \\relax", "\\dimen1=3zz", "\\skip1=1pt plus 1fillll", "$\\par$",
            "\\endcsname", "\\else", "\\fi", "\\or", "\\char256", "\\char-1",
            "\\catcode300=1 ", "\\penalty", "\\setbox300\\hbox{}", "\\box-1",
            "\\hrule\\vrule", "\\halign{#\\cr a&b\\cr}", "\\halign{#&#\\cr\\noalign{x}}",
            "\\left(", "$\\right)$", "$\\middle|$", "$a\\over b\\over c$", "$x^1^2$", "$x_1_2$",
            "\\insert255{x}", "\\vadjust{\\vbox}", "\\accent 300 x", "\\mathchar\"8000x",
            "\\showbox-5", "\\unhbox\\voidb@x", "\\unvbox30 ", "\\lastbox", "\\vsplit30 to 1pt",
            "\\patterns{a1b}", "\\hyphenation{fo-o-}", "\\hyphenation{a\\relax b}", "\\errmessage{custom}",
            "\\errhelp{help}\\errmessage{h}", "\\the\\toks-1", "\\def\\x#2{}", "\\outer\\def\\o{}\\def\\y{\\o}",
            "\\csname\\relax\\endcsname", "\\expandafter\\ifx\\noexpand\\zz\\relax T\\fi",
            "\\numexpr 1 2\\relax", "\\dimexpr 1pt*\\relax", "\\interactionmode=%s" % self.pick(["0", "1", "2", "4"]),
            "\\batchmode", "\\nonstopmode", "\\scrollmode",
            "{\\setbox0\\hbox{\\vrule height 1pt}\\setbox0\\copy0 \\wd0=-5pt \\box0}",
            "\\hbox{\\hskip 0pt minus 1fil\\hskip 0pt plus -1fil}",
            "\\hbox to 10pt{\\hskip 1pt plus -1fil}", "\\hbox to -1pt{x}",
            "\\vbox to 1pt{\\vskip 5pt minus 1pt}", "\\hbox to 1pt{\\hskip 1pt minus -1fil}",
        ])

    def output_routine(self):
        return self.pick([
            "\\output={\\message{[top:\\topmark|first:\\firstmark|bot:\\botmark|tm1:\\topmarks1|fm2:\\firstmarks2|bm300:\\botmarks300]}"
            "\\showbox255 \\shipout\\box255 \\ifvoid100 \\else\\showbox100 \\setbox0\\box100 \\fi}",
            "\\output={\\setbox0\\vbox{\\unvbox255 \\showlists}\\shipout\\box0 \\global\\setbox100\\box100}",
            "\\output={\\global\\holdinginserts=1 \\showbox255 \\shipout\\box255 }",
            "\\output={\\shipout\\vbox{\\box255 \\box100 \\message{\\the\\outputpenalty,\\the\\insertpenalties}}}",
            "\\output={\\plainoutput}",
            "\\vsize=%s \\maxdepth=%s" % (self.pick(["1in", "2in", "50pt", "10pt"]), self.sdim()),
            "\\count100=%s \\dimen100=%s \\skip100=%s" % (self.pick(["500", "1000", "0", "2000"]),
                                                          self.pick(["1in", "10pt", "0pt", "\\maxdimen"]),
                                                          self.glue()),
            "\\pagegoal=%s\\pagetotal=%s\\pageshrink=%s" % (self.dim(), self.sdim(), self.sdim()),
            "\\topskip=%s \\splittopskip=%s \\vfuzz=%s \\vbadness=%s" % (self.glue(), self.glue(), self.sdim(), self.int_()),
            "\\shipout\\hbox{%s}" % self.inline(2),
            "\\shipout\\vbox{\\hsize=1in %s}" % self.paragraph(),
            "\\deadcycles=%s" % self.pick(["0", "24", "25", "100"]),
            "\\pdfpagewidth=%s \\pdfpageheight=%s" % (self.pick(["0pt", "3in", "8.5in", "-1in"]), self.pick(["0pt", "5in", "11in"])),
            "\\pdfhorigin=%s \\pdfvorigin=%s" % (self.pick(["0pt", "1in", "-1in"]), self.pick(["0pt", "1in", "2in"])),
            "\\hoffset=%s \\voffset=%s" % (self.sdim(), self.sdim()),
        ])

    def pdfstuff(self):
        return self.pick([
            "\\pdfliteral{%s}" % self.pick(["0 0 m 10 10 l S", "q Q", "", "BT ET"]),
            "\\pdfliteral direct{0 g}", "\\pdfliteral page{1 0 0 1 0 0 cm}",
            "\\special{%s}" % self.pick(["pdf:literal 0 g", "color push Black", "papersize=3in,4in", "", "ps: 1 1 moveto"]),
            "\\immediate\\pdfobj{<< /A 1 >>}\\message{\\the\\pdflastobj}",
            "\\pdfobj reserveobjnum \\message{\\the\\pdflastobj}",
            "\\setbox40\\hbox{%s}\\immediate\\pdfxform40 \\pdfrefxform\\pdflastxform" % self.words(1, 3),
            "\\setbox41\\hbox{x}\\pdfxform attr{/Q 1} resources{/R 2} 41 \\pdfrefxform\\pdflastxform",
            "\\pdfdest num %d xyz \\pdfdest name{d%d} fit" % (self.n(1, 9), self.n(0, 9)),
            "\\pdfannot width 10pt height 5pt depth 1pt {/Subtype /Text}",
            "\\pdfstartlink attr{/Border[0 0 0]} goto num 1 %s\\pdfendlink" % self.words(1, 8),
            "\\pdfoutline goto num 1 count %d {Out%d}" % (self.n(-2, 2), self.n(0, 9)),
            "\\pdfinfo{/Title (fz)}\\pdfcatalog{/PageMode /UseNone}\\pdfnames{/X 1}\\pdftrailer{/Y 2}",
            "\\pdfsavepos\\message{\\the\\pdflastxpos,\\the\\pdflastypos}",
            "\\pdfrefobj%d " % self.n(1, 3), "\\pdfthread num %d " % self.n(1, 3),
            "\\pdfsetmatrix{0.5 0 0 0.5}\\pdfsave\\pdfrestore", "\\pdfcolorstack0 push{0 g}\\pdfcolorstack0 pop",
            "\\pdfpageattr{/Rotate 90}\\pdfpagesattr{/X 1}\\pdfpageresources{/Y 2}",
            "\\pdfsnaprefpoint\\pdfsnapy 2pt", "\\pdfspacefont\\tenrm\\pdfinterwordspaceon %s\\pdfinterwordspaceoff" % self.words(2, 4),
            "\\pdfmapline{+cmr10 CMR10 <cmr10.pfb}", "\\pdfgentounicode=1 \\pdfglyphtounicode{f}{0066}",
            "\\pdfresettimer\\message{\\the\\pdflastlinkdepth}", "\\pdfprimitive\\relax \\ifpdfprimitive\\par T\\fi",
            "\\pdfsetrandomseed %d \\message{\\pdfuniformdeviate 100 \\pdfnormaldeviate}" % self.n(0, 99999),
        ])

    def iostuff(self):
        return self.pick([
            "\\immediate\\openout3=fzout.tex \\immediate\\write3{\\relax a#b}\\immediate\\closeout3 "
            "\\openin4=fzout \\read4 to\\x \\show\\x \\readline4 to\\y \\show\\y \\ifeof4 E\\fi \\closein4",
            "\\openin5=nonexistentzz \\message{\\ifeof5 eof\\else open\\fi}",
            "\\write16{%s}\\write-1{log only}\\immediate\\write17{term}" % self.words(1, 4),
            "{\\newlinechar=`\\^^J \\message{a^^Jb}\\immediate\\write16{c^^Jd}}",
            "\\everyeof{\\noexpand}\\edef\\z{\\scantokens{ab\\x}}\\show\\z",
            "\\scantokens{\\message{st}\\par}", "\\everyeof{}\\scantokens{x}",
            "\\openout7=fzw \\write7{deferred \\the\\count0}\\closeout7 ",
            "\\message{\\jobname,\\the\\inputlineno,\\the\\lastlinefit}",
            "\\uppercase{\\message{abc}}\\lowercase{\\message{XYZ}}",
            "\\def\\a#1#{[#1]}\\message{\\a x{}}", "\\long\\def\\b#1\\par#2{#2#1}\\message{\\b u\\par v}",
            "\\protected\\def\\p{P}\\edef\\q{\\p\\p}\\show\\q", "\\showtokens\\expandafter{\\jobname}",
            "\\ifdefined\\undefinedq D\\else U\\fi \\ifcsname relax\\endcsname C\\fi \\iffontchar\\tenrm`a F\\fi",
            "\\message{\\ifcase\\numexpr 2\\relax a\\or b\\or c\\else d\\fi}", "\\unless\\ifx aa N\\else Y\\fi",
            "\\interactionmode=1 \\message{\\the\\interactionmode}\\interactionmode=2 ",
            "\\showgroups{\\begingroup\\showgroups\\endgroup}", "\\middle|", "\\pdfstrcmp{a}",
        ])

    def charset(self):
        """8-bit and control characters through printing, \\write and lost-char tracing."""
        return self.pick([
            "\\message{^^e9^^ff^^80^^7f^^00^^1f}", "\\showtokens{^^e9^^M^^I a}",
            "\\immediate\\write16{^^e9^^a0 x^^7f}", "\\immediate\\write-1{^^c3^^a9}",
            "{\\tracinglostchars=%s \\tenrm ^^e9^^80\\char200 \\char`\\^^A x}" % self.pick(["1", "2", "3"]),
            "\\catcode`^^e9=11 \\def\\^^e9{E}\\show\\^^e9", "\\catcode`\\^^@=12 \\message{a^^@b}",
            "{\\escapechar=%s \\message{\\string\\par\\meaning\\relax}}" % self.pick(["-1", "`\\^^e9", "`/", "256", "10"]),
            "{\\newlinechar=%s \\message{a^^Jb%%c}\\immediate\\write16{x^^Jy}}" % self.pick(["`\\^^J", "`\\%", "-1", "`a"]),
            "{\\endlinechar=%s \\scantokens{a\n b}}" % self.pick(["-1", "`A", "13", "256"]),
            "\\message{%s}" % ("long" * self.n(200, 300)),
            "\\message{%s\\undefinedlongline}" % ("x" * self.n(240, 600)),
            "\\hbox{%s\\undefinedB %s}" % (" ".join(self.pick(WORDS) for _ in range(60)),
                                              " ".join(self.pick(WORDS) for _ in range(60))),
            "\\uccode`^^e9=`^^c9 \\uppercase{\\message{^^e9}}",
        ])

    def capacity(self):
        """Runs that end in an overflow or fatal error (rare; ends the document)."""
        return self.pick([
            "\\def\\ovA{\\ovA x}\\ovA", "\\def\\ovG{{\\ovG}\\ovG",
            "\\def\\ovP#1{\\ovP{#1#1}}\\ovP x", "\\def\\ovL{\\ovL\\relax}\\setbox0\\hbox{\\ovL}",
            "\\def\\ovS{\\begingroup\\count1=1 \\ovS}\\ovS",
            "\\def\\ovI{\\ifnum1=1 \\ovI\\fi}\\ovI",
            "\\def\\ovC{\\csname\\ovC\\endcsname}\\ovC",
            "\\count1=0 \\def\\ovH{\\advance\\count1 1 \\expandafter\\def\\csname h\\the\\count1\\endcsname{}\\ovH}\\ovH",
            "\\errorstopmode\\undefinedatend", "\\errorstopmode\\read16 to\\x",
            "\\endinput", "\\input nonexistentfilezz ", "\\batchmode\\input nonexistentfilezz ",
            "{\\fz", "\\def\\run#1{#1}\\run{\\par", "\\outer\\def\\ou{}\\message{\\ou}",
            "\\count1=0 {\\loop\\advance\\count1 1 \\ifnum\\count1<120 \\undefinedErr\\repeat}",
        ])

    def web2cstuff(self):
        """pdfTeX's C parts and web2c: regex matching, file primitives, file
        names, the TeX Live feature change files, pdfTeX's newer parameters."""
        pats = ["a(b*)c", "^x", "x$", "[[:digit:]]+", "(a|b)+", "a{2,3}", "(", "[",
                "a**", "\\\\(", "", ".*", "(x)(y)?(z)", "[^a-c]", "\\\\.", "a|", "()",
                "([a-z]+)@([a-z]+)", "^$", "(((a)))"]
        subj = ["xabbbcx", "abc", "", "x", "aaa", "123abc456", "bab", "a.b", "xyz",
                "me@host", "((", "^^e9a", "ab ab"]
        def match():
            opts = self.pick(["", "icase ", "subcount %d " % self.n(-1, 12), "icase subcount 2 "])
            m = "\\message{[\\pdfmatch %s{%s}{%s}]}" % (opts, self.pick(pats), self.pick(subj))
            for _ in range(self.n(0, 3)):
                m += "\\message{(\\pdflastmatch%d)}" % self.n(-1, 6)
            return m
        return self.pick([
            match(), match(), match(),
            "\\message{\\pdffilesize{doc.tex}/\\pdffilesize{nonexistentzz}/\\pdffilesize{}}",
            "\\message{\\pdffiledump offset %d length %d{doc.tex}}" % (self.n(-2, 40), self.n(-1, 30)),
            "\\message{\\pdffiledump length 5{nonexistentzz}|\\pdffiledump{doc.tex}}",
            "\\message{\\pdfmdfivesum file{doc.tex}|\\pdfmdfivesum file{nonexistentzz}}",
            "\\message{\\pdffilemoddate{doc.tex}|\\pdffilemoddate{nonexistentzz}|\\pdfcreationdate}",
            "\\immediate\\openout3=\"fz out.tex\" \\immediate\\write3{\\relax q}\\immediate\\closeout3 "
            "\\openin4=\"fz out\" \\read4 to\\x \\show\\x \\closein4 \\input{\"fz out\"}",
            "\\immediate\\openout3={fzb.tex}\\immediate\\write3{\\message{in-fzb}}\\immediate\\closeout3 "
            "\\input{fzb}\\input fzb.tex \\relax",
            "\\immediate\\openout3=fzc.x.y \\immediate\\write3{c}\\immediate\\closeout3 "
            "\\message{\\pdffilesize{fzc.x.y}}\\openin5=fzc.x \\ifeof5 E\\else O\\closein5\\fi",
            "\\pdfximage{nonexistentzz.png}", "\\pdfximage width 1pt{doc.tex}",
            "{\\tracingstacklevels=%d \\tracingmacros=2 \\def\\ta{\\tb}\\def\\tb{x}\\ta}" % self.n(0, 3),
            "{\\showstream=%d \\showthe\\count0 \\show\\par}" % self.pick([-1, 16, 3]),
            "{\\partokenname\\fzpar \\message{\\meaning\\fzpar}\\def\\fzq#1{}\\fzq\n\n}",
            "{\\partokencontext=%d \\partokenname\\fzpar \\noindent x\\par\\showlists}" % self.n(0, 2),
            "\\setbox3\\hbox{x\\kern1pt}\\message{\\the\\lastnodetype}"
            "\\hbox{x\\kern1pt\\message{\\the\\lastnodetype}\\penalty0\\message{\\the\\lastnodetype}}",
            "\\message{\\the\\pdfinsertht%d}" % self.pick([100, 254, 255, 0]),
            "\\pdfstartthread num %d \\pdfendthread" % self.n(1, 4),
            "\\message{\\pdfcolorstackinit page{0 g}|\\pdfcolorstackinit direct{1 g}}",
            "\\message{\\the\\pdfshellescape|\\pdftexbanner|\\the\\pdftexversion}",
            "\\quitvmode\\message{\\the\\lastnodetype}",
            "{\\pdfdecimaldigits=%d \\pdfliteral{q}\\pdfsetmatrix{0.12345 0 0 0.6789}\\hbox{x}}" % self.n(-1, 6),
            "{\\pdfpxdimen=%s \\message{\\the\\pdfpxdimen}}" % self.pick(["1pt", "-1pt", "0pt", "1sp"]),
            "{\\pdfadjustinterwordglue=1 \\knbscode\\tenrm`a=%d \\stbscode\\tenrm`a=%d \\shbscode\\tenrm`a=%d "
            "\\hbox{a b a}\\showbox0}" % (self.n(-1000, 1000), self.n(-1000, 1000), self.n(-1000, 1000)),
            "{\\pdfprependkern=1 \\pdfappendkern=1 \\knbccode\\tenrm`a=%d \\knaccode\\tenrm`a=%d "
            "\\setbox5\\hbox{aba a}\\showbox5}" % (self.n(-500, 500), self.n(-500, 500)),
            "{\\pdftracingfonts=%d \\setbox5\\hbox{\\tenbf x}\\showbox5}" % self.n(0, 2),
            "\\pdfmapfile{%snonexistentzz.map}" % self.pick(["+", "", "=", "-"]), "\\pdfmapline{=cmr10 CMR10 <cmr10.pfb}",
            "\\pdfincludechars\\tenrm{abc}\\pdfomitcharset 1 ",
            "{\\pdffakespace\\pdfinterwordspaceon x y\\pdfinterwordspaceoff}",
            "\\message{\\pdfescapestring{(a)\\\\b}|\\pdfescapename{a b#/}|\\pdfescapehex{^^ff}}",
            "\\message{\\pdfstrcmp{\\relax}{\\relax}\\pdfstrcmp{^^e9}{e}}",
            "\\pdfsetrandomseed %d \\message{\\pdfuniformdeviate %s}" % (
                self.n(0, 9), self.pick(["0", "-5", "2147483647", "1"])),
            "\\pdfsetrandomseed %s \\message{\\the\\pdfrandomseed}" % self.pick(["0", "-1", "2147483647", "123"]),
        ])

    def pdfwriter(self):
        """pdfTeX's PDF writer (C parts): objects, streams, annotations, boxes,
        version and info keys, image errors; and web2c's openout_any/openin_any."""
        return self.pick([
            "\\immediate\\pdfobj stream attr{/T 1}{q 0 g Q}\\pdfrefobj\\pdflastobj",
            "\\immediate\\pdfobj file{doc.tex}\\message{\\the\\pdflastobj}",
            "\\immediate\\pdfobj stream file{nonexistentzz}",
            "\\pdfobj reserveobjnum \\immediate\\pdfobj useobjnum\\pdflastobj{(late)}",
            "\\immediate\\pdfobj useobjnum %d{(bad)}" % self.n(-1, 40),
            "\\pdfannot reserveobjnum \\pdfannot useobjnum\\pdflastannot width 2pt{/Subtype/Text}",
            "\\message{\\the\\pdflastannot|\\the\\pdflastlink|\\the\\pdfretval}",
            "\\pdfstartlink user{/Subtype/Link/A<</S/URI/URI(http://x)>>}%s\\pdfendlink" % self.words(1, 30),
            "\\pdfstartlink goto name{d%d}%s\\pdfendlink" % (self.n(0, 9), self.words(1, 4)),
            "\\pdfstartlink goto file{x.pdf} page %d{/Fit}x\\pdfendlink" % self.n(-1, 3),
            "\\pdfstartlink thread num 1 x\\pdfendlink", "\\pdfendlink",
            "\\vbox{\\pdfstartlink goto num 2 x\\pdfendlink}",
            "\\pdfdest name{d%d} %s" % (self.n(0, 9), self.pick(["xyz zoom 2000", "fitb", "fith", "fitv", "fitbh",
                                                              "fitbv", "fitr width 10pt height 5pt", "xyz"])),
            "\\pdfdest num %d fit" % self.pick([0, -1, 1, 2147483647]),
            "\\pdfoutline user{<</Type/Outlines>>} count 0 {U}",
            "\\pdfoutline goto name{d1} {(paren) \\\\ back ^^e9}",
            "\\pdfthread name{t%d} width 5pt{/I<</T(x)>>}" % self.n(0, 2),
            "\\pdfinfo{/Author (a)}\\pdfinfo{/Author (b)}\\pdfinfoomitdate=%d " % self.n(0, 1),
            "\\pdfsuppressptexinfo=%d \\pdftrailerid{%s}" % (self.n(-1, 15), self.pick(["", "x", "[<00><01>]"])),
            "\\pdfminorversion=%d \\pdfmajorversion=%d " % (self.n(0, 9), self.n(1, 2)),
            "\\pdfcompresslevel=%d \\pdfobjcompresslevel=%d " % (self.n(0, 9), self.n(0, 3)),
            "\\pdfpagebox=%d \\pdfforcepagebox=%d " % (self.n(0, 6), self.n(0, 6)),
            "\\pdfpkresolution=%d \\pdfimageresolution=%d " % (self.n(0, 1200), self.n(0, 1200)),
            "\\pdfximage{nonexistentzz.pdf}", "\\pdfximage page 2{doc.tex}",
            "\\message{\\the\\pdflastximage|\\the\\pdflastximagepages}",
            "\\pdfrefximage %d " % self.n(-1, 3),
            "\\pdfrefxform %d " % self.n(-1, 3), "\\pdfrefobj %d " % self.n(-1, 50),
            "\\setbox9\\hbox{%s}\\pdfxform 9 \\pdfxform 9 \\message{\\the\\pdflastxform}" % self.words(1, 3),
            "\\setbox9\\vbox{\\pdfliteral{0 g}\\hrule}\\immediate\\pdfxform resources{/X 1} 9 "
            "\\setbox8\\hbox{\\pdfrefxform\\pdflastxform}\\showbox8",
            "\\pdfliteral shipout{q}\\pdfliteral{Q}\\pdfliteral page{BT ET}",
            "\\pdfcolorstack %d current" % self.n(0, 2), "\\pdfcolorstack0 set{1 0 0 rg}",
            "\\pdfcolorstack0 pop", "\\pdfrestore", "\\pdfsetmatrix{1 2}", "\\pdfsetmatrix{0 0 0 0}",
            "\\pdfpageattr{/MediaBox [0 0 %d %d]}" % (self.n(-10, 600), self.n(-10, 800)),
            "\\pdfpagewidth=%s \\pdfpageheight=%s " % (self.pick(DIMS), self.pick(DIMS)),
            "\\pdfhorigin=%s \\pdfvorigin=%s " % (self.pick(DIMS), self.pick(DIMS)),
            "\\pdfnames{/Dests <</Names [(x) 1 0 R]>>}", "\\pdfcatalog{/OpenAction 1 0 R} openaction goto num 1",
            "\\pdfmovechars=%d " % self.n(0, 2), "\\pdfgentounicode=%d " % self.n(0, 2),
            "\\pdfinclusionerrorlevel=%d \\pdfinclusioncopyfonts=%d " % (self.n(-1, 2), self.n(0, 1)),
            "\\immediate\\openout3=/tmp/fzabs.tex \\immediate\\closeout3 ",
            "\\immediate\\openout3=../fzup.tex \\immediate\\closeout3 ",
            "\\immediate\\openout3=.fzdot.tex \\immediate\\write3{d}\\immediate\\closeout3 ",
            "\\openin3=/dev/null \\ifeof3 E\\else O\\closein3\\fi",
            "\\openin3=../doc.tex \\ifeof3 E\\else O\\closein3\\fi",
            "\\immediate\\openout3=fz.fls \\immediate\\closeout3 ",
            "\\pdfstrcmp{\\pdfescapestring{\\pdfunescapehex{%s}}}{x}" % self.pick(["ff00", "4", "zz", "", "6g"]),
            "\\message{\\pdfmdfivesum{%s}}" % self.pick(["", "abc", "^^e9", "\\relax"]),
        ])

    def media(self):
        """Images (PDF, PNG, JPEG inclusion: pdfTeX's C parts and xpdf),
        virtual fonts, font expansion and letterspacing at shipout."""
        imgs = ["example-image-a.pdf", "example-image-a.png", "example-image-a.jpg",
                "example-image.pdf", "example-grid-100x100pt.pdf", "example-grid-100x100bp.png",
                "example-image-a4-numbered.pdf"]
        def ximage():
            opts = []
            if self.chance(0.4):
                opts.append(self.pick(["width", "height", "depth"]) + " " + self.pick(SMALLDIMS + ["3cm", "0.3\\hsize"]))
            # pdftex.web's scan_image order: rule spec, attr, page, colorspace, box
            if self.chance(0.2):
                opts.append("attr{/Interpolate true}")
            if self.chance(0.3):
                opts.append("page %d" % self.n(0, 4))
            if self.chance(0.1):
                opts.append("colorspace %d" % self.n(0, 3))
            if self.chance(0.3):
                opts.append(self.pick(["mediabox", "cropbox", "bleedbox", "trimbox", "artbox"]))
            return "\\pdfximage %s{%s}" % (" ".join(opts), self.pick(imgs))
        vf = self.pick(["ptmr7t", "pplr7t", "phvr7t", "ptmb7t", "ptmri7t"])
        return self.pick([
            ximage() + "\\pdfrefximage\\pdflastximage",
            ximage() + "\\message{\\the\\pdflastximage,\\the\\pdflastximagepages}"
            "\\setbox7\\hbox{\\pdfrefximage\\pdflastximage}\\showbox7 \\box7",
            "\\immediate" + ximage() + "\\immediate" + ximage() + "\\pdfrefximage\\pdflastximage",
            "{\\pdfimageresolution=%d \\pdfimagehicolor=%d \\pdfimageapplygamma=%d \\pdfimagegamma=%d %s"
            "\\pdfrefximage\\pdflastximage}" % (self.n(0, 600), self.n(0, 1), self.n(0, 1), self.n(0, 3000), ximage()),
            "{\\pdfinclusionerrorlevel=%d \\pdfinclusioncopyfonts=%d %s\\pdfrefximage\\pdflastximage}"
            % (self.n(-1, 1), self.n(0, 1), ximage()),
            "{\\font\\fzv=%s at %s \\fzv %s}" % (vf, self.pick(["10pt", "7pt", "12.3pt", "1pt"]), self.words(2, 12)),
            "{\\font\\fzv=%s \\setbox7\\hbox{\\fzv %s}\\showbox7 \\box7}" % (vf, self.words(1, 4)),
            "{\\font\\fzv=%s \\fzv \\char%d \\char%d}" % (vf, self.n(0, 255), self.n(0, 255)),
            "{\\font\\fzv=%s \\pdffontexpand\\fzv %d %d %d %s\\pdfadjustspacing=%d \\fzv \\hsize=%s %s\\par}"
            % (vf, self.n(0, 40), self.n(0, 40), self.n(1, 10), "autoexpand", self.n(0, 2),
               self.pick(["100pt", "150pt", "3in"]), self.words(10, 30)),
            "{\\pdffontexpand\\tenrm %d %d %d autoexpand \\pdfadjustspacing=2 \\efcode\\tenrm`e=%d "
            "\\hsize=%s \\tenrm %s\\par}" % (self.n(1, 50), self.n(1, 50), self.n(1, 10), self.n(0, 1000),
                                               self.pick(["100pt", "150pt", "2in"]), self.words(15, 40)),
            "{\\letterspacefont\\fzls\\tenrm %d \\fzls %s}" % (self.n(-200, 400), self.words(1, 6)),
            "{\\pdfcopyfont\\fzcp\\tenrm \\fzcp %s \\message{\\fontname\\fzcp}}" % self.words(1, 3),
            "{\\font\\fzv=%s \\letterspacefont\\fzlv\\fzv %d \\fzlv %s}" % (vf, self.n(-100, 300), self.words(1, 5)),
            "{\\pdfprotrudechars=%d \\rpcode\\tenrm`.=%d \\lpcode\\tenrm``=%d \\hsize=%s %s\\par}"
            % (self.n(0, 2), self.n(-1000, 1000), self.n(-1000, 1000), self.pick(["120pt", "200pt"]),
               self.words(15, 40) + "."),
            "{\\pdfpkresolution=%d \\pdfpkmode{%s}\\pdfpkfixeddpi=%d }" % (self.n(0, 1200), self.pick(["", "ljfour", "x"]),
                                                                         self.n(0, 1)),
            "\\pdfnobuiltintounicode\\tenrm \\pdfgentounicode=1 \\pdfglyphtounicode{ff}{0066 0066}\\tenrm ff",
            "{\\pdfomitcharset=%d \\pdfomitcidset=%d \\pdfomitprocset=%d \\pdfomitinfodict=%d }"
            % (self.n(0, 1), self.n(0, 1), self.n(-1, 2), self.n(0, 1)),
            "\\pdffontattr\\tenrm{/Fz 1}\\message{\\the\\pdffontobjnum\\tenrm,\\pdffontname\\tenrm,\\pdffontsize\\tenrm}",
        ])

    def etex(self):
        """e-TeX's less used parts: TeX--XeT, the font and paragraph-shape
        queries, group and conditional state, \\showtokens, \\interactionmode."""
        w = lambda: self.words(1, 6)
        return self.pick([
            "{\\TeXXeTstate=1 \\beginR %s\\endR\\ %s\\par}" % (w(), w()),
            "{\\TeXXeTstate=1 \\noindent\\beginL %s \\beginR %s\\endR %s\\endL\\par}" % (w(), w(), w()),
            "{\\TeXXeTstate=1 \\setbox3\\hbox{\\beginR %s\\endR}\\showbox3 \\box3}" % w(),
            "{\\TeXXeTstate=1 \\beginR %s $x+y$ %s\\par}" % (w(), w()),
            "{\\TeXXeTstate=1 \\endL\\endR\\beginR %s\\par}" % w(),
            "{\\TeXXeTstate=1 \\hsize=%s \\beginR %s\\endR\\par}" % (self.pick(["50pt", "100pt", "2in"]), self.words(10, 30)),
            "{\\TeXXeTstate=1 \\beginR %s\\hfil\\break %s\\endR\\par}" % (w(), w()),
            "{\\TeXXeTstate=1 \\beginR %s\\leaders\\hrule\\hskip 20pt %s\\endR\\showlists\\par}" % (w(), w()),
            "{\\TeXXeTstate=1 \\predisplaydirection=%d \\beginR %s$$x\\eqno(1)$$\\endR\\par}" % (self.n(-1, 1), w()),
            "{\\TeXXeTstate=0 \\beginR x\\endR\\par}",
            "{\\TeXXeTstate=1 \\setbox3\\hbox{\\beginR a\\kern%s b\\vrule width1pt\\endR}\\unhbox3 \\showlists}" % self.pick(SMALLDIMS),
            "\\message{\\the\\fontcharwd\\tenrm`a,\\the\\fontcharht\\tenbf`g,\\the\\fontchardp\\tenit`y,"
            "\\the\\fontcharic\\tenit`f,\\the\\fontcharwd\\tenrm 300}",
            "{\\parshape 3 1pt 2pt 3pt 4pt 5pt 6pt \\message{\\the\\parshapelength%d,\\the\\parshapeindent%d,"
            "\\the\\parshapedimen%d}}" % (self.n(-1, 5), self.n(-1, 5), self.n(-1, 7)),
            "\\begingroup{\\message{\\the\\currentgrouplevel:\\the\\currentgrouptype}}"
            "\\ifnum1=1 \\unless\\ifx ab\\message{\\the\\currentiflevel:\\the\\currentiftype:\\the\\currentifbranch}\\fi\\fi\\endgroup",
            "\\hbox{\\message{\\the\\currentgrouptype}}$\\message{\\the\\currentgrouptype}$\\vtop{\\message{\\the\\currentgrouptype}}",
            "\\showtokens{a#b\\par}\\showtokens\\expandafter{\\the\\everypar}",
            "\\message{\\detokenize{a#b \\x}|\\unexpanded{\\y #}}",
            "{\\interactionmode=%d \\message{\\the\\interactionmode}}" % self.n(-1, 4),
            "\\message{\\number\\numexpr %s\\relax,\\the\\dimexpr %s\\relax,\\the\\glueexpr %s\\relax}" % (
                self.pick(["7*11/3", "(-7)/2", "2147483647/2*2", "-(3)", "1+(2*(3+4))", "65536*32768",
                           "-2147483647-1", "7/0"]),
                self.pick(["1pt*3/4", "(1pt+2pt)/3", "16383pt*2", "1sp*-1", "-(1pt)", "\\maxdimen/2*2"]),
                self.pick(["1pt plus 2fil*3", "(1pt minus 1pt)/2", "\\baselineskip*2", "0pt plus 1fill-1pt"])),
            "\\message{\\the\\gluestretch\\glueexpr 1pt plus 3fil\\relax,\\the\\gluestretchorder\\skip0,"
            "\\the\\glueshrinkorder\\glueexpr 0pt minus 2filll\\relax,\\the\\mutoglue\\muexpr 3mu plus 1fill\\relax,"
            "\\the\\gluetomu\\skip0}",
            "{\\lastlinefit=%d \\parfillskip=0pt plus 1fil \\hsize=%s %s\\par}" % (self.pick([0, 500, 1000, 2000, -1]),
                                                                           self.pick(["100pt", "200pt"]), self.words(10, 40)),
            "{\\savingvdiscards=1 \\setbox3\\vbox{a\\par\\penalty0 \\vskip 3pt b\\par}\\setbox4\\vsplit3 to 5pt "
            "\\showbox3 \\message{\\the\\lastpenalty}\\vbox{\\splitdiscards}\\vbox{\\pagediscards}}",
            "\\showgroups\\showifs{\\iftrue\\showifs\\showgroups\\fi}",
            "{\\tracingnesting=2 \\scantokens{\\begingroup}\\endgroup}",
            "{\\tracingnesting=2 \\scantokens{\\iftrue}\\fi}",
            "\\protected\\edef\\fzp{\\noexpand\\relax}\\show\\fzp \\message{\\meaning\\fzp}",
            "\\middle", "\\showtokens{\\showtokens}", "\\interactionmode=\\interactionmode",
            "\\message{\\ifcsname fz\\endcsname Y\\else N\\fi\\ifdefined\\fzundef Y\\else N\\fi\\iffontchar\\tenrm 200 Y\\else N\\fi}",
        ])

    def files(self):
        """web2c's file handling (tex.ch, texmfmp.c): pipes and \\write18 with
        shell escape off, odd file names, directories and devices, \\endinput,
        nested \\input, \\openin/\\read edge cases."""
        mk = "\\immediate\\openout3=%s \\immediate\\write3{%s}\\immediate\\closeout3 "
        return self.pick([
            "\\immediate\\write18{echo fz}", "\\write18{echo fz}", "\\message{\\the\\pdfshellescape}",
            "\\input|\"echo fz\" ", "\\openin3=|\"echo fz\" \\ifeof3 E\\else O\\read3 to\\x\\show\\x\\closein3\\fi",
            "\\immediate\\openout3=|\"cat\" \\immediate\\write3{x}\\immediate\\closeout3 ",
            mk % ("fzi.tex", "\\message{inner}\\endinput after") + "\\input fzi ",
            mk % ("fzj.tex", "a\\endinput b") + "\\input fzj c\\par",
            mk % ("\"fz j.tex\"", "\\message{spaced}") + "\\input \"fz j\" \\input{\"fz j.tex\"}",
            mk % ("fzk", "\\message{noext}") + "\\input fzk \\input fzk.tex ",
            mk % ("fzl.tex", "\\input fzl ") + "\\input fzl ",
            mk % ("fzm.tex", "\\message{\\the\\inputlineno}^^J\\message{\\the\\inputlineno}") + "\\input fzm ",
            mk % ("fzn.tex", "{") + "\\input fzn }",
            mk % ("fzo.tex", "\\iftrue") + "\\input fzo \\fi",
            mk % ("fzp.tex", "\\def\\fzq#1{#1}\\fzq") + "\\input fzp {x}",
            mk % ("fzr.tex", "line one^^Jline two^^J^^J") + "\\openin4=fzr \\read4 to\\a\\read4 to\\b\\read4 to\\c\\read4 to\\d"
            "\\show\\a\\show\\b\\show\\c\\show\\d\\ifeof4 E\\fi\\closein4",
            mk % ("fzs.tex", "{a") + "\\openin4=fzs \\read4 to\\a\\show\\a\\closein4",
            mk % ("fzt.tex", "\\par\\par") + "\\openin4=fzt \\readline4 to\\a\\show\\a\\closein4",
            "\\openin3=. \\ifeof3 E\\else O\\closein3\\fi", "\\openin3=/dev/null \\ifeof3 E\\else O\\read3 to\\x\\show\\x\\fi",
            "\\input /dev/null \\message{after-null}", "\\input \"\" ", "\\input .tex ", "\\input{} ",
            "\\openin3=doc \\read3 to\\x \\show\\x \\closein3", "\\openin3=doc.tex \\readline3 to\\x \\show\\x \\closein3",
            "\\read-1 to\\x \\show\\x", "\\read16 to\\x", "\\read3 to\\x \\show\\x", "\\closein3 \\closein15 \\closein-1",
            "\\openin16=doc ", "\\immediate\\openout16=fzx \\immediate\\openout-1=fzy ",
            "\\immediate\\write3{unopened}\\immediate\\write15{unopened}",
            "\\immediate\\openout3=fz^^e9.tex \\immediate\\write3{e}\\immediate\\closeout3 \\input fz^^e9 ",
            "\\immediate\\openout3=fz\\jobname.tex \\immediate\\closeout3 \\message{\\pdffilesize{fzdoc.tex}}",
            "\\everyeof{\\message{eof}}" + mk % ("fzu.tex", "u") + "\\input fzu \\everyeof{}",
            "\\message{\\jobname}\\edef\\fzj{\\jobname}\\show\\fzj",
            "\\special{%s}" % self.pick(["", "x", "header=a.ps", "pdf:dest (d) [@thispage /Fit]"]),
            "\\endinput \\message{same-line}",
        ])

    def rawbytes(self):
        """The input file's own bytes through texmfmp.c's input_line: CR and
        CR LF line ends, trailing spaces and tabs, form feeds, NUL, DEL and
        8-bit bytes, a line of blanks, \\endlinechar and \\catcode changes
        that see them."""
        w = self.words(1, 4)
        return self.pick([
            "\\message{cr}\r\\message{after-cr}\r\n\\message{crlf}",
            "{\\catcode13=12 \\message{a}\r\\message{b}}",
            "\\message{trail}   \t  \n\\message{x}\t\n",
            "\\def\\fzt{a  \t }\\show\\fzt",
            "%s\x0c%s\\par" % (w, w),
            "{\\catcode`\\^^L=12 \\message{\x0c}}",
            "{\\catcode0=12 \\message{a\x00b}}", "\\message{\x7f}", "{\\catcode127=12 \\message{\x7f}}",
            "\\message{\xe9\xff\x80}", "{\\catcode`\\\xe9=11 \\def\\\xe9x{E}\\show\\\xe9x}",
            "   \t \n\\message{blank-line-before}",
            "{\\endlinechar=%s \\message{a}\n\\message{b}}" % self.pick(["-1", "32", "`\\%", "65"]),
            "{\\catcode`\\ =12 \\message{sp ace}  }",
            "{\\catcode9=12 \\message{tab\there}}", "{\\catcode9=10 \\message{tab\there}}",
            "\\message{\\the\\inputlineno}\r\\message{\\the\\inputlineno}\r\r\\message{\\the\\inputlineno}",
            "{\\obeylines\\obeyspaces a  b\r c\n}",
            "\\def\\fzl{%s}\\message{\\fzl}" % ("y" * self.n(900, 1100)),
        ])

    def latexstuff(self):
        """LaTeX-level material for pdflatex documents; a command from a
        package that is not loaded is an undefined control sequence, which
        both engines report the same way."""
        w = lambda: self.words(1, 8)
        return self.pick([
            "\\section{%s}%s\\label{s%d}" % (w(), w(), self.n(0, 3)),
            "See \\ref{s%d} and \\pageref{s%d} and \\cref{s1}." % (self.n(0, 3), self.n(0, 3)),
            "%s\\footnote{%s}\\marginpar{%s}" % (w(), w(), w()),
            "\\begin{tikzpicture}\\draw[%s] (0,0) -- (%d,1) circle (3pt) node[above]{%s};\\fill[blue!30] (0,0) rectangle (1,%d);\\end{tikzpicture}"
            % (self.pick(["thick", "red", "->", "dashed"]), self.n(-2, 3), self.words(1, 2), self.n(1, 3)),
            "\\tikz[remember picture,overlay]\\node at (current page.center){%s};" % w(),
            "\\textcolor{%s}{%s}\\colorbox{yellow}{%s}" % (self.pick(["red", "blue!50!black", "Maroon", "nonexistentcolor"]), w(), w()),
            "\\begin{tabular}{|l|c|r|}\\hline %s & %s & %s\\\\\\hline\\multicolumn{2}{c}{%s} & x\\\\\\end{tabular}" % (w(), w(), w(), w()),
            "\\begin{tabularx}{\\linewidth}{lX}%s & %s\\\\\\end{tabularx}" % (w(), self.words(10, 30)),
            "\\begin{longtable}{ll}%s\\end{longtable}" % "".join("%s & %s\\\\" % (self.words(1, 2), self.words(1, 2)) for _ in range(self.n(1, 60))),
            "\\begin{lstlisting}\nint main() { return %d; }\n  x <- y;\n\\end{lstlisting}" % self.n(0, 9),
            "\\url{http://example.com/a_b%%20c#d}\\href{http://x.org}{%s}" % w(),
            "\\begin{figure}[%s]\\centering\\includegraphics[width=%s]{example-image-a}\\caption{%s}\\end{figure}"
            % (self.pick(["htbp", "h", "!t", "H", "p"]), self.pick(["2cm", "0.5\\linewidth", "10pt"]), w()),
            "\\begin{itemize}[%s]\\item %s\\item %s\\end{itemize}" % (self.pick(["", "label=--", "noitemsep", "leftmargin=*"]), w(), w()),
            "\\begin{align}a&=b\\label{eq%d}\\\\c&=d\\notag\\end{align}\\eqref{eq%d}" % (self.n(0, 2), self.n(0, 2)),
            "\\SI{%s}{\\metre\\per\\second}\\num{%s}" % (self.pick(["3.5e3", "-1.2", "1/2"]), self.pick(["12345.678", "0.0001", "1e10"])),
            "\\cite{knuth84}\\citep{x}\\nocite{*}",
            "\\so{%s}\\ul{%s}\\sout{%s}" % (w(), w(), w()),
            "\\begin{wrapfigure}{r}{2cm}x\\end{wrapfigure}%s" % self.words(20, 50),
            "\\begin{framed}%s\\end{framed}\\begin{mdframed}%s\\end{mdframed}" % (w(), w()),
            "\\begin{tcolorbox}[title=%s]%s\\end{tcolorbox}" % (w(), w()),
            "\\includepdf[pages=%s]{example-image-a4-numbered.pdf}" % self.pick(["1", "2-3", "-"]),
            "\\begin{multicols}{%d}%s\\end{multicols}" % (self.n(1, 4), self.words(40, 120)),
            "\\newpage\\thispagestyle{%s}%s" % (self.pick(["empty", "plain", "fancy", "headings"]), w()),
            "\\tableofcontents\\listoffigures", "\\clearpage", "\\appendix\\section{%s}" % w(),
            "\\begin{equation}\\bm{x}\\coloneqq\\frac{1}{2}\\end{equation}",
            "\\verb|%s|\\texttt{%s}\\emph{%s}" % (self.words(1, 3), w(), w()),
            "\\selectlanguage{%s}%s\\today" % (self.pick(["english", "ngerman", "french"]), w()),
            "\\char\"E9\\'e\\\"o\\ss{}\\c{c}\\textquotedblleft x\\textquotedblright\\texteuro",
        ])

    def web2cch(self):
        """What changes/web2c.ch re-specifies from tex.ch: more than 256 fonts
        (16-bit font numbers, DVI fnt_def2), repeated \\hyphenation words,
        fonts at 2048pt or more, TFM names past 255 characters, \\mkern and
        \\mskip with non-mu units, many fonts in one DVI page."""
        n = self.n(250, 300)
        return self.pick([
            "{\\count1=0 \\loop\\advance\\count1 1 \\expandafter\\font\\csname fzf\\the\\count1\\endcsname=cmr10 at \\the\\count1 sp"
            "\\ifnum\\count1<%d \\repeat \\setbox3\\hbox{\\csname fzf%d\\endcsname a\\csname fzf%d\\endcsname b}\\showbox3 \\shipout\\box3}"
            % (n, self.n(1, n), self.n(1, n)),
            "{\\count1=0 \\loop\\advance\\count1 1 \\expandafter\\font\\csname fzg\\the\\count1\\endcsname=cmr10 scaled %d"
            "\\csname fzg\\the\\count1\\endcsname x\\ifnum\\count1<%d \\repeat\\par}" % (self.n(500, 3000), self.n(5, 40)),
            "\\hyphenation{%s %s %s}\\showhyphens{%s}" % ("fu-zz-word", "fuz-zword", "f-uzzword", "fuzzword"),
            "\\hyphenation{%s}\\hyphenation{%s}\\showhyphens{%s %s}" % (
                self.pick(["ab-cd-ef", "abc-def"]), self.pick(["a-bcdef", "abcd-ef", "ab-cd-ef"]), "abcdef", "abcdefs"),
            "{\\lccode`\\^^e9=`\\^^e9 \\hyphenation{caf-^^e9s}\\showhyphens{caf^^e9s}}",
            "{\\uchyph=%d \\hyphenation{FUZZ-WORD}\\showhyphens{Fuzzword FUZZWORD fuzzword}}" % self.n(-1, 1),
            "\\font\\fzbig=cmr10 at %s \\fzbig x" % self.pick(["2048pt", "2047.99998pt", "4095pt", "-1pt", "0pt", "16383.99999pt"]),
            "\\font\\fzbig=cmr10 scaled %d \\fzbig x" % self.pick([32768, 32769, 0, -1, 204800, 1]),
            "\\font\\fzlong=%s \\fzlong x" % ("a" * self.pick([254, 255, 256, 300])),
            "$\\mkern %s \\mskip %s x$" % (self.pick(["3pt", "1em", "-2mu", "0.5\\hsize"]), self.pick(["1pt plus 1fil", "2mu minus 1mu", "\\baselineskip"])),
            "$\\mskip\\thinmuskip\\mkern\\dimen0 \\mskip\\skip0 x$",
            "{\\count1=0 \\loop\\advance\\count1 1 \\expandafter\\font\\csname fzh\\the\\count1\\endcsname=cmr%s at %dpt"
            "\\ifnum\\count1<%d \\repeat \\message{\\fontname\\csname fzh%d\\endcsname}}"
            % (self.pick(["10", "7", "5"]), self.n(1, 30), self.n(257, 300), self.n(1, 256)),
            "\\showhyphens{%s}" % " ".join(self.pick(WORDS) for _ in range(self.n(3, 12))),
            "{\\language=%d \\showhyphens{%s}}" % (self.n(0, 3), " ".join(self.pick(WORDS) for _ in range(self.n(3, 8)))),
        ])

    def chunk(self):
        if self.latex and self.chance(0.4):
            return self.latexstuff()
        k = self.n(0, 150)
        if k < 22:
            return self.paragraph()
        if k < 28:
            return self.display()
        if k < 34:
            return self.vsplit()
        if k < 42:
            return self.align()
        if k < 50:
            return "\\message{[%s]}" % self.expr() if self.chance(0.6) else self.expr()
        if k < 55:
            return self.strprim()
        if k < 63:
            return self.fontstuff()
        if k < 72:
            return self.showstuff()
        if k < 79:
            return self.errorstuff()
        if k < 86:
            return self.output_routine() if not self.latex else self.paragraph()
        if k < 92:
            return self.vlist()
        if k < 96:
            return self.hyph_setting()
        if k < 98:
            return self.pdfstuff()
        if k < 99:
            return self.iostuff()
        if k < 103:
            return self.charset()
        if k < 111:
            return self.web2cstuff()
        if k < 118:
            return self.pdfwriter()
        if k < 126:
            return self.media()
        if k < 133:
            return self.etex()
        if k < 140:
            return self.files()
        if k < 145:
            return self.rawbytes()
        if k < 150:
            return self.web2cch()
        return "\\setbox%d\\hbox{%s}" % (self.n(10, 12), self.inline(2))

    def hyph_setting(self):
        return self.pick([
            "\\hyphenation{%s}" % " ".join(self.pick(["of-fi-ce", "fjo-rd", "dif-fer-ent", "Mis-sis-sip-pi",
                                                      "ta-ble", "TeX", "x-ray", "re-pre-sen-ta-tion"])
                                          for _ in range(self.n(1, 3))),
            "{\\language=1 \\hyphenation{ab-cd-ef}}", "\\lccode`\\-=`\\- ",
            "{\\pretolerance=-1 \\hyphenpenalty=-1000 \\hsize=%s \\language=%d %s\\par}" % (
                self.pick(["1in", "0.7in", "30pt"]), self.n(0, 2), self.words(10, 25)),
            "{\\hyphenpenalty=10000 \\exhyphenpenalty=10000 \\hsize=1in %s\\par}" % self.words(6, 15),
            "\\showhyphens{%s}" % self.words(3, 8) if False else
            "{\\setbox0\\vbox{\\parfillskip=0pt \\hsize=16383pt \\pretolerance=-1 \\tolerance=-1 \\hbadness=-1 \\hskip0pt %s}}" % self.words(2, 6),
            "\\savinghyphcodes=1 \\lccode`A=`a \\hyphenation{AB-ba}",
            "{\\lefthyphenmin=1 \\righthyphenmin=1 \\hsize=1pt \\pretolerance=-1 %s\\par}" % self.words(2, 4),
        ])


TRACING = [
    "\\tracingonline=1", "\\tracingparagraphs=1", "\\tracingpages=1", "\\tracingoutput=1",
    "\\tracinglostchars=%s", "\\tracingcommands=%s", "\\tracingmacros=%s",
    "\\tracingrestores=1", "\\tracingassigns=1", "\\tracinggroups=1", "\\tracingifs=1",
    "\\tracingnesting=%s", "\\tracingscantokens=1",
]


def preamble(g, pdf):
    lines = ["\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6",
             "\\input plain"]
    pats = []
    if g.chance(0.4):
        pats.append("{\\language=1 \\lccode`\\'=`\\' \\patterns{%s}}" % " ".join(
            g.pick([".ab1c", "1ff", "f1f", "a1b", "2b1c", "o1f", "1fi", "e1r", "1ta", "n2t", "4x3", ".a5",
                    "s1s", "i1o", "1n", "8z", "c1c."]) for _ in range(g.n(2, 8))))
    if g.chance(0.2):
        pats.append("{\\language=2 \\savinghyphcodes=1 \\lccode`Q=`q \\patterns{1q 2qq}}")
    lines += pats
    lines.append("\\pdfoutput=%d" % (1 if pdf else 0))
    # pdfTeX seeds its generator from the clock; a deviate that is expanded
    # before the \pdfsetrandomseed beside it runs (in an argument, a
    # \write) would otherwise differ from run to run.
    lines.append("\\pdfsetrandomseed 271828")
    if pdf:
        lines.append("\\pdfcompresslevel=0 \\pdfobjcompresslevel=0")
    lines.append("\\showboxdepth=%s \\showboxbreadth=%s" % (
        g.pick(["10000", "10000", "3", "1"]), g.pick(["10000", "10000", "5", "2"])))
    trace = []
    for t in TRACING:
        if g.chance(0.6):
            trace.append(t % g.pick(["1", "2", "3"]) if "%s" in t else t)
    lines.append(" ".join(trace))
    lines.append("\\errorcontextlines=%s" % g.pick(["5", "100", "-1", "0"]))
    lines.append(g.pick(["", "\\batchmode", "\\scrollmode", ""]))
    lines.append("\\setbox10\\hbox{box ten}\\setbox11\\hbox{\\kern1pt eleven}\\setbox12\\hbox{}")
    lines.append("\\insert100{}\\count100=1000 \\dimen100=\\maxdimen \\skip100=0pt")
    return "\n".join(l for l in lines if l) + "\n"


LATEX_PACKAGES = ["amsmath", "amssymb", "microtype", "array", "multicol", "color",
                  "graphicx", "hyperref", "fontenc", "lmodern", "textcomp", "booktabs",
                  "tikz", "xcolor", "geometry", "fancyhdr", "longtable", "tabularx",
                  "listings", "url", "caption", "enumitem", "mathtools", "bm",
                  "babel", "inputenc", "siunitx", "cleveref", "natbib", "pdfpages",
                  "soul", "ulem", "float", "wrapfig", "framed", "mdframed", "tcolorbox"]


def latex_preamble(g):
    """A pdflatex document head. The plain font names the grammar uses
    (\\tenrm ...) are declared with \\font, and \\output is left alone."""
    lines = ["\\documentclass%s{%s}" % (g.pick(["", "[twocolumn]", "[12pt]", "[draft]"]),
                                        g.pick(["article", "article", "report", "book"]))]
    for p in sorted(set(g.pick(LATEX_PACKAGES) for _ in range(g.n(0, 4)))):
        opt = {"microtype": g.pick(["", "[protrusion=true,expansion=true]", "[final]", "[expansion=false]"]),
               "fontenc": "[T1]", "hyperref": g.pick(["", "[draft]"]),
               "babel": g.pick(["[english]", "[ngerman]", "[french,english]"]),
               "inputenc": "[latin1]", "geometry": "[a5paper,margin=1cm]",
               "xcolor": g.pick(["", "[dvipsnames]"])}.get(p, "")
        lines.append("\\usepackage%s{%s}" % (opt, p))
    lines.append("\\font\\tenrm=cmr10 \\font\\tenbf=cmbx10 \\font\\tenit=cmti10 \\font\\tensl=cmsl10"
                 " \\font\\tentt=cmtt10 \\font\\sevenrm=cmr7 \\font\\fiverm=cmr5 \\font\\teni=cmmi10")
    lines.append("\\pdfsetrandomseed 271828")
    lines.append("\\begin{document}")
    lines.append("\\showboxdepth=%s \\showboxbreadth=%s" % (g.pick(["10000", "3"]), g.pick(["10000", "5"])))
    trace = [t % g.pick(["1", "2"]) if "%s" in t else t for t in TRACING if g.chance(0.4)]
    lines.append(" ".join(trace))
    lines.append(g.pick(["", "\\batchmode", "\\scrollmode", ""]))
    lines.append("\\setbox10\\hbox{box ten}\\setbox11\\hbox{\\kern1pt eleven}\\setbox12\\hbox{}")
    return "\n".join(l for l in lines if l) + "\n"


def generate(seed, nchunks=None, latex=None):
    """latex: None picks (15% LaTeX documents), True/False forces."""
    rng = random.Random(seed)
    g = G(rng)
    is_latex = rng.random() < 0.15
    if latex is not None:
        is_latex = latex
    if is_latex:
        g.latex = True
        pre = latex_preamble(g)
        k = nchunks or g.n(3, 10)
        chunks = [g.chunk() for _ in range(k)]
        return Doc(seed, pre, chunks, True, fmt="pdflatex")
    pdf = rng.random() < 0.8
    pre = preamble(g, pdf)
    k = nchunks or g.n(4, 14)
    chunks = [g.chunk() for _ in range(k)]
    if g.chance(0.04):
        chunks.append(g.capacity())
    return Doc(seed, pre, chunks, pdf)


if __name__ == "__main__":
    import sys
    s = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    sys.stdout.write(generate(s).render())
