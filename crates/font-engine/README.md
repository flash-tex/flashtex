# flashtex-font-engine

Original Rust font engine for FlashTeX: font loading and metrics, TeX-oriented
shaping with source-byte clusters, deterministic subsetting, and the data a PDF
writer needs to embed a font. No external crates (matching the sibling crates),
edition 2024, no existing TeX engine or shaping library involved.

Task: FT-018 rev 1. Owner: `mac-font-engine` (Claude Code subagent, parent
`mac-claude-a`, machine `mac-m1max-a`). Nothing under `crates/compiler` is
touched; the compiler may adopt this crate later through its own owner.

Default face policy (Commander, 2026-09-12): LaTeX's default is Computer
Modern, so the pipeline's default font is **Latin Modern** (GUST Font License,
OpenType CFF, shipped with BasicTeX); the Adobe Core 14 tables keep Times /
Helvetica / Courier / Symbol available for `\usepackage{times}` and the
standard PDF fonts.

```sh
cd crates/font-engine
cargo test            # 63 tests + 2 compile_fail doctests; font-file tests skip with a message if a file is absent
cargo clippy --all-targets
swift examples/compare_coretext.swift   # macOS: CoreText cross-check (see "Engine comparison")
```

## What is implemented

| Area | Implemented | Not implemented (declared, never silent) |
| --- | --- | --- |
| Font programs | TrueType / OpenType with `glyf` outlines and OpenType `CFF ` (`OTTO`, e.g. Latin Modern); `.ttc` collections by face index (`head`, `hhea`, `hmtx`, `maxp`, `loca`/`glyf` or `CFF `, `cmap` formats 4 and 12, `OS/2`, `post`, `name`, `MATH`) | CFF charstring parsing (outlines are exposed as the raw `CFF ` table only; no glyph bbox derivation for CFF), variable fonts (`fvar`/`gvar` rejected), bitmap-only fonts, cmap formats other than 4/12, Type 1 (`.pfb`/`.pfa`) programs, AFM files other than the built-in Core 14 tables |
| Base-14 metrics | Times-Roman, Times-Bold, Times-Italic, Times-BoldItalic, Helvetica, Courier, Symbol: Adobe AFM widths for every AFM glyph with an Adobe Glyph List code point (322 code points per text face, 194 for Symbol), AFM `KPX` kerning pairs, header metrics (bbox, cap/x height, ascender/descender, italic angle, StdVW, underline) | Helvetica-Bold/Oblique variants, Courier variants, ZapfDingbats (the seven faces the task named are included; adding the rest is a one-line change in `tools/gen_tables.py`) |
| Kerning | `GPOS` `kern` feature: PairPos format 1 and 2 (lookup type 2), including type 9 Extension wrappers; legacy `kern` table format 0 (Microsoft and Apple headers); AFM `KPX`. Features are selected through ScriptList: `DFLT` script (else `latn`, else the first script), default LangSys (else the first) | GPOS lookup types other than 2 under `kern` (contextual 7/8, single 1) — recorded in `Face::unsupported()`; `kern` formats 1–3, vertical and cross-stream subtables; second-glyph value records and placement (only `xAdvance` of the first glyph is applied); per-run language tags (only the default language system is used, so Latin Modern's Turkish/Polish `liga`/`kern` variants are never applied) |
| Ligatures | `GSUB` `liga` feature: LigatureSubst (lookup type 4), including type 7 Extension wrappers, applied as one pass per lookup in LookupList order (so Latin Modern's `f`+`f`→`ff`, then `ff`+`i`→`ffi` works); cmap fallback for `ff fi fl ffi ffl` → U+FB00..U+FB04 as a final pass over single-character clusters (used by the Core 14 faces, and by fonts such as Times New Roman whose `liga` is Arabic-only); never applied to fixed-pitch faces | Every other GSUB feature (`dlig`, `clig`, `calt`, `smcp`, `locl`, …) and lookup type (single, multiple, alternate, contextual, chained, reverse); lookup flags (mark filtering, ignore-marks) |
| Marks | Base + combining mark composed through canonical pairwise compositions (870 primary composites, Unicode 15.0.0 via Python `unicodedata`) when the face has the precomposed glyph; otherwise the mark joins the base's cluster with zero advance, positioned by GPOS `mark` MarkToBase anchors (lookup type 4, +Extension) when the font has them (`Face::mark_attachment`), else by the documented approximation (zero-advance marks left where the designer hung them, spacing marks centred) | MarkToLigature (5) and MarkToMark (`mkmk`, 6) — recorded as unsupported; lookup flags |
| Scripts | Latin, Greek, Cyrillic and other left-to-right scripts without reordering; CJK ideographs (1 em advances verified with Arial Unicode) | Hebrew, Arabic, Syriac, Thaana, NKo, Indic, Thai, Lao, Tibetan, Myanmar, Khmer, Mongolian and bidi control characters: `shape` returns `Error::UnsupportedScript { ch, byte_offset, reason }` and produces nothing (fail closed) |
| Direction | Horizontal left-to-right | Vertical layout, right-to-left |
| Subsetting | Deterministic glyf subset: `.notdef` + requested glyphs + transitive composite components, renumbered in ascending original order; `head`/`hhea`/`maxp`/`hmtx`/`loca`(long)/`glyf` rewritten, `cvt `/`fpgm`/`prep` copied; table checksums and `head.checkSumAdjustment`; `verify_checksums` re-checks a program | Subsetting of `cmap`/`name`/`OS/2`/`post`/`GPOS`/`GSUB` (deliberately dropped: the embedded program is glyph-addressed and already shaped); **CFF subsetting** (`subset` returns `Error::Unsupported` for CFF faces; whole-program embedding is used instead — a later follow-up) |
| Embedding data | `PdfFontProgram`: `/BaseFont` (subset tag + name, or bare name for whole programs), `font_file` = `FontFile::TrueTypeSubset` (`/FontFile2`, `CIDFontType2`) or `FontFile::OpenTypeProgram` (`/FontFile3` `/Subtype /OpenType`, `CIDFontType0`, CIDs = original glyph ids), `/W` array as CID runs, `CIDToGIDMap /Identity`, `glyph_map` (original → CID), descriptor (flags, bbox, ascent, descent, cap height, x height, italic angle, placeholder StemV), `ToUnicode` CMap with multi-character `bfchar` destinations, OS/2 `fsType` | Writing PDF objects (a PDF crate does that); bare `/Subtype /Type1C` CFF streams (the whole OpenType wrapper is embedded instead); Type 1 embedding; `/Widths` arrays for standard-14 fonts beyond `embed::standard_font_widths` |
| Math | `MATH` table: all 56 `MathConstants`, per-glyph italics correction and top-accent attachment (`TrueTypeFace::math()`, `math::MathTable`) | `MathVariants` (stretchy delimiter construction), `MathKernInfo`, extended-shape coverage, device tables |
| Resolution | `FontSearch`: explicit directory list, `dir/<file name>` probes only, macOS system directories offered as a constant; `manifest::PinnedFontSet` loads a declared, hashed set from an explicit root | Directory scanning, fontconfig, name matching, `FLASHTEX_*` environment lookups |
| Fallback | `shape_with_fallback(&[faces], ..)`: explicit ordered chain, first face that has the character wins, per-cluster `Cluster::font` index into `Shaped::fonts` (FontIds), characters no face has → `.notdef` on the primary + `missing` | Any implicit or platform fallback; kerning/ligatures across a face boundary; faces with different `units_per_em` in one chain (error) |
| TeX encodings | `encoding::EncodingCode(u8)` + `Encoding::{OT1, T1}::to_unicode` / `from_unicode`; the three identities `EncodingCode` / `char` / `GlyphId` have no conversions between them (compile_fail doctests) | TFM parsing (lives in `crates/font-resources`); other encodings (OMS, OML, TS1, LY1) |

Everything in the right-hand column is either an error, an empty result, or an
entry in `Face::unsupported()` (copied into every `Shaped`), never a silently
wrong number.

## API for preview and PDF consumers

```rust
use flashtex_font_engine::{Face, load_from_path, ShapeOptions, shape};
use flashtex_font_engine::core14::{Core14, Core14Face};
use flashtex_font_engine::embed::EmbedPlan;

// Base-14 metrics without any font file:
let times = Core14Face::new(Core14::TimesRoman);
let s = shape(&times, "Hello", &ShapeOptions::PLAIN)?;
assert_eq!(s.width_pt(12.0), 26.664);          // 2222 units at 1000/em

// A real font program:
let tnr = load_from_path("/System/Library/Fonts/Supplemental/Times New Roman.ttf".as_ref())?;
let s = shape(&tnr, "office AV", &ShapeOptions::default())?;   // ligatures + kerning
for cluster in &s.clusters {
    // cluster.source_range: byte range of the input; cluster.text: that text;
    // cluster.glyphs: ORIGINAL glyph ids with advances (font units) incl. kerning.
}
assert!(s.missing.is_empty());                  // or list of (char, byte_offset)

// PDF embedding data:
let mut plan = EmbedPlan::new();
plan.add_shaped(&s);
let pdf = plan.finish(&tnr)?;                   // subset program, /W, ToUnicode, descriptor
let cid = pdf.cid(s.clusters[0].glyphs[0].gid); // original gid -> subset CID
```

Key types (all in `src/lib.rs` unless noted):

* `FontId { family, weight, style, source, content_sha256 }` — content-addressed
  identity (SHA-256 of the whole program bytes + 4-byte face index; Core 14
  faces hash their generated table). Equal ids ⇒ identical glyph ids, metrics
  and shaping.
* `GlyphId(u16)` — always an ORIGINAL id in `Face` and `Shaped`. Subset ids
  exist only in `subset::Subset { old_to_new, new_to_old }` and
  `embed::PdfFontProgram::cid()`.
* `Face` trait: `units_per_em`, `advance(gid)`, `glyph_id(char)`,
  `vertical_metrics()` (ascender, descender, line gap, cap height, x height
  with `*_declared` flags), `bbox()`, `italic_angle()`, `kerning(l, r)`,
  `kerning_source()`, `ligature(&[gid])`, `unsupported()`, `postscript_name()`,
  `is_fixed_pitch()`, and the converters `to_points(units, size_pt)` /
  `to_pdf_units(units)`.
* `shape::shape(face, text, &ShapeOptions) -> Result<Shaped>`;
  `Shaped { clusters, missing, unsupported, units_per_em, kerning_source, ligatures_applied }`;
  `Cluster { glyphs, source_range, text }`; `Glyph { gid, advance, x_offset, y_offset }`.
  Helpers: `advance_units()`, `width_pt(size)`, `cluster_at_byte(offset)`,
  `cluster_at_x(units)`.
* `ShapeOptions { ligatures, kerning, compose_marks, cmap_ligature_fallback }`;
  `ShapeOptions::PLAIN` is mapping only.
* `subset::subset(&TrueTypeFace, &[GlyphId]) -> Subset` (glyf only); `subset::verify_checksums`.
* `embed::EmbedPlan` → `PdfFontProgram { base_font, cid_font_subtype, font_file, cid_widths, glyph_map, descriptor, to_unicode, to_unicode_cmap, fs_type, subset }`; `embed::to_unicode_cmap` / `parse_to_unicode`.
* `TrueTypeFace::outlines()` (`Outlines::Glyf | Cff`), `cff_table()`, `math()`.
* `math::MathConstants` (font units; percent fields in percent), `MathTable::italics_correction(gid)`, `top_accent_attachment(gid)`.
* `resolve::FontSearch`.

Latin Modern in practice:

```rust
let lm = load_from_path("/usr/local/texlive/2026basic/texmf-dist/fonts/opentype/public/lm/lmroman10-regular.otf".as_ref())?;
assert_eq!(lm.outlines(), Outlines::Cff);
let s = shape(&lm, "Hello", &ShapeOptions::PLAIN)?;      // 2250 units = 22.5 pt at 10 pt
let math = load_from_path(".../lm-math/latinmodern-math.otf".as_ref())?;
let c = &math.math().unwrap().constants;                // c.axis_height == 250
```

Units: `Face` and `Shaped` values are font units (`units_per_em` = 1000 for Core
14, typically 2048 for TrueType). Preview converts with `to_points`; PDF uses
`to_pdf_units` (1/1000 em, rounded). Kerning is folded into the advance of the
glyph before the pair; `x_offset`/`y_offset` are only non-zero for unattached
marks.

## Pinned licensed fonts (`fonts/manifest.json`)

`manifest::pinned_latin_modern()` declares the default font set — Latin Modern
Roman 10 (regular, bold, italic, bold italic), LM Sans 10, LM Mono 10 and
Latin Modern Math — with the eight `FontDescriptor` fields and the
`LicenseMetadata` fields of `crates/font-resources` (`schema_version: 1`),
so the same JSON can be handed to that loader once it accepts
`format: "opentype-cff"` (its current profile is `static-truetype` only; that
is why `PinnedFontSet::load` performs the identical checks here: byte length,
SHA-256, units per em, glyph count, PostScript name, licence-text SHA-256; any
mismatch refuses the whole set). `fonts/manifest.json` is the byte-stable output
of `cargo run --example write_manifest`; `fonts/GUST-FONT-LICENSE.txt` is the
licence text (SHA-256 `49ea6cb9…8050`). Paths are relative to an explicit root
(`BASICTEX_OPENTYPE_ROOT` on this project's Macs); font files are never
committed. Times/Helvetica/Courier/Symbol stay available through the Core 14
tables (`\usepackage{times}`).

## Fallback identity

```rust
let chain: Vec<&dyn Face> = vec![&lm_roman, &lm_sans, &lm_mono, &symbol];
let s = shape_with_fallback(&chain, "a\u{2211}b", &ShapeOptions::default())?;
// s.clusters[1].font == 3  →  s.fonts[3] is Symbol's FontId; never silent.
```

The choice is a pure function of the chain order and the text (first face whose
cmap has the scalar; a combining mark stays with its base's face so composition
can apply); the test suite asserts identical output across runs and that
reordering the chain changes the result exactly as declared.

## TeX encodings and the three identities

`EncodingCode(u8)` (a TFM slot), `char` (Unicode) and `GlyphId` (a font's
original glyph order) are distinct newtypes with no `From`/`Into` between them;
`encoding.rs` carries two `compile_fail` doctests proving that
`let g: GlyphId = EncodingCode(0x41)` and `let c: char = EncodingCode(0x41)`
do not compile. Mapping example (tested in `tests/pinned.rs`):

```text
OT1 slot 12  --Encoding::OT1.to_unicode-->  U+FB01 'ﬁ'  --Face::glyph_id-->  LM Roman gid 125
T1  slot 0xE9 --Encoding::T1.to_unicode-->  U+00E9 'é'  --Face::glyph_id-->  LM Roman gid 251
```

TFM parsing itself is not duplicated here; `fixtures/tfm/` supplies the real
`ec-lmr10` fixture for `crates/font-resources`' `BoundTfmFont` (see its README).

## Native rendering and PDF round-trip checks (`tools/`)

* `cargo run --example pdf_roundtrip -- <font> <size> <out.pdf> <text>` writes a
  one-page PDF from `EmbedPlan` output (Type0 / Identity-H, `/W`, `ToUnicode`,
  `TJ` runs carrying the shaper's kerning, `/Span << /ActualText >>` around
  ligature and mark clusters) and prints the glyph positions it used.
* `swift tools/pdf_extract_check.swift <pdf> <expected text> [png]` extracts
  the page text with PDFKit and compares it with the original Unicode, and
  renders the page with CoreGraphics to prove the embedded font loads.
* `swift tools/render_check.swift <font> <size> <positions> <text> [prefix]`
  draws the engine's glyph ids at the engine's positions with CoreText from
  the SAME font file (A) and lets CoreText shape and draw the text itself (B),
  then counts differing pixels at 8×.

Results on macOS 26.3.1 (2026-09-12), 12 pt:

| font | embedding | text | PDFKit extraction | render check (A vs B) |
| --- | --- | --- | --- | --- |
| lmroman10-regular.otf | `/FontFile3 /CIDFontType0C`, CIDFontType0, identity CIDs | "Efficient AV office, café — naïve “quoted” 100% ffi" | exact (`TEXT PASS`, incl. ffi/fi through ActualText+ToUnicode) | **0 differing pixels** of 25 420 |
| Times New Roman.ttf | `/FontFile2` subset `HGXALI+TimesNewRomanPSMT`, CIDFontType2 | "Efficient AV office, café — naïve x́ “quoted” 100% ffi" | exact (`TEXT PASS`, x + U+0301 attached by MarkToBase) | 1.19 ratio — expected: CoreText applies no f-ligatures for TNR (Arabic-only `liga`), the engine's cmap fallback does; everything after "Effi" shifts |
| Times New Roman.ttf | same | "Elegant AV noise, café — naïve x́ “quoted” 100% Today" (no f-ligatures) | exact | **0 differing pixels** of 31 398 (kern table and MarkToBase agree with CoreText) |

`/CIDFontType0C` for the bare `CFF ` table is the subtype CoreGraphics accepts
(PDF worker's finding); `/Type1C` is wrong for a CIDFont and the whole-sfnt
`/OpenType` route is no longer used.

## Consumer adapters (rev 3)

`adapters::paragraph` (`FontMetricsSource` + `glyph_run`), `adapters::pdf`
(`to_pdf_embedded_subset`), `adapters::math` (`MathFontMetrics` from `MATH`)
and `adapters::preview` (`face_metrics.json`) let compiler layout, PDF, math
and the native preview read one `Face`. The sibling crates are optional path
dependencies behind the default features `paragraph`, `math`, `pdf`. Evidence
and the remaining per-consumer gaps are quantified in
[`docs/consumers.md`](docs/consumers.md): on the visual corpus, paragraph
(shaped route), PDF `/W`+`TJ` and CoreText drawing of the exported positions
all agree with the engine to 0.0000 pt and 0 differing pixels.

Unsupported lookup types met by a *requested* feature are an error by default
(`ShapeOptions::fail_on_unsupported_lookups`, `Error::UnsupportedFeature`) —
for example Arial Unicode's MarkToLigature lookups when an unattached mark is
shaped — never a silent approximation.

## Proposed ABI (non-authoritative until Commander accepts)

This crate is a library for a future negotiated display format. It emits no
runtime-v1 item kinds and proposes no protocol change. Before paragraph/math
integration, the following would be the agreed contract between compiler,
preview and PDF consumers:

1. **Font identity** is `FontId.content_sha256` (hex via `content_hex()`),
   never a file path or family name. Consumers key caches and PDF font
   resources by it. Path and face index travel alongside for diagnostics.
2. **Glyph ids in shaping output are original ids** of that face. Renumbering
   happens only inside `subset`/`embed`, which return the explicit
   `old_to_new` map; a PDF writer translates with `PdfFontProgram::cid()` at
   write time. No other component ever sees subset ids.
3. **Clusters carry source byte ranges** (`Range<usize>` into the UTF-8 input,
   half-open) and the exact source text. Click-to-source maps a hit glyph to
   its cluster, then to the byte range; source-to-preview maps a byte to
   `cluster_at_byte`. Ligatures merge ranges; empty-glyph clusters keep
   default-ignorable bytes addressable.
4. **Multi-glyph and multi-character clusters** expose `Cluster::text` so a PDF
   writer can emit `/Span << /ActualText (...) >>` for ligatures and composed
   marks; `ToUnicode` is the fallback (first recorded text per CID), not the
   only mechanism.
5. **Metrics** are font units plus `units_per_em`; conversion to points is
   `units * size / units_per_em` and to PDF glyph space is
   `round(units * 1000 / units_per_em)`. Vertical metrics come from OS/2 typo
   values when `USE_TYPO_METRICS` is set, else `hhea`; cap/x height from OS/2
   v2+ or the `H`/`x` glyph `yMax` with `*_declared = false`.
6. **Missing glyphs** are reported as `(char, byte_offset)` and rendered as
   `.notdef` (advance of `.notdef`; 0 for Core 14). Consumers must surface them
   (compiler diagnostic, preview marker). Nothing is replaced by `?`.
7. **Unsupported shaping** is an error (`UnsupportedScript`) for scripts the
   engine cannot lay out, and a list (`Shaped::unsupported`) for features the
   font has that the engine skipped. Partial output is never returned for the
   former.
8. **Font resolution** is a bounded explicit search list supplied by the
   caller; the engine never scans directories or reads environment variables.

## Provenance, licences, hashes

Nothing binary is committed. Inputs to the generated tables:

| Input | Where obtained on `mac-m1max-a` | SHA-256 |
| --- | --- | --- |
| `Times-Roman.afm` | matplotlib `mpl-data/fonts/pdfcorefonts` (Adobe Core 14 AFMs, version 002.000, 1997) | `86136b527a5acee0c3283d56a6b68fc6a3d60982eb3bb0fb6cc54e83ecc8d77a` |
| `Times-Bold.afm` | same | `7104e6af62c53f029013fb0641a81e31c98589c8f27b5ea6289b25b092c74321` |
| `Times-Italic.afm` | same | `6cae69b92329193fd85082f0c89ad7a318df8fb5c939d7efe5343f88ab09473c` |
| `Times-BoldItalic.afm` | same | `a7358e772726e91aa87015a001563926ad33dc5a7b2eea9680b34cc78aba385c` |
| `Helvetica.afm` | same | `db772f2830fb6d000907791d8d26a12524d96943a9a739e520ee855c6b25c96f` |
| `Courier.afm` | same | `31d72adad79910126b22a5579ec9e179eee86674ecebe3fcff6f76916193af0e` |
| `Symbol.afm` | same | `3f951aa17af8cb4aa1e1288c6b9baa8a30d3e990ebdbe8cf9a3d5acabcb9f771` |
| `glyphlist.txt` | TeX Live 2026 `texmf-dist/fonts/map/glyphlist/glyphlist.txt` (Adobe Glyph List) | recorded in `src/generated.rs` header |

Adobe's AFM licence (the `readme.txt` shipped with them): the files "may be
used, copied, and distributed for any purpose and without charge, with or
without modification, provided that all copyright notices are retained". The
widths are facts about the fonts; the generated Rust table records the source
digests and does not reproduce the AFM files. TeX Live's
`fonts/afm/adobe/times/*.afm` carry identical `WX` widths (only glyph bounding
boxes differ between the older and 1997 sets), checked with `diff` on
`ptmr8a.afm` and `psyr.afm`.

The compiler's `crates/compiler/src/metrics.rs` on `de1020c` embeds the same
Times/Helvetica/Courier ASCII and Latin-1 widths; `core14::tests` asserts
equality on the values it quotes (`M` 889, space 250, "Mac" 21.324 pt).

Latin Modern (GUST Font License — free to use, copy, modify and redistribute,
with the usual renaming clause for modified versions; the licence text ships in
TeX Live as `doc/fonts/lm/` and on gust.org.pl). Read from the local BasicTeX
installation; nothing is committed:

| File | SHA-256 |
| --- | --- |
| `/usr/local/texlive/2026basic/texmf-dist/fonts/opentype/public/lm/lmroman10-regular.otf` | `1aa18cfefa58132c52ce5de70db1fd1154201c19cd2b2cdaffba4906a33e6852` |
| `/usr/local/texlive/2026basic/texmf-dist/fonts/opentype/public/lm-math/latinmodern-math.otf` | `6075562b771f8b82f0c179e363389684f2dd09de30038269e2628e504bd7be0f` |

The other Latin Modern faces the tests open (`lmroman10-bold/italic/bolditalic`,
`lmroman12-regular`, `lmsans10-regular`, `lmmono10-regular`) come from the same
directory; they are parsed and shaped but no numbers are asserted for them.

Reference fonts used only by tests and the comparison script (Apple-supplied,
licensed for use on the device; redistribution, including inside a PDF, is the
user's decision and this crate exposes `fsType` so a writer can warn):

| File | SHA-256 (macOS 26.3.1) |
| --- | --- |
| `/System/Library/Fonts/Supplemental/Times New Roman.ttf` | `f3b4ffff71c2a0c7227d37497683b2498fb2d0a4e8beae26f022e3ccfcaabfa3` |
| `/System/Library/Fonts/Supplemental/Times New Roman Italic.ttf` | `7bb99c3a67697fbce15ab66d806d9a2151432ce1dab2eaba4156e1251da486c0` |
| `/System/Library/Fonts/Supplemental/Arial.ttf` | `525979822591a3447cfc49d943d6f7683508e25543407871c0ed8fed05fd2bd9` |
| `/System/Library/Fonts/Supplemental/Arial Unicode.ttf` | `876af2cd4854644e7f3e7feb2f688997fdb3343c6df6693611209c9dfb47ccec` |
| `/System/Library/Fonts/Supplemental/Iowan Old Style.ttc` | `3a6091b6c7e3cf97bf0a749f7f17769a97e5c46228aead256e2c6ccd94b43a7f` |
| `/System/Library/Fonts/Times.ttc` | `20e3dc89912f4b37f2c29add764855164efafb4c66ca32551d9eb52c7411c7c7` |

Which font exercises which path: Times New Roman — legacy `kern` table, cmap
f-ligatures (its GSUB `liga` is Arabic-only, verified), mark composition,
subsetting, ToUnicode; Times New Roman Italic and Arial — GPOS PairPos (Arial's
GPOS values equal its legacy table's, asserted); Iowan Old Style — GPOS
Extension (type 9 → 2) and a real Latin GSUB `liga`; Arial Unicode — CJK
repertoire; Times.ttc — collection face indexing.

## Reproducibility

`src/generated.rs` is produced by:

```sh
python3 tools/gen_tables.py \
  --afm-dir <dir with the seven AFMs> \
  --glyphlist <glyphlist.txt> \
  --out src/generated.rs
```

Running it twice on the same inputs gives byte-identical output (verified with
`cmp` against the committed file). The header records the input digests and
the Python `unicodedata` version (15.0.0 on the generating machine); a
different Python may change the composition table, which the header would show.
`lib.rs` marks the module `#[rustfmt::skip]` so `cargo fmt` cannot drift it.
Aliases added by the generator (documented in `ALIASES`): U+00A0 → space,
U+00AD/U+2010/U+2011 → hyphen, U+0394 → Delta, U+03A9 → Omega, U+03BC → mu,
U+02C9 → macron.

Known AFM quirk: Adobe's 1997 AFMs list `Euro` with `WX 500` and an empty
bounding box in every text face (a placeholder); the tables carry it as-is, so
U+20AC measures 500 units while Apple's Times draws a 744-unit Euro.

## Latin Modern versus CoreText (same file, 10 pt)

| text | mode | coretext | engine | delta |
| --- | --- | --- | --- | --- |
| "Hello" | plain | 22.5000 | 22.5000 | 0.0000 |
| "AV fi fl ffi ffl" | plain | 57.8000 | 57.8000 | 0.0000 |
| "The quick brown fox … fluffy waffle." | plain | 337.2200 | 337.2200 | 0.0000 |
| "Café naïve façade — “quoted” … 100% €5 ™" | plain | 197.2200 | 197.2200 | 0.0000 |
| "AV" | shaped | 13.8900 | 13.8900 | 0.0000 |
| "ffi" | shaped | 8.3300 | 8.3300 | 0.0000 (was +0.2800 before per-lookup passes) |
| "AV fi fl ffi ffl" | shaped | 54.9900 | 54.9900 | 0.0000 |
| "The quick brown fox … fluffy waffle." | shaped | 329.9500 | 329.9500 | 0.0000 |

The two shaped mismatches the first run showed (+0.28 pt wherever `ffi`
occurred) were engine bugs and are fixed: language-specific `liga` records were
being unioned in, and ligature lookups were applied in one combined pass instead
of sequentially. `tests/latin_modern.rs` pins these CoreText numbers.

`latinmodern-math.otf` MathConstants (font units): axisHeight 250,
fractionRuleThickness 40, fractionNumeratorShiftUp 394 (display 677),
fractionDenominatorShiftDown 345 (display 686), superscriptShiftUp 363,
subscriptShiftDown 247, radicalRuleThickness 40, radicalVerticalGap 50 (display
148), radicalKernBeforeDegree 278, radicalKernAfterDegree −556,
scriptPercentScaleDown 70, scriptScriptPercentScaleDown 50 — all verified
against an independent parse of the file, not from memory.

## Engine comparison (CoreText)

`examples/compare_coretext.swift` measures each case twice per side: `plain`
(sum of per-glyph advances, no kerning/ligatures: `CTFontGetAdvancesForGlyphs`
vs `ShapeOptions::PLAIN`) and `shaped` (`CTLineGetTypographicBounds` with
CoreText's default kerning/ligatures vs `ShapeOptions::default()`). Text is
passed to the Rust side on stdin so macOS cannot NFD-rewrite it. Run on
macOS 26.3.1, 12 pt, delta = engine − CoreText in points:

| face | mode | coretext | engine | delta | note |
| --- | --- | --- | --- | --- | --- |
| Times New Roman.ttf | plain | 26.6602 | 26.6602 | +0.0000 | "Hello" |
| Times New Roman.ttf | plain | 376.2246 | 376.2246 | −0.0000 | "The quick brown fox jumps over the lazy dog. AVATAR office, fluffy waffle." |
| Times New Roman.ttf | plain | 231.3340 | 231.3340 | +0.0000 | "Café naïve façade — “quoted” … 100% €5 ™ e◌́" |
| Times New Roman.ttf | plain | 52.3184 | 52.3184 | +0.0000 | "AV fi fl ffi" |
| Times New Roman.ttf | shaped | 369.5273 | 367.9922 | −1.5351 | 3 ligatures via cmap fallback (CoreText applies none: TNR has no Latin `liga`), kern table |
| Times New Roman.ttf | shaped | 50.3379 | 48.5859 | −1.7520 | same cause: fi/fl/ffi ligated by the engine only |
| Times-Roman (Core 14 vs Apple Times) | plain | 26.6602 | 26.6640 | +0.0038 | "Hello": 26.664 pt is the AFM value exactly (2222 units) |
| Times-Roman | plain | 376.2246 | 376.2480 | +0.0234 | sentence |
| Times-Bold | plain | 391.6406 | 391.6320 | −0.0086 | sentence |
| Times-Italic | plain | 360.2754 | 360.3240 | +0.0486 | sentence |
| Times-BoldItalic | plain | 376.3125 | 376.3320 | +0.0195 | sentence |
| Helvetica | plain | 394.1719 | 394.1640 | −0.0079 | sentence |
| Courier | plain | 532.8867 | 532.8000 | −0.0867 | sentence (60 glyphs × 0.5-unit rounding bound = 0.36 pt) |
| Symbol | plain | 33.9316 | 33.9360 | +0.0044 | "αβγ ∑∫" |
| Times-Roman | plain | 238.2598 | 231.3600 | −6.8998 | accented string: AFM Euro placeholder (−2.93 pt) and U+0301 missing in PLAIN mode (Core 14 has no combining acute; CoreText falls back) |
| Times-Roman | shaped | 368.0098 | 366.8400 | −1.1698 | sentence: Adobe AFM `KPX` pairs differ from Apple Times' kern pairs; both ligate fi/fl/ff |

Summary: TrueType advances are exact against CoreText (0.0000 pt on every
plain case, same program on both sides). Core 14 tables versus Apple's
different Times/Helvetica/Courier/Symbol programs agree within 0.05 pt on ASCII
sentences (≤ 0.09 pt for 60 Courier glyphs), as expected for 1/1000-em
rounding. Shaped deltas of 1–2 pt on a 60-glyph line are kerning-table and
ligature-policy differences, itemised above, not measurement error. Full
output: run the script; the table above is the 2026-09-12 run.

## Tests

`cargo test` — 63 tests (+ 2 `compile_fail` doctests):

* unit (8): SHA-256 vectors; Core 14 values equal to compiler `metrics.rs`;
  synthetic gid round trip for all seven faces; AFM kerning; stable identity;
  manifest JSON round trip and stability; bounded manifest paths; OT1/T1 tables.
* `tests/pinned.rs` (6): committed `fonts/manifest.json` equals the generator
  output and parses back; the pinned set loads with every field verified and
  refuses tampered glyph counts, digests and licence hashes; fallback chain
  order, per-cluster FontId, missing on primary, determinism; OT1/T1 → Unicode
  → LM gid mapping (and the three T1 slots LM lacks); subset + ToUnicode bytes
  identical across runs for glyf and CFF; the `ec-lmr10` TFM fixture's hashes,
  GIDs (re-derived through cmap) and widths (within half a unit of the OTF),
  and its A/V kern equal to GPOS.
* `tests/core14.rs` (14): "Hello" = 26.664 pt and "Mac" = 21.324 pt; ASCII and
  Latin-1 coverage of every text face; Unicode beyond Latin-1 (em dash, quotes,
  Euro, Greek Delta/Omega, summation; CJK absent); missing glyphs listed with
  byte offsets and never replaced by `?`; `e` + U+0301 ≡ `é`; unknown mark
  stays in its cluster as missing; `fi` on/off with identical source bytes;
  AV kerning −135 and optional; Hebrew/Arabic/bidi controls/Devanagari fail
  closed; default ignorables keep bytes; cluster hit testing; AFM header
  metrics; Courier never ligates.
* `tests/latin_modern.rs` (9, skip if BasicTeX's Latin Modern is absent): OTTO
  parse (`Outlines::Cff`, PostScript name, declared cap/x height, explicit
  unsupported note); "Hello" 10 pt = 22.5000 pt and the 74-glyph line = 337.22 pt
  within 0.01 pt of CoreText on the same file; AV via GPOS and 13.89 pt CTLine
  match; `fi` and two-step `ffi` (833 units) via GSUB with the cmap fallback off,
  and the full line at 54.99 pt; é/— in cmap with byte-accurate clusters and
  composition; raw `CFF ` bytes equal to the directory entry, glyf-only
  operations refused; whole-program embedding with identity CIDs, unkerned /W
  widths and "ffi" in ToUnicode; MathConstants and italics corrections equal to
  the file; every listed LM text face parses and shapes, monospaced ones never
  ligate.
* `tests/truetype.rs` (18, each skips with a message when its font is absent):
  metrics and content identity; "Hello" within 0.05 pt of Core 14; AV kerning
  negative via kern table; `fi` one glyph vs two with the same source bytes;
  mark composition via cmap; é/—/中 in Arial Unicode with byte-accurate
  clusters and a 1 em ideograph; missing glyph → `.notdef` listed;
  deterministic subsetting (order/duplicates irrelevant, byte-identical output,
  checksums verified, subset re-parsed with advances preserved, distinct tags);
  ToUnicode round trip including "fi" and "e◌́" multi-character entries and a
  CID for every shaped glyph; `.ttc` face indexing; bounded resolution; malformed
  and OTTO inputs are errors; GPOS PairPos in Arial equal to its legacy table;
  GPOS in Times New Roman Italic; GPOS Extension in Iowan Old Style; TNR's
  Latin ligatures come from cmap, not its Arabic-only `liga`; a real Latin GSUB
  `liga` (Iowan, fi → gid 192) with the cmap fallback disabled; GPOS
  MarkToBase places x + U+0301 at the anchor pair. (TNR's
  Arabic-only `liga` is no longer selected at all now that features go through
  the default language system.)

## Relationship to sibling crates

* `crates/pdf` (branch `agent/mac-pdf/pdf-output`, `5b5f7b5`) has its own
  `truetype.rs`/`embed.rs`. This crate reuses the same sfnt writer and checksum
  approach and extends it (explicit maps, composite bbox, multi-char ToUnicode,
  `verify_checksums`). `crates/pdf` could delegate to `subset` and `embed` here
  and drop its copies; that is its owner's call.
* `crates/fontmetrics` (branch `agent/mac-claude-a/fontmetrics`, `30a14a6`) is
  the superseded CoreText-measured proposal; this crate replaces it with
  AFM-exact tables plus a CoreText cross-check rather than CoreText-derived data.
* `crates/compiler/src/metrics.rs` (`de1020c`) stays the compiler's own table
  until its owner chooses to consume this crate.
