# flashtex-microtype — integration contract (KC-102)

Status: proposal. This crate edits nothing outside `crates/microtype/`; the
hooks below are for the owners of `crates/paragraph-layout` (Knuth–Plass) and
`crates/render-pipeline` to adopt.

## What pdfTeX does (verified)

Sources: pdfTeX 1.40.27 `pdftex.web` (TeX Live trunk), `utils.c`
(`extxnoverd`), microtype v3.2d `microtype.sty` / `microtype-pdftex.def` /
`microtype.cfg` / `mt-cmr.cfg`; section names are cited in `src/*.rs`.
Every rule below is exercised by `tests/oracle.rs` (32 pdflatex fixtures, all
identical breaks, identical per-glyph expansion, identical margin kerns,
glyph x error ≤ 0.00018 pt) and each one is shown to matter by
`MT_MUTATE=<rule> cargo test --test oracle` (each mutation breaks fixtures).

`\usepackage{microtype}` on pdfTeX in PDF mode sets `\pdfprotrudechars=2`,
`\pdfadjustspacing=2`, and for every font in the default sets
(protrusion: `alltext` = all text encodings; expansion: `alltext-nott` = text
encodings, `\rmdefault`/`\sfdefault` families only, **not** typewriter, **not**
math fonts): `\lpcode`/`\rpcode` from the matched list converted to
thousandths of an em as `round(charwd · value / \fontdimen6)`, and
`\pdffontexpand <f> 20 20 1 autoexpand` with every `\efcode` 1000
(non-selected).

1. **Line breaking** (`try_break`). For a candidate line r→cur:
   `shortfall = hsize − natural` then
   * `shortfall += total_pw` — `left_pw(first protrudable char) + right_pw(last)`,
     with the char found by skipping "skipable" nodes (penalties, font kerns,
     zero-width math/kerns, empty discretionaries, zero *parameter* glue);
     a discretionary break protrudes the last pre-break char (right) and the
     first post-break char (left). At the paragraph end the right side hits
     `\parfillskip` and yields 0.
   * if `shortfall ≠ 0`: with `FS = Σ char_stretch + Σ kern_stretch` over the
     line (`cur_active_width[7]`, likewise `FK` for shrink):
     `0 < shortfall < FS` → `shortfall = (FS div (max_stretch div step)) div 2`;
     `shortfall ≥ FS > 0` → `shortfall −= FS`; symmetric for shrink.
     (`adjust_shortfall`.) Badness/fitness/demerits are then TeX's, unchanged.
   * `char_stretch(c) = round(round(w·(1000+20)/1000) − w) · ef/1000`; kern
     stretch only for font kerns directly between two chars of one expandable
     font. Discretionary pre/post/replace texts contribute to slots 7/8 like
     slot 1.
2. **Packing** (`post_line_break` + `hpack(…, cal_expand_ratio)`):
   * margin kern `−right_pw` inserted after the last protrudable node (also on
     the last line, before `\parfillskip`), `−left_pw` at the line start;
   * `ratio = divide_scaled(w − natural, FS or FK, 3)` clipped to ±1000, only if
     the dominating glue order is finite; each char then gets
     `e = fix_expand_value(ext_xn_over_d(ratio·ef, 20, 10^6))` (±20, step 1),
     width `round(w·(1000+e)/1000)`; kerns and margin kerns keep their widths;
   * glue is set on the remaining excess exactly as in TeX.
3. **Output**: glyph origin = TeX `hlist_out` position with expanded widths;
   the glyph itself is drawn horizontally scaled by `(1000+e)/1000`
   (pdfTeX autoexpand writes it through the text matrix).

## API (this crate)

```rust
use flashtex_microtype::{config::*, pdftex::*};

// once per font (NFSS name + TFM metrics)
let cfg = MicrotypeConfig::bundled();                       // microtype.cfg + mt-cmr.cfg
let defaults = NfssDefaults::latex("T1", "cmr", "cmss", "cmtt");
let font = NfssFont::parse("T1/cmr/m/n/10.95").unwrap();
let rf: ResolvedFont = cfg.resolve(&Options::default(), &defaults, &font, &metrics)?;
let fp: FontParams = rf.params;  // quad, lpcode/rpcode/efcode[256], expansion limits

// per glyph / per line (all i32 scaled points)
fp.left_protrusion(slot); fp.right_protrusion(slot);
fp.char_stretch(slot, base_w); fp.char_shrink(slot, base_w);
fp.kern_stretch(left_slot, kern_w); fp.kern_shrink(left_slot, kern_w);
let mut par = ParagraphExpansion::default(); par.note_font(&fp)?;
adjust_shortfall(shortfall, fs, fk, 0, 0, par);
let ratio = line_expand_ratio(excess, fs, fk, finite_order);
let e = fp.char_expansion(slot, ratio); expanded_width(base_w, e);
```

`metrics` implements `FontMetrics { char_width(slot), quad() }` from the TFM
(render-pipeline `tfm.rs`). Everything is integer sp: the rounding is part of
the contract, so integrate in sp, not f64 points.

## Proposed hooks — `crates/paragraph-layout`

