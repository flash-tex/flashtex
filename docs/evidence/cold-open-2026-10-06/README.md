# COLD-OPEN: where a document's first open goes (2026-10-06)

Lane **COLD-OPEN** (engineer under kabir-claude; DESIGN.md §1.2 "Opening", owner 2026-10-06:
*"the initial render of the document takes really long (22s with this good cpu)"* on the 1,000-page
test book; *"when all pages have not been initially loaded, editing is really slow"*).

**Machine.** The NixOS PC (16 cores, x86-64), inside the agents' CPU-capped systemd slice and shared
with other lanes and CI (load 4–60 during these runs). **No wall time here is a reference.** The
measure is the engine thread's instructions (`DONE.stages.instr_k`, `perf_event_open`). `perf
record -e instructions:u` profiles come from `perf` 7.2 (nixpkgs).

**Engine.** main `b76f057f1`, release. **Client.** `dl3-coldopen` (this PR): one `COMPILE` as the app
sends it (incremental, `progress-v1`), timing every `PAGE`, each pass's start and `DONE`; optionally a
keystroke during the open. **Driver.** `tools/incr-bench/coldopen.py`: a fresh copy of the document
(no `.aux`) and a fresh host per rep.

**Documents.**
- `book.tex`: the owner's 1,072-page test book (single file; geometry, ams*, tikz, siunitx, hyperref,
  cleveref; no labels). Not committed.
- `plain-1000`, `full-1000`: `tools/incr-bench/mkdocs.py`.
- `infdesc-x2.tex`: *Infinite Descent* twice, 1,130 pages (`tools/parity/corpus/infdesc_x2.py`, Codeberg
  cnewstead/infdesc `48825c5`). Not committed.

## The breakdown (VERIFIED, `raw/book-main.jsonl`, `raw/base-main.jsonl`)

| | pages | passes | first page | every page seen | `DONE` | engine instructions | of them, first page |
|---|---:|---:|---:|---:|---:|---:|---:|
| book.tex | 1,072 | 2 | 0.97 s | 12.2 s | 22.9 s | 132.0 G | 5.7 G |
| plain-1000 | 1,001 | 2 | 0.14 s | 6.1 s | 12.1 s | 68.5 G | 0.7 G |
| full-1000 | 1,002 | 3 | 0.76 s | 24.5 s | 69.7 s | 259.3 G | 3.8 G |
| infdesc-x2 (tools on) | 1,130 | 1, then tools | 6.4 s | 183.8 s | 184.3 s | 606.2 G | 21.3 G |

(Wall times from `COMPILE` sent; the host's start-up and warm-up before it was 0.5–0.8 s.)

The 22 s on book.tex has three parts:
1. **Page 1:** the preamble and the first page, 5.7 G instructions.
2. **Pass 1:** every page, shown as it is shipped. All of them are on screen at 12.2 s.
3. **Pass 2, from the format:** hyperref's `.out` appeared ("Rerun to get outlines right", as
   pdflatex says too). Its outline objects renumber every later PDF object, so the PDF changes on every
   page, although no display list does (pass 2 sent no page: 1,072 `PAGE` frames in all). pdflatex
   needs the same 2 passes (7.7 s and 7.1 s of user time on this PC).

Where the engine thread's instructions go (`raw/perf-book-main-groups.txt`, `raw/perf-book-main-top.txt`):
TeX itself 58 %, display-list emission 17 % (`Interp::show`, `dl_note_node`, the page hash and
encoding), checkpoints 16 % (`Arena::checkpoint`, sealing), malloc/memmove 5 %, PDF writing and zlib
3 %.

## Edits during the open (VERIFIED, `raw/book-edit-main.jsonl`, `raw/book-keepaux.jsonl`)

A letter typed during the open (`--edit-at-ms`, `--edit-viewport`):

| keystroke at | the open was at | edited page | its compile | instructions to the edited page | wall |
|---|---|---:|---|---:|---:|
| 4 s | page 382, pass 1 | 61 | **cold**: "looking up main.aux finds another file now" | 5,742 M | 1.8 s |
| 4 s | page 248, pass 1 | 928 | cold, same reason | (from page 0) | 12.1 s |
| 16 s | pass 2 | 61 | incremental, from page 17 | 54 M | 0.4 s |
| 16 s | page 1,015, pass 1 | 928 | cold, same reason | 5,747 M | 12.2 s |

With the previous run's `.aux` present (`--keep-aux`, the second rep), the same keystroke is
incremental and its page arrives in 30 ms.

**Why.** S₀ is taken when `\document` ends, after it has looked for `main.aux` and found none, so that
lookup is in S₀'s key. The run then writes `main.aux`. The next compile's key check finds the file and
restarts from the format, and so does the open's own second pass. The fix is
`agent/kabir-claude/coldopen-anchor`, with its own evidence.

## Reproduce

```sh
INCR_BENCH_DIR=$HOME/coldopen/ib tools/incr-bench/mkeng.sh main
python3 tools/incr-bench/coldopen.py $HOME/coldopen/ib/main DOCDIR main.tex OUT.jsonl \
    [--edit-at-ms 4000 --edit-line 1001 --edit-viewport] [--keep-aux --reps 2]
```

Profiles: `perf record -m 8 -e instructions:u -F 1000 -- python3 tools/incr-bench/coldopen.py ...`,
then `scripts/pgroup.py` (grouped shares) and `scripts/pdiff.py` (two profiles, by symbol).
