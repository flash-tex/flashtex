#!/usr/bin/env python3
"""Grammar-based generator of INITEX plain-TeX documents for the replay fuzzer.

A case is a dict of files (`t.tex`, the main file, and `u.tex`, a second file
the document may `\\input`). `t.tex` defines `\\measure` and calls it several
times, with assignments between the calls that change what the calls read.
Every body item and every top-level step is one line ending in `%`, so a
line-level delta debugger (minimize.py) never splits a group.

The same seed always gives the same case (`random.Random(seed)`, no other
source of randomness), so a seed names a case for good, as long as this file
does not change. A change to the grammar renumbers the cases; the fixed seed
set of test_fuzz_replay.py then needs no update, because it only checks that
the seeds run clean.

Stdlib only.
"""
import random
import re

# Output registers the calls write and `\show` prints.
SHOW = (r"\def\show{\message{[\the\count1/\the\dimen1/\the\count3/"
        r"\the\wd4/\the\toks4/\meaning\z]}}%")

PRELUDE = [
    r"\catcode`\{=1 \catcode`\}=2 \catcode`\#=6 \catcode`\&=4 "
    r"\catcode`\^=7 \catcode`\$=3 \catcode`\~=13 \catcode`\^^I=10 \scrollmode",
    r"\hsize=100pt \vsize=200pt \baselineskip=12pt \tolerance=10000 "
    r"\parfillskip=0pt plus 1fil \maxdepth=2pt",
    SHOW,
    # doubles \toks2: the long token lists of long_mutation
    r"\def\dbl{\toks2=\expandafter{\the\toks2 \the\toks2}}%",
]


