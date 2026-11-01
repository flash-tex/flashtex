# `\mathbb` from New Computer Modern Math (lane mac-mathbb-font, 2026-09-12)

User report: "the default `\mathbb` font is weird (it should look like it does
on Overleaf / New Computer Modern, but it looks more 'sans')". pdfLaTeX's
`\mathbb` is AMS `msbm10` (serifed double-struck); Latin Modern Math's
double-struck block is the sans-like open face. `flashtex-render` now draws every
double-struck code point from `NewCMMath-Regular.otf` (sha256 `60394d35…`,
bundled in `apps/mac/Fonts`) as a secondary math face (`mathfont::BB_FONT`);
without the file it falls back to Latin Modern Math and emits one
`math_resource_profile` note (`msbm10: …`).

## HW1 (`fixtures/real-world/hw1/HW1.tex`, unmodified), page 1, gs `-r150`

Rows: pdfLaTeX reference (`reference-mactex2026.pdf`), flashtex with the font
directory (`--font-dir apps/mac/Fonts`, exact PDF via `flashtex-pdf-exact
from-v2`), flashtex before (Latin Modern Math only).

* `hw1-mathbb-crop-zgt0.png` — "variables range over $\Z_{>0}$" (HW1.tex:1893).
* `hw1-mathbb-crop-line1.png` — "Let $a, b \in \mathbb{Z}_{>0}$".

Display-list check (`--v2`): with the font directory all 11 double-struck runs
on the three pages reference the `NewCMMath-Regular` `fonts[]` entry
(`sha256 60394d357348f68cd301764fe61cc502a5858e1c4ff21b948a1d14d82586a7a2`,
1,187,476 bytes); without it they reference `LatinModernMath-Regular` and the
diagnostics carry exactly one `msbm10` profile note. Every other glyph keeps its
face and glyph id (`tests/math_symbols.rs::mathbb_paints_from_new_computer_modern_when_bundled`).

`flashtex-pdf-exact from-v2` embeds the new face without changes (258,293-byte
PDF, no diagnostics). The pipeline's `--pdf` debug writer (v1 text items through
the pdf sibling) cannot represent U+2124/U+211D/U+211A in WinAnsi/Symbol/embedded
LMRoman and writes `?` — the same pre-existing limitation it has for Latin Modern
Math's `∖` (U+2216); it does not embed either math face.
