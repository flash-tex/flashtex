# FlashTeX Engine v2 — Design Specification

**Status:** ACTIVE, the governing design for FlashTeX engine work as of 2026-09-29.
**Owner:** Kabir (project owner). **Author:** kabir-claude (Commander, mac-m5pro-kabir).
**Authority:** this document is the **single, ultimate source of truth** for FlashTeX
engine work. It is binding on every Commander, every lane and every agent on every
machine. Where it conflicts with any other file, issue comment, handoff or agent
instruction, it wins. Deviations need evidence and owner approval and are recorded in
§13 (Decision log). It is kept current by the mandatory two-weekly design review (§14).
Appendix A is the master prompt for any Commander session.

**Next design review due: 2026-10-13** (then every 14 days; see §14).

**Phase status (verified gates):** P0 ✔ (2026-09-29) · P1 ✔ trip byte-identical (2026-09-29) ·
P2 ✔ verified independently on main `d4f2a1581` (2026-09-29): trip, etrip 18/18,
pdfTeX regression 7/7, lockstep 260/260, parity fixtures P-T1 83/83 and P-T2 83/83,
T2 LaTeX suites 1,520/1,529 with 0 unexpected failures · P3 and P4 in progress.

---

## 0. Summary

FlashTeX replaces its hand-written LaTeX re-implementation with a **faithful TeX
engine**: a Rust port of pdfTeX (TeX82 + e-TeX + pdfTeX primitives) that runs the
**real `latex.ltx` format and unmodified TeX Live packages**. Parity with pdflatex
therefore holds by construction, not by porting packages one at a time.

Speed does not come from doing less than TeX does. It comes from:

- a resident engine;
- page-level copy-on-write checkpoints;
- restarting at the edit and stopping once the engine state converges;
- viewport-first typesetting;
- a native Core Graphics/Core Text preview that is pixel-identical to the exported PDF.

Measured on an M5 Pro, real TeX already sets a page of text in 0.8–4 ms. Its visible
slowness is fixed per-run cost (170–410 ms), and that cost is what we remove.

---

## 1. Goals, priorities, success metrics

Priorities, in strict order (owner, 2026-09-21; unchanged):

1. **Full parity.** 100% usable for any TeX document that pdflatex compiles.
2. **Speed.** No *noticeable* latency on warm compiles, including long and complex
   documents. 1 ms versus 10 ms doesn't matter; noticeable delays do.
3. **Maintainability and stability.** Nothing is hand-added one symbol or package at
   a time. Nothing crashes. The code is readable, and every behaviour has a reference.

**Reuse before building (owner, 2026-09-29).** When an open-source component already
does exactly what we need with no compromise, use it; don't reinvent it. "No
compromise" means it meets our parity bar, it is at least as fast on every path that
matters, its licence fits its side of the §3 boundary (nothing GPL on the MIT side),
and it is maintained or small enough to own. Build our own only when it is measurably
faster on a path users notice (a faster wheel), or when nothing existing meets the bar.
Every lane that builds something states in its report what it evaluated and why it
did or didn't reuse it. Porting `pdftex.web` rather than writing a new TeX (D2/D3) and
shipping TeX Live's zlib (§6.3) are this rule applied.

### 1.1 Parity definition (gating tiers)

| Tier | Definition | Role |
|---|---|---|
| **P-T1** | At every `\shipout`: identical box dumps (`\showbox`/`\tracingoutput`, `\showboxdepth=\showboxbreadth=∞`) and identical `\tracingall` logs to a pinned pdfTeX, both **in PDF mode**. | **Primary gate** |
| **P-T2** | Identical embedded font subsets and identical content streams after qpdf normalisation and object renumbering. | Export gate |
| **P-T3** | Byte-identical PDF, including `/Producer`, IDs and dates. | **Optional** "reproducible export" mode only; never a gate |

**P-T1 normalisation (ruled 2026-09-29).** Only capacity and output-size accounting
is normalised out of P-T1 logs, identically in `tools/lockstep` and `tools/parity`:
- `\tracingstats` memory-usage lines ("Memory usage before/after", "still untouched");
- the end-of-run "Here is how much of TeX's memory you used" block;
- the "PDF statistics" block;
- the **byte count** in "Output written on … (N pages, B bytes)". The page count stays
  compared.

These reflect the memory representation (§4.2) and the PDF writer (P-T2 territory),
not typesetting. Each harness still reports them as a separate, non-gating
**accounting check**, so drift stays visible. Everything else in the log stays strict.

Byte identity is not the goal. Matching bytes would mean forging pdfTeX's
`/Producer` and banner strings and chasing every pdfTeX release. Users perceive
breaks, positions and glyph shapes, which P-T1 and P-T2 cover completely.

### 1.2 Latency targets (M-series; checked by benchmark gates in §9)

| Situation | Target, keystroke to pixels |
|---|---|
| Edit inside a paragraph, any document length (up to 1,000 pages) | ≤ 16 ms p95 for the edited page |
| Edit that reflows later pages | edited page ≤ 16 ms; later pages refreshed in the background, stale ones marked |
| Preamble edit | ≤ 400 ms to first visible page (full run from the format) |
| Reopening a recently edited document | ≤ 100 ms to first visible page (persisted snapshot) |
| Scroll and zoom | 120 Hz; no drawing on the main thread |

---

## 2. Decisions and their evidence

These decisions came from a six-track research study, a fresh parity evaluation, pdfTeX
benchmarks, an adversarial review and preview-renderer measurements, all dated
2026-09-29. The evidence is summarised in Appendix B.

