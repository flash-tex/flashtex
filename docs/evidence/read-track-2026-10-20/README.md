# READ-TRACK: a dormant line, measured (2026-10-20)

This folder holds the evidence for lane READ-TRACK's rule,
[`docs/design/engine-v2/READ-TRACK.md`](../../design/engine-v2/READ-TRACK.md).
The rule lets a run jump past a line that the engine has read but not yet used.

Where and how it was measured:
- **Machine:** the NixOS PC (x86-64, loaded: other lanes' runs and the CI runners share it).
- **Builds:** release builds. "main" is main `4972cd46c` (the Unicode host) or `1aa1018ef`
  (the Classic binaries; its engine code is the same). "Branch" is
  `agent/kabir-claude/read-track-dormant` at `ae61798ba` (the Unicode host at `ef2409fff`,
  whose Unicode code is the same; the later commits format the code and add a test).
- **TeX Live:** TeX Live 2026 at `~/texlive/2026`.
- **Counts:** every instruction count is callgrind's (valgrind 3.27.1). M is a million
  instructions.
- **Times:** wall times are given only as context.

**A host session's count depends on the clock unless the checkpoints are pinned.** The Classic
host takes timed, segment and preamble-line checkpoints by elapsed time, and under callgrind
the time depends on the PC's load: two runs of main's full-100 settle differed by 1.3 %, more
than the effects measured here. Every
Classic session below therefore runs with the checkpoints taken at shipouts only:
`FLASHTEX_TIMED_S=1000000000 FLASHTEX_SEGMENT_S=off FLASHTEX_PREAMBLE_LINE_S=off
FLASHTEX_NO_PREPARE=1 FLASHTEX_HOST_KEEP_WARM_MS=0`. Three runs of one session then agreed to
0.002 %. A binary's first session also builds its format image once (+1.3 G); a warm-up session
runs first.

The scripts are in [`scripts/`](scripts/) and use the PC's paths: `~/rt`, and valgrind in the Nix
store.
- `rt_end.py` drives Classic `flashtex-host iserve`: it settles the document, types keystrokes,
  and verifies each compile against scratch runs. The checks are incr_bench.py's: the PDF, the
  log less its accounting, and the `.aux`.
- `measure6.sh` runs the final Classic numbers: cold compiles (`cg.sh`), then the sessions.
- `ucg.sh` runs the Unicode M3 latency rows under callgrind, main against the branch.
- `prof.sh` profiles b4's keystrokes with each page's dump kept; `calls.py` counts calls in a
  callgrind file.
- `gen_dormant.py` writes the soundness test's documents; `mutate.sh` disables each rule in
  turn.
- `cg.sh` and `cgfn.sh` count and profile command-line compiles.
- `exp_eqtb_barrier.py` and `exp_probe.py` are the measurement-only builds of
  READ-TRACK.md §3 and §4. They were never committed.

## Unicode: the M3 row

`ucg.sh`: `tools/incr-bench/unicode.py --keys 3 --callgrind`, the edited page's instructions and
the whole compile's (M), main against the branch:

| row | main k1 / k2 / k3 | branch k1 / k2 / k3 |
|---|---|---|
| b4 end, to the edited page | 128.7 / **502.0** / 503.3 | 128.7 / **322.6** / 322.6 |
| b4 end, the whole compile | 869.7 / 709.3 / 711.2 | 757.5 / 528.8 / 528.1 |
| b4 EDITPOINT, edited page | 131.2 / 82.2 / 82.2 | 131.3 / 82.2 / 82.2 |
| b1-article end | 66.5 / 66.6 / 66.6 | 67.1 / 67.1 / 67.1 |
| b2-fontspec-named end | 18.6 / 18.6 / 18.6 | 18.6 / 18.6 / 18.6 |
| b3-unicode-math end | 38.1 / 38.1 / 38.1 | 38.1 / 38.1 / 38.1 |

The M3 row is the median of the three keystrokes: **502.0 M → 322.6 M** (−36 %). Keystroke 1 is
not covered (READ-TRACK.md §5.3). b1's +0.6 M (+0.9 %) is the watch on the last page's
re-run: b1's keystrokes run no convergence test.

Cold compiles (M, the first page / the whole compile), main against the branch: b1 1100.7 /
1783.6 against 1101.7 / 1785.1 (+0.08 %); b2 3275.7 / 5618.3 against 3275.8 / 5618.6; b3
7413.7 / 14450.7 against 7413.8 / 14450.9; b4 2932.4 / 7897.4 against 2932.5 / 7897.7.

**Where b4's keystroke 2 goes** (`prof.sh`, the dumps of each page sent; 323 M to the edited
page with `FLASHTEX_UNICODE_CONV_DEBUG` on):

