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
  v3.1 still carries TeX/pdfTeX assumptions; the capability-gated v3.2 extension makes it
  carry Typst output (§15.4). No Apache-only or GPL code may enter its MIT crate (§15.9).
- **Typst host (`flashtex-typst-host`, MIT, linked to Apache-2.0 crates):** a separate
  process per Typst document; never linked to the engine (§15.2).
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
   *Refinement (measured 2026-09-29, PR #1230):* for macros that don't exist when the
   format is built (e.g. hyperref's `\pdfstringdefPreHook`, filled in by siunitx), the
   dependency set is recorded on the macro's first run and guarded the same way. The
   first intrinsic cut `\pdfstringdef` from ~1.1 ms to 45 µs per call. Result: full-1000
   documents went from 6.7 to 4.1 ms/page, faster than pdflatex (4.1 s vs 6.3 s). The
   both-paths diff showed 0 differences over 53,401 calls.
5. NEON input scanning only if the profile shows scanning matters.
7. **Pipeline and export parallelism (identical output only).** TeX's typesetting is
   inherently sequential and stays single-threaded (speculative parallel typesetting
   (L7) was dropped: when state converges, the old pages are reused anyway; when it
   doesn't, the speculative work is wrong). Measured candidates:
   - hand each shipped page to a second thread that encodes the display list and the PDF
     page while the engine continues;
   - compress page streams and embed fonts in parallel at export. zlib is
     deterministic, so the PDF stays byte-identical.
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
  Engine-neutral for Typst only with the v3.2 extension (§15.4).
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
| Typst 0.x churn: every minor since 0.6 broke embedders; releases every 4–8 months; 5–13% of older templates fail on 0.15.1 (§15) | Exact pin; all typst code in one crate; one deliberate upgrade lane re-running the T1 gates; current + previous host shipped; per-project pin and upgrade assistant |
| Typst seeded compile re-implements a private function (`compile_impl`) over unstable public APIs | Re-verified against the standard compile every release (≥ 192 edits) and when idle; export always uses the standard compile |
| Typst memory: 1 GB per 300-page and 3–4 GB per 1,000-page document; +70 MB per keystroke without eviction | Mandatory `comemo::evict` after pages are sent; one process per document; RSS ceiling with watchdog restart |
| Typst package supply chain: TLS-only downloads, no lockfile, 7.3% GPL-family packages, unlimited WASM plugins | FlashTeX package lock (SHA-256), offline mode, first-use consent, never bundle Universe wholesale, path confinement (typst#5454), watchdog |

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
| 2026-09-29 | L6: intrinsic dependency sets may be recorded at first run (guarded); pipeline and export parallelism added as measured L6 items; core typesetting stays single-threaded | Commander, from evidence |
| 2026-09-30 | Add Typst support as a second engine in its own process (§15); lowest priority, must not impede LaTeX | Owner |
| 2026-09-30 | TY1: the §15 sketch corrected — v3 is not engine-neutral today, so an additive capability-gated v3.2 plus PDF islands comes first; separate processes are chosen for isolation and the MIT app's options (Apache-2.0 *is* GPLv3-compatible); the real licence risk is the shared MIT `display-list-v3` crate (§15.1, §15.4) | Commander, from evidence (Tracks A, C) |
| 2026-09-30 | TY2: `flashtex-typst-host` (MIT), one process per Typst document, own workspace and lock, `typst` pinned exactly and unmodified, per-project version pin (current + previous shipped, upgrade assistant), watchdog, mandatory `comemo::evict`, per-project fonts, package lock + offline + consent, symlink-escape guard, fonts as files (§15.2) | Commander, from evidence (Tracks A, C) |
| 2026-09-30 | TY3: Typst latency as measured (seeded 1-pass loop, verified 192/192, re-verified every release and checked when idle); §1.2 met to about 100 pages. **Target for larger Typst documents and reopen: pending owner** (relaxed target, or upstream contribution; never a private fork) (§15.3) | Commander, from evidence (Track A); **pending owner** |
| 2026-09-30 | TY4: Typst preview = Core Graphics/Core Text from the display list with PDF-derived f64 positions; gate pixel-identical at 2×/3× (1× documented floor); typst-render rejected (§15.5) | Commander, from evidence (Track A) |
| 2026-09-30 | TY5: editing UX in two tiers (in-app syntax; host-served `lang-v1`), `typst-syntax` as the app's first linked Rust (MIT/Apache only), typst-ide in the Typst host, tinymist-query optional and pinned, tinymist-LSP fallback only, language-provider refactor, UX rules (§15.6) | Commander, from evidence (Track B) |
| 2026-09-30 | TY6: LaTeX gains in priority order; structured LaTeX diagnostics are a P5 prerequisite (§15.7) | Commander, from evidence (Track B) |
| 2026-09-30 | TY7: Typst legal obligations list; "Typst support" wording; owner to contact Typst GmbH before public release (§15.8) | Commander, from evidence (Track C); owner action |
| 2026-09-30 | TY8: guard-rails — separate workspace, path-filtered CI outside LaTeX required checks, additive capability-gated protocol with LaTeX parity fixtures on every shared-crate change and CODEOWNERS, extended licence check, no Typst app work before P3-APP-V3, flag, ≤ 1 Typst lane, merge-queue priority below LaTeX (§15.9) | Commander, from evidence (Track C) |
| 2026-09-30 | TY9: Typst phases T0–T3 with measurable exit gates (§15.10) | Commander, from evidence (Tracks A–C) |

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

## 15. Typst support (owner, 2026-09-30; analysed design 2026-09-30)

FlashTeX will also compile Typst documents, as a second engine behind the same preview
path. It is the **lowest priority** and must not slow the LaTeX work. This section replaces
the first sketch of 2026-09-30 (PR #1243). It rests on three analysis tracks; evidence
index: `docs/evidence/typst-design-2026-09-30/README.md` (abbreviated `TE/` below):

- **Track A**, engine integration, measured in-process on Typst **0.15.1** (`=0.15.1`,
  tag `9dfd3a08`): `TE/track-a-engine.md`, raw rows in `TE/raw-a/`, the prototype and
  pixel-diff harness in `TE/prototype/` (MIT, evidence only, not linked into anything).
- **Track B**, editing parity in both directions: `TE/track-b-ux.md`.
- **Track C**, red-team, licensing and alternatives: `TE/track-c-redteam.md`.

All measurements are M5 Pro, 15 cores, 24 GB, at a 1-minute load average of 8–60 from
other agents' benchmarks, so absolute times may be 5–30% high; ratios were taken back to
back (Track A §8.8).

**Numbering.** Typst decisions are **TY1–TY9**. Typst phases are **T0–T3** (§15.10);
they are not the §8 verification tiers of the same names.

### 15.1 Corrections to the first sketch (TY1)

| Sketch said | Established | Evidence |
|---|---|---|
| "`display-list-v3` stays free of TeX-only assumptions" | **False today.** v3.1 carries `tex_name`, TFM sizes, Type 1 `format` with 8-bit `code` → 256-name `encoding` (OpenType, TrueType and Type 3 *reserved and refused*), pdfTeX `/Fm<n>` ids, DeviceGray/RGB/CMYK only, `gs`/`sh`/patterns INCOMPLETE, sp-only coordinates, `counts[10]` = TeX's `\count0–9`. **Every** page of a plain Typst document uses ICCBased colour and Type0/CIDFontType0 (CFF) fonts, so every page would arrive INCOMPLETE. Typst also needs OTF glyph ids + variation coordinates, f64 positions, ICC/alpha/spot colour, stroked text, image bytes (GIF, WebP, `image(bytes)`), page labels and bleed. | Track C §2.1 (PDF scan of `p10.typ`); Track A §3.2 table + census `TE/raw-a/census.json` (2,203 test-suite snippets, 2,623 pages: 738 gradients, 37 tilings, 164 alpha paints, 175 variable-font runs, 64 colour-glyph runs, 31 stroked-text runs) |
| "Typst emits `display-list-v3`" | It emits **v3.2**: an **additive, capability-gated** extension that comes first (§15.4), plus **PDF islands** — small typst-pdf exports carried through v3's *existing* `IMAGE` type `pdf` — for gradients, tilings, SVG images and colour glyphs. | Track A §3.3 (E1–E8); Track C §2.1 |
| "Apache-2.0 is incompatible with GPL-2.0, so Typst never links into the engine" | **Wrong reason.** Apache-2.0 **is** compatible with GPLv3 (FSF licence list; ASF), and the engine binary is distributable under GPL v2 or v3 because of xpdf (§3). Linking Typst in would be possible but would make the engine GPLv3-only. Separate processes are chosen for **crash, hang and memory isolation** and to **keep the MIT app's options** (and the engine's GPLv2 option). **The real licence risk is the shared MIT crate `display-list-v3` (`flashtex-display-list`)**, linked by both hosts: Apache-only code in it makes the engine GPLv3-only; GPL code in it (e.g. moving `flashtex-engine/src/displaylist/interp.rs` there) makes it and the Typst host GPL. No Apache-only or GPL code may flow into it; the licence-boundary check is extended (§15.9). | Track C §1.1 (gnu.org/licenses/license-list#apache2; apache.org/licenses/GPL-compatibility.html; `scripts/check-license-boundary.sh` checks neither provenance nor the shared crate's dependency licences) |
| "Edit latency meets §1.2" | Met only up to about 100 pages (§15.3). | Track A §2 |
| "Upstream crates unmodified" (one version) | Still unmodified, but **pinned per project**, with current + previous shipped (§15.2). | Track C §2.4 |

### 15.2 Architecture (TY2)

```
 Mac app (MIT, Swift) ── display-list-v3.2 + lang-v1 over Unix socket ──┐
   ├── flashtex-host        (GPL-2+, one per open .tex document)          │
   └── flashtex-typst-host  (MIT; typst crates Apache-2.0; one per open .typ document)
```

- **`flashtex-typst-host`** is our own **MIT** code, linked only to the Apache-2.0 `typst`
  crates and the MIT `flashtex-display-list`. It never links, calls or shares files with
  `flashtex-engine`. It speaks the same socket protocol as `flashtex-host` and mirrors its
  `COMPILE` semantics (Track A §7).
- **One process per open Typst document.** comemo's cache is process-global; killing the
  process frees 1–4 GB at once, where `comemo::evict(0)` took 6.1 s and returned only part
  (`TE/raw-a/mem.jsonl`). Idle host: 12.7 MB; binary 46 MB stripped (Track A §7).
- **Own Cargo workspace and `Cargo.lock`** in a top-level directory (e.g. `typst-host/`),
  **never** in the root `crates/*` members: otherwise Typst's ~425-crate tree enters the
  shared lock, every `cargo test --workspace`, the release profile perf baselines depend on,
  and Typst's MSRV (1.92 at 0.15.1) drags the engine's toolchain (Track C §4.1). It depends
  on `flashtex-display-list` by path only.
- **Upstream `typst` crates pinned exactly (`=0.15.1`) and unmodified.** Every 0.x minor
  since 0.6 has broken embedders (Track A §1.4; releases every 4–8 months). All
  typst-touching code sits in one crate so an upgrade is one diff. Any patch needs an
  Apache-2.0 §4(b) modification notice, so the rule is: no `[patch]`, no fork.
- **Per-project Typst version pin** in `flashtex.toml`; FlashTeX ships **the current and the
  previous minor** as separate host binaries, selected by the pin, plus an **upgrade
  assistant** (compile with both, diff diagnostics and page hashes), as the Typst web app
  does. Measured on 0.15.1: 12/94 (13%) Universe templates from the ≤ 0.12 era and 4/80
  (5%) from 0.13–0.14 fail to compile (Track C §2.4).
- **Watchdog.** No Typst compile can be cancelled, WASM plugins run in wasmi with **no fuel
  or memory limit** (up to 4 GiB each), and `for` over a huge range is unbounded (Track A
  §6; Track C §2.7). The app kills and restarts a host that exceeds a wall-time budget
  (starting point 10 s) or an RSS ceiling, marks its pages stale and cold-compiles.
- **Memory.** `comemo::evict(10)` is **mandatory after the pages are sent**, off the
  critical path (p50 9.5–10.3 ms). Without it: +70 MB per keystroke, 21.7 GB after 300
  keystrokes at 300 pages; with it, flat at about 1 GB (`TE/raw-a/mem.jsonl`). Budget about
  1 GB per open 300-page document and 3–4 GB at 1,000 pages.
- **Fonts: per-project font list** (family → file SHA-256) recorded in the project;
  missing or changed fonts are flagged prominently, because an unknown family is only a
  warning and silently reflows (Track C §2.5). **Fonts ship as separate files**: the host is
  built **without** `typst-kit/embedded-fonts`, so GPL-3 NewCM10-Regular is never compiled
  into a binary; the four default families live in the bundle's `Resources/` (Track C
  §1.2). System fonts are scanned lazily (786 faces in 177 ms; Track A §5.1).
- **Packages: a FlashTeX package lock** (`package@version` → tarball SHA-256) written on
  first fetch and verified on every later fetch — typst-kit verifies nothing but TLS and
  the index has no hash field (Track A §6; Track C §2.6). **Offline mode**; a **first-use
  network consent** before contacting packages.typst.org (disclosed in the privacy text);
  fetches run in the background and never block a keystroke; a "vendor packages into the
  project" action; a FlashTeX User-Agent and no bulk prefetch.
- **File access.** Our `World` canonicalises every path and refuses to leave the project
  root or package directory, which closes typst#5454 (symlink escape, open upstream) for
  us, as §4.5 does for LaTeX.
- **Rejected:** Typst linked into the app (multi-GB caches, rayon pools and hangs in the UI
  process); Typst linked into the GPL engine (above); tinymist as the compiler
  (a second compile of every document; §15.6).

### 15.3 Performance (TY3, measured)

Typst compiles the **whole document** per edit: `typst::compile` returns one finished
`PagedDocument`, with no page streaming, no viewport-first and no cancellation (Track A
§2.5). The output delta is still small (1–2 changed pages per keystroke, at most 6).
Keystroke = `Source::edit` + compile, non-repeating edits, 40 keystrokes per location,
evict(10) after each (`TE/raw-a/bench-typing-evict10.jsonl`, `seeded.jsonl`):

| pages | standard `typst::compile` p95 | seeded 1-pass loop p95 |
|---|---|---|
| 10 | 4.8–5.2 ms | 2.0–2.6 ms |
| 100 | 53–60 ms | 16.4–17.4 ms |
| 300 | 205–227 ms | 60–64 ms |
| 1,000 | about 1.0–1.4 s (load 25–60) | 353–387 ms (load 11–13) |

- **The seeded loop** runs `typst::compile`'s fixed-point loop from public crates
  (`typst_eval::eval`, `Engine`, `Output::create`, `comemo::Constraint`) with the previous
  keystroke's introspector as the first one: 1 layout iteration instead of 4. **Verified
  192/192 edits page-hash-identical to the standard compile** (d10/d100/d300, 4 locations,
  `TE/raw-a/seeded-validate.jsonl`) and about ⅓ of the memory. It re-implements a private
  function (`compile_impl`), so it **must be re-verified on every Typst release**, and the
  host **re-checks it against the standard compile when idle** and always exports with the
  standard compile.
- The edited page reaches the socket about 1 ms after the compile (display-list encoding
  30 µs per page, single-page PDF export 0.44–0.53 ms), viewport page first. Keystrokes
  coalesce (newest state wins), and the previous pages stay shown, not marked stale.
- Cost grows with introspection density, not position: 300-page ablation p50 is 33.5 ms
  bare → 48 plain → 101 no-citations → 190 ms full (`TE/raw-a/bench-ablation.jsonl`,
  indicative, load 24–40). Track C's lighter document (no bibliography) measured 28.7 ms
  median / 79.4 ms p95 at 301 pages through `typst watch` with a one-page PNG (Track C
  §2.3). Chapter `#pagebreak()`s help: c300 seeded 46 vs 60 ms, cold 486 vs 1,575 ms.
- **Reopen:** comemo is in-memory only, so reopening is a cold compile (1.6 s at 300 pages,
  12 s at 1,000 under load; Track A §2.4). The app shows the previous session's page
  rasters, keyed by the v3 content hash, marked stale until the first compile lands.

**Verdict.** §1.2's **≤ 16 ms p95 edited-page target is met for Typst up to about 100
pages** (seeded 16.4–17.4 ms p95 at 100 pages — at the limit). Beyond that, and for the
≤ 100 ms reopen, the Typst target is **pending an owner decision** (§15.11): (a) accept a
relaxed, documented target for large Typst documents; or (b) contribute page-level
incremental layout (or a page-streaming callback) **upstream to Typst**, with the numbers
above as motivation — **never a private fork** (the reuse rule, §1). No truncated-document
provisional compiles: they break parity (Track A §2.5 item 6).

### 15.4 Protocol: `display-list-v3.2` (part of TY1)

The protocol owner specifies and lands this **before any host code** (T0). All of it is
additive: new JSON keys, section tags and message kinds are minor changes; new item opcodes
are sent only to a client whose `HELLO` says `[3, 2]` and lists the capability. A 3.1
client receives `INCOMPLETE` pages and falls back to `DONE.pdf`, as today. The owner of
`docs/protocol/display-list-v3.md` rules whether a negotiated opcode is a minor change.
The LaTeX host is never required to emit any of it.

| # | Extension | Carries |
|---|---|---|
| E1 | `FONT.format: "opentype"` (value already reserved) | glyph ids; `face_index`, `variations`, `units_per_em`, file + `program_sha256` (an empty program allowed when the file is readable and matches: CJK collections are 20+ MB) |
| E2 | section `ORIGINS_F64` | per-glyph f64 bp origins as the PDF viewer computes them (§15.5) |
| E3 | section `COLORSPACES` + `FILL/STROKE_COLOR_CS`, `ALPHA` | ICC profile bytes, Separation (spot), constant alpha; components as the PDF's u8-quantised values |
| E4 | `LINE_STATE` | stroked text (or via E5) |
| E5 | **PDF islands** — no protocol change | constructs v3 can't express are exported by typst-pdf as a one-page frame of their bounding box (`page_ranges`, `tagged: false`) and sent as `IMAGE` type `pdf`: gradients (incl. conic), tilings, SVG images, colour glyphs, gradient-filled text. Parity by construction is **belief** until its gate row passes |
| E6 | `IMAGE` from bytes (`IMAGE_DATA`), types `gif`/`webp`, `interpolate`, `icc` | the pixels the PDF has, not the source file |
| E7 | section `PAGE_META` | page label, logical number, bleed, trim box, `engine`; `counts` stays TeX-only |
| E8 | `RESOLVE`/`LOCATE` messages | on-demand span → source and source → page; eager re-declaration of all spans costs 507 ms at 300 pages (`TE/raw-a/jumps-d300.jsonl`), click → source 4–37 µs, forward search 2.9–3.2 ms |

Most of E3/E5 also serves LaTeX (TikZ opacity and shadings, xcolor transparency; Track C
§2.1). Hashes: v3 §4.6 plus E2/E3/E7 sections; host-side change detection uses Typst's
per-page `hash128`.

### 15.5 Rendering and parity (TY4)

- The preview is the existing **Core Graphics/Core Text** path (D6, §6.2) drawing from the
  display list; the app loads the OpenType fonts from the same bytes Typst used (no Type 1
  conversion; typst-pdf's CFF subsets keep the original hints, Track A §5.2).
- **Gate:** pixel-identical to typst-pdf's exported PDF **as drawn by the platform's
  reference rasteriser** (the §16 wording, PR #1245; §6.2 zero tolerance) at **2× and 3×**.
  **1× is a documented floor**, not a gate (1,202 differing pixels on d10 page 2, Track A
  §5.2).
- **f64 glyph positions are required**, re-derived from typst-pdf's content stream for each
  changed page (single-page export 0.44–0.53 ms, positions identical to the full export),
  mirroring v3 §4.2. Measured (`TE/raw-a/pixel-parity.jsonl`, 7,039 glyphs on 2 pages,
  subpixel quantisation off): PDF-derived f64 origins **0 differing pixels at 2×** on both
  pages and at 3× on page 2; Typst's own frame positions (within 5.8 × 10⁻⁵ bp) give
  31–5,324; positions rounded to sp give 118–581. The 408 pixels (≤ 1 level) at 3× on
  d300 page 150 appear with every origin source and are believed to be rules drawn from the
  frame; T1 must clear them.
- **Rejected: typst-render** (tiny-skia) for the preview: about 23 ms per page at 2× versus
  0.75 ms for Core Graphics (B.3), its own glyph rasteriser fails the zero-pixel check, no
  tiling (Track C §2.8). Debug aid only. **Rejected:** a whole-PDF export per edit shown by
  PDFKit (77–90 ms at 301 pages; Track C §5).
- Untested and each needing its own gate row before parity is claimed: non-black ICC
  colour, alpha, gradients, raster/SVG/PDF images, colour glyphs, variable-font instances.

### 15.6 Editing UX (TY5, Track B)

- **Two tiers.**
  - **Per-keystroke, in-app, synchronous (< 1 ms):** highlighting, bracket matching,
    auto-close, Return rules, comment toggle, prose ranges, folds, lexical outline, include
    scan. LaTeX keeps its tested Swift (`FlashTeXEditorCore`). Typst gets a small Rust
    library over **`typst-syntax`** (incremental `Source::edit`, 22 highlight tags) with a
    C ABI — **the first Rust linked into the app; MIT/Apache dependencies only**, covered by
    the licence check, built for macOS and iOS.
  - **Slow, async, revision-bound:** completion, hover, definition, references, rename,
    signature, formatting, code actions. Served by **each engine host** over a new
    **`lang-v1`** message family on the same socket (LSP-shaped JSON payloads, UTF-8 byte
    offsets, our revision binding; `HELLO.capabilities` lists the kinds, and the app hides
    what a provider lacks; replies for an older revision are refused). `COMPILE` semantics
    are untouched.
- **Typst host:** `typst-ide` first (it shares the host's `World`, comemo cache and last
  document, which label completions need); `typstyle-core` for formatting;
  **`tinymist-query` optional, pinned exactly** and moved only with `typst` (its API is
  explicitly unstable) for references, rename, signature help, inlay hints and code
  actions. **tinymist as an LSP subprocess only as a fallback**: it would compile every
  document a second time (double CPU and memory) at a possibly different revision.
- **App:** the **language-provider refactor** (`SyntaxProvider` / `SemanticProvider`) moves
  today's LaTeX code behind the protocols with **no behaviour change**, gated by the hosted
  tests and TypingBench (Track B §4.3). 36 `FlashTeXMac` files call LaTeX scanners directly
  today.
- **UX rules.**
  - **Typst** errors yield no document: the preview **keeps the last good render** with an
    error chip ("Last good render · rev 41 · 2 errors"), pages crisp.
  - **LaTeX** shows the pages the run shipped immediately, others marked stale
    (`PAGES.stale`); last good render only after `failed` or zero pages.
  - Problems rows gain `file:line:col`, hints and an expandable trace for both languages.
  - **Package consent sheet** (the existing `ProjectPackages` sheet) on the first
    `@preview` fetch; offline shows a located diagnostic with Retry.
  - **New Project language picker** (LaTeX | Typst) over parallel template lists.
  - **Settings › Languages** with one page per language (Return rules, formatter,
    compile options, package network policy).
  - Shared `.bib` files format differently under hayagriva/CSL and BibTeX/biblatex; the UI
    says so (Track C §3).

### 15.7 Vice versa: LaTeX gains (TY6, Track B §3)

In priority order. Items touching `flashtex-engine` are LaTeX engine lanes, gated by P-T1,
and are side channels only: terminal and log output never change.

1. **Structured LaTeX diagnostics** — column (input stack `loc` at `print_err`), macro
   trace (token-list levels of the input stack), help text (`help_line`s) and parsed warning
   ranges. **A P5 prerequisite:** the v3 `DIAGNOSTIC` is `{severity, message, file?, line?}`,
   so the new engine's diagnostics are less precise than runtime-v1's byte spans today.
2. **Engine-truth completion** — commands and environments actually defined at the caret,
   with definition sites (a `cs → (file, line)` side table excluded from the D8 hash and
   restored with checkpoints), **replacing `supported-latex.json`**; Go to Definition into
   any `.sty`/`.cls`.
3. **Real `\ref`/`\cite` numbers on hover** (and inlay hints) from the engine's `\r@`/`\b@`
   meanings in an `INDEX` message.
4. **Rename across the include graph** the engine actually opened.
5. **Numbered outline** with pages, from the shipped `toc` records merged with the instant
   lexical outline.
6. **Verified formatting**: `tex-fmt` (MIT), **accepted only if the box dumps are
   unchanged** (P-T1 machinery).

Catcode-exact semantic highlighting is deferred until after L6 (hot-path cost).

### 15.8 Legal obligations (TY7, Track C §1)

- **Apache-2.0 §4:** ship the licence text (a); mark any modified file (b) — hence no
  patching; retain notices in distributed source (c); reproduce upstream `NOTICE`
  attributions in a NOTICE file or About › Acknowledgements (d).
- **NOTICE files:** `typst/typst` (398 lines; includes **LPPL-1.3** Babel/cleveref
  translations, BSD-3 Skia, Apache-LLVM `powi`), `typst/typst-assets` (1,552 lines; OFL
  Libertinus, GUST NewCM, **GPL-3 + font exception NewCM10-Regular**, Bitstream Vera, CC0
  ICC), `hayagriva` (**CC BY-SA 3.0** CSL styles and locales, compiled in).
- **~425 dependencies'** licence texts (Typst 0.15.1 `Cargo.lock`), generated at DMG build
  (`cargo about` or equivalent); the build **fails on an unknown licence or missing
  notice**.
- **Font licences** shipped with the font files (OFL, GUST, GPL-3 + exceptions, Bitstream
  Vera); GPL fonts never compiled into a binary (§15.2).
- **Never bundle Universe packages wholesale**: of 1,635 packages, **119 (7.3%) are
  GPL-family, 20 AGPL** (`packages.typst.org/preview/index.json`, 2026-09-29). Pre-seeded
  caches, starter templates or mirrors make us the distributor. Bundle only named, reviewed
  templates (MIT-0/0BSD preferred).
- **Trademark:** product text says **"Typst support"** (nominative); never "FlashTeX
  Typst" or the Typst "t" as an icon. Typst's brand policy requires authorization for
  commercial use of the name: **the owner contacts Typst GmbH before public release.**
- The §3 legal review covers the Typst host too.

### 15.9 Guard-rails for the LaTeX roadmap (TY8, Track C §4)

- **Build and CI:** the separate workspace (§15.2); **path-filtered** Typst CI jobs
  (`typst-host/**`), **never in the LaTeX required checks or merge queue**, counted against
  §9.5's per-machine limit, first to be cancelled when runners are busy; build outputs
  covered by `clean-worktrees.sh`.
- **Shared protocol and decoder** (`docs/protocol/display-list-v3.md`,
  `crates/display-list-v3`, the Swift renderer): changes additive and capability-gated;
  **the LaTeX parity fixtures and the positions check run on every change to a shared
  crate**; a **CODEOWNERS** entry names the LaTeX display-list owner; spec changes batched
  per §14 review.
- **Licence-boundary check (§9.6) extended:** `flashtex-display-list` and every dependency
  of `flashtex-engine` must be MIT/BSD/ISC/Zlib-compatible (a per-crate `cargo-deny`
  allowlist; no Apache-2.0-only crate reaches the engine); a provenance check that no file
  from `crates/flashtex-engine` is copied into the shared crate or the Typst host; the Typst
  workspace may not depend on `flashtex-engine`; the app's Rust syntax library is
  MIT/Apache-only.
- **App:** **no Typst app work until P3-APP-V3** (the LaTeX v3 client, §12 P3) lands; Typst
  reuses that client. Typst UI stays **behind a feature flag** until T2's gate. Typst lanes
  change `V2PreparedPage`/`GlyphRunRenderer` only through a reviewed protocol extension.
- **People:** **at most one Typst lane at a time**, staffed only when LaTeX lanes are
  fully staffed; reports batched into normal checkpoints. **Merge-queue priority below
  LaTeX**, enforced by queue configuration.
- Typst lanes never touch `flashtex-engine`, `tools/web2rust` or the LaTeX harnesses.

### 15.10 Phases and exit gates (TY9)

| Phase | Scope | Exit gate (measurable) |
|---|---|---|
| **T0 Spec + spike** | v3.2 spec (E1–E8) reviewed by the protocol owner; in-process spike (from `TE/prototype/`) settling parity and latency on a fixed corpus (d10/d100/d300/d1000, c300) | Spec merged with LaTeX parity fixtures unchanged; spike reports p95 per size (seeded and standard), memory with eviction, 0 differing pixels at 2×/3× on ≥ 2 text pages with PDF-derived f64 origins (408-pixel case explained), and a gate row per construct class (colour, alpha, gradient island, images, colour glyphs, variable fonts) measured or marked open |
| **T1 Host** | `flashtex-typst-host` in its own workspace: World (confinement, lock, offline, fonts), seeded loop + idle check, per-page PDF-derived positions, v3.2 writer, evict, watchdog; CI path-filtered | **Positions checker vs typst-pdf**: 0 mismatches on the corpus and on every Typst test-suite snippet that compiles; seeded == standard page hashes on ≥ 192 edits; **latency** within the §15.3 table (≤ 16 ms p95 to 100 pages; larger per the owner decision); **memory** flat (≤ 1.1 GB at 300 pages over 1,000 keystrokes); **watchdog** kills and recovers a hanging plugin and a runaway `for` within budget; licence/NOTICE check green |
| **T2 App integration** | Typst documents open in the app behind a flag via the P3-APP-V3 client; version pin + current/previous hosts; package consent; last-good chip; cached rasters on reopen | App preview pixel-identical to typst-pdf at 2×/3× on the corpus; LaTeX TypingBench and preview gates unchanged; upgrade assistant diffs a 0.14→0.15 project |
| **T3 Editing parity** | Language-provider refactor; `typst-syntax` library; `lang-v1` with typst-ide (+ optional tinymist-query); Settings › Languages; New Project picker; the §15.7 LaTeX items (item 1 before P5) | Every §15.6 feature available for both languages or explicitly hidden by capability; syntax tier < 1 ms per keystroke at 200 KB; semantic replies revision-bound; TypingBench no regression; LaTeX diagnostics carry a column on the fixtures tier |

T0 may start only when LaTeX lanes are fully staffed (§15.9). No phase blocks any LaTeX
phase.

### 15.11 Open owner decisions

1. **Typst latency beyond about 100 pages** (and reopen): relaxed documented target, or
   upstream page-level incremental layout (§15.3). Pending owner.
2. **Contact Typst GmbH** about the name before public release (§15.8). Owner action.

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
- Typst `comemo`: tracked reads and constraint validation; measured in-process on
  0.15.1 at 205–227 ms p95 per edit at 300 pages (60–64 ms seeded; §15.3).
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
