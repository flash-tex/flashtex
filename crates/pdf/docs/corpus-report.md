# Corpus regression: `tests/tex-corpus` → compiler → `flashtex-pdf`

Generated on `mac-m1max-a` (Darwin arm64) on 2026-09-12 by
`crates/pdf/scripts/corpus_report.py`. Every case in `tests/tex-corpus`
(origin/main `342e1e0`, 14 cases) was emitted with the corpus's own
`validate.py --emit-request`, compiled by the Rust compiler binary named in
each section, and rendered by this crate's `flashtex-pdf --verify` (structural
self-check of header, xref offsets, and trailer), then opened with macOS
`/usr/bin/sips`. No TeX engine was involved at any step. Artifacts were written
to a local scratch directory and are not committed; the tables are the record.

How to reproduce:

```sh
cargo build --release --manifest-path crates/compiler/Cargo.toml
cargo build --release --manifest-path crates/pdf/Cargo.toml
python3 crates/pdf/scripts/corpus_report.py --corpus tests/tex-corpus \
    --compiler crates/compiler/target/release/flashtex-compiler \
    --pdf crates/pdf/target/release/flashtex-pdf --embed-font auto
```

## What the results mean, before the tables

- **Compiler on `origin/main` (`342e1e0`) is the FT-002 foundation.** It has
  no `\documentclass`, no math, no macros, no `\input`. Every case therefore
  compiles as `recovered` with explicit diagnostics and provisional text. That
  is a compiler limitation recorded honestly here, not a PDF result. The PDF
  writer's job is to reproduce whatever positioned text the compiler emitted,
  and it did so for 14/14 cases with a valid structure that `sips` opens.
- **Default fonts (base-14 Times-Roman + Symbol): 12/14 cases with zero
  warnings.** The two warnings are the CJK literals in `unicode-literals`
  (`東京`) and `literal-source-map` (`尾`), written as `?` and named with their
  code points. `Café naïve —` and `éé` are WinAnsi and render as text.
- **`--embed-font auto` picks Times New Roman on this Mac**, which has no CJK
  glyphs, so those two cases still warn (now naming the embedded font). With
  `--embed-font "/System/Library/Fonts/Supplemental/Arial Unicode.ttf"` all
  14 cases render with **zero warnings**; CJK is embedded as a subset.
  Whether a system font may be embedded in a redistributed PDF is the user's
  licensing decision; see `crates/pdf/README.md`.
- **Compiler at `de1020c`** (branch `agent/claude/compiler-foundation`, not yet
  on main) is included for comparison because it is the revision issue #9 was
  filed against: it typesets `math-inline-display` as a real fraction
  (`────` rule item, which this crate draws as a rectangle) with scripts, and
  the PDF side is warning-free there too.
- Nothing in this report asserts visual correctness of layout; it asserts
  that no text was lost between the compiler's output and the PDF, and that
  every substitution was reported. Rasterised visual checks are in
  `visual-checks.md` alongside this file.

## Table A — compiler `origin/main` `342e1e0`, default fonts vs `--embed-font auto`

| case | compiler status | diags | pages | items | PDF default fonts | PDF --embed-font auto |
|---|---|---|---|---|---|---|
| plain-paragraphs | recovered | 1 | 1 | 5 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| macro-arguments | recovered | 4 | 1 | 7 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| macro-scope | recovered | 8 | 1 | 3 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| unicode-literals | recovered | 1 | 1 | 7 | exit 0, 1 warning(s), opens | exit 0, 2 warning(s), opens |
| math-inline-display | recovered | 4 | 1 | 11 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| included-file | recovered | 3 | 1 | 4 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| include-scope | recovered | 7 | 1 | 3 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| comments-escapes | recovered | 1 | 1 | 17 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| unknown-command | recovered | 2 | 1 | 3 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| unclosed-group | recovered | 2 | 1 | 5 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| extra-closing-group | recovered | 2 | 1 | 3 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| missing-include | recovered | 2 | 1 | 4 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| literal-source-map | recovered | 1 | 1 | 4 | exit 0, 1 warning(s), opens | exit 0, 2 warning(s), opens |
| tikz-required | recovered | 4 | 1 | 7 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |

## Table B — compiler `origin/main` `342e1e0`, default fonts vs Arial Unicode

