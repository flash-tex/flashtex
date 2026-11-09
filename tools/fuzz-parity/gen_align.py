#!/usr/bin/env python3
"""Alignment and output-routine grammar for the parity and replay fuzzers. MIT.

The hand audits keep finding holes in two parts of tex.web: alignments
(Part 37, `\\halign`/`\\valign`, `\\span`, `\\omit`, `\\noalign`, `\\tabskip`,
`\\cr`/`\\crcr` reached through macros, `&` through `\\let`, `\\everycr`) and
the page builder and output routines (Parts 44-46: `\\box255`, inserts,
marks, `\\vsplit`, `\\holdinginserts`, `\\deadcycles`, `\\outputpenalty`).
This module generates documents aimed only at those two parts:

* `generate(seed)` gives a `gen.Doc` for tools/fuzz-parity (`fuzz.py
  --grammar align`): the fuzz-parity preamble (INITEX + plain.tex, tracing)
  and alignment / page chunks;
* `generate_replay(seed, fonts)` gives a `{t.tex, u.tex}` case for
  tools/fuzz-replay (`fuzz_replay.py run --grammar align`): no plain.tex,
  `\\measure` holds alignment and page items, so BOX-MEMO and D9 memoise
  and replay them.

Every choice goes through one `random.Random(seed)`. Each chunk is balanced
on its own and is one line, so the line passes of both minimisers keep it
whole. Stdlib only.
"""
import os
import random
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)
import gen  # noqa: E402  (tools/fuzz-parity/gen.py: Doc, G, preamble)

GLUES = ["0pt", "0pt plus 1fil", "0pt plus 1fill", "3pt plus 1pt minus 1pt",
         "2pt", "-1pt", "0pt minus 1fil", "1pt plus 2pt", "5pt minus 5pt",
         "0pt plus -1fil", "1em plus 1fil"]
DIMS = ["0pt", "1pt", "2pt", "5pt", "10pt", "20pt", "-3pt", "1sp", "50pt",
        "100pt", "0.5\\hsize", "\\maxdimen"]
SMALL = ["0pt", "1pt", "2pt", "3pt", "5pt", "10pt", "-2pt"]
PENS = ["0", "50", "-50", "100", "10000", "-10000", "-10001", "-20000",
        "150", "-1073741824", "1000"]
INSERTS = ["100", "101", "254"]


