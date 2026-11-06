# P6-HYPEROPT: profile-first speedups of the v3 engine (2026-10-04)

Lane **P6-HYPEROPT** (mac-claude-a, mac-m1max-a; owner 2026-10-04: "hyperoptimize the v3 engine").
DESIGN.md §1.2 (latency targets, decision 1), §5.2-§5.3 (checkpoints, convergence), §5.6 (L6),
§5.7, §12 P6. Every change keeps the output byte-identical; each PR states its gates.

**Machine.** M1 Max (10 cores, 32 GB), macOS Low Power Mode on, on AC, shared with other sessions:
**load1 10-200 during this lane** (most of it 40-190). So **no wall time here is a reference**. The
measure is the **engine thread's instruction count** (and cycles): on macOS the kernel keeps
per-thread fixed counters (`thread_selfcounts`), which this lane reports in `DONE.stages`
(PR "counters"); they repeat within ±0.5 % across keystrokes and do not move with load. Cold
compiles use `/usr/bin/time -l` (instructions retired, cycles elapsed). Wall times are quoted
only where marked, as "loaded".

**Profiles.** Instruments' *CPU Profiler* template (`xctrace record`; PMU-cycle-weighted
samples, on-CPU only), summarised by `scripts/xcprof.py` (self and inclusive cycle share, with
`--under`/`--not` stack filters) and `scripts/xcaddr.py` (cycles per instruction address).
The engine's own macro profiler (`FLASHTEX_MACRO_PROFILE`) for TeX-level costs.

## Hotspots (VERIFIED, ranked by share of the workload's engine-thread cycles)

### Cold compile (one settled pass from the format; `raw/profile-cold-*.txt`)

| rank | where | full-100 | beamer long-deck (118 slides) | note |
|---|---|---|---|---|
| 1 | `get_next` (token reader), self | 20.8 % | 31.1 % | prologue/epilogue and the cs/char branch; `raw/get_next.hist` |
| 2 | `macro_call` (argument matching), self | 12.3 % | 12.2 % | |
| 3 | `end_token_list` + `begin_token_list` + `back_input` | 8.3 % | 10.3 % | |
| 4 | kpathsea `init_db` (ls-R hash, `hash_insert_normalized`) | 10.0 % incl. | 2.9 % | per process start; pdfTeX pays it too |
| 5 | `id_lookup` | 4.2 % | 4.8 % | |
| 6 | `get_avail`/`get_node`/`free_node`/`delete_token_ref` | 7.7 % | 8.2 % | |
| 7 | font map (`fm_read_info`, pdftex.map) | 5.1 % incl. | 1.6 % | first use of a font |
| 8 | PDF page out (`pdf_ship_out`) | 10.3 % incl. | 2.7 % | zlib 2-3 % |

The engine against pdfTeX on 13 arXiv papers (every 12th of `arxiv-2025-01`; `raw/cold-arxiv-base.jsonl`):
**0.887× pdfTeX's cycles** in total (cycles measured at load 26-68 in macOS Low Power Mode: not a reference) (0.76-0.96× per paper), 0.86× its instructions, and
**every PDF byte-identical to pdfTeX's**.

Beamer, by TeX macro (`raw/macro-profile-long-deck.tsv`, inclusive): the output routine
(`\@outputpage`) is 59 % of the deck, the headline alone 38 %; xcolor's `\colorlet`
(`\XC@col@rlet`, 69,961 calls, 593 a slide) is 47 %, reached from `\usebeamercolor` (5,178 calls).

### Keystroke (host, `flashtex-host --socket` + `dl3-keys`, letter at the middle unless named)

Engine-thread instructions per keystroke on main (+ the counters), p50:

| document | whole compile | edited page | convergence tests | pages re-typeset | where the rest goes |
|---|---:|---:|---:|---:|---|
| full-100 | 698 M | 103 M (restore 12 M, display list ~15 M) | **148 M** | 2 | the test's structural walk (`Iso`, 70 %) and its chunk rewind (20 %); the convergence jump; the fonts re-subset at every keystroke (`writet1`, ~7 % of the compile) |
| plain-100 | **2,169 M** | 56 M | 88 M | **33** | a stale `link(temp_head)` failed every test for 16+ pages |
| full-100, sentence | **4,164 M** | 104 M | 67 M | **33** | a stale `link(backup_head)` |
| full-1000 | 3,760 M | 123 M | 53 M | **33** | the PDF object-stream buffer differs until it is flushed |
| beamer long-deck, 1-slide frame | 1,248 M | **420 M** | 40 M | 2 | one slide of beamer's templates (xcolor) |
| beamer, 3rd slide of a 3-slide frame | 4,578 M | 475 M (1st slide) | 145 M | 9 | three slides of TeX before the watched one |
| *Infinite Descent* (592 pp.), page 261 | **179,756 M** | 130 M (restore 42 M) | 0.4 M | **319** | never converges: the old run reads a barrier later (imakeidx's `\write18` makeindex at `\printindex`) |

On the edited page's path (full-100, cycles, tests excluded; `raw/profile-host-full-100-edited-path.txt`):
TeX ~60 %, the display list's content-stream interpreter 18-20 % (per glyph: two 128-bit matrix
products, a SipHash of the matrix, three thread-local hashed look-ups, decimal-to-double loops),
checkpoints 8 %, the restore 4.5 %.

## Wins (one PR each; before = main + the counters, after = + the change; instructions, p50)

