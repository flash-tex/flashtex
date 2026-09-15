# Incremental cross-references (design proposal)

Status: **proposal for the Commander's decision. Nothing here is implemented.**
Task `GH-INCREMENTAL-XREF-DESIGN`, by daniel-parent, 2026-09-15, against
origin/main `36fe7ec3`. Source finding: PR #535 (`GH-65-COMPILER-PERF`).

## 1. Problem

In `crates/compiler`, `ParsedDocument::document_global_state` is set by any
of: `\label`, `\ref`, `\pageref`, `\eqref`, `\cref`, `\cite`, `\bibitem`,
`\tableofcontents`, `\footnote`, `\thanks`, `\chapter`, `\numberwithin`,
`subequations`, `\crefname`, or enumitem `series`/`resume`. That is 18 sites in
`parser.rs`. When the flag is set, `incremental::Session::compile_project_with`
skips block reuse entirely and calls `layout::layout_converged_with_options`.
That function lays out the whole document again and again until labels and
contents entries stop changing. Almost every real paper uses at least one of
these commands, so for real papers the warm session is a cold compile.

Measured in #535 on an Apple M5 Pro (release build, shared machine at load
90–106, medians):

| 302-page article: 637 761 B, 5 912 blocks | ms |
|---|---:|
| cold `compile_full_project` (after #535) | 203.1 |
| one-character edit, warm `Session` | 265.3, with `blocks_reused: 0, full_recompile: true` |
| the same edit in a quieter window (single run) | 119.1 |
| **for contrast:** 500 KB document with no cross-references, one-character edit, warm `Session` p50 | **28.5** (reuses 7 753 of 7 754 blocks) |

#535's profile puts `parse_project_with` at about 15% of samples. Every
revision parses the whole project (§2), so parsing sets a latency floor that no
layout-reuse design can remove. FT-070's target (below) is well under that
floor.

## 2. Current dependency model

### 2.1 `crates/compiler` (the `flashtex-compiler` worker; Core 14 layout)

- **The parser is global and exact.** Counters are stepped during parsing
  (`xref.rs`: reset lists and `\the<name>` formats), and their values are
  stored in the block: `Block::Heading { number }`,
  `Inline::Label { key, value, kind }`. Citations are resolved during parsing
  too: `bib::prescan` runs before the parse, then `bib::cite_inlines` emits
  label text such as `[3]` directly. So when a section is inserted, the later
  headings' `number` fields change and ordinary block equality recomputes
  them. **Counters and cites are not the problem.**
- **Block reuse is layout-only.** A `CachedBlock` is reused when all of these
  hold:
  - the shifted `Block` is structurally equal;
  - the `MacroDependency` reads are equal;
  - the preamble bytes and `LayoutConstraints` are equal;
  - `FlowState::same_geometry` holds: the same page index, x, y, line metrics,
    trailing items, `content_end` and `closed_line_skip`.
- **What the flag protects** is state that layout reads from *other* blocks,
  none of which a `CachedBlock` records:

| Construct | Layout reads | Exact after parse? |
|---|---|---|
| `\ref` / `\eqref` / `\cref` number | `resolved_labels[key].{number,kind}` | **yes**, from the `Inline::Label` values |
| `\pageref` / `\cpageref` | `resolved_labels[key].page` | no |
| `\tableofcontents` | `resolved_toc`: every numbered heading's text, number and page | page: no |
| `\footnote` | `FootnoteState`, the page-bottom insertions carried across blocks, which `FlowState` does not include | no |
| `\cite` / `\bibitem` | nothing; the text is already in the block | yes. The flag looks conservative |
| `\numberwithin`, `subequations`, enumitem `series`/`resume`, body `\crefname` | the value in the block, or `CleverefConfig` | believed yes; not audited |

- **Each keystroke costs a parse plus at least two full layouts.**
  `layout_converged_with_options` starts from an *empty* label table:
  1. Pass 1 sets every reference to `??` and collects the labels.
  2. Pass 2 sets the real numbers, whose widths can move line and page breaks.
  3. It stops once collected == consumed, after
     `REFERENCE_ITERATION_LIMIT = 5` passes, or on oscillation, with a warning.
- **A limitation that is independent of cross-references:** geometry must match
  bit for bit, and nothing resynchronises at page boundaries. So an edit that
  changes one paragraph's line count recomputes every later block.

### 2.2 `crates/render-pipeline` (`flashtex-render`, the IDE's primary worker)

`apps/mac` `ShellModel.locateDefaultProducer` prefers `flashtex-render` and falls back
to `flashtex-compiler`. preview-controller spawns the worker and talks JSON
Lines to it (`render_pipeline::protocol::handle_line`). The render pipeline
parses with its **vendored, diverged** copy of the compiler
(`crates/render-pipeline/vendor/compiler`) and **ignores
`document_global_state`**. Its model is already most of what §3 proposes for
the compiler:

- `adapter::Labels::from_parsed` takes the reference numbers from the parse, so
  there is no `??` pass for numbers.
- `lower_inline` turns each `\ref`/`\pageref` into plain text *before* the
  block is hashed. The `RenderCache` key is a hash of the block's content, with
  source offsets relative to the block, and not its geometry. So only blocks
  whose resolved text changed miss the cache. `Labels::toc_pages` is kept
  separate "so the paragraph cache key is unaffected".
- Page numbers still start **empty on every request**. `render_cached` runs up
  to `MAX_LABEL_PASSES = 3` passes, each a full page build plus assembly,
  whenever `needs_pages` is true or the document has contents lists. Within a
  pass, blocks come from the cache.

**The 0-of-5 912 finding therefore belongs to the compiler worker, not to the
render pipeline.** Nobody has yet measured #535's 302-page document through
`flashtex-render`. That measurement is step 0 below.

## 3. Options

### Option A — resolve before comparing, plus a fixpoint over dirty page keys

This ports the render pipeline's decoupling into `incremental.rs` and adds
what neither worker has yet: page values carried over between revisions.

1. **Numbers come from the parse.** Walk the blocks once to build
   `key → (number, kind)`, last definition wins, exactly like
   `Labels::from_parsed`. The resolved reference *text* of each block goes
   into its comparison key, so a changed `\ref` value recomputes exactly its
   referrers. This needs no new read-tracking for numbers.
2. **Page reads are recorded.** A `CachedBlock` gains
   `label_writes: [(key, page)]` and `toc_writes`, so a reused block still
   contributes them; its page index is guaranteed equal by `same_geometry`. It
   also gains `page_reads: [(key, observed page)]`, plus a TOC fingerprint for
   `Block::TableOfContents`. The reuse condition becomes: today's condition
   **and** every page read equals the current table.
3. **The fixpoint is the existing reuse loop run again.** Pass 1 starts from
   the previous revision's *final* page table instead of an empty one. If the
   collected pages or TOC differ from the table the pass consumed, the loop
   runs again. The previous "revision" is then the last pass, with no byte
   changes, so a clean block with an unchanged read is reused. It keeps the
   same limit and the same oscillation detection. **When it oscillates or hits
   the limit, it falls back to today's clean `layout_converged` path.**
4. **Narrow the flag site by site,** each site with a warm == fresh proof test:
   - cites, `\bibitem`, `series`/`resume` (the value is already in the block);
   - labels, refs and the TOC (items 1–3 above);
   - footnotes, which need `FootnoteState` inside `FlowState`;
   - every unaudited site keeps the flag.

*Correctness risks.*
- (a) **Where the fixpoint starts.** A clean compile starts from empty pages
  and the warm one from the previous revision's pages. A document with two
  self-consistent fixpoints could therefore render differently in the two
  paths, e.g. a `\pageref` whose own width decides whether it is "9" or "10".
  The render pipeline would have the same risk if it seeded its pages.
- (b) **An unrecorded page read gives stale output with no error.** Route every
  `resolved_labels.page` and `resolved_toc` access through one recording
  accessor.
- (c) **`page` must keep today's meaning,** which is `pages.len()` when the
  label is emitted.
- (d) **Undefined-reference warnings** (`visit_references`) must still be
  recomputed on every revision.

Counters add no new risk; they stay in the parser.

*Expected latency.*
- **About 30–40 ms** under load for the 302-page document, down from 265: a
  parse plus a reuse walk, in line with the 500 KB figure.
- **Up to one full layout** when an edit changes a line count early in the
  document, or changes the height of the contents list. That is still below
  today's two or more.

*Complexity.* Medium. About 500–800 lines across `incremental.rs`,
`layout.rs` and the flag sites, plus tests. No protocol change.
*Reversibility.* High. The clean path remains the authority and the fallback,
and one `Session` switch turns the new behaviour off.

*Tests.*
- Extend the `incremental_json_identity.rs` byte-identity replay to fixtures
  that use labels, refs, the TOC, `\pageref`, cites and footnotes.
- On #535's `large_document_timing` generator, assert
  `full_recompile == false` and at least 99% `blocks_reused` for a
  mid-document edit.
- Targeted cases:
  - a section inserted before a referenced label (only referrers recompute);
  - label rename, deletion (`??` plus a warning) and duplicates;
  - forward references;
  - an edit that pushes a label onto the next page, under both `\pageref` and
    `\cpageref`;
  - typing in a heading with and without a line-count change (the TOC is
    reused or reflows);
  - `\eqref` inside `align`;
  - a `\cite` key edit;
  - `\crefname` in the body;
  - an oscillating `\pageref` fixture, which must fall back and equal the clean
    compile.
- A randomized edit-script differential test: `Session` against
  `compile_full_project`.

### Option B — incremental parsing with parser-state checkpoints per block

Snapshot the parser state at block boundaries: counters, the macro table, the
group stack, catcodes, the `bib::prescan` cursor, the list stack and the label
table. On an edit, re-parse from the first dirty block and stop where a
boundary snapshot matches the previous revision's.

*Correctness risks.* **High.** Every mutable piece of parser state has to be
in the snapshot, including grouped `\def`, conditionals, `\input`, recovery
paths and the prescan lockstep with `\bibitem`. A missing field silently
corrupts later output. The module header of `incremental.rs` currently rules
this out ("When in doubt, rebuild").
*Latency.* It removes the parse floor, bringing edits down to a few ms. On its
own it does **nothing** for cross-references, because layout still reads
tables that span blocks. This is an add-on to A, not an alternative.
*Complexity.* High; most of `parser.rs` (~10 k lines) is involved.
*Reversibility.* Good if it stays behind a switch.
*Tests.* Snapshot-equality fuzzing, and every parser test re-run through a warm
re-parse.

### Option C — the aux-file model: stale references while typing, reconciled in the background

Treat the previous revision's labels and TOC like LaTeX's `.aux` file. The
interactive compile does **one** reuse pass and never iterates. If the table it
collects differs from the one it consumed, the reply is marked `xrefs_stale`.
A debounced background full compile (on idle, or about 300 ms after the last
edit) then publishes the reconciled revision. Export always uses the clean
path.

*Correctness risks.*
- Values change visibly after a pause, e.g. "Section 3" becomes "Section 4".
- A warm reply is no longer byte-identical to a clean compile, which relaxes
  FT-070's hard gate to *eventual* identity.
- A background result must never overwrite a newer revision. `SESSIONS` in
  `protocol.rs` is one `Mutex`.
- Reuse still needs A's resolve-before-compare step, or reused blocks keep
  stale text.

*Latency.* The interactive path matches A's first pass and is never more than
one pass. Reconciliation costs one cold compile per pause (100–200 ms of CPU).
*Complexity.* Low–medium in the compiler. Medium across components: a new
runtime-v1 reply field, preview-controller or host scheduling, and UI for
provisional values.
*Reversibility.* Medium; a protocol field and client behaviour are sticky.
*Tests.* Eventual identity after the pause; a stale result never replaces a
newer one; export == clean compile; the stale marker is set exactly when the
tables differ.

## 4. Interaction with FT-070 and the render pipeline

- **FT-070 owns `crates/compiler`** (kabir-claude; `coordination/assignments/FT-070.json`).
  - Its targets: a warm keystroke under **2 ms at 500 KB** and under **10 ms at
    2 MB**, and **byte-identical output as a hard gate**.
  - Its technique list already names "incremental re-expansion/re-parse by byte
    range" (Option B), "paragraph-level layout reuse keyed by content hash
    (already partially in render-pipeline — make the compiler's own layout do
    the same)", and "page-level reuse with break-point checkpoints".
  - Option A is the cross-reference part of that second technique, so it
    should land **as an FT-070 lane or with FT-070's agreement**, not
    alongside it.
  - The 2 ms target is below the parse floor, so FT-070 will need Option B
    eventually. A does not block B.
- **Open FT-070 PRs do not touch `crates/compiler/src/incremental.rs`.**
  - #206: the `crates/perf-bench` harness. A's acceptance numbers should be
    reported through it once it merges.
  - #273, #232, #283 and #299: `Rc`-shared item vectors, cluster and
    `SourceRange` shrinking. They touch `render-pipeline/src/{incremental,typeset,…}.rs`.
  - #294: a negotiated page window for over-limit replies.
  - A port of the page-table seeding into the render pipeline would conflict
    with #273 and #283 in `render-pipeline/src/incremental.rs`, so it should
    be sequenced after them.
- **Related open reference work:**
  - #486 (`\refstepcounter` feeding `\label`) and #131 (hyperref
    `\autoref`/`\nameref`) add label kinds. A's label table must include
    them, so they are best merged first or rebased onto A.
  - #535 changes `layout.rs` (`inline_box` lends `resolved_labels`) and should
    merge before A starts.
