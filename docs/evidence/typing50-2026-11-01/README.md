# TYPING-50MS: a key's own compile paints its edited page (2026-11-01)

Lane **TYPING-50MS** (kabir-claude-typing50, under Commander kabir-claude). DESIGN.md §1.2: key → commit
≤ 20 ms at 50 ms typing on the slowest supported Mac (M1-class), *Infinite Descent ×2* included; every
key painted by its own compile. The row: T7 `typing@50ms@x2-late` (PR #1889's harness: infdesc-x2,
`x2/book/real-numbers/inequalities.tex`, 0-based page 887, external tools on, 1,152 pages).

## 1. Where a key's time goes (VERIFIED, M5 Pro, main `df75c3a20`)

mac-m5pro-kabir, Super cores (no throttle), load1 27-33 (other agents): **the ms are not reference
numbers; the instruction counts are** (DONE `stages.marks`: wall, user, system, instructions per phase).
`raw/m5-main-typing@150ms@x2-late.jsonl.gz`, keys 150 ms apart; medians of the 12 keys that insert
(READ-REVALIDATE path; `scripts/marks.py`):

| from COMPILE arrival | wall ms | user ms | sys ms | instructions |
|---|---:|---:|---:|---:|
| queue (the previous compile's background work) | 4.3 | | | 36 M |
| request, spans moved (`p`, `m`) | 1.0 | 0.3 | 0.7 | 10 M |
| **find** (`f`: S₀ key check 8.2 ms, what changed 4.0 ms) | **13.2** | 5.1 | **7.6** | **135 M** |
| restore to P0 (`r`) | 4.8 | 1.4 | 0.7 | 28 M |
| probe, verdict, reattach (`k q v j`) | 5.1 | 3.4 | 1.2 | 30 M |
| restore near the edit (`f r k`) | 5.6 | 2.4 | 0.6 | 40 M |
| **typeset to the edited page, display list** (`d`) | **17.6** | 16.8 | 0.3 | **144 M** |
| **total to the edited page** | **52.9** | | | **~420 M** |

The delete keys (text = the last complete run's) skip the revalidation: 38 ms, find 14 ms, typeset 18 ms.
What a key waited for when it came during the previous compile's background work (`scripts/qb.py`):
the convergence test 52-155 M instructions (2.5-7 ms here), the jump's adopt 27-83 M (3-9 ms), never
more than one at a time.

**On an M1 core** (MEASURED on the hosted M1 VM, §3; same split, `raw/m1vm-run1-change-2`, keys 150
ms apart, 13 inserting keys): 81 ms to the edited page: queue 6.7, find 13.9 (CPU 13.3), restore to P0
13.9 (CPU 5.0), revalidation 7.5 (CPU 6.2), restore near the edit 13.2 (CPU 4.4), typeset and display
list 22.1 (CPU 21.9). Per phase the M1's CPU time is 1.0-1.3× the M5's (typesetting 21.9 vs 16.8 ms:
the clock ratio, 3.2 vs 4.3 GHz, at about the same IPC; Geekbench's 1.75 overstates it for this code).
The restores' off-CPU time (about 9 ms each) is the VM's (page faults, which a guest pays for in VM
exits); without it the M1 would need about 55-60 ms. **The edited page alone does not fit in 50 ms on an
M1 yet**; it does on the M5 (40-55 ms at load 30). The find (135 M, a third of the instructions and a quarter of the wall,
7.6 ms of it system time: ~3,700 system calls before the first page) and the typesetting (144 M) are the
two big items: lanes EDITED-PAGE / FANCYHDR / FS-WATCH, not this one.

## 2. The scheduling change

**Before** (LIVE-30MS): a compile was protected from newer work up to its first changed page only when
the compile before it had been stopped before its page ("starved"), so at most every other compile was
held. A compile that was not protected and had not reached its page when the next key came painted
nothing. With the previous compile's background work in the way (a test or a jump to stop, 3-9 ms), or
an edited page a little longer than the interval, every other key was lost. Main at 28 ms on the M5
at load 80-96 (the edited page then longer than the interval, as on an M1 at 50 ms): 10 of 12 compiles
that ran never shipped their page (`sum.py`: "noship"). On the hosted M1 VMs: 20 of 39 at 50 ms (§4).

**After**: every incremental compile's first pass is protected to its first changed page (or 3 pages when
none changed; a CANCEL still stops it), and the protection ends as soon as the host has sent the client
a page it did not hold (`Session::set_painted`). The engine's "changed" is against the last complete run:
a key that returns to that run's text (a letter typed and deleted) changes no page against it, and its
run was held three pages after the page the client lacked had gone out (194 ms of the next key's wait on
a hosted M1). When the edited page alone takes less than the interval, every
key is painted by its own compile: the next key's compile waits at most for the page in progress, starts
right after it (the protected run stops at that page's checkpoint, no background work to stop), and so
does not drift. When it takes longer, a key waits at most for the page in progress, and the keys that
came meanwhile go into one compile: the own-compile share is about interval / page time, where the
alternating rule gave at most one in two. Stopping later than asked changes nothing the engine computes.

The tools' `compile-interrupt` (the soundness driver's interleaved edits) still stops a run exactly
where it asks (`Session::set_protect_edit(false)`), before its edited page too, which a CANCEL still
does in the product.

**The typist** (dl3-keys `--interval-ms --kind letter`): two letters typed and deleted in turn ("x",
"xy", "y", none). With one letter typed and deleted, every other key's text is the one of two keys
before; when the host folds the key in between into the next compile, that compile's edited page is the
one the client already holds, the host rightly sends nothing, and the key reads as not painted by its
own compile while the client shows the right text. In the LATE-EDIT run of `typing@50ms@x2-late`
(`/tmp/fth-le/t7-rp3`, main `8e0faf20d`) the protected compiles 6, 8, …, 16 all reached their edited page
(`edited_instr_k` set) and none sent it: that is most of the "12 of 15 not painted".

## 3. M1-class measurement

- **No bare-metal M1 runs here.** The runner list (`gh api …/actions/runners`, 2026-11-01) has
  mac-m5pro-kabir-1..3 (this Mac), mac-m5pro-dq222-* (offline) and the NixOS PC; mac-m1max-a-1/-2
  (Jaysen's M1 Max, the only M1-class machine on the owner's list) are no longer registered. The PC
  needs a Tailscale SSH re-authentication (`ssh nixos` asks for a login URL).
- **GitHub-hosted macos-15** is "Apple M1 (Virtual)", 3 vCPU: the engine thread's CPU time per phase
  there is M1 silicon's time for that work (VM exits inflate system time), but it has no instruction
  counters, the host's other threads share the 3 vCPUs, and VMs differ in speed (letter@x2-late p50
  38-80 ms across the VMs below). Since #1909 the typist keeps its clock there: no key below went more
  than 5 ms late. Branch agent/kabir-claude-typing50/m1-measure (measurement only: PR #1889's harness,
  this branch's typist): this branch's host and main's, one VM each, dispatched twice (runs 41327596723
  and 41332429511), two t7 runs per VM. Results in §4.
- **What is missing for an honest reference:** a bare-metal M1 (or M1 Pro/Max: the same Firestorm core)
  registered as a self-hosted runner (e.g. label `flashtex-m1`), on AC, idle (t7's reference
  conditions), running `realdoc-gate.sh --strict`. Until then: instruction counts on the M5 × an M1
  instruction rate measured on the hosted VM's CPU times.

## 4. Results

Hosted M1 VMs, infdesc-x2 at x2-late, 15 keys a row (key 0 a warm-up), two dispatches × two t7 runs
(`raw/m1vm-run{1,2}-{main,change}-{1,2}`; `scripts/sum.py`). Run 1's "change" leg has the protection, the
`compile-interrupt` rule and the typist; run 2's adds the protection's end at the client's changed page. Both legs type with the new typist.

| row | keys painted by their own compile, main | this branch | compiles that ran and shipped no page, main | this branch |
|---|---|---|---|---|
| typing@50ms | 6, 2, 1, 4 = **13 / 60** | 4, 8, 7, 8 = **27 / 60** | **20 / 39** | **2 / 31** |
| typing@100ms | 5 (6 keys never painted), 9, 10, 10 = 34 / 54 | 9, 12, 10, 11 = 42 / 60 | 11 / 48 | 0 / 46 |
| typing@150ms | 8, 13, 14, 10 = **45 / 60** | 13, 15, 15, 14 = **57 / 60** | 8 / 60 | 0 / 61 |

Latency to the watched page from the key's own or a later compile, p95 (ms; per VM, so only within a
dispatch): typing@50ms main 441, 503 | 852, 523; this branch 258, 99 | 70, 129. typing@150ms main 548,
171 | 181, 128; this branch 176, 148 | 77, 92.

- **Every compile that runs now ships its page** (2 of 31 did not at 50 ms; on main, half did not).
  The keys still not their own at 50 ms are folded: on these VMs the edited page takes 38-80 ms, more
  than the interval, so about interval / page time is the most a scheduler can give (§2).
- **At 150 ms** (page time below the interval) main lost 15 of 60 keys and this branch 3: on main a
  compile that came during a jump or a test (20-35 ms there) reached its page after the next key and
  was not protected (main's queue there: jumps and tests). The 3 left are not analysed.
- **Not measured: the 20 ms target on an M1.** The edited page itself is 55-80 ms there (§1).
