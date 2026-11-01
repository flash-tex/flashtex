# COLD-SPEED: where a cold compile of *Infinite Descent* goes, and what makes it faster (2026-10-04)

Lane **COLD-SPEED** (mac-claude-a, mac-m1max-a; owner 2026-10-04: "a full compile of Infinite Descent
takes about 20 s; make it much faster"). DESIGN.md §1.2, §4.4, §5.1, §5.5, §5.6 (L6), D10.

**Machine.** M1 Max (10 cores, 32 GB), macOS Low Power Mode on, shared with other sessions: **load1
55-217 during every measurement here**. So **no wall time here is a reference**. The measure is
instructions retired and cycles (`/usr/bin/time -l` for whole processes; the host's own per-thread
counters, `DONE.stages.instr_k`/`cycles_k`, for the engine thread). They do not move with load.
Profiles: Instruments' *CPU Profiler* (`xctrace record`, PMU-cycle-weighted), summarised with
`scripts/xcstk.py` (built on P6-HYPEROPT's `xcprof.py`). TeX-level costs: `FLASHTEX_MACRO_PROFILE`.

**Document.** A copy of `~/Documents/infdesc` (Codeberg `cnewstead/infdesc` `48825c5`; the original
is never touched), `infdesc.tex`, 592 pages. **Oracle.** TeX Live 2026 pdfTeX 1.40.29.
**Engine.** main `d07191179` (`flashtex-initex`, `flashtex-host`, release).

## Phase 1: the breakdown (VERIFIED unless marked)

### Passes, and why each rerun happens (`raw/passes.jsonl`, `scripts/passes.py`)

From a clean copy (no `.aux`), `pdflatex` until the log asks for no rerun:

| pass | pages | rerun because | files changed by the pass |
|---|---:|---|---|
| 1 | 588 | no `.aux`, no `.toc`: 1,808 undefined references; "Label(s) may have changed"; lastpage | `.aux`, `.toc`, 4 × `.idx`/`.ind`/`.ilg` |
| 2 | 592 | the `.toc` now exists (+4 pages: every later page number moves), so every label's page changed: "Label(s) may have changed" | `.aux`, `.toc`, the `.idx`/`.ind` files |
| 3 | 592 | none: converged | none |

The engine runs the same three passes; **every pass's PDF is byte-identical to pdflatex's**.

### Per pass, engine against pdflatex (instructions retired, whole process)

| pass | pdflatex instr / cycles | `flashtex-initex` instr / cycles | ratio (instr) |
|---|---:|---:|---:|
| 1 | 290.6 G / 76.4 G | 284.8 G / 73.3 G | 0.980 |
| 2 | 292.0 G / 73.8 G | 286.3 G / 78.9 G | 0.980 |
| 3 | 292.0 G / 79.3 G | 286.3 G / 76.1 G | 0.980 |
| all | **874.6 G** / 229.5 G | **857.4 G** / 228.3 G | 0.980 |

(Belief, not measured: about 23 s per pass at 3.2 GHz on a quiet machine, which is the owner's "about
20 s".)

### Inside one pass (`raw/profile-initex-pass.txt`, cycles)

| where | share |
|---|---:|
| `main_control` (TeX itself) | **98.5 %** |
| of it, self: `get_next` 32.1, `macro_call` 12.2, `end_token_list` 7.6, `expand` 6.5, `id_lookup` 6.4, `conditional` 4.2, `get_avail` 3.7, `delete_token_ref` 3.1 | |
| PDF `ship_out` (incl. zlib 0.8) | 1.7 % |
| kpathsea (`hash_insert_normalized`) | 0.25 % |

Fixed costs (`raw/fixed-cost.txt`): the format load and kpathsea start take 0.78 G instructions (0.3 %
of a pass), and the same for pdftex. **makeindex** (`raw/makeindex.txt`): imakeidx runs it 4 times
per pass through restricted `\write18`, at 0.51-0.53 G instructions and 0.12 s each (about 0.5 s
and 0.7 % of a pass). Item 2 runs it in-process instead (a Rust port, identical to TeX Live's
on 10,000 fuzz cases and 6,160 corpus runs): `makeindex/README.md`.

At TeX level (`raw/macroprof-top300.tsv`, inclusive): ntheorem's framed theorem boxes with TikZ
frames (`\fb@put@frame`, 1,531 boxes, each frame measured and drawn by a `tikzpicture`) take 45 % of
macro time. pgfmath parsing (`\pgfmath@parse@next` and the rest) takes about 22 %. This is pure macro
interpretation. Replaying pgfmath is MACRO-REPLAY's work (P6-HYPEROPT), not this lane's.

### The app's path: the socket host, cold (`raw/host-main.jsonl`, `raw/profile-host-cold*.txt`)

`flashtex-host --socket` and dl3-client, one `COMPILE` of a fresh copy, as the app sends it:

- **1,108 G instructions, 338 G cycles: 1.27× pdflatex's three passes** (874.6 G). It ran 3 passes
  and **re-typeset all of them (1,736 pages)**. `iserve` (the resident engine without a display list)
  took 934 G, 1.07× pdflatex (`raw/iserve-main.out`).
- First page after 2.4-4.8 s (loaded): 25 G instructions, almost all of it the preamble.
- The engine thread does 99.9 % of the process's work; no other thread works.
- Host overhead, as a share of the compile's cycles (`raw/profile-host-cold-groups.txt`):

| what | share |
|---|---:|
| `displaylist::dl_note_node`: every token or node allocated is given its source position, and each call walked the input stack down to the innermost file | **11.75 %** |
| `diag::dg_def`: the definition site of every `\def` (another walk, and a file-name `Vec` per definition) | 6.7 % |
| checkpoints (`Arena::checkpoint`, `retain`, `Core::save`, seal) | 6.4 % |
| malloc / free / memmove | 4.4 % |
| `pdf_ship_out` (preview: zlib level 0) | 2.2 % |
| display-list emit and send | 1.4 % |

### Why passes 2 and 3 re-typeset everything (`raw/l5probe.out`, `raw/aux-open-debug.txt`)

L5 (DESIGN.md §5.5) never applies to this book: every incremental pass reports `"no .aux point"`. The
`.aux` point is requested when `\document` opens the `.aux` while the S₀ arm is set
(`Globals::note_aux_open`). In this book S₀ is taken **before** `\document` reads the `.aux`, so the
open happens at arm level 0, after S₀, and no point is taken (debug build: `[aux] open ./infdesc.aux:
arm_level 0 ... s0 Some(66)`). This holds with the `.aux` present from the start too
(`scripts/l5probe.sh`: pdflatex's pass-1 files, then the engine: 2 passes, 1,184 pages re-typeset).
So every pass restarts at S₀.

### The app: every relaunch is the full cold compile

`EngineV3Mirror` puts each app instance's copy of the project in
`engine-v3/projects/<hash>-<pid>-<n>/{src,out}`. A relaunch therefore starts with no `.aux`, and the
S₀ cache key (it holds the root and output paths) is new. **Every relaunch is the 3-pass cold
compile**, never the 1-pass compile an existing `.aux` allows. (Code reading of
`apps/mac/Sources/FlashTeXMac/EngineV3Session.swift`, confirmed by the owner's own copies in
`~/Library/Caches/FlashTeX/engine-v3/projects/`.)

## Phase 2

### PR 1: `cold-host-overhead`: the host's per-allocation and per-definition bookkeeping

Three changes. None changes what TeX computes: no PDF, log or `.aux` byte changes.

1. **`dl_note_node` and `dg_here` no longer walk the input stack** (`Globals::file_level`). Every
   input level whose state is not `token_list` was pushed by `begin_file_reading`, which gives it
   `index:=in_open` (§328, §329). So the level found last time is still the innermost file level
   exactly when its state is not `token_list` and its index is `in_open`. The hint is checked
   against the live state each time, so a restore leaves it exact. The test
   `file_level_finds_what_the_walk_finds` compares it with the walk on 200,000 random pushes and
   pops (files, `\read` levels, pseudo files, token lists, and foreign hints).
2. **`back_input`'s token and the condition stack's node are not noted**
   (`changes/displaylist.ch`: `dl_token_begin`/`dl_token_end`). Neither ever becomes part of a list
   TeX ships, and they are most of the allocations made outside the scanner. A location's side-table
   entry is set again when it is next allocated as a node. The side table is already dead to the
   convergence test (`incr::dead_word`).
3. **`dg_def`** keeps each definition site's file once (`St::files`, a number per site instead of a
   `Vec<u8>` per `\def`), with a fast hasher for the site map.

| infdesc, socket host, cold (3 passes, 1,736 pages) | main | (1)+(3) | (1)+(2)+(3) | change |
|---|---:|---:|---:|---:|
| engine-thread instructions | 1,108.3 G | 993.4 G | **962.9 G** | **−13.1 %** |
| engine-thread cycles | 338.5 G | 302.4 G | **288.8 G** | **−14.7 %** |
| first-page instructions | 25.0 G | 24.5 G | 24.0 G | −4.1 % |

(`raw/host-main.jsonl`, `raw/host-dg.jsonl`, `raw/host-tok.jsonl`; load 55-150.) After the change,
`dl_note_node` takes 5.2 % of the compile's cycles with only (1)+(3) applied, and almost nothing with
(2) as well (`raw/profile-host-dg-groups.txt`).

Keystrokes (`raw/abkeys-tok.txt`, P6-HYPEROPT's `abkeys.py`, 10 letters in the middle, p50
instructions per compile): full-100 628.6 → 621.8 M (−1.1 %), plain-100 377.5 → 375.7 M (−0.5 %).
The edited page is unchanged (97.9/98.0 M, 52.0/52.0 M). These generated documents are light on
macros. The gain grows with macro expansion, as on the book.

**Identity gates.**
- Display lists and diagnostics: `scripts/dlsame.py` (P6-HYPEROPT's, plus `--diag`). Three compiles
  of every fixture with a `main.tex` (138) through each host, then `dl3-dump --canonical` and every
  `diag-v1` message, compared: **138/138 the same** (`raw/dlsame-tok2.txt`). One caveat: span and
  file **ids** are the host's handles for (file, line) pairs and paths, numbered as the run first
  meets each, and a host that notes fewer allocations meets fewer pairs. So ids are compared by what
  they name (`unnumber_spans`). Every glyph, link and message names the same file and line as on
  main. Compared raw, ids included (P6-HYPEROPT's script), (1) alone and (1)+(3) were identical on all 138 as well (`raw/dlsame-dlc.txt`, `raw/dlsame-dg.txt`; the one difference in the latter is a `TOOL` message, which arrives on its own schedule).
- PDF, log, `.aux` and P-T1/P-T2: `scripts/gate.sh pr` (the parity fixtures, lockstep, trip and
  etrip; see the PR).
