# flashtex-compiler

Original Rust compiler foundation for FlashTeX (task FT-002). No existing TeX
engine is invoked, linked, or shelled out to. The compiler has no registry
dependencies: it uses the in-repository `../font-engine` crate through a path
dependency, so the build remains offline and deterministic. The JSON transport
is hand-written.

Speaks runtime protocol v1 (`docs/contracts/runtime-v1.md`) over JSON Lines on
stdin/stdout.

## Negotiated layout capabilities

A compile request may add `payload.layout_capabilities`. It must be a
duplicate-free list of at most 16 non-empty strings, each at most 64 UTF-8
bytes. Unknown names are ignored for acceptance; malformed fields are rejected
with an explicit diagnostic. A `compile_result` echoes only supported names
that were requested, in request order. When the request field is omitted, the
response field is also omitted and the base runtime-v1 output is unchanged.
Negotiation is per request, and warm incremental sessions are isolated by the
accepted capability set.

This revision supports:

- `rules-v1`: fractions use an opaque black `rule` item with top-left `x_pt`
  and `y_pt`, positive `width_pt` and `height_pt`, and the source range of the
  generating `\frac` command. Without this capability, the legacy U+2500 text
  approximation remains and produces the existing PDF-export warning.
- `font-hints-v1`: every text item adds a `font` object containing the family,
  `normal` or `bold` weight, and `normal` or `italic` style selected by layout.
  Body text reports the face its text style selects (Times-Roman by default;
  Times-Bold, Times-Italic, Times-BoldItalic, Helvetica or Courier under the
  style commands), headings start in Times-Bold, and supported mathematical
  symbols report Symbol.

This additive extension is not rendering-v2 activation or a claim of exact
LaTeX PDF identity. Font hints do not identify font bytes, glyph IDs, shaping,
encoding, or exact advances.

```sh
cd crates/compiler
cargo test
echo '{"protocol_version":1,"id":"a","type":"compile","payload":{"project_id":"demo","revision":1,"entry_path":"main.tex","documents":[{"path":"main.tex","text":"Hello FlashTeX.\n"}]}}' \
  | cargo run --quiet --bin flashtex-compiler
```

## Status of this milestone

Implemented and tested:

- Tokenizer with exact UTF-8 byte spans. Ordinary text and literal math items
  slice back to their emitted text, verified on multi-byte input
  (`héllo — naïve café`). A substituted math glyph such as `α` instead maps to
  the command source (`\alpha`) that produced it; the span remains exact and
  slice-safe but the source slice intentionally differs from the output glyph.
- A finite parser for the subset listed below, with error recovery.
- LaTeX preamble recognition: `\documentclass[options]{class}` records the
  class, and `\usepackage[options]{a,b,c}` records the package names and emits
  one warning listing exactly those unimplemented packages. When a document
  environment exists, only its body is typeset; bare fragments retain the
  previous typeset-everything behavior. A `10pt`, `11pt` or `12pt` class
  option sets the body size (no option keeps the 12pt default), and preamble
  `\setlength{\parskip}{..}` replaces the gap between paragraphs, with `em`
  and `ex` relative to that body size. `\setlength{\parindent}{0pt}` is exact
  because paragraphs are never indented; any other `\parindent`, any other
  length, or `\setlength` in the body is reported as not implemented.
- TeX macro expansion through `crates/tex-expansion` (round 2), run as a
  compiler-owned pass in front of the parser (`src/expansion.rs`, see
  "Macro expansion and source mapping" below): category codes, `\def`/`\let`,
  `\newcommand`/`\newenvironment` with optional arguments, TeX and e-TeX
  conditionals, registers and LaTeX counters, with the step limit bounding
  runaway expansion.
- Project-relative `\input` expansion across supplied documents, with included
  text and diagnostics retaining the included document's path and byte ranges.
- Dependency-aware incremental layout reuse behind unchanged runtime-v1 messages.
  A resumable cursor in `src/layout.rs` is the only layout engine used by both
  clean and incremental builds. Per-block cache validation includes exact macro
  definitions read, preamble bytes, layout constraints, source mapping, and the
  flow geometry entering the block; `ReuseStats` reports actual reuse.