| PR | change | measured |
|---|---|---|
| #1494 counters | `DONE.stages` instruction counts; `dl3-keys --edit` | tooling; no output change |
| #1495 iso | a failing convergence test walks the page builder's lists first; static dispatch; a fast hash | full-100: tests 148 → 47 M, compile 699 → 579 M (−17 %); beamer 3-slide: tests 145 → 112 M |
| #1496 dl-glyphs | the glyph loop computes per string what is per string | edited page: plain-100 57.4 → 49.8 M (−13 %), full-100 102.4 → 96.8 M (−5.5 %); display lists identical on 32 documents |
| #1497 scratch-heads | `link(temp_head)`, `link(backup_head)` dead between commands | plain-100 2,173 → 421 M (−81 %, 33 → 2 pages); plain-300 1,199 → 533 M (9 → 2); full-100 sentence 4,164 → 627 M (−85 %, 33 → 2) |
| #1498 barrier-rerun | a barrier blocks reuse only past where it is read; `name_key` without building names | *Infinite Descent*: 179.8 G → 16.3 G (−91 %), 319 → 49 pages, converged 0/7 → 7/7, engine CPU 24 → ~2.3 s per keystroke (thread CPU at load 28-180 in Low Power Mode: not a reference; the instruction counts are) (`raw/infdesc-*.jsonl`); a 24-page imakeidx book 855 → 354 M, 19 → 3 pages |
| #1499 get-next (draft) | `get_next` fast path inlined (throughput.ch [3], web2rust `--inline`) | instructions −2.8 % over 14 documents (beamer −5.9 %); cycles not resolved on this host |

Combined (`raw/ab-all.out`, columns in `raw/ab-all.out.md`; `all1` = #1494-#1497 without `backup_head`): plain-100 −82 %, full-100
−20 %, full-1000 −8 %, plain-1000 −6 %, beamer −3 to −4 % per keystroke; the edited page −6 to −14 %
on plain/full documents. Soundness sweep A converged in 367 of 2,112 compiles against main's 178.

## Soundness (VERIFIED, mac-m1max-a)

`tools/incr-bench/soundness.py` (each compile and each revert equal, byte for byte, to from-scratch
runs: PDF, log, aux, out, toc, terminal), on the lane's **combined build** (`all3` = main + the
counters, iso, dl-glyphs, scratch-heads and barrier-rerun PRs), with `scripts/sweeps.sh`:

| sweep | documents | compiles | mismatches | converged |
|---|---:|---:|---:|---:|
| A (replace/insert/delete, 12 trials + reverts; every fixture and probe + plain-120 + full-100) | 88 | 2,112 | **0** | **367** (main: 178, same sweep) |
| C (structural: sentence, section, label, ref, cite, footnote, unlabel, unsection; 6 trials; + refs-30/120, full-100) | 89 | 580 | **0** | 26 |
| D (interleaved, preempted; 6 trials) | 89 | 819 (708 verified, 111 interrupted) | **0** | 99 |
| imakeidx book (`raw/idx.tex`, two indices, makeindex via restricted `\write18`): A 16 + A 40 (with sentences) + C 10 + D 8 trials | 1 | 32 + 80 + 14 + 17 | **0** | 29 + 70 + 3 + 12 |

Sweep C's one error is the harness's known "0 trials" on `beamer-sans-operators-professional`
(no applicable structural edit; also in #1439). The same sweeps on `all2` (without the barrier PR)
gave the same counts (A 2,112/0/367, C 580/0/26, D 819/0/101).

Also on the combined build: `cargo test --release` incremental (29 tests, the lane's three new or
changed ones included), host_incremental, display_list_host, intrinsics, the engine's lib tests
and the display-list crate's: all pass. trip and etrip pass. Display lists (`scripts/dlsame.py`:
a compile through the host, `dl3-dump --canonical`, both engines in one work directory): main and
the dl-glyphs PR **identical on 32 documents** (every fixture with a `main.tex`, long-deck,
full-100; `raw/dlsame-all.txt`).

## Next candidates (measured basis; not done here)

1. **`get_next` fast path** (draft PR: throughput.ch [3], web2rust `--inline`): instructions
   −2.9 % (full-100), −5.9 % (beamer), −2.8 % over 14 documents (`raw/cold-gn2*.jsonl`), every
   PDF and the format identical, lockstep 1,464/1,464, etrip passes. Cycles are not resolved on
   this host (−4.7 to +4.9 % per paper, −0.1 % in all); a hand-written version measured −1.6 to
   −5.4 % (`raw/cold-exp.jsonl`). Needs a quiet host before it lands.
2. **Converge again after a barrier**: *Infinite Descent* still re-typesets the back matter
   from the first `\printindex` (47 of 49 pages) on every keystroke.
3. **The PDF object-stream buffer** (`pdf_os_*`, `pdf_ptr`): full-1000 re-typesets 33 pages per
   letter until the object stream is flushed (DESIGN §5.3's "PDF writer's buffers" class).
4. **The convergence test's rewind of the old future** (`rewound_until` over every later log):
   0.57 G instructions per test on *Infinite Descent*; a per-chunk index of the branch's logs.
5. **Beamer's slide**: 420 M instructions, half of it xcolor's `\colorlet` from
   `\usebeamercolor`: guarded intrinsics for macros with arguments (D9) would replay it.
6. **Font subsetting at every keystroke** (~7 % of a full-100 keystroke): cache by font and
   character set.
