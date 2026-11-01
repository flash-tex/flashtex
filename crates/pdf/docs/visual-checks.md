# Native visual checks of `flashtex-pdf` output

Performed on `mac-m1max-a` on 2026-09-12 by rasterising PDFs with macOS's own
PDF engine (`/usr/bin/sips -s format png`, 72 dpi, then cropping and
upscaling the text region for inspection) and looking at the result. These are
descriptions of what the raster actually shows, including the defects. They
are not automated assertions; the automated checks are in `tests/render.rs`.

Regenerate any of them with:

```sh
/usr/bin/sips -s format png FILE.pdf --out page.png
/usr/bin/sips -c HEIGHT WIDTH --cropOffset ROW COL page.png --out crop.png
/usr/bin/sips -z 4*HEIGHT 4*WIDTH crop.png --out crop.png
```

## 1. `math-inline-display` corpus case, compiler `de1020c`, default fonts

Input: `tests/tex-corpus/cases/math-inline-display/main.tex`
(`Inline $x_1^2 + y$ ends.` and a display `\[ \frac{a+b}{c} = d \]`),
compiled by the compiler-foundation branch at `de1020c`, rendered with base-14
fonts only. Compiler diagnostics: `\documentclass` unsupported (its argument
is typeset as text).

What is visible:

- First line: `article Inline x²₁+y ends.` — the word `article` is the leaked
  `\documentclass` argument (compiler recovery, expected). The `x` carries a
  superscript `2` and a subscript `1` stacked at the same x, both at 8.4 pt,
  clearly above and below the 12 pt baseline. `+y` follows with no math
  spacing around `+` (compiler spacing, not a PDF issue). Everything is
  Times-Roman, black, on white.
- Second block, centred at about x = 291–320 pt: `a+b` at 8.4 pt above a
  drawn horizontal rule, `c` at 8.4 pt below it, then `=d` at 12 pt to the
  right with the `=` sitting at rule height. The rule is a filled rectangle
  from the `────` item (4 × 0.5 × 8.4 = 16.8 pt wide, 0.72 pt thick); it
  renders as a crisp line, not as box-drawing glyphs. No `?` anywhere.
- Defects seen: the numerator `a+b` touches the rule with very little gap,
  and `=d` has no thin space after the fraction. Both come from the
  compiler's math spacing constants; the PDF places items exactly where told.

## 2. `unicode-literals` corpus case, compiler `origin/main` `342e1e0`, with and without embedding

Input: `Café naïve — 東京.` / `After Unicode.` Compiled by the compiler on
`origin/main`, rendered twice: default fonts, and
`--embed-font /System/Library/Fonts/Supplemental/Arial Unicode.ttf`.

Default fonts, what is visible: `article Café naïve —??. After Unicode.` —
`é` and `ï` render correctly from WinAnsi, the em dash is a real dash, and
the two ideographs are two `?` characters followed by the period, exactly as
the writer warned (`U+6771`, `U+4EAC`). Word spacing is uneven (the
compiler's placeholder widths), but nothing overlaps.

With Arial Unicode embedded, what is visible: `article Café naïve —東京After
Unicode.` — the ideographs **render as real glyphs** from the embedded
subset, serif Latin text next to sans CJK (different font families, as
documented). **Defect:** the period after `京` is not visible and `After`
starts immediately after `京`. Cause: the compiler laid the line out assuming
each unknown glyph is 0.5 em (`metrics.rs` `DEFAULT_ADVANCE_UNITS = 500`), so
it placed `After` at x = 208.8 pt, but real ideographs are 1 em wide, so
`東京.` actually extends to about 214 pt and overlaps it. The PDF writer
faithfully placed every item where the compiler said; the compiler does not
know the embedded font's advances. This is the same "positions come from the
compiler" limitation stated in the README, made visible for the first time by
a font with wide glyphs. Fixing it needs either the compiler to consult the
same font's metrics or a contract-level font/width exchange; it is not
something the PDF writer can correct without re-flowing text, which it
deliberately does not do.

## 3. Issue #9 reproduction `$\frac{a}{b}+\alpha+\sqrt{x}$`, compiler `de1020c`, default fonts

Input: `crates/pdf/tests/fixtures/math-compile-result.json`.

What is visible: a small `a` above a drawn fraction bar above a small `b`,
then `+α+√x` on the main baseline. `α` is the Symbol font's alpha (an upright
Greek alpha, visibly a different design from Times), `√` is Symbol's radical
sign — a bare check-shaped radical **with no overbar** over the `x`, because
base-14 Symbol has no stretchy radical rule and the compiler emits `√` as a
plain character. No `?` anywhere. The fraction bar is 8.4 pt wide and hugs
the baseline of its item, as the tests assert.

Defects seen: no overbar on the radical; `+` has no math spacing; the
fraction is tight. All are upstream (contract has no rule/path primitive
beyond the U+2500 convention; Symbol is a placeholder for a real math font).

## 4. Two-line embedding sample, both system fonts

Input: `Latin café — Greek αβγ — Cyrillic жизнь — CJK 中文 — ℝ ∫ 😀` and
`Ǆǅ Ŵŷ Ȩ ẞ ﬁ ﬂ (composite glyphs: é ñ ü ő)` as two 14 pt items.

With Arial Unicode: every script renders — Greek from Symbol, Cyrillic, CJK,
`ℝ`, `Ǆǅ`, `Ŵŷ`, `ő` (composite glyphs, so the subsetter's component
resolution works) and the `ﬁ ﬂ` ligature glyphs from the embedded subset;
`?` appears exactly for `😀`, `Ȩ`, and `ẞ`, the three characters the warning
listed. With Times New Roman: same, except `中文` and `ℝ` are `?` and `Ȩ`
renders, again exactly matching that run's warnings. Serif/sans mixing is
visible in the Arial case; the Times New Roman case is visually uniform.

## Summary

- Nothing was lost in any check; every `?` on the page corresponds to a
  warning naming the code point.
- Fraction rules, scripts, Greek, and radical signs render natively without
  substitution.
- Embedded subsets render correctly in macOS's PDF engine, including
  composite glyphs.
- The one new defect found is layout, not encoding: the compiler's 0.5 em
  placeholder for glyphs it has no metrics for collides with real 1 em CJK
  glyphs once a real font is embedded. Reported here for the compiler and
  contract owners; not fixable in `crates/pdf` without re-flowing text.
