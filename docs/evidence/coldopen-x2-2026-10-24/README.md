# COLD-OPEN: the first open of *Infinite Descent ×2* (2026-10-24)

Lane **COLD-OPEN** (kabir-claude-coldopen under kabir-claude). The owner reported that the first
compile of the live preview of *Infinite Descent ×2* (1,152 pages) takes far too long. DESIGN.md
§1.2 ("Opening": the visible page ≤ 1 s after open, then the rest streams in), §5.1, §5.2, §5.6.

**Machines.**
- **Mac:** this Mac (M5 Pro, 15 cores, 24 GB, on AC power). Other lanes were working (load1 1.7–6.1),
  so the wall times are **non-reference**. Every Mac run held `/tmp/fth-mac.lock`, with no other
  lane's measurement running, and used `nice -n 5`.
- **PC:** the NixOS PC (16 cores) in `flashtex-agents.slice`, with TeX Live 2025 from nix
  (`TEXMFCNF` set). It was loaded by its own gate run, so only its instruction counts mean
  anything.

**Path.** These runs use the app's path: `flashtex-host --socket --s0-cache DIR --profile
balanced|high-performance`, a fresh copy of the document and a fresh S₀ cache each rep, then
`dl3-coldopen` with one `COMPILE` and `--external-tools auto` (a trusted project, as the app
sends it). The driver is `tools/incr-bench/coldopen.py`. Scripts are in this directory and raw
lines in `raw/`.

## 1. Where the open goes (main `94f52aa26`)

| | balanced | high-performance | pdflatex (oracle) |
|---|---:|---:|---:|
| host ready (spawn → `listening`: format, kpathsea, font map, warm-up) | 0.51 s | 0.04 s* | — |
| first visible page (page 1; a first open sends no viewport) | **0.87 s** | 0.86 s | — |
| the page the app shows (page 1: the app restores no scroll position, see §4) | 0.87 s | 0.86 s | — |
| pass 1 done (1,142 pages seen) | 32.3–43.5 s | 32.7 s | 25.9–26.6 s |
| all 1,152 pages seen (the last 10 come only in pass 2, after the toc) | 64.9–80.1 s | 64.7 s | — |
| `DONE` (3 passes, all pages final) | **100.3–123.6 s** | 103.0 s | 78.7 s (3 × ~26 s) |
| engine instructions | 1,759–1,762 G | 1,772 G | 3 × 548 G |
| CPU cores busy | 1 (the engine thread) | 1 | 1 |

(*) the high-performance rep's host was started while the page cache was warm.

The passes are not redundant. pdflatex needs the same three: pass 1 has no `.aux` or `.toc`,
pass 2 adds the toc pages so every label's page moves, and pass 3 converges. Its pass 4 equals
pass 3 (`pdfl.sh`). The host ends pass 1 and pass 2 early because makeindex is due (#1308),
runs makeindex, and its follow-up compiles are passes 2 and 3 (`dones`: cold, then `tools`,
`tools`). Per pass, the host ran 1.11× pdflatex's instructions (607 G against 548 G) and 1.33×
its cycles (144 G against 108.5 G, `/usr/bin/time -l`).

**Engine thread, by component** (macOS `sample`, 1 ms, 95 k samples over the whole open,
`raw/mac-sample-groups.txt`):

| component | share |
|---|---:|
| TeX interpreter (the generated `tex.web` body, arena reads) | 77.3 % |
| checkpoints: `pack_words` 3.8, `Arena::checkpoint` 3.5, `seal` 2.2, `merge_sealed` 1.5; half of it `thin` → `retain`, which merges away segment checkpoints just taken | **10.8 %** |
| display list (`dl_note_node`, emit, send) | 3.9 % |
| diagnostics' definition sites (`dg_def`, `dg_name_id`), BOX-MEMO (0.6 %) | 3.5 % |
| malloc / memmove | 2.6 % |
| kpathsea, lookups, file checks | 0.8 % |
| incremental session (tests, finding the restart point) | 0.8 % |
| PDF writer and zlib (shipout) | 0.4 % |

The TeX share, 77 % of 144 G cycles (111 G), is about pdflatex's whole pass (108 G). So the
cycle gap is the host's own work around TeX, not a slower interpreter. Format loading, kpathsea
and the font map are in the 0.51 s warm-up, before `COMPILE`. On a first open there is no S₀
to load (it is saved after the run, off the engine thread). Rastering is the app's work and is
not on this path.

**A keystroke during passes 2 and 3** preempts them and is served first (VERIFIED, PC, far
build, `raw/edit-far-*.jsonl`): a letter in `book/number-theory/modular-arithmetic.tex` l. 47
(page 266), typed 350 s into the open (pass 3 on the loaded PC), arrived 233 ms after its
`COMPILE`, and the passes went on to all 1,152 pages. The same letter at 180 s (still pass 1 on
that PC) arrived after 591 ms.

## 2. Change 1: a held segment checkpoint far from the cursor is not taken (this PR)

Retention (`incr::thin`) keeps segment checkpoints only within `dense` pages of the cursor. In a
run from scratch, an `.aux`/tools pass, and the background after an edited page, almost every
segment checkpoint was taken, sealed and packed, then merged away within 32 checkpoints. Now
segment checkpoints are held in every run (#1614's mechanism). `Obs::take_held_segment` takes
one only:
- where newer work stops the run;
- before the edited line in an edit's first pass (as before);
- elsewhere, within `dense` pages of the cursor (the ones retention keeps).

Which checkpoints exist changes nothing a run computes. This is a re-port of the never-merged
`backup/cold-far-segments` (4fe07c4b4, 294e7b768) onto main.

| infdesc-x2 cold open, balanced | main `94f52aa26` | this branch `961e7751f` | change |
|---|---:|---:|---:|
| Mac: first visible page | 868–870 ms | 852–860 ms | (noise) |
| Mac: pass 1 done | 32.3–32.7 s | 27.9 s | −14 % |
| Mac: all 1,152 pages seen | 64.9–67.0 s | 54.7–54.8 s | −16 % |
| Mac: `DONE`, all pages final | 100.3–101.2 s | 83.2–85.2 s | **−16 %** |
| Mac: engine instructions | 1,759.3–1,760.6 G | 1,604.3–1,605.0 G | −8.8 % |
| Mac: last pass, cycles | 142–148 G | 122–130 G | −13 % |
| Mac: peak RSS | 990–1,031 MB | 1,076–1,093 MB | +6 % |
| PC: engine instructions, per pass | 598.4 / 639.0 / 636.3 G | 549.9 / 593.0 / 587.2 G | −7.7 % |

(`raw/ab1-*.jsonl` on the Mac, two reps each, interleaved within one lock; `raw/pc-*.jsonl` on
the PC.)

**After the independent review (head `91e924cdd`).** The review found that in-run retention
(`thin`, every 32 checkpoints taken) ran only every 32 pages once far segments were held, which
explains the +6 % peak RSS above. Retention now counts held segment points too, `REQ_PREEMPT` records
the stop it takes, and a continued run starts with no held stop. VERIFIED:

| at `91e924cdd` | main `94f52aa26` | this branch |
|---|---:|---:|
| PC: engine instructions | 1,873.7 G | 1,692.9 G (−9.6 %) |
| PC: per pass | 598.4 / 639.0 / 636.3 G | 547.9 / 552.4 / 592.6 G |
| PC: peak RSS | 1,024 MB | 993 MB |
| Mac (load 7.0–9.3, busier than the run above): `DONE` | 157.9–159.6 s | 136.3–143.5 s (−12 %) |
| Mac: all pages seen | 103.7–104.8 s | 88.9–93.8 s |
| Mac: engine instructions | 1,766.2–1,766.4 G | 1,638.1–1,640.8 G (−7.2 %) |
| Mac: peak RSS | 1,089 MB | 1,070–1,075 MB |

(`raw/ab2-*.jsonl`, `raw/pc-far2.jsonl`.) PC tests at this head: incremental 81/81, including the
new `newer_work_far_from_the_cursor_stops_at_a_held_segment`; host_incremental 27/27; checkpoint
3/3. Pass 1 is now 1.07× pdflatex's wall time on this Mac (27.9 s against 26 s). On the Mac
the wall time falls by more than the instruction count, because fewer checkpoints means fewer
chunks copied and sealed, and so fewer cache misses.

What is left outside TeX after this change (PC `perf`, instructions, `raw/pc-perf-far-top.txt`):
- `dl_note_node` 4.6 %;
- checkpoints (`pack_words` 2.6, `Arena::checkpoint` 1.9, `Delta::unpack` 1.1, `merge_sealed`
  0.9);
- definition sites (the `diag::Def` map insert 1.2, its thread-local 1.0, `dg_def` and
  `dg_def_begin` 1.4);
- `lineshift`'s thread-local, 1.5 %.
These are the next items.

**A reopen with the previous session's `.aux` and S₀ takes one pass, as pdflatex would**
(VERIFIED, PC, far build, `raw/pc-reopen-far.jsonl`, `coldopen.py --keep-aux --keep-s0 --reps 2`).
The first open runs 3 passes: 1,670 G instructions, `DONE` at 142.1 s on the loaded PC. The reopen
runs from the persisted S₀ in one pass: 536 G, page 1 at 650 ms, `DONE` at 51.8 s.

## 3. Reproduce

```sh
CARGO_TARGET_DIR=/tmp/fth-co/target cargo build --release -p flashtex-engine -p flashtex-display-list
CARGO_TARGET_DIR=/tmp/fth-co/target INCR_BENCH_DIR=/tmp/fth-co/ib tools/incr-bench/mkeng.sh NAME
INCR_BENCH_DIR=/tmp/fth-co/ib python3 tools/incr-bench/coldopen.py /tmp/fth-co/ib/NAME DOCDIR infdesc-x2.tex OUT.jsonl \
    --external-tools auto --host-arg=--profile --host-arg=balanced
```

`macab.sh` (the Mac A/B under the lock), `pcab.sh`, `pcperf.sh`, `pcedit.sh` (the PC),
`pdfl.sh` (the oracle's passes) and `sgroup.py` (groups a `sample` profile). DOCDIR is
`tools/parity/corpus/infdesc_x2.py`'s output. It is not committed.

## 4. Found on the way

- The app restores no preview scroll position on relaunch. `EngineV3Snapshot` keeps the pages
  near the viewport but no page index, so the reopen `COMPILE` has no viewport and the page shown
  is page 1. The next COLD-OPEN change saves the visible page and the caret and sends them on
  reopen, with a persisted checkpoint at that page on the host.
- imakeidx's own `\write18{makeindex ...}` fails in the host ("Input index file … not found";
  cwd is the root, the `.idx` is in the output directory). This is the known output-directory
  case (`docs/evidence/infdesc-2026-10-03/`). The host's external-tools pass then runs
  makeindex as latexmk would, so the output is right.
