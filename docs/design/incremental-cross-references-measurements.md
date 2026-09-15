# Incremental cross-references: step 0 measurements

Status: **measurement only. No behaviour change, and no design decision.**
Task `GH-XREF-MEASURE-STEP0`, by daniel-parent, 2026-09-15, against origin/main
`36fe7ec3`. This is step 0 of the design proposal in PR #575
(`docs/design/incremental-cross-references.md` on
`agent/daniel-parent/incremental-xref-design`, §2.2 and §5 step 0). It is a
separate file so that it does not conflict with #575.

## What was measured

- **Worker path.** `flashtex-render`'s request path:
  `protocol::handle_line` with one `RenderCache` kept across requests, as in
  `protocol::serve`. The capabilities are `rules-v1` and `font-hints-v1`.
- **Counters.** `RenderCache::counters()` is new in this PR. It reports hits
  and misses for all three maps: `adapted`, `blocks` (the existing `stats()`)
  and `assembled`. It also reports `label_passes`, one per layout pass of
  `render_cached`, so the number of page-number relayouts is `passes - 1`. The
  counters are plain `Cell`s that nothing on the render path reads.
- **Documents.**
  - `535` is #535's `large_document(450)`, byte for byte: 637 761 B, with
    labels, `\ref`, `\eqref`, `\cite` and `thebibliography`. It has no
    `\pageref` and no contents list. The generator was checked against #535's
    `large_doc_bench.rs` and produced identical bytes.
  - `535+toc+pageref` is the same document plus `\tableofcontents` and one
    `\pageref{sec:N}` per section (649 369 B), so that page numbers are
    needed.
  - This pipeline sets `535` on **398 pages**; the compiler worker uses 302.
    `535+toc+pageref` has 419 pages.
