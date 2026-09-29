# Large reflows: the "one long page" (galley) idea, verified

Question from the owner, 2026-09-29: an edit at the top of a 1,000-page document with
no forced page breaks shifts every later page, so §5.3 convergence never fires. Can
we treat the document as one long page (a galley of lines) and only re-split it into
pages?

This was checked in two independent ways: a measurement on pdfTeX 1.40.29 (`measure/`)
and an adversarial source review (`redteam/`). Machine: Apple M5 Pro, TeX Live 2026.

## Verdict

1. **Cutting a long page at fixed heights: rejected.** It can't match pdfTeX.
   - pdfTeX picks the lowest-cost break, not the first overflow (tex.web §1005).
   - It discards glue at the top of a page (§1000), inserts `\topskip` (§1001) and sets
     the glue in box 255 (§1017).
   - It accounts for inserts and footnotes (§§1008–1010).
   - The output routine changes the next page's `\vsize` (§987, LaTeX `\@colroom`).
2. **Refined form: sound, with a measured but moderate gain.** Cache the galley
   material, then re-run TeX's page builder and output routine live.
   - **Cache unit:** a *segment*, meaning everything executed between two consecutive
     outer `build_page` calls (§§812, 1026, 1054, 1076, 1091, 1094, 1100, 1103, 1145,
     1200). Not a paragraph: the output routine can fire after a paragraph's indent box
     and before `\everypar` (§1091, confirmed in initex), and around display math.
   - **Key:** the state the segment reads, compared by value: eqtb, input stack,
     nesting state, page-builder state, object counters, random seed, fonts, files and
     `.aux` entries.
   - **Replay:** the write-set, spliced in as fresh node copies.
   - **Holes that must be closed explicitly:**
     - the page builder and output routine mutate nodes in place (§§1010, 1013, 1017),
       so every splice must be a copy;
     - memory-usage statistics in the log (§639, §1334) must be excluded, or caching
       turned off;
     - nondeterministic reads (`\pdfelapsedtime`, `\write18`) are barriers.
   - **Hidden reads to track:**
     - `\pagetotal`/`\pagegoal` (§421);
     - `\lastpenalty`/`\lastskip` (§424), which perpage relies on;
     - `\pdflastximage`: 32 of 43 image ids shift after an edit;
     - output-routine `\aftergroup` tokens (afterpage);
     - absolute line numbers in warnings.
3. **Gain, measured.** Full-document reflow for 1,000 pages, from the edit to the end:

   | body | not cached | galley cached | speed-up |
   |---|---|---|---|
   | prose | 0.57 s | 0.31 s | 1.8× |
   | + math | 0.77 s | 0.32 s | 2.4× |
   | + footnotes + siunitx (hyperref) | 5.87 s | 2.33 s | 2.5× |
   | same, hyperref page labels/anchors off | 3.76 s | 0.31 s | 12× |
   | TeX by Topic (311 pp., real) | | | 1.6× |
   | memoir manual (525 pp., real) | | | 4.4× |

   - The **page builder itself costs about 0.01 ms per page.** Pagination (P) is almost
     entirely LaTeX output-routine macro code.
   - With hyperref, two per-page PDF-string calls, for the page label and the page
     anchor, dominate. They cost 0.15 ms per page, rising to **1.96 ms per page once
     siunitx is loaded**. That is a floor no galley cache can remove.
   - Line breaking is only 0.5–6% of CPU samples. The galley (G) is mostly the body's
     macro expansion.
4. **Hit rate (projection, not yet measured on the engine).** From 149 arXiv papers:
   - about 91% of segments survive a page-shifting edit;
   - 25–30% miss when cleveref is used, because its `\label` reads `\c@page`;
   - an edit that adds an equation or footnote renumbers everything after it, and those
     segments miss too.

## Consequences for DESIGN.md

- The galley cache goes in as **L3.5 "segment memo"**, planned for P4 and **gated**
  (§5.7). It is built only if, on the real engine, it beats the uncached background
  reflow by ≥ 2× on the §1.2 1,000-page benchmark with P-T1 unchanged.
- **The bigger lever is making the per-page output routine cheap**, because P is the
  floor for any strategy. hyperref's per-page PDF-string work is now a named first
  target of the L6 guarded-intrinsics program (§5.6).
- §5.4's per-page background cost is corrected with these measurements: 0.6–5.9 ms
  per page, depending on packages, rather than 2.4 ms.

## Files

- `measure/`: generators, benchmark drivers, `verify.sh` (the chunked split gives
  identical DVI and text output), `ortime.tex` (the output-routine timer),
  `analysis.md` and raw `results*.jsonl`.
- `redteam/`: `SUMMARY.md`, the corpus scanners and the initex probes (`tiny/`).
