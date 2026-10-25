# COLD-OPEN probe: does pass N+1 equal pass N modulo the L5 patch? (2026-10-24)

Lane COLD-OPEN (kabir-claude-coldopen-probe under kabir-claude-coldopen). A **measurement probe
on a scratch branch** (`agent/kabir-claude-coldopen/probe-modpatch`, never merged). It answers
one question for two ideas that would shorten a cold open's passes 2 and 3 (DESIGN.md §5.3,
§5.5):

- (A) *convergence modulo the L5 patch*: keep the rest of pass N once pass N+1's state equals
  pass N's with the changed `.aux` meanings patched in;
- (B) *exact parallel stretches*: run pass N+1 in stretches that start from pass N's checkpoints
  with the patch applied, and verify each join by state equality.

Both need to know at which page checkpoints `state(pass N+1, k) == patch(state(pass N, k))`.

**Answer: at almost none.** On the app's path the answer is 0 of 592 pages (*Infinite Descent*)
and 0 of 1,152 (*×2*) for the strict test. The relaxed upper bound gives 1 page, page 3.
Prior work: `docs/evidence/pass-reuse-2026-10-09/` (no checkpoint of pass 3 equals pass 2's;
`\@abspage@last` is read at every shipout). The current pass structure is in
`docs/evidence/coldopen-x2-2026-10-24/`.

## Setup

- NixOS PC, `flashtex-agents.slice` (`ftx-run`), TeX Live 2025 from nix, `CARGO_BUILD_JOBS=6`.
  Only this lane's runs were going. Wall times include the probe and are **not references**.
- Base: `agent/kabir-claude-coldopen/far-segments` `0f4a7784e`, plus the probe commit
  `414a2ffaf`.
- Path: `tools/incr-bench/coldopen.py` with `--external-tools auto --profile balanced`, as
  the app opens a document. That gives three passes, `cold`, `tools`, `tools`, with makeindex
  run between them. `FLASHTEX_BUDGET=6–7 GB`, so that retention kept every page checkpoint
  (`old_kept` = `old_pages`).

## What the probe compares

With `FLASHTEX_MODPATCH_PROBE=N` set, every incremental pass keeps the old run's future, also
pass 2 (normally `no_tests` discards pass 1's). It runs no convergence tests and no
branch-window thinning. At every N-th page checkpoint (N = 1 here; pass 2 used every 10th
page in the final runs, and every page in run v1) it compares the live state with the old
run's page checkpoint at the same input position (`input_key`: every file global's stream,
offset and lookahead). One JSON line per page goes to stderr. With the variable unset, the
probe code does not run. The scratch branch also bounds-checks `readset::View::meaning`/`name`,
which turns a panic into an error.

- **Strict.** The comparison is modulo the run's L5 patch (`Session::l5_restart`'s
  `patch` + `defpatch`). A run without one (pass 2) takes the meanings that differ at its first
  aligned page instead. It uses:
  - `readset::aux_delta(…, before = patch)`: every name whose live meaning differs from
    `patch(old)`;
  - the convergence test's word filters (`dead_word`, `position_only`, the `pdf_char_used`
    superset, free cells), with the patched names' `eqtb` and `hash` words left out;
  - `iso::Iso::check` with the patched names' `eqtb` slots skipped and their token lists
    covered in each state alone (`iso::with_skip`);
  - `CState::same_as`, part by part (`CState::probe_parts`).
- **Relaxed (an upper bound).** As strict, plus three things:
  - the patch is extended by every other meaning that differs at the first aligned page
    (names the `\document` hooks derive from the `.aux`);
  - the image table is compared by its encoding (`img~`);
  - string numbers are compared through a renumbering, accepted only if the live strings are
    the old ones with the patch's new names' strings inserted (`str_map`), and then applied to
    the string-valued words and the input levels' file names.

  This relaxation is not sound: hash chains are compared leniently, and any word whose halves
  map is accepted. It can only raise the count of matches.

What the probe cannot see:
- The structural walk stops at the **first** difference, so per page only the first one is
  known.
- `aux_delta` lists names at differing slots, as L5 does.
- PDF **object numbers** are not relocated. The test relocates byte offsets only. See blocker 1.

## Results (VERIFIED, raw lines in `raw/`)

