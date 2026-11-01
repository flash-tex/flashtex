# App parity, current gap list (2026-10-03)

> **Superseded** by [app-parity-2026-10-05](../app-parity-2026-10-05/README.md), a re-audit
> against `origin/main` `0752115a1` with the same row ids. This file is kept as the record at `442ca71bb`.

Lane **P5-APP-PARITY-2** (mac-claude-a). DESIGN §12 P5 decision 3 gates the switch-over
on the §10 app-parity checklist: the app loses nothing when the old engine is deleted.
This file is the current state of that checklist.

## Basis

- **Rows:** the 69 rows of #1337's audit (`docs/evidence/app-parity-gap-2026-10-01.md`
  on that branch, against `d8c3009d9`). Row ids (A1 … E5) are #1337's, so the two can
  be read side by side.
- **Checked against:** `origin/main` at `442ca71bb`, after #1340–#1344 (one engine at
  a time, ⌘B and auto-compile, export and print, bibliography and index), #1332
  (trust, stored-page reopen), #1255 (diag-v1) and #1287/#1395 (zoom, pinch, tiles).
  Each row was re-read in the code by four read-only explorers, one per section.
  Load-bearing claims were then re-checked by hand.
- **This lane's PRs:** #1408, #1409 and #1410 (below). The "after" column assumes they land.
- **Verified vs believed:**
  - A row marked **done** names the test that proves it.
  - "(no test)" means the code does it, but no test pins it.
  - Rows without a test are code reading, not observation.

**Status values:**

| Status | Meaning |
|---|---|
| **done** | The feature is in v3 |
| **partial** | v3 has some of the feature |
| **missing** | v3 lacks the feature |
| **different** | v3 does something else on purpose, or the row is not a v3 gap |

## Counts

| | done | partial | missing | different |
|---|---|---|---|---|
| #1337 at `d8c3009d9` | 14 | 9 | 29 | 17 |
| **main `442ca71bb`** | **32** | **9** | **18** | **10** |
| after #1408 + #1409 + #1410 | 41 | 7 | 11 | 10 |

## The DESIGN §6.2 / §10 named items