| case | compiler status | diags | pages | items | PDF default fonts | PDF --embed-font Arial Unicode.ttf |
|---|---|---|---|---|---|---|
| plain-paragraphs | recovered | 1 | 1 | 5 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| macro-arguments | recovered | 4 | 1 | 7 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| macro-scope | recovered | 8 | 1 | 3 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| unicode-literals | recovered | 1 | 1 | 7 | exit 0, 1 warning(s), opens | exit 0, 0 warning(s), opens |
| math-inline-display | recovered | 4 | 1 | 11 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| included-file | recovered | 3 | 1 | 4 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| include-scope | recovered | 7 | 1 | 3 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| comments-escapes | recovered | 1 | 1 | 17 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| unknown-command | recovered | 2 | 1 | 3 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| unclosed-group | recovered | 2 | 1 | 5 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| extra-closing-group | recovered | 2 | 1 | 3 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| missing-include | recovered | 2 | 1 | 4 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| literal-source-map | recovered | 1 | 1 | 4 | exit 0, 1 warning(s), opens | exit 0, 0 warning(s), opens |
| tikz-required | recovered | 4 | 1 | 7 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |

## Table C — compiler `de1020c` (compiler-foundation branch), default fonts vs Arial Unicode

| case | compiler status | diags | pages | items | PDF default fonts | PDF --embed-font Arial Unicode.ttf |
|---|---|---|---|---|---|---|
| plain-paragraphs | recovered | 1 | 1 | 5 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| macro-arguments | recovered | 4 | 1 | 7 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| macro-scope | recovered | 8 | 1 | 3 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| unicode-literals | recovered | 1 | 1 | 7 | exit 0, 1 warning(s), opens | exit 0, 0 warning(s), opens |
| math-inline-display | recovered | 1 | 1 | 15 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| included-file | recovered | 3 | 1 | 4 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| include-scope | recovered | 7 | 1 | 3 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| comments-escapes | recovered | 1 | 1 | 17 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| unknown-command | recovered | 2 | 1 | 3 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| unclosed-group | recovered | 2 | 1 | 5 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| extra-closing-group | recovered | 2 | 1 | 3 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| missing-include | recovered | 2 | 1 | 4 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |
| literal-source-map | recovered | 1 | 1 | 4 | exit 0, 1 warning(s), opens | exit 0, 0 warning(s), opens |
| tikz-required | recovered | 4 | 1 | 7 | exit 0, 0 warning(s), opens | exit 0, 0 warning(s), opens |

## Per-case detail (Table A run)

### plain-paragraphs

Compiler: status `recovered`, 1 diagnostic(s), 1 page(s), 5 item(s).
- error: \documentclass is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)

Text items: `article` `First` `paragraph.` `Second` `paragraph.`

PDF (default fonts): exit 0, 0 warning(s), opens

PDF (`--embed-font auto`): exit 0, 0 warning(s), opens
- note: embedding subset of TimesNewRomanPSMT from /System/Library/Fonts/Supplemental/Times New Roman.ttf

### macro-arguments

Compiler: status `recovered`, 4 diagnostic(s), 1 page(s), 7 item(s).
- error: \documentclass is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \newcommand is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \pair is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \pair is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)

Text items: `article` `[2]` `#1` `then` `#2` `left` `right`

PDF (default fonts): exit 0, 0 warning(s), opens

PDF (`--embed-font auto`): exit 0, 0 warning(s), opens
- note: embedding subset of TimesNewRomanPSMT from /System/Library/Fonts/Supplemental/Times New Roman.ttf

### macro-scope

Compiler: status `recovered`, 8 diagnostic(s), 1 page(s), 3 item(s).
- error: \documentclass is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \newcommand is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \word is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \word is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \renewcommand is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \word is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \word is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \word is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)

Text items: `article` `outer` `inner`

PDF (default fonts): exit 0, 0 warning(s), opens

PDF (`--embed-font auto`): exit 0, 0 warning(s), opens
- note: embedding subset of TimesNewRomanPSMT from /System/Library/Fonts/Supplemental/Times New Roman.ttf

### unicode-literals

Compiler: status `recovered`, 1 diagnostic(s), 1 page(s), 7 item(s).
- error: \documentclass is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)

Text items: `article` `Café` `naïve` `—` `東京.` `After` `Unicode.`

