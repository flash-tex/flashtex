# Reference comparison: flashtex-math-layout vs pdfTeX

## Declaration

| item | value |
| --- | --- |
| Reference engine | pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026), BasicTeX at `/Library/TeX/texbin/pdflatex`, format `pdflatex 2026.3.1`; used **only** as an oracle in a scratch directory, never in the product path |
| Reference document | `docs/oracle/compare.tex` — `article` 10pt, `\displaystyle`, `\pdfcompresslevel=0` so the page stream is readable |
| Reference fonts | Computer Modern Type 1 (`cmr10`, `cmr7`, `cmmi10`, `cmmi7`, `cmsy10`, `cmex10`, subsetted by pdfTeX); metrics from the `cm` TFM files of the same TeX Live |
| Ours | `flashtex-math-layout` at this revision with `CmMathMetrics::latex_10pt()`, `Style::DISPLAY`; runs emitted by `cargo run --example emit_runs` |
| Units / DPI | vector comparison in PDF points (bp, 1/72 in); no rasterisation, so no DPI applies. Ours is converted from TeX points once with 72/72.27 |
| Extraction | (1) page content stream parsed by `tools/oracle_compare.py`: glyph origins from `Tf`/`Td`/`TJ` with the PDF's own `/Widths`, rules from the stroked `w … m … l S` segments pdfTeX draws for thin rules; (2) Apple PDFKit per-character selection boxes from `tools/oracle_glyphs.swift` (macOS 26.3.1) as an independent cross-check of glyph left edges and advances |
| Alignment | per formula, ours is translated so its first glyph's origin coincides with the same glyph's origin in the reference; every other glyph/rule is then compared directly. No scaling or fitting |
| Overlays | not produced (optional in the assignment); the tables below are the quantitative evidence |

Fonts are the same on both sides, so the comparison is absolute, not just
relative: Computer Modern is what the reference uses and what the adapter
embeds. (A Times-based comparison would only be meaningful as ratios; the
Times adapter is an approximation and is not compared here.)

## Headline

