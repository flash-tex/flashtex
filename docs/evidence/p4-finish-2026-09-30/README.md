# P4-FINISH: the host's keystroke path, convergence, the owner's book (2026-09-30)

Lane **P4-FINISH** (kabir-claude, mac-m5pro-kabir): DESIGN.md §1.2, §5.2–§5.6 and §12's P4 exit
gate. Branch `agent/kabir-claude/p4-finish`, from `origin/main` `02dcf9d07`, with
`agent/kabir-claude/l6-optimizations` merged in (**#1241 had not landed**; it carries #1230's
intrinsics, so every number here has intrinsics on, their default). The Commander added
mid-lane: the owner's 1,072-page `book.tex`, and track 1 of the 2026-09-30 review (convergence
false negatives, silent host exits, one harness under `tools/`).

**Engines** (each a `tools/incr-bench/mkeng.sh` copy):

| label | commit | what it has |
|---|---|---|
| base | `15f96b026` | main + #1241, before this lane |
| fin2 | `c7688f958` | the host-path fixes (§2) |
| fin3 | `84bfff01a` | plus the convergence rules, with the `pdf_char_used` union |
| **fin4** | `f99f8ea82` | the final engine: the union backed out (§4) |

Raw files are in [`raw/`](raw/); drivers are in [`tools/incr-bench/`](../../../tools/incr-bench/)
(moved there by this lane) and [`scripts/`](scripts/) (this lane's measurement sequences).

**Machines.** The Mac is an M5 Pro (5 "Super" and 10 "Performance" cores) on macOS 26.6. It was
shared with other lanes, and the load was 2–6 during the measurements; each table gives it.
Soundness and gates ran on the NixOS PC (Ryzen 7 7800X3D, TeX Live 2026).

**Labels.** VERIFIED means measured here; the command and raw file are named. BELIEF means an
inference that was not measured.

## 1. Where the host's time goes (VERIFIED)

**The problem.** The app measured 13–20 ms from COMPILE to the edited page inside the host
(P3-APP-V3). The engine benchmarks gave 5–10 ms.

**The instrument.** DONE now carries `stages`, in ms:
- queue, apply, move_spans;
- find (key + changes), restore;
- the first page's wall and thread CPU;
- display-list building and socket writes up to the first page;
- the convergence tests, and the whole compile.

**The run.** `dl3-keys` typed through the socket as the app does: page 1, no `viewport`.
The engine was base plus the stage timings (`raw/keys/`).

| plain-10, page 1 | first page wall / CPU | apply | find | restore | display list | rest (TeX up to the shipout) |
|---|---|---|---|---|---|---|
| keystrokes back to back | 6.7 / 6.5 | 0.08 | 0.17 | 0.69 | 0.71 | ≈ 4.9 |
| 300 ms apart (the app's bench) | 18.1 / 17.9 | 0.42 | 0.67 | 1.97 | 1.39 | ≈ 13.4 |

**Findings.**
- **After an idle gap every stage is 2–3× slower.** The time is thread CPU, not waiting. First
  page p50 against the gap: 0 ms 6.8, 5 ms 6.8, 20 ms 7.8, 50 ms 9.5, 100 ms 12.5, 300 ms ≈ 18
  (`raw/keys/plain-10-0.05-qos-gap*.jsonl`).
- **It is the machine, not the host.** A C loop of pure ALU work does the same:
  - 7.0 ms back to back;
  - 13–15 ms after 300 ms idle, for the whole 16 ms burst (`raw/probe/`).

  Three remedies each helped in some runs and not in others: QoS `USER_INTERACTIVE`, the Mach
  time-constraint policy, and an `os_workgroup` interval with a deadline. None was reliable. The
  penalty mostly disappears when other lanes' builds keep the cores busy (load 4–6,
  `raw/measure-fin1.log`).
- **The host's own code is ~1.3 ms of the 6.5** at full speed (apply, find, restore), plus 0.7 ms
  for the display list. The 2× does not come from there.

## 2. Fixes, each with before and after (VERIFIED unless marked)

**`--keep-warm MS`** (opt-in; off by default). After a compile the engine thread polls for the
next request instead of sleeping.
- plain-10, keystrokes 300 ms apart, 3 interleaved rounds, first page p50: off 11.6 / 18.5 /
  8.9 ms, on 8.1 / 7.8 / 8.3 ms (`raw/keys/ab-keep-warm.txt`).
- More results are in §3 and §6.
- It costs one busy core while the user types. **Whether to turn it on is the owner's or the
  Commander's decision; this lane did not take it.**

**QoS `USER_INTERACTIVE`** on the engine and connection threads. No reliable change was
measured. It is kept, because that is what this work is.

**No convergence test at the first unchanged page before the edited one.**
- Why it matters: a restart just before the edited paragraph ships the page before it again. A
  test at that page cost up to ~20 ms on 1,000 pages and delayed the edited page.
- book.tex, a letter on pages 5 and 130: edited page p50 34 → 17 ms (`raw/book/m1-*` → `m2-*`,
  load 4–5).

**The convergence test is interruptible.**
- Newer work is checked every 64 logs of the rewind through the old run's future, and every 1,024
  tasks of the structural walk.
- On book.tex the rewind alone was 3,400 logs, 2.6 M entries and 17 ms (`FLASHTEX_INCR_DEBUG`).
- The stop latency was not timed separately (BELIEF: ≤ 0.3 ms).

**`diff_edit` compares blocks (memcmp), not single bytes.**
- plain-1000 `changes`: 1.77 → 0.80 ms (0.31 ms with warm cores).
- book.tex `find`: 2.6–3.0 → 1.3–1.5 ms.

**The host keeps the bytes it last wrote to each file.** An edit splices into them and rewrites
the file from the edit on.
- book.tex `apply`: 1–4.5 → 0.6–1.3 ms.
- The bytes on disk are the same, and the engine still reads the file.

**`max_buf_stack` and the other high-water marks count as accounting** (§1.1), not as state.
- plain-10, an edit on page 1: the run used to converge only at page 9 of 10. Now it converges at
  the next page when the edit does not reflow.

**A `\pdfdest`'s unset dimensions are dead unless a `\pdfsetmatrix` reads them.**
- Before, on book.tex every keystroke re-typeset 943 pages (5.5–6.9 s in the background), and the
  next keystroke restored back over all of them (34–41 ms).
- Now it converges within 3–33 pages.

**Emitting the edited page first needed no code change.** The display list is built and sent at
`pdf_ship_out`, before the checkpoint and before any test. The test-skip above removes the one
test that was delaying it.

**Shipout pipelining was not done.** In preview mode (the keystroke path) nothing is compressed.
There its ceiling is the display-list build, 0.7–0.8 ms of the edited page (`first_page_dl`).
L6's −14 % / −20 % applies to exports and cold runs (BELIEF, from L6's numbers).