| part | M |
|---|---:|
| the restore and the frames before the restart point | 18 |
| typesetting from the restart point to page 12 | 62 |
| the test at page 12: the structural walk | 107 |
| the test: the diff, the checkpoint | 19 |
| the jump to page 18 (`set_from` now 1 M; it was 12 M, 53 M at keystroke 3) | 15 |
| pages 13–17 built again and sent (`replay_page`, their content hash) | 40 |
| page 19 typeset live, to the edited page | 54 |

The walk runs on every Unicode convergence test, also when every word is equal: it also
compares the native glyph handles and the source map's side table, which live outside the word
space. Here, once the dormant and the dead words are set aside, no word is left for it. That
is the next lever for this row (about 100 M), and it is a change to the general Unicode test,
not to this rule.

**Soundness.** `tests/host.rs::a_dormant_line_is_kept_until_it_is_read` passed on the PC
(TeX Live 2026; `end` and `read`: each compile's PDF, `.aux` and log equal a process of its
own's, the log reports the jump, no `UNSOUND`). `sweeps-unicode.yml` on the branch: see the PR.

## Classic

The documents are `scripts/c1-lipsum.tex` (b4 without fontspec) and `scripts/c2-ifnextchar.tex`
(a `\newcommand` with an optional argument, alone before `\end{document}`). Each keystroke types
`x` at the start of the `\end{document}` line, or (`{Thr`) inside `\section{Three}` of c1, the
line `\lipsum[16-35]` peeks at.

**Pages re-run per keystroke** (the host's `rerun_pages`; `rt_end.py --verify`, `BAD 0` in every
run):

| document | main k1 / k2 / k3 / k4 | branch k1 / k2 / k3 / k4 |
|---|---|---|
| c1-lipsum end (19 pages) | 8 / 8 / 8 / 8 | 8 / 2 / 2 / 2 |
| c2-ifnextchar end (22 pages) | 13 / 13 / 13 / 13 | 13 / 2 / 2 / 2 |
| c1-lipsum `{Thr` | 6 / 17 / 17 / 17 | 6 / 17 / 17 / 17 |

**Instructions per keystroke** (M, the whole host, pinned checkpoints; each figure is the
difference between sessions that settle and then take 0, 1, 2 or 3 keystrokes):

| session | main k1 / k2 / k3 (sum) | branch k1 / k2 / k3 (sum) | change |
|---|---|---|---:|
| c1-lipsum end | 168.7 / 166.2 / 165.8 (500.7) | 179.4 / 230.1 / 120.2 (529.6) | +5.8 % |
| c2-ifnextchar end | 203.7 / 201.3 / 200.3 (605.3) | 212.9 / 166.7 / 81.2 (460.8) | −23.9 % |
| c1-lipsum `{Thr`, 3 keys | 1,273.0 | 1,278.7 | +0.45 % |
| plain-100 `Section 13`, 3 keys | 4,315.5 | 4,330.9 | +0.36 % |
| full-100 `Section 13`, 3 keys | 29,020.1 | 29,203.6 | +0.63 % |
| plain-100, the settle | 4,666.4 | 4,681.2 | +0.32 % |
| full-100, the settle | 16,095.8 | 16,182.5 | +0.54 % |

- **Keystroke 1** costs 5–6 % more: one test past the line, which finds that a meaning
  differs (`\l_peek_token`), and the run then tests no more of them.
- **Keystroke 2 on c1** costs 38 % more. The previous run recorded BOX-MEMO entries, and the
  replays put 82 words of memory in other places, so the test walks the whole structure,
  including lipsum's text in its macros (about 100 M). On c2 the same walk pays for itself.
- **Keystroke 3 onward**, the typing case, costs 27 % less on c1 and 60 % less on c2.
- **In-body edits** (the T7 kind) cost +0.36 % and +0.63 %: the watch, below.

**The hot path** (command-line cold compiles, callgrind, the same session; the PDFs are
identical):

| document | main | branch | change |
|---|---:|---:|---:|
| plain-100 | 2,277,253,693 | 2,284,052,725 | +0.30 % |
| full-100 | 8,605,041,252 | 8,653,796,709 | +0.57 % |
| c1-lipsum | 929,539,900 | 930,669,711 | +0.12 % |
| c2-ifnextchar | 611,837,236 | 610,990,415 | −0.14 % |

- **The watch costs about +0.5 %.** It sits in the functions where `pop_input` is inlined:
  `get_next_slow`, `expand` and `macro_call` (`cgfn.sh` diffs).
- **The format loader.** Before the change to `system::get_word`, the same code measured
  +1.11 %, +0.78 % and +2.15 %. The extra was not in the engine. Adding code had moved the
  crate's codegen units, so `get_word` lost its inlining in `load_fmt_file`, and a drop glue
  call came in (+16 to +52 M). `get_word` now reads the word straight from the reader's
  buffer. That makes it 5 M instructions cheaper than on main, which hides part of the watch's
  cost on c1 and c2.
- **Rejected, for comparison:** a host call in place of the inline counter costs the same as
  the inline counter. The web2rust read barrier on `eqtb` (READ-TRACK.md §4) costs +2.02 % to
  +14.36 %.

**Phase 2's corrections, each measured:**
- **`set_from` in one pass** for all the words (it was one call per word): b4's jump, 12 M → 1 M
  at keystroke 2 and 53 M → 1 M at keystroke 3 (Unicode, 328 → 323 M and 369 → 323 M).
- **A failed exemption turns the rule off for the run.** In an in-body edit the innermost file
  level is often below token lists, so the rule let tests past the edited file's lookahead
  that then failed on the words: full-100, 304 diffs against main's 202 over three keystrokes.
- **The worth rule** (READ-TRACK.md §5.1). Without it, c1's `{Thr` keystroke 1 jumped from page
  4 to page 6 and typeset the 13 pages after it, where main converges at page 8: +51 M (+12 %).
  With it, keystroke 1 re-runs main's 6 pages. Its first form tested after the diff (+1.5 %
  over three keys); the test is now made before it (+0.45 %).
