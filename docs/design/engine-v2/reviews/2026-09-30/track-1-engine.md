# Nightly design review 2026-09-30 — Track 1: engine, incremental system, performance, maintainability

**Scope:** DESIGN.md §14 steps 1–4 for the engine (`crates/flashtex-engine`, `tools/web2rust`),
the incremental system (§5), performance (§1.2, §4.2, §5.6) and engine maintainability. It is an
input to the Commander's review file; it changes no product code and no DESIGN.md text.
**Reviewer:** claude-opus-5-5 (kabir-claude NIGHTLY-REVIEW-T1, mac-m5pro-kabir), 2026-09-30.

**What was measured.**

| label | commit | what |
|---|---|---|
| `main` | `02dcf9d07` | origin/main at the start of the review |
| `p4f` | `8b8101c9f` | lane P4-FINISH's local head at 01:48 EDT: main + #1230 + #1241 merged (`15f96b026`) + 3 lane commits. The lane has since moved to `f043785e2` (local only). |
| #1230 | `dab3e2fe6` | L6 hyperref intrinsics (read, not built alone) |
| #1241 | `003528a62` | L6 optimisations; stacked on #1230, merge base `1c19f22b8` (old p4-l2-l3 head) |
| #1254 / #1255 | `db0dd708b` / `75c65cc95` | app host follow-ups / diagnostics (read) |
| P3-FONTS-2 | `02dcf9d07` | the lane branch has no commits yet |

**Host:** the NixOS PC (AMD Ryzen 7 7800X3D, 16 threads, 30 GiB, rustc 1.97.1, gcc 15.3,
TeX Live 2026 at `~/texlive/2026`). It was **shared the whole time**: two CI runners, lane
P4-FINISH's soundness sweeps, P3-FONTS-2's build and another review's arXiv run. The 1-minute load
was 10–53. So wall and CPU seconds are upper bounds. `perf stat` user-space **instruction counts
are load-independent** and carry the comparisons; cycles move less than seconds. Nothing heavy ran
on the Mac.

**Labels.** VERIFIED = measured or read by me in this review (the command and raw file are
named). REPORTED = taken from a lane's evidence or PR without re-measuring. BELIEF = my inference.

Raw files are in [`raw/t1/`](raw/t1/): `gates-linux.txt`, `bench-linux.jsonl`, `host-memory.txt`,
`preamble-edit.jsonl`, `convergence-*.md`, `upgrade-dry-run.txt`, `loc-main-02dcf9d07.txt`, and
the scripts that produced them (`bench2.py`, `convergence.py`, `preamble_edit.py`,
`upgrade-dry-run.sh`, `chcheck.py`, `loc.py`).

---

## 0. Findings ranked by impact

