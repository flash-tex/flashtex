# T7-BEAMER: the beamer rows with BOX-MEMO on in the host (2026-10-20)

Lane T7-BEAMER (Commander kabir-claude). Design: `docs/design/engine-v2/BOX-MEMO.md` §11.

The machine is the NixOS PC (x86-64, shared, with the `flashtex-agents.slice` CPU quota), so **every time here is non-reference**. In the final run the quota stopped the slice for 168 s, because of other agents' jobs.

The quantity measured is the engine thread's **instructions per key to the edited page**: the host's `DONE.stages.edited_instr_k`, or `first_page_instr_k` for the newline rows, which have no `edited_*` fields. It does not move with load.

- **main** is `fc9fcf5a5` (engine `base`).
- **BOX-MEMO** is `agent/kabir-claude/t7-beamer-memo` at `8d028f845`, the same tree as `8ecc2d6c0` (engine `memo8`). BOX-MEMO is on by the host's default.
- **T7 command:** `tools/incr-bench/t7.py --docs beamer-5,beamer-30 --preamble 6 --reopen 4`, with every in-body and typing phase at its default keys.
- **Files:**
  - `table-main.md` and `table-boxmemo.md`: the T7 tables.
  - `raw.tgz`: the raw records and summaries (`full-base/`, `full-memo8/`).
  - The table below is the p50 over each row's keys.

## Instructions per key to the edited page (VERIFIED)

| row | main | BOX-MEMO | change |
|---|---:|---:|---:|
| beamer-5 letter@start | 200.1 M | 37.5 M | −81 % |
| beamer-5 letter@middle | 200.1 M | 37.5 M | −81 % |
| beamer-5 letter@end | 199.1 M | 36.5 M | −82 % |
| beamer-5 sentence@middle | 200.1 M | 37.5 M | −81 % |
| beamer-5 newline@middle (first page) | 201.8 M | 39.4 M | −80 % |
| beamer-5 split@middle | 200.1 M | 37.5 M | −81 % |
| beamer-5 typing@50 / 60 / 80 / 100 / 150ms | 200.4–203.8 M | 37.6–37.8 M | −81 % |
| beamer-30 letter@start | 486.1 M | 168.2 M | −65 % |
| beamer-30 letter@middle | 470.4 M | 131.7 M | −72 % |
| beamer-30 letter@end | 470.5 M | 131.8 M | −72 % |
| beamer-30 sentence@middle | 470.4 M | 131.8 M | −72 % |
| beamer-30 newline@middle (first page) | 467.9 M | 130.1 M | −72 % |
| beamer-30 split@middle | 469.9 M | 131.9 M | −72 % |
| beamer-30 typing@50 / 60 / 80 / 100 / 150ms | 468.5–475.0 M | 132.0–133.4 M | −72 % |

Convergence is unchanged: every in-body row converges on every key in both runs (T7's `conv` column).

**beamer-30 letter@start** is the deck's first block frame. Each keystroke there loads fonts and makes the shadow's shading objects for the first time. Font versions named by their changes and the doomed entries took it from 667 M down to 168 M (BOX-MEMO.md §11.5).

**Times (non-reference, PC).** In a quieter run of an earlier build of this branch with the same instructions, beamer-5's in-body rows met 11 ms p95 on the PC (4.6–6.5 ms), and so did typing@50, @60 and @100ms. beamer-30 ran at 13–16 ms p50 on the PC.

**Belief, not measured:** M5-class hardware runs the engine at about 1.3–1.4× the PC's instructions per second, so beamer-30 should land near 10–11 ms there. The M5 or hosted T7 run decides.

**Memory.** A mixed beamer-30 session (several pages and typing) peaked at 245 MB with BOX-MEMO and 188 MB without. The store is capped at 32 MB, with the keys' macro bodies interned once.

## Where the slide's instructions went (VERIFIED, main)

- **Per frame**, cold command line, callgrind (x86): pdfTeX spends 312 M on a Madrid block frame and 134 M on a default-theme frame; the engine spends 415 M and 176 M, 1.33× pdfTeX. Raw speed alone cannot reach 11 ms.
- **Edit window**, `FLASHTEX_MACRO_PROFILE_CLOCK=instr` with roots (each instruction charged to the innermost listed macro):
  - xcolor's `\XC@col@rlet` (`\colorlet` under beamer's `\usebeamercolor`): 49 % of beamer-30's window (709 calls) and 60 % of beamer-5's (363 calls);
  - pgfmath: 16 % and 13 %;
  - inclusive, the output routine is 54 % and 90 %; Madrid's block is 38 % of beamer-30.
- **What remains in beamer-30's 132 M with BOX-MEMO on:**
  - Madrid's block. Its end declares the shadow's shadings as `\immediate` objects (35–45 M), and its begin opens a group that the end closes.
  - The output routine's own work, and the frame body (paragraphs, fonts).
  - The host's side channels: the diagnostics' definition sites are about 5 % of all typesetting, and the display list's node notes 2 %.

## Correctness

- **Verify mode in the host** (`FLASHTEX_BOXMEMO=verify`; every admitted call is run both ways), letter@start and letter@middle:
  - beamer-5: 968 replays;
  - beamer-30: 27,870 replays, and on a later build 17,727 on letter@start;
  - **0 differences** in the operation log and the fragments, and 0 in the whole engine state.
  - These runs predate `caa840b3f`, which turned the full-state check off in the host (BOX-MEMO.md §7: its checkpoint stayed among the restart points). The host still checks the operation log and the fragments.
- **Verify-mode soundness**, `soundness.py` with sweep C's kinds, 20 trials, final code: framed-tikz-theorems 26 compiles, beamer-madrid 12, beamer-blocks-columns 16; 0 bad.
  - The verify sweep on #1692 had found two holes in the key, which are fixed here and there (BOX-MEMO.md §11.6). Before the fix, framed-tikz-theorems stopped the host.
- **Command line** (`FLASHTEX_BOXMEMO=on|verify`, which records anywhere there), the default names, 25-frame probe decks: 7,421 replays (Madrid block frames) and 1,114 (default theme), **0 differences** in verify mode. The PDFs with BOX-MEMO off, on and verify are byte-identical.
- **T1 lockstep:** 1,468 cases, 1,468 equal.
- **P-T fixtures:**
  - BOX-MEMO off: P-T1 88/88, P-T2 88/88;
  - `FLASHTEX_BOXMEMO=on`: P-T1 88/88, P-T2 88/88;
  - `on` with `--pt pt2` (untraced, where replays happen): P-T2 88/88.
- **`tests/boxmemo.rs`** (9 tests): off, on and verify give the same log and the same PDF bytes for links, form references, `\lastskip` glue, local groups and font parameters.
- **`scripts/gate.sh quick`** (final code): rustfmt, clippy, the web2rust drift test, trip and etrip pass. Tests: one failure, `fontmap_cache::a_cached_parse_writes_what_a_parse_writes`, which is environmental. pdfTeX's warning names the executable's path, and on this PC's longer target directory it wraps inside the phrase the test looks for. It failed the same way on #1692's base.
- **Soundness sweeps** (`sweeps.yml`): every gate passed with 0 bad, 0 wrong and 0 aborts, in BOX-MEMO's verify mode on #1692 (run 39389835057) and in the product default on `8ecc2d6c0` (run 39391094661). Final-head runs are in the PR.