class A:
    """The grammar. `plain` says whether plain.tex is loaded (its macros and
    fonts may be used); without it the items use primitives and `\\fa`."""

    def __init__(self, rng, plain=True, fonts=True):
        self.r = rng
        self.plain = plain
        self.fonts = fonts
        self.depth = 0
        # quiet: no \show..., \message or group/page queries (a replay
        # recording refuses output and those reads)
        self.quiet = False

    def pick(self, seq):
        return self.r.choice(seq)

    def chance(self, p):
        return self.r.random() < p

    def n(self, lo, hi):
        return self.r.randint(lo, hi)

    # -- leaves -------------------------------------------------------------
    def text(self):
        if not self.fonts:
            return self.rule()
        w = self.pick(["A", "Ab", "x", "fi", "AV", "Wo", "q", "ffl", "1", "."])
        if self.plain and self.chance(0.2):
            w = self.pick(["\\bf ", "\\it ", "\\sevenrm ", "\\tt "]) + w
        return w

    def rule(self):
        k = self.n(0, 4)
        if k == 0:
            return "\\vrule width %s" % self.pick(SMALL)
        if k == 1:
            return "\\vrule height %s depth %s width %s" % (
                self.pick(SMALL), self.pick(SMALL), self.pick(SMALL))
        if k == 2:
            return "\\kern%s" % self.pick(SMALL)
        if k == 3:
            return "\\hskip%s" % self.pick(GLUES)
        return "\\vrule"

    def vrule(self):
        return self.pick(["\\hrule", "\\hrule height %s" % self.pick(SMALL),
                          "\\kern%s" % self.pick(SMALL), "\\vskip%s" % self.pick(GLUES),
                          "\\hrule width %s depth %s" % (self.pick(SMALL), self.pick(SMALL))])

    def hstuff(self):
        k = self.n(0, 9)
        if k <= 3:
            return self.text()
        if k <= 5:
            return self.rule()
        if k == 6:
            return self.pick(["\\hfil", "\\hfill", "\\hss", "\\hskip 0pt minus 1fil"])
        if k == 7:
            return "\\penalty%s" % self.pick(PENS)
        if k == 8:
            return "\\hbox to %s{%s}" % (self.pick(SMALL), self.text())
        return "\\unskip" if self.chance(0.5) else "\\vadjust{\\kern1pt}"

    def vstuff(self):
        k = self.n(0, 6)
        if k <= 1:
            return "\\hbox{%s}" % self.text()
        if k == 2:
            return self.vrule()
        if k == 3:
            return "\\penalty%s" % self.pick(PENS)
        if k == 4:
            return "\\vbox to %s{\\vss}" % self.pick(SMALL)
        if k == 5:
            return "\\vfil" if self.chance(0.5) else "\\vss"
        return "\\noindent %s\\par" % self.text()

    # -- alignments ------------------------------------------------------------
    def template(self, vertical):
        u, v = "", ""
        k = self.n(0, 9)
        fil = "\\vfil" if vertical else "\\hfil"
        if k == 0:
            u = fil
        elif k == 1:
            v = fil
        elif k == 2:
            u, v = fil, fil
        elif k == 3:
            u, v = ("(", ")") if self.fonts and not vertical else ("\\kern1pt", "\\kern2pt")
        elif k == 4:
            u = "\\tabskip=%s " % self.pick(GLUES)
        elif k == 5:
            u = "\\span\\pre "  # \span expands \pre in the preamble
        elif k == 6:
            v = "\\relax\\the\\count7 " if not vertical else "\\relax"
        elif k == 7 and not vertical:
            u = "$" if self.fonts and self.plain else ""
            v = "$" if u else ""
        elif k == 8:
            u = "\\global\\advance\\count7 by 1 "
        if vertical and u.startswith("$"):
            u = v = ""
        if not self.quiet and self.chance(0.05):
            return u + v  # no #: "Missing # inserted in alignment preamble"
        if not self.quiet and self.chance(0.03):
            return u + "#" + v + "#"  # two #s: "Only one # is allowed per tab"
        return u + "#" + v

    def preamble(self, vertical, ncols):
        parts = []
        for i in range(ncols):
            t = self.template(vertical)
            if i and self.chance(0.4):
                # a \tabskip assignment in u_j, before the #, sets the glue
                # after this column (tex.web §778)
                t = "\\tabskip=%s " % self.pick(GLUES) + t
            parts.append(t)
        sep = self.pick(["&", "&", "&", "\\amp "])
        s = sep.join(parts)
        if self.chance(0.25):
            # a periodic preamble: && starts the repeated part
            cut = self.n(0, max(0, ncols - 1))
            ps = parts[:]
            s = sep.join(ps[:cut]) + ("&" if cut else "") + "&" + sep.join(ps[cut:])
        return s + self.cr()

    def cr(self):
        return self.pick(["\\cr", "\\cr", "\\cr", "\\crcr", "\\endrow ", "\\mycr ",
                          "\\cr\\noalign{\\kern%s}" % self.pick(SMALL)]).rstrip() + " "

    def cell(self, vertical, depth):
        bits = []
        if self.chance(0.15):
            bits.append("\\omit")
        for _ in range(self.n(0, 3)):
            if vertical:
                bits.append(self.vstuff())
            else:
                bits.append(self.hstuff())
        if self.chance(0.05):
            bits.append("\\tabskip=%s" % self.pick(GLUES))
        if not self.quiet and self.chance(0.04):
            bits.append("\\showlists")
        if not self.quiet and self.chance(0.04):
            bits.append("\\message{[\\the\\currentgrouptype/\\the\\currentgrouplevel/\\the\\lastnodetype]}")
        if not self.quiet and self.chance(0.03):
            bits.append("\\showgroups")
        if depth < 1 and self.chance(0.07):
            bits.append(self.alignment(depth + 1, vertical=self.chance(0.2), wrap=True))
        return " ".join(bits)

    def row(self, vertical, ncols, depth):
        cells = []
        k = self.n(1, ncols + (2 if not self.quiet and self.chance(0.15) else 0))
        for i in range(k):
            cells.append(self.cell(vertical, depth))
        out = ""
        for i, c in enumerate(cells):
            if i:
                out += self.pick(["&", "&", "&", "\\amp ", "\\span ", "&\\omit ", "\\span\\omit "])
            out += c
        return out + self.cr()

    def noalign(self, vertical):
        stuff = []
        for _ in range(self.n(0, 2)):
            if vertical:
                stuff.append(self.hstuff())
            else:
                stuff.append(self.pick([self.vrule(), "\\penalty%s" % self.pick(PENS),
                                        "\\vskip%s" % self.pick(GLUES), "\\mark{na}",
                                        "\\global\\tabskip=%s" % self.pick(GLUES),
                                        "\\showlists", "\\tabskip=%s" % self.pick(GLUES)]))
        return "\\noalign{%s}" % " ".join(stuff)

    def alignment(self, depth=0, vertical=None, wrap=False):
        if vertical is None:
            vertical = self.chance(0.2)
        prim = "\\valign" if vertical else "\\halign"
        spec = self.pick(["", "", "", "to %s" % self.pick(DIMS), "spread %s" % self.pick(SMALL),
                          "to 0pt", "spread -5pt"])
        ncols = self.n(1, 4)
        pre = self.preamble(vertical, ncols)
        body = []
        if self.chance(0.1):
            body.append("\\noalign{}")
        for _ in range(self.n(0, 5)):
            if self.chance(0.2):
                body.append(self.noalign(vertical))
            body.append(self.row(vertical, ncols, depth))
        if self.chance(0.08):
            body.append(self.pick(["\\crcr", "\\cr", "\\omit\\cr", "\\noalign{\\hrule}\\crcr"]))
        ts = ""
        if self.chance(0.4):
            ts = "\\tabskip=%s " % self.pick(GLUES)
        a = "%s%s%s{%s%s}" % (ts, prim, " " + spec if spec else "", pre, "".join(body))
        if wrap or self.chance(0.6):
            box = self.pick(["\\vbox", "\\vtop", "\\hbox", "\\vbox to 30pt", "\\hbox spread 3pt"])
            if box.startswith("\\hbox") and not vertical:
                box = "\\vbox"
            if box.startswith("\\v") and vertical:
                box = "\\hbox"
            a = "%s{%s}" % (box, a)
        return a

    def align_chunk(self):
        k = self.n(0, 11)
        setup = []
        if self.chance(0.3):
            setup.append("\\everycr{%s}" % self.pick(
                ["\\noalign{\\kern1pt}", "", "\\noalign{\\penalty7}", "\\noalign{\\showlists}",
                 "\\global\\advance\\count8 by 1 ", "\\noalign{\\global\\advance\\count8 1}"]))
        if self.chance(0.2):
            setup.append("\\def\\pre{%s}" % self.pick(["\\hfil", "\\tabskip=3pt", "#", "\\kern1pt", "\\pre"]))
        a = self.alignment()
        if k <= 4:
            reg = self.n(0, 3)
            if not (a.startswith("\\vbox") or a.startswith("\\vtop") or a.startswith("\\hbox")):
                a = ("\\hbox{%s}" if "\\valign" in a[:40] else "\\vbox{%s}") % a
            body = "\\setbox%d%s\\showbox%d" % (reg, a, reg)
            if self.chance(0.4):
                body += "\\message{[\\the\\wd%d/\\the\\ht%d/\\the\\dp%d]}" % (reg, reg, reg)
        elif k <= 6:
            body = "\\par %s\\par" % a  # in vertical mode: goes to the page
        elif k == 7:
            body = "\\noindent %s %s\\par" % (self.text(), a)  # \halign in a paragraph ends it
        elif k == 8 and self.plain:
            body = "\\noindent %s$$%s\\halign{%s}$$%s\\par" % (
                self.text(), self.pick(["", "\\tabskip=3pt", "\\displayindent=5pt"]),
                self.preamble(False, self.n(1, 3)) + self.row(False, 2, 1), self.text())
        elif k == 9:
            # an alignment interrupted: & or \cr outside, or \noalign/\omit misplaced
            body = self.pick(["\\hbox{a&b}", "\\cr", "\\noalign{}", "\\omit", "\\span",
                              "\\hbox{\\halign{#\\cr x}}", "\\vbox{\\halign{#&#\\cr a&b&c\\cr}}",
                              "\\vbox{\\halign{#\\cr\\noalign{\\vskip 1pt}\\omit\\span\\omit\\cr}}",
                              "\\vbox{\\halign{#\\cr a\\par b\\cr}}", "\\vbox{\\halign{#\\cr\\endtemplate}}",
                              "\\vbox{\\halign{\\span#\\cr x\\cr}}", "\\vbox{\\halign{##\\cr x\\cr}}",
                              "\\vbox{\\halign{#\\cr}\\halign{#\\cr a\\cr}}",
                              "\\vbox{\\halign{#\\cr{a&b}\\cr}}",
                              "\\vbox{\\halign{#\\cr\\global\\let\\amp=&a\\amp b\\cr}}",
                              "\\hbox{\\valign{#\\cr\\hbox{a}\\cr\\noalign{\\kern1pt}\\cr}}",
                              "\\vbox{\\halign to -10pt{#\\tabskip 0pt plus 1fil&#\\cr a&b\\cr}}",
                              "\\vbox{\\halign spread 10pt{#\\tabskip 0pt minus 1pt&#\\cr a&b\\cr}}"])
            if self.fonts is False:
                body = body.replace("a", "\\kern1pt ").replace("b", "\\kern2pt ")
        elif k == 10:
            body = "\\setbox3\\vbox{%s}\\setbox4\\vsplit3 to %s\\showbox3\\showbox4" % (
                a, self.pick(SMALL))
        else:
            # \lastbox, \unskip, \unpenalty on the alignment's rows
            body = "\\setbox5\\vbox{%s\\unskip\\unpenalty\\setbox6\\lastbox\\showbox6}\\showbox5" % a
        return "{" + "".join(setup) + body + "}"

    # -- the page builder and output routines -----------------------------
    def output_routine(self):
        parts = []
        putback = False
        for _ in range(self.n(1, 5)):
            k = self.n(0, 21)
            if k == 0:
                parts.append("\\showbox255")
            elif k == 1:
                parts.append("\\message{[op=\\the\\outputpenalty/dc=\\the\\deadcycles/ip=\\the\\insertpenalties]}")
            elif k == 2:
                parts.append("\\message{[t=\\topmark/f=\\firstmark/b=\\botmark]}")
            elif k == 3:
                parts.append("\\message{[m1=\\topmarks1/\\firstmarks1/\\botmarks1]}")
            elif k == 4:
                parts.append("\\setbox0\\vsplit255 to %s\\showbox0\\message{[sf=\\splitfirstmark/sb=\\splitbotmark]}"
                             % self.pick(SMALL + ["30pt"]))
            elif k == 5:
                parts.append("\\showbox%s" % self.pick(INSERTS))
            elif k == 6:
                parts.append("\\global\\holdinginserts=%s" % self.pick(["0", "1", "-1"]))
            elif k == 7:
                # put the page back (then no shipout: an output routine that
                # both ships and puts back can loop for ever on empty pages)
                parts.append("\\unvbox255")
                putback = True
            elif k == 8:
                parts.append("\\global\\setbox9\\box255")
            elif k == 9:
                parts.append("\\deadcycles=%s" % self.pick(["0", "1", "5", "30"]))
            elif k == 10:
                parts.append("\\showlists")
            elif k == 11:
                parts.append("\\message{[g=\\the\\pagegoal/t=\\the\\pagetotal/d=\\the\\pagedepth/s=\\the\\pageshrink]}")
            elif k == 12:
                parts.append("\\global\\vsize=%s" % self.pick(["50pt", "100pt", "20pt", "200pt"]))
            elif k == 13:
                parts.append("\\unvbox%s" % self.pick(INSERTS))
            elif k == 14:
                parts.append("\\insert100{\\hbox{re}}")
            elif k == 15:
                parts.append("\\mark{from-output}")
            elif k == 16:
                parts.append("\\vbox{\\unvcopy255}\\setbox255\\box255")
            elif k == 17:
                parts.append("\\global\\advance\\count9 by 1 ")
            elif k == 18:
                parts.append("\\outputpenalty=%s" % self.pick(PENS))
            elif k == 19:
                parts.append("\\message{[\\the\\splitmaxdepth/\\the\\splittopskip/\\the\\lastnodetype]}")
            elif k == 20:
                parts.append("\\ifnum\\outputpenalty<-9999 \\unvbox255 \\else\\shipout\\box255 \\fi")
                putback = True
            else:
                parts.append("{\\setbox0\\vbox{\\pagediscards}\\showbox0}")
        ship = self.pick([
            "\\shipout\\box255", "\\shipout\\box255", "\\shipout\\vbox{\\box255\\box100}",
            "\\shipout\\vbox{\\ifvoid100\\else\\unvbox100\\fi\\box255}", "\\setbox0\\box255",
            "", "\\shipout\\copy255 \\setbox255\\box255",
            "\\ifvoid255\\else\\shipout\\box255\\fi"])
        if putback:
            ship = ""
        if self.chance(0.5):
            parts.append(ship)
        else:
            parts.insert(0, ship)
        return "\\output={%s}" % " ".join(parts)

    def page_item(self):
        k = self.n(0, 18)
        if self.quiet and k in (11, 14, 18):
            k = 0  # page-builder integers, \showlists, \message
        if k <= 3:
            return self.vstuff()
        if k <= 5:
            return "\\insert%s{%s}" % (self.pick(INSERTS), " ".join(
                self.vstuff() for _ in range(self.n(0, 3))))
        if k == 6:
            return "\\mark{m%d}" % self.n(0, 9)
        if k == 7:
            return "\\marks%d{e%d}" % (self.n(0, 2), self.n(0, 9))
        if k == 8:
            return "\\penalty%s" % self.pick(PENS)
        if k == 9:
            return "\\noindent %s\\vadjust{\\penalty%s\\mark{va}}\\par" % (self.text(), self.pick(PENS))
        if k == 10:
            return "\\count%s=%s \\dimen%s=%s \\skip%s=%s" % (
                self.pick(INSERTS), self.pick(["0", "500", "1000", "2000"]),
                self.pick(INSERTS), self.pick(DIMS), self.pick(INSERTS), self.pick(GLUES))
        if k == 11:
            return "\\floatingpenalty=%s \\insertpenalties=%s" % (self.pick(PENS), self.pick(["0", "100"]))
        if k == 12:
            return "\\topskip=%s \\maxdepth=%s \\splitmaxdepth=%s \\splittopskip=%s" % (
                self.pick(GLUES), self.pick(SMALL), self.pick(SMALL), self.pick(GLUES))
        if k == 13:
            return "\\vsize=%s" % self.pick(["30pt", "50pt", "100pt", "300pt", "1pt"])
        if k == 14:
            return "\\showlists"
        if k == 15:
            return "\\par %s\\par" % self.alignment()
        if k == 16:
            return "\\vbox to %s{%s}" % (self.pick(DIMS[:8]), " ".join(self.vstuff() for _ in range(2)))
        if k == 17:
            return "\\savingvdiscards=%s \\holdinginserts=%s" % (self.pick(["0", "1"]), self.pick(["0", "1"]))
        return "\\message{[%s]}" % self.pick(["\\the\\pagegoal", "\\the\\pagetotal", "\\the\\deadcycles",
                                              "\\the\\insertpenalties", "\\the\\outputpenalty"])

    def output_chunk(self):
        items = []
        if self.chance(0.7):
            items.append(self.output_routine())
        if self.chance(0.3):
            items.append("\\maxdeadcycles=%s" % self.pick(["1", "2", "3", "25"]))
        for _ in range(self.n(2, 12)):
            items.append(self.page_item())
        if self.chance(0.6):
            items.append(self.pick(["\\penalty-10000", "\\vfill\\penalty-10000", "\\penalty-20000",
                                    "\\vfill\\penalty-20000", "\\eject" if self.plain else "\\penalty-10000",
                                    "\\vskip 0pt plus 1fil\\penalty-10001"]))
        # the next chunks get a normal output routine back
        tail = "\\penalty-20000\\deadcycles=0 \\output={%s}" % (
            "\\plainoutput" if self.plain else "\\shipout\\box255")
        return " ".join(items) + " " + tail

    def chunk(self):
        return self.align_chunk() if self.chance(0.5) else self.output_chunk()