| # | finding (evidence in the section named) | proposed action | decides |
|---|---|---|---|
| 1 | **CI does not gate the new engine's parity.** CI runs trip, etrip, pdfTeX's regression tests and `cargo test`. It does not run P-T1/P-T2 on the fixtures for `flashtex-initex` (the CI "parity fixtures" job builds the *old* `flashtex-cli`), lockstep (T1), the T2 LaTeX suites, incremental soundness or the T7 latency matrix. P2's "verified" gate is therefore a one-off lane measurement, and #1230/#1241 (≈ 40k changed lines with regenerated code and evidence; unchecked reads) would land with lane-run evidence only (§2.1). | PR tier on the NixOS runners: lockstep (58–68 s here, loaded) and P-T1/P-T2 on the 83 fixtures for `flashtex-initex` against the runner's TeX Live 2026 pdfTeX (135 s at `-j 6`, loaded). Nightly: soundness sweep, `checked-arrays` lockstep+parity, T2, `FLASHTEX_INTRINSICS=verify`. | Commander (refinement) |
| 2 | **L3 convergence rarely succeeds on hyperref documents, and nothing measures it.** P4-L5's own matrix: full-* single-character edits converge in 8–22 of 48 compiles, sentence insertions in 0 of 18 (plain: 30–40 / 48). Every non-converged edit re-typesets to the end in the background: full-1000 p95 5.7 s of engine time per keystroke. On the owner's 1,072-page document 943 pages (5.5–6.9 s) per keystroke (P4-FINISH `f043785e2`, REPORTED). The main causes are false negatives: PDF link/dest whatsit words, the monotone `pdf_char_used` set, and statistics. **#1230's intrinsics add a new one:** `intr_state[...]` blocks 10 full-1000 convergences on p4f. The PC reproduces the Mac's rates, so the rate is deterministic and gateable (§3.2). | Put the convergence rate and background pages per edit into the T7 gate and the P4 exit gate. Fix the false-negative classes in order of count: link/dest whatsit words (P4-FINISH has started on this), `pdf_char_used` as a per-page external effect (union at the end), the intrinsics' state as derived state, relocatable object numbers (as §5.3 already specifies). | Commander (refinement) |
| 3 | **DESIGN.md mis-states what was built** (§14 step 5: "nothing may remain that the review found to be wrong"). D8/§5.3: there is no incremental 128-bit state hash, and object/font numbers are not relocatable; the comparison is a word diff plus a structural isomorphism (`iso.rs`). D7: checkpoints are now also taken between pages (0.5 ms of engine time apart, 2.2 a page). §4.1 step 2 ("remove globals into an `Engine` struct") is not being done, and rightly so: generated code is never edited. §8 T0: "28 CTAN regression directories" is really TeX Live's 7 `pdftex_tests`. §5.6: "full-1000 4.1 s vs pdflatex 6.3 s" is a branch result, not main. §9.2: "parity fixtures" in the PR tier is the old engine's (§4). | Update §5.3, §4.1, §8, §5.6 and §9.2, and record the lanes' deviations in §13. The **D8 wording** (hash → comparison, relocation not yet built) and the **D3 step 2 wording** are D-number text: propose them to the owner. | Owner (D8, D3 text); Commander (the rest) |
| 4 | **#1241's unchecked reads trade the no-panic contract for 1–8 %.** On x86 the lane measured −1.3 to −3.2 % CPU; on the M5 −5 to −8 % cycles. That is not noticeable under §1's priorities. The PR text says an out-of-range read "reads a neighbouring array", but `get_unchecked` past a slice is undefined behaviour, and a negative index cast to `usize` leaves the mapping altogether (§3.4). | Keep reads checked in product builds until nightly `checked-arrays` runs and T6 fuzzing have run clean. Better: check against compile-time capacities (`consts.rs`), which removes the length reload the lane identified as the cost. | Commander (refinement) |
| 5 | **Stacked, stale PR bases.** #1241 is stacked on #1230 with merge base `1c19f22b8`. Built alone, its binary cannot find TeX Live on the PC (no `pdflatex.ini`, no tcx: it predates main's +266-line resolver discovery), so its Linux numbers describe an old base. #1254 is stacked on #1247. This breaks DESIGN §9.5 ("no stacking"). | Land #1230, then merge main into #1241 and re-run its gates. `p4f` already did this merge: lockstep 260/260, P-T1 83/83, P-T2 83/83 (VERIFIED here). | Commander |
| 6 | **Hand-copied pdftex.web knowledge is the upgrade hazard; the change files are not.** The dry run replayed pdfTeX 1.40.28→1.40.29 against our change files: 1 of 174 hunks broke, a re-indent fixed it, and web2rust regenerated cleanly. But about 290 pdftex.web numeric macros are hand-copied (`iso.rs` 114, #1230's `intrinsics.rs` 148, `readset.rs` 14, `checkpoint.rs` 9). Hand-proved "dead word" rules (`incr.rs:675`, P4-FINISH `f043785e2`) cite pdftex.web sections, and the 1.40.29 patch changed exactly the text-mode code `dead_word` reasons about (§4.2). | web2rust emits every numeric `@d` as a `const` in `generated/`, and hand-written code imports them. Each hand-proved rule records the pdftex.web § it relies on plus a hash of that section's text, and a unit test fails when an upgrade changes one. | Commander (refinement) |
| 7 | **P4 exit-gate items are unmeasured or unowned.** "Preamble edit ≤ 400 ms to first page" has no measurement anywhere in `docs/evidence/`; here, engine side: 264–461 ms for full-*, 70–100 ms for plain. "Reopen ≤ 100 ms" is met only in a pre-warmed host (fresh process 150–450 ms, REPORTED P4-L2-L3). T7-LATENCY-GATE sits third on mac-claude-a's list, behind old-pane tiles. The P4 benchmark and soundness harnesses exist only as evidence scripts copied between five directories with hard-coded `/tmp` and Mac worktree paths (`incr_bench.py:206`) (§5). | Make P4-FINISH own T7 by promoting its scripts to `tools/incr-bench/` (matrix, soundness, preamble, reopen, convergence) and dropping the duplicate harness planned in `tools/latency-bench/`. Decide explicitly whether a pre-warmed host counts for "reopen". | Commander; the reopen interpretation → owner |
| 8 | **Effort not on the P3–P5 critical path.** #1228 (tiles for the old v2 pane), #1252 (perf audit of the old pane), old-engine *feature* PRs still open (#1136, #1151, #1157, #1159, #1181: new symbols and commands, contrary to D13), the Typst design (#1250) and the cross-platform set (#1244–#1246). The critical path is: P3 = P3-FONTS-2 (no commits yet) + landing #1247/#1254; P4 = items 2 and 7; P5 = #1224 + the retirement plan (§5). | Close the D13-violating PRs. Pause old-pane work. Put T7 and P3-FONTS-2 first. | Commander |
| 9 | **Duplication worth deleting:** two `Session`s (`host/mod.rs:282`, the L1-only one, used only by `host/tools.rs` measuring subcommands; `incr.rs:1370` is production); `tools/snapshot-bench` (5,167 lines, research for a closed decision); about 2,400 lines of copied evidence scripts (§4.4). | Route `host/tools.rs` through `incr::Session` and delete the L1 `Session` (about 500 lines). Archive snapshot-bench. The evidence scripts go away with item 7's promotion. | Commander |
| 10 | **Minor:** (a) with another TeX Live's `kpsewhich` first on `PATH` (Nix's TL 2025 here), the engine silently falls through to INITEX's `**` prompt and "End of file on the terminal", instead of saying which TeX Live it picked and what it could not find. (b) Three `flashtex-host iserve` sessions on main (a preamble-edit session on full-1000, and matrix sessions full-300 start/char and full-1000 middle/char) ended with the host gone and nothing captured on stderr. Reruns (3 preamble edits; the full-1000 session with the same seed) passed; cause unknown, and no kernel segfault or OOM record. (c) Two lanes on the PC used the same script name (`pc-gates.sh`), so a `pkill -f` by one can kill the other's run. | (a) A diagnostic naming the discovered TeX Live. (b) Harnesses keep host stderr; re-run the soundness sweep on the PC. (c) Name scripts per lane. | Commander |

The rest of this file is the evidence behind each row.

---

## 1. Audit and re-measurement

### 1.1 Code size (VERIFIED, `raw/t1/loc-main-02dcf9d07.txt`, `loc.py`)

