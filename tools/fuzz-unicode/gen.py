#!/usr/bin/env python3
"""Document generator for the Unicode (XeTeX) differential fuzzer. MIT.

    python3 tools/fuzz-unicode/gen.py SEED [--latex]

`generate(seed)` is a pure function of the seed (one `random.Random`), so a
seed number plus this file's revision is the whole test case. A document is
an `xetex -ini` plain-TeX run (catcodes set by hand, no format) made of a
preamble and 4-12 independent chunks, each balanced on its own and each on
its own lines, so `minimize.py`'s line pass never splits a group. With
`latex=True` it is an `xelatex`/fontspec document instead.

The fonts are TeX Live's OpenType and TrueType fonts named by file
(`"[name.otf]"`), so kpathsea finds them in both engines and no system font
lookup is involved: Latin Modern, TeX Gyre, Libertinus, Amiri (Arabic),
David CLM (Hebrew), and the Latin Modern, TeX Gyre and Libertinus math
fonts.
"""
import random
import sys

TEXT_FONTS = [
    # (file, scripts it covers)
    ("lmroman10-regular.otf", "latn"),
    ("lmroman10-italic.otf", "latn"),
    ("lmsans10-regular.otf", "latn"),
    ("lmmono10-regular.otf", "latn"),
    ("texgyretermes-regular.otf", "latn"),
    ("texgyrepagella-regular.otf", "latn"),
    ("texgyreheros-regular.otf", "latn"),
    ("LibertinusSerif-Regular.otf", "latn hebr"),
    ("LibertinusSerif-Italic.otf", "latn"),
    ("Amiri-Regular.ttf", "arab latn"),
    ("DavidCLM-Medium.otf", "hebr latn"),
    # Variable fonts (fvar wght/wdth): xetex shapes their default instance.
    ("JunicodeVF-Roman.ttf", "latn"),
    ("PennstanderVF.ttf", "latn"),
    ("CharisSIL-Regular.ttf", "latn"),
    ("FandolSong-Regular.otf", "hani latn"),
    ("Garuda.otf", "thai latn"),
    ("Norasi.otf", "thai latn"),
]
THAI_WORDS = ["ภาษาไทย", "สวัสดีครับ", "ประเทศไทยมีประชากร", "การเขียนโปรแกรม",
              "กรุงเทพมหานคร", "๑๒๓"]
CJK_WORDS = ["中文字体", "你好世界", "排版系统，标点。", "「引号」", "汉字测试（括号）",
             "日本語の文章", "一二三四五六七八九十"]
MATH_FONTS = ["latinmodern-math.otf", "texgyretermes-math.otf",
              "texgyrepagella-math.otf", "LibertinusMath-Regular.otf"]

LATIN_WORDS = [
    "office", "difficult", "affluent", "flag", "fjord", "AVATAR", "Wo", "Ta.",
    "Yo!", "efficient", "shuffle", "Th", "fi", "ffl", "naïve", "café",
    "Zürich", "crème", "brûlée", "Ŵales", "straße", "œuvre", "Æsop",
    "1234567890", "1/2", "3/4", "“quotes”", "‘single’", "--", "---", "x–y",
    "a—b", "co-operate", "e.g.", "(paren)", "[brack]", "Ǆ", "ĳ", "ﬁ",
    "Ω", "π", "Ā", "ŏ", "ǅ", "ATT",
]
HEBREW_WORDS = ["שלום", "עולם", "ספר", "תורה", "ירושלים", "אבג", "מִלָּה",
                "בְּרֵאשִׁית", "קטן", "גדול"]
ARABIC_WORDS = ["مرحبا", "بالعالم", "كتاب", "العربية", "سلام", "لا", "الله",
                "محمد", "بِسْمِ", "قلم", "١٢٣", "ـــ"]
DIGITS = ["123", "4.5", "(67)", "2026", "-8", "9‰"]
# Directional and joining controls, and spaces XeTeX treats specially.
CONTROLS = ["\u200e", "\u200f", "\u202a", "\u202b", "\u202c", "\u202d",
            "\u202e", "\u2066", "\u2067", "\u2068", "\u2069", "\u200d",
            "\u200c", "\u00ad", "\u00a0", "\u2009", "\u061c"]

FEATURES = ["+liga", "-liga", "+kern", "-kern", "+smcp", "+onum", "+lnum",
            "+frac", "+sups", "+dlig", "+hlig", "+swsh", "+salt", "+ss01",
            "+tnum", "+pnum", "+case", "+zero", "+calt", "-calt", "+init",
            "+medi", "+fina", "+ccmp", "-mark", "+locl", "+zzzz"]
SCRIPTS = ["latn", "arab", "hebr", "grek", "cyrl", "DFLT"]
LANGS = ["DEU", "TRK", "ARA", "URD", "IWR", "ENG", "NLD", "ZZZ"]

PATTERNS = ("1ba 1be 1bi 1bo 1bu 1ca 1ce 1ci 1co 1cu 1da 1de 1di 1do 1du "
            "1fa 1fe 1fi 1fo 1fu 1la 1le 1li 1lo 1lu 1ma 1me 1mi 1mo 1mu "
            "1na 1ne 1ni 1no 1nu 1pa 1pe 1pi 1po 1pu 1ra 1re 1ri 1ro 1ru "
            "1sa 1se 1si 1so 1su 1ta 1te 1ti 1to 1tu 2ff1 2ffi 1ffl 2fl "
            "1gr 1qu 1xa 1za 1ze .of5f 1é 1ü 2ß1")

