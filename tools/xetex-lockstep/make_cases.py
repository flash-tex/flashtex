#!/usr/bin/env python3
"""Write tools/xetex-lockstep/cases/*.tex: the XeTeX-specific lockstep cases.

The cases are generated, not hand-edited, because several hold bytes a text
editor would change: Latin-1 bytes after \\XeTeXinputencoding "bytes",
invalid UTF-8, CR line ends, UTF-16 files with a byte order mark. Run it after
changing a case here and commit both. Each case starts with \\input prelude
(prelude.tex here) and ships at least one box; `% lockstep: no-halt` marks a
case that ends with an error on purpose (tools/lockstep/README.md).
"""
import os

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "cases")
os.makedirs(OUT, exist_ok=True)

MATH = r"""\font\tenrm=cmr10 \font\sevenrm=cmr7 \font\fiverm=cmr5
\font\teni=cmmi10 \font\seveni=cmmi7 \font\fivei=cmmi5
\font\tensy=cmsy10 \font\sevensy=cmsy7 \font\fivesy=cmsy5
\font\tenex=cmex10
\skewchar\teni='177 \skewchar\seveni='177 \skewchar\fivei='177
\skewchar\tensy='60 \skewchar\sevensy='60 \skewchar\fivesy='60
\textfont0=\tenrm \scriptfont0=\sevenrm \scriptscriptfont0=\fiverm
\textfont1=\teni \scriptfont1=\seveni \scriptscriptfont1=\fivei
\textfont2=\tensy \scriptfont2=\sevensy \scriptscriptfont2=\fivesy
\textfont3=\tenex \scriptfont3=\tenex \scriptscriptfont3=\tenex
"""

cases = {}


def case(name, desc, body, no_halt=False, raw=None):
    head = "%% %s: %s\n" % (name, desc)
    head += "% XeTeX-specific (docs/design/xetex/PLAN.md, phase S0: TFM fonts)\n"
    if no_halt:
        head += "% lockstep: no-halt\n"
    text = head + "\\input prelude\n\\def\\space{ }\n" + body
    if not text.endswith("\n"):
        text += "\n"
    cases[name] = raw(text) if raw else text.encode("utf-8")


case("x001-utf8-text", "UTF-8 letters in a TFM font: present ones typeset, absent ones are lost characters (char-warning-xetex.ch)",
     r"""\font\x=cmr10 \x
\setbox0=\hbox{cafe naive é ü ✓ 雪 🍌 Ω}
\lsshipbox0
\end""")

case("x002-hat4", "^^^^ and ^^^^^^ notation for characters above 255",
     r"""\catcode`\^^^^00e9=11
\def\t^^^^00e9st{ok}
\message{x002 \meaning\t^^^^00e9st}
\count1=`^^^^00e9 \message{x002 \the\count1}
\count2=`^^^^^^01f34c \message{x002 \the\count2}
\count3=`^^41 \message{x002 \the\count3}
\setbox0=\hbox{\count1=`^^^^0041 \vrule width\count1 sp}
\lsshipbox0
\end""")

case("x003-uchar", "\\Uchar makes a character token of a Unicode scalar",
     r"""\font\x=cmr10 \x
\message{x003 [\Uchar"41][\Uchar"E9][\Uchar"1F34C]}
\edef\a{\Uchar"42\Uchar"3B1}
\message{x003 \meaning\a}
\setbox0=\hbox{\Uchar"41\Uchar"7A\Uchar"E9}
\lsshipbox0
\end""")

case("x004-ucharcat", "\\Ucharcat makes a character token of a given category",
     r"""\font\x=cmr10 \x
\edef\a{\Ucharcat"41 11 \Ucharcat"42 12 \Ucharcat"20 10 \Ucharcat"5B 1 \Ucharcat"5D 2 }
\message{x004 \meaning\a}
\edef\b{\Ucharcat`\$ 3 \Ucharcat`\& 4 \Ucharcat`\^ 7 \Ucharcat`\_ 8 }
\message{x004 \meaning\b}
\setbox0=\hbox{\Ucharcat"61 11 \Ucharcat"62 12}
\lsshipbox0
\end""")

case("x005-umathcode", "\\Umathcode, \\Umathcodenum and \\mathcode with Unicode math codes",
     MATH + r"""\Umathcode`a="7 "1 "61
\message{x005 \the\Umathcodenum`a}
\message{x005 \the\Umathcodenum`a}
\message{x005 \the\mathcode`a}
\Umathcodenum`b="E200062
\message{x005 \the\Umathcodenum`b}
\Umathcode`c="1 "FF "1F34C
\message{x005 \the\Umathcodenum`c \the\mathcode`c}
\setbox0=\hbox{$ab$}
\lsshipbox0
\end""", no_halt=True)