- Diagnostics carrying severity, message, source range, and a recovery note.
- The shared original Rust font engine is the single measurement path. Core 14
  shaping supplies exact AFM advances and pair kerning; its standard ligatures
  are enabled. Every shaping cluster retains the exact input byte range and text.
  Literal cluster-relative ranges are translated back into the originating
  document, while generated macro text keeps its real invocation span.
- TeX's classic text-mode input ligatures are converted before layout: a
  double backtick or a double apostrophe becomes a curly double quote, a lone
  backtick or apostrophe becomes a curly single quote (a lone apostrophe is
  always the right-hand form, exactly as plain typing behaves), three hyphens
  become an em dash, two hyphens become an en dash, and an exclamation or
  question mark followed by a backtick becomes the inverted exclamation or
  question mark. Conversion runs on ordinary text words only — math is parsed
  through an entirely separate path and is never touched, and `\verb` and the
  `verbatim`/`lstlisting` environments capture their raw text separately for
  the same reason (`\texttt`/`\ttfamily` text is still converted, an accepted
  simplification). A converted word's item keeps its exact original source span; only
  its rendered text changes, the same rule already used for a
  command-substituted glyph such as `\alpha`. All eight resulting codepoints
  (curly quotes, en/em dash, inverted `!`/`?`) have Times-Roman AFM widths and
  encode to WinAnsi, so none of this produces a new PDF-export warning.
- Greedy line breaking and page breaking onto 612×792 pt pages.
- Inline math (`$...$`) and display math (`$$...$$` and `\[...\]`), including
  nested fractions, square roots, superscripts, and subscripts.
- Numbered `\section{...}` and `\subsection{...}` headings, numbered display
  equations (`$$...$$`, `\[...\]`, and `\begin{equation}...\end{equation}`),
  and LaTeX-style subsection reset when a section advances.
- `\label{key}`, `\ref{key}`, and `\pageref{key}` with forward-reference and
  page-number convergence (at most five layout passes). Undefined references
  render `??`; duplicate labels warn and the later definition wins.
- `\begin{figure}...\caption{...}\label{key}...\end{figure}` with centred,
  numbered `Figure N: ...` captions. Figure bodies may contain supported text
  and math, but this milestone does not load images or place floating objects.
- `\begin{itemize}...\item...\end{itemize}` and
  `\begin{enumerate}...\item...\end{enumerate}` with bullet and decimal markers.
  `\setlist[<env>]{itemsep=<dimen>,topsep=<dimen>}` changes the vertical gap
  between items and around the list; other enumitem keys (`leftmargin`,
  `label`, `parsep`, `partopsep`, ...) have no layout equivalent yet and are
  named in a diagnostic instead.