## 3. The owner's book.tex (1,072 pages; VERIFIED, `raw/book/`, `tools/incr-bench/keys_*`)

**Setup.**
- Our own host and our own S₀ cache; the owner's app and host were not touched.
- Keystrokes 300 ms apart, 6 per session: 3 edits, each followed by its revert.
- Pages 5, 130, 540 and 1,000 (0-based 4, 129, 539, 999).
- Each at the start, middle and end of a paragraph's line.
- Each as a single letter and as twelve inserted words.

**Restart point.**
- **Every keystroke restarted from the newest checkpoint before the edit.** The next checkpoint
  reads the file past the edit: `restart_next_gap` > 0 on 144/144 keystrokes, for each of fin2
  and fin3.
- Checkpoints between pages are taken after `build_page`, at least 0.5 ms of engine time apart
  (P4-L5).
- So the newest one is usually in the edited line or the blank line before it. For 12–29 of each
  144 it is up to one segment earlier: at most 3.7 KB of source, about 0.5 ms of typesetting.
- No restart went back further than one segment.
- When the edited paragraph begins a page, the page before it ships again first (TeX breaks the
  page only after reading that paragraph).

**Latency, interleaved** (`scripts/measure5.sh`, `raw/book/ab*`). A letter mid-line, 2 rounds, load
3–4. Edited page, client side, p50 per round (p95):

