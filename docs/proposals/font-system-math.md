# Font system, math half: `\setmathfont` over the OpenType `MATH` table

**Status:** implemented on `lane/mathfont` (mac-claude-a, 2026-09-19) under
the Commander's rulings on §5 — (1) LuaTeX's `mlist.c` is the oracle for
OpenType math geometry, (2) `\usepackage{unicode-math}` alone switches the
document to Latin Modern Math, (3) the `MathVariants`/`MathKernInfo`
parsing moved into `crates/font-engine/src/math.rs`. What landed, against
the sections below:

* §1: font-engine's `MathTable` now reads `MathVariants` (variants,
  assemblies, `min_connector_overlap`), `MathKernInfo` (`kern`) and the
  extended-shape coverage; the pipeline's private `parse_variants` is
  gone. `MathFonts` (`crates/render-pipeline/src/mathfont.rs`) is the
  provider for a document's own font (`MathFonts::named`) as well as the
  no-TFM fallback.
* §2.1: `OpenTypeExtras`, `OpenTypeMathConstants` grown by the same
  constants, `MathFontMetrics::opentype_extras` (default `None`), and the
  rules behind it in `layout.rs` — fractions/stacks, scripts, radicals with
  the degree, accents, over/underbar. Two rules the contract did not list
  turned out to matter and are in: a box nucleus's script drops use the
  *current* style's `SuperscriptBaselineDropMax`/`SubscriptBaselineDropMin`
  (LuaTeX), and the radical sign is re-boxed so its ink top meets the
  rule's top (STIX Two Math's sign sits above the baseline).
* §2.2: `KernCorner`, `MathFontMetrics::math_kern`, LuaTeX's
  `find_math_kern` in `make_scripts` (the smaller of the two heights'
  sums), applied to a character nucleus with a one-character script.
* §2.3: as unicode-math actually sets them (LuaLaTeX measured): `\mathcal`,
  `\mathfrak`, `\mathbb` are the math face's own blocks; `\mathbf`,
  `\mathsf`, `\mathit`, `\mathtt` and `\mathrm` are the *text* faces of the
  document's text family (`MathFonts::with_text_alphabets`), not the
  U+1D400… blocks the contract proposed — unicode-math's `\symbf` etc. would
  be those, and the compiler has no model for them yet. The secondary
  double-struck face is not used by a named font. Script sizes are the
  face's `ScriptPercentScaleDown`/`ScriptScriptPercentScaleDown` with its
  `ssty` alternates (script form at script size, scriptscript form below).
* §2.4: `fontspec::Settings::math` (`MathSelection`): the last preamble
  `\setmathfont` of the entry document > manifest `[fonts] math` >
  `unicode-math`'s Latin Modern Math; `\setmathfont` in the body or an
  included file is a `math_font_ignored` note; options (`Scale=`, `range=`)
  are a `fontspec_feature_ignored` note; a family that is missing or has no
  `MATH` table is a `math_font_unavailable` warning and TeX's metrics.
  The provider follows every text size (`math_fonts_at`).
* §3 holds: the corpus (70 fixtures, `scripts/render-corpus-v2.sh`) is
  byte-identical before and after.
* §4: `crates/render-pipeline/tests/setmathfont.rs` (the hw1 envelope
  below, the LuaTeX oracle numbers for Latin Modern Math and STIX Two
  Math), `crates/math-layout/tests/opentype_extras.rs`, font-engine's
  `math::tests`.

Test 1's finding, measured on hw1 (2743 glyphs, 286 from the math face):
the median delta is 0 bp; 125 glyphs move more than 0.5 bp. The owners, in
order: `\mathbb` (2.07 bp — LuaLaTeX sets ℝ from Latin Modern Math's
open-face design, 6.39 pt, where the TeX route draws msbm's 7.22 pt design
from New Computer Modern Math; the text after such an inline formula moves
with it), `\bigl(` (1.46 bp — cmex's 12 pt paren against the face's
10.95 pt variant, what LuaLaTeX picks too), a subscript alone (1.06 bp —
LuaTeX drops it `SubscriptShiftDown` 2.47 pt, TeX σ₁₆ 1.5; the table has no
σ₁₆), a display-style superscript (0.54 bp — `SuperscriptShiftUp` 3.63
against σ₁₃ 4.12), `\forall`/`\exists` advances (0.73 bp). Formulas of
letters, digits and relations agree within 0.25 bp (the italic corrections
differ: 𝐷 before `(` by 0.21 bp). So "within 0.5 bp for every glyph" does
not hold, and cannot for those constructs: they are where LuaTeX's geometry
and pdfTeX's differ by design.