case("x006-umathchardef", "\\Umathchardef and \\Umathcharnumdef define math characters",
     MATH + r"""\Umathchardef\alphaX="0 "1 "0B
\Umathcharnumdef\betaX="20000C
\message{x006 \meaning\alphaX}
\message{x006 \meaning\betaX}
\message{x006 \the\alphaX\space\the\betaX}
\setbox0=\hbox{$\alphaX+\betaX$}
\lsshipbox0
\end""")

case("x007-udelcode", "\\Udelcode and \\Udelcodenum",
     MATH + r"""\Udelcode`(="3 "28
\message{x007 \the\Udelcodenum`(}
\Udelcodenum`["300005B
\message{x007 \the\Udelcodenum`[}
\message{x007 \the\delcode`(}
\delcode`(="028300
\message{x007 \the\Udelcodenum`(}
\setbox0=\hbox{$\left(x\right)$}
\lsshipbox0
\end""", no_halt=True)

case("x008-umathchar-math", "\\Umathchar and \\Umathaccent in math with TFM families",
     MATH + r"""\setbox0=\hbox{$\Umathchar"0"1"61 \Umathchar"2"2"0 \Umathchar"0"1"62$}
\setbox1=\hbox{$\Umathaccent"0"0"5E{x}$}
\lsshipbox0
\lsshipbox1
\end""")

case("x009-udelimiter-radical", "\\Udelimiter and \\Uradical with TFM families",
     MATH + r"""\setbox0=\hbox{$\left\Udelimiter"4"3"28 x \right\Udelimiter"5"3"29$}
\setbox1=\hbox{$\Uradical"3"70 {xy}$}
\lsshipbox0
\lsshipbox1
\end""")

case("x010-xetexmath-names", "\\XeTeXmathcode, \\XeTeXmathchardef and \\XeTeXdelcode, the older names",
     MATH + r"""\XeTeXmathcode`d="0 "1 "64
\message{x010 \the\XeTeXmathcodenum`d}
\XeTeXmathchardef\dd="0 "1 "64
\message{x010 \meaning\dd}
\XeTeXdelcode`)="3 "29
\message{x010 \the\XeTeXdelcodenum`)}
\setbox0=\hbox{$\dd \XeTeXmathchar"0 "1 "65$}
\lsshipbox0
\end""")

case("x011-charclass", "\\XeTeXcharclass and \\XeTeXinterchartoks between character classes",
     r"""\font\x=cmr10 \x
\XeTeXcharclass`a=3 \XeTeXcharclass`b=4
\message{x011 \the\XeTeXcharclass`a \the\XeTeXcharclass`b \the\XeTeXcharclass`c}
\XeTeXinterchartoks 3 4 {\kern1pt}
\XeTeXinterchartoks 0 3 {\kern2pt}
\XeTeXinterchartoks 4 4095 {\kern3pt}
\message{x011 [\the\XeTeXinterchartoks 3 4]}
\XeTeXinterchartokenstate=1
\setbox0=\hbox{cab ab ba}
\XeTeXinterchartokenstate=0
\setbox1=\hbox{cab ab ba}
\lsshipbox0
\lsshipbox1
\end""")

case("x012-charclass-boundary", "interchar tokens at boundaries, with groups and the ignored class 4096",
     r"""\font\x=cmr10 \x
\XeTeXcharclass`a=1 \XeTeXcharclass`b=2 \XeTeXcharclass`.=4096
\XeTeXinterchartoks 4095 1 {[}
\XeTeXinterchartoks 2 4095 {]}
\XeTeXinterchartoks 1 2 {-}
\XeTeXinterchartokenstate=1
\setbox0=\hbox{ab a.b {a}b ab}
\lsshipbox0
\end""")

case("x013-version", "\\XeTeXrevision and \\XeTeXversion",
     r"""\message{x013 [\XeTeXrevision] [\the\XeTeXversion] [\number\XeTeXversion]}
\edef\r{\XeTeXrevision}\message{x013 \meaning\r}
\setbox0=\hbox{\vrule width\XeTeXversion pt}
\lsshipbox0
\end""")

case("x014-fonttype", "\\XeTeXfonttype of TFM fonts and of \\nullfont",
     r"""\font\x=cmr10
\message{x014 \the\XeTeXfonttype\x\space\the\XeTeXfonttype\nullfont}
\setbox0=\hbox{\x A}
\lsshipbox0
\end""")

case("x015-native-queries-tfm", "the native-font queries applied to a TFM font are errors",
     r"""\font\x=cmr10 \x
\message{x015 \the\XeTeXcountglyphs\x}
\message{x015 \the\XeTeXcountfeatures\x}
\message{x015 \the\XeTeXcountvariations\x}
\message{x015 \the\XeTeXfirstfontchar\x\space\the\XeTeXlastfontchar\x}
\message{x015 \the\XeTeXcharglyph`a}
\message{x015 \the\XeTeXglyphindex "a" }
\setbox0=\hbox{A}
\lsshipbox0
\end""", no_halt=True)

