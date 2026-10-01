# Nightly design review 2026-09-30, Track 4: parity, verification coverage and the road to P5

**Scope:** DESIGN.md §1.1 (parity tiers), §8 (verification tiers T0–T7), §12 (phases and exit
gates) and §14 (review duty), audited against what exists on `main` `02dcf9d07` and the open
lanes, plus a fresh real-world parity measurement.
**Reviewer:** kabir-claude NIGHTLY-REVIEW-T4 (claude-opus-5-5), mac-m5pro-kabir. The corpus
runs were made on the NixOS PC (`nixos`, 16 cores, 30 GiB), as the CPU rule requires.
**Labels:** **VERIFIED** means I measured or read it myself in this review. **REPORTED** means
it comes from a PR body or an issue #2 comment and I did not re-measure it. **BELIEF** marks
my own estimates and judgements.

---

## 0. Summary

1. **arXiv L1 today (the P5 gate metric).** On current main, re-measured on the NixOS PC:
   the new engine scores **129/131 = 98.5%** (Wilson 95% interval 94.6–99.6%). v1 scores
   **1/131 = 0.8% (Wilson 0.1–4.2%)** on the same documents (§2.2). The Mac measurement in #1224 (REPORTED, an
   older engine) gave 137/140 = 97.9% against v1's 0.7%. **The 90% gate is met with a wide
   margin on both hosts.** Every failure is one of three open engine issues:
   - #1218 (PK/Type 3);
   - #1220 (font expansion);
   - #1219 (a panic). Its arXiv trigger is excluded on the PC, because pdflatex fails there
     too.

   #1218 is claimed (P3-FONTS-2); #1219 and #1220 have no fix branch.
2. **Verification coverage is the real gap, not parity.** On main, **CI does not run the new
   engine against pdfTeX at all.** It runs trip, etrip, the pdfTeX regression scripts and
   the crate tests. Every other gate runs the **old (v1) CLI**: the PR/queue
   `parity-fixtures` job, nightly `parity-corpus` and `perf`. So T1, T2 and T3 for the new
   engine exist only as manual lane runs. T4, T6 and T7 are not on main. T5 has no lane at
   all. P2's and P3's parity results can regress on main without a red check. Even trip and
   etrip (the P1/P2 gates) are **not in the required `CI required` check**, so they don't
   block the merge queue.