The breaker currently works in f64 points (`linebreak.rs`); exact pdfTeX
parity needs sp integers for these sums. Proposed, in order of need:

1. **Item data.** `Glyph` needs the TFM slot (today `gid` is used for that in
   the TFM path) and each `GlyphRun` a handle to the run font's `FontParams`
   (e.g. `LineBreakParams::microtype: Option<&dyn MicrotypeFonts>` with
   `fn params(&self, FontId) -> Option<&FontParams>`). Inter-glyph kerns
   stored in `Glyph::kern` are font kerns (`kern_stretch` applies); `Item::Kern`
   from `\kern` is explicit (no stretch); one new flag is needed for font kerns
   that cross run boundaries (`Item::Kern { font_kern: bool }`).
2. **Prefix sums** (`prefix_sums`, l. 255): add two slots, `font_stretch` and
   `font_shrink` = Σ `char_stretch`/`kern_stretch` (only when
   `adjust_spacing > 1`), including discretionary pre-break text
   (`pre_break`, l. 310) and, for the width after a discretionary break, the
   post-break text minus replaced text.
3. **`measure`** (l. 371): after `natural` is known, compute
   `shortfall = target − natural + total_pw(start, brk)` (protrusion ≥ 2) and
   pass it through `adjust_shortfall` before the badness branch (l. 402–420).
   `total_pw` needs `line_start` (l. 330) semantics for the left char and the
   "skipable" walk described above.
4. **Line packing** (the per-line `ratio` computed after breaking, feeding
   `PositionedRun`): insert the margin kerns, compute
   `line_expand_ratio`, give each glyph `e = char_expansion`, recompute widths,
   then set glue on the new natural width. Expose `e` per glyph
   (`PositionedGlyph::expansion: i16`, thousandths) and the margin-kern
   shifts in the run x positions.
5. Ragged-right / first-fit paths: pdfTeX applies the same shortfall rule; with
   `\rightskip` fil glue the expansion ratio is 0 on every line.

`tests/common/mod.rs` in this crate is a complete reference (TeX's
active-list order, tie rules, three passes, discretionary break widths,
pruning) that the owner can diff against.

## Proposed hooks — `crates/render-pipeline`

1. `typeset.rs` `line_params` (l. 1038) → set `adjust_spacing = 2`,
   `protrude_chars = 2` when the preamble loads `microtype` (and its options
   `protrusion=`, `expansion=`, `stretch=`, `shrink=`, `step=`, `factor=`,
   `selected=` map to `Options`).
2. `fonts.rs` font loading (`layout_id`, l. 328): resolve `FontParams` once per
   (NFSS name, size) with `MicrotypeConfig::resolve`; the NFSS name is the one
   LaTeX would select (e.g. `T1/cmr/bx/n/12`). Math fonts (OML/OMS/U) are
   outside both sets: `FontParams::plain(quad)`.
3. `layout_paragraph` calls (l. 1114, l. 1281): pass the font-params lookup.
4. PDF emission (`pdf.rs`): for a glyph with `e ≠ 0` emit it with horizontal
   scale `(1000+e)/1000` (`Tz` or a text-matrix `a` of `(1000+e)/1000`) at the
   position from layout; advance with the expanded width.

## HW1 / HW2 delta (pdflatex, MacTeX 2026)

`tests/oracle/hw_delta.py` builds each file with and without
`\usepackage{microtype}` and compares the PDF text line by line (Ghostscript
`txtwrite`, whitespace-insensitive):

| file | lines with / without | changed line breaks | pages |
|---|---|---|---|
| HW1 | 77 / 77 | none — every text line identical; microtype changes only glyph positions (expansion + margin kerns) | 3 / 3 |
| HW2 | 79 / 80 | 2 paragraphs reflow: "…nonempty sets A,B,C ⊆ Z." now fits line 1 of the Problem statement (3 lines re-broken), and "…h is odd, and" pulls "and" up, saving one line | 3 / 3 |

So for HW1 the break mismatch FT-064 sees is **not** caused by microtype
breaks (glyph x will still differ by up to ~1.4 pt without margin kerns and
expansion); HW2 needs this model for two paragraphs.

## Not modelled / known limits

* Non-autoexpand expansion (separate expanded TFMs) and "variations of
  marginal kerns" (always 0 with autoexpand because `\fontdimen6` is shared).
* `kern_stretch` assumes the kern pair exists in the font's lig/kern program
  (true for every font kern TeX inserts between two chars).
* Config: size-restricted sets/lists, `unit=`, `preset=`, `context=`, and
  named slots outside T1/LY1 return `ResolveError` instead of guessing.
  Only `microtype.cfg` + `mt-cmr.cfg` are bundled.
* Oracle glyph x positions are computed from pdfTeX's own `\showbox` data
  (exact sp widths, expansion, margin kerns; glue set printed to 5 digits),
  not re-extracted from the PDF.
* `\parshape`, `\hangindent`, `\looseness ≠ 0`, `\lastlinefit`, TeXXeT,
  inserts/marks/adjusts in the reference breaker; no math in the fixtures.

Regenerate fixtures: `python3 crates/microtype/tests/oracle/generate.py`.
