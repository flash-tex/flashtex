# Proposal: `display-list-v2-window` — a bounded resident page window

Revision **r2, 2026-09-14** (lane linux-primary, FT-070 structural half).
Status: **PROPOSAL + producer, negotiated and opt-in.** `v1.rs` accepts the
name next to `display-list-v2`, exactly as `-only` and `-delta` — themselves
proposals — are accepted today. **No in-tree consumer sends it**, so every
reply on every route is byte-for-byte what it is now; a reply is windowed only
when a request names the capability *and* says where the viewer is.

**What changed in r2**, and why the name is on:

* **`required_features` is whole-document.** r1 derived it from the resident
  pages, so a document whose only rule sat on page 300 would have announced no
  `rule` to a consumer windowed on page 1 — the same failure as a smaller font
  closure, and the one §3 and §5.6 forbid. Assembly now harvests it over every
  block in the same pass as the faces and the diagnostics. This was the reason
  r1 left the capability unnegotiated; it is fixed, so the capability is on.
* **`status` is window-independent** (§4.1). `v1::fallback` derived "did the
  compile produce anything" from the pages it built, which on a windowed reply
  are the window's; it now asks the same whole-document harvest.
* **An over-limit window is narrowed and served, not declined** (§8).
  `MAX_WINDOW_PAGES` bounds what a viewer shows, not bytes, so a consumer may
  legitimately ask for more pages than a reply can carry. Declining that is the
  `status: failed` this proposal exists to remove, so the producer measures the
  pages it built, re-derives the window through `PageWindow::fitting`, and
  serves that. The echoed `window` is the authority on what was served.

**What is implemented, precisely**, so the rest of this document is not read as
a description of working code:

| section | r2 |
|---|---|
| §5.1 `Page.content: PageContent` | **implemented** |
| §5.2 `render_windowed` | **implemented** |
| §5.3 streaming assembly, closure and diagnostics kept exact | **implemented** |
| §5.4 `assembled` scoped to the window, `blocks`/`adapted` whole-document | **implemented** (retain-by-window; the byte budget is not) |
| §5.6 pdf / delta refusals, `required_features` over every block | **implemented** (r1 had the refusals; r2 has the features) |
| §4 wire shape | **implemented and negotiated**, producer side; no consumer sends it |
| §1.0 `PageWindow::fitting` (widest window a reply limit can carry) | **implemented and called**: it is what narrows an over-limit window instead of declining it |
| §5.5 `PageRecipe` retained across requests | **not implemented**: each windowed render re-runs layout and rebuilds the recipe from a fresh `Laid`. Serving a scroll without re-layout is the point of retaining it, and is the next revision |
| §6 scroll served from a retained recipe | **not implemented**, follows §5.5 |

Sibling proposals, both unchanged by this one:
`docs/proposals/display-list-v2-delta.md` (r5) and
`protocol/proposals/display-list-v2-only.md` (r1).

## 0. Ten-line summary

