# App parity gap: old preview path vs engine-v3 pane (2026-10-01)

Lane **J3 `P5-APP-PARITY`**, phase 1 (gap audit). Owner decision 3A: the old engine is
deleted once the new one has full parity with it, and the app must lose nothing
(DESIGN.md §10, §12 P5). This file lists every user-visible feature of the Mac app's
old compile/preview path that the engine-v3 pane lacks or handles differently, and
proposes an order for closing the gaps.

## Basis

- **Code:** `origin/main` at `d8c3009d9`, read only. Nothing was built or run.
- **Open PRs:** #1332 (P3-APP-V3: instant reopen, project trust, key→presented) and
  #1255 (P5-DIAGNOSTICS, diag-v1) are both open. Their effect is noted per row as
  *pending*. #1296 (external tools in the host, protocol 3.2) is merged.
- **The old path:** the old engine (`crates/compiler`, `crates/render-pipeline`,
  `crates/preview-controller`, `crates/flashtex-cli`) and the app's side of it
  (`WorkerClient`, `PreviewControllerClient`, `ShellModel+Controller`, `model.result`
  / `displayListV2`). It drives the v1 pane (`PreviewView.swift`) and the v2 pane
  (`PreviewV2View.swift`, the shipped default).
- **The v3 path:** `model.engineV3Enabled` (`ShellModel.swift:113`) switches the
  pane to `PreviewV3Pane` (`ContentView.swift:318`). The session is
  `EngineV3Session.swift`, the separate process is `flashtex-host`, and the
  protocol is display-list-v3.
- **Paths:** relative to `apps/mac/Sources/FlashTeXMac/` unless they start with
  `apps/`, `crates/` or `docs/`.
- **Verified vs inferred:** four read-only explorers covered (1) export, print and
  SyncTeX, (2) preview interactions, (3) compile triggers and projects, and
  (4) diagnostics, settings, companion and CLI. I re-checked every load-bearing claim
  (marked ✔ below) against the code. Rows marked *(inferred)* describe runtime
  behaviour reasoned from code, not observed.

**Status values:**

| Status | Meaning |
|---|---|
| **has** | v3 provides the feature |
| **partial** | v3 provides some of it |
| **missing** | v3 does not provide it |
| **different** | v3 provides something else, or the feature silently uses the old engine's data |

**Size:** S is under a day, M is a few days, L is a week or more.

## Root cause behind many rows

Turning v3 on does **not** stop the old engine. ✔
- The worker still attaches at launch (`ShellModel.swift:797-800`).
- It still compiles on open (`DocumentFiles.swift:864`).
- It still compiles on ⌘B (`ShellModel.swift:1237`, no v3 check).
- Turning v3 on doesn't clear `result` or `displayListV2` (`ShellModel.swift:117`).
- Only per-keystroke auto-compile is skipped (`ShellModel.swift:992`).

So with v3 on, `result`, `resultID`, `compiledDocuments` and `displayListV2` hold
**old-engine data from open time, or from the last ⌘B**. These features read that
data and so mix the two engines without saying so: underlines, explanations,
navigation rebasing, the Problems status line, the latency readout, the capability
chip, the pane's accessibility value, Export and Print.

## Gap table

### A. Compile triggers and projects

