//! The LaTeX-kernel prelude: macros that real LaTeX (and plain TeX)
//! define *in TeX*, reproduced here verbatim or near-verbatim from
//! `latex.ltx`/`plain.tex` so that their expansion behaviour (including
//! `\meaning`, `\ifx` comparisons and `\protect` interaction) matches the
//! real thing rather than a Rust approximation. Executed once when the
//! first engine is created (see `expand.rs::build_initial_state`); every
//! engine starts from the resulting state.
//!
//! Only expansion-relevant kernel macros are here. Typesetting commands
//! (`\section`, `\textbf`, ...) are deliberately absent: they pass
//! through to the typesetter untouched (see CONTRACT.md).

pub const PRELUDE: &str = r"\catcode`\@=11
\def\makeatletter{\catcode`\@=11 }
\def\makeatother{\catcode`\@=12 }
\def\@empty{}
\def\space{ }
\chardef\@ne=1
\chardef\tw@=2
\chardef\thr@@=3
\chardef\sixt@@n=16
\chardef\@cclv=255
\mathchardef\@cclvi=256
\mathchardef\@m=1000
\mathchardef\@M=10000
\mathchardef\@MM=20000
\newcount\m@ne \m@ne=-1
\newcount\count@
\newdimen\dimen@
\newdimen\z@ \z@=0pt
\newskip\skip@
\long\def\@gobblefour#1#2#3#4{}
\long\def\@car#1#2\@nil{#1}
\long\def\@cdr#1#2\@nil{#2}
\long\def\@onlypreamble#1{}
\long\def\@gobble#1{}
\long\def\@gobbletwo#1#2{}
\long\def\@firstofone#1{#1}
\long\def\@firstoftwo#1#2{#1}
\long\def\@secondoftwo#1#2{#2}
\def\@nnil{\@nil}
\let\@@par\par
\let\endgraf\par
\let\bgroup={
\let\egroup=}
\long\def\@namedef#1{\expandafter\def\csname #1\endcsname}
\def\@nameuse#1{\csname #1\endcsname}
\def\@ifundefined#1{%
  \ifcsname#1\endcsname
    \expandafter\ifx\csname#1\endcsname\relax
      \expandafter\expandafter\expandafter\@firstoftwo
    \else
      \expandafter\expandafter\expandafter\@secondoftwo
    \fi
  \else
    \expandafter\@firstoftwo
  \fi}