PICTURES = ["p-plain.png", "p-300dpi.png", "p-aspect.png", "p-jfif-dpi.jpg",
            "p-exif.jpg", "p-info.bmp", "p-pages.pdf", "missing-file.png"]

PREAMBLE_CATCODES = (
    "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\$=3 \\catcode`\\&=4 "
    "\\catcode`\\#=6 \\catcode`\\^=7 \\catcode`\\_=8 \\catcode`\\~=13 "
    "\\catcode`\\%=14 \\catcode`\\^^I=10 \\def~{\\penalty10000\\ }%")


class Doc:
    def __init__(self, lines, latex=False):
        self.lines = lines
        self.latex = latex

    def render(self):
        return "\n".join(self.lines) + "\n"


class Gen:
    def __init__(self, seed, area=None):
        self.r = random.Random(seed)
        self.area = area
        # LaTeX documents dense in \pdfstringdef calls (fuzz.py --replay).
        self.replay = False
        self.fonts = []      # control-sequence names of loaded text fonts
        self.font_scripts = {}
        self.math = False
        self.n = 0

    # -- helpers -------------------------------------------------------
    def prime(self, name):
        """Usually type a space in the new font (`\\ ` in a box), which makes
        its `font_glue`: with \\XeTeXinterwordspaceshaping>0, an explicit
        glue between its first two native words otherwise reads
        width(font_glue[f]) of null, which crashes xetex (#2391)."""
        if self.chance(0.85):
            return " \\setbox0=\\hbox{\\%s\\ }" % name
        return ""

    def pick(self, xs):
        return self.r.choice(xs)

    def chance(self, p):
        return self.r.random() < p

    def dim(self, lo=1, hi=30, unit="pt"):
        return "%s%s" % (self.r.choice([lo, hi, self.r.randint(lo, hi),
                                         round(self.r.uniform(lo, hi), 2)]), unit)

    def word(self, script=None):
        script = script or self.pick(["latn", "latn", "latn", "hebr", "arab", "num"])
        if script == "hebr":
            return self.pick(HEBREW_WORDS)
        if script == "arab":
            return self.pick(ARABIC_WORDS)
        if script == "num":
            return self.pick(DIGITS)
        if script == "thai":
            return self.pick(THAI_WORDS)
        if script == "hani":
            return self.pick(CJK_WORDS)
        return self.pick(LATIN_WORDS)

    def text(self, n=None, fonts=True):
        """A run of words, sometimes with font changes inside words, bidi
        controls, glyphs and explicit kerns or penalties."""
        n = n or self.r.randint(2, 14)
        out = []
        for _ in range(n):
            w = self.word()
            k = self.r.random()
            if k < 0.08 and len(w) > 2 and fonts and self.fonts:
                # A font change inside a word.
                i = self.r.randint(1, len(w) - 1)
                w = "%s{\\%s %s}" % (w[:i], self.pick(self.fonts), w[i:])
            elif k < 0.14:
                w = self.pick(CONTROLS) + w + self.pick(CONTROLS + [""])
            elif k < 0.18:
                w = w + self.pick(["\\/", "\\-", "\\kern1pt ", "\\penalty50 ",
                                   "\\hskip2pt ", "\\penalty10000 ", "\\penalty-10000 ",
                                   "\\discretionary{-}{}{}"]) + self.word()
            elif k < 0.21:
                w = w + "\\XeTeXglyph%d " % self.r.choice([0, 1, 3, 36, 60, 200, 999, 99999])
            elif k < 0.24:
                w = w + "\\char\"%X " % self.r.choice([0x41, 0xE9, 0x5D0, 0x627, 0x2014, 0xFB01, 0x1D400, 0x10FFFF])
            elif k < 0.27:
                w = "\\Uchar\"%X%s" % (self.r.choice([0x5D0, 0x627, 0x41, 0x301, 0x200F]), w)
            elif k < 0.30 and fonts and self.fonts:
                w = "{\\%s %s}" % (self.pick(self.fonts), w)
            elif k < 0.33:
                # \accent with a native font (make_accent's native branch).
                w = "\\accent%s %s%s" % (self.pick(["\"301 ", "\"7E ", "`^", "\"2D9 ", "\"5B4 ", "\"64E "]),
                                         self.pick(["e", "o", "A", "\u05d1", "^^^^0628", "{}", "\\XeTeXglyph36 "]),
                                         w)
            elif k < 0.35:
                w = "\\uppercase{%s}" % w if self.chance(0.5) else "\\lowercase{%s}" % w
            out.append(w)
        sep = self.pick([" ", " ", " ", "~", " \\ "])
        return sep.join(out)

    def fontspec(self, file):
        opts = []
        for _ in range(self.r.choice([0, 0, 1, 2, 3])):
            k = self.r.random()
            if k < 0.45:
                opts.append(self.pick(FEATURES))
            elif k < 0.6:
                opts.append("script=" + self.pick(SCRIPTS))
            elif k < 0.7:
                opts.append("language=" + self.pick(LANGS))
            elif k < 0.76:
                opts.append("mapping=tex-text")
            elif k < 0.82:
                opts.append("letterspace=%s" % self.r.choice([-5, 2, 10, 25.5]))
            elif k < 0.86:
                opts.append("extend=%s" % self.r.choice([0.8, 1.2, 1.5]))
            elif k < 0.90:
                opts.append("slant=%s" % self.r.choice([0.2, -0.167, 0.5]))
            elif k < 0.93:
                opts.append("embolden=%s" % self.r.choice([1, 2, 4]))
            elif k < 0.96:
                opts.append("color=%s" % self.r.choice(["FF0000", "00FF0080", "12", "zz"]))
            else:
                opts.append(self.pick(["vertical", "+vert", "fallback=x", "nosuch"]))
        spec = "[%s]" % file
        if opts:
            spec += ":" + ";".join(opts)
        size = self.r.choice(["", "", " at %s" % self.dim(4, 24), " scaled %d" % self.r.choice([500, 900, 1000, 1200, 2000])])
        return '"%s"%s' % (spec, size)

    # -- preamble ------------------------------------------------------
    def preamble(self):
        L = [PREAMBLE_CATCODES]
        L.append("\\tracingonline=1 \\showboxdepth=10000 \\showboxbreadth=10000 \\errorcontextlines=100%")
        trace = []
        for p, v in (("tracingcommands", "3"), ("tracingparagraphs", "1"),
                     ("tracingoutput", "1"), ("tracinglostchars", self.pick(["1", "2", "3"])),
                     ("tracingmacros", "2"), ("tracingassigns", "1"),
                     ("tracinggroups", "1"), ("tracingifs", "1"),
                     ("tracingpages", "1"), ("tracingrestores", "1"),
                     ("tracingnesting", "2"), ("XeTeXtracingfonts", "1")):
            if self.chance(0.5 if p != "tracingoutput" else 0.9):
                trace.append("\\%s=%s" % (p, v))
        L.append(" ".join(trace) + "%")
        L.append("\\hsize=%s \\vsize=%s \\parindent=%s \\baselineskip=%s \\topskip=10pt \\maxdepth=4pt%%" % (
            self.dim(80, 400), self.dim(200, 700), self.dim(0, 20), self.dim(10, 16)))
        L.append("\\tolerance=%d \\pretolerance=%d \\emergencystretch=%s \\parfillskip=0pt plus 1fil%%" % (
            self.r.choice([200, 1000, 9999, 10000]), self.r.choice([-1, 100, 1000]), self.dim(0, 10)))
        L.append("\\hyphenpenalty=%d \\exhyphenpenalty=%d \\lefthyphenmin=%d \\righthyphenmin=%d \\uchyph=%d%%" % (
            self.r.choice([0, 50, 1000]), self.r.choice([0, 50]), self.r.randint(1, 3),
            self.r.randint(1, 3), self.r.randint(0, 1)))
        # Letters for hyphenation: lc codes of the Latin letters used.
        letters = "abcdefghijklmnopqrstuvwxyzéüßèêûâ"
        L.append("".join("\\lccode`%s=`%s " % (c, c) for c in letters) + "%")
        L.append("".join("\\lccode`%s=`%s " % (c.upper(), c) for c in "abcdefghijklmnopqrstuvwxyz") + "%")
        if self.chance(0.4):
            L.append("".join("\\catcode`%s=11 " % c for c in "éüßèêûâ") + "%")
        if self.chance(0.6):
            L.append("\\patterns{%s}%%" % PATTERNS)
            L.append("\\hyphenation{of-fice dif-fi-cult af-flu-ent ef-fi-cient}%")
        # Fonts.
        files = self.r.sample(TEXT_FONTS, self.r.randint(2, 5))
        for i, (f, scripts) in enumerate(files):
            name = "f" + "abcdefgh"[i]
            self.fonts.append(name)
            self.font_scripts[name] = scripts
            L.append("\\font\\%s=%s%s%%" % (name, self.fontspec(f), self.prime(name)))
            if self.chance(0.4):
                L.append("\\hyphenchar\\%s=%s%%" % (name, self.pick(["`-", "\"2010", "-1", "\"AD"])))
        if self.chance(0.4):
            self.math = True
            mf = self.pick(MATH_FONTS)
            L.append('\\font\\ma="[%s]" at 10pt \\font\\mb="[%s]" at 7pt \\font\\mc="[%s]" at 5pt%%' % (mf, mf, mf))
            if self.chance(0.3):
                L.append('\\font\\ma="[%s]:+ssty" at 10pt%%' % mf)
            L.append("\\textfont2=\\ma \\scriptfont2=\\mb \\scriptscriptfont2=\\mc "
                     "\\textfont3=\\ma \\scriptfont3=\\mb \\scriptscriptfont3=\\mc "
                     "\\textfont1=\\ma \\scriptfont1=\\mb \\scriptscriptfont1=\\mc "
                     "\\textfont0=\\ma \\scriptfont0=\\mb \\scriptscriptfont0=\\mc%")
            L.append("\\Umathcode`+=\"2 \"1 \"2B \\Umathcode`==\"3 \"1 \"3D \\Umathcode`(=\"4 \"1 \"28 \\Umathcode`)=\"5 \"1 \"29%")
            L.append("".join("\\Umathcode`%s=\"7 \"1 `%s " % (c, c) for c in "abcxyzn") + "%")
            L.append("\\Udelcode`(=\"1 \"28 \\Udelcode`)=\"1 \"29 \\Udelcode`[=\"1 \"5B \\Udelcode`]=\"1 \"5D \\Udelcode`|=\"1 \"7C%")
            L.append("\\Umathchardef\\sum=\"1 \"1 \"2211 \\Umathchardef\\int=\"1 \"1 \"222B \\Umathchardef\\alpha=\"7 \"1 \"1D6FC%")
            L.append("\\def\\sqrt{\\Uradical \"1 \"221A }\\def\\hat{\\Umathaccent \"0 \"1 \"302 }\\def\\widehat{\\Umathaccent \"0 \"1 \"302 }%")
            L.append("\\thinmuskip=3mu \\medmuskip=4mu plus 2mu minus 4mu \\thickmuskip=5mu plus 5mu \\delimiterfactor=901 \\delimitershortfall=5pt \\scriptspace=0.5pt%")
        L.append("\\%s%%" % self.fonts[0])
        for p, vals in (("XeTeXinterwordspaceshaping", ["0", "1", "2"]),
                        ("XeTeXdashbreakstate", ["0", "1"]),
                        ("XeTeXupwardsmode", ["0", "0", "0", "1"]),
                        ("XeTeXuseglyphmetrics", ["0", "1"]),
                        ("XeTeXinputnormalization", ["0", "0", "1", "2"]),
                        ("XeTeXtracingfonts", ["0", "1"]),
                        ("XeTeXgenerateactualtext", ["0", "1"]),
                        ("XeTeXhyphenatablelength", ["3", "8", "63", "255"])):
            if self.chance(0.5):
                L.append("\\%s=%s%%" % (p, self.pick(vals)))
        if self.chance(0.25):
            L.append('\\XeTeXlinebreaklocale "%s" \\XeTeXlinebreakskip=0pt plus %s \\XeTeXlinebreakpenalty=%d%%' % (
                self.pick(["th", "ja", "en", "zh"]), self.dim(0, 2), self.r.choice([0, 50])))
        if self.chance(0.3):
            L.append("\\output={\\showbox255 \\shipout\\box255}%")
        return L

    # -- chunks --------------------------------------------------------
    def chunk_paragraph(self):
        L = []
        if self.chance(0.5):
            L.append("\\hsize=%s \\rightskip=%s \\leftskip=%s%%" % (
                self.dim(40, 300), self.pick(["0pt", "0pt plus 1fil", "0pt plus 20pt"]),
                self.pick(["0pt", "5pt", "0pt plus 1fil"])))
        if self.chance(0.3):
            L.append("\\XeTeXinterwordspaceshaping=%d%%" % self.r.randint(0, 2))
        if self.chance(0.2):
            L.append("\\looseness=%d%%" % self.r.choice([-1, 1]))
        mode = self.pick(["par", "vbox", "vbox", "hbox"])
        txt = self.text(self.r.randint(6, 40))
        if mode == "par":
            L.append("\\noindent %s\\par" % txt if self.chance(0.3) else txt + "\\par")
        elif mode == "vbox":
            L.append("\\setbox1=\\vbox{%s\\par}\\showbox1 \\shipout\\box1%%" % txt)
        else:
            L.append("\\setbox1=\\hbox%s{%s}\\showbox1 \\message{[\\the\\wd1][\\the\\ht1][\\the\\dp1]}\\shipout\\box1%%" % (
                self.pick(["", " to %s" % self.dim(20, 300), " spread %s" % self.dim(-10, 30)]), txt))
        return L

    def chunk_bidi(self):
        L = []
        script = self.pick(["hebr", "arab"])
        fonts = [f for f in self.fonts if script in self.font_scripts[f]]
        fnt = fonts[0] if fonts else self.fonts[0]
        words = []
        for _ in range(self.r.randint(3, 12)):
            s = self.pick([script, script, "latn", "num"])
            w = self.word(s)
            if self.chance(0.2):
                w = self.pick(CONTROLS) + w + self.pick(CONTROLS)
            if self.chance(0.1):
                w = "{\\%s %s}" % (self.pick(self.fonts), w)
            words.append(w)
        body = " ".join(words)
        if self.chance(0.3):
            L.append("\\TeXXeTstate=1%")
            body = self.pick(["\\beginR ", "\\beginL "]) + body + self.pick(["\\endR", "\\endL", ""])
        L.append("\\XeTeXinterwordspaceshaping=%d%%" % self.r.randint(0, 2))
        L.append("\\setbox2=\\%s{\\%s %s%s}\\showbox2 \\shipout\\box2%%" % (
            self.pick(["hbox", "vbox"]), fnt, body, "\\par" if self.chance(0.5) else ""))
        return L

    def chunk_interchar(self):
        L = ["\\XeTeXinterchartokenstate=%d%%" % self.pick([1, 1, 0])]
        classes = list(range(1, self.r.randint(2, 5)))
        chars = ["`a", "`o", "`,", "`.", "`(", "`)", "\"5D0", "\"627", "`1", "\"3000", "`!"]
        for c in self.r.sample(chars, self.r.randint(2, 6)):
            L.append("\\XeTeXcharclass %s=%d%%" % (c, self.pick(classes + [0, 4095])))
        for _ in range(self.r.randint(1, 4)):
            a, b = self.pick([0, 4095] + classes), self.pick([0, 4095] + classes)
            tok = self.pick(["\\kern%s" % self.dim(0, 3), "\\hskip 1pt plus 1pt", "{\\%s}" % self.pick(self.fonts),
                             "\\penalty 0 ", "\\message{ic}", "\\penalty10000\\hskip0pt", "\\kern-1pt", "\\relax"])
            L.append("\\XeTeXinterchartoks %d %d = {%s}%%" % (a, b, tok))
        L.append("\\setbox3=\\hbox{%s}\\showbox3 \\shipout\\box3%%" % self.text(self.r.randint(3, 10)))
        if self.chance(0.5):
            L.append("\\XeTeXinterchartokenstate=0%")
        return L

    def chunk_queries(self):
        f = self.pick(self.fonts)
        qs = [
            "\\the\\XeTeXcountglyphs\\%s" % f, "\\the\\XeTeXfonttype\\%s" % f,
            "\\the\\XeTeXfirstfontchar\\%s" % f, "\\the\\XeTeXlastfontchar\\%s" % f,
            "\\the\\XeTeXOTcountscripts\\%s" % f,
            "\\the\\XeTeXOTscripttag\\%s 0" % f,
            "\\the\\XeTeXOTcountlanguages\\%s \"6C61746E" % f,
            "\\the\\XeTeXOTcountfeatures\\%s \"6C61746E 0" % f,
            "\\the\\XeTeXOTfeaturetag\\%s \"6C61746E 0 1" % f,
            "\\the\\XeTeXcharglyph`%s" % self.pick(["A", "f", "é", "\\^^^^05d0", "1"]),
            "\\the\\XeTeXglyphindex \"%s\" " % self.pick(["f_f_i", "A", "uni05D0", "nosuch", "fi", ".notdef"]),
            "\\fontname\\%s" % f, "\\the\\fontdimen%d\\%s" % (self.r.randint(1, 8), f),
            "\\the\\XeTeXcountvariations\\%s" % f,
            "\\the\\XeTeXisdefaultselector\\%s 0" % f,
            "\\XeTeXglyphname\\%s %d" % (f, self.r.choice([0, 1, 36, 100, 9999])),
            "\\the\\XeTeXfindvariationbyname\\%s \"wght\"" % f,
            "\\the\\XeTeXfindfeaturebyname\\%s \"Ligatures\"" % f,
            "\\the\\XeTeXglyphbounds%d %d" % (self.r.randint(1, 4), self.r.choice([0, 3, 36, 70])),
            "\\the\\XeTeXcharclass`a",
            "\\the\\XeTeXinterwordspaceshaping",
            "\\the\\XeTeXmathcode`a", "\\the\\Umathcode`+" if self.math else "\\the\\Umathcodenum`x",
            "\\the\\Umathcodenum`=",
            "\\the\\fontcharwd\\%s`%s" % (f, self.pick(["A", "é", "f", "\\^^^^05d0", "\\^^^^0627"])),
            "\\the\\fontcharht\\%s`%s" % (f, self.pick(["A", "g", "\\^^^^fb01"])),
            "\\the\\fontchardp\\%s`%s" % (f, self.pick(["g", "y", "("])),
            "\\the\\fontcharic\\%s`%s" % (f, self.pick(["f", "A"])),
            "\\iffontchar\\%s`%s Y\\else N\\fi" % (f, self.pick(["A", "\\^^^^05d0", "\\^^^^0e01", "\\^^^^4e2d"])),
            "\\the\\XeTeXlastfontchar\\%s" % f,
        ]
        return ["{\\%s %s}%%" % (f, "".join("\\message{[%s]}" % q
                                             for q in self.r.sample(qs, self.r.randint(2, 7))))]

    def chunk_ligkern(self):
        """Native words with ligatures and kerns across font changes and
        \\/, \\noboundary, \\char and explicit kerns."""
        a, b = self.pick(self.fonts), self.pick(self.fonts)
        pairs = ["f{\\%s f}i" % b, "of{\\%s f}ice" % b, "A{\\%s V}A" % b,
                 "T{}o", "f\\/f", "f\\noboundary fi", "W{\\%s o}" % b,
                 "ff\\kern0pt i", "fi\\char`f l", "AV\\-AT", "{\\%s ffi}" % b,
                 "\\%s fl\\%s ig\\%s ht" % (a, b, a), "Ta{\\%s .}" % b, "L'A", "f\\penalty10000 i"]
        body = " ".join(self.r.sample(pairs, self.r.randint(2, 6)))
        return ["\\setbox4=\\hbox{\\%s %s}\\showbox4 \\shipout\\box4%%" % (a, body)]

    def chunk_glyph(self):
        f = self.pick(self.fonts)
        items = []
        for _ in range(self.r.randint(1, 6)):
            items.append("\\XeTeXglyph%d" % self.r.choice([0, 1, 2, 3, 36, 66, 100, 300, 65535]))
            if self.chance(0.4):
                items.append(self.word())
        return ["\\setbox5=\\hbox{\\%s %s}\\showbox5 \\shipout\\box5%%" % (f, " ".join(items))]

    def chunk_hyph(self):
        f = self.pick(self.fonts)
        words = " ".join(self.pick(["office", "difficult", "affluent", "efficient",
                                    "fluffiest", "different", "biological",
                                    "straße", "Zürich", "café", "crème",
                                    "co-operate", "x\\-y", "mini\\-mum",
                                    "remarkable", "effective"]) for _ in range(self.r.randint(5, 25)))
        L = []
        if self.chance(0.4):
            L.append("\\language=%d%%" % self.r.choice([0, 0, 1]))
        L.append("\\setbox6=\\vbox{\\hsize=%s \\%s %s\\par}\\showbox6 \\shipout\\box6%%" % (
            self.dim(20, 120), f, words))
        if self.chance(0.4):
            L.append("\\showhyphens{\\%s %s}%%" % (f, words) if False else "\\setbox7=\\vbox{\\hsize=1sp \\hbadness=10000 \\%s\\hskip0pt %s\\par}\\showbox7%%" % (f, words))
        return L

    def chunk_math(self):
        if not self.math:
            return self.chunk_paragraph()
        atoms = ["a+b", "x^2", "x_n^2", "\\sum_{n=1}^{x}", "\\int_a^b", "\\sqrt{x+y}",
                 "\\hat{a}", "\\widehat{xyz}", "{a\\over b}", "{a\\atop b}",
                 "{a\\above 1pt b}", "\\left(x\\middle| y\\right)", "\\left[{a\\over b}\\right]",
                 "\\Umathchar\"7\"1\"1D44E", "\\Umathchar\"1\"1\"2211 ", "\\alpha",
                 "\\Umathcharnum\"E0003D ", "\\mathchoice{a}{b}{c}{x}", "\\scriptstyle x",
                 "\\displaystyle\\sum", "\\Udelimiter\"4\"1\"28 ", "\\Uoverdelimiter\"1\"23DE {x+y}",
                 "\\Uunderdelimiter\"1\"23DF {x+y}", "\\Uroot\"1\"221A {3}{x}",
                 "\\mathop{\\Umathchar\"1\"1\"222B }\\limits_a^b", "(a)", "x\\mkern3mu y",
                 "\\Umathaccent bottom \"0\"1\"332 {x}",
                 "\\Umathaccent fixed \"0\"1\"303 {x}"]
        body = " ".join(self.r.sample(atoms, self.r.randint(1, 5)))
        L = []
        if self.chance(0.3):
            L.append("\\XeTeXmathcode`b=\"7 \"1 `b \\Umathcodenum`c=\"E00063%")
        if self.chance(0.3):
            L.append("\\Umathchardef\\foo=\"0 \"1 \"%X \\message{[\\meaning\\foo]}%%" % self.r.choice([0x41, 0x1D400, 0x2192]))
        if self.chance(0.5):
            L.append("\\setbox8=\\hbox{$%s$}\\showbox8 \\shipout\\box8%%" % body)
        else:
            L.append("\\setbox8=\\vbox{\\noindent text $$%s%s$$\\par}\\showbox8 \\shipout\\box8%%" % (
                body, self.pick(["", "\\eqno(1)", "\\leqno(2)"])))
        return L

    def chunk_picture(self):
        pic = self.pick(PICTURES)
        prim = "\\XeTeXpdffile" if pic.endswith(".pdf") else "\\XeTeXpicfile"
        kw = self.r.sample([" width %s" % self.dim(5, 100), " height %s" % self.dim(5, 100),
                            " scaled %d" % self.r.choice([500, 1000, 2000]),
                            " rotated %s" % self.r.choice([0, 45, 90, -30, 180.5]),
                            " xscaled 500", " yscaled 2000"] +
                           ([" page %d" % self.r.choice([0, 1, 2, 5, 9]),
                             " %s" % self.pick(["crop", "media", "bleed", "trim", "art"])] if pic.endswith(".pdf") else []),
                           self.r.randint(0, 3))
        L = ["\\setbox9=\\hbox{%s \"%s\"%s x}\\showbox9 \\shipout\\box9%%" % (prim, pic, "".join(kw))]
        if pic.endswith(".pdf") and self.chance(0.5):
            L.append("\\message{[\\the\\XeTeXpdfpagecount \"%s\"]}%%" % pic)
        return L

    def chunk_fontchange(self):
        """A new font in the middle, \\fontdimen changes, letterspace and
        \\XeTeXinterwordspaceshaping around font switches."""
        f, scripts = self.pick(TEXT_FONTS)
        name = "fz" + "abcdefghijklmnopqrstuvwxyz"[self.n % 26] * (1 + self.n // 26)
        self.n += 1
        L = ["\\font\\%s=%s%s%%" % (name, self.fontspec(f), self.prime(name))]
        if self.chance(0.4):
            L.append("\\fontdimen2\\%s=%s \\fontdimen3\\%s=%s%%" % (name, self.dim(1, 8), name, self.dim(0, 3)))
        L.append("\\setbox10=\\hbox{\\%s %s}\\showbox10 \\shipout\\box10%%" % (name, self.text(self.r.randint(2, 10))))
        self.fonts.append(name)
        self.font_scripts[name] = scripts
        return L

    def chunk_locale(self):
        """ICU line breaking of native words: Thai (dictionary breaks) and
        CJK, under \\XeTeXlinebreaklocale, in narrow paragraphs."""
        script = self.pick(["thai", "hani"])
        f, scripts = self.pick([t for t in TEXT_FONTS if script in t[1]])
        name = "fl" + "abcdefghijklmnopqrstuvwxyz"[self.n % 26] * (1 + self.n // 26)
        self.n += 1
        L = ["\\font\\%s=%s%s%%" % (name, self.fontspec(f), self.prime(name))]
        words = []
        for _ in range(self.r.randint(3, 15)):
            words.append(self.word(self.pick([script, script, script, "latn", "num"])))
        sep = self.pick(["", "", " ", "\\hskip0pt plus 1pt "])
        L.append('\\setbox12=\\vbox{\\hsize=%s \\XeTeXlinebreaklocale "%s" \\XeTeXlinebreakskip=0pt plus %s '
                 '\\XeTeXlinebreakpenalty=%d \\%s %s%s\\par}\\showbox12 \\shipout\\box12%%' % (
                     self.dim(15, 150), self.pick(["th", "zh", "ja", "", "en"]), self.dim(0, 2),
                     self.r.choice([0, 0, 50, 10000]), name, sep.join(words),
                     self.pick(["", " \\" + self.pick(self.fonts) + " " + self.word("latn")])))
        return L

    def chunk_encoding(self):
        """\\XeTeXinputencoding for the next line, then back to UTF-8; a
        TECkit mapping (tex-text) and \\Ucharcat."""
        # XeTeX's built-in names and one ICU does not know; ICU converter
        # names (latin1, cp1252, ...) are #2382, a known gap, left out so
        # it does not mask other findings.
        enc = self.pick(["bytes", "utf8", "auto", "nosuch"])
        f = self.pick(self.fonts)
        L = ['\\XeTeXinputencoding "%s"%%' % enc,
             "\\setbox13=\\hbox{\\%s %s}\\showbox13 \\shipout\\box13%%" % (
                 f, self.text(self.r.randint(2, 6), fonts=False)),
             '\\XeTeXinputencoding "utf8"%']
        if self.chance(0.5):
            L.append("\\setbox13=\\hbox{\\%s \\Ucharcat \"%X 11 \\Ucharcat `x 12 \\Uchar \"%X }\\showbox13%%" % (
                f, self.r.choice([0x41, 0x5D0, 0x627, 0xE9, 0x1F600]), self.r.choice([0x301, 0x200D, 0xFB01])))
        if self.chance(0.4):
            m = "fm" + "abcdefghijklmnopqrstuvwxyz"[self.n % 26] * (1 + self.n // 26)
            self.n += 1
            L.append('\\font\\%s="[%s]:mapping=tex-text"\\setbox13=\\hbox{\\%s ``a\'\' -- b --- c ,,d<<e>> !` ?`}'
                     '\\showbox13 \\shipout\\box13%%' % (m, self.pick(TEXT_FONTS)[0], m))
        return L

    def chunk_print(self):
        """Printing Unicode: \\message, \\write, \\show, \\string, \\meaning,
        \\detokenize and \\csname of odd characters (controls, bidi marks,
        combining marks, astral and private-use characters, U+FFFD, a lone
        U+FEFF), with \\newlinechar and \\escapechar changes, and box dumps
        cut short by small \\showboxbreadth and \\showboxdepth."""
        odd = ["^^^^0007", "^^^^007f", "^^^^0085", "^^^^200f", "^^^^202e", "^^^^0301",
               "^^^^fffd", "^^^^feff", "^^^^e000", "^^^^^1f600", "^^^^^10ffff", "^^^^00ad",
               "^^^^2028", "^^^^0000", "^^^^001b", "é", "ß", "א", "ا"]
        s = "".join(self.pick(odd) for _ in range(self.r.randint(1, 6)))
        k = self.r.random()
        if k < 0.2:
            return ["\\message{[%s]}%%" % s]
        if k < 0.35:
            return ["{\\newlinechar=%s \\immediate\\write16{[%s]}}%%" % (self.pick(["-1", "10", "`^^^^0085", "`é"]), s)]
        if k < 0.5:
            return ["\\def\\x{%s}{\\escapechar=%s \\show\\x \\message{\\meaning\\x}}%%" % (
                s, self.pick(["-1", "92", "`é", "\"1F600", "\"200F"]))]
        if k < 0.6:
            return ["\\message{\\detokenize{%s}\\string\\%s}%%" % (s, self.pick(["é", "א", "x", "^^^^0301"]))]
        if k < 0.7:
            return ["\\expandafter\\def\\csname q%s\\endcsname{1}\\expandafter\\show\\csname q%s\\endcsname%%" % (s, s)]
        if k < 0.8:
            return ["\\showtokens{%s}%%" % s]
        f = self.pick(self.fonts)
        return ["{\\showboxbreadth=%d \\showboxdepth=%d \\setbox14=\\hbox{\\%s %s \\hbox{\\%s %s}}\\showbox14}%%" % (
            self.r.choice([-1, 0, 1, 2, 3]), self.r.choice([-1, 0, 1, 2]), f,
            self.text(self.r.randint(2, 8), fonts=False), f, s)]

    def chunk_showlists(self):
        return ["\\setbox11=\\hbox{%s\\showlists}\\shipout\\box11%%" % self.text(self.r.randint(1, 5))]

    def chunks(self):
        kinds = [self.chunk_paragraph] * 4 + [self.chunk_bidi] * 3 + [self.chunk_interchar] * 2 + \
                [self.chunk_queries] * 2 + [self.chunk_ligkern] * 3 + [self.chunk_glyph] + \
                [self.chunk_hyph] * 2 + [self.chunk_math] * 3 + [self.chunk_picture] + \
                [self.chunk_fontchange] * 2 + [self.chunk_showlists] + [self.chunk_locale] * 2 + [self.chunk_encoding] + [self.chunk_print] * 2
        if self.area:
            # A dense run of one area (`--area bidi`, ...): its chunks only.
            kinds = [getattr(self, "chunk_" + self.area)]
        out = []
        for _ in range(self.r.randint(4, 12)):
            out += self.pick(kinds)()
        return out

    def generate(self):
        L = self.preamble() + self.chunks()
        L.append("\\end")
        return Doc(L)

    # -- LaTeX (xelatex + fontspec) --------------------------------------
    def generate_latex(self):
        r = self.r
        # By file name: on macOS xetex finds fonts by name through Core Text,
        # which does not see TeX Live's fonts.
        main = self.pick(["lmroman10-regular.otf", "texgyretermes-regular.otf",
                          "texgyrepagella-regular.otf", "LibertinusSerif-Regular.otf",
                          "texgyreheros-regular.otf", "CharisSIL-Regular.ttf"])
        L = ["\\documentclass{article}", "\\usepackage{fontspec}"]
        L.append("\\setmainfont{%s}%s" % (main, self.pick(["", "[Ligatures=TeX]", "[Numbers=OldStyle]",
                                                           "[Letters=SmallCaps]", "[LetterSpace=5]",
                                                           "[WordSpace=1.5]", "[Kerning=Off]"])))
        L.append("\\newfontfamily\\hebrewfont{DavidCLM-Medium.otf}[Script=Hebrew]")
        L.append("\\newfontfamily\\arabicfont{Amiri-Regular.ttf}[Script=Arabic]")
        if self.chance(0.5):
            L.append("\\usepackage{unicode-math}\\setmathfont{%s}" % self.pick(MATH_FONTS))
        siunitx = self.chance(0.4)
        if siunitx:
            L.append("\\usepackage{siunitx}")
        # hyperref: every \pdfstringdef (bookmarks, page labels, anchors)
        # runs \pdfstringdefPreHook, XeTeX's guarded intrinsic (fuzz.py
        # --replay).
        hyper = self.replay or self.chance(0.6)
        if hyper:
            L.append("\\usepackage[%s]{hyperref}" % self.pick(["", "bookmarks", "pdfencoding=auto",
                                                              "unicode,bookmarksnumbered", "colorlinks"]))
        L.append("\\begin{document}")
        for _ in range(r.randint(12, 30) if self.replay else r.randint(3, 8)):
            k = r.random()
            if hyper and k < (0.5 if self.replay else 0.15):
                title = self.pick([
                    lambda: " ".join(self.word("latn") for _ in range(r.randint(1, 4))),
                    lambda: "{\\hebrewfont %s}" % self.word("hebr"),
                    lambda: "Über \\texorpdfstring{$x^2$}{x2} café",
                    lambda: "\\qty{3.5}{\\kilo\\metre} and \\num{12345}" if siunitx else "Größe ½",
                    lambda: "\\textbf{Bold} \\emph{ēmph} — “q”",
                ])()
                L.append("\\%s{%s}\\label{s%d}" % (self.pick(["section", "subsection", "section*"]), title, self.n))
                self.n += 1
                if self.chance(0.4):
                    L.append("See \\ref{s%d}, \\pageref{s%d}, \\href{https://example.org/ü}{link ü}." % (self.n - 1, self.n - 1))
                if self.chance(0.3):
                    L.append("\\newpage")
                continue
            if k < 0.3:
                L.append(self.text(r.randint(10, 60), fonts=False) + "\n")
            elif k < 0.45:
                L.append("{\\hebrewfont %s}" % " ".join(self.word("hebr") for _ in range(r.randint(2, 8))))
            elif k < 0.6:
                L.append("{\\arabicfont %s}" % " ".join(self.word("arab") for _ in range(r.randint(2, 8))))
            elif k < 0.7:
                L.append("{\\addfontfeatures{%s}%s}" % (self.pick(["Letters=SmallCaps", "Numbers=Lining", "LetterSpace=10",
                                                                 "FakeSlant=0.2", "FakeStretch=1.2", "Ligatures=Rare"]),
                                                      self.text(r.randint(3, 15), fonts=False)))
            elif k < 0.85:
                L.append("$%s$ and \\[ %s \\]" % (self.pick(["a+b=c", "x^2_n", "\\sum_{i=1}^n i", "\\frac{a}{b}"]),
                                               self.pick(["\\int_0^1 f(x)\\,dx", "\\sqrt{x^2+y^2}", "\\left(\\frac{1}{2}\\right)"])))
            else:
                L.append("\\XeTeXinterwordspaceshaping=%d %s" % (r.randint(0, 2), self.text(r.randint(5, 20), fonts=False)))
        L.append("\\end{document}")
        return Doc(L, latex=True)


AREAS = ["paragraph", "bidi", "interchar", "queries", "ligkern", "glyph", "hyph", "math",
         "picture", "fontchange", "showlists", "locale", "encoding", "print"]


def generate(seed, latex=False, area=None, replay=False):
    g = Gen(seed, area)
    g.replay = replay
    return g.generate_latex() if latex else g.generate()


if __name__ == "__main__":
    s = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    area = next((a.split("=", 1)[1] for a in sys.argv if a.startswith("--area=")), None)
    sys.stdout.write(generate(s, latex="--latex" in sys.argv, area=area).render())