3. **Critical path to P5:**
   - wire the new engine into CI: T1, T2 and T3 against the engine, then T7 (retirement
     plan stage S4);
   - fix #1219, #1218 and #1220;
   - land P3-APP-V3 (#1247) and the display-list/preview parity on real documents;
   - make the app path meet §1.2. Measured through the app, keystroke to pixels is
     p95 18.5 ms at 10 and 120 pages and 37.7 ms at 1,000 pages (REPORTED, #1247), against
     a 16 ms target. The engine alone meets it: CPU p95 ≤ 14.1 ms (REPORTED, P4-L5
     evidence).

   BELIEF: P3 closes in 1–2 days, P4 in 3–6 days, and P5's gate numbers in 5–8 days if S4
   starts now. "Retirement complete" (S6–S8) needs a further 2–3 weeks, because of the
   14-day soak.
4. **The gates need tightening.** L1 (same page count) is far too weak for the new engine.
   Every compiled document is already at L4. The gate should be P-T2 and P-T1 on the arXiv
   *and* T4 corpora, plus:
   - crash-freedom;
   - "no TeX Live" (bundle) parity;
   - an incremental-equals-scratch check on real documents;
   - keystroke-to-pixels latency measured in the app, not only in the engine host.

   Details and proposed wording are in §5.

---

## 1. Verification tiers T0–T7: status on main `02dcf9d07`

Sources: `.github/workflows/{ci,nightly,perf}.yml` and `scripts/gate.sh` on main (VERIFIED,
read); run history from `gh run list` (VERIFIED); lane state from open PRs, remote branches,
`coordination-claims` and #2 (all 260 comments since 2026-09-29 were read by grep).

| Tier | DESIGN §8 | Exists? | Wired into | Last result | Gap |
|---|---|---|---|---|---|
| **T0** | unit tests, web2rust round-trip, trip, etrip, "pdfTeX's 28 CTAN regression directories" | yes | `ci.yml` jobs `trip`, `etrip`, `pdftex-regression` and `rust-workspace` (full tier: `merge_group`, push to main); crate tests of touched crates in the PR tier (`gate.sh quick`) | main push CI **success** on `02dcf9d07` (run 36666582731, 7 min 46 s) (VERIFIED) | (1) **T0 doesn't block merges.** The only required status check on `main` is `CI required` (ruleset, VERIFIED via `gh api …/rules/branches/main`). Its `needs:` list omits `trip`, `etrip` and `pdftex-regression`. So a trip or etrip failure turns main red after the merge but never stops the queue. (2) `scripts/pdftex-regression.sh` runs the **7** `pdftex_tests` of TeX Live's `pdftex.am`, not "28 CTAN directories". Either DESIGN §8 is wrong or 21 directories are missing; nobody has reconciled them. |
| **T1** | lockstep against the pinned pdfTeX, every PR | `tools/lockstep`: 260 cases on main | **none**: `ci.yml` and `gate.sh` never call `tools/lockstep` (VERIFIED, grep) | REPORTED: 260/260 at the P2 gate; 837/837 on `11d364f75` (#1233); wave PRs #1248/#1251/#1253/#1256 add ≈280 cases, all equal against the candidate | Not a CI gate. Growth continues (≈1,117 cases once the waves land), but **no lockstep case has found an engine difference since P2**. Every real difference came from real documents or the document fuzzer (§2.3). |
| **T2** | LaTeX team suites, touched on PRs, full in the queue | `tools/latex-suites` | **none** | REPORTED: 1,520/1,529, 0 unexpected, at P2 (`d4f2a1581`); hardening #1213 is in the queue | Not a CI gate. |
| **T3** | scoreboard: fixtures, arXiv and templates tiers, in the queue | `tools/parity`: fixtures in the PR tier and the queue; arXiv/templates nightly | `parity-fixtures` (PR and queue) and nightly `parity-corpus` both build **`crates/flashtex-cli` (v1)** and never pass `--engine` (VERIFIED, `.github/actions/parity-fixtures/action.yml`, `nightly.yml`) | the v1 fixtures baseline holds; nightly v1 run green 2026-09-29 14:52Z | **The new engine's P-T1/P-T2 is gated nowhere.** #1224 (harness fixes and the engine scoreboard) is 13th of 17 in the merge queue (06:34Z). Its first queue run failed on an unrelated self-hosted runner loss (`rust render-pipeline (macos-15)`, steps never completed). The packages tier is on a branch. |
| **T4** | ~5,000 documents, nightly | branch `agent/flashtex-2a/nightly-corpus`: `nightly.py`, 50 shards, host-labelled ratchet, `corpus-t4` job on `flashtex-linux` | not on main; no PR yet | REPORTED: proven on a 30-document sample. `corpus/nightly-5k.json` is **not committed yet** (the README describes a 20 categories × 10 years × 25 grid); no baseline | no manifest, no PR, no baseline |
| **T5** | 30,000+ documents, weekly | **nothing** | — | — | **No lane, no owner.** Not mentioned on #2. |
| **T6** | differential fuzzing plus parser fuzzers, continuous | #1234 (`tools/fuzz`) and #1240 (seeds, in the queue) | not wired: #1234 touches no workflow; `nightly.yml` wiring is a stated follow-up | REPORTED: MERGE-READY at `150886bf2` (05:08Z) but **not queued**. Findings: #1219 (≈7% of generated documents), the #1220 family, #1237. 15,000 parser iterations with 0 panics | queue #1234; wire it into `nightly.yml` |
| **T7** | latency benchmarks at 10/100/300/1,000 pages, merge queue | **no harness in CI.** Measurement scripts exist as lane evidence (`docs/evidence/p4-l{1,2-l3,5}-2026-09-29/scripts/`: engine host) and `EngineV3Bench` in #1247 (app) | `perf.yml` runs `crates/perf-bench`, which is **v1** | REPORTED: engine host p95 wall 16.3 ms / CPU 14.1 ms on full-1000 (P4-L5); app keystroke to pixels p95 18.5 ms (10 and 120 pages), 37.7 ms (1,000) (#1247) | Lane `T7-LATENCY-GATE` was assigned to mac-claude-a at 01:10Z and re-affirmed twice. **No branch exists.** |

**ETAs from #2.** None of the missing tiers has an ETA on #2 (VERIFIED: grep over every
comment since 2026-09-29). The table below gives owners from #2 and my estimates (BELIEF), at
current lane loads.

| Missing piece | Owner (lane, machine) | Depends on | ETA (BELIEF) |
|---|---|---|---|
| T1/T2/T3 against the new engine in CI (retirement plan stage S4) | **unowned.** Proposal: flashtex-2a (mac-m5pro-dq222), who wrote the harnesses and has landing delegation for `tools/**`. The `ci.yml` edit itself needs the Commander (workflow files). | #1224, #1213, #1233 landed; the formats built once per runner | 1–2 days |
| T4 | flashtex-2a, `T4-NIGHTLY-CORPUS`, runs on the NixOS PC | the manifest (arXiv selection is throttled to 1 request per 3 s, so ≈4 h for 5k), a PR, the first `workflow_dispatch` baseline | 2–3 days |
| T5 | **none.** Proposal: the same harness, sharded across Daniel's, Jaysen's and the Commander's Macs *and* the PC, each with a host-labelled ratchet | T4 | 1–2 weeks |
| T6 in nightly | daniel-muse-lead, `T6-DIFF-FUZZ` | queueing #1234, a `nightly.yml` job on the PC | 1 day |
| T7 | mac-claude-a (Jaysen), `T7-LATENCY-GATE`, not started | the #1247 app path; the reference host chosen (retirement plan Q9) | 3–5 days once started |

---

## 2. Real-world parity

### 2.1 The existing measurement (REPORTED, #1224, mac-m5pro-dq222, engine `3a5ba1cbf`+`572cc3388`)

| tier | engine | P-T1 | P-T2 | L1 |
|---|---|---|---|---|
| arXiv (140 measured, 9 excluded) | new | 131/136 | 136/140 | **137/140 = 97.9%** |
| | v1 | n/a | 0/140 | 1/140 = 0.7% |
| templates (18 measured) | new | 17/18 | 17/18 | 18/18 |
| | v1 | n/a | 0/18 | 2/18 |

An independent review reproduced it byte-identically on 13 documents on a different host
(REPORTED, #1224 comment 5896172295).

### 2.2 Re-measurement on current main (VERIFIED, this review)

**Setup:**
- Engine `flashtex-initex` built from main `02dcf9d07` (sha256 `48d374fc…`) on the NixOS PC.
  Its formats were built by the engine itself from the PC's TeX Live 2026.
- Harness: #1224's head `679ba3c00`, because main's harness lacks #1224's fixes.
- Oracle: the PC's own pdfTeX 1.40.29, with Ghostscript 10.07.1 from nixpkgs on PATH for
  epstopdf.
- Each engine runs in its default `\write18` mode (restricted).
- `--pt1-max-log-mb 2048`, and the same four `--pt1-skip` giants as #1224.
- Commands: the appendix. The side-by-side report (`engines.py`) is in `track-4-raw/`.

| tier | engine | measured | P-T1 | P-T2 | L0 | **L1** | L4 |
|---|---|---|---|---|---|---|---|
| arXiv | new | 131 (18 excluded) | 125/129 (96.9%) | 128/131 (97.7%) | 129/131 | **129/131 = 98.5%** | 129/129 |
| | v1 | 131 (same) | n/a | 0/131 | 2/131 | **1/131 = 0.8%** | 0/131 |
| | pdflatex (self-test, P-T2 only) | 131 (same) | n/a | 131/131 | 131/131 | 131/131 | 131/131 |
| templates | new | 11 (9 excluded) | 11/11 | 11/11 | 11/11 | 11/11 | 11/11 |
| | v1 | 11 (same) | n/a | 0/11 | 2/11 | 2/11 | 0/11 |
| | pdflatex (self-test, P-T2 only) | 11 (same) | n/a | 11/11 | 11/11 | 11/11 | 11/11 |

**The P5 metric now: arXiv L1 = 98.5% for the new engine and 0.8% (1/131) for v1**, on the
same 131 documents. Glyph-position error over the 6,156,320 aligned glyphs is
0.000 bp at p50, p95 and max: whenever the engine compiles a document, it places every
glyph exactly.

The run found **no new difference and no fix**. The failing set is exactly #1224's, minus
2501.07457v1 (#1219), which pdflatex itself can't compile on the PC (below). The 33-minute
run (`-j 4`, `nice 19`) shared the PC with three CI runners and two other lanes' benchmarks.

### 2.3 Every failure, classified

| document | P-T1 | P-T2 | level | class | cause | issue |
|---|---|---|---|---|---|---|
| 2501.08371v3 | fail | fail | below L0 | b, engine | `bbm12` has no `pdftex.map` entry: pdfTeX embeds a PK bitmap as Type 3; `writet3.rs` is a stub that aborts the run | #1218 (lane P3-FONTS-2, claimed 05:33Z) |
| 2501.08775v2 | fail | fail | below L0 | b, engine | the same, `bbm10` | #1218 |
| 2501.10230v1 | fail | fail | L4 | b, engine | microtype font expansion: one expanded font fewer, so `/F<n>` shifts; the log differs at shipout 18 | #1220 |
| 2501.07077v1 | fail | pass | L4 | b, engine | `hpack(cal_expand_ratio)` leaves glue fields unset (shipout 3, box line 465) | #1220 |
| 2501.07457v1 *(Mac)* | fail | fail | below L0 | b, engine | **panic** in `get_kn_bs_code` (microtype `spacing`), which breaks §4.5's no-panic contract; excluded on the PC because pdflatex fails there too | #1219 |
| templates/moderncv-template *(Mac)* | fail | fail | L4 | b | #1220 | #1220 |

**Class a** (a package or font missing from the user's TeX Live): **0**.
**Class c** (harness): `engines.py` counts **6**. They are the 6 templates excluded at
fetch, because the PC's TeX Live has no `doc/` tree or has other file versions: host
setup, **none affecting engine results**. Three harness findings:
- The Mac run's notes file (`arxiv-scoreboard-2026-09-29.notes.json`) overrides the
  automatic class by document, so on the PC it relabels 2501.07457v1 (pdflatex fails
  here) and moderncv (not fetched here) as class b. Notes should be keyed by host or
  oracle outcome. The "b = 6" in the raw report is therefore 4 measured here plus 2
  carried over from the Mac.
- The root-cause table attributes an engine abort to "crates/render-pipeline (worker
  robustness)". That is a v1-era owner mapping in `definers.py`/`parity.py`, and it will
  misroute engine bugs.
- `corpus.py` accepts `--cache`/`--texmf` only before the subcommand. My first attempt
  failed on that (a usability nit).

**Class d** (pdflatex fails too): **18 on the PC against 9 on the Mac.** The 9 extra are
host-dependent, **not engine-dependent**:
- The PC's TeX Live 2026 was updated with tlmgr (`texlive.tlpdb` dated 2026-09-29; babel-french
  v4.1a of 2026-06-06). This Mac's MacTeX 2026 is the initial snapshot (babel-french v4.0e of
  2025-08-15).
- Under the update, 6 e-prints fail in **pdflatex** with `Command \c@lemma/\c@corollary/\c@cor/\c@clm
  already defined`. I reproduced one: thmtools' `thm-amsthm.sty`/`thm-listof.sty` together with
  `aliascnt` and cleveref.
- 2501.07105v1 fails with babel's `Unknown option 'francais'`.
- On the templates tier, 7 of 20 are "fetch" exclusions: the PC's TeX Live has no `doc/` tree,
  and moderncv/amsldoc differ by hash.

**Class e** (never converges): 2501.07495v1.

**Out of scope, but worth measuring.** Documents that only another engine compiles (XeLaTeX,
LuaLaTeX, or `latex`+dvips with PSTricks) are excluded by construction. arXiv compiles with
pdflatex/latex only, so this corpus under-samples them. Overleaf-style user populations don't
(§3, item 12).

### 2.4 Statistical reading (BELIEF, arithmetic VERIFIED)

At n = 131, the Wilson 95% interval on 98.5% is **94.6–99.6%**; on the Mac's 137/140 it is
93.9–99.3%. Both lower bounds clear 90%, so the gate as written is met with statistical
confidence.

The sample is narrow, though:
- 7 primary categories (math, cs, hep-th, quant-ph, cond-mat, astro-ph, gr-qc), 10 each;
- one submission week, 13–17 January 2025;
- one TeX Live snapshot per host.

The three real engine bugs (#1218, #1219, #1220) each hit 1–2 documents in 140. At that
rate the 140-document tier can't tell 97% from 99%. T4 (5,000 documents, about ±0.8% at
90%) is the tier that can.

---

## 3. Parity risk inventory

| # | Risk | Evidence | Impact | Owner, state |
|---|---|---|---|---|
| 1 | **PK/Type 3 bitmap fonts** (`writet3`) | stub that aborts (`crates/flashtex-engine/src/pdftex/writet3.rs`, VERIFIED). Triggers: bbm, yfonts' `yinit` (#1238), `wnr10` in fontenc's `encguide` (packages tier), punk | a hard failure (below L0): 2 of 131–140 arXiv documents (≈1.5%); any font without a Type 1 map entry | P3-FONTS-2 (kabir-claude), claimed 05:33Z; a font census is running on the PC. Needs `mktexpk`/METAFONT at runtime (an external effect, an L3 barrier) |
| 2 | **TrueType/OpenType embedding** (`writettf`/`writeotf`) | `writefont.rs:392`: "TrueType/OpenType font embedding is not implemented yet" (VERIFIED) | any map entry naming `.ttf`/`.otf` (via `\pdfmapline` or package map files) aborts. Frequency unmeasured; 0 in 140 arXiv documents | **unowned**; nothing on #2 |
| 3 | **Font expansion** (microtype `expansion`) | #1220: pdfTeX loads one more expanded font; `hpack(cal_expand_ratio)` glue fields | output differs (P-T1 and P-T2); microtype is very common in templates (moderncv) and on arXiv | kabir-claude, no branch (VERIFIED: no branch names #1220) |
| 4 | **A panic under microtype `spacing`** | #1219: a 6-line reproducer; the document fuzzer hits it in ≈7% of generated documents | a crash (§4.5). In the app the host dies. | kabir-claude, no branch; #2 calls it "top priority" |
| 5 | Stack overflow on a self-recursive Type 1 subroutine | #1237 (pdfTeX segfaults too) | a crash on malformed fonts only | open |
| 6 | **SyncTeX** | `-synctex` is refused (`cli.rs:32`, VERIFIED); source mapping travels as display-list span ids (§6.1) | no `.synctex.gz` for external viewers or editors. Inside the app, forward and inverse search depend on span ids, now in lane P3-APP-V3 | P3-APP-V3 (kabir-claude). SyncTeX file output: **unowned** |
| 7 | **`\write18`** | restricted mode is the default and passes `write18.rs` tests; epstopdf seeding is harness-proven (#1224) | commands outside `shell_escape_commands` (minted, gnuplottex, svg/inkscape) need an opt-in, exactly as in pdflatex. An executed command is an L3 barrier (convergence stops). Also `\write18` output depends on host tools. The PC has no Ghostscript by default, and #1232's nightly measured only 127 arXiv documents there (REPORTED). This review put nixpkgs' gs on PATH and measured 131. So EPS documents silently drop out of the denominator on a host without gs. | covered; a documented UX risk. Harness: fail loudly when `gs` is missing |
| 8 | **The bundle fallback without TeX Live** | #1216: the TTBv1 reader, a *fixture* bundle, 75/82 then (REPORTED). **No real bundle is hosted** (owner: hosting) | a user without MacTeX gets **no engine at all**, while v1 works without TeX Live. A P5 flip would regress these users 100% | open; hosting needs an owner decision |
| 9 | **TeX Live snapshot skew** (new, VERIFIED here) | the PC's updated TL2026 fails 9 e-prints in pdflatex that the Mac's MacTeX 2026 compiles | parity holds against the *user's* pdflatex by construction (D12). But: (a) T4/T5 baselines are per host; (b) "pinned TeX Live" (§7) isn't true across the fleet; (c) users on updated TeX Live meet upstream breakages that v1 never saw | process: record the TL snapshot (tlpdb date/revision) in every scoreboard's metadata |
| 10 | **TeX Live discovery picks another TeX tree** (new, VERIFIED here) | on the PC, `~/.nix-profile/bin/kpsewhich` (home-manager's TeX) comes first on PATH. With it the engine found no `pdflatex.ini` and the format build aborted, until `FLASHTEX_TEXLIVE_BIN` was set | on a Mac with Homebrew `texlive` plus MacTeX, the engine may use a different tree from the one the user's pdflatex uses. That is by design ("what a login shell runs"), but it needs a diagnostic naming the tree and a setting | P3-DISTRIBUTION follow-up |
| 11 | **What P-T2 doesn't compare**: annotations, link destinations, outlines, the catalogue | `tools/parity/README.md` "Honest limits" (VERIFIED) | hyperref links, bookmarks, PDF metadata and `\pdfcatalog` can differ with every gate green, and users see broken links. The per-page resources comparison doesn't catch `/Annots` content or outline structure | proposal in §5.1 |
| 12 | Documents only other engines compile | out of scope by §1.1 ("any document pdflatex compiles") | users with XeLaTeX/LuaLaTeX documents (fontspec) get an error. The app must say so clearly, not show a parity failure | product decision (owner): report "needs XeLaTeX/LuaLaTeX" as a first-class diagnostic |
| 13 | **Preview parity** | #1247: zero tolerance on 83 fixtures at 2× and 4× (220/220), 1× 216/220 at the rounding floor; **30 page renders fall back to the exported PDF** (transparency, shadings, patterns) | fallback pages cost a full PDF render per update (≈25 ms, Appendix B.3), a latency cliff on tikz-heavy documents. Parity is measured on fixtures only, not arXiv | P3-APP-V3; proposal §5.1 |
| 14 | **The incremental path's parity on real documents** | soundness: 13,743 compiles, 0 mismatches, but on the 83 fixtures plus synthetic documents only (REPORTED, P4-L5) | the host path (checkpoints, restore, convergence, `.aux` read-sets) is what users run. arXiv is only measured through the batch `flashtex-initex` | proposal §5.1 |
| 15 | Huge traced logs | 4 of 140 arXiv documents skip P-T1 (3–25 GB logs) | P-T1 not evaluated on the heaviest documents, which are where state bugs hide | streaming comparison in lockstep `capture()` (#1210 follow-up) |
| 16 | `~/.xpdfrc` read by xpdf | §11 | a sandbox and parity edge | §11, before release |
| 17 | The old-path IDE features lose their data source | retirement plan R2: `supported-latex.json` (completion) comes from the compiler | at the flip, completions regress unless S1(c) lands | P5-RETIREMENT-PLAN |

---

## 4. Roadmap: the critical path to each remaining exit gate

Owners come from #2 and `coordination-claims` (VERIFIED). Durations are BELIEF, assuming the
current staffing.

### P3 Output: gate "P-T2 on fixtures; preview parity check at zero tolerance"

**State:**
- P-T2 on fixtures is met: 83/83 at the P2 verification and 83/83 in the P4-L5 evidence
  (REPORTED).
- Preview parity is met on the fixtures in #1247 (REPORTED): 2× and 4× 220/220; 1× at a
  documented rounding floor; 30 renders on the PDF fallback.
- **Remaining:**
  - Land #1247 (queued at position 10) and #1249 (the Type 1 built-in encoding, queued).
  - The Commander rules on two questions: is the 1× floor accepted, and do fallback pages
    count as "parity"?
- **Not gate items, but P3 scope in §12:** `writet3` (#1218) and TrueType (§3, item 2) are
  part of "ported PDF backend". The gate text doesn't name them.
- **ETA (BELIEF): 1–2 days**, bounded by the merge queue (below), not by work.
- **Blocker: merge-queue throughput** (VERIFIED at 06:34Z).
  - The queue holds 17 entries. **Nothing has landed on main since 03:17Z** (3 h 17 min).
  - #1239 was first in the queue at this review's first check and is now last, so its
    group failed or was rebuilt.
  - The ruleset builds at most **3 entries at once** (`max_entries_to_build: 3`). A
    merge-group run takes 30–43 min plus runner waits, which caps landings at ≈4 an hour
    even with no failures.
  - One run (#1224's) was lost to a self-hosted runner dying mid-job.
  - Proposal (Commander, BELIEF):
    - raise `max_entries_to_build` to match the runner count (3 per Mac plus 3 on the PC);
    - route docs-only and `tools/**`-only PRs to a lighter merge-group path;
    - add `trip`, `etrip` and `pdftex-regression` (and later lockstep/parity against the
      engine) to `CI required`'s `needs`, so the queue enforces them.

### P4 Incremental: gate "§1.2 latency targets on 10/100/300/1,000-page benchmarks"

**State (REPORTED, P4-L5 evidence on main):** engine-host edited-page p95 is CPU ≤ 14.1 ms on
every document. Wall-clock p95 is 16.3 ms on full-1000, and 8 of 528 compiles exceed 16 ms.
Checkpoints cost 0.27–0.35 ms per page, and 13,743 soundness compiles showed 0 mismatches.

**Not yet met, or not measured:**

1. **Keystroke to pixels in the app** (the §1.2 definition): p95 18.5 ms at 10 and 120 pages,
   37.7 ms at 1,000 (REPORTED, #1247). At 1,000 pages, 11.5 ms is the editor's own work.
   This is the P4 gate's real gap. Owners: APP-PERF-AUDIT (#1252) and P3-APP-V3.
2. **The preamble edit, ≤ 400 ms:** not measured as such. The fresh-process first page takes
   150–450 ms depending on load.
3. **120 Hz scroll and zoom:** tiles for the new pane (`P3-V3-ZOOM-TILES`, mac-claude-a) are
   not started. #1228 for the old pane is open.
4. **Open items 1–5 in the P4-L5 evidence**, e.g. preempted keystrokes at 17.4/25.1 ms on
   the 120-page full document.
5. **T7 exists nowhere as a gate.**

**Critical path:** T7 (the harness, the host and a noise protocol) → app-side latency
fixes (#1252 findings 1–3: raster only changed pages, layout-trait measuring,
main-thread render pass) → P4-FINISH (kabir-claude, claimed 05:11Z) closes the open
items → the T7 gate is green on the chosen reference host.

**ETA (BELIEF): 3–6 days.**

**Parallelisable:** T7 can start today from `docs/evidence/p4-l5-2026-09-29/scripts/matrix.py`
plus `EngineV3Bench`, without waiting for the app work.

### P5 Switch-over: gate "new ≥ old on every tier; arXiv L1 ≥ 90%; retirement complete"

**The numbers are there now** (§2), but they aren't *gated*. The critical path, in order,
following the retirement plan's (#1236) stages:

| Step | Work | Owner | Depends on | ETA (BELIEF) |
|---|---|---|---|---|
| 1 | Land #1224, #1213, #1233, #1234, #1240 and the packages tier | flashtex-2a (delegated landing for `tools/**`); Commander | the merge queue | 1 day |
| 2 | **S4: CI runs the new engine**: `parity-fixtures` with `--engine flashtex-initex` (both engines during the overlap); T1 lockstep on touched engine areas (PR); T2 and T3 arXiv/templates/packages in the queue or nightly; T6 nightly | proposal: flashtex-2a (harness) + Commander (`ci.yml`) | step 1; formats cached per runner (`formats.rs` cache) | 1–2 days |
| 3 | Engine fixes #1219 (panic), #1218 (writet3), #1220 (expansion); TrueType | kabir-claude (P3-FONTS-2 claimed; #1219 and #1220 have no branch) | — | 2–4 days; **parallelisable** (below) |
| 4 | T4 baseline on the PC; T7 gate | flashtex-2a; mac-claude-a | step 2 | 2–5 days |
| 5 | S3 app flag (default legacy), then the S5 flip per document, then a **14-day soak** | Commander (P3-APP-V3) | P3, P4 gates; step 2 green for N queue runs | flip at about day 5–8; soak to about day 20 |
| 6 | S6–S8 retirement (≈276k Rust source LOC plus ≈119k test LOC, retirement plan §0, if every "[ruling needed]" crate goes); 88 old-path PRs closed or landed | Commander + flashtex-2a | the soak | about +1 week after the soak |

**The biggest blocker is not typesetting.** It is that the Commander's own machine holds
most of the critical path:
- engine fixes #1218, #1219 and #1220;
- P3-APP-V3, P4-FINISH, P5-DIAGNOSTICS and APP-PERF-AUDIT;
- the L6 and Typst design work;
- every `ci.yml` and engine landing.

Meanwhile, the Muse lanes produce lockstep cases that find nothing new.

**What to parallelise (BELIEF, with the evidence cited):**
- **Jaysen (mac-m1max-a, mac-claude-a):** T7 now (no branch yet), and V3 zoom tiles. Also a
  good home for **#1220 (font expansion)**. It's self-contained in pdfTeX's expansion code
  (`hpack`, `get_expand_font`), with a minimal reproducer and an oracle.
- **Daniel (mac-m5pro-dq222: flashtex-2a, daniel-muse-lead):**
  - flashtex-2a: S4 CI wiring, T4, the packages tier.
  - daniel-muse-lead: **redirect Muse from lockstep waves to real-world breadth.**
    MUSE-PACKAGE-SMOKE to the full ~500 packages, ranked by real arXiv usage (the lane says
    its list is a guess); T4/T5 manifest curation; the "other engines" census (§3, item 12).
    Lockstep waves 1–4 found 0 differences, and review shows a rising duplicate rate
    (11 of 60 in #1256).
- **The NixOS PC:** T4 nightly, T6 nightly, T2 nightly on Linux, and every heavy corpus
  run. Record the TeX Live snapshot with each baseline (§3, item 9). It's already contended:
  three CI runners, the P3-FONTS-2 census and L6 `perf stat` runs overlapped this review's
  measurement.
- **The Commander:** #1219 first (a crash), then writet3; hand #1220 and TrueType to
  another machine.

---

## 5. Red-team of §1.1, §8 and §12

### 5.1 Gates that are too weak (could pass while users still see problems)

1. **"arXiv L1 ≥ 90%" (§12 P5) measures the wrong thing for a faithful engine.**
   - L1 is "same page count". With the new engine, L0 = L1 = L4 on every document (§2.2):
     when it compiles, it's exact. So L1 only measures "does it compile".
   - The threshold was set when v1 scored L1 8.6% (Appendix B.2). The new engine could
     pass at 90% while 1 document in 10 aborts or differs.
   - **Proposal (owner decision, since it changes a gate):**
     - "arXiv and T4: **P-T2 ≥ 99% and P-T1 ≥ 98% of measured documents**, with the Wilson
       lower bound reported";
     - "**0 panics or aborts** on T4 + T6 over 7 consecutive nights";
     - every remaining failure has an `engine-diff` issue.
   - Today's evidence would fail this until #1218, #1219 and #1220 are fixed, and that is
     the right outcome.
2. **"new ≥ old on every tier" is vacuous on the real-world tiers.** v1 scores 0–1% at
   P-T2 and L1 there. It has teeth only on the fixtures tier, where v1 was tuned.
   - The fixtures tier's L-levels compare against committed references. Twelve of those
     real-world fixtures and every divergence probe were made with **TeX Live 2025**
     (README, "Honest limits"). A faithful engine on TL2026 can fall below v1 there because
     of the oracle's version, not a bug. This is BELIEF: no published run gives the new
     engine's fixture L-levels against `baseline-fixtures.json`. Only its P-T1/P-T2 against
     a fresh oracle have been published.
   - **Proposal:** compare the fixtures tier's L-levels against the pinned oracle (as P-T1/P-T2
     already do), or regenerate those references from TL2026 (§8's rule: expected data only
     from the oracle).
3. **Nothing gates the path users actually run.**
   - P-T1/P-T2 are measured through the batch `flashtex-initex`.
   - The app runs `flashtex-host`: preview mode, checkpoints, restore/converge and display
     list.
   - Soundness (incremental = scratch) was shown on fixtures and synthetic documents only,
     and it is **in no gate's text**: P4's gate is latency alone.
   - **Proposal:**
     - add to P4's gate: "incremental = from-scratch (P-T1 at every shipout, byte-identical
       PDF/log/aux) over ≥ 50 random edits on each T3 arXiv document";
     - add to P3/P5: "display-list glyph positions = exported-PDF positions on the arXiv
       tier";
     - make P-T1/P-T2 run through the host binary as well.
4. **P-T2 ignores annotations, destinations, outlines and the catalogue** (§3, item 11). A
   hyperref regression would pass every gate. **Proposal:** a P-T2 component that compares
   `/Annots` (subtype, rect, action and destination), `/Outlines` and `/Names` by content,
   with object numbers resolved. `pdftext.py` already hashes objects by content.
5. **The §1.2 latency is gated at the wrong boundary.** §1.2 says "keystroke to pixels". P4's
   evidence measures `compile` → shipout in the host (≤ 16 ms), while the app measures
   18.5–37.7 ms p95.
   - Benchmark documents are synthetic (`gen.py`), and every measurement ran on a machine
     at load 3–82.
   - **Proposal:**
     - T7 measures keystroke → `CALayer` commit in the app, on the synthetic set *and* on
       the 5 longest arXiv documents;
     - it runs on a quiet, named reference host (retirement plan Q9);
     - it gates on p95 with a documented noise margin.
6. **Preview parity allows unlimited PDF fallback.** A page drawn from the exported PDF is
   trivially "identical". **Proposal:**
   - the P3 gate states the fallback rate, measured on arXiv, not only on fixtures;
   - T7 includes a fallback-heavy (tikz shadings) document, because fallback costs ≈25 ms
     a page.
7. **No gate covers the machine without TeX Live** (§3, item 8). P5 would flip users who
   rely on v1's bundled resources to an engine that can't run for them. **Proposal:**
   - P5 adds "**bundle mode: P-T2 on fixtures and arXiv with no TeX Live on PATH**, from a
     hosted, pinned bundle";
   - or the flip is limited to machines with TeX Live, until the bundle exists (owner
     decision: hosting).
8. **The sample is narrow** (§2.4). **Proposal:** P5's corpus numbers come from T4 (5,000
   documents, 20 categories × 10 years) as well as T3.
9. **T0 says "must pass", but nothing enforces it.**
   - `trip`, `etrip` and `pdftex-regression` are outside `CI required`, the only required
     check (§1).
   - T0's regression item is also mis-specified: "28 CTAN regression directories" against
     7 `pdftex.am` tests.
   - **Proposal:**
     - add the three jobs to `CI required`'s `needs`;
     - reconcile the text with what exists, or add the missing directories.
10. **"Main green" means v1 green.** `perf` and `parity-fixtures` on main measure v1 only
    (§1), and the Commander's master prompt treats their green as the health signal.
    **Proposal:** until S4 lands, record in DESIGN's phase status that P2/P3 parity is
    *not CI-protected*.

### 5.2 Gates that are unnecessarily strict, or misplaced

1. **"Retirement complete" inside the P5 *exit gate*** couples switching users over (a
   product milestone) with deleting ≈276k Rust source LOC (maintainability). The retirement plan
   already inserts a 14-day soak between them. **Proposal:** split P5 into P5a (flip: the
   scoreboard, latency, bundle and crash gates) and P5b (retirement complete, after the
   soak). It is sequencing, not a weaker bar.
2. **P-T1 on documents whose logs are nondeterministic in pdfTeX itself** (`\int_rand`,
   `\pdfelapsedtime`, the tabu stop-time). Reviewers already mark these "not evaluated"
   (5902143192). **Proposal:** codify `\pdfsetrandomseed` pinning (applied to both engines,
   like `SOURCE_DATE_EPOCH`) as a run setting in §1.1. It isn't a normaliser, so P-T1 stays
   strict and fewer documents drop out of it.
3. **P-T1 byte-identical `\tracingall`** stays the right primary gate (BELIEF). It is what
   found #1220's unset glue fields, which P-T2 missed on 2501.07077v1 (P-T2 pass, P-T1
   fail). Nothing to relax.
4. **Growing T1 toward 1,500+ lockstep cases** isn't too strict, but it is poorly aimed now
   (§4). **Proposal:** keep T1 as a CI gate at its current size. Move growth effort to
   coverage-guided selection: add cases only where engine line coverage from T3/T4 shows
   gaps.

### 5.3 Proposed changes, and who decides

| Change | Decides |
|---|---|
| Replace "arXiv L1 ≥ 90%" with P-T2/P-T1 thresholds plus crash-freedom (§5.1.1) | **owner** (a phase gate) |
| Split P5 into P5a flip and P5b retirement (§5.2.1) | **owner** (a phase) |
| A bundle-mode gate, or limit the flip to machines with TeX Live (§5.1.7) | **owner** (product and hosting) |
| Incremental = scratch in P4's gate; host-path P-T1/P-T2; display-list positions on arXiv (§5.1.3) | Commander (a technique refinement within §8) |
| P-T2 compares annotations, outlines and names (§5.1.4) | Commander |
| T7 at keystroke → pixels in the app, on a named quiet host (§5.1.5) | Commander |
| Fixtures L-levels against the pinned oracle, or regenerated TL2026 references (§5.1.2) | Commander |
| `trip`/`etrip`/`pdftex-regression` into `CI required`; `max_entries_to_build` raised to the runner count (§1, §4 P3) | Commander |
| TL snapshot recorded in scoreboard metadata (§3, item 9); T0 text reconciled (§5.1.9) | Commander |
| An owner and a lane for T5, TrueType embedding and SyncTeX file output | Commander |

---

## Appendix: reproduction

On the NixOS PC, from `~/code/flashtex-review4` (main `02dcf9d07`) and
`~/code/flashtex-review4-harness` (#1224 `679ba3c00`):

```sh
export PATH=~/texlive/2026/bin/x86_64-linux:$(nix eval --raw nixpkgs#ghostscript.outPath)/bin:~/.nix-profile/bin:$PATH
CARGO_BUILD_JOBS=6 cargo build --release --locked -p flashtex-engine --bin flashtex-initex
# formats (FLASHTEX_TEXLIVE_BIN is required on the PC: see §3 item 10)
FLASHTEX_TEXLIVE_BIN=~/texlive/2026/bin/x86_64-linux FLASHTEX_POOL=<eng>/pdftex.pool SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 \
  <eng>/flashtex-initex -ini -jobname=pdflatex -progname=pdflatex -translate-file=cp227.tcx '*pdflatex.ini'
python3 tools/parity/corpus.py --cache <C> --texmf ~/texlive/2026/texmf-dist fetch
python3 tools/parity/parity.py --tier arxiv --tier templates --cache <C> --texbin ~/texlive/2026/bin/x86_64-linux \
  --texmf ~/texlive/2026/texmf-dist --oracle-pdftex ~/texlive/2026/bin/x86_64-linux/pdftex --pt1-max-log-mb 2048 \
  --pt1-skip arxiv/2501.08663v2 --pt1-skip arxiv/2501.08950v2 --pt1-skip arxiv/2501.07413v3 --pt1-skip arxiv/2501.10183v1 \
  -j 4 --engine <eng>/flashtex-initex --engine-kind tex --engine-env FLASHTEX_FORMATS=<fmt> \
  --engine-env FLASHTEX_POOL=<eng>/pdftex.pool --engine-env FLASHTEX_TEXLIVE_BIN=~/texlive/2026/bin/x86_64-linux --out <out>/new
# v1: --engine crates/flashtex-cli's release `flashtex` --pt pt2; self-test: --engine <TL>/pdftex --engine-kind tex --pt pt2
```

Raw outputs are in `track-4-raw/engines-nixos-2026-09-30.{md,json}`, the side-by-side
from `engines.py` with `--notes tools/parity/reports/arxiv-scoreboard-2026-09-29.notes.json`.
Home-directory paths are replaced by `<TEXLIVE>`/`<path>`.

Run times: the new engine took 33 min (`-j 4`, cold oracle including traced passes); v1
and the pdfTeX self-test took 16.5 min together (`-j 3` each, `--pt pt2`). All ran at
`nice 19` on a PC also running three CI runners and other lanes' benchmarks (load up to 44).
