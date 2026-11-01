# PASS-REUSE: can pass N+1 take pass N's pages? (2026-10-09)

Lane PASS-REUSE asked for later passes of a cold compile to converge against the previous pass
(DESIGN.md §5.3 machinery, §5.5 read-sets) and keep its remaining pages. COMPILE-SPEED (#1319,
2026-10-06) had found that pass 3 changes the output of about 1 % of pages (Infinite Descent 6 of
592, the arXiv paper 2 of 45) and projected a 26–31 % saving. This note measures what pass 3
*depends on*, rather than what it outputs. The finding is that the premise does not hold.
**No product code changes.** Below are a design proposal for a §5.5 amendment and a recommendation
for the Commander.

Machine: mac-m1max-a (M1 Max, macOS 26.3.1), heavily loaded during the runs (load 300+), so
wall times are not references; instruction counts are. Engine: main `adc0ee899`, release build.
Oracle: MacTeX 2026 `pdflatex` (pdfTeX 1.40.29), `SOURCE_DATE_EPOCH=1700000000`.

## 1. What the passes do today (VERIFIED, `*-main-host.json`, `*-incr-debug.txt`)

One cold `flashtex-host iserve` compile per document (`hostrun.sh`):

| document | passes | modes | instructions | pass 3 |
|---|---|---|---:|---|
| arXiv 2501.07023 (`p-trees`, 45 pp.) | 3 | cold, incremental, incremental | 50.8 G | L5 patch refused (`besides 360 changed entries: … long_state: 0x72 -> 0x73`); restarts at the `.aux` point; 7 tests, all fail: `the old run reads the changed ./p-trees.aux later` |
| Infinite Descent (592 pp.) | 3 | cold, incremental, incremental | 773.1 G | **no `.aux` point at all** (anchor S₀, L5 `no .aux point (None)`); restarts at checkpoint 525; 32 tests, all fail (`.toc`, then `.aux` "read later" through page 578) |
| long-deck (118 pp.) | 2 | cold, incremental | 83.8 G | none (pass 2 is the last pass; it re-runs from the anchor with no tests because the `.aux` appeared) |

So pass 3 currently re-typesets every body page. The long deck has no pass 3 to reuse.

## 2. Why pass 3's state never equals pass 2's (VERIFIED)

The `.aux` that pass 3 reads (aux₂, written by pass 2) differs from the one pass 2 read (aux₁,
written by pass 1, which had no `.toc`). Oracle `pdflatex`, three runs:

| document | aux₁ → aux₂ lines changed | `\@abspage@last` | `\newlabel`s changed | aux₂ = aux₃ |
|---|---:|---|---:|---|
| arXiv 2501.07023 | 461 | 44 → 45 | 360 control sequences | yes |
| Infinite Descent | 1,719 | 588 → 592 | 1,101 lines | yes |

When pass 2 fills in the table of contents, the front matter grows and every later page moves.
Every label's absolute-page field changes (Infinite Descent: `{[1][13][]14}` → `{[1][14][]14}`),
and so does `\@abspage@last`. Pass 3's state therefore holds other meanings for hundreds to
thousands of control sequences **at every checkpoint after the `.aux` read**. No checkpoint of
pass 3 equals pass 2's, and the convergence test can never match (§5.3 (d)).

A sound test would need to compare *modulo the L5 patch*, and then keep pass 2's later pages
only where pass 2 never reads a changed entry again. But LaTeX reads `\@abspage@last` at
**every `\shipout`**: `latex.ltx` (TL 2026) l. 19971,
`\int_compare:nNnT \@abspage@last = \g_shipout_readonly_int`. So every page interval of pass 2
reads a changed entry whenever pass 1 → 2 changed the page count. That is the usual reason
for a third pass. On top of that, every `\ref`/`\cref` of a moved label is also a read.

The probe (`watch-probe.patch`) logs **every** read, not only the first, of the control
sequences that the `.aux` read defined. Pass 2 of the arXiv paper made 40,846 such reads
(`window.py`, `arx-probe-windows.txt`):

- **45 of 45** page intervals read a name whose meaning differs between aux₁ and aux₂.
  `\@abspage@last` is read on all of them; for 12 pages it is the only changed name read.
- So page-granular reuse ("take pass N's remaining pages") keeps **0 pages**.