case("x016-tracingfonts", "\\XeTeXtracingfonts reports a TFM font's lookup",
     r"""\XeTeXtracingfonts=1
\font\x=cmr10 \font\y=cmr7 scaled 1200 \font\z=cmr5 at 7pt
\x
\setbox0=\hbox{A\y B\z C}
\lsshipbox0
\end""")

case("x017-font-not-found", "a font that is neither TFM nor installed",
     r"""\font\x=flashtexnosuchfont
\message{x017 \fontname\x}
\setbox0=\hbox{\x A}
\lsshipbox0
\end""", no_halt=True)

case("x018-suppressfontnotfound", "\\suppressfontnotfounderror makes a missing font silent",
     r"""\suppressfontnotfounderror=1
\font\x=flashtexnosuchfont
\message{x018 \fontname\x}
\ifx\x\nullfont\message{x018 null}\fi
\setbox0=\hbox{\x A}
\lsshipbox0
\end""")

case("x019-bytes-encoding", "\\XeTeXinputencoding \"bytes\" reads the rest of the file byte by byte",
     "", raw=lambda t: t.encode("utf-8") + b"""\\font\\x=cmr10 \\x
\\XeTeXinputencoding "bytes"
\\setbox0=\\hbox{caf\xe9 \xfc ab}
\\message{x019 [caf\xe9]}
\\XeTeXinputencoding "utf8"
\\setbox1=\\hbox{caf\xc3\xa9}
\\lsshipbox0
\\lsshipbox1
\\end
""")

case("x020-bad-utf8", "invalid UTF-8 is read as U+FFFD with a warning",
     "", raw=lambda t: t.encode("utf-8") + b"""\\font\\x=cmr10 \\x
\\setbox0=\\hbox{a\xffb\xc3c\xe2\x82d}
\\message{x020 [x\xc0\xafy]}
\\setbox1=\\hbox{\xed\xa0\x80z}
\\lsshipbox0
\\lsshipbox1
\\end
""")

case("x021-crlf", "CR LF and lone CR line ends",
     "", raw=lambda t: t.encode("utf-8") + b"\\font\\x=cmr10 \\x\r\n\\setbox0=\\hbox{a\r\nb}\r\\message{x021 one}\r\\message{x021 two}  \r\n\\lsshipbox0\r\n\\end\r\n")

case("x022-utf16le", "a UTF-16LE file with a byte order mark (u_open_in sniffs it)",
     "", raw=lambda t: b"\xff\xfe" + (t + "\\font\\x=cmr10 \\x\n\\setbox0=\\hbox{UTF16 é}\n\\message{x022 [é🍌]}\n\\lsshipbox0\n\\end\n").encode("utf-16-le"))

case("x023-utf16be", "a UTF-16BE file with a byte order mark",
     "", raw=lambda t: b"\xfe\xff" + (t + "\\font\\x=cmr10 \\x\n\\setbox0=\\hbox{big endian ö}\n\\lsshipbox0\n\\end\n").encode("utf-16-be"))

case("x024-utf8-bom", "a UTF-8 byte order mark is skipped",
     "", raw=lambda t: b"\xef\xbb\xbf" + t.encode("utf-8") + "\\font\\x=cmr10 \\x\n\\setbox0=\\hbox{bom ä}\n\\lsshipbox0\n\\end\n".encode("utf-8"))

case("x025-strcmp", "\\strcmp compares UTF-16 strings",
     r"""\message{x025 \strcmp{abc}{abd} \strcmp{abd}{abc} \strcmp{é}{é} \strcmp{é}{e}}
\message{x025 \strcmp{🍌}{\Uchar"1F34C} \strcmp{}{a} \strcmp{a}{}}
\setbox0=\hbox{\vrule width\strcmp{b}{a}pt}
\lsshipbox0
\end""")

case("x026-mdfivesum", "\\mdfivesum of strings (as UTF-8) and of a file",
     r"""\message{x026 \mdfivesum{abc}}
\message{x026 \mdfivesum{café}}
\message{x026 \mdfivesum{🍌}}
\message{x026 \mdfivesum file {prelude.tex}}
\setbox0=\hbox{\vrule width1pt}
\lsshipbox0
\end""")

case("x027-filesize-dump", "\\filesize and \\filedump of a file",
     r"""\message{x027 \filesize{prelude.tex}}
\message{x027 \filedump offset 2 length 12{prelude.tex}}
\message{x027 [\filesize{flashtex-no-such-file.tex}]}
\setbox0=\hbox{\vrule width1pt}
\lsshipbox0
\end""")