PARITY_SETUP = ("\\let\\amp=& \\let\\endrow=\\cr \\def\\mycr{\\cr}\\def\\pre{\\hfil}"
                "\\count7=0 \\count8=0 \\count9=0 \\hsize=100pt \\vsize=100pt \\parindent=5pt"
                "\\tracingonline=1 \\tracingoutput=1 \\showboxdepth=10000 \\showboxbreadth=10000\n")


def generate(seed, nchunks=None, latex=None):
    """A fuzz-parity document (latex is accepted and ignored: plain only)."""
    rng = random.Random(seed)
    g = gen.G(rng)
    pdf = rng.random() < 0.7
    pre = gen.preamble(g, pdf) + PARITY_SETUP
    if rng.random() < 0.4:
        pre += "\\tracingpages=1 "
    if rng.random() < 0.2:
        pre += "\\tracingcommands=2 "
    a = A(rng, plain=True, fonts=True)
    k = nchunks or a.n(3, 10)
    chunks = [a.chunk() for _ in range(k)]
    return gen.Doc(seed, pre, chunks, pdf)


REPLAY_PRELUDE = [
    r"\catcode`\{=1 \catcode`\}=2 \catcode`\#=6 \catcode`\&=4 "
    r"\catcode`\^=7 \catcode`\$=3 \catcode`\~=13 \catcode`\^^I=10 \scrollmode",
    r"\hsize=100pt \vsize=100pt \baselineskip=12pt \tolerance=10000 "
    r"\parfillskip=0pt plus 1fil \maxdepth=2pt \maxdeadcycles=25 \showboxdepth=10000 \showboxbreadth=10000",
    r"\let\amp=& \let\endrow=\cr \def\mycr{\cr}\def\pre{\hfil}\count7=0 \count8=0 \count9=0 %",
    r"\def\show{\message{[\the\count7/\the\count8/\the\count9/\the\dimen1/\the\dimen2/\the\dimen3]}}%",
]


