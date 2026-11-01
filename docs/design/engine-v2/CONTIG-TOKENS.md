# CONTIG-TOKENS: contiguous token-list storage under pdfTeX's observables

Lane **CONTIG-TOKENS** (DESIGN.md §4.2 "contiguous token-list storage, but only after P-T1 on the
full suite", §5.6 item 2, §12 P6). Status: design, measured on main, and one prototype measured
against it. Evidence: [`docs/evidence/contig-tokens-2026-11-01/`](../../evidence/contig-tokens-2026-11-01/).
DESIGN.md stays the source of truth.

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
engine itself reads addresses: the display list's side table (`dl_new_node`, by address), the
convergence test (`crate::iso`, free cells from the `avail` chain, the dead-cell rule),
`readset`'s same-address patches, lineshift's taint bits and BOX-MEMO's freed-list hook (by head).

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
print slices, and every routine that splices or edits a list in place must copy instead (`\the`
inside a definition, `ins_the_toks`; `macro_call`'s brace stripping; alignment templates), and the
convergence test must learn the new representation. That is MACRO-REPLAY-sized work (§5.6 item 2's
"argument matching without allocation"); §3 and §4 measure part of what it could buy.

## 3. What contiguity could buy

A contiguous list would buy two things: an O(1) free (its end is known) and reads from consecutive
words. The free is measured, on the prototype's own binary (§4, switch on against off): the walk in
`flush_list` is about 2 % of a cold compile's instructions and 3–6 % of its cycles. The reads are
not measured: the per-line cachegrind run that would show how often a token read misses
(`contig-bench`, run 41335270525, a simulated M1 cache) was still queued on GitHub's runners when
this was written. A belief, not a measurement: the room is small, since main retires 2.6
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
engine and document, medians; run 41338025620, raw/pc-2/; main `3f835f488`):

| | instructions | cycles | runs with fewer cycles than main |
|---|---:|---:|---|
| *Infinite Descent ×2*, switch on | −0.36 % | **−4.23 %** (159.4–164.7 G against 168.1–174.9 G) | 5 of 5 |
| 13 arXiv papers (sums), switch on | −0.40 % | **−2.41 %** (per paper −0.7 to −7.5 %) | 13 of 13 papers |
| *Infinite Descent ×2*, switch off | +1.74 % | −0.27 % | |
| 13 arXiv papers, switch off | +1.15 % | +0.50 % | 2 of 13 papers |

An earlier build of the same idea (the switch as a test of its own, the one-node test last; run
41335252513, raw/pc-1/) gave −6.68 % and −3.00 % cycles: the PC was shared, so the cycle deltas carry about ±2 points between runs, and
every one of the 28 document medians in the two runs was below main's. Every PDF (210 compiles per
run) is main's byte for byte, and so are the logs and their memory lines, but for one paper whose
log prints the binary's own path in four pdfTeX warnings (raw/pc-2/…-log-main-vs-proto.diff).

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
2. **Land lazy flush always on, in every binary, as its own lane** (not in this one):
   - without the runtime switch: its off path costs 1.2–1.7 % instructions, so landing it off is a
     regression, and landing it on for the command line only would leave the hosts paying that;
   - in the hosts, link the segments into one chain at the entry of `flashtex_checkpoint_hook`
     (each freed list is linked at most once, so this costs at most the walks saved; to be checked:
     that every checkpoint, restore and convergence test runs from there), or teach
     `incr::free_cells` (the hand-written reader of the free list) the segments;
   - gates: trip, etrip, lockstep with its accounting check, the P-T1/P-T2 fixtures, T2, and the
     incremental soundness sweeps, the hosts' only new risk. Expected: −2 to −7 % cycles on cold
     compiles, from §4.
3. **Contiguity by placement (Rule 1)** only if a read-side measurement shows token reads missing the
   cache (§3); its parity risk is nil, but it competes with LIFO reuse, which keeps hot cells hot.