case("x028-creationdate", "\\creationdate under SOURCE_DATE_EPOCH",
     r"""\message{x028 \creationdate}
\setbox0=\hbox{\vrule width1pt}
\lsshipbox0
\end""")

case("x029-random", "\\setrandomseed, \\uniformdeviate, \\normaldeviate, \\randomseed",
     r"""\setrandomseed 12345
\message{x029 \the\randomseed\space\uniformdeviate 1000 \uniformdeviate 1000000 \normaldeviate\space\normaldeviate}
\setrandomseed -7
\message{x029 \the\randomseed\space\uniformdeviate 100}
\setbox0=\hbox{\vrule width\uniformdeviate 10 pt}
\lsshipbox0
\end""")

case("x030-shellescape-primitive", "\\shellescape, \\primitive and \\ifprimitive",
     r"""\message{x030 \the\shellescape}
\let\relax=\undefined
\primitive\relax
\message{x030 \ifprimitive\par yes\else no\fi\space\ifprimitive\relax yes\else no\fi}
\setbox0=\hbox{\vrule width1pt}
\lsshipbox0
\end""")

case("x031-xetex-int-params", "XeTeX's integer parameters and \\XeTeXlinebreakskip",
     r"""\message{x031 \the\XeTeXdashbreakstate\space\the\XeTeXuseglyphmetrics\space\the\XeTeXinputnormalization}
\XeTeXdashbreakstate=1 \XeTeXuseglyphmetrics=1 \XeTeXlinebreakpenalty=50
\XeTeXlinebreakskip=0pt plus 1pt \XeTeXhyphenatablelength=40
\message{x031 \the\XeTeXdashbreakstate\space\the\XeTeXuseglyphmetrics\space\the\XeTeXlinebreakpenalty\space\the\XeTeXlinebreakskip\space\the\XeTeXhyphenatablelength}
\XeTeXlinebreaklocale "en"
\XeTeXprotrudechars=2 \XeTeXgenerateactualtext=1 \XeTeXinterwordspaceshaping=1
\message{x031 \the\XeTeXprotrudechars\space\the\XeTeXgenerateactualtext\space\the\XeTeXinterwordspaceshaping}
\setbox0=\hbox{\vrule width1pt}
\lsshipbox0
\end""")

case("x032-upwards", "\\XeTeXupwardsmode builds vertical lists upwards",
     r"""\font\x=cmr10 \x \baselineskip=12pt \lineskip=1pt \lineskiplimit=0pt
\XeTeXupwardsmode=1
\setbox0=\vbox{\hbox{one}\hbox{two}\kern3pt\hbox{three}\vskip 4pt plus 1fil\hbox{}}
\XeTeXupwardsmode=0
\setbox1=\vbox{\hbox{one}\hbox{two}}
\message{x032 \the\ht0 \the\dp0 \the\ht1 \the\dp1}
\lsshipbox0
\lsshipbox1
\end""")

case("x033-csname-unicode", "Unicode control sequences, \\string and \\meaning",
     r"""\catcode`雪=11 \catcode`ü=11
\def\雪{snow}\def\ü{ue}
\expandafter\def\csname 🍌\endcsname{banana}
\message{x033 \string\雪\space\meaning\雪\space\string\ü}
\message{x033 \expandafter\meaning\csname 🍌\endcsname}
\message{x033 \expandafter\string\csname 🍌x\endcsname}
\message{x033 \the\catcode`雪\space\the\catcode`🍌}
\setbox0=\hbox{\vrule width1pt}
\lsshipbox0
\end""")

case("x034-case-codes", "\\lccode, \\uccode and \\sfcode of characters above 255",
     r"""\font\x=cmr10 \x
\message{x034 \the\lccode`É\space\the\uccode`é\space\the\sfcode`É\space\the\lccode`A}
\lccode`É=`é \uccode`é=`É \lccode`Ω=`ω
\lowercase{\message{x034 [ÉΩA]}}
\uppercase{\message{x034 [éωa]}}
\sfcode`é=999
\setbox0=\hbox{\lowercase{É} é. a}
\lsshipbox0
\end""")

case("x035-char-big", "\\char above 255 in a TFM font is a lost character",
     r"""\font\x=cmr10 \x \tracinglostchars=2
\setbox0=\hbox{\char"41 \char"FF \char"100 \char"1F34C \char"10FFFF}
\lsshipbox0
\end""")

case("x036-lostchars-error", "\\tracinglostchars=3 makes a lost character an error (char-warning-xetex.ch)",
     r"""\font\x=cmr10 \x \tracinglostchars=3
\setbox0=\hbox{a€b}
\lsshipbox0
\end""", no_halt=True)

