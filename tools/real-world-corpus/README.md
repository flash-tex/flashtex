# Real-world corpus acceptance harness

Owner: lane `mac-realworld-corpus` (Claude Code subagent, parent `mac-claude-a`,
machine `mac-m1max-a`). Created 2026-09-12. Python 3 standard library only.
Measures how far the project's own producers are from pdfLaTeX on *real
documents* — the user's `HW1.tex` plus typical documents written for this
corpus — and derives a per-construct coverage table from the sources, so every
crate owner can see which constructs their crate does not yet handle.

```sh
# one-shot with this checkout's compiler, the pinned flashtex-render scratch
# build and both PDF routes (see run.sh for the env overrides):
tools/real-world-corpus/run.sh                 # -> docs/evidence/real-world-corpus-<UTC>/report.{md,json}
tools/real-world-corpus/run.sh --only hw1 --no-oracle --out /tmp/rw   # quick, no pdflatex
python3 -m unittest discover -s tools/real-world-corpus -p 'test_*.py' -v   # 6 pure-python tests
```

## What one run does

1. **Fixtures**: every directory under `fixtures/real-world/` with `main.tex`
   (or a single `*.tex`); every `.tex` below it is sent as a runtime-v1
   `documents[]` entry so `\input` works.
2. **Reference**: `reference.pdf`, else `*-reference.pdf`. Missing references
   are generated with MacTeX 2026 `pdflatex` (`SOURCE_DATE_EPOCH=0
   FORCE_SOURCE_DATE=1`, `-halt-on-error`, repeated until the PDF stops
   changing and pdflatex stops asking for a rerun — two passes are not always
   enough) and recorded in
   `reference.json` (SHA-256, pages, pdflatex version, argv, overfull/warning
   counts). A user-provided reference is never overwritten: `--regenerate`
   writes `reference-mactex2026.pdf` beside it and records its own SHA plus the
   pixel difference against the committed file. **pdflatex is an oracle only;
   it is never in the product path.**
3. **Producers** (fresh process per fixture, one `compile` request on stdin,
   180 s timeout): `compiler` = `flashtex-compiler`, `render` =
   `flashtex-render --v2 <list>`. Status, page count, every diagnostic with
   its source span, wall time and the stderr tail are kept; a crash is reported
   as `no_reply` with exit code and panic line.
4. **PDF routes**: `render` → `flashtex-pdf-exact from-v2 <list> --out …
   --font-dir apps/mac/Fonts` (the same invocation as the Mac shell's
   `ExactPDFExport.swift`); `compiler` → `flashtex-pdf <compile_result> --out …
   --verify` (runtime-v1 text items, base-14 fonts — a much weaker route,
   reported as such).
5. **Pixels**: reference and producer PDFs rasterised to 8-bit grey PGM at
   144 dpi with `pdftoppm` if present, else Ghostscript `gs -sDEVICE=pgmraw`
   (this Mac has `gs` 10.08.0 and no `pdftoppm`; the report names the one
   used). Per page: differing pixels, fraction, max grey delta, ink pixels on
   both sides; page-count and size mismatches are reported, not compared.
6. **Constructs**: a tokenizer (not a parser) over every `.tex` source yields
   control sequences (star kept), `\begin{env}` environments, packages with
   their options, the class with its options, `$`/`$$` and `\[`/`\(`, and the
   characters `& ~ ^ _ -- --- `` ` plus non-ASCII. Macros defined in the
   document (`\newcommand`, `\def`, `\DeclareMathOperator`, `\newtheorem`, …)
   are flagged `(user)`. Per producer each construct is `unsupported` (a
   diagnostic names it and says not supported / not implemented / requires),
   `recovered` (named with other wording, or a short diagnostic span covers an
   occurrence, or the macro call whose expansion produced the diagnostic), or
   `no-diagnostic` (the producer said nothing — **not** proof it renders like
   pdfLaTeX).
7. **Ranked list**: diagnostics normalised (numbers → `<N>`), counted across
   the corpus per producer, ranked by the larger count, each with severity,
   owning crate (see below), fixtures and up to six exact `path:start-end`
   spans with a source excerpt. The Commander-designated first item
   (`\subsection requires a braced argument`, issue #2) is pinned to rank 1.

## Ownership used in the report

Per `coordination/authority.json` and the crate READMEs: `crates/compiler`
(parser/expansion, including the math parser `src/math.rs`) = Commander/main;
`crates/math-layout` (math boxes) = FT-020; `crates/render-pipeline`
(paragraph/page typesetting, TFM/OpenType producer) = mac-claude-a;
`crates/pdf` = mac-pdf. The `compiler` producer on main emits diagnostics
without a `code`, so the message family decides; the `render` worker's codes
decide the rest. No second parser/expansion layer is suggested anywhere:
parser gaps are routed to the compiler.

## Helper misbehaviour found by the first run (report, not patch)

`flashtex-render` from `origin/agent/mac-render-pipeline/unified` @ `9aaec57a`
panics (exit 101, no `compile_result` line) on any paragraph whose last item
is a forced line break. Minimal reproduction, one request on stdin:

```
\documentclass{article}
\begin{document}
Hello \\

\end{document}
```

```
thread 'main' panicked at vendor/paragraph-layout/src/linebreak.rs:988:21:
slice index starts at 6 but ends at 5
```

`Hello \\ world` and `A & B` (no trailing `\\`) do not panic. In the corpus
this is triggered by `tabular`/`tabular*` bodies (typeset as plain text
because the environment is not implemented) whose rows end in `\\`:
`fixtures/real-world/cv` and `fixtures/real-world/article-twocolumn`. Owner:
`crates/paragraph-layout` (vendored into render-pipeline). The Mac shell's
crash/restart path is what a user would see on every keystroke in such a
document.

## Honest limits

- Pixel differences on `recovered` documents are large by construction (every
  document in the corpus is `recovered` today). They are printed, never ranked,
  and never summarised into a score.
- The committed `HW1-reference.pdf` is the user's own pdfLaTeX output and is
  byte-immutable. MacTeX 2026 does not reproduce it byte-for-byte
  (`reference-mactex2026.pdf`, 5113 / 704 / 2402 differing pixels on its three
  pages at 144 dpi — a different TeX Live build), so both are kept and named.
- The tokenizer counts constructs, it does not expand macros: `\mathbb` appears
  4 times in HW1.tex but its diagnostic fires 11 times through `\Z`, `\R`, …
  The construct table shows both numbers.
- `no-diagnostic` is silence, not support.
- Rasterisation uses Ghostscript's anti-aliasing; an exact-equality claim would
  need the `tools/raster-compare` comparator with provenance, which this harness
  does not attempt.