- **The render pipeline already has A's numbers-from-parse and content-keyed
  reuse.** What it lacks is page values seeded from the previous revision, the
  same step 3. That is a small, separate change with the same fixpoint-start
  risk (a). The vendored compiler copy picks up A only when it is next synced.

## 5. Recommendation

**Measure first. Then adopt Option A in phases inside FT-070, keep Option C in
reserve, and leave Option B to FT-070's parse work.**

0. **Measure** #535's 302-page document through `flashtex-render`, both a cold
   compile and a warm one-character edit, and record the number of label passes
   and the cache hits and misses. If the IDE path is already fast, A becomes a
   fix for the fallback worker and drops in priority. If it isn't, seeding the
   page table in the render pipeline is the cheapest first win.
1. **Flag audit.** A small PR that stops setting the flag where the value is
   provably in the block (cites, `\bibitem`, likely `series`/`resume`), each
   with a warm == fresh test. Bibliography-only documents then get block reuse
   immediately.
2. **A, phase 1:** numbers resolved from the parse into the comparison key,
   page read/write sets, a page table seeded from the previous revision, and a
   dirty-key fixpoint that falls back to the clean path. **Acceptance** on
   #535's 302-page document:
   - a one-character edit gives `full_recompile == false` and at least 99%
     `blocks_reused`;
   - p50 is within 1.5× of the 500 KB document without references;
   - warm == fresh byte for byte across the fixture corpus and the random
     edit-script test.
