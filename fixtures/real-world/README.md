# Real-world document corpus

Owner: lane `mac-realworld-corpus` (parent `mac-claude-a`). Driven by
`tools/real-world-corpus/run.py`; the latest report is under
`docs/evidence/real-world-corpus-<UTC>/`.

| fixture | what it exercises | source |
|---|---|---|
| `hw1` | the user's target: `article` 11pt, `geometry`, `amsmath`/`amssymb`/`amsthm`, `enumitem`, `\newcommand` with arguments, `\subsection*` via a macro, inline and display math, `array`, `center`, `quote` | user-provided `HW1.tex` + `HW1-reference.pdf` — **byte-immutable** (Commander ruling, issue #2); `reference-mactex2026.pdf` is the harness's separate MacTeX regeneration |
| `article-twocolumn` | `[twocolumn]` article, `\maketitle`, `abstract`, `\section`/`\subsection`, `\label`/`\ref`/`\eqref`, `figure` with `[demo]` `\includegraphics` placeholder, `table` + `tabular` + `booktabs`, `equation` with `cases`, `thebibliography` + `\cite` | written for this corpus |
| `lecture-notes` | `amsthm` `\newtheorem` (numbered, shared counter, starred), `theorem`/`lemma`/`definition`/`proof` with optional titles, `\tableofcontents`, `align*`, `\DeclareMathOperator`, `enumerate` with `[(i)]` | written for this corpus |
| `cv` | `itemize[nosep]` (`enumitem`), `tabular*` with `\extracolsep`, `\hfill`, `\rule`, `\href`, size/weight switches, `\pagestyle{empty}` | written for this corpus |
| `letter` | `letter` class: `\signature`, `\address`, `\opening`, `\closing`, `\encl`, `\cc`, `\ps` | written for this corpus |
| `input-bibliography` | `\input{sections/…}` (two files), `\appendix`, `equation` with `\frac`/`\sum`/`\left(\right)`, `thebibliography` + `\cite` | written for this corpus |
| `math-sheet` | `align`, `align*`, `gather`, `equation`, `cases`, `pmatrix`/`bmatrix`, `\frac`/`\dfrac`, `\sum`/`\int` with limits, `\left…\right`, `\bigl…\bigr`, `\binom`, `\operatorname`, Greek letters and relation symbols | written for this corpus |
| `inline-math` | the user's Chain Rule theorem/proof paragraph (inline math running off the margin) plus five paragraphs of dense inline math: line breaks inside `$…$` after Bin/Rel atoms, `\lim_{h\to 0}`, `\|L_f\|_{\mathrm{op}}`, `\left…\right`, `\bigl…\bigr`, `\sum`/`\prod` with limits, `\mathbb` | written for this corpus (lane `mac-inline-math-breaks`); `crates/render-pipeline/tests/inline_math_breaks.rs` gates the line breaks against `reference.pdf` |
| `unicode-accents` | UTF-8 accented Latin, the same via accent commands, ligatures/dashes/quotes, `CJKutf8` Japanese and Chinese | written for this corpus |

## 2026-09-14 additions — documents that look like what people submit

Owner: lane `linux-unknown-unknowns` (machine `linux-primary`). Written for
this corpus to widen the net beyond the per-feature suites: every one of these
twelve diverges from pdfLaTeX somewhere, and every reference below was
generated with **TeX Live 2025**, not MacTeX 2026 (each `reference.json`
carries `"reference_engine": "TeX Live 2025"`). The minimal reproducers cut
out of them live in [`fixtures/divergence-probes/`](../divergence-probes/README.md).