case("x037-picfile-missing", "\\XeTeXpicfile of a file that does not exist",
     r"""\setbox0=\hbox{\XeTeXpicfile "flashtex-no-such-picture.png" scaled 500 \vrule width1pt}
\setbox1=\hbox{\XeTeXpdffile "flashtex-no-such.pdf" page 2 crop}
\lsshipbox0
\lsshipbox1
\end""", no_halt=True)

case("x038-glyph-tfm", "\\XeTeXglyph in a TFM font is an error",
     r"""\font\x=cmr10 \x
\setbox0=\hbox{a\XeTeXglyph 5 b}
\lsshipbox0
\end""", no_halt=True)

case("x039-write-utf8", "\\message, \\write and \\immediate\\write with Unicode text",
     r"""\immediate\write-1{x039 [ü 雪 🍌 \string\雪]}
\immediate\write16{x039 [Ω≠ω]}
\newlinechar=`¶
\message{x039 a¶b¶c}
\setbox0=\hbox{\vrule width1pt}
\lsshipbox0
\end""")

case("x040-active-unicode", "an active Unicode character, and a Unicode math character set to \"8000",
     MATH + r"""\catcode`→=13 \def→{\to}
\def\to{TO}
\message{x040 [→]}
\mathcode`←="8000
\catcode`\~=13
\begingroup\lccode`~=`← \lowercase{\endgroup\def~{\mathchar"2190}}
\message{x040 \the\mathcode`←}
\setbox0=\hbox{$a$}
\lsshipbox0
\end""")

case("x041-hyphenation-unicode", "\\hyphenation and patterns with characters above 255",
     r"""\lccode`é=`é \lccode`ü=`ü
\patterns{1b1 é1t}
\hyphenation{über-wür-de}
\font\x=cmr10 \x \hyphenchar\x=`- \hsize=1pt \parindent=0pt \pretolerance=-1 \hyphenpenalty=0
\setbox0=\vbox{\noindent abab cbétb überwürde}
\lsshipbox0
\end""")

case("x042-interchar-math", "interchar tokens do not apply in math, and the state is an integer",
     MATH + r"""\XeTeXcharclass`x=7 \XeTeXinterchartoks 7 7 {\kern1pt}
\XeTeXinterchartokenstate=1
\setbox0=\hbox{\tenrm xx $xx$ xx}
\message{x042 \the\XeTeXinterchartokenstate}
\lsshipbox0
\end""")

case("x043-uchar-errors", "\\Uchar and \\Ucharcat out of range",
     r"""\message{x043 [\Uchar"110000]}
\message{x043 [\Ucharcat"41 5]}
\message{x043 [\Ucharcat"41 16]}
\setbox0=\hbox{\vrule width1pt}
\lsshipbox0
\end""", no_halt=True)

case("x044-umathcode-errors", "\\Umathcode with an invalid class, family or character",
     MATH + r"""\Umathcode`a="8 "1 "61
\Umathcode`b="7 "100 "62
\Umathcode`c="7 "1 "110000
\message{x044 \the\Umathcode`a \the\Umathcode`b \the\Umathcode`c}
\setbox0=\hbox{\vrule width1pt}
\lsshipbox0
\end""", no_halt=True)

case("x045-defaultencoding", "\\XeTeXdefaultencoding and \\XeTeXinputencoding with built-in names",
     r"""\XeTeXdefaultencoding "utf16le"
\XeTeXdefaultencoding "auto"
\XeTeXinputencoding "UTF8"
\XeTeXinputencoding "auto"
\setbox0=\hbox{\vrule width1pt}
\lsshipbox0
\end""", no_halt=True)

case("x046-xetex-tracing-showbox", "a box of Unicode characters shown with \\showbox",
     r"""\font\x=cmr10 \x
\setbox0=\hbox{Aé\char"7F\kern1pt ü}
\showbox0
\lsshipbox0
\end""", no_halt=True)

case("x047-lastnodetype", "\\lastnodetype after characters, glue and math",
     MATH + r"""\setbox0=\hbox{\tenrm a\message{x047 \the\lastnodetype}\hskip1pt\message{x047 \the\lastnodetype}$x$\message{x047 \the\lastnodetype}}
\lsshipbox0
\end""")

case("x048-dash-break", "\\XeTeXdashbreakstate with a TFM font (it serves native fonts)",
     r"""\font\x=cmr10 \x \hsize=20pt \parindent=0pt \XeTeXdashbreakstate=1
\setbox0=\vbox{aaa--bbb---ccc ddd}
\lsshipbox0
\end""")

case("x049-chardef-big", "\\chardef and \\mathchardef of large values",
     MATH + r"""\chardef\big="1F34C
\message{x049 \meaning\big\space\the\big}
\chardef\small="41
\mathchardef\mc="7161
\message{x049 \meaning\mc}
\setbox0=\hbox{\tenrm\small$\mc$}
\lsshipbox0
\end""")

