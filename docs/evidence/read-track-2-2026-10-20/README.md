# READ-TRACK-2: the convergence test's walk, measured (2026-10-20)

Lane READ-TRACK-2 follows [READ-TRACK](../read-track-2026-10-20/README.md) (#1861). It makes
three changes:
- the Unicode convergence test of an equal state no longer walks the structure;
- the walk pairs equal tokens without a compare (both engines);
- the kept pages after a Unicode jump reuse their frames.

The rule itself is in [READ-TRACK.md §12](../../design/engine-v2/READ-TRACK.md).

Where and how it was measured:
- **Machine:** the NixOS PC. Measurements ran one host at a time.
- **Builds:** release builds, one tree, the same toolchain. "main" is main `99a06dc01` (#1861
  merged). "Branch" is `agent/kabir-claude/read-track-2` at `21a7efc5e`; the later commits
  change only formatting, tests and docs.
- **TeX Live:** TeX Live 2026.
- **Counts:** callgrind (valgrind 3.27.1); M is a million instructions.
- **Classic sessions:** pinned checkpoints, as in READ-TRACK's evidence:
  `FLASHTEX_TIMED_S=1000000000 FLASHTEX_SEGMENT_S=off FLASHTEX_PREAMBLE_LINE_S=off
  FLASHTEX_NO_PREPARE=1 FLASHTEX_HOST_KEEP_WARM_MS=0`.
- **"before #1861":** the figures from READ-TRACK's evidence (main `1aa1018ef`), as context.

The scripts are in [`scripts/`](scripts/). They use READ-TRACK's (`~/rt`: `rt_end.py`, `cg.sh`,
`docs/`).
- `build.sh TAG` builds both engines and keeps the binaries.
- `measure.sh TAG [cold|unicode|classic|c12|all]` measures the rows below.
- `prof.sh TAG` profiles b4 page by page.
- `cgdiff.sh` subtracts two callgrind profiles by function.
- `tests.sh TAG` runs both soundness tests and `rt_end.py --verify`.
- `gate.sh` runs `scripts/gate.sh quick` with CI's toolchain.

## Unicode: the M3 row

`tools/incr-bench/unicode.py --keys 3 --callgrind`, instructions to the edited page, then for
the whole compile (M):

| row | main k1 / k2 / k3 | branch k1 / k2 / k3 |
|---|---|---|
| b4 end, to the edited page | 128.7 / **322.6** / 322.7 | 128.7 / **194.9** / 195.1 |
| b4 end, the whole compile | 757.7 / 528.8 / 528.2 | 757.6 / 401.2 / 401.6 |
| b4 EDITPOINT, edited page | 131.2 / 82.0 / 82.0 | 131.3 / 82.2 / 82.1 |
| b4 EDITPOINT, the whole compile | 691.9 / 738.8 / 751.1 | 582.2 / 591.8 / 596.0 |
| b1-article end | 67.1 / 67.1 / 67.1 | 66.5 / 66.6 / 66.6 |
| b2-fontspec-named end | 18.6 / 18.6 / 18.6 | 18.5 / 18.5 / 18.6 |
| b3-unicode-math end | 38.1 / 38.1 / 38.1 | 38.1 / 38.1 / 38.1 |

The M3 row takes the median of the three keystrokes. It goes from **322.6 M to 194.9 M**
(−40 %). Before #1861 it was 502.0 M. Keystroke 1 is not covered (READ-TRACK.md §5.3).

Cold compiles (M, the first page / the whole compile), main against the branch:

| document | main | branch |
|---|---|---|
| b1 | 1101.7 / 1785.1 | 1100.5 / 1783.6 |
| b2 | 3275.8 / 5618.6 | 3275.2 / 5617.5 |
| b3 | 7413.8 / 14450.7 | 7413.0 / 14449.8 |
| b4 | 2932.5 / 7898.1 | 2932.2 / 7898.6 |

**Where b4's keystroke 2 goes now** (`prof.sh`, the dump after each page sent; 195.4 M with
`FLASHTEX_UNICODE_CONV_DEBUG` on):

| part | branch | main (READ-TRACK's table) |
|---|---:|---:|
| the restore, pages 1–11 sent again | 18.0 | 18 |
| page 12 typeset | 61.7 | 62 |
| the test, the jump to page 18, pages 13–18 built again and sent | 61.6 | 181 |
| page 19 typeset live | 54.1 | 54 |

In main, the third row was the walk (107 M), the diff and checkpoint (19 M), the jump (15 M) and
the kept pages (40 M). On the branch, page 13's dump holds the test (the diff 12.3 M, the
checkpoint 9.4 M, `same_outside` 0.6 M), the jump and page 13 built again: 39.4 M. Pages 14–18
cost 4.4 M each.

- **The walk is gone from this row.** No word is left for the walk here once the dormant and
  dead words are set aside. The test then compares only what the walk would read outside the
  word space: every glyph-info handle and every side-table entry. That costs 0.6 M, where the
  walk cost 107 M.
- **A kept page costs 4.4 M instead of 7.9 M.** Of the 7.9 M, 3.6 M went on its frame (the
  content hash, SHA-256, 2.8 M). The frame is now the old run's when the page built again has
  the old display list and the same resource keys. Building the page again from its XDV
  (3.4 M) stays: it makes the document's fonts, forms and annotations.

