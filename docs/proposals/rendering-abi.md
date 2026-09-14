# Rendering ABI: what `crates/render-pipeline` emits, and what it asks of its siblings

Status: proposal / deviation register maintained by mac-render-pipeline.
Authoritative contracts remain `docs/contracts/runtime-v1.md`,
`docs/contracts/runtime-v1-layout-capabilities.md`,
`docs/contracts/rendering-v2-proposal.md` and
`protocol/rendering-v2.schema.json` (validated by `crates/rendering-core`).
Reviewed on `origin/main` `7fea005b0611e2cc565ec38b9a705eff7cc4532c`
(rendering-core, font-resources, compiler `9026d8a`); re-checked at `8e2c70a`:
the schema's `format` is still `const "static-truetype"`, while font-resources
now carries staged CFF1 parsing (`cff::Cff::parse` over font-engine's
`cff_table()`, "not production activated") — the request below is the
activation of that path for the display list.

## 1. Display list v2 as emitted (`crates/render-pipeline/src/display.rs`)

Envelope: `{"protocol_version":2,"id":…,"type":"display_list","payload":{…}}`
with `render_format:"display-list-v2"`, `coordinate_unit:"bp_2pow20"`,
`color_space:"srgb"`, `text_extraction:"cluster-actualtext"`,
`required_features`, `documents[]`, `fonts[]`, `pages[]`, `diagnostics[]`.