- **Edits** (§5 step 0). Each run starts from a fresh cache warmed with the
  unedited document. The table reports only the edit request.
  - `no-label` types one character mid-document (#535's edit).
  - `renumber` inserts `\section{An inserted section}\label{sec:inserted}`
    and a paragraph before section 1. Every later section number and every
    `\ref{sec:N}` changes.
  - `move-label` moves one paragraph that contains no label from after a
    section heading to before it. The bench picks the first mid-document
    section where that moves its `\label` to another page. It chose section
    226 in `535` (page 198 → 199) and section 231 in `535+toc+pageref` (page
    223 → 224). In both cases exactly 1 of the 450 section headings changed
    page, and the page count stayed the same.
- **Latency** is the median of 5 runs on an Apple M5 Pro in a release build.
  The machine was shared, with a load average of 23 / 21 / 19 at the start.
  - `render ms` is `Rendered::elapsed_ms`.
  - `handle_line ms` adds the reply. For these documents the reply is the
    over-limit failure: the `compile_result` would be 17.6 MB, over the
    16 MiB line limit (#294 is the fix). So `render ms` is the meaningful
    figure.

## Results

"miss / lookups" counts only the edit request. Lookups include every pass.
In the cold request, 898 `adapted` lookups per pass also hit, because
repeated identical blocks (for example `\item a third observation.`) share a
key. So "every distinct block missed" appears as 4 115–4 117 misses per pass,
not 5 013.

| document | edit | adapted miss / lookups | blocks miss / lookups | assembled miss / lookups | layout passes | handle_line ms (median) | render ms (median) |
|---|---|---:|---:|---:|---:|---:|---:|
| 535 | no-label | 0 / 5013 | 1 / 5461 | 1 / 5461 | 1 | 18493.1 | 18463.1 |
| 535 | renumber | 4117 / 5015 | 902 / 5463 | 902 / 5463 | 1 | 17383.1 | 17354.2 |
| 535 | move-label | 1 / 5013 | 3 / 5461 | 3 / 5461 | 1 | 16189.0 | 16162.4 |
| 535+toc+pageref | no-label | 0 / 10026 | 1 / 10924 | 1 / 5462 | 2 | 34935.8 | 34902.1 |
| 535+toc+pageref | renumber | 8234 / 10030 | 1239 / 10928 | 1236 / 5464 | 2 | 35206.4 | 35172.1 |
| 535+toc+pageref | move-label | 4116 / 10026 | 4 / 10924 | 4 / 5462 | 2 | 35405.4 | 35373.7 |

For comparison, the **cold** request, which is the same for every edit:

| document | adapted miss / lookups | blocks miss / lookups | assembled miss / lookups | passes | render ms (first run) |
|---|---:|---:|---:|---:|---:|
| 535 | 4115 / 5013 | 5012 / 5461 | 5012 / 5461 | 1 | 18277.6 |
| 535+toc+pageref | 8230 / 10026 | 5463 / 10924 | 5013 / 5462 | 2 | 34737.7 |

## What the numbers show

1. **§2.2's invalidation analysis holds.**
   - `renumber` misses `adapted` for every distinct block in every pass, both
     with and without `\pageref`.
   - `move-label` misses it for every distinct block in pass 2 only: pass 1
     starts from empty page numbers and hits.
   - Below the adapter, reuse is selective, as §2.2 says:
     - `move-label` misses 3–4 of about 5 460 `blocks`;
     - `renumber` misses 902 in `535` and 1 239 in `535+toc+pageref`. Those
       are the blocks whose adapted items actually changed: the renumbered
       headings, blocks with a `\ref{sec:N}`, the inserted section and, in
       the second document, the contents list and the `\pageref` blocks.
   - `no-label` misses one block, and the page numbers take no extra pass
     beyond the second pass that `\pageref` or a contents list always costs.
2. **The warm latency is the cold latency, whatever the edit.** A
   one-character edit that reuses 5 460 of 5 461 blocks takes 18.5 s, and
   the cold render takes 18.3 s. On this pipeline, the cache misses above
   are not what costs the time.
3. **The time is in the adapter, and it grows quadratically.** It is not in
   the maps. The committed `stages` example times a render twice; the second
   run is warm, with every lookup a hit:

   | sections | parse | adapt | typeset (shape + break + pages) | assemble v2 | v1 | json |
   |---:|---:|---:|---:|---:|---:|---:|
   | 150 (136 pages) | 5.0 | **2013.1** | 21.3 | 15.0 | 1.5 | 3.6 |
   | 450 (398 pages) | 14.9 | **16368.2** | 46.6 | 36.5 | 4.9 | 11.1 |

   Three times the sections cost 8.1 times the adapt time. Temporary timing
   probes placed inside `adapter::adapt_cached` (not committed) on the
   150-section document put 1.84–1.91 s of its 1.96 s in one call to
   `split_at_page_breaks`. Three per-block scans account for almost all of
   that:
   - **717 ms in `in_theorem_environment(text, block_start, …)`.** This runs
     for every block. It walks `text[..block_start]` from byte 0, and each
     step searches again for both the next `\begin` and the next `\end`.
     It does not return early when the document defines no theorem-like
     environment. It was added by `2945f61a` (amsthm as a `\trivlist`,
     2026-09-13).
   - **831 ms in the `CBlock::ListItem` branch.** It calls
     `list_stack_at(src, at.start)`, which has the same prefix-walk shape, as
     well as `list_seps_with` and `list_margins`. The probes did not split the
     time between these three.
   - **290 ms in the branch that runs after a list.** It calls `list_stack_at`
     for the previous block's end.

   For comparison, the unit loop's match arms took 100 ms, and building the
   `items_cached` keys took 1 ms. These are profiling notes for whoever owns
   `adapter.rs`, not a proposed change.

## Consequences for #575 step 0 (facts only, no ruling)

- The step 0 question was whether the IDE path is already fast on all three
  edits. It is **not fast on any of them** for this document: 16–18 s with
  one pass and 35 s with two. The reason is not cross-reference
  invalidation.
- As long as the adapter's prefix scans dominate, narrowing `labels_fp`
  (Option A step 2) or seeding page numbers (Option A step 4) in the render
  pipeline would not visibly change latency on this document. The
  counters show what those two changes would save: re-adapting 4 116 blocks
  per affected pass, and the second pass itself. Their latency value can
  only be measured after the scans are linear. Re-running this bench then
  gives the number.
- Nothing here changes #575's compiler-worker analysis (§2.1).

## Commands

From a worktree at this branch:

```sh
cd crates/render-pipeline
export CARGO_BUILD_JOBS=4 CARGO_TARGET_DIR=/Users/dqi26/flashtex/target-xrefmeasure

# The table above (about 12 minutes on a loaded machine):
cargo run --release --example xref_measure -- 5 450
# Quick check (40 sections, 2 runs, about 20 s):
cargo run --release --example xref_measure -- 2 40
# The documents:
cargo run --release --example xref_measure -- --emit 450      > doc535.tex
cargo run --release --example xref_measure -- --emit 450 toc  > doctoc.tex
cargo run --release --example xref_measure -- --emit 150      > doc150.tex

# Stage split (second line = warm):
cargo run --release --example stages -- doc150.tex 2
cargo run --release --example stages -- doc535.tex 2

# The counters' own test:
cargo test --test cache_counters
```

- `XREF_MEASURE_DEBUG=1` prints the page of every section heading and each
  `move-label` candidate the bench rejects.
- On a 40-section document (62 KB, 40 pages) the same bench measured about
  200 ms cold and 190 ms warm, and showed the same miss pattern.