| document, pair | pages tested | aligned by input | equal, strict | equal, relaxed | longest run |
|---|---:|---:|---:|---:|---|
| infdesc, pass 2 vs 1 (v1, every page) | 592 | 430 (offsets −2…−16) | 0 | — | 0 |
| infdesc, pass 2 vs 1 (v2, every 10th) | 60 | 40 | 0 | 0 | 0 |
| infdesc, pass 3 vs 2 | 592 | 589 (offset 0) | **0** | **1** (page 3) | 1 |
| infdesc ×2, pass 2 vs 1 (every 10th) | 116 | 81 | 0 | 0 | 0 |
| infdesc ×2, pass 3 vs 2 | 1,152 | 1,140 (offset 0) | **0** | **1** (page 3) | 1 |

The L5 patch of pass 3 has 1,104 names (infdesc) and 2,880 (×2).

**Pass 3 vs pass 2, first blocker per page.** Counts are infdesc / ×2. Summaries are in
`raw/mpstat-*.txt`.

1. **The `.toc` pass 3 reads holds 3 more entries (583 / 1,134 pages).**
   - Before pass 1 there is no `.ind`, so pass 1's `.toc` has no index chapters. makeindex runs
     after pass 1, pass 2 typesets the index and writes its entries (`appendix*.16`–`.18` in
     the oracle's `.toc`, `raw/oracle-infdesc.toc*`), and pass 3's table of contents links to
     them.
   - From the page after the table of contents (page 10), pass 3 therefore has **+3 PDF
     objects, +3 strings and 3 more named destinations** (the first new string is
     `appendix*.17`; `obj_ptr` 0xd3 → 0xd6 … 0x1ba8 → 0x1bab, a constant +3; `dest_names`,
     `obj_tab`).
   - It also has another pdfTeX name tree (`CState.avl`, the first C-part difference on every
     one of these pages). Object numbers then differ inside whatsits and nodes (`whatsit … word
     differs` 80 / 142, `obj_ptr differs`). The string numbers differ in every input level's
     file name (`input name differs`, 449 / 895).
   - The offsets disappear only where pass 2 creates the same destinations, at the index
     (pages 581–585). The strings there still differ by 1–2.
   - The `.toc` is read as typeset input, so no `.aux` meaning patch describes it. (Why the
     `.toc` gains entries is BELIEF from the string, the object counts and the oracle's
     `.toc`. The host's own `.toc` files were not kept.)
2. **`\PreviousTotalPages` (every aligned page, 589 / 1,140).** It sits outside the L5 patch:
   the `\document` hook derives it from `\@abspage@last` (latex.ltx TL 2025 l. 20278). The
   extended patch removes it.
3. **The image table (25 / 575 pages, `img`, unequal even by encoding)**, and the per-image
   control sequence (`\book/media/donkey.png image`) on the pages after the image is used.
4. **The display list's side table (5 pages: 1, 2, 4, 5, 6).** A node's source position is
   `0x3153000f` in pass 2 and `0` in pass 3. These are the only differences on those pages
   under the relaxed test. Page 3 matched.
5. **No old page at the same input position (3 / 12 pages)**: the table-of-contents pages,
   which read the changed `.toc`.
6. **Real typesetting differences (`char node differs`, 45 / 86 pages)**: moved
   `\pageref`/`\cpageref` targets, and table-of-contents numbers.

**Pass 2 vs pass 1: hopeless (VERIFIED).**
- Pagination differs: the table of contents adds pages, so the aligned offset is −2 to −16, and
  162 of 592 pages have no old page at their input position.
- On every aligned page the C-part state (virtual fonts `vf` and name trees `avl`) and the
  font tables differ.
- The `.aux` read creates 2,896 / 2,912 new control sequences and their strings. The live
  strings are not a pure insertion of these: `__file_seen_*` strings for the files that now
  exist come first.
- SyncTeX tags shift by the extra file opened.

**Test cost (VERIFIED, PC, both comparisons per page).**
- Pass 3 vs 2:
  - infdesc: median 179 ms and 2.1 G instructions per page;
  - ×2: median 707 ms, p90 1.05 s and 8.5 G instructions.
  - The strict comparison alone (v1, infdesc) is a median of 125 ms and 1.4 G instructions.
  - Both runs have written most chunks since the restart near page 0, so each diff is
    large. A page of ×2 costs pdflatex about 23 ms.
- Pass 2 vs 1: about 1 s and 8–10 G instructions per page (infdesc), 1.9 s (×2).
- Peak RSS with both passes' futures kept: 905 MB (infdesc) and 1,811 MB (×2).

## The old future's reads of patched names (VERIFIED: the probe logs first reads per page)

- On every aligned page, pass 2's next page interval reads at least one name of the (extended)
  patch: 589 / 589 (infdesc) and 1,140 / 1,140 (×2).
- `\@abspage@last` is logged in 570 / 843 intervals. It is read at every `\shipout`
  (latex.ltx l. 19971). Where it is missing, the read falls before the page checkpoint.
- Label macros `\r@*` are read 623 / 4,521 times, from `\ref`/`\cref`/`\pageref`. The `\r@*`
  of moved labels changed in their page field.

## Verdict for A (soundness)

**Not sound, and with no convergence point to use on this path.**
- Even if some state matched, keeping pass N's later pages would be unsound without
  value-level read tracking: every page interval of the old future reads a changed name.
- Most of these reads are benign at the value level. `\int_compare` against
  `\@abspage@last` is false except at the last page, and `\ref` uses the label's unchanged
  number. Some are not: `\pageref`/`\cpageref` print the moved page, which is the "char node
  differs" on 45 / 86 pages, and the last-page hook fires at another page.
- Telling the two apart needs the result of each read (the conditional's outcome, the
  typeset text), which amounts to re-running the read. The only sound form is the one
  `docs/evidence/pass-reuse-2026-10-09/` §2 (b) proposed: re-run live from the checkpoint
  before each read, and converge again after it. That needs a state that converges, and the
  probe finds none: blocker 1 persists from the table of contents to the index.

## Verdict for B (feasibility, BELIEF unless stated)

- **As specified, no speedup on this path.** The probe (VERIFIED) says no join would verify,
  so every stretch after the first would be redone.
- **What it would need first** (each is a new comparison rule with a soundness case):
  - (a) PDF object-number, named-destination (`avl`) and string renumbering, in the patch
    and in the comparison, for the `.toc`'s extra entries. This is a relabelling across
    `obj_tab`, `pdf_mem`, whatsits, the name trees and the PDF bytes already written;
  - (b) the patch taken after the `\document` hooks rather than at `AuxDone`;
  - (c) an exact image-table comparison;
  - (d) the side table's source positions.
- **Engineering for the parallel run:**
  - several `Globals` per process, each with its own arena. The arena is 508 MiB virtual for
    infdesc (532,908,032 bytes); ×2's single-engine peak RSS is about 1.1 GB. Guess: 0.3–0.6
    GB more per extra engine;
  - per-engine state that is now process-wide (the `system` file journal, outputs and
    terminal);
  - private PDF streams per stretch, stitched with object renumbering and xref relocation;
  - a join test per stretch (0.35–0.7 s each on ×2 at today's diff sizes).
- **Speedup if (a)–(d) were solved:**
  - pass 3 only, because pass 2 does not match pass 1 at all. Pass 3 also cannot start before
    pass 2 has written the whole `.aux` at `\end{document}`;
  - with 6 stretches on the M5 Pro, pass 3 drops from about 28 s to about 6–8 s, so `DONE`
    goes from about 83 s to about 60 s (about −25 %);
  - 8 GB or more of memory for six engines on ×2.

## Reproduce

```sh
git checkout agent/kabir-claude-coldopen/probe-modpatch     # 414a2ffaf
cargo build --release -p flashtex-engine -p flashtex-display-list   # then mkeng.sh NAME
FLASHTEX_MODPATCH_PROBE=1 FLASHTEX_MODPATCH_STRIDE_NOL5=10 FLASHTEX_BUDGET=7000000000 \
  python3 tools/incr-bench/coldopen.py IB/NAME DOCDIR infdesc-x2.tex OUT.jsonl \
  --external-tools auto --host-arg=--profile --host-arg=balanced
python3 docs/evidence/pass-modpatch-probe-2026-10-24/mpstat.py OUT.jsonl.host-stderr
```

The raw `[modpatch]` lines are in `raw/*.modpatch.txt.gz`. `v1-inf` is the strict-only first
version, with every page of pass 2; its lines have the older flat format. `raw/mpstat-*.txt`
are the summaries, `raw/*.jsonl` the coldopen lines, and `raw/base-inf.jsonl` an unprobed
infdesc open (67.7 s `DONE` on the PC).
