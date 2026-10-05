# App parity, re-audit (2026-10-05)

Lane **P5-CHECKS** item 1 (kabir-claude). This file supersedes
[app-parity-2026-10-03](../app-parity-2026-10-03/README.md). DESIGN §12 P5 decision 3
gates the switch-over on the §10 app-parity checklist: the app must lose nothing when
the old engine is deleted. Row ids (A1 … E5) are the same as in the 2026-10-03 file and
in #1337.

## Basis

- **Checked against:** `origin/main` at **`0752115a1`** (2026-10-05). It includes:
  - #1408, #1409, #1410 (links, editor marks, preview navigation);
  - #1416, #1431, #1438 (VoiceOver page text and the Pages rotor);
  - #1417 (stall bound, Stop Compile);
  - #1421, #1427 (per-document engine choice);
  - #1430 (quick fixes, Create);
  - #1432 (keystroke main thread);
  - #1434 (caret mark, page labels);
  - #1435 (math hover);
  - #1436 (`flashtex-v3` CLI);
  - #1437 (polish);
  - #1444 (V3-PACKAGE-RESOLUTION);
  - #1471, #1523 (no-TeX-Live bundle);
  - #1553 (best-effort error recovery).
- **Method:** each row was re-read in the current Swift code under
  `apps/mac/Sources/FlashTeXMac/`. Each named test was found by `grep` under
  `apps/mac/Tests/FlashTeXMacTests/` (or in Rust, where marked). Nothing was run: the
  Mac suite was not run for this audit.
- **Evidence key**, in the Evidence column:
  - **[V]** verified: the test exists on `0752115a1`, and its assertions were read in
    this audit and match the claim.
  - **[F]** found: the test exists under that name, but its assertions were not re-read
    in this audit. They were read in the 2026-10-03 audit.
  - **[B]** believed: code reading only, with no test pinning it. "(no test)" means
    the same.
- Paths are under `apps/mac/Sources/FlashTeXMac/` unless shown otherwise. Line numbers
  are at `0752115a1`.

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
| main `442ca71bb` (2026-10-03 file) | 32 | 9 | 18 | 10 |
| 2026-10-03's "after #1408 + #1409 + #1410" | 41 | 7 | 11 | 10 |
| **main `0752115a1` (this audit)** | **56** | **3** | **1** | **9** |

By section at `0752115a1`:

| Section | done | partial | missing | different |
|---|---|---|---|---|
| A (21) | 17 | 0 | 1 | 3 |
| B (13) | 12 | 0 | 0 | 1 |
| C (25) | 21 | 2 | 0 | 2 |
| D (5) | 4 | 1 | 0 | 0 |
| E (5) | 2 | 0 | 0 | 3 |
| **Total (69)** | **56** | **3** | **1** | **9** |

**What moved since 2026-10-03:**
- To **done**: A8, A15, A17, A21, B4, B5, B8, B10, B11, B12, C2, C4, C6, C7, C8, C10,
  C12, C16, C17, C18, C19, C20, C22, C24.
- **B9** moved from missing to **different**: it is absent on both paths (see the row).
- **D5** moved from different to **partial**: the successor exists, but it is not
  shipped under the old name yet.
- **A9** is still **missing**, and is now **P5 scope**: the owner ruled (2026-10-05,
  Q-new-1 of `reviews/2026-10-05.md`; relayed by the Commander, kabir-claude) that the Unicode mode (XeTeX, lane XETEX-S1)
  carries project fonts before P5, rather than retiring them.

## The DESIGN §6.2 / §10 named items