case("x051-the-umathcode-error", "\\the\\Umathcode is an error: use \\Umathcodenum",
     MATH + r"""\Umathcode`a="7 "1 "61
\message{x051 \the\Umathcode`a}
\message{x051 \the\Udelcode`(}
\setbox0=\hbox{\vrule width1pt}
\lsshipbox0
\end""", no_halt=True)

case("x052-noexpand-endwrite", "tex.ch [25.369]: \\noexpand before the \\endwrite of a \\write does not take it",
     r"""\immediate\write16{x052 a\noexpand}
\immediate\write-1{x052 b\noexpand}
\setbox0=\hbox{\vrule width1pt}
\lsshipbox0
\end""")

case("x053-mkern-nonmu", "tex.ch [26.449]: \\mkern and \\mskip with a non-mu internal dimension or glue",
     MATH + r"""\dimen0=3pt \skip0=4pt plus 1pt
\setbox0=\hbox{$x\mkern\dimen0 x\mskip\skip0 x\mkern\count0 x$}
\lsshipbox0
\end""", no_halt=True)

case("x050-catcode-default", "the INITEX category codes of Unicode letters and others",
     r"""\message{x050 \the\catcode`a \the\catcode`é \the\catcode`Ω \the\catcode`雪 \the\catcode`🍌 \the\catcode`1 \the\catcode`. \the\catcode`€}
\setbox0=\hbox{\vrule width1pt}
\lsshipbox0
\end""")


# ---------------------------------------------------------------------------
# Phase S1: native fonts by file name (OpenType and TrueType files that
# kpathsea finds in TeX Live, or by absolute path). The fonts are TeX Live's
# Latin Modern and TeX Gyre and two of macOS's own; a machine without one
# of them fails the same way in both engines.
# ---------------------------------------------------------------------------


def ncase(name, desc, body, no_halt=False):
    head = "%% %s: %s\n" % (name, desc)
    head += "% XeTeX-specific (docs/design/xetex/PLAN.md, phase S1: native fonts)\n"
    if no_halt:
        head += "% lockstep: no-halt\n"
    text = head + "\\input prelude\n\\def\\space{ }\n" + body
    if not text.endswith("\n"):
        text += "\n"
    cases[name] = text.encode("utf-8")


PARA = (r"""\hsize=210pt \parindent=12pt \parfillskip=0pt plus 1fil
\baselineskip=13pt \tolerance=10000 \pretolerance=-1
""")
WORDS = ("We shall find that office workers, efficient and affluent, "
         "flatly refuse difficult fjords; AVATAR, Wo, Ta, T. Yo! "
         "Naïve café owners in Zürich serve crème brûlée -- or not --- "
         "with 1234567890 “quotes” and ‘single’ ones.")

ncase("n001-lm-text", "Latin Modern Roman OTF by file name: ligatures, kerns, accents",
      r"""\font\x="[lmroman10-regular.otf]" \x
\setbox0=\hbox{office difficult flag AVAT Ta. Wo, ``quotes'' -- --- é ñ ß Ŵ}
\lsshipbox0
\end""")

ncase("n002-lm-paragraph", "a paragraph in Latin Modern Roman broken into lines",
      PARA + r"""\font\x="[lmroman10-regular.otf]" \x
\setbox0=\vbox{""" + WORDS + " " + WORDS + r"""\par}
\lsshipbox0
\end""")

ncase("n003-lm-sizes", "sizes: at, scaled, and the design size of the size feature",
      r"""\font\a="[lmroman10-regular.otf]" at 5pt
\font\b="[lmroman10-regular.otf]" scaled 1200
\font\c="[lmroman12-regular.otf]"
\font\d="[lmroman17-regular.otf]" scaled 500
\font\e="[lmroman10-regular.otf]" at 17.28pt
\message{[\fontname\a][\fontname\b][\fontname\c][\fontname\d][\fontname\e]}
\message{[\the\fontdimen6\c][\the\fontdimen6\d]}
\setbox0=\hbox{\a office \b office \c office \d office \e office}
\lsshipbox0
\end""")

ncase("n004-gyre-features", "OpenType features of TeX Gyre Termes: smcp, onum, -liga, -kern, script and language, a bad option",
      r"""\font\a="[texgyretermes-regular.otf]:+smcp;+onum"
\font\b="[texgyretermes-regular.otf]:-liga;-kern"
\font\c="[texgyretermes-regular.otf]:script=latn;language=DEU;+liga"
\font\d="[texgyretermes-regular.otf]:+frac;+sups"
\font\e="[texgyretermes-regular.otf]:nosuchoption;+zzzz"
\setbox0=\hbox{\a Office 1234 \b office AVA \c office \d 1/2 3/4 \e office}
\lsshipbox0
\end""")

