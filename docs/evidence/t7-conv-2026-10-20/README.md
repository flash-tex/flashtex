# T7-CONV: side-table entries compared as places (2026-10-20)

Lane **T7-CONV** (Commander kabir-claude), after T7-FIX's diagnosis in #1850. All runs are on the
NixOS PC (`Linux 6.18.45 x86_64`, 16 threads), under `ftx-run`, one host at a time. Every run is
**non-reference**, because t7.py records no power source on Linux, so the latency verdicts are
for comparison only. Convergence and re-typeset pages do not depend on load.

## The miss: plain-10 letter@start never converged

amsmath's `\every@math@size` sets `\strutbox@` (`\box54`) globally at the document's first inline
math, which is on the edited line. The box is live in every later state. The host retires an
edited line's span (`displaylist::move_lines`) and gives the new run's material a new one. So
the convergence test compared `0x4cd014c` with `0x567014c`, the same column of the same line
under two span ids, and failed at every page after the edit. Before this, the test compared
span ids, not places.

**The fix:** `displaylist::same_place` compares two entries as (file, line, column), and both
side-table checks in `iso.rs` use it. Spans on one line stay on one line (`move_spans_of` moves
or retires every span of a line alike), so either entry, shipped now or after later edits, names
the same place. Without the writer, only equal entries are equal. The XeTeX engine already
compared places in the same way (`flashtex-xetex` `conv.rs`, `Tables::place`).

## A/B: main's convergence test against the fix (`ab-main/`, `ab-fix/`)

`t7.py --docs plain-10,full-10 --phases letter@start,letter@middle --keys 20 --order fixed`.
`ab-main`'s engine is this branch's first commit with `iso.rs` reverted to main `fc9fcf5a5`, so
its `HEAD` line names the branch commit. Medians over keys 1–19 come from the raw `DONE` stages:

| row | converged | re-typeset pages | edited page, instr / CPU | whole compile, instr / CPU |
|---|---|---|---|---|
| plain-10 letter@start, main | 0/19 | 10 | 58.6 M / 5.8 ms | 811.6 M / 60.8 ms |
| plain-10 letter@start, fix | 19/19 | 2 | 58.5 M / 6.0 ms | 325.8 M / 26.5 ms |
| full-10 letter@start, main | 19/19 | 2 | 166.8 M / 13.5 ms | 650.8 M / 49.6 ms |
| full-10 letter@start, fix | 19/19 | 2 | 166.8 M / 13.5 ms | 651.0 M / 49.8 ms |

The other rows are unchanged within noise.

**The first keystrokes after an open stay slower; the fix does not remove that.** In both
engines, plain-10 letter@start keys 0–6 take about 10 ms of thread CPU and the later keys about
5.8 ms. full-10 letter@start shows the same with keys 0–4 at about 19 ms and the later keys at
13.3 ms. Instructions rise only 8 % (63.4 M against 58.5 M), while CPU rises 75 %. With
`--gap-ms 2000` the slow phase covers only keys 0–1. So it depends on the time since the open,
not on how many keys are typed or whether they converge. Its cause is not isolated here.

## full-10 letter@start: 167 M instructions to the edited page (letter@middle: 99 M)

dl3-keys types at byte 577, right after the document's first inline math (536–575). The restart
is 347 bytes back, at the paragraph start, so every keystroke re-runs the first math's setup and
the rest of page 0. Callgrind ran over 6 keystrokes with the whole compile instrumented, against
letter@middle (`callgrind_control` toggled after a warm-up). Per keystroke:

- `new_font`: 29 calls and 26 `read_font_info` (TFM loads). letter@middle has none.
- `search_string`: +13.6 M instructions. This is tex.ch's string recycling in `end_name`, a
  linear scan of the pool, called about 44 times.
- `macro_call`: +19 M, and `get_next`/`expand`/`intr_next`: +13 M.
- `apply_under` and `prepare_go_on`: +20 M, for restore preparation from a page-0 checkpoint.

A variant with `\sbox0{$x^2$}` right after `\begin{document}` (`full-10m`) moves the first math
out of the re-run. That gives 143.0 M against 166.8 M instructions (−24 M), 11.0 against 13.3 ms
CPU, and 12 `new_font` and 9 TFM loads per keystroke instead of 29 and 26. The first-math
setup is therefore about 24 M of the 68 M excess. Much of the rest is first-use setup on page 0
(footnote-size fonts and the first shipout's writes); that part is belief, not measured.

Candidate fixes, none of them in this lane:
- a mid-paragraph restart point after an inline math, like PREAMBLE-MIDLINE's mid-line
  checkpoint (about −24 M here);
- a faithful indexed `search_string` (at most about −13 M; it also speeds every `\input`);
- a parsed-TFM cache (about −7 M).

## Full T7 run and the new baseline (`full/`)

`t7.py --engine t7conv --wait-load 5 --write-baseline ...` over all ten documents (engine
`5f460dd63`). `tools/incr-bench/t7-baseline.json` is that run's `--write-baseline` output,
unedited. No row missed on convergence or pages against the old baseline. Every in-body row now
converges (rate 1.0), including the rows the old baseline held at 0.0 (for example full-1000
newline@middle and split@middle, 501 pages there). The 56 latency misses in `full/table.md` come
from a shared, loaded PC (load1 1–4, off-CPU p95 up to 52 ms on typing rows). They are not this
change's.