An upper bound for anything finer is to re-run only a window around each read of a changed entry
and converge again right after it. Here is how much of pass 2's body work such windows cover:

| re-run window | share of pass-2 body instructions (18.59 G) | windows |
|---|---:|---:|
| (a) from the checkpoint before a page's first changed read to the page's end | 68.4 % | 45 |
| (b) from the checkpoint before each changed read to the next checkpoint (segment checkpoints included) | 28.7 % | 124 |

Before the per-window costs, the work that could be skipped is therefore at most **31.6 %** of
pass 3 for (a), about 9–10 % of the 3-pass compile, or at most **71.3 %** for (b), about 24 %.
The per-window costs are a restore, a convergence test (2–10 ms each per §5.3) and a jump
(≤ 3 ms) for each window. The arXiv paper's pass 3 runs about 1.4–1.5 s for 45 pages, so 124
windows at 3–5 ms each cost about 0.4–0.6 s. That leaves (b) a net gain of roughly 8–11 % of
the compile at best (BELIEF; not built, not measured). Infinite Descent was not probed: it
takes no `.aux` point, so none of this applies to it until that is fixed.

## 3. What a sound design needs (proposal for a §5.5 amendment, not built)

1. **Convergence modulo the L5 patch.**
   - The test compares the new state with `P(old)`. In the structural comparison
     (`iso.rs`), each patched name is checked against its patch meaning and its old token list
     is excluded.
   - The same rule lets keystrokes converge against checkpoints that `defpatch` marks. Today
     these are refused outright (`Obs::test`).
2. **Complete read logs for `.aux`-defined names.**
   - The read-set records first reads only (`rs_seen`). The names the `.aux` read defines must
     instead be logged on every read, and never marked seen.
   - This needs a word-space `rs_watch` array (`readset.ch`), so that restores and jumps carry
     it. The probe kept it host-side.
3. **Read barriers with repeated jumps within one pass.**
   - A changed name that the old run reads later is a barrier (as `rerun_point` handles
     `\write18`).
   - After the jump, the run goes back live from the checkpoint before the barrier, with
     convergence tests against the old run's later pages, and jumps again after it. Today one
     convergence ends the pass's tests.
   - This needs `defpatch` registered on every kept old checkpoint, and the old run's segment
     checkpoints kept near every read site, which affects retention and the memory budget.
4. **Test (b) bounded at the old run's end point** (reads after it are `\end{document}`'s,
   which always re-runs). The `.aux` "read later" failures above come from this. It is
   independent of the rest and belongs to the convergence lanes.
5. **Prerequisites found on the way** (each a separate fix, in P4-CONVERGE-REWIND / COLD-OPEN
   territory):
   - Infinite Descent never takes an `.aux` point, so L5 is off for the whole book.
   - The arXiv paper's L5 patch is refused on `long_state`. That is a scalar `macro_call`
     always sets before reading it, so it is dead at `Point::AuxDone`. This is a candidate for
     `dead_word` with a §5.3 soundness case.

The soundness surface is large: three new convergence rules (1, 3, 4), each of which needs a
soundness case and sweeps A/C/D. The projected gain is ≤ 9–11 % of a 3-pass cold compile on the
one document measured, not 26–31 %.

## 4. Recommendation

- Do **not** build pass reuse as specified. Its premise, that a later checkpoint's state
  equals pass N's, fails on both gate documents, and the sound version's ceiling is about a
  third of the estimate.
- The independent items 4 and 5(a–b) are cheaper and help both pass 3 and keystroke L5 passes.
  Route them to the convergence lane.
- If a third-pass cut is still wanted, measure (b) on Infinite Descent once it has an `.aux`
  point before committing to items 1–3.

## Reproducing

- `hostrun.sh DOC ENG TAG [ENV=…]`: one cold host compile, with sources under `/tmp/pr/src`
  and engines from `tools/incr-bench/mkeng.sh` under `/tmp/pr/ib`.
- `git apply watch-probe.patch`, build, then run with `FLASHTEX_WATCH=1`. It logs every read of
  the `.aux`-defined names, and every checkpoint, to stderr.
- `python3 window.py HTIME aux₁ aux₂` computes the window shares from that log and the oracle's
  first two `.aux` files.