| # | Decision | Rejected alternatives and why |
|---|---|---|
| D1 | **Faithful TeX engine running the real `latex.ltx` and packages** | *A (continue hand-porting)*: 70 packages hand-ported (≈42k LOC), 1 real `.sty`, arXiv L0 = 1.4%, and a structural ceiling. *C (embed pdfTeX/XeTeX C)*: rejected by the owner. |
| D2 | **Port pdfTeX (GPL-2+) in its own crate; the rest stays MIT** | *Clean-room MIT*: same runtime speed, but slower to build and less exact. The engine's language of origin has no effect on speed (§4). |
| D3 | **Port method: mechanical WEB→Rust translation of `pdftex.web`, keeping identifiers, § numbers and comments, then modernise under lockstep differential tests.** pdfTeX's C parts (writet1, image and PDF inclusion) are ported per file. | *c2rust of web2c's C*: loses names and comments (Tectonic's attempt was never merged). *Unaided hand port*: tex-rs stalled before reaching `\dump`/trip. web2js and web2w show that WEB translation passes trip. |
| D4 | **Keep TeX's data model** (flat `mem`/`eqtb`/save-stack/string-pool arenas, u32 tokens). Modernise only its representation, with semantics identical. | *Object-graph redesign*: NTS was trip-compatible but 10–20× slower and never extensible. RusTeX's approximate typesetting fails parity. |
| D5 | **Parity gate = P-T1 and P-T2**; P-T3 optional | Byte identity forces forged producer strings and gives nothing a user can perceive. |
| D6 | **Preview = Core Graphics/Core Text drawing from the display list**, cached per page, tiled at high zoom, composited by Core Animation (GPU) | *Custom Metal glyph renderer*: saves ≤ 0.4 ms of an 8.3 ms frame, differs from the exported PDF on 0.05% of pixels (fails the zero-tolerance parity check), and doubles renderer maintenance. *PDF + PDFKit per update*: +25 ms per update, +31 ms cold. *Vello/Slug*: don't match Core Text. See Appendix B.3. |
| D7 | **Checkpoint at every `\shipout` plus every ~20 ms of engine time; not per paragraph** | Measured: about 0.25–0.5 MB dirtied per page, against about 5× the memory for paragraph checkpoints that save at most about 2.4 ms. |
| D8 | **Convergence by a canonical incremental 128-bit state hash, plus external read-sets** | Raw memory comparison never matches after an edit (addresses shift). INCTeX's lesson is that dependency checks are required alongside state comparison. |
| D9 | **Guarded intrinsics deferred** until P-T1 parity | The review found tracing, `\globaldefs`, `\afterassignment` and error-path hazards. |
| D10 | **Speculative parallel segments dropped** | They win only when L3 has already stopped early. |
| D11 | **App ⇄ engine over a public, versioned display-list protocol on a Unix socket** | *Shared memory of internal structures*: the GPL FAQ treats that as close to linking. The socket costs 61 µs/MB. |
| D12 | **Use the user's TeX Live when present, a Tectonic-style bundle otherwise; build our format from their `latex.ltx`, cached by content hash** | *Our own format against their packages*: fails with "Mismatched expl3 files". |
| D13 | **Old engine frozen: fixes only** | Effort on code that D1 deletes is waste. |
| D14 | **Distribution outside the Mac App Store (DMG); iPad stays a separate MIT companion with no engine code** | The App Store conflicts with the GPL. The iPad talks to the Mac over its own protocol. |

---

## 3. System architecture and licensing boundary

```
 ┌───────────────────────── Mac app (MIT, Swift) ─────────────────────────┐
 │ editor · IDE · preview renderer (CG/CT → per-page/tiled layers → CA)    │
 │ protocol client ── display-list-v3 over Unix socket ──┐                 │
 └───────────────────────────────────────────────────────┼─────────────────┘
                                                         │ (public, versioned)
 ┌──────────────── flashtex-engine host process (GPL-2+) ┴─────────────────┐
 │ engine core (ported pdftex.web) · incremental system · file resolver     │
 │ display-list writer · PDF backend (ported writet1/images/pdftoepdf)      │
 └──────────────────────────────────────────────────────────────────────────┘
 iPad companion (MIT) ── nearby-v1/transfer-v1 ──► Mac app   (no engine code)
 TeX Live content (LPPL etc., unmodified) — user's install or fetched bundle
```

- **`crates/flashtex-engine`:** GPL-2-or-later, with a `LICENSE` in that directory.
  Nothing MIT may depend on it at link time, which a CI check enforces (§9).
- **Display-list protocol (`display-list-v3`):** documented in `docs/contracts/`,
  engine-independent (DVI/XDV-like semantics), MIT specification.
- **iPad target:** may not link any GPL crate, which a CI dependency-graph check
  enforces. The bundled `supported-latex.json` must be generated from MIT data, not
  from GPL code.
- **Ghostscript, if bundled for EPS** (Ventura removed native EPS conversion): AGPL,
  so run it strictly as a separate process.
- **Naming:** never "TeX engine" or "pdfTeX" in product text. Use "pdfLaTeX-compatible".
- **Linked upstream libraries inside the engine (measured choices, P3):** TeX Live
  2026's kpathsea (LGPL-2.1), zlib (zlib licence), libpng (libpng licence) and
  **xpdf 4.06, which is GPL v2 or v3 only (not "or later")**. A binary that links xpdf
  is therefore distributable under GPL v2 or v3 only, exactly as pdfTeX itself is. The
  engine's own code stays GPL-2.0-or-later. Every vendored library carries its version,
  licence and sha256 in `third_party/<lib>/README.md`.
- **Legal review:** the owner arranges a review of §3 before public release. Development
  may proceed.

---

## 4. Engine specification

### 4.1 Source of record and port method (D3)

- **Reference:** `pdftex.web` (TeX Live 2026, pdfTeX **1.40.29**, pinned), which
  already includes e-TeX. The system-dependent behaviour web2c adds (file lookup,
  capacities, `\write18`, SyncTeX) is re-specified by us.
- **Step 1, `web2rust`.** A translator (in `tools/web2rust/`) takes the WEB Pascal subset
  to Rust. It keeps every identifier and section number (`// §NNNN`) and carries the
  WEB commentary as doc comments. It handles `goto` with labelled loops, as web2js
  did. Its output is committed and regenerable.
- **Step 2, modernise in place, one commit at a time,** each under the lockstep
  differential harness (§8). Remove globals into an `Engine` struct and use typed
  indices. Every change must keep P-T1.
- **C parts** (`writet1.c`, `writeimg`, `pdftoepdf.cc` (xpdf-based), `writeenc`,
  `writefont`, `mapfile.c`, `utils.c`) are ported per file under the same harness.
- **Existing crates** (tex-expansion, tex-boxes, paragraph-layout, math-layout,
  page-builder, TFM readers) become **oracles and test fixtures**. They are not the
  engine; one engine means one source of truth.

### 4.2 Data model and representation changes (semantics identical)

- `mem`, `eqtb`, `hash`, `save_stack`, `str_pool`, `font_info` and `trie` are flat
  arenas of plain-old-data. They are snapshot-friendly (§5.2) and pointer-free, so the
  format can be memory-mapped.
- **Allowed optimisations,** each proven identical by the lockstep harness:
  - `divide_scaled` as one exact 64-bit operation (it is 13% of a run today);
  - the font map parsed once per process, not per run (80–90 ms per run today);
  - an uncompressed, page-aligned, memory-mapped format (a gzipped format costs about
    27 ms to load);
  - contiguous token-list storage, but only after P-T1 on the full suite (§12, P6).
- **Floating point:** no fused multiply-add. Match pdfTeX's operation order in glue
  setting. (The reference arm64 binary contains 0 fmadd instructions.)

### 4.3 Scope of primitives

- All of TeX82, e-TeX and pdfTeX 1.40.29, including every primitive l3kernel requires:
  `\expanded`, `\pdfstrcmp`, `\pdfmdfivesum`, `\pdffilesize`, `\pdffilemoddate`,
  `\pdffiledump`, `\pdfsavepos`/`\pdflastxpos`/`\pdflastypos`, `\pdfuniformdeviate`
  and the other random primitives, `\pdfelapsedtime`, `\pdfprimitive`/`\ifpdfprimitive`,
  `\ifincsname`, `\pdfshellescape`, `\tracingstacklevels`, `\tracinglostchars` and
  the rest.
- Font expansion and protrusion (`\pdfadjustspacing`, `\pdfprotrudechars`, `\efcode`,
  `\lpcode`/`\rpcode`) are required by microtype.
- **No "lenient mode" ever.** An unknown primitive is a hard, reported error.

### 4.4 Files and distribution (D12)

- A kpathsea-compatible resolver: honour `ls-R`, `TEXMFHOME`, `TEXMFLOCAL` and
  `texmf.cnf`. GUI apps don't inherit the shell environment, so resolve explicitly.
- Prefer the user's TeX Live. Build our format from *their* `latex.ltx` and hyphenation
  patterns (about 4 s), cached by the content hash of every input. Use *their* `pdftex.map`.
- **Fallback:** a content-addressed bundle (SHA-256-pinned, byte-range fetch, Tectonic
  model) with a small core plus on-demand packages.
- TeX Live content is shipped or used **unmodified** only (LPPL clause 6 is never needed).

### 4.5 Determinism and safety

- Pin `\time`, `\day`, `\month`, `\year` and the random seed per editing session,
  because pgf seeds from `\time*\year`. Export honours `SOURCE_DATE_EPOCH`.
- **`\write18` is restricted by default, exactly like TeX Live's pdflatex** (owner,
  2026-09-29). Only texmf.cnf's `shell_escape_commands` run (repstopdf, makeindex,
  bibtex, kpsewhich, extractbb and the rest), with web2c's argument quoting, and
  `\pdfshellescape` reads 2. Full shell escape needs an explicit per-project opt-in
  with a warning, and off stays available. Rationale: parity with stock pdflatex (EPS
  via epstopdf, automatic indexes, and l3kernel's `\sys_if_shell` code paths) outweighs
  the small residual risk of the vetted list. Harness reference runs use pdfTeX's
  default mode. Any executed command is an L3 barrier (§5.3).
- File reads and writes are confined to the project and the TeX trees.
- Resource limits: time, memory and recursion.
- **No-panic contract:** every engine error becomes a TeX error or a structured
  diagnostic. The host process isolates crashes.

---

## 5. Incremental system

### 5.1 L1: resident engine and post-preamble snapshot

- The engine host process stays resident per document. At `\begin{document}` it takes
  snapshot S₀.
- The key for S₀ is the **read-set**: the content hash of every file read, the `.aux`,
  the pinned date, the job name and the engine build. Never key on preamble text alone.
- S₀ persists to disk (memory-mapped), which gives the ≤ 100 ms reopen target.

### 5.2 L2: checkpoints

- **When:** at every `\shipout`, plus every ~20 ms of engine time (for heavy tikz and
  pgfplots pages, measured at about 90 ms per plot page).
- **Mechanism (measured 2026-09-29, `docs/evidence/snapshot-bench-2026-09-29/`):**
  software copy-on-write, specifically **(b3)**:
  - All mutable arenas live in **one flat word space** with a 16 KB dirty bitmap. Reads
    are plain loads, and only writes pass the barrier (+0.27–0.33 ns per access on a
    replay loop).
  - A checkpoint seals an **undo log** holding each chunk's contents before its first
    write since the previous checkpoint. The snapshot costs 2.3–7.5 µs at 64–200 MB.
  - Chunks come from a slab, not `malloc`, which carries 1.24× overhead on 16 KB blocks.
  - **Restoring** to an old checkpoint walks the chained logs. Restoring the newest
    takes 120–259 µs; 999 checkpoints back takes 5.4 / 18.5 ms serially, or 1.7 / 5.6 ms
    with 8 workers, at 64 / 200 MB. **Parallel restore is required.**
  - A restore **captures a redo log** of the chunks it overwrites. After convergence
    (§5.3) the engine jumps back to the old run's later state and keeps its later
    checkpoints (≤ 3 ms, bit-exact in tests).
  - **Adjacent logs are compacted** for memory (2.70 GiB → 680 MiB over 1,000
    checkpoints at 64 MB). Compaction doesn't shorten restores.
  - **Rejected, with measurements:**
    - (a) `mach_vm_remap`/`vm_copy`: copy faults cost 12× a software chunk copy, and
      the kernel's private copies don't appear in `phys_footprint`, so the memory budget
      can't be enforced in-process.
    - `Arc` chunk tables: over 1 ms per snapshot at 200 MB.
    - A full memcpy: 1.2–3.6 ms.
  - **Still open, closed in P4:** the 3% hot-loop gate needs the real engine's hot loop
    as its denominator. Re-run the barrier phase there. If far-back restores dominate
    the p95 of edited-page latency, build shadow-paged keyframes as the fallback.
- **Each checkpoint records:** input consumed per file, **at line granularity**
  (`\futurelet` looks ahead), the running state hash, and the per-page external-effect
  logs (`\write`, `\openout`, PDF objects, `\pdfsavepos`, marks and inserts).
- **Retention:** dense near the cursor and viewport, log-spaced elsewhere (TeXpresso's
  decimation). The **budget drives the policy** (default 1 GB): a fixed dense-16 plus
  4-per-octave scheme measured 0.68–3.69 GiB over 1,000 checkpoints, so the spacing
  is derived from the budget, not fixed.

### 5.3 L3: restart and converge (D8)

- **Restart** from the newest checkpoint whose consumed input is entirely before the
  edited line.
- **Stop early** at a later checkpoint *k* when both hold:
  - (a) the canonical 128-bit state hash equals the old run's at *k*;
  - (b) no external input read by old pages ≥ *k* has changed.
- The state hash is structural and incremental, updated on every `eq_define` and
  combined with hashes of box registers and lists. It is independent of addresses,
  string numbers, hash slots and font indices.
- **PDF object numbers and font numbers are relocatable.** Reuse is blocked only if
  later pages read `\pdflastobj`, `\pdflastxform`, `\pdflastannot` or `\pdfpageref`.
- **Barriers** (they block reuse past the point where they are read): `\write18`,
  `\pdfelapsedtime`, `\pdffilemoddate` of a changed file, `\read` of a file written in
  this run.
- **External writes** are spliced from per-page logs (`.aux`, `.log`, `.toc` and
  others). `\end{document}` always re-runs.

### 5.4 L4: viewport first

- Re-typeset from the restart point up to the page being viewed, render it, then
  continue to convergence (or the end) in the background.
- Later pages that aren't current yet are shown **marked stale**.
- **Page-count shifts:** an edit that adds a page changes `\c@page` for the rest of
  the document, so the state never converges. That common case is handled by viewport
  first, not by convergence.
  - The background pass then runs to the end.
  - Measured on pdfTeX at 1,000 pages: 0.6–0.8 ms per page for prose and math, and
    5.9 ms per page with hyperref, siunitx and footnotes.
  - Evidence: `docs/evidence/reflow-galley-2026-09-29/`.
  - Two things shorten it: §5.6 item 4 (hyperref's per-page cost, a floor for every
    strategy) and §5.7 (the segment memo).

### 5.5 L5: multi-pass and memoisation

- **`.aux`:** the previous run's `.aux` is fixed input. Each page records which
  `\r@…`/`\b@…` entries it read. Re-run in the background only when a read entry
  changed. Detect oscillation (repeated states) as well as capping at 5 passes.
- **tikz/pgfplots picture memoisation** comes later (P6). It needs epoch-level
  read-sets and must handle `remember picture`, global counters and shadings. The
  CTAN `memoize` package is prior art.

### 5.6 L6: raw-speed program (after P-T1 parity)

1. Profile first; every item needs a measured win.
2. Contiguous token lists and argument matching without allocation.
3. A fast `\csname` lookup.
4. **Guarded intrinsics** (D9). Only pure, non-erroring leaf functions. The dependency
   set is computed when the format is built. An O(1) counter of redefinitions acts as
   the guard. Preconditions are `\globaldefs=0` and no pending `\afterassignment`.
   **In CI both paths run and the full state change is diffed.**
5. NEON input scanning only if the profile shows scanning matters.
6. **First named intrinsics target: the per-page output routine.** Measured 2026-09-29:
   - hyperref's two per-page PDF-string calls (page label and page anchor) cost
     0.15 ms per page, and 1.96 ms per page once siunitx is loaded;
   - that is most of LaTeX's pagination cost, and the floor under every
     large-reflow strategy.

### 5.7 L3.5: segment memo for large reflows (planned for P4, gated)

This is the owner's "one long page" idea in its verified form. Evidence:
`docs/evidence/reflow-galley-2026-09-29/`.

- **Cutting at fixed heights is rejected.** It can't match pdfTeX's page breaking,
  glue setting, inserts or output routine (tex.web §§987, 1000–1017).
- **What is cached:** the galley material, while TeX's page builder and output routine
  always re-run live.
  - **Cache unit:** a *segment*, meaning everything executed between two consecutive
    outer `build_page` calls. Not a paragraph: the output routine can fire inside one
    (§1091, and around display math at §§1145 and 1200).
  - **Key:** the values of everything the segment read: eqtb, input stack, nesting
    state, page-builder state, object counters, random seed, fonts, files and `.aux`
    entries.
  - **Hidden reads that must be tracked:** `\pagetotal`, `\pagegoal`,
    `\lastpenalty`, `\lastskip`, `\pdflastximage`, output-routine `\aftergroup`
    tokens and absolute line numbers.
  - **Replay:** the write-set. Nodes are spliced in as fresh copies, because the page
    builder mutates nodes in place. Memory-usage log statistics are excluded, and
    nondeterministic reads are barriers.
- **Measured potential on pdfTeX:** 1.6–4.4× on real documents and 1.8–2.5× on
  synthetic ones. It rises to 12× only when the per-page output routine is cheap (see
  §5.6 item 4).
- **Projected hit rate:** about 91% of segments. It is lower with cleveref (25–30%
  misses), and after edits that renumber equations or footnotes.
- **Gate:** build it only if, on the real engine, it cuts the 1,000-page background
  reflow by ≥ 2× with P-T1 unchanged. Every shipout of a memoised run must equal a
  from-scratch run. Drop it if the median miss rate exceeds 30%.

---

## 6. Output and preview

### 6.1 Display list (`display-list-v3`)

- Per page: positioned glyphs (font resource id, glyph id, x/y in scaled points),
  rules, paths, images, links and source-span ids.
- Engine-independent and versioned. It extends today's `display-list-v2`, with
  per-page content hashes for caching.
- **Source mapping (SyncTeX-equivalent)** travels in the same stream, keyed by stable
  span ids rather than byte offsets.

### 6.2 Preview renderer (D6)

- **Keep and extend the existing path:** `V2PreparedPage`, then
  `GlyphRunRenderer.draw` (`CTFontDrawGlyphs` at absolute origins), then
  `V2PageRasterizer`, then per-page bitmaps as `layer.contents`, composited by Core
  Animation. `V2Parity` stays at zero tolerance.
- **Add tiling above about 3 pixels per point:** 512 px tiles for the visible area
  only, rasterised in parallel (about 0.1 ms per tile), using a custom tile layer
  rather than `CATiledLayer`, to avoid its fade-in and resize jitter.
- **During a pinch,** transform the current bitmap on the GPU, then re-raster the
  visible tiles once the gesture settles (1–3 ms).
- **Font smoothing:** decide by capturing Preview.app on screen and matching its ink.
- **Type 1 fonts** (from the ported writet1 export): Core Text no longer supports
  Type 1 (Ventura). The preview draws the same outlines through a converted in-memory
  font or via FreeType (MIT-compatible).
  - **Gate:** zero pixel difference against Core Graphics' rendering of the exported PDF.
- **A custom Metal renderer is justified only if** profiling shows raster time
  dominating, for example pages above 50k glyphs. Even then it would be a Metal
  compositor over Core Graphics-rasterised tiles, never a separate glyph rasteriser.

### 6.3 PDF export

- Port pdfTeX's backend: writet1 (the same Type 1 subsets as pdflatex, which is
  required for identical rendering in every viewer), encodings, image inclusion that
  copies PNG IDAT unchanged, and PDF inclusion.