PDF (default fonts): exit 0, 1 warning(s), opens
- warning: page 1: item 4 "東京.": '東' (U+6771), '京' (U+4EAC) not representable in WinAnsiEncoding or Symbol; written as '?'

PDF (`--embed-font auto`): exit 0, 2 warning(s), opens
- note: embedding subset of TimesNewRomanPSMT from /System/Library/Fonts/Supplemental/Times New Roman.ttf
- warning: embedded font TimesNewRomanPSMT (/System/Library/Fonts/Supplemental/Times New Roman.ttf) has no glyph for '京' (U+4EAC), '東' (U+6771); those characters fall back to '?'
- warning: page 1: item 4 "東京.": '東' (U+6771), '京' (U+4EAC) not representable in WinAnsiEncoding, Symbol, or embedded BJACAS+TimesNewRomanPSMT; written as '?'

### math-inline-display

Compiler: status `recovered`, 4 diagnostic(s), 1 page(s), 11 item(s).
- error: \documentclass is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: math mode is not implemented in this version (recovery: skipped the math shift character; no math was typeset)
- error: math mode is not implemented in this version (recovery: skipped the math shift character; no math was typeset)
- error: \frac is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)

Text items: `article` `Inline` `x_1^2` `+` `y` `ends.` `[` `a+b` `c` `=d` `]`

PDF (default fonts): exit 0, 0 warning(s), opens

PDF (`--embed-font auto`): exit 0, 0 warning(s), opens
- note: embedding subset of TimesNewRomanPSMT from /System/Library/Fonts/Supplemental/Times New Roman.ttf

### included-file

Compiler: status `recovered`, 3 diagnostic(s), 1 page(s), 4 item(s).
- error: \documentclass is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \input is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- warning: 2 documents were supplied; this version compiles only the entry document (recovery: compiled the entry document alone)

Text items: `article` `Before.` `parts/section` `After.`

PDF (default fonts): exit 0, 0 warning(s), opens

PDF (`--embed-font auto`): exit 0, 0 warning(s), opens
- note: embedding subset of TimesNewRomanPSMT from /System/Library/Fonts/Supplemental/Times New Roman.ttf

### include-scope

Compiler: status `recovered`, 7 diagnostic(s), 1 page(s), 3 item(s).
- error: \documentclass is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \newcommand is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \word is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \input is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \word is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \word is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- warning: 2 documents were supplied; this version compiles only the entry document (recovery: compiled the entry document alone)

Text items: `article` `outer` `local`

PDF (default fonts): exit 0, 0 warning(s), opens

PDF (`--embed-font auto`): exit 0, 0 warning(s), opens
- note: embedding subset of TimesNewRomanPSMT from /System/Library/Fonts/Supplemental/Times New Roman.ttf

### comments-escapes

Compiler: status `recovered`, 1 diagnostic(s), 1 page(s), 17 item(s).
- error: \documentclass is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)

Text items: `article` `A` `B` `Price` `$` `5;` `50` `%` `;` `A` `&` `B;` `C` `_` `D;` `#` `1.`

PDF (default fonts): exit 0, 0 warning(s), opens

PDF (`--embed-font auto`): exit 0, 0 warning(s), opens
- note: embedding subset of TimesNewRomanPSMT from /System/Library/Fonts/Supplemental/Times New Roman.ttf

### unknown-command

Compiler: status `recovered`, 2 diagnostic(s), 1 page(s), 3 item(s).
- error: \documentclass is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \flashtexUnknownCommand is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)

Text items: `article` `Before.` `After.`

PDF (default fonts): exit 0, 0 warning(s), opens

PDF (`--embed-font auto`): exit 0, 0 warning(s), opens
- note: embedding subset of TimesNewRomanPSMT from /System/Library/Fonts/Supplemental/Times New Roman.ttf

### unclosed-group

Compiler: status `recovered`, 2 diagnostic(s), 1 page(s), 5 item(s).
- error: \documentclass is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: unmatched '{' — group never closed (recovery: treated the rest of the document as part of the group)

Text items: `article` `Before.` `inside` `Still` `inside.`

PDF (default fonts): exit 0, 0 warning(s), opens

PDF (`--embed-font auto`): exit 0, 0 warning(s), opens
- note: embedding subset of TimesNewRomanPSMT from /System/Library/Fonts/Supplemental/Times New Roman.ttf

### extra-closing-group