class Gen:
    """One document. `style` is "box" (typesetting calls, BOX-MEMO's kind)
    or "pure" (calls without parameters that only assign and expand, D9's
    kind); both styles are run under both subsystems."""

    def __init__(self, seed, fonts=False):
        self.r = random.Random(seed)
        self.fonts = fonts
        self.style = self.r.choice(["box", "box", "pure"])
        self.arity = 0 if self.style == "pure" else self.r.choice([0, 1, 1, 2])
        self.delim = self.arity >= 1 and self.r.random() < 0.15
        self.uses_input = False
        # registers by \countdef'd names (assign_int, which D9 may read)
        # rather than as \count2 (register, which it may not)
        self.regdefs = self.r.random() < (0.8 if self.style == "pure" else 0.3)

    # -- small pieces -----------------------------------------------------
    def c(self, *xs):
        return self.r.choice(xs)

    def num(self):
        return str(self.c(0, 1, 2, 3, 5, 7, -1, 12, 100))

    def dim(self):
        return self.c("0pt", "1pt", "2pt", "2.5pt", "3pt", "5pt", "10pt",
                      "-1pt", "20pt")

    def param(self):
        if self.arity == 0:
            return None
        return "#" + str(self.r.randint(1, self.arity))

    def numexpr(self):
        p = self.param()
        opts = [r"\count2", r"\count3", self.num(), r"\numexpr\count2+1\relax",
                r"\catcode`A", r"\lccode`a", r"\escapechar",
                r"\currentgrouplevel", r"\inputlineno"]
        if p and self.style == "box":
            opts += [p, p]
        if self.style == "box":
            opts += [r"\badness", r"\lastpenalty", r"\lastnodetype",
                     r"\wd0", r"\ht0"]
        return self.c(*opts)

    def dimexpr(self):
        p = self.param()
        opts = [r"\dimen2", r"\dimen3", self.dim(), r"\wd0", r"\ht0", r"\dp0",
                r"\fontdimen2\nullfont", r"\dimexpr\dimen2+1pt\relax",
                r"\lastkern", r"\hsize", r"\skip2"]
        if p:
            opts += [p, p]
        return self.c(*opts)

    def cond(self, a, b):
        k = self.r.randint(0, 11)
        if k == 0:
            return rf"\ifnum\count2>{self.num()} {a}\else {b}\fi"
        if k == 1:
            return rf"\ifx\x\cr {a}\else {b}\fi"
        if k == 2:
            return rf"\ifdim\dimen2>{self.dim()} {a}\else {b}\fi"
        if k == 3:
            return rf"\ifcase\count3 {a}\or {b}\else {a}{b}\fi"
        if k == 4:
            return rf"\ifdefined\c{self.c(0, 1, 2)} {a}\else {b}\fi"
        if k == 5:
            return rf"\ifcsname c\the\count2\endcsname {a}\else {b}\fi"
        if k == 6:
            return rf"\ifodd\count2 {a}\else {b}\fi"
        if k == 7:
            return rf"\ifx\inner\relax {a}\else {b}\fi"
        if k == 8:
            return rf"\if\y a{a}\else {b}\fi"
        if k == 9:
            return rf"\ifcat\y A{a}\else {b}\fi"
        if k == 10:
            return rf"\ifvoid2 {a}\else {b}\fi"
        return rf"\unless\ifnum\count3<{self.num()} {a}\else {b}\fi"

    def hlist(self, depth=0):
        """Material for an \\hbox."""
        items = []
        for _ in range(self.r.randint(1, 3)):
            k = self.r.randint(0, 17)
            p = self.param()
            if k == 0:
                items.append(rf"\vrule width{self.dim()} height{self.dim()}")
            elif k == 1:
                items.append(rf"\kern{self.dimexpr()}")
            elif k == 2:
                items.append(rf"\hskip{self.dim()} plus 1fil")
            elif k == 3 and p:
                items.append(p)
            elif k == 4:
                items.append(r"\the\toks2")
            elif k == 5:
                items.append(r"\inner")
            elif k == 6:
                items.append(r"\csname c\the\count2\endcsname")
            elif k == 7 and depth < 2:
                items.append(self.cond(self.hlist(depth + 1),
                                       self.hlist(depth + 1)))
            elif k == 8 and self.fonts:
                items.append(self.c(r"\fa AV", r"\fb x\fa y", r"\fa\char`A",
                                    r"\fa ff", r"\fa\accent18 o"))
            elif k == 9:
                items.append(r"\unhcopy2")
            elif k == 10:
                items.append(r"\penalty" + self.num())
            elif k == 11:
                items.append(r"\raise" + self.dim() + r"\hbox{\vrule width1pt}")
            elif k == 12:
                items.append(r"\x")
            else:
                items.append(r"\vrule width1pt")
        return "".join(i + " " for i in items).strip()

    def vlist(self, depth=0):
        items = []
        for _ in range(self.r.randint(1, 3)):
            k = self.r.randint(0, 12)
            if k == 0:
                items.append(rf"\hrule height{self.dim()}")
            elif k == 1:
                items.append(rf"\kern{self.dimexpr()}")
            elif k == 2:
                items.append(rf"\hbox{{{self.hlist(depth + 1)}}}")
            elif k == 3:
                items.append(r"\noindent\vrule width 3pt height 5pt\par")
            elif k == 4:
                items.append(r"\halign{##\x\cr \vrule height2pt\cr}")
            elif k == 5:
                items.append(r"\vskip" + self.dim())
            elif k == 6:
                items.append(r"\penalty" + self.num())
            elif k == 7:
                items.append(r"\noindent\vrule width 60pt\ \vrule width 60pt"
                             r"\ \vrule width 60pt\par")
            elif k == 8:
                p = self.param()
                items.append(p if p else r"\inner")
            elif k == 9:
                items.append(r"\copy2")
            else:
                items.append(r"\hrule height1pt")
        return " ".join(items)

    # -- the body of \measure ---------------------------------------------
    def typeset_item(self):
        """Typesetting that reads parameters straight from eqtb (not through
        scan_something_internal): line breaking, box packaging and its
        warnings, alignments, \\vsplit."""
        w = self.c("10pt", "30pt", "45pt", "60pt")
        par = (r"\noindent\vrule width " + w + r"\ \vrule width " + w +
               r"\ \vrule width " + w + r"\ \vrule width 20pt\par")
        k = self.r.randint(0, 17)
        if k == 0:
            return r"\setbox0\hbox to5pt{\vrule width10pt}\global\dimen1=\wd0 "
        if k == 1:
            return r"\setbox0\hbox to50pt{\vrule width10pt\hskip0pt plus1pt}\global\count1=\badness "
        if k == 2:
            return (r"\setbox0\vbox{\hbox{\vrule height5pt depth2pt}\hbox{\vrule height"
                    + self.c("5pt", "1pt", "12pt") + r"}}\global\dimen1=\ht0 ")
        if k == 3:
            return r"\setbox0\vbox{" + par + r"\global\count1=\prevgraf}\global\dimen1=\ht0 "
        if k == 4:
            return r"\setbox0\vbox{\indent\vrule width 30pt\ \vrule width 50pt\par}\global\dimen1=\ht0 "
        if k == 5:
            return (r"\setbox0\vbox{" + par + r"}\setbox0\vbox{\setbox6\vsplit0 to"
                    + self.c("5pt", "12pt", "20pt") + r"\global\dimen1=\ht6 }")
        if k == 6:
            return r"\setbox0\vbox{\halign to50pt{\tabskip=0pt plus1fil##\cr\vrule width3pt\cr}}\global\dimen1=\ht0 "
        if k == 7:
            return r"\setbox0\vbox to5pt{\hrule height10pt}\global\dimen1=\dp0 "
        if k == 8:
            return r"\setbox0\vbox to50pt{\hrule height10pt\vskip0pt plus1pt}\global\count1=\badness "
        if k == 9:
            return r"\setbox0\vtop{" + par + r"}\global\dimen1=\dp0 "
        if k == 10:
            return r"\setbox0\hbox{\leaders\hrule\hskip10pt\discretionary{\kern1pt}{}{}}\global\dimen1=\wd0 "
        if k == 11:
            return r"\setbox0\vbox{\vskip5pt\global\dimen1=\lastskip\unskip\global\count1=\lastnodetype}"
        if k == 12:
            return r"\setbox0\vbox{" + par + r"\global\count1=\lastpenalty}"
        if k == 13:
            return r"\setbox0\hbox{\beginL\vrule width2pt\endL}\global\dimen1=\wd0 "
        if k == 14:
            return r"\setbox0\vbox{\noindent\vrule width 30pt\vadjust{\kern3pt}\par}\global\dimen1=\ht0 "
        if k == 15:
            return r"\setbox0\vbox{\hsize=40pt " + par + r"}\global\dimen1=\ht0 "
        if k == 16:
            return r"\setbox0\vbox{\everypar{\kern1pt}\indent\vrule\par}\global\dimen1=\ht0 "
        return r"\setbox0\vbox{\halign{##\cr\vrule height 3pt\cr\noalign{\kern2pt}\vrule\cr}}\global\dimen1=\ht0 "

    def param_mutation(self):
        ints = [r"\hbadness", r"\vbadness", r"\tolerance", r"\pretolerance",
                r"\linepenalty", r"\clubpenalty", r"\widowpenalty",
                r"\interlinepenalty", r"\brokenpenalty", r"\adjdemerits",
                r"\looseness", r"\hangafter", r"\showboxdepth",
                r"\showboxbreadth", r"\lastlinefit", r"\uchyph",
                r"\TeXXeTstate", r"\savingvdiscards", r"\tracinglostchars",
                r"\endlinechar", r"\newlinechar", r"\holdinginserts",
                r"\tracingparagraphs", r"\tracingonline", r"\errorcontextlines",
                r"\predisplaydirection", r"\pdfadjustspacing"]
        dims = [r"\hfuzz", r"\vfuzz", r"\parindent", r"\hangindent",
                r"\emergencystretch", r"\lineskiplimit", r"\boxmaxdepth",
                r"\splitmaxdepth", r"\hsize", r"\maxdepth"]
        glues = [r"\baselineskip", r"\lineskip", r"\leftskip", r"\rightskip",
                 r"\parskip", r"\spaceskip", r"\xspaceskip", r"\parfillskip",
                 r"\splittopskip", r"\tabskip"]
        toks = [r"\everypar", r"\everyhbox", r"\everyvbox", r"\everycr"]
        k = self.r.randint(0, 9)
        if k <= 3:
            return self.c(*ints) + "=" + self.c("0", "1", "-1", "2", "10", "100",
                                                "1000", "10000", "13") + " %"
        if k <= 5:
            return self.c(*dims) + "=" + self.c("0pt", "1pt", "5pt", "20pt",
                                                "-3pt", "100pt", "40pt") + "%"
        if k <= 7:
            return self.c(*glues) + "=" + self.c("0pt", "1pt", "12pt",
                                                 "2pt plus 1fil", "3pt minus 1pt",
                                                 "0pt plus 1fil") + "%"
        if k == 8:
            return self.c(*toks) + "{" + self.c("", r"\kern1pt", r"\inner",
                                                r"\noalign{\kern1pt}") + "}%"
        return self.c(r"\interlinepenalties", r"\clubpenalties",
                      r"\widowpenalties") + " 2 " + self.c("50", "60") + " 70%"

    def box_item(self):
        if self.r.random() < 0.3:
            return self.typeset_item()
        k = self.r.randint(0, 25)
        p = self.param()
        if k <= 2:
            return rf"\setbox0\hbox{{{self.hlist()}}}"
        if k == 3:
            return rf"\setbox0\vbox{{{self.vlist()}}}"
        if k == 4:
            return rf"\setbox0\hbox to{self.dim()}{{{self.hlist()}}}"
        if k <= 6:
            return rf"\global\dimen1={self.c(r'\wd0', r'\ht0', r'\dp0', self.dimexpr())} "
        if k == 7:
            return rf"\global\count1={self.numexpr()} "
        if k == 8:
            return rf"\global\advance\count1 by {self.numexpr()} "
        if k == 9:
            cell = p or r"\vrule height 2pt"
            return rf"\setbox0\vbox{{\halign{{##\cr {cell}\x}}}}"
        if k == 10:
            return rf"\setbox0\vbox{{\halign{{##\hfil&##\cr \vrule width{self.dim()}&\inner\cr\noalign{{\kern1pt}}\x}}}}"
        if k == 11:
            return self.cond(self.box_item_simple(), self.box_item_simple())
        if k == 12:
            return r"\begingroup " + self.box_item_simple() + r"\endgroup "
        if k == 13:
            return r"\begingroup\aftergroup\inner " + self.box_item_simple() + r"\endgroup "
        if k == 14:
            return r"\afterassignment\inner\count5=" + self.num() + " "
        if k == 15:
            return r"\global\setbox4\hbox{" + self.hlist() + "}"
        if k == 16:
            return r"\csname c\the\count2\endcsname "
        if k == 17:
            return r"\edef\tmp{\the\count2 \the\toks2}\global\toks4=\expandafter{\tmp}"
        if k == 18:
            self.uses_input = True
            return r"\input u "
        if k == 19 and self.fonts:
            return r"\font\fc=cmr10 \setbox0\hbox{\fc A}\global\dimen1=\wd0 "
        if k == 20:
            return r"\setbox0\hbox{\unhcopy2}\global\dimen1=\wd0 "
        if k == 21:
            return r"\setbox0\vbox{\hsize=50pt \noindent\vrule width 30pt\ \vrule width 30pt\par\global\count1=\prevgraf}"
        if k == 22:
            return r"\global\let\z=" + self.c(r"\inner", r"\relax", "a", r"\x", r"\y")
        if k == 23:
            return r"\setbox0\hbox{\lowercase{\global\toks4={AbC}}}"
        if k == 24:
            return r"\setbox0\hbox{\vrule width\fontdimen" + self.c("2", "6", "8") + r"\nullfont}\global\dimen1=\wd0 "
        return self.pure_item()

    def box_item_simple(self):
        k = self.r.randint(0, 4)
        if k == 0:
            return rf"\setbox0\hbox{{{self.hlist(1)}}}\global\dimen1=\wd0 "
        if k == 1:
            return rf"\global\count1={self.numexpr()} "
        if k == 2:
            return rf"\global\dimen1={self.dimexpr()} "
        if k == 3:
            return r"\global\advance\count3 by 1 "
        return self.pure_item_simple()

    def pure_item_simple(self):
        k = self.r.randint(0, 5)
        if k == 0:
            return rf"\global\count3={self.numexpr()} "
        if k == 1:
            return r"\global\let\z" + self.c(r"\inner", r"\relax", r"\y", r"\x")
        if k == 2:
            return r"\xdef\z{\the\count2 \noexpand\inner}"
        if k == 3:
            return r"\global\toks4=\expandafter{\the\toks2}"
        if k == 4:
            return r"\global\advance\count1 by " + self.numexpr() + " "
        return r"\gdef\z{" + self.c("a", r"\inner", r"\x") + "}"

    def pure_item(self):
        k = self.r.randint(0, 22)
        if k <= 1:
            return self.pure_item_simple()
        if k == 2:
            return self.cond(self.pure_item_simple(), self.pure_item_simple())
        if k == 3:
            return r"\begingroup\count3=" + self.num() + r" \global\count1=\count3 \endgroup "
        if k == 4:
            return r"\edef\tmp{\the\count2}\global\count1=\tmp\relax "
        if k == 5:
            return r"\expandafter\global\expandafter\let\csname c\the\count3\endcsname\inner "
        if k == 6:
            return r"\global\count1=\numexpr\count2*" + self.num() + r"+\count3\relax "
        if k == 7:
            return r"\catcode`\A=" + self.c("11", "12") + r" \global\count1=\catcode`\A "
        if k == 8:
            return r"\afterassignment\inner\global\count3=" + self.numexpr() + " "
        if k == 9:
            return r"\begingroup\aftergroup\inner\endgroup "
        if k == 10:
            return r"\xdef\z{\meaning\inner}"
        if k == 11:
            return r"\xdef\z{\detokenize\expandafter{\the\toks2}}"
        if k == 12:
            return r"\xdef\z{\string\inner\romannumeral\count2 }"
        if k == 13:
            return r"\global\count1=\pdfstrcmp{\the\toks2}{\inner}\relax "
        if k == 14:
            return r"\lccode`\A=`\b \lowercase{\gdef\z{A}}"
        if k == 15:
            return r"\global\toks4=\expandafter{\the\toks2 x}"
        if k == 16:
            return r"\futurelet\z\inner"
        if k == 17:
            return r"\global\count1=\ifcsname c\the\count2\endcsname 1\else 0\fi\relax "
        if k == 18:
            return r"\inner "
        if k == 19:
            return r"\global\escapechar=" + self.c("92", "-1", "47") + r" \xdef\z{\string\y}\global\escapechar=92 "
        if k == 20:
            return r"\expandafter\def\csname c\the\count2\endcsname{\global\count3=4 }\csname c\the\count2\endcsname"
        if k == 21:
            return r"\global\dimen1=" + self.dimexpr() + " "
        return r"\count3=" + self.num() + r" \global\count1=\count3 "

    def last_item(self):
        """An item that, last in the body, reads on past the end of the call."""
        p = self.param()
        return self.c(
            r"\global\count1=5",
            r"\global\count1=\count2",
            r"\global\dimen1=1pt",
            r"\global\advance\count1 by",
            r"\global\let\z=",
            r"\global\skip2=1pt",
            r"\global\dimen1=" + (p if p else "2pt"),
            r"\futurelet\z\relax",
            r"\global\count1=\numexpr\count2",
            r"\afterassignment\inner\global\count3=1",
            r"\global\toks4=\expandafter",
        )

    def body(self):
        n = self.r.randint(1, 5)
        items = []
        for _ in range(n):
            items.append(self.box_item() if self.style == "box"
                         else self.pure_item())
        if self.r.random() < 0.35:
            items.append(self.last_item())
        return items

    # -- definitions and steps --------------------------------------------
    def inner_def(self):
        return self.c(
            r"\def\inner{}%",
            r"\def\inner{\global\advance\count3 by 1 }%",
            r"\def\inner{\kern1pt}%",
            r"\let\inner\relax%",
            r"\def\inner{\x}%",
            r"\def\inner{\global\let\z\y}%",
            r"\def\inner{\count5=\count2 }%",
            r"\def\inner{\vrule width 2pt}%",
            r"\def\inner{\global\advance\dimen1 by 1pt }%",
        )

    def x_def(self):
        return self.c(
            r"\let\x\cr%", r"\let\x\relax%", r"\def\x{\cr\noalign{\kern5pt}}%",
            r"\def\x{\cr}%", r"\let\x\crcr%", r"\def\x{}%",
            r"\let\x\span%", r"\def\x{\global\advance\count3 by 1 }%",
        )

    def arg(self):
        opts = ["{7}", "{1}", "{2pt}", r"{\vrule height 2pt}", r"{\count2}",
                r"{\kern3pt}", "{-1}", r"{\the\toks3}", r"{\inner}", "{12}",
                r"{\vrule height 2pt\x}", r"{\dimen2}", "{}", r"{\x}"]
        return self.c(*opts)

    def call(self):
        if self.delim:
            args = "".join(self.c("7.", "1pt.", r"\count2.", "{1}.", "3.")
                           for _ in range(self.arity))
        else:
            args = "".join(self.arg() for _ in range(self.arity))
        tail = self.c("", "", "", " ", r"\relax", r"7\relax", "8\\relax",
                      " plus 1pt\\relax", " by 2\\relax", "\\par", "\\x", "a",
                      r"\inner", "{}")
        k = self.r.randint(0, 9)
        core = r"\measure" + args
        if k == 0:
            return r"\setbox4\hbox{" + core + tail + "}" + r"\show%"
        if k == 1:
            return r"\begingroup " + core + tail + r"\endgroup\show%"
        if k == 2:
            return r"\caller{" + core + "}" + tail + r"\show%"
        if k == 3 and self.arity == 1 and not self.delim:
            return r"\pcaller{" + self.c(r"7\relax", "1", r"\vrule height 2pt") + r"}\show%"
        if k == 3 and self.arity == 0:
            # the caller's own #1 is read past the end of the call
            return r"\qcaller{" + self.c(r"7\relax", r"8\relax", "plus1pt", r"\x") + r"}\show%"
        if k == 5:
            return r"\expandafter" + core + r"\csname relax\endcsname" + tail + r"\show%"
        if k == 6:
            self.uses_input = True
            return r"\input u \show%"
        if k == 4:
            return r"\setbox4\vbox{\halign{#\cr " + core + tail + r"\cr}}\show%"
        return core + tail + r"\show%"

    def long_mutation(self):
        r"""A long token list: \toks2 doubled to 40, 81,920 or 163,840 tokens
        (BOX-MEMO's key keeps lists up to 65,536), sometimes with a token
        changed at its end, and copied into \inner or a call's argument."""
        n = self.c("3", "13", "14", "15")
        out = r"\toks2={\kern.1pt}" + r"\dbl" * int(n)
        k = self.r.randint(0, 4)
        if k == 1:
            out += r"\toks2=\expandafter{\the\toks2 \kern.2pt}"
        elif k == 2:
            out += r"\edef\inner{\the\toks2}"
        elif k == 3 and self.arity == 1 and not self.delim:
            out += r"\expandafter\measure\expandafter{\the\toks2}\show"
        return out + "%"

    def mutation(self):
        if self.r.random() < 0.06:
            return self.long_mutation()
        if self.r.random() < 0.3:
            return self.param_mutation()
        k = self.r.randint(0, 34)
        if k <= 2:
            return rf"\count2={self.num()} %"
        if k <= 4:
            return rf"\dimen2={self.dim()} %"
        if k == 5:
            return rf"\count3={self.num()} %"
        if k == 6:
            return self.inner_def()
        if k == 7:
            return self.x_def()
        if k == 8:
            return r"\toks2={" + self.c(r"\kern1pt", "ab", r"\inner", "",
                                        r"\vrule width3pt") + "}%"
        if k == 9:
            return r"\toks3={" + self.c("5", "6", r"\count2") + "}%"
        if k == 10:
            return r"\def\c" + self.c("0", "1", "2") + "{" + self.c(
                r"\global\advance\count1 by 1 ", r"\kern2pt", "") + "}%"
        if k == 11:
            return r"\let\c" + self.c("0", "1", "2") + r"\undefined%"
        if k == 12:
            return r"\everyeof{" + self.c(r"7\relax", r"8\relax", "",
                                          r"\inner") + "}%"
        if k == 13:
            return r"\everyhbox{" + self.c("", r"\kern1pt", r"\inner") + "}%"
        if k == 14:
            return r"\fontdimen" + self.c("2", "6") + r"\nullfont=" + self.dim() + "%"
        if k == 15:
            return r"\parshape 1 0pt " + self.c("50pt", "80pt", "100pt") + "%"
        if k == 16:
            return r"\hsize=" + self.c("50pt", "100pt", "70pt") + "%"
        if k == 17:
            return r"\catcode`\A=" + self.c("11", "12") + "%"
        if k == 18:
            return "\\lccode`\\A=`\\" + self.c("a", "b") + "%"
        if k == 19:
            return r"\setbox2\hbox{" + self.c(r"\vrule width3pt", r"\kern5pt",
                                              "", r"\hskip 2pt") + "}%"
        if k == 20:
            return r"\let\y=" + self.c("a", "A", r"\relax", r"\x") + "%"
        if k == 21:
            return r"\escapechar=" + self.c("92", "-1", "64") + "%"
        if k == 22:
            return r"\globaldefs=" + self.c("0", "0", "1") + "%"
        if k == 23:
            return r"\tracingonline=1 \tracingcommands=" + self.c("0", "2") + "%"
        if k == 24:
            return r"\tracingmacros=" + self.c("0", "1", "2") + "%"
        if k == 25:
            return r"\shipout\copy4%"
        if k == 26:
            return r"\shipout\hbox{\copy2\vrule height \dimen1}%"
        if k == 27:
            return r"\widowpenalties 2 " + self.c("100", "200") + " 0%"
        if k == 28:
            return r"\begingroup\count2=" + self.num() + r" " + self.call().rstrip("%") + r"\endgroup%"
        if k == 29 and self.fonts:
            return r"\font\fb=cmr" + self.c("7", "10") + "%"
        if k == 30:
            return r"\skip2=" + self.dim() + " plus " + self.dim() + "%"
        if k == 31:
            return r"\advance\count2 by 1 %"
        if k == 32:
            return "\\uccode`\\a=`\\" + self.c("A", "B") + "%"
        if k == 33:
            self.uses_input = True
            return r"\input u \show%"
        return r"\let\measure\measure%"

    def u_file(self):
        return self.c(
            "\\measure" + "{1}" * self.arity + "\n",
            "\\relax\\measure" + "{1}" * self.arity + "\n",
            "\\global\\count3=\\count2 \n",
            "\\global\\advance\\count1 by 1\n",
            "7\\relax\n",
            "\n",
            "\\global\\dimen1=\\dimen2 \\inner\n",
        )

    def build(self):
        lines = list(PRELUDE)
        lines.append(r"\pdfoutput=" + self.c("0", "1") + " %")
        if self.fonts:
            lines.append(r"\font\fa=cmr10 \font\fb=cmr7 %")
        lines.append(r"\let\x\cr \let\y a\let\z\relax%")
        lines.append(self.inner_def())
        lines.append(r"\setbox2\hbox{\vrule width 4pt}%")
        lines.append(r"\toks2={\kern1pt}\toks3={5}%")
        # the macro under test
        if self.delim:
            ptext = "".join(f"#{i}." for i in range(1, self.arity + 1))
        else:
            ptext = "".join(f"#{i}" for i in range(1, self.arity + 1))
        body = self.body()
        lines.append(r"\def\measure" + ptext + "{%")
        lines.extend(b + "%" for b in body)
        lines.append("}%")
        lines.append(r"\def\caller#1{#1}%")
        lines.append(r"\def\pcaller#1{\measure{#1}}%")
        lines.append(r"\def\qcaller#1{\measure#1}%")
        # the steps: calls, with assignments between them
        # one or two call forms, each repeated, so that a replay is possible
        # and a mutation between two equal calls tests the key
        forms = [self.call() for _ in range(self.r.randint(1, 2))]
        steps = [forms[0], forms[0]]
        for _ in range(self.r.randint(4, 12)):
            if self.r.random() < 0.5:
                x = self.r.random()
                f = self.r.choice(forms)
                if x < 0.15 and "7" in f:
                    # the same call with another token after it, or another argument
                    f = f.replace("7", self.c("8", "5"))
                elif x < 0.25 and "2pt" in f:
                    f = f.replace("2pt", "3pt")
                elif x < 0.35:
                    f = self.call()
                steps.append(f)
            else:
                steps.append(self.mutation())
        steps.append(forms[0])
        if self.regdefs:
            lines.insert(3, r"\countdef\ca=1 \countdef\cb=2 \countdef\cc=3 "
                            r"\countdef\cd=5 \dimendef\da=1 \dimendef\db=2 "
                            r"\toksdef\ta=2 \toksdef\tb=3 \toksdef\tc=4 %")
            steps = [_regnames(x) for x in steps]
            lines = lines[:4] + [_regnames(x) for x in lines[4:]]
        lines.extend(steps)
        lines.append(r"\shipout\hbox{\copy4\copy2}%" if self.r.random() < 0.3 else "%")
        lines.append(r"\end")
        files = {"t.tex": "\n".join(lines) + "\n"}
        files["u.tex"] = self.u_file()
        return files


_REGNAMES = {"count1": r"\ca", "count2": r"\cb", "count3": r"\cc",
             "count5": r"\cd", "dimen1": r"\da", "dimen2": r"\db",
             "toks2": r"\ta", "toks3": r"\tb", "toks4": r"\tc"}
_REGPAT = re.compile(r"\\(count[1235]|dimen[12]|toks[234])(?![0-9])\s?")


def _regnames(line):
    return _REGPAT.sub(lambda m: _REGNAMES[m.group(1)] + " ", line)


def generate(seed, fonts=False):
    """The case for `seed`: {filename: text}."""
    return Gen(seed, fonts=fonts).build()


if __name__ == "__main__":
    import sys
    s = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    fs = generate(s, fonts="--fonts" in sys.argv)
    for name, text in fs.items():
        print(f"%%%% {name}")
        print(text, end="")
