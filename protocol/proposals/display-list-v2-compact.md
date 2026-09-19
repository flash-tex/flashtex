# Proposal: `display-list-v2-compact` — derivable carets/hit rects, run-level sources, array glyphs

Revision r1, 2026-09-14 (lane mac-perf-4, FT-071). Status: **implemented
behind the opt-in capability** in `crates/render-pipeline`
(`src/display_list_compact.rs`, producer) and `apps/mac`
(`Sources/FlashTeXProtocol/DisplayListCompact.swift` + `RenderingV2Fast.swift`,
consumer; `DisplayListDelta.swift` accounting); `protocol/rendering-v2.schema.json`
and `docs/contracts/runtime-v1*.md` are unchanged in meaning. A consumer that
does not send the name sees byte-identical replies (measured: the reply-stream
MD5 of every seed and mode is the same on `origin/main eea975c3` and on this
branch, `docs/evidence/perf-mac-2026-09-14T0100Z`).

## Why (measured)

The rendering-v2 `display_list` line costs ~420 bytes per glyph on this
producer (`docs/evidence/perf-mac-2026-09-13T1740Z`, addendum table): per
cluster, `carets` 85.8 B (20.4 %), `hit_rects` 79.1 B (18.8 %), `sources`
64.8 B (15.4 %), the cluster's own `text_*_byte` + framing 41 B (9.8 %), and
the glyph object 101 B (24.1 %). HW1 (3 pages) is 1 152 024 B, demo (2 pages)
1 941 498 B. The 60 KB body (`tools/typing-bench/run.sh`'s `body60k`, the
demo's paragraphs repeated to ≥ 60 KB: 22 pages) is estimated at ~21 MB by
`estimated_json_bytes`, over the 16 MiB reply cap (`protocol::MAX_REPLY_BYTES`,
`JSONLines.maxLineBytes`), so the producer declines `display-list-v2`
(`display_list_declined`) on **every** request and the Mac falls back to the
v1 pane: 2 494 207 B of v1 `pages` per keystroke and no v2 frame at all.
`display-list-v2-delta` cannot help there — a delta needs an installed base,
and the first full frame never fits — and `-only` elides v1 pages only when a
sibling exists. Within the frozen schema nothing is optional: `clusters`,
`carets`, `hit_rects` and `sources` are required, so dropping them "when
derivable" needs a negotiated encoding, which is this proposal.

PR #232 (FT-070 memory, `agent/linux-primary/ft070-memory`) measured the
invariants this encoding derives from on 1 991 552 corpus clusters with zero
exceptions: `carets.first` **is** `hit_rect` + `text_start_byte`; the end
caret sits only on a run's last cluster with that cluster's `top`/`height`;
what is not derivable is the end caret's `x` (the TikZ path clamps the hit
width to one tick but not the caret) and which run carries it. #232 keeps the
`hit_rect` in memory and derives the carets (`GlyphRun::end_caret`,
`Cluster::first_caret`); this proposal is the same split on the wire, and the
producer here is written so that #232's `carets_of(i)` slots in for the two
`c.carets` reads in `write_glyph_run` — nothing else in it depends on stored
carets.

## Negotiation

- Request: `payload.layout_capabilities` gains `"display-list-v2-compact"`.
  Meaningful only next to `"display-list-v2"`; listing it alone is legal but
  it is never accepted (`v1::Capabilities::negotiate`).
- Reply: accepted **iff** the reply carries a sibling line (`display_list` or
  `display_list_delta`), exactly like `display-list-v2-delta`: the echoed
  `layout_capabilities` then include the name and the sibling's payload
  carries `"cluster_encoding":"compact-1"`. When the sibling is declined for
  size or the result is `failed`, the echo omits the name and the reply is
  today's. The echo is the consumer's mode signal; the payload key is the
  line's own self-description (a stored compact line is never mistaken for
  a schema-valid rendering-v2 document: it is not one).