3. **A, phase 2:** footnote state in `FlowState`.
4. **C** only if phase 1 misses the budget on documents with a TOC or heavy
   `\pageref` use. It then only adds "don't iterate interactively" on top of
   A.

Why A: it is the only option that **keeps FT-070's byte-identity gate on every
reply** and still brings documents with cross-references down to
no-reference latency. It needs no protocol or client change. It reuses the
existing reuse loop as its fixpoint engine and copies a pattern the render
pipeline has already proven, so no new mechanism is invented.

## 6. Open questions for the Commander

1. **Identity.** Is "warm reply == clean compile, byte for byte, on every
   revision" non-negotiable? If yes, C is out. If no, is "eventually identical,
   with a stale marker" acceptable?
2. **Multiple fixpoints.** For the rare document where the warm and clean
   fixpoints could differ, is falling back on oscillation enough? Or must CI
   fuzz warm convergence against the clean compile, and does that test block
   merging?
3. **Which worker matters.** Is `flashtex-compiler` (Core 14) still a product
   latency target, or only a fallback? That decides whether step 2 is worth
   doing before or after the render-pipeline page seeding.
4. **Ownership.** Should A run as an FT-070 lane (kabir-claude owns
   `crates/compiler`), or as a Daniel lane with FT-070's review? Who syncs the
   vendored compiler?
5. **Budget.** What per-keystroke budget applies to a ~300-page document with
   cross-references? FT-070 names 500 KB and 2 MB but not reference-heavy
   documents.
6. **Order.** May steps 0 and 1 (measurement and the flag audit) go ahead now,
   before a ruling on A, B or C?