| schema element | pipeline behaviour | status |
| --- | --- | --- |
| ticks | `Tick(i64)`, 1 048 576 per PDF point; one rounding point (`Tick::from_tex_pt`, round-half-away) from f64 TeX points; same input, same output | conforms |
| pages | contiguous 1-based numbers, width/height in ticks (US Letter 612×792 bp) | conforms |
| `glyph_run` | `font_id`, `font_size` ticks, logical `text`, glyphs `{gid, origin_x, baseline_y, advance_x, advance_y, cluster}` with **absolute** origins and original GIDs, clusters `{text_start_byte, text_end_byte, hit_rects[], carets[], provenance}` | conforms; `gid: 0` never emitted (missing glyphs are `missing_glyph` diagnostics and draw nothing) |
| `rule` | `{x, top, width, height, paint, provenance}`; top-left, positive dimensions; math-layout's fraction/radical rules | conforms |
| `hit_rects[]` | each is the cluster's **laid-out TeX box**: `width` the advance TeX positions the next atom from (the TFM width, pdfTeX's `/Widths`), `top` the baseline minus the box height, `height` the box height plus depth. Uniform across text, single math glyphs and stacked extensible delimiters | conforms; the schema now says so (`$defs.cluster.hit_rects.description`). Until #273 a single math glyph reported the painted outline's ink instead, which made `\Bigg(` measure 29.90 pt inside its exact 30.00029 pt box and cost three lanes a false defect report |
| `ink_rect` | the painted outline's tight bounding box, when the producer knows it | **proposal**, off unless `display-list-v2-ink-rect` is negotiated (`protocol/proposals/display-list-v2-ink-rect.md`); `crates/rendering-core`'s `deny_unknown_fields` must learn the key before it can be accepted |
| provenance | `Sources[{path,start_byte,end_byte}]` per cluster into the declared document revisions (multi-file `\input` keeps each cluster's own path); `Synthetic(reason)` reserved, currently unused | conforms |
| `documents[]` | `{path, revision, sha256, byte_length}` of the exact request text | conforms |
| `fonts[]` | `{font_id, sha256, byte_length, format, face_index:0, units_per_em, glyph_count, postscript_name}` where `font_id == sha256` = SHA-256 of the raw file bytes (font-engine's bytes‖face_index id is kept separately as `engine_id`, not published) | **deviates**: `format` is `"opentype-cff"` for Latin Modern (Roman and Math); Times is `"core14-afm"` with `byte_length: 0` (metrics only, no glyph bytes — the schema requires ≥ 1) |
| `required_features` | `glyph_run`, `rule` (when present), `rgba-srgb`, `cluster-actualtext`; `static-truetype` only if such a font is used (never, today) | **deviates**: rendering-core marks `static-truetype` as used by every glyph run |
| diagnostics | `{severity, code, message, sources[], recovery}`; codes are identifiers | conforms |

Validation evidence (`crates/render-pipeline/docs/oracle-evidence.md`): with
the `format` token alone rewritten, all 18 corpus envelopes pass
rendering-core's `validate_display` (`valid_experimental_roundtrip`). The
font profile is the single blocking deviation.

### Requested schema / rendering-core changes

1. Add `"opentype-cff"` (sfnt `OTTO`, face 0) to the `format` enum and a
   `opentype-cff` feature next to `static-truetype`. Latin Modern — the
   product's default face — exists only as OpenType CFF and Type 1. Glyph
   ids are the CFF charset order, which is the OTF's glyph order, so
   original-GID semantics are unchanged.
2. Resource location stays out of the envelope (font-resources separates
   paths from descriptors); the pipeline's `FontResource.path` is a local
   convenience for `--pdf` and tests only.
3. Metric-only fonts: either allow `format: "core14-afm"` with
   `byte_length: 0`, or the pipeline drops Times entirely once Latin Modern
   is bundled with the app. Preference: drop Times.

### Requested font-resources change

`inspect_opentype_cff(&[u8]) -> FontMetadata` (and `FontResource::from_bytes`
accepting it): parse the `OTTO` table directory, take `units_per_em` from
`head`, `glyph_count` from `maxp` cross-checked with the `CFF ` charstring
count via the existing `cff::Cff::parse`. The pipeline's `src/cff.rs`
(Type 2 charstring bounds, 668 lines) duplicates `font-resources/src/cff/type2.rs`
and should be deleted in favour of it once glyph bounds are exposed there
(`bounds(gid) -> [x_min, y_min, x_max, y_max]`).

## 2. Runtime-v1 mapping (`src/v1.rs`, `src/protocol.rs`)

Derived from the immutable v2 list per request, so cached/incremental replies
are bound to the accepted capability set by construction.

| v2 | v1 (legacy) | v1 with `rules-v1` | v1 with `font-hints-v1` |
| --- | --- | --- | --- |
| text `glyph_run` | one `text` item: `text`, `x_pt`/`baseline_y_pt` of the first glyph, `font_size_pt`, `source` = union of the run's cluster sources (same path) | same | `+ font:{family,weight,style}` |
| math `glyph_run` | one `text` item **per glyph** (each has its own origin) | same | `+ font` (`Latin Modern Math`) |
| `rule` | `text` item of N × U+2500 at size `height/0.0857`, baseline = rule bottom (the compiler's convention) | `{"kind":"rule", x_pt, y_pt(top), width_pt, height_pt, source}` | — |
| diagnostics | `{severity, message, source (first), recovery, code}` | same | same |

`layout_capabilities` on the reply is present iff the request carried the
field; it lists the accepted requested names in request order and never an
unrequested one. Invalid lists (not an array, non-string, empty, > 64
bytes, duplicate, > 16 entries) fail that request with `status: "failed"`.

## 3. Requested compiler API (`crates/compiler`)

The adapter re-derives from source bytes what the parse tree drops. Each
item is exact today because spans are exact, but it is duplicated logic:

1. Style scopes: `\textbf`/`\emph`/`\textit` (and later `\texttt`,
   `\textsc`, size commands) as `Inline::Text { style }` or scoped groups.
2. Interword gaps: whether whitespace separated two `Inline::Text`s (TeX's
   rules for `{`, `}`, control words, `%` comments, `\input` boundaries).
3. Preamble facts in `Parsed`: class options, `\setlength{\parindent}`,
   `\setcounter{secnumdepth}`, `geometry` options — instead of
   "unsupported in the preamble" diagnostics plus source scanning.
4. Page-break commands (`\newpage`, `\clearpage`, `\pagebreak`) as block
   boundaries instead of unsupported-command errors.
5. Number only `equation` displays: `parser.rs` increments the equation
   counter for every closed `\[`/`$$`, which LaTeX does not.
6. Math: `\left`/`\right`, Greek letters and the plain.tex symbol control
   words (`\nu`, `\alpha`, `\leq`, …) in `math::parse_tokens`; today they
   are errors and the corpus fixtures 13/14 cannot be typeset faithfully.
7. Lists: `itemize`/`enumerate` structure (nesting depth, marker) rather
   than a marker word prepended to a paragraph.

## 4. Requested sibling APIs

- **paragraph-layout** (`70209e2`): a penalty-based page builder (TeX
  §980–1028; `render-pipeline/src/pagebuild.rs` is a working reference:
  badness/cost, `\clubpenalty`/`\widowpenalty` as costs, `\nobreak`,
  `\predisplaypenalty`, per-block `\baselineskip`, `\nointerlineskip`,
  `\topskip`, `\maxdepth`, `\raggedbottom`), replacing `layout_pages`'s hard
  club/widow minimums. `Lines`/`Stats` field additions since `7af5c05`
  (`hfuzz`, `hbadness`, `diagnostics`, `emergency_pass_used`) were adopted.
- **math-layout** (`db90047`): none new; `positioned_runs` + explicit rules
  are what v2 needs.
- **font-engine** (`f418238`): none new. The `paragraph`/`math`/`pdf`
  adapter features pull those crates by `../<name>` path, which is why the
  vendored copies use plain crate names.
- **pdf** (`4bd8c2e`): a glyph-run entry point (font id, original GIDs,
  tick positions, embedded bytes) so the export stops re-encoding text by
  character; until then `--pdf` goes through the negotiated v1 route
  (`rules-v1` + `font-hints-v1`) and Latin Modern Math glyphs fall back.
- **document-style** (`bfc980d`): none.