| # | Feature | Old path (file:line) | v3 status | Evidence (v3) | User impact | Size |
|---|---|---|---|---|---|---|
| A1 | Manual compile: ⌘B, the title-bar ▶ and the palette | `FlashTeXMacApp.swift:304-306`, `TitleBar.swift:118`, `CommandPalette.swift:102` → `ShellModel.swift:1237` | **missing** | None of them reach `engineV3.compile`. v3 compiles only on start, open, edit and recover (`EngineV3Session.swift:185,295,380,400,585`) ✔ | ⌘B gives no fresh v3 compile. It runs a hidden old-engine compile instead, and it is greyed out when no worker is attached | S |
| A2 | One engine at a time | `ShellModel.swift:117, 797-800`; `DocumentFiles.swift:864` | **different** | The old worker keeps running alongside v3 (root cause above) ✔ | Two engines use CPU, and many features below show old-engine data | S |
| A3 | Auto-compile toggle (Settings ▸ Compile) | `EditorPreferences.swift:900`; `ShellModel.swift:510, 998-1006` | **different** | Ignored. v3 compiles every edit (`ShellModel.swift:985`), and `autoCompile` never appears in `EngineV3*` | The user can't turn off live compiling, and the "⌘B to compile" hint (`ShellChrome.swift:152`) is wrong | S |
| A4 | Recompile when an unopened `\input` file changes on disk (git checkout, another editor) | `ProjectDocuments.swift:682-706` (per-path watchers → `model.compile()`) | **missing** | The watchers call the old path. v3 reads the file through a symlink (`EngineV3Session.swift:957-958`), but nothing triggers a recompile | The preview stays stale until the next keystroke | S |
| A5 | New project files created outside the app (a chapter or figure added in Finder) | `ProjectDocuments.swift:561` (closure rediscovered per compile) | **partial** | Only a pasted image is linked at once (`EngineV3Session.swift:407-410`). Other files are linked only on non-edit compiles (`:479`) | A new file is not found until the next open or explicit compile *(inferred)* | S |
| A6 | `\include` / `\input` resolution | `ProjectDocuments.swift:643-675` (the app reads unopened includes and sends them, 8 MiB cap) | **different** | Real TeX resolves files in a symlinked copy of the project (`EngineV3Session.swift:935-961`, 20,000-entry cap, hidden directories skipped) | Closer to pdflatex. A project with more than 20,000 entries is cut short | — |
| A7 | Which file is compiled when a chapter tab is active | `ProjectDocuments.swift:443`, `ShellModel.swift:1289`: always the entry | **different** | The entry, else the first open document with `\documentclass` (`EngineV3Session.swift:386-394`) | A chapter opened on its own compiles the root only if the root is also open | S |
| A8 | `flashtex.toml` texinputs, package inputs, `[packages]`; File ▸ Fetch Missing Packages… | `ProjectDocuments.swift:663-673`; `ProjectPackages.swift:95-133, 288-292`; `FlashTeXMacApp.swift:269` | **missing** | No manifest or package input appears in `EngineV3*`. Only files inside the project root are linked | A project using a shared style directory outside its root fails under v3 | M |
| A9 | System and project fonts (`[fonts]`, File ▸ Project Fonts…, fontspec-style documents) | `ShellModel.swift:1306`; `ProjectFonts.swift`; `ShellModel+Controller.swift:176-179`; `FlashTeXMacApp.swift:265` | **missing** | v3 is pdfLaTeX only: `format = "pdflatex"` (`apps/mac/Sources/FlashTeXDisplayListV3/DL3Connection.swift:19`), and only that format is prepared | Documents that relied on the old engine's font support fail, and the sheet does nothing | L (XeTeX/LuaTeX are outside DESIGN.md; needs a per-document fallback, see order) |
| A10 | Bibliography and index: BibTeX, biblatex/biber, makeindex | Old: `\bibliography` warns that the compiler doesn't read it (`crates/compiler/src/parser.rs:8585-8592`); biblatex emulation (`crates/compiler/src/biblatex.rs` with `crates/bibliography`); `\index` accepted but not typeset (`crates/compiler/src/supported.rs:789`) | **missing** | The host runs real bibtex/biber/makeindex since #1296, but only on `COMPILE.external_tools: "auto"`. The Swift request has no such field (grep: 0 hits in `apps/mac/Sources`) and announces protocol 3.1 (`apps/mac/Sources/FlashTeXDisplayListV3/DisplayListV3.swift:18-19`). The Rust client has the field (`crates/display-list-v3/src/client.rs:102`) ✔ | Citations show `[?]` unless a `.bbl` already exists; biblatex documents are worse than on the old path | S to wire; M with trust UI |
| A11 | Tool-run status (`TOOL` run/done/skip/settled; "would have run" notice) | none on the old path | **missing** | `EngineV3Reader.handle` has no `TOOL` case, so `.other` is dropped (`EngineV3Session.swift:790-829`) | Tool runs and their failures are invisible | M |
| A12 | Shell escape and project trust | The old compiler has no `\write18` *(inferred)* | **different** | The app sends `shell_escape: "default"` (`DL3Connection.swift:20`), texmf.cnf's restricted mode, for every project. *Pending #1332*: `EngineV3Trust.swift` turns shell escape off for quarantined projects until the user trusts them (owner 9A) | A downloaded project can run restricted `\write18` today | M (in #1332) |
| A13 | Crash recovery | Worker and controller: 3 relaunches a minute with backoff (`ShellModel.swift:350-352, 1466-1490`; `ShellModel+Controller.swift:345-362`) | **has** | 3 restarts a minute; pages kept and marked stale; stale hosts killed (`EngineV3Session.swift:196, 235-251`) | Comparable | — |
| A14 | Retry after the crash limit | Old retries with backoff | **partial** | After 3 crashes in a minute the user must toggle View ▸ Engine v3 Preview (`EngineV3Session.swift:242`) | An awkward recovery step | S |
| A15 | Runaway compile bound | Save timeout of 10 s (`ShellModel+Controller.swift:586-602`); release bound 40-250 ms (`ControllerRelease.swift:9-14`) | **partial** | No app-side watchdog. `--budget` and `--tool-timeout` are not passed (`EngineV3Host.swift:146-149`); a newer edit preempts the running compile | A `\loop` without end is bounded only by the next keystroke *(inferred)* | M |
| A16 | Cancel and supersede | One compile in flight plus a queue (`ShellModel.swift:1246-1250`) | **has** | A newer COMPILE preempts the running one; a cancelled DONE is handled (`EngineV3Session.swift:587-600`) | Better | — |
| A17 | Works without a TeX installation | Bundled Latin Modern metrics and resources (`BundledMetrics.swift:3-14`, `BundledResources.swift:3-14`) | **different** | The format is built from the user's TeX Live (D12). Only `pdftex.pool` is bundled (`EngineV3Host.swift:49-60`). There is no setting to choose a TeX Live, only a failure line (`EngineV3Session.swift:263-264`) | A Mac without TeX Live gets no v3 preview | M (needs a fallback and setup UI) |
| A18 | Output and aux location | Nothing written next to the source | **different** | `~/Library/Caches/FlashTeX/engine-v3/projects/<hash>-<pid>-<n>/out` (`EngineV3Session.swift:890-895, 417`). The manifest's `[project] output` is ignored | No `.pdf`, `.aux` or `.log` next to the source (see D1) | M |
| A19 | Tabs and multiple open documents | `DocumentTabBar.swift:4-12`; every member sent (`ShellModel.swift:1263`) | **has** | Every open buffer is sent (`EngineV3Session.swift:449-507`) | — | — |
| A20 | Multiple windows | One App-level `ShellModel` (`FlashTeXMacApp.swift:96`, ✔); New Window replaced (`:250`) | **has** (same limit) | Same single model, so one host per **app**. `apps/mac/docs/engine-v3-preview.md` and `EngineV3Session.swift:16` wrongly say one host per window | Docs only | S (doc fix) |
| A21 | Compile status and log | Status line plus Problems panel. `Transcript.swift` is a developer RPC trace (`FLASHTEX_TRANSCRIPT`) | **partial** | The pane shows status and first error (`EngineV3Preview.swift:31-46`). TeX's `.log` sits in the cache `out` directory with no way to open it *(inferred)* | Errors with no line are hard to diagnose | M |