## Classic

**Instructions per keystroke** (M, the whole host, pinned checkpoints; each figure is the
difference between sessions that settle and then take 0, 1, 2 or 3 keystrokes):

| session | before #1861 | main | branch | branch vs main | branch vs before #1861 |
|---|---:|---:|---:|---:|---:|
| c1-lipsum end, k1 / k2 / k3 | 168.7 / 166.2 / 165.8 | 179.5 / 229.9 / 120.1 | 179.7 / 183.4 / 119.4 | | |
| c1-lipsum end, 3 keys | 500.7 | 529.5 | **482.4** | −8.9 % | −3.6 % |
| c2-ifnextchar end, k1 / k2 / k3 | 203.7 / 201.3 / 200.3 | 212.9 / 166.2 / 82.3 | 212.8 / 133.9 / 82.3 | | |
| c2-ifnextchar end, 3 keys | 605.3 | 461.4 | **429.0** | −7.0 % | −29.1 % |
| c1-lipsum `{Thr`, 3 keys | 1,273.0 | 1,279.1 | **1,245.4** | −2.6 % | −2.2 % |
| plain-100 `Section 13`, 3 keys | 4,315.5 | 4,332.3 | **4,271.7** | −1.4 % | −1.0 % |
| full-100 `Section 13`, 3 keys | 29,020.1 | 29,203.4 | **28,380.3** | −2.8 % | −2.2 % |
| plain-100, the settle | 4,666.4 | 4,681.0 | 4,680.9 | −0.00 % | +0.31 % |
| full-100, the settle | 16,095.8 | 16,182.3 | 16,181.9 | −0.00 % | +0.54 % |

The settles' +0.3 % and +0.5 % against before #1861 are #1861's watch in `pop_input`. This lane
does not change that cost.

- **c1's keystroke 2 now costs 183 M instead of 230 M.** The test past the dormant line still
  walks the whole structure: BOX-MEMO's replays put words of memory in other places (READ-TRACK's
  evidence). Measured with `cgdiff.sh` (keystroke 2's session minus keystroke 1's), the test
  (`same_words`) went from 119 M to 73 M:
  - The walk (`Iso::check`) went from 92 M to 54 M. Its token lists (`chain_from` and `pair`,
    self cost) went from 69 M to 38 M: a token whose word is equal at the same address in both
    states is now paired without a compare or a cover.
  - The old run's free lists are now read through the chunk table, not with a map lookup per
    word: `free_cells` went from 17.4 M to 7.8 M.
- **Over three keystrokes c1 costs 3.6 % less than before #1861**, where #1861 left it 5.8 %
  more. Keystroke 1 still costs 11 M more: one test past the line finds that a meaning differs.
  The in-body sessions (T7's kind) now cost less than before #1861.
- **Pages re-run per keystroke**, from the host's `rerun_pages`, are the same as main's in
  every session: c1 end 8/2/2/2, c2 end 13/2/2/2, `{Thr` 6/17/17/17, plain-100 102/2/53,
  full-100 110/110/110.

**Cold compiles** (`cg.sh`, the whole compile; the PDFs are identical to main's):

| document | main | branch | change |
|---|---:|---:|---:|
| plain-100 | 2,284,037,785 | 2,284,201,827 | +0.007 % |
| full-100 | 8,653,800,890 | 8,653,851,273 | +0.001 % |
| c1-lipsum | 930,670,466 | 930,829,263 | +0.017 % |
| c2-ifnextchar | 611,012,855 | 611,176,382 | +0.027 % |

No engine path of a cold compile changed. The differences are code layout.

## Soundness

`tests.sh rt2c` on the PC, TeX Live 2026:
- `tests/incremental.rs::a_dormant_line_is_kept_until_it_is_read` passed (Classic).
- `tests/host.rs::a_dormant_line_is_kept_until_it_is_read` passed (Unicode).
- `rt_end.py --verify` gave `BAD 0` for c1-lipsum end (4 keys), c2-ifnextchar end (4 keys),
  c1-lipsum `{Thr` (3 keys) and plain-100 `Section 13` (3 keys). In each run, each compile's
  PDF, log and `.aux` equal a scratch compile's.
- Unicode b4 end with `FLASHTEX_UNICODE_CONV_DEBUG`: the claim holds at page 12 for keystrokes
  2 and 3 (the host's own check against the old run's pages).

**Sweeps** (GitHub-hosted, `ref=agent/kabir-claude/read-track-2` at `76072a9c3`): `sweeps.yml`
run 39529723930 and `sweeps-unicode.yml` run 39529727186 both passed.

**Suites** (`gate.sh`: `scripts/gate.sh quick` at `76072a9c3`, rustup stable 1.98.1, a fresh
target directory, TeX Live 2026):
- passed: rustfmt, clippy for both crates, trip and etrip;
- tests: every test passed except `fontmap_cache::a_cached_parse_writes_what_a_parse_writes`. It
  fails on main on this PC too (READ-TRACK's evidence).
- the new unit tests `dlsource::tests::side_diffs_finds_the_entries_that_may_differ` and
  `state::tests::same_glyphs_compares_every_glyph_info_handle` passed;
- `tests/incremental.rs` (79 tests) and the XeTeX `tests/host.rs` (20 tests) passed.
