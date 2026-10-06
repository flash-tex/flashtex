# Beamer decks on the new engine (lane BEAMER-V3, 2026-10-03)

Lane **BEAMER-V3** (mac-claude-a, mac-m1max-a). Standing owner goal: full beamer (slide deck)
support, now end to end on the new engine (`flashtex-initex`, `flashtex-host`, the v3 preview).
DESIGN.md §1.1 (P-T1, P-T2), §5.3 (restart and converge), §6.2 (preview), §8 (T3, T7).

**Engine under test:** main `3a4ed639c` (parity), built by `scripts/engine-parity.sh build`
(release, `pdflatex.fmt` made by the engine from TeX Live's `pdflatex.ini`); the incremental runs
add this lane's two convergence fixes (#1446, #1448). **Oracle:** TeX Live 2026 pdfTeX 1.40.29
(`/Library/TeX/texbin/pdftex`), MacTeX on mac-m1max-a, oracle only.

**Machine:** M1 Max shared with other lanes; **load 15–77 during every timing below**, so every
latency here is an upper bound and **not a reference figure** (T7's reference conditions are not
met). Convergence and pages re-typeset do not depend on load.

## Headline

| check | result |
|---|---|
| Corpus | 10 committed beamer fixtures + 24 more decks (11 TeX Live examples, 13 written here); 1 TeX Live example excluded (pdflatex cannot compile it from the installation) |
| P-T1 | **34/34 pass** (fixtures 10/10, `beamer` tier 24/24) |
| P-T2 | **34/34 pass** |
| L0–L3 | 100 % (every glyph within 0 bp) |
| Typesetting divergences | **none**: no engine change was needed for parity |
| Incremental, before | an edit before most of a deck **re-typeset it to its end** (0/19 letter edits converged on page 7 of 118; 112 pages per keystroke, DONE 8.6 s p50) |
| Incremental, after #1446 + #1448 | **every letter and sentence edit converges** (19/19 per row): 2 pages re-typeset after an edit in a one-slide frame, 9 in a three-slide frame |
| Edited slide (host share, loaded) | 53–61 ms p50 for the first slide of the edited frame; 133 ms for the third slide of a three-slide frame. Not the 11 ms target: one slide costs pdfTeX itself about 38 ms here (below) |
| App (`FLASHTEX_ENGINE_V3=1`) | page counts and 4:3 / 16:9 geometry right; 166/175 captured pages pixel-identical to the PDF at fit width and in tiles; three app bugs fixed (#1450, #1451, #1452); [App](#app) |

## Corpus

Three sets, all through `tools/parity/parity.py` (P-T1 with `\tracingall` and box dumps at every
`\shipout`, P-T2 after qpdf normalisation, both against a fresh pdfTeX run to convergence):

1. **The fixtures tier's ten beamer decks** (`fixtures/real-world/beamer-*`, in CI's merge queue
   already): default, Madrid, overlays, blocks and columns, fragile frames, Polish, sans operator
   names ×3, visuals.
2. **TeX Live's own beamer examples** (the `tl-*` entries of the new `beamer` tier, copied from the
   installation and pinned by SHA-256, nothing committed): metropolis's `demo.tex` (pgfplots,
   booktabs, `allowframebreaks` bibliography, appendix numbering), the conference talk, the lecture
   in beamer and in article mode (`beamerarticle`, with JPEG and PDF pictures), the three
   `solutions` talks (ornate themes, `thebibliography`, `\pause`), and the prosper, seminar, foils
   and texpower emulations.
3. **Decks written for this lane** (`fixtures/beamer-v3/*`, the `v3-*` entries):