- **Rejected: no walk past a dormant line.** It made c1's keystroke 2 cost main's, but c2's
  keystroke 2 then re-ran 13 pages instead of 2.

**Which rule each case needs** (`mutate.sh` on the final code: one rule disabled in turn, and
`a_dormant_line_is_kept_until_it_is_read` run):

| disabled | the first case that fails |
|---|---|
| the return count (`dorm_fired`) | `end`, keystroke 2: the PDF differs |
| the print count | `show`, keystroke 2: the log differs |
| the dormant words written into the kept states | `end`, keystroke 2: the PDF differs |
| the open count | none (READ-TRACK.md §5.2) |
| the stream record | none (READ-TRACK.md §5.2) |

The `late` case restarts at the kept, corrected checkpoint: an edit of the file the line opens
gives `restart_pages` = `rerun_from` = 9. The `mid` case (Phase 2) asserts no jump.

**Suites** (PC, release, the branch): `scripts/gate.sh quick` with CI's toolchain (rustup
stable 1.98.1) and TeX Live 2026: rustfmt, clippy for both crates, web2rust drift, trip and
etrip pass; both `a_dormant_line_is_kept_until_it_is_read` tests pass. Two tests fail, on main
too on this PC: `fontmap_cache::a_cached_parse_writes_what_a_parse_writes` (fails on main
`4972cd46c` the same way) and `host::cancels_are_never_lost` (fails 1 run in 3 on main and on
the branch).
