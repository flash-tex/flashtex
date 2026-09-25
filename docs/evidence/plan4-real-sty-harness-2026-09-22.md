# plan4 real-.sty harness — 2026-09-22 (UTC)

Minimal document `\documentclass{article}\usepackage{<pkg>}` with a package reader serving ONLY that package's real TeX Live `.sty` file; nested `\RequirePackage`s are declined (pass through) and listed below. Signal for "unmodelled": the engine emits an unknown command with no diagnostic, so run 1 collects control sequences the package file's execution emits and run 2 classifies each with an `\ifcsname` probe tail (controls `\par`, `\relax`, `\usepackage`, `\@empty` must stay unflagged and a `HARNESS-PROBE-DONE` sentinel must be present, else the row is `inconclusive`). Times are one debug-build run each (`std::time::Instant` around `Engine::run`). Rows with errors may mix root gaps with cascade fallout once argument parsing derails (see per-package diagnostics). Method pinned by `crates/tex-expansion/tests/real_sty_probe_tests.rs`.

| Package | Completed | Unmodelled (n) | Unmodelled names | Time (ms) | Nested deps requested (declined) |
| --- | --- | --- | --- | --- | --- |
| `appendix` | Yes | 0 | — | 2.31 | — |
| `calc` | Yes | 0 | — | 2.86 | — |
| `enumerate` | Yes | 0 | — | 1.27 | — |
| `relsize` | Yes | 0 | — | 2.01 | — |
| `ifthen` | No | 1 | `\active` | 1.67 | — |
| `titling` | No | 1 | `\if@titlepage` | 2.15 | — |
| `xspace` | Yes | 1 | `\text@command` | 1.39 | — |
| `setspace` | Yes | 6 | `\abovedisplayshortskip`, `\abovedisplayskip`, `\baselinestretch`, `\belowdisplayshortskip`, `\belowdisplayskip`, `\everydisplay` | 3.04 | — |
| `parskip` | No | 26 | `\@startsection`, `\@starttoc`, `\@tempskipa`, `\@xsect`, `\DeclareCurrentRelease`, `\DeclareRelease`, `\DeclareStringOption`, `\ProcessKeyvalOptions`, `\SetupKeyvalOptions`, `\addvspace`, `\itemsep`, `\lastskip`, `\leftmargin`, `\leftmargini`, `\parfillskip`, `\parindent`, `\parsep`, `\parskip`, `\parskip@indent`, `\parskip@parfill`, `\parskip@skip`, `\parskip@tocskip`, `\partopsep`, `\patchcmd`, `\topsep`, `\vskip` | 2.30 | etoolbox.sty, kvoptions.sty |
| `etoolbox` | No | 120 | `\#`, `\-`, `\<`, `\=`, `\>`, `\@argdef`, `\@ifdefinable`, `\@makeother`, `\@nocounterr`, `\@star@or@long`, `\AddToHook`, `\AfterEndDocument`, `\AfterEndPreamble`, `\AtEndPreamble`, `\DeclareListParser`, `\appto`, `\apptocmd`, `\boolfalse`, `\booltrue`, `\csappto`, `\csdef`, `\csdimdef`, `\csdimgdef`, `\cseappto`, `\csedef`, `\csepreto`, `\csgappto`, `\csgdef`, `\csgluedef`, `\csgluegdef`, `\csgpreto`, `\csgundef`, `\cslet`, `\csletcs`, `\csmudef`, `\csmugdef`, `\csnumdef`, `\csnumgdef`, `\cspreto`, `\csshow`, `\csundef`, `\csxappto`, `\csxdef`, `\csxpreto`, `\defcounter`, `\deflength`, `\dimdef`, `\dimgdef`, `\docsvlist`, `\eappto`, `\epreto`, `\etb@listremove`, `\etb@undefined`, `\forcsvlist`, `\gappto`, `\gluedef`, `\glueexpr`, `\gluegdef`, `\gpreto`, `\gundef`, `\ifboolexpr`, `\ifcsltxprotect`, `\ifcsstring`, `\ifdefltxprotect`, `\ifdefstrequal`, `\ifdefstring`, `\ifinlist`, `\ifinlistcs`, `\ifpatchable`, `\ifstrequal`, `\letcs`, `\listadd`, `\listcsadd`, `\listcseadd`, `\listcsgadd`, `\listcsgremove`, `\listcsremove`, `\listcsxadd`, `\listeadd`, `\listgadd`, `\listgremove`, `\listremove`, `\listxadd`, `\mudef`, `\mugdef`, `\newbool`, `\newtoggle`, `\numdef`, `\numgdef`, `\patchcmd`, `\preto`, `\pretocmd`, `\protected@cseappto`, `\protected@csedef`, `\protected@csepreto`, `\protected@csxappto`, `\protected@csxdef`, `\protected@csxpreto`, `\protected@eappto`, `\protected@epreto`, `\protected@xappto`, `\protected@xpreto`, `\providebool`, `\providerobustcmd`, `\providetoggle`, `\renewrobustcmd`, `\robustify`, `\setbool`, `\settoggle`, `\show`, `\togglefalse`, `\toggletrue`, `\undef`, `\unlessboolexpr`, `\whileboolexpr`, `\xappto`, `\xifinlist`, `\xifinlistcs`, `\xpreto`, `\z@skip` | 16.10 | etex.sty |

## Diagnostics per package (run 1)

### appendix — 0 error(s), 2 warning(s)
- [Warning] Package appendix Warning: No \protect \appendix command in this document class! Trying to create an appendix will probably fail.
- [Warning] Package appendix Warning: There is no \protect \chapter or \protect \section command. The appendix package will not be used.