| part of `crates/flashtex-engine` on main | lines |
|---|---|
| `src/generated/` (web2rust output, committed, drift-gated) | 59,852 |
| hand-written Rust, total | **35,696** |
| — `src/pdftex/` (pdfTeX's C parts, ported per file) | 11,667 |
| — `src/displaylist/` | 3,377 |
| — `src/incr.rs` (L2–L5) | 3,272 |
| — `src/host/` | 3,145 |
| — `src/system.rs` (tex.ch / texmfmp.c layer) | 3,073 |
| — `src/iso.rs` (structural state comparison) | 2,466 |
| — `src/arena.rs` (word space, checkpoints' storage) | 2,076 |
| — `src/bundle/`, `formats.rs`, `resolver.rs` (distribution) | 3,186 |
| — `checkpoint.rs`, `readset.rs`, `persist.rs`, `statediff.rs`, others | 3,434 |
| change files (`changes/*.ch`, ours) | 2,236 |
| C/C++ shims (`csrc/`, `kpathsea-config/`) | 1,233 |
| engine integration tests (`tests/*.rs`) | 3,909 |
| `tools/web2rust/src` | 5,493 |
| harnesses: `tools/parity` 3,740, `tools/lockstep` 3,119, `tools/snapshot-bench` 5,167, `crates/display-list-v3` 3,141 | |

Open PRs add to the hand-written part: #1230 `intrinsics.rs` 1,658, `intrinsics_verify.rs` 699,
`macroprof.rs` 223 and `changes/intrinsics.ch` 435; #1241 `ix.rs` 97; #1255 `diag.rs` 1,262,
`host/diag.rs` 778 and `changes/diagnostics.ch` 124. With them the hand-written engine is about
40k lines against 60k generated.

**Largest functions.** Hand-written: `displaylist/interp.rs:589 op` 322 lines;
`pdftex/pdftoepdf.rs:873 write_epdf` 288; `host/resident.rs:396 compile` 288;
`pdftex/writet1.rs:117 standard_glyph_name` 278 (a table); `incr.rs:940 check_mem` 262;
`incr.rs:2871 after_run` 222. 36 of 1,544 hand-written functions exceed 100 lines, 6 exceed 200.
Generated: `close_files_and_terminate` 1,737, `print_cmd_chr` 1,562, `pdf_ship_out` 1,343,
`main_control` 1,213. Those are pdftex.web's own modules expanded, and nobody edits them.

**Unused code.** The hand-written engine has one `#[allow(dead_code)]` (`incr.rs:3271`). The
generated code allows everything, as it must. Code kept on purpose but not in the product path: the
L1-only `host::Session` (§4.4), `host/tools.rs` (584 lines of measuring subcommands), the
`bench-no-barrier` and `bench-count-writes` features, and `tools/snapshot-bench`.

**Duplication.** Inside the engine there is **no second TFM reader**. Fonts come only through the
generated `read_font_info`. The four TFM readers of DESIGN Appendix B.2 are old-engine crates,
frozen and scheduled for P5 retirement. The real duplicates are in §4.4.

### 1.2 Tests, gates and their run times (VERIFIED, `raw/t1/gates-linux.txt`)

| gate | main `02dcf9d07` | p4f `8b8101c9f` | time (loaded PC) |
|---|---|---|---|
| trip | pass | – | 19 s |
| etrip (TeX Live's filters) | pass (all PASS lines) | – | – |
| pdfTeX regression (`pdftex_tests`) | **7/7** | – | 4 s |
| web2rust drift | pass | – | 1 s (built) |
| lockstep | **260/260**, accounting 0 | **260/260**, accounting 0 | 68 s / 58 s |
| P-T1 / P-T2, 83 fixtures (oracle: the PC's TeX Live 2026 pdfTeX 1.40.29) | **83/83 / 83/83**, L3 98.8 % | **83/83 / 83/83**, L3 98.8 % | 138 s / 135 s (`-j 6`) |
| `cargo test --release -p flashtex-engine` | **74 passed**, 2 ignored | – | 139 s incl. build |
| `cargo test --release -p web2rust` | 10 passed | – | 2 s |

- P-T1/P-T2 hold on a second OS and architecture. That is new evidence: every earlier P-T1 run was
  on the Mac.
- The ignored tests are `arena::tests::seal_cost` (a measurement) and
  **`every_fixture_edits_equal_scratch_compiles`**: the fixture-wide incremental soundness test is
  `#[ignore]`d, so no `cargo test` in CI runs it.
- On Linux `tools/parity` defaults its oracle to `/Library/TeX/texbin/pdftex` (`tiers.py:35`) and
  silently reports `P-T1 n/a` when that is missing (my first run). It needs `--oracle-pdftex` and
  `--texbin`, which #1232 wires into nightly.
- T2 (LaTeX suites) was **not** re-run here. REPORTED at P2: 1,520/1,529 with 0 unexpected
  failures.

### 1.3 Engine build times (VERIFIED, NixOS PC, `CARGO_BUILD_JOBS=12`, loaded)

| build | seconds |
|---|---|
| clean `cargo build --release -p flashtex-engine --bins` (fresh target dir) | 52.6 |
| … after touching one generated file (`body_3.rs`) | 38.2 |
| … after touching one hand-written file (`incr.rs`) | 27.3 |
| `cargo test -p flashtex-engine --no-run` (debug, after the release build) | 24.4 |
| #1241 clean release (fresh target dir) | 63.9 |

The engine is one 95k-line crate, so every edit recompiles all of it (27–38 s of release build).
That is within §9's 10-minute PR target. Splitting it is not worth doing now (BELIEF): the
generated `impl Globals` blocks and the hand-written modules call each other in both directions.

### 1.4 Full runs against pdflatex (VERIFIED, `raw/t1/bench-linux.jsonl`, `bench2.py`)

Each document is settled first (`.aux` stable). Then 5 interleaved runs per engine, medians, user
space `perf stat`, load 19–27. The documents are P4-L2-L3's generator (`plain-N`: article, amsmath;
`full-N`: plus hyperref, siunitx, cleveref, xcolor, footnotes) and a one-line `hello`.

| doc | pdflatex instr / cycles / CPU | main instr (×) / cycles (×) / CPU | p4f instr (×) / cycles (×) / CPU |
|---|---|---|---|
| hello | 1.35G / 0.84G / 0.23 s | 1.60G (1.19) / 0.82G (0.98) / 0.23 s | 1.52G (1.13) / 0.79G (0.94) / 0.23 s |
| plain-10 | 1.72G / 1.05G / 0.28 s | 2.20G (1.28) / 1.12G (1.07) / 0.30 s | 2.05G (1.19) / 1.08G (1.03) / 0.29 s |
| plain-100 | 2.66G / 1.67G / 0.41 s | 4.21G (1.58) / 2.21G (1.32) / 0.54 s | 3.45G (1.30) / 1.89G (1.13) / 0.47 s |
| plain-1000 | 12.12G / 8.10G / 1.81 s | 24.33G (2.01) / 12.78G (1.58) / 2.83 s | 17.67G (1.46) / 10.23G (1.26) / 2.28 s |
| full-10 | 4.19G / 2.39G / 0.57 s | 6.07G (1.45) / 2.93G (1.22) / 0.69 s | 5.25G (1.25) / 2.72G (1.14) / 0.65 s |
| full-100 | 9.90G / 5.89G / 1.33 s | 16.01G (1.62) / 7.84G (1.33) / 1.76 s | 10.42G (1.05) / 5.69G (0.97) / 1.30 s |
| full-1000 | 67.30G / 41.51G / 9.04 s | 115.92G (1.72) / 59.22G (1.43) / 12.89 s | **59.92G (0.89) / 35.01G (0.84) / 7.63 s** |

Peak RSS: pdflatex 53–57 MB; main 75–80 MB; p4f 76–83 MB.

Against the record:
- **DESIGN §5.6 item 4** states "full-1000 … faster than pdflatex (4.1 s vs 6.3 s)" as a result.
  On **main** the engine is still 1.43× pdflatex's cycles on full-1000 (1.72× instructions): the
  intrinsics are not merged. With them (`p4f`) it is 0.84× cycles, which confirms the direction.
  **Mis-stated as achieved.**
- #1241's evidence (x86, quiet): plain-1000 17.3G instructions against pdflatex's 12.1G (1.43×).
  Here p4f: 17.67G / 12.12G (1.46×). **Consistent.**
- On x86 FlashTeX still loses plain documents: 1.13–1.26× cycles on p4f. That does not matter for
  §1.2 (keystroke to pixels). It matters for cold full runs and export.

### 1.5 Memory (VERIFIED, `raw/t1/host-memory.txt`)

`flashtex-host bench --reps 0`, one cold run, max RSS:

| | none | a checkpoint after every shipout | checkpoint CPU per page |
|---|---|---|---|
| main plain-1000 | 85 MB | 284 MB | 0.39 ms |
| main full-1000 | 91 MB | 340 MB | 0.73 ms |
| p4f plain-1000 | 86 MB | 285 MB | 0.45 ms |
| p4f full-1000 | 96 MB | 343 MB | 0.53 ms |

These are single samples on a loaded machine. They agree with P4-L2-L3's Mac figures (337 MB,
0.41 ms). With P4-L5's between-page checkpoints the Mac measured 382 → 638 MB RSS on full-1000
(REPORTED). Both are inside the 1 GB budget.

### 1.6 Incremental, engine side

P4-L5's latency matrix driver (`matrix.py` → `incr_bench.py`, unchanged apart from two path
fixes), run on the PC: main on all eight documents, p4f on the 1,000-page ones. Load was 13–53, so
**wall figures are upper bounds and not comparable with the Mac's**. The raw records are in
`raw/t1/edited-page-pc.md` and `convergence-pc-*.md`. Two main sessions (full-300 start/char and
full-1000 middle/char) ended with the host gone; a rerun of the second with the same seed passed
(finding 10b).

| engine | doc | compiles | edited page wall p50 / p95 ms | edited page CPU p50 / p95 ms | restore p50 / p95 ms |
|---|---|---|---|---|---|
| main | full-10 | 66 | 8.7 / 17.3 | 8.2 / 16.6 | 0.6 / 0.9 |
| main | full-100 | 66 | 8.7 / 14.9 | 7.9 / 13.7 | 1.1 / 1.7 |
| main | full-300 | 50 | 16.5 / 29.4 | 13.0 / 18.2 | 3.5 / 6.0 |
| main | full-1000 | 50 | 36.5 / 56.5 | 26.5 / 34.5 | 11.4 / 32.2 |
| main | plain-10 | 66 | 6.2 / 12.4 | 3.4 / 11.8 | 1.6 / 5.1 |
| main | plain-100 | 66 | 9.5 / 27.2 | 5.6 / 14.3 | 3.0 / 6.4 |
| main | plain-300 | 66 | 7.9 / 13.9 | 4.7 / 8.9 | 3.9 / 8.1 |
| main | plain-1000 | 66 | 16.5 / 51.0 | 9.5 / 44.0 | 9.5 / 19.0 |
| p4f | full-1000 | 66 | 35.1 / 48.4 | 19.3 / 24.6 | 16.4 / 30.6 |
| p4f | plain-1000 | 66 | 12.1 / 39.3 | 7.6 / 30.6 | 5.9 / 15.3 |

- On this loaded x86 box, 1,000-page edits miss 16 ms even in CPU: full-1000 p50 19–27 ms CPU.
  Restores take 6–16 ms p50, against 0.6–3 ms on the Mac, because the 8 restore workers compete
  with the load.
- The Mac evidence (P4-L5: every document ≤ 14.1 ms CPU p95) is therefore not transferable. **The
  T7 gate needs a quiet, dedicated Mac runner that records its load.** The shared PC cannot be the
  latency gate host.
- For a future Linux target (DESIGN §16), full-1000 needs about 2× on this path. The P4-L5 README's
  far-restore and per-page floors are where it would come from.
- p4f against main on full-1000: CPU p50 19.3 against 26.5 ms, the intrinsics' gain.

### 1.7 CI health (VERIFIED, `gh run list`, runs since 2026-09-29)

- `main` pushes: 24 success, 5 failure, 1 cancelled of 30. Median 7.6 min, p90 99 min (queued self-hosted runs).
- Merge queue: 24 success, 11 failure, 2 cancelled, 3 running of 40. Median 24 min, p90 40 min.
  The failing jobs are spread out: mac app 3, quick 3, iPad 2, rust workspace 2+2, flashtex-cli 2,
  render-pipeline, trip, etrip and parity fixtures 1 each. No single engine gate dominates.

---

## 2. Correctness risks

### 2.1 What CI actually guards for the new engine (VERIFIED, `.github/workflows/ci.yml`, `.github/actions/parity-fixtures/action.yml:34`)

| DESIGN §8 tier | in CI? |
|---|---|
| T0 unit tests, web2rust round-trip (drift), trip, etrip, pdfTeX regression | yes (drift and unit tests only in the `full` rust-workspace job, i.e. the merge queue) |
| T1 lockstep | **no**: not in `ci.yml` and not in `scripts/gate.sh pr` |
| T2 LaTeX suites | **no** |
| T3 parity fixtures P-T1/P-T2 | **no for the new engine**: the job builds `crates/flashtex-cli` (old engine) |
| T6 fuzzing | not yet (#1234 open) |
| T7 latency | **no** (evidence scripts only) |
| incremental soundness (L1–L5) | 8 + 5 small tests (`tests/incremental.rs`, `tests/host_incremental.rs`); the fixture-wide test is `#[ignore]` |

The PR tier I propose costs about 3.5 minutes on the NixOS runner at the load measured here:
lockstep plus P-T1/P-T2 on the 83 fixtures. It keeps P2's gate true after every landing. Nightly
should add: the soundness sweep (A/C/D) on generated 120/300-page and hyperref documents; lockstep
and parity with `--features checked-arrays` (#1241's own recommendation, not wired up);
`FLASHTEX_INTRINSICS=verify` on the fixtures (#1230's both-paths diff); `FLASHTEX_CHECKMEM` (the
root-set completeness walk, `incr.rs:937`); and T2.

### 2.2 Soundness assumptions of the incremental system, and their tests

Convergence (`incr.rs:1–30`, `same_words` at `incr.rs:560`) accepts that the new run has reached
the old run's state at checkpoint *k* when all of these hold:
(a) the same input position, modulo the edit's length;
(b) no changed file is read by the old run later, and no file written by the runs is read later;
(c) pdfTeX's C-part state is equal (`cstate.same_as`);
(d) the word space is equal except for a whitelist, or else structurally isomorphic (`iso.rs`).

What that rests on, and how well each part is tested:

| assumption | where | tested by | gap and proposed test |
|---|---|---|---|
| Every engine state lives in the word space, the C-part record or the file journal | `checkpoint.rs`, `arena.rs`, `pdftex/*` Rust caches (`MapCache`, `Fonts`, images) | soundness A–D, 13,743 compiles, 0 mismatches (REPORTED P4-L5); `tests/incremental.rs` | Rust-side caches outside the word space are covered only by that sweep. **Test:** edit `pdftex.map`, or replace a font, image or `\input` file *between* compiles while the main file is unchanged, and require from-scratch equality. |
| The dead-word whitelist (`dead_word`, `incr.rs:675`), file positions (`position_only`, `incr.rs:1209`), free-memory cells (`drop_free_mem`) | hand-proved from pdftex.web § citations | the sweep (statistical) | Each rule is a proof about pdfTeX's code. **Test:** pin the pdftex.web sections each rule cites (a hash of their text) in a unit test, so an upgrade forces a re-review (§4.2). |
| The structural walk's root set is complete | `iso.rs` roots; `FLASHTEX_CHECKMEM` | manual (`checkmem` command) | **Test:** run `checkmem` after every page of the fixtures, nightly. |
| Barriers: `\write18`, `\pdfelapsedtime`, `\pdffilemoddate` of a changed file, `\read` of a file written in the run | `incr.rs` (b'), barrier tests | `barrier_test.py` (evidence), `tests/incremental.rs` | **Test:** a restricted `\write18` (epstopdf) that regenerates a PDF which a later page includes. |
| Line-granularity input consumption (`\futurelet` lookahead) | checkpoint host record | the sweep's random letters | Edits that land exactly at line ends, inside `\verb` or after catcode changes are rare in random letter edits. **Test:** targeted edits at those positions. |
| Preemption leaves a restorable state | `set_preempt` | soundness D, 1,469 compiles; one socket test | Open item 5 of P4-L5: an edit arriving during a convergence test. **Test:** preempt at every stage (restore, run, test). |
| Intrinsics replay equals expansion; the guard sees every write | #1230 `intrinsics.rs` (state in the word space, good) | `verify` mode, 0 differences in 53,401 calls (REPORTED); fault injection (`Fault::NoReadset` and others, `intrinsics.rs:348`) | Evidence only. **Test:** make the fault-injection runs `cargo test`s; run `verify` nightly. **Measured here: `intr_state[...]` blocks 10 full-1000 convergences on p4f (§3.2).** Measure the convergence rate with intrinsics on versus off in T7. |
| #1241: reads in range | `ix.rs` | debug-build tests only | See §3.4. |

### 2.3 Change files instead of hand edits: the right call (VERIFIED, §4.2)

All engine behaviour beyond pdftex.web is 2,236 lines of our change files plus TeX Live's
unmodified ones. That is how TANGLE and web2c work, and the upgrade dry run shows why it pays off:
a real release delta broke 1 of 174 hunks, and the fix is mechanical. DESIGN §4.1's step 2
("modernise in place … remove globals into an `Engine` struct and use typed indices") has not been
done and should not be. Hand-modernising 60k generated lines would turn every upgrade into a
60k-line merge. The project already does modernisation through translator options instead
(`--index-type` in #1241, `--arena-cap`, `--scalar`). Recommend rewording D3 step 2 to that
effect (owner: it is D-number text).

### 2.4 Drift-gate coverage (VERIFIED)

Covered:
- `src/generated/` and `pdftex.pool` against `web2rust-default.args` (`tools/web2rust/tests/drift.rs`);
- the trip and etrip configurations, which are regenerated by their scripts;
- the pinned upstream files (`shasum -c` of `third_party/pdftex` in the etrip job, `ci.yml:543`).

Not covered:
- the ~290 hand-copied pdftex.web constants. Only #1230's `webconsts.py` evidence script checks
  its own, and an `iso.rs` unit test checks the eqtb region bounds;
- the hand-proved whitelist rules (§2.2).

---

## 3. The incremental system in detail

### 3.1 D8 as specified and as built (VERIFIED, `incr.rs`, `arena.rs:1159`, `checkpoint.rs:494`)

DESIGN §5.3 specifies "the canonical 128-bit state hash … updated on every `eq_define` …
independent of addresses, string numbers, hash slots and font indices", and says "PDF object
numbers and font numbers are relocatable".

What is built:
- An on-demand comparison. `Arena::diff_branch` (`arena.rs:1371`) finds the chunks either run
  wrote; words are then dropped as dead, position-only or free.
- When allocation differs, `iso.rs` walks both states from the roots in lock step: a bijection that
  compares every data field.
- `state_hash` (`checkpoint.rs:494`) is a hash of the raw nonzero chunks, used only by the
  measuring tools.
- Object and font numbers must be **equal**. Only PDF file offsets are relocated.

P4-L2-L3 recorded this deviation in its README ("no running state hash"; "PDF object and font
numbers are not relocatable"). DESIGN §5.3, D8 and §13 were never updated.

The built design is sound by construction: it cannot collide, and any differing word outside the
walk fails the test. It costs 2–10 ms per test (REPORTED). The owner should see the D8 text change.

### 3.2 Convergence rates and what blocks them (VERIFIED from P4-L5's raw matrix, `raw/t1/convergence-p4l5-1773a1a61.md`)

P4-L5's final matrix (engine `1773a1a61`, Mac, 528 compiles), re-analysed with `convergence.py`:

| doc | compiles | converged (char / sentence) | compile total p50 / p95 / max s | re-run pages p50 / max | edited page CPU p95 ms |
|---|---|---|---|---|---|
| full-10 | 66 | 14 (14/48 / 0/18) | 0.031 / 0.156 / 0.18 | 3 / 22 | 12.5 |
| full-100 | 66 | 10 (10/48 / 0/18) | 0.296 / 0.630 / 0.66 | 49 / 111 | 10.4 |
| full-300 | 66 | 8 (8/48 / 0/18) | 0.876 / 1.896 / 1.91 | 147 / 292 | 9.5 |
| full-1000 | 66 | 22 (22/48 / 0/18) | 0.389 / 5.681 / 5.88 | 64 / 920 | 14.1 |
| plain-10 | 66 | 34 (30/48 / 4/18) | 0.019 / 0.050 / 0.05 | 2 / 10 | 5.0 |
| plain-100 | 66 | 25 (18/48 / 7/18) | 0.034 / 0.130 / 0.13 | 5 / 95 | 6.3 |
| plain-300 | 66 | 52 (36/48 / 16/18) | 0.041 / 0.202 / 0.39 | 5 / 294 | 3.1 |
| plain-1000 | 66 | 52 (40/48 / 12/18) | 0.040 / 1.242 / 1.92 | 3 / 1552 | 10.5 |

The reason the last convergence test failed, for compiles that changed exactly one page in one pass
and still did not converge (which is exactly where convergence should succeed):

| kind | count | reason |
|---|---|---|
| full | 84 | `page N: structures differ: whatsit N word N differs` |
| plain | 36 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| full | 28 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| plain | 14 | `page N: structures differ: glue set differs (walking Some((List, N, N)))` |
| plain | 14 | `page N: structures differ: char node differs (N vs N) (walking Some((List, N, N)))` |
| full | 8 | `page N: structures differ: token differs (N vs N) (walking Some((Chain, N, N)))` |
| full | 6 | `page N: N differs outside what the structural comparison reads: pdf_ptr: N -> N` |
| plain | 6 | `page N: N differs outside what the structural comparison reads: max_buf_stack: N -> N` |
| full | 4 | `page N: N differs outside what the structural comparison reads: pdf_os_buf[N]: N -> N` |
| full | 2 | `page N: N differs outside what the structural comparison reads: hc[N]: N -> N` |
| plain | 2 | `page N: N differs outside what the structural comparison reads: best_height_plus_depth: N -> N` |

Reading the classes (pdfTeX subtype numbers: whatsit 16 = `pdf_start_link_node`, 19 =
`pdf_dest_node`):
- **Link and destination whatsits (84, all hyperref).** These carry object numbers, and for
  non-`fitr` destinations they also hold uninitialised words. P4-FINISH `f043785e2` fixes the dest
  case (REPORTED): on book.tex a letter at page 130 now converges at page 162 instead of never.
  Object numbers remain non-relocatable.
- **`pdf_char_used` (64).** A monotone per-font set of the characters shipped so far. An edit that
  uses a glyph for the first time (or removes the only use) makes every later checkpoint differ,
  forever. It only feeds font subsetting at the end, so it can be kept per page and unioned, like
  the other per-page external effects of §5.2.
- **Statistics** (`max_buf_stack` and friends, 6+). P4-FINISH `e3a7acc77` now treats the stacks'
  high-water marks as accounting (REPORTED).
- **Glue set / char node differences (28, plain).** A reflow that has not settled by the page
  tested. Genuine, and expected.

Cost of a miss: the background runs to the end. full-1000: p50 0.39 s, p95 5.7 s of engine time per
compile; up to 920 pages re-run. The edited page is unaffected (viewport first), but later pages
stay stale that long, the laptop burns the CPU, and any edit made meanwhile has to preempt it.
**Nothing in the P4 gate or T7 measures this.**

My re-run of the same matrix on the PC (main and p4f) is below:

**main** (PC, same seeds):

| doc | compiles | converged (char / sentence) | compile total p50 / p95 / max s | re-run pages p50 / max | edited page CPU p95 ms |
|---|---|---|---|---|---|
| full-10 | 66 | 16 (16/48 / 0/18) | 0.043 / 0.211 / 0.23 | 3 / 22 | 16.6 |
| full-100 | 66 | 10 (10/48 / 0/18) | 0.438 / 0.926 / 0.99 | 49 / 111 | 13.7 |
| full-300 | 50 | 6 (6/32 / 0/18) | 1.274 / 3.111 / 3.96 | 147 / 292 | 18.2 |
| full-1000 | 50 | 12 (12/32 / 0/18) | 5.538 / 13.376 / 13.56 | 493 / 920 | 34.5 |
| plain-10 | 66 | 34 (30/48 / 4/18) | 0.042 / 0.126 / 0.15 | 2 / 10 | 11.8 |
| plain-100 | 66 | 25 (18/48 / 7/18) | 0.087 / 0.316 / 0.34 | 5 / 95 | 14.3 |
| plain-300 | 66 | 52 (36/48 / 16/18) | 0.106 / 0.469 / 0.96 | 5 / 294 | 8.9 |
| plain-1000 | 66 | 52 (40/48 / 12/18) | 0.124 / 3.107 / 4.68 | 3 / 1552 | 44.0 |

Last failed convergence test of compiles that changed one page in one pass and did not converge:

| kind | count | reason |
|---|---|---|
| full | 66 | `page N: structures differ: whatsit N word N differs` |
| plain | 36 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| full | 26 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| plain | 14 | `page N: structures differ: glue set differs (walking Some((List, N, N)))` |
| plain | 14 | `page N: structures differ: char node differs (N vs N) (walking Some((List, N, N)))` |
| full | 10 | `page N: structures differ: token differs (N vs N) (walking Some((Chain, N, N)))` |
| full | 6 | `page N: N differs outside what the structural comparison reads: pdf_ptr: N -> N` |
| plain | 6 | `page N: N differs outside what the structural comparison reads: max_buf_stack: N -> N` |
| full | 4 | `page N: N differs outside what the structural comparison reads: pdf_os_buf[N]: N -> N` |
| full | 2 | `page N: N differs outside what the structural comparison reads: hc[N]: N -> N` |
| plain | 2 | `page N: N differs outside what the structural comparison reads: best_height_plus_depth: N -> N` |

**p4f** (main + #1230 + #1241 + P4-FINISH's first fixes), 1,000-page documents:

| doc | compiles | converged (char / sentence) | compile total p50 / p95 / max s | re-run pages p50 / max | edited page CPU p95 ms |
|---|---|---|---|---|---|
| full-1000 | 66 | 18 (18/48 / 0/18) | 4.350 / 8.281 / 8.53 | 493 / 920 | 24.6 |
| plain-1000 | 66 | 54 (40/48 / 14/18) | 0.076 / 2.029 / 3.73 | 3 / 1552 | 30.6 |

Last failed convergence test of compiles that changed one page in one pass and did not converge:

| kind | count | reason |
|---|---|---|
| full | 10 | `page N: N differs outside what the structural comparison reads: intr_state[N]: N -> N` |
| full | 8 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| plain | 8 | `page N: N differs outside what the structural comparison reads: pdf_char_used[N]: N -> N` |
| full | 6 | `page N: structures differ: whatsit N word N differs` |

- **Convergence is deterministic.** The PC reproduces the Mac's counts almost exactly: plain-1000
  52/66 on both; full-10 16 against 14. So the rate is a load-independent metric a gate can hold.
- **The intrinsics add a new false negative (VERIFIED).** On p4f, 10 of the full-1000 failures stop
  on `intr_state[...]`, #1230's recording state in the word space. The structural comparison
  (`iso_covers`) does not treat it as covered, and it differs between runs because recordings
  happen at different moments. The whatsit class drops from 66 to 6 there, masked at least partly
  by the earlier `intr_state` failure.
- **Fix:** treat the intrinsics' caches as derived state. Either exclude them from the comparison
  with a proof that a replay after the jump re-validates its guard, or compare only the recordings'
  guards. **Test:** run the convergence rate with `FLASHTEX_INTRINSICS=off` and on, in T7.
- **Background cost on this box:** full-1000 p50 4.4–5.5 s, p95 8.3–13.4 s of engine time per
  keystroke, with 493 pages re-run at the median.

### 3.3 Preamble edit (VERIFIED, `raw/t1/preamble-edit.jsonl`, `preamble_edit.py`)

`flashtex-host iserve`; the document is settled; a `\newcommand` line is added to the preamble,
then `compile 1` (stop once page 1 is shipped). Three edits per document, loaded PC:

| engine | plain-10 | full-10 | full-100 | full-1000 |
|---|---|---|---|---|
| main | 99–102 ms | 281–445 ms | 265–285 ms | 284–486 ms |
| p4f | 70–72 ms | 430–461 ms | 275–393 ms | 354–363 ms |

§1.2's target is ≤ 400 ms to the first visible page. The full documents are at or over it on this
loaded machine. The load inflated times by about 1.5×: pdflatex's plain-1000 took 1.81 s CPU here
against 1.21 s on the same PC when quiet (#1241's evidence) and 1.17 s on the M5. So a quiet
machine probably meets 400 ms, at about 180–310 ms (BELIEF), but **nobody has measured it**. It is
the one §1.2 row with no evidence at all.

p4f is *slower* than main on full-10 and full-1000 page 1. A plausible cause (BELIEF, not
verified): intrinsics recording on the first run of each macro. Worth a look by the intrinsics lane.

### 3.4 #1241's unchecked reads (VERIFIED reading `ix.rs`; measured side-effects)

- `ix.rs` replaces every generated subscript read with `get_unchecked` in optimised builds, for
  139 arena arrays, 2 fixed arrays and 3 `Vec`s of files (`generated/globals.rs`).
- The module text says an out-of-range read "reads a neighbouring array". That holds only for a
  small overshoot inside the arena mapping. A negative `i32` index cast to `usize` is far outside
  the mapping (SIGSEGV instead of a caught panic). In Rust, any `get_unchecked` past the slice is
  undefined behaviour the optimiser may exploit.
- DESIGN §4.5's no-panic contract relies on panics being caught (`catch_unwind` in
  `checkpoint.rs`). An unchecked read turns an engine bug into a crash or into silent wrong output,
  and for parity silent wrong output is the worse of the two.
- The gain is 1.3–3.2 % CPU on x86 (REPORTED #1241) and 5–8 % cycles on the M5. Under §1 (1 ms vs
  10 ms doesn't matter), that does not buy a safety net.
- Alternative with most of the gain: check against the compile-time capacities in
  `generated/consts.rs` / `--arena-cap`. The lane found the cost was reloading each array's length
  after `&mut` writes; a constant bound needs no reload. That is an A/B the L6 lane can measure.

Checked: the `p4f`/#1241 failures I hit on the PC were **not** from the unchecked reads. A
`--features checked-arrays` build of #1241 failed identically, because of the stale resolver
(finding 5).

---

## 4. Maintainability

### 4.1 Can a future engineer understand it? (VERIFIED)

Yes, with two gaps.
- Every hand-written module has a `//!` header that says what it is and which DESIGN section it
  serves (checked for all files under `src/`).
- `changes/README.md` explains every change file and lists the deliberate differences from TeX Live.
- `tools/web2rust/README.md` gives the regeneration command and every translator option.
- `lib.rs` maps the crate.

The gaps:
1. The incremental system's design lives in `incr.rs`'s header and five evidence READMEs, not in
   one document. DESIGN §5 is its spec, and it is out of date (§3.1).
2. Names: two `Session` types with different meanings, and `Point::Segment` checkpoints that D7's
   text rejects.

### 4.2 Upgrading pdftex.web: a dry run (VERIFIED, `raw/t1/upgrade-dry-run.txt`, `upgrade-dry-run.sh`)

**Upstream state.** The newest `texk/web2c/pdftexdir/pdftex.web` in
<https://github.com/TeX-Live/texlive-source> is commit `2d38d53fe0` (2026-02-15). It is
byte-identical to our pin, so there is no newer pdfTeX to upgrade to (VERIFIED, GitHub API). The
dry run therefore replays the last real release delta, 1.40.28 (`741a186324`) → 1.40.29: 35
changed lines in 9 hunks.

| step | result |
|---|---|
| apply our 22 change files (174 hunks) with web2rust | **1 hunk fails**: `precedence.ch` change 2. Upstream re-indented that line inside the new `must_set_text_pos` logic. |
| fix | re-indent the hunk's `@x` text (1 line) |
| regenerate | clean: 1,897 sections, 1,816 pool strings, 598 routines |
| generated diff against the committed `src/generated/` | **6,311 lines in 11 files, of which 6,280 differ only in integer literals** (string-pool numbers: one pool string added early renumbers all later ones) |
| build and lockstep of the 1.40.28-generated engine | builds (80 s, loaded) and passes **lockstep 260/260** (the logs normalise the version banner) |

What an upgrade really costs:
1. The change files: minutes. The mechanism is sound.
2. The generated diff: 6k lines of renumbering hide the ~30 lines that matter. **Proposal:** have
   web2rust emit pool references symbolically (e.g. a generated `const S_IGNORED: i32` table, or
   one `pool::id(<hash of text>)` lookup), so a regenerated diff is about the size of the upstream
   delta. Expected saving: ~6k reviewed lines per release.
3. The hand-written knowledge of pdftex.web (§2.2, §2.4). The 1.40.29 patch changed
   `pdf_begin_string`/`pdf_begin_text`'s text-mode logic: exactly the code whose invariants
   `dead_word` (`incr.rs:685–695`) cites for `pdf_h`, `pdf_v`, `pdf_tj_start_h`, `pdf_f`… Here the
   rule happened to survive. Nothing would have said so if it hadn't. **Proposal:** web2rust emits
   all numeric `@d` macros into `generated/` (it already tangles them), hand-written code imports
   them (about 290 constants), and each hand-proved rule carries the § numbers it relies on plus a
   hash of their text, checked by a unit test.
4. The oracle data: regenerate every oracle from the pinned TeX Live, per §7.

### 4.3 Concrete simplifications (expected savings)

| simplification | saves |
|---|---|
| Delete the L1-only `host::Session` (`host/mod.rs:282`–~800); route `host/tools.rs` through `incr::Session` with checkpoints off | ~500 lines, one concept |
| Promote the P4 evidence scripts (gen, incr_bench, matrix, soundness, reopen, keys, ckcpu, now in 5 directories) to `tools/incr-bench/` | ~2,400 copied lines; one T7 harness instead of two (#8) |
| Archive `tools/snapshot-bench` (decision made, §13) | 5,167 lines out of the workspace build |
| Generated constants for pdftex.web macros | ~290 hand-copied lines, and the upgrade risk |
| Symbolic pool strings in web2rust | ~6k-line upgrade diffs down to ~upstream size |

### 4.4 Duplication details (VERIFIED)

Evidence-script copies (same name in two or more `docs/evidence/*-2026-09-29/scripts/`, many edited
per copy): `gen.py`, `matrix.py`, `matrix_sum.py`, `incr_bench.py` (274 → 428 lines),
`soundness.py`, `reopen.py`, `ckcpu.py`, `barrier_test.py`, `fatal_test.py`, `mkeng.sh`, `host.sh`,
`ab.sh`. P4-FINISH adds a third set.

They hard-code `/tmp/p4l5`, `/tmp/p4l2` and a Mac worktree (`incr_bench.py:206`), and `host.sh`
needs `/bin/bash`, which NixOS lacks. I had to patch both to run them on the PC.

---

## 5. Effectiveness: the critical path to P3, P4 and P5

**P3 gate** (P-T2 on fixtures; preview parity at zero tolerance):
- P-T2 83/83 is re-verified here on Linux.
- Remaining: the Type 1 preview path (**P3-FONTS-2: branch = main, no commits yet**), #1249 (Type 1
  built-in encoding), and landing the app-v3 pane (#1247, then #1254 stacked on it) with its parity
  check.

**P4 gate** (§1.2 on the 10/100/300/1,000-page benchmarks):
- Edited page: met in CPU on the Mac; full-1000 wall p95 16.3 ms (REPORTED P4-L5).
- Preamble edit: never measured (§3.3).
- Reopen: met only with a pre-warmed host.
- 120 Hz: app side.
- The T7 harness is not built, and the convergence cost (§3.2) is not measured.

P4-FINISH is on the right items: convergence false negatives, stage timings, keep-warm. Two
problems remain. T7-LATENCY-GATE was assigned separately to mac-claude-a (third in his queue), so
there will be two harnesses. And the reopen interpretation needs a decision.

**P5 gate** (scoreboard ≥ old on every tier; arXiv L1 ≥ 90 %): #1224 reports 97.9 % L1 (REPORTED,
queued), and the retirement plan is #1236.

**Not on the critical path:** #1228 (old-pane tiles), #1252 (old-pane perf audit), the old-engine
feature PRs (#1136, #1151, #1157, #1159, #1181; D13 says fixes only), #1250 (Typst design; the owner
ranked Typst last), #1244–#1246 (cross-platform). Lane duplication: T7-LATENCY-GATE against
P4-FINISH's matrix and soundness drivers; P3-V3-ZOOM-TILES against #1228.

---

## 6. Red team of the engine decisions

| decision | still best? | evidence and alternative |
|---|---|---|
| **D1** faithful engine | **Yes**, strengthened | P-T1/P-T2 83/83 now on two OSes (VERIFIED). arXiv L1 97.9 % new against 0.7 % old (REPORTED #1224). |
| **D2** GPL port in its own crate | Yes | Nothing new. xpdf pins the binary to GPL v2/v3 only (§3), unchanged. |
| **D3** mechanical WEB→Rust | **Yes**; reword step 2 | trip, etrip, lockstep and parity hold. The upgrade dry run costs 1 hunk per release. Step 2 ("remove globals, typed indices, by hand") should become "modernise only through web2rust options and change files" (§2.3). Owner (D-number text). |
| **D4** TeX's data model | Yes | The word space made checkpoints cost 0.4–0.7 ms per page (VERIFIED §1.5) and the structural comparison possible. |
| **D5** P-T1 + P-T2 | Yes | But unenforced in CI (finding 1). |
| **D7** shipout + ~20 ms checkpoints | **Amend the text** | Built: shipout + ~20 ms + between-page segment points 0.5 ms apart (2.2 a page; logs 248 → 436 MB on full-1000, REPORTED P4-L5), because TeX reads a whole paragraph before breaking it and so the restart would otherwise be a page early. D7's rejection was of *per-paragraph* checkpoints at 5× memory. The built scheme costs 1.8× and is measured. Record it (Commander, from evidence). |
| **D8** canonical 128-bit state hash | **Amend the text** (owner) | Built: diff + isomorphism (§3.1). Better on soundness. It needs relocation of object numbers and `pdf_char_used` handling before it converges on real (hyperref) documents (§3.2). An incremental hash would make tests cheaper, but it must first be shown to cover what the walk covers. Not worth it until tests show up in the latency profile (they don't: 2–10 ms, after the edited page). |
| **D9** intrinsics after P-T1 | Yes | P-T1 was reached first. #1230 keeps its state in the word space, feeds L5's read-set, and has a both-paths verifier. Open: hand-copied command codes (148), and the guard's cost (2.3 % of full-1000, REPORTED #1241). |
| **D10** no speculative parallel segments | Yes | Nothing new. |
| **D11** socket protocol | Yes | Nothing new in this track. |
| **D12** the user's TeX Live | Yes; harden the diagnostics | A foreign `kpsewhich` first on PATH gives a silent INITEX prompt (finding 10a). |
| **§4.2** allowed optimisations | **Tighten** | #1241's unchecked reads are not in §4.2's list and are not "semantics identical" when an invariant breaks. Treat as in finding 4. |
| **§5.4** L4 viewport first | Yes | Carries the edited page even when convergence fails. |
| **§5.7** segment memo | Still gated | Nothing measured since. Convergence false negatives (§3.2) are the cheaper win for large documents and should come first. |
| **§6.3** TeX Live's zlib | Yes | #1241 re-measured the alternatives: zlib-rs differs on 336/352 streams, miniz_oxide on 350/352; the system zlib 1.2.12 is byte-identical but no faster (REPORTED). |

**New prior art since 2026-09-29** (VERIFIED through the GitHub API; the web-search quota of this
session was exhausted):
- TeXpresso <https://github.com/let-def/texpresso>: last commit 2026-07-22 (SyncTeX page bounds, idle re-run).
- Tectonic <https://github.com/tectonic-typesetting/tectonic>: last push 2026-08-01.
- pdfTeX: nothing newer than 1.40.29 (texlive-source, last `pdftex.web` commit 2026-02-15).

Nothing new bears on the engine decisions.

---

## 7. Method notes and caveats

- The PC was loaded (10–53) and shared, so timings are upper bounds; the instruction counts are
  robust. The incremental latencies in §1.6 are not comparable to the Mac evidence in absolute
  terms, only in their shape.
- `p4f` is the P4-FINISH lane's head at 01:48 EDT, carried to the PC as a git bundle (it is not
  pushed). The lane kept committing during the review (`f043785e2`).
- At 01:44 EDT, before I knew that other lanes ran on the PC, I ran `pkill -f lockstep/run.py` and
  `pkill -f pc-gates.sh` there to stop my own first attempt. As far as I can tell no other
  session's run of those names was active at that moment. If a lockstep run on the PC died around
  05:44Z, that was me.