- `\verb|...|` (any matching delimiter, `\verb*` shows interword spaces as a
  middle dot) and the `verbatim`/`verbatim*`/`lstlisting` environments: raw
  source text set in Courier at body size, one output line per source line,
  each tab set as a single space (as LaTeX's active tab is, rather than as an
  editor-style jump to a column stop), with `%`, `\`, `$`, `{`, and `}` never
  given their usual meaning. `lstlisting`'s `[options]` are parsed and honestly discarded (no
  syntax highlighting); an unterminated `\verb` gets a source-located
  diagnostic and recovers at end of line.
- `compile` → `compile_result`, and `error` envelopes for unknown protocol
  versions, unknown message types, and malformed JSON.
- Rejection of absolute paths and parent traversal in document paths.
- An 8 MiB JSON Lines request limit enforced while reading, without buffering an
  arbitrarily large line; the worker consumes an oversized line and continues.

Required, outstanding — this is a foundation, not a LaTeX implementation:

- Macro expansion is not interleaved with layout: `\ifvmode`/`\ifhmode`/
  `\ifmmode`/`\ifinner` see no real typesetting mode, and `em`/`ex` inside
  expansion-time dimensions use 10pt Computer Modern placeholders.
- Math remains a declared subset: matrices, alignment environments,
  `\left`/`\right` delimiter sizing and real math-font parameters are not
  implemented.
- Package declarations are recognised but packages are not loaded: package
  commands, TikZ, bibliographies, and `\cite` remain missing.
- `tabular`/`tabular*` follow the LaTeX kernel's alignment geometry (see
  `src/tabular.rs`); `longtable`, the `array` package's column types, image
  loading (`\includegraphics`), and float placement remain missing.
  `\includegraphics` emits an explicit unsupported diagnostic; a `figure` is
  laid out in source order and is not a real LaTeX float.
- Environments other than `document`, `equation`, `figure`, `itemize`,
  `enumerate`, `verbatim`/`verbatim*`, and `lstlisting` warn and typeset as
  plain text.
- No PDF output. `pdf_path` is always `null`, as the contract permits for now.
- No bidi, joining, complex-script reordering, hyphenation, or TeX optimal
  paragraph breaking. The font engine reports unsupported shaping and missing
  glyphs explicitly; the compiler never silently substitutes a missing glyph.
- Text styles use only the Core 14 metric faces. Times has real bold, italic
  and bold-italic variants; slanted shapes (`\textsl`, `\slshape`) use
  Times-Italic, and sans/typewriter text uses upright Helvetica/Courier even
  when bold or italic is also requested (the engine has no other variants).

## Supported commands

`\documentclass[options]{class}`, `\usepackage[options]{a,b,c}`,
`\newcommand{\name}{body}`, `\newcommand{\name}[n]{body}`,
`\renewcommand{\name}{body}`, `\renewcommand{\name}[n]{body}`,
`\section{...}`, `\subsection{...}`, `\label{key}`, `\ref{key}`,
`\pageref{key}`, `\caption{...}`, `\textbf`, `\emph`, `\textit`,
`\textsl`, `\texttt`, `\textrm`, `\textsf`, `\textmd`, `\textup`,
`\textnormal`, the group- and environment-scoped declarations `\bfseries`,
`\mdseries`, `\itshape`, `\slshape`, `\upshape`, `\ttfamily`, `\rmfamily`,
`\sffamily`, `\normalfont`, `\em`, and the LaTeX 2.09 forms `\bf`, `\it`,
`\sl`, `\tt`, `\rm`, `\sf`, the group- and environment-scoped size
declarations `\tiny`, `\scriptsize`, `\footnotesize`, `\small`,
`\normalsize`, `\large`, `\Large`, `\LARGE`, `\huge`, and `\Huge`,
`\begin`/`\end` for `document`, `equation`, `figure`, `itemize`, and
`enumerate` (plus the amsmath displays `alignat`, `flalign` and `multline`,
starred or not; `multline` numbers only its last line), `verbatim`,
`verbatim*`, and `lstlisting` (options parsed and discarded), `\verb`
(any matching delimiter, starred or not), `\item`, `\par`,
`\hfill`, `\hfil`, `\hspace{<dimen>}`, `\hspace*{<dimen>}`, `\\`,
`\listfiles`, `\noindent`, `\quad`, `\qquad`, `\bigskip`, `\medskip`, and
`\smallskip`. Macro
argument counts are decimal integers from 0 through 9, and replacement
parameters are `#1` through `#9`. Paragraphs are separated by blank lines.
`%` begins a comment. Any other command produces an explicit "not supported by
this compiler version" diagnostic — never silent output.

`\listfiles` is accepted anywhere and is always a no-op: MacTeX uses it to log
package version banners, and this compiler has no log stream to write them to,
so silently doing nothing is the honest behaviour rather than a fabricated log.
`\noindent` is likewise always a no-op: no paragraph in this layout model is
ever given a first-line indent, so there is no indent for it to suppress.

