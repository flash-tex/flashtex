# Unicode mode's M3 gate, measured (2026-10-11)

Lane UNICODE-M3 (engineer under the Commander `kabir-claude`). The gate is the
M3 row of [`docs/design/modes/PROPOSAL.md`](../../design/modes/PROPOSAL.md)
("Unicode in the product"). Retirement stages S6/S7 wait on it. Every
criterion that GitHub-hosted runners can measure was run there by
[`.github/workflows/unicode-m3.yml`](../../../.github/workflows/unicode-m3.yml).
The harnesses are in [`tools/unicode-m3/`](../../../tools/unicode-m3/README.md).

- **Oracle:** TeX Live 2026's `xelatex`/`xetex`, XeTeX 3.141592653-2.6-0.999998.
  This is the scheme-full `texlive/texlive@sha256:a7ae4dfa…` image that
  nightly.yml pins and the no-TeX-Live bundle is packed from. It is never in
  the product path.
- **Runners:** ubuntu-latest (Linux x86-64; `perf` cannot count there, so
  instruction counts come from callgrind) and macos-15 (Apple M1, virtualised,
  with no TeX Live).
- **Runs:** [38178866539](https://github.com/flash-tex/flashtex/actions/runs/38178866539)
  for every criterion, and [38185041540](https://github.com/flash-tex/flashtex/actions/runs/38185041540) for P-T1 (with the panic's frames) and the export after the colour fix. Its no-TeX-Live and Core Graphics totals equal the first run's.

## Verdict, 2026-10-21

**Verdict: M3 is MET.** The two rows still open on 10-20, row 3 (the §1.2 edit row at b4's
last line) and row 5 (no TeX Live, in the product path), are met now. Rows 1, 2, 4, 6 and 7
stand as re-measured on 10-20 below. Nothing was re-run for this verdict. It cites runs that
are already on main.

| # | M3 criterion | 2026-10-21 verdict | Evidence (verified unless marked belief) |
|---|---|---|---|
| 3 | §1.2 latency on the bench documents | **met** | **Edit row**, measured on the reference Mac (Apple M5 Pro, mains power), by `scripts/t7-reference.sh`'s Unicode leg on main `8b3f9ec62`, which is after #1871 (`cb0aa96a7`): [`t7-reference-2026-10-21d-…/unicode/table.md`](../t7-reference-2026-10-21d-Kabirs-MacBook-Pro/unicode/table.md) (#1874). Edited page p95 against the host's ≤ 11 ms share: b1 6.8 / 7.8, b2 2.1 / 2.7, b3 3.7 / 3.7, **b4 5.2 / 9.1** (EDITPOINT / end), b5 9.2 at EDITPOINT. On main `58d6d773f` ([`…-21e-…/unicode/table.md`](../t7-reference-2026-10-21e-Kabirs-MacBook-Pro/unicode/table.md), #1876), b4's end is 9.7 ms p95. That run was on battery. Its header says NON-REFERENCE because the script prints that, but its "Owner ruling (2026-10-21)" section overrides the label and records it as a reference run, under the owner's standing direction for this Mac. b5's end (26.4 ms p95 on `8b3f9ec62`) is reported only, as in [`unicode-m3-mac-2026-10-21`](../unicode-m3-mac-2026-10-21/README.md). **Preamble row**: met there on the same Mac (main `5439b97ea`): the largest first page is 347.9 ms against ≤ 400 ms. **Reopen row**: reported only, as in Classic T7. The host's share is 20–44 ms to page 1 from the persisted S₀ in both runs above |
| 5 | No-TeX-Live run of the corpus from the bundle | **met** | `texbundle-tl2026-5` was published on 2026-10-21 (01:20Z). `unicode-m3.yml` run [39642712484](https://github.com/flash-tex/flashtex/actions/runs/39642712484) (`69b0bb0de`, the S5 branch merged as #1849): its macOS leg fetched the lock's release over https (HELLO `texlive=None resolver=bundle 9c4e31ad…`, the digest `flashtex-bundle.lock` pins). **44 of 50** cases compiled with a PDF. The other 6 are the cases xelatex itself errors on. **47 PDFs were byte-identical** to the Linux TeX Live run's, with 0 structural differences against xelatex |

## Re-measured on main, 2026-10-20

Every row was run again on main `f5ef18a85` (after #1833 FMT-NUM, #1838
font cache, #1817/#1818 S₀ delta and GC, #1842 STATSIG-CTIME, #1780).
The procedure is this README's own. Runs:
- [39422784768](https://github.com/flash-tex/flashtex/actions/runs/39422784768): `unicode-m3.yml`. It covers P-T1, export, host, Classic and the macOS leg.
- [39422789482](https://github.com/flash-tex/flashtex/actions/runs/39422789482): `sweeps-unicode.yml`, `ref=main`. It covers the sweep, the reopen scenarios and the callgrind latency job.
- [39422832866](https://github.com/flash-tex/flashtex/actions/runs/39422832866): `unicode-m3-bundle.yml`, which runs the corpus with no TeX Live from the bundle the lock pins.

The NixOS PC (x86-64, callgrind 3.27.1, TeX Live 2026 at `~/texlive/2026`) ran the latency rows the hosted job lacks. These are marked PC.

**Verdict: M3 is still NOT MET.** One row misses: the §1.2 edit row for a keystroke at b4's last line. Two rows are met but wait on outside steps. The no-TeX-Live row needs the locked bundle published. The wall-time rows need the reference Mac.

| # | M3 criterion | 2026-10-20 verdict | Evidence (verified unless marked belief) |
|---|---|---|---|
| 1 | Unicode corpus P-T1 100 % | **met** | 39422784768 `linux pt1`: **49 cases, 49 equal, 0 differ** (`latex.py`, P-T1 log, exit status, XDV) |
| 2 | Export gate (Q2, R1) | **met** | 39422784768 `linux export`: **48 of 48 pass**, 1 listed and not counted (xeCJK-example-fallback: xelatex writes no PDF). 0 structural differences. XDV check max 0.000252 bp. macOS Core Graphics: 48 pairs, 0 structural, 1 at 0 px, 360,324 px at 2× and 783,451 px at 3×. These totals equal 10-12's, and pixels are reported, not gated (the 10-09 ruling) |
| 3 | §1.2 latency on the bench documents | **edit row not met at b4's end**; edit row at the edit point and reopen row met by estimate; preamble row **pending the reference Mac** | See "Latency, 2026-10-20" below |
| 4 | Incremental soundness sweeps on Unicode | **met** (it runs as defined now: the incremental Unicode engine landed) | 39422789482: **76 documents, 684 compiles, 0 bad**. Each compile is equal, every file and every display-list frame, to a from-scratch compile in a separate process. `reopen` job (`unicode_reopen.py`): **0 bad** over 43 documents. Each is compiled cold, then reopened four ways: unchanged, after a body edit, after a preamble edit, and after an edit at the preamble's end. Each reopen goes on from S₀ or is refused, as it must be, and equals a from-scratch compile. The host's own spares-against-cold sweep (39422784768 `linux host`): 53 documents, 318 of 318 compiles equal |
| 5 | No-TeX-Live run of the corpus from the bundle | **met for the locked bundle; not yet in the product path** | 39422832866 packed the bundle from main's lists. Its digest is `9c4e31ad…`, **the digest `flashtex-bundle.lock` pins** (texbundle-tl2026-5). On macos-15, sandboxed with no TeX Live, **44 of 50** cases compile cleanly. The other 6 are exactly the cases where xelatex itself errors (fontspec-example, polyglossia, u04, unimath-symbols, xeCJK-example-fallback, xeCJK-example-listings), and the TeX Live run fails them the same way. **47 of 47 PDFs written are byte-identical** to the Linux TeX Live run's. The other 3 have no Linux PDF to compare: l004 needs macOS fonts; unimath-symbols and fallback write none. **But texbundle-tl2026-5 is not published** (`gh release view`: release not found). So `unicode-m3.yml`'s macOS leg, which fetches the lock's release over https, ran with no bundle: 0 of 50, and `xelatex.fmt` failed to build. Publishing the release (bundle-publish.yml) closes this row. It is a Commander/owner step |
| 6 | `[fonts]` fixtures and precedence | **met** | f01–f06 are among the 49 P-T1 cases and the 48 export passes above. `tests/host.rs` with TeX Live (39422784768 `linux host`): **18 of 18** pass, including the `[fonts]` precedence test |
| 7 | Classic unchanged | **met for the U3 merges**; later shared-crate merges not paired | 39422784768 `classic`. Each U3 merge equals its first parent in size and SHA-256: pre-U3 and #1732 merge are 10,043,944 bytes, `9c813bd0…`; `08708c4e2^1` and #1734 are 10,043,944 bytes, `9c813bd0…`; `6b97bae22^1` and #1762 are 10,798,296 bytes, `76511179…`. Start-up to HELLO for the same pairs: 311.6 / 311.5 M, 311.6 / 311.6 M and 302.2 / 302.3 M instructions. Main's `flashtex-host` today is 11,272,736 bytes and starts in 219.2 M. That comes from Classic's own work since. Five later Unicode merges also touched shared crates, as deliberate shared changes: #1775 and #1784 (runtime 2b and 2c: zlib and kpathsea moved into `flashtex-runtime`), #1842 (`StatSig` change time), #1840 (SHA-256) and #1838 (`crates/pdf` TrueType). They are not paired here; Classic's own CI gates them. Classic's T7 latency: pending the reference Mac |

### Latency, 2026-10-20

All figures are M instructions of the whole host process, from callgrind on
x86-64 (39422789482's `latency` job unless marked PC). "Edited page" means
up to the frame of the page the keystroke changed (`flashtex_page_sent`).

| bench | keystroke at EDITPOINT: edited page / DONE | keystroke before `\end{document}`: edited page / DONE | reopen from the persisted S₀ in a new host: first page / DONE |
|---|---:|---:|---:|
| b1 article | 63.1 / 93.8 | 63.1 / 93.9 | 327.8 / 382.9 |
| b2 fontspec, by name | 17.9 / 28.2 | 18.0 / 28.3 | 297.9 / 329.1 |
| b3 unicode-math | 37.4 / 50.2 | 37.4 / 50.3 | 311.6 / 349.1 |
| b4 fontspec, 19 pages | 83.5 / 763.4 | **545.3** / 755.3 | 270.9 / 1,504.0 |

- **Edit row (≤ 16 ms, host ≤ 11 ms).** Belief, not measured: at about 10 G
  instructions/s on an M1-class core, every keystroke at EDITPOINT reaches
  its page in about 2–8 ms. That meets the row. b4's keystroke before
  `\end{document}` does not: 545 M is about 55 ms (PC on main `41a3b38cc`:
  500 M). Why: `\lipsum[61-100]`'s optional-argument peek reads the
  `\end{document}` line at page 11. From then on, the two runs differ in
  `buffer`, `first`, `last` and in `eqtb` (`\l_peek_token`) through page 18
  (the `[conv]` notes of the latency job). So no page from 12 to 18 passes
  the convergence test, and the run rebuilds pages 11–19.
  Converging soundly needs a rule for words that differ but are never read
  before they are overwritten. That rule needs read tracking on `eqtb` (a
  web2rust read barrier). It is new machinery, not a port: XETEX-INCR
  stopped there on 10-12 and asked the Commander to choose. This lane
  measured it again and did not start that work.
- **The first keystroke right after a cold compile** (PC, `unicode.py
  --no-s0-wait`, one keystroke at EDITPOINT, edited page / DONE). On main
  `f5ef18a85` it queues behind the S₀ copy: b1 997.3 / 1,065.0, b2 983.5 /
  999.6, b3 1,035.8 / 1,055.5, b4 1,117.4 / 1,747.2. With #1836 (the S₀ is
  saved when the engine is idle, and the save stops for a request): b1
  72.1 / 102.1, b2 47.6 / 58.1, b3 81.9 / 95.0, b4 150.3 / 715.1. The hosted
  latency job on #1836's branch ([39434091379](https://github.com/flash-tex/flashtex/actions/runs/39434091379))
  gives b1 71.5 / 101.6, b2 45.6 / 56.0, b3 82.0 / 95.1 and b4 153.1 / 735.5.
  Its other rows are within 1 M of main's in the table above.
- **Reopen row (≤ 100 ms to the first page).** Belief: 271–328 M is about
  27–33 ms, which meets the row.
- **Preamble row (≤ 400 ms):** pending the reference Mac, as before.
- **`fmt_num` (#1833).** PC on main `41a3b38cc`, b4 keystroke at EDITPOINT,
  744.8 M to DONE. `out::num::push_num`, which replaced `fmt_num`, is
  **29.1 M** (inclusive). The whole preview PDF write (`out::finish`) is 56.1 M.
  On 10-12 the write was 441 M, with 275 M of it in `fmt_num`. The largest
  cost outside the engine now is SHA-256, 181.3 M: the PDF `/ID` hash thread
  (#1840) is 136.6 M of it, off the engine thread. It is not a gate row.

## Measured 2026-10-11

**Verdict (2026-10-11): M3 is NOT MET.** Of the gate's seven criteria:
- two are met: the `[fonts]` fixtures, and Classic unchanged so far as hosted
  runners can show it;
- four are not met: Unicode corpus P-T1, the export gate, the no-TeX-Live run,
  and the §1.2 edit and reopen rows;
- one cannot run as defined: Unicode mode has no incremental engine to sweep.

The §1.2 wall-time rows also wait for the reference Mac. Below, **verified**
means a number measured in the runs above. **Belief** means an inference that
was not measured.

## Checklist

| # | M3 criterion | Verdict | Numbers (verified unless marked) |
|---|---|---|---|
| 1 | Unicode corpus (fontspec, unicode-math, polyglossia, xeCJK; Q2's corpus) at P-T1 100 % | **met** (2026-10-12) | **49 of 49** equal in run [38226934484](https://github.com/flash-tex/flashtex/actions/runs/38226934484), after ICU's bidi (#1812), the line breaker (#1816) and one font file per name for the oracle (#1822); the measurement of 2026-10-11 follows. Then: **46 of 49** equal (P-T1 log, exit status, XDV): fontspec 15/15, `[fonts]` 6/6, unicode-math 7/7, xeCJK 16/16 (thuthesis included), polyglossia 2/5. Failing: u03 (Hebrew) and u04 (Arabic), XDV differs (no bidi, ranked 1); `polyglossia` (the manual), the port panics (ranked 2). 224.7 M log lines compared |
| 2 | Export gate per Q2 (structural; 2×/3× pixels) | **met** (2026-10-12, with R1) | Run [38274237575](https://github.com/flash-tex/flashtex/actions/runs/38274237575): **48 of 48** pass (5 of them under R1: fontspec-example, polyglossia, u04, unimath-symbols, xeCJK-example-listings), 0 structural differences, XDV check max 0.000252 bp; xeCJK-example-fallback listed, not counted (xelatex writes no PDF). Before R1, run 38250091155: 2026-10-12, run [38250091155](https://github.com/flash-tex/flashtex/actions/runs/38250091155): **43 of 49 pass, 0 structural differences in all 49**. The 6 others fail only because xelatex itself exits 1 there (R1 below proposes how to count them). The measurement of 2026-10-11 follows. Then: 23 of 49 cases pass structurally. Another 19 differ only in `ToUnicode`; 15 of them because FlashTeX writes a `ToUnicode` where xdvipdfmx writes none (CID-keyed CJK fonts, by design, PLAN.md §4B). 3 differ for real: u03, u04 (no bidi) and u11 (beamer's links); xeCJK-example-mathblock's colour was a fourth, fixed here. 4 are cases the oracle itself errors on (2 with no comparable PDF). 204,703 glyphs compared; positions within 0.0003 bp of TeX's XDV wherever glyphs pair. Core Graphics: 47 pairs, 1 at 0 px; 187,677 px in all at 2×, 432,795 px at 3×. That is a floor, not 0, as the 2026-10-09 ruling expects |
| 3 | §1.2 latency on xelatex bench documents | **not met** (edit and reopen rows); preamble row **pending the reference Mac** | Every Unicode compile is a full run. Start-up to the format loaded is 848 M instructions. A full run is 1,504 M (article) to 30,554 M (300 pages); xelatex takes 1,514 M to 14,253 M. Host on ubuntu-latest, first page: article 144 ms, unicode-math 663 ms, edit = preamble = reopen. Belief: 848 M instructions take far more than 16 ms (edit) or 100 ms (reopen) on any M-series core |
| 4 | Incremental soundness sweeps on Unicode | **cannot run as defined** (no incremental Unicode engine, INCREMENTAL.md §5) | What does exist was swept: the host's reuse between compiles (hot spares, `.aux` passes). 53 documents, 318 compiles; host A (spares) equals host B (cold) in pages, fonts, forms and images, frame for frame, in 318 of 318 |
| 5 | A no-TeX-Live run of the corpus from the bundle | **not met** | macos-15, sandboxed, bundle `dc72dde1…` (HELLO: no TeX Live, resolver `bundle`). 27 of 50 compile cleanly with a PDF. Each of those, except the macOS-only l004, is **byte-identical** to the Linux TeX Live run's PDF (26 of 26). The other 23: 10 need packages the bundle lacks (`bidi`, `thuthesis`, `zhnumber`, `l3draw`, `showexpl`, `typogrid`, `unicode-math.dtx`). 13 end with status `error` and fonts missing: fonts the bundle lacks, or TeX Gyre named by family, which only TeX Live's fontconfig set-up finds here (f01–f04, f06, u08, fontspec-example). fontspec-example and unimath-symbols fail with TeX Live too |
| 6 | `[fonts]` fixtures: P-T1 + XDV against xelatex on the effective source; precedence (document font commands win) | **met** | 6 of 6 fixtures: P-T1 log, exit status and XDV equal to xelatex on the same first line, which comes from FlashTeX's own generator (`fonts_line`). Expected and forbidden fonts hold in both engines: the role applies, `\setmainfont` and `\setmathfont` in the document win, a local `\fontspec` group wins inside it, and a refused name is not applied. All 6 are also structurally equal in the PDF. `tests/host.rs` with TeX Live: 7 of 7 pass (the `[fonts]` precedence test is one) |
| 7 | Classic lockstep, P-T1/P-T2 fixtures, latency, `flashtex-host` size and start-up unchanged | **met** for what hosted runners show | The U3 merges (#1732, #1734, #1762) change no file of Classic's engine: only `crates/flashtex-xetex`, the bundle lock and contents, packaging, docs, and `scripts/gate.sh`'s list of what builds flashtex-xetex (each merge diffed against its first parent). `flashtex-host` built at each U3 merge and at its first parent: size equal for each pair (10,043,160 / 10,043,160; 10,043,160 / 10,043,160; 10,798,384 / 10,798,384 bytes); SHA-256 equal for all three pairs in run 38173318789. Start-up to HELLO: 311.9 M against 312.0 M, 312.1 M against 312.1 M, and 302.3 M against 302.4 M instructions. ci.yml after #1762's merge (run 38028759045): engine parity lockstep, P-T fixtures 1/2 and 2/2, and engine tests all pass. Classic's T7 latency needs the reference Mac: **pending** |

## 1. P-T1 + XDV (criterion 1)

`tools/xetex-lockstep/latex.py --cases-dir` (run 38185041540). Each
engine builds its own `xelatex.fmt`. The two format logs differ in one known
line, the string pool's count (97,840 against 97,825 strings; latex.py
explains it). Each case then runs untraced, then traced. 46 of the 49 Linux
cases (l004 needs macOS's fonts) are compared under `\tracingall`. Three long
documents (polyglossia, thuthesis-example, unimath-symbols) are compared by
box dumps at every `\shipout` only.

| family | cases | equal | not equal |
|---|---:|---:|---|
| fontspec (l001–l010 less l004, d01–d05, u08, u11, u12, fontspec-example) | 15 | 15 | - |
| `[fonts]` fixtures (f01–f06) | 6 | 6 | - |
| unicode-math (l007–l009, u09, u10, unimath-example, unimath-symbols) | 7 | 7 | - |
| xeCJK (u05–u07, the 12 xeCJK examples, thuthesis-example) | 16 | 16 | - |
| polyglossia (u01–u04, the polyglossia manual) | 5 | 2 | u03, u04, polyglossia |
| **all** | **49** | **46** | |

Six cases end in a TeX error in xelatex too. Four of them end in the same
error in both engines, so they count as equal:
- fontspec-example and xeCJK-example-fallback: fontspec cannot find a font;
- unimath-symbols: `unicode-math.dtx` is not in the tree;
- xeCJK-example-listings.

The other two count as failures: u04 (`Missing number`, and the bidi
difference) and polyglossia (the missing Syriac font, then the panic).

## 2. Export (criterion 2)

**2026-10-12 (lane M3-EXPORT), run
[38250091155](https://github.com/flash-tex/flashtex/actions/runs/38250091155):
43 of 49 pass; 0 structural differences in every case.** Each class of
the earlier failures, and what was done:

| class (2026-10-11) | cases | resolution |
|---|---|---|
| `ToUnicode` where xdvipdfmx writes none (Adobe-GB1/Japan1 fonts: Fandol, HaranoAji) | 15 CJK + polyglossia, thuthesis | **fixed**: FlashTeX writes none for a standard Adobe collection either (`type0.c` `CIDFont_is_ACCFont`), PLAN.md §4B updated |
| `ToUnicode` code-point choices and coverage (Amiri's positional forms, TeX Gyre small capitals, `.notdef`, math variants, `visiblespace`) | u04, l007, l008, u08, u09, xeCJK-example-listings, polyglossia | **fixed**: `ToUnicode` is a port of xdvipdfmx's (`tt_cmap.c`, `tt_gsub.c`, `agl.c`, `pdfencoding.c`; `src/out/tounicode.rs`, `src/out/agl.rs`). A Python prototype of the same algorithm matched xelatex's PDFs on 243 of 248 native fonts and 33 of 33 Type 1 fonts before the port; the 5 others were local math-font versions |
| glyph outline (u03's DavidCLM, polyglossia's LinLibertine: 5 glyphs) | u03, polyglossia | **comparator fixed, not widened**: xdvipdfmx writes a charstring operand within 3.0e-5 of an integer as that integer (`cs_type2.c` `clear_stack`), FlashTeX the font's own value; the outlines differ by at most 3.05e-5 font units, but straddled the digest's 1/100-unit rounding. The comparator now compares coordinates within half of its stated 1/100-unit resolution (`fonts.py` `same_outline`, `test_outline.py`) |
| Core Graphics pixels | all | **already ruled**: the 2026-10-09 ruling on #1712 makes the gate structural parity with positions within 0.01 bp + 0.005 em, not 0 px; the pixels are reported, not gated |
| the oracle errors (xelatex exits 1) | fontspec-example, unimath-symbols, xeCJK-example-fallback (no PDF), xeCJK-example-listings, u04, polyglossia | **needs a ruling (R1)**: structurally equal (0 differences) in all five that have a PDF; they fail only on the reference's exit status |

**R1 (approved by the Commander, 2026-10-12; applied in `tools/xetex-pdfparity/run.py`).** As P-T1 counts a case
whose two engines end in the same TeX error as equal (§1), the export gate
counts a case whose reference exits non-zero as passing when the candidate
exits with the same status and the same first error, both write a PDF, and
it has 0 structural differences and passes the XDV position check (where
the reference wrote an XDV). A case whose reference writes no PDF
(xeCJK-example-fallback) cannot be compared: it is listed, not counted, and
the denominator is the cases with a reference PDF. With R1: **48 of 48**
expected; measured in run [38274237575](https://github.com/flash-tex/flashtex/actions/runs/38274237575): **48 cases, 48 pass, 0 fail**, 1 more listed and not counted (xeCJK-example-fallback); 0 structural differences; XDV check max 0.000252 bp.

The macOS leg (Core Graphics, run 38250091155): 48 pairs, every one with 0 structural differences; 1 at 0 px; 360,324 px in all at 2×, 783,451 px at 3×, the same totals as before the `ToUnicode` port (it draws nothing). The rise from 187,677 px (2026-10-11) is the polyglossia manual, which now runs to its end (182,476 px over 85 pages). A floor, not 0, under the 2026-10-09 ruling.

Before that (2026-10-11):

`tools/xetex-pdfparity/run.py --cases-dir … --no-visual` on Linux, then
`tools/unicode-m3/visual.py` on macos-15 over the same PDFs (Core Graphics):

| class | cases |
|---|---|
| structurally equal | 23 of 49: d01–d05, f01–f06, l001–l003, l005, l006, l009, l010, u01, u02, u10, u12, unimath-example |
| `ToUnicode` only, xdvipdfmx writes none (CID-keyed CJK fonts) | 15: u05–u07, thuthesis-example, 11 xeCJK examples (mathblock included, after the colour fix) |
| `ToUnicode` only, code-point choice (two code points, one glyph) | l007, l008, u08 (small capitals read as PUA) |
| `ToUnicode` of `.notdef` | u09 (FlashTeX U+FFFF, xdvipdfmx none) |
| differs | 3: u03 and u04 (right-to-left), u11 (beamer's navigation links). xeCJK-example-mathblock differed in 128 glyph colours until this lane's fix (run 38178866539 against 38185041540) |
| the oracle errors in this environment | fontspec-example, unimath-symbols (equal otherwise); xeCJK-example-fallback (xelatex writes no PDF); polyglossia (FlashTeX panics, damaged PDF); xeCJK-example-listings is also counted above (`ToUnicode` only, including CMTT9's visible space, which xelatex maps and FlashTeX does not) |

Positions: once glyphs pair, the PDF's glyph origins are within 0.000252 bp
of TeX's XDV positions (0.001 bp is the tolerance). The XDV check now puts
the page origin at the paper-size special's unrounded height, as dvipdfmx
does. Before that change, beamer's 96 mm page showed a spurious 0.004 bp.

## 3. Latency (criterion 3)

Run 38178866539, ubuntu-latest. Instructions are user-space, counted by
callgrind; each is the best of 3 runs with a warm `.aux`. Wall times are
from wait4 on a shared x86 runner, so they are **not** the reference
hardware's.

| document | FlashTeX, PDF (M instr) | ms | FlashTeX `-no-pdf` | xelatex `-no-pdf` | xelatex + xdvipdfmx |
|---|---:|---:|---:|---:|---:|
| b1 article + amsmath + graphicx | 1,504 | 156 | 1,307 | 1,514 | 2,473 |
| b2 fontspec, TeX Gyre Termes by name | 3,320 | 440 | 3,284 | 2,528 | 3,448 |
| b3 unicode-math + amsmath | 7,001 | 656 | 6,900 | 5,289 | 6,220 |
| b4 fontspec, 11 pages | 4,565 | 502 | 3,671 | 3,109 | 4,146 |
| b5 fontspec, about 300 pages | 30,554 | 4,160 | 15,425 | 11,241 | 14,253 |
| start-up to the format loaded | 848 | 72 | | 1,087 | |

The host (`flashtex-host-unicode`, dl3-client), time to the first page in ms:

| document | open | reopen | 5 edits in a paragraph | preamble edit |
|---|---:|---:|---|---:|
| b1 | 145 | 144 | 142–145 | 144 |
| b2 | 446 | 446 | 444–447 | 449 |
| b3 | 663 | 662 | 662–667 | 664 |
| b4 | 287 | 290 | 285–293 | 288 |
| b5 | 296 | 293 | 292–295 | 294 |

Against the §1.2 targets:
- **Edit, ≤ 16 ms (host ≤ 11 ms):** not met. Every keystroke is a full run;
  the spare saves only the process start.
- **Reopen, ≤ 100 ms to the first page:** not met.
- **Preamble edit, ≤ 400 ms to the first page:** met on this runner for b1,
  b4 and b5; not met for b2 (446 ms) and b3 (664 ms). Pending the reference
  Mac.

Two further observations:
- **FlashTeX's own PDF writing costs more than xdvipdfmx at length:** b5
  costs 15.1 G instructions beyond `-no-pdf`, where xdvipdfmx adds 3.0 G.
- **`flashtex-host-unicode` itself:** 8,589,016 bytes (7,197,256 stripped),
  HELLO in 38 ms median, 644 M instructions.

## 4. Soundness (criterion 4)

`tools/unicode-m3/host_sweep.py`: the whole corpus and the five bench
documents. Each gets five scripted edits and a revert, compiled by host A
(spares on) and by host B (spares off, every compile cold), each in its own
copy of the project. Result: 318 of 318 compiles are equal. Of these, 40
compile to a TeX error in both hosts (the cases the oracle errors on). This
does not replace the sweep the gate asks for: that needs the incremental
Unicode engine of INCREMENTAL.md §5.

## 5. No TeX Live (criterion 5)

`tools/unicode-m3/notex_corpus.py --mode notex` on macos-15 ran the host
with an empty environment, the lock's bundle and the user's consent, and
sandbox-exec denying TeX Live's paths. The macOS PDFs are compared with the
Linux run (`--mode texlive`) and with xelatex's PDFs. All 26 clean cases that
both runs have are byte-identical across platforms and resolvers.

Missing from the bundle:
- **packages:** `bidi` (polyglossia's right-to-left languages), `thuthesis`,
  `zhnumber` (ctex's `\zhnumber`, xeCJK's CJKfntef example), `l3draw`,
  `showexpl`, `typogrid`, and `unicode-math.dtx` (a source file).
- **fonts:** Harano Aji, Fira Math, New Computer Modern, UnBatang.
- **TeX Gyre by family name:** the bundle's fonts are not in the font index.
  Belief, not measured: xelatex on a Mac without those fonts installed fails
  the same way, because Core Text does not see TeX Live's fonts either.

## 6. `[fonts]` (criterion 6)

The details are in criterion 6's row of the checklist above.

## Ranked failures, with repros

Each repro is a case of the corpus: `python3 tools/unicode-m3/corpus.py
assemble --out C --fonts-line BIN --only NAME`, then
`python3 tools/xetex-lockstep/latex.py --engine flashtex-xetex --cases-dir C`
or `tools/xetex-pdfparity/run.py --engine … --cases-dir C --no-visual`.

1. **Right-to-left text is laid out left to right (no ICU bidi).**
   `crates/flashtex-xetex/src/native/mod.rs` `set_native_metrics` warns and
   lays out LTR: ICU is not vendored, so `ubidi` is absent. P-T1 logs are
   equal, but the XDV differs from byte 570 (u03, Hebrew) and from byte 192
   (u04, Arabic). In the PDF, 100 of 154 Hebrew glyphs are unpaired.
   Repros: u03-polyglossia-hebrew, u04-polyglossia-arabic. Size: a lane of its
   own (vendor TeX Live 2026's ICU, port `measure_native_node`'s bidi branch
   and `\XeTeXlinebreaklocale`).
   **Fixed by lane M3-RTL** (2026-10-12, PR #1812): a port of ICU 78.2's
   bidi algorithm (`src/native/ubidi`, 2,104 vectors equal to ICU's);
   u03 and u04 equal in P-T1 and XDV.
2. **Engine panic on polyglossia's manual.**
   `flashtex-engine/src/ix.rs:44` (index out of bounds) in both passes, after
   the fontspec error for the missing Syriac font that xelatex also reports.
   xelatex exits 1 and writes 106 pages; the port exits 101 with a damaged
   PDF. The message is `index out of bounds: the len is 5000000 but the index is 18446744073441116161`: `mem[null]`, null being min_halfword −0x0FFFFFFF. The frames are `Globals::main_control` ← `tex_body` ← `driver::run`, which fits the native-word merge loop of `main_control` (`main_ppp` walking `link`s, xetex.web's "collapse native words") running off the list. Belief: the earlier divergence is the right-to-left text that is laid out left to right (the bidi warning precedes the panic). Repro: case `polyglossia`.
   **Root cause (lane M3-PANIC, 2026-10-12), not bidi:** ICU's line breaker was a stub (`linebreak_next` returned `UBRK_DONE` at once), so under `\XeTeXlinebreaklocale` `do_locale_linebreaks` appended nothing. A word merged with the native word before it then left `tail` at the flushed node, and the walk at `body_7.rs:6738` ran off the list (located with `#[track_caller]`, run 38201493482). polyglossia's Chinese sets the locale `zh` (`\textchinese{\chinesenumeral{753}}`). Fixed by `crates/flashtex-xetex/src/native/linebreak.rs`: ICU 78.2's own line-break tables (`third_party/icu`) run by a port of ICU's rule-based break iterator, ICU4X's dictionaries for Thai, Lao, Khmer and Myanmar. With it the manual runs to its end in both passes (exit 1 and 85 pages, as xelatex; the untraced log equal but for the string pool's count). Its box dumps (130,657 lines) still differ in 19 lines: box heights, depths and widths around right-to-left text (Amiri Arabic inside `\beginR`, laid out left to right: failure 1). Belief: all 19 are failure 1. **Measured after #1812:** the box dumps are equal (run 38221376131); the one difference left was at XDV byte 200172, page 7's pointer to page 6, every record equal: the two engines had loaded DejaVu Sans Mono from different files (fontconfig hands xelatex duplicates in hash order). Fixed in the oracle's environment by `tools/unicode-m3/fontconfig.py` (one file per font name, PR #1822): 49 of 49.
3. **beamer's navigation-symbol links are missing.** 63 links in xelatex's
   PDF are absent from FlashTeX's (3 pages), and FlashTeX writes 3 links
   xelatex has not. They are pgf-drawn icons inside `\hyperlink`. Repro:
   u11-beamer-fontspec.
   **Fixed by lane M3-LINKS** (2026-10-12, PR #1809). u11 now passes
   structurally, with 0 differences against the 67 recorded above. That
   makes 24 of 49 cases equal in the export run
   [38211798848](https://github.com/flash-tex/flashtex/actions/runs/38211798848);
   no other case changed. There were three causes:
   - **Phantom links were dropped.** hyperref's `hdvipdfmx` writes
     `dvipdfmx:catch_phantom`, which FlashTeX ignored. dvipdfmx widens a
     link by the kerns inside it, so 60 links had no rectangle. The 3
     footline links were 1.07 bp short.
   - **A soft mask's form was never written.** Only a named object
     (`/SMask <</G @pgfsmask5>>`) referred to it, so it was written as
     `null`.
   - **The comparator was too strict about soft masks.** It compared a
     mask's group by its content-stream tokens; it now compares what the
     group draws, as it does for other forms.
4. **The bundle lacks what the corpus reads** (criterion 5's list).
   Owner: BUNDLE-PUBLISH (`tools/bundle/tl2026/extra.txt`).
5. **`ToUnicode` for `.notdef` is U+FFFF where xelatex writes none** (Fira
   Math, u09). It is marked as measured in `out/tounicode.rs`, so the rule
   needs a second measurement.
6. **Small-capital glyphs read as PUA code points** (TeX Gyre Termes, u08).
   xelatex reads them as the letter; the same class as the known l007/l008
   differences.
7. **Fixed in this lane:** a coloured font's RGB (fontspec `Color=`) is now
   written with 3 decimals, as dvipdfmx writes it (xeCJK-example-mathblock:
   128 glyphs, 0.1412 against .141). `crates/flashtex-xetex/src/out/page.rs`.

Not FlashTeX failures, but corpus cases whose oracle errors in this
environment (left in, because the engines must still agree):
- fontspec-example: a fontspec error, a font it names is not found here;
- unimath-symbols: needs `unicode-math.dtx`;
- xeCJK-example-fallback and xeCJK-example-listings: missing fonts and
  packages;
- u04 also has a "Missing number" error in both engines.