def generate_replay(seed, fonts=False):
    """A fuzz-replay case: {t.tex, u.tex}, `\\measure` holding alignment and
    page items, called repeatedly with assignments between calls."""
    rng = random.Random(seed)
    a = A(rng, plain=False, fonts=fonts)
    lines = list(REPLAY_PRELUDE)
    lines.append(r"\pdfoutput=%d %%" % a.n(0, 1))
    if fonts:
        lines.append(r"\font\fa=cmr10 \fa %")
    lines.append(r"\output={\shipout\box255}%")
    style = a.pick(["box", "box", "page"])
    arity = a.pick([0, 0, 1])
    a.quiet = True
    body = []
    for _ in range(a.n(1, 4)):
        # a measure call (BOX-MEMO.md §2): local boxes inside a group,
        # results out through \global registers
        if style == "box" or a.chance(0.5):
            al = a.alignment(wrap=True).replace("#", "##")  # inside \def
            reg = a.n(0, 4)
            body.append("\\begingroup\\setbox%d%s\\global\\dimen1=\\wd%d \\global\\dimen2=\\ht%d "
                        "\\global\\advance\\count9 by\\dp%d \\endgroup" % (reg, al, reg, reg, reg))
        else:
            items = " ".join(a.page_item() for _ in range(a.n(1, 5))).replace("#", "##")
            items = items.replace("\\showlists", "").replace("\\message", "\\setbox7\\hbox")
            # (no \vsplit: a recording refuses it)
            body.append("\\begingroup\\setbox0\\vbox%s{%s}\\setbox1\\copy0 "
                        "\\global\\dimen1=\\ht1 \\global\\dimen2=\\dp0 \\endgroup"
                        % (a.pick(["", " to 30pt", " spread 5pt"]), items))
    if arity:
        body.append("\\begingroup\\setbox1\\hbox{#1}\\global\\dimen3=\\wd1 \\endgroup")
    lines.append("\\def\\measure%s{%%" % ("#1" if arity else ""))
    lines.extend(b + "%" for b in body)
    lines.append("}%")
    call = "\\measure" + ("{\\kern1pt}" if arity else "")
    steps = [call + "\\show%", call + "\\show%"]
    for _ in range(a.n(3, 10)):
        x = a.r.random()
        if x < 0.45:
            steps.append(call + "\\show%")
        elif x < 0.55:
            steps.append("\\input u \\show%")
        elif x < 0.65:
            steps.append("\\everycr{%s}%%" % a.pick(["", "\\noalign{\\kern1pt}", "\\global\\advance\\count8 1 "]))
        elif x < 0.72:
            steps.append("\\def\\pre{%s}%%" % a.pick(["\\hfil", "\\kern2pt", "\\tabskip=3pt"]))
        elif x < 0.8:
            steps.append("\\tabskip=%s%%" % a.pick(GLUES))
        elif x < 0.85:
            steps.append("\\let\\endrow=%s%%" % a.pick(["\\cr", "\\crcr", "\\crcr", "\\cr", "\\relax"]))
        elif x < 0.9:
            steps.append(a.output_routine() + "%")
        elif x < 0.95:
            steps.append("\\hsize=%s \\vsize=%s%%" % (a.pick(["50pt", "100pt"]), a.pick(["40pt", "100pt"])))
        else:
            steps.append("\\vbox{" + call + "}\\show%")
    steps.append(call + "\\show%")
    lines.extend(steps)
    lines.append(r"\penalty-10000 \deadcycles=0 \output={\shipout\box255}%")
    lines.append(r"\shipout\hbox{\copy4\copy2}%" if a.chance(0.3) else "%")
    lines.append(r"\end")
    return {"t.tex": "\n".join(lines) + "\n", "u.tex": call + "\n"}


if __name__ == "__main__":
    s = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    if "--replay" in sys.argv:
        for name, text in generate_replay(s, fonts="--fonts" in sys.argv).items():
            print("%%%% " + name)
            sys.stdout.write(text)
    else:
        sys.stdout.write(generate(s).render())