\long\def\@ifnextchar#1#2#3{%
  \let\reserved@d=#1%
  \def\reserved@a{#2}%
  \def\reserved@b{#3}%
  \futurelet\@let@token\@ifnch}
\def\@ifnch{%
  \ifx\@let@token\@sptoken
    \let\reserved@c\@xifnch
  \else
    \ifx\@let@token\reserved@d
      \let\reserved@c\reserved@a
    \else
      \let\reserved@c\reserved@b
    \fi
  \fi
  \reserved@c}
\def\:{\let\@sptoken= } \: %
\def\:{\@xifnch} \expandafter\def\: {\futurelet\@let@token\@ifnch}
\let\kernel@ifnextchar\@ifnextchar
\long\def\@testopt#1#2{\kernel@ifnextchar[{#1}{#1[{#2}]}}
\def\@protected@testopt#1{\ifx\protect\@typeset@protect\expandafter\@testopt\else\@x@protect#1\fi}
\long\def\@x@protect#1\fi#2#3{\fi\protect#1}
\def\@ifstar#1{\@ifnextchar *{\@firstoftwo{#1}}}
\long\def\loop#1\repeat{%
  \def\iterate{#1\relax\expandafter\iterate\fi}%
  \iterate
  \let\iterate\relax}
\let\repeat\fi
\toksdef\toks@=0
\long\def\g@addto@macro#1#2{%
  \begingroup
    \toks@\expandafter{#1#2}%
    \xdef#1{\the\toks@}%
  \endgroup}
\def\@begindocumenthook{}
\def\@enddocumenthook{}
\long\def\AtBeginDocument#1{\g@addto@macro\@begindocumenthook{#1}}
\long\def\AtEndDocument#1{\g@addto@macro\@enddocumenthook{#1}}
\let\@typeset@protect\relax
\let\protect\@typeset@protect
\def\@unexpandable@protect{\noexpand\protect\noexpand}
\def\set@display@protect{\let\protect\string}
\def\protected@edef{%
  \let\@@protect\protect
  \let\protect\@unexpandable@protect
  \afterassignment\restore@protect
  \edef}
\def\protected@xdef{%
  \let\@@protect\protect
  \let\protect\@unexpandable@protect
  \afterassignment\restore@protect
  \xdef}
\def\restore@protect{\let\protect\@@protect}
\def\@currentlabel{}
\def\@currenvir{document}
\newdimen\p@ \p@=1pt
\def\@plus{plus}
\def\@minus{minus}
\newdimen\maxdimen \maxdimen=16383.99999pt
\newskip\z@skip \z@skip=0pt plus0pt minus0pt
\newskip\@flushglue \@flushglue=0pt plus 1fil
\newskip\fill \fill=0pt plus 1fill
\newskip\hideskip \hideskip=-1000pt plus 1fill
\mathchardef\@Mi=10001
\mathchardef\@Mii=10002
\mathchardef\@Miii=10003
\mathchardef\@Miv=10004
\def\@vpt{5}
\def\@vipt{6}
\def\@viipt{7}
\def\@viiipt{8}
\def\@ixpt{9}
\def\@xpt{10}
\def\@xipt{10.95}
\def\@xiipt{12}
\def\@xivpt{14.4}
\def\@xviipt{17.28}
\def\@xxpt{20.74}
\def\@xxvpt{24.88}
\newcount\@tempcnta
\newcount\@tempcntb
\newcount\@lowpenalty
\newcount\@medpenalty
\newcount\@highpenalty
\newcount\@beginparpenalty
\newcount\@endparpenalty
\newcount\@itempenalty
\newcount\@clubpenalty
\newcount\@topnum
\newcount\@botnum
\newcount\@dbltopnum
\newcount\@listdepth
\newcount\@enumdepth
\newcount\@itemdepth
\newcount\interfootnotelinepenalty \interfootnotelinepenalty=100
\newdimen\@tempdima
\newdimen\@tempdimb
\newdimen\@tempdimc
\newskip\@tempskipa
\newskip\@tempskipb
\newcount\@secpenalty \@secpenalty=-300
\newcount\col@number \col@number=1
\newif\if@nobreak
\long\def\@dblarg#1{\kernel@ifnextchar[{#1}{\@xdblarg{#1}}}
\long\def\@xdblarg#1#2{#1[{#2}]{#2}}
\def\secdef#1#2{\@ifstar{#2}{\@dblarg{#1}}}
\def\@seccntformat#1{\csname the#1\endcsname\quad}
\newdimen\paperwidth
\newdimen\paperheight
\newdimen\textwidth
\newdimen\textheight
\newdimen\oddsidemargin
\newdimen\evensidemargin
\newdimen\topmargin
\newdimen\headheight
\newdimen\headsep
\newdimen\footskip
\newdimen\marginparwidth
\newdimen\marginparsep
\newdimen\marginparpush
\newdimen\columnwidth
\newdimen\columnsep
\newdimen\columnseprule
\newdimen\linewidth
\newdimen\leftmargin
\newdimen\rightmargin
\newdimen\listparindent
\newdimen\itemindent
\newdimen\labelwidth
\newdimen\labelsep
\newdimen\leftmargini
\newdimen\leftmarginii
\newdimen\leftmarginiii
\newdimen\leftmarginiv
\newdimen\leftmarginv
\newdimen\leftmarginvi
\newdimen\footnotesep
\newdimen\jot
\newdimen\arraycolsep
\newdimen\tabcolsep
\newdimen\arrayrulewidth
\newdimen\doublerulesep
\newdimen\fboxsep
\newdimen\fboxrule
\newdimen\unitlength \unitlength=1pt
\newdimen\@maxdepth
\newdimen\@wholewidth
\newdimen\@halfwidth
\newdimen\@totalleftmargin
\newdimen\@colht
\newdimen\@colroom
\newdimen\@pageht
\newdimen\@pagedp
\newdimen\@textmin
\newdimen\@textfloatsheight
\newskip\topsep
\newskip\partopsep
\newskip\itemsep
\newskip\parsep
\newskip\@topsep
\newskip\@topsepadd
\newskip\floatsep
\newskip\textfloatsep
\newskip\intextsep
\newskip\dblfloatsep
\newskip\dbltextfloatsep
\newskip\@fptop
\newskip\@fpsep
\newskip\@fpbot
\newskip\@dblfptop
\newskip\@dblfpsep
\newskip\@dblfpbot
\newskip\smallskipamount
\newskip\medskipamount
\newskip\bigskipamount
\long\def\@for#1:=#2\do#3{%
  \expandafter\def\expandafter\@fortmp\expandafter{#2}%
  \ifx\@fortmp\@empty \else
    \expandafter\@forloop#2,\@nil,\@nil\@@#1{#3}\fi}
\long\def\@forloop#1,#2,#3\@@#4#5{\def#4{#1}\ifx #4\@nnil \else
       #5\def#4{#2}\ifx #4\@nnil \else#5\@iforloop #3\@@#4{#5}\fi\fi}
\long\def\@iforloop#1,#2\@@#3#4{\def#3{#1}\ifx #3\@nnil
       \expandafter\@fornoop \else
      #4\relax\expandafter\@iforloop\fi#2\@@#3{#4}}
\long\def\@fornoop#1\@@#2#3{}
\long\def\@tfor#1:=#2\do#3{%
  \edef\@fortmp{\unexpanded{#2}}%
  \ifx\@fortmp\@empty \else
    \@tforloop#2\@nil\@nil\@@#1{#3}\fi}
\long\def\@tforloop#1#2\@@#3#4{\def#3{#1}\ifx #3\@nnil
       \expandafter\@fornoop \else
      #4\relax\expandafter\@tforloop\fi#2\@@#3{#4}}
\DeclareRobustCommand{\MakeUppercase}[1]{{\protected@edef\reserved@a{#1}\expandafter\uppercase\expandafter{\reserved@a}}}
\DeclareRobustCommand{\MakeLowercase}[1]{{\protected@edef\reserved@a{#1}\expandafter\lowercase\expandafter{\reserved@a}}}
\let\uppercase@\uppercase
\let\lowercase@\lowercase
\newif\if@afterindent \@afterindenttrue
\newif\if@compatibility
\newif\if@endpe
\newif\if@eqnsw
\newif\if@fcolmade
\newif\if@filesw
\newif\if@firstamp
\newif\if@firstcolumn
\newif\if@font@series@context
\newif\if@forced@series
\newif\if@in@minipage@env
\newif\if@includeinrelease
\newif\if@inlabel
\newif\if@insert
\newif\if@mparswitch
\newif\if@negarg
\newif\if@newlist
\newif\if@nmbrlist
\newif\if@noitemarg
\newif\if@noparitem
\newif\if@noparlist
\newif\if@noskipsec \@noskipsectrue
\newif\if@ovb
\newif\if@ovhline
\newif\if@ovl
\newif\if@ovr
\newif\if@ovt
\newif\if@ovvline
\newif\if@partsw
\newif\if@pboxsw
\newif\if@reversemargin
\newif\if@rjfield
\newif\if@specialpage
\newif\if@tempswa
\newif\if@twocolumn
\newif\if@twoside
\newif\ifdt@p
\newif\ifh@
\newif\ifin@
\newif\ifmath@fonts
\newif\ifmaybe@ic
\newif\ifv@
\@fileswtrue
\newcount\insc@unt \insc@unt=\@cclv
\newcount\allocationnumber
\def\newinsert#1{%
  \global\advance\insc@unt\m@ne
  \allocationnumber\insc@unt
  \global\chardef#1\allocationnumber}
\newinsert\@mpfootins
\newinsert\footins
\newcommand\encodingdefault{OT1}
\newcommand\rmdefault{cmr}
\newcommand\sfdefault{cmss}
\newcommand\ttdefault{cmtt}
\newcommand\bfdefault{b\@empty}
\newcommand\mddefault{m\@empty}
\newcommand\itdefault{it}
\newcommand\sldefault{sl}
\newcommand\scdefault{sc}
\newcommand\updefault{up}
\newcommand\ulcdefault{ulc}
\newcommand\swdefault{sw}
\newcommand\sscdefault{ssc}
\newcommand\familydefault{\rmdefault}
\newcommand\seriesdefault{\mddefault}
\newcommand\shapedefault{n}
\def\@font@warning#1{\flashtex@latex@warning{LaTeX Font Warning: #1}}
\def\@nomath#1{\relax\ifmmode
   \@font@warning{Command \noexpand#1invalid in math mode}\fi}
\def\@setfontsize#1#2#3{\@nomath#1%
    \ifx\protect\@typeset@protect
      \let\@currsize#1%
    \fi
    \fontsize{#2}{#3}\selectfont}
\def\@setsize#1#2#3#4{\@setfontsize#1{#4}{#2}}
\def\@defaultunits{\afterassignment\remove@to@nnil}
\def\remove@to@nnil#1\@nnil{}
\catcode`P=12 \catcode`T=12
\lowercase{\def\@rem@pt@def{\def\rem@pt##1.##2PT{##1\ifnum##2>\z@.##2\fi}}}
\@rem@pt@def
\catcode`P=11 \catcode`T=11
\def\strip@pt{\expandafter\rem@pt\the}
\def\f@size{10}
\def\f@baselineskip{12.0pt}
\def\f@linespread{1}
\let\size@update\relax
\def\set@fontsize#1#2#3{%
    \@defaultunits\@tempdimb#2pt\relax\@nnil
    \edef\f@size{\strip@pt\@tempdimb}%
    \@defaultunits\@tempskipa#3pt\relax\@nnil
    \edef\f@baselineskip{\the\@tempskipa}%
    \edef\f@linespread{#1}%
    \def\size@update{%
      \baselineskip\f@baselineskip\relax
      \baselineskip\f@linespread\baselineskip
      \let\size@update\relax}%
    }
\def\fontsize#1#2{\set@fontsize\f@linespread{#1}{#2}\flashtexfontsizedone{\f@size}{\f@baselineskip}}
\def\selectfont{\size@update\flashtexselectfontdone}
\catcode`\@=12
";