`\tiny` through `\Huge` scale text relative to `\normalsize` using the real
LaTeX class files' own tables (`size10.clo`/`size11.clo`/`size12.clo`),
selected by the active `10pt`/`11pt`/`12pt` class option (default 12pt); the
three tables are not a uniform scale of each other (e.g. `\large` is the same
absolute size as `\Large` in the 10pt/11pt classes, but the 12pt class's own
`\normalsize`-plus-one-step). `\normalsize` always resolves to exactly the
active body size rather than the class table's own value, so text with no
size declaration in effect is unaffected by this feature existing at all,
including for the 11pt class, where this compiler's body size is a literal
11pt rather than real LaTeX's 10.95pt `\normalsize`. Like `\bfseries` and
friends, a size
declaration stays in effect until its enclosing group or environment closes;
`\Large{...}` (a common `\textbf{...}`-style misuse) is a declaration, not an
argument-taking command, so its size stays active past the immediate group,
matching real LaTeX.

`\hfill`/`\hfil` are real infinite-stretch horizontal glue: they push the rest
of the current line to the right margin, and multiple fills on one line share
the leftover width equally, resolved once the line is known to be complete
(`layout::LayoutCursor::resolve_hfill`). `\hfil` is not distinguished from
`\hfill` by TeX's fil/fill stretch order — this layout has only one order of
infinite glue, an accepted simplification. `\hspace{<dimen>}`/`\hspace*{<dimen>}`
insert a fixed, non-stretching space instead; both forms behave identically
here since this layout never discards glue at a line break (the one place real
TeX treats the starred and unstarred forms differently). A dimension is a
number followed by `pt`, `em`, `ex`, `in`, `cm`, `mm`, or `bp`
(`parser::parse_dimen_pt`); `em`/`ex` are relative to the compiler's fixed body
size, and `ex` uses the common approximation of half an em. `\hfill`/`\hfil`
inside heading, caption, or `\textbf`-style content (which reaches the page
through `inlines_from_tokens` rather than `command`'s ordinary dispatch) are
supported, since that is exactly where `\problem{...}{...}`-style macros put
them; `\hspace` in that same position is not yet, since it needs a following
brace argument that function does not consume.

An unsupported command's diagnostic always survives, but a directly following
`{...}` argument is now sometimes also skipped rather than typeset as text: see
the recovery-policy comment on `parser::unsupported` for the exact,
conservative rule (a fixed short list of known-arity commands, or content that
looks like a bare dimension or a two-or-more-letter lowercase keyword). A
prose argument to a genuinely unknown command is never swallowed by this.

`\quad` and `\qquad` insert explicit horizontal glue of 1em/2em of the current
body text size in running text (they are also recognised inside math, where
they behave the same way). `\bigskip`, `\medskip`, and `\smallskip` end the
current paragraph and add 12pt/6pt/3pt of vertical space, plain TeX's
conventional flat amounts; this layout model has no rubber lengths, so their
usual `plus`/`minus` stretch and shrink are honestly dropped rather than
approximated.

## Macro expansion and source mapping

`src/expansion.rs` runs `flashtex-tex-expansion` over the entry document and
the files it `\input`s before the parser sees any token, so the parser never
sees a macro call. Everything the engine does not define (every typesetting
command, grouping braces, math shifts) passes through with its exact span;
the parser's own command names are declared to the engine as host commands, so
`\newcommand` refuses them and `\renewcommand` accepts them. Diagnostics use
TeX's and LaTeX's own messages (`LaTeX Error: Command \x already defined.`,
`Illegal parameter number in definition of \x.`); a runaway definition hits
the engine's step limit, which names the looping invocation, and the rest of
the document is typeset without expansion. `\verb` arguments, the bodies of
`verbatim`/`verbatim*`/`lstlisting`, and `\url`/`\href` URLs are hidden from
the engine (their bytes are blanked in a private copy, offsets unchanged), and
`\arraystretch` is read where LaTeX reads it, at `\begin{tabular}`.

Tokens substituted for `#1` through `#9` retain the real byte spans of the
argument text the author supplied. Replacement-text tokens carry the span of the
outermost invocation's control word (as before) and, additionally, their
definition span: the exact bytes of the definition they were copied from.
`Parsed::expansions` lists every run of replacement text as
`(invocation, definition)`, so a consumer can read the definition's own
spacing and glue instead of re-parsing `\newcommand` bodies.

Large entry documents (4 KB and up, no `\input`) keep a per-thread incremental
expansion cache behind `parser::parse_project`: a keystroke re-expands from the
nearest engine checkpoint before the edit and re-converts only until the old
output can be spliced back. `tests/expansion_incremental.rs` checks that the
cached result equals a from-scratch expansion after random structural edits.
Engine checkpoints are taken every 512 bytes of source: the engine's state is
copy-on-write, so a checkpoint is a few reference-count bumps and a
convergence check compares only what changed. The parser's in-place splits of
glued words (`\\[3pt]Next`, a row's `\\*`, `\cmidrule(lr)`) borrow the cached
stream instead of copying it and are undone before it goes back
(`tests/expansion_lend.rs`).

`\DeclareMathOperator{\cmd}{text}` (and `*`) is a host-prelude definition in
the expansion pass: `\cmd` becomes `\operatorname{text}` (`\operatorname*`),
defined with `\newcommand` semantics, so a second declaration of the same name
reports LaTeX's "Command \cmd already defined." at that declaration.

## Supported math

Math atoms are ordinary characters and digits. `\frac{num}{den}` and `\sqrt{x}`
may be nested, and `^` superscripts and `_` subscripts accept either one token or
a braced math list, including `x^{a_b}` and `\frac{a^2}{b_1}`.

The named symbols `\alpha`, `\beta`, `\gamma`, `\delta`, `\theta`, `\lambda`,
`\mu`, `\pi`, `\sigma`, `\phi`, `\omega`, `\times`, `\div`, `\pm`, `\leq`,
`\geq`, `\neq`, `\approx`, `\cdot`, `\infty`, `\sum`, and `\int` map to Unicode.
So do `\epsilon`, `\varepsilon`, `\zeta`, `\eta`, `\vartheta`, `\iota`, `\kappa`,
`\nu`, `\xi`, `\varpi`, `\rho`, `\varsigma`, `\tau`, `\upsilon`, `\varphi`,
`\chi`, `\psi`, `\Gamma`, `\Delta`, `\Theta`, `\Lambda`, `\Xi`, `\Pi`, `\Sigma`,
`\Upsilon`, `\Phi`, `\Psi`, `\Omega`, `\le`, `\ge`, `\ne`, `\equiv`, `\sim`,
`\cong`, `\propto`, `\perp`, `\partial`, `\nabla`, `\prod`, `\ast`, `\prime`,
`\cup`, `\cap`, `\subset`, `\subseteq`, `\supset`, `\supseteq`, `\notin`, `\ni`,
`\emptyset`, `\varnothing`, `\oplus`, `\otimes`, `\wedge`, `\land`, `\lor`,
`\to`, `\rightarrow`, `\leftarrow`, `\gets`, `\uparrow`, `\downarrow`,
`\leftrightarrow`, `\implies`, `\Leftarrow`, `\impliedby`, `\Leftrightarrow`,
`\iff`, `\Uparrow`, `\Downarrow`, `\therefore`, `\angle`, `\aleph`, `\Re`, `\Im`,
`\wp`, `\langle`, `\rangle`, `\lvert`, `\rvert`, `\lVert`, `\rVert`,
`\setminus`, and `\Longrightarrow`. `\mathbb{A}` through `\mathbb{Z}` map to
the Unicode double-struck capitals; other arguments are rejected explicitly.
`\epsilon` is the lunate U+03F5 and `\varepsilon` the open U+03B5 (cmmi
`"0F`/`"22`); the base-14 Symbol export has no lunate epsilon and draws both
with the open glyph. Symbol has no double bar, so `\lVert`, `\rVert` and `\|` are two real
vertical bars. `\iint` and `\iiint` repeat the integral glyph (Symbol has no
U+222C/U+222D). `\oint`, `\mapsto`, `\mp`, `\ll`, `\gg`, `\lfloor`, `\lceil`,
`\vdots`, `\ddots`, `\ell` and `\hbar` have no Symbol glyph and stay diagnostics.

Operator names typeset as upright roman words: `\sin`, `\cos`, `\tan`, `\cot`,
`\sec`, `\csc`, `\arcsin`, `\arccos`, `\arctan`, `\sinh`, `\cosh`, `\tanh`,
`\coth`, `\log`, `\ln`, `\lg`, `\exp`, `\lim`, `\liminf`, `\limsup`, `\max`,
`\min`, `\sup`, `\inf`, `\det`, `\gcd`, `\deg`, `\dim`, `\ker`, `\arg`, `\hom`,
`\Pr`, `\sgn`, `\bmod`, `\mod`, and `\operatorname{name}` (starred form too).
In displays, scripts on `\lim`, `\liminf`, `\limsup`, `\max`, `\min`, `\sup`,
`\inf`, `\det`, `\gcd`, `\Pr`, `\sum` and `\prod` stack centred above and
below the operator (top level of the display only); inline and on other atoms,
including integrals, they stay beside it. `\dfrac`, `\tfrac` and `\cfrac` lay out as `\frac`.
`\ldots`/`\dots` are three periods and `\cdots` three math dots. `\left`,
`\right`, `\big`, `\Big`, `\bigg`, `\Bigg` and their `l`/`r`/`m` forms keep the
requested delimiter at ordinary size (`.` is the invisible null delimiter).
`\mathrm`, `\mathit`, `\mathsf`, `\mathtt`, `\boldsymbol` and `\mbox` typeset
their argument in the current math face (no distinct face yet). `\displaystyle`,
`\textstyle`, `\limits` and `\nolimits` are accepted without changing size.
`\,` `\:` `\>` `\;` `\ ` and `\!` are math spaces, added to TeX's
inter-atom spacing: atoms are classed ord/op/bin/rel/open/close/punct/inner
and spaced by the TeXbook Chapter 18 table (thin 3mu, medium 4mu, thick 5mu
of the current math size; scripts keep only the unparenthesised thin
entries), and a binary operator with no left operand is ordinary. Fences are
classed by glyph as open/close rather than inner, `\operatorname{name}` is
ordinary, and a math `-` is the minus sign U+2212. The math environments
`split`, `aligned`, `alignedat` and `gathered` lay out as grids.

`\binom{n}{k}` (and `\dbinom`, `\tbinom`) is a two-row grid in parentheses.
`\sqrt[n]{x}` raises the index as a script ahead of the radical sign.
`\mathbf{text}` typesets literal text in Times-Bold. `\boxed{...}`,
`\overline{...}` and `\underline{...}` draw real rules around, over or under
their math list. `\tag{x}` places `(x)` two quads after the display content
(`\tag*{x}` without parentheses); it is not right-aligned yet. `\pmod{n}`
typesets `(mod n)`. `\overset{over}{base}`, `\stackrel{over}{base}` and
`\underset{under}{base}` centre a script-size list directly above or below the
base. The TeX infix forms `{n \choose k}` and `{a \over b}` build the same
grid and fraction as `\binom` and `\frac`.
The corresponding Unicode glyph must exist in the Symbol face selected by the
export mapping, except blackboard bold, `\setminus` and `\Longrightarrow`:
those are drawn from the pinned Latin Modern Math resource (`lm.math`, see
`src/lm_math.rs`). Its Unicode-math designs and widths differ from pdfLaTeX's
msbm10/cmsy10, and the base-14 PDF export reports that it cannot embed them.
`\mathcal{A-Z}` emits Unicode script capitals bound to New Computer Modern Math
(`newcm.math`, `src/newcm_math.rs`; the Mac app bundles
`apps/mac/Fonts/NewCMMath-Regular.otf`). Its default script design is not
cmsy10's calligraphic one: against a pdfLaTeX oracle
(`docs/evidence/ft060-oracle/`) only E G I N P R T U V W X are within 0.5pt at
10pt. `\varnothing` keeps the Symbol glyph with msbm10's 0.777781em advance.
A single Latin letter (a math variable) renders in Times-Italic; digits and
multi-letter names use Times-Roman. Unknown math commands produce an explicit
diagnostic naming the command and are rendered literally, never silently
dropped.

Script sizes and shifts and fraction geometry use named classic-proportion
constants in `src/math.rs`. They approximate TeX's font-parameter-driven values;
the compiler does not yet read a real math font.

`\hat`, `\bar`, `\vec`, `\tilde`, `\dot`, `\ddot`, `\acute`, and `\grave` place a
real base-14 accent glyph over `{body}`, symmetrically centered, plus an
italic-angle skew (shifted right) when `{body}` is a single Latin letter —
those render in Times-Italic, and TeX's real skewchar-kern skew (TeXbook
Appendix G, rule 12) has no equivalent in Adobe Core 14 AFM metrics, so
`crate::layout::italic_skew_pt` derives an equivalent shift from
Times-Italic's real `ItalicAngle` (-15.5 degrees) instead. Digits,
multi-letter bodies and Symbol-font Greek stay upright and keep plain
symmetric centering. `\vec` uses the Symbol arrowright glyph and `\dot` uses
the middle dot `\cdot` already renders with — the closest real glyphs
available, not TeX's exact short arrow or raised dot. `\widehat`/`\widetilde`
reuse the plain `\hat`/`\tilde` glyph unstretched (no cmex-style growing
glyph exists here), which is diagnosed when the base is more than one
symbol. `\check` and
`\breve` have no representable base-14 glyph (no caron or breve in WinAnsi
or the Symbol encoding) and are diagnosed rather than faked; the base still
typesets without a mark. `\overline{body}` and `\underline{body}` draw a
real rule spanning `body`'s width, the same rule/legacy-glyph pattern the
fraction bar uses.

## Font shaping and layout limits

Layout measures body text in the Core 14 face its text style selects, headings
in Times-Bold unless restyled, and supported math symbols in Symbol through `flashtex-font-engine::shape`. The returned cluster
advances already include AFM pair kerning and enabled standard ligatures. One
item is still emitted per word rather than per line; its span is derived from the
shaped clusters and remains an exact document byte range for literal text.

This is real Core 14 shaping, but it is not full TeX paragraph layout. Greedy
line breaking, approximate math constants, no hyphenation, and the lack of a
negotiated original-glyph rendering contract still prevent pixel or PDF identity
claims.

## Pinned evidence and separate fidelity gates

`tests/pinned_fixtures.rs` compiles plain text, a heading, inline/display math,
a macro, an include, a forward reference, and `AV Wa To Ty`. It compares the
complete live `compile_result` JSON against committed JSONL bytes in
`tests/pinned/`, once with no capabilities and once with `rules-v1` plus
`font-hints-v1`. It does not parse, round, reorder, or normalize either stream;
on failure it lists every differing byte offset with context. The regeneration
command is pinned in the test header and is ignored during ordinary test runs.

This compiler evidence is distinct from three other gates: raw PDF-byte equality,
raster pixel equality, and incremental-versus-clean equivalence. The first two
require downstream PDF/native artifacts and are not asserted in this crate;
incremental-clean equivalence remains covered by the compiler's session tests.
No reference TeX engine is invoked in production.

## Negotiated representation and source attribution

**Fraction rules retain a legacy route.** A client that negotiates `rules-v1`
receives a typed rectangle and no box-drawing fraction glyph. An old client, or
one that does not request the capability, still receives the original text item
and its explicit export-approximation warning. Typed rule paint order is its
position in the page's item list.

**Substituted glyphs span their source command.** `\alpha` emits an item whose
text is the Greek letter but whose span covers `\alpha` in the source, six bytes.
Generated section, equation, figure, and list numbers follow the same rule: their
spans cover the `\section`, display delimiter/`\begin`, `\caption`, or `\item`
command that produced them. For these items the span does not slice back to the
item's text, unlike ordinary words. That is deliberate: source navigation must
land on the command the author typed. The ordinary-text invariant — every
ordinary word item's span slices back to exactly that word — is unchanged.

## Scaling, measured

Run `cargo run --release --bin scaling_bench`. It prints the SHA-256 of every
generated input and of the binary, so a number can be tied to what produced it.
Inputs are generated by a seeded LCG, so they are identical on every machine.

One-word edit, p95, before and after the revision-7 parser fix:

| Size | Blocks | Before | After |
|---|---|---|---|
| 5 KB | 84 | 0.801 ms | 0.551 ms |
| 50 KB | 792 | 7.402 ms | 2.541 ms |
| 500 KB | 7 754 | 420.606 ms | **29.401 ms** |

Scaling is now approximately linear. It was not: 10x the blocks cost 64x the
time between 50 KB and 500 KB, because every macro invocation removed the
invocation token and then spliced its expansion into the gap, moving the tail of
the token vector twice. Replacing the token in a single splice makes a one-token
expansion an in-place overwrite that shifts nothing. Parse fell from 402.586 ms
to 12.254 ms at 500 KB.

The pinned byte-exact fixtures were unchanged by this work, which is the evidence
that it is a pure speedup and not a change in output.

These are COMPILER-ONLY measurements, from source text to laid-out result. UI
paint, scheduling, IPC transport and PDF writing are outside this crate. Native
paint parity and raw PDF byte equality are separate gates and are not claimed
here. The product target of under 200 ms from keystroke to visible output
REMAINS UNPROVEN and can only be established by measuring the real application.

## Recovery behaviour

`status` is `ok` with no diagnostics, `recovered` when diagnostics were produced
but text was still positioned, and `failed` when nothing could be produced.
Recovered cases include unmatched `{`, stray `}`, unterminated environments,
mismatched `\end`, unknown commands, and empty required arguments. Each carries a
`recovery` string stating what was rendered provisionally.

## Incremental safety boundary

The parser executes the complete document on every changed revision so macro and
group state, diagnostics, and recovery are identical to a clean build. Only
positioned block-layout fragments are reused. Changing a macro definition
invalidates every block that actually read that definition; unrelated blocks may
still be reused if their entering flow geometry matches. A changed flow state
(for example, because an earlier edit adds a line) recomputes the affected suffix
until geometry matches again.

A first compile, any preamble-byte change through `\begin{document}` (including
`\documentclass` or `\usepackage`), font-size or measure changes, malformed input,
or any diagnostic/unsupported construct forces a full layout rebuild. Category
codes, registers, assignments and conditionals are executed by the expansion
pass on every revision, so their effects are already in the blocks that layout
reuse compares; auxiliary files, output routines, external effects, and future
constructs are not modeled and must force a full rebuild if introduced. An exactly unchanged snapshot may
return its already-produced output, including diagnostics, because no execution
or layout result can differ.

Counters are embedded in parsed counter-bearing blocks, so changed incoming
counter values invalidate those blocks and geometry invalidates affected suffixes.
Labels and references are more global: any changed snapshot containing either is
laid out conservatively from scratch and passed through the bounded convergence
loop. This intentionally sacrifices reuse to keep every incremental result
byte-identical to a clean build.

## Measured incremental latency

Measured on `mac-m5pro-kabir` on 2026-09-12 with:

```sh
cargo run --release --bin incremental_bench
```

The deterministic `generated-500-paragraphs` fixture is built by the benchmark:
500 multi-line paragraphs, 118,700 UTF-8 bytes, with one user-macro expansion,
inline scripted math, a named math symbol, and a fraction in every paragraph.
Fifty samples produced these actual compiler-only measurements:

| Case | Actual latency | Reuse |
|---|---:|---:|
| First cold compile | 45.762 ms | 0 / 500 blocks |
| Cold compile | median 37.882 ms, p95 40.453 ms | 0 / 500 blocks |
| Warm unchanged | median 0.728 ms, p95 0.879 ms | 500 / 500 blocks |
| One-word edit in paragraph 250 | median 27.459 ms, p95 29.225 ms | 499 / 500 blocks |
| Global macro-definition edit | median 40.211 ms, p95 42.009 ms | 0 / 500 blocks |

These numbers were re-measured after adopting shared Core 14 shaping. Cold and
global-macro cases include shaping every recomputed block. The one-word edit
still reuses 499 of 500 blocks.

The measured compiler work is below the 200 ms ordinary warm-edit target; the
one-word edit p95 is 29.225 ms, leaving 170.775 ms of that budget. This is not an
end-to-end keystroke-to-visible measurement: scheduling, JSON transfer, native UI
drawing, and artifact publication are excluded, so the full product target still
requires integration measurement. The benchmark intentionally does not claim a
guarantee for arbitrary documents or TeX programs.