| | page 5 | page 130 |
|---|---|---|
| base | 75.8 / 75.6 (108 / 95) | 50.3 / 48.4 (52 / 52) |
| fin2 | 16.7 / 25.4 (19 / 29) | 27.3 / 21.3 (33 / 29) |
| fin3 | 21.6 / 27.0 (29 / 29) | 27.1 / 27.4 (31 / 30) |
| fin3 `--keep-warm 400` | **14.1 / 13.7 (17 / 16)** | **15.2 / 13.6 (18 / 15)** |

**The whole matrix, fin2 at load 4–5** (warm cores). Edited page p50 / p95 (`raw/book/after-fin2-*`):

| | page 5 | page 130 | page 540 | page 1,000 |
|---|---|---|---|---|
| a letter | 14.1 / 16.7 | 13.7 / 14.5 | 11.7 / 14.5 | 7.5 / 8.3 |
| twelve words | 14.1 / 18.2 | 14.3 / 19.4 | 12.0 / 13.7 | 7.9 / 8.6 |

The same matrix for fin3 at load 2–3 is 17–28 ms p50, 21–32 p95 (`raw/book/book-fin3-*`). That
is the idle-clock effect of §1: it has 13–17 ms of first-page CPU, against 7–8 ms warm.

**One keystroke on page 130 (fin2), step by step.**
- apply 0.6–1.3 ms, find 1.4–1.5 ms, restore 5.5–6.1 ms.
- The first page (129, shipped again) at 10.5–11.2 ms.
- The edited page at 13.8–14.5 ms.
- DONE at 92–98 ms, after converging at page 131.
- Before, on base: restore 34–41 ms, a test ahead of the edited page, the edited page at 79–93 ms,
  and DONE at 5.5–6.9 s.

## 4. Convergence (review track 1, item 1; VERIFIED, `raw/convergence-*.md`, `raw/matrix.tar.gz`)

**What was measured.** The latency matrix (`tools/incr-bench/matrix.py`): 8 documents × start /
middle / end × 16 letters and 6 sentences, each with its revert. Every engine used the same seeds.
The table counts the compiles that converged.

| doc | base | fin4 (final) | compile total p95, s: base → fin4 | pages re-run p50: base → fin4 |
|---|---|---|---|---|
| full-10 | 14/66 (14/48 letters, 0/18 sentences) | 22/66 (22/48, 0/18) | 0.10 → 0.10 | 3 → 2 |
| full-100 | 10/66 (10/48, 0/18) | **38/66 (36/48, 2/18)** | 0.35 → 0.32 | 49 → 2 |
| full-300 | 8/66 (8/48, 0/18) | **50/66 (40/48, 10/18)** | 0.90 → 0.79 | 147 → 10 |
| full-1000 | 22/66 (22/48, 0/18) | **50/66 (38/48, 12/18)** | 3.22 → 2.72 | 64 → 10 |
| plain-10 | 34/66 | 34/66 | 0.04 → 0.04 | 2 → 2 |
| plain-100 | 25/66 | 27/66 | 0.09 → 0.10 | 5 → 5 |
| plain-300 | 52/66 | 54/66 | 0.14 → 0.14 | 5 → 5 |
| plain-1000 | 52/66 | 56/66 | 0.87 → 0.87 | 3 → 3 |

fin3, with the `pdf_char_used` union, reached full-100 50/66, full-300 58/66, full-1000 60/66,
plain-300 66/66 and plain-1000 64/66. That union was **unsound across compiles**, so it was backed
out (below).

**Why these tests were failing.** Each rule below compared something pdfTeX never reads. The code
cites the pdftex.web or `intrinsics.rs` source at each rule.
- The high halves of whatsit words that pdfTeX reads only as `.sc` or `.int`: annot, link, thread
  and destination dimensions and object numbers. This was the review's largest class, 84
  full-document failures.
- Words pdfTeX never sets or reads:
  - word 4 of annots, links, threads and destinations before shipout;
  - word 1 of the small whatsits that carry no data;
  - `info(p+3)` of an action.