| Item | Status | Evidence |
|---|---|---|
| Export and Print through the new engine | **done** (#1343) | [V] `EngineV3ExportTests.testExportWritesTheCompressedPDFWithResolvedReferences`, `.testPrintUsesTheExportedPDF`, `.testCancelBeforeTheRunWritesNothing` |
| Hyperref link clicks | **done** (#1408) | [V] `EngineV3LinksTests.testHyperrefLinksInThePaneOpenURLsAndScrollToTargets`, plus 4 rule tests in the same file |
| VoiceOver page text and the Pages rotor | **done** (#1416, #1431, #1438) | [V] `EngineV3AccessibilityTests.testAPageReadsItsTextLineByLineAsOnV2`, `.testPagesRotorListsEveryPageAndLoadsOneNotBuilt`, `.testGlyphNamesMapByPdfTeXsTableThenTheAGLRules`. Limit: Type 3 bitmap fonts read letters and digits only |
| Scroll anchoring across resize | **done** (#1287, #1450) | [F] `EngineV3PreviewNavTests.testCollapsingAndReexpandingThePaneKeepsTheReadingPosition` (the split-resize case, untested on 2026-10-03), `.testZoomKeepsTheReadingPosition`; [V] `EngineV3ZoomTilesTests.testZoomedPaneShowsExactTilesAroundTheViewport` |
| Scroll anchoring across reflow | **done** (#1410) | [V] `EngineV3PreviewNavTests.testReadingPositionSurvivesAReflowAboveIt` |
| Main-thread work per keystroke | **done** (#1432) | 41.0 → 11.0 ms per keystroke: [keystroke-main-2026-10-03](../keystroke-main-2026-10-03/README.md). [F] `KeystrokeInvalidationTests` (7 tests), `EditorEditTailTests` (10 tests). The F3 measurement section of the 2026-10-03 file is historical |
| Forward and inverse search | **done** (#1259) | [V] `EngineV3SearchTests.testForwardAndReverseAcrossAnInput` |
| Error UI through diag-v1 | **done** (#1255, #1409, #1553) | [V] `EngineV3DiagMappingTests.testDiagsMapToExactColumns`, `EngineV3EditorMarksTests.testV3RowsDrawTheSameEditorMarksAsTheOldPath`. Since #1553 the default is best effort: TeX's recoverable errors show as warnings, "… (pdfLaTeX would report an error here)", and fatal errors stay errors ([F] `EngineV3BestEffortTests`) |
| Multi-file projects, `\include` | **done** (different mechanism, A6) | [V] `EngineV3SearchTests.testForwardAndReverseAcrossAnInput` (`\input`), `EngineV3CompileCommandTests.testOutsideChangeToAnUnopenedInputRecompiles` |
| External-viewer SyncTeX | not a gap: absent on both paths | [B] `crates/flashtex-engine/src/cli.rs:32` still lists `-synctex` as not implemented |
| Find-in-preview overlay | not a gap: absent on both paths | [B] Neither pane has `NSTextFinder` or a find bar |

## Gap table

### A. Compile triggers and projects

| # | Feature | main `0752115a1` | Evidence / test |
|---|---|---|---|
| A1 | ⌘B, the ▶ button, the palette | done | [V] `EngineV3CompileCommandTests.testCompileCommandNeverReachesTheOldWorkerUnderV3`; [F] `.testCanCompileWithoutAWorkerOnlyUnderV3` |
| A2 | One engine at a time | done | [V] `EngineV3OneEngineTests.testTurningV3OnDropsTheOldResultAndStopsOldCompiles`. [B] The direct worker still attaches at launch (`ShellModel.swift:929–932`), but under v3 it compiles nothing |
| A3 | Auto-compile setting | done | [V] `EngineV3CompileCommandTests.testAutoCompileOffWaitsForCompileCommand` |
| A4 | Unopened `\input` changed on disk | done | [V] `EngineV3CompileCommandTests.testOutsideChangeToAnUnopenedInputRecompiles` |
| A5 | New files created outside the app | done | [V] `EngineV3ToolsTests.testAQuarantinedFileAppearingInTheProjectIsCheckedBeforeTheNextCompile` (FSEvents watcher, `EngineV3ProjectWatcher.swift`) |
| A6 | `\include` / `\input` resolution | different (intended) | [B] Real TeX over a symlinked copy, capped at 20,000 entries (`EngineV3Snapshot.swift:69`, `maxEntries`) |
| A7 | Which file compiles from a chapter tab | different | The entry; else the first open document with `\documentclass` (`EngineV3Session.mainFile`, `EngineV3Session.swift:808`). [V] `EngineV3OpenTests.testOpeningAnotherFileCompilesThatFile` |
| A8 | `flashtex.toml` texinputs and packages, Fetch Missing Packages | **done** (#1421, #1444) | [V] `EngineChoiceTests.testTheNewEngineFindsTexinputsPackages` (texinputs, real host); [F] `EngineV3PackagesTests.testAPinnedPackageAndALibraryCompileUnderTheNewEngine` (pins and libraries), `EngineChoiceTests.testManifestRules`, `.testTexinputsLinksYieldToProjectFiles` |
| A9 | Project and system fonts (`[fonts]`) | **missing**; **P5 scope via the Unicode mode** (owner, 2026-10-05) | v3 is pdfLaTeX only. Since #1421, a `[fonts]` project visibly falls back to the old engine, with a banner, the status item and a VoiceOver announcement (`EngineChoice.Blocker.projectFonts`, `EngineChoice.swift:78`). [V] `EngineChoiceTests.testProjectFontsFallBackAndSayWhy`, `.testStatusItemAndBannerShowTheFallback`. The owner ruled (2026-10-05, Q-new-1) that the Unicode mode (XeTeX, lane XETEX-S1) carries `[fonts]` in P5, so this row closes when a `[fonts]` project compiles on the Unicode mode instead of falling back |
| A10 | BibTeX, biber, makeindex | done (trusted projects) | [V] `EngineV3ToolsTests.testTrustedProjectRunsBibtexAndTheCitationResolves`; [F] `.testRequestCarriesExternalToolsAndTheClientSays32` |
| A11 | Tool-run status | done | [V] `EngineV3ToolsTests.testUntrustedProjectRunsNoToolAndSaysWhy`, `.testTrustingTheProjectMidSessionRunsTheTools`, `EngineV3RunawayTests.testASupersededRunLeavesTheCurrentCyclesRowsAndNote` |
| A12 | Shell escape and project trust | done | [V] `EngineV3TrustTests.testQuarantinedProjectCompilesWithShellEscapeOffUntilTrusted`; [F] 11 more in `EngineV3TrustTests` |
| A13 | Crash recovery | done | At most 3 restarts a minute, with the pages kept stale (`EngineV3Session.swift:546–548`). [V] `EngineV3InstanceTests.testAVanishedCopyIsRecreated`. There is still no crash-limit test |
| A14 | Retry after the crash limit | done (no test) | [B] ⌘B restarts a failed host with a fresh budget (`EngineV3Session.compileNow`, `EngineV3Session.swift:790–801`). No test covers it after 3 crashes |
| A15 | Runaway compile bound | **done** (#1417) | 30 s of host silence while typing, 5 min under ⌘B; tool phases and exports are exempt. Stop Compile (⌘.) appears after 2 s. [V] `EngineV3RunawayTests.testAnEndlessLoopIsStoppedAndTheNextEditCompiles`, `.testTheBoundIs30sForTypingAnd5MinutesForCommandB`, `.testALongSilentToolPhaseIsNotStopped`, `.testStopCompileEndsARunningCompile` |
| A16 | Cancel and supersede | done | [V] `EngineV3RunawayTests.testStopCompileEndsARunningCompile` (cancel), `.testASupersededRunLeavesTheCurrentCyclesRowsAndNote` (superseded tool cycle). [B] A newer COMPILE preempts at a checkpoint |
| A17 | Works without TeX Live | **done** (#1421, #1471, #1523) | `make-app.sh` ships `tools/bundle/tl2026/flashtex-bundle.lock`, which points at a GitHub Release bundle that is downloaded after consent. Declining, or having neither TeX Live nor a bundle, falls back visibly. [V] `EngineChoiceTests.testTheShippedLockIsFoundInTheAppsResources`, `.testTheShippedLockIsAReleaseAssetBehindConsent`; [F] `.testNoTeXLiveFallsBackAndTheChoiceSaysWhy`, `.testAConfiguredBundleIsADistributionBehindConsent`; Rust [F] `crates/flashtex-engine/tests/bundle_notex.rs` `compiles_from_a_bundle_with_no_texlive_visible`. There is no app test of a real download and compile |
| A18 | Output and aux location | different | [B] `~/Library/Caches/FlashTeX/engine-v3/projects/…/out` (`EngineV3Session.swift:2135`); the manifest's `output` is ignored |
| A19 | Tabs, several open documents | done (no test) | [B] Every open buffer is sent |
| A20 | Several windows | done | One `EngineV3Session` per window (`ShellModel.swift:137`), so each window has its own host and copy. The 2026-10-03 file's "one host per app" was wrong. [V] `EngineV3InstanceTests.testTwoConcurrentSessionsKeepTheirOwnCopies` |
| A21 | Compile status and log | **done** (#1437) | **Show TeX Log** opens the last compile's `.log`. [V] `EngineV3PolishTests.testShowTeXLogFindsTheLastLog`, `EngineV3OpenTests.testErrorsReachTheProblemsPanel` |

### B. Errors and warnings

Since #1553, best effort is the default. TeX's recoverable errors arrive as warnings
marked "(pdfLaTeX would report an error here)", and a fatal error keeps the last good
pages, dimmed. Strict mode sends halt-on-error ([F] `EngineV3BestEffortTests`, 6 tests;
[V] the best-effort branch of `EngineV3DiagMappingTests.testDiagsMapToExactColumns`).

| # | Feature | main `0752115a1` | Evidence / test |
|---|---|---|---|
| B1 | Problems panel | done | [V] `EngineV3OpenTests.testErrorsReachTheProblemsPanel` (now asserts severity `.warning` for an undefined control sequence, per #1553) |
| B2 | Exact spans (diag-v1) | done | [V] `EngineV3DiagMappingTests.testDiagsMapToExactColumns`; Rust `corpus_positions_equal_pdflatex` [F] |
| B3 | Located warnings, overfull and underfull boxes | done | [B] diag-v1 sends boxes as `warning`; only tight and loose (`info`) are dropped. [V] `EngineV3DiagMappingTests.testDiagsMapToExactColumns` asserts that `info` is dropped. No Mac test shows a host-sent box warning |
| B4 | Underlines, gutter, Error Lens | **done** (#1409) | [V] `EngineV3EditorMarksTests.testV3RowsDrawTheSameEditorMarksAsTheOldPath`; [F] `.testHostErrorIsUnderlinedAndAnnounced` |
| B5 | Next and previous diagnostic (⌘⇧] / ⌘⇧[) | **done** (#1409) | [V] `EngineV3EditorMarksTests.testNextAndPreviousDiagnosticStepThroughV3MarksAsOnTheOldPath` |
| B6 | Jump from a panel row | done | [V] `EngineV3OneEngineTests.testV3CompiledTextsAreTheNavigationBaselineOnlyUnderV3` |
| B7 | Line label, Copy as Text | done | [V] The same baseline and test |
| B8 | Panel status line | **done** (#1437) | A compile that wrote no page shows "compile failed: the previous preview is kept". [V] `EngineV3PolishTests.testACompileWithNoPageSaysSoInProblems`, `.testAnotherProjectDropsTheLastOnesFailedStatus` |
| B9 | Explanations (`flashtex-explain`) | **different** (absent on both paths) | [B] Neither `flashtex-explain` nor its catalogue is on main. `ExplanationClient.locate()` (`EditorDiagnostics.swift:816`) finds nothing in a main build, so v2 shows no explanations either (#1430). Explanations for TeX's own messages would be a new feature for the owner to decide |
| B10 | Quick fix, Tab fix | **done** (#1430) | [V] `EngineV3FixesTests.testAMistypedCommandIsFixedWithTabUnderV3`; [F] `.testDidYouMeanFollowsTheOldCompilersRule`, `.testCaretFixAndFixPreviewMatchTheOldPath` |
| B11 | Create a missing include, Fetch a package | **done** (#1430, #1444) | [V] `EngineV3FixesTests.testAMissingInputOffersCreateUnderV3`, `.testAMissingPackageOffersCreateUnderV3`; [F] `.testAMissingClassOffersCreateClassUnderV3`, `EngineV3PackagesTests.testFetchIsOfferedAndFetchesIntoTheCacheUnderTheNewEngine`, `.testNothingIsSentToTheSourceBeforeConsent` |
| B12 | VoiceOver "N errors, M warnings" | **done** (#1409) | [V] `EngineV3EditorMarksTests.testV3CompileAnnouncesChangedCountsLikeTheOldPath`; [F] `.testHostErrorIsUnderlinedAndAnnounced` |
| B13 | First error in the HUD and pane | done | [V] `EngineV3OpenTests.testErrorsReachTheProblemsPanel`; [V] `EngineV3RunawayTests.testAnEndlessLoopIsStoppedAndTheNextEditCompiles` asserts `firstError`. Under best effort, a recoverable error is shown as a warning |

### C. Preview pane

| # | Feature | main `0752115a1` | Evidence / test |
|---|---|---|---|
| C1 | Zoom commands, saved zoom | done | [V] `EngineV3ZoomTilesTests.testZoomedPaneShowsExactTilesAroundTheViewport`; [F] `ZoomTests.testMenuCommandsChangeTheModelAndFitWidthRestoresOne` |
| C2 | Fit Page (⌘⇧9) | **done** (#1410) | [F] `EngineV3PreviewNavTests.testFitPageFitsTheTallestPageToThePane`, `.testFitPageFitsTheTallestPageWithLegacyScrollers` (both through the `fitPage` helper, which was not re-read) |
| C3 | Pinch | done | [V] `EngineV3ZoomTilesTests.testZoomedPaneShowsExactTilesAroundTheViewport`; [F] `EngineV3PreviewNavTests.testAPinchAnchoredInTheLeftMarginKeepsItsDistance` |
| C4 | Double-click for Fit Width | **done** (#1437) | [V] `EngineV3PolishTests.testDoubleClickFitsTheWidthAsOnV2`, which sends a real 2-click event through the window |
| C5 | Fit width capped at 100 % | different (intended) | [B] v3 is uncapped, which DESIGN §6.2's scale sweep assumes; horizontal scrolling is on |
| C6 | Page Up / Page Down | **done** (#1410) | [V] `EngineV3PreviewNavTests.testPageUpAndDownStepWholePagesAsOnV2` |
| C7 | Reading position across resize and reflow | **done** (#1410, #1450) | [V] `EngineV3PreviewNavTests.testReadingPositionSurvivesAReflowAboveIt`; [F] `.testCollapsingAndReexpandingThePaneKeepsTheReadingPosition` |
| C8 | Hyperref links, hand cursor, tooltip | **done** (#1408) | [V] `EngineV3LinksTests.testHyperrefLinksInThePaneOpenURLsAndScrollToTargets`; [F] 4 rule tests |
| C9 | "N / M" page readout | done (no test) | [B] `ContentView.swift:470–473` reads `engineV3.pageCount` |
| C10 | Dark preview | **done** (#1410) | [V] `EngineV3PreviewNavTests.testDarkPreviewDarkensTheGroundAsOnV2` |
| C11 | Smooth fonts | done | [F] `PreviewFontSmoothingTests`, `EngineV3SmoothingRaceTests` |
| C12 | Persistent caret mark and paragraph band on the page | **done** (#1434, #1460) | [V] `EngineV3CaretMarkTests.testTheCaretIsMarkedOnThePageAndFollows`; [F] 14 more in `EngineV3CaretMarkTests` |
| C13 | Preview follows the caret | done | [V] `EngineV3SearchTests.testForwardAndReverseAcrossAnInput` |
| C14 | Forward search (⌘⇧J) | done | [V] The same test. [B] Known limit (#1435): with the caret inside an inline formula, it lands on the glyph before the formula, because TeX gives the formula the closing `$`'s column |
| C15 | Reverse search (click) | done | [V] The same test |
| C16 | Hover highlight and source tooltip | **done** (#1408, #1437) | [V] `EngineV3PolishTests.testHoveringTextNamesItsSource` ("main.tex, line 3, column 12 (click to go there)") |
| C17 | "page N" label on each page | **done** (#1434) | [V] `EngineV3CaretMarkTests.testEachPageIsLabelled` |
| C18 | VoiceOver page text (lines, "Go to source") | **done** (#1416, #1431, #1438) | [V] `EngineV3AccessibilityTests.testAPageReadsItsTextLineByLineAsOnV2`; [F] `.testOT1AccentsReadAsAccentedLetters`, `.testT1PrecomposedGlyphsReadTheSame`. Limit: Type 3 fonts read letters and digits only |
| C19 | VoiceOver Pages rotor | **done** (#1416) | [V] `EngineV3AccessibilityTests.testPagesRotorListsEveryPageAndLoadsOneNotBuilt` |
| C20 | Pane accessibility value; page-jump announcement | **done** (#1410) | [B] Value: `ContentView.swift:379`. [V] Announcement: `EngineV3PreviewNavTests.testPageUpAndDownStepWholePagesAsOnV2` ("Page 2 of 4") |
| C21 | Status chips, HUD tooltip | **partial** | Fixed: the preview HUD tooltip names the v3 engine (`ContentView.swift:489`), and the v3 status card has Stop Compile (#1417). Still old: the title bar's Compile tooltip still says "producer: " + `producerSummary`, the old worker's summary (`TitleBar.swift:131`). [B] No test pins either tooltip |
| C22 | Latency readout | **done** (#1437) | [V] `EngineV3PolishTests.testTheLatencyReadoutIsV3s`, `.testTheLatencyRecordIsBounded` |
| C23 | Stale-page look | different (intended) | [B] Dimmed, with an orange border |
| C24 | Math hover preview | **done** (#1435) | [V] `EngineV3MathHoverTests.testHoveringAFormulaShowsItsCropFromThePage` |
| C25 | Preview Debug Status | **partial** | [B] Developer-facing. Under v3 the toggle shows only the session's environment note (`EngineV3Preview.swift:102`), not v2's debug strip |

### D. Export, print, CLI

| # | Feature | main `0752115a1` | Evidence / test |
|---|---|---|---|
| D1 | Export PDF (menu, toolbar, palette) | done | [V] `EngineV3ExportTests.testExportWritesTheCompressedPDFWithResolvedReferences`; [F] `.testNoHostRefusesExportAndPrintWithTheV3Reason` |
| D2 | Export session: atomic write, overwrite conflict, cancel | done | [V] Cancel: `EngineV3ExportTests.testCancelBeforeTheRunWritesNothing`. The overwrite-conflict check is shared code but is still tested only on the old path ([F] `ExportSessionTests.testDestinationChangedSinceChosenIsRefusedBeforeLaunch`) |
| D3 | Print | done | [V] `EngineV3ExportTests.testPrintUsesTheExportedPDF` (checks the page count, not the bytes) |
| D4 | Print Source | done | [F] Engine-independent (`PrintControllerTests`) |
| D5 | `flashtex build/check/watch` CLI | **partial** (#1436) | **Built:** `crates/flashtex-build` builds the binary `flashtex-v3` (MIT, drives `flashtex-host` over display-list-v3). **Not shipped:** naming (it becomes `flashtex` when `crates/flashtex-cli` is deleted) and release packaging are RQ6. `crates/flashtex-cli` is still on main. Rust [F] `crates/flashtex-build/tests/cli.rs`: `build_writes_the_pdf_pdflatex_writes`, `build_runs_bibtex_as_latexmk_would`, `check_reports_errors_with_their_place_and_fails` (they skip without a host and TeX Live) |

### E. Companion, capture, history

| # | Feature | main `0752115a1` | Evidence / test |
|---|---|---|---|
| E1 | iPad Nearby pairing and capture inbox | done | [F] Engine-independent (`Nearby*Tests`) |
| E2 | Capture destination `projectId` | different | [B] `ShellModel.projectId` is still `result?.projectId ?? "demo"` (`ShellModel.swift:669`), so under v3 it is always `"demo"`. The old path's request reads the same property (`ShellModel.swift:1503`) |
| E3 | Capture proposal preview | different (intended) | [V] Refused under v3: `EngineV3OneEngineTests.testCaptureReviewDoesNotCompileWithTheOldEngineUnderV3` |
| E4 | Bridge anchors follow typing | done (no test) | [B] `bridgeTextChanged` (`ShellModel+Bridge.swift:210`) runs on every edit |
| E5 | Durable History | different (paused under v3) | [V] `EngineV3OneEngineTests.testTheDurableHelperIsSetAsideUnderV3AndComesBack` |

### F. Settings and engine choice

- **Per-document engine choice exists** (#1421, #1427):
  - the status bar's engine item;
  - *View ▸ Engine for This Document* (`FlashTeXMacApp.swift:193`);
  - *Settings ▸ Compile ▸ Engine for other documents*.
  - [F] `EngineChoiceTests`, 29 tests, among them
    `.testResolutionOrderEnvironmentWindowUserSettingRecordDefault` and
    `.testFlippingTheDefaultKeepsTypesetDocumentsOnTheirEngine`.
- **The default is not flipped:** `EngineChoice.defaultForNewDocuments` is still
  `.previous` (`EngineChoice.swift:130`). Flipping it is the S5 gate and an owner
  decision. It is not a parity row.
- **Remaining fallbacks:**
  - `[fonts]` (A9);
  - no TeX Live and no bundle;
  - a declined bundle download (`EngineChoice.swift:68–81`).

## Remaining P5 app-parity work

These are the 4 rows still missing or partial, ranked by user impact, highest first:

1. **A9, project fonts (`[fonts]`): missing; P5 scope via the Unicode mode (owner,
   2026-10-05).** Every `[fonts]` project stays on the old engine (with a visible
   banner), so it blocks deleting the old engine for those users until the Unicode
   mode (XeTeX, lane XETEX-S1) compiles them.
2. **D5, `flashtex build/check/watch` successor: partial.** `flashtex-v3` works, but
   the shipped `flashtex` CLI still links the old engine. The rename and packaging
   (RQ6) are needed before the old crates are deleted.
3. **C21, title bar Compile tooltip: partial.** Under v3 the tooltip still names the
   old worker as "producer", which misleads the user about which engine typeset the
   page. It is a one-line fix in `TitleBar.swift:131`.
4. **C25, Preview Debug Status: partial.** This is developer-facing only. The v3 pane
   has no debug strip like v2's, and it does not affect users.

**Done, but no test pins it** (coverage, not parity):
- A13: the 3-a-minute crash limit;
- A14: ⌘B after the crash limit;
- A19: several open buffers sent;
- C9: the "N / M" readout;
- C21: either tooltip;
- E4: bridge anchors;
- D2: the overwrite conflict under v3;
- B3: a host-sent box warning.

## Could not verify

- No test was run, by instruction: this was a light Mac check. A row marked [V] means
  its assertions were read, not that the test passes on `0752115a1`.
- [F] rows were found by name only. The C2 Fit Page tests and the C7 collapse test go
  through helpers that were not re-read.
- **A17:** the real download from the GitHub Release asset is not exercised by any app
  test. The Rust `bundle_notex.rs` test builds its own bundle from the local TeX Live.