Largest absolute deviation over all glyph origins and rule geometry in the
three formulas: **0.003 bp** (pdfTeX writes three decimals, so this is the
reference's own rounding). All 20 glyphs and 4 rules were matched by font,
glyph id, position and, for rules, length and thickness. The comparison
found no missing or extra glyphs and no limitation reports.

| formula | LaTeX | glyphs | rules | max abs Δ (bp) |
| --- | --- | --- | --- | --- |
| A | `\frac{\frac{a}{b}}{c}=1` | 5 | 2 | 0.003 |
| B | `\sqrt{x}+\left(\frac{a}{b}\right)` | 7 | 2 | 0.003 |
| C | `\sum_{i=1}^{n}x_i^2` | 8 | 0 | 0.001 |

What that covers: stacked fraction numerator/denominator shifts and both rule
positions (axis-centred, 0.398 bp thick), Inner–Rel–Ord spacing, radical sign
choice/raise and overbar, the 12pt `cmex10` parenthesis selection and axis
centring, display-size `\sum` with limits above and below, and sub/superscript
shifts on a character nucleus.

## Reproduce

```sh
# in a scratch directory (oracle only)
/Library/TeX/texbin/pdflatex -interaction=nonstopmode -output-directory=. crates/math-layout/docs/oracle/compare.tex
swiftc -O -o oracle_glyphs crates/math-layout/tools/oracle_glyphs.swift && ./oracle_glyphs compare.pdf > pdfkit.json
cargo run -q --manifest-path crates/math-layout/Cargo.toml --example emit_runs > runs.json
python3 crates/math-layout/tools/oracle_compare.py compare.pdf runs.json pdfkit.json
```

`docs/oracle/` keeps the inputs and the captured outputs (`runs.json`,
`pdfkit.json`, `report.md`) from the run recorded below.

## PDFKit cross-check notes

PDFKit reports left edges identical to the content-stream origins (Δ = 0.000)
for every glyph it can select. Two caveats are visible in the tables:
`cmex10` glyphs (the delimiters and the display `\sum`) have no Unicode
mapping in the subset font, so PDFKit does not expose them as characters;
and PDFKit merges some adjacent characters into one selection box (`+` and `b` with
the following delimiter, `2` with `i`, `n` with `\sum`), which widens those
boxes beyond the single-glyph advance. Neither affects the origin comparison.

## Full report (captured 2026-09-12)

Δ columns are ours − reference in bp. "ours baseline"/"ref baseline" are
relative to the anchor glyph's baseline, positive up.
Largest absolute deviation over all formulas: 0.003 bp

### Formula A

| glyph | font | ours x | ref x | Δx | ours baseline | ref baseline | Δy |
|---|---|---|---|---|---|---|---|
| a | cmmi7 gid 97 | 0.000 | 0.000 | +0.000 | -0.000 | 0.000 | -0.000 |
| b | cmmi7 gid 98 | 0.409 | 0.409 | -0.000 | -7.358 | -7.358 | -0.000 |
| c | cmmi10 gid 99 | 0.005 | 0.005 | +0.000 | -18.077 | -18.077 | -0.000 |
| = | cmr10 gid 61 | 9.480 | 9.480 | -0.000 | -11.243 | -11.243 | -0.000 |
| 1 | cmr10 gid 49 | 19.996 | 19.999 | -0.003 | -11.243 | -11.243 | -0.000 |

| rule | ours x | ref x | Δx | ours centre y | ref centre y | Δy | ours w | ref w | Δw | ours h | ref h | Δh |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 0.000 | 0.000 | +0.000 | -1.432 | -1.432 | +0.000 | 4.321 | 4.321 | +0.000 | 0.398 | 0.398 | +0.000 |
| 1 | -1.196 | -1.195 | -0.001 | -8.753 | -8.752 | -0.001 | 6.712 | 6.712 | +0.000 | 0.398 | 0.398 | +0.000 |

Largest absolute deviation in formula A: 0.003 bp; limitations: none

PDFKit cross-check (content-stream x vs PDFKit box left edge, same page):
| ref glyph | stream x | PDFKit left | Δ | PDFKit width | stream advance | Δ |
|---|---|---|---|---|---|---|
| cmmi7 97 | 188.906 | 188.906 | +0.000 | 4.322 | 4.322 | +0.000 |
| cmmi7 98 | 189.315 | 189.315 | +0.000 | 3.504 | 3.504 | +0.000 |
| cmmi10 99 | 188.911 | 188.911 | +0.000 | 4.312 | 4.312 | -0.000 |
| cmr10 61 | 198.386 | 198.386 | +0.000 | 7.749 | 7.749 | -0.000 |
| cmr10 49 | 208.905 | 208.905 | +0.000 | 4.981 | 4.981 | +0.000 |

### Formula B

| glyph | font | ours x | ref x | Δx | ours baseline | ref baseline | Δy |
|---|---|---|---|---|---|---|---|
| √ | cmsy10 gid 112 | 0.000 | 0.000 | +0.000 | -0.000 | 0.000 | -0.000 |
| x | cmmi10 gid 120 | 8.302 | 8.302 | +0.000 | -7.662 | -7.662 | -0.000 |
| + | cmr10 gid 43 | 16.210 | 16.207 | +0.003 | -7.662 | -7.662 | -0.000 |
| ( | cmex10 gid 16 | 26.173 | 26.173 | -0.000 | 3.396 | 3.396 | +0.000 |
| a | cmmi10 gid 97 | 33.318 | 33.318 | +0.000 | -0.922 | -0.923 | +0.001 |
| b | cmmi10 gid 98 | 33.813 | 33.813 | +0.000 | -14.496 | -14.496 | -0.000 |
| ) | cmex10 gid 17 | 39.780 | 39.780 | -0.000 | 3.396 | 3.396 | +0.000 |

| rule | ours x | ref x | Δx | ours centre y | ref centre y | Δy | ours w | ref w | Δw | ours h | ref h | Δh |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 8.302 | 8.302 | +0.000 | 0.199 | 0.199 | +0.000 | 5.694 | 5.694 | -0.000 | 0.398 | 0.398 | +0.000 |
| 1 | 33.318 | 33.318 | +0.000 | -5.172 | -5.172 | +0.000 | 5.266 | 5.266 | +0.000 | 0.398 | 0.398 | +0.000 |

Largest absolute deviation in formula B: 0.003 bp; limitations: none

PDFKit cross-check (content-stream x vs PDFKit box left edge, same page):
| ref glyph | stream x | PDFKit left | Δ | PDFKit width | stream advance | Δ |
|---|---|---|---|---|---|---|
| cmsy10 112 | 186.100 | 186.100 | +0.000 | 8.302 | 8.302 | +0.000 |
| cmmi10 120 | 194.402 | 194.402 | -0.000 | 5.694 | 5.694 | +0.000 |
| cmr10 43 | 202.307 | 202.307 | +0.000 | 15.915 | 7.749 | +8.166 |
| cmmi10 97 | 219.418 | 219.418 | +0.000 | 5.266 | 5.266 | +0.000 |
| cmmi10 98 | 219.913 | 219.913 | +0.000 | 11.917 | 4.276 | +7.641 |

### Formula C

| glyph | font | ours x | ref x | Δx | ours baseline | ref baseline | Δy |
|---|---|---|---|---|---|---|---|
| n | cmmi7 gid 110 | 0.000 | 0.000 | +0.000 | -0.000 | 0.000 | -0.000 |
| ∑ | cmex10 gid 88 | -4.733 | -4.733 | +0.000 | -2.989 | -2.989 | +0.000 |
| i | cmmi7 gid 105 | -3.991 | -3.990 | -0.001 | -24.208 | -24.208 | +0.000 |
| = | cmr7 gid 61 | -1.172 | -1.171 | -0.001 | -24.208 | -24.208 | +0.000 |
| 1 | cmr7 gid 49 | 4.944 | 4.945 | -0.001 | -24.208 | -24.208 | +0.000 |
| x | cmmi10 gid 120 | 11.318 | 11.318 | +0.000 | -12.453 | -12.454 | +0.001 |
| 2 | cmr7 gid 50 | 17.012 | 17.012 | +0.000 | -8.340 | -8.340 | +0.000 |
| i | cmmi7 gid 105 | 17.012 | 17.012 | +0.000 | -14.916 | -14.917 | +0.001 |

| rule | ours x | ref x | Δx | ours centre y | ref centre y | Δy | ours w | ref w | Δw | ours h | ref h | Δh |
|---|---|---|---|---|---|---|---|---|---|---|---|---|

Largest absolute deviation in formula C: 0.001 bp; limitations: none

PDFKit cross-check (content-stream x vs PDFKit box left edge, same page):
| ref glyph | stream x | PDFKit left | Δ | PDFKit width | stream advance | Δ |
|---|---|---|---|---|---|---|
| cmmi7 110 | 190.971 | 190.971 | +0.000 | 9.658 | 4.925 | +4.733 |
| cmmi7 105 | 186.981 | 186.981 | +0.000 | 2.819 | 2.819 | +0.000 |
| cmr7 61 | 189.800 | 189.800 | +0.000 | 6.116 | 6.116 | +0.000 |
| cmr7 49 | 195.916 | 195.916 | +0.000 | 3.972 | 3.972 | -0.000 |
| cmmi10 120 | 202.289 | 202.289 | +0.000 | 5.694 | 5.694 | +0.000 |
| cmr7 50 | 207.983 | 207.983 | +0.000 | 2.819 | 3.972 | -1.153 |
| cmmi7 105 | 207.983 | 207.983 | +0.000 | 2.819 | 2.819 | +0.000 |