### B. Errors and warnings

**diag-v1 wiring today ✔**
- The **app side is on main.** The decoder is `apps/mac/Sources/FlashTeXDisplayListV3/DL3Diag.swift`. HELLO `accept` is at `EngineV3Session.swift:281`, and the column/range/trace/help mapping is at `:643-680`.
- The **host side is #1255, which is open.** `crates/flashtex-engine/src/host/server.rs:519-534` on main does not offer `diag-v1`, and `crates/display-list-v3/src/diag.rs` does not exist.
- So today's v3 diagnostics are scraped from the terminal (`server.rs:1004-1077`):
  - Errors carry file and line only, and the span is the whole line.
  - Warnings have no file.
  - Overfull and underfull boxes are not reported.
  - Every row's `code` is `"engine-v3"`.

| # | Feature | Old path (file:line) | v3 status | Evidence (v3) | User impact | Size |
|---|---|---|---|---|---|---|
| B1 | Problems panel: list, counts, severity filter | `ProblemsPanel.swift:24-56`; `ShellModel.swift:444-446` | **has** | Fed from `engineV3Diagnostics` (`ShellModel.swift:445`) | — | — |
| B2 | Precise spans: exact byte range, codes, labels | `RuntimeV1.Diagnostic` (`apps/mac/Sources/FlashTeXProtocol/RuntimeV1.swift:424-433`) | **partial** | Whole-line spans from scraped `DIAGNOSTIC`s (`EngineV3Session.swift:618-637`). *Pending #1255*: exact column, range, macro chain and help (93/93 positions vs pdflatex, per its evidence) | A click lands on the line, not on the command | M (merge #1255) |
| B3 | Warnings with a location; overfull/underfull boxes | Old typesetter reports boxes with spans (`crates/render-pipeline/src/typeset.rs:9278`) | **missing** | Warnings are sent with `file=None` (`server.rs:1076`), and box reports are not scraped. diag-v1 sends box reports as `info`, which the app drops (`EngineV3Session.swift:652`) | Bad boxes are silent | S-M |
| B4 | Inline underlines, gutter marks, Error Lens (and its settings) | `ShellModel.swift:605-619` → `ShellModel+DiagnosticRetention.swift:29-33`; `SourceEditorView.swift:235-236`; `ErrorLens.swift:36` | **missing** | Built only from the old `result` ✔ (`ShellModel.swift:607`: `guard let result`). With v3 on, old open-time marks show, rebased or withheld | No v3 errors in the editor; old-engine errors may show instead | M |
| B5 | Next and previous diagnostic (⌘⇧] / ⌘⇧[) | `Navigation.swift:634-676` | **missing** | Uses `editorMarkReport` and `result.diagnostics` | Steps through old-engine marks | S (after B4) |
| B6 | Jump to source from a panel row (Return, double-click) | `DiagnosticsPanel.swift:214-235` → `Navigation.swift:319-345` | **partial** (bug) | The v3 range is computed against the current text, but `navigateExactly` rebases it from the old `compiledDocuments` (`Navigation.swift:333, 377-378`) | After an edit, the jump can be refused or land in the wrong place *(inferred)* | S |
| B7 | Row location label; Copy Diagnostics as Text (⌘⌥C) | `DiagnosticsPanel.swift:78-96, 480-488` | **partial** | Line numbers come from the old `compiledDocuments` | Shows "bytes a..<b" or the wrong line | S |
| B8 | Panel status line ("compile failed: previous preview kept") | `ProblemsPanel.swift:44-46` → `ShellModel.swift:329` (`result?.status`) | **different** | Shows the old result's status | A stale old-engine failure note | S |
| B9 | Explanations (`flashtex-explain`, ExplanationMemo) | `ShellModel.swift:731-751`; `ExplanationMemo.swift:42-92` | **missing** (bug) | Never fetched for v3. Rows look up `explanation(resultID: model.resultID, index: i)`, pairing an old result id with a v3 row index (`DiagnosticsPanel.swift:517, 585, 607`) | No explanations, or an old one attached to an unrelated v3 row *(inferred)* | M |
| B10 | Quick fix ("Fix…" sheet, Tab caret fix) | `ShellModel.swift:644-728`; `EditorDiagnostics.swift:1132-1175` (needs `help.replacement` or `suggestion`) | **missing** | v3 help is text only (`EngineV3Session.swift:675`); diag-v1 has no replacement field | No one-step fixes | M-L |
| B11 | Create a missing include; Create or Fetch a missing package | `ProjectScaffold.swift:478-499`; `ProjectPackages.swift:95-133` (match the old compiler's message wording) | **missing** | TeX's wording (``File `x.sty' not found``) is not matched *(inferred)* | No one-click fix for a missing file | S |
| B12 | VoiceOver "3 errors, 1 warning" after each compile | `DiagnosticsPanel.swift:329`, called from `ShellModel.swift:49, 486` | **missing** | No call from `EngineV3*` | Blind users get no compile outcome | S |
| B13 | First error in the HUD and pane | `ShellChrome.swift:113-114` | **has** | `EngineV3Session.swift:563-579`; `EngineV3Preview.swift:35-37` | — | — |

### C. Preview pane

| # | Feature | Old path (file:line) | v3 status | Evidence (v3) | User impact | Size |
|---|---|---|---|---|---|---|
| C1 | Zoom In / Out / Actual Size / Fit Width (⌘= ⌘- ⌘0 ⌘9, title bar, palette); saved zoom | `FlashTeXMacApp.swift:186-193`; `TitleBar.swift:33-40`; `PreviewZoom.swift:112-116`; read at `PreviewView.swift:43`, `PreviewV2View.swift:1130` | **missing** | `previewZoom` never appears in `EngineV3*`. The scale is always fit-width (`EngineV3Preview.swift:271`). The commands look enabled but do nothing | No zoom | M |
| C2 | Fit Page (⌘⇧9) | `PreviewView.swift:72-79` via `ContentView.swift:330` | **missing** | As C1. (Also broken in v2: nothing there reports `previewFitPageZoom`) | No fit-page | S (with C1) |
| C3 | Pinch to zoom | `PreviewZoom.swift:61-74`, attached at `ContentView.swift:321, 335` | **missing** | Not attached to `PreviewV3Pane` (`ContentView.swift:318-319`) | Pinch does nothing | M (with C1) |
| C4 | Double-click for Fit Width | `ContentView.swift:370` | **different** | It fires but only sets `previewZoom`. The NSView's `mouseDown` also reverse-searches on each click *(inferred)* | No visible effect | S |
| C5 | Fit width capped at 100%; horizontal scrolling | `PreviewView.swift:42`, `PreviewV2View.swift:1129` (`min(1, …)`) | **different** | No cap (`EngineV3Preview.swift:271`); `hasHorizontalScroller = false` (`:69`) | Pages upscale in wide panes | S |
| C6 | Page Up / Page Down by page | `PreviewView.swift:69-71`; `PreviewV2View.swift:1164-1166`; `PreviewAnchor.swift:377-404` | **missing** | No `acceptsFirstResponder` or `keyDown` in `EngineV3Preview.swift` | No keyboard paging | S |
| C7 | Reading position kept across resize and page-count changes | `PreviewAnchor.swift:293`; `PreviewView.swift:63`; `PreviewV2View.swift:1157` | **missing** | `relayout()` rescales but never restores the scroll origin (`EngineV3Preview.swift:267-291`) | Resizing the split loses the place | M |
| C8 | Hyperref links: internal `\ref`/`\cite`/ToC and http/https/mailto URIs, with hand cursor and tooltip | `PreviewV2View.swift:960-962, 1324-1327, 1339-1342, 1368`; `ShellModel.swift:492-509`; `DisplayListLinks.swift:12` | **missing** | `DL3Page.links` and `dests` are decoded (`apps/mac/Sources/FlashTeXDisplayListV3/DisplayListV3.swift:157-196`) ✔ but never read in `FlashTeXMac`. A click on a link reverse-searches instead (`EngineV3Preview.swift:248-261`) ✔ | Links in the PDF are dead in the preview | M |
| C9 | Page readout "N / M" | `ContentView.swift:453-460` | **has** | Uses `engineV3.pageCount` (`ContentView.swift:453`) | — | — |
| C10 | Dark preview | `PreviewV2View.swift:1168, 1298`; `PreviewView.swift:87, 112` | **partial** | Pages go dark (`EngineV3Preview.swift:92, 351-358`), but the ground stays `surfaceGround` (`:22`) | A light gutter around dark pages | S |
| C11 | Smooth fonts in preview | `PreviewFontSmoothing.swift:15-46`; `PreviewV2View.swift:644-645` | **has** | `EngineV3Session.swift:131-149` | — | — |
| C12 | A persistent caret mark and paragraph band on the page | v1 `PreviewView.swift:170-179`; v2 `PreviewV2View.swift:1149, 1307-1310, 952-954` | **missing** | v3 only flashes on an explicit forward search (`EngineV3Preview.swift:225-243`) | No "you are here" on the page | M |
| C13 | Preview follows the caret while typing | `CaretFollow.swift`, `PreviewAnchor.swift:332` | **has** | `CaretFollow.swift:298`; `EngineV3Session.swift:541`; `EngineV3Preview.swift:200-224` | — | — |
| C14 | Forward search (⌘⇧J); v3 adds ⌘-click | `Navigation.swift:681-716` | **has** | `Navigation.swift:687`; `EngineV3Session.swift:165-176` | — | — |
| C15 | Reverse search (click a page) | `PreviewView.swift:204-208`; `PreviewV2View.swift:1339-1344` | **has** | `EngineV3Preview.swift:248-261` (also opens unopened files) | — | — |
| C16 | Hover highlight and source tooltip | `PreviewView.swift:196-203`; `PreviewV2View.swift:1314-1338, 1367-1373` | **missing** | No tracking area. `DL3SourceIndex.hit` already exists | No affordance before a click | S |
| C17 | "page N" label on each page | `PreviewView.swift:114-116`; `PreviewV2View.swift:1349-1352` | **missing** | `EngineV3PageView` has no label | Cosmetic | S |
| C18 | VoiceOver reads page text (lines, "Go to source") | `PreviewView.swift:111`; `PreviewV2View.swift:1361-1365`; `PreviewV2Accessibility.swift` | **missing** | Each page is an image labelled "Page N" (`EngineV3Preview.swift:326-328`) | VoiceOver users can't read the document | L |
| C19 | VoiceOver Pages rotor | `PreviewPagesRotor.swift:50, 239` | **missing** | No anchor probe or rotor in v3 | No page navigation by rotor | M |
| C20 | Pane accessibility value "Page x of y"; page-jump announcement | `ContentView.swift:331, 363` | **different** | Uses the old `toolbarPageCount` (`ShellModel.swift:316`) | VoiceOver gives the wrong total | S |
| C21 | Status chips and the HUD tooltip | `ContentView.swift:353-355, 417-481`; `ShellChrome.swift` | **different** | The HUD route is `.engineV3` (`ShellChrome.swift:86-111`), and the pane adds its own status card (`EngineV3Preview.swift:23-53`): two chips. The tooltip still names the old producer and capabilities (`ContentView.swift:479-481`). The capability chip is not cleared (`ShellChrome.swift:173`) | Duplicated or misleading status | S |
| C22 | Latency readout in the status bar | `ContentView.swift:558`; `ShellChrome.swift:121-124` (`lastLatencyMs`) | **different** | Shows the old worker's last latency; v3's own (`EngineV3Latency.swift`) is not shown | A wrong number | S |
| C23 | Stale-page treatment | v2 keeps the stale frame unchanged; HUD HISTORICAL chip (`ContentView.swift:435-437`) | **different** (intended) | Stale pages are dimmed with an orange border (`EngineV3Preview.swift:155-160`) | Intended change | — |
| C24 | Math hover preview (a formula bitmap in the editor) | `ContentView.swift:197-199` reads `displayListV2`; `MathHoverPreview.swift:49` | **missing** | No v3 source. It can crop an old-engine bitmap right after open or ⌘B *(inferred)* | No formula hover with v3 | M |
| C25 | Preview Debug Status (View menu) | `FlashTeXMacApp.swift:162`; `ShellModel.swift:126` | **partial** | Only the environment note (`EngineV3Preview.swift:45`) | Developer-facing | S |

### D. Export, print, CLI

| # | Feature | Old path (file:line) | v3 status | Evidence (v3) | User impact | Size |
|---|---|---|---|---|---|---|
| D1 | File ▸ Export PDF… (⌘⇧E), the toolbar Export button, the palette | `FlashTeXMacApp.swift:293-295`; `ExactPDFExport.swift:115-174`; `TitleBar.swift:156-170`; `CommandPalette.swift:103` | **missing** | No v3 check. Enabled from `displayListV2` (`ShellModel.swift:324-325`). With a worker attached it exports the **old engine's open-time frame**; with none it refuses with "Attach the render pipeline (⌘⇧R)" (`ExactPDFExport.swift:120`). The host offers `export` (`server.rs:532`) ✔, and `DL3CompileRequest.export` exists (`DL3Connection.swift:31, 49`) ✔, but the app never sets it | The PDF the user saves is not the pdfTeX preview on screen | M |
| D2 | Export session: atomic replace, overwrite-conflict check, progress, Cancel | `ExportSession.swift:72-131` (hard-codes `from-v2` at `:90`); `ShellModel+ExportSession.swift:30-48`; `ContentView.swift:601-608` | **missing** | Only reachable through D1; the process is fixed to `flashtex-pdf-exact from-v2` | No safe v3 export | M (with D1) |
| D3 | File ▸ Print… (⌘P) | `FlashTeXMacApp.swift:315-319`; `PrintController.swift:78-115, 196-208` | **missing** | Same source as D1. `prepareDocument(pdfData:)` (`PrintController.swift:141`) could take v3's PDF bytes | Prints old-engine or stale pages, or nothing | S (after D1) |
| D4 | File ▸ Print Source… | `PrintController.swift:132-138, 211-217` | **has** | Depends only on the editor text | — | — |
| D5 | `flashtex build` / `check` / `watch` CLI (`flashtex install-cli` links it into `/usr/local/bin`) | `crates/flashtex-cli/src/main.rs:146-167, 237-312, 1275` | **different** | Links only the old engine (`crates/flashtex-cli/Cargo.toml:21-24`). v3 has `flashtex-host` and `flashtex-initex` (pdftex-style CLI) but no `flashtex build` route | CLI PDFs come from the old engine and are deleted with it | L |

### E. Companion, capture, history

| # | Feature | Old path (file:line) | v3 status | Evidence (v3) | User impact | Size |
|---|---|---|---|---|---|---|
| E1 | iPad Nearby pairing and capture inbox | `NearbyProtocol.swift:21-206`; `ShellModel+Nearby.swift:20-56` | **has** | Engine-independent: Nearby-v1 sends destinations and caret context only, never pages (`NearbyDestination.swift:62-86`) | — | — |
| E2 | Capture destination `projectId` | `ShellModel.swift:558` (`result?.projectId ?? "demo"`) | **different** | Comes from the old result, or "demo" *(inferred)* | Destinations may be mis-keyed | S |
| E3 | Capture proposal preview (shadow compile, page thumbnail, new diagnostics) | `ProposalPreview.swift:7-18, 209, 389-442` | **different** | Runs its own old-engine worker | The proposal's preview and diagnostics don't match the v3 pane | M |
| E4 | Bridge anchors following typing | `ShellModel+Bridge.swift:210-211` | **has** | Called on every edit regardless of the flag | — | — |
| E5 | Durable History and the controller edit ledger (developer route) | `EditHistoryPanel.swift`; `ShellModel+Controller.swift:147`; `HistoricalPreview.swift:445-462` | **missing** | `controllerSubmitEdit` runs only from `scheduleAutoCompile` (`ShellModel.swift:1000`), which v3 skips | The history doesn't grow under v3 (developer-only route) | S |

### F. Settings that only the old path honours

These are rows above, gathered here for the settings pane:
- auto-compile (A3)
- zoom and fit modes (C1-C3)
- Error Lens and its warnings toggle (B4)
- project fonts and packages, and texinputs (A8, A9)
- Preview Debug Status (C25)

`FLASHTEX_PREVIEW_V2`, `FLASHTEX_LAYOUT_CAPABILITIES`, the display-delta switches and Helper Display Candidates are old-only by design and retire with it.

v3's only user-facing switch is View ▸ Engine v3 Preview (`FlashTeXMacApp.swift:165`), which is app-wide and remembered. There is **no per-document engine choice and no Settings entry**, and owner decision P5 "default per document" needs both (see the order below).

### Not parity gaps: absent on both paths

These are not regressions, so they are not counted. They are listed because the owner may want them, and v3's display list and source map make several of them cheap.

| Feature | Note |
|---|---|
| SyncTeX file for Skim and other external viewers | No emitter anywhere. The v3 engine refuses `-synctex` (`crates/flashtex-engine/src/cli.rs:32-36, 73, 98`) ✔. v3 has an in-app source map only. Size L |
| `% !TEX root` / `% !TEX program` magic comments | grep finds none in `apps/` or `crates/` |
| A PDF beside the source; Save PDF; Share; Open in Preview; Reveal PDF | Neither path does this; once D1 exists, each is S |
| Copy text, text selection and find in the PDF | No selection or find on either pane. `PreviewTextCache` is a draw cache |
| Page thumbnails and a PDF-outline sidebar | The outline sidebar is a lexical source scan (`DocumentOutline.swift:3-9`), so it is unaffected |
| URL scheme, Services, AppIntents/Shortcuts, AppleScript, document types, Dock drop | None in `Info.plist.template` or `Sources`. The only launch hook is `FLASHTEX_OPEN` (`FlashTeXMacApp.swift:110`) |
| Raw `.log` viewer, draft mode | Absent on both |
| 512 px tiles above 3 px/pt (DESIGN §6.2) | #1228 (v2 tiles) was closed unmerged, so neither pane has them |

## Counts

These cover the rows in tables A-E (69 rows), tallied from the table.

| Status | Rows |
|---|---|
| has | 14 |
| partial | 9 |
| missing | 29 |
| different | 17 |

By section:

| Section | has | partial | missing | different |
|---|---|---|---|---|
| A. Compile and projects (21) | 4 | 4 | 6 | 7 |
| B. Errors and warnings (13) | 2 | 3 | 7 | 1 |
| C. Preview (25) | 5 | 2 | 12 | 6 |
| D. Export, print, CLI (5) | 1 | 0 | 3 | 1 |
| E. Companion, capture, history (5) | 2 | 0 | 1 | 2 |

Sizes of the 55 rows that are not **has**: 28 S, 1 S-M, 20 M, 1 M-L (B10), 3 L (A9, C18, D5), and 2 need no work (A6, C23: intended differences).

## Proposed order for closing the gaps

### Tier 0: blockers for "new engine default per document"

Each item would make a user worse off on a document switched to v3.

1. **One engine at a time (A2, then B6-B8, C20-C22, E2). S-M.**
   - With v3 active for a document, stop or detach the old worker.
   - Clear `result`, `displayListV2` and `compiledDocuments` for that document.
   - Keep a v3 "compiled text" snapshot per DONE for rebasing diagnostics and navigation (this also fixes the span drift when typing during a compile).
   - Removes the silent mixing of the two engines that most `different` rows come from. Do it first, because otherwise every other fix is tested against polluted state.
2. **⌘B and auto-compile semantics on v3 (A1, A3). S.**
   - ⌘B sends a non-edit COMPILE. That relinks new files, covering A5.
   - Auto-compile off means v3 compiles on ⌘B only.
3. **Export PDF, Print and the export session through the host's `export: true` (D1-D3). M.**
   - Wait for DONE, then publish atomically through `ExportSession`'s publish step.
   - Without this, a v3 document exports the old engine's output.
4. **Bibliography and index (A10, A11, A12).**
   - Swift client: protocol 3.2 and `external_tools: "auto"` for trusted projects (S).
   - `TOOL` status in the pane (M).
   - Merge #1332 for trust, so that tools and `\write18` follow owner 9A.
5. **Diagnostics in the editor (B2-B5, B12).**
   - Merge #1255.
   - Build editor marks, Error Lens and next/previous from v3 diagnostics.
   - Report bad boxes as warnings, not dropped `info`.
   - Add the VoiceOver compile announcement.
6. **Zoom and navigation (C1-C7). M.** Zoom commands, fit page, pinch, Page Up/Down, reading-position anchor, 100% cap and horizontal scroll. This overlaps the tiles work of DESIGN §6.2.
7. **Hyperref links (C8). M.** The data is already decoded.
8. **Per-document engine selection with automatic fallback (F, A9, A17, A8). M.**
   - A per-document choice; the default is v3 only when the environment can run it.
   - Fall back to the old engine (until it is deleted) when:
     - there is no TeX Live, or the format failed;
     - the project needs fonts, packages or texinputs that v3 doesn't honour.
   - This is what "default per document" literally needs, and it keeps A8/A9/A17 from being regressions in the meantime.
9. **External file changes (A4, A5). S.** Route the include watchers to a v3 recompile.

### Tier 1: before retiring the old engine (P5 exit)

Here "lose nothing" has to be literally true.

10. **Accessibility** (C18, C19, C16, C17). VoiceOver page text from the display list's glyphs and the source map, plus the Pages rotor. L.
11. **Explanations and quick fixes on v3** (B9, B10, B11). Fix the cross-wired index first (S). Add a replacement field to diag-v1 or a TeX-message catalogue (M-L), and match TeX's wording for missing files and packages.
12. **Caret mark on the page and math hover** (C12, C24). M.
13. **Capture proposal preview on v3** (E3). M.
14. **`flashtex build/check/watch` on the new engine** (D5). L. Either a new backend or a successor CLI over `flashtex-host`, before the old engine's crates are deleted.
15. **Runaway compile bound and crash-limit retry** (A15, A14). Pass `--budget` and `--tool-timeout`; offer a Retry button. S-M.
16. **Log access and output location** (A21, A18). "Show Log" and an optional output directory from the manifest. M.
17. **Durable history under v3** (E5). S, or a decision to retire it with the controller.

### Tier 2: polish

18. Dark gutter (C10), double-click (C4), duplicate status chips and the HUD tooltip (C21), latency readout (C22), debug status (C25), the main-file heuristic and `% !TEX root` (A7), and the one-host-per-app doc fix (A20).
