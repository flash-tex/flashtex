\NeedsTeXFormat{LaTeX2e}
\ProvidesClass{pset_d}[2016/2/10 Arun Debray's problem set template]
% I decided to split my pset template into a class because:
%	- my pset template is currently duplicated with each new math class I take; and
%	- that code is intermixed with macros that affect my lecture notes.
% Thus, on this relatively big revision of my template, I might as well ensure that I centralize everything and
% organize it.

% Provides a switch gto check whether you're using this class (e.g. \ifdefined\UsingPsetD)
\def\UsingPsetD{}

% Some package options
% 1. Option `boldsets' replaces \mathbb N, Z, Q, R, ... with \mathbf. `bbsets` (default) uses \mathbb.
\newcommand{\m@thbb}{\@empty}
\DeclareOption{boldsets}{\renewcommand{\m@thbb}{\mathbf}}
\DeclareOption{bbsets}{\renewcommand{\m@thbb}{\mathbb}}
% 2. Option `rmdiff' (default) typesets the d in \int f(x) dx upright; `itdiff` sets it in italic.
\DeclareOption{rmdiff}{}
\DeclareOption{itdiff}{\def\df@nt{}}
% 3. Option `thminenum' (default) numbers theorems inside problem numbers, e.g. Theorem 27.1; `thmalone' numbers
% them independently, e.g. Theorem 1.
\DeclareOption{thminenum}{}
\DeclareOption{thmalone}{\def\thm@enum{}}
% 4. Option `eqninenum' numbers equations inside problem numbers, e.g. (27.1); `eqnalone' (default) tags them with
% dingbats, e.g. (*), (§), etc.
\DeclareOption{eqninenum}{\def\eqn@enum{}}
\DeclareOption{eqnalone}{}
% 5. Option 'onesided' places a header on each page; `twosided' (default) on each odd-numbered page.
\DeclareOption{onesided}{\def\@nesided{}}
\DeclareOption{twosided}{}

% Other options can be handed off to the amsart class (e.g. fontsize)
\DeclareOption*{\PassOptionsToClass{\CurrentOption}{amsart}}

\ExecuteOptions{bbsets}
\ExecuteOptions{rmdiff}
\ExecuteOptions{thmienum}
\ExecuteOptions{eqnalone}
\ExecuteOptions{twosided}
\ProcessOptions\relax

\LoadClass[reqno]{amsart}

% This allows us to use characters like é and ö directly, rather than having to write \'e and \"o
\RequirePackage[utf8]{inputenc}

% Sets the margin.
\RequirePackage[margin=0.75in]{geometry}

% Other basic packages I load.

% Sets up the title -- each document specifies the \title, \author, \course, \psnum, and \date, and \maketitle
% formats the title based on those constraints.
% TODO may want to rearrange
\newcommand{\@uthor}{\@empty}
\newcommand{\c@urse}{\@empty}
\newcommand{\psn@m}{\@empty}
\newcommand{\d@e}{\@empty}
\let\@oldauthor\author % preserves original amsart definitions
\renewcommand{\author}[1]{\renewcommand{\@uthor}{#1}}
\newcommand*{\course}[1]{\renewcommand{\c@urse}{\MakeUppercase{#1}}}
\newcommand*{\psnum}[1]{\renewcommand{\psn@m}{#1}}
\renewcommand*{\date}[1]{\renewcommand{\d@e}{#1}}

% Format title based on amsart's native title command
\title{\c@urse{} Homework \psn@m}
\@oldauthor{\@uthor\\\d@e}

% The header is primarily for keeping the document organized, e.g. if the pages are separated, it should be easy to
% put it back together. Options `onesided' and `twosided' determine whether the header is on each odd-numbered page
% or all pages.
\RequirePackage{fancyhdr}

% Format: ARUN DEBRAY M383C HW1
\newcommand{\he@der}{\footnotesize{\MakeUppercase{\@uthor{}} \c@urse{} HW\psn@m}}

\pagestyle{fancy}
\fancyfoot[C]{\footnotesize{\thepage}}
\fancyhead[L,R]{}
\ifdefined\@nesided
	\fancyhead[C]{\he@der}
\else
	\fancyhead[CO]{\he@der}
\fi

\renewcommand{\headrulewidth}{0pt}

% Math macros I use.

\RequirePackage{amssymb}
\RequirePackage{mathtools}

% Shortcuts for blackboard bold math symbols
\newcommand{\N}{\m@thbb N}
\newcommand{\Z}{\m@thbb Z}
\newcommand{\Q}{\m@thbb Q}
\newcommand{\R}{\m@thbb R}
\newcommand{\C}{\m@thbb C}
\newcommand{\F}{\m@thbb F}
% allows additional user-defined symbols, e.g. \newbbsym{\P}{P} makes \P into either \mathbb{P} or \mathbf{P},
% depending on class settings. You can use \newbbsym to overwrite preexisting commands as well.
\newcommand*{\newbbsym}[2]{
	\providecommand{#1}{\@empty}
	\renewcommand{#1}{\m@thbb{#2}}
}

% Some shortcuts I use a lot
\newcommand{\e}{\varepsilon}
\newcommand{\vp}{\varphi}
\renewcommand*{\vec}[1]{\boldsymbol{\mathbf{#1}}} % so that both `\vec x` and `\vec 1` produce bold
\newcommand{\inj}{\hookrightarrow}
\newcommand{\surj}{\twoheadrightarrow}
\newcommand*{\many}[1]{#1 \dotsb #1}

% Delimiters.
% \paren and \bkt autoexpand by default, and the rest only expand if you use the starred version.
% You may disagree with this approach, but it's worked quite well for me: I use () and [] for parentheses or
% brackets that I don't want to expand, and \paren{} and \bkt{} for those that I want to expand. However, for the
% other delimiters, I tend to use them everywhere, e.g. I'll write \abs{A_1} rather than |A_1| or \lvert A_1
% \rvert. Hence, since they generally shouldn't be expanded inline, I make the default unexpanded, with the starred
% option changing it.
\DeclarePairedDelimiter{\paren}{(}{)}
\DeclarePairedDelimiter{\set}{\{}{\}}
\DeclarePairedDelimiter{\ang}{\langle}{\rangle}
\DeclarePairedDelimiter{\abs}{\lvert}{\rvert}
\DeclarePairedDelimiter{\norm}{\lVert}{\rVert}
\DeclarePairedDelimiter{\bkt}{[}{]}
\let\@oldparen\paren
\def\paren{\@ifstar{\@oldparen}{\@oldparen*}}
\let\@oldbkt\bkt
\def\bkt{\@ifstar{\@oldbkt}{\@oldbkt*}}

% Shortcuts for calculus.
% Use \ud for integrals, e.g. \int f(x) \ud x, and \d to override spacing, e.g. \int f(x,y) \ud x\wedge\d y.
% \dfr{f}{x} typesets df/dx, and \pfr{f}{x} typesets ∂f/∂x.
% Some packages that I use often (e.g. font packages) overwrite \d, and therefore this class needs to define \d
% after it's overwritten, so \AtBeginDocument to postpones these definitions.
% TODO: better macros for higher partial derivatives. One clear example would be \pfr[3]{f}{x} for the third
% derivative with respect to x.
% TODO: I would like the spacing to look nicer for itdiff
\AtBeginDocument{
	\ifdefined\df@nt
		\renewcommand{\d}{d}
	\else
		\renewcommand{\d}{\mathrm d}
	\fi
	\newcommand{\ud}{\,\d}
	\newcommand*{\dfr}[2]{\frac{\d #1}{\d #2}}
	\newcommand*{\pfr}[2]{\frac{\partial #1}{\partial #2}}
}

% Enables a"1 as a shortcut for a^{(1)}; a"{10} is a^{(10)}. I found myself using this a lot.
% The catcode needs to be changed thrice because packages that the user might add later (in particular mathpazo)
% need " to not be active.
\AtBeginDocument{\catcode`\"=13}
\catcode`\"=13
\newcommand*{"}[1]{^{(#1)}}%
\catcode`\"12

% Theorems: numbered (thm, lem) and unnumbered (thm*, lem*) versions are provided. If `\thm@enum` is defined, i.e.
% if the `thminenum' option was passed, theorems are numbered within problems.
\ifdefined\thm@enum
	\newtheorem{thm}{Theorem}
\else
	\newtheorem{thm}{Theorem}[enumi]
\fi
% everything else follows from that definition
\newtheorem{lem}[thm]{Lemma}
\newtheorem*{thm*}{Theorem}
\newtheorem*{lem*}{Lemma}
\newtheorem{claim}[thm]{Claim}
\newtheorem*{claim*}{Claim}

\renewcommand{\qedsymbol}{\small{\ensuremath{\boxtimes}}}

% Equation numbering.
% TODO: right now, equation numbering only works when problems are numbered consecutively, since `\item[custom
% number]` doesn't update the item counter. This is part of the reason that `eqnalone` is the default option. This
% is also a problem for \ref. That is, I would like to be able to define \item[2.A.] and automatically number
% equations inside it as (2.A.1), etc., and have references to it evaluate to 2.A. One way this might work is to
% use this to define some command like \itemname and then, instead of using \numberwithin as I do below, to use
% \renewcommand{\theequation}{\itemname . \arabic{equation}}, but I haven't begun to look at this.
\ifdefined\eqn@enum
	\numberwithin{equation}{enumi}
\else
	% If you use symbols to number equations, there's a good chance you may want to number at least ten equations.
	% However, if you do that, the default implementation of fnsymbol throws an error; it only supports up to nine
	% symbols. This code extends it to allow more symbols, up to 24. If you have more than 24 equations, it's
	% likely a good idea to switch to numbering them, as that will confuse your reader less; however, equation
	% numbering is not yet working the way I like it to (see above).

	\def\@fnsone{*}
	\def\@fnstwo{\dagger}
	\def\@fnsthree{\ddagger}
	\def\@fnsfour{\mathsection}
	\def\@fnsfive{\natural}
	\def\@fnssix{\asymp} % these are a little arbitrary, but I liked them better than the default choices
	\def\@fnsseven{\clubsuit}
	\def\@fnseight{\lozenge}
	
	\newcommand{\@dbl}[1]{#1 #1}
	\newcommand{\@tpl}[1]{#1 #1 #1}

	\renewcommand{\@fnsymbol}[1]{\ensuremath {\ifcase #1\or
	\@fnsone\or \@fnstwo\or \@fnsthree\or \@fnsfour\or
	\@fnsfive\or \@fnssix\or \@fnsseven\or \@fnseight\or
	\@dbl\@fnsone\or \@dbl\@fnstwo\or \@dbl\@fnsthree\or \@dbl\@fnsfour\or 
	\@dbl\@fnsfive\or \@dbl\@fnssix\or \@dbl\@fnsseven\or \@dbl\@fnseight\or 
	\@tpl\@fnsone\or \@tpl\@fnstwo\or \@tpl\@fnsthree\or \@tpl\@fnsfour\or 
	\@tpl\@fnsfive\or \@tpl\@fnssix\or \@tpl\@fnsseven\or \@tpl\@fnseight
	\else \@ctrerr \fi }}

	\renewcommand{\theequation}{\fnsymbol{equation}}
\fi

% Problem numbering.
% By default, problems are numbered in bold, with a period at the end: 1. 2. 3.
% I also adjusted the spacing of lists: after every item in a list, there is a little space, and inside itemize or
% enumerate environments, paragraphs are double-spaced rather than indented. I found this to look less cramped, and
% therefore easier to read.
% TODO: oftentimes, I must specify the problem number by hand, e.g. as `\item[3.4.]`. When this happens, the bold
% formatting is lost, and references to that item don't automatically use 3.4 (indeed, the item counter isn't even
% updated). This is the current major thing I want to fix in this package.
\RequirePackage{enumitem}

\def\thisitem{\empty}

\setlist{parsep=1.5ex, listparindent=0cm}
\setlist[enumerate, 1]{label=\textbf{\arabic*.},
					   %\textbf{\protect\thisitem.},
					   %ref={\protect\thisitem},
					   ref=\arabic{enumi},
					   leftmargin=0cm,
					   parsep=2ex
					  }

% let's see if this works (won't work for nested items, yet)
% adapted from http://tex.stackexchange.com/a/97437/55879
% TODO this causes the description environment to fail horribly
% \let\@olditem\item
% \renewcommand{\item}[1][]{%
% 	%\stepcounter{enumi}
% 	\if\relax\detokenize{#1}\relax
% 		\def\thisitem{\arabic{enumi}}%
% 	\else
% 		\def\thisitem{#1}%
% 	\fi
% 	\@olditem%
% }
