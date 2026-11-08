# CONTIG-TOKENS: contiguous token-list storage under pdfTeX's observables

Lane **CONTIG-TOKENS** (DESIGN.md §4.2 "contiguous token-list storage, but only after P-T1 on the
full suite", §5.6 item 2, §12 P6). Status: design, measured on main, and one prototype measured
against it; the prototype's idea is landed always on by PR #2016 (LAZY-FLUSH), whose gates are
summarised in §5. Evidence: [`docs/evidence/contig-tokens-2026-11-01/`](../../evidence/contig-tokens-2026-11-01/).
DESIGN.md stays the source of truth: this document's recommendation (§5) is not a decision until
§13 of DESIGN.md records one.

## 1. What is stored today

A token list is a chain of one-word nodes in hi memory (`mem` is 5,000,000 8-byte words): `info` is
the token, `link` the next node. Macro bodies and `\toks` values start with a reference count. Free
one-word nodes are one LIFO chain from `avail`; `get_avail` (pdftex.web §138) pops it, and when it
is empty takes `hi_mem_min-1` (`mem_end=mem_max`, so hi memory only grows down). `flush_list`
(§141) walks a list to its end, counting each node out of `dyn_used`, and links the end to `avail`.
The generated engine calls `get_avail` from 93 places, `flush_list` from 31, `delete_token_ref` from
79, `begin_token_list` from 51 and `show_token_list` from 18.

## 2. What a run can see of that storage

1. **Counts.** `\tracingstats>1` prints `var_used&dyn_used` before and after each shipout's flush
   and `hi_mem_min-lo_mem_max-1` ("still untouched", §667); the end of the run prints
   `lo_mem_max-mem_min+mem_end-hi_mem_min+2` words; the format's dump prints its locations and
   usage. `get_node` grows the variable-size region by `hi_mem_min-lo_mem_max` (§144).
2. **Overflow.** `get_avail` stops with `main memory size=5000000` when `hi_mem_min` meets
   `lo_mem_max`, after `runaway` (§328) has printed the list being built (`def_ref`, `temp_head`,
   `hold_head`): the *moment* matters, since the partial list is printed.
3. **Content.** Logs, `\showlists`, `\tracingmacros`, `show_context` and `runaway` print tokens,
   never addresses; `show_context` marks `loc` by identity (`p=q`), not by order.
4. **Kind.** `is_char_node(p)` is `p>=hi_mem_min`; `show_token_list` prints `CLOBBERED` outside
   `hi_mem_min..mem_end`.

P-T1 normalises items 1's lines and the end-of-run block (DESIGN.md §1.1) but both harnesses still
report them as an accounting check; this design keeps them exact. Beside pdfTeX's observables the
engine itself reads addresses. Each reader, and why §4's lazy flush (as #2016 lands it) leaves it
as it was:

- the display list's side table (`dl_new_node`, keyed by the address allocated): every node gets
  pdfTeX's address in pdfTeX's order (§4), so every key is the same;
- the convergence test, checkpoints, restores and hashes (`crate::iso`, `incr::free_cells` walking
  the `avail` chain, the dead-cell rule): they read the word space only after `spill_scalars`,
  which #2016 makes link the pending segments into one chain first, so they see pdftex.web's words;
- `readset`'s same-address patches and lineshift's taint bits: keyed by a live list's head or a
  live node's address, which (as for the side table) do not move;
- BOX-MEMO's freed-list hook (by head): it is told the head that `flush_list` frees, which it still
  gets; it never follows the freed list's links;
- pdfTeX's own readers of a freed cell's link: `check_mem` and `search_mem` (pdftex.web l.3966,
  l.4053) are in `debug ... gubed`, which `web2rust-default.args` does not compile in; `show_token_list` of a dangling list, or a use after free
  on an error path, would read the one word that differs (a pending segment's last link is null,
  where pdfTeX's points at the older free list). pdftex.web has no such read (it would corrupt
  pdfTeX's free list too), and trip and etrip, which reach the error paths, pass on #2016. A
  verify-mode assertion on such reads (review of #2008, item 5) is not implemented.

Token lists are shared, not only owned, which matters to Rule 2 below. pdftex.web calls
`add_token_ref` at 26 places: every macro or `\toks` list put on the input stack
(`begin_token_list`, l.8225), `\let` of a macro (l.31759) and `\toks` copied between registers
(l.31908), marks (`\mark` nodes copied, l.4661, and the page builder's `top_mark`, `first_mark`,
`bot_mark` and `split_bot_mark`, l.27781 and l.28424–28495), e-TeX's mark classes (l.39792–39829), and the
pdfTeX whatsits `copy_node_list` copies (`\write`'s tokens, literals, colour stacks, `\pdfsetmatrix`,
annotations, link attributes, destination and thread ids; l.35790–35867, l.36629, l.36651). A store
outside `mem` would need shared, reference-counted, immutable slices that live across the page
builder and shipout for all of them. Other routines hand a list's ownership
to the input stack (`back_list`, `ins_list`), make one-node `backed_up` lists (`back_input`,
`\expandafter`), or rewrite `info` in place before handing it on (`\uppercase`, `\lowercase`).
Assignments (`\def`, `\edef`, `\toks`) replace by reference (`eq_define` and `delete_token_ref`)
and `\afterassignment` holds one token, so those need nothing new.

**Rule 1: the layout is free.** Any allocator that returns *some* free one-word cell when one exists,
and takes `hi_mem_min-1` only when none does, leaves items 1, 2 and 4 unchanged: the number of free
cells moves by one per allocation and per freed node whichever cell is chosen, so the moments
`hi_mem_min` moves, and the overflow, are the same, and `dyn_used` is the same sequence. Item 3
needs only that no cell is read before it is written, which pdftex.web keeps: 82 of the 93
`get_avail` sites write the new cell's `info` at once, and the other 11 copy a whole word into it or
make a list head whose `info` nothing reads. Rule 1 is why contiguity *by placement* is safe, if
the placement is a function of the word space (the convergence test compares states, and a
restore must place as the old run did).

**Rule 2: the counts are not free.** Storing tokens outside `mem`, or not copying them (an argument
as a slice of the caller's list), changes how many cells are in use. It stays exact only with a
phantom count Φ of the cells pdfTeX would hold: print `dyn_used+Φ`; take `hi_mem_min-1` whenever the
free chain has at most Φ cells (pdfTeX's would be empty); and do it *per token*, not per list, so that
the overflow and its `runaway` text come at pdfTeX's token. Then `runaway` and `show_context` must
print slices, every sharer of §2's catalogue needs a reference-counted slice, and every routine
that splices or edits a list in place must copy instead (`\the`
inside a definition, `ins_the_toks`; `macro_call`'s brace stripping; alignment templates), and the
convergence test must learn the new representation. That is MACRO-REPLAY-sized work (§5.6 item 2's
"argument matching without allocation"); §3 and §4 measure part of what it could buy.

## 3. What contiguity could buy

A contiguous list would buy two things: an O(1) free (its end is known) and reads from consecutive
words. The free is measured, on the prototype's own binary (§4, switch on against off): the walk in
`flush_list` is about 2 % of a cold compile's instructions and 3–6 % of its cycles. The reads are
not measured: the per-line cachegrind run that would show how often a token read misses
(`contig-bench`, run 41335270525, a simulated M1 cache) was cancelled in the queue and never ran,
so no read-side measurement exists. A belief, not a measurement: the room is small, since main retires 2.6
instructions a cycle on the PC (*Infinite Descent ×2*: 450 G in 169–176 G cycles) and P6-HYPEROPT
measured 3.4 on an M1, and LIFO reuse already hands a list the cells its predecessor just freed.

## 4. Prototype: `flush_list` without the walk

The prototype gets the O(1) free without moving a token (`changes/lazyflush.ch` and
`src/lazyflush.rs` on branch `agent/kabir-claude-contig/lazy-flush`, commit `cb7108610`;
`FLASHTEX_LAZY_FLUSH=1`, the command line only). The free list becomes a stack of
segments: `flush_list(p)` of a list of two or more nodes pushes `avail` and sets `avail:=p`, without
walking; `get_avail`, finding `avail` null, pops the next segment before it touches `hi_mem_min`. The
free list is the same *sequence* as pdftex.web's after every operation, so it is stronger than Rule 1:
every node gets **pdfTeX's own address, in pdfTeX's order**, and items 1, 2 and 4 cannot move. The one
cost is `dyn_used`, which no longer counts the lists freed; the memory-usage lines print it worked out
as pdftex.web's dump does (hi memory less the free list, which pdfTeX's counter always equals), and the
dump first links the segments into one chain, so the format's words are pdfTeX's. The hosts never
switch it on, so `crate::iso` and checkpoints see one chain. A one-node list (most are a backed-up
token) is freed by `free_avail`, which is what the walk does to it; a longer one is walked as before
while 16,384 segments are pending, or always with the switch off (one limit test, `seg_limit=0`).

**Expected** (written before the first measurement, from P6-HYPEROPT's M1 profile of full-100,
where `end_token_list` and `delete_token_ref`, both with `flush_list` inlined, were 9 % of a cold
compile's cycles): instructions −1.5 to −3.5 %, cycles a little more; the switch compiled in but off
within ±0.2 % of main.

**Measured** (VERIFIED: the NixOS PC, AMD 7800X3D, `perf stat`, 5 interleaved compiles per
engine and document, medians; run 41338025620, raw/pc-2/; main `3f835f488`; the build measured is
`3cb785555`, whose tree differs from `cb7108610` only in `src/generated/`, which the run
regenerates from the same change files, and in `scripts/bench.py`):

| | instructions | cycles | runs with fewer cycles than main |
|---|---:|---:|---|
| *Infinite Descent ×2*, switch on | −0.36 % | **−4.23 %** (159.4–164.7 G against 168.1–174.9 G) | 5 of 5 |
| 13 arXiv papers (sums), switch on | −0.40 % | **−2.41 %** (per paper −0.7 to −7.5 %) | 13 of 13 papers |
| *Infinite Descent ×2*, switch off | +1.74 % | −0.27 % | |
| 13 arXiv papers, switch off | +1.15 % | +0.50 % | 2 of 13 papers |

An earlier build of the same idea (the switch as a test of its own, the one-node test last; run
41335252513, raw/pc-1/) gave −6.68 % and −3.00 % cycles: the PC was shared, so the cycle deltas carry about ±2 points between runs, and
every one of the 28 document medians in the two runs was below main's. Every PDF (210 compiles per
run, each run a different build: pc-1 is `d362f5381`) is main's byte for byte, and so are the logs,
but for one paper whose log prints the binary's own path in four pdfTeX warnings
(raw/pc-2/…-log-main-vs-proto.diff). None of these documents sets `\tracingstats>1`, so their logs
hold only the end-of-run "words of memory" line, which reads `hi_mem_min` but not `dyn_used`: these
runs do **not** test the recomputed `dyn_used`. #2016 tests it against pdfTeX itself (§5). The
`maxrss_kb` field of these runs is `RUSAGE_CHILDREN`'s running maximum, the same in every row, so
they carry no memory data; §6's runs record each compile's own peak.

Two biases these cycle numbers do not bound (review of #2008, item 4): the PC was shared (load
2.9–5.0), and `bench.py`'s rotation put `on` straight after `off` (the same binary and format) in
4 of 5 reps while `main` never followed its own build, which on a 96 MB L3 could favour `on` in
cycles but not in instructions. The `off` build's instruction count is measured; the always-on
build's is not (on and off share a binary). §6 repeats the measurement on #2016's always-on build
in a random order with an A/A control. All of it is x86; nothing here is measured on an M-series Mac.

So the guess was wrong in an instructive way. The walk is only about 2 % of the instructions (on
against off, the same binary: −1.8 % on the book, −1.5 % on the papers) but 3–6 % of the cycles.
The likely reason (a reading of the numbers, not profiled): the walk is a tight chain of dependent
loads that nothing overlaps, while `get_avail` touches the same nodes later one at a time, between
other work, where the out-of-order core hides the latency. And the switch is not free: with it off
the build retires 1.2–1.7 % more instructions than main, from the extra tests in `flush_list`'s 31
inlined copies and `get_avail`'s 93.

## 5. Recommendation

1. **Do not move tokens into contiguous storage.** Rule 2 keeps a separate token store exact only
   with per-token phantom accounting and a new consumer for every one of the list routines above;
   what linked storage measurably costs is the *free*, and §4 removes that without moving a token.
2. **Land lazy flush always on, in every binary.** Done as its own change, PR #2016
   (`changes/lazyflush.ch`, `src/lazyflush.rs`), with no runtime switch, since the off path costs
   1.2–1.7 % instructions. The hosts see one chain: `spill_scalars`, where every reading of the
   word space starts (checkpoint, restore, hash, convergence test, format image), first links the
   pending segments (`flashtex_merge_free`). Each node it walks was freed since the previous merge,
   so it walks at most the nodes the walking `flush_list` would have, but it walks them in a batch,
   on the edit path (review of #2008, item 3): §6 measures what that does to the host. #2016's
   gates (VERIFIED unless a line says otherwise):
   - perf A/B on the PC against its base `c5763d9b8` (run 41348384137, raw/pc-3/): *Infinite
     Descent ×2* −0.61 % instructions, −6.33 % cycles; 13 papers −0.66 % instructions, −2.69 %
     cycles, fewer cycles on 12 of 13; 140 PDFs identical. Same rotation as §4, so its cycle
     numbers carry §4's bias; §6 bounds it.
   - `\tracingstats=2` against TeX Live's pdfTeX (run 41349282689, re-compared with
     `scripts/stats.py --logs`, raw/stats-1/): on a beamer deck (249 memory-usage lines) every
     field of every line equals pdfTeX's; on arXiv 2501.06980v1 (13 lines) #2016 prints exactly its
     base's lines and its base's whole log, and both differ from pdfTeX in `dyn_used` by +33 and
     +44, a difference of main's own (#2039).
   - T2 on the PC (run 41348460353): reference and candidate each fail the same 22 tests; fixture-wide
     incremental soundness: 108 documents, 217 compiles, 0 differences.
   - PR CI (trip, etrip, lockstep, the P-T fixtures, the engine's tests) and the soundness sweeps
     `sound-a`, `sound-timed`, `sound-d`, `sound-budget-d` with BOX-MEMO on, verify and off (runs
     41420459736, 41420480889, 41420501650 on `49a5c303b`). On the head after main was merged in
     (`e0d4f2485`), PR CI's engine jobs (lockstep, the P-T fixtures, the engine tests, T0) passed
     (run 42502271950); its sweeps are to be run again there once sweeps run on the PC (#2158).
   - Forced "main memory size" overflow (2026-11-08, the PC, raw/ovf-1/; #2016's merged head
     `e0d4f2485` against main `6e66beddf` and TeX Live 2026's pdfTeX): `ovf1.tex` doubles a macro
     with `\edef` until memory runs out, so every earlier body is freed as a segment; `ovf2.tex`
     first frees 30,000 short argument and `\toks` lists, then overflows in an `\edef` of a
     macro argument (the `<argument>` context).
     Both stop at the same token with the same `Runaway definition?` text and context as pdfTeX,
     and #2016's logs equal main's byte for byte; pdfTeX's logs differ from both only in the two
     string-pool capacity lines ("out of 467050" against 467068), which P-T1 normalises.
   - The pdflatex format #2016 builds is byte-identical to main's (SHA-256 `47b8b5dc…`, both).
   - Not done from the review's list: a test with the segment stack full (16,384 segments
     pending; `seg_limit` is a constant), a verify-mode assertion on reads of a pending segment's
     last link, a restore after a caught panic in the middle of a segment, and any measurement on
     an M-series Mac.
3. **Contiguity by placement (Rule 1)** only if a read-side measurement shows token reads missing the
   cache (§3); its parity risk is nil, but it competes with LIFO reuse, which keeps hot cells hot.

## 6. Follow-up measurement: A/A control, random order, the host (2026-11-08)

Run on the NixOS PC (AMD 7800X3D) inside the agents' CPU-capped slice, on #2016's head after main
was merged in (`e0d4f2485`, `lazy`) against main `6e66beddf` built twice from the same tree
(`main` and `mainB`: the same binary, the A/A control). Scripts and raw records:
`docs/evidence/contig-tokens-2026-11-01/raw/ab-1/` and `scripts/{bench2.py,hostab.sh,absum.py}`.
The PC was shared with about 30 other lanes for the whole run (load 0.2 to 28, the slice's CPU
quota split between them), so wall-clock times are not comparable at all and cycles are noisy;
instructions do not move with load. Every row says which it is.

**Host (review of #2008, item 3).** The socket host as the app starts it (checkpoints and
BOX-MEMO on), *Infinite Descent ×2*, 20 letters typed 300 ms apart into a prose line on page 842
(the book's second copy, `x2/book/number-theory/modular-arithmetic.tex`), each waiting for
`DONE`; the whole host process under `perf stat`, so a session is the cold open (with its
`makeindex` passes) plus the 20 keystrokes. Engines in a seeded random order per round. Three
sessions hit the time limit on the starved PC and are left out (main's round 2 twice, lazy's once,
which a rerun then completed), and round 3 was stopped to leave the PC to the cold compiles: what
is left is main 1, mainB 2 and lazy 2 sessions.

| engine | sessions | session instructions | Δ vs main | session cycles | Δ vs main | request CPU ms, p50 / p95 | edited page CPU ms, p50 / p95 |
|---|---:|---:|---:|---:|---:|---:|---:|
| main | 1 | 1,634.18 G | | 929.8 G | | 313 / 357 | 25.8 / 28.0 |
| mainB (A/A) | 2 | 1,632.65 G | −0.09 % | 971.8 G | +4.5 % | 358 / 395 | 27.7 / 34.8 |
| lazy | 2 | 1,626.56 G | **−0.47 %** | 1,012.7 G | +8.9 % | 361 / 459 | 31.6 / 41.2 |

- VERIFIED: in the host, LAZY-FLUSH retires fewer instructions than main (−0.47 %, against an
  A/A spread of 0.09 %), so batching the walks at `spill_scalars` does not cost more instructions
  than the walks it replaces.
- NOT SHOWN either way: cycles and the per-keystroke CPU times. The A/A pair, one binary, already
  differs by +4.5 % in cycles and +14 % in request CPU time, and lazy's second session ran at load
  22; in the first round, at load 0.2 to 17, lazy's session took 944.6 G cycles against main's
  929.8 G and mainB's 947.4 G. A host cycle cost below about 5 % on the edit path can be neither
  shown nor excluded on this PC, and §1.2's 20 ms key-to-commit target is about an M1-class Mac,
  where nothing was measured.

**Cold compiles (items 2 and 4).** `bench2.py`: `bench.py` with a seeded random order per rep
(each run records its predecessor), each compile's own peak RSS (`wait4`), the document settled
once and the settled tree copied to every engine (each engine's own pass must leave it settled).

The 13 papers, 5 reps per engine and paper, 195 compiles (raw/ab-1/cold/; the table per paper is
`absum.py`'s, in raw/ab-1/absum.md):

| engine | instructions (sum of medians) | Δ vs main | cycles (sum of medians) | Δ vs main | papers with fewer cycles than main |
|---|---:|---:|---:|---:|---|
| main | 155.09 G | | 89.74 G | | |
| mainB (A/A) | 155.09 G | −0.00 % | 88.85 G | −1.00 % | 8 of 13 |
| lazy | 154.15 G | **−0.60 %** | 86.88 G | **−3.19 %** | 12 of 13 |

- VERIFIED: −0.60 % instructions (each paper −0.22 % to −1.28 %; the A/A pair is exact to 0.01 %),
  the same as #2016's pc-3 run (−0.66 %) and the prototype's (−0.40 %), so the merge-in of main
  changed nothing here.
- VERIFIED, with its bound: −3.19 % cycles in the sum, against an A/A difference of −1.00 % in the
  same run; per paper the 95 % bootstrap intervals are wide on this PC (the A/A's reach −18 % to
  +29 %), and lazy's interval lies wholly below zero on 3 papers, the A/A's on 1. Cycles were about
  1.5× pc-3's for the same compiles: the slice was starved and, for part of the run, the PC was
  short of memory. Order: no run was measurably helped by following its own engine (a run's cycles
  against its median: lazy after another engine +0.59 %, after itself +0.99 %), so the rotation bias
  of §4 does not explain the result.
- Output: all 195 PDFs are main's byte for byte; the logs are main's but for 2501.07084v3, whose
  warnings print the binary's own path (mainB's differ from main's the same way).
- Peak RSS per compile (now measured per child): 60–85 MB, within 1 MB of each other for the three
  engines on every paper.

**What the recomputed `dyn_used` costs (item 2).** *Infinite Descent ×2* with `\tracingstats=2`
from its first line, so each of its 1,140 shipouts prints a memory-usage line and LAZY-FLUSH
walks the whole free list twice for it. One settled compile per engine (instructions do not move
with load; cycles from one rep are not reported):

| engine | instructions | Δ vs main | whole log, PDF |
|---|---:|---:|---|
| main | 454.81 G | | |
| mainB (A/A) | 454.81 G | −0.00 % | main's |
| lazy | 452.80 G | **−0.44 %** | main's, byte for byte |

- VERIFIED: with the walks the statistics cost, LAZY-FLUSH still retires 0.44 % fewer instructions
  than main on the book, and every one of the 1,140 memory-usage lines (the whole log) is main's.
  Without `\tracingstats`, #2016 saved 0.61 % on the book against its old base (pc-3); the
  difference, about 0.2 points, is a belief about what the walks cost, since the two numbers come
  from different bases and runs.
- So lockstep and P-T1, which run under `\tracingall`, pay for the recomputation but remain
  below main's instruction count on this document. Lockstep's own wall time was not measured.

**What this settles.** On the evidence above, LAZY-FLUSH retires fewer instructions everywhere it
was measured: cold papers, the book with and without `\tracingstats`, and the host's open plus
keystrokes. Its cold-compile cycle gain on the papers (−3.19 %) is about three times this run's
A/A difference (−1.00 %), so it survives the review's bias control. The host's cycles and
per-keystroke CPU time are within this PC's A/A noise and remain unmeasured to the precision the
review asked for; so does everything on an M-series Mac.
