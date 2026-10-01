# Hybrid conceal

Settings ▸ Conceal shows LaTeX markup as what it means while you are not
editing it: `\alpha` as α, `\leq` as ≤, `x^2` as x², `\textbf{bold}` as a bold
**bold** with the command hidden, ``` ``quoted'' ``` as “quoted”. The source
comes back as soon as the insertion point reaches it. It is **off by
default**; turn on "Conceal LaTeX markup".

Concealment is display only. The text is never changed: copying, Find, undo,
saving, compiling, the preview's source mapping and VoiceOver all work on the
real source (`HybridConcealEditorTests` checks the storage, undo and copy).

## When the source shows

| Setting | Source shows while… |
|---|---|
| On the caret's line (default) | the insertion point, or a selection, is on the line. |
| At the caret | the insertion point touches the construct, or a selection overlaps it. |
| Always show source | always: nothing is concealed. |

A selection longer than 400 lines reveals only the lines at its two ends.

## What can be concealed

Every kind has its own switch. Fractions and section headings are off by default.

| Kind | Example | Where |
|---|---|---|
| Greek letters | `\alpha` → α, `\Gamma` → Γ, `\varepsilon` → ε | math |
| Math symbols and operators | `\leq` → ≤, `\to` → →, `\infty` → ∞, `\in` → ∈, `\sum` → ∑, `\int` → ∫ | math (`\ldots`, `\S`, `\copyright` … also in text) |
| Subscripts and superscripts | `^2` → ², `_i` → ᵢ; without a Unicode form, `^{q}` is drawn smaller and raised | math |
| Font commands | `\textbf{x}` bold, `\textit`/`\emph` italic, `\texttt` as is, `\mathbb{R}` → ℝ, `\mathcal{A}` → 𝒜, `\mathfrak{g}` → 𝔤 | text and math; the argument must close on the same line |
| Fractions (off) | `\frac{a}{b}` → a⁄b | math |
| Quotes and dashes | ``` `` ``` → “, `''` → ”, `--` → –, `---` → — | text only (in math `--` stays two minus signs) |
| List items | `\item` → • | text |
| Dim comments | `% note` drawn at half strength | never concealed |
| Section headings (off) | `\section{Intro}` → § Intro in bold, `\subsection` → §§ | text |

**Never conceal** takes command names separated by commas (`\textbf, \phi`) —
for example "conceal all Greek except `\phi`", or "conceal Greek but keep
`\textbf` as typed". The non-command forms are `--`, `---`, ```` `` ````, `''`,
`^` and `_`. A name also covers its starred form (`section` covers `\section*`).

Math and text are told apart by the syntax highlighter's own mode tracking
(`$…$`, `\[…\]`, `\(…\)`, math environments, verbatim and comments), so the
same rules decide colour and concealment. BibTeX and TOML buffers are never
concealed.

## Moving the insertion point

The insertion point always moves over real characters. On the caret's line
everything is revealed, so arrow keys go one character at a time. Moving up
or down (or clicking) onto a line that was concealed never leaves the
insertion point inside source that was hidden a moment before — it snaps to
the nearer end of the hidden `\textbf{` or `\alpha`. Text that stayed visible
(the bold word itself) is left as is.

## VoiceOver

VoiceOver reads the source, never the concealed display: the editor's
accessibility value is the text storage, so `\alpha` is read as it is written.
Hearing a symbol that is not in the file would make editing by ear
unreliable, and the rotor, the caret announcements ("line 3, column 5") and
the diagnostics all refer to the source.

## How it is drawn (TextKit 1)

The implementation is `Sources/FlashTeXMac/HybridConcealDisplay.swift`; the
rules are the platform-free `Sources/FlashTeXEditorCore/HybridConceal.swift`.

- Hidden source gets the null glyph property in
  `layoutManager(_:shouldGenerateGlyphs:…)` (the delegate code folding already
  used).
- A replaced construct's first glyph becomes a control glyph laid out as
  whitespace exactly as wide as the replacement; the replacement is drawn
  into that box after the text, in the theme's conceal colour (math) or text
  colour.
- Bold and italic kept text are temporary attributes (a hard one-point
  shadow in the run's colour; obliqueness), painted with the syntax colours
  over the same window.
- Spans are computed per line from the highlighter's per-line lexing and
  cached relative to the line start; an edit recomputes only the edited lines
  (and lines whose math mode it changed). A caret move to another line
  re-lays out the two lines involved, and only when they have something
  concealed.

Measured on the 560 KB bench document (every paragraph has math, `\textbf`
and `\emph`), conceal on versus off in the same process, thread CPU p50: a
keystroke near the start 5.4 vs 7.6 ms, near the end 49.8 vs 49.4 ms, arrow
down 3.4 vs 3.6 ms per step — within noise on a machine at load ~500
(`HybridConcealEditorTests.testLargeDocumentKeystrokesWithConcealOnAndOff`).