| deck | covers |
|---|---|
| `warsaw-beaver` | `\usetheme{Warsaw}` + `\usecolortheme{beaver}`, sections, navigation bars, TOC, blocks, columns |
| `madrid-dolphin` | `\usetheme{Madrid}` + `\usecolortheme{dolphin}`, `\logo`, theorem environments, `\againframe` |
| `metropolis` | `\usetheme{metropolis}` under pdflatex, progress bar, standout frame, appendix |
| `overlays-advanced` | `\pause`, `\only`, `\uncover`, `\visible`, `\invisible`, `\alt`, `\temporal`, `<+->`, `[<+->]`, `\alert<>`, `overlayarea`, `actionenv`, transitions, buttons, relative overlay specs |
| `handout-2on1` | `handout` mode with `| handout:` specs, `pgfpages` 2 on 1 |
| `graphics` | `\includegraphics` (PDF, PNG, JPEG) with overlays, a background image, a `\logo` image, a rotated PDF page |
| `tikz-overlays` | TikZ with `\only`, `\visible`, `\pause`, an `onslide` style, `remember picture, overlay`, shading |
| `notes` | `\note`, `show notes on second screen=right` (pgfpages) |
| `bibliography` | `thebibliography` with beamer's bibitem templates, `\cite` |
| `allowframebreaks` | `[allowframebreaks]` on a long list, long prose and a long bibliography |
| `widescreen-169` | `aspectratio=169` with columns and blocks |
| `article-mode` | `beamerarticle` |
| `long-deck` | 60 frames, 118 pages (overlays of 1, 2 and 3 slides), for the incremental benchmark |

### Results (VERIFIED, mac-m1max-a)

| document | pages | P-T1 | P-T2 | level |
|---|---:|---|---|---|
| fixtures/real-world/beamer-blocks-columns | 6 | pass | pass | L3 |
| fixtures/real-world/beamer-default | 7 | pass | pass | L3 |
| fixtures/real-world/beamer-fragile | 8 | pass | pass | L3 |
| fixtures/real-world/beamer-madrid | 7 | pass | pass | L3 |
| fixtures/real-world/beamer-overlays | 14 | pass | pass | L3 |
| fixtures/real-world/beamer-polish | 13 | pass | pass | L3 |
| fixtures/real-world/beamer-sans-operators | 1 | pass | pass | L3 |
| fixtures/real-world/beamer-sans-operators-professional | 1 | pass | pass | L3 |
| fixtures/real-world/beamer-sans-operators-serif | 1 | pass | pass | L3 |
| fixtures/real-world/beamer-visuals | 7 | pass | pass | L3 |
| tl-conference-ornate-en | 23 | pass | pass | L3 |
| tl-conference-talk | 31 | pass | pass | L3 |
| tl-emulation-foils | 4 | pass | pass | L3 |
| tl-emulation-prosper | 9 | pass | pass | L3 |
| tl-emulation-seminar | 12 | pass | pass | L3 |
| tl-emulation-texpower | 56 | pass | pass | L3 |
| tl-generic-ornate-en | 14 | pass | pass | L3 |
| tl-lecture-beamer | 33 | pass | pass | L3 |
| tl-lecture-print | 8 | pass | pass | L3 |
| tl-metropolis-demo | 32 | pass | pass | L3 |
| tl-speaker-intro-en | 1 | pass | pass | L3 |
| v3-allowframebreaks | 14 | pass | pass | L3 |
| v3-article-mode | 1 | pass | pass | L3 |
| v3-bibliography | 3 | pass | pass | L3 |
| v3-graphics | 7 | pass | pass | L3 |
| v3-handout-2on1 | 2 | pass | pass | L3 |
| v3-long-deck | 118 | pass | pass | L3 |
| v3-madrid-dolphin | 6 | pass | pass | L3 |
| v3-metropolis | 10 | pass | pass | L3 |
| v3-notes | 5 | pass | pass | L3 |
| v3-overlays-advanced | 27 | pass | pass | L3 |
| v3-tikz-overlays | 15 | pass | pass | L3 |
| v3-warsaw-beaver | 9 | pass | pass | L3 |
| v3-widescreen-169 | 4 | pass | pass | L3 |