Compiler: status `recovered`, 2 diagnostic(s), 1 page(s), 3 item(s).
- error: \documentclass is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: unmatched '}' — no group is open here (recovery: ignored the stray brace and continued)

Text items: `article` `Before.` `After.`

PDF (default fonts): exit 0, 0 warning(s), opens

PDF (`--embed-font auto`): exit 0, 0 warning(s), opens
- note: embedding subset of TimesNewRomanPSMT from /System/Library/Fonts/Supplemental/Times New Roman.ttf

### missing-include

Compiler: status `recovered`, 2 diagnostic(s), 1 page(s), 4 item(s).
- error: \documentclass is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \input is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)

Text items: `article` `Before.` `missing-file` `After.`

PDF (default fonts): exit 0, 0 warning(s), opens

PDF (`--embed-font auto`): exit 0, 0 warning(s), opens
- note: embedding subset of TimesNewRomanPSMT from /System/Library/Fonts/Supplemental/Times New Roman.ttf

### literal-source-map

Compiler: status `recovered`, 1 diagnostic(s), 1 page(s), 4 item(s).
- error: \documentclass is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)

Text items: `article` `éé` `UniqueAnchor` `尾`

PDF (default fonts): exit 0, 1 warning(s), opens
- warning: page 1: item 3 "尾": '尾' (U+5C3E) not representable in WinAnsiEncoding or Symbol; written as '?'

PDF (`--embed-font auto`): exit 0, 2 warning(s), opens
- note: embedding subset of TimesNewRomanPSMT from /System/Library/Fonts/Supplemental/Times New Roman.ttf
- warning: embedded font TimesNewRomanPSMT (/System/Library/Fonts/Supplemental/Times New Roman.ttf) has no glyph for '尾' (U+5C3E); those characters fall back to '?'
- warning: page 1: item 3 "尾": '尾' (U+5C3E) not representable in WinAnsiEncoding, Symbol, or embedded BJACAS+TimesNewRomanPSMT; written as '?'

### tikz-required

Compiler: status `recovered`, 4 diagnostic(s), 1 page(s), 7 item(s).
- error: \documentclass is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- error: \usepackage is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)
- warning: environment 'tikzpicture' is not implemented; its body is typeset as plain text (recovery: typeset the body without the environment's formatting)
- error: \draw is not supported by this compiler version (recovery: skipped the command; any braced argument was typeset as plain text)

Text items: `article` `tikz` `(0,0)` `--` `(1,0)` `--` `(1,1);`

PDF (default fonts): exit 0, 0 warning(s), opens

PDF (`--embed-font auto`): exit 0, 0 warning(s), opens
- note: embedding subset of TimesNewRomanPSMT from /System/Library/Fonts/Supplemental/Times New Roman.ttf

## Per-case detail for the cases that differ under Table C

### unicode-literals

Compiler: status `recovered`, 1 diagnostic(s), 1 page(s), 7 item(s).
- error: \documentclass is not supported by this compiler version; unrestricted TeX math mode is not implemented (recovery: skipped the command; any braced argument was typeset as plain text)

Text items: `article` `Café` `naïve` `—` `東京.` `After` `Unicode.`

PDF (default fonts): exit 0, 1 warning(s), opens
- warning: page 1: item 4 "東京.": '東' (U+6771), '京' (U+4EAC) not representable in WinAnsiEncoding or Symbol; written as '?'

PDF (`--embed-font /System/Library/Fonts/Supplemental/Arial Unicode.ttf`): exit 0, 0 warning(s), opens
- note: embedding subset of ArialUnicodeMS from /System/Library/Fonts/Supplemental/Arial Unicode.ttf

### math-inline-display

Compiler: status `recovered`, 1 diagnostic(s), 1 page(s), 15 item(s).
- error: \documentclass is not supported by this compiler version; unrestricted TeX math mode is not implemented (recovery: skipped the command; any braced argument was typeset as plain text)

Text items: `article` `Inline` `x` `2` `1` `+` `y` `ends.` `a` `+` `b` `────` `c` `=` `d`

PDF (default fonts): exit 0, 0 warning(s), opens

PDF (`--embed-font /System/Library/Fonts/Supplemental/Arial Unicode.ttf`): exit 0, 0 warning(s), opens
- note: embedding subset of ArialUnicodeMS from /System/Library/Fonts/Supplemental/Arial Unicode.ttf