Not done, deliberately: `flac` flattened accents (LuaTeX's default
`\mathflattenmode` does not apply them either: `\hat{A}` keeps the plain
hat), `\sqrt[n]` through the pipeline (the compiler drops the degree; the
placement is implemented and unit-tested in math-layout), `\int\limits` on
a symbol (the compiler does not carry `\limits` on it), `\mathbb` without
`amsfonts` (the compiler rejects it before this code runs), `\ldots` as a
single U+2026 glyph, `\symbf` and friends, `range=`/`Scale=`.

The original contract follows, unchanged.

---

Written by the S4 text lane (mac-claude-a / lane-fonts) for the owner of
`crates/math-layout` (Daniel's crate) and whoever picks up the pipeline
side. The text half — discovery, `Family::Named`, fontspec's
`\setmainfont`/`\setsansfont`/`\setmonofont`/`\newfontfamily`/`\fontspec`,
the manifest's `[fonts] text/sans/mono` — is on `lane/fonts`
(`crates/font-discovery`, `crates/render-pipeline/src/fontspec.rs`,
`fonts.rs`). This document is what the math half needs from `math-layout`,
in the crate's own types, and where the rest of it already exists.

Direction it serves: `docs/proposals/packages-fonts-manifest.md` §2.4 —
"support using any font on the computer, including for math", `unicode-math`
and `\setmathfont` keep meaning what they mean, and **TFM Computer Modern
stays the default so pdflatex parity holds when nothing is set**.

## 1. What already exists (measured on `main` `4acae1c65`)

Most of the machinery is in place; the missing piece is smaller than the
slice's name suggests.

| piece | where | state |
| --- | --- | --- |
| `MATH` table parsing: all 56 `MathConstants`, `MathGlyphInfo` italics correction and top-accent attachment | `crates/font-engine/src/math.rs` (`MathTable::parse`, `italics_correction`, `top_accent_attachment`) | done |
| `MathVariants` vertical + horizontal variants and glyph assemblies (`minConnectorOverlap`, part records, extender repeat) | `crates/render-pipeline/src/mathfont.rs` (`parse_variants`, `vertical_assembly`) — read there because font-engine does not expose them; `docs/proposals/rendering-abi.md` asks for that API | done in the pipeline |
| `OpenTypeMathConstants` → TeX σ/ξ (the LuaTeX correspondence) | `crates/math-layout/src/metrics.rs` `MathParams::from_opentype` | done, 18 constants |
| An OpenType `MathFontMetrics` provider (`MathFonts`), with `ssty` script alternates, a secondary double-struck face, Unicode math-alphanumeric glyph selection for italic letters | `crates/render-pipeline/src/mathfont.rs`, selected as `MathProvider::Otf` **only when the `lm` TFMs are missing** (`typeset::Context::math_fonts`) | done for Latin Modern Math |
| TeX's own metrics (`TexMathMetrics`: `cmsy`/`cmex`/`cmmi` TFM parameters and lig/kern, Latin Modern Math outlines) | `crates/render-pipeline/src/mathtex.rs`, `MathProvider::Tex` | the default, pdflatex parity |
| Which faces on the machine carry `MATH` | `crates/font-discovery` `FontIndex::math_fonts()` (this Mac: Latin Modern Math, STIX Two Math, Libertinus Math, IBM Plex Math, NewCM Math ×3, NewCM Sans Math) | done |
| `[fonts] math` in the manifest | `RenderOptions::fonts.math` (`crates/render-pipeline/src/lib.rs`) | carried, unread |
| `\setmathfont{X}` in a document | `crates/render-pipeline/src/fontspec.rs` reads it and emits one `math_font_not_implemented` note naming this document | read, unapplied |

So `\setmathfont{STIX Two Math}` is: *select a named face with a `MATH`
table, build `MathFonts` on it, use `MathProvider::Otf` for the document*,
plus the four gaps below.

## 2. The four gaps, and the signatures wanted from `math-layout`

### 2.1 Constants TeX's parameters cannot express

`MathParams` is Appendix G's σ₅..σ₂₂ and ξ₈..ξ₁₃. OpenType math fonts carry
parameters TeX approximates with fixed rules, and the layout must read them
when the provider is an OpenType face, or STIX/Libertinus set fractions and
scripts visibly differently from XeTeX/LuaTeX. The rules that differ
(`tex.web` §§ 743–757 vs. OpenType MATH § 6.3 and LuaTeX's `mlist.c`):

| construction | TeX rule | OpenType constant to read instead |
| --- | --- | --- |
| fraction clearance (Rule 15e) | `3ξ₈` display / `ξ₈` text | `FractionNum(Denom)DisplayStyleGapMin` / `FractionNumerator(Denominator)GapMin` |
| stack (`\atop`) clearance | `7ξ₈` / `3ξ₈` | `StackDisplayStyleGapMin` / `StackGapMin`; shifts `StackTop(Bottom)DisplayStyleShiftUp(Down)` |
| sub/superscript gap (Rule 18e) | `4ξ₈` | `SubSuperscriptGapMin`, and `SuperscriptBottomMaxWithSubscript` for the pair |
| subscript top limit (Rule 18b) | `⅘σ₅` | `SubscriptTopMax` |
| superscript bottom (Rule 18c) | `¼σ₅` | `SuperscriptBottomMin` |
| space after a script | `\scriptspace` 0.5pt | `SpaceAfterScript` |
| radical (Rule 11) | rule `ξ₈`, clearance `ξ₈` text / `ξ₈+¼σ₅` display | `RadicalRuleThickness`, `RadicalVerticalGap` / `RadicalDisplayStyleVerticalGap`, `RadicalExtraAscender`; degree: `RadicalKernBeforeDegree`, `RadicalKernAfterDegree`, `RadicalDegreeBottomRaisePercent` |
| accents (Rule 12) | skew via the skew char, base cap at `σ₅` | `AccentBaseHeight`, `FlattenedAccentBaseHeight` (select the `flac` alternate), top accent from `MathGlyphInfo` (already read) |
| overline/underline | `3ξ₈`/`ξ₈` gaps | `Overbar(Underbar)VerticalGap`, `Overbar(Underbar)RuleThickness`, `Overbar(Underbar)ExtraAscender(Descender)` |
| `\big` operators in display | one `cmex` variant | `DisplayOperatorMinHeight` (smallest variant at least this tall) |
| script sizes | LaTeX's `\DeclareMathSizes` | `ScriptPercentScaleDown` / `ScriptScriptPercentScaleDown` when `unicode-math` is loaded (see 2.4) |

Wanted in `math-layout`, additive (no existing field changes):

```rust
// crates/math-layout/src/metrics.rs
/// The OpenType constants TeX's σ/ξ cannot express; `None` from a TFM
/// provider, in which case every rule keeps its Appendix G form.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OpenTypeExtras {
    pub fraction_numerator_gap_min: f64,          // pt, already scaled
    pub fraction_num_display_style_gap_min: f64,
    pub fraction_denominator_gap_min: f64,
    pub fraction_denom_display_style_gap_min: f64,
    pub stack_gap_min: f64,
    pub stack_display_style_gap_min: f64,
    pub stack_top_display_style_shift_up: f64,
    pub stack_bottom_shift_down: f64,
    pub stack_bottom_display_style_shift_down: f64,
    pub sub_superscript_gap_min: f64,
    pub superscript_bottom_max_with_subscript: f64,
    pub subscript_top_max: f64,
    pub superscript_bottom_min: f64,
    pub space_after_script: f64,
    pub radical_rule_thickness: f64,
    pub radical_vertical_gap: f64,
    pub radical_display_style_vertical_gap: f64,
    pub radical_extra_ascender: f64,
    pub radical_kern_before_degree: f64,
    pub radical_kern_after_degree: f64,
    pub radical_degree_bottom_raise_percent: f64,
    pub accent_base_height: f64,
    pub flattened_accent_base_height: f64,
    pub overbar_vertical_gap: f64,
    pub overbar_rule_thickness: f64,
    pub overbar_extra_ascender: f64,
    pub underbar_vertical_gap: f64,
    pub underbar_rule_thickness: f64,
    pub underbar_extra_descender: f64,
    pub display_operator_min_height: f64,
}

pub trait MathFontMetrics {
    // ...existing methods unchanged...
    /// `Some` when the provider is an OpenType `MATH` face: the layout
    /// reads the constants above where the table has a value TeX's
    /// parameters lack, exactly as LuaTeX does (`mlist.c`, `math_param_*`
    /// with the OpenType defaults). `None` (the default) is Appendix G.
    fn opentype_extras(&self, _size: SizeClass) -> Option<OpenTypeExtras> { None }
}
```

`OpenTypeMathConstants` (the 18-field struct math-layout already has) should
grow the same fields in font units, and `MathParams::from_opentype` should
return `(MathParams, OpenTypeExtras)` or get a sibling
`OpenTypeExtras::from_opentype(&OpenTypeMathConstants, size)`. The pipeline
fills the struct from font-engine's `MathConstants`, which has every one of
them today.

### 2.2 Cut-in kerning (`MathKern`) for scripts

OpenType fonts (STIX, Libertinus, Latin Modern Math) attach staircase kern
tables to glyphs; XeTeX/LuaTeX shift a superscript left of an `f` and a
subscript under a `V` by them (Rule 18 as amended in `mlist.c` `make_scripts`,
"math kerning"). TeX has nothing like it, so this is a new method:

```rust
/// Which corner of a glyph a script attaches at (OpenType MathKernInfo).
pub enum KernCorner { TopRight, TopLeft, BottomRight, BottomLeft }

pub trait MathFontMetrics {
    /// The kern (pt, usually negative) the font asks for at `corner` of
    /// `glyph` for a script whose relevant edge is `height` pt above the
    /// baseline: the staircase table's value for that height. `0.0` for a
    /// TFM provider and for glyphs without a table.
    fn math_kern(&self, _glyph: &Glyph, _corner: KernCorner, _height: f64) -> f64 { 0.0 }
}
```

font-engine's `MathTable` does not parse `MathKernInfo` yet; the pipeline
will read it the way it reads `MathVariants` (in `mathfont.rs`) until
font-engine exposes it. Layout side: `make_scripts` adds
`math_kern(nucleus, TopRight, sup_shift) + math_kern(sup, BottomLeft, …)`
to the superscript's x, symmetrically for the subscript; both applied only
when `opentype_extras()` is `Some`.

### 2.3 Glyph selection: Unicode math alphanumerics, not CM slots

`MathProvider::Tex` selects glyphs through `math-layout`'s `cm` slot tables
(`cm::symbol_slot`, family + TFM code). `MathFonts::glyph` already selects
by *character*: `MathFonts::math_char` maps `a..z`/`A..Z`/Greek to the
Mathematical Alphanumeric Symbols italic block, then `cmap`. What a
`\setmathfont` document needs on top, all in the pipeline (no math-layout
change):

* the `\mathbf`/`\mathsf`/`\mathtt`/`\mathcal`/`\mathfrak`/`\mathbb`/
  `\mathscr` alphabets become code-point ranges (U+1D400 bold, U+1D5A0 sans,
  U+1D670 mono, U+1D49C script, U+1D504 fraktur, U+1D538 double-struck),
  with `unicode-math`'s `math-style`/`bold-style` defaults (ISO: Latin and
  Greek lowercase italic, uppercase Greek upright — `unicode-math.pdf` §5.2)
  replacing `mathalpha.rs`'s text-font alphabets;
* symbols keep their Unicode code points (the compiler's `MathList` already
  carries `char`s), so `\sum` is U+2211 with `MathVariants` display sizes
  (`large_operator` — already implemented), `\left(` is U+0028 with its
  vertical variants and assembly (already implemented);
* the secondary double-struck face (`BB_FONT`, New Computer Modern Math for
  msbm's design) is a Latin-Modern-only fidelity trick: a named math font
  draws its own double-struck block.

`MathFontMetrics::glyph(ch, size)` stays the interface; only the provider
behind it changes.

### 2.4 Selection and precedence

* `\setmathfont[opts]{Family}` (fontspec/unicode-math syntax; `range=`
  options are out of scope for the first cut and noted) and the manifest's
  `[fonts] math = "..."` select the face via `FontIndex::find` restricted to
  `math_fonts()`; a family without a `MATH` table is a
  `math_font_unavailable` warning and math stays on the default.
* `\usepackage{unicode-math}` alone (no `\setmathfont`) selects Latin
  Modern Math through the OpenType provider — that is what unicode-math
  does under XeLaTeX — and is a visible change from TFM geometry (see §3).
* Precedence: `\setmathfont` > manifest `math` > default. The default is
  `MathProvider::Tex` (TFM CM/LM) exactly as today; **`\setmainfont` alone
  never changes math** (fontspec's rule too).
* The math provider is per document, not per group: `\setmathfont` in the
  body is honoured from its position only when it is at the top level of
  the preamble or the body start; a mid-document `\setmathfont` is noted
  and ignored (unicode-math's own documented limitation).
* Sizes: `\DeclareMathSizes` (LaTeX's table, `MathSizes`) by default; with
  `unicode-math` loaded, `ScriptPercentScaleDown`/`ScriptScriptPercentScaleDown`
  from the table (70/50 in Latin Modern Math; STIX Two 70/55) — `MathSizes`
  gets a `from_percent_scale_down(text: f64, c: &MathConstants)` constructor.

## 3. What stays on TFM Computer Modern (pdflatex parity)

A document that sets no math font renders through `MathProvider::Tex`
byte-for-byte as today: `hw1`, every corpus fixture, and the visual-oracle
gate are unaffected by any of this. The OpenType path is opt-in per
document. `\usepackage{amsmath}`/`amssymb`/`lmodern`/`mathptmx` keep their
current meaning; `mathptmx` under `\setmathfont` is an error-free no-op
noted once ("math font already selected by \setmathfont").

## 4. Test plan

1. **`hw1` under `\setmathfont{Latin Modern Math}`** (the acceptance in the
   proposal): every math glyph within 0.5 bp of the TFM layout. Latin
   Modern Math's `MathConstants` were derived from CM's `\fontdimen`s and
   its glyph advances from the `lm*` TFMs, so the difference is the rules
   in 2.1 (OpenType gaps vs Appendix G's multiples of ξ₈) and the cut-ins in
   2.2, both of which are below 0.5 bp for `hw1`'s constructions (fractions,
   scripts, `\sum` limits, `\left(`). Oracle: the existing
   `fixtures/real-world/hw1/reference.json` — no new TeX run. A second
   control: the same document through XeLaTeX with `unicode-math` (MacTeX
   `xelatex`, oracle only) for the constructions where OpenType differs from
   TeX on purpose, so the test says which of the two geometries it matches.
2. **STIX Two Math and Libertinus Math** (installed here; `math_fonts()`
   lists them): a probe with `\frac`, `x^2_i`, `\sqrt[3]{x}`, `\sum_{i=0}^n`,
   `\left(\frac{a}{b}\right)`, `\hat{f}`, `\overline{AB}` against XeLaTeX +
   unicode-math (oracle), 0.5 bp on glyph origins and rules. This is the test
   that exercises 2.1 and 2.2 for real: TFM-derived CM cannot.
3. **Alphabets**: `\mathbf{x}\mathsf{y}\mathtt{z}\mathcal{A}\mathfrak{B}\mathbb{C}`
   resolve to the Unicode code points above and the glyphs the face maps
   them to (`cmap` presence asserted per face; a face lacking a block falls
   back to the default math face per glyph with one `missing_glyph`).
4. **Missing/absent**: `\setmathfont{Helvetica}` (no `MATH` table) →
   `math_font_unavailable`, math unchanged; `\setmathfont{No Such Math}` →
   the same with the directories searched.
5. **Byte-identical control**: `scripts/render-corpus-v2.sh` before/after,
   as for the text half (70 fixtures on this branch, all identical).

## 5. Decisions that need a ruling

1. **Which geometry is "right" for an OpenType math font: LuaTeX's or
   XeTeX's?** They differ (LuaTeX reads the 2.1 gaps and kerns by default;
   XeTeX's `\Umath*` parameters are set by unicode-math from the same table
   but a few rules — radical degree, accent flattening — differ in detail).
   Recommendation: LuaTeX's `mlist.c`, because it is the documented
   OpenType→TeX mapping (LuaTeX manual §7.3) and what `MathParams::from_opentype`
   already cites. This decides the oracle for tests 1–2.
2. **`unicode-math` without `\setmathfont`: switch to the OpenType provider
   (XeLaTeX's behaviour) or stay on TFM CM?** The proposal says the package
   "keeps meaning what it means", which is the switch; it changes `hw1`-like
   documents that happen to load unicode-math (none in the corpus). The
   contract above takes the switch; the owner should confirm.
3. **Where `MathKernInfo` and `MathVariants` parsing lives.** Today the
   pipeline parses `MathVariants` itself because font-engine does not expose
   it; the cut-in tables would go the same way. The clean home is
   `crates/font-engine/src/math.rs` (Daniel's `font-engine` too) with
   `MathTable::variants(gid)`, `assembly(gid)`, `kern(gid, corner, height)`,
   and the pipeline's copies deleted. Ruling wanted on whether that move is
   part of this slice or a follow-up.
