# BOX-MEMO on *Infinite Descent*: edited-page windows, off against on (2026-10-07)

Lane P6-INFDESC-PAGE, Step 3: the first prototype of `docs/design/engine-v2/BOX-MEMO.md`, the measure memo for
framed.sty's `\fb@sizeofframe`.

**Setup.**
- Machine: the NixOS PC, load 8–17.
- Engine: `bm5`, branch `agent/kabir-claude/box-memo` at `630c25f68`, built with
  `tools/incr-bench/mkeng.sh`.
- Both rows use the same engine, the same profiler (`FLASHTEX_MACRO_PROFILE_CLOCK=instr`, the
  roots from the Step 1 README), and the same keystrokes. The only difference is
  `FLASHTEX_BOXMEMO=off|on`.
- Run: `../scripts/sites.sh` with `KEYARGS=--overlap`, 8 letter keystrokes per site, as typing sends
  them.
- The number is the instructions in each window, from the restore to the edited page's shipout,
  with the profiler included (about 1–2 %). It does not move with load.

## Results (VERIFIED, `raw.tgz`: `b5off*/prof.N`, `b5on*/prof.N`)

Steady state is every window after the first at a site. The first window is shown separately: it
records.

| site (line, 0-based page) | off | on | Δ | first window off → on |
|---|---:|---:|---:|---:|
| equivalence-relations.tex:430 (theorem), p. 208 | 855–859 M | 748–750 M | **−13 %** | 6,139 → 6,950 M (restart at p. 200, 9 pages) |
| equivalence-relations.tex:426 (prose), p. 207 | 126–127 M | 85–87 M | **−32 %** | 126 → 140 M |
| sets.tex:195, p. 100 | 664–666 M | 374–376 M | **−44 %** | 752 → 878 M |
| discrete-probability-spaces.tex:13, p. 415 | 245–250 M | 166–168 M | **−32 %** | 247 → 273 M |

- **Where the gain comes from.** `\fb@sizeofframe`'s share drops:
  - from 116 to 7 M at the theorem site;
  - from 310 to 18–20 M at sets:195;
  - from 41 to 2 M at the prose site;
  - from 83 to 4–5 M at the probability site.

  Everything else is unchanged to within 1 M.
- **What is left.** A replay costs the K2/K4 snapshot and the key comparison, plus the
  operation log: a few thousand `eq_define`s per call.
- **The first window at a place** records every call and pays about +30 % on each, so the window
  costs 11–32 % more. Every later keystroke there replays.
- **Statistics (`b5on-stats.txt`).** Totals over both runs' host lifetimes: 9,197 offers, 2,834
  recordings, 2,833 committed. Offers outside an edit's window are neither recorded nor (with no
  entry yet) replayed. The `hits` line in that file lags, because it is written every 32 events.

## Correctness so far

- **Engine tests** (`crates/flashtex-engine/tests/boxmemo.rs`, 4 of 4 pass), each run with BOX-MEMO
  off, on and verify:
  - the logs are byte-identical across the three modes;
  - calls are replayed;
  - the key falls back on a changed argument, register, macro, parameter, or a name that is now
    defined;
  - output, a non-void box read from outside and the outer list are never replayed;
  - void box reads are part of the key.
- **CLI test documents** (framed.sty with `\fbox`, and with `\fcolorbox`): the PDF is byte-identical
  to pdflatex's with BOX-MEMO off, on and verify.
- **Host typing, verify mode** (`bmver` run, 2 sites): 22 replays checked against the normal path,
  0 differences.
- **The hosted soundness sweeps** with BOX-MEMO on by default (the sweep-only branch
  `agent/kabir-claude/box-memo-sweep`) are in run 37571698072. The result is in the PR.