### calc — 0 error(s), 0 warning(s)
- none

### enumerate — 0 error(s), 0 warning(s)
- none

### relsize — 0 error(s), 1 warning(s)
- [Warning] Package relsize Warning: Failed to get list of defined font sizes. Size scaling will attempt arbitrary sizes.

### ifthen — 2 error(s), 0 warning(s)
- [Error] Missing number, treated as zero.
- [Error] Missing number, treated as zero.

### titling — 6 error(s), 0 warning(s)
- [Error] Extra \else.
- [Error] Extra \fi.
- [Error] Extra \else.
- [Error] Extra \fi.
- [Error] Extra \else.
- [Error] Extra \fi.

### xspace — 0 error(s), 0 warning(s)
- none

### setspace — 0 error(s), 0 warning(s)
- none

### parskip — 12 error(s), 0 warning(s)
- [Error] You can't use `\parfillskip' after \advance.
- [Error] Missing number, treated as zero.
- [Error] Illegal unit of measure (pt inserted).
- [Error] Missing = inserted for \ifdim.
- [Error] Missing number, treated as zero.
- [Error] Missing number, treated as zero.
- [Error] Missing number, treated as zero.
- [Error] Missing number, treated as zero.
- [Error] Missing number, treated as zero.
- [Error] Missing number, treated as zero.
- [Error] You can't use `\@tempskipa' after \advance.
- [Error] You can't use `\@tempskipa' after \advance.

### etoolbox — 71 error(s), 0 warning(s)
- [Error] Argument of \@secondoftwo has an extra }.
- [Error] Runaway argument? / ! Paragraph ended before \@secondoftwo was complete.
- [Error] Argument of \@secondoftwo has an extra }.
- [Error] Runaway argument? / ! Paragraph ended before \@secondoftwo was complete.
- [Error] Argument of \@secondoftwo has an extra }.
- [Error] Runaway argument? / ! Paragraph ended before \@secondoftwo was complete.
- [Error] Argument of \@secondoftwo has an extra }.
- [Error] Runaway argument? / ! Paragraph ended before \@secondoftwo was complete.
- [Error] Argument of \@secondoftwo has an extra }.
- [Error] Runaway argument? / ! Paragraph ended before \@secondoftwo was complete.
- [Error] Argument of \@secondoftwo has an extra }.
- [Error] Runaway argument? / ! Paragraph ended before \@secondoftwo was complete.
- [Error] Argument of \@secondoftwo has an extra }.
- [Error] Runaway argument? / ! Paragraph ended before \@secondoftwo was complete.
- [Error] Argument of \@secondoftwo has an extra }.
- [Error] Runaway argument? / ! Paragraph ended before \@secondoftwo was complete.
- [Error] Argument of \@secondoftwo has an extra }.
- [Error] Runaway argument? / ! Paragraph ended before \@secondoftwo was complete.
- [Error] Package etoolbox Error: Invalid boolean expression.
- [Error] Package etoolbox Error: Invalid boolean expression.
- [Error] Missing { inserted.
- [Error] Missing { inserted.
- [Error] Missing { inserted.
- [Error] Missing { inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing number, treated as zero.
- [Error] Illegal unit of measure (pt inserted).
- [Error] Missing number, treated as zero.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing number, treated as zero.
- [Error] Missing number, treated as zero.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing number, treated as zero.
- [Error] Missing number, treated as zero.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing number, treated as zero.
- [Error] Missing number, treated as zero.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Missing number, treated as zero.
- [Error] Missing control sequence inserted.
- [Error] Argument of \@secondoftwo has an extra }.
- [Error] Runaway argument? / ! Paragraph ended before \@secondoftwo was complete.
- [Error] Argument of \@secondoftwo has an extra }.
- [Error] Runaway argument? / ! Paragraph ended before \@secondoftwo was complete.
- [Error] Package etoolbox Error: #1 not a macro.
- [Error] Missing control sequence inserted.
- [Error] Missing control sequence inserted.
- [Error] Parameters must be numbered consecutively.
- [Error] Parameters must be numbered consecutively.
- [Error] Argument of \@firstoftwo has an extra }.
- [Error] Runaway argument? / ! Paragraph ended before \@firstoftwo was complete.

## Environment

- kpsewhich: `/Library/TeX/texbin/kpsewhich`
- measured: 2026-09-22T02:19:58Z UTC
- appendix: `/usr/local/texlive/2026/texmf-dist/tex/latex/appendix/appendix.sty`
- calc: `/usr/local/texlive/2026/texmf-dist/tex/latex/tools/calc.sty`
- enumerate: `/usr/local/texlive/2026/texmf-dist/tex/latex/tools/enumerate.sty`
- relsize: `/usr/local/texlive/2026/texmf-dist/tex/latex/relsize/relsize.sty`
- ifthen: `/usr/local/texlive/2026/texmf-dist/tex/latex/base/ifthen.sty`
- titling: `/usr/local/texlive/2026/texmf-dist/tex/latex/titling/titling.sty`
- xspace: `/usr/local/texlive/2026/texmf-dist/tex/latex/tools/xspace.sty`
- setspace: `/usr/local/texlive/2026/texmf-dist/tex/latex/setspace/setspace.sty`
- parskip: `/usr/local/texlive/2026/texmf-dist/tex/latex/parskip/parskip.sty`
- etoolbox: `/usr/local/texlive/2026/texmf-dist/tex/latex/etoolbox/etoolbox.sty`