Excluded: `tl-beamer-userguide` (beamer's user guide; pdflatex stops on ``File `beamerugthemedefault'
not found``: the guide needs theme pictures TeX Live does not ship). It is left out of the manifest.

The only difference in any log is the non-gating accounting (DESIGN §1.1): the capacity totals of
the end-of-run block (`strings out of 467525` vs `467557`), because the two `pdflatex.fmt` files
were built separately. A by-hand check outside the harness agrees: on all 13 decks of set 3, the
engine's third pass writes a PDF **byte-identical** to pdflatex's.

`report-fixtures.md` and `report-beamer.md` are the harness's reports; `scoreboard-*.json` its
summaries.

Run it with:

```
CARGO_BUILD_JOBS=4 scripts/engine-parity.sh --work <work> build
python3 tools/parity/parity.py --tier beamer --engine <work>/eng/flashtex-initex \
  --engine-env FLASHTEX_FORMATS=<work>/fmt --engine-env FLASHTEX_POOL=<work>/eng/pdftex.pool \
  --pt on --raster none --pt1-timeout 3600
```

The `beamer` tier is `on_demand` (a bare `corpus.py fetch` stays the T3 tiers'). Its `repo`
entries are this repository's own decks: `tools/parity/corpus.py` copies such an entry's
directory into the cache and copies it again when its content changes (tests
`test_repo_entry_is_copied_with_its_directory_and_recopied_when_edited`, `test_beamer_manifest`).

## Incremental

`long-deck` (118 pages, 60 frames) through `flashtex-host --socket` with `dl3-keys`, the T7
socket client, as the app talks to the host: 20 keystrokes per row (each undone by the next), the
first a warm-up, 300 ms apart, viewport on the watched page. The driver is
`scripts/beamer_bench.py` here (t7.py's `Host` and `keys`).

**`dl3-keys` could not pick a line in a beamer deck** (it looks for a prose line whose glyphs are
on exactly one page). A beamer frame's body is collected as an argument and typeset at
`\end{frame}`, so every glyph of a frame carries the `\end{frame}` line, as pdfTeX's SyncTeX
records it, and an overlay repeats the frame on several pages. `dl3-keys --line N --page P` (this
lane) types in a given line and watches a given page; the driver takes the frame's pages from a
dump of the display list.

### Before and after (VERIFIED; times loaded, see above)

| edit (frame's slides; watched slide) | converged | pages re-typeset (median) | edited slide p50/p95 ms | DONE p50 ms |
|---|---|---|---|---|
| letter, page 7 (1 slide) | 0/19 → **19/19** | 112 → **2** | 54/100 → 53/84 | 8,644 → **244** |
| letter, page 63 (1 slide) | 19/19 → 19/19 | 2 → 2 | 49/60 → 61/114 | 138 → 177 |
| letter, page 111 (1 slide) | 0/19 → **19/19** | 8 → **2** | 48/89 → 129/295 | 404 → 394 |
| letter, pages 56–58 (3 slides; the first) | 0/19 → **19/19** | 63 → **9** | 56/118 → 61/86 | 2,890 → **526** |
| letter, pages 56–58 (3 slides; the third) | 0/19 → **19/19** | 63 → **9** | 143/165 → 133/147 | 3,005 → **463** |
| sentence, pages 56–58 (3 slides; the third) | 0/19 → **19/19** | 63 → **9** | 168/344 → 134/140 | 4,657 → **460** |
| newline, pages 59–60 (2 slides; the second) | 0/19 → 0/19 | 60 → 60 | 81/101 → 76/88 | 2,968 → 2,583 |
| split, pages 59–60 (2 slides; the second) | 0/19 → 0/19 | 60 → 60 | 86/124 → 79/82 | 3,857 → 2,570 |
| letter, pages 112–114 (3 slides; the third) | 0/19 → 0/19 | 7 → 7 | 147/185 → 150/258 | 413 → 410 |
| preamble (page 1) | — | 118 (cold) | → 763/769 | → 6,169 |

Pages are 1-based here (`dl3-keys` and the logs count from 0). Load before → after: 11–42 →
15–77. Every keystroke's later pages were marked stale and all current at DONE (`complete` 19/19
in every row). The "before" run is main `3a4ed639c`; its preamble row did not run (`before.log`
ends in `dl3-keys`' "a plain word in the line": the driver passed a line without one, fixed for
the "after" run). The "after" run is main plus #1446 and #1448, with `FLASHTEX_INCR_DEBUG=1`
(the reasons above come from its stderr). Opening the deck (a cold compile, 2 passes, 236 pages)
took 33.9 s at load ~60; the host's peak RSS was 398 MB.

**Why it did not converge, and the fixes** (`FLASHTEX_INCR_DEBUG=1`):

1. `pdf_mem` (#1446). After the edited page every test failed on
   `pdf_mem[196]: 0x1c3e -> 0x1c3d`: the box pointer of a form that had been shipped (a pgf
   shading). A shipped form's box, attributes and resources pointers are dead (pdfTeX flushes the
   box and deletes the token lists when it ships the form, and never reads them again). And for a
   form not yet shipped (`pdf_mem[586]`, made by an overlay frame), the structural comparison
   already follows those pointers but was never handed the words. Regression cases:
   `a_written_form_converges`, `an_unwritten_form_converges`.
2. User actions (#1448). Then `action type/named differs (196611 vs 196610)`: beamer's navigation
   symbols are `user` link actions, and `scan_action` leaves a `user` action's named flag,
   identifier, new-window flag and structure identifier unset (memory leftovers no reader reads).
   Regression case: `beamer_navigation_actions_converge`.

**What still re-runs to the end, not changed here:**

- `newline` and `split` edits: the input `line` is compared (DESIGN §5.3 status: line shifting is
  not built), as for any document.
- An edit in the last frames re-runs the few pages to the end (`\end{document}` always re-runs).

**The edited slide is not 11 ms.** pdfTeX itself spends about 38 ms on a slide of this deck: one
converged pass of `long-deck` takes 4.42–4.56 s in pdflatex and 4.35–4.52 s in the engine
(118 pages, three runs each, interleaved, load 49–63). The host re-typesets the edited frame from
the checkpoint before it, so the first slide arrives after one slide of TeX work, and the third
slide of a three-slide frame after three. Reaching the target on beamer needs the raw-speed
program (DESIGN §5.6, L6) or a finer restart than the frame (the frame body is one argument), not
better convergence.

**Soundness (VERIFIED)**, with both fixes (`tools/incr-bench/soundness.py`, random
single-character replacements, insertions and deletions, each compile and each revert compared
byte for byte with from-scratch runs: PDF, log, aux, out, toc, terminal):

| sweep | documents | compiles | equal to scratch | converged |
|---|---:|---:|---:|---:|
| the 13 decks of set 3, 8 trials each (`incremental/soundness-beamer.jsonl`) | 13 | 208 | **208** | 30 |
| every parity fixture and divergence probe, 4 trials each (`incremental/soundness-fixtures.jsonl`) | 86 | 688 | **688** | 44 |

Both fixes also pass `cargo test --release -p flashtex-engine --test incremental --test
checkpoint` on their own branches.

## App

The Mac app with `FLASHTEX_ENGINE_V3=1`, driven by environment hooks only (as INFDESC-APP did),
on all 13 decks of set 3 and the 10 beamer fixtures; the pane's own bitmaps against Core
Graphics' drawing of pdflatex's PDF at the pane's scale. The sub-lane's evidence is `app/`
(#1454, with new capture hooks for tiles, a typed edit, search lines and the compile's PDF).
Summary as the sub-lane reported it (load 7–94):

- **Pages and geometry:** every deck's page count equals pdflatex's. 4:3 slides fill a 729 pt pane
  at 1038 × 779 px (2.86 px/pt, so beamer pages are tiled from 3 px/pt up at ordinary zooms);
  `aspectratio=169` at 1038 × 584; the notes deck (double width) at 1038 × 390; the 2-on-1 handout
  is two A4 portrait sheets. Nothing is cropped or letterboxed once #1450 is in.
- **Pixels:** after the fixes, 166 of 175 captured pages are identical at fit width, and the same
  pages are identical in 4.25 px/pt tiles. The other 9: 7 pages with an included PDF image (at most
  2–3 grey levels), the bibliography's online icon (up to 161 levels: several transparency-group
  PDFs on one page, drawn directly rather than as the Form XObject pdfTeX writes; open after two
  attempts), and the handout's second sheet (1 px narrower, below).
- **PDF fallback:** 137 of 284 pages draw the PDF (exact): every page with shaded balls or
  headlines, TikZ opacity or shading, buttons, or pgfpages sheets (Warsaw 1–9, allowframebreaks
  1–14, long-deck 74 of 118, notes 1–5).
- **Tiles:** no seams, no missing or stale tiles, each tile exactly its 512 px rectangle, on PDF
  fallback pages, pages with forms and pages with images too.
- **Editing long-deck in the app:** a text edit in frame 20 changed pages 40–42, exactly the pages
  pdflatex changes; an added `\item<4->` took the deck from 118 to 119 pages and refreshed pages
  40–119, none left stale; every captured page equals pdflatex's edited PDF.
- **Search (engine-faithful, not changed):** a click anywhere on a slide selects the frame's
  `\end{frame}` line, and forward search from inside a frame body finds nothing, because every
  glyph of a frame carries that line (as pdfTeX's SyncTeX records it).

App fixes, one PR per root cause, each with a test that fails without it:

| PR | bug | test |
|---|---|---|
| #1450 | with a restored zoom above fit, the pane opened scrolled to the right edge (the first real layout reused an anchor made before the pane had a width) | `EngineV3PreviewNavTests.testFirstRealLayoutOpensAtTheLeftEdgeWhenZoomedIn` |
| #1451 | PNG/JPEG images drawn with antialiased edges, which Core Graphics' PDF drawing does not do for an unrotated image (up to 2,852 px, Δ 252) | `RasterImageEdgeTests` |
| #1452 | pages of one PDF drawn from several threads at once; Core Graphics then draws shadings wrongly at random (balls up to 75 levels off). One lock per document now | `PDFConcurrentDrawTests` |

**Checked here, not an engine issue:** the sub-lane reported the handout's 1 px as the host
sending A4's exact 595.2756 bp as the page box. The host sends `[0, 0, 595.276, 841.89]`, as the
PDF has it (`dl3-dump --summary` of the handout's display list, VERIFIED); 595.2756 bp is TeX's
page width in sp (`width` 39158276). The 1 px is the bitmap's size rounding at 1.7437 px/pt, left
as reported.

## What remains

- Edited-slide latency (above): L6, or a sub-frame restart.
- SyncTeX-equivalent source mapping in frames points at `\end{frame}`, as pdfTeX's SyncTeX does,
  so search inside a frame does not work in the app; an app-side fallback (the frame's line range)
  is possible and not done.
- Several transparency-group PDFs on one page (beamer's online bibliography icon) differ by up to
  161 grey levels in the pane: draw included PDF pages as the Form XObject pdfTeX writes.
- 137 of 284 beamer pages draw the whole PDF (shadings, transparency): correct, but a full PDF draw
  per page.
- `newline`/`split` convergence (line shifting, DESIGN §5.3), not beamer-specific.
- The `beamer` tier is on demand; adding it to nightly is the Commander's call.
- Timing on a quiet reference host.

## Files

- `report-fixtures.md`, `report-beamer.md`, `scoreboard-fixtures.json`, `scoreboard-beamer.json`:
  the harness's reports.
- `incremental/`: `before.log`, `after.log` (one JSON line per row), `raw-*.tar.gz` (dl3-keys'
  lines per keystroke), `summary-after.json`.
- `scripts/beamer_bench.py`: the incremental driver.