ncase("n005-common-options", "letterspace, color, extend, slant and embolden options",
      r"""\font\a="[lmroman10-regular.otf]:letterspace=10"
\font\b="[lmroman10-regular.otf]:color=FF0000"
\font\c="[lmroman10-regular.otf]:color=00FF0080"
\font\d="[lmroman10-regular.otf]:extend=1.2"
\font\e="[lmroman10-regular.otf]:slant=0.2"
\font\f="[lmroman10-regular.otf]:embolden=2"
\font\g="[lmroman10-regular.otf]:extend=0.85;slant=-0.167"
\font\h="[lmroman10-regular.otf]:color=12"
\message{[\the\fontdimen1\d][\the\fontdimen1\e][\the\fontdimen1\g][\the\fontdimen2\a]}
\setbox0=\hbox{\a office \b office \c office \d office \e office \f office \g office \h office}
\lsshipbox0
\end""")

ncase("n006-glyph-queries", "glyph and font queries of a native font",
      r"""\font\x="[lmroman10-regular.otf]"
\message{[\the\XeTeXcountglyphs\x][\the\XeTeXfirstfontchar\x][\the\XeTeXlastfontchar\x][\the\XeTeXfonttype\x]}
\x
\message{[\the\XeTeXcharglyph`A]}
\count1=\XeTeXglyphindex "f_f_i" \relax
\message{[\the\count1]}
\message{[\XeTeXglyphname\x 12][\XeTeXglyphname\x 300]}
\message{[\the\fontcharwd\x`A][\the\fontcharht\x`A][\the\fontchardp\x`g][\the\fontcharic\x`f][\the\fontcharic\x`A]}
\message{[\the\XeTeXglyphbounds1 36][\the\XeTeXglyphbounds2 36][\the\XeTeXglyphbounds3 36][\the\XeTeXglyphbounds4 36]}
\message{[\the\fontdimen1\x][\the\fontdimen2\x][\the\fontdimen3\x][\the\fontdimen4\x][\the\fontdimen5\x][\the\fontdimen6\x][\the\fontdimen7\x][\the\fontdimen8\x]}
\setbox0=\hbox{\XeTeXglyph36 \XeTeXglyph 12 x}
\lsshipbox0
\end""")

ncase("n007-ot-layout-queries", "\\XeTeXOT... queries: scripts, languages, features",
      r"""\font\x="[texgyretermes-regular.otf]"
\count1=\XeTeXOTcountscripts\x
\message{[\the\count1]}
\def\one#1{\count3=\XeTeXOTscripttag\x#1
  \message{[script \the\count3: \the\XeTeXOTcountlanguages\x\count3 languages, \the\XeTeXOTcountfeatures\x\count3 0 features]}
  \message{[\the\XeTeXOTfeaturetag\x\count3 0 0][\the\XeTeXOTfeaturetag\x\count3 0 5][\the\XeTeXOTlanguagetag\x\count3 0]}
  \message{[\the\XeTeXOTcountfeatures\x\count3 "44455520 ][\the\XeTeXOTfeaturetag\x\count3 "44455520 1]}}
\one0 \one1 \one2 \one7
\setbox0=\hbox{\x x}
\lsshipbox0
\end""")

ncase("n008-glyph-metrics", "\\XeTeXuseglyphmetrics: heights and depths from the glyphs",
      r"""\font\x="[lmroman10-regular.otf]" \x
\setbox0=\hbox{\XeTeXuseglyphmetrics=1 ace gpy AT \XeTeXglyph36 \XeTeXuseglyphmetrics=0 ace gpy}
\lsshipbox0
\end""")

ncase("n009-gyre-families", "TeX Gyre Heros, Pagella italic, Cursor and Latin Modern Mono in one paragraph",
      PARA + r"""\font\a="[texgyreheros-regular.otf]" \font\b="[texgyrepagella-italic.otf]"
\font\c="[texgyrecursor-regular.otf]" \font\d="[lmmono10-regular.otf]"
\font\e="[texgyreheros-bold.otf]" \font\f="[texgyrebonum-regular.otf]"
\setbox0=\vbox{\a """ + WORDS + r""" \b """ + WORDS + r""" \c office \d office \e """ + WORDS + r""" \f """ + WORDS + r"""\par}
\lsshipbox0
\end""")

ncase("n010-system-ttf", "a macOS TrueType font by absolute path (Times New Roman, GSUB/GPOS)",
      PARA + r"""\font\x="[/System/Library/Fonts/Supplemental/Times New Roman.ttf]" \x
\setbox0=\vbox{""" + WORDS + r"""\par}
\lsshipbox0
\end""")