0. **A 500 KB document cannot be previewed at all today.** Its display list is
   164 MB against a 16 MiB reply limit, the v1 fallback is 20.3 MB for the same
   385 pages, and the request ends `status: failed` (PR #273). Neither `-delta`
   (which needs the full list first) nor `-only` (which needs a sibling to
   elide v1 in favour of) fixes that. A window does, because it is the only one
   that makes the producer build less. §1.0.
1. FT-070's 100 MB target is separately not reachable while the whole
   document's display list is resident. 100 MB over the 2 MB corpus case's 1507
   pages is ~68 KB a page — less than one page of glyph data at any
   representation that keeps per-glyph carets and source ranges (PR #232, §5).
   Both roads end at the same mechanism, which is why this is one proposal.
2. So the document stops being resident and a **window** of pages does: the
   pages the viewer is showing, plus a bounded margin.
3. **The window is a production bound inside the pipeline, declared at the
   protocol boundary.** A serialisation filter at the wire would cut wire bytes
   and leave resident memory exactly where it is, because the producer has
   already built all 1507 pages before it decides what to send. §2 argues this.
4. Layout still runs over the whole document, always. Page breaking, `\pageref`,
   floats, contents lists and footnote numbering are global; a window over them
   would change output. Only **assembly** — turning laid-out lines into glyph
   runs, clusters and hit rects — is windowed.
5. The recipe for re-materialising a page already exists in the pipeline:
   `typeset::Laid::pages` holds, per page, per line, `(block index, line index,
   baseline)` and `Laid::line_dx` the x offsets. It is ~40 bytes a line, against
   ~180 KB a page of assembled items. Retaining it and dropping the items is the
   whole mechanism.
6. Re-materialising a page is therefore: for each line, take the block's
   `CachedBlock` (layout records, already cached whole-document), assemble it,
   place it. No re-parse, no re-shape of unrelated text, no re-break.
7. **The completeness contract is the hard part, not the memory.** Fonts, the
   math resource profile and the unmapped-glyph diagnostics are *derived during
   assembly over every block*. A window that assembles only its own pages
   silently emits a smaller font closure and fewer diagnostics. §3 specifies
   that they stay whole, and §5.3 the streaming pass that keeps them exact.
8. A windowed reply is **an incomplete view, not a complete compile** — the
   ruling `display-list-v2-delta` §9 already made. It is typed as such, it
   never authorises a source action outside its coverage, and `Page` gains an
   explicit `Elided` state so no consumer can mistake an unmaterialised page
   for an empty one.
9. `-window` composes with `-only` and is **mutually exclusive with `-delta`
   in r1** (§7): the delta's `list_digest` binds a digest for every page, and a
   windowed producer cannot digest a page it has not materialised.
10. Acceptance is *equality*, not a digest: for every page of every corpus case,
    the page materialised through a window must be byte-identical to that page
    in the unwindowed render. §9.

## 1. Why — and it is not only memory

### 1.0 A 500 KB document has no reply at all today

The lane that profiled the warm keystroke (PR #273, issue #2 comment
5658075424) measured the 500 KB corpus case, 385 pages:

- the `display_list` sibling would be **164 MB** against a **16 MiB** line
  limit, so `handle_line` declines it *without serialising it*;
- the runtime-v1 `compile_result` that would carry the pages instead is
  **20 339 909 bytes** for those 385 pages — also over the limit;
- so the request ends **`status: failed`**. Not slow. Failed.

This is a product bug, and it is arguably more urgent than the 100 MB target
this proposal was written for. It also cannot be fixed by either existing
sibling capability:

- **`-delta` does not help.** `try_delta` needs the full `DisplayList` from
  `render_cached` before it can diff anything, so it shrinks the wire *after*
  the producer has already built and failed to fit the whole document.
- **`-only` alone does not help.** It empties the v1 `pages` only when a sibling
  line is present, and at 385 pages the sibling is the 164 MB line that was
  declined. Emptying v1 leaves nothing to paint.

A window is the shape the limit requires, because it is the only one of the
three that makes the producer build less. At the measured ~426 KB a page a
16 MiB line carries ~39 pages of that document and the runtime's 8 MiB framed
default ~19 — both comfortably more than a viewer shows, and both finite where
1507 pages is not. `MAX_WINDOW_PAGES` is set from this arithmetic (64, with
margin), not from a memory budget.

**And `-window` and `-only` together are what make a long document repliable at
all**, which is the first hard reason to compose two capabilities rather than
pick one: `-only` removes the ~53 KB-a-page v1 payload that fails on its own at
385 pages, and the window bounds the v2 sibling that fails on its own at 164 MB.
Neither is sufficient. §7.

**Measured through the worker, r2** (`flashtex-render` over JSON Lines, the
`synthetic-500kb` corpus case, the real 16 MiB limit, zero font-failure
diagnostics in every run):

| request | reply |
|---|---|
| `["display-list-v2"]` — today's consumer | **`status: failed`**, one line, no pages: *"compile_result would be 20 339 674 bytes for 385 pages, over the 16 777 216-byte reply limit"* |
| `+ -only + -window`, `{1, 16}` | `status: recovered`, two lines, sibling **5 937 794 B**, 385 pages / 16 resident / 369 elided |
| `+ -only + -window`, `{300, 16}` | as above, sibling **6 369 270 B**, window at page 300 |
| `+ -only + -window`, `{1, 64}` | narrowed to `{13, 40}` and served, **15 211 146 B** (§8) |
| `+ -only + -window`, no `display_list_window` | `status: failed` — the capability without a position is an unwindowed reply (§4) |

The unwindowed render of the same document, written to a file rather than a
reply line because no reply can carry it, is **152 106 263 bytes**.

The rest of this section is the memory argument the proposal started from. Both
lead to the same mechanism, which is why it is one proposal.

### 1.1 The memory arithmetic (measured, not promised)

PR #232 measured the 2 MB corpus case (`synthetic-2mb`, 1507 pages, 1.48 M
glyphs) per structure after its own win. Resident 2570 MiB, live 1458 MiB:

| bucket | MiB | what it is |
|---|---:|---|
| allocator retention + fragmentation | 1111 | ~1.48 M small allocations in ~300 k vectors |
| `RenderCache.assembled` | ~390 | glyph-level assembly, one entry per block |
| display list (`DisplayList.pages`) | 263 | the same glyphs again, placed |
| `RenderCache.blocks` | ~294 | layout records (`BuiltBlock.items`, `ClusterRec`, `BoxRec`) |
| `RenderCache.adapted` | 52 | adapter items |
| font / expander / compiler caches | ~495 | not reachable from the display-list walk |

Two of those lines — `assembled` and the display list, ~653 MiB — are
*per-page glyph data for pages nobody is looking at*, held because the API
returns one `DisplayList` containing every page. A third, the fragmentation,
is largely the allocation pattern those two create.

#232's arithmetic on the target stands and is the reason this proposal exists:
at a perfect 40-byte glyph and 64-byte cluster with zero duplication the
display list alone is ~154 MB at 1507 pages, over target before the cache, the
box tree or the page item slots. Representation cannot close it. Residency can.

What a window does **not** touch is equally important: `RenderCache.blocks`
(~294 MiB) and the unreached caches (~495 MiB) are whole-document by
construction here. §10 states the honest landing.

## 2. Where the window belongs

The task this proposal answers was posed as: the protocol already has
`display-list-v2-delta` and `display-list-v2-only`; does the window belong at
that boundary rather than inside the renderer? The answer is **both, in
different senses, and neither alone**.

**Not the wire alone.** `-only` and `-delta` are both *serialisation* levers:
they choose what of an already-built `DisplayList` reaches the consumer.
`delta::snapshot` (`crates/render-pipeline/src/delta.rs:580`) even clones the
full `pages` vector to keep as a base. A `-window` built the same way would
filter `assemble`'s output after `assemble` had allocated all 1507 pages and
inserted every block into `RenderCache.assembled`. Peak RSS, which is what
FT-070 measures, would be unchanged. The wire is downstream of the problem.

**Not the renderer alone.** If the pipeline quietly returns a `DisplayList`
whose out-of-window pages have empty `items`, then `pdf::export`,
`v1::fallback`, `DisplayList::required_features`, `delta::page_digest` and
`DisplayList::to_json` all keep compiling and all produce wrong output — a PDF
missing 1491 pages, a feature list that omits `rule` because no windowed page
had one. Every one of those reads `.items` with no way to ask whether the page
was built. Residency is a fact about the reply, and a fact about the reply
belongs in the contract.

**So:** the window *takes effect* in the pipeline, at `typeset::assemble`
(`crates/render-pipeline/src/typeset.rs:6594`), which is the first point where
per-page glyph data exists. It is *declared* in the request and *echoed and
described* in the reply, because whether a reply covers the whole document is
something the consumer must be told rather than infer. And it is made
unmissable in the Rust API by giving `Page` an explicit residency state, so
that every one of the consumers listed above fails to compile until it has
said what it does with an elided page.

## 3. What stays whole

A windowed reply is smaller in exactly one respect: the glyph-level content of
pages outside the window. Everything else is the complete document's, byte for
byte identical to the unwindowed reply.

| part | windowed? | why |
|---|---|---|
| page count | **no** | the consumer's scroll extent; and knowing page 900 exists requires laying out 1..899 anyway |
| every page's `number`, `width`, `height` | **no** | the page frame is decided by layout, costs ~16 bytes a page, and the consumer needs it to place the scroll view |
| `documents` | **no** | request-wide |
| `fonts` (the resource closure) | **no** | per face, not per glyph; and a partial closure would fail the consumer's font validation for pages it later asks for. §5.3 |
| `diagnostics` | **no** | a diagnostic on page 900 must be reported when the window is 1–10. Silently dropping it is the failure mode this section exists to forbid |
| `required_features` | **no** | derived over all pages, or a consumer negotiates a feature set it cannot paint the rest of the document with |
| page **items** outside the window | **yes** | this is the whole mechanism |

Layout runs over the whole document on every render. This is not a concession
to be optimised away later: page breaking, `\pageref` resolution (which needs a
second pass over page numbers, `lib.rs:250`), float placement, contents lists
and footnote numbering are all global, and a window over them would change
where page breaks fall. The window is strictly *downstream of layout*.

## 4. Wire shape (additive; negotiated, opt-in, unused by any consumer)

- **Request.** `payload.layout_capabilities` gains `"display-list-v2-window"`.
  It is meaningful only next to `"display-list-v2"`. When present, the request
  carries one additive optional field:

  ```json
  "display_list_window": { "first_page": 41, "page_count": 16 }
  ```

  `first_page` is 1-based; a window running past the last page is clamped, not
  refused. Absent field with the capability listed = the producer chooses
  nothing and answers unwindowed (so a consumer can advertise support before it
  knows where the viewer is).

- **Reply.** When and only when accepted, the `display_list` sibling gains one
  object and the echoed capabilities include `"display-list-v2-window"`:

  ```json
  "window": { "first_page": 41, "page_count": 16, "document_page_count": 1507 }
  ```

  `pages[]` still has `document_page_count` entries in order. Each entry
  outside the window carries `number`, `width`, `height` and
  `"resident": false`, and **no `items` key at all** — absent, not `[]`, so a
  consumer that never read the flag gets a decode error rather than a blank
  page.

  Entries inside the window are today's page objects **with no marker added**.
  A `"resident": true` on every page would change every page's bytes and so
  every `dl2-canon-1` page digest and every committed fixture digest, for no
  information: the `window` object already names the range, and a page object
  without `resident` is resident. The asymmetry is deliberate and is what keeps
  an unwindowed line — every line on the wire today — byte-for-byte unchanged.

- **Declined.** The producer answers unwindowed and omits the name from the
  echo, exactly as `display-list-v2-only` does. The echo is the consumer's only
  signal; it never infers from the shape of `pages[]`.

- **Old producers and old consumers** are unaffected: unknown capability names
  are not accepted (`v1.rs:55`) and unknown payload keys are read with
  `payload.get` (`protocol.rs:117`). A consumer that does not name the
  capability cannot be given a windowed reply, and the name being accepted
  changes nothing for it — the same relationship `-only` and `-delta` already
  have with the installed base.

### 4.1 What a windowed reply does not authorise

Per `display-list-v2-delta` §9, a filtered view is an incomplete view. A
windowed reply:

- **is not a complete compile** and must never be digested as one, cached as a
  delta base, or used as the source of a PDF export;
- **never authorises a source action outside its coverage.** Caret sync, click
  navigation and select-to-source are valid only for resident pages. A consumer
  that wants them for page 900 requests a window containing page 900;
- **does not change `status`.** A windowed compile that succeeded is `ok`. The
  window is not a diagnostic and emits none.

## 5. Producer API change (Rust)

This is the part that needs co-signing, because it changes public types in
`flashtex-render-pipeline` that four other crates read.

### 5.1 `Page` gains a residency state

```rust
pub enum PageContent {
    Resident(Vec<Item>),
    /// Laid out and counted, glyph content not materialised.
    Elided,
}

pub struct Page {
    pub number: u32,
    pub width: Tick,
    pub height: Tick,
    pub content: PageContent,
}
```

`pub items: Vec<Item>` becomes `pub content: PageContent`. This is deliberately
a breaking change rather than an added `resident: bool` beside a retained
`items`, because the failure mode being designed against is precisely a
consumer that reads `items` without checking. There is no silent path.

`Page::items()` returns `Option<&[Item]>` for the common read.

### 5.2 The window is a render argument, not an option

```rust
pub struct PageWindow { pub first_page: u32, pub page_count: u32 }

pub fn render_windowed(
    documents: &[SourceDocument<'_>], entry_path: &str, revision: u64,
    project_id: &str, fonts: &FontSet, options: &RenderOptions,
    cache: Option<&RenderCache>, window: Option<PageWindow>,
) -> Rendered;
```

`render` and `render_cached` keep their signatures and call this with `None`,
which is today's behaviour exactly. The window is not in `RenderOptions`
because `RenderOptions` is cache-keyed and a window must not partition the
block cache — two requests differing only in window must share every layout
record.

`Rendered` gains `window: Option<PageWindow>` (the clamped, effective one).

### 5.3 The streaming assembly pass

`typeset::assemble` today builds `assembled: Vec<Option<Rc<AssembledBlock>>>`
over every block (`typeset.rs:6617`), then walks pages. Three document-wide
results are harvested from that vector after the page loop:

- `used` — the font closure (a `BTreeMap` keyed by `font_id`, so
  order-independent);
- `profiles` — `math_resource_profile` diagnostics, first occurrence in **block
  order**;
- `unmapped` — `math_glyph_unmapped` diagnostics, first occurrence in **block
  order**.

A window that assembles only the blocks its pages reference would change all
three. The pass is therefore restructured to **visit every block in the same
order and retain selectively**:

1. Build a block → `[(page index, line index)]` index from `Laid::pages`. It is
   built from data that already exists and costs one `usize` pair per placed
   line.
2. For each block in order: obtain its `AssembledBlock` (cache hit, or
   `assemble_block`); harvest `faces`, `resources`, `unmapped` — unchanged
   order, unchanged first-occurrence semantics; place its lines into whichever
   windowed pages reference them; then drop the `Rc` unless a windowed page
   needs it or the (windowed) assembled cache keeps it.
3. Pages outside the window get `PageContent::Elided`.

Because step 2 visits every block in the original order and harvests before it
drops, the closure and both diagnostic lists are **bit-identical to the
unwindowed pass by construction**, not by test. The test in §9 is there to
prove the construction, not to substitute for it.

Cost: the whole document is still assembled once per render. The window buys
**residency, not work**. Reducing the work is §10's follow-up, and needs the
block cache to survive a keystroke, which it already does.

### 5.4 The render cache splits by lifetime

`RenderCache` (`incremental.rs:73`) holds three maps, each bounded by
`MAX_BLOCKS = 50_000` entries and cleared wholesale. They are not the same kind
of thing and the window separates them:

| map | holds | lifetime under a window |
|---|---|---|
| `adapted` | adapter items | **whole document** — what makes a re-layout cheap |
| `blocks` | `CachedBlock`: layout records, rebased | **whole document** — what makes re-materialising a page cheap |
| `assembled` | `AssembledBlock`: glyph runs, clusters | **window-scoped** — bounded by resident pages, evicted outside |

`blocks` being whole-document is what makes the promise in §0.6 true:
`CachedBlock` carries its own `recs` and `maths`, so `assemble_block` can run
from it without `Laid`. Re-materialising page 900 after a scroll needs the
recipe and the cached blocks its lines name — no compiler, no paragraph
breaking.

In r1 a windowed render ends by retaining in `assembled` exactly the blocks its
resident pages used (`RenderCache::retain_assembled`). Without that the cache is
append-only and a viewer scrolling a long document accumulates every page it
passed, so the window would buy nothing beyond the first render. A byte budget
and eviction by distance from the window — which would keep a margin of recently
left pages rather than dropping them at once — are the obvious refinement and are
not in r1. (#232's follow-up 3 asks for a byte budget on `blocks` too; that is a
separate change and not proposed here.)

An unwindowed render never calls `retain_assembled` and treats every block as
wanted, so a complete compile's cache behaviour is exactly what it was.

### 5.5 The recipe is retained, not invented

```rust
pub struct PageRecipe {
    pub number: u32, pub width: Tick, pub height: Tick,
    pub lines: Vec<PlacedLine>,   // (block cache key, line index, baseline, dx)
}
```

This is a projection of `Laid::pages` + `Laid::line_dx`, both of which the
pipeline already computes and then drops when `assemble` consumes `Laid` by
value. Retaining the projection alongside the cache is what makes a later
window servable without re-layout; at ~40 bytes a placed line it is ~1.5 MB for
the 2 MB case against the ~650 MiB it replaces.

**Not in r1.** r1 re-runs layout for every window, which is correct but pays
whole-document layout on a scroll. The memory this proposal measures does not
depend on the recipe being retained — the retention that matters is the
`assembled` and `Page.items` bytes — so the recipe is separated out as r2
rather than bundled in unmeasured.

### 5.6 Consumers that must be updated (all in-tree)

`pdf.rs:108`, `v1.rs:192`, `display.rs:532/559/562/575/584/592/656/720`,
`delta.rs:470/573/623/635`, `memsize.rs:203/361`. Each must state its rule:

- **`pdf::export` refuses an elided page.** A PDF is a complete document; it
  never silently omits or blanks a page. Export requests an unwindowed render.
- **`v1::fallback` skips an elided page** rather than sending it as an empty
  one: there is no v1 shape for "this page was not built", and a page-shaped
  object claiming the page is empty is the exact confusion `PageContent` exists
  to prevent. The pages it does send carry their real `number`, so the subset
  is unambiguous, and a windowed list reaches it only because the consumer
  asked for a window and was told so in the echo. A consumer that needs the v1
  payload to stand for the whole document does not negotiate this capability —
  and at scale it composes with `-only`, which empties those pages anyway (§7).
  `status` is derived from the whole-document harvest, not from the pages that
  happen to be resident (§4.1).
- **`DisplayList::to_json`** emits the §4 shape.
- **`delta`** refuses a windowed list as a snapshot base (§7).
- **`required_features`** is computed during the streaming pass, over every
  block, not over `pages` (it is the one place today's code reads all items
  purely to classify them). `assemble_windowed` harvests a `DocumentFeatures`
  beside the faces and the diagnostics, adding the items no block owns — float
  images, the `\pagecolor` rule, the document default colour written into every
  paint after placement. An **unwindowed** list still scans its own pages, so
  every line on the wire today is byte-for-byte unchanged by the harvest
  existing; only a windowed list reads it.

## 6. The edit path

An edit invalidates by source range, as it does today; nothing in that changes.
What changes is what is rebuilt after it:

1. Changed documents re-parse; `adapted` entries whose blocks moved are
   rebased, the changed ones rebuilt (today's behaviour).
2. `blocks` entries are reused by key for unchanged blocks (today's behaviour).
   This is the expensive cache and the window does not touch it.
3. Layout re-runs whole-document and produces a fresh `Laid` — fresh page
   breaks, fresh recipes. Cheap relative to assembly: it is line boxes and
   heights, no glyphs.
4. Only the window is assembled.

So a keystroke's assembly cost goes from 1507 pages to the window, and the
number the editor feels is bounded by the window rather than the document. That
is a claim about work, and this proposal deliberately does not quantify it: the
measuring box is shared and under load (§10's note), and #232 made the same
call. Memory is reported; timings are not.

**Scroll** is the new path: the viewer moves to page 900, the consumer requests
a window there, and the producer serves it from the recipe and `blocks` without
re-parsing or re-breaking. Whether the recipe is still valid is decided by the
same revision check the delta uses — a recipe belongs to one revision, and a
scroll at a stale revision is a fresh render, not a patched one.

## 7. Composition

**With `display-list-v2-only`: composes, and at scale each is useless without
the other.** `-only` elides the runtime-v1 `pages` when a v2 sibling is present;
`-window` bounds that sibling. Neither reads the other's field, and a request
may list both — but §1.0's measurement is the reason it *should*: at 385 pages
the v1 payload alone is 20.3 MB and the v2 sibling alone is 164 MB, so a reply
that fits needs both levers. A consumer that wants large documents to work sends
`display-list-v2`, `display-list-v2-only` and `display-list-v2-window` together.

**With `display-list-v2-delta`: mutually exclusive in r1.** The delta's
identity (`-delta` §2) binds `page_count`, a digest for **every** page, and a
`list_digest` over all of them. A windowed producer does not have the digest of
a page it has not materialised, and materialising every page to digest it is
the thing this proposal exists to stop. A request listing both is answered with
the **window**, and the echo says so; the consumer treats `-delta`'s absence
from the echo as a decline and does not send `display_list_base`.

Composing them is a real and probably desirable r2 — a delta over the window's
pages, against a base that is itself windowed — but it needs a digest identity
that is honest about partial coverage, and inventing one here would be
speculation. Named as deferred, not as solved.

## 8. Refusals

Typed, and all on the producer side in r1 (there is no consumer yet):

| condition | producer answer |
|---|---|
| `-window` listed without `display-list-v2` | not accepted; unwindowed reply, name absent from echo |
| `first_page` 0, or `page_count` 0 | not accepted; unwindowed reply |
| `first_page` past the last page | clamped to the last `page_count` pages |
| `page_count` above `MAX_WINDOW_PAGES` | clamped, and the echoed `window` states the clamp |
| the window still does not fit the reply limit | **narrowed** around its centre to what the measured page size says fits, and served; the echoed `window` states what was served |
| request also lists `-delta` | window wins; `-delta` absent from echo (§7) |
| a windowed list reaching `delta::snapshot` | refused — no snapshot taken, chain cleared, exactly as today's no-snapshot path (`-delta` §7: the full unchanged line, `status` stays `ok`, no diagnostic) |

A clamp or a narrowing is never a diagnostic and never changes `status`. The
echoed `window` is the authority on what was served — the consumer never infers
the coverage from the shape of `pages[]`.

The narrowing deserves its own line, because it is what makes the capability
worth negotiating rather than merely specifying. `MAX_WINDOW_PAGES` is a bound
on what a viewer shows; the reply limit is a bound on bytes, and at ~426 KB a
page the two disagree by a factor of four. A producer that declined the
difference would hand a consumer the same `status: failed` for asking for too
many pages as it does today for asking for the document — which is §1.0's bug
wearing a different hat. So the producer re-derives the window from what it
measured and serves it; one extra assembly pass over a *narrower* window,
against a cache that is already warm, in exchange for a reply where there was
none.

## 9. Acceptance — and what producers must co-sign

**The gate is equality, not a digest.** For every corpus case, and for every
page `p` of it:

> the page object produced for `p` by a render windowed on `p`, serialised,
> must be byte-identical to the page object produced for `p` by the unwindowed
> render of the same revision.

and, over the whole reply:

> `documents`, `fonts`, `diagnostics` and `required_features` of a windowed
> reply must be byte-identical to the unwindowed reply's.

Equality rather than a digest, because a digest over a windowed list would
prove only that the consumer rebuilt what the producer sent — the distinction
`-delta` §5 already draws, and here the risk is precisely that the producer
sends a *correct-looking smaller* closure.

Then the standing gates, unchanged: amsmath 59/59 against the pdflatex oracle;
HW1 and HW2 3 pages, 0 errors, 0 overfull, 0 font diagnostics; every
`crates/perf-bench` fixture digest identical to a same-base control run (not to
the committed baseline, which predates current main and differs independently
on 50 of 90 digests — reported on #206); `cargo test -p
flashtex-render-pipeline` no worse than the control's failing set.

At r2 that digest gate is **190 digests over 26 cases, 0 differing**, against a
control built and run in the same directory from the same merge of main with
this lane's changes reverted, `unmeasured: []` on both runs.

**r2 adds two gates to `tests/`, both of which fail without their fix:**

* `cli_e2e::a_document_over_the_reply_limit_has_a_reply_when_a_window_is_negotiated`
  is §1.0 itself, end to end through the worker: the same three refusals, put
  into the same ratio through `FLASHTEX_MAX_REPLY_BYTES` so it costs seconds
  rather than minutes of debug-build layout. Nothing in it is hard-coded to a
  page size — the limit comes from a measured window and the whole-document
  size is read back out of the producer's own refusal — so it keeps testing the
  relation it is about when page sizes drift.
* `page_window::a_window_announces_a_feature_that_only_an_elided_page_uses`
  is the closure gate for `required_features`: a document whose only rule sits
  on a late page, windowed on page 1, must still announce `rule`, and the test
  asserts the resident page carries no rule of its own so the feature can only
  have come from the whole-document harvest. With the harvest reverted it
  reports `["glyph_run", "rgba-srgb", "cluster-actualtext"]` against
  `["glyph_run", "rule", "rgba-srgb", "cluster-actualtext"]`.

The §9 equality gate was also run against the **real 500 KB case** rather than
a 20-page stand-in: for windows at pages 1, 185, 300 and 378, every resident
page object is byte-identical to that page in the unwindowed 152 MB render, all
385 page frames match, and `documents`, `fonts`, `diagnostics` and
`required_features` are byte-identical.

### Co-signers

| who | what they are co-signing | r2 |
|---|---|---|
| **`crates/render-pipeline`** (producer, this lane) | §5's type change, the §9 equality gate in `tests/`, and the negotiation | **signed** — implemented and gated |
| **Mac shell / `apps/mac`** (consumer) | that the v2 pane can hold a partial frame: elided pages render as placeholders at their known size, caret sync and click-to-source are disabled on them rather than wrong, and leaving the window re-requests. Also that `File > Export PDF` refuses an elided result, as it already does for `-only`. | **required before the shell sends the name** |
| **`crates/rendering-core`** (renderer) | that `V2Frame.prepare` and the font binding accept a frame whose font closure covers pages it was not given, and that an elided page is a painted placeholder, never a blank page of the right size (which is indistinguishable from a genuinely empty page). | **required before the shell sends the name** |
| **`crates/preview-controller`** / **`crates/document-runtime`** (forwarding) | that the new sibling field passes through unaltered and that the window is not re-derived anywhere in the middle of the route. | **required before the shell sends the name** |
| **Commander** | that `-window` and `-delta` are exclusive in r1/r2 (§7), and the `docs/contracts/runtime-v1*` amendment that follows once a consumer exists. | **required before the contract is amended** |

**What the producer being negotiated does and does not commit anyone to.** A
reply is windowed only when a request names the capability *and* carries
`display_list_window`; no in-tree consumer does either (`apps/mac` sends
`display-list-v2` and `display-list-v2-images`, `preview-controller` adds and
removes only `display-list-v2`). So accepting the name changes no reply on any
route today, exactly as accepting `-only` and `-delta` — both still proposals —
changed none. What it does change is that the fix is *reachable*: a consumer
that signs its row can turn it on for itself, one request at a time, without a
producer release.

The consumer rows above are therefore gating the **consumer**, not this
producer. Concretely: `apps/mac` must not add the name to `setLiveV2`'s
capability list until the pane paints an elided page as a placeholder, because
today it would paint nothing there and a blank page is indistinguishable from a
page that is genuinely empty — the confusion `PageContent::Elided` exists to
prevent on the producer side and which the consumer must not reintroduce.

## 10. What this buys, and what it does not

Windowing removes `RenderCache.assembled` (~390 MiB) and the display list
(~263 MiB) for pages outside the window, and should take a large share of the
678 MiB of real fragmentation with them, since those two structures are where
the ~1.48 M small allocations live.

It does **not** remove `RenderCache.blocks` (~294 MiB), `adapted` (~52 MiB) or
the font/expander/compiler caches (~495 MiB). Those are whole-document by
construction in this design — `blocks` deliberately so, since it is what makes
re-materialisation cheap.

So the honest projection is a landing well under half of today's 2570 MiB at
2 MB, and **not 100 MB**. Reaching 100 MB additionally needs the block cache
bounded by bytes with re-layout on miss (#232 follow-up 3), the adapter items
shared rather than copied (#232 follow-up 1), and the compiler's own parse tree
and the expander's thread-local caches bounded — none of which are display-list
structures and none of which this proposal touches.

### Measured

A 16-page window, `crates/perf-bench`'s `memprofile`, against an unwindowed
control taking the same code path (`--render-only`, so the comparison does not
credit the window with the v1 payload and JSON line that the protocol warm path
also builds). The table below is r1's. Re-running the 2 MB pair at r2, after
the rebase onto current main and the negotiation, gives **2919.5 → 2037.0 MiB**
VmRSS and **1442.9 → 922.2 MiB** live against r1's 2912.3 → 2035.8 and
1440.7 → 920.0 — 0.2% apart, with the display-list and render-cache rows
(247.5 → 2.5 and 693.6 → 416.1 MiB) identical to the digit. Reproducing the
control is the point of re-running it:

| | 500 KB / 385 pp | | 2 MB / 1507 pp | |
|---|---:|---:|---:|---:|
| | unwindowed | window 16 | unwindowed | window 16 |
| VmRSS | 748.8 | **541.9** | 2912.3 | **2035.8** |
| VmRSS after `malloc_trim` | 562.8 | **408.0** | 2218.5 | **1576.4** |
| live | 363.2 | **238.1** | 1440.7 | **920.0** |
| walked | 234.7 | **109.2** | 941.1 | **418.6** |
| — display list | 61.7 | **2.5** | 247.5 | **2.5** |
| — render cache | 173.1 | **106.8** | 693.6 | **416.1** |

−36% of live bytes and −30% of RSS at 2 MB; −34% / −28% at 500 KB. The display
list falls to the window's own size and stops scaling with the document, which
is the property the design is for. What remains of the render cache is
`RenderCache.blocks` — 160.2 MiB of `BuiltBlock.items` and 73.1 MiB of
`ClusterRec` at 2 MB — exactly the whole-document retention §5.4 keeps on
purpose, and exactly what #232's follow-up 3 proposes to bound next.

This confirms §10's projection, including the part that says **the target is
still not reached**: 2036 MiB is 20x the 100 MB target, and closing that needs
the block cache, the adapter items and the compiler/expander caches, none of
which a page window touches.

r2 can also measure the **protocol** path windowed, because the capability is
negotiated — the product path, v1 payload and JSON line included:

| case, protocol path | unwindowed | window 16 |
|---|---:|---:|
| 2 MB / 1507 pp, VmRSS | 2889.0 | **2048.5** |
| 2 MB / 1507 pp, live | 1440.8 | **920.1** |
| 500 KB / 385 pp, VmRSS | 751.3 | **540.3** |
| 500 KB / 385 pp, live | 363.3 | **238.2** |

A note on that row existing at all: `memprofile`'s own request builder emitted
a JSON-RPC 2.0 envelope, which `protocol::handle_line` answers
`missing_protocol_version`, so its default-mode warm keystrokes had been
rendering nothing. The window's numbers were never affected — they were
measured `--render-only` on both sides, which builds no request — and
`crates/perf-bench` proper builds a correct request, so the digests and timing
gates were never affected either. Fixed in r2, and recorded here rather than
quietly, because a harness that measures an empty cache is the kind of thing
this document's §7 note exists to warn about.