- Use **TeX Live's own zlib version**, statically linked (never miniz, the system
  libz, or a fork).
- **Gate:** P-T2.

---

## 7. Stability

- No panics (§4.5).
- Fuzzing (§8, T6).
- A deterministic engine.
- The host process isolates the app from engine faults.
- Pinned TeX Live, LaTeX and pdfTeX versions, upgraded twice a year with the LaTeX
  release cadence.
- On each upgrade, regenerate every oracle in a pinned environment and read pdfTeX's
  `ChangeLog`/`NEWS` diff before merging.

---

## 8. Verification tiers

| Tier | Contents | When | Gate |
|---|---|---|---|
| T0 | Unit tests; `web2rust` round-trip; **trip** and **etrip** (TeX Live's accepted-difference filters); pdfTeX's 28 CTAN regression directories | Every commit | must pass |
| T1 | **Lockstep differential tests** against the pinned pdfTeX: `\tracingall` logs and box dumps on primitive-level cases | Every PR | 0 new differences |
| T2 | LaTeX team suites: latex2e `base`+`required` (1,522 `.lvt`) and l3kernel/expl3 (267 `.lvt`) against pdfTeX `.tlg`, normalised by l3build | Every PR (touched areas); merge queue (full) | must match |
| T3 | Parity scoreboard (`tools/parity`): fixtures, arXiv and templates tiers, switched to box-dump plus P-T2 levels | Merge queue | never falls (ratchet) |
| T4 | About 5,000-document corpus | Nightly | ratchet |
| T5 | 30,000+ documents, kept out of the repo, about 0.01% resolution | Weekly | tracked |
| T6 | Differential coverage-guided fuzzing of the tokenizer, expander and box builder against pdfTeX, plus parser fuzzers (TFM, Type 1, OpenType, PNG, JPEG, PDF) | Continuous | no panics, no new differences |
| T7 | Latency benchmarks (§1.2) on 10/100/300/1,000-page documents | Merge queue | targets met |

Rules:
- **Expected data is only ever regenerated from the oracle**, never edited by hand.
- **Never record host-dependent data on a different host.** The `unicode-accents`
  perf digest is the precedent: Mac-rendered digests broke the Linux baseline.

---

## 9. Development process and CI (owner-approved, 2026-09-29)

1. **Local-first gates.** `scripts/gate.sh {quick|pr|full}` runs exactly the CI steps.
   Agents run `pr` before pushing, so CI mostly confirms.
2. **Tiered CI.**
   - **PR (required, target ≤ 10 min):** build, touched-crate tests, T0, T1 on touched
     areas, parity fixtures, licence-boundary check.
   - **Merge queue:** full matrix, T2, T3, T7.
   - **Nightly:** T4, T6. **Weekly:** T5.
   - Branch protection plus a GitHub merge queue on `main`.
3. **Self-hosted runners on the team's Apple Silicon Macs,** with the security rules a
   public repo requires:
   - never run on `pull_request` from forks;
   - self-hosted jobs only on `push` to in-repo branches, `merge_group`, `schedule` and
     `workflow_dispatch`;
   - outside-contributor workflows need approval;
   - an ephemeral working directory per job;
   - labelled runner groups (`macos-arm64-selfhosted`) with a GitHub-hosted fallback.
4. **Retire `crates/render-pipeline/vendor/` now.** No landing ever needs a re-pin
   again (the re-pins were about a third of all commits).
5. **Coherent landings.** No stacking, at most 3 branches per machine in CI, and an
   in-repo branch per lane.
6. **Licence-boundary check in CI:**
   - no MIT crate or app target links `flashtex-engine`;
   - the iPad target's dependency graph contains no GPL.
7. **Disk hygiene on every machine.** Each agent worktree carries a multi-GB Cargo
   `target/`; 503 GB had built up on mac-m5pro-kabir by 2026-09-29.
   `scripts/clean-worktrees.sh --apply` deletes build outputs in idle worktrees and
   removes clean agent worktrees under `.claude/worktrees/` (a detached head that no
   branch contains is kept as `archive/<name>` first). It never touches a worktree
   that is locked, in use, modified in the last 6 hours or has uncommitted changes.
   - A lane removes its own worktree when it lands or is parked.
   - The Commander runs the script at every checkpoint.
   - Every machine runs it at least daily (a scheduled job on each Mac and Linux host).

---

## 10. What is reused, retired, and newly built

- **Reused as-is or extended:**
  - the Mac app and IDE;
  - the preview renderer (§6.2);
  - the parity tooling (`tools/parity`, visual-oracle);
  - oracle fixtures and the corpora;
  - `display-list-v2`, which becomes v3;
  - the iPad companion;
  - the coordination tooling.
- **Kept as oracles and tests:** tex-expansion, tex-boxes, paragraph-layout,
  math-layout, page-builder, the TFM readers.
- **Retired at P5 switch-over:**
  - the compiler's hand parser, about 42k LOC of package ports and the marker hand-off;
  - render-pipeline's adapter and typeset paths that the engine supersedes;
  - the three math layout copies, the vendored copies and the ten unused crates.
- **New:** `tools/web2rust`, `crates/flashtex-engine`, the incremental system,
  `display-list-v3`, the socket protocol, `scripts/gate.sh`, and the CI tiers and
  runners.

---

## 11. Risks and open questions

| Risk | Mitigation |
|---|---|
| Legal interpretation of §3 | Owner-arranged review; boundary enforced in CI from day one |
| xpdf reads `~/.xpdfrc` (outside the project and TeX trees), as pdfTeX does | Confine it with the §4.5 file sandbox before release; keep pdfTeX behaviour for parity runs |
| `web2rust` translation harder than expected | P0 spike; gate is trip passing in P1; fallback is a hand port guided by the same § numbers |
| Snapshot memory on very long documents | Retention budget; measured in P4 |
| Page-count shifts defeat convergence | Viewport first (L4) is the answer; convergence is an optimisation |
| Preview of Type 1 fonts | Gate in §6.2 |
| Upstream churn (LaTeX twice a year, pdfTeX fixes) | Pinning plus a twice-yearly upgrade with regenerated oracles |
| User TeX Live skew | Format built from their files (D12) |

---

## 12. Phases and exit gates

| Phase | Scope | Exit gate |
|---|---|---|
| **P0 Foundations** | Freeze the old path (fixes only); retire `vendor/`; tiered CI with merge queue, branch protection, `gate.sh` and self-hosted runners; engine crate skeleton with GPL LICENSE and boundary check; `web2rust` spike on `tex.web`; snapshot-mechanism micro-benchmark; land the stuck compiler re-pin (#1122) and critical fixes for users | All of those in place; the spike translates `tex.web` and compiles |
| **P1 TeX82 core** | `web2rust` over `tex.web`; INITEX, `\dump`/undump, byte-exact logs, DVI writer for the trip test; file resolver | **trip passes** |
| **P2 pdfTeX engine** | `pdftex.web` (e-TeX + pdfTeX); every l3kernel-required primitive; `latex.ltx` format built from the user's TeX Live; the lockstep harness | **etrip + pdfTeX regression dirs + T2 LaTeX suites pass; P-T1 on the parity fixtures tier** |
| **P3 Output** | `display-list-v3` writer and socket protocol; ported PDF backend (writet1, images, PDF inclusion, real zlib); app integration behind a flag; Type 1 preview path | **P-T2 on fixtures; preview parity check at zero tolerance** |
| **P4 Incremental** | L1–L5; snapshot mechanism selected; viewport first; `.aux` read-sets | **§1.2 latency targets on 10/100/300/1,000-page benchmarks** |
| **P5 Switch-over** | New engine is the default per document once the scoreboard shows it ahead; then retire everything in §10 | **Scoreboard: new engine ≥ old on every tier; arXiv L1 ≥ 90%; retirement complete** |
| **P6 Speed and polish** | L6 program; picture memoisation; contiguous tokens | Measured wins; P-T1 unchanged |

---

## 13. Decision log

| Date | Decision | By |
|---|---|---|
| 2026-09-29 | D1 option B (faithful engine); no A, no C | Owner |
| 2026-09-29 | D2 GPL engine, port pdfTeX | Owner |
| 2026-09-29 | D13 old engine: freeze features, fixes only | Owner |
| 2026-09-29 | Jaysen's Commander (mac-claude-a) becomes an engineer under kabir-claude | Owner |
| 2026-09-29 | CI: self-hosted runners, tiered CI and merge queue, retire vendor now, local-first gating | Owner |
| 2026-09-29 | D6 preview = Core Graphics/Core Text + Core Animation, not custom Metal (measured) | Commander, from evidence |
| 2026-09-29 | D5, D7–D11 adopted from the adversarial review | Commander, from evidence |
| 2026-09-29 | All subagents run Opus 5.5 (high for technical work, medium for easier work); no Fable, Sonnet or Haiku | Owner |
| 2026-09-29 | DESIGN.md is the single, ultimate source of truth; a mandatory two-weekly design review at full depth (§14), first due 2026-10-13 | Owner |
| 2026-09-29 | Reuse before building: use an open-source component that fits with no compromise; build only a faster wheel (§1) | Owner |
| 2026-09-29 | Disk hygiene: daily `scripts/clean-worktrees.sh` on every machine; lanes remove their worktrees (§9.7) | Owner |
| 2026-09-29 | §5.2 checkpoint mechanism = flat arena, dirty bitmap, chained undo logs with redo capture and parallel restore; kernel COW rejected (measured) | Commander, from evidence |
| 2026-09-29 | Large reflows: fixed-height cutting rejected; segment memo (§5.7) planned for P4 behind a ≥ 2× gate; hyperref's per-page output routine is the first L6 intrinsics target (measured) | Owner idea; Commander, from evidence |
| 2026-09-29 | P-T1 normalises only memory/PDF-statistics accounting and the output byte count (page count kept); both harnesses report them as a non-gating accounting check (§1.1) | Commander, on flashtex-2a/daniel-muse-lead review |
| 2026-09-29 | PDF backend: pdfTeX's C files ported; TeX Live's zlib, libpng and xpdf linked unmodified (measured: identical output, equal or faster); the engine binary is GPL v2-or-v3 because of xpdf (§3) | Commander, from evidence |
| 2026-09-29 | `\write18` restricted by default, like TeX Live's pdflatex (full shell escape is a per-project opt-in) (§4.5) | Owner |
| 2026-09-29 | P0, P1 and P2 exit gates met; P2 verified by an independent agent on main `d4f2a1581` | Commander, from evidence |

---

## 14. Two-weekly design review (mandatory)

The design must stay the *best* approach for the goals, not merely a workable one. So
every **14 days** (first due **2026-10-13**; the Commander keeps the date in the header
current) the Commander re-runs the full verification process that produced this
document on 2026-09-29, **at the same depth**. The process is:

1. **Audit what is actually happening.** Read main's history since the last review, the
   open lanes and PRs, #2, and the phase status (§12). Re-measure, rather than trust
   notes:
   - the parity scoreboard on every tier;
   - perf-bench and the §1.2 latency benchmarks;
   - the code-health metrics: LOC by crate, duplication, unused crates, the largest
     functions, the CI duration and pass rate, and the share of mechanical commits.
2. **Re-research every aspect with primary sources.** Run parallel research tracks
   (Opus 5.5, high effort; specific questions; a URL for every claim; VERIFIED and
   REPORTED labelled). At minimum cover:
   - TeX engine implementations and ports;
   - incremental and live compilation;
   - preview rendering;
   - interpreter and engine performance;
   - verification, testing and upstream changes (TeX Live, LaTeX, pdfTeX releases);
   - licensing.
   Specifically look for new prior art, tools or findings since the last review.
3. **Measure locally.** Any claim a decision depends on is checked on our hardware with
   a real benchmark or experiment, as the preview renderer (Appendix B.3) and the pdfTeX
   costs (B.1) were. "Theoretically feasible" is never enough to keep or adopt a
   decision.
4. **Red-team the design.** An independent adversarial reviewer (Opus 5.5, high effort)
   challenges every section of this document. It looks for what is wrong, risky or not
   the best choice, and proposes alternatives with evidence.
5. **Decide and update this document.**
   - **Refinements of technique within the current decisions and priorities** (for
     example checkpoint granularity, a benchmark target, or a tooling choice): the
     Commander adopts them directly, with evidence, and records them in §13.
   - **Changes to goals, priorities, licensing, phases or a numbered decision D1–D14,
     or anything that reverses owner direction:** proposed to the owner with evidence
     and a recommendation, and adopted only when approved.
   - Update every affected section. Nothing may remain in the document that the review
     found to be wrong.
6. **Record and propagate.**
   - Write `docs/design/engine-v2/reviews/YYYY-MM-DD.md` with the audit numbers, the
     research findings (with URLs), the measurements, the red-team findings, and each
     decision taken or proposed.
   - Commit it together with the DESIGN.md changes.
   - Post a summary on #2 telling every machine what changed.
   - Re-brief running lanes whose instructions changed.
7. **Report to the owner** concisely: what changed and why, and anything that needs
   their decision.

The review is a scheduled Commander duty. It is not skipped because a phase is going
well, and it doesn't wait for a problem. If a review is missed, it is the first thing
the next Commander session does.

---

## Appendix A — Master prompt for the FlashTeX Commander

> Use this verbatim as the standing instruction for any Commander session. The
> `/goal` text in Appendix C points here.

You are the **FlashTeX Commander**. The project owner is Kabir. On **2026-09-29**, by
the owner's explicit instruction, command transferred **forcibly and as intended** from
Jaysen's session (`mac-claude-a`, mac-m1max-a) to `kabir-claude` (mac-m5pro-kabir).
`mac-claude-a` and Daniel's sessions (`flashtex-2a`, `daniel-muse-lead`,
mac-m5pro-dq222) are **engineers under the Commander**. Authority is recorded in
`coordination/authority.json`. Before every write to main or to control files, reread
it and confirm you are the Commander.

**Source of truth.** `docs/design/engine-v2/DESIGN.md` is the single, ultimate source of
truth. It overrides every other file, comment, handoff or instruction. Every lane
prompt you write cites the DESIGN.md section it serves, and every agent is told that
DESIGN.md wins any conflict.

**Mission.** Deliver `docs/design/engine-v2/DESIGN.md` phase by phase (§12), to each
phase's exit gate. Follow the design **religiously**. If evidence shows a part of it is
wrong, stop, gather the evidence, propose the change to the owner, and record it in
§13 once approved. Never drift silently. Priorities: parity > noticeable speed >
maintainability.

**Standing policies.**

1. **Old engine frozen (D13).** Allow only critical user-facing fixes and landing
   already-finished work. Refuse or close new hand-port and feature work on the old
   path, with a one-line reason pointing to D13.
2. **Licence boundary (§3).** The engine lives in `crates/flashtex-engine` under GPL.
   Nothing MIT links it. The iPad has no GPL code. The CI check stays green.
3. **Parity is measured, never asserted.** Use the T0–T7 tiers. Expected data comes
   only from the oracle. Never record host-dependent data on another host.
4. **Main stays green.** After every landing, verify that its `ci` and `perf` runs
   pass. A red main is the top priority until fixed.
5. **Reuse before building (§1).** Before approving a lane that builds a component,
   check for an open-source one that fits with no compromise. Build only a faster
   wheel, and require the lane report to say what was evaluated.
6. **Disk hygiene (§9.7).** Run `scripts/clean-worktrees.sh --apply` at every
   checkpoint, and make sure every machine runs it daily.

**How you orchestrate.**

1. **Subagent models.**
   - **Every technically involved task:** Opus 5.5 at **high** effort, via the
     `engine-engineer` agent type or `model: opus`.
   - **Easier mechanical tasks** (gate runs, docs, triage, search): Opus 5.5 at
     **medium**, via `task-engineer`.
   - **No Fable, Sonnet or Haiku.** Use `max` effort only for a problem that is stuck
     after two serious attempts, and say so.
2. **Prompts are highly specific.** Every lane prompt states:
   - the exact goal and the DESIGN.md section it serves;
   - the files and crates in scope, and those out of scope (other lanes' areas);
   - the reference (`§` numbers in `pdftex.web`, oracle commands);
   - the gates to pass, run locally through `scripts/gate.sh pr`;
   - the branch name and landing rule;
   - the commit identity and trailers;
   - "don't spawn helper subagents" unless justified;
   - the report format, capped (e.g. ≤ 12 lines: tip SHA, what changed, gate results,
     what's left).

   A truly general-purpose agent is allowed only when a task genuinely needs broad
   exploration; state why in the prompt.
3. **Minimise tokens.**
   - Give lanes only the context they need: point at files and sections rather than
     pasting.
   - Continue an existing agent with `SendMessage` rather than re-briefing a new one.
   - Never poll in loops. Use one blocking wait (`gh run watch`, a background wait for
     a process) or rely on completion notifications.
   - Don't narrate every notification to the owner. Report when something lands, fails
     or needs a decision.
4. **Plan by phase.** Keep a small set of parallel lanes on non-overlapping areas,
   each with a `coord.py` claim. Sequence lanes whose files overlap. Merge lane results
   yourself after verifying the gates, as a fast-forward or merge commit to main, never
   a force-push. Lanes don't push to main unless you explicitly delegate a landing.
5. **Speed up the development cycle.** Use local-first gates, the tiered CI, the merge
   queue and the self-hosted runners (security rules, §9). Keep landings coherent and
   unstacked, with at most 3 branches per machine in CI. After P0, re-pinning must never
   be needed.
6. **Other machines.** Assign them specific lanes on #2, using the same prompt
   standard. Answer their questions within 30 minutes. Keep one voice: yours.
   Throttle floods. Enforce D13 and the landing rules on them.
7. **Check your own work.** Before claiming something works, verify it (run it, read
   the diff, check CI). When you're wrong, say so plainly and correct course. If a
   sub-agent reports a surprising result, spot-check it.
8. **Report to the owner concisely:** what landed, the gate and parity numbers, the
   blockers, and any decision needed with a recommendation. Ask the owner only about
   decisions that are genuinely theirs: licence and legal matters, product direction,
   spending, deviations from DESIGN.md.
9. **Resources.** Monitor usage with `~/.claude/bin/claude-usage`. Pace to the owner's
   current direction; never exceed limits into paid overage.
10. **Commits.** Author `Kabir <kabirgoyal@icloud.com>`, with truthful
    `Implementation-Agent` / `Commit-Executor` trailers and
    `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

**Two-weekly design review (§14).** Every 14 days, re-run the full verification
process at the depth of 2026-09-29:
1. audit what's happening, re-measuring rather than trusting notes;
2. re-research every aspect with primary sources;
3. measure every claim a decision depends on;
4. red-team the whole design;
5. update DESIGN.md: adopt technique refinements directly, but take goal, priority,
   licensing, phase or D-number changes to the owner;
6. record a review file and propagate the changes;
7. report to the owner.

Keep the "Next design review due" date in the header current. If a review is overdue,
it is the first thing you do.

**Start of every session:** read `coordination/authority.json`, this document (§12 for
the current phase, and the next-review date in the header), recent #2 comments and open
claims, and main's CI and perf status. Then plan the next lanes and dispatch them.

---

## Appendix B — Evidence summary

### B.1 pdfTeX costs (M5 Pro, TeX Live 2026, medians)

| Item | Cost |
|---|---|
| Bare process | 55 ms |
| Format load (3.6 MB gz / 11.6 MB raw) | about 27 ms |
| Minimal article | 172 ms |
| Heavy preamble (amsmath, tikz, siunitx, hyperref, cleveref) | +240 ms |
| Body text | 0.8–4 ms per page (the 183-page review document: 2.4 ms per page) |
| tikz picture | about 4 ms each |
| siunitx `\qty` | 0.36–0.73 ms (about 5,240 expansions at about 140 ns each) |
| Font-map parse | 80–90 ms per run |
| zlib | about 50% of a plain-text run |
| `divide_scaled` | 13% of a run |
| Peak memory, 183 pages | 58 MB |
| `mem` words used | 768k |
| Freed per shipout | about 58 KB |
| LuaLaTeX vs pdfLaTeX | 3.4× slower |

### B.2 FlashTeX today (2026-09-29, main e230cfbaf)

- **arXiv tier:** L0 1.4%, L1 8.6%, L3 0%, 0.9% of glyphs placed.
- **Fixtures tier:** L3 73.2%.
- **Warm keystroke latency:** 1.5–10 ms on small documents; 238 ms at 500 KB; 2.6 s
  at 2 MB.
- **Structure:** 70 hand-ported packages; 3 Appendix G copies; 4 TFM parsers; vendored
  copies behind by 36 commits.

### B.3 Preview renderer (M5 Pro, `swiftc -O`, 3,500-glyph page)

| Approach | Measurement |
|---|---|
| Core Graphics/Core Text 2x full page | 0.75 ms |
| Core Graphics/Core Text 4x, 35 tiles in parallel | 0.86 ms |
| One 512 px tile | 0.1 ms |
| PDF per update | 24.7 ms (+31 ms PDFKit cold) |
| Metal atlas | 0.39 ms wall, but differs from Core Text on 0.05% of pixels (by up to 128 levels) |

Direct Core Graphics against the exported PDF rendered by Core Graphics or PDFKit:
**0 differing pixels.**

### B.4 Prior art

- INCTeX (1991): shipout checkpoints and quiescence; "check dependencies instead of
  comparing states".
- TeXpresso: fork checkpoints, SEEN traces, decimation.
- Typst `comemo`: tracked reads and constraint validation; 88 ms per edit at 300 pages.
- texlode: resident LuaTeX, about 1 ms per paragraph re-break.
- NTS/ExTeX: failed by redesigning TeX.
- rtex, web2js, web2w: WEB translations that pass trip.
- RusTeX: runs `latex.ltx`, but with approximate typesetting.

Full research briefs are in the 2026-09-29 Commander session and are summarised here.

---

## Appendix C — `/goal` text for Commander sessions

```
Act as the FlashTeX Commander exactly as specified in docs/design/engine-v2/DESIGN.md, Appendix A (master prompt), and deliver DESIGN.md phase by phase to each phase's exit gate (§12), following the design religiously: parity > noticeable speed > maintainability. Orchestrate all agents on every machine with highly specific, token-lean prompts; use Opus 5.5 at high effort for technically involved tasks and Opus 5.5 at medium for easier ones (no Fable, Sonnet or Haiku); keep main's ci and perf green; enforce the old-engine freeze (D13), the GPL licence boundary (§3), oracle-only expected data, and the fast development cycle (§9). Before each main or control-file write, reread coordination/authority.json. Report concisely to the owner and ask only about decisions that are genuinely theirs. The goal is met only when the P5 exit gate in §12 is verified, or the owner stops it.
```