| fixture | what it exercises | pdfLaTeX pages / overfull | render pages |
|---|---|---|---|
| `ps-calculus` | maths problem set: `article` 11pt, `amsmath`/`amssymb`, `enumitem` `[label=(\alph*)]`/`[(\roman*)]`, `\section*`, `\paragraph`, `equation`+`\eqref`, `align` with `\notag`, `align*` column pairs, `pmatrix`, `\dfrac`, `\left(\right)` | 3 / 0 | 3 |
| `lab-report` | physics lab report: `abstract`, numbered sections, `figure` + `\includegraphics[draft]`, `table` + `booktabs` + `siunitx` `S` columns, `\SI`/`\num`/`\si`, `\footnote` | 3 / 0 | **2** |
| `twelvept-plain` | 12 pt with **no amsmath**: kernel-only `\frac`, `\sqrt`, `\bigl`…`\Biggl`, `\left\|`, `\langle`, `\overline`, `\vec` | 3 / 0 | **4** |
| `plain-article` | **no packages at all** (OT1 CM): `\maketitle`, `abstract`, sections to `\subsubsection`, `description`, `quote`, `tabular`, `thebibliography`, ligatures, dashes, `\LaTeX`/`\TeX` | 3 / 2 | 3 |
| `conf-paper` | two-column conference paper: `abstract`, `figure*`, `booktabs` table, `hyperref`, `\url`, `\cite`, dense prose | 4 / 0 | **2** |
| `thesis-chapter` | `report` class, `\chapter`, two `\input{sections/…}` files, `natbib` `\citep`/`\citet`, cross-file `\ref` | 5 / 0 | **3** |
| `lmodern-report` | ordinary report with `lmodern`, `\tableofcontents`, `booktabs`, `description`, `quote`, `\textsc` — **no amsmath** | 4 / 1 | 4 |
| `siunitx-tables` | data tables: `booktabs`, siunitx `S[table-format=…]`, `\qty`/`\unit` and legacy `\SI`/`\si`, `\multicolumn`, `\cmidrule` | 2 / 0 | 2 |
| `listings-manual` | software manual: `listings`, `lstlisting`, `\lstinline`, `verbatim`, `\texttt`, `hyperref` — **measured on substituted `ectt` metrics** (see below) | 3 / 1 | 3 |
| `enumitem-worksheet` | worksheet: `enumitem` three levels deep, `\setlist`, `[nosep]`, `[leftmargin=*]`, `description[style=nextline]`, `\hrulefill` | 3 / 0 | 3 |
| `hyperref-toc` | documentation: `hyperref` + `\hypersetup`, `\tableofcontents`, `\listoffigures`, `\listoftables`, `\addcontentsline`, `\pageref` — **substituted `ectt` metrics** | 4 / 1 | 3 |
| `natbib-review` | literature review: `natbib` `\citep`/`\citet`/`\citeauthor`/`\citeyearpar`, `\newblock` bibliography, long cited prose | 3 / 0 | 3 |

## 2026-09-18 additions — beamer slide decks

Owner: lane `beamer/corpus` (mac-claude-a). Written for this corpus as the
conformance set for full `\documentclass{beamer}` support (issue #841, PRs
#855 and #887). Each deck is at most eight frames; the expected page count
is the reference's, which is what the overlay rule and frame breaking must
reproduce. References were generated with MacTeX 2026 (TeX Live 2026) and
say so in `reference_engine`. The first audit is under
`docs/evidence/beamer-audit-2026-09-18T172738Z/`.

| fixture | what it exercises | pdfLaTeX pages / overfull |
|---|---|---|
| `beamer-default` | default theme, navigation symbols **on**: `\title`/`\subtitle`/`\author`/`\institute`/`\date` + `\titlepage` frame, six `\frametitle` frames with prose, `itemize`, `enumerate`, `\section` between frames (must not print) | 7 / 0 |
| `beamer-madrid` | the same content with `\usetheme{Madrid}`: short `\title[..]`/`\author[..]`/`\institute[..]`, filled frametitle bar, three-box infolines footline with frame/total counter, 1em text margins | 7 / 0 |
| `beamer-overlays` | `\pause` in a list, `\item<2->`/`<3->`, `\only<1>{}\only<2>{}`, `\uncover<2->{}`, `\alert<2>{}`, `\onslide<2->`/`<3->`: frames of 3, 3, 2, 2, 3 and 1 slides | 14 / 0 |
| `beamer-blocks-columns` | `block`/`alertblock`/`exampleblock`, `columns[T]` with two `\column{.5\textwidth}`, `\includegraphics[width=.6\textwidth]` (`figure.png`, copied from `tests/grok-corpus/images/clean-quadratic-formula.png`), `table` + `booktabs` `tabular` + `\caption` in a frame, `\footnote` in a frame, `columns` inside a `block` | 6 / 0 |
| `beamer-fragile` | `[fragile]` with `verbatim` and `lstlisting`, `[allowframebreaks]` with a twenty-item list that breaks onto a second page (`A long list I`/`II`), the `\frame{...}` command form, `[plain]`, `\note{...}` (must not print) | 8 / 0 |

`listings-manual` and `hyperref-toc` (and `siunitx-tables`) contain `\texttt`
or `verbatim`. The bundled `texmf/fonts/tfm/jknappen/ec` ships no `ectt*.tfm`,
so the renderer falls back to `ec-lmr*` metrics and the harness reports
`ec_metrics_unavailable` as a FONT-ENV FAILURE: **their word positions are not
a measurement of line breaking** until those metrics are bundled. The other
nine run with zero font-FAILURE diagnostics.

Every `reference.pdf` was generated by the harness with MacTeX 2026
(`pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026)`, `SOURCE_DATE_EPOCH=0
FORCE_SOURCE_DATE=1`, run until the PDF stops changing); `reference.json`
records the pass count it took, the SHA-256, page
count and argv. The twelve fixtures added on 2026-09-14 were generated with
TeX Live 2025 instead and say so in their sidecar's `reference_engine`.
pdflatex is an oracle only and never runs in the product.

## Adding a fixture

Create `fixtures/real-world/<id>/main.tex` (plus any `\input` files below it),
run `tools/real-world-corpus/run.sh --only <id>` — the reference and sidecar
are generated on the first run — and commit the `.tex`, `reference.pdf` and
`reference.json`. Only write documents yourself: no downloads, no third-party
copyrighted text. Keep a user-provided reference byte-immutable; the harness
never overwrites one.
