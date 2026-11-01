# Nightly design review 2026-09-30, Track 2: the Mac app, preview, editing UX, maintainability

- **Reviewer:** kabir-claude, task NIGHTLY-REVIEW-T2 (mac-m5pro-kabir). Research only; no
  product code changed.
- **Governing text:** DESIGN.md §1, §3, §6, §12, §15 on `origin/main` 02dcf9d07, and §16 from
  `agent/kabir-claude/design-xplat` 82ef5ac40 (PR #1245; not on main yet). The §14 process
  steps 1–4 are covered for the app track.
- **Inputs read:**

  | input | ref |
  |---|---|
  | `apps/mac` on main | 02dcf9d07 |
  | #1247 P3-APP-V3 | `agent/kabir-claude/app-v3` c6c026f95 |
  | #1254 P3-APP-V3 follow-up | `agent/kabir-claude/app-v3-2` db0dd708b |
  | #1252 APP-PERF-AUDIT | `agent/kabir-claude/app-perf` 16866b974 |
  | #1228 tiles | `agent/mac-claude-a/preview-tiles` 0e2fdc358 |
  | Typst track B (`track-b-ux.md`) | `agent/kabir-claude/typst-design` 71b8c1977 |
  | #1236 P5 retirement plan | `agent/flashtex-2a/retirement-plan` 950f7fa2f |
  | #2 | lane assignments to 05:33Z |
  | CI merge-group run 36670850148 | the `mac app` job and its swift-test log |

- **CPU rule.** This Mac is the owner's laptop. I ran no build, benchmark or app instance,
  and did not touch `~/Library/Caches/FlashTeX`. Every number is from code (`wc -l`,
  `grep`), from committed evidence, or from the CI log above.
- **Labels.** **VERIFIED** means read in code, CI or committed evidence (file:line given).
  **BELIEF** means inference or estimate, and each one says how to check it.
- **Line references.** `EngineV3*.swift`, `DL3*.swift` and `FlashTeXPreviewV3` refer to
  `app-v3-2` db0dd708b. Every other file:line refers to main 02dcf9d07.

---

## 0. Top findings, ranked

"Class" follows §14 step 5. *Refinement* means the Commander can adopt it with evidence.
*Owner* means it changes a goal, a phase gate or a numbered decision.

| # | Finding | Evidence | Proposed action | Class |
|---|---|---|---|---|
| 1 | **The P5 switch-over would regress the app, and no gate would catch it.** The v3 pane has none of these: export and print, SyncTeX, zoom, dark preview, VoiceOver page text, link clicks, scroll anchoring, `.aux` reruns or BibTeX. P5's exit gate (§12) and the retirement plan's S5 gate are typesetting plus latency only. | §2 table; `ExactPDFExport.swift:147` is v2-only; `EngineV3Session.swift:759` drops `SOURCES`; #1236 S5 row | Add an **app feature-parity checklist** (§2) to the P5 exit gate. Give the six unowned rows an owner now. | Gate text: **owner**. Owners: refinement |
| 2 | **"Keystroke to pixels ≤ 16 ms" is not a measurable definition.** At 120 Hz a Core Animation commit reaches the glass 1–2 frames later (8.3–16.7 ms; measured p50 12–15 ms). Glass-level p95 ≤ 16 ms would need ≈ 0 ms of compute. T7 measures only the host. | `app-v3-preview` stages: `commit_to_vsync` 11.5–15.4 ms p50, `key_to_vsync` 28–40 ms p50 | Adopt the §3.4 definition: `NSEvent.timestamp` → commit of the edited page, p95 ≤ 16 ms. Split it into a T7 host budget (≤ 11 ms) and an app budget (≤ 4 ms). Report the glass estimate without gating it. Add an **editor-echo** target (≤ 8.3 ms p95). | Refinement (benchmark definition). Editor echo: refinement, owner FYI |
| 3 | **The v3 app path is 1.7–2.9 ms p50 and close to optimal. Three measured or code-evident costs remain.** (a) The fast path counts UTF-8 over the whole prefix or suffix on every keystroke, which is O(n). (b) SwiftUI invalidates the main thread for 12–26 ms per keystroke (F3), and no lane owns it. (c) With v3 on, the old worker still attaches and compiles, at launch and on every open. | `EngineV3Session.swift:296,305,333`; `hook→sent` p95 6.97 ms at 1,000 pages; #1252 F3; `ShellModel.swift:775-778`, `DocumentFiles.swift:864` | (a) An incremental UTF-16↔UTF-8 index, with the deleted byte count captured before the edit. (b) An **APP-SHELL-PERF** lane: one edit-delta stream (fixes F3 and F7). (c) Never attach the old worker while v3 is on. | Refinement |
| 4 | **Duplicated app lanes, and a design text that no longer matches the code.** #1228 builds tiles on the v2 pane, which retires at P5 (and D13 applies). P3-V3-ZOOM-TILES re-does them for v3. §6.2 says "keep and extend `V2PreparedPage`/`GlyphRunRenderer`", but P3-APP-V3 built `DL3Renderer`. #1236 plans to *adapt* RenderingV2 (2,331 LOC). There are four latency harnesses with four definitions. | §5 | Extract the tile core from #1228 once into `FlashTeXPreviewV3`, then stop work on the old pane. Rewrite §6.2 around `DL3Renderer`. Retire RenderingV2 outright (don't adapt it). Keep one latency harness. | Refinement |
| 5 | **The preview parity gate is weaker than §6.2 states.** At 1× there is an allowed floor: 85 px on 4 pages. Only the integer scales 1×, 2× and 4× are tested, but users see the fit-width scale (≈ 2.28 px/pt by default). With smoothing off, text is 25 % lighter than in Preview.app. | `app-v3-preview` README; `DL3Renderer.swift:484`; #1228 smoothing evidence (ink ratio 1.2525) | Sweep scales 1–8 px/pt in 0.25 steps plus the default fit-width scales. Test smoothing **on for both sides**. Take the 3.2 exact-origin proposal to the engine lane. | Refinement |
| 6 | **Four IDE features never work in the shipped app.** Find in Project (1,347 LOC), Rename Citation (557), Durable History (852) and document kinds (505) need the preview-controller. `make-app.sh` bundles it, but the app attaches it only when two env vars are set. | `make-app.sh:81`; `ShellModel.swift:770-774`; `ProjectSearchPanel.swift:287`; `CitationRename.swift:228` | Decide #1236's Q7 now: "direct v3". Move the controller's IDE services (index, search, ledger) into a helper that is always attached, with no compile route, or into the app. | Refinement (P3/P4 integration, per #1236) |
| 7 | **CI barely exercises the app on the new path.** `mac app` runs only in the full tier (19 min 40 s: helpers 435 s, `swift build` 147 s, `swift test` 552 s). The v3 host tests and the 83-fixture parity test skip in CI. DesignSnapshots skip in CI and fail locally (2–16 tests), so they gate nothing. | CI run 36670850148; `SnapshotEnvironment.swift`; `environment.json` has no `macOS` | Build `flashtex-host` in `build-helpers.sh`. Run the v3 app suites on a self-hosted Mac with TeX Live. Record `macOS` in the snapshot manifest and re-record on one pinned runner. | Refinement |
| 8 | **Maintainability.** `ShellModel` is 5,450 lines in 12 files and is extended by 39 files. P5 can retire ≈ 13.2k LOC of Swift (≈ 20 %) and ≈ 30 % of the Mac test time. `EngineV3Session` (893 LOC) sits in the AppKit target, against §16 rule 3. Rule 3 also contradicts §16's own Tauri/Qt choice. | §1, §4 | Split along the P5 seams: `EngineClient` (Foundation), `ProjectSession`, `PreviewState`, `Diagnostics`. Fix the wording of rule 3. Do the language-provider refactor after S6, except the diagnostics and INDEX parts. | Refinement; rule 3: refinement with owner FYI |
| 9 | **Project semantics are thinner than a first-class LaTeX IDE's.** There is one `ShellModel` per app (so one project per app), which contradicts the v3 docs' "one host per window". There is no `% !TEX root`, no rerun on `.aux` change, and no BibTeX, Biber or makeindex. DESIGN does not say who runs BibTeX. | `FlashTeXMacApp.swift:96`; `EngineV3Session.swift:16-23,536`; protocol §6.3 "the client decides when to rerun" | Specify external tools in §5.5 (reuse `crates/bibtex-bst` or TeX Live's `bibtex`). Add rerun-to-convergence to the session. Decide whether multi-window is a P5 or P6 item. | §5.5 addition: refinement. Multi-window scope: **owner** |

---

## 1. Audit

### 1.1 LOC by target (VERIFIED, `wc -l`, main 02dcf9d07)

| Target | Files | LOC |
|---|---:|---:|
| `FlashTeXMac` (executable, AppKit/SwiftUI) | 132 | 58,438 |
| `FlashTeXProtocol` (runtime-v1, transfer-v1, rendering-v2) | 11 | 4,562 |
| `FlashTeXAccessibility` | 6 | 2,093 |
| `FlashTeXEditorCore` (Foundation-only; shared with the iPad) | 6 | 1,981 |
| **Sources total** | 155 | **67,074** |
| `Tests/FlashTeXMacTests` | 152 | 54,394 |
| `Tests/FlashTeXAccessibilityTests` / `ProtocolTests` / `DesignSnapshots` / `HostedWindows` | 8 / 6 / 8 / 1 | 1,392 / 683 / 875 / 136 |
| `tools/nearby-client` (test-only package) | 19 | 4,176 |

- **Largest files.** `Completion.swift` 4,053, `VimMode.swift` 2,378, `ShellModel.swift` 1,686,
  `ProjectDocuments.swift` 1,519, `DocumentFiles.swift` 1,489, `PreviewV2View.swift` 1,466.
- **Engine-v3 code (not on main).** #1247 and #1254 add **+3,541 Sources LOC** in 16 files
  and **+695 test LOC** (18 tests):
  - `FlashTeXDisplayListV3`: Foundation-only decoder, connection and diag-v1;
  - `FlashTeXPreviewV3`: `DL3Renderer` and `DL3Parity`;
  - `EngineV3Host` / `Session` / `Preview` / `Latency` / `Bench` in `FlashTeXMac`.

  The rest of each PR is evidence JSON. #1254's `raw/stages` alone is 8 × 1,232 lines.

### 1.2 Old engine vs new engine (VERIFIED by reading each file's header)

**Only the old producer can drive these, so they retire at P5: 13,229 LOC ≈ 19.7 % of Sources.**

| Group | Files (LOC) | Subtotal |
|---|---|---:|
| Producer routes | `WorkerClient` 167, `PreviewControllerClient` 424, `ShellModel+Controller` 666, `ControllerRelease` 64, `AdmissionCorrelation` 148, `ShellModel+DisplayCandidates` 836, `ShellModel+OutputBounds` 246, `HistoricalPreview` 472, `Transcript` 40 | 3,063 |
| display-list-v2 negotiation | `DisplayListDelta` 433, `V2PageWindow` 165, `WholeDocumentList` 210, `DisplayListLinks` 99 | 907 |
| v1/v2 panes and renderer | `PreviewV2View` 1,466, `GlyphRunRenderer` 615, `V2PageCache` 176, `V2ImageStore` 171, `V2PathGeometry` 115, `PreviewView` (v1) 210, `PreviewTextCache` 383, `RuleGeometry` 31, `CaretSync` 174, `MathCaretHighlight` 249, `MathHoverPreview` 123 | 3,713 |
| Old-engine export and resources | `ExactPDFExport` 207, `PDFExport` 80, `ProposalPreview` 529, `BundledMetrics` 155, `Fonts` 358, `ProjectFonts` 440, `ProjectPackages` 491, `ExplanationMemo` 94 | 2,354 |
| `FlashTeXProtocol` | `RenderingV2` 1,102, `RenderingV2Fast` 1,229, `FastJSON` 765 (its users are the controller route and RuntimeV1 only), `LayoutNegotiation` 74, `Rules` 22 | 3,192 |

**Parts of shared files also retire:**

- **`RuntimeV1.swift` (791).** 45 `FlashTeXMac` files use `RuntimeV1.*`, mostly
  `RuntimeV1.Diagnostic` as *the* diagnostic type. The v3 path maps diag-v1 into it too.
  An engine-neutral `Diagnostic` in a Foundation target would let all of it retire.
- **About 600 of `ShellModel.swift`'s 1,686 lines** (BELIEF, from the 122 worker/controller
  mentions).

**Port, then retire the v2 binding: 2,904 LOC.** This is v2-pane feature logic the v3 pane
still lacks:

| File | LOC |
|---|---:|
| `TypingBench` | 515 |
| `PreviewAnchor` | 456 |
| `PreviewV2Accessibility` | 413 |
| `CaretFollow` | 356 |
| `PrintController` | 317 |
| `ExportSession` | 251 |
| `PreviewPagesRotor` | 243 |
| `PreviewAnnouncements` | 231 |
| `PreviewZoom` | 122 |

**Ratio.** 13.2k LOC serve the old engine against 3.5k for the new, about **3.7 : 1**.

### 1.3 Dead, or reachable only through an environment variable (VERIFIED)

| Code | Why it is unreachable in the product | LOC |
|---|---|---:|
| v1 pane: `PreviewView`, `PreviewTextCache`, `RuleGeometry` | `previewV2` defaults on (`ShellModel.swift:105`); v1 needs `FLASHTEX_PREVIEW_V2=0` | 624 |
| Controller route: `ShellModel+Controller`, `PreviewControllerClient`, `DisplayCandidates`, `HistoricalPreview`, `AdmissionCorrelation`, `ControllerRelease` | `attachController` is called only when `FLASHTEX_AUTOATTACH=1` and `FLASHTEX_PREVIEW_CONTROLLER` are both set (`ShellModel.swift:770-774`). There is no locator for the bundled helper, although `make-app.sh:81` bundles it. | 2,610 |
| **Features that depend on the controller route** | `helperAvailable` is false in the product (`ProjectSearchPanel.swift:287`, `CitationRename.swift:228`, `EditHistoryPanel.swift:475`, `DocumentKinds.swift:256`). The menu items exist but can never work. This is finding 6. | ProjectSearchPanel 1,347 + CitationRename 557 + EditHistoryPanel 852 + DocumentKinds 505 |
| Developer items in the product File menu | "Open Compile Result Fixture…", "Reload Fixture", "Open Display List (v2)…", three "Attach … Worker/Compiler/Render Pipeline" items (`FlashTeXMacApp.swift:277-300`) | — |

D13 allows deleting unreachable code now: removing it is not a feature. I recommend doing
that for the v1 pane immediately. The controller route should wait for the Q7 decision
(finding 6).

### 1.4 Tests and runtime (VERIFIED, CI merge-group run 36670850148, macos-26 hosted)

- **Job time.** `mac app (swift build + test)` took **19 min 40 s** (`ci.yml:573-581`, full tier
  only):
  - build every bundled helper (old-engine crates): **435 s**;
  - `swift build`: 147 s;
  - `swift test`: **552 s**. XCTest reports **1,701 tests, 30 skipped, 0 failures, 438.5 s**.
    It runs serially: `ci.yml:620` still says "877 tests in ~5 min", which is stale.
- **Where the time goes.** FlashTeXMacTests take 433 s of it. The slowest suites are process
  and recovery tests:

  | Suite | Time |
  |---|---:|
  | `DisplayListDeltaTests` | 41.1 s (5 tests) |
  | `BridgeRecoveryTests` | 29.9 s |
  | `LargeDocumentEditorTests` | 29.4 s |
  | `PasteRecoveryTests` | 17.8 s |
  | `DocumentFilesTests` | 16.9 s |

- **Old-engine share (heuristic).** Suites whose names match the old-engine features in §1.2
  (delta, controller, worker, output bounds, historical, proposal, v2, candidates,
  admission, export, anchoring, pane accessibility and so on) come to **≈ 360 tests and
  ≈ 130–138 s, ≈ 30 % of the test time**. The heuristic has a few false positives, such as
  `NearbyTranscriptAcceptanceTests` at 3.5 s.
- **Coverage of the new path in CI: minimal.**
  - `build-helpers.sh` on #1254 does not build `flashtex-host`, and the hosted runner has no
    TeX Live. So the `EngineV3Instance` and `EngineV3Open` tests throw `XCTSkip("no
    flashtex-host built")`.
  - The 83-fixture zero-tolerance parity test skips without `target/dl3-positions/`.
  - What runs in CI is the decoder and parity checks on one checked-in fixture
    (`beamer-overlays.dl3`) plus pure unit tests.
  - The 83/83 decoder and 220/220 pixel results are **local** evidence only.
- **The PR tier never builds the app.** `gate.sh pr` has no Swift step (`scripts/gate.sh:568`
  is in `full`). An app PR's first signal is therefore the merge queue.
- **Flaky under load, or under `--parallel`** (#1228 review):
  - the `EditorIndentation` perf budget;
  - `AutomaticCompletion` and `Snippet`, which fail only under `--parallel`.

### 1.5 DesignSnapshots (VERIFIED)

- **The references.** They were recorded at @2x on the owner's Retina Mac (2026-09-15; see
  `snapshot-record.yml` header). `__Snapshots__/environment.json` holds only
  `{"backingScale": 2}`. It has no `macOS`, and `SnapshotEnvironment.matches` treats a
  missing OS as "matches anything".
- **In CI.** The macos-26 runner captures at @1x, so every snapshot **skips**: 11/11 in
  `ShellSnapshotTests`, plus the Completion, Control and Harness suites, in the log above.
- **Locally.** Every Retina Mac compares against references drawn by another OS build, so
  they **fail**:
  - #1247, M5 Pro, macOS 26.6: 4 settings and palette PNGs;
  - #1252: `testCommandPalette` and `testSettings`;
  - #1228, M1 Max, macOS 26.3: 16, including `testMainWindowLivePreview` light and dark,
    which also fail on main;
  - d-q222's review: 8 on main d4f2a1581.
- **Net effect.** The suite gates nothing anywhere, and every app PR carries a "fails
  locally, not mine" paragraph.
- **Fix** (small, refinement):
  1. Run `snapshot-record.yml` on **one pinned self-hosted Mac** (it currently records on the
     hosted runner, which then skips its own references).
  2. Write `macOS` into the manifest, so that other machines skip loudly instead of failing.
  3. Gate the job on that runner label.
- **Delete now.** `testMainWindowLivePreview` depends on the v1/v2 panes, so delete it rather
  than re-record it.

### 1.6 The startup path (VERIFIED from code)

1. `@main FlashTeXMacApp` creates **one** `ShellModel` at App scope
   (`FlashTeXMacApp.swift:96`) and shares it with every scene: the main window, Settings,
   Nearby, Durable History, Find in Project and Rename Citation.
2. `ShellModel.init` (`ShellModel.swift:743-796`):
   - loads a **runtime-v1 fixture** (`Samples/compile-result.json` in the bundle, else
     `protocol/fixtures`) as the first document and preview;
   - then attaches the bundled old producer (`flashtex-render` or `flashtex-compiler`) and
     **compiles** (L775-778);
   - then attaches the capture bridge.

   None of this is gated on `engineV3Enabled`.
3. `applicationDidFinishLaunching` holds a `.latencyCritical` activity for the app's lifetime
   (L64-66). It is measured harmless at idle (#1252 F11: 1 ms of CPU in 40 s).
4. With v3 on, `PreviewV3Pane.onAppear` runs `EngineV3Session.start`:
   - locate the host, kill stale hosts, remove abandoned mirrors;
   - spawn `flashtex-host --once`, which prepares the format;
   - connect, then COMPILE "open".
5. **Measured:**
   - old engine, plain-10, loaded machine: first paint **778 ms** after launch
     (`app-perf raw/base-plain-10-mid-paragraph/startup.txt`);
   - v3: first-use format build **4,752.8 ms**, then warm-up **177.5 ms**
     (`app-v3-preview raw/host-first-start.txt`).
   - There is **no launch → first-page number for v3**.
   - §1.2's "reopen ≤ 100 ms to first visible page" is therefore unmeasured. Host warm-up
     alone exceeds it (see §6.2).

---

## 2. UX gaps: the engine-v3 pane vs the old app and a first-class LaTeX IDE

The reference IDEs are Overleaf, TeXShop, Texifier and VS Code with LaTeX Workshop, all of
which have every row below. The lanes named are the ones assigned on #2 at 05:33Z.

| Feature | Old app (v2 pane, product default) | v3 pane (#1254) | In flight | At P5 if nothing changes |
|---|---|---|---|---|
| **Zoom and tiles** | Pinch, zoom menu, Actual Size, Fit Page (`PreviewZoom`, `PreviewMagnify`). #1228 adds 512 px tiles | Fit-width only | **P3-V3-ZOOM-TILES** (mac-claude-a) | regresses unless that lane lands |
| **Forward/reverse search** | Exact byte spans (`CaretSync`, `Navigation.navigateExactly`, ⌘⇧J) | `SOURCES` ignored (`EngineV3Session.swift:759`) | **P3-APP-V3** (P3-SOURCE-MAP moved there) | regresses |
| **Follow-edit scrolling** | `CaretFollow` (debounced) | none | P3-APP-V3 | regresses; it should also drive COMPILE `viewport` (L4) |
| **Dark preview** | Ink inversion, images excluded; title-bar toggle and preference (`GlyphRunRenderer.swift:328-360`, `TitleBar.swift:137`) | white pages only | P3-APP-V3 | regresses |
| **Errors** | Byte-span underlines, error lens, Problems panel, explanations | Problems panel with diag-v1: exact column, macro chain, help (#1254 db0dd708b, #1255) | P5-DIAGNOSTICS | **better than old**; the `flashtex-explain` catalogue is keyed to old-compiler messages and needs TeX-error entries |
| **Export and print** | `flashtex-pdf-exact` from the v2 list (`ExactPDFExport.swift:147`) | **none**: with v3 on, Export and Print still read the *old* engine's last v2 list | **unowned** (#1236 S6 says "→ engine PDF") | **regresses**; should become the host's `export: true` (P-T2) |
| **VoiceOver page text and pages rotor** | `PreviewV2Accessibility`, `PreviewPagesRotor` | pages are unlabelled images ("Page N") | **unowned** | regresses |
| **Link clicks** (hyperref) | `DisplayListLinks` | none | **unowned** | regresses |
| **Scroll anchoring** across reflow and resize | `PreviewAnchor` | none (NSScrollView keeps the offset in points) | **unowned** | regresses; easier in AppKit than in SwiftUI |
| **Stale and last-good** | `HistoricalPreview` (display-only) | Stale pages dimmed with an orange border (`EngineV3Preview.swift:145-150`) | done | better |
| **`.aux` reruns, BibTeX, index** | old compiler internal | **none**. `DONE.mode` only appears in the status text (`EngineV3Session.swift:536`). The mirror is fresh per session, so the first compile after open has no `.aux` (BELIEF: refs show `??` until the next edit) | **unowned**; DESIGN §5.5 is silent on external tools | **gap vs every IDE**; latexmk-style convergence is expected |
| **Main-file detection** | manifest or the only `.tex` file | a `\documentclass` regex over the open documents (`EngineV3Session.mainFile`) | — | missing `% !TEX root` and `% !TEX program`, which TeXShop, Texifier and LaTeX Workshop support |
| **Multi-window / multi-project** | one project per app (a single App-scope model) | same; the v3 docs claim "one host per window" (`EngineV3Session.swift:16-23`), which the app doesn't provide | — | a gap vs every IDE; **owner** scope call |
| **First run** | runtime-v1 fixture as the first document; developer menu items | "Preparing the pdfLaTeX format…" plus a timer (4.75 s once). **No TeX Live gives a failure**: D12's bundle is lane #1216 | #1216 | needs a welcome/recent-projects view, format pre-build at first launch before a document opens, and an actionable no-TeX-Live screen |
| **Settings** | Editor, Compile, Conversion tabs | env vars and a View-menu toggle only | — | missing: engine per document (#1236 flag design), TeX Live location, **per-project shell-escape opt-in** (§13 `\write18` decision; no UI exists), format rebuild |
| **Completion vocabulary** | `supported-latex.json`, the old compiler's inventory, with "not supported by this compiler version" marks (`Completion.swift:20,66-70`) | same | #1236 Q5; Track B V3 | the marks become false at P5; use an engine-truth INDEX on the Mac, and an MIT source for the iPad (§3) |
| **Capture proposal preview** | shadow compile on a second worker (`ProposalPreview`) | none | — | regresses; could become a host shadow COMPILE from the checkpoint |
| **Find in Project, Rename Citation, Durable History** | unavailable in the product (§1.3) | unavailable | — | unchanged; fix through Q7 (finding 6) |

**Verdict.** Six rows have no owner: export and print, VoiceOver, links, anchoring, reruns
and BibTeX, and engine settings including shell-escape. These are exactly the rows the P5
gate needs.

---

## 3. The latency path

### 3.1 Stage budget, p50 / p95 ms

Source: `app-v3-preview raw/stages/stages.txt`, fast path; release build; 60 keystrokes; load
2–4.

| document | key→hook | hook→sent | host first page | decode+prepare | raster | raster→commit | **key→commit** | commit→"vsync" | key→"vsync" |
|---|---|---|---|---|---|---|---|---|---|
| plain-10 | 0.74/1.07 | 0.41/0.62 | 19.74/21.43 | 0.09 | 1.57/1.79 | 0.16/0.28 | **22.9/25.0** | 15.0/18.6 | 38.9/39.6 |
| plain-120 | 0.30/0.42 | 0.27/1.14 | 13.21/14.39 | 0.07 | 1.18/1.24 | 0.13/0.14 | **15.4/17.2** | 13.9/18.8 | 28.1/35.0 |
| plain-1000 | 0.24/0.43 | 0.26/**6.97** | 17.92/24.45 | 0.06 | 1.07/1.35 | 0.12/0.14 | **22.9/31.6** | 15.3/21.5 | 39.6/49.0 |

**The app-owned stages** (key→hook, hook→sent, decode, prepare, raster, commit) sum to
**≈ 1.7–2.9 ms p50** (VERIFIED). The host is 13–20 ms of the 15–23 ms total.

### 3.2 Is the app side optimal? Mostly, with three exceptions

**What is already right (VERIFIED):**

- **The edit hook comes first.** `NSTextStorage.didProcessEditingNotification` fires before
  the editor's own work (`EngineV3Session.swift:141-146,282-316`).
- **Raster and commit happen on the socket reader thread,** into an IOSurface, with an
  explicit `CATransaction` plus `flush` (`EngineV3Preview.swift:108-119`). The main thread
  is not on the path to the render server.
- **Commits are made as soon as a page exists, not aligned to a display link.** Aligning
  them to vsync would only add waiting, so **no frame is wasted by pacing**. The earlier
  3.5 ms `CGImage` commit and the 1–3 ms main-thread hop were removed and measured.

**Exception (a): O(n) UTF-8 counting in the fast path** (VERIFIED in code; the cost is BELIEF).

- `prefix = utf8Count(text, 0..<r.location)` runs `CFStringGetBytes` over the whole prefix on
  every keystroke (L296).
- For a deletion, `suffix` counts the whole rest of the document (L305).
- The bench edits page 1 and alternates insert and delete. The inserts have a short prefix
  and cost 0.26 ms. The deletes count the ≈ 4 MB suffix, which explains **hook→sent p95
  6.97 ms at 1,000 pages** (1.14 ms at 120 pages and 0.62 ms at 10 pages scale with size).
- Typing at the end of a 1,000-page document would pay it on every insert.
- The slow path adds `activeText.utf8.count` on a bridged string, O(n) (L333), plus the
  model's whole-buffer copy (#1252 F7: 4.24 ms at 4 MB). Both are on main, after the send.
- **Fix:**
  1. Keep an incremental UTF-16→UTF-8 offset index: per-line or per-chunk byte counts in a
     Fenwick tree, updated from `editedRange`/`changeInLength`, so each lookup is O(log n).
  2. Capture the deleted byte count in `textView(_:shouldChangeTextIn:replacementString:)`
     before the storage changes.
  3. Make the bench edit at the start, middle and end (T7 already does this for the host).

**Exception (b): main-thread saturation is unowned** (VERIFIED, #1252 F3).

- Editor-only typing costs **12.1 / 15.3 / 26.4 ms of main-thread work per keystroke**
  (10 / 120 / 1,000 pages): SwiftUI `GraphHost.flushTransactions`, `CA::Transaction::commit`
  and `NSHostingView.layout`.
- The v3 fast path routes the preview *around* the main thread. But the editor's own echo
  commits at the end of the same run-loop turn (BELIEF, from the structure of that turn).
  So on large documents **the typed character itself can appear later than one 120 Hz
  frame**, possibly later than the preview page.
- #1228's scroll bench traces its last dropped frames to the same `NSHostingView.layout`
  (13–18 % of main).
- The #1252 table assigns F3 to "app shell (next lane)". **No such lane exists on #2.**

**Exception (c): the old engine still runs with v3 on** (VERIFIED in code).

- `ShellModel.init` attaches the bundled producer and calls `compile()` unconditionally
  (`ShellModel.swift:775-778`).
- `openTex` → `if workerAttached { compile() }` (`DocumentFiles.swift:864,890`).
- ⌘B still compiles with the old worker.
- On 1,000 pages each old compile is 5–7 s of CPU (#1252 F8; 42 s of startup compiles, F5),
  competing with the host for cores during exactly the moments the §1.2 targets measure.
- **Fix:** don't attach the worker, bridge-free, while `engineV3Enabled`. It is a few lines.

### 3.3 Frame pacing: what "commit → next frame 12–15 ms" means

- **How it is measured.** `vsync` is the `targetTimestamp` of the first `CADisplayLink`
  callback after the commit (`EngineV3Latency.swift:124-129`). The link is armed only after
  the commit has been recorded on main (`EngineV3Preview.swift:288-308`, via `onMain`).
- **The model.** A commit made between vsync N−1 and N is composited for, and shown at,
  N+1, which is callback N's `targetTimestamp`. Commit→present is therefore uniform on
  [1, 2] frames:
  - at **120 Hz**: 8.3–16.7 ms, mean 12.5 ms;
  - at **60 Hz**: 16.7–33.3 ms, mean 25 ms.
- **Measured:** p50 11.5–15.4 ms, p95 15.5–21.5 ms. That fits **120 Hz**, not 60 Hz. The
  README's "on this 60 Hz path" is most likely wrong (BELIEF; check by logging the link's
  `duration`).
- **Why p95 exceeds 16.7 ms.** It is a measurement artefact. When the main-thread hop (to_main
  p95 3.4–4.9 ms) straddles vsync N, arming slips a frame.
- **Measurement fix.** Keep one display link running during the bench, on its own thread.
  Record the vsync timeline and match each commit to the first vsync after it, plus one
  frame. Don't arm after a hop.
- **Consequence: keystroke-to-glass ≈ key→commit + 8.3–16.7 ms at 120 Hz.** Today that is
  p50 28–40 ms. Even with a 0 ms host, app work plus the compositor gives p50 ≈ 14 ms and
  p95 ≈ 18–19 ms. **A glass-level "≤ 16 ms p95" is physically unattainable on macOS at
  120 Hz.**
- **ProMotion idle behaviour** (BELIEF): a panel that has dropped its refresh rate while idle
  may add latency to an *isolated* keystroke. A running display link forces 120 Hz, which
  changes what it observes. Only an external camera can settle this, once, as calibration.

### 3.4 Proposed exact definition for the §1.2 gate (refinement)

**KTP (gated), keystroke to pixels:**

- **Formula.** KTP = t_commit − t_key.
- **t_key.** The `NSEvent.timestamp` of the keyDown, converted to `CLOCK_UPTIME_RAW` ns. The
  session already records it (`EngineV3Session.swift:136-138`).
  - The scripted bench must deliver keys as real `NSEvent`s through
    `NSApp.sendEvent`/`keyDown`, as `FlashTeXMacApp.swift:123-126` already does, instead of
    calling `insertText` directly. That way the key dispatch, `interpretKeyEvents` and
    input-method costs are inside the measurement.
- **t_commit.** When `CATransaction.commit()` + `flush()` returns after installing a bitmap
  (page or tile) that contains the edit, for the page holding the caret, while that page is
  on screen.
- **Coalescing.** A keystroke superseded before it paints is measured to the first commit
  that includes it. `EngineV3Latency.committed` already does this.
- **Population.** The T7 documents (10/100/300/1,000 pages, plain and full). Edits at the
  start, middle and end; insert and delete; reflowing and not. At least 200 keys at a 125 ms
  cadence plus 30 ms bursts. Release build. Reference machine: M-series, built-in 120 Hz
  display, default window at fit width, load average < 2.
- **Gate.** p95 ≤ **16 ms** per document. That is about 2 frames at 120 Hz: the edited page
  lands in the 2nd–3rd composited frame after the key.
- **Budget split:**
  - **T7 host gate:** `COMPILE` written → edited-page `PAGE` frame read, p95 ≤ **11 ms**.
    Today it is 13.2–19.7 ms p50.
  - **App gate** (in-app bench, host time subtracted): p95 ≤ **4 ms**. Today ≈ 1.7–2.9 ms p50,
    with the p95 outlier from (a).

**Reported, not gated:**

- **Keystroke to glass (estimated):** KTP plus the matched vsync + 1 frame (§3.3).
- **Editor echo:** t_key → the commit that shows the typed glyph in the editor. **Proposed
  new §1.2 row: ≤ 8.3 ms p95 at any document size** (one frame at 120 Hz). Report it now;
  gate it once APP-SHELL-PERF exists. Today's main-thread cost suggests it misses on large
  documents (BELIEF, unmeasured).
- **Scroll and zoom:** replace "no drawing on the main thread" with **"0 dropped frames
  (interval > 1.5 × 8.33 ms) in the in-app scroll and pinch benches, and no main-thread
  run-loop pass > 4 ms"**. Main-thread drawing is necessary but not sufficient: #1228 showed
  3–5 drops per 8 s caused by layout, not drawing.
- **Reopen:** define t0 as the open command, or process start for a launch-restore. Page
  pixels means the persisted previous bitmap, marked stale, is an acceptable first page
  (§6.2).

---

## 4. Maintainability

### 4.1 ShellModel (VERIFIED)

- **Size.** 5,450 lines in 12 `ShellModel*.swift` files. `extension ShellModel` appears in
  **39** files. The main file declares ≈ 242 stored properties, and the 12 files hold
  ≈ 215 funcs (grep-level counts).
- **Responsibilities:**
  - documents and project;
  - three producer routes (worker, controller, fixtures) with capability negotiation,
    output bounds, historical frames and admission correlation;
  - capture bridge and Nearby;
  - diagnostics and retention;
  - navigation and hover;
  - preview zoom and caret follow;
  - export;
  - packages, fonts and manifest;
  - the chrome mirrors.
- **One instance per app** (`FlashTeXMacApp.swift:96`), which also rules out multi-window.
- **Good precedent.** `EngineV3Session` (893 LOC) is a separate `@Observable` object with
  narrow hooks: 3 lines in `updateActiveText`, `replaceProject` and `displayedDiagnostics`.
  That is the right pattern to grow.

### 4.2 Proposed split along the P5 seams (BELIEF on the savings; LOC are counts)

1. **Delete** (S6): the §1.2 "retire" set of ≈ 13.2k Sources LOC (≈ 20 %), plus ≈ 360 tests and
   ≈ 30 % of the `swift test` time. The `mac app` job drops its old helper build (435 s → the
   host plus 3 IDE helpers; BELIEF ≈ 150 s).
2. **`FlashTeXEngineClient`** (new, Foundation-only):
   - the v3 session logic now in `EngineV3Session`/`Host`: splices, mirror, restart policy,
     request building, diag mapping, latency bookkeeping;
   - parameterised by engine (`flashtex-host` | `flashtex-typst-host`), so §15 plugs in
     without touching `ShellModel`.

   Only the `NSTextStorage` and `NSEvent` hooks stay in AppKit. This complies with §16
   rule 3 (finding 8).
3. **`ProjectSession`** (documents, files, manifest, main-file and `% !TEX root`),
   **`PreviewState`** (pages, stale, zoom, sync), **`DiagnosticsStore`** (an engine-neutral
   `Diagnostic` in a Foundation target, which lets `RuntimeV1` retire), and **`CaptureSession`**
   (bridge, Nearby).

   `ShellModel` becomes a per-window composition root. This also unlocks multi-window.
4. **One edit stream.** `NSTextStorage` edits become `EditDelta` values, consumed by the
   engine session, the model, bridge rebasing and diagnostics rebasing. This removes the
   whole-buffer binding (F7: 4.24 ms per key at 4 MB and 3× document RAM) and most of F3's
   observed-state churn.

   **Expected:** −8–10 ms of main thread per keystroke on large documents (#1252's estimate).

### 4.3 The language-provider refactor (Track B §4.3)

- **The direction is right.** 36 `FlashTeXMac` files call LaTeX scanners directly (Track B,
  VERIFIED there). A `SyntaxProvider` in `FlashTeXEditorCore` plus a revisioned
  `SemanticProvider` is the right shape.
- **The sequencing is wrong if it starts now.** Abstracting while the v1, v2 and v3 routes
  coexist means building adapters over runtime-v1 completion, the controller's `complete`
  and the inventory, all of which retire. I recommend:
  - **Now,** as part of the v3 work: the `SemanticProvider` slice for engine-backed features
    only. That is diagnostics (diag-v1, done), INDEX (labels and numbers, cs table, outline)
    and completion vocabulary, because both hosts answer over the same socket.
  - **After S6:** the syntax-provider generalisation (6–9 days, belief), when there is one
    route and Typst lanes are ready to consume it. §15 puts Typst below every LaTeX phase.
- **Red-team of Track B's staticlib.** Track B links a `typst-syntax` Rust staticlib into
  the app, the app's first linked Rust. Its reason is that "a process hop would put
  [highlighting] behind a compile". That holds only for a single-threaded host.
  - A syntax thread in `flashtex-typst-host` answering over the existing socket costs
    ≈ 61 µs/MB (D11) plus a round trip (BELIEF: < 0.3 ms). That fits the < 1 ms budget
    without SwiftPM+cargo universal builds or a new licence check C.
  - The staticlib is still needed for the iPad editor, which has no host (D14). Build it
    when the iPad needs Typst, not before.

### 4.4 Portability rules (§16) as they apply to the app

- **Rule 3** says "portable Rust helpers **or Foundation-only Swift targets**". §16's own
  plan for other OSes is Tauri 2 + CodeMirror 6 or Qt 6. Neither can reuse Swift, so
  Foundation-only Swift is *not* portable under that plan.
- **Proposed wording:** "Logic that a non-Mac shell would need (edit splicing, protocol
  client, project model, diagnostics mapping) goes into Rust. Foundation-only Swift is for
  logic shared with the iPad." (Refinement; owner FYI because it shapes the §16 cost
  estimate.)
- **Today's v3 code against the rules:**
  - `FlashTeXDisplayListV3` is Foundation-only and imports `Glibc` on Linux. Good.
  - `DL3Canonical` uses **CryptoKit**, which is Apple-only. Use swift-crypto, or keep the
    canonical hash in tests.
  - `EngineV3Session` imports AppKit for two hooks. Split it (§4.2).
- **Rule 5** (the preview gate "as drawn by the platform's reference rasteriser") fits the
  v3 parity test, which renders the engine's PDF live on the same machine. The comparison
  is therefore OS-version-independent. Good.

---

## 5. Effectiveness: sequencing and duplication

**Gate position:**

- **P3's app part is nearly met:** a flag, a separate process, 83/83 decoder parity, and
  220/220 pages at 2× and 4×. What remains open is the 1× floor and the untested fit-width
  scales (finding 5).
- **P4 is the host** (T7). No P4 work is blocked on the app.
- **P5 has no app criterion** (finding 1).

**Stacking risk.**

- The v3 app path lives only in two open PRs, #1247 (+23.9k) and #1254 (+13.0k, based on
  #1247).
- P3-APP-V3 (sync, follow, search highlights, dark) and P3-V3-ZOOM-TILES both edit
  `EngineV3Preview.swift` and `DL3Renderer.swift`.
- **Action:**
  1. Land #1247 and #1254 behind the flag now. With the flag off there is no behaviour change,
     and it is D13-safe.
  2. Keep bulky raw JSON out of the PRs.
  3. Before the two lanes collide, split the pane into owned files: `EngineV3TileLayer`
     (tiles and zoom, mac-claude-a) and `EngineV3Sync` (P3-APP-V3).

**Duplication:**

- **#1228 on the v2 pane vs P3-V3-ZOOM-TILES.**
  - #1228's value is the tile *core*: `tileOrigin`'s phase-boundary nudge, exact rule edges,
    cut-from-page for path and image pages, off-main colour conversion, and the 0.24–0.32 ms
    tile cost. All of that is renderer-agnostic.
  - The Commander still lists "finish #1228 for the old pane". That pane retires at S6, and
    under D13 new features on the old preview path are waste.
  - **Action:** land the tile core once, in `FlashTeXPreviewV3`, for `DL3Renderer`. Close the
    old-pane part of #1228, or land it only as the source of that code.
- **APP-PERF-AUDIT vs P3-APP-V3.** They are complementary, not duplicate.
  - F2, F5, F9 and F10 are solved by v3's design: per-page frames, a binary encoding and
    off-main work.
  - F1 is a D13 fix, and F4 and F6 are engine-independent editor fixes. Keep all three.
  - F3 and F7 are unowned (§3.2).
- **Four latency harnesses, four definitions:**

  | Harness | Definition |
  |---|---|
  | `TypingBench` (515) | keystroke → v1/v2 paint |
  | `PerfSignposts` (#1252) | signposts on the old path |
  | `EngineV3Latency` + `EngineV3Bench` | key → commit / "vsync" |
  | T7 `tools/latency-bench` | edit → frame at the socket |

  `perf-bench` retires as well. **Action:** one definition (§3.4), with `TypingBench`'s driver
  (typed-200 at 30 ms, no OS permissions) feeding `EngineV3Latency`'s stages. T7 stays the
  host-only sub-gate.
- **#1236 vs P3-APP-V3.** The retirement plan's `PreviewProducer`/`EngineChoice` design and
  "adapt RenderingV2 to feed `V2PreparedPage`" predate what P3-APP-V3 built: a separate pane,
  a separate renderer and a global flag.
  - **Action:** update #1236 §1.4 to retire RenderingV2, RenderingV2Fast and FastJSON
    (3,096 LOC) outright.
  - Retire `ProjectFonts` too, rather than adapting it. `[fonts]` is `\setmainfont`
    (fontspec) semantics, which pdfLaTeX does not have.
  - Evolve the global `engineV3Enabled` flag into #1236's per-document `EngineChoice`.

---

## 6. Red team

### 6.1 §6.2, the preview renderer decision

1. **The text no longer describes the code.**
   - §6.2 says "keep and extend `V2PreparedPage` → `GlyphRunRenderer.draw` →
     `V2PageRasterizer`". P3-APP-V3 built a new path instead: `DL3PreparedPage` →
     `DL3Renderer` → IOSurface layer contents.
   - The reason is Type 1. The v3 path loads the engine's Type 1 programs through
     `CGFont(CGDataProvider)`, the same rasteriser that draws the PDF's embedded subsets, so
     neither the "converted font" nor the "FreeType" branch of §6.2 is needed. This is
     measured on macOS 26.
   - D6 still stands: Core Graphics/Core Text plus Core Animation, no Metal.
   - **Action:** rewrite §6.2 around `DL3Renderer` and the v3 pane, and record the Type 1
     finding in §13 with its risk.
   - **The risk.** Type 1 in `CGFont` is undocumented and could disappear, as it did from
     Core Text. Add a canary test on each macOS beta. Keep engine-side CFF conversion as the
     fallback; the protocol does not change.
2. **"Zero tolerance" is not what is gated.**
   - The v3 test allows ≤ 64 px per page at 1× (four pages, 85 px in total).
   - It tests only 1×, 2× and 4×. The pane runs at scale × backing, 2.28 px/pt for the
     default window (`EngineV3Preview.swift:211`), which no test covers.
   - #1228 found scale-dependent phase-boundary misses in *tiles* at 3.25–8 px/pt:
     6 differing tiles before its fix, 0 after.
   - **Action:**
     1. A nightly sweep over 1–8 px/pt in 0.25 steps plus the default fit-width scales at
        @1x and @2x.
     2. The 3.2 exact-origin proposal: have the display list carry pdfTeX's `TJ`/`Td`
        grid, or the PDF's decimal origin.
     3. Until then, state the 1× floor in §6.2. It violates the P3 exit text, so write it
        down rather than leave it implied.
3. **The two smoothing requirements contradict each other on the v2 path, but may not on
   v3.**
   - Smoothing off gives text **25 % lighter** than Preview.app: an ink ratio of 1.2525 at
     Preview's scale (#1228).
   - The v2 conflict came from `CTFont` vs the embedded-font path. In v3, **both sides draw
     the same `CGFont` programs**, so smoothing on, applied to both, may keep 0 px
     (BELIEF).
   - **Action:** one test run with `setShouldSmoothFonts(true)` on both the `DL3Renderer`
     context and the `drawPDFPage` reference, at the default and swept scales. If it gives
     0 px, adopt smoothing on. It is the #1228 ruling, which the Commander still owes.
4. **Tiling threshold.** At fit width on Retina a Letter page is 1395 × 1806 px ≈ 10 MB. With
   the ±1 screen margin that is about 40–60 MB resident (BELIEF, arithmetic), so "tile only
   above ≈ 3 px/pt" is still right.
   - A keystroke re-rasters a whole visible page (1.1–1.6 ms).
   - With tiles and a per-tile content key, only the changed tiles would re-raster
     (≈ 0.24–0.32 ms each, #1228). That is a worthwhile app-budget item, gated by the
     parity sweep.
5. **Threading.** Layer contents are installed from the reader thread in explicit
   transactions, while AppKit sets the frames of the same layers on main
   (`EngineV3Preview.swift:94-119,205`). It works as measured.
   - The risk (BELIEF): a frame change and a contents install in different transactions
     can show one frame with new contents at the old geometry during a resize.
   - The check is a resize-while-typing bench.
6. **"A custom Metal renderer is justified only if…"** still holds. Nothing in the evidence
   shows raster time dominating.

### 6.2 §1.2, the app-side targets

- **Keystroke to pixels:** ill-defined. See §3.3–3.4 for the proposed definition and split.
- **Editor echo is missing.** Users look at the caret while typing. At 120 Hz, echo within
  one frame is the bar that native editors meet (BELIEF). See §3.4.
- **Reopen ≤ 100 ms.**
  - Host warm-up is 177.5 ms, and app launch plus the window is several hundred ms (old
    path 778 ms to first paint).
  - Only an **app-side persisted page cache** can meet 100 ms: the last `DL3PreparedPage`s
    or bitmaps per document, keyed by source hash, painted stale until the host confirms.
  - That is cheap and robust, but it needs an owner. The engine's S₀ snapshot alone cannot
    get there.
- **"Scroll and zoom: 120 Hz; no drawing on the main thread"**: restate it as a frame-drop
  and main-pass budget (§3.4).

### 6.3 §15 and §16, app parts

- **§15 "the app selects the engine by file type".** Today `EngineV3Session` is
  LaTeX-specific: the `\documentclass` main-file regex, `.tex` jobname, format-preparation
  text and TeX error wording. **Action:** the `FlashTeXEngineClient` split in §4.2 is the
  cheap prerequisite. Do it now (it also serves P5), not in a Typst lane.
- **§15 "must not slow LaTeX".**
  - Track B's step 1 is a 6–9-day provider refactor inside `apps/mac`, while four LaTeX app
    lanes are live.
  - #2 correctly keeps Typst lanes out of `apps/mac` "until I say so".
  - Keep it that way until S6 (§4.3).
- **§16 rule 3** conflicts with the Tauri/Qt plan (§4.4).
- **§16 rule 6** (one cache resolver). The v3 code has its own resolver
  (`EngineV3Host.swift:80-88`, `FLASHTEX_V3_CACHE`). Fold it into #1246's resolver when that
  lands.

---

## 7. Decisions proposed

**Refinements** (the Commander adopts with this evidence and records them in §13):

1. The §1.2 measurement definition and budget split (§3.4). Rewrite the scroll/zoom row as
   a frame budget. Report editor echo now.
2. Rewrite §6.2 around `DL3Renderer` and the v3 pane, including the `CGFont` Type 1 finding
   and its canary. Add the scale sweep to the parity gate. Decide smoothing by the
   both-sides test.
3. Stop feature work on the v2 pane. Move #1228's tile core into `FlashTeXPreviewV3`. Keep
   one latency harness.
4. Create an **APP-SHELL-PERF** lane: the edit-delta stream (F3, F7), the incremental UTF-8
   index, no old worker while v3 is on, and the `ShellModel` split along the P5 seams.
5. Give owners to export and print (host `export: true`), VoiceOver, links, anchoring,
   reruns with BibTeX/makeindex, and engine settings including the per-project
   shell-escape opt-in. Specify the external tools in §5.5; the MIT `crates/bibtex-bst`
   is reuse-first (§1).
6. Decide Q7 as direct v3. Re-home the controller's IDE services so that Find in Project,
   Rename Citation and Durable History work in the product.
7. CI:
   - build `flashtex-host` in `build-helpers.sh`;
   - run the v3 app suites and the 83-fixture parity on a self-hosted Mac with TeX Live;
   - record the DesignSnapshots on one pinned runner, with `macOS` in the manifest;
   - fix the stale `ci.yml:620` comment.
8. Land #1247 and #1254 behind the flag. Split the pane into owned files before
   P3-V3-ZOOM-TILES and P3-APP-V3 collide.
9. Update #1236: retire RenderingV2, RenderingV2Fast, FastJSON and ProjectFonts outright,
   and move the global flag to a per-document `EngineChoice`.

**Owner decisions:**

1. **Add app feature parity to the P5 exit gate** (§12), using the checklist in §2. This
   changes a phase gate.
2. **Multi-window / multi-project scope.** Should it be required for P5, or wait for P6? It
   is a product-scope call. The code cost is the per-window `ShellModel` of §4.2.
3. **FYI: §16 rule 3 wording** (Rust vs Foundation-only Swift for shared logic). It affects
   the §16 cost estimate for the future non-Mac shell.

---

## Appendix: how the numbers were obtained

- **LOC.** `wc -l` and a Python walker over `apps/mac` at main 02dcf9d07. The v3 deltas are
  `git diff --stat origin/main origin/agent/kabir-claude/app-v3-2 -- apps/mac/Sources` and
  `-- apps/mac/Tests`.
- **CI.** `gh run view 36670850148 --json jobs` gave the step durations.
  `gh run download 36670850148 -n mac-swift-test-log` gave the per-test times, which were
  parsed from the `Test Case '-[…]' passed (N seconds)` lines.
- **Latency.** Taken from committed evidence only:
  - `docs/evidence/app-v3-preview-2026-09-29/` on `app-v3-2` (`raw/stages/stages.txt`);
  - `docs/evidence/app-perf-2026-09-30/` on `app-perf`;
  - `docs/evidence/preview-smoothing-2026-09-29/` on `preview-tiles`.

  Nothing was re-measured tonight (CPU rule).
- **Lane state.** #2 comments to 2026-09-30T05:33Z, and the PR bodies and review comments of
  #1228, #1247, #1252 and #1254.