| Item | Status | Evidence |
|---|---|---|
| Export and Print through the new engine | **done** (#1343) | `EngineV3ExportTests.testExportWritesTheCompressedPDFWithResolvedReferences`, `.testPrintUsesTheExportedPDF`, `.testCancelBeforeTheRunWritesNothing` |
| Hyperref link clicks | **done in #1408** | `EngineV3LinksTests` (5 tests, one in a hosted pane) |
| VoiceOver page text and the Pages rotor | **missing** (C18, C19) | No v3 text exists: `DL3SourceIndex` has no characters. See "What is left" |
| Scroll anchoring across resize | **done** (#1287) | `relayout(anchor:)` keeps the page point on a scale change. `EngineV3ZoomTilesTests.testZoomedPaneShowsExactTilesAroundTheViewport` pins the pinch case; the split-resize case has no test |
| Scroll anchoring across reflow | **done in #1410** | `EngineV3PreviewNavTests.testReadingPositionSurvivesAReflowAboveIt`, which fails with the old rule (y 201 → 1.8 pt) |
| Main-thread work per keystroke | **done** (P5-KEYSTROKE-MAIN) | 41.0 → 11.0 ms per keystroke at 30 ms typing, 96 % → 35 % busy: [keystroke-main-2026-10-03](../keystroke-main-2026-10-03/README.md). `KeystrokeInvalidationTests`, `EditorEditTailTests` |
| Forward and inverse search | **done** (#1259) | `EngineV3SearchTests.testForwardAndReverseAcrossAnInput` |
| Error UI through diag-v1 | **done** (#1255), editor marks **in #1409** | `EngineV3DiagMappingTests.testDiagsMapToExactColumns`; `EngineV3EditorMarksTests` |
| Multi-file projects, `\include` | **done** (different mechanism, A6) | `EngineV3SearchTests.testForwardAndReverseAcrossAnInput` (`\input`), `EngineV3CompileCommandTests.testOutsideChangeToAnUnopenedInputRecompiles` |
| External-viewer SyncTeX | not a gap: absent on both paths | `crates/flashtex-engine/src/cli.rs` refuses `-synctex`; the old engine never wrote one |
| Find-in-preview overlay | not a gap: absent on both paths | Neither pane has a find bar, `NSTextFinder` or text selection |

## Gap table

Paths are under `apps/mac/Sources/FlashTeXMac/` unless shown otherwise. Tests are in
`apps/mac/Tests/FlashTeXMacTests/`.

### A. Compile triggers and projects

| # | Feature | main `442ca71bb` | Evidence / test |
|---|---|---|---|
| A1 | ⌘B, the ▶ button, the palette | done | `EngineV3CompileCommandTests.testCompileCommandNeverReachesTheOldWorkerUnderV3`, `.testCanCompileWithoutAWorkerOnlyUnderV3` |
| A2 | One engine at a time | done | `EngineV3OneEngineTests.testTurningV3OnDropsTheOldResultAndStopsOldCompiles`. The worker process still attaches at launch (`ShellModel.swift:832`) but compiles nothing |
| A3 | Auto-compile setting | done | `EngineV3CompileCommandTests.testAutoCompileOffWaitsForCompileCommand` |
| A4 | Unopened `\input` changed on disk | done | `EngineV3CompileCommandTests.testOutsideChangeToAnUnopenedInputRecompiles` |
| A5 | New files created outside the app | done | `EngineV3ToolsTests.testAQuarantinedFileAppearingInTheProjectIsCheckedBeforeTheNextCompile` (FSEvents watcher) |
| A6 | `\include` / `\input` resolution | different (intended) | Real TeX over a symlinked copy, capped at 20,000 entries (`EngineV3Snapshot.maxEntries`) |
| A7 | Which file compiles from a chapter tab | different | The entry, else the first open document with `\documentclass` (`EngineV3Session.swift:489`). `EngineV3OpenTests.testOpeningAnotherFileCompilesThatFile` |
| A8 | `flashtex.toml` texinputs and packages, Fetch Missing Packages | **missing** | Nothing in `EngineV3*`. `DL3CompileRequest` has no field for them. *Later:* texinputs in #1421; packages, `[packages] pin` and `path` (no fallback) in V3-PACKAGE-RESOLUTION: `EngineV3PackagesTests.testAPinnedPackageAndALibraryCompileUnderTheNewEngine`, `EngineChoiceTests.testManifestRules` |
| A9 | Project and system fonts (`[fonts]`) | **missing** | v3 is pdfLaTeX only (`DL3Connection.swift:19`); needs the per-document fallback |
| A10 | BibTeX, biber, makeindex | done (trusted projects) | `EngineV3ToolsTests.testTrustedProjectRunsBibtexAndTheCitationResolves`, `.testRequestCarriesExternalToolsAndTheClientSays32` |
| A11 | Tool-run status | done | `EngineV3ToolsTests.testUntrustedProjectRunsNoToolAndSaysWhy`, `.testTrustingTheProjectMidSessionRunsTheTools` |
| A12 | Shell escape and project trust | done | `EngineV3TrustTests.testQuarantinedProjectCompilesWithShellEscapeOffUntilTrusted` and 11 more |
| A13 | Crash recovery | done | 3 restarts a minute with pages kept stale (`EngineV3Session.swift:317`). `EngineV3InstanceTests.testAVanishedCopyIsRecreated`; no crash-limit test |
| A14 | Retry after the crash limit | done (no test) | ⌘B restarts the host (`EngineV3Session.swift:481`); nothing tests it after 3 crashes |
| A15 | Runaway compile bound | **missing** | No `--budget` or `--tool-timeout` on the host (`EngineV3Host.swift:182`); only a newer edit preempts |
| A16 | Cancel and supersede | done (no test) | A newer COMPILE preempts; a cancelled DONE is handled |
| A17 | Works without TeX Live | different | D12: the format is built from the user's TeX Live; needs the per-document fallback |
| A18 | Output and aux location | different | `~/Library/Caches/FlashTeX/engine-v3/projects/…/out`; the manifest's `output` is ignored |
| A19 | Tabs, several open documents | done (no test) | Every open buffer is sent |
| A20 | Several windows | done (same limit as v2) | One host per app; the docs were fixed |
| A21 | Compile status and log | partial | Status line, first error and tool note are shown (`EngineV3OpenTests.testErrorsReachTheProblemsPanel`); no way to open TeX's `.log` |

### B. Errors and warnings

| # | Feature | main | after this lane | Evidence / test |
|---|---|---|---|---|
| B1 | Problems panel | done | done | `EngineV3OpenTests.testErrorsReachTheProblemsPanel` |
| B2 | Exact spans (diag-v1) | done | done | `EngineV3DiagMappingTests.testDiagsMapToExactColumns`; Rust `corpus_positions_equal_pdflatex` |
| B3 | Located warnings, overfull and underfull boxes | done | done | diag-v1 sends boxes as `warning`; only tight and loose (`info`) are dropped, on purpose. #1337 was wrong here. No Mac test shows a box warning |
| B4 | Underlines, gutter, Error Lens | **missing** (none at all under v3 since #1340) | **done in #1409** | `EngineV3EditorMarksTests.testV3RowsDrawTheSameEditorMarksAsTheOldPath`, `.testHostErrorIsUnderlinedAndAnnounced` |
| B5 | Next and previous diagnostic (⌘⇧] / ⌘⇧[) | **missing** | **done in #1409** | `EngineV3EditorMarksTests.testNextAndPreviousDiagnosticStepThroughV3MarksAsOnTheOldPath` |
| B6 | Jump from a panel row | done | done | v3 texts are the baseline at DONE: `EngineV3OneEngineTests.testV3CompiledTextsAreTheNavigationBaselineOnlyUnderV3` |
| B7 | Line label, Copy as Text | done | done | Same baseline and test |
| B8 | Panel status line | partial | partial | No stale note now, but a failed v3 compile shows only in the pane, not in the panel's status line |
| B9 | Explanations (`flashtex-explain`) | **missing** | missing | Never fetched for v3 (`ShellModel.swift:760`). No longer cross-wired: `resultID` is nil under v3 |
| B10 | Quick fix, Tab fix | **missing** | missing | diag-v1 help is text only; `caretFix` needs an old result |
| B11 | Create a missing include, Fetch a package | **missing** | missing | Only the old compiler's wording is matched, not TeX's ``File `x.sty' not found``. *Later:* Fetch under v3 in V3-PACKAGE-RESOLUTION: `EngineV3PackagesTests.testFetchIsOfferedAndFetchesIntoTheCacheUnderTheNewEngine` (Create: #1430) |
| B12 | VoiceOver "N errors, M warnings" | **missing** | **done in #1409** | `EngineV3EditorMarksTests.testV3CompileAnnouncesChangedCountsLikeTheOldPath`, `.testHostErrorIsUnderlinedAndAnnounced` |
| B13 | First error in the HUD and pane | done | done | `EngineV3OpenTests.testErrorsReachTheProblemsPanel` |

### C. Preview pane

| # | Feature | main | after this lane | Evidence / test |
|---|---|---|---|---|
| C1 | Zoom commands, saved zoom | done | done | `EngineV3ZoomTilesTests.testZoomedPaneShowsExactTilesAroundTheViewport`; `ZoomTests.testMenuCommandsChangeTheModelAndFitWidthRestoresOne` |
| C2 | Fit Page (⌘⇧9) | **missing** (acted as Fit Width) | **done in #1410** | `EngineV3PreviewNavTests.testFitPageFitsTheTallestPageToThePane` |
| C3 | Pinch | done | done | `EngineV3ZoomTilesTests.testZoomedPaneShowsExactTilesAroundTheViewport` |
| C4 | Double-click for Fit Width | partial | partial | Each click also reverse-searches; whether the SwiftUI double-tap reaches the pane is unverified |
| C5 | Fit width capped at 100 % | different | different (intended) | v3 is uncapped, which DESIGN §6.2's scale sweep assumes (about 2.28 px/pt by default); horizontal scrolling is on |
| C6 | Page Up / Page Down | **missing** | **done in #1410** | `EngineV3PreviewNavTests.testPageUpAndDownStepWholePagesAsOnV2` |
| C7 | Reading position across resize and reflow | partial (scale changes only) | **done in #1410** | `EngineV3PreviewNavTests.testReadingPositionSurvivesAReflowAboveIt` |
| C8 | Hyperref links (internal and external), hand cursor, tooltip | **missing** (a click reverse-searched) | **done in #1408** | `EngineV3LinksTests.testHyperrefLinksInThePaneOpenURLsAndScrollToTargets` and 4 rule tests against `DisplayListLinks` |
| C9 | "N / M" page readout | done (no test) | done | `ContentView.swift:456` reads `engineV3.pageCount` |
| C10 | Dark preview | partial (light ground) | **done in #1410** | `EngineV3PreviewNavTests.testDarkPreviewDarkensTheGroundAsOnV2` |
| C11 | Smooth fonts | done | done | `PreviewFontSmoothingTests`, `EngineV3SmoothingRaceTests` |
| C12 | Persistent caret mark and paragraph band on the page | **missing** | missing | Only the flash on an explicit forward search |
| C13 | Preview follows the caret | done | done | `EngineV3SearchTests.testForwardAndReverseAcrossAnInput` (the follower's request) |
| C14 | Forward search (⌘⇧J) | done | done | Same test |
| C15 | Reverse search (click) | done | done | Same test |
| C16 | Hover highlight and source tooltip | **missing** | partial | Links get the hand cursor and tooltip in #1408; non-link hover (the source tooltip) is still missing |
| C17 | "page N" label on each page | **missing** | missing | Cosmetic |
| C18 | VoiceOver page text (lines, "Go to source") | **missing** | missing | Each page is an image labelled "Page N" (`EngineV3Preview.swift:563`) |
| C19 | VoiceOver Pages rotor | **missing** | missing | No rotor on the v3 pane |
| C20 | Pane accessibility value; page-jump announcement | partial (value right, no announcement) | done | Value: `ContentView.swift:365`. Announcement: `EngineV3PreviewNavTests.testPageUpAndDownStepWholePagesAsOnV2` |
| C21 | Status chips, HUD tooltip | partial | partial | Two status cards; the tooltip still shows `producerSummary` |
| C22 | Latency readout | partial | partial | No longer wrong (nil under v3), but v3's own latency isn't shown |
| C23 | Stale-page look | different (intended) | different | Dimmed, with an orange border |
| C24 | Math hover preview | **missing** | missing | No v3 source. `EngineV3OneEngineTests.testMathHoverHasNoOldEngineFrameUnderV3` proves it is absent, not wrong |
| C25 | Preview Debug Status | partial | partial | Developer-facing |

### D. Export, print, CLI

| # | Feature | main | Evidence / test |
|---|---|---|---|
| D1 | Export PDF (menu, toolbar, palette) | done | `EngineV3ExportTests.testExportWritesTheCompressedPDFWithResolvedReferences`, `.testNoHostRefusesExportAndPrintWithTheV3Reason` |
| D2 | Export session: atomic write, overwrite conflict, cancel | done | Atomic write and cancel are tested (`.testCancelBeforeTheRunWritesNothing`). The overwrite-conflict check is shared code but is tested only on the old path (`ExportSessionTests.testDestinationChangedSinceChosenIsRefusedBeforeLaunch`) |
| D3 | Print | done | `EngineV3ExportTests.testPrintUsesTheExportedPDF`, which checks the page count, not the bytes |
| D4 | Print Source | done | Engine-independent (`PrintControllerTests`) |
| D5 | `flashtex build/check/watch` CLI | different | The CLI links only the old engine; it needs a successor over `flashtex-host` before the old crates are deleted |

### E. Companion, capture, history

| # | Feature | main | Evidence / test |
|---|---|---|---|
| E1 | iPad Nearby pairing and capture inbox | done | Engine-independent (`Nearby*Tests`) |
| E2 | Capture destination `projectId` | different | `ShellModel.projectId` is `result?.projectId ?? "demo"`, so under v3 it is always `"demo"`. I believe (not verified) the old path also sent and echoed `"demo"`, because the request's id comes from the same property |
| E3 | Capture proposal preview | different (intended) | Refused under v3: `EngineV3OneEngineTests.testCaptureReviewDoesNotCompileWithTheOldEngineUnderV3` |
| E4 | Bridge anchors follow typing | done (no test) | `bridgeTextChanged` runs on every edit |
| E5 | Durable History | different (paused under v3) | `EngineV3OneEngineTests.testTheDurableHelperIsSetAsideUnderV3AndComesBack` |

### F. Settings and engine choice

- Honoured by v3 now: auto-compile, zoom, Fit Width, Actual Size and (after #1410) Fit Page.
- Error Lens is honoured after #1409.
- **Still old-only:** project fonts, packages and texinputs (A8, A9). *Later:* texinputs (#1421) and packages (V3-PACKAGE-RESOLUTION) are the new engine's too; fonts (A9) still fall back.
- **There is no per-document engine choice and no Settings entry.** The only switch is View ▸ Engine v3 Preview, app-wide (`FlashTeXMacApp.swift:165`). P5's "default per document" needs both.

## Main thread per keystroke (measured)

- **Cell:** release `FlashTeXMac` built from this lane's tree. The engine-v3 bench was
  `FLASHTEX_V3_BENCH`, typing 200 keys every 30 ms into `plain-10`. The document came
  from `docs/evidence/app-perf-2026-09-30/scripts/gen.py`.
- **Profiling:** the Time Profiler was attached with `xcrun xctrace`, and the trace was
  summarised with that directory's `mainthread.py`.
- **Machine:** mac-m1max-a. **The load average was 155 before the run and 233 after**,
  so every number below is inflated by contention.

| Measure | v3, 10 pages |
|---|---|
| Main thread, whole window | 8,149 ms in 8.9 s (**91 % busy**) |
| Per keystroke | **40.7 ms** |
| `CA::Transaction::commit` | 20.4 ms/key |
| `GraphHost.flushTransactions` (SwiftUI) | 12.3 ms/key |
| `NSHostingView.layout()` | 4.9 ms/key |
| `EngineV3*` frames | 2.8 ms/key |
| `textDidChange` | 0.6 ms/key |
| Key → first changed page committed | p50 192 ms, p95 582 ms; 83 of 200 keys sampled, the rest coalesced |

**Reading.** The split matches #1252 F3. Per keystroke, most main-thread time goes to
SwiftUI's transaction flush and the Core Animation commit of the hosting tree, not to
the v3 pane: its own frames take 2.8 ms of 40.7. #1340 removed the old worker's compile
under v3, but that was not the expensive part. **The F3 fix is still unowned:** stop
the per-keystroke invalidation of observed state, for example
`documents → ContentView`, the status items and the menus. Re-measure at normal load
before quoting an absolute figure; the 12–26 ms in DESIGN §10 was measured editor-only
at load 41–45.

## This lane's PRs

| PR | Rows | Named tests |
|---|---|---|
| #1408 | C8, C16 (links) | `EngineV3LinksTests` (5) |
| #1409 | B4, B5, B12 | `EngineV3EditorMarksTests` (4) |
| #1410 (stacked on #1408) | C2, C6, C7, C10, C20 | `EngineV3PreviewNavTests` (4) |

## What is left before the §10 checklist is closed

These are the 11 missing rows, ordered by user impact, plus the unowned F3 work.

1. **VoiceOver page text and the Pages rotor (C18, C19). L.**
   - v3 carries glyph codes, not text. A page's text needs either glyph name → Unicode
     on the app side (the Type 1 encodings the renderer already reads, plus the Adobe
     Glyph List) or text from the host (ToUnicode or `\pdfgentounicode`).
   - Then add v2's line grouping (`V2PageText`) and a rotor over `EngineV3PagesView`'s
     frames.
   - This is the largest remaining item and an accessibility regression at switch-over.
2. **Main-thread work per keystroke (F3).** Measured above; owner needed.
3. **Per-document engine choice with a fallback (F, A8, A9, A17).** It is what "new
   engine default per document" literally needs, and it covers fonts, texinputs and
   no-TeX-Live.
4. **Explanations, quick fixes and missing-file fixes (B9, B10, B11).**
5. **Runaway compile bound (A15).** Pass `--budget` and `--tool-timeout`.
6. **Caret mark on the page and math hover (C12, C24); page labels (C17).**
7. **Polish:** status chips and tooltip (C21), latency readout (C22), log access (A21),
   panel status line (B8), double-click (C4), source hover (C16).
8. **Before deleting the old crates:** a `flashtex build` successor (D5).

## About #1337

#1337's audit was accurate for `d8c3009d9` and its row ids are used here. Since then,
#1340–#1344, #1332, #1255 and #1287 changed about 25 of its rows. Three of its claims
were wrong when written:
- B3: box warnings are `warning`, not `info`.
- The magic-comments line: `% !TEX program` is parsed in
  `crates/unicode-tex/src/detect.rs`.
- E2: it said destinations "may" be mis-keyed; they are certainly `"demo"`.

**Recommendation:** close #1337 without landing it. Its content is folded in here with
current status and tests. Landing a stale table next to this one would give two
conflicting gap lists.
