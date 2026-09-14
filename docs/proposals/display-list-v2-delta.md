# Proposal: `display-list-v2-delta` — a bounded, opt-in delta sibling for `display-list-v2`

Revision: **r5, 2026-09-12** — amended for the Commander's r4 review (issue #2
comment 5646803033): the residency charge is no longer an estimate. Choice
**(b) exact cached accounting**: the producer, which serialises every page it
emits, carries the exact serialised byte length of EVERY page of the target in
the delta (`page_bytes[]`, accounting metadata OUTSIDE the digested semantic
model — the `dl2-canon-1` vectors are unchanged); the consumer verifies each
entry (changed pages: measured length of the received page object; unchanged
pages: cached length of the installed page plus the exact decimal-width change
of its relocated offsets — no serialisation, no estimate), mismatch =
`delta_page_bytes_mismatch`; the target's exact size = fixed framing + measured
header parts + Σ `page_bytes` + separators, computed before any allocation.
In-flight PRERASTER bitmaps (`V2Loader.preraster` runs in the same off-main
job) are added to the peak accounting. Appendix A re-run: digests identical,
exact target size equals the fresh line length. r4 was 0569b742 — amended for
the Commander's r3 review (issue #2 comment 5646664026): an over-cap reconstructed target is REJECTED before any
allocation or paint (typed `delta_target_oversize` naming the (since r5: exact) size
vs the cap, last installed frame kept marked stale, chain cleared → full
resync, which itself refuses honestly per §8); residency accounting now counts
one IN-FLIGHT reconstruction (a running off-main callback is never terminated
by replacing the queued callback) plus one queued wire line plus the painted
frame's retained resources (§6.1–6.3, §10.2, §10.3); the renderer's
machine-readable review scenarios (`crates/rendering-core/docs/handoffs/page-delta/refusal-scenarios.json`)
are referenced where they plug into the gates (§10.4). Wire shape and Appendix A
vectors unchanged. r3 was 05c0e619 — amended for the Commander's r2 response
(issue #2 comment 5646611117): residency charges the RECONSTRUCTED target and
states the peak bound (§6.2); stale refusal is preserved — a stale sibling is
never installed, no reconstruction cache (§6.1, §6.3); digests do not establish
completeness of diagnostics or any omitted part — only the fresh-full oracle
does (§5.1, §5.5, §10.1); canonical float rule fixed (−0.0 → +0.0, non-finite
refused; §5.1, Appendix A); font/profile validation named by its actual entry
points — Mac `RenderingV2.validate` → `V2FontStore.resolve` → `V2Frame.prepare`,
rendering-core `PipelineCff::bind` / `helper_candidate::bind` — and the
consequence that rendering-core binds ORIGINAL bytes (§5.4a, §10). r2 was
96e95628 (installed-base acknowledgement, §5.5, §6, §8 — §8 accepted as is);
r1 was c797c5cf. The delta wire shape is unchanged since r1; Appendix A vectors
re-run for r3 are identical.

Status: **PROPOSAL ONLY (producer side, mac-render-pipeline lane; written by the
mac-display-delta-proposal lane on `agent/mac-render-pipeline/delta-proposal`
from `agent/mac-render-pipeline/unified` @ 9aaec57a). Not on the wire. No
producer code, no runtime/helper code, no edits to Commander-owned contract files
(`docs/contracts/*`, `crates/preview-controller/*`, `crates/rendering-core/*`).
Requested by the Commander (issue #2 comment 5646362851) as one reviewable
producer/consumer schema + acceptance plan, to be co-signed by the consumer
side (mac-helper-display / Mac shell) before any wire activation.**

Base contracts, all unchanged by this proposal: runtime-v1, its layout
capabilities, `docs/contracts/runtime-v1-display-list-v2.md` (the full sibling
line and its decline fallback), `crates/preview-controller/docs/display-forwarding.md`,
`crates/document-runtime/docs/display-candidates.md`, `producer-size-contract.md`.

## 0. Ten-line summary (for relay)

1. New sibling opt-in layout capability `display-list-v2-delta`; only meaningful next to `display-list-v2`; unknown to old producers, never sent by old consumers; declined → today's full `display_list` line or today's `display_list_declined` fallback, byte-for-byte unchanged.
2. When accepted, the ONE sibling line after `compile_result` is `type: "display_list_delta"` (protocol_version 2) instead of `display_list`; the echoed capability list says which of the two the consumer must expect — a mismatch is a protocol violation.
3. A delta names an INSTALLED base exactly (`request_id`, `project_id`, `revision`, `page_count`, `list_digest`): the snapshot the consumer acknowledged as installed in the request's `display_list_base` field (installed = the frame currently published in the v2 pane after full validation and digesting; a stale sibling is never installed, a producer write is never installation), which must also be the producer's last emitted sibling; each side holds at most old + new (§6.2); any reply without a sibling, a stale sibling, a restart or a session change clears the chain (full resync).
4. A delta carries the complete header (`documents`, `fonts` = the full resource closure, `diagnostics`, `required_features`), the full ordered `page_count`, a digest for EVERY page of the new list, the changed pages in full (exact page objects of the full reply), the explicit `removed_pages`, and per-document source relocations (`edit_start`, `edit_end`, `delta`) so unchanged pages' source spans are moved exactly as the producer's own block cache moves them.
5. Digests (`dl2-canon-1`) are SHA-256 over a specified canonical binary encoding of the semantic model (glyph runs, rules, clusters, hit rects, carets, source spans, paint, fonts, documents, diagnostics) — implementable identically in Rust, Swift and the Python reference in Appendix A, with test vectors; digests prove the consumer rebuilt what the producer sent, never that the producer sent everything (that is the fresh-full oracle's job, §10.1 P2).
6. Reconstruction yields COMPLETE new semantics — documents, fonts/resource closure, diagnostics, required_features and every page's full model are produced by the reconstruction (changed pages as sent, unchanged pages relocated), nothing inherited implicitly — then the normal full validation. Invariant: `to_json(reconstructed)` is byte-identical to a fresh full compile's `display_list` line (same id), including unchanged pages, resources, source spans and diagnostics — proven with the existing `tests/incremental.rs` gate shape (200 edits × 27 pages; 30 × 107).
7. Refusals are typed: the producer never emits a delta without a snapshot of its own (no snapshot → the unchanged full line, `status` stays `ok`, no diagnostic); the consumer verifies the delta's `base` against the snapshot it holds and refuses `delta_base_mismatch`, `delta_page_count`, `delta_relocation_invalid`, `delta_digest_mismatch`, `delta_list_digest_mismatch`, `delta_oversize`, `delta_unsolicited` and resyncs by requesting without `-delta` (→ full).
8. Bounded state: every snapshot is charged by its EXACT reconstructed serialised size BEFORE allocation — no estimator: the delta carries `page_bytes[]` (exact per-page serialised lengths, verified by the consumer, outside the digests) and the target size is framing + measured header parts + Σ `page_bytes` (`MAX_SNAPSHOT_PAGES` 1 024, `MAX_SNAPSHOT_BYTES` 16 MiB, retained texts ≤ 8 MiB each / 32 MiB total); an over-cap target is refused, never built or painted (`delta_target_oversize`); consumer peak = `installed` (model + painted raster) + one in-flight reconstruction (input line + target + its PRERASTER bitmaps) + one queued wire line; producer peak = `old` + `new` + the line being written + the request line; eviction order in §6.2.
8b. The full resync path never truncates: when the full reply itself exceeds a limit, the reply is the existing typed refusal naming that limit (`display_list_declined` / `failed` / the runtime's `serialization_refused`), the consumer keeps its last installed frame marked stale and drops its base, and no page is ever omitted to fit (§8).
9. Visibility page filtering is NOT this proposal: a filtered view is an incomplete view, never a complete compile, and never authorizes source actions outside its validated coverage; a reconstructed delta result IS a complete compile because it is verified equal to one.
10. Acceptance is three separate gates — producer (cargo, byte identity + digests + refusals), consumer (Swift, reconstruction equality + typed refusals + V2Parity 0 px), transport (size/latency measured on the direct route and, after an FT-049 runtime change to accept the new sibling type, the helper route) — nothing about size or latency is claimed from this schema.

## 1. Why (measured, not promised)

`crates/render-pipeline/docs/oracle-evidence.md` at 9aaec57a: with all three
caches a one-word edit on the 27-page document costs 27.1 ms in-process, 33.6 ms
wall over the protocol; of that, ~12 ms is producing the reply lines and ~6 ms
is request parse plus moving ~4.3 MB of reply through the pipe. The consumer
then decodes and validates the whole frame again. The runtime note
`producer-size-contract.md` shows the other end of the scale: a 26-page fixture
whose v2 line is 25 120 854 bytes is declined at 16 MiB and would exceed the
runtime's 8 MiB framed default anyway. A delta reply that carries only what
changed is the remaining lever the producer lane identified; the Commander
ruled that it must be bounded, opt-in, digest-verified, and must never replace
or truncate the full reply. This document is that ruling turned into a schema.

Explicitly out of scope: the runtime-v1 `compile_result` (the product preview)
stays a full reply; nothing here changes its bytes or size. Page filtering by
visibility (§9) is a different feature and is not proposed.

## 2. Identity (immutable, all present on every delta)

| identity | where | meaning |
|---|---|---|
| document/source | `documents[]` = `{path, revision, sha256, byte_length}` of EVERY request document (complete set, as in the full line) | raw UTF-8 SHA-256 and length of the request text this compile ran on; `revision` is the compile revision (runtime-v1), not an editor document version |
| compile | envelope `id` = the compile request id; `project_id`, `revision` = the `compile_result`'s | the same rule 1 as the full sibling |
| session | not on the wire (the full line has none and stays unchanged); bound by the consumer to the transport it owns: the worker process it spawned (direct route) or the helper `session_id` (helper route) | a restarted producer has no snapshot and answers full; a consumer drops its base on any restart/reattach/session change |
| installed base | request `payload.display_list_base = {request_id, project_id, revision, page_count, list_digest}` (§3); delta `base` = the same five fields | the consumer's installed-base acknowledgement: the one snapshot it has decoded/reconstructed, validated (§5.4a entry points), digested, PUBLISHED as the current v2 frame, and retained as its sole installed base. The producer emits a delta only against a base that is BOTH acknowledged installed by the request AND its own last emitted sibling on this stream; the consumer verifies the delta's `base` field-by-field against the snapshot it acknowledged |
| new snapshot | `page_count`, `page_digests[1..N]`, `list_digest` | what the reconstructed list must hash to; becomes the next base on both sides |

`list_digest` binds the header (including `project_id`/`revision`, documents,
fonts, diagnostics, features) and every page digest in order, so two snapshots
with the same pages but different diagnostics or revision are different bases.

## 3. Negotiation

- Request: `payload.layout_capabilities` gains the string `display-list-v2-delta`,
  and — only when that string is present — the request carries ONE additive
  optional field, the **installed-base acknowledgement**
  `payload.display_list_base = {"request_id", "project_id", "revision",
  "page_count", "list_digest"}`. Old producers ignore both (unknown capability
  names are never accepted, `v1.rs:55`; unknown payload keys are read with
  `payload.get`, `protocol.rs:117-119`, and the compiler's negotiation ignores
  unknown names, `crates/compiler/src/protocol.rs:275`). A request listing
  `-delta` without `display-list-v2`, or without `display_list_base`, is legal
  but `-delta` is never accepted for it.
- What the acknowledgement asserts (consumer obligation): the named snapshot is
  INSTALLED — the consumer decoded (or reconstructed) it, ran the unchanged full
  validator on it, computed its `dl2-canon-1` digests, published it as the
  current v2 frame (a sibling that was stale by the time it validated is never
  installed — §6.1), and retains it as its sole installed base. Receipt, admission, a helper's
  write, or a queued-but-unvalidated frame is NOT installation and must not be
  acknowledged.
- What the producer may assume from it: only that the consumer can reconstruct
  against exactly that snapshot. Not that the frame was painted, not pixel
  parity, not that any later sibling arrived. The producer never assumes
  installation from its own write: an emitted sibling is "emitted", never
  "installed", until a later request acknowledges it.
- Consumer rule: send `-delta` + `display_list_base` only while (a) the v2 pane
  is consumable and (b) it holds an installed base; the acknowledged base is
  always the installed one, never a candidate still validating. Otherwise
  request `display-list-v2` alone.
- Producer decision per request, in order: `display-list-v2` accepted and
  result non-`failed` → `-delta` requested with a well-formed
  `display_list_base` → the producer holds a last-emitted snapshot and ALL five
  acknowledged fields equal it (an acknowledgement of an older or unknown
  snapshot, or of one the producer evicted, means no delta) → document set
  (paths) identical → relocation computable (§5.3) → delta serialised and ≤
  line limit → delta smaller than the full line would be (policy; the crate
  constant, proposed 3/4) → **accept**: echo both `display-list-v2` and
  `display-list-v2-delta`, emit ONE `display_list_delta` line. Any step fails →
  **do not accept**: echo omits `-delta`, and the reply is exactly today's: the
  full `display_list` line, or the existing `display_list_declined` warning +
  `recovered` when that would be oversize (§8).
- Pipelining consequence (stated, not hidden): with one request in flight the
  consumer can only acknowledge N−1 while the producer's last emitted sibling is
  N, so the producer answers N+1 in full. Deltas are therefore a steady-state
  feature of one-request-at-a-time use (the Mac shell's typing loop); they are
  never wrong under pipelining, only absent.
- Acceptance is bound to the applied result (layout-capabilities contract);
  a stale reply never changes the consumer's mode.
- Ordering on the stream is unchanged: `compile_result(id)` then the sibling
  `(id)`, contiguous, before any reply to a later request. Exactly one sibling
  per accepted non-failed result, of the type the echo announced.

The producer's wrong-base refusal is the silent full reply: an acknowledgement
that does not equal its last emitted snapshot (first request, after a restart,
after eviction, after a dropped or unvalidated sibling, under pipelining) is
the normal path, the full line is the complete answer, and `status` stays `ok`
with no diagnostic. The consumer-side typed refusal `delta_base_mismatch` (§7)
remains for the residual case where a delta arrives naming a base the consumer
no longer holds (a consumer that changed its installed base between sending
the acknowledgement and receiving the reply — forbidden by §6.1 — or a
duplicated/replayed request); it is evidence of a bug, not a normal path.

## 4. Wire shape of `display_list_delta`

```
{"protocol_version":2,"id":"<compile request id>","type":"display_list_delta","payload":{
  "render_format":"display-list-v2",            -- the format of the RECONSTRUCTED list
  "coordinate_unit":"bp_2pow20","color_space":"srgb","text_extraction":"cluster-actualtext",
  "project_id":"…","revision":N,                -- this compile (= compile_result)
  "required_features":[…],                      -- of the reconstructed list
  "digest_scheme":"dl2-canon-1",                -- §5.1; unknown scheme → refuse
  "base":{"request_id":"…","project_id":"…","revision":N-1,"page_count":B,"list_digest":"<hex64>"},
  "documents":[…],                              -- complete, as in the full line
  "fonts":[…],                                  -- complete resource closure of the reconstructed list
  "diagnostics":[…],                            -- complete, as in the full line
  "relocations":[{"path":"main.tex","edit_start":a,"edit_end":b,"delta":d}, …],   -- ≤ 1 per document
  "page_count":N,                               -- full ordered page count of the reconstructed list
  "page_digests":["<hex64>", … N entries],      -- page 1..N, ALL pages
  "page_bytes":[int, … N entries],              -- exact serialised byte length of each page object of the FULL line (accounting metadata, not digested; §6.2)
  "changed_pages":[<page>, …],                  -- full page objects, ascending number, exactly as the full line would carry them
  "removed_pages":[B'+1, …, B],                 -- explicit; must equal exactly the base numbers > N
  "list_digest":"<hex64>"                       -- of the reconstructed list
}}
```

Rules:

1. `changed_pages[].number` are distinct, ascending, within `1..N`. Unchanged
   pages are `{1..N} \ changed`, and each must satisfy `number ≤ base.page_count`.
   Page numbers are contiguous `1..N` in every list (rendering-core rule), so
   `removed_pages` is redundant with `base.page_count` and `page_count` on
   purpose: the consumer verifies it equals `[N+1 .. base.page_count]` (empty
   when `N ≥ base.page_count`) and refuses otherwise.
2. `page_digests[i-1]` is the `dl2-canon-1` page digest of page `i` of the
   reconstructed list (changed and unchanged alike).
2b. `page_bytes[i-1]` is the exact UTF-8 byte length of page `i`'s JSON object
   exactly as the producer's writer emits it inside the full line's `pages`
   array (`json::write(&page_json(p))`, `display.rs:443`): for a changed page it
   equals the length of the page object as it appears in this delta line; for
   an unchanged page it equals the length of the relocated base page (§6.2
   gives the exact arithmetic). `page_bytes` is accounting metadata: it is NOT
   an input of any `dl2-canon-1` digest, and a wrong value is a typed refusal,
   never a silent correction.
3. `fonts` is the complete closure: every `font_id` referenced by any page of
   the reconstructed list, unchanged pages included, is declared, and nothing
   undeclared is referenced (checked by the unchanged full validator on the
   reconstructed list). Fonts are small (≤ 256 × ~400 B) and are never
   delta-encoded.
4. `documents` and `diagnostics` are always complete, never delta-encoded
   (diagnostics carry relocated source spans exactly as the full line would).
5. `relocations` has at most one entry per declared document path and only
   for paths in `documents`; `0 ≤ edit_start ≤ edit_end ≤ base byte_length of
   that document`, `delta = new byte_length − old byte_length` of that document;
   a document without an entry has the same `sha256`/`byte_length` in the base.
6. Bounds (mirroring `RenderingV2.Bounds` / rendering-core): `page_count`
   0..10 000; `changed_pages` ≤ `page_count`; `relocations` ≤ `documents`
   (1..4 096); every hex digest exactly 64 lowercase hex; the whole line within
   the producer's line limit (16 MiB, `JSONLines.maxLineBytes`) — the
   consumer's framing limit is separate and is not negotiated here (see
   `producer-size-contract.md`).

## 5. Semantics

### 5.1 Digests — `dl2-canon-1`

SHA-256 over a canonical binary encoding of the semantic model, NOT over JSON
bytes (the consumer does not have the producer's JSON writer and must be able
to verify a page it relocated itself). Encoding primitives: integers as 8-byte
two's-complement little-endian (`i64`); counts as `i64`; strings as `i64` byte
length then UTF-8 bytes; paint components as IEEE-754 binary64 little-endian
**raw bit patterns after normalising negative zero**: a value equal to zero is
encoded as +0.0 (`0x0000000000000000`); NaN and ±infinity are not encodable —
the encoder refuses them (they cannot appear on the JSON wire and the consumer
validator rejects a non-finite or out-of-range paint before hashing), so no
NaN bit pattern is ever hashed. This makes the digest consistent with the
producer's model equality (`f64::eq`, which treats −0.0 == +0.0 and is what
§5.2 classification uses): two pages equal under `PartialEq` hash equal, and a
page whose only difference is the sign of a zero paint component is the same
page on the wire. Domain-separated prefixes:

- `page_digest(page) = SHA256("flashtex:dl2:page:1\0" ‖ number ‖ width ‖ height ‖ item_count ‖ items…)`
  - glyph run: `0x01 ‖ font_id ‖ font_size ‖ text ‖ glyph_count ‖ (gid, origin_x, baseline_y, advance_x, advance_y, cluster)… ‖ cluster_count ‖ per cluster (text_start_byte, text_end_byte, hit_rect_count, (x, top, width, height)…, caret_count, (text_byte, x, top, height)…, provenance) ‖ paint(r, g, b, a)`
  - rule: `0x02 ‖ x ‖ top ‖ width ‖ height ‖ paint ‖ provenance`
  - provenance: sources → `0x10 ‖ count ‖ (path, start_byte, end_byte)…`; synthetic → `0x11 ‖ reason`
- `header_digest(list) = SHA256("flashtex:dl2:header:1\0" ‖ render_format ‖ coordinate_unit ‖ color_space ‖ text_extraction ‖ project_id ‖ revision ‖ features(count ‖ str…) ‖ documents(count ‖ (path, revision, sha256, byte_length)…) ‖ fonts(count ‖ (font_id, sha256, byte_length, format, face_index, units_per_em, glyph_count, postscript_name)…) ‖ diagnostics(count ‖ (code, message, severity, sources(count ‖ (path, start_byte, end_byte)…) ‖ [suggestion iff `display-list-v2-diagnostics` is negotiated and the value is a non-empty string])…))`
- `list_digest(list) = SHA256("flashtex:dl2:list:1\0" ‖ header_digest (32 raw bytes) ‖ page_count ‖ page_digest[1] ‖ … ‖ page_digest[N] (32 raw bytes each))`

Every field of the wire model is covered (glyph runs, rules, clusters, hit
rects, carets, source spans and synthetic reasons, paint, fonts, documents,
diagnostics, features, identity); `page_bytes` (§4 rule 2b) is deliberately
outside the digested model — it describes the serialisation, not the
semantics, and is verified separately (§6.2). What a digest proves: that the list the
consumer holds after reconstruction is, field for field, the list the producer
computed digests over. What it cannot prove: that the producer's list is
complete relative to a fresh full compile — a producer that dropped a
diagnostic, a font or a page from both the delta and its digests would still
verify. Completeness of diagnostics and of every omitted/relocated part is
established only by the fresh-full oracle in the producer gate (§10.1 P2),
never by digest agreement. The reference implementation is Appendix A;
its test vectors are the worked example's digests (Appendix B). Both producer
(Rust) and consumer (Swift) implementations must reproduce those vectors; a
scheme change is a new name (`dl2-canon-2`), never a silent change.
Optional negotiated fields are not a scheme rename: `page_digest` already
includes image items iff `display-list-v2-images` is on the wire, and
`header_digest` includes `suggestion` iff `display-list-v2-diagnostics` is
on the wire and the value is a non-empty string. Diagnostics-off bytes and
digests stay the Appendix B `dl2-canon-1` vectors.

### 5.2 Relocation (what "unchanged page" means)

The producer's block cache (`src/incremental.rs`) reuses a block after an edit
by moving every source offset by the block's byte delta (`relocate_block`,
`place_item`). A page after the edit is therefore typically identical to the
base page except that every source span moved by the same integer. The delta
encodes exactly that move per document as `{edit_start: a, edit_end: b,
delta: d}` in the BASE document's byte coordinates (`[a, b)` is the replaced
region of the old text; the new region is `[a, b + d)`); the producer computes
it as the common-prefix/common-suffix diff of the previous and new request
text of that document (`a` = common prefix length; `b` = old length − common
suffix length, with `a + suffix ≤ min(old, new)`; `d` = new − old).

Applying a relocation to one `SourceRange {path, start_byte, end_byte}` whose
`path` has an entry:

```
if end_byte <= a:        unchanged
elif start_byte >= b:    start_byte += d; end_byte += d
else:                    INVALID for an unchanged page (the span intersects the edited region)
```

Applied to every source span of the page: cluster provenance (`sources`,
one or several ranges), rule provenance, and — for the header — nothing
(diagnostics are sent complete). Synthetic provenance and spans in documents
without an entry are untouched. `a == b` is a pure insertion; `d < 0` a net
deletion. The producer classifies a page as unchanged only when
`relocate(base_page) == new_page` as a model (Rust `PartialEq` on `Page`,
whose only floating fields are paint components) and the relocation is valid
for every span on it; otherwise the page is sent in `changed_pages`. `to_json`
is a pure function of the model, and for the ticks/offsets (integers) model
equality is byte equality; for paint, `PartialEq` equates −0.0 and +0.0 while
the writer could print them differently — the producer gate P2 compares
bytes against the fresh compile and would surface any such case; the canon
(§5.1) normalises the sign of zero so the digest side is unaffected.

### 5.3 When the producer must send full instead

Any of: no snapshot; `project_id` differs; the set of document paths differs;
a document's previous text is not retained (over the retention cap); the delta
line would exceed the line limit; the delta is not smaller than the full line
by the policy factor. The multi-edit case (two distant edits, a paste) is
handled by the same rule — `[a, b)` simply spans both edits and every page
with a span inside it is a changed page; the size policy then decides.

### 5.4 Reconstruction (consumer)

```
verify base == held snapshot (all five fields)      else delta_base_mismatch
verify digest_scheme known                           else delta_unsupported_scheme
verify page_count, changed numbers, removed_pages    else delta_page_count / delta_removed_pages
verify page_bytes[n] for every n (§6.2 exact rule)   else delta_page_bytes_mismatch(n)
charge target = framing + header parts + Σ page_bytes + separators ≤ MAX_SNAPSHOT_BYTES   else delta_target_oversize  (BEFORE any allocation)
for n in 1..=page_count:
    page = changed[n] if present else relocate(base.pages[n], relocations)   (invalid span → delta_relocation_invalid)
    verify page_digest(page) == page_digests[n-1]    else delta_digest_mismatch(n)
list = header fields from the delta + pages
verify list_digest(list) == delta.list_digest        else delta_list_digest_mismatch
run the UNCHANGED full validation on `list` — the actual entry points, §5.4a
```

Only then does `list` replace the base; painting uses the existing v2 pipeline
(font resolution by content hash, page preparation off-main, identity recheck
before paint) exactly as for a full frame. Nothing is rendered partially: any
failure keeps the previous verified frame and the base is dropped (§7).

#### 5.4a Validation entry points (named, not generic)

| consumer | entry points on the reconstructed list | what they bind |
|---|---|---|
| Mac shell (model-level) | `RenderingV2.validate(_:)` (`apps/mac/Sources/FlashTeXProtocol/RenderingV2.swift`, structural: features, bounds, ticks, cluster partition, provenance vs `documents`, `face_index == 0`, format ∈ {`static-truetype`, `opentype-cff`} paintable / `core14-afm` metrics-only) → `V2FontStore.resolve(_:)` (`GlyphRunRenderer.swift:74`, raw-byte SHA-256 + `byte_length` + glyph count + units/em + PostScript name, bytes re-hashed at load, GH31) → `V2Frame.prepare(_:store:)` (`GlyphRunRenderer.swift:233`, page preparation) | all operate on the decoded/reconstructed MODEL, so a reconstructed list takes exactly the path a full frame takes |
| rendering-core (Linux helper consumer / export) | `PipelineCff::bind(bytes, capabilities, documents: BTreeMap<String, SourceSnapshot>, resources: BTreeMap<String, Arc<CffFontResource>>)` (`crates/rendering-core/src/pipeline_cff.rs:24`) and the helper wrapper `helper_candidate::bind(event, result, current: &CurrentHelper, capabilities, resources)` (`helper_candidate.rs:64`); `validate_profile(capabilities, true)` runs inside `bind_list` (`pipeline_cff.rs:56`) | the ORIGINAL producer bytes (`PipelineCff` keeps `original: Vec<u8>`; `helper_candidate::bind` documents "Typed decoding still reads ORIGINAL bytes") plus the immutable CFF registry resources |

Consequence stated plainly: a reconstructed list has no producer bytes. The
rendering-core CFF binder therefore cannot bind a delta-reconstructed list
without either a canonical re-serialisation (a second writer — NOT proposed
here) or a model-level bind entry point (a rendering-core owner decision — NOT
proposed here). This proposal scopes delta consumption to the Mac model-level
path above; the rendering-core path keeps consuming full siblings only. The
generic `validate_display` example / default static-TrueType profile is not a
binding path for `opentype-cff` output and is not cited as one (the draft's
L50–53 and `oracle-evidence.md` agree: the generic profile refuses the
pipeline's format as emitted). On the producer side the byte-identity gate
(§10.1 P2) feeds the producer's OWN `to_json` bytes of the reconstructed list
to `PipelineCff::bind` — the producer has the writer, so no second parser or
writer is involved.

### 5.5 Complete reconstruction — nothing inherited implicitly

The reconstructed list is a complete new `display_list` payload whose every
part is produced by the reconstruction step, never carried over from the base
by default:

| part of the full payload | reconstructed from | inherited from the base? |
|---|---|---|
| `render_format`, `coordinate_unit`, `color_space`, `text_extraction`, `project_id`, `revision`, `required_features` | the delta's own header fields (complete) | no |
| `documents[]` (every path, compile revision, raw sha256, byte_length) | the delta's complete `documents` | no — the base's documents describe the OLD text and are discarded |
| `fonts[]` (the resource closure) | the delta's complete `fonts` | no — the base's font list is discarded even when equal; the consumer re-resolves every font by content hash for the new list |
| `diagnostics[]` (with relocated source spans) | the delta's complete `diagnostics` | no |
| changed pages | the delta's `changed_pages` objects, complete | no |
| unchanged pages | `relocate(base.pages[n])`: a NEW page model whose geometry, glyphs, clusters, hit rects, carets and paint are copied and whose source spans are moved by the relocation rule | the base page is INPUT to a function that produces a new page; it is never referenced after reconstruction (the base snapshot is evicted per §6.2) |
| page order and count | `page_count` and `1..N` | no |
| per-page and list digests | recomputed by the consumer over the reconstructed models and compared with the delta's | no |

Consequences, and who checks them: a font that only unchanged pages reference
must still be in the delta's `fonts` — a delta whose `fonts` omit it fails the
consumer's full validator (`font resource … is not declared in fonts`), so this
one IS consumer-detectable. A diagnostic that belonged to an unchanged page
must still be in the delta's `diagnostics` with its relocated span — the
consumer CANNOT detect its omission (the digest would be computed over the
same incomplete list on both sides); only the producer gate's fresh-full
oracle detects it (§10.1 P2 compares the reconstructed line with a fresh
compile's line byte for byte, diagnostics included). A document whose text did
not change is still listed in `documents` with the same sha256 (the consumer
checks it against its own text as for a full frame). There is no "same as
base" marker anywhere in the delta by design, and no claim is made that
digest agreement establishes completeness of anything the producer omitted.

## 6. Installation, retained state, residency bounds

### 6.1 Installation (the acknowledgement's meaning)

A snapshot is **installed** on the consumer when, and only when, all of:

1. its envelope was decoded (full line; the exact byte length of every page
   object is recorded from the line by the existing fast typed reader
   (`RenderingV2Fast`, which already scans object ranges) and cached with the
   snapshot) or, for a delta, its reconstructed target's EXACT size was
   computed from `page_bytes` and admitted before reconstruction (§6.2 — no
   target allocation is needed to compute it) and then reconstructed (§5.4);
2. the unchanged full validation path accepted it (§5.4a: `RenderingV2.validate`
   → `V2FontStore.resolve` → `V2Frame.prepare`);
3. its `dl2-canon-1` page digests and `list_digest` were computed (and, for a
   delta, matched the wire values);
4. it was PUBLISHED as the current v2 frame — its `compile_result` is the
   applied one and its load ticket is current (`deliverDisplayListV2`,
   `PreviewV2View.swift:287-292`); a sibling that is stale by the time it
   validates (a newer result applied, a newer load ticketed, the pane reset) is
   REFUSED exactly as today (`PreviewV2View.swift:216-221`) — it is never
   installed and never acknowledged, and the chain is cleared (§6.3);
5. it replaced the previously installed snapshot as the consumer's sole
   installed base in the same main-thread step as the publish.

Decision (r3): stale refusal is preserved; there is no non-authoritative
reconstruction cache. Rationale: the alternative (installing a stale
reconstruction so the next acknowledgement can name it) would hold the painted
frame A, a stale installed B and a candidate C at once, breaking the old+new
accounting and lending a stale frame base authority that the helper contract
denies ("retained old frames must not acquire new source authority"). What
the preserved rule costs: a stale sibling breaks the chain, the next request
carries no acknowledgement, and the producer answers in full — exactly the
reply it would give under pipelining anyway (§3), so nothing is lost in the
one-request-at-a-time typing loop where deltas are meant to occur.

Receipt of a line, helper admission, a helper's successful write, a queued
frame, or a frame still validating off-main is not installation. The consumer
acknowledges (request `display_list_base`) only the installed snapshot and,
after sending an acknowledgement, keeps that snapshot installed until the reply
to that request has been processed: with one request in flight the outstanding
reply IS the only candidate, so `installed` + `in-flight` (+ one `queued` line) is the whole state (§6.2);
if the consumer sends another request before that reply (pipelining), the
reply's sibling will be stale on arrival, is refused per item 4, and the chain
is cleared — consistent with the producer answering the pipelined request in
full (§3).

On the producer, "installed" is a fact about the consumer that the producer
learns only from the next request's acknowledgement. The producer's own record
is "last emitted"; a delta requires acknowledged == last emitted.

### 6.2 Residency: at most old + new on each side

**Producer** (per stream):

| slot | content | when it exists |
|---|---|---|
| `old` | last emitted snapshot: `DisplayList` model, page digests, `list_digest`, `request_id`, the request texts it was compiled from | from emitting a sibling until the next sibling is emitted or a clearing event |
| `new` | the list being produced for the current request (already needed to write the reply) | during one reply only |

Eviction order per request: (1) a clearing event (reply without a sibling —
not requested / declined / `failed`; `project_id` change; process exit) evicts
`old` before anything is retained; (2) `new` is built; (3) if a sibling is
emitted, `new` becomes `old` and the previous `old` is dropped at that moment
(the diff/classification that needed both is complete before the line is
written); (4) if no sibling is emitted, `new` is dropped after the reply and
`old` was already cleared by (1). At no time are three snapshots live. The
existing block caches (`incremental.rs`, `MAX_BLOCKS`) are not snapshots and
are unaffected.

**Consumer** (per transport):

| slot | content | charged as | when it exists |
|---|---|---|---|
| `installed` | the acknowledged base: validated model, digests, identity, transport binding, AND its painted-frame resources (`V2Frame`: prepared pages, resolved `CGFont`s shared with the store, prerastered bitmaps at the pane's last scale) | model ≤ `MAX_SNAPSHOT_BYTES` serialised-equivalent; bitmaps separately by the pane's raster policy (page count × pixels; a §10.3 measurement) | from installation until replaced or cleared |
| `in-flight` | the ONE off-main preparation currently running (`V2Loader.queue`, `startDisplayListV2`): its input wire line (captured by the closure), for a delta the target model under construction, the prepared pages, AND the candidate's PRERASTER bitmaps (`V2Loader.preraster` runs inside the same job at the pane's last scale/appearance BEFORE delivery, `PreviewV2View.swift` `startDisplayListV2`), which become `candidate` on completion | input line ≤ 16 MiB + target ≤ `MAX_SNAPSHOT_BYTES` (admitted by the exact pre-allocation charge, else refused before building) + preraster bitmaps of the candidate (page count × pixels at the pane's scale — the same size class as the painted frame's bitmaps) | from dispatch of the callback until it completes — replacing or dropping the QUEUED callback never terminates a RUNNING one, so this slot, bitmaps included, is live for the whole callback regardless of newer arrivals |
| `queued` | the newest wire line waiting behind `in-flight` (`V2QueuedLoad`; older queued lines are dropped undecoded, existing coalescing) | one line ≤ 16 MiB | from arrival while `in-flight` is busy until it is dispatched (becomes `in-flight`) or replaced by a newer arrival |

`candidate` is not a fourth slot: it is the completed `in-flight` result during
the single main-thread step that either installs it (publish as the current
frame) or drops it (stale, refused, or a clearing event happened meanwhile).
Peak consumer residency is therefore `installed` (model + painted-frame
resources incl. its bitmaps) + `in-flight` (one line + one target + the
candidate's preraster bitmaps) + `queued` (one line), i.e. ≤ 2 ×
`MAX_SNAPSHOT_BYTES` exact serialised size + 2 × 16 MiB of wire lines + TWO
sets of raster bitmaps (painted + prerastered candidate) at the pane's scale.
Dropping the queued line frees only that line; the running callback's
allocations, bitmaps included, are released only when it returns.

Eviction order: (1) a clearing event (a `compile_result` without accepted
`display-list-v2`; any §7 refusal; pane hidden; worker/helper restart,
reattach or session change; project change) drops `queued`, marks the ticket
of `in-flight` stale (its result will be dropped on return — the callback
itself runs to completion and its memory is counted until then), then drops
`installed`; (2) when `in-flight` completes and its ticket is current, it is
published and becomes `installed` and the old `installed` (model and frame
resources) is dropped in the same main-thread step; otherwise its result is
dropped and the chain cleared (§6.1 item 4); (3) a newer arrival while
`in-flight` is busy replaces `queued` (the older queued line is dropped
undecoded); (4) `queued` is dispatched only after `in-flight` returns. The
frame on screen while `in-flight` runs IS the `installed` snapshot's frame,
never an extra snapshot.

**Caps (per snapshot, both sides; over any cap → the snapshot is not retained
→ full replies only, no delta):**

| cap | value | why |
|---|---|---|
| `MAX_SNAPSHOT_PAGES` | 1 024 pages | rendering-core allows 10 000; 1 024 keeps digest recomputation and page relocation bounded at ~40× the measured 27-page document |
| `MAX_SNAPSHOT_BYTES` | 16 MiB of EXACT serialised size of the RECONSTRUCTED TARGET, computed BEFORE any target allocation — **choice (b), no estimator exists**: `target_bytes = F + H + Σ page_bytes[1..N] + (N − 1)` where `F` is the fixed framing of the full line for this id (the writer's constant keys and punctuation, `display.rs:329-395`, plus `json(id)`), `H` is the byte length of the header parts as they appear IN THE DELTA LINE (`required_features`, `documents`, `fonts`, `diagnostics` arrays and the scalar header values — the same writer's bytes as in the full line, measured on the received delta by the existing fast reader, never re-serialised), and `page_bytes` are the producer's exact per-page lengths (§4 rule 2b) each VERIFIED by the consumer before use: for a changed page, the measured length of that page object in the delta line; for an unchanged page, `cached_bytes(installed page n) + Σ over its relocated source ranges of (digits(start+d) − digits(start) + digits(end+d) − digits(end))` — exact because the producer prints integral values as plain decimal (`crates/compiler/src/json.rs:312-313`) and relocation changes nothing else in the page; any inequality is `delta_page_bytes_mismatch(n)` (§7). At install of a FULL line the per-page lengths are measured from the received line. The old producer heuristic `estimated_json_bytes()` is NOT used anywhere in this proposal (the Commander's evidence at main a744cd86 shows it under-charges schema-valid values, e.g. a 4 214-byte document declaration charged 200 B) | a delta line is small by construction while its target can be up to the full-line limit or, if the document grew, beyond it. **Decision (r4, kept): an over-cap target is REJECTED before reconstruction and before paint** — typed `delta_target_oversize` naming the exact target size and the cap; the last installed frame stays on screen marked stale; the chain is cleared; the next request omits `-delta` and the full reply is subject to its own honest refusal (§8). Nothing over the cap is ever built, validated, painted or retained, so the painted frame is always within the cap. Appendix A implements `target_bytes` and checks it equals the fresh full line's length on the worked example (3 498 bytes) and that the digit-width arithmetic equals actual serialisation across 2→3-digit boundaries |
| **peak residency (per side)** | consumer: `installed` (model + painted bitmaps) + `in-flight` (one wire line + one target + the candidate's preraster bitmaps) + `queued` (one wire line) ≤ 2 × `MAX_SNAPSHOT_BYTES` exact serialised size + 2 × 16 MiB + 2 × raster set; producer: `old` + `new` ≤ 2 × `MAX_SNAPSHOT_BYTES` + the reply line being written (≤ 16 MiB) + the request line (≤ 8 MiB input) + retained texts | the in-memory model overhead factor over exact serialised size and the raster set size are §10.3 measurements, not claims; the serialised-size part of the bound is exact, not conservative |
| retained request texts (producer only) | ≤ 8 MiB per document (rendering-core document bound), ≤ 32 MiB total | needed for the relocation diff; over the cap → no snapshot |
| documents / fonts per snapshot | 4 096 / 256 (the existing validator bounds) | unchanged |

### 6.3 Lifecycle events

- **Ordering under pipelining:** FIFO stream; see §3 — an acknowledgement can
  only name the last installed snapshot, so with a request in flight the
  producer answers in full. Nothing breaks; deltas simply do not occur.
- **Stale siblings:** refused exactly as today (`PreviewV2View.swift:216-221`,
  ticket gate `:287-292`): never validated into the chain, never installed,
  never acknowledged. A refused-stale delta or full clears `candidate` and, if
  the stale sibling was the reply to the request that carried the current
  acknowledgement, the chain is broken: the next request omits `-delta` and
  the producer answers in full (§8). A delta whose `base` is not the currently
  installed (published) frame is `delta_base_mismatch` → full resync (§7).
- **Over-cap target:** refused before allocation (`delta_target_oversize`,
  §7); the delta line itself is dropped after the exact `page_bytes`
  accounting; `installed` stays painted and marked stale; chain cleared; the
  next full reply is refused honestly if it too exceeds a limit (§8). There is
  no "validate but do not retain" path since r4.
- **In-flight work is never cancelled:** replacing or dropping `queued`, a
  clearing event, or a newer result only marks the running callback's ticket
  stale; the callback completes, its result is dropped on return, and its
  memory is counted in the peak until then. Backpressure is the existing
  one-in-flight + one-queued coalescing; the consumer never dispatches a second
  reconstruction concurrently.
- **Reset/cancel:** runtime-v1 has no cancel; superseded requests are answered
  in order. A consumer that cannot validate a sibling (helper dropped it as
  oversize/busy per `display-forwarding.md`; decode or validation failure)
  clears per 6.2 and sends its next request without `-delta` (§8).
- **Restart:** a restarted producer has no `old`; every acknowledgement
  mismatches; it answers in full. A restarted helper session requires the
  existing fresh opt-in (`configure_display_candidates`), and the consumer
  clears both slots on the session change.

## 7. Typed refusals (consumer) and the wrong-base rule

| code | when | action |
|---|---|---|
| `delta_unsolicited` | `display_list_delta` line while `-delta` was not accepted for that id, or `display_list` while it was | protocol violation, as today for unexpected v2 lines |
| `delta_unsupported_scheme` | `digest_scheme` ≠ `dl2-canon-1` | refuse, resync |
| `delta_base_mismatch` | any of the five `base` fields ≠ held base, or no base held | refuse, resync; counted separately as it indicates a chain break |
| `delta_page_count` / `delta_removed_pages` | bounds, ordering, or `removed_pages` ≠ `[N+1..B]` | refuse, resync |
| `delta_relocation_invalid` | a relocation names an undeclared path, violates §4 rule 5, or an unchanged page's span intersects the edited region | refuse, resync |
| `delta_digest_mismatch(n)` | a page digest differs after reconstruction | refuse, resync; evidence-worthy: reconstruction or producer classification bug |
| `delta_list_digest_mismatch` | header/list digest differs | refuse, resync |
| `delta_oversize` | line over the consumer's framing budget | dropped before parse (existing behaviour), resync |
| `delta_page_bytes_mismatch(n)` | `page_bytes[n-1]` ≠ the measured length of changed page `n` in the delta line, or ≠ the cached-plus-digit-delta length of unchanged page `n` (§6.2) | refuse BEFORE building the target; chain cleared; resync; evidence-worthy (producer writer or consumer accounting bug) |
| `delta_target_oversize` | the EXACT pre-allocation size of the reconstructed target exceeds `MAX_SNAPSHOT_BYTES` (or `page_count` > `MAX_SNAPSHOT_PAGES`) | refuse BEFORE building the target: message names "reconstructed size N bytes / P pages over the cap C"; last installed frame kept marked stale; chain cleared; resync (whose full reply refuses honestly per §8 if it too is over a limit) |
| existing full-validation errors | the reconstructed list fails `RenderingV2.validate` | refuse, resync (same codes as a full frame) |

"Refuse" = keep the previous verified frame on screen (labelled stale as
today), publish nothing, log the code, drop the base. "Resync" = the next
compile request omits `display-list-v2-delta`; its reply is the unchanged full
sibling (or the unchanged decline), which re-establishes the base.

## 8. Full resync path — and its honest refusal

Always available and always the unchanged contract: request `display-list-v2`
without `-delta` (and without `display_list_base`) → the full `display_list`
line. Used on: no installed base, any §7 refusal, any dropped or unvalidated
sibling, any restart or session change, and after every pipelined request.

When the full reply itself does not fit, the resync is **refused, typed, and
names the limit** — it is never satisfied by omitting pages, dropping
resources, or shortening diagnostics:

| layer | refusal today (unchanged) | what it names | consumer behaviour |
|---|---|---|---|
| producer v2 line over `max_reply_bytes()` | `display_list_declined` warning, `display-list-v2` removed from the echo, `status` `recovered`, no sibling (`protocol.rs:241-252`) | "the display_list line would be about N bytes for P pages, over the L-byte line limit" | keeps the last installed frame on screen marked stale, drops `installed` and `candidate` (clearing event, §6.2), shows the diagnostic; requests full again only when the document changes |
| producer v1 line over the limit | explicit `failed` result, no sibling (`protocol.rs:254-270`) | "compile_result would be N bytes for P pages, over the L-byte reply limit" | same; the v1 preview also keeps its last result (existing behaviour) |
| runtime/helper frame over its bound | runtime frame refusal / helper `serialization_refused` (`display-forwarding.md`, `producer-size-contract.md`) | the framed byte count and bound in the helper's diagnostics | candidate never arrives; the consumer's sibling timeout/deferred bound elapses; clears per §6.2; the v1 fallback stays |
| consumer line reader | protocol violation at 16 MiB (`WorkerClient.swift:90-91`, `PreviewControllerClient.swift:206-211`) | the line size and its limit | nothing partial is decoded; clears per §6.2 |

A document whose full line is refused therefore never obtains an installed
base and never gets a delta: the delta does not raise, bypass, or hide any
full-reply bound, and the 18.2 MiB / 25.1 MB cases in the runtime's size
review stay refused exactly as today. A delta line that would itself exceed
the producer limit falls back to the full line, which is then subject to the
same refusal. There is no third outcome.

## 9. Not a page filter, not a partial compile

A view that requests or paints only visible pages is a DIFFERENT, incomplete
view. It must never be labelled a complete compile, must never be used as a
base, and must never authorize source actions (navigation, edits, exports)
outside the pages it validated. This proposal is the opposite: a delta is
accepted only when the reconstructed list is verified equal (digests, then the
full validator, and in the producer gate byte-identical) to an unchanged fresh
full compile, so the reconstructed list carries exactly the authority of a full
`display_list` — and no more: the existing consumer rule stands that a v2
frame never authorizes source actions beyond the v2 pane's own navigation gate
(buffer-hash equality plus the stale refusal). Candidate flags on the helper
route (`untrusted:true`, `source_actions_enabled:false`) stay as they are.

## 10. Acceptance plan (three separate gates; none claims the others)

### 10.1 Producer gate (`crates/render-pipeline`, cargo tests, no wire activation until reviewed)

- P1 negotiation: not requested → never emitted; `-delta` without
  `display-list-v2` or without `display_list_base` → never accepted; first
  request of a process → full line, echo without `-delta`; second request
  acknowledging that full → delta, echo with both; an acknowledgement of any
  other snapshot (older, unknown, evicted, or the last emitted one under
  pipelining) → full, no diagnostic, `status` `ok`; after a `failed` /
  declined / not-requested reply → `old` cleared → full again.
  Golden-byte check: for requests that do not list `-delta`, `compile_result`
  and `display_list` bytes are unchanged (existing `golden_v1.rs` fixtures +
  `v2_and_math.rs`).
- P2 byte identity (the Commander's invariant, `tests/incremental.rs` shape):
  for each edit `i` of the existing 200-edit script over the 27-page document
  (and the ignored 30 × 107 run): fresh full line `F_i` (no cache, no delta);
  delta-mode worker with the caches → sibling `S_i` (delta or full); reference
  `apply_delta` in the crate reconstructs `R_i`; assert `write(R_i.to_json(id))
  == F_i` bytes, every `page_digests` entry equals `page_digest` of `R_i`'s
  page, `list_digest` matches, and `compile_result` bytes equal the non-delta
  run's. Record per edit: pages, changed-page count, delta bytes vs full bytes
  (evidence, not a claim).
- P3 refusal/wrong base: request acknowledges N−1 while the producer's last
  emitted is N → full (producer-side refusal); a delta replayed to a reference
  consumer whose installed base differs → `delta_base_mismatch`; a tampered
  page digest → mismatch; residency: the producer never holds more than `old`
  + `new` (assert in the test harness at every step of the 200-edit script);
  removed pages after the paragraph-deletion edits → exact `removed_pages`;
  a document-set change → full; a paste spanning pages → changed pages or full
  by policy.
- P4 size: a delta over the line limit → full; a full over the limit → the
  existing decline bytes unchanged; policy factor honoured.
- P5 vectors: Rust `dl2-canon-1` reproduces Appendix B's digests from the
  Appendix B JSON, including the −0.0 → +0.0 rule (a paint component written
  as `-0` and as `0` hash equal) and refusal of non-finite components; the
  producer's `to_json` bytes of each reconstructed `R_i` are also fed to
  rendering-core `PipelineCff::bind` with the gate's source snapshots and the
  pinned LM registry resources (§5.4a) — the CFF binder, not the generic
  static-TrueType profile.

### 10.2 Consumer gate (`apps/mac`, Swift tests; fake worker script + real `flashtex-render`)

- C1 `DisplayListDelta.apply(base:delta:)` on the Appendix B vectors → the
  reconstructed `RenderingV2.DisplayList` is `Equatable`-equal to
  `RenderingV2.decode(fresh full)`; digests reproduce the vectors.
- C2 every §7 refusal with a fake worker (unsolicited type, wrong base, bad
  digest, bad relocation, removed-page mismatch, unknown scheme); previous
  frame stays; base dropped; next request omits `-delta`; the following full
  re-establishes the base.
- C3 installation and residency: a sibling is acknowledged only after
  validation + publish as the current frame (a frame still validating is never
  named in a request); a stale-by-ticket sibling is REFUSED, not installed, and
  the next request carries no acknowledgement; with one request in flight the
  acknowledged base stays installed until that reply is handled; at most
  `installed` + `in-flight` + `queued` exist at any time and the charge of each target is the
  reconstructed target's EXACT serialised size from verified `page_bytes`,
  computed BEFORE allocation (assert in the fake-worker test: a delta whose
  target is cap + 1 byte is refused with `delta_target_oversize` naming both
  numbers, no target object is created, the previous frame stays painted and
  marked stale, the next request omits `-delta`; a delta whose `page_bytes`
  entry is off by one for a changed or an unchanged page is refused with
  `delta_page_bytes_mismatch(n)` before allocation; scenario
  `small_delta_oversized_reconstruction` in §10.4); residency: at most `installed` + `in-flight` + `queued` (assert with
  a slow fake reconstruction: a newer line arriving mid-callback lands in
  `queued`, a third replaces it, the running callback completes and its result
  is dropped as stale — scenario `coalescing_peak_inflight_and_queued`); a stale
  late sibling is never installed/acknowledged and never used for navigation
  (scenario `painted_a_stale_b_candidate_c`); all slots cleared on decline/
  failed/restart/session change/pane hidden; the full-reply refusal
  (`display_list_declined`) leaves the last installed frame on screen marked
  stale and clears the chain.
- C4 real producer: `flashtex-render` in delta mode over an edit script; every
  reconstructed frame passes `RenderingV2.validate` → `V2FontStore.resolve` →
  `V2Frame.prepare` (§5.4a) and reaches `V2Parity` with 0 differing pixels at
  1 and 2 px/pt (the existing parity gate) — parity is measured, never inferred
  from digests; completeness of the reconstructed diagnostics is NOT a consumer
  gate item (it is P2's fresh-full oracle).
- C5 helper route: blocked until the runtime accepts sibling
  `type: "display_list_delta"` (`crates/document-runtime/src/display_candidate.rs`,
  `raw_display.rs` currently require `display_list`; FT-049 owner) and the
  helper forwards it inside the same `display_candidate` update; the
  `DisplayCandidateGate` identity rules apply unchanged. Until then the
  consumer gate runs on the direct route only, and direct-route results are
  not evidence for the helper route.

### 10.3 Transport gate (measured separately; never claimed from the schema)

- Bytes per edit: delta line vs full line over the 200-edit script (from P2).
- Producer cost: in-process stage timings (`examples/stages.rs`) with the
  digest/classification pass added; worker CPU per request; snapshot RSS.
- Consumer cost: decode+apply+verify of a delta vs decode+validate of a full
  frame, off-main, on the same frames.
- Consumer peak residency: RSS at the moment `installed` + `in-flight` +
  `queued` coexist (slow-reconstruction harness), split into model bytes,
  wire-line bytes and painted-frame raster resources (prepared pages +
  prerastered bitmaps at 1 and 2 px/pt), and the per-page `page_bytes` cache; the
  in-memory factor over serialised-equivalent reported as a range.
- Exact accounting check: over the 200-edit script, `target_bytes` computed
  from `page_bytes` before reconstruction equals the byte length of the fresh
  full line (P2's oracle) at every edit — an equality assertion, not a ratio;
  plus the cost of measuring per-page lengths at install and of the
  digit-width pass per delta.
- Raster peak: bitmap bytes of the painted frame plus the in-flight
  candidate's preraster set at 1 and 2 px/pt, measured at the moment both
  exist.
- Wall: keystroke→paint through the v2 pane (`TypingBench`) with and without
  `-delta`, load-aware (`uptime` recorded; skipped above 1-min load 20).
- Report as ranges with load; the 33.6 ms / 4.3 MB baseline in
  `oracle-evidence.md` is the comparison point.

### 10.4 Machine-readable review scenarios (rendering-core handoff)

The rendering-core owner publishes review scenarios at
`crates/rendering-core/docs/handoffs/page-delta/refusal-scenarios.json` (main;
`schema_version` 1, pinned to this proposal's commit) with `inputs`/`states`
and a `required_outcome` per case, plus `review-r3.md`,
`r3-reference-results.json` (the independent Appendix A reproduction) and
`source-pins.json`. They plug into the gates as fixture inputs, one test per
case, asserting every `required_outcome` key:

| scenario id | gate | what the test feeds and asserts |
|---|---|---|
| `small_delta_oversized_reconstruction` | consumer C3 (fake worker); producer P4 | a 4 096-byte delta whose exact target size (Σ verified `page_bytes` + header + framing) is cap + 1 → `delta_target_oversize` before allocation, `install_as_base:false`, `publish_under_claimed_candidate_cap:false`, `old_frame_mutated:false`, next request = full resync under existing bounds (producer side: the producer itself never emits a delta whose target it would not retain, so P4 asserts a full reply there) |
| `painted_a_stale_b_candidate_c` | consumer C2/C3 | painted A (rev 12), late B (rev 11), candidate C (rev 13): `install_late_b:false`, `acknowledge_late_b:false`, `use_b_for_current_export_or_hit:false`, `old_frame_mutated:false` (§6.1 item 4; the helper route's `DisplayCandidateGate` and, once D2 of the draft review is fixed, membership generation) |
| `coalescing_peak_inflight_and_queued` | consumer C3 + §10.3 peak measurement | live objects painted A, active preparation B, queued C: `count_live_callback_allocations:true`, `count_queued_wire_bytes:true`, `count_retained_frame_resources:true`, `dropping_queue_counts_as_cancelling_active_work:false` (§6.2 slots) |

New cases added to that file after this revision are picked up by the same
mechanism; a case whose `required_outcome` the proposal cannot satisfy is a
blocking finding for the next revision, not something the gate may skip.

## 11. What this proposal does not do

No change to `compile_result`; no change to the full `display_list` bytes; no
change to the decline fallback; exactly one additive optional request field
(`display_list_base`, present only with `-delta`, ignored by producers that do
not know it); no change to the runtime-v1 stream framing; no helper/runtime changes (C5 names the owner); no
size or latency claims. Adoption order after co-signature: producer behind a
build flag with P1–P5 green → Commander records the contract → runtime/helper
change → consumer gate → transport measurements → then, and only then, a
decision to request `-delta` by default in the v2 pane.

## Appendix A — `dl2-canon-1` reference and example generator (Python, stdlib only)

Running `python3 dl2_delta_example.py` prints the digests below;
`… base|delta|fresh` prints the envelopes. This is the normative description of
the digest scheme and of the consumer's reconstruction algorithm; the Rust and
Swift implementations must agree with it on the vectors.

```python
#!/usr/bin/env python3
import hashlib, json, math, struct, sys

# ---- canonical encoding (dl2-canon-1) -------------------------------------
def i64(n):  return struct.pack('<q', int(n))
def f64(x):
    x = float(x)
    if not math.isfinite(x): raise ValueError('non-finite paint component is not encodable')
    return struct.pack('<d', 0.0 if x == 0.0 else x)   # -0.0 -> +0.0; raw IEEE-754 bits otherwise
def s(t):
    b = t.encode('utf-8'); return i64(len(b)) + b
def ranges(rs):
    out = i64(len(rs))
    for r in rs: out += s(r['path']) + i64(r['start_byte']) + i64(r['end_byte'])
    return out
def provenance(o):
    if 'sources' in o: return b'\x10' + ranges(o['sources'])
    return b'\x11' + s(o['synthetic_reason'])
def paint(p): return f64(p['r']) + f64(p['g']) + f64(p['b']) + f64(p['a'])
def item(it):
    if it['kind'] == 'glyph_run':
        out = b'\x01' + s(it['font_id']) + i64(it['font_size']) + s(it['text'])
        out += i64(len(it['glyphs']))
        for g in it['glyphs']:
            out += i64(g['gid']) + i64(g['origin_x']) + i64(g['baseline_y']) + i64(g['advance_x']) + i64(g['advance_y']) + i64(g['cluster'])
        out += i64(len(it['clusters']))
        for c in it['clusters']:
            out += i64(c['text_start_byte']) + i64(c['text_end_byte'])
            out += i64(len(c['hit_rects']))
            for r in c['hit_rects']: out += i64(r['x']) + i64(r['top']) + i64(r['width']) + i64(r['height'])
            out += i64(len(c['carets']))
            for k in c['carets']: out += i64(k['text_byte']) + i64(k['x']) + i64(k['top']) + i64(k['height'])
            out += provenance(c)
        return out + paint(it['paint'])
    if it['kind'] == 'rule':
        return b'\x02' + i64(it['x']) + i64(it['top']) + i64(it['width']) + i64(it['height']) + paint(it['paint']) + provenance(it)
    raise ValueError(it['kind'])
def page_digest(p):
    h = hashlib.sha256(b'flashtex:dl2:page:1\0')
    h.update(i64(p['number']) + i64(p['width']) + i64(p['height']) + i64(len(p['items'])))
    for it in p['items']: h.update(item(it))
    return h.hexdigest()
def header_digest(pl):
    h = hashlib.sha256(b'flashtex:dl2:header:1\0')
    for k in ('render_format', 'coordinate_unit', 'color_space', 'text_extraction', 'project_id'): h.update(s(pl[k]))
    h.update(i64(pl['revision']))
    h.update(i64(len(pl['required_features'])))
    for f in pl['required_features']: h.update(s(f))
    h.update(i64(len(pl['documents'])))
    for d in pl['documents']: h.update(s(d['path']) + i64(d['revision']) + s(d['sha256']) + i64(d['byte_length']))
    h.update(i64(len(pl['fonts'])))
    for f in pl['fonts']:
        h.update(s(f['font_id']) + s(f['sha256']) + i64(f['byte_length']) + s(f['format']) + i64(f['face_index']) + i64(f['units_per_em']) + i64(f['glyph_count']) + s(f['postscript_name']))
    h.update(i64(len(pl['diagnostics'])))
    for d in pl['diagnostics']: h.update(s(d['code']) + s(d['message']) + s(d['severity']) + ranges(d['sources']))
    return h.hexdigest()
def list_digest(pl, page_digests):
    h = hashlib.sha256(b'flashtex:dl2:list:1\0')
    h.update(bytes.fromhex(header_digest(pl)) + i64(len(page_digests)))
    for d in page_digests: h.update(bytes.fromhex(d))
    return h.hexdigest()


# ---- exact size accounting (r5): page_bytes is OUTSIDE the digested model ---
def wire(o): return json.dumps(o, separators=(',', ':'))   # stand-in for the producer writer (illustrative)
def page_bytes(p): return len(wire(p).encode('utf-8'))
def digits(n): return len(str(int(n)))
def relocated_page_bytes(base_page, cached_bytes, relocs):
    """Exact byte length of relocate(base_page) from the cached length: only the
    decimal width of moved offsets can change. No serialisation needed."""
    by_path = {r['path']: r for r in relocs}
    delta = 0
    def moved(r):
        rl = by_path.get(r['path'])
        if rl is None or r['end_byte'] <= rl['edit_start']: return 0
        if r['start_byte'] >= rl['edit_end']:
            d = rl['delta']
            return (digits(r['start_byte'] + d) - digits(r['start_byte'])) + (digits(r['end_byte'] + d) - digits(r['end_byte']))
        raise ValueError('span intersects edited region')
    for it in base_page['items']:
        provs = [c for c in it['clusters']] if it['kind'] == 'glyph_run' else [it]
        for o in provs:
            for r in o.get('sources', []): delta += moved(r)
    return cached_bytes + delta
def target_bytes(delta_env, base_env, cached_page_bytes):
    """Exact serialised size of the reconstructed target, computed BEFORE
    reconstruction: header parts measured on the delta line (same writer bytes),
    pages from page_bytes, fixed framing from the writer's layout."""
    d = delta_env['payload']
    changed = {p['number']: p for p in d['changed_pages']}
    total_pages = 0
    for num in range(1, d['page_count'] + 1):
        if num in changed:
            got = page_bytes(changed[num])
        else:
            got = relocated_page_bytes(base_env['payload']['pages'][num - 1], cached_page_bytes[num - 1], d['relocations'])
        assert got == d['page_bytes'][num - 1], f'delta_page_bytes_mismatch page {num}: {got} != {d["page_bytes"][num - 1]}'
        total_pages += got
    header = {k: d[k] for k in ('render_format', 'coordinate_unit', 'color_space', 'text_extraction', 'project_id', 'revision', 'required_features', 'documents', 'fonts')}
    header['pages'] = []; header['diagnostics'] = d['diagnostics']
    frame = len(wire({'protocol_version': 2, 'id': delta_env['id'], 'type': 'display_list', 'payload': header}).encode('utf-8'))
    return frame + total_pages + max(0, d['page_count'] - 1)   # commas between page objects

# ---- relocation + reconstruction (consumer algorithm) ---------------------
def relocate_range(r, reloc):
    a, b, d = reloc['edit_start'], reloc['edit_end'], reloc['delta']
    if r['end_byte'] <= a: return dict(r)
    if r['start_byte'] >= b: return dict(r, start_byte=r['start_byte'] + d, end_byte=r['end_byte'] + d)
    raise ValueError(f"span {r} intersects edited region [{a},{b}) of {reloc['path']}")
def relocate_prov(o, by_path):
    if 'sources' not in o: return o
    return dict(o, sources=[relocate_range(r, by_path[r['path']]) if r['path'] in by_path else dict(r) for r in o['sources']])
def relocate_page(p, relocs):
    by_path = {r['path']: r for r in relocs}
    items = []
    for it in p['items']:
        it2 = dict(it)
        if it['kind'] == 'glyph_run': it2['clusters'] = [relocate_prov(c, by_path) for c in it['clusters']]
        else: it2 = relocate_prov(it2, by_path)
        items.append(it2)
    return dict(p, items=items)
def apply_delta(base_env, delta_env):
    base, d = base_env['payload'], delta_env['payload']
    assert d['base']['request_id'] == base_env['id'], 'delta_base_mismatch'
    assert (d['base']['project_id'], d['base']['revision'], d['base']['page_count']) == (base['project_id'], base['revision'], len(base['pages'])), 'delta_base_mismatch'
    base_pd = [page_digest(p) for p in base['pages']]
    assert d['base']['list_digest'] == list_digest(base, base_pd), 'delta_base_mismatch'
    n = d['page_count']; assert len(d['page_digests']) == n, 'delta_page_count'
    cached = [page_bytes(p) for p in base['pages']]          # measured at install of the base
    assert target_bytes(delta_env, base_env, cached) <= MAX_SNAPSHOT_BYTES, 'delta_target_oversize'
    changed = {p['number']: p for p in d['changed_pages']}
    assert sorted(changed) == [p['number'] for p in d['changed_pages']], 'delta_pages_unordered'
    assert d['removed_pages'] == list(range(n + 1, len(base['pages']) + 1)), 'delta_removed_pages'
    pages = []
    for num in range(1, n + 1):
        if num in changed: p = changed[num]
        else:
            assert num <= len(base['pages']), 'delta_page_count'
            p = relocate_page(base['pages'][num - 1], d['relocations'])
        assert page_digest(p) == d['page_digests'][num - 1], f'delta_digest_mismatch page {num}'
        pages.append(p)
    full = {k: d[k] for k in ('render_format', 'coordinate_unit', 'color_space', 'text_extraction', 'project_id', 'revision', 'required_features', 'documents', 'fonts')}
    full['pages'] = pages; full['diagnostics'] = d['diagnostics']
    assert list_digest(full, d['page_digests']) == d['list_digest'], 'delta_list_digest_mismatch'
    return {'protocol_version': 2, 'id': delta_env['id'], 'type': 'display_list', 'payload': full}

# ---- worked example ---------------------------------------------------------
T = 1 << 20
MAX_SNAPSHOT_BYTES = 16 * 1024 * 1024
FONT = 'c1f0e5d6a7b8c9d0e1f2a3b4c5d6e7f8091a2b3c4d5e6f708192a3b4c5d6e7f8'  # illustrative
def rect(x, w): return {'x': x, 'top': 78643200, 'width': w, 'height': 12582912}
def caret(tb, x): return {'text_byte': tb, 'x': x, 'top': 78643200, 'height': 12582912}
def run(text, glyphs, src0):
    """glyphs: [(gid, advance)] laid out from x=72bp; one cluster per glyph; one source byte each from src0."""
    x = 72 * T; gs, cs = [], []
    for i, (gid, adv) in enumerate(glyphs):
        gs.append({'gid': gid, 'origin_x': x, 'baseline_y': 84 * T, 'advance_x': adv, 'advance_y': 0, 'cluster': i})
        carets = [caret(i, x)] + ([caret(i + 1, x + adv)] if i == len(glyphs) - 1 else [])
        cs.append({'text_start_byte': i, 'text_end_byte': i + 1, 'hit_rects': [rect(x, adv)], 'carets': carets,
                   'sources': [{'path': 'main.tex', 'start_byte': src0 + i, 'end_byte': src0 + i + 1}]})
        x += adv
    return {'kind': 'glyph_run', 'font_id': FONT, 'font_size': 12 * T, 'text': text, 'glyphs': gs, 'clusters': cs,
            'paint': {'r': 0, 'g': 0, 'b': 0, 'a': 1}}
def page(n, items): return {'number': n, 'width': 612 * T, 'height': 792 * T, 'items': items}
def header(rev, text):
    return {'render_format': 'display-list-v2', 'coordinate_unit': 'bp_2pow20', 'color_space': 'srgb', 'text_extraction': 'cluster-actualtext',
            'project_id': 'demo', 'revision': rev, 'required_features': ['glyph_run', 'rgba-srgb', 'cluster-actualtext'],
            'documents': [{'path': 'main.tex', 'revision': rev, 'sha256': hashlib.sha256(text.encode()).hexdigest(), 'byte_length': len(text.encode())}],
            'fonts': [{'font_id': FONT, 'sha256': FONT, 'byte_length': 154012, 'format': 'opentype-cff', 'face_index': 0, 'units_per_em': 1000, 'glyph_count': 821, 'postscript_name': 'LMRoman12-Regular'}],
            'diagnostics': []}

T0 = "\\begin{document}\nHi\n\\newpage\nBye\n\\end{document}\n"
T1 = "\\begin{document}\nHio\n\\newpage\nBye\n\\end{document}\n"
assert T0.index('Hi') == 17 and T0.index('Bye') == 29 and T1.index('Bye') == 30
H, I, O = (43, 9437184), (76, 3495390), (82, 6291456)
B, Y, E = (37, 8912896), (92, 6640435), (72, 5767168)

base = header(7, T0); base['pages'] = [page(1, [run('Hi', [H, I], 17)]), page(2, [run('Bye', [B, Y, E], 29)])]
base_env = {'protocol_version': 2, 'id': 'mac-42', 'type': 'display_list', 'payload': base}
fresh = header(8, T1); fresh['pages'] = [page(1, [run('Hio', [H, I, O], 17)]), page(2, [run('Bye', [B, Y, E], 30)])]
fresh_env = {'protocol_version': 2, 'id': 'mac-43', 'type': 'display_list', 'payload': fresh}

base_pd = [page_digest(p) for p in base['pages']]
fresh_pd = [page_digest(p) for p in fresh['pages']]
delta = {k: fresh[k] for k in ('render_format', 'coordinate_unit', 'color_space', 'text_extraction', 'project_id', 'revision', 'required_features')}
delta['digest_scheme'] = 'dl2-canon-1'
delta['base'] = {'request_id': 'mac-42', 'project_id': 'demo', 'revision': 7, 'page_count': 2, 'list_digest': list_digest(base, base_pd)}
delta['documents'] = fresh['documents']; delta['fonts'] = fresh['fonts']; delta['diagnostics'] = fresh['diagnostics']
delta['relocations'] = [{'path': 'main.tex', 'edit_start': 19, 'edit_end': 19, 'delta': 1}]
delta['page_count'] = 2
delta['page_digests'] = fresh_pd
delta['page_bytes'] = [page_bytes(p) for p in fresh['pages']]
delta['changed_pages'] = [fresh['pages'][0]]
delta['removed_pages'] = []
delta['list_digest'] = list_digest(fresh, fresh_pd)
delta_env = {'protocol_version': 2, 'id': 'mac-43', 'type': 'display_list_delta', 'payload': delta}

recon = apply_delta(base_env, delta_env)
assert recon == fresh_env, 'reconstruction differs'
assert json.dumps(recon, sort_keys=True, separators=(',', ':')) == json.dumps(fresh_env, sort_keys=True, separators=(',', ':'))

if __name__ == '__main__':
    what = sys.argv[1] if len(sys.argv) > 1 else 'digests'
    if what == 'base': print(json.dumps(base_env, indent=1))
    elif what == 'delta': print(json.dumps(delta_env, indent=1))
    elif what == 'fresh': print(json.dumps(fresh_env, indent=1))
    else:
        print('T0 sha256', hashlib.sha256(T0.encode()).hexdigest(), len(T0.encode()))
        print('T1 sha256', hashlib.sha256(T1.encode()).hexdigest(), len(T1.encode()))
        print('base page digests', base_pd)
        print('base header', header_digest(base)); print('base list', delta['base']['list_digest'])
        print('fresh page digests', fresh_pd)
        print('fresh header', header_digest(fresh)); print('fresh list', delta['list_digest'])
        print('base line bytes', len(json.dumps(base_env, separators=(',', ':'))), 'delta line bytes', len(json.dumps(delta_env, separators=(',', ':'))), 'fresh line bytes', len(json.dumps(fresh_env, separators=(',', ':'))))
        print('page_bytes base', [page_bytes(p) for p in base['pages']], 'fresh', delta['page_bytes'])
        print('exact target bytes', target_bytes(delta_env, base_env, [page_bytes(p) for p in base['pages']]), '== len(fresh line)', len(wire(fresh_env).encode()))
        print('reconstruction == fresh: True')
```

Output on 2026-09-12 (Python 3, mac-m1max-a), re-run unchanged for r3 after the
`f64` rule change (the example's paint components are 0 and 1, so the digests
are unaffected; a separate check confirmed that `r: -0.0` hashes equal to
`r: 0` and that a NaN component raises `ValueError` instead of hashing):

```
T0 sha256 580f3c9f730136acf7979d99d8702b41220588ad2cf482a773b7a2b277db29f2 48
T1 sha256 1e2dc5fa4cfdf13ffde12d7e17a142ad0b7389be3aabe5ce54535f221d4b1eee 49
base page digests ['154625d5e6e3596b9af130702f332a98141e08984216fbe68947183ac0f9aa77', '1748b1d3e714ec6222ee2fdf8d06f86943e0bab41806c3a0715435ef29a7a061']
base header 1ce217312d6d30d9809528a6d656025f4711b1c037cd7b75b9c46a74ff600618
base list 68db4fe3ae528414058efecd7d5870c689d598f415a4c6045a86173d7514eec1
fresh page digests ['2a2f15ca94dd927ace6c633f69cf5db765ccdf8622df40ca107b93d850edee94', '4d71e83b0f5b28c5db04e4232d75fde0deb61912acb6e4f5b37c0969ecf7518e']
fresh header 7411c7fde8668f1bf2e8cf39a762d2e4b7eac6c4db3967fd8cfb4f2096503ab8
fresh list 6e93b60654755c252c79580be04e5d964d7b9fc4aee167d6124afc766a6c4e1e
base line bytes 3145 delta line bytes 2701 fresh line bytes 3498
page_bytes base [1014, 1367] fresh [1367, 1367]
exact target bytes 3498 == len(fresh line) 3498
reconstruction == fresh: True
```

(r5: the delta line grew by 25 bytes for `page_bytes`; a separate check moved a
page whose spans sit at 97–100 by +1, +3, −90 and +903 and found the
digit-width arithmetic equal to the actual serialised length in every case.)

(The byte counts are Python's compact `json.dumps` of the example, not the
producer writer's; the example is too small for the delta to be meaningfully
smaller — it exists for the digests and the algorithm, not for size.)

## Appendix B — worked example: base full list → one-word edit → delta → reconstruction

Geometry is illustrative (a 12 pt run at the 1 in margin, baseline at 84 bp),
not pipeline output; the font id/sha256 `c1f0e5d6…` is the 64-hex constant
`FONT` of Appendix A, abbreviated here for width only (digests were computed
over the full value). Source: `T0` = `\begin{document}\nHi\n\newpage\nBye\n\end{document}\n`
(48 bytes; `Hi` at bytes 17–19, `Bye` at 29–32). The edit inserts `o` at byte
19 (`T1`, 49 bytes; `Bye` now at 30–33): relocation `{edit_start: 19,
edit_end: 19, delta: 1}`.

### B.1 Base: the full `display_list` sibling of request `mac-42` (revision 7)

```json
{"protocol_version":2,"id":"mac-42","type":"display_list","payload":{
  "render_format":"display-list-v2",
  "coordinate_unit":"bp_2pow20",
  "color_space":"srgb",
  "text_extraction":"cluster-actualtext",
  "project_id":"demo",
  "revision":7,
  "required_features":["glyph_run","rgba-srgb","cluster-actualtext"],
  "documents":[{"path":"main.tex","revision":7,"sha256":"580f3c9f730136acf7979d99d8702b41220588ad2cf482a773b7a2b277db29f2","byte_length":48}],
  "fonts":[{"font_id":"c1f0e5d6…","sha256":"c1f0e5d6…","byte_length":154012,"format":"opentype-cff","face_index":0,"units_per_em":1000,"glyph_count":821,"postscript_name":"LMRoman12-Regular"}],
  "pages":[
    {"number":1,"width":641728512,"height":830472192,"items":[
     {"kind":"glyph_run","font_id":"c1f0e5d6…","font_size":12582912,"text":"Hi",
      "glyphs":[
       {"gid":43,"origin_x":75497472,"baseline_y":88080384,"advance_x":9437184,"advance_y":0,"cluster":0},
       {"gid":76,"origin_x":84934656,"baseline_y":88080384,"advance_x":3495390,"advance_y":0,"cluster":1}
      ],
      "clusters":[
       {"text_start_byte":0,"text_end_byte":1,"hit_rects":[{"x":75497472,"top":78643200,"width":9437184,"height":12582912}],"carets":[{"text_byte":0,"x":75497472,"top":78643200,"height":12582912}],"sources":[{"path":"main.tex","start_byte":17,"end_byte":18}]},
       {"text_start_byte":1,"text_end_byte":2,"hit_rects":[{"x":84934656,"top":78643200,"width":3495390,"height":12582912}],"carets":[{"text_byte":1,"x":84934656,"top":78643200,"height":12582912},{"text_byte":2,"x":88430046,"top":78643200,"height":12582912}],"sources":[{"path":"main.tex","start_byte":18,"end_byte":19}]}
      ],
      "paint":{"r":0,"g":0,"b":0,"a":1}}
    ]},
    {"number":2,"width":641728512,"height":830472192,"items":[
     {"kind":"glyph_run","font_id":"c1f0e5d6…","font_size":12582912,"text":"Bye",
      "glyphs":[
       {"gid":37,"origin_x":75497472,"baseline_y":88080384,"advance_x":8912896,"advance_y":0,"cluster":0},
       {"gid":92,"origin_x":84410368,"baseline_y":88080384,"advance_x":6640435,"advance_y":0,"cluster":1},
       {"gid":72,"origin_x":91050803,"baseline_y":88080384,"advance_x":5767168,"advance_y":0,"cluster":2}
      ],
      "clusters":[
       {"text_start_byte":0,"text_end_byte":1,"hit_rects":[{"x":75497472,"top":78643200,"width":8912896,"height":12582912}],"carets":[{"text_byte":0,"x":75497472,"top":78643200,"height":12582912}],"sources":[{"path":"main.tex","start_byte":29,"end_byte":30}]},
       {"text_start_byte":1,"text_end_byte":2,"hit_rects":[{"x":84410368,"top":78643200,"width":6640435,"height":12582912}],"carets":[{"text_byte":1,"x":84410368,"top":78643200,"height":12582912}],"sources":[{"path":"main.tex","start_byte":30,"end_byte":31}]},
       {"text_start_byte":2,"text_end_byte":3,"hit_rects":[{"x":91050803,"top":78643200,"width":5767168,"height":12582912}],"carets":[{"text_byte":2,"x":91050803,"top":78643200,"height":12582912},{"text_byte":3,"x":96817971,"top":78643200,"height":12582912}],"sources":[{"path":"main.tex","start_byte":31,"end_byte":32}]}
      ],
      "paint":{"r":0,"g":0,"b":0,"a":1}}
    ]}
  ],
  "diagnostics":[]
}}
```

Base snapshot (both sides): `page_digests = [154625d5…aa77, 1748b1d3…a061]`,
`list_digest = 68db4fe3ae528414058efecd7d5870c689d598f415a4c6045a86173d7514eec1`.

### B.2 The edit and the request

`Hi` → `Hio`. The consumer installed `mac-42` (validated, digested, painted)
and acknowledges it in the request:

```json
{"protocol_version":1,"id":"mac-43","type":"compile","payload":{
  "project_id":"demo","revision":8,"entry_path":"main.tex",
  "documents":[{"path":"main.tex","text":"\\begin{document}\nHio\n\\newpage\nBye\n\\end{document}\n"}],
  "layout_capabilities":["rules-v1","font-hints-v1","display-list-v2","display-list-v2-delta"],
  "display_list_base":{"request_id":"mac-42","project_id":"demo","revision":7,"page_count":2,
                       "list_digest":"68db4fe3ae528414058efecd7d5870c689d598f415a4c6045a86173d7514eec1"}}}
```

The producer's last emitted snapshot is `mac-42` with that `list_digest`, so
the acknowledgement matches; the document set is the same; the diff
gives `a = 19, b = 19, d = +1`. Page 1 retypesets (changed); page 2's only
block is a cache hit relocated by +1 and `relocate(base page 2) == new page 2`
(unchanged). The `compile_result` for `mac-43` echoes both capabilities.

### B.3 The delta sibling of `mac-43`

```json
{"protocol_version":2,"id":"mac-43","type":"display_list_delta","payload":{
  "render_format":"display-list-v2",
  "coordinate_unit":"bp_2pow20",
  "color_space":"srgb",
  "text_extraction":"cluster-actualtext",
  "project_id":"demo",
  "revision":8,
  "required_features":["glyph_run","rgba-srgb","cluster-actualtext"],
  "digest_scheme":"dl2-canon-1",
  "base":{"request_id":"mac-42","project_id":"demo","revision":7,"page_count":2,"list_digest":"68db4fe3ae528414058efecd7d5870c689d598f415a4c6045a86173d7514eec1"},
  "documents":[{"path":"main.tex","revision":8,"sha256":"1e2dc5fa4cfdf13ffde12d7e17a142ad0b7389be3aabe5ce54535f221d4b1eee","byte_length":49}],
  "fonts":[{"font_id":"c1f0e5d6…","sha256":"c1f0e5d6…","byte_length":154012,"format":"opentype-cff","face_index":0,"units_per_em":1000,"glyph_count":821,"postscript_name":"LMRoman12-Regular"}],
  "diagnostics":[],
  "relocations":[{"path":"main.tex","edit_start":19,"edit_end":19,"delta":1}],
  "page_count":2,
  "page_digests":[
    "2a2f15ca94dd927ace6c633f69cf5db765ccdf8622df40ca107b93d850edee94",
    "4d71e83b0f5b28c5db04e4232d75fde0deb61912acb6e4f5b37c0969ecf7518e"
  ],
  "page_bytes":[1367,1367],
  "changed_pages":[
    {"number":1,"width":641728512,"height":830472192,"items":[
     {"kind":"glyph_run","font_id":"c1f0e5d6…","font_size":12582912,"text":"Hio",
      "glyphs":[
       {"gid":43,"origin_x":75497472,"baseline_y":88080384,"advance_x":9437184,"advance_y":0,"cluster":0},
       {"gid":76,"origin_x":84934656,"baseline_y":88080384,"advance_x":3495390,"advance_y":0,"cluster":1},
       {"gid":82,"origin_x":88430046,"baseline_y":88080384,"advance_x":6291456,"advance_y":0,"cluster":2}
      ],
      "clusters":[
       {"text_start_byte":0,"text_end_byte":1,"hit_rects":[{"x":75497472,"top":78643200,"width":9437184,"height":12582912}],"carets":[{"text_byte":0,"x":75497472,"top":78643200,"height":12582912}],"sources":[{"path":"main.tex","start_byte":17,"end_byte":18}]},
       {"text_start_byte":1,"text_end_byte":2,"hit_rects":[{"x":84934656,"top":78643200,"width":3495390,"height":12582912}],"carets":[{"text_byte":1,"x":84934656,"top":78643200,"height":12582912}],"sources":[{"path":"main.tex","start_byte":18,"end_byte":19}]},
       {"text_start_byte":2,"text_end_byte":3,"hit_rects":[{"x":88430046,"top":78643200,"width":6291456,"height":12582912}],"carets":[{"text_byte":2,"x":88430046,"top":78643200,"height":12582912},{"text_byte":3,"x":94721502,"top":78643200,"height":12582912}],"sources":[{"path":"main.tex","start_byte":19,"end_byte":20}]}
      ],
      "paint":{"r":0,"g":0,"b":0,"a":1}}
    ]}
  ],
  "removed_pages":[],
  "list_digest":"6e93b60654755c252c79580be04e5d964d7b9fc4aee167d6124afc766a6c4e1e"
}}
```

### B.4 Reconstruction on the consumer

1. `base` equals the held snapshot: `mac-42` / `demo` / 7 / 2 pages /
   `68db4fe3…eec1`. `digest_scheme` known. `page_count` 2, `changed_pages`
   = {1}, `removed_pages` = [] = `[3..2]`. OK.
2. Page 1 := the changed page as sent; `page_digest` = `2a2f15ca…ee94` = `page_digests[0]`. OK.
3. Page 2 := relocate(base page 2, `{19, 19, +1}`): every span has
   `start_byte ≥ 19`, so `29–30 → 30–31`, `30–31 → 31–32`, `31–32 → 32–33`;
   glyphs, hit rects, carets, paint untouched. `page_digest` =
   `4d71e83b…518e` = `page_digests[1]`. OK.
3b. Accounting, done BEFORE steps 2–3 in the real consumer: `page_bytes[0]`
   = 1 367 = measured length of the changed page object in the delta line;
   `page_bytes[1]` = 1 367 = cached length of installed page 2 (1 367) + digit
   deltas of its three relocated spans (29→30, 30→31, 31→32, 32→33: all stay
   2 digits, so 0). Exact target = framing + header parts + 1 367 + 1 367 + 1
   separator = 3 498 bytes = the length of the fresh full line (Appendix A
   prints both); 3 498 ≤ `MAX_SNAPSHOT_BYTES`, admitted. (Byte counts here use
   the reference's compact writer as a stand-in; the producer's writer is
   normative and the gate compares against its bytes.)
4. Header from the delta (`documents` with the new sha256/49 bytes, `fonts`,
   `diagnostics`, features, `demo`/8) + pages → `list_digest` =
   `6e93b606…4e1e` = the delta's. OK.
5. The full validator runs on the result; it is then the new base
   (`mac-43` / `demo` / 8 / 2 / `6e93b606…4e1e`).

The reconstructed envelope (with `type` `display_list` and id `mac-43`) equals
the fresh full list for `T1`, field for field (Appendix A asserts it); in the
producer gate the same equality is asserted on the producer's own JSON bytes.
