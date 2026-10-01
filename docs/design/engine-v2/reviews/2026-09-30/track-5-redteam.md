# Design review 2026-09-30, track 5: adversarial red-team of the whole design

**Reviewer:** claude-opus-5-5, launched by kabir-claude as NIGHTLY-REVIEW-T5 on
mac-m5pro-kabir. I am independent of every lane. Read-only: I ran no builds, benchmarks
or engine runs on the owner's laptop.

**Base:** `origin/main` `02dcf9d07` (2026-09-30 03:17Z).

**What I read:**
- DESIGN.md from end to end, including all sections and appendices;
- every evidence directory it cites, and the 2026-09-29 lane evidence (`p4-l2-l3`,
  `p4-l5`, `host-unify`, `display-list-v3`, `distribution`);
- the open PRs that DESIGN.md or the lanes depend on (#1230, #1241, #1254, #1257,
  #1245);
- `coordination/authority.json`, `AGENTS.md`, `.github/workflows/ci.yml`,
  `.github/actions/parity-fixtures`, `scripts/ci/install-selfhosted-runner.sh`,
  `tools/latex-suites/README.md` and `third_party/pdftex/regression/README.md`;
- the 48 h of main history (239 commits, 34 merged PRs) and the last 80 comments on #2.

**Labels:**
- **VERIFIED:** I checked it myself in this session, against a file at `02dcf9d07`,
  `gh` output or a primary web source. The file, command or URL is given.
- **REPORTED:** a number from a lane's evidence README, PR body or #2 comment that I
  did not re-run. The source is given.
- **BELIEF:** my judgement, with the reasoning stated.

Each finding is marked **[refinement]** (the Commander may adopt it under §14.5) or
**[owner decision]** (it changes a goal, a priority, licensing, a phase gate, a
numbered decision or owner direction).

---

## 0. Ranked findings (summary)

| # | Finding | Kind | Proposed change |
|---|---|---|---|
| 1 | **CI does not gate the new engine, and the gated path is not the product path.** CI's parity job builds the *old* `flashtex-cli`. Lockstep, the T2 suites and incremental soundness are not in CI at all. TeX Live-dependent engine tests skip as a pass. What ships (incremental, intrinsics on, unchecked reads, PGO, preview mode) is never what the gates run. | refinement | Put the new engine's P-T1/P-T2 fixtures and lockstep in the merge queue. Run the T2 suites, a soundness sample, intrinsics `verify-all`, `checked-arrays` and the PGO dist binary nightly. With `FLASHTEX_REQUIRE_TEXLIVE=1`, a skipped test fails. |
| 2 | **DESIGN.md is already stale in at least 12 places** (§1): D7, D8, §5.2, §5.3, §5.5, §6.2, §4.1, §8, §5.6 and B.2. Lanes record their deviations in evidence READMEs, not in §13. | refinement | Correct every item in §1 of this review. Add a landing rule: a PR whose evidence has a "Deviations from DESIGN.md" section carries the DESIGN.md edit and a §13 row. |
| 3 | **§1.2's "≤ 16 ms keystroke to pixels" is measured four different ways.** Measured end to end, it can't be met on a 60 Hz display: key → commit is 17–32 ms p95, then 12–15 ms more to the next frame. | owner decision | Split the target: the engine's edited page ≤ 16 ms p95 (host CPU, quiet machine), and key → CA commit ≤ 33 ms p95. Key → photon is reported, not gated. T7 gates instructions or CPU, not wall time. |
| 4 | **Self-hosted runners on a public repository run under the owner's working account.** Any branch push that edits `ci.yml` can target them: the "only merge_group or main" rule lives in the workflow file of the ref being pushed. | refinement | Run each runner as a dedicated standard macOS user with no credentials. Restrict the runner group to selected workflows pinned to `main`, or use a VM. Route to hosted runners when none is online. |
| 5 | **Unchecked array reads (#1241)** buy 5–8 % of cycles on M5 and 1–3 % on x86, which no user notices. In a process that parses untrusted input, they turn a caught out-of-range panic into a silent wrong read. | refinement | Ship the checked build unless a §1.2 target needs the difference. Nightly, T4 and T6 always run `checked-arrays`. |
| 6 | **A nightly review "at the same depth" (#1257)** costs a six-track study's tokens every night and churns the single source of truth that every lane is bound by. | owner decision | Nightly: a cheap delta audit and a drift fix. Weekly: a red-team of the changed sections. Every 2–4 weeks and at every phase exit: full re-research. |
| 7 | **Inserting a line never converges** (`line` is in the state), and no latency or soundness matrix contains a newline edit. | refinement | Make convergence relocatable in `line` (details in §3). Add newline, paragraph-split and paragraph-join edits to T7 and to soundness. |
| 8 | **Soundness is tested on generated documents with benign edits.** Missing: the transient broken states of real typing, real documents, and edits to the preamble, `\include`d files, `.bib` files and images. | refinement | Keystroke-replay soundness on a T4 subset, in the nightly run. |
| 9 | **Parity gaps the design never names:** PK/Type 3 fonts (#1218, 4 real package documents so far), mktexpk/METAFONT, and the bibtex, biber and makeindex workflows. For bundle-only users, restricted `\write18` has none of its binaries. | owner decision | Scope "100 % usable": port `bibtex.web` and `mf.web` through web2rust (reuse, §1), port or link makeindex, and use the user's biber. |
| 10 | **The P5 gate "arXiv L1 ≥ 90 %" measures page count only**, and it is already 97.9 % on 140 documents. | owner decision | P5 gate: P-T2 (or L3) ≥ 99 % on the T4 ~5k corpus with every miss triaged, plus product-path soundness (#1). |
| 11 | **Diminishing returns in the effort mix.** About 500 new lockstep cases found 0 divergences, and each wave takes 1–3 Opus review rounds. Real-document gaps (#1218) wait meanwhile. | refinement | Measure the generated engine's coverage (`cargo llvm-cov`). Write cases only for uncovered §§, and move the capacity to T4 and PK fonts. |
| 12 | **D13 and focus drift.** 87 open old-path PRs, #1121 (+2,968 LOC in the frozen engine) merged on 09-30, tiles on the old pane, and P5, P6, Typst and cross-platform lanes all running during P3/P4. | refinement | Close old-path PRs (Appendix A, policy 1). Fix D13's wording. Stop tile work on the v2 pane. Keep at most one non-P3/P4 lane at a time. |
| 13 | **Intrinsics, read-sets and incremental are coupled.** Each accelerator must report its reads to the journal, the read-set, the convergence check and the verifier. P-T1 can't see intrinsics, which are off under tracing. | refinement | One read and effect accounting API that every accelerator goes through. Nightly L5 soundness with intrinsics on. P-T2 with intrinsics on as the gate for them. |
| 14 | **Full-`\tracingall` P-T1 doesn't scale to T4:** 51 GB and 4.6 GB logs so far. | refinement | At corpus scale, P-T1 = box dumps at every shipout, compared by streamed hash. Full `\tracingall` stays for lockstep, fixtures and minimisation. |
| 15 | **macOS: memory, energy and reopen are unspecified.** 1 GB per document, 337–638 MB RSS, no memory-pressure response, no QoS policy. Reopen ≤ 100 ms holds only in a pre-warmed resident host. | refinement | One global checkpoint budget across hosts, plus memory-pressure thinning. Background passes at utility QoS. A mmap cache of kpathsea's database and the font map, so a fresh process reopens in ≤ 100 ms. |
| 16 | **Auto-compiling a newly opened, untrusted project** runs whitelisted `\write18` commands, reads any file (`openin_any=a` parity) and feeds xpdf/libpng in an unsandboxed process. | owner decision | Project trust: the first compile of an untrusted project uses `shell_escape=f`, and the user is asked. Plus a seatbelt profile for the host (no network; writes only to the project and cache). |
| 17 | **Licensing loose ends:** GPL corresponding source for the DMG (build scripts, the PGO training inputs, vendored libraries), and a per-file licence manifest for the TTB bundle. | refinement | A release checklist and a tlpdb-derived `LICENSES` in the bundle. The legal review (§3) covers the PGO question. |
| 18 | **Governance files contradict authority.json.** AGENTS.md still names `orchestrator-astra` on linux-primary. "Muse" lanes are outside DESIGN's model rule. The landing delegation isn't recorded. | refinement | Rewrite AGENTS.md to the current authority. Add Muse to Appendix A. Record delegations in `authority.json`. |

Sections 1–6 below give the evidence.

---

## 1. Internal contradictions and stale statements

Every item below is VERIFIED against the named file at `02dcf9d07`, unless it is
marked REPORTED.

| # | DESIGN.md says | Reality | Fix |
|---|---|---|---|
| C1 | §5.3 and D8: convergence is "a canonical incremental 128-bit state hash … updated on every `eq_define`" | The landed engine keeps no running hash. It compares the two states directly: a word diff plus a structural bijection of the node graphs (`src/iso.rs`, 2,466 LOC). REPORTED cost: 2–10 ms per test. `docs/evidence/p4-l2-l3-2026-09-29/README.md`, "Deviations". | Rewrite D8/§5.3 to what landed, and record the swap in §13. Direct comparison can't collide, which suits parity-first better than a hash. Keep the hash only as a future optimisation, with a proof that it covers what the comparison covers. |
| C2 | §5.3: "PDF object numbers and font numbers are relocatable" | Not relocatable: they must be equal. Only PDF file positions are relocated (same README). | Update §5.3. |
| C3 | D7: "every `\shipout` plus every ~20 ms …; **not per paragraph**" | #1242 added checkpoints after `build_page`, at least 0.5 ms apart: 2.2 per page, logs 222 → 356 MB (REPORTED, `p4-l5-2026-09-29`). That is per paragraph, in effect. | Update D7 to the measured policy. It is a technique refinement, so no owner decision is needed, but D7 is a numbered decision: log it in §13. |
| C4 | §5.2: "16 KB dirty bitmap", "Parallel restore is **required**" | 1 KB chunks with sealed per-word deltas. Restore runs sequentially, going parallel only above 4,096 chunks. Under load, the parallel restore's workers waiting for cores produced a 95 ms outlier and 13–16 ms wall p95s (REPORTED, `p4-l2-l3`, `p4-l5`). | Update §5.2. Also record that parallel restore *hurts* tail latency on a loaded machine, so it should stay the exception. |
| C5 | §5.5: "each page records which `\r@…`/`\b@…` entries it read … re-run in the background" | Every control-sequence read is recorded per checkpoint, and the passes run on the engine thread within the same `compile` call (REPORTED, `p4-l5`, "Deviations"). | Update §5.5. |
| C6 | §5.1: "S₀ persists to disk … **which gives the ≤ 100 ms reopen target**" | A fresh process takes 150–521 ms. The target is met only by a pre-warmed resident host (REPORTED, `p4-l2-l3`, "Reopen"). | Say what actually meets it, or fix it (finding 15). |
| C7 | §6.2: "Core Text no longer supports Type 1 … converted in-memory font or FreeType" | `CGFont(CGDataProvider)` loads the PFB directly on macOS 26, and P3-APP-V3 chose that (REPORTED, `docs/evidence/app-v3-preview-2026-09-29/README.md` on #1254). | Update §6.2. Record the risk that Apple removes Type 1 from `CGFont`, with its fallback. |
| C8 | §4.1 step 2: "modernise in place … remove globals into an `Engine` struct" | The engine is regenerated from an unmodified `pdftex.web` plus 12 WEB change files (2,236 lines, `crates/flashtex-engine/changes/`), with a drift test. The generated code is never hand-edited. | Make the change-file model canonical in §4.1 and drop step 2. It is the better choice: upstream upgrades (§7) become re-applying the change files, it follows Knuth's own "modify via change files" terms (`third_party/knuth/tex.web` header), and the drift test forbids hand edits anyway. |
| C9 | §8 T0: "pdfTeX's **28** CTAN regression directories" | 7 tests, from `pdftex_tests` in `pdftex.am` (`third_party/pdftex/regression/README.md`). The numbered `doc/pdftex/tests/NN-*` directories are not run. | Either add those directories, or correct §8. |
| C10 | §8 T2: "1,522 `.lvt` + 267 `.lvt`" | 989 `.lvt` files, 1,531 executions and 1,529 unique names; l3kernel is 206 (`tools/latex-suites/README.md`). | Use the runner's numbers. |
| C11 | D9: "guarded intrinsics **deferred** until P-T1 parity". §5.6 item 4 reports "Result: full-1000 … 6.7 to 4.1 ms/page". | #1230 is **still open** (`gh pr view 1230`, state OPEN). The design reports an unmerged PR's result as if it were adopted. "P-T1 parity" is also undefined: fixtures only, or the full suite (§4.2 says full suite for P6)? | Mark it as a measured proposal until it lands, and define "P-T1 parity" as a named tier and corpus. |
| C12 | §5.6 numbering runs 1, 2, 3, 4, 5, **7, 6**. §5.4 and §5.7 cite "§5.6 item 4 (hyperref's per-page cost)", which is item 6. | — | Renumber, and fix the cross-references. |
| C13 | Header: "Next design review due 2026-10-13" (two-weekly) | This nightly review is running under #1257, which is still unmerged. | Settle the cadence first (finding 6). |
| C14 | Appendix B.2: "FlashTeX today" is the old engine at `e230cfbaf`. | Superseded by P2/P3/P4. #1224 reports arXiv L1 at 97.9 % for the new engine (REPORTED). | Replace B.2 with the new engine's numbers, or date-stamp it as the baseline. |
| C15 | Appendix A, policy 1: "allow … landing already-finished work". D13: "fixes only". | #1121 (minipage and `\vfill`, +2,968 −228, a feature) landed on 09-30 as "already finished". | Pick one rule. I recommend "fixes only; finished features are closed with D13 as the reason", because every such landing is code that P5 deletes. |
| C16 | AGENTS.md, below its banner: "Current sole orchestrator: **orchestrator-astra** … linux-primary". | `coordination/authority.json`: `commander_id: kabir-claude`. | Delete AGENTS.md's historical staffing blocks (finding 18). |

---

## 2. Decisions D1–D14 and §13 re-examined

| Decision | Verdict | Reason and evidence |
|---|---|---|
| D1 faithful engine, D2 GPL port, D3 WEB translation | **Keep** | P0–P2 met within a day. #1233 reports 837/837 lockstep cases equal, and the Muse waves report 0 divergences in about 280 more (#2). The package tier reports P-T2 90/91 (#1224). The approach is doing what it promised. |
| D4 flat arenas, u32 tokens | **Keep** | It is what made the 2–7 µs snapshots and 1 KB deltas possible (REPORTED, `snapshot-bench`, `p4-l2-l3`). |
| D5 P-T1/P-T2 | **Keep, and clarify scale** | See finding 14. P-T1 as defined (`\tracingall`) is right for lockstep and fixtures, and infeasible for T4/T5: #2 reports a 51 GB self-test that swamped the owner's Mac, a 4.6 GB oracle log for pgfmath and a 585 MB one for fancyvrb. |
| D6 CG/CT preview, D11 socket | **Keep** | Measured and sound. Deriving the display list from the PDF content stream (#1217) makes the preview equal the PDF by construction. It costs the content-stream interpreter about 38 ms per 120-page cold compile (BELIEF from sampling, `host-unify`), which is negligible for one edited page. The parity-first choice is right. |
| D7 checkpoint cadence | **Update** (C3) | Superseded by between-page segment checkpoints. |
| D8 state hash | **Update** (C1) | Superseded by direct structural comparison. |
| D9 intrinsics deferred | **Update** (C11), with guard-rails from finding 13 | The owner's per-page floor for full-1000 comes from the output routine, so intrinsics are justified. Their verification must run in CI, not only in the lane. |
| D10 no speculative parallelism; single-threaded core | **Keep** | Nothing measured since contradicts it. Pipeline and export parallelism (§5.6 item 7) is the right, identical-output-only exception. |
| D12 user's TeX Live, else a bundle | **Keep, extend** | Finding 9: the bundle path lacks every external binary that pdflatex's default `\write18` runs (repstopdf, extractbb, makeindex, bibtex, kpsewhich), and mktexpk. A bundle-only user gets different output from "stock pdflatex", which §4.5 promises. |
| D13 freeze | **Keep, enforce** (C15, finding 12) | 87 of the 145 open PRs look like old-path work (VERIFIED: `gh pr list --state open`, titles starting `compiler`, `render-pipeline`, `tikz` and similar). The retirement plan counts 88 (#1236). |
| D14 DMG | **Keep** | Adds the GPL-source and notarization checklist (finding 17). |
| §13 "unchecked array reads" (#1241, not yet in §13) | **Reject for the shipped build** (finding 5) | The measured win is not noticeable; the stability cost is real. |
| §13 PGO dist build | **Keep, but gate the shipped binary** | #1241 reports byte-identical output on 89 documents. The shipped binary must still pass P-T1/P-T2 and lockstep in CI, not only the release build. |
| §15 Typst in its own host | **Keep** | The process boundary is the right answer to Apache-2.0 against GPL-2.0-only xpdf. A second, non-TeX client of `display-list-v3` also strengthens the argument that the protocol is independent, not "intimate" (§3; BELIEF, for the legal review). |
| §14 nightly full-depth review (#1257) | **Change** (finding 6) | See below. |

### Nightly reviews: cost against benefit (finding 6, owner decision)

- **Cost.** §14 at "the same depth" means six research tracks, local measurement, a
  red-team and a DESIGN.md rewrite, all on Opus high, against a shared Max quota that
  AGENTS.md already rations. Most aspects don't change overnight: the licensing,
  pdfTeX upstream and preview-renderer research tracks will be re-verifying unchanged
  facts.
- **Churn.** DESIGN.md is binding on every lane, and §14.5 says "nothing may remain
  that the review found to be wrong". A nightly rewrite means nightly re-briefs.
- **Where the benefit actually is.** This review's highest-value findings (C1–C16,
  findings 1, 3 and 7) all come from *delta drift*: landed work that the document
  doesn't reflect. A cheap nightly job catches those.
- **Proposal.**
  - Nightly: one agent, a delta audit of the last 24 h (merged PRs against the DESIGN
    sections they touch, CI truth, new deviations), fixing stale text directly.
  - Weekly: one red-team agent on the sections that changed.
  - Every two to four weeks, and at every phase exit: the full-depth re-research.

---

## 3. Maintainability traps and test gaps

### 3.1 The gates don't see the product path (finding 1, top priority)

All VERIFIED at `02dcf9d07`:
- `.github/actions/parity-fixtures/action.yml` builds `crates/flashtex-cli` (the old
  engine) and checks `tools/parity/baseline-fixtures.json`. No CI job runs
  `tools/parity --engine flashtex-initex`, `tools/lockstep`, `tools/latex-suites`, or
  the incremental soundness drivers.
- New-engine tests that need TeX Live print "no TeX Live found; skipping" and pass:
  `tests/incremental.rs:244,284,349`, `tests/checkpoint.rs`,
  `tests/display_list_host.rs` and `tests/latex_format.rs`. On a hosted runner the L2–L5
  suites are therefore green without running.
- So the P2, P3 and P4 gate numbers in DESIGN's status line are lane-run on
  mac-m5pro-kabir. They are real, but nothing stops a later PR from regressing them.
  #1242 landed with "(draft)" in its title on the strength of the lane's own runs.
- The shipped path is: resident host, incremental restarts, L5 patches, intrinsics on
  (they switch off under tracing, so P-T1 never exercises them), unchecked reads,
  PGO, and zlib level 0 in preview mode. Each is verified once, in its lane
  (13,743 soundness compiles, 53,401 both-paths intrinsic calls; REPORTED), and then
  never again.

**Proposal:**
- **Merge queue** (self-hosted Mac, where MacTeX exists as the oracle):
  - `tools/parity --tier fixtures --engine flashtex-initex`, P-T1 and P-T2, 83/83;
  - lockstep, all cases;
  - `cargo test -p flashtex-engine` with `FLASHTEX_REQUIRE_TEXLIVE=1`, which turns a
    skip into a failure.
- **Nightly:**
  - T2 suites;
  - soundness A–D at reduced trials, with intrinsics on;
  - `FLASHTEX_INTRINSICS=verify-all` over the fixtures and T4;
  - a `checked-arrays` build over T4 and T6;
  - the P-T1/P-T2 gates on the PGO dist binary.
- **Switch** the CI parity job to the new engine, and keep the old engine's run as a
  non-gating comparison until P5.

### 3.2 Incremental-system complexity

- The hand-written incremental layer is about 18.7k LOC (`wc -l
  crates/flashtex-engine/src/*.rs src/host/*.rs`, VERIFIED): `incr.rs` 3,272,
  `iso.rs` 2,466, `arena.rs` 2,076, `checkpoint.rs` 1,253, `readset.rs` 856 and the
  host. It interacts with preemption (keep, abandon, reattach), L5 patches that must
  not be converged into, redo logs and retention.
- The lanes found 13 soundness bugs so far (REPORTED: 6 in `p4-l2-l3` and 7 in
  `p4-l5`), several "found by review, not by a mismatch".
- **The oracle design is right.** Every incremental compile is compared with a
  from-scratch run. It is the only reason this complexity is tolerable, which is why
  it has to live in CI (§3.1).

**Coupling trap (finding 13).**
- #1242's "Open" item 4 says an intrinsic "must report the control sequences it reads
  (`Globals::flashtex_cs_read`), or L5 misses those reads".
- #1230 fixed that afterwards (commit `40cef9523`, VERIFIED on its branch), but only
  because the lane author noticed.
- The next accelerator (the `\csname` fast path, contiguous tokens, the segment memo
  of §5.7) has the same obligations: journal, read-set, convergence model and both-paths
  verification.

**Proposal:**
- One accounting trait, `Reads`/`Effects`, that every accelerator must call. A unit
  test enumerates the registered accelerators and fails if one lacks a read-set
  fault-injection test. #1230's `no-readset` fault is the template.
- §5.6 gets a checklist line: "every accelerator: journal, read-set, convergence,
  verify path; each with a failing-when-disabled test".

### 3.3 Newline edits never converge (finding 7)

- `docs/evidence/host-unify-2026-09-29/README.md`, "Limits" (REPORTED): "Line
  insertions never converge (`line` is in the state)". I found no later change that
  handles it; `iso.rs` and `incr.rs` compare the input-stack line fields directly.
  That is a BELIEF, not re-run.
- The edit kinds in every latency and soundness matrix are: "replace, insert or delete
  a letter", "sentence insertions of twelve words", and structural (section, label,
  ref, cite, footnote). **None inserts or removes a newline** (VERIFIED from the
  matrix descriptions in `p4-l2-l3` and `p4-l5`).
- Yet pressing Return is among the commonest edits. After one, the background pass
  re-typesets to the end of the document: about 4 s for full-1000 at 4 ms per page,
  with the battery that costs.
- **Proposal (refinement).** Treat TeX's `line` and the `line_stack` as relocatable by
  the edit's line delta, just as byte offsets are already shifted.
  - Journal every *read* of `line` that reaches state: `\inputlineno`, and text written
    to a `\write` stream or the `.aux`.
  - Journal log-only prints separately: `show_context`, "on input line N", "at lines
    N--M".
  - Converge modulo the delta when the old run's later reads are all log-only, and
    rewrite those log prints in the per-page log splice.
  - Any read into state stays a barrier.
  - Add "Return", "join paragraphs" and "blank line" edits to T7 and to soundness.

### 3.4 Soundness corpus gaps (finding 8)

The corpus is 83 fixtures, generated `plain-N`/`full-N` documents, and `refs-30`/
`refs-120` (REPORTED, `p4-l5`). The missing cases are exactly where incremental TeX
systems break:
- **Transient broken states of real typing.** `\begin{equ` unfinished, an unmatched
  `{`, `\end{itemize}` missing, a runaway argument eating the rest of the file,
  `\verb|` open. With `-interaction=nonstopmode`, error recovery inserts tokens and
  changes state, and convergence after an error has not been exercised.
- **Real documents:** arXiv and T4, with `\include`/`\includeonly`, `tikz remember
  picture`, zref-savepos, `\pdfsavepos` across passes, and `\openin` of `\write`
  output.
- **Edits outside the body:** preamble, `.bib` (needs finding 9), `.sty` in the
  project, image files replaced on disk, files deleted, and a TeX Live update
  mid-session.

**Proposal:** keystroke-replay soundness. Take a real document, rebuild a region
character by character (including the broken intermediate states), compile after every
keystroke, and compare each compile with a from-scratch run. Nightly on a 50-document
T4 subset.

### 3.5 Test-effort allocation (finding 11)

- **REPORTED from #2:** 837 + 98 + 59 + 60 + 60 lockstep cases. Every new wave was
  equal on the candidate: "no new divergence from this wave; the value is P-T1
  breadth".
- Each wave took 1–3 Opus review rounds for citations, duplicates (11 of 60 in wave
  4) and vacuous cases.
- Meanwhile the only real-document divergence the package smoke found is #1218, the
  PK fonts, in yfonts, dingbat and concmath.
- **Proposal:** measure line coverage of `src/generated/` and `src/pdftex/` over trip,
  etrip, lockstep, fixtures and T2 (`cargo llvm-cov`, one run on the NixOS PC).
  - Write lockstep cases only for uncovered §§.
  - Aim the Muse capacity at T4 real documents and minimising their divergences.
  - This replaces the hand-maintained "thinnest areas" guesswork with a number.

### 3.6 Operational fragility

- **Runner availability.** `plan` routes to self-hosted whenever
  `vars.FLASHTEX_SELFHOSTED_MAC == 1` (VERIFIED, `ci.yml` 125–128), whether or not a
  runner is online. Daniel's three runners work 09:00–15:30Z only (#2), and the queue
  timeout was raised from 60 to 240 minutes (#1205). A queue entry can therefore wait
  hours for an offline laptop.
  - **Proposal:** make the NixOS PC plus one always-on Mac the primary runners. Have a
    scheduled job flip the variable from a runner-online probe (a PAT with
    runner-read, run from `schedule`), rather than a manual setting.
- **Benchmarks on loaded, shared laptops.** Measurements ran at loads from 3 to 250.
  The lanes rightly report thread CPU and instruction counts, but T7 as planned
  ("fails when a §1.2 target regresses") will flap on wall time.
  - **Proposal:** gate on instructions and CPU, and report wall time only.
- **Evidence scripts hard-code `/Users/kubar/...`** (VERIFIED: 10 files under
  `docs/evidence/*2026-09-29*`, e.g. `host-unify-2026-09-29/scripts/bench.sh`). They
  can't be reproduced on another host, and they publish a local username in a public
  repository.
  - **Proposal:** use `$REPO`/`$TMP` variables.

---

## 4. Risks to the owner's priorities (parity > noticeable speed > maintainability)

1. **Optimising for a number users can't perceive, while parity gaps wait.** The
   owner's rule is "1 ms versus 10 ms doesn't matter; noticeable delays do".
   - Engine edited-page CPU p95 is already ≤ 14.7 ms for every document up to 1,000
     pages (REPORTED, `p4-l5`).
   - Yet effort continues on unchecked reads (5–8 %), PGO (12–15 %) and the intrinsics
     guard (2.3 %), all P6 items, while #1218 (PK fonts), bibliography tools and CI
     coverage of the engine are not staffed.
   - **Proposal:** freeze L6 work until findings 1, 9 and 10 are done, except the
     output-routine intrinsic, which cuts full-1000's end-page floor.
2. **The §1.2 target is mis-specified (finding 3).**
   - §1.2 says "keystroke to pixels".
   - The lanes report four different quantities:
     - host `edited` CPU;
     - host wall;
     - socket client time;
     - app key → CA commit: 15.4–22.9 ms p50 and 17.2–31.6 ms p95, plus commit →
       next frame 13.9–15.3 ms p50 on a 60 Hz path (REPORTED, #1254 evidence,
       "Fast path").
   - On a 60 Hz display no pipeline can show pixels in ≤ 16 ms p95 after the host
     returns: the frame alone is 16.7 ms.
   - The current wording invites either an impossible chase or a silent redefinition.
     This is an owner decision because §1.2 is a success metric.
3. **Priority inversion on stability (finding 5).**
   - #1241's own words: "an invariant of TeX's program, not a proof"; "a small overshoot
     reads a neighbouring array".
   - For documents from arXiv or colleagues, a port bug would then produce plausible
     wrong output instead of a reported panic. That is the worst outcome for a
     parity-first product, and it is also invisible to T6, whose crash oracle is the
     panic.
   - Stability outranks a gain that isn't noticeable.
4. **A weak P5 gate (finding 10).**
   - `tools/parity/README.md` defines L1 as "same page count" (VERIFIED).
   - "New engine ≥ old on every tier" is trivially true: v1 P-T2 is 0/91 (REPORTED,
     #2).
   - As written, P5 could be declared with large glyph-level differences
     unmeasured.
5. **"100 % usable" is undefined for tool workflows (finding 9).** A student's first
   `.bib` needs bibtex or biber. pdflatex alone doesn't run them; latexmk and every IDE
   do. See §6.1.

---

## 5. Security, privacy and licensing

1. **Self-hosted runners (finding 4).**
   - **GitHub's advice, VERIFIED:** "We recommend that you only use self-hosted runners
     with private repositories"
     (<https://docs.github.com/en/actions/hosting-your-own-runners/managing-self-hosted-runners/managing-access-to-self-hosted-runners-using-groups>).
     The repository is PUBLIC (VERIFIED, `gh repo view`).
   - **Where the gate lives, VERIFIED:** fork PRs are excluded, but the self-hosted
     selection happens inside `ci.yml`'s own `plan` job (lines 118–128). A `push`
     event runs the workflow file of the pushed commit. Any account or agent that can
     push a branch can therefore edit `runs-on`/`if:` and execute on the owner's Mac.
     The installer says so itself: "Anyone who can push a branch here can run commands
     on it. That is the accepted trade-off" (`install-selfhosted-runner.sh`).
     `ci.yml`: "Self-hosted Macs run the owner's working account".
   - **Why that matters here:** at least five agent sessions on three machines push
     branches. The account holds `~/.ssh`, `gh` tokens, `~/.claude` credentials and
     the login keychain.
   - **Proposal (refinement: it tightens the owner-approved §9.3, it doesn't change
     it):**
     - (a) Run each runner as a dedicated standard macOS user with no SSH keys, no
       `gh` login and no Claude credentials.
     - (b) Restrict the runner group to "selected workflows" pinned to
       `flash-tex/flashtex/.github/workflows/ci.yml@refs/heads/main` and `nightly.yml`.
       The feature is documented, VERIFIED:
       <https://docs.github.com/en/enterprise-cloud@latest/actions/how-tos/manage-runners/self-hosted-runners/manage-access>
       ("You can configure a runner group to run either selected workflows or all
       workflows"). I could not verify which plans offer it for organisation runner
       groups. Check it in the org settings.
     - (c) Otherwise, run jobs in an ephemeral macOS VM (Apple Virtualization;
       REPORTED option, licence to be checked).
2. **Auto-compile on open (finding 16).**
   - **VERIFIED history:** CVE-2016-10243, "TeX Live allows remote attackers to execute
     arbitrary commands by leveraging inclusion of mpost in shell_escape_commands"
     (<https://cveawg.mitre.org/api/cve/CVE-2016-10243>). The restricted whitelist
     that §4.5 adopts has been exploitable before.
   - **The difference from pdflatex:** a live-preview app compiles the moment a
     downloaded project is opened, before the user chooses to run anything.
   - **Separately:** TeX Live's default `openin_any=a` lets `\input` read any file
     the user can, and embed it in the PDF. §4.5's "reads confined to the project and
     TeX trees" contradicts parity with pdflatex here, and the design doesn't say which
     wins.
   - **Proposal (owner decision, since §4.5 is owner-set):**
     - a "project trust" state, in the style of VS Code's workspace trust;
     - untrusted projects compile with `shell_escape=f` and `openin_any=p`, with a
       banner offering "Trust this project" (stock-pdflatex behaviour);
     - the host runs under a seatbelt profile: no network, writes only to the project,
       the output and the cache;
     - xpdf and libpng parse untrusted figures in that same process.
3. **The GPL corresponding source for the DMG (finding 17).**
   - **VERIFIED:** GPLv2 §3 requires "the scripts used to control compilation and
     installation of the executable" (`crates/flashtex-engine/LICENSE`, lines
     158–159).
   - The shipped engine is the PGO build (#1241, `scripts/build-engine-dist.sh`). So
     the source offer must include that script, the pinned `third_party/` trees, and
     either the training inputs or the `.profdata`.
   - BELIEF, for the legal review: shipping the profile file is the safe choice.
   - **Proposal:** add a release checklist (D14):
     - an in-app GPL notice with a written source offer or a source tarball per
       release;
     - notarization with the hardened runtime for both the app and `flashtex-host`;
     - the xpdf "GPL v2 or v3 only" statement in the About panel.
4. **TTB bundle redistribution.** Pinning the digest is good (REPORTED,
   `distribution`). The bundle redistributes TeX Live packages under mixed licences.
   - **Proposal:** generate `LICENSES.tsv` from `texlive.tlpdb`'s `catalogue-license`
     field, ship it in the bundle, and fail bundle creation on `nosell`/`nonfree`/
     unknown.
5. **Privacy.** I found no telemetry in the design. Keep it that way, and say so in
   §4.5.
   - The persisted S₀ snapshots (21–28 MB each, REPORTED) contain document text in
     the app cache. Exclude them from Time Machine and iCloud (`isExcludedFromBackup`)
     and delete them on "Forget project".

---

## 6. What the experts would say is missing

### 6.1 An expert TeX/LaTeX maintainer

1. **Bitmap fonts.** PK and Type 3 fonts, and **mktexpk/mktextfm** (METAFONT) for fonts
   with no Type 1 (#1218: `yinit`, `ark10`, `ccr10`, `wnr10`; REPORTED, #2).
   - pdflatex's behaviour depends on kpathsea running `mf` on demand.
   - **Proposal:** port `mf.web` with the same web2rust pipeline, in the spirit of §1
     "reuse": it is Knuth WEB and has its own trip test, *trap*. Also emit Type 3 in
     the PDF backend (pdfTeX's `writet3.c`).
2. **Bibliography and index tools.** bibtex, biber, makeindex/xindy and makeglossaries,
   with latexmk-style orchestration: rerun when `.bbl`/`.ind` change, detected through
   the existing read-sets.
   - **Proposal:**
     - `bibtex.web` through web2rust (a WEB program, reuse);
     - port or link makeindex (C);
     - biber from the user's TeX Live only (it is a packed Perl binary);
     - the orchestration rules come from latexmk's documented rules;
     - an owner decision on scope for the bundle-only user.
3. **Error-state policy.** Which interaction mode does editing use, what does the
   preview show on a fatal error ("keep the last good page and mark it"), and how is a
   runaway argument contained? None of this is in §5 or §6.
4. **User TeX Live skew.** D12 builds the format from the user's `latex.ltx`, but the
   oracle is TL2026 and pdfTeX 1.40.29. Users on TL2023–2025 get 1.40.29 semantics
   with an older kernel.
   - **Proposal:** a "user TL year" matrix leg (TL2024, TL2025 formats) in the weekly
     T5 run, and a documented support window.
5. **SyncTeX interoperability.** Span ids are fine internally, but external tools
   (Skim, `synctex view`) and users' habits expect `.synctex.gz`. `-synctex` is
   currently refused (`changes/README.md`).
   - **Proposal:** export a `.synctex.gz` from the span table at export time.

### 6.2 An expert macOS engineer

1. **Memory.**
   - The 1 GB budget is per document. Measured RSS is 337–638 MB (REPORTED) on 24 GB
     machines, with no response to memory pressure.
   - **Proposal:** a global budget across `flashtex-host` processes; a
     `DISPATCH_SOURCE_TYPE_MEMORYPRESSURE` handler that thins retention; idle hosts
     dropping far checkpoints after N minutes.
2. **Energy and scheduling.** Every keystroke restarts the engine, and background
   passes run to convergence or the end (the full-120 background is 0.4–0.5 s,
   REPORTED).
   - **Proposal:**
     - the edited page at `userInteractive` QoS;
     - background passes at `utility`, pausing under Low Power Mode or
       `thermalState ≥ .serious`;
     - background passes coalesce while the user is typing, which preemption already
       half-does.
3. **Reopen without a resident daemon (C6).**
   - A pre-warmed host is a process kept alive for nothing. The 150–180 ms fresh-start
     cost is kpathsea initialisation plus the font map (REPORTED).
   - **Proposal:** a content-hash-keyed, mmap-able cache of the parsed `ls-R`
     databases and `pdftex.map`, so a fresh process meets ≤ 100 ms.
4. **Text layer.** `display-list-v3` removed v2's text clusters ("text/source: clusters
   with byte ranges" → "per-item source spans", `docs/protocol/display-list-v3.md`
   line 42; VERIFIED). Glyph names are available, but no text mapping is specified.
   - Copy, Find, VoiceOver and Services need Unicode that matches the exported PDF's
     text layer.
   - **Proposal:** specify glyph → Unicode in the protocol, using pdfTeX's
     `glyphtounicode.tex` tables as `\pdfgentounicode` does, and gate it: extracted
     text equals `pdftotext` of the export.
5. **Font smoothing ruling (#1228, pending on #2).**
   - The conflict was measured on the v2 pane, whose fonts differed from the embedded
     subsets.
   - The v3 pane draws the embedded Type 1 programs through `CGFont` (C7).
   - **Proposal:** re-measure smoothing ON against CG's render of the export on the
     v3 path. If it is still pixel-identical, turn it on to match Preview.app. BELIEF:
     likely, since both sides then use the same program and rasteriser.
   - Also stop the tile work on the v2 pane (#1228), which P5 deletes.
6. **Distribution hygiene.** Hardened runtime, notarization, Sparkle EdDSA-signed
   updates (#694), and the GUI-app `PATH` problem for the user's TeX Live
   (`/Library/TeX/texbin`, already handled per §4.4). Also TCC and iCloud "dataless"
   placeholders: the S₀ key's `stat` check (size/mtime/inode, REPORTED,
   `distribution`) can trigger downloads of evicted files, or misread a placeholder's
   metadata.
   - **Proposal:** test with iCloud Drive "Optimise Mac Storage" on.

---

## 7. Governance and instructions (finding 18)

- AGENTS.md at `02dcf9d07` still contains seven superseded staffing blocks naming
  `orchestrator-astra`, linux-primary Codex and paused lanes, under a banner that
  names kabir-claude. Agents read AGENTS.md before DESIGN.md.
  - **Proposal:** reduce AGENTS.md to the banner, the commit rules and pointers.
    Historical blocks move to `docs/history/`.
- **"Muse"** is Daniel's Meta-API model (`coordination/DANIEL-PARENT-HANDOFF-2026-09-18.md`
  line 175). It runs lanes (#2, 2026-09-30 01:10Z) that Appendix A's model rule
  ("no Fable, Sonnet or Haiku") neither permits nor forbids.
  - **Proposal:** add Muse to Appendix A, with its current constraints: oracle-checked
    only, no engine code, Opus review.
- The landing delegation to flashtex-2a (#2, 03:41Z) exists only in a comment.
  - **Proposal:** record the scoped delegation in `authority.json`, which every
    writer rereads.
- The Commander's local auto-memory (outside the repository) still says "Sonnet
  medium docs/QA, Haiku low search", contrary to the 2026-09-29 owner rule. The
  Commander should update it.

---

## 8. Verified and reported, separately

**VERIFIED in this session:**
- the file and line references in §1 (C1–C16);
- CI composition: `ci.yml`, `parity-fixtures/action.yml`, the installer;
- the TeX-Live-skip tests;
- 145 open PRs, of which 87 look like old-path work;
- #1230 is OPEN and already contains read-set reporting (`40cef9523`);
- the repository is PUBLIC;
- the three web sources cited in §5 (GitHub's two documentation pages and the
  CVE-2016-10243 record);
- the GPLv2 §3 text;
- main's `CI` and `perf` push runs were green for 01:02–03:55Z.

**REPORTED, not re-run:** every latency, memory, soundness, instruction-count and
parity number, taken from the evidence READMEs and PR bodies named inline.

**BELIEF:** the smoothing outcome on the v3 pane; the legal reading of PGO profiles;
that newline edits still don't converge after #1242; the cost estimate for nightly
reviews.