- Interaction with `-delta`: the compact encoding is a wire option of the
  producer's snapshot (`Wire::compact`, next to images and device colour); a
  delta is emitted only against a base emitted with the same options, and its
  `changed_pages` are compact, its payload carries the same
  `cluster_encoding` key. A consumer whose installed base is compact refuses a
  delta in another encoding (`delta_encoding_mismatch`, typed, before any
  page is built) and vice versa.
- Interaction with `-only`: unchanged; `-only` elides v1 pages whenever a
  sibling exists, compact or not.
- Interaction with images / device colour: unchanged; a compact run's `paint`
  is written by the same paint writer, rules/paths/images are untouched.
- `page_window` (reserved, **not implemented**): the memory lane (#232) and
  this lane converge on one later request field
  `payload.page_window = {"first_page": n, "count": k}` (1-based, inclusive
  count) that would let a consumer ask for a sibling carrying only the pages
  it can show, with the remaining page numbers listed but empty. Reserving the
  name here so that neither lane invents a second one; its semantics (what
  "empty page" means for hit-testing and for the delta digests) are that
  proposal's to state. A producer ignores the field until then.

## Wire shape (compact-1)

Payload: `"cluster_encoding":"compact-1"` (sorted position: after
`changed_pages` in a delta, first key of a full line's payload; before
`color_space`). Everything outside glyph runs is byte-for-byte the full
writer's.

Glyph run (keys sorted; `⋯` = unchanged from the full form):

```
{"clusters":[<compact cluster>, …],
 "end_caret":{"text_byte":T,"x":X},          -- optional: the run's last cluster carries a second caret
 "font_id":⋯,"font_size":⋯,
 "glyphs":[[gid,origin_x,baseline_y,advance_x,advance_y,cluster], …],   -- arrays, same 6 fields, same order as the schema's keys sorted
 "hit_height":H,"hit_top":T,                 -- present iff the run has clusters: the default (top,height) of every cluster's hit rect
 "kind":"glyph_run","paint":⋯,
 "sources":[{"end_byte":E,"path":P,"start_byte":S}],   -- optional: the run's source span (below)
 "text":⋯}
```

Compact cluster (every key optional; `{}` is the common case):

| key | value | default when absent |
| --- | --- | --- |
| `ts` | text start byte | the previous cluster's text end (0 for the first) |
| `l` | text byte length | the UTF-8 length of the scalar at the text start (0 if none) |
| `h` | `[x, top, width, height]` explicit hit rect | derived (below) |
| `hv` | `[top, height]` vertical override, never together with `h` | run `hit_top`/`hit_height` |
| `c` | explicit carets `[[text_byte, x, top, height], …]` (1–2) | derived (below) |
| `sources` / `synthetic_reason` | explicit provenance exactly as the full cluster's | implicit (below) |
| `s` | source start delta from the chain | 0 |
| `e` | source range length | the cluster's text byte length |

Derivation, per run, clusters in order; `prev_text_end` starts at 0,
`prev_src_end` at the run span's `start_byte`:

```
text_start = ts ?? prev_text_end
text_end   = text_start + (l ?? scalar_len(text, text_start));  prev_text_end = text_end
hit_rect   = h ?? { x:      origin_x of the cluster's FIRST glyph in glyphs order (0 without glyphs),
                    top:    hv[0] ?? hit_top,
                    width:  Σ advance_x over the cluster's glyphs (0 without glyphs),
                    height: hv[1] ?? hit_height }
carets     = c ?? [ {text_start, hit_rect.x, hit_rect.top, hit_rect.height} ]
                  + (last cluster && end_caret ? [ {end_caret.text_byte, end_caret.x, hit_rect.top, hit_rect.height} ] : [])
provenance = sources / synthetic_reason if present (chain untouched)
             else one range { path: run span path, start: prev_src_end + (s ?? 0), end: start + (e ?? (text_end − text_start)) };
                  prev_src_end = end
```

`hit_rects` is always the one derived rectangle (this producer emits exactly
one per cluster). A cluster object carrying `hit_rects` is a full cluster; a
run never mixes the two forms (the reader refuses a mix).

Run-level definitions (model predicates, used by both sides):

- **run path** = the path of the first range of the first cluster that has a
  range; none if no cluster has one.
- a cluster is **implicit** iff it has exactly one range and that range is
  on the run path. Every other cluster (several ranges, another document,
  synthetic) is written with its explicit `sources`/`synthetic_reason`.
- **run span** = `[min start_byte, max end_byte]` over the implicit clusters'
  ranges, on the run path; the `sources` key is present iff at least one
  cluster is implicit. A run with no `sources` and an implicit cluster is
  malformed (refused).
- `hit_top`/`hit_height` = the most frequent `(top, height)` pair over the
  run's clusters (ties: the earliest); the choice is the producer's, the
  decoder only reads it.
- `end_caret` = the second caret of the run's last cluster, when it has one.

Producer rule (correct by construction, not by invariant): the writer runs
the derivation on the model it is about to write and emits an override for
every field whose derived value differs, so decode ∘ encode is the identity
on any glyph run. On the real-world corpus (`tests/display_list_compact.rs`,
11 fixtures, 22 616 clusters) the override counts are: `c` 0, `h` 2 (zero-
advance combining marks in `unicode-accents`), `hv` 4 450 (per-glyph vertical
extents: math, and letters of different height within a word), `l` 53
(ligatures, decomposed accents), `s` 2 118 and `e` 4 386 (math: spaces and
macros between consecutive glyph spans), `ts` 0, explicit provenance 0 —
i.e. the #232 invariants hold on this producer too.

## Interaction with `display-list-v2-delta` — exact `page_bytes`

r5 §6.2 charges an unchanged (relocated) page as `cached_bytes + Σ over its
relocated ranges of the decimal-width change of start/end`. Under compact-1
the per-cluster offsets are chain deltas, invariant under a uniform move, so
the rule becomes:

```
page_bytes(relocated page) = cached_bytes
    + Σ over glyph runs with a run span:        Δdigits(span.start) + Δdigits(span.end)
    + Σ over their explicit clusters' ranges:   Δdigits(start) + Δdigits(end)
    + Σ over rules/paths/images:                as in r5
```

where the span is relocated as a range (`end ≤ a` → unchanged, `start ≥ b`
→ moved). For this to be exact the run's implicit ranges must all be on the
same side of the edit; the **producer** classifies a page whose run straddles
the edit as changed (`display_list_compact::source_width_delta` returns
`None`), so the consumer never sees a straddling unchanged page and, if a
producer ever sent one, `relocate(span)` is invalid → `delta_relocation_invalid`
or the digit check fails → `delta_page_bytes_mismatch`. The fixed framing `F`
of the target's full line gains the `cluster_encoding` key's 32 bytes
(`display::full_line_frame_bytes(wire)`, `DisplayListDelta.frameConstantBytes(clusterEncoding:)`).
The `dl2-canon-1` digests are over the decoded model and are unchanged: a
compact frame and a full frame of the same model have the same digests.

Gates: Rust `deltas_reconstruct_to_the_fresh_compact_line_over_an_edit_sequence`
(60 cumulative edits on a 40-section document: 38 deltas, 22 full; every
delta reconstructs byte-identically to the fresh compact line; the width rule
verified against actual serialisation on 335 relocated pages); Swift
`DisplayListCompactProducerTests.testDeltaChainUnderTheCompactEncoding`
(real producer: reconstruction == fresh compact decode, `page_bytes` == the
fresh line's raw page ranges, exact target == fresh line length, 0-pixel
parity) and `testEncodingMismatchIsATypedRefusal`.

## Consumer rules (Mac shell, implemented)

- `RenderingV2Fast` reads compact and full runs alike into the same
  `RenderingV2.GlyphRun`/`Cluster` values; `RenderingV2.validate` runs
  unchanged on the result and refuses an unknown `cluster_encoding`
  (`unsupported_feature`). The `JSONDecoder` slow path cannot read compact
  clusters; a compact line that the fast reader refuses is therefore refused,
  never partially read.
- Every consumer of clusters — hit-testing, follow-caret, source mapping,
  `V2PageCache`, the parity gate, `dl2-canon-1` — is untouched: it sees the
  derived values. `DisplayListCompactProducerTests` asserts, against the real
  producer over 30 062 clusters, that the compact and the full decode are
  `Equatable`-equal and that every caret/hit-rect coordinate agrees within
  1e-6 bp (they are integer ticks, so exactly).
- Requested per request next to `display-list-v2` (a per-request capability
  like `-delta`/`-only`, never part of `requestedLayoutCapabilities`;
  `DisplayListDelta.perRequestCapabilities`). `FLASHTEX_DISPLAY_LIST_COMPACT=0`
  stops requesting it. **The one-line request hunk in `ShellModel.compile()`
  is parent-retained and is listed in the lane report, not applied here.**

## Compatibility

- A request that does not list the name gets byte-identical output to today:
  `write_page` dispatches to the compact writer only under `Wire::compact`;
  `Wire` defaults to `false`; the CLI `--v2` file output never sets it. Proof:
  reply-stream MD5 per seed (HW1, HW2, demo, body60k) × mode (`v2`,
  `v2+delta+only`) identical between the `origin/main` control binary and
  this branch (`raw/md5-control.txt` vs `raw/md5-branch.txt` in the evidence
  directory), plus the corpus round-trip test which also reads the full line
  back to the same model.
- rendering-core, the helper route and the CLI never request the capability
  and never see the encoding.
- A compact line is not schema-valid rendering-v2 (`glyphs` are arrays,
  clusters lack required keys); it is only ever sent to a consumer that
  requested it, and it says so in `cluster_encoding`.

## Measured (`ipc_bench.py`, 20 warm keystrokes typed before `\end{document}`, this Mac at load ~10; bytes are exact, latencies load-affected)

| seed | first frame v2 line: full → compact | warm keystroke v1 + v2 before (`--delta --only`) → after (`+compact`) |
| --- | ---: | ---: |
| HW1 (3 pages, 2 742 glyphs) | 1 152 024 → **356 207 B** (30.9 %) | 1 879 + 259 481 → **1 905 + 77 752 B** |
| HW2 (3 pages, 2 412 glyphs) | 1 029 984 → **340 583 B** (33.1 %) | 2 780 + 150 710 → **2 806 + 58 709 B** |
| demo (2 pages, 4 699 glyphs) | 1 941 498 → **540 454 B** (27.8 %) | 280 + 807 836 → **306 + 225 400 B** |
| body60k (22 pages, 51 729 glyphs) | declined (est. > 16 MiB) → **5 986 729 B, accepted** | 2 494 207 (v1 pane, no v2) → **306 + 228 992 B** (20/20 deltas) |

The 60 KB body's first frame is under the cap, so it now receives
`display-list-v2` + deltas + elided v1 pages like HW1. Its warm producer round
trip on the delta path is 138 ms p50 here (the r5 per-page `dl2-canon-1`
hashing and lockstep compare over 22 pages, plus load ~10); that is the
per-page digest cache the perf-3 addendum already names, not this encoding.

## Follow-ups (not in r1)

- `hv` is the one frequent override (~15–25 % of clusters at ~26 B); a
  per-run table of `(top, height)` pairs with a small index would take it to
  ~6 B. Left out to keep the rule set small.
- `page_window` (above).
- Once #232 lands, `write_glyph_run`'s two `c.carets` reads become
  `r.carets_of(i)`; the `c` override then never fires and could be dropped
  from a `compact-2`.