- `max_buf_stack` and the other high-water marks, which are accounting.
- `best_height_plus_depth`.
- `hc` and `hu`.
- While no recording is in progress, the intrinsics' recording scratch (`intr_state` 2..23) and
  `intr_pre`. This is the review's `intr_state` class.

**What still blocks convergence (fin4), in order of count.**
1. `pdf_char_used`, 64: a character's first use added or removed before the convergence point.
2. Reflows that have not settled yet (plain documents; genuine).
3. `token differs` in hyperref's chains.
4. `intr_data`.
5. `pdf_ptr` and `pdf_os_buf` (the PDF writer's buffers).
6. `page_so_far`.

**The `pdf_char_used` union, and why it came out.**
- What it did: converge when the new set holds the old one, and OR the new characters into the
  old run's later checkpoints.
- What went wrong: a later compile compared against a checkpoint as stored, without the OR, and
  then restored the OR. So the revert of an edit that added a glyph kept that glyph in the font
  subsets.
- Soundness sweep A on fin3 (PC): 103 of 8,500 compiles. All had larger font subsets than
  pdflatex, with logs that differed only in accounting.
- The correct version compares against the patched values, or keeps the sets per page (the
  review's proposal). **It is the largest remaining class.**

**A barrier the test missed.** This lane's soundness sweep on the PC found it, on beamer-fragile
under load.
- The (b′) test skipped the old run's later reads of files that it closes again.
- So a convergence before a fragile frame's `\input` of `main.vrb` kept a page typeset from the
  old run's `.vrb`. That page had another frame's title, and the font subsets were short of its
  glyphs (sweeps on `2b58e8fc9` and `c7688f958`: 25 and 7 mismatches).
- Fixed: (b′) now sees every later read.

## 5. Engine latency matrix (VERIFIED, `raw/matrix-table-*.md`; iserve, compiles back to back)

Edited page over all 66 compiles, p50 / p95 wall (p95 thread CPU), in ms:

| doc | base | fin4 |
|---|---|---|
| plain-10 | 2.0 / 4.4 (4.1) | 1.9 / 3.2 (2.9) |
| plain-100 | 2.7 / 5.8 (5.3) | 2.6 / 3.3 (2.8) |
| plain-300 | 2.8 / 3.4 (2.3) | 2.5 / 3.1 (2.6) |
| plain-1000 | 5.1 / 12.9 (9.5) | 3.5 / 5.3 (3.6) |
| full-10 | 4.3 / 9.9 (9.4) | 4.2 / 10.6 (10.1) |
| full-100 | 3.9 / 6.3 (5.4) | 3.7 / 6.2 (5.5) |
| full-300 | 5.7 / 7.2 (6.0) | 4.9 / 5.5 (4.4) |
| full-1000 | 9.2 / 12.2 (8.1) | 7.2 / 8.9 (6.2) |

**Preamble edit** (§1.2: ≤ 400 ms to the first page; `tools/incr-bench/preamble.py`). One line was
added after `\documentclass`, then `compile 1` was timed (engine side), 5 edits each:

| doc | base | fin3 |
|---|---|---|
| plain-100 | 40.3–40.7 ms | 40.5–40.9 ms |
| full-100 | 176–190 ms | 175–181 ms |
| full-1000 | 194–237 ms | 191–281 ms (the slow two ran concurrently with §3's runs) |

## 6. Through the socket, as the app types (VERIFIED, `raw/measure-fin3.log`, `raw/keys/`)

**Setup:** page 1, no viewport, 30 keystrokes 300 ms apart, engines interleaved, 2 rounds, load
2–3. Engine fin3; fin4 changes only convergence, not this path.

First page p50 per round (p95), in ms:

| doc | before (base) | fin3 | fin3 `--keep-warm 400` |
|---|---|---|---|
| plain-10 | 9.8 / 12.0 (21.5 / 18.6) | 8.1 / 18.6 (10.8 / 20.4) | **8.0 / 7.9 (14.6 / 8.9)** |
| plain-120 | 17.5 / 18.4 (21.5 / 21.3) | 19.1 / 19.1 (20.4 / 20.6) | **8.0 / 7.9 (12.5 / 8.3)** |
| plain-1000 | 26.2 / 25.8 (28.5 / 28.3) | 22.8 / 23.1 (24.9 / 25.0) | **10.1 / 10.2 (11.1 / 15.4)** |

**The app bench (keystroke → commit) was not re-run.** The owner was using the app on this Mac
(`flashtex-v3-try`), and the Commander's instruction was not to touch it. A second GUI instance
typing into its own window would share the owner's defaults and cores.

BELIEF: keystroke → commit ≈ the host's first page above plus the app's own stages, which
P3-APP-V3 measured at 1.7–2.9 ms p50. That gives ≈ 10.5 / 10.5 / 12.7 ms p50 with `--keep-warm`,
against the 22.9 / 15.4 / 22.9 ms it measured. Without `--keep-warm`, plain documents keep their
before-values; the book improves (§3).

## 7. Host exits (review track 1, item 2; VERIFIED)

`src/host/crash.rs` makes each of these leave a line on stderr, and in
`FLASHTEX_HOST_CRASH_LOG` when that is set:

| event | what the line gives |
|---|---|
| a panic | the message, the request being served, a backtrace |
| SIGABRT (also how Rust reports a stack overflow) | the signal and the request being served, written from the handler with `write(2)` only |
| SIGTERM, SIGINT, SIGHUP | as for SIGABRT |
| a normal end | its reason and the number of requests served |

**iserve now runs the engine on a 512 MB stack thread,** like the socket host. It used to run on
the main thread, which has the system's 8 MB.

**The three silent exits the review saw were not reproduced.** BELIEF: a stack overflow is the
likeliest cause. The review's harness discarded stderr, and that is where Rust writes its overflow
message.

**The harness now keeps the host's stderr** (`*.host-stderr`).

## 8. Gates (VERIFIED)

**Where they ran.** The engine gates ran on the NixOS PC (`tools/incr-bench/gates.sh`, `raw/pc/`). The
engine there was **f99f8ea82**, merged locally with #1232's rpath fix (`9ad08bda3`), with
intrinsics on. `scripts/gate.sh pr` ran on the Mac (`raw/gate-pr-mac.txt`), because the PC's
toolchain has no rustfmt or clippy.

| gate | result |
|---|---|
| soundness A: 50 letters per document plus reverts; 83 fixtures, plain-120, full-100 | **8,500 compiles, 0 mismatches** (737 converged; 10 logs differ in accounting only) |
| soundness C: 20 structural edits (sentence, section, label, ref, cite, footnote, unlabel, unsection); fixtures, refs-30/120, full-100 | **1,966, 0 mismatches** |
| soundness D: 12 interleaved (preempted) edits | **1,409 verified + 223 interrupted, 0 mismatches** |
| soundness on book.tex: 8 letters and 4 sentences, plus reverts | **24, 0 mismatches** (all converged) |
| P-T1 / P-T2, 83 fixtures | **83/83 / 83/83** |
| lockstep | **260/260**, accounting 0 |
| trip, etrip, drift | pass |
| display-list positions checker | **83/83** exact (230 pages, 118,899 glyphs) |
| cargo tests: incremental (10, with the two new ones), host_incremental, display_list_host, intrinsics, lib | pass |
| `scripts/gate.sh pr` (Mac) | **passed** (rustfmt, clippy, tests, licence boundary, parity self-tests, fixture baseline) |

**Earlier sweeps that found the two bugs in §4** (`raw/pc/soundness-a-*`):

| engine | sweep A mismatches |
|---|---|
| `c7688f958` (the `.vrb` barrier, beamer-fragile) | 7 of 8,500 |
| `84bfff01a` (the `pdf_char_used` union) | 103 of 8,500 |
| `bf2c8d76b` (the first convergence fixes) | 0 of 8,500 |

## 9. What remains for §1.2

1. **The idle clock (Apple Silicon).** Without `--keep-warm`, an edit on page 1 of plain-120 or
   plain-1000 takes 19–23 ms in the host at a quiet load, and book.tex 21–27 ms. That is TeX's own
   ~6–10 ms of work run at a third of the clock. `--keep-warm` brings it to 8–15 ms p50.
   **Turning it on by default is an energy trade: the owner's or the Commander's decision.** It
   could also be limited to the app's typing bursts.
2. **Restoring from the document's end.** After a convergence the host jumps to the old run's end
   state, so the next keystroke's restore walks back over every later page: 2.5 ms on plain-1000
   page 1, and about 6 ms on book.tex's early pages. Keeping the live state at the convergence
   point (a lazy redo) would remove it. This is an arena change, not attempted here.
3. **Convergence.** Five classes remain:
   - `pdf_char_used`: 64 cases, compared against the patched values;
   - `intr_data`;
   - hyperref's token chains;
   - the PDF writer's buffers (`pdf_ptr`, `pdf_os_buf`);
   - `page_so_far` while the page is empty.

   Each miss re-typesets to the end in the background: full-1000 p95 2.7 s of CPU.
4. **The test's own cost.** On 1,000 pages, rewinding the old run's future takes ~17 ms per test
   (3,400 logs). It is now interruptible, but it is still background CPU per keystroke. An index
   from each chunk to the logs that hold it would cut it.
5. **Not measured here:**
   - the app bench, keystroke → commit, to re-run when the app is free;
   - reopen ≤ 100 ms;
   - Linux latency (the review: full-1000 needs about 2× there).

## 10. Follow-up (owner decision 10A; restore from the end; glyph usage)

Commits `ea02a29d4`, `e58798bbb` (in #1269) and `e83bfa04b` and later (branch
`agent/kabir-claude/p4-finish-2`). The earlier sections describe `f99f8ea82`; where they
disagree, this section is newer.

### Keep-warm during typing, on by default (VERIFIED, `raw/warm-cost-fin6.log`)

**What it does.** After each compile the host keeps a core warm for 2 s (`--keep-warm MS`; 0
turns it off). It never does so while idle. By default it alternates 100 µs sleeps with 100 µs
spins (`--keep-warm-pause US`; 0 spins throughout).

**Measurement** (`tools/incr-bench/warm_cost.py`): socket, page 1, 100 keystrokes 300 ms apart,
load 3–6. Each run gives the edited page p50 / p95 in ms and the host's CPU seconds per minute of
typing. Every configuration used 0 CPU while idle. The keep-warm tail after the last keystroke
was 1.7 CPU-s spinning and 0.7–0.8 CPU-s pausing.

| doc | off | spin | 100 µs pause (the default) |
|---|---|---|---|
| plain-10 | 7.8 / 27.4 · 10 | 8.0 / 9.7 · 60 | 8.4 / 9.3 · 32 |
| full-10 | 42.4 / 46.0 · 15 | 14.5 / 15.8 · 60 | 15.1 / 16.6 · 33 |
| plain-120 | 25.4 / 29.6 · 10 | 8.7 / 11.0 · 60 | 8.1 / 9.9 · 30 |
| full-120 | 41.1 / 46.6 · 38 | 15.3 / 16.8 · 60 | 15.7 / 17.7 · 47 |
| plain-1000 | 28.1 / 30.5 · 17 | 8.2 / 8.8 · 60 | 8.3 / 8.9 · 32 |
| full-1000 | 51.8 / 89.2 · 51 | not measured | not measured |

The full-1000 spin and pause runs hit the harness's 20-minute limit. The reason is that each
keystroke there re-typesets most of the document before DONE, and the harness waits for DONE.

**What it rests on.** "Warm" works because an idle Apple Silicon core runs a short burst at a
half to a third of its speed (§1). The pause variant keeps the spin's latency at about half its
CPU. Energy is only a CPU-time proxy; `powermetrics` was not used.

### The next keystroke's restore, prepared while idle (VERIFIED, `raw/prepared-restore-*.txt`)

**Before.** After a convergence the live state is the document's end. So the next restore
rewound every later page's log.

**After.** Once DONE is out, the host works out the restore to the last restart point
(`Session::prepare_next` → `Arena::prepare_restore`). A compile that restarts there copies the
chunks in.
- The prepared state is used only while the checkpoint list is unchanged and no log was changed in
  place.
- A new request stops the preparation.
- The preparation itself took 19–43 ms, off the keystroke path.

Measured undo part of the restore (`FLASHTEX_INCR_DEBUG`):

| document | rewound | prepared |
|---|---|---|
| book.tex, page 130 | 13.4 ms (1,363 logs) | 0.86–0.88 ms (1,938 chunks) |
| plain-1000, page 1 | 4.6 ms | 0.3–0.9 ms |

The rest of the restore (host state 2.7 ms, output tails 0.5–1 ms) remains.

### Glyph usage, done soundly

`Arena::or_from` writes the new run's extra characters into the history itself: the old run's
checkpoints from the convergence point on, and the live state. It also records the word's earlier
value in the log into that checkpoint, so the earlier checkpoints keep it. Unit tests:
`or_from_rewrites_the_later_history`, `prepared_restores_equal_plain_ones`.

Converged compiles out of 66, matrix, same seeds (`raw/convergence-fin5.md`):

| doc | base | f99f8ea82 | ea02a29d4 |
|---|---|---|---|
| full-10 | 14 | 22 | 23 |
| full-100 | 10 | 38 | 44 |
| full-300 | 8 | 50 | 54 |
| full-1000 | 22 | 50 | 55 |
| plain-10 | 34 | 34 | 35 |
| plain-100 | 25 | 27 | 32 |
| plain-300 | 52 | 54 | 60 |
| plain-1000 | 52 | 56 | 60 |

What is left of the glyph class (32 cases) are sets that lost a character.

### Gates at `b02654d8c` (#1269's head, main merged)

| gate | result |
|---|---|
| soundness A | 8,500 compiles, 0 mismatches (768 converged) |
| soundness C | 1,966, 0 mismatches |
| soundness D | 1,409 verified + 223 interrupted, 0 mismatches |
| soundness on book.tex | 24, 0 mismatches |
| P-T1 / P-T2 | 83/83 / 83/83 |
| lockstep | 1,145/1,145; 1 case differs in accounting only, which does not gate |
| trip, etrip, drift | pass |
| display-list positions | 83/83 |
| cargo tests | pass |

The sweeps run iserve, which now prepares restores after each compile, so they cover the
prepared restores.

### Host memory, a finding that predates this lane (VERIFIED, NixOS PC)

Peak RSS (`VmHWM`) of the socket host, typing on page 1:

| engine | document | peak RSS |
|---|---|---|
| `15f96b026` (before this lane) | plain-1000 | 7.5 GB |
| `b02654d8c` | plain-1000, with and without prepared restores | 5.4 GB |
| `b02654d8c` | full-1000, 10 keystrokes | 20 GB |

P4-L5 had reported 0.4–0.6 GB on the Mac. On the Mac, the book and full-1000 hosts of these runs
pushed the machine into 4.5 GB of swap. That is why the last book runs are not quoted here. This
needs its own lane.

## Reproducing

```
cargo build --release -p flashtex-engine -p flashtex-display-list
export INCR_BENCH_DIR=/tmp/p4f
tools/incr-bench/mkeng.sh NAME && python3 tools/incr-bench/mkdocs.py
python3 tools/incr-bench/matrix.py NAME OUT && python3 tools/incr-bench/convergence.py OUT
tools/incr-bench/ab_engines.sh plain-10 0 300 2 "a=NAME:" "w=NAME:--keep-warm 400"
tools/incr-bench/keys_matrix.sh NAME TAG ~/Documents/FlashTeX-1000-page-test/book.tex
python3 tools/incr-bench/keys_sum.py TAG ~/Documents/FlashTeX-1000-page-test/book.tex
tools/incr-bench/gates.sh            # Linux, TeX Live 2026: parity ... soundness A/C/D/book
```

The `raw/book/*.jsonl.gz` and `raw/keys/*.jsonl.gz` files are gzipped. `keys_sum.py` reads them
unpacked.