ncase("n011-system-ttc", "macOS Helvetica.ttc faces by absolute path and index (shaped with HarfBuzz in the file form)",
      PARA + r"""\font\a="[/System/Library/Fonts/Helvetica.ttc]" \font\b="[/System/Library/Fonts/Helvetica.ttc:1]"
\font\c="[/System/Library/Fonts/Times.ttc:0]"
\message{[\fontname\a][\fontname\b][\the\fontdimen1\b]}
\setbox0=\vbox{\a """ + WORDS + r""" \b office AVAT \c """ + WORDS + r"""\par}
\lsshipbox0
\end""")

ncase("n012-same-font-twice", "a native font loaded twice is one font; \\fontname and \\the\\font",
      r"""\font\a="[lmroman10-regular.otf]" \font\b="[lmroman10-regular.otf]"
\font\c="[lmroman10-regular.otf]" at 10.0001pt
\message{[\fontname\a][\fontname\b][\fontname\c][\meaning\a][\meaning\b]}
\setbox0=\hbox{\a x\b x\c x}
\lsshipbox0
\end""")

ncase("n013-tracing-fonts", "\\XeTeXtracingfonts reports the file a font came from",
      r"""\XeTeXtracingfonts=1
\font\a="[lmroman10-regular.otf]" \font\b="[texgyretermes-bold.otf]"
\setbox0=\hbox{\a x\b x}
\lsshipbox0
\end""")

ncase("n014-missing-chars", "characters a native font lacks are lost (\\tracinglostchars)",
      r"""\font\x="[lmroman10-regular.otf]" \x \tracinglostchars=2
\setbox0=\hbox{a雪b🍌c\char"E000 d}
\lsshipbox0
\end""")

ncase("n015-hyphenation", "native words hyphenated from \\hyphenation exceptions",
      r"""\hsize=60pt \parindent=0pt \parfillskip=0pt plus 1fil \tolerance=10000 \pretolerance=-1
\hyphenpenalty=0 \lefthyphenmin=2 \righthyphenmin=2 \baselineskip=12pt
\hyphenation{dif-fi-cult ef-fi-cient af-flu-ent of-fice work-ers}
\font\x="[lmroman10-regular.otf]" \x \hyphenchar\x=`-
\setbox0=\vbox{difficult efficient affluent office workers difficult efficient affluent office workers\par}
\lsshipbox0
\end""")

ncase("n016-interword-shaping", "\\XeTeXinterwordspaceshaping 0, 1 and 2",
      PARA + r"""\font\x="[texgyretermes-regular.otf]" \x
\setbox0=\vbox{\XeTeXinterwordspaceshaping=0 AV AT Wo. office\par
\XeTeXinterwordspaceshaping=1 AV AT Wo. office\par
\XeTeXinterwordspaceshaping=2 AV AT Wo. office """ + WORDS + r"""\par}
\lsshipbox0
\end""")

ncase("n017-justified-boxes", "native words in boxes set to a width (stretch and shrink)",
      r"""\font\x="[lmroman10-regular.otf]" \x
\setbox1=\hbox to 200pt{office and difficult words}
\setbox2=\hbox to 80pt{office and difficult words}
\setbox3=\hbox spread 20pt{AVA Ta We}
\setbox0=\vbox{\box1 \box2 \box3}
\lsshipbox0
\end""")

ncase("n018-pagella-math-chars", "punctuation, digits, symbols and combining marks in TeX Gyre Pagella",
      r"""\font\x="[texgyrepagella-regular.otf]" \x
\setbox0=\hbox{0123456789 +−×÷=≠ ¶§†‡ ©®™ ½ e\char"0301 o\char"0308 fi ffl ſt}
\lsshipbox0
\end""")

ncase("n019-xdv-many-fonts", "many native fonts and sizes in one XDV file, several pages",
      r"""\font\a="[lmroman10-regular.otf]" \font\b="[lmroman10-bold.otf]" at 12pt
\font\c="[lmroman10-italic.otf]" scaled 900 \font\d="[lmsans10-regular.otf]"
\font\e="[texgyreadventor-regular.otf]" \font\f="[texgyrechorus-mediumitalic.otf]"
\setbox0=\hbox{\a Roman \b Bold \c Italic \d Sans}
\lsshipbox0
\setbox0=\hbox{\e Adventor \f Chorus \a again}
\lsshipbox0
\end""")

ncase("n020-unknown-font", "a bracketed font file that does not exist, and one that is not a font",
      r"""\font\a="[nosuchfont.otf]"
\setbox0=\hbox{x}
\lsshipbox0
\end""", no_halt=True)

for name, data in cases.items():
    with open(os.path.join(OUT, name + ".tex"), "wb") as fh:
        fh.write(data)
print(len(cases), "cases")
